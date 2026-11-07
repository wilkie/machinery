//! The emitter: what a generated machine builds a compiled run's host code
//! with, one step at a time.
//!
//! The machine's generated code runs at compile time, with what the step
//! was decoded with (its operands, latches, pc) in hand. A value is either
//! known then ([`V::K`], folded in Rust: no host code) or computed by the
//! host code ([`V::D`]); every operation folds when its inputs are known, so
//! a choice made by an operand or a latch costs the compiled code nothing.
//!
//! Places in the machine are byte offsets with a native type ([`Place`]).
//! Registers are written through to the machine at once (so a Rust call
//! from the compiled code sees them), and what is known of their values is
//! remembered until a call or a join forgets it. Locals live in host
//! registers (Cranelift variables), and are known values while they can be.

use std::collections::HashMap;

use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{types, AbiParam, Block, InstBuilder, MemFlags, SigRef, Signature, Type, Value};
use cranelift_codegen::isa::CallConv;
use cranelift_frontend::{FunctionBuilder, Variable};

/// A native integer type: what the generated machine stores a value as.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum N {
    Bool,
    U8,
    U16,
    U32,
    U64,
    U128,
    I8,
    I16,
    I32,
    I64,
    I128,
}

impl N {
    /// Bits of the native type (a bool is a byte, 0 or 1).
    pub fn width(self) -> u32 {
        match self {
            N::Bool | N::U8 | N::I8 => 8,
            N::U16 | N::I16 => 16,
            N::U32 | N::I32 => 32,
            N::U64 | N::I64 => 64,
            N::U128 | N::I128 => 128,
        }
    }
    pub fn signed(self) -> bool {
        matches!(self, N::I8 | N::I16 | N::I32 | N::I64 | N::I128)
    }
    /// The native type for `bits` bits.
    pub fn of_bits(bits: u32, signed: bool) -> N {
        match (bits, signed) {
            (0..=8, false) => N::U8,
            (9..=16, false) => N::U16,
            (17..=32, false) => N::U32,
            (33..=64, false) => N::U64,
            (_, false) => N::U128,
            (0..=8, true) => N::I8,
            (9..=16, true) => N::I16,
            (17..=32, true) => N::I32,
            (33..=64, true) => N::I64,
            (_, true) => N::I128,
        }
    }
    fn ir(self) -> Type {
        match self.width() {
            8 => types::I8,
            16 => types::I16,
            32 => types::I32,
            64 => types::I64,
            _ => types::I128,
        }
    }
    fn mask(self) -> u128 {
        if self.width() == 128 {
            u128::MAX
        } else {
            (1u128 << self.width()) - 1
        }
    }
    /// `v` (this type's bits) sign- or zero-extended to 128 bits.
    fn widen(self, v: u128) -> i128 {
        let w = self.width();
        if self.signed() && w < 128 {
            ((v << (128 - w)) as i128) >> (128 - w)
        } else {
            v as i128
        }
    }
}

/// A native Rust value's type and bits: what a known value is made from.
pub trait Native: Copy {
    const N: N;
    fn bits(self) -> u128;
}
macro_rules! native {
    ($($t:ty => $n:ident, $u:ty);* $(;)?) => {$(
        impl Native for $t {
            const N: N = N::$n;
            fn bits(self) -> u128 { self as $u as u128 }
        }
    )*};
}
native!(u8 => U8, u8; u16 => U16, u16; u32 => U32, u32; u64 => U64, u64; u128 => U128, u128;
        i8 => I8, u8; i16 => I16, u16; i32 => I32, u32; i64 => I64, u64; i128 => I128, u128);
impl Native for bool {
    const N: N = N::Bool;
    fn bits(self) -> u128 {
        self as u128
    }
}

/// The native type of a field, from a projection to it: `n_of(|m: &M|
/// &m.field)`.
pub fn n_of<M, T: Native>(_: fn(&M) -> &T) -> N {
    T::N
}

/// A known value from a native Rust one.
pub fn k<T: Native>(v: T) -> V {
    V::K(v.bits())
}

/// A value: known at compile time (its bits, zero-extended from its native
/// width), or computed by the compiled code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum V {
    K(u128),
    D(Value),
}

impl V {
    pub fn known(self) -> Option<u128> {
        match self {
            V::K(v) => Some(v),
            V::D(_) => None,
        }
    }
}

/// A place in the machine: a field at a byte offset, or bits of one, or a
/// choice among places made by a value.
#[derive(Clone, Debug)]
pub enum Place {
    /// A whole field; a write bumps the epochs in `bump` (see
    /// [`Em::set_epochs`]).
    Mem { off: usize, n: N, bump: u8 },
    /// `width` bits from bit `shift` of a field of type `reg`, read as `n`
    /// (a bool: nonzero; signed: sign-extended).
    Bits { off: usize, reg: N, shift: u32, width: u32, n: N, bump: u8 },
    /// A local.
    Local(Slot),
    /// A field of element `index` of an array of `count` elements of
    /// `stride` bytes at `base`: the field at `off` within it, of type `n`.
    Elem { base: usize, stride: usize, count: usize, off: usize, index: V, n: N, bump: u8 },
    /// The first arm whose patterns match `scrut` (of type `n`).
    Switch { scrut: V, n: N, arms: Vec<(Vec<Pat>, Place)> },
}

/// A pattern of a `match` arm.
#[derive(Clone, Copy, Debug)]
pub enum Pat {
    Any,
    Is(i128),
    Range(i128, i128),
}

/// A local's handle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Slot(usize);

#[derive(Clone, Copy, Debug)]
struct SlotState {
    var: Variable,
    n: N,
    /// Known value; else the variable holds it.
    k: Option<u128>,
}

/// What is known of a register field: its value (a host value defined
/// where it dominates every use), or nothing (load it).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Known {
    K(u128),
    D(Value),
}

/// An open region: what was known at its start, where its returns go,
/// and what each return left known.
type Region = (State, Block, Vec<Option<(State, Block)>>);

#[derive(Clone)]
struct State {
    slots: Vec<Option<u128>>,
    regs: HashMap<(usize, N), Known>,
    dirty: Vec<(usize, N)>,
    pieces: HashMap<(usize, N), Vec<Piece>>,
}

/// What a run knows of the machine where it goes round (`Em::loop_back`):
/// the fields it has as host values (not known constants), the bits
/// written into each (`Piece`s' shifts and widths), and those it owes.
#[derive(Clone, Debug, Default)]
pub(crate) struct LoopPlan {
    carried: Vec<(usize, N)>,
    shapes: Vec<Vec<(u32, u32)>>,
    owed: Vec<(usize, N)>,
}

impl LoopPlan {
    pub(crate) fn is_empty(&self) -> bool {
        self.carried.is_empty()
    }
}

/// What a call leaves of the machine (`Em::call_fx`, `Em::call_touching`).
#[derive(Clone, Copy)]
enum Fx<'s> {
    Keep(&'s [(usize, usize)], &'s [(usize, usize)]),
    Touch(&'s [(usize, usize)]),
}

/// A bit field written into a cached field and not yet put in it: the
/// field is its cached value with these bits replaced (`settle`). A read
/// of the bits gets the value as written; bits written again before the
/// field is needed whole are never put in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Piece {
    shift: u32,
    width: u32,
    /// The bits' value (masked to `width`, unshifted, of the field's type).
    v: V,
}


/// Binary operations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    Add,
    Sub,
    Mul,
    And,
    Or,
    Xor,
    /// Shifts as Rust's `checked_shl/shr(n).unwrap_or(0 or -1)`.
    Shl,
    Shr,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    /// Division and remainder (a zero divisor traps, where Rust panics).
    Div,
    Rem,
}

/// Builds one compiled run (see the module docs).
pub struct Em<'a, 'f> {
    pub(crate) b: &'a mut FunctionBuilder<'f>,
    pub(crate) ptr: Type,
    pub(crate) cc: CallConv,
    pub(crate) m: Value,
    pub(crate) out: Block,
    /// The step being emitted.
    pub(crate) k: usize,
    live: bool,
    slots: Vec<SlotState>,
    regs: HashMap<(usize, N), Known>,
    sigs: HashMap<(usize, bool), SigRef>,
    regions: Vec<Option<Region>>,
    epochs: Option<[usize; 3]>,
    /// Fields that calls change behind the code's back: never remembered.
    volatile: Vec<(usize, usize)>,
    bumped: bool,
    /// Calls emitted that may write the machine's memory (code): see
    /// `code_writes`.
    code_calls: usize,
    /// Inside a trial (`try_inline`): the instruction count past which it
    /// stops emitting.
    trial_limit: Option<usize>,
    trial_over: bool,
    /// Instructions of trials that failed (dead code): not counted against
    /// a trial that contains them.
    wasted: usize,
    /// Fields written but not yet stored.
    dirty: Vec<(usize, N)>,
    /// A region's return that leaves the run: by region, the slot that
    /// holds the return, the call made first and whether the step was
    /// made (`exit_on_true_return`).
    ret_exits: Vec<Option<(Slot, usize, bool)>>,
    next_ret_exit: Option<(usize, bool)>,
    /// Bit fields written into dirty fields, by field (see `Piece`).
    pieces: HashMap<(usize, N), Vec<Piece>>,
    /// Added to a step's number for the steps made: minus `from`, plus the
    /// run's length for each time it went round (`loop_back`).
    made_base: Variable,
    /// The first step's block and the run's length (`loop_back`).
    pub(crate) first: Option<Block>,
    pub(crate) steps: usize,
    /// Going round: what the first step takes (a second build), and what
    /// a first build saw.
    plan: Option<LoopPlan>,
    seen: Option<LoopPlan>,
    /// The first step's parameters: each carried field, and its bits.
    loop_params: Vec<(Value, Vec<Value>)>,
    /// The pads the switch enters later entries through.
    pub(crate) entry_pads: HashMap<usize, Block>,
    /// Values that are one of two known constants: the condition, the
    /// value when it holds, and when not (`select`). Arithmetic with a
    /// constant goes into both (a time charged one of two ways stays one
    /// of two values through the sums and the comparisons).
    sels: HashMap<Value, (Value, u128, u128)>,
    /// What a condition says of a field: the field, its value when the
    /// condition was made, the constant it was compared with, and whether
    /// the condition holds when they are equal. A branch on it knows the
    /// field on each side (`learn`).
    facts: HashMap<Value, ((usize, N), Value, u128, bool)>,
    /// Fields narrower than their native type (`declare_bits`).
    bits: HashMap<(usize, N), u32>,
    /// Conditions made of two (`and`, `or` of bools): a branch on one
    /// learns of both where it can.
    combos: HashMap<Value, (Op, V, V)>,
}

