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
type Region = (State, Block, Vec<Option<State>>);

#[derive(Clone)]
struct State {
    slots: Vec<Option<u128>>,
    regs: HashMap<(usize, N), Known>,
    dirty: Vec<(usize, N)>,
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
}

/// Builds one compiled run (see the module docs).
pub struct Em<'a, 'f> {
    pub(crate) b: &'a mut FunctionBuilder<'f>,
    pub(crate) ptr: Type,
    pub(crate) cc: CallConv,
    pub(crate) m: Value,
    pub(crate) from: Value,
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
    /// Fields written but not yet stored.
    dirty: Vec<(usize, N)>,
}

impl<'a, 'f> Em<'a, 'f> {
    pub(crate) fn new(b: &'a mut FunctionBuilder<'f>, ptr: Type, cc: CallConv, m: Value, from: Value, out: Block) -> Self {
        Em { b, ptr, cc, m, from, out, k: 0, live: false, slots: Vec::new(), regs: HashMap::new(), sigs: HashMap::new(), regions: Vec::new(), epochs: None, volatile: Vec::new(), bumped: false, dirty: Vec::new() }
    }

    /// Code is being emitted (not after an exit or a return).
    pub fn live(&self) -> bool {
        self.live
    }

    pub(crate) fn begin_step(&mut self, k: usize, block: Block, entry: bool) {
        if self.live {
            if entry {
                self.flush();
            }
            self.b.ins().jump(block, &[]);
        }
        self.b.switch_to_block(block);
        self.k = k;
        self.live = true;
        // Entered from the switch too: nothing is known.
        if entry {
            self.regs.clear();
        }
    }

