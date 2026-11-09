//! The trace core (`core: trace;`, SPEC 12, "Cores"): runs of decoded
//! instructions kept and run again without decoding.
//!
//! As the processor runs, each instruction it decodes is recorded with what
//! decoding left behind (its `Instr`, its length, the latches its prefixes
//! set, the pc after it, the decoder's clocks), appended to the run under
//! way while the instructions it reaches stay in its page of code memory
//! (past taken jumps, calls and returns), up to one it already holds. When
//! a run's start is reached again, its instructions are replayed: the pc,
//! latches and decoder state are set as decoding left them, the fetches are
//! charged as they cost (`<space>_fetch_skip`), and the instruction runs.
//!
//! A replay is exactly the run the decoder would have made, and the
//! generator derives what that takes:
//! - The decoder reads some state (the operand size's default: `CS` on the
//!   386). A run starts only where that state is as it was (`TKey`,
//!   compared by value), at the same pc and physical address.
//! - The istream's address, window and limit fns read more (the segment,
//!   paging, the TLB). Every assignment the generator emits to state the
//!   decoder or those fns read, besides the pc, bumps `tr_epoch`: a run
//!   stops when it changes, and the next instruction is decoded. What is
//!   not in the key also bumps `tr_aepoch`: while it is unchanged and the
//!   key holds, the pc still leads where it did, and a run that followed
//!   another is entered from it directly (a link), without finding where
//!   the pc is.
//! - The pc must be where the run goes next (each step's own pc): a jump
//!   taken otherwise than it was recorded leaves it.
//! - Code memory: each `direct` memory marks the 128-byte blocks of each
//!   4 KB page that hold recorded code; a write to a marked block counts
//!   against the page (the processor's, DMA's, any device's). A run is
//!   replayed only while its page's count is unchanged: checked before an
//!   instruction when code memory's write total has moved.
//!
//! Interrupts, device catch-up and timing run per instruction as with the
//! decode core.

use std::fmt::Write as _;

use machine_sema::hir::{ArmBody, Block, Expr, ExprKind as H, Stmt, StmtKind as S, TailExpr};
use machine_sema::program::DefId;
use machine_sema::ty::Ty;

use super::{ident, Cx, Gen, GenError, R};

/// Runs kept, direct-mapped by physical address. A game touches tens of
/// thousands of instructions: at 4096, runs sharing a slot evicted each
/// other before they were entered often enough to compile (Xmas
/// Lemmings' bit-counting loop never was); 16384 keeps them, at a few MB.
const TRACES: usize = 16384;
/// Instructions in a run at most.
const STEPS: usize = 64;

/// State read: a whole register, one of its fields, or the mode.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Cell {
    Reg(DefId),
    Field(DefId, u32),
    Mode,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct TracePlan {
    /// Registers the decoder reads, besides the pc: the run's key.
    pub key: Vec<DefId>,
    pub key_mode: bool,
    /// What a run depends on staying put: assignments to it bump the epoch.
    pub watched: Vec<Cell>,
    /// The registers the istream's `next` fn advances.
    pub pc: Vec<DefId>,
    /// Registers the JIT core speculates on (`jit_spec_regs`).
    pub spec: Vec<DefId>,
}

struct Reads<'g, 'a> {
    g: &'g Gen<'a>,
    cells: Vec<Cell>,
    seen: Vec<DefId>,
    /// A fn not to walk into (the address fn, when walking the decoder).
    skip: Option<DefId>,
}