impl<'a, 'f> Em<'a, 'f> {
    pub(crate) fn new(b: &'a mut FunctionBuilder<'f>, ptr: Type, cc: CallConv, m: Value, out: Block, made_base: Variable) -> Self {
        Em { b, ptr, cc, m, out, k: 0, live: false, slots: Vec::new(), regs: HashMap::new(), sigs: HashMap::new(), regions: Vec::new(), epochs: None, volatile: Vec::new(), bumped: false, code_calls: 0, trial_limit: None, trial_over: false, wasted: 0, dirty: Vec::new(), ret_exits: Vec::new(), next_ret_exit: None, pieces: HashMap::new(), made_base, first: None, steps: 0, plan: None, seen: None, loop_params: Vec::new(), entry_pads: HashMap::new(), sels: HashMap::new(), facts: HashMap::new(), bits: HashMap::new(), combos: HashMap::new() }
    }

    /// Code is being emitted (not after an exit or a return).
    pub fn live(&self) -> bool {
        self.live
    }

    pub(crate) fn begin_step(&mut self, k: usize, block: Block, entry: bool) {
        // An entry past the first, which the switch enters through a pad:
        // the step before passes what it knows as host values, the pad
        // loads them.
        if entry && k > 0 {
            if let Some(pad) = self.entry_pads.get(&k).copied() {
                let plan = if self.live { self.loop_seen() } else { LoopPlan::default() };
                let params: Vec<(Value, Vec<Value>)> = plan
                    .carried
                    .iter()
                    .zip(&plan.shapes)
                    .map(|((_, n), sh)| {
                        let b = self.b.append_block_param(block, n.ir());
                        let ps = sh.iter().map(|_| self.b.append_block_param(block, n.ir())).collect();
                        (b, ps)
                    })
                    .collect();
                let limit = self.trial_limit.take();
                if self.live {
                    let args = self.carry_args(&plan.carried, &plan.shapes);
                    self.dirty.retain(|d| !plan.carried.contains(d));
                    self.flush();
                    self.b.ins().jump(block, &args);
                }
                self.b.switch_to_block(pad);
                self.b.seal_block(pad);
                self.live = true;
                self.regs.clear();
                self.pieces.clear();
                self.dirty.clear();
                let args = self.carry_args(&plan.carried, &plan.shapes);
                self.b.ins().jump(block, &args);
                self.trial_limit = limit;
                self.b.switch_to_block(block);
                self.k = k;
                self.b.set_srcloc(cranelift_codegen::ir::SourceLoc::new(k as u32));
                self.regs.clear();
                self.pieces.clear();
                self.dirty.clear();
                for ((key, (v, bits)), sh) in plan.carried.iter().zip(params).zip(&plan.shapes) {
                    self.regs.insert(*key, Known::D(v));
                    if !sh.is_empty() {
                        let ps = sh.iter().zip(bits).map(|(&(shift, width), b)| Piece { shift, width, v: V::D(b) }).collect();
                        self.pieces.insert(*key, ps);
                    }
                    if plan.owed.contains(key) {
                        self.dirty.push(*key);
                    }
                }
                self.live = true;
                return;
            }
        }
        if self.live {
            if entry {
                self.flush();
            }
            self.b.ins().jump(block, &[]);
        }
        self.b.switch_to_block(block);
        self.k = k;
        self.live = true;
        // The step's code carries its number, for reading dumps and profiles.
        self.b.set_srcloc(cranelift_codegen::ir::SourceLoc::new(k as u32));
        // Entered from the switch too: nothing is known.
        if entry {
            self.regs.clear();
            self.pieces.clear();
            self.dirty.clear();
            // The first step of a run that goes round: what it takes.
            if k == 0 {
                if let Some(plan) = self.plan.clone() {
                    for ((key, (v, bits)), sh) in plan.carried.iter().zip(self.loop_params.clone()).zip(&plan.shapes) {
                        self.regs.insert(*key, Known::D(v));
                        if !sh.is_empty() {
                            let ps = sh.iter().zip(bits).map(|(&(shift, width), b)| Piece { shift, width, v: V::D(b) }).collect();
                            self.pieces.insert(*key, ps);
                        }
                        if plan.owed.contains(key) {
                            self.dirty.push(*key);
                        }
                    }
                }
            }
        }
    }

    pub(crate) fn finish(&mut self, n: usize) {
        if self.live {
            self.flush();
            let c = self.b.ins().iconst(types::I32, n as i64);
            let bv = self.b.use_var(self.made_base);
                let c = self.b.ins().iadd(c, bv);
            self.b.ins().jump(self.out, &[c.into()]);
            self.live = false;
        }
    }

    /// The machine pointer.
    fn base(&mut self) -> Value {
        self.m
    }

    fn val(&mut self, v: V, n: N) -> Value {
        match v {
            V::D(x) => x,
            V::K(c) => self.konst(n, c),
        }
    }

    fn konst(&mut self, n: N, c: u128) -> Value {
        let c = c & n.mask();
        if n.width() == 128 {
            let lo = self.b.ins().iconst(types::I64, c as u64 as i64);
            let hi = self.b.ins().iconst(types::I64, (c >> 64) as u64 as i64);
            self.b.ins().iconcat(lo, hi)
        } else {
            self.b.ins().iconst(n.ir(), c as u64 as i64)
        }
    }

    // -- the machine ----------------------------------------------------------------

    /// The field at `off` (no caching: for fields the step's own code does
    /// not write).
    pub fn load(&mut self, off: usize, n: N) -> V {
        if !self.live {
            return V::K(0);
        }
        self.flush_overlapping(off, n, None);
        let m = self.base();
        V::D(self.b.ins().load(n.ir(), MemFlags::trusted(), m, off as i32))
    }

    /// The `n` at host address `p` (a u64 the code computed): memory
    /// outside the machine, which the emitter does not remember.
    pub fn load_at(&mut self, p: V, n: N) -> V {
        if !self.live {
            return V::K(0);
        }
        let a = self.val(p, N::U64);
        V::D(self.b.ins().load(n.ir(), MemFlags::trusted(), a, 0))
    }

    /// Store `v` (an `n`) at host address `p`.
    pub fn store_at(&mut self, p: V, n: N, v: V) {
        if !self.live {
            return;
        }
        let a = self.val(p, N::U64);
        let x = self.val(v, n);
        self.b.ins().store(MemFlags::trusted(), x, a, 0);
    }

    /// Store `v` in the field at `off`.
    pub fn store(&mut self, off: usize, n: N, v: V) {
        if !self.live {
            return;
        }
        // The field holds it already.
        if let V::K(c) = v {
            if self.regs.get(&(off, n)) == Some(&Known::K(c & n.mask())) && !self.pieces.contains_key(&(off, n)) {
                return;
            }
        }
        // The whole field is written: its bits written before are not.
        self.pieces.remove(&(off, n));
        // Other fields it overlaps are written first, then forgotten.
        self.flush_overlapping(off, n, Some((off, n)));
        self.regs.retain(|(o, m), _| (*o, *m) == (off, n) || *o + m.width() as usize / 8 <= off || off + n.width() as usize / 8 <= *o);
        // Stored when the machine must hold it (a call, an exit, a join).
        let k = match v {
            V::K(c) => Known::K(c & n.mask()),
            V::D(x) => Known::D(x),
        };
        self.regs.insert((off, n), k);
        if !self.dirty.contains(&(off, n)) {
            self.dirty.push((off, n));
        }
    }

    /// Store every field written since the machine last held it.
    pub fn flush(&mut self) {
        if !self.live {
            self.dirty.clear();
            return;
        }
        for (off, n) in std::mem::take(&mut self.dirty) {
            self.settle_why("flush", off, n);
            let x = match self.regs.get(&(off, n)) {
                Some(Known::K(c)) => {
                    let c = *c;
                    self.konst(n, c)
                }
                Some(Known::D(v)) => *v,
                None => continue,
            };
            { let m = self.base(); self.b.ins().store(MemFlags::trusted(), x, m, off as i32); }
        }
    }

    /// Store the written fields overlapping `n` at `off` (but `except`).
    fn flush_overlapping(&mut self, off: usize, n: N, except: Option<(usize, N)>) {
        let w = n.width() as usize / 8;
        let hit: Vec<(usize, N)> = self.dirty.iter().copied().filter(|(o, m)| Some((*o, *m)) != except && off < o + m.width() as usize / 8 && *o < off + w).collect();
        for (o, m) in hit {
            self.settle_why("flush_overlapping", o, m);
            self.dirty.retain(|d| *d != (o, m));
            if let Some(k) = self.regs.get(&(o, m)).copied() {
                let x = match k {
                    Known::K(c) => self.konst(m, c),
                    Known::D(v) => v,
                };
                { let m = self.base(); self.b.ins().store(MemFlags::trusted(), x, m, o as i32); }
            }
        }
    }

    fn settle_why(&mut self, why: &str, off: usize, n: N) {
        if self.live && self.pieces.contains_key(&(off, n)) && std::env::var_os("MACHINE_JIT_SETTLE").is_some() {
            eprintln!("settle {why} k={} off={off:#x} {:?} cold={}", self.k, n, self.b.current_block().map(|b| self.b.func.layout.is_cold(b)).unwrap_or(false));
            if std::env::var("MACHINE_JIT_SETTLE").as_deref() == Ok("bt") {
                eprintln!("{}", std::backtrace::Backtrace::force_capture());
            }
        }
        self.settle(off, n);
    }

    /// Put the bit fields written into the field at `off` in it.
    fn settle(&mut self, off: usize, n: N) {
        let Some(ps) = self.pieces.remove(&(off, n)) else { return };
        if !self.live {
            return;
        }
        // Not a trial's to stop: a flush or a join puts the field in, and
        // goes on emitting (the instructions count, after).
        let limit = self.trial_limit.take();
        self.settle_ops(off, n, &ps);
        self.trial_limit = limit;
    }

    fn settle_ops(&mut self, off: usize, n: N, ps: &[Piece]) {
        let base = match self.regs.get(&(off, n)) {
            Some(Known::K(c)) => V::K(*c),
            Some(Known::D(v)) => V::D(*v),
            None => return,
        };
        let mut clear = 0u128;
        let mut set = 0u128;
        for p in ps {
            let m = if p.width >= 128 { u128::MAX } else { (1u128 << p.width) - 1 };
            clear |= m << p.shift;
            if let V::K(c) = p.v {
                set |= (c & m) << p.shift;
            }
        }
        let mut x = self.op(Op::And, base, V::K(!clear & n.mask()), n);
        x = self.op(Op::Or, x, V::K(set & n.mask()), n);
        for p in ps {
            if let V::D(_) = p.v {
                let t = self.op(Op::Shl, p.v, V::K(p.shift as u128), n);
                x = self.op(Op::Or, x, t, n);
            }
        }
        let k = match x {
            V::K(c) => Known::K(c & n.mask()),
            V::D(v) => Known::D(v),
        };
        self.regs.insert((off, n), k);
    }