    pub(crate) fn finish(&mut self, n: usize) {
        if self.live {
            self.flush();
            let c = self.b.ins().iconst(types::I32, n as i64);
            let c = self.b.ins().isub(c, self.from);
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
            if self.regs.get(&(off, n)) == Some(&Known::K(c & n.mask())) {
                return;
            }
        }
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

    /// Read a place.
    pub fn read(&mut self, p: &Place) -> V {
        if !self.live {
            return V::K(0);
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
                let whole = self.read(&Place::Mem { off: *off, n: *reg, bump: 0 });
                let s = self.op(Op::Shr, whole, V::K(*shift as u128), *reg);
                let mask = if *width >= 128 { u128::MAX } else { (1u128 << width) - 1 };
                let raw = self.op(Op::And, s, V::K(mask), *reg);
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
            Place::Mem { n, .. } | Place::Bits { n, .. } => *n,
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
                let fv = self.op(Op::And, fv, V::K(mask), *reg);
                let fv = self.op(Op::Shl, fv, V::K(*shift as u128), *reg);
                let whole = self.read(&Place::Mem { off: *off, n: *reg, bump: 0 });
                let keep = !(mask << shift) & reg.mask();
                let kept = self.op(Op::And, whole, V::K(keep), *reg);
                let nv = self.op(Op::Or, kept, fv, *reg);
                self.store(*off, *reg, nv);
                self.bump(*bump);
            }
            Place::Local(s) => self.set(*s, v),
            Place::Switch { scrut, n, arms } => self.write_switch(*scrut, *n, arms, v),
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
    pub fn region_begin(&mut self) {
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
        let Some(Some((before, join, _))) = self.regions.last() else { return };
        let (before, join) = (before.clone(), *join);
        let st = self.leave(&before, join);
        if let Some(Some((_, _, ends))) = self.regions.last_mut() {
            ends.push(st);
        }
    }

    pub fn region_end(&mut self) {
        self.ret();
        match self.regions.pop() {
            Some(Some((before, join, ends))) => {
                self.b.seal_block(join);
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
        let zero = self.konst(n, 0);
        self.b.def_var(var, zero);
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
        State { slots: self.slots.iter().map(|s| s.k).collect(), regs: self.regs.clone(), dirty: self.dirty.clone() }
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
                self.flush();
                let tb = self.b.create_block();
                let fb = self.b.create_block();
                self.b.ins().brif(cv, tb, &[], fb, &[]);
                self.b.seal_block(tb);
                self.b.seal_block(fb);
                let before = self.state();
                let join = self.b.create_block();
                self.b.switch_to_block(tb);
                t(self);
                let a = self.leave(&before, join);
                self.restore(&before);
                self.live = true;
                self.b.switch_to_block(fb);
                f(self);
                let b = self.leave(&before, join);
                self.b.seal_block(join);
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
    /// leave them otherwise), and a jump to `join`. None when it left the
    /// code.
    fn leave(&mut self, before: &State, join: Block) -> Option<State> {
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
        self.flush();
        let st = self.state();
        self.b.ins().jump(join, &[]);
        self.live = false;
        Some(st)
    }

    /// After the branches: what every branch that comes back left known.
    fn enter(&mut self, before: &State, join: Block, ends: &[Option<State>]) {
        let ends: Vec<&State> = ends.iter().flatten().collect();
        self.slots.truncate(before.slots.len());
        if ends.is_empty() {
            // No branch comes back: the join is never reached.
            self.live = false;
            return;
        }
        self.b.switch_to_block(join);
        self.live = true;
        for i in 0..self.slots.len() {
            let first = ends[0].slots[i];
            self.slots[i].k = if ends.iter().all(|e| e.slots[i] == first) { first } else { None };
        }
        let mut regs = HashMap::new();
        for (key, v) in &ends[0].regs {
            if ends.iter().all(|e| e.regs.get(key) == Some(v)) {
                // A host value made in one branch does not reach the join.
                let ok = match v {
                    Known::K(_) => true,
                    Known::D(_) => ends.len() == 1 || before.regs.get(key) == Some(v),
                };
                if ok {
                    regs.insert(*key, *v);
                }
            }
        }
        self.regs = regs;
        self.dirty.clear();
    }

    fn restore(&mut self, st: &State) {
        self.slots.truncate(st.slots.len());
        for (i, k) in st.slots.iter().enumerate() {
            self.slots[i].k = *k;
        }
        self.regs = st.regs.clone();
        self.dirty = st.dirty.clone();
    }

    /// Where the locals stand: locals made after it are released by
    /// `release` (they are out of scope).
    pub fn mark(&self) -> usize {
        self.slots.len()
    }

    pub fn release(&mut self, mark: usize) {
        self.slots.truncate(mark);
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
                then(self);
                self.exit(made);
                self.restore(&before);
                self.b.switch_to_block(cb);
                self.live = true;
            }
        }
    }

    /// Leave the compiled run here.
    pub fn exit(&mut self, made: bool) {
        if !self.live {
            return;
        }
        self.flush();
        let c = self.b.ins().iconst(types::I32, self.k as i64 + made as i64);
        let c = self.b.ins().isub(c, self.from);
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
                let dirty = self.dirty.clone();
                self.flush();
                self.dirty = dirty;
                let made = self.b.ins().icmp_imm(IntCC::Equal, rv, 1);
                let made = self.b.ins().uextend(types::I32, made);
                let c = self.b.ins().iconst(types::I32, self.k as i64);
                let c = self.b.ins().iadd(c, made);
                let c = self.b.ins().isub(c, self.from);
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
        // The machine holds what the code wrote before the call sees it.
        self.flush();
        let fp = self.b.ins().iconst(self.ptr, f as i64);
        let call = self.b.ins().call_indirect(sig, fp, &a);
        let r = self.b.inst_results(call)[0];
        if pure {
            self.forget_volatile();
        } else {
            self.regs.clear();
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
        if !self.live {
            return V::K(0);
        }
        if let (V::K(x), V::K(y)) = (a, b) {
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
            (V::D(x), N::Bool) => V::D(self.b.ins().icmp_imm(IntCC::Equal, x, 0)),
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
                let (x, y) = (self.val(a, n), self.val(b, n));
                V::D(self.b.ins().select(cv, x, y))
            }
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