impl Reads<'_, '_> {
    fn add(&mut self, c: Cell) {
        if !self.cells.contains(&c) {
            self.cells.push(c);
        }
    }
    fn fn_body(&mut self, d: DefId) {
        if Some(d) == self.skip || self.seen.contains(&d) {
            return;
        }
        self.seen.push(d);
        if let Some(f) = self.g.fns.get(&d) {
            if !f.external {
                self.block(&f.body);
            }
        }
    }
    fn block(&mut self, b: &Block) {
        for s in &b.stmts {
            self.stmt(s);
        }
    }
    fn arm(&mut self, a: &ArmBody) {
        match a {
            ArmBody::Expr(e) => self.expr(e),
            ArmBody::Block(b) => self.block(b),
        }
    }
    fn stmt(&mut self, s: &Stmt) {
        match &s.kind {
            S::Let(_, Some(e)) | S::Return(Some(e)) | S::Expr(e) | S::Assert(e, _) => self.expr(e),
            S::If(c, b, e) => {
                self.expr(c);
                self.block(b);
                if let Some(e) = e {
                    self.stmt(e);
                }
            }
            S::Match(e, arms) => {
                self.expr(e);
                for a in arms {
                    self.arm(&a.body);
                }
            }
            S::For(_, _, _, b) => self.block(b),
            S::Assign(p, v) => {
                self.place(p);
                self.expr(v);
            }
            S::SpaceWrite(_, _, a, v) => {
                self.expr(a);
                self.expr(v);
            }
            S::OperandWrite(_, v) => self.expr(v),
            S::Raise(_, args) => args.iter().for_each(|a| self.expr(a)),
            _ => {}
        }
    }
    /// A place assigned to: only its indexes are read.
    fn place(&mut self, p: &Expr) {
        match &p.kind {
            H::Index(b, i) => {
                self.place(b);
                self.expr(i);
            }
            H::Field(b, _) | H::TupleField(b, _) | H::Slice(b, ..) => self.place(b),
            H::Register(_) | H::RegField(..) | H::RegView(..) | H::Mode(_) | H::Latch(_) | H::Local(_) => {}
            _ => self.expr(p),
        }
    }
    fn expr(&mut self, e: &Expr) {
        match &e.kind {
            H::Register(d) => self.add(Cell::Reg(*d)),
            H::RegField(d, i) => self.add(Cell::Field(*d, *i)),
            H::RegView(d, i) => {
                for f in self.g.regs[d].1.views[*i as usize].mapping.clone() {
                    self.add(Cell::Field(*d, f));
                }
            }
            H::Mode(_) => self.add(Cell::Mode),
            H::Unary(_, a) | H::Cast(a) | H::Field(a, _) | H::TupleField(a, _) | H::Slice(a, ..) | H::ArrayRepeat(a, _) | H::OperandField(a, _) | H::OperandTail(a) | H::TailOperand(a, _) | H::SpaceRead(_, _, a) | H::SpaceCell(_, a) => self.expr(a),
            H::Binary(_, a, b) | H::Index(a, b) | H::SpaceWrite(_, _, a, b) => {
                self.expr(a);
                self.expr(b);
            }
            H::Ternary(a, b, c) => {
                self.expr(a);
                self.expr(b);
                self.expr(c);
            }
            H::Call(_, args) | H::Builtin(_, args) | H::BundleLit(_, args) | H::Tuple(args) | H::Array(args) | H::KindSelect(_, args) => args.iter().for_each(|a| self.expr(a)),
            H::Match(s, arms) => {
                self.expr(s);
                for a in arms {
                    self.arm(&a.body);
                }
            }
            H::If(c, t, f) => {
                self.expr(c);
                self.block(t);
                if let Some(f) = f {
                    self.expr(f);
                }
            }
            H::Block(b) => self.block(b),
            H::FnCall(d, args) => {
                args.iter().for_each(|a| self.expr(a));
                self.fn_body(*d);
            }
            _ => {}
        }
    }
    fn tail(&mut self, t: &TailExpr) {
        if let TailExpr::If(c, a, b) = t {
            self.expr(c);
            self.tail(a);
            if let Some(b) = b {
                self.tail(b);
            }
        }
    }
}