    /// Settle every field the bytes at `off` (`n` wide) overlap.
    fn settle_overlapping(&mut self, off: usize, n: N) {
        let w = n.width() as usize / 8;
        let hit: Vec<(usize, N)> = self.pieces.keys().copied().filter(|(o, m)| off < o + m.width() as usize / 8 && *o < off + w).collect();
        for (o, m) in hit {
            self.settle_why("overlapping_read", o, m);
        }
    }

    /// Read a place.
    pub fn read(&mut self, p: &Place) -> V {
        self.trial_check();
        if !self.live {
            return V::K(0);
        }
        if let Place::Mem { off, n, .. } = p {
            if !self.pieces.is_empty() {
                self.settle_overlapping(*off, *n);
            }
        }
        match p {
            Place::Mem { off, n, .. } => match self.regs.get(&(*off, *n)) {
                Some(Known::K(c)) => V::K(*c),
                Some(Known::D(v)) => V::D(*v),
                None => {
                    self.flush_overlapping(*off, *n, None);
                    let m = self.base(); let v = self.b.ins().load(n.ir(), MemFlags::trusted(), m, *off as i32);
                    self.regs.insert((*off, *n), Known::D(v));
                    V::D(v)
                }
            },
            Place::Bits { off, reg, shift, width, n, .. } => {
                let mask = if *width >= 128 { u128::MAX } else { (1u128 << width) - 1 };
                let lo = *shift;
                let hi = shift + width;
                let ps = self.pieces.get(&(*off, *reg)).cloned().unwrap_or_default();
                let raw = if let Some(p) = ps.iter().find(|p| p.shift == lo && p.width == *width) {
                    // The bits as written.
                    p.v
                } else {
                    if ps.iter().any(|p| p.shift < hi && lo < p.shift + p.width) {
                        self.settle_why("bits_read", *off, *reg);
                    }
                    // Other bits written leave these as the field has them.
                    let whole = match self.regs.get(&(*off, *reg)) {
                        Some(Known::K(c)) if self.pieces.contains_key(&(*off, *reg)) => V::K(*c),
                        Some(Known::D(v)) if self.pieces.contains_key(&(*off, *reg)) => V::D(*v),
                        _ => self.read(&Place::Mem { off: *off, n: *reg, bump: 0 }),
                    };
                    let s = self.op(Op::Shr, whole, V::K(*shift as u128), *reg);
                    self.op(Op::And, s, V::K(mask), *reg)
                };
                match n {
                    N::Bool => self.op(Op::Ne, raw, V::K(0), *reg),
                    _ if n.signed() => {
                        let x = self.cast(raw, *reg, *n);
                        let kk = n.width() - width;
                        let l = self.op(Op::Shl, x, V::K(kk as u128), *n);
                        self.op(Op::Shr, l, V::K(kk as u128), *n)
                    }
                    _ => self.cast(raw, *reg, *n),
                }
            }
            Place::Local(s) => self.get(*s),
            Place::Switch { scrut, n, arms } => self.read_switch(*scrut, *n, arms),
            Place::Elem { base, stride, count, off, index, n, bump } => match index {
                V::K(k) if (*k as usize) < *count => self.read(&Place::Mem { off: base + (*k as usize) * stride + off, n: *n, bump: *bump }),
                V::K(_) => {
                    self.trap();
                    V::K(0)
                }
                V::D(_) => {
                    let a = self.elem_addr(*base, *stride, *count, *off, *index, false);
                    V::D(self.b.ins().load(n.ir(), MemFlags::trusted(), a, 0))
                }
            },
        }
    }

    fn read_switch(&mut self, scrut: V, n: N, arms: &[(Vec<Pat>, Place)]) -> V {
        let Some(((pats, p), rest)) = arms.split_first() else { return V::K(0) };
        if rest.is_empty() {
            return self.read(p);
        }
        let rn = self.place_n(p);
        let c = self.test(scrut, n, pats);
        let (p, rest) = (p.clone(), rest.to_vec());
        self.if_val(c, rn, |e| e.read(&p), |e| e.read_switch(scrut, n, &rest))
    }

    fn place_n(&self, p: &Place) -> N {
        match p {
            Place::Mem { n, .. } | Place::Bits { n, .. } | Place::Elem { n, .. } => *n,
            Place::Local(s) => self.slots[s.0].n,
            Place::Switch { arms, .. } => arms.first().map(|(_, p)| self.place_n(p)).unwrap_or(N::U8),
        }
    }

    /// Write a place (the value already of the place's type).
    pub fn write(&mut self, p: &Place, v: V) {
        if !self.live {
            return;
        }
        match p {
            Place::Mem { off, n, bump } => {
                self.store(*off, *n, v);
                self.bump(*bump);
            }
            Place::Bits { off, reg, shift, width, bump, .. } => {
                let mask = if *width >= 128 { u128::MAX } else { (1u128 << width) - 1 };
                let fv = self.cast(v, self.place_n(p), *reg);
                let fv = if self.place_n(p) == N::Bool || self.place_n(p).width() == *width && !self.place_n(p).signed() {
                    // Already in range.
                    fv
                } else {
                    self.op(Op::And, fv, V::K(mask), *reg)
                };
                // The bits wait in a piece: the field gets them when it is
                // needed whole (`settle`).
                let key = (*off, *reg);
                // The field as cached (with its bits waiting), or loaded.
                let whole = match self.regs.get(&key) {
                    Some(Known::K(c)) => V::K(*c),
                    Some(Known::D(v)) => V::D(*v),
                    None => self.read(&Place::Mem { off: *off, n: *reg, bump: 0 }),
                };
                if !self.live {
                    return;
                }
                let lo = *shift;
                let hi = shift + width;
                let mut ps = self.pieces.remove(&key).unwrap_or_default();
                if ps.iter().any(|p| p.shift < hi && lo < p.shift + p.width && !(p.shift == lo && p.width == *width)) {
                    // Overlapping other bits: put those in first.
                    self.pieces.insert(key, ps);
                    self.settle_why("bits_write", key.0, key.1);
                    ps = Vec::new();
                } else {
                    ps.retain(|p| !(p.shift == lo && p.width == *width));
                }
                let base = match self.regs.get(&key) {
                    Some(Known::K(c)) => V::K(*c),
                    Some(Known::D(v)) => V::D(*v),
                    None => whole,
                };
                // As `store` does for the field: overlapping fields written
                // first, then forgotten; the field owed.
                self.flush_overlapping(key.0, key.1, Some(key));
                self.regs.retain(|(o, m), _| (*o, *m) == key || *o + m.width() as usize / 8 <= key.0 || key.0 + key.1.width() as usize / 8 <= *o);
                let k = match base {
                    V::K(c) => Known::K(c),
                    V::D(x) => Known::D(x),
                };
                self.regs.insert(key, k);
                if !self.dirty.contains(&key) {
                    self.dirty.push(key);
                }
                ps.push(Piece { shift: lo, width: *width, v: fv });
                self.pieces.insert(key, ps);
                self.bump(*bump);
            }
            Place::Local(s) => self.set(*s, v),
            Place::Switch { scrut, n, arms } => self.write_switch(*scrut, *n, arms, v),
            Place::Elem { base, stride, count, off, index, n, bump } => match index {
                V::K(k) if (*k as usize) < *count => self.write(&Place::Mem { off: base + (*k as usize) * stride + off, n: *n, bump: *bump }, v),
                V::K(_) => self.trap(),
                V::D(_) => {
                    let a = self.elem_addr(*base, *stride, *count, *off, *index, true);
                    let x = self.val(v, *n);
                    self.b.ins().store(MemFlags::trusted(), x, a, 0);
                    self.bump(*bump);
                }
            },
        }
    }

    fn write_switch(&mut self, scrut: V, n: N, arms: &[(Vec<Pat>, Place)], v: V) {
        let Some(((pats, p), rest)) = arms.split_first() else { return };
        if rest.is_empty() {
            return self.write(p, v);
        }
        let c = self.test(scrut, n, pats);
        let (p, rest) = (p.clone(), rest.to_vec());
        self.if_(c, |e| e.write(&p, v), |e| e.write_switch(scrut, n, &rest, v))
    }

    /// `n` bytes at `off` change behind the code's back (by calls it
    /// declares pure, too): forgotten at every call.
    pub fn set_volatile(&mut self, off: usize, n: usize) {
        if !self.volatile.contains(&(off, n)) {
            self.volatile.push((off, n));
        }
    }

    /// The machine's epochs (u64 fields), which a write to a place bumps
    /// by its flags (1, 2, 4: the first, second, third).
    pub fn set_epochs(&mut self, offs: [usize; 3]) {
        self.epochs = Some(offs);
    }

    /// A trial past its budget emits no more (the rest is dead code).
    fn trial_check(&mut self) {
        if let Some(l) = self.trial_limit {
            if self.live && self.emitted() > l {
                // Dead from here: the block ends, as at any exit.
                self.b.ins().trap(cranelift_codegen::ir::TrapCode::user(4).unwrap());
                self.live = false;
                self.trial_over = true;
            }
        }
    }

    /// Instructions emitted that may run (failed trials' are not).
    fn emitted(&self) -> usize {
        self.b.func.dfg.num_insts() - self.wasted
    }

    /// The machine's field at `off` holds `v` (a check the code made says
    /// so): remembered, nothing stored. Not when the code has written it
    /// since (its own value is the one to keep).
    pub fn assume(&mut self, off: usize, n: N, v: u128) {
        if !self.live {
            return;
        }
        let w = n.width() as usize / 8;
        if self.dirty.iter().any(|(o, m)| off < o + m.width() as usize / 8 && *o < off + w) {
            return;
        }
        self.regs.retain(|(o, m), _| *o + m.width() as usize / 8 <= off || off + w <= *o);
        self.regs.insert((off, n), Known::K(v & n.mask()));
    }

    /// `inline` when it emits at most `budget` host instructions (what is
    /// known folds it: a big fn may be small here), else `fallback`. The
    /// trial is a branch on a constant, set to the fallback when the trial
    /// is too big (it is then never run; a trial stops emitting past its
    /// budget).
    pub fn try_inline<T>(&mut self, budget: usize, inline: impl FnOnce(&mut Self) -> T, fallback: impl FnOnce(&mut Self) -> T) -> T {
        if !self.live {
            return fallback(self);
        }
        let t = self.b.create_block();
        let f = self.b.create_block();
        let one = self.b.ins().iconst(types::I8, 1);
        let one_inst = self.b.func.dfg.value_def(one).unwrap_inst();
        self.b.ins().brif(one, t, &[], f, &[]);
        self.b.seal_block(t);
        self.b.seal_block(f);
        let before = self.state();
        let (regions, bumped) = (self.regions.len(), self.bumped);
        self.b.switch_to_block(t);
        let n0 = self.emitted();
        // Nested in another trial: within what is left of its budget too.
        let (outer_limit, outer_over) = (self.trial_limit, self.trial_over);
        let limit = match outer_limit {
            Some(l) => l.min(n0 + budget),
            None => n0 + budget,
        };
        self.trial_limit = Some(limit);
        self.trial_over = false;
        let r = inline(self);
        let over = self.trial_over || self.emitted() > limit;
        if std::env::var_os("MACHINE_JIT_TRIALS").is_some() {
            eprintln!("machine-jit: trial of {} instructions (budget {budget}){}", self.emitted() - n0, if over { ", over" } else { "" });
        }
        self.trial_limit = outer_limit;
        self.trial_over = outer_over;
        if !over {
            // Kept: on to a block of its own; the other branch never runs.
            let cont = if self.live {
                let c = self.b.create_block();
                self.b.ins().jump(c, &[]);
                self.b.seal_block(c);
                Some(c)
            } else {
                None
            };
            self.b.switch_to_block(f);
            self.b.ins().trap(cranelift_codegen::ir::TrapCode::user(4).unwrap());
            match cont {
                Some(c) => {
                    self.b.switch_to_block(c);
                    self.live = true;
                }
                None => self.live = false,
            }
            return r;
        }
        // Too big: the branch goes to the fallback, and the trial is dead.
        if self.live {
            self.b.ins().trap(cranelift_codegen::ir::TrapCode::user(4).unwrap());
        }
        self.wasted += self.emitted() - n0;
        self.b.func.dfg.replace(one_inst).iconst(types::I8, 0);
        self.regions.truncate(regions);
        self.ret_exits.truncate(regions);
        self.restore(&before);
        self.bumped = bumped;
        self.live = true;
        self.b.switch_to_block(f);
        fallback(self)
    }

    /// The code that reaches here stops the host (the interpreter would
    /// panic: `unreachable`, an index out of bounds).
    pub fn trap(&mut self) {
        if !self.live {
            return;
        }
        self.flush();
        self.b.ins().trap(cranelift_codegen::ir::TrapCode::user(2).unwrap());
        self.live = false;
    }

    /// The address of an array element's field, after its bounds check;
    /// what the emitter remembers of the array is written first (and, for
    /// a store, forgotten).
    fn elem_addr(&mut self, base: usize, stride: usize, count: usize, off: usize, index: V, write: bool) -> Value {
        let len = stride * count;
        let hit: Vec<(usize, N)> = self.dirty.iter().copied().filter(|(o, _)| *o >= base && *o < base + len).collect();
        if !hit.is_empty() {
            self.flush();
        }
        if write {
            self.regs.retain(|(o, _), _| !(*o >= base && *o < base + len));
        }
        let i = self.val(index, N::U64);
        let ok = self.b.ins().icmp_imm(IntCC::UnsignedLessThan, i, count as i64);
        let in_b = self.b.create_block();
        let out_b = self.b.create_block();
        self.b.ins().brif(ok, in_b, &[], out_b, &[]);
        self.b.seal_block(in_b);
        self.b.seal_block(out_b);
        self.b.switch_to_block(out_b);
        self.b.ins().trap(cranelift_codegen::ir::TrapCode::user(3).unwrap());
        self.b.switch_to_block(in_b);
        let scaled = self.b.ins().imul_imm(i, stride as i64);
        let m = self.base();
        let a = self.b.ins().iadd(m, scaled);
        self.b.ins().iadd_imm(a, (base + off) as i64)
    }

    /// How many calls that may write memory have been emitted (code that
    /// can change code), and a write that is known to (`note_code_write`):
    /// a step with none since its start cannot have changed its run's code.
    pub fn code_writes(&self) -> usize {
        self.code_calls
    }

    pub fn note_code_write(&mut self) {
        self.code_calls += 1;
    }

    /// Leave the compiled run whatever `r` (a step's answer from a call:
    /// 1 made, else not made).
    pub fn exit_with(&mut self, r: V) {
        if !self.live {
            return;
        }
        match r {
            V::K(1) => self.exit(true),
            V::K(_) => self.exit(false),
            V::D(rv) => {
                self.flush();
                let made = self.b.ins().icmp_imm(IntCC::Equal, rv, 1);
                let made = self.b.ins().uextend(types::I32, made);
                let c = self.b.ins().iconst(types::I32, self.k as i64);
                let c = self.b.ins().iadd(c, made);
                let bv = self.b.use_var(self.made_base);
                let c = self.b.ins().iadd(c, bv);
                self.b.ins().jump(self.out, &[c.into()]);
                self.live = false;
            }
        }
    }

    /// Whether a write bumped an epoch since the last call.
    pub fn take_bumped(&mut self) -> bool {
        std::mem::take(&mut self.bumped)
    }

    fn bump(&mut self, flags: u8) {
        let Some(offs) = self.epochs else { return };
        if flags & 1 != 0 && self.live {
            self.bumped = true;
        }
        for (i, off) in offs.iter().enumerate() {
            if flags & (1 << i) != 0 {
                let p = Place::Mem { off: *off, n: N::U64, bump: 0 };
                let v = self.read(&p);
                let v = self.op(Op::Add, v, V::K(1), N::U64);
                self.store(*off, N::U64, v);
            }
        }
    }

    /// A choice of places by `scrut` (of type `n`): the place itself when
    /// `scrut` is known.
    pub fn place_switch(&mut self, scrut: V, n: N, arms: Vec<(Vec<Pat>, Place)>) -> Place {
        let mut rest = Vec::new();
        for (pats, p) in arms {
            match self.test(scrut, n, &pats) {
                V::K(0) => continue,
                V::K(_) if rest.is_empty() => return p,
                _ => rest.push((pats, p)),
            }
        }
        if rest.len() == 1 {
            return rest.pop().unwrap().1;
        }
        Place::Switch { scrut, n, arms: rest }
    }

    // -- regions --------------------------------------------------------------------

    /// A body that may `ret` from inside its branches (an inlined fn, an
    /// instruction's semantics): the code after `region_end` follows every
    /// `ret` and the body's end.
    /// The next region begun with `region_begin_ret` (a fn's body): a
    /// return of a known true value calls `call` and leaves the run (`made`
    /// as for `exit`) where it is, instead of joining the other returns. For
    /// a fn whose true result the caller would exit on (the paths that
    /// leave share no stores with the ones that go on).
    pub fn exit_on_true_return(&mut self, call: usize, made: bool) {
        self.next_ret_exit = Some((call, made));
    }

    /// A region whose returns are in `ret` (see `exit_on_true_return`).
    pub fn region_begin_ret(&mut self, ret: Slot) {
        let ex = self.next_ret_exit.take().map(|(c, m)| (ret, c, m));
        self.region_begin();
        if let Some(top) = self.ret_exits.last_mut() {
            *top = ex;
        }
    }

    pub fn region_begin(&mut self) {
        self.next_ret_exit = None;
        self.ret_exits.push(None);
        if !self.live {
            self.regions.push(None);
            return;
        }
        let before = self.state();
        let join = self.b.create_block();
        self.regions.push(Some((before, join, Vec::new())));
    }

    /// Return from the innermost region.
    pub fn ret(&mut self) {
        if !self.live {
            return;
        }
        if let Some(Some((slot, call, made))) = self.ret_exits.last().copied() {
            if matches!(self.get(slot), V::K(v) if v != 0) {
                let _ = self.call(call, &[], N::U64, false);
                self.exit(made);
                return;
            }
        }
        let Some(Some((before, join, _))) = self.regions.last() else { return };
        let (before, join) = (before.clone(), *join);
        let st = self.leave(&before, join);
        if let Some(Some((_, _, ends))) = self.regions.last_mut() {
            ends.push(st);
        }
    }

    pub fn region_end(&mut self) {
        self.ret();
        self.ret_exits.pop();
        match self.regions.pop() {
            Some(Some((before, join, ends))) => {
                self.enter(&before, join, &ends);
            }
            _ => self.live = false,
        }
    }

    /// Forget what is known of the machine's fields (after a call that may
    /// have changed them).
    pub fn forget(&mut self) {
        self.flush();
        self.regs.clear();
        self.pieces.clear();
    }

    /// Forget the fields calls change (`set_volatile`).
    fn forget_volatile(&mut self) {
        let vol = self.volatile.clone();
        self.regs.retain(|(o, m), _| !vol.iter().any(|(vo, vl)| *o < vo + vl && *vo < o + m.width() as usize / 8));
    }

    // -- locals ---------------------------------------------------------------------

    /// A new local of type `n`, holding `v`.
    pub fn local(&mut self, n: N, v: V) -> Slot {
        let var = self.b.declare_var(n.ir());
        // Past an exit or a trap nothing is emitted (the slot is never
        // read by code that runs).
        if self.live {
            let zero = self.konst(n, 0);
            self.b.def_var(var, zero);
        }
        self.slots.push(SlotState { var, n, k: Some(0) });
        let s = Slot(self.slots.len() - 1);
        self.set(s, v);
        s
    }

    pub fn get(&mut self, s: Slot) -> V {
        if !self.live {
            return V::K(0);
        }
        let st = self.slots[s.0];
        match st.k {
            Some(c) => V::K(c),
            None => V::D(self.b.use_var(st.var)),
        }
    }

    pub fn set(&mut self, s: Slot, v: V) {
        if !self.live {
            return;
        }
        let n = self.slots[s.0].n;
        match v {
            V::K(c) => self.slots[s.0].k = Some(c & n.mask()),
            V::D(x) => {
                self.b.def_var(self.slots[s.0].var, x);
                self.slots[s.0].k = None;
            }
        }
    }

    fn state(&self) -> State {
        State { slots: self.slots.iter().map(|s| s.k).collect(), regs: self.regs.clone(), dirty: self.dirty.clone(), pieces: self.pieces.clone() }
    }