impl Gen<'_> {
    /// What the trace core needs to know of the isa; None without an
    /// istream window (instructions at consecutive addresses).
    /// The registers the JIT core compiles a run for the values of (a
    /// guard at entry checks them; a write ends the run): plain registers
    /// that the isa's fns read (the mode the memory path and the decoder's
    /// helpers branch on), that no istream hook writes (they change every
    /// step), and that at most three instructions write themselves (on the
    /// 386: CR0, CR3, the debug and test registers).
    fn jit_spec_regs(&self, pc: &[DefId], key: &[DefId]) -> Vec<DefId> {
        use std::collections::{HashMap, HashSet};
        fn root(e: &Expr) -> Option<DefId> {
            match &e.kind {
                H::Register(d) | H::RegField(d, _) | H::RegView(d, _) => Some(*d),
                H::Field(b, _) | H::Index(b, _) | H::Slice(b, ..) => root(b),
                _ => None,
            }
        }
        let mut read: HashSet<DefId> = HashSet::new();
        for f in self.fns.values() {
            machine_sema::hir::visit_block(&f.body, &mut |_| {}, &mut |x| {
                if let H::Register(d) | H::RegField(d, _) = &x.kind {
                    read.insert(*d);
                }
            });
        }
        // What the istream's hooks write themselves: every step (the Z80's
        // R, the 386's interrupt shadow). What fns they call write is a rare
        // path (an interrupt delivered), and a write still ends the run.
        let ist = self.isa.istream.as_ref().unwrap();
        let mut hooks: Vec<DefId> = vec![ist.next];
        hooks.extend(ist.opcode);
        hooks.extend(ist.boundary);
        let mut every: HashSet<DefId> = HashSet::new();
        for f in hooks {
            let Some(fb) = self.fns.get(&f) else { continue };
            machine_sema::hir::visit_block(&fb.body, &mut |st| {
                if let S::Assign(p, _) = &st.kind {
                    if let Some(d) = root(p) {
                        every.insert(d);
                    }
                }
            }, &mut |_| {});
        }
        // Registers an operand kind selects are the instructions' operands:
        // written all the time, whatever the count of direct writes says.
        for k in self.kinds.values() {
            for r in &k.rules {
                if r.kind != machine_sema::hir::RuleKind::Select {
                    continue;
                }
                let mut on = |x: &Expr| {
                    if let H::Register(d) | H::RegField(d, _) | H::RegView(d, _) = &x.kind {
                        every.insert(*d);
                    }
                };
                match &r.body {
                    ArmBody::Expr(x) => machine_sema::hir::visit_expr(x, &mut |_| {}, &mut on),
                    ArmBody::Block(b) => machine_sema::hir::visit_block(b, &mut |_| {}, &mut on),
                }
            }
        }
        let mut writers: HashMap<DefId, usize> = HashMap::new();
        for i in &self.isa.instructions {
            let Some(b) = &i.semantics else { continue };
            let mut mine: HashSet<DefId> = HashSet::new();
            machine_sema::hir::visit_block(b, &mut |st| {
                if let S::Assign(p, _) = &st.kind {
                    if let Some(d) = root(p) {
                        mine.insert(d);
                    }
                }
            }, &mut |_| {});
            for d in mine {
                *writers.entry(d).or_default() += 1;
            }
        }
        let mut out: Vec<DefId> = self
            .regs
            .iter()
            .filter(|(d, (_, r))| {
                matches!(r.ty, Ty::UInt(_)) && read.contains(d) && !every.contains(d) && !pc.contains(d) && !key.contains(d) && writers.get(d).copied().unwrap_or(0) <= 3
            })
            .map(|(d, _)| *d)
            .collect();
        out.sort();
        out
    }

    pub(crate) fn plan_trace(&self, jit: bool) -> R<TracePlan> {
        let ist = self.isa.istream.as_ref().ok_or_else(|| GenError::new("`core: trace` needs an istream"))?;
        if ist.window.is_none() {
            return Err(GenError::new("`core: trace` needs `istream { window ... }`: instructions at consecutive addresses"));
        }
        if self.spaces[&ist.space].cell_bits != 8 {
            return Err(GenError::new("`core: trace` needs a byte-addressed code space"));
        }
        // The decoder: its `next` fn (not the address it fetches from), the
        // latches' reset values, decode conditions, prefixes, operand tails.
        let mut dec = Reads { g: self, cells: Vec::new(), seen: Vec::new(), skip: Some(ist.address) };
        dec.fn_body(ist.next);
        for l in &self.latches {
            dec.expr(&l.init);
        }
        for i in &self.isa.instructions {
            if let Some(c) = &i.when {
                dec.expr(c);
            }
            if i.prefix {
                if let Some(b) = &i.semantics {
                    dec.block(b);
                }
                if let Some(t) = &i.timing {
                    dec.expr(t);
                }
                for b in &i.operands {
                    if let Some(k) = self.kinds.get(&b.kind) {
                        for r in &k.rules {
                            dec.arm(&r.body);
                        }
                    }
                }
            }
        }
        for o in &self.isa.operands {
            if let Some(t) = &o.tail {
                dec.tail(t);
            }
        }
        // The pc: what `next` assigns.
        let mut pc = Vec::new();
        let next = self.fns[&ist.next];
        fn assigned(b: &Block, out: &mut Vec<DefId>) {
            for s in &b.stmts {
                match &s.kind {
                    S::Assign(p, _) => {
                        let mut e = p;
                        loop {
                            match &e.kind {
                                H::Register(d) | H::RegField(d, _) | H::RegView(d, _) => {
                                    if !out.contains(d) {
                                        out.push(*d);
                                    }
                                    break;
                                }
                                H::Field(b, _) | H::Index(b, _) | H::Slice(b, ..) => e = b,
                                _ => break,
                            }
                        }
                    }
                    S::If(_, b, e) => {
                        assigned(b, out);
                        if let Some(e) = e {
                            assigned(&Block { stmts: vec![(**e).clone()], span: b.span }, out);
                        }
                    }
                    _ => {}
                }
            }
        }
        assigned(&next.body, &mut pc);
        // The address, window and limit fns.
        let mut addr = Reads { g: self, cells: dec.cells.clone(), seen: Vec::new(), skip: None };
        addr.fn_body(ist.address);
        if let Some(w) = ist.window {
            addr.fn_body(w);
        }
        if let Some(l) = ist.limit {
            addr.fn_body(l);
        }
        let not_pc = |c: &Cell| match c {
            Cell::Reg(d) | Cell::Field(d, _) => !pc.contains(d),
            Cell::Mode => true,
        };
        let mut key = Vec::new();
        let mut key_mode = false;
        for c in dec.cells.iter().filter(|c| not_pc(c)) {
            match c {
                Cell::Reg(d) | Cell::Field(d, _) => {
                    if !key.contains(d) {
                        key.push(*d);
                    }
                }
                Cell::Mode => key_mode = true,
            }
        }
        let mut watched: Vec<Cell> = addr.cells.into_iter().filter(not_pc).collect();
        // The JIT core's speculated registers: watched, so a write ends the
        // run and, inside one, they hold the values its entry checked.
        let spec = if jit { self.jit_spec_regs(&pc, &key) } else { Vec::new() };
        for d in &spec {
            if !watched.contains(&Cell::Reg(*d)) {
                watched.push(Cell::Reg(*d));
            }
        }
        Ok(TracePlan { key, key_mode, watched, pc, spec })
    }

    /// The epoch bump after an assignment to `place`, when it writes
    /// watched state.
    pub(super) fn trace_bump(&self, cx: &Cx, place: &Expr) -> String {
        if cx.comp.is_some() {
            return String::new();
        }
        self.trace_bump_in(cx.recv, place)
    }

    /// The epoch bumps a write to `place` makes, on the processor (`self`).
    pub(super) fn trace_bump_text(&self, place: &Expr) -> String {
        self.trace_bump_in("self", place)
    }

    fn trace_bump_in(&self, recv: &str, place: &Expr) -> String {
        let Some(p) = self.trace.as_ref() else { return String::new() };
        let mut e = place;
        let written: Vec<Cell> = loop {
            match &e.kind {
                H::Register(d) => break vec![Cell::Reg(*d)],
                H::RegField(d, i) => break vec![Cell::Field(*d, *i)],
                H::RegView(d, i) => break self.regs[d].1.views[*i as usize].mapping.iter().map(|f| Cell::Field(*d, *f)).collect(),
                H::Mode(_) => break vec![Cell::Mode],
                H::Index(b, _) | H::Field(b, _) | H::TupleField(b, _) | H::Slice(b, ..) => e = b,
                _ => return String::new(),
            }
        };
        let bits = |d: DefId, i: u32| -> (u32, u32) {
            let f = &self.regs[&d].1.fields[i as usize];
            let w = match &f.ty {
                Ty::UInt(n) | Ty::SInt(n) => *n,
                Ty::Bool => 1,
                Ty::Enum(e) => self.enums[e].width,
                _ => 8,
            };
            (f.offset, f.offset + w)
        };
        let hits = |c: &Cell| {
            written.iter().any(|w| match (w, c) {
                (Cell::Reg(a), Cell::Reg(b) | Cell::Field(b, _)) => a == b,
                (Cell::Field(a, _), Cell::Reg(b)) => a == b,
                (Cell::Field(a, i), Cell::Field(b, j)) => {
                    let ((a0, a1), (b0, b1)) = (bits(*a, *i), bits(*b, *j));
                    a == b && a0 < b1 && b0 < a1
                }
                (Cell::Mode, Cell::Mode) => true,
                _ => false,
            })
        };
        // The key (compared by value where runs are entered) only stops a
        // run; the rest also stops links and entries checked by epoch.
        let in_key = |c: &Cell| match c {
            Cell::Reg(d) | Cell::Field(d, _) => p.key.contains(d),
            Cell::Mode => p.key_mode,
        };
        let any = p.watched.iter().any(hits);
        let outside = p.watched.iter().filter(|c| !in_key(c)).any(hits);
        let mut s = String::new();
        if any {
            let _ = writeln!(s, "{0}.tr_epoch = {0}.tr_epoch.wrapping_add(1);", recv);
        }
        if outside {
            let _ = writeln!(s, "{0}.tr_aepoch = {0}.tr_aepoch.wrapping_add(1);", recv);
        }
        if p.watched.iter().filter(|c| in_key(c)).any(hits) || written.iter().any(|w| match w {
            Cell::Reg(d) | Cell::Field(d, _) => p.key.contains(d),
            Cell::Mode => p.key_mode,
        }) {
            let _ = writeln!(s, "{0}.tr_kepoch = {0}.tr_kepoch.wrapping_add(1);", recv);
        }
        s
    }

    /// `TKey`, `TStep` and `Trace` (module level).
    pub(crate) fn emit_trace_types(&mut self) -> R<()> {
        let Some(p) = self.trace.clone() else { return Ok(()) };
        let mut key = String::new();
        for d in &p.key {
            let r = self.regs[d].1;
            let _ = writeln!(key, "    pub r_{}: {},", ident(&r.name), self.rty(&r.ty)?);
        }
        if p.key_mode {
            let mode_ty = match self.isa.mode {
                Some(m) => super::native(self.enums[&m].width, false).1,
                None => "u8",
            };
            let _ = writeln!(key, "    pub mode: {mode_ty},");
        }
        let mut step = String::new();
        for l in &self.latches {
            let _ = writeln!(step, "    pub l_{}: {},", ident(&l.name), self.rty(&l.ty)?);
        }
        for d in &p.pc {
            let r = self.regs[d].1;
            let _ = writeln!(step, "    pub pc_{}: {},", ident(&r.name), self.rty(&r.ty)?);
        }
        let jit = self.jit_trace_fields();
        let _ = write!(
            self.out,
            "/// What the decoder reads besides the bytes and the pc: a run starts\n/// only where it is as it was.\n#[derive(Clone, Copy, Debug, Default, PartialEq)]\npub struct TKey {{\n{key}}}\n/// An instruction of a run, decoded, and what decoding left behind.\n#[derive(Clone, Copy, Debug)]\npub struct TStep {{\n    pub instr: Instr,\n    /// Its bytes: from the run's start, and how many.\n    pub off: u16,\n    pub len: u8,\n    pub known: bool,\n    pub prefix_cycles: u32,\n    pub decode_at: [u8; 16],\n    /// Opcode bytes fetched (`istream {{ opcode }}` runs that often).\n    pub opcodes: u8,\n{step}}}\n/// A run of instructions in a page of code memory, from `phys` (pc\n/// `pc`): valid while the page's write count is `stamp`, or none of its\n/// blocks has been written since. Its key was last\n/// found to hold at epoch `ok_ep`; and the runs that followed it, by pc,\n/// at an address epoch (while it is current, and the key holds, the pc\n/// still leads there).\n#[derive(Clone, Debug)]\npub struct Trace {{\n    pub phys: u64,\n    pub pc: u64,\n    pub key: TKey,\n    pub page: u32,\n    pub stamp: u32,\n    /// The page's 128-byte blocks its bytes are in: while none of them is\n    /// written, it holds though the page's count moved (`_code_fresh`).\n    pub mask: u32,\n    pub ok_ep: u64,\n    pub links: [TLink; 2],\n    pub steps: Vec<TStep>,\n{jit}}}\n/// A run that followed another: at pc `pc`, run `slot` (at `phys`), at\n/// address epoch `ep`; its key last found to hold at key epoch `kep`.\n#[derive(Clone, Copy, Debug, Default)]\npub struct TLink {{\n    pub pc: u64,\n    pub phys: u64,\n    pub ep: u64,\n    pub kep: u64,\n    pub slot: u32,\n}}\n"
        );
        let jt = self.jit_types();
        self.out.push_str(jt);
        Ok(())
    }

    /// The machine's small trace fields (hot) and its tables.
    pub(crate) fn trace_fields(&self) -> (&'static str, &'static str) {
        if self.trace.is_none() {
            return ("", "");
        }
        (
            "    /// The trace core: replaying run `tr_cur` (when `tr_on`) at\n    /// instruction `tr_idx`, entered at epoch `tr_ep0`; the epoch, bumped\n    /// by writes to what runs depend on; recording this step; code memory's\n    /// write total when last checked; the run that just ended.\n    pub tr_on: bool,\n    pub tr_rec_now: bool,\n    pub tr_idx: u32,\n    pub tr_cur: u32,\n    pub tr_prev: u32,\n    pub tr_epoch: u64,\n    pub tr_ep0: u64,\n    pub tr_aepoch: u64,\n    pub tr_kepoch: u64,\n    pub tr_w: u64,\n    pub tr_phys: u64,\n    /// The replay loop: the step's prologue left the step to the\n    /// instruction (`replay_prologue`).\n    pub tr_go: bool,\n",
            "    /// The runs kept, and the one being recorded (from epoch\n    /// `tr_rec_ep`).\n    pub traces: Box<[Option<Trace>]>,\n    pub tr_rec: Option<Trace>,\n    pub tr_rec_ep: u64,\n",
        )
    }

    pub(crate) fn trace_init(&self) -> String {
        if self.trace.is_none() {
            return String::new();
        }
        format!("            tr_on: false,\n            tr_rec_now: false,\n            tr_idx: 0,\n            tr_cur: 0,\n            tr_prev: u32::MAX,\n            tr_epoch: 0,\n            tr_ep0: 0,\n            tr_aepoch: 0,\n            tr_kepoch: 0,\n            tr_w: u64::MAX,\n            tr_phys: 0,\n            tr_go: false,\n            traces: vec![None; {TRACES}].into_boxed_slice(),\n            tr_rec: None,\n            tr_rec_ep: 0,\n")
    }

    /// The trace core's methods (inside the machine's impl).
    pub(crate) fn emit_trace_fns(&mut self) -> R<()> {
        let Some(p) = self.trace.clone() else { return Ok(()) };
        let ist = self.isa.istream.as_ref().unwrap();
        let sp = ident(&self.spaces[&ist.space].name);
        let mut key = String::new();
        for d in &p.key {
            let n = ident(&self.regs[d].1.name);
            let _ = write!(key, "r_{n}: self.r.{n}, ");
        }
        if p.key_mode {
            key.push_str("mode: self.mode, ");
        }
        // The pc as a number (the run's `pc`), and its registers saved and
        // restored.
        let pc0 = ident(&self.regs[&p.pc[0]].1.name);
        let mut save = String::new();
        let mut restore = String::new();
        for l in &self.latches {
            let n = ident(&l.name);
            let _ = write!(save, "l_{n}: self.latch_{n}, ");
            let _ = writeln!(restore, "        self.latch_{n} = s.l_{n};");
        }
        for d in &p.pc {
            let n = ident(&self.regs[d].1.name);
            let _ = write!(save, "pc_{n}: self.r.{n}, ");
            let _ = writeln!(restore, "        self.r.{n} = s.pc_{n};");
        }
        let mut runs = String::new();
        for i in self.isa.instructions.iter().filter(|i| !i.prefix) {
            let _ = writeln!(runs, "            Instr::{n}(ops) => self.run_{n}(ops),", n = ident(&i.name));
        }
        let mask = TRACES - 1;
        // `istream { opcode }`: a replay makes the decode's calls again.
        let (opcodes, opcode_replay) = match ist.opcode {
            Some(h) => ("self.opcodes".to_string(), format!("        self.opcodes = opcodes;\n        for _ in 0..opcodes {{ let _ = self.fn_{}(); }}\n", ident(&self.fns[&h].name))),
            None => ("0".to_string(), "        let _ = opcodes;\n".to_string()),
        };
        let jit_init = self.jit_trace_init();
        let jit_clear = if self.jit { "        self.jit.clear();\n" } else { "" };
        let _ = write!(
            self.out,
            r#"    #[inline]
    fn tr_slot(phys: u64) -> usize {{ ((phys ^ (phys >> 12)) as usize) & {mask} }}
    /// What the decoder reads: a run's key.
    #[inline]
    fn tkey(&self) -> TKey {{ TKey {{ {key}}} }}
    /// Can the run being replayed go on with its next instruction? When
    /// not, it stops replaying (and nothing else changes).
    #[inline]
    fn trace_check(&mut self) -> bool {{
        let cur = self.tr_cur as usize;
        let idx = self.tr_idx as usize;
        let Some(t) = self.traces[cur].as_ref() else {{ self.tr_on = false; return false; }};
        let Some(s) = t.steps.get(idx) else {{ self.tr_on = false; return false; }};
        if self.tr_epoch != self.tr_ep0 || self.r.{pc0} as u64 != t.pc.wrapping_add(s.off as u64) {{
            self.tr_on = false;
            self.tr_prev = cur as u32;
            return false;
        }}
        let w = self.bus.{sp}_code_writes();
        if w != self.tr_w {{
            let page = t.page;
            if !self.tr_fresh(cur, page) {{
                self.tr_on = false;
                self.tr_prev = u32::MAX;
                return false;
            }}
            self.tr_w = w;
        }}
        true
    }}
    /// Whether run `slot`, in `page`, holds: the page's write count is
    /// still the run's, or none of the run's blocks has been written since
    /// (a write beside it, in the same page, leaves it be: it takes the
    /// page's count, as if recorded now).
    #[inline]
    fn tr_fresh(&mut self, slot: usize, page: u32) -> bool {{
        let now = self.bus.{sp}_code_stamp(page);
        let (stamp, mask) = match self.traces.get(slot).and_then(|t| t.as_ref()) {{
            Some(t) if t.page == page => (t.stamp, t.mask),
            _ => return false,
        }};
        if now == stamp {{
            return true;
        }}
        if !self.bus.{sp}_code_fresh(page, mask, stamp) {{
            return false;
        }}
        if let Some(t) = self.traces[slot].as_mut() {{
            t.stamp = now;
        }}
        true
    }}
    /// The next instruction of the run, checked: the state set as decoding
    /// left it, and the run advanced. Its address, the instruction, its page
    /// and length.
    #[inline]
    fn trace_restore(&mut self) -> (u64, Instr, u32, u8) {{
        let cur = self.tr_cur as usize;
        let idx = self.tr_idx as usize;
        let t = self.traces[cur].as_ref().unwrap();
        let s = &t.steps[idx];
        let phys = t.phys + s.off as u64;
        let page = t.page;
        let last = idx + 1 == t.steps.len();
        let instr = s.instr;
        let len = s.len;
        self.fetched = len;
        self.known_opcode = s.known;
        self.prefix_cycles = s.prefix_cycles;
        if B::SCHEDULE {{ self.decode_at = s.decode_at; }}
        let opcodes = s.opcodes;
{restore}        self.tr_idx += 1;
{opcode_replay}
        if last {{
            self.tr_on = false;
            self.tr_prev = cur as u32;
        }}
        (phys, instr, page, len)
    }}
    /// The next instruction of the run being replayed, run, and where it
    /// was; None when the run ends here (it stops replaying).
    #[inline]
    fn trace_go(&mut self) -> Option<(u64, Result<(), Fault>)> {{
        if !self.trace_check() {{
            return None;
        }}
        let (phys, instr, page, len) = self.trace_restore();
        self.tr_phys = phys;
        self.restarted = phys == self.last_start;
        self.last_start = phys;
        self.bus.{sp}_fetch_page(page, phys, len as u32);
        Some((phys, self.run_instr(instr)))
    }}
    /// The replay loop: the steps of the run being replayed, one after
    /// another, each as `step` and `System::step` make it (its prologue,
    /// the interrupt boundary among it; the step restored, its fetch
    /// charged, run, its tail; the devices after it), with the run in hand
    /// (taken from its slot for the loop: nothing a step does reaches the
    /// table). It leaves the steps it cannot make as `step` would to
    /// `step`: a pc not the run's, code written since the last check, the
    /// processor halted, a prologue that ends the step (an interrupt taken,
    /// the reset line, shutdown: that step made here), the run's end, or
    /// `max` made. The steps made (0: none to replay).
    pub fn trace_burst(&mut self, max: u64) -> Result<u64, Fault> {{
        if !self.tr_on || max == 0 {{
            return Ok(0);
        }}
        let cur = self.tr_cur as usize;
        let Some(t) = self.traces[cur].take() else {{ return Ok(0) }};
        let mut n = 0u64;
        let r = loop {{
            if n >= max || !self.tr_on || self.halted {{
                break Ok(());
            }}
            let idx = self.tr_idx as usize;
            let Some(s) = t.steps.get(idx) else {{ break Ok(()) }};
            // `trace_check`, the run in hand (code written since: `step`
            // checks it, with the table).
            if self.tr_epoch != self.tr_ep0 || self.r.{pc0} as u64 != t.pc.wrapping_add(s.off as u64) {{
                self.tr_on = false;
                self.tr_prev = cur as u32;
                break Ok(());
            }}
            if self.bus.{sp}_code_writes() != self.tr_w {{
                break Ok(());
            }}
            self.tr_go = false;
            let r = self.replay_prologue();
            if !self.tr_go {{
                // The step ended in its prologue.
                self.bus.after_step();
                n += 1;
                break r;
            }}
            self.tr_rec_now = false;
            self.decoded = false;
            // `trace_restore`, the run in hand.
            let phys = t.phys + s.off as u64;
            let page = t.page;
            let last = idx + 1 == t.steps.len();
            let instr = s.instr;
            let len = s.len;
            self.fetched = len;
            self.known_opcode = s.known;
            self.prefix_cycles = s.prefix_cycles;
            if B::SCHEDULE {{ self.decode_at = s.decode_at; }}
            let opcodes = s.opcodes;
{restore}            self.tr_idx += 1;
{opcode_replay}
            if last {{
                self.tr_on = false;
                self.tr_prev = cur as u32;
            }}
            self.tr_phys = phys;
            self.restarted = phys == self.last_start;
            self.last_start = phys;
            self.bus.{sp}_fetch_page(page, phys, len as u32);
            let r = self.run_instr(instr);
            let r = self.replay_tail(phys, r);
            self.bus.after_step();
            n += 1;
            if r.is_err() {{
                break r;
            }}
        }};
        self.traces[cur] = Some(t);
        r.map(|()| n)
    }}
    /// The run the last one led to before, at this pc and epoch: entered
    /// without finding where the pc is (it is where it was).
    #[inline]
    fn trace_follow(&mut self) -> Option<(u64, Result<(), Fault>)> {{
        let prev = self.tr_prev as usize;
        let pc = self.r.{pc0} as u64;
        let li = self.traces.get(prev)?.as_ref()?.links.iter().position(|l| l.pc == pc && l.ep == self.tr_aepoch)?;
        let l = self.traces[prev].as_ref()?.links[li];
        // The key: unchanged since it was last found to hold, else compared.
        let kep = self.tr_kepoch;
        let key_known = l.kep == kep;
        let key = if key_known {{ None }} else {{ Some(self.tkey()) }};
        let t = self.traces[l.slot as usize].as_ref()?;
        if t.phys != l.phys || t.pc != pc || key.is_some_and(|k| t.key != k) {{
            return None;
        }}
        if !key_known {{
            if let Some(p) = self.traces[prev].as_mut() {{ p.links[li].kep = kep; }}
        }}
        self.tr_on = true;
        self.tr_cur = l.slot;
        self.tr_idx = 0;
        self.tr_ep0 = self.tr_epoch;
        self.tr_w = u64::MAX;
        self.trace_go()
    }}
    /// The run at `start`, if there is one and it holds: entered (and
    /// linked from the run before), and its first instruction.
    #[inline]
    fn trace_enter(&mut self, start: u64) -> Option<(u64, Result<(), Fault>)> {{
        let slot = Self::tr_slot(start);
        let pc = self.r.{pc0} as u64;
        let ep = self.tr_epoch;
        let aep = self.tr_aepoch;
        let key_ok = match self.traces[slot].as_ref() {{
            Some(t) if t.phys == start && t.pc == pc => t.ok_ep == ep || t.key == self.tkey(),
            _ => false,
        }};
        if !key_ok {{
            self.tr_prev = u32::MAX;
            return None;
        }}
        self.trace_close();
        if let Some(t) = self.traces[slot].as_mut() {{
            t.ok_ep = ep;
        }}
        let prev = self.tr_prev as usize;
        if let Some(p) = self.traces.get_mut(prev).and_then(|t| t.as_mut()) {{
            let l = TLink {{ pc, phys: start, ep: aep, kep: self.tr_kepoch, slot: slot as u32 }};
            if p.links[0].pc == pc || p.links[0].ep != aep {{ p.links[0] = l; }} else {{ p.links[1] = l; }}
        }}
        self.tr_on = true;
        self.tr_cur = slot as u32;
        self.tr_idx = 0;
        self.tr_ep0 = ep;
        self.tr_w = u64::MAX;
        self.trace_go()
    }}
    /// The instruction at `start` is about to be decoded: record it, in the
    /// run being recorded if it follows on, else in a new one. It follows
    /// on wherever the step before led in the run's page (a jump, a call
    /// or a return taken too: replay checks each step's pc), but not back
    /// before the run's start or to a step already in it (the run ends,
    /// and one starts there).
    #[inline(never)]
    fn trace_note(&mut self, start: u64) {{
        self.tr_prev = u32::MAX;
        // (Its code as recorded holds: the page unwritten, or none of the
        // blocks it has so far, as `tr_fresh` asks of a run.)
        let follows = match self.tr_rec.as_ref() {{
            Some(t) => {{
                let off = start.wrapping_sub(t.phys);
                t.steps.len() < {STEPS} && start >= t.phys && start >> 12 == t.phys >> 12 && self.r.{pc0} as u64 == t.pc.wrapping_add(off) && self.tr_epoch == self.tr_rec_ep && !t.steps.iter().any(|s| s.off as u64 == off) && (self.bus.{sp}_code_stamp(t.page) == t.stamp || self.bus.{sp}_code_fresh(t.page, t.mask, t.stamp))
            }}
            None => false,
        }};
        if follows {{
            if let Some(t) = self.tr_rec.as_mut() {{
                t.stamp = self.bus.{sp}_code_stamp(t.page);
            }}
        }}
        if !follows {{
            self.trace_close();
            if let Some(page) = self.bus.{sp}_code_page(start) {{
                let stamp = self.bus.{sp}_code_stamp(page);
                self.tr_rec = Some(Trace {{ phys: start, pc: self.r.{pc0} as u64, key: self.tkey(), page, stamp, mask: 0, ok_ep: self.tr_epoch, links: [TLink::default(); 2], {jit_init}steps: Vec::new() }});
                self.tr_rec_ep = self.tr_epoch;
            }}
        }}
        self.tr_phys = start;
        self.tr_rec_now = self.tr_rec.is_some();
    }}
    /// The run being recorded, kept.
    fn trace_close(&mut self) {{
        if let Some(mut t) = self.tr_rec.take() {{
            if !t.steps.is_empty() {{
                let slot = Self::tr_slot(t.phys);
                if (self.tr_on && self.tr_cur as usize == slot) || self.tr_prev as usize == slot {{
                    self.tr_on = false;
                    self.tr_prev = u32::MAX;
                }}
                t.ok_ep = u64::MAX;
                self.traces[slot] = Some(t);
            }}
        }}
    }}
    /// A decoded instruction into the run being recorded: when its bytes lay
    /// in the window it was decoded through, in the run's page.
    #[inline(never)]
    fn trace_record(&mut self, i: Instr) {{
        self.tr_rec_now = false;
        let n = self.fetched;
        let ok = self.win_from == 1 && n > 0 && (n as u32) <= self.win_len && (self.tr_phys + n as u64 - 1) >> 12 == self.tr_phys >> 12;
        let s = TStep {{ instr: i, off: 0, len: n, known: self.known_opcode, prefix_cycles: self.prefix_cycles, decode_at: self.decode_at, opcodes: {opcodes}, {save}}};
        let page = match self.tr_rec.as_mut() {{
            Some(t) if ok => {{
                t.steps.push(TStep {{ off: (self.tr_phys - t.phys) as u16, ..s }});
                let (b0, b1) = ((self.tr_phys & 0xFFF) >> 7, ((self.tr_phys & 0xFFF) + n as u64 - 1) >> 7);
                t.mask |= (u32::MAX >> (31 - b1)) & (u32::MAX << b0);
                t.page
            }}
            _ => {{
                self.trace_close();
                return;
            }}
        }};
        // Its bytes hold code now: writes to them count.
        self.bus.{sp}_code_mark(page, self.tr_phys, n as u32);
    }}
    /// Forget the runs (after code memory or state was changed other than by
    /// the description).
    pub fn trace_clear(&mut self) {{
        for t in self.traces.iter_mut() {{ *t = None; }}
        self.tr_rec = None;
        self.tr_on = false;
        self.tr_prev = u32::MAX;
        self.tr_epoch = self.tr_epoch.wrapping_add(1);
        self.tr_aepoch = self.tr_aepoch.wrapping_add(1);
        self.tr_kepoch = self.tr_kepoch.wrapping_add(1);
{jit_clear}    }}
    /// Runs a decoded instruction.
    fn run_instr(&mut self, i: Instr) -> Result<(), Fault> {{
        match i {{
{runs}        }}
    }}
"#
        );
        if self.jit {
            let _ = write!(
                self.out,
                r#"    /// A compiled step's `trace_check`, from what it was compiled with.
    #[inline(always)]
    fn jit_check(&mut self, j: &JitStep) -> bool {{
        if self.tr_epoch != self.tr_ep0 || self.r.{pc0} as u64 != j.pc {{
            self.tr_on = false;
            self.tr_prev = j.slot;
            return false;
        }}
        let w = self.bus.{sp}_code_writes();
        if w != self.tr_w {{
            if !self.tr_fresh(j.slot as usize, j.page) {{
                self.tr_on = false;
                self.tr_prev = u32::MAX;
                return false;
            }}
            self.tr_w = w;
        }}
        true
    }}
    /// A run just ended: the run its link leads to, when that is compiled,
    /// made the current run (as `trace_follow` would, without its first
    /// step); its code and length.
    fn jit_follow(&mut self) -> Option<(usize, u64)> {{
        let prev = self.tr_prev as usize;
        let pc = self.r.{pc0} as u64;
        let li = self.traces.get(prev)?.as_ref()?.links.iter().position(|l| l.pc == pc && l.ep == self.tr_aepoch)?;
        let l = self.traces[prev].as_ref()?.links[li];
        let kep = self.tr_kepoch;
        let key_known = l.kep == kep;
        let key = if key_known {{ None }} else {{ Some(self.tkey()) }};
        let t = self.traces[l.slot as usize].as_ref()?;
        if t.phys != l.phys || t.pc != pc || key.is_some_and(|k| t.key != k) {{
            return None;
        }}
        let code = t.code.addr()?;
        let len = t.steps.len() as u64;
        // The run before checked its code page at `tr_w`: the same page, at
        // the same stamp, needs no check again, its blocks among those the
        // check covered (a run may hold by its blocks alone: `tr_fresh`).
        let same = self.traces[prev].as_ref().is_some_and(|p| p.page == t.page && p.stamp == t.stamp && t.mask & !p.mask == 0);
        if !key_known {{
            if let Some(p) = self.traces[prev].as_mut() {{ p.links[li].kep = kep; }}
        }}
        self.tr_on = true;
        self.tr_cur = l.slot;
        self.tr_idx = 0;
        self.tr_ep0 = self.tr_epoch;
        if !same {{
            self.tr_w = u64::MAX;
        }}
        Some((code, len))
    }}
    /// A compiled step's `trace_restore`.
    #[inline(always)]
    fn jit_restore(&mut self, j: &JitStep) {{
        let s = &j.step;
        self.fetched = s.len;
        self.known_opcode = s.known;
        self.prefix_cycles = s.prefix_cycles;
        if B::SCHEDULE {{ self.decode_at = s.decode_at; }}
        let opcodes = s.opcodes;
{restore}        self.tr_idx += 1;
{opcode_replay}
        if j.last {{
            self.tr_on = false;
            self.tr_prev = j.slot;
        }}
    }}
"#
            );
        }
        Ok(())
    }
}