    // -- control --------------------------------------------------------------------

    /// `if c { t } else { f }`: one of them when `c` is known.
    pub fn if_(&mut self, c: V, t: impl FnOnce(&mut Self), f: impl FnOnce(&mut Self)) {
        if !self.live {
            return;
        }
        match c {
            V::K(x) => {
                if x != 0 {
                    t(self)
                } else {
                    f(self)
                }
            }
            V::D(cv) => {
                // No flush: each branch keeps the stores it owes, and the
                // join decides (`enter`).
                let tb = self.b.create_block();
                let fb = self.b.create_block();
                self.b.ins().brif(cv, tb, &[], fb, &[]);
                self.b.seal_block(tb);
                self.b.seal_block(fb);
                let before = self.state();
                let join = self.b.create_block();
                self.b.switch_to_block(tb);
                self.learn(c, true);
                t(self);
                let a = self.leave(&before, join);
                self.restore(&before);
                self.live = true;
                self.b.switch_to_block(fb);
                self.learn(c, false);
                f(self);
                let b = self.leave(&before, join);
                self.enter(&before, join, &[a, b]);
            }
        }
    }

    /// `if c { t } else { f }` as a value of type `n`.
    pub fn if_val(&mut self, c: V, n: N, t: impl FnOnce(&mut Self) -> V, f: impl FnOnce(&mut Self) -> V) -> V {
        if !self.live {
            return V::K(0);
        }
        if let V::K(x) = c {
            return if x != 0 { t(self) } else { f(self) };
        }
        let r = self.local(n, V::K(0));
        self.if_(
            c,
            |e| {
                let v = t(e);
                e.set(r, v)
            },
            |e| {
                let v = f(e);
                e.set(r, v)
            },
        );
        self.get(r)
    }

    /// A branch's end: what it leaves known, its known locals (those from
    /// before the branch) put in their variables (the other branch may
    /// leave them otherwise), and a jump to a block of its own (where the
    /// join, knowing whether another branch comes back, stores what it owes
    /// or not). None when it left the code.
    fn leave(&mut self, before: &State, _join: Block) -> Option<(State, Block)> {
        if !self.live {
            return None;
        }
        let n = before.slots.len().min(self.slots.len());
        for i in 0..n {
            let s = self.slots[i];
            if let Some(c) = s.k {
                let x = self.konst(s.n, c);
                self.b.def_var(s.var, x);
            }
        }
        let st = self.state();
        let pad = self.b.create_block();
        self.b.ins().jump(pad, &[]);
        self.b.seal_block(pad);
        self.live = false;
        Some((st, pad))
    }

    /// After the branches. One that comes back: it goes on as it was, its
    /// deferred stores still owed (so a branch that raised or exited costs
    /// the other nothing). Several: each stores what it owes, and they meet
    /// at `join`, which keeps what all of them left known.
    fn enter(&mut self, before: &State, join: Block, ends: &[Option<(State, Block)>]) {
        // The pads' stores and values are the branches' to finish: not a
        // trial's to stop (it checks again past the join).
        let limit = self.trial_limit.take();
        self.enter_pads(before, join, ends);
        self.trial_limit = limit;
    }

    fn enter_pads(&mut self, before: &State, join: Block, ends: &[Option<(State, Block)>]) {
        let ends: Vec<&(State, Block)> = ends.iter().flatten().collect();
        self.slots.truncate(before.slots.len());
        match ends.as_slice() {
            [] => {
                self.b.seal_block(join);
                self.live = false;
                return;
            }
            [(st, pad)] => {
                self.b.seal_block(join);
                self.b.switch_to_block(*pad);
                self.restore(st);
                self.slots.truncate(before.slots.len());
                self.live = true;
                return;
            }
            _ => {}
        }
        // A store some branch owes, where every branch has the same value
        // for the field (owed, or stored already: a branch that called out
        // stored it first), stays owed past the join; each branch stores
        // only the rest. Storing it again later, on a path that had stored
        // it, is harmless.
        let mut common: Vec<(usize, N)> = Vec::new();
        for (st, _) in &ends {
            for key in &st.dirty {
                if common.contains(key) {
                    continue;
                }
                let v = st.regs.get(key);
                let ps = st.pieces.get(key);
                if v.is_some() && ends.iter().all(|(e, _)| e.regs.get(key) == v && e.pieces.get(key) == ps) {
                    common.push(*key);
                }
            }
        }
        // The other owed fields go on owed, their values passed to the join:
        // a branch that does not know one (it called out: the machine holds
        // it) loads it. Fields owed in shapes that overlap are stored
        // instead.
        let overlap = |a: &(usize, N), b: &(usize, N)| a.0 < b.0 + b.1.width() as usize / 8 && b.0 < a.0 + a.1.width() as usize / 8;
        let mut owed: Vec<(usize, N)> = Vec::new();
        for (st, _) in &ends {
            for key in &st.dirty {
                if !owed.contains(key) {
                    owed.push(*key);
                }
            }
        }
        let carried: Vec<(usize, N)> = owed
            .iter()
            .copied()
            .filter(|k| !common.contains(k) && !owed.iter().any(|o| o != k && overlap(o, k)))
            .collect();
        // A carried field's bits written in a branch go on as bits (one
        // value each): the branches that wrote the same bits pass them as
        // they are, the others the bits of the field as they have it.
        // Different bits in different branches: the field, settled.
        let shapes: Vec<Vec<(u32, u32)>> = carried
            .iter()
            .map(|key| {
                let mut shape: Option<Vec<(u32, u32)>> = None;
                for (st, _) in &ends {
                    if let Some(ps) = st.pieces.get(key) {
                        let mut sh: Vec<(u32, u32)> = ps.iter().map(|p| (p.shift, p.width)).collect();
                        sh.sort();
                        match &shape {
                            None => shape = Some(sh),
                            Some(s0) if *s0 == sh => {}
                            Some(_) => return Vec::new(),
                        }
                    }
                }
                shape.unwrap_or_default()
            })
            .collect();
        let params: Vec<(Value, Vec<Value>)> = carried
            .iter()
            .zip(&shapes)
            .map(|((_, n), sh)| {
                let b = self.b.append_block_param(join, n.ir());
                let ps = sh.iter().map(|_| self.b.append_block_param(join, n.ir())).collect();
                (b, ps)
            })
            .collect();
        for (st, pad) in &ends {
            self.b.switch_to_block(*pad);
            self.restore(st);
            self.live = true;
            let args = self.carry_args(&carried, &shapes);
            self.dirty.retain(|d| !common.contains(d) && !carried.contains(d));
            self.flush();
            self.b.ins().jump(join, &args);
        }
        self.b.seal_block(join);
        self.slots.truncate(before.slots.len());
        self.b.switch_to_block(join);
        self.live = true;
        let ends: Vec<&State> = ends.iter().map(|(st, _)| st).collect();
        for i in 0..self.slots.len() {
            let first = ends[0].slots[i];
            self.slots[i].k = if ends.iter().all(|e| e.slots[i] == first) { first } else { None };
        }
        let mut regs = HashMap::new();
        for (key, v) in &ends[0].regs {
            // A field with bits written in a branch is what that branch
            // stored (unless the bits are the same in every branch: below).
            if ends.iter().any(|e| e.pieces.contains_key(key)) {
                continue;
            }
            if ends.iter().all(|e| e.regs.get(key) == Some(v)) {
                // A host value made in one branch does not reach the join.
                let ok = match v {
                    Known::K(_) => true,
                    Known::D(_) => before.regs.get(key) == Some(v),
                };
                if ok {
                    regs.insert(*key, *v);
                }
            }
        }
        // The common stores' values (the same in every branch) are known.
        for key in &common {
            if let Some(v) = ends[0].regs.get(key) {
                regs.insert(*key, *v);
            }
        }
        let mut pieces = HashMap::new();
        for key in &common {
            if let Some(ps) = ends[0].pieces.get(key) {
                pieces.insert(*key, ps.clone());
            }
        }
        // The carried fields: their values as passed, still owed.
        let mut dirty = common;
        for ((key, (p, bits)), sh) in carried.iter().zip(params).zip(&shapes) {
            regs.retain(|k, _| k == key || !overlap(k, key));
            regs.insert(*key, Known::D(p));
            dirty.push(*key);
            if !sh.is_empty() {
                let ps = sh.iter().zip(bits).map(|(&(shift, width), v)| Piece { shift, width, v: V::D(v) }).collect();
                pieces.insert(*key, ps);
            }
        }
        self.regs = regs;
        self.dirty = dirty;
        self.pieces = pieces;
    }

    /// The values of `carried` fields as this path has them, for a block
    /// that takes them (and each field's bits in `shapes`) as parameters:
    /// the bits as written where the path wrote the same, else taken out
    /// of the field; a field it does not know, loaded.
    fn carry_args(&mut self, carried: &[(usize, N)], shapes: &[Vec<(u32, u32)>]) -> Vec<cranelift_codegen::ir::BlockArg> {
        let mut args = Vec::with_capacity(carried.len());
        for (&(off, n), sh) in carried.iter().zip(shapes) {
            let mine: Option<Vec<Piece>> = self.pieces.get(&(off, n)).cloned().and_then(|mut ps| {
                ps.sort_by_key(|p| p.shift);
                (ps.iter().map(|p| (p.shift, p.width)).collect::<Vec<_>>() == *sh).then_some(ps)
            });
            let bits: Vec<V> = match &mine {
                Some(ps) if !sh.is_empty() => {
                    self.pieces.remove(&(off, n));
                    ps.iter().map(|p| p.v).collect()
                }
                _ => {
                    self.settle_why("join", off, n);
                    Vec::new()
                }
            };
            let x = match self.regs.get(&(off, n)).copied() {
                Some(Known::K(c)) => self.konst(n, c),
                Some(Known::D(v)) => v,
                None => {
                    // The machine holds it (no other shape of it owed).
                    self.flush_overlapping(off, n, Some((off, n)));
                    let m = self.base();
                    self.b.ins().load(n.ir(), MemFlags::trusted(), m, off as i32)
                }
            };
            args.push(cranelift_codegen::ir::BlockArg::Value(x));
            if bits.len() == sh.len() {
                for (b, nb) in bits.into_iter().zip(sh) {
                    let _ = nb;
                    let v = self.val(b, n);
                    args.push(cranelift_codegen::ir::BlockArg::Value(v));
                }
            } else {
                // The field whole: its bits taken out of it.
                for &(shift, width) in sh {
                    let m = if width >= 128 { u128::MAX } else { (1u128 << width) - 1 };
                    let t = self.op(Op::Shr, V::D(x), V::K(shift as u128), n);
                    let t = self.op(Op::And, t, V::K(m), n);
                    let v = self.val(t, n);
                    args.push(cranelift_codegen::ir::BlockArg::Value(v));
                }
            }
        }
        args
    }

    fn restore(&mut self, st: &State) {
        // Locals made since `st` are gone; ones it had that are gone now
        // (released at a join) stay gone.
        self.slots.truncate(st.slots.len());
        for (i, k) in st.slots.iter().enumerate().take(self.slots.len()) {
            self.slots[i].k = *k;
        }
        self.regs = st.regs.clone();
        self.dirty = st.dirty.clone();
        self.pieces = st.pieces.clone();
    }

    /// Where the locals stand: locals made after it are released by
    /// `release` (they are out of scope).
    pub fn mark(&self) -> usize {
        self.slots.len()
    }

    pub fn release(&mut self, mark: usize) {
        self.slots.truncate(mark);
    }

    /// A rare path: when `c` holds, `f`, which may only make calls that
    /// leave the processor's fields alone (the board's, declared volatile,
    /// are forgotten after) and leave the run. The common path keeps what
    /// it knows and the stores it has deferred.
    pub fn cold(&mut self, c: V, f: impl FnOnce(&mut Self)) {
        if !self.live {
            return;
        }
        match c {
            V::K(0) => {}
            V::K(_) => {
                f(self);
                self.forget_volatile();
            }
            V::D(_) => {
                // A branch like any other: what the rare path's calls
                // change it loads for the join (`enter`), so the common
                // path keeps it, owed, where it is.
                self.if_(c, |e| {
                    let blk = e.b.current_block().expect("in a block");
                    e.b.set_cold_block(blk);
                    f(e)
                }, |_| {});
            }
        }
    }

    /// Leave the compiled run when `c` holds, after `then` (on that path
    /// only); this step counted as made or not.
    pub fn exit_if(&mut self, c: V, made: bool, then: impl FnOnce(&mut Self)) {
        if !self.live {
            return;
        }
        match c {
            V::K(0) => {}
            V::K(_) => {
                then(self);
                self.exit(made);
            }
            V::D(cv) => {
                let eb = self.b.create_block();
                let cb = self.b.create_block();
                self.b.ins().brif(cv, eb, &[], cb, &[]);
                self.b.seal_block(eb);
                self.b.seal_block(cb);
                let before = self.state();
                self.b.switch_to_block(eb);
                self.b.set_cold_block(eb);
                self.learn(c, true);
                then(self);
                self.exit(made);
                self.restore(&before);
                self.b.switch_to_block(cb);
                self.live = true;
                self.learn(c, false);
            }
        }
    }

    /// The way in to the first step of a run built with `plan`: the
    /// fields it takes, loaded.
    pub(crate) fn entry_pad(&mut self, pad: Block, first: Block, plan: LoopPlan) {
        self.loop_params = plan
            .carried
            .iter()
            .zip(&plan.shapes)
            .map(|((_, n), sh)| {
                let b = self.b.append_block_param(first, n.ir());
                let ps = sh.iter().map(|_| self.b.append_block_param(first, n.ir())).collect();
                (b, ps)
            })
            .collect();
        self.b.switch_to_block(pad);
        self.b.seal_block(pad);
        self.live = true;
        self.regs.clear();
        self.pieces.clear();
        self.dirty.clear();
        let args = self.carry_args(&plan.carried, &plan.shapes);
        self.b.ins().jump(first, &args);
        self.live = false;
        self.plan = Some(plan);
    }

    /// What is known here, as a plan for going round: the fields held as
    /// host values (known constants are stored, and known again past the
    /// first step's checks), but not two shapes of the same bytes.
    fn loop_seen(&self) -> LoopPlan {
        let overlap = |a: &(usize, N), b: &(usize, N)| a.0 < b.0 + b.1.width() as usize / 8 && b.0 < a.0 + a.1.width() as usize / 8;
        let held: Vec<(usize, N)> = self.regs.iter().filter(|(_, v)| matches!(v, Known::D(_))).map(|(k, _)| *k).collect();
        let mut carried: Vec<(usize, N)> = held.iter().copied().filter(|k| !self.regs.keys().any(|o| o != k && overlap(o, k))).collect();
        carried.sort_by_key(|(o, n)| (*o, n.width(), n.signed()));
        let shapes = carried
            .iter()
            .map(|k| {
                let mut sh: Vec<(u32, u32)> = self.pieces.get(k).map(|ps| ps.iter().map(|p| (p.shift, p.width)).collect()).unwrap_or_default();
                sh.sort();
                sh
            })
            .collect();
        let owed = carried.iter().copied().filter(|k| self.dirty.contains(k)).collect();
        LoopPlan { carried, shapes, owed }
    }

    /// What the run knew going round (a first build).
    pub(crate) fn take_seen(&mut self) -> Option<LoopPlan> {
        self.seen.take()
    }

    /// The run's length.
    pub fn steps(&self) -> usize {
        self.steps
    }

    /// The steps made, this step's included (a u32).
    pub fn made_through(&mut self) -> V {
        if !self.live {
            return V::K(0);
        }
        let bv = self.b.use_var(self.made_base);
        V::D(self.b.ins().iadd_imm(bv, self.k as i64 + 1))
    }

    /// When `c` holds, `then`, and the run again from its first step (the
    /// machine's fields stored, as on entering it); else on. For a run whose
    /// end leads back to its start, where the caller would enter it again.
    pub fn loop_back(&mut self, c: V, then: impl FnOnce(&mut Self)) {
        if !self.live {
            return;
        }
        let Some(first) = self.first else { return };
        let cv = match c {
            V::K(0) => return,
            V::K(_) => self.konst(N::Bool, 1),
            V::D(v) => v,
        };
        let lb = self.b.create_block();
        let cont = self.b.create_block();
        self.b.ins().brif(cv, lb, &[], cont, &[]);
        self.b.seal_block(lb);
        self.b.seal_block(cont);
        let before = self.state();
        self.b.switch_to_block(lb);
        then(self);
        let limit = self.trial_limit.take();
        let args = match self.plan.clone() {
            // Built to go round: the first step takes what is known.
            Some(plan) => {
                let args = self.carry_args(&plan.carried, &plan.shapes);
                self.dirty.retain(|d| !plan.carried.contains(d));
                args
            }
            None => {
                if self.seen.is_none() {
                    self.seen = Some(self.loop_seen());
                }
                Vec::new()
            }
        };
        self.flush();
        self.trial_limit = limit;
        let bv = self.b.use_var(self.made_base);
        let nb = self.b.ins().iadd_imm(bv, self.steps as i64);
        self.b.def_var(self.made_base, nb);
        self.b.ins().jump(first, &args);
        self.restore(&before);
        self.b.switch_to_block(cont);
        self.live = true;
    }

    /// Leave the compiled run here.
    pub fn exit(&mut self, made: bool) {
        if !self.live {
            return;
        }
        self.flush();
        let c = self.b.ins().iconst(types::I32, self.k as i64 + made as i64);
        let bv = self.b.use_var(self.made_base);
                let c = self.b.ins().iadd(c, bv);
        self.b.ins().jump(self.out, &[c.into()]);
        self.live = false;
    }

    /// Leave when a step's answer `r` (a u32 from a call: 0 go on, 1 made
    /// and stop, 2 not made) says so.
    pub fn exit_on(&mut self, r: V) {
        if !self.live {
            return;
        }
        match r {
            V::K(0) => {}
            V::K(1) => self.exit(true),
            V::K(_) => self.exit(false),
            V::D(rv) => {
                let eb = self.b.create_block();
                let cb = self.b.create_block();
                self.b.ins().brif(rv, eb, &[], cb, &[]);
                self.b.seal_block(eb);
                self.b.seal_block(cb);
                self.b.switch_to_block(eb);
                self.b.set_cold_block(eb);
                // (What the flush settles is this path's: the other goes on
                // as it was.)
                let before = self.state();
                self.flush();
                self.restore(&before);
                let made = self.b.ins().icmp_imm(IntCC::Equal, rv, 1);
                let made = self.b.ins().uextend(types::I32, made);
                let c = self.b.ins().iconst(types::I32, self.k as i64);
                let c = self.b.ins().iadd(c, made);
                let bv = self.b.use_var(self.made_base);
                let c = self.b.ins().iadd(c, bv);
                self.b.ins().jump(self.out, &[c.into()]);
                self.b.switch_to_block(cb);
            }
        }
    }

    /// Call `f`, an `extern "C" fn(machine, u64...) -> u64`, with `args`
    /// (each as its bits); its answer as `ret`. What is known of the
    /// machine is forgotten unless `pure` (it changes no field the code
    /// reads).
    pub fn call(&mut self, f: usize, args: &[(V, N)], ret: N, pure: bool) -> V {
        self.call_with(f, args, ret, pure, None)
    }

    /// A call that may change the machine (not pure) but leaves the bytes
    /// in `kept` alone (neither reads nor writes them) and only reads those
    /// in `read`: what the code owes in `kept` stays owed (in host values),
    /// what it knows of `kept` and `read` stays known. (A fault the callee
    /// raises is not delivered by the call: what reads all the machine is
    /// called after the code stores what it owes.)
    pub fn call_fx(&mut self, f: usize, args: &[(V, N)], ret: N, kept: &[(usize, usize)], read: &[(usize, usize)]) -> V {
        self.call_with(f, args, ret, false, Some(Fx::Keep(kept, read)))
    }

    /// A call that reads and writes only the bytes in `touched` (and the
    /// volatile fields): those are stored first and forgotten after; the
    /// rest stays owed and known (a bus access's shim: the access count,
    /// the cycles, the bus).
    pub fn call_touching(&mut self, f: usize, args: &[(V, N)], ret: N, touched: &[(usize, usize)]) -> V {
        self.call_with(f, args, ret, false, Some(Fx::Touch(touched)))
    }

    /// `call_touching`, for a call that writes no memory (a bus read):
    /// the run's code is not checked after it.
    pub fn call_reading(&mut self, f: usize, args: &[(V, N)], ret: N, touched: &[(usize, usize)]) -> V {
        self.call_with(f, args, ret, true, Some(Fx::Touch(touched)))
    }

    fn call_with(&mut self, f: usize, args: &[(V, N)], ret: N, pure: bool, fx: Option<Fx>) -> V {
        if !self.live {
            return V::K(0);
        }
        let sig = match self.sigs.get(&(args.len(), true)) {
            Some(s) => *s,
            None => {
                let mut s = Signature::new(self.cc);
                s.params.push(AbiParam::new(self.ptr));
                for _ in args {
                    s.params.push(AbiParam::new(types::I64));
                }
                s.returns.push(AbiParam::new(types::I64));
                let r = self.b.import_signature(s);
                self.sigs.insert((args.len(), true), r);
                r
            }
        };
        let mut a = vec![self.base()];
        for (v, n) in args {
            let x = self.val(*v, *n);
            let x = match n.width() {
                64 => x,
                128 => self.b.ins().ireduce(types::I64, x),
                _ => self.b.ins().uextend(types::I64, x),
            };
            a.push(x);
        }
        // The machine holds what the code wrote before the call sees it
        // (all of it, but what the call leaves alone).
        let within = |set: &[(usize, usize)], (o, n): (usize, N)| set.iter().any(|(so, sl)| o >= *so && o + n.width() as usize / 8 <= so + sl);
        let meets = |set: &[(usize, usize)], (o, n): (usize, N)| set.iter().any(|(so, sl)| o < so + sl && *so < o + n.width() as usize / 8);
        let vol = self.volatile.clone();
        match fx {
            None => self.flush(),
            Some(Fx::Touch(touched)) => {
                let limit = self.trial_limit.take();
                let hit: Vec<(usize, N)> = self.dirty.iter().copied().filter(|k| meets(touched, *k) || meets(&vol, *k)).collect();
                for (o, m) in hit {
                    self.flush_overlapping(o, m, None);
                }
                self.trial_limit = limit;
            }
            Some(Fx::Keep(kept, _)) => {
                let limit = self.trial_limit.take();
                let hit: Vec<(usize, N)> = self.dirty.iter().copied().filter(|k| !within(kept, *k)).collect();
                for (o, m) in hit {
                    self.flush_overlapping(o, m, None);
                    // (flush_overlapping stores the fields over these bytes.)
                }
                self.trial_limit = limit;
            }
        }
        if !pure {
            self.code_calls += 1;
        }
        let fp = self.b.ins().iconst(self.ptr, f as i64);
        let call = self.b.ins().call_indirect(sig, fp, &a);
        let r = self.b.inst_results(call)[0];
        match fx {
            Some(Fx::Touch(touched)) => {
                self.regs.retain(|k, _| !meets(touched, *k));
                self.forget_volatile();
            }
            _ if pure => self.forget_volatile(),
            None => {
                self.regs.clear();
                self.pieces.clear();
            }
            Some(Fx::Keep(kept, read)) => {
                self.regs.retain(|k, _| within(kept, *k) || within(read, *k));
                self.pieces.retain(|k, _| within(kept, *k));
                self.forget_volatile();
            }
        }
        let r = match ret.width() {
            64 => r,
            128 => self.b.ins().uextend(types::I128, r),
            _ => self.b.ins().ireduce(ret.ir(), r),
        };
        V::D(r)
    }

    // -- values ---------------------------------------------------------------------

    /// `a op b`, both of type `n` (a shift's amount of any type); a
    /// comparison's result is a bool.
    pub fn op(&mut self, op: Op, a: V, b: V, n: N) -> V {
        let v = self.op_inner(op, a, b, n);
        // (A new value only: an identity hands back an operand.)
        if let (Op::And | Op::Or, N::Bool, V::D(r)) = (op, n, v) {
            if V::D(r) != a && V::D(r) != b {
                self.combos.insert(r, (op, a, b));
            }
        }
        if let (Op::Eq | Op::Ne, V::D(r)) = (op, v) {
            let (x, k) = match (a, b) {
                (V::D(x), V::K(k)) | (V::K(k), V::D(x)) => (Some(x), k),
                _ => (None, 0),
            };
            if let Some(x) = x {
                if let Some(key) = self.regs.iter().find(|(_, v)| **v == Known::D(x)).map(|(k, _)| *k) {
                    self.facts.insert(r, (key, x, k & key.1.mask(), op == Op::Eq));
                }
            }
        }
        v
    }

    /// The field at `off` holds at most `bits` bits (a register declared
    /// narrower than its native type): a condition that it is not one
    /// value says it is the other, for one bit.
    pub fn declare_bits(&mut self, off: usize, n: N, bits: u32) {
        self.bits.insert((off, n), bits);
    }

    /// On the side of a branch where `c` is `holds`: what it says of a
    /// field, known (when the field still holds the value compared).
    fn learn(&mut self, c: V, holds: bool) {
        let V::D(cv) = c else { return };
        // Both true where `and` holds; both false where `or` does not.
        if let Some((op, a, b)) = self.combos.get(&cv).copied() {
            if (op == Op::And) == holds {
                self.learn(a, holds);
                self.learn(b, holds);
            }
            return;
        }
        let Some((key, x, k, eq)) = self.facts.get(&cv).copied() else {
            // A bool field as the condition itself.
            if let Some(key) = self.regs.iter().find(|(k, v)| k.1 == N::Bool && **v == Known::D(cv)).map(|(k, _)| *k) {
                if !self.pieces.contains_key(&key) {
                    self.regs.insert(key, Known::K(holds as u128));
                }
            }
            return;
        };
        if self.regs.get(&key) != Some(&Known::D(x)) || self.pieces.contains_key(&key) {
            return;
        }
        if eq == holds {
            self.regs.insert(key, Known::K(k));
        } else if self.bits.get(&key) == Some(&1) && k <= 1 {
            self.regs.insert(key, Known::K(1 - k));
        }
    }

    fn op_inner(&mut self, op: Op, a: V, b: V, n: N) -> V {
        self.trial_check();
        if !self.live {
            return V::K(0);
        }
        if let (V::K(x), V::K(y)) = (a, b) {
            // A known zero divisor: the code that reaches it traps.
            if matches!(op, Op::Div | Op::Rem) && y & n.mask() == 0 {
                self.trap();
                return V::K(0);
            }
            return V::K(fold(op, x, y, n));
        }
        // Identities that keep a value known.
        match (op, a, b) {
            (Op::And, V::K(0), _) | (Op::And, _, V::K(0)) => return V::K(0),
            (Op::And, x, V::K(m)) | (Op::And, V::K(m), x) if m & n.mask() == n.mask() => return x,
            (Op::Or | Op::Xor | Op::Add | Op::Sub, x, V::K(0)) => return x,
            (Op::Or | Op::Xor | Op::Add, V::K(0), x) => return x,
            (Op::Shl | Op::Shr, x, V::K(0)) => return x,
            _ => {}
        }
        // One of two constants with a constant (or two, chosen alike): one of
        // two results.
        if !matches!(op, Op::Div | Op::Rem) {
            let arms = match (self.sel_of(a), self.sel_of(b), a, b) {
                (Some((c, x0, x1)), None, _, V::K(k)) => Some((c, fold(op, x0, k, n), fold(op, x1, k, n))),
                (None, Some((c, y0, y1)), V::K(k), _) => Some((c, fold(op, k, y0, n), fold(op, k, y1, n))),
                (Some((c, x0, x1)), Some((c2, y0, y1)), _, _) if c == c2 => Some((c, fold(op, x0, y0, n), fold(op, x1, y1, n))),
                _ => None,
            };
            if let Some((c, r0, r1)) = arms {
                let rn = if matches!(op, Op::Eq | Op::Ne | Op::Lt | Op::Le | Op::Gt | Op::Ge) { N::Bool } else { n };
                return self.select(V::D(c), V::K(r0), V::K(r1), rn);
            }
        }
        let w = n.width() as u128;
        let x = self.val(a, n);
        let v = match op {
            Op::Add => match b {
                V::K(c) => self.add_k(x, c, n),
                V::D(y) => self.b.ins().iadd(x, y),
            },
            Op::Sub => {
                let y = self.val(b, n);
                self.b.ins().isub(x, y)
            }
            Op::Mul => {
                let y = self.val(b, n);
                self.b.ins().imul(x, y)
            }
            // The host has no 128-bit division: Rust's, through a call.
            Op::Div | Op::Rem if n.width() == 128 => return self.wide_div(op, x, b, n),
            Op::Div | Op::Rem => {
                let y = self.val(b, n);
                match (op, n.signed()) {
                    (Op::Div, false) => self.b.ins().udiv(x, y),
                    (Op::Div, true) => self.b.ins().sdiv(x, y),
                    (_, false) => self.b.ins().urem(x, y),
                    (_, true) => self.b.ins().srem(x, y),
                }
            }
            Op::And => {
                let y = self.val(b, n);
                self.b.ins().band(x, y)
            }
            Op::Or => {
                let y = self.val(b, n);
                self.b.ins().bor(x, y)
            }
            Op::Xor => {
                let y = self.val(b, n);
                self.b.ins().bxor(x, y)
            }
            Op::Shl | Op::Shr => {
                // The amount: as Rust's `as u32`, then past the width 0
                // (or the sign).
                match b {
                    V::K(c) => {
                        if c >= w {
                            if op == Op::Shr && n.signed() {
                                let t = self.b.ins().sshr_imm(x, (w - 1) as i64);
                                return V::D(t);
                            }
                            return V::K(0);
                        }
                        match op {
                            Op::Shl => self.b.ins().ishl_imm(x, c as i64),
                            _ if n.signed() => self.b.ins().sshr_imm(x, c as i64),
                            _ => self.b.ins().ushr_imm(x, c as i64),
                        }
                    }
                    V::D(_) => return V::D(self.shift_dyn(op, x, b, n)),
                }
            }
            Op::Eq | Op::Ne | Op::Lt | Op::Le | Op::Gt | Op::Ge => {
                let y = self.val(b, n);
                let cc = match (op, n.signed()) {
                    (Op::Eq, _) => IntCC::Equal,
                    (Op::Ne, _) => IntCC::NotEqual,
                    (Op::Lt, false) => IntCC::UnsignedLessThan,
                    (Op::Le, false) => IntCC::UnsignedLessThanOrEqual,
                    (Op::Gt, false) => IntCC::UnsignedGreaterThan,
                    (Op::Ge, false) => IntCC::UnsignedGreaterThanOrEqual,
                    (Op::Lt, true) => IntCC::SignedLessThan,
                    (Op::Le, true) => IntCC::SignedLessThanOrEqual,
                    (Op::Gt, true) => IntCC::SignedGreaterThan,
                    _ => IntCC::SignedGreaterThanOrEqual,
                };
                self.b.ins().icmp(cc, x, y)
            }
        };
        V::D(v)
    }

    /// A 128-bit division or remainder by [`wide_div`], a half a call.
    fn wide_div(&mut self, op: Op, x: Value, b: V, n: N) -> V {
        let y = self.val(b, n);
        let (alo, ahi) = (self.b.ins().ireduce(types::I64, x), {
            let h = self.b.ins().ushr_imm(x, 64);
            self.b.ins().ireduce(types::I64, h)
        });
        let (blo, bhi) = (self.b.ins().ireduce(types::I64, y), {
            let h = self.b.ins().ushr_imm(y, 64);
            self.b.ins().ireduce(types::I64, h)
        });
        let code = (op == Op::Rem) as u128 | ((n.signed() as u128) << 1);
        let args = |hi: bool| -> [(V, N); 5] {
            [(V::K(code | ((hi as u128) << 2)), N::U64), (V::D(alo), N::U64), (V::D(ahi), N::U64), (V::D(blo), N::U64), (V::D(bhi), N::U64)]
        };
        let lo = self.call(wide_div as *const () as usize, &args(false), N::U64, true);
        let hi = self.call(wide_div as *const () as usize, &args(true), N::U64, true);
        let (V::D(lo), V::D(hi)) = (lo, hi) else { return V::K(0) };
        V::D(self.b.ins().iconcat(lo, hi))
    }

    fn add_k(&mut self, x: Value, c: u128, n: N) -> Value {
        if n.width() == 128 {
            let y = self.konst(n, c);
            self.b.ins().iadd(x, y)
        } else {
            self.b.ins().iadd_imm(x, (c & n.mask()) as u64 as i64)
        }
    }

    /// A shift by a computed amount: the amount as a u32 (Rust's `as`),
    /// past the width giving 0 (or the sign, shifting a signed value right).
    fn shift_dyn(&mut self, op: Op, x: Value, b: V, n: N) -> Value {
        let V::D(amt) = b else { unreachable!() };
        let at = self.b.func.dfg.value_type(amt);
        // The amount is a u32 of its bits: wider amounts past 2^32 wrap
        // as Rust's `as u32` does.
        let amt32 = if at.bits() > 32 {
            self.b.ins().ireduce(types::I32, amt)
        } else if at.bits() < 32 {
            self.b.ins().uextend(types::I32, amt)
        } else {
            amt
        };
        let w = self.b.ins().iconst(types::I32, n.width() as i64);
        let big = self.b.ins().icmp(IntCC::UnsignedGreaterThanOrEqual, amt32, w);
        let s = match op {
            Op::Shl => self.b.ins().ishl(x, amt32),
            _ if n.signed() => self.b.ins().sshr(x, amt32),
            _ => self.b.ins().ushr(x, amt32),
        };
        let past = if op == Op::Shr && n.signed() {
            self.b.ins().sshr_imm(x, (n.width() - 1) as i64)
        } else {
            self.konst(n, 0)
        };
        self.b.ins().select(big, past, s)
    }

    /// `!a` (bits), or `!a` of a bool.
    pub fn not(&mut self, a: V, n: N) -> V {
        if !self.live {
            return V::K(0);
        }
        match (a, n) {
            (V::K(x), N::Bool) => V::K((x == 0) as u128),
            (V::K(x), _) => V::K(!x & n.mask()),
            (V::D(x), N::Bool) => {
                let r = self.b.ins().icmp_imm(IntCC::Equal, x, 0);
                if let Some((key, v, k, eq)) = self.facts.get(&x).copied() {
                    self.facts.insert(r, (key, v, k, !eq));
                }
                V::D(r)
            }
            (V::D(x), _) => V::D(self.b.ins().bnot(x)),
        }
    }

    /// `a.wrapping_neg()`.
    pub fn neg(&mut self, a: V, n: N) -> V {
        if !self.live {
            return V::K(0);
        }
        match a {
            V::K(x) => V::K(x.wrapping_neg() & n.mask()),
            V::D(x) => V::D(self.b.ins().ineg(x)),
        }
    }

    /// `a as to` (Rust's `as` between native integers; a bool is 0 or 1).
    pub fn cast(&mut self, a: V, from: N, to: N) -> V {
        if !self.live {
            return V::K(0);
        }
        if let Some((c, x, y)) = self.sel_of(a) {
            let f = |v: u128| (from.widen(v & from.mask()) as u128) & to.mask();
            return self.select(V::D(c), V::K(f(x)), V::K(f(y)), to);
        }
        match a {
            V::K(x) => V::K((from.widen(x & from.mask()) as u128) & to.mask()),
            V::D(x) => {
                let (fw, tw) = (from.width(), to.width());
                V::D(if fw == tw {
                    x
                } else if fw > tw {
                    self.b.ins().ireduce(to.ir(), x)
                } else if from.signed() {
                    self.b.ins().sextend(to.ir(), x)
                } else {
                    self.b.ins().uextend(to.ir(), x)
                })
            }
        }
    }

    /// `c ? a : b`, all of type `n` (`c` a bool).
    pub fn select(&mut self, c: V, a: V, b: V, n: N) -> V {
        if !self.live {
            return V::K(0);
        }
        match c {
            V::K(x) => {
                if x != 0 {
                    a
                } else {
                    b
                }
            }
            V::D(cv) => {
                if let (V::K(x), V::K(y)) = (a, b) {
                    let (x, y) = (x & n.mask(), y & n.mask());
                    if x == y {
                        return V::K(x);
                    }
                    let (xv, yv) = (self.konst(n, x), self.konst(n, y));
                    let v = self.b.ins().select(cv, xv, yv);
                    self.sels.insert(v, (cv, x, y));
                    return V::D(v);
                }
                let (x, y) = (self.val(a, n), self.val(b, n));
                V::D(self.b.ins().select(cv, x, y))
            }
        }
    }

    /// `v` as one of two known constants, when it is.
    fn sel_of(&self, v: V) -> Option<(Value, u128, u128)> {
        match v {
            V::D(x) => self.sels.get(&x).copied(),
            V::K(_) => None,
        }
    }

    /// Whether `v` (of type `n`) matches one of `pats`.
    pub fn test(&mut self, v: V, n: N, pats: &[Pat]) -> V {
        if !self.live {
            return V::K(0);
        }
        let mut acc = V::K(0);
        for p in pats {
            let t = match *p {
                Pat::Any => V::K(1),
                Pat::Is(c) => self.op(Op::Eq, v, V::K(c as u128 & n.mask()), n),
                Pat::Range(lo, hi) => {
                    let a = self.op(Op::Ge, v, V::K(lo as u128 & n.mask()), n);
                    let b = self.op(Op::Le, v, V::K(hi as u128 & n.mask()), n);
                    self.op(Op::And, a, b, N::Bool)
                }
            };
            acc = self.op(Op::Or, acc, t, N::Bool);
            if acc == V::K(1) {
                break;
            }
        }
        acc
    }

    /// Bit counts: `count_ones`, `leading_zeros`, `trailing_zeros` of `a`
    /// (of type `n`), as a u32.
    pub fn count(&mut self, which: Count, a: V, n: N) -> V {
        if !self.live {
            return V::K(0);
        }
        match a {
            V::K(x) => {
                let x = x & n.mask();
                V::K(match which {
                    Count::Ones => x.count_ones(),
                    Count::Leading => x.leading_zeros() - (128 - n.width()),
                    Count::Trailing => x.trailing_zeros().min(n.width()),
                } as u128)
            }
            V::D(x) => {
                let r = match which {
                    Count::Ones => self.b.ins().popcnt(x),
                    Count::Leading => self.b.ins().clz(x),
                    Count::Trailing => self.b.ins().ctz(x),
                };
                let to = N::U32;
                V::D(match n.width() {
                    32 => r,
                    w if w > 32 => self.b.ins().ireduce(to.ir(), r),
                    _ => self.b.ins().uextend(to.ir(), r),
                })
            }
        }
    }
}

/// Which bits a count counts.
#[derive(Clone, Copy, Debug)]
pub enum Count {
    Ones,
    Leading,
    Trailing,
}

/// `x op y` on known values of type `n`.
fn fold(op: Op, x: u128, y: u128, n: N) -> u128 {
    let m = n.mask();
    let (x, y) = (x & m, y);
    let w = n.width() as u128;
    let sx = n.widen(x);
    let r = match op {
        Op::Add => x.wrapping_add(y),
        Op::Sub => x.wrapping_sub(y),
        Op::Mul => x.wrapping_mul(y),
        Op::Div | Op::Rem => {
            let y = y & m;
            if y == 0 {
                return 0;
            }
            if n.signed() {
                let (a, b) = (sx, n.widen(y));
                (if op == Op::Div { a.wrapping_div(b) } else { a.wrapping_rem(b) }) as u128
            } else if op == Op::Div {
                x / y
            } else {
                x % y
            }
        }
        Op::And => x & y,
        Op::Or => x | y,
        Op::Xor => x ^ y,
        Op::Shl => {
            if y >= w {
                0
            } else {
                x << y
            }
        }
        Op::Shr => {
            if n.signed() {
                if y >= w {
                    if sx < 0 {
                        u128::MAX
                    } else {
                        0
                    }
                } else {
                    (sx >> y) as u128
                }
            } else if y >= w {
                0
            } else {
                x >> y
            }
        }
        Op::Eq | Op::Ne | Op::Lt | Op::Le | Op::Gt | Op::Ge => {
            let y = y & m;
            let c = if n.signed() {
                let sy = n.widen(y);
                match op {
                    Op::Eq => sx == sy,
                    Op::Ne => sx != sy,
                    Op::Lt => sx < sy,
                    Op::Le => sx <= sy,
                    Op::Gt => sx > sy,
                    _ => sx >= sy,
                }
            } else {
                match op {
                    Op::Eq => x == y,
                    Op::Ne => x != y,
                    Op::Lt => x < y,
                    Op::Le => x <= y,
                    Op::Gt => x > y,
                    _ => x >= y,
                }
            };
            return c as u128;
        }
    };
    r & m
}

/// 128-bit division for hosts without it: `op` bit 0 the remainder, bit 1
/// signed, bit 2 the result's high half. A zero divisor (or the signed
/// quotient that overflows) panics, as the interpreter's does.
extern "C" fn wide_div(_machine: usize, op: u64, alo: u64, ahi: u64, blo: u64, bhi: u64) -> u64 {
    let a = ((ahi as u128) << 64) | alo as u128;
    let b = ((bhi as u128) << 64) | blo as u128;
    let r = match op & 3 {
        0 => a / b,
        1 => a % b,
        2 => ((a as i128) / (b as i128)) as u128,
        _ => ((a as i128) % (b as i128)) as u128,
    };
    if op & 4 != 0 {
        (r >> 64) as u64
    } else {
        r as u64
    }
}
