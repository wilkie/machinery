//! The `bulk` check of a `repeat` clause, when the description is built: is
//! the instruction's element (its semantics, once) a stream the cores may
//! run as a block? And, for a fill, copy, scan, compare or load, the kernel
//! that does.
//!
//! The element is evaluated abstractly, every fn and unit it calls inlined.
//! A value is a constant; an invariant (the same every element: latches,
//! operands, state the element never writes); a register as the element
//! found it plus an invariant (a pointer that moves the same way each
//! time); or an element value (what it loaded, and anything computed from
//! that and invariants alone).
//!
//! A branch on an invariant splits the evaluation: each path is a case the
//! runtime would tell apart (real or protected mode, paging, the address
//! size). A condition is keyed by what it tests, so `CR0.PG == 0` gets the
//! same answer everywhere in a path. A branch on an element value where one
//! arm always raises is a fault check (a segment limit): the path takes the
//! other. Any other branch on an element value ends the path: not a stream.
//!
//! A path is a stream when its pointers move by invariants, nothing else
//! but its results changes, and its accesses are one of: a fill (one write
//! of an invariant), a copy (a read, and a write of it), a scan (one read,
//! results from it), a compare (two reads), a load (one read into a
//! register), a port-in (a port read written to memory) or a port-out (a
//! memory read written to a port). `bulk` claims at least one path is.
//!
//! The kernel (`bulk_<name>`, for the JIT core's `repeat: bulk(n)`) runs the
//! element's own code dry (`probe_<name>`: its accesses noted, not made) at
//! a few elements: how the registers move, where it reads and writes, and
//! whether it faults. The paths are told apart where the code tells them
//! apart, by running it; what the dry code may not run past is where a path
//! stops being one the kernel can take (a branch on where the element is
//! that is not a fault check holding on an interval of elements, anything
//! not a stream): there it gives up. So every path the dry code finishes is
//! one the kernel takes, whose fault checks, holding at the first and the
//! last element, hold between; or it raised. When some path is neither (and
//! is not stopped), the instruction has no kernel.
//!
//! A scan's or compare's `while` condition (REPE, REPNE) depends on what
//! the element loaded: the kernel tests it, element by element, as its
//! closed form (`bulk_<name>_cont`): the condition with the element's
//! results in place, in terms of the loaded values and invariants, a branch
//! on an invariant kept a branch. It stops before the element that ends the
//! repeat, which the step runs: the results (the flags) are each element's
//! own, from what it loaded, so that element's are the instruction's.

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::rc::Rc;

use machine_sema::builtins::Builtin;
use machine_sema::hir::{self, ArmBody, Block, Expr, ExprKind as H, LetTarget, LocalId, Pat, StmtKind as S};
use machine_sema::program::DefId;
use machine_sema::ty::Ty;
use machine_syntax::ast::{BinaryOp, UnaryOp};
use machine_syntax::span::Span;

use super::{ident, Gen, GenError, R};

/// A place in a register: the register, its first bit and width.
type Loc = (DefId, u32, u32);

/// A value's closed form, where it has one: a description expression over
/// the state the element starts in and the values it loaded (the locals
/// `ld<n>`). It is how the value is computed, not part of what it is: any
/// two compare equal.
#[derive(Clone)]
struct X(Option<Rc<Expr>>);

impl PartialEq for X {
    fn eq(&self, _: &X) -> bool {
        true
    }
}

impl std::fmt::Debug for X {
    fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}

const NO: X = X(None);

/// Where an element-dependent condition holds, over a run of elements whose
/// pointers move without wrapping: on an interval of them, the rest being
/// one too (a comparison of a pointer with an invariant); on an interval (a
/// conjunction of those, an equality); outside one (a disjunction, an
/// inequality); or anywhere.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Conv {
    Half,
    Convex,
    Concave,
    Other,
}

impl Conv {
    fn not(self) -> Conv {
        match self {
            Conv::Convex => Conv::Concave,
            Conv::Concave => Conv::Convex,
            c => c,
        }
    }
    fn interval(self) -> bool {
        matches!(self, Conv::Half | Conv::Convex)
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Sv {
    K(i128),
    /// An invariant, named by where it came from; a bool one with its
    /// polarity (`!c` is `c` negated: the same choice).
    Inv(String, X),
    InvB(String, bool, X),
    /// A register as the element found it, plus an invariant (or constant);
    /// and whether the value may have wrapped (overflowed its type); its
    /// closed form (the register as it is: in a closed form only where the
    /// value is proven not to matter).
    Aff(Loc, Box<Sv>, bool, X),
    /// A condition on where the element is; the comparison, when it is one;
    /// the expression that made it.
    Cond(Conv, Option<Box<(BinaryOp, Sv, Sv)>>, usize),
    /// The nth access's value as it was loaded, and a value computed from
    /// loads (the first of them) and invariants.
    Load(u32, X),
    Elem(u32, X),
    /// Anything else, and the expression that made it (0: none known).
    Dep(usize),
    Agg(Vec<Sv>),
    Unit,
}

impl Sv {
    fn invariant(&self) -> bool {
        matches!(self, Sv::K(_) | Sv::Inv(..) | Sv::InvB(..))
    }
    /// What the element loaded, and what it computed from that alone.
    fn data(&self) -> bool {
        matches!(self, Sv::Load(..) | Sv::Elem(..))
    }
    fn load(&self) -> Option<u32> {
        match self {
            Sv::Load(n, _) | Sv::Elem(n, _) => Some(*n),
            _ => None,
        }
    }
    fn key(&self) -> String {
        match self {
            Sv::K(c) => c.to_string(),
            Sv::Inv(k, _) => k.clone(),
            Sv::InvB(k, neg, _) => {
                if *neg {
                    format!("!{k}")
                } else {
                    k.clone()
                }
            }
            other => format!("{other:?}"),
        }
    }
    fn x(&self) -> Option<&Rc<Expr>> {
        match self {
            Sv::Inv(_, x) | Sv::InvB(_, _, x) | Sv::Load(_, x) | Sv::Elem(_, x) | Sv::Aff(.., x) => x.0.as_ref(),
            _ => None,
        }
    }
    fn with_x(mut self, e: Option<Rc<Expr>>) -> Sv {
        if let (Sv::Inv(_, x) | Sv::InvB(_, _, x) | Sv::Load(_, x) | Sv::Elem(_, x) | Sv::Aff(.., x), Some(e)) = (&mut self, e) {
            x.0 = Some(e);
        }
        self
    }
    fn wraps(&self) -> bool {
        matches!(self, Sv::Aff(_, _, true, _))
    }
}

/// How a path left a block.
enum Flow {
    Next,
    Ret(Sv),
    Raise,
    Break,
}

#[derive(Default)]
struct Path {
    /// Each invariant condition's answer, by key (arm index for a match).
    choices: HashMap<String, u32>,
    /// The conditions met this run with no answer given: answered 0, to be
    /// tried otherwise.
    fresh: Vec<(String, u32)>,
    writes: Vec<(Loc, Sv)>,
    /// Accesses: the space, the address, the bits (a read: the value's
    /// type; a write: the value).
    mem_reads: Vec<(DefId, Sv, u32, Ty)>,
    mem_writes: Vec<(DefId, Sv, Sv, u32)>,
    loads: u32,
    /// The expression each load is.
    load_sites: Vec<usize>,
    fail: Option<String>,
    /// It raised (the dry code raises too).
    raised: bool,
    /// It reached a statement the dry code does not run past.
    stopped: bool,
    /// Outside the element (the `next` clause, the timing entry): why the
    /// path is not one a kernel takes.
    kfail: Option<String>,
}

/// A stream's kind (`bulk`'s claim).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Shape {
    Fill,
    Copy,
    Scan,
    Compare,
    Load,
    PortIn,
    PortOut,
}

impl Kernel {
    pub(crate) fn fns(&self) -> &[DefId] {
        &self.fns
    }
    pub(crate) fn stops(&self) -> &[usize] {
        &self.stops
    }
}

/// An instruction's kernel: how its paths run as a block.
#[derive(Clone, Debug)]
pub(crate) struct Kernel {
    shape: Shape,
    /// The bits of each access.
    bits: u32,
    /// The registers that move: each whole register, and the width (from
    /// bit 0) it moves in on some path; the count last (counted down by
    /// one, the instruction repeating while it is not zero).
    moving: Vec<(DefId, u32)>,
    /// The registers the element may write, on any path, before it ends.
    written: Vec<DefId>,
    /// The fns the element may call.
    fns: Vec<DefId>,
    /// The statements and expressions its dry code stops at.
    stops: Vec<usize>,
    /// The `while` condition's closed form (Rust, over `l<n>_ld<n+1>`, the
    /// values loaded), and those values' types.
    cond: Option<String>,
    loads: Vec<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Element,
    /// The `next` clause and the timing entry: for the kernel only. In the
    /// timing entry `restarted()` holds (the elements after the first).
    Side { steady: bool },
    /// The `while` condition: for the kernel only; a branch on an invariant
    /// is kept a branch (both arms, in its closed form).
    Cond,
}

struct Eval<'g, 'a> {
    g: &'g Gen<'a>,
    /// The locations the element (and `next`) may write, anywhere (a
    /// register read that none of them overlaps is an invariant).
    written: Vec<Loc>,
    mem: DefId,
    p: Path,
    depth: u32,
    /// The fns being evaluated (for saying where a path stops).
    at: Vec<String>,
    phase: Phase,
    /// The statement being evaluated, and those the dry code does not run
    /// past.
    site: usize,
    stops: HashSet<usize>,
    /// Inside a unit (inlined); and whether it did what the evaluation does
    /// not follow (it is then a call whose value is computed from its
    /// arguments).
    in_unit: u32,
    unit_bad: bool,
}

const MAX_PATHS: usize = 4096;

fn site(st: &hir::Stmt) -> usize {
    st as *const hir::Stmt as usize
}

impl Gen<'_> {
    /// Whether instruction `i`'s element is a stream on some path (`bulk`),
    /// and which; an error naming why not. With it, its kernel, or why it
    /// has none.
    pub(crate) fn bulk_plan(&self, i: &hir::Instruction) -> R<(Shape, Result<Kernel, String>)> {
        let Some(sem) = &i.semantics else {
            return Err(GenError::new(format!("`{}`: `bulk` with no semantics", i.name)));
        };
        let Some(rep) = &i.repeat else {
            return Err(GenError::new(format!("`{}`: `bulk` with no repeat", i.name)));
        };
        let mem = self.isa.istream.as_ref().map(|s| s.space).ok_or_else(|| GenError::new("`bulk` needs an istream"))?;
        let mut side: Vec<&Expr> = vec![&rep.next];
        side.extend(rep.cond.iter());
        let (written, fns) = self.bulk_written(sem, &side);
        let mut todo: Vec<HashMap<String, u32>> = vec![HashMap::new()];
        let mut reasons: Vec<String> = Vec::new();
        let mut shape = None;
        let mut stops: HashSet<usize> = HashSet::new();
        // What the kernel's paths share, and why there is none.
        let mut kern: Option<Kernel> = None;
        let mut why: Option<String> = None;
        let mut regs_written: Vec<DefId> = Vec::new();
        let mut runs = 0;
        let trace = std::env::var_os("MACHINE_BULK").is_some();
        while let Some(choices) = todo.pop() {
            runs += 1;
            if runs > MAX_PATHS {
                why.get_or_insert_with(|| "too many paths to follow".into());
                break;
            }
            let mut e = Eval {
                g: self,
                written: written.clone(),
                mem,
                p: Path { choices: choices.clone(), ..Default::default() },
                depth: 0,
                at: vec![i.name.clone()],
                phase: Phase::Side { steady: true },
                site: 0,
                stops: std::mem::take(&mut stops),
                in_unit: 0,
                unit_bad: false,
            };
            // The timing entry as the element starts, the element, and what
            // the repeat makes of it.
            let t0 = i.timing.as_ref().map(|t| e.expr(&i.timing_locals, &mut HashMap::new(), t));
            e.phase = Phase::Element;
            let mut env: HashMap<LocalId, Sv> = HashMap::new();
            let flow = e.block(&i.locals, &mut env, sem);
            if matches!(flow, Flow::Raise) && e.p.fail.is_none() {
                e.p.fail = Some("it raises".into());
                e.p.raised = true;
            }
            let class = e.classify();
            let elem_writes = e.p.writes.clone();
            let accesses = (e.p.mem_reads.len(), e.p.mem_writes.len());
            e.phase = Phase::Side { steady: false };
            let nx = e.expr(&rep.locals, &mut HashMap::new(), &rep.next);
            e.phase = Phase::Cond;
            let cv = rep.cond.as_ref().map(|c| e.expr(&rep.locals, &mut HashMap::new(), c));
            e.phase = Phase::Side { steady: true };
            let t1 = i.timing.as_ref().map(|t| e.expr(&i.timing_locals, &mut HashMap::new(), t));
            // Each condition met for the first time, answered otherwise: with
            // the answers given before it (those after it may not be met).
            let mut before = choices.clone();
            for (key, arity) in std::mem::take(&mut e.p.fresh) {
                for alt in 1..arity {
                    let mut c = before.clone();
                    c.insert(key.clone(), alt);
                    todo.push(c);
                }
                before.insert(key, 0);
            }
            if trace {
                eprintln!("bulk path {}: writes {:?} reads {:?} stores {:?} fail {:?} raised {} stopped {}", i.name, e.p.writes, e.p.mem_reads, e.p.mem_writes, e.p.fail, e.p.raised, e.p.stopped);
            }
            for (l, _) in &elem_writes {
                if !regs_written.contains(&l.0) {
                    regs_written.push(l.0);
                }
            }
            match &class {
                Ok(s) => {
                    shape.get_or_insert(*s);
                }
                Err(w) => {
                    if !reasons.contains(w) {
                        reasons.push(w.clone());
                    }
                }
            }
            // The dry code stops on this path, or raises: the kernel need
            // not take it.
            if !(e.p.stopped || e.p.raised) {
                let k = match class {
                    Ok(s @ (Shape::Fill | Shape::Copy | Shape::Scan | Shape::Compare | Shape::Load)) => {
                        let timing_ok = [&t0, &t1].iter().all(|t| t.as_ref().is_none_or(Sv::invariant));
                        if !timing_ok {
                            Err("its time depends on the element".to_string())
                        } else if (e.p.mem_reads.len(), e.p.mem_writes.len()) != accesses {
                            Err("`next`, `while` or the timing entry reach memory".to_string())
                        } else {
                            e.kernel(s, &elem_writes, &nx, cv.as_ref())
                        }
                    }
                    Ok(s) => Err(format!("a {s:?} path")),
                    Err(w) => Err(format!("a path the dry code finishes that is not a stream ({w})")),
                };
                match (k, &mut kern) {
                    (Ok(k), None) => kern = Some(k),
                    (Ok(k), Some(have)) => {
                        if k.shape != have.shape || k.bits != have.bits || k.loads != have.loads {
                            why.get_or_insert_with(|| "paths of different kinds or sizes".into());
                        }
                        if k.cond != have.cond {
                            why.get_or_insert_with(|| "paths whose `while` conditions differ".into());
                        }
                        let (mut a, b) = (std::mem::take(&mut have.moving), k.moving);
                        // The count stays last.
                        let (ac, bc) = (a.pop().unwrap(), b.last().copied().unwrap());
                        if ac.0 != bc.0 {
                            why.get_or_insert_with(|| "counts in different registers".into());
                        }
                        for (d, w) in &b[..b.len() - 1] {
                            match a.iter_mut().find(|x| x.0 == *d) {
                                Some(x) => x.1 = x.1.min(*w),
                                None => a.push((*d, *w)),
                            }
                        }
                        a.push((ac.0, ac.1.min(bc.1)));
                        have.moving = a;
                    }
                    (Err(w), _) => {
                        why.get_or_insert(w);
                    }
                }
            }
            stops = std::mem::take(&mut e.stops);
        }
        let Some(s) = shape else {
            return Err(GenError::new(format!(
                "`{}`: `repeat ... bulk`, but its element is not a fill, copy, scan, compare, load or port stream on any path ({})",
                i.name,
                reasons.join("; ")
            )));
        };
        let k = match (why, kern) {
            (Some(w), _) => Err(w),
            (None, None) => Err("no path the dry code finishes".into()),
            (None, Some(mut k)) => {
                // A register the element moves on one path and not another
                // (or the count): it is saved round the dry code all the same.
                if k.moving.iter().map(|m| m.0).collect::<HashSet<_>>().len() != k.moving.len() {
                    Err("a register both moved and counted".into())
                } else {
                    for (d, _) in &k.moving {
                        if !regs_written.contains(d) {
                            regs_written.push(*d);
                        }
                    }
                    k.written = regs_written;
                    k.fns = fns;
                    k.stops = stops.into_iter().collect();
                    Ok(k)
                }
            }
        };
        Ok((s, k))
    }

    /// The register locations the element may write (its semantics, the
    /// exprs in `also`, and the fns they call; an operand kind's selected
    /// registers all of them), and the fns it may call.
    fn bulk_written(&self, b: &Block, also: &[&Expr]) -> (Vec<Loc>, Vec<DefId>) {
        let out = std::cell::RefCell::new(Vec::new());
        let mut seen: HashSet<DefId> = HashSet::new();
        let mut fns: Vec<DefId> = Vec::new();
        let stack = std::cell::RefCell::new(Vec::<DefId>::new());
        let mut blocks: Vec<&Block> = vec![b];
        let place_locs = |p: &Expr, out: &mut Vec<Loc>| {
            let mut e = p;
            loop {
                match &e.kind {
                    H::Register(_) | H::RegField(..) | H::RegView(..) => {
                        out.extend(self.bulk_locs(e));
                        return;
                    }
                    H::KindSelect(k, _) => {
                        for r in &self.kinds[k].rules {
                            if r.kind == hir::RuleKind::Select {
                                let mut on = |x: &Expr| out.extend(self.bulk_locs(x));
                                match &r.body {
                                    ArmBody::Expr(x) => hir::visit_expr(x, &mut |_| {}, &mut on),
                                    ArmBody::Block(bl) => hir::visit_block(bl, &mut |_| {}, &mut on),
                                }
                            }
                        }
                        return;
                    }
                    H::Field(b, _) | H::Index(b, _) | H::Slice(b, ..) | H::TupleField(b, _) => e = b,
                    _ => return,
                }
            }
        };
        let mut side_fns: Vec<DefId> = Vec::new();
        for x in also {
            hir::visit_expr(
                x,
                &mut |_| {},
                &mut |x| {
                    if let H::FnCall(g, _) = &x.kind {
                        side_fns.push(*g);
                    }
                },
            );
        }
        // The element's fns first (the dry code calls them), then `next`'s.
        for phase in 0..2 {
            if phase == 1 {
                stack.borrow_mut().extend(side_fns.iter().copied());
            }
            loop {
                let Some(blk) = blocks.pop() else {
                    let Some(f) = stack.borrow_mut().pop() else { break };
                    if seen.insert(f) {
                        if let Some(fb) = self.fns.get(&f) {
                            if !fb.external {
                                if phase == 0 {
                                    fns.push(f);
                                }
                                blocks.push(&fb.body);
                            }
                        }
                    }
                    continue;
                };
                hir::visit_block(
                    blk,
                    &mut |st| {
                        if let S::Assign(p, _) = &st.kind {
                            place_locs(p, &mut out.borrow_mut());
                        }
                    },
                    &mut |x| {
                        if let H::FnCall(g, _) = &x.kind {
                            stack.borrow_mut().push(*g);
                        }
                    },
                );
            }
        }
        (out.into_inner(), fns)
    }

    /// The locations a register place covers (a view: each field it maps).
    fn bulk_locs(&self, e: &Expr) -> Vec<Loc> {
        match &e.kind {
            H::Register(d) => match self.regs.get(d) {
                Some((_, r)) => vec![(*d, 0, ty_bits(&r.ty))],
                None => Vec::new(),
            },
            H::RegField(d, i) => match self.regs.get(d) {
                Some((_, r)) => {
                    let f = &r.fields[*i as usize];
                    vec![(*d, f.offset, ty_bits(&f.ty))]
                }
                None => Vec::new(),
            },
            H::RegView(d, v) => match self.regs.get(d) {
                Some((_, r)) => r.views[*v as usize]
                    .mapping
                    .iter()
                    .map(|i| {
                        let f = &r.fields[*i as usize];
                        (*d, f.offset, ty_bits(&f.ty))
                    })
                    .collect(),
                None => Vec::new(),
            },
            _ => Vec::new(),
        }
    }

    /// The locals standing for the values an element loads (`ld1`, `ld2`,
    /// ...), of these types.
    fn bulk_load_locals(tys: &[Ty]) -> Vec<hir::Local> {
        tys.iter().enumerate().map(|(n, t)| hir::Local { name: format!("ld{}", n + 1), ty: t.clone(), span: Span::default(), mutable: false }).collect()
    }

    /// Instruction `i`'s kernel (`bulk_<name>`) and its dry code
    /// (`probe_<name>`), for the JIT core: the elements after the current
    /// one, run as a block. (Every kernel's stops are in `bulk_stops`: the
    /// dry fns are shared.)
    pub(crate) fn emit_bulk_kernel(&self, i: &hir::Instruction, k: &Kernel, out: &mut String) -> R<()> {
        let n = ident(&i.name);
        let Some(sem) = &i.semantics else { return Ok(()) };
        // The dry element.
        let cx = super::Cx { locals: &i.locals, fallible: true, recv: "self", ops: Some(i), comp: None, opt: false };
        self.probing.set(true);
        let body = self.block_stmts(&cx, sem);
        self.probing.set(false);
        let body = body?;
        let ops = Self::ops_struct_name(i);
        let _ = write!(out, "    /// `{name}`'s element, dry: its accesses noted (`probe_access`), not made.\n    #[allow(unused_variables, unused_parens, unreachable_code, clippy::all)]\n    fn probe_{n}(&mut self, ops: {ops}) -> Result<(), Fault> {{\n{body}Ok(())\n}}\n", name = i.name);
        let reg = |d: &DefId| -> String { format!("self.r.{}", ident(&self.regs[d].1.name)) };
        let nm = k.moving.len();
        let q = nm - 1;
        let r0: Vec<String> = k.moving.iter().map(|(d, _)| format!("{} as u64", reg(d))).collect();
        let widths: Vec<String> = k.moving.iter().map(|(_, w)| w.to_string()).collect();
        let full: Vec<String> = k.moving.iter().map(|(d, _)| ty_bits(&self.regs[d].1.ty).to_string()).collect();
        let mut save = String::new();
        let mut restore = String::new();
        for (x, d) in k.written.iter().enumerate() {
            let _ = writeln!(save, "        let s{x} = {}.clone();", reg(d));
            let _ = writeln!(restore, "        {} = s{x};", reg(d));
        }
        let mut moved = Vec::new();
        let mut set = String::new();
        let mut done = String::new();
        for (x, (d, _)) in k.moving.iter().enumerate() {
            let rty = self.rty(&self.regs[d].1.ty)?;
            moved.push(format!("{} as u64", reg(d)));
            let _ = writeln!(set, "        {} = ((r0[{x}] as i128 + j * s[{x}]) as u64) as {rty};", reg(d));
            let place = Expr { kind: H::Register(*d), ty: self.regs[d].1.ty.clone(), span: sem.span };
            let rhs = format!("((r0[{x}] as i128 + made as i128 * s[{x}]) as u64) as {rty}");
            let cx0 = super::Cx { locals: &[], fallible: true, recv: "self", ops: None, comp: None, opt: false };
            let _ = writeln!(done, "        {}", self.assign(&cx0, &place, &rhs)?);
        }
        let mi = self.isa.spaces.iter().position(|s| Some(s.def) == self.isa.istream.as_ref().map(|i| i.space)).unwrap_or(0) as u8;
        // The accesses, as the element makes them: whether each writes.
        let writes: &[bool] = match k.shape {
            Shape::Fill => &[true],
            Shape::Copy => &[false, true],
            Shape::Compare => &[false, false],
            _ => &[false],
        };
        let accs = writes.len();
        let expect: Vec<String> = writes.iter().map(|w| format!("({}u8, {})", (mi << 1) | *w as u8, k.bits)).collect();
        let wflags: Vec<String> = writes.iter().map(|w| w.to_string()).collect();
        let value = if k.shape == Shape::Fill { "self.probe_acc[0].3" } else { "0" };
        let bytes = k.bits / 8;
        let mv = match k.shape {
            Shape::Fill => "self.bus.direct_poke(at(0), n, v);".to_string(),
            Shape::Copy => "let x = self.bus.direct_peek(at(0), n).unwrap_or(0); self.bus.direct_poke(at(1), n, x);".to_string(),
            _ => String::new(),
        };
        // The `while` condition, tested on what each element loads.
        let (cont_fn, cont) = match &k.cond {
            Some(c) => {
                let params: Vec<String> = k.loads.iter().enumerate().map(|(x, t)| format!("l{x}_ld{}: {t}", x + 1)).collect();
                let peeks: String = k.loads.iter().enumerate().map(|(x, t)| format!("let ld{x} = self.bus.direct_peek(at({x}), n).unwrap_or(0) as {t}; ")).collect();
                let args: Vec<String> = (0..k.loads.len()).map(|x| format!("ld{x}")).collect();
                (
                    format!("    /// `{name}`'s `while` condition, on the values an element loads.\n    #[allow(unused_variables, unused_parens, clippy::all)]\n    fn bulk_{n}_cont(&mut self, {}) -> Result<bool, Fault> {{\n        Ok({c})\n    }}\n", params.join(", "), name = i.name),
                    format!("{peeks}if !self.bulk_{n}_cont({}).unwrap_or(false) {{ break 'run; }}", args.join(", ")),
                )
            }
            None => (String::new(), String::new()),
        };
        let mask = |w: &str| format!("(if {w} >= 64 {{ u64::MAX }} else {{ (1u64 << {w}) - 1 }})");
        let _ = write!(
            out,
            "{cont_fn}    /// `{name}`'s element, dry, as the machine is: the addresses it reaches,
    /// the value it writes, and the moving registers after it. None when it
    /// faults or is not a {shape:?} the kernel takes.
    #[allow(clippy::all)]
    fn bulk_{n}_dry(&mut self) -> Option<([u64; 2], u128, [u64; {nm}])> {{
{save}        self.probe_bad = false;
        self.probe_n = 0;
        let r = self.probe_{n}({ops} {{}});
        let moved: [u64; {nm}] = [{moved}];
{restore}        let expect: [(u8, u32); {accs}] = [{expect}];
        if r.is_err() || self.probe_bad || self.probe_n as usize != {accs} {{ return None; }}
        let mut a = [0u64; 2];
        for x in 0..{accs} {{
            if (self.probe_acc[x].0, self.probe_acc[x].1) != expect[x] {{ return None; }}
            a[x] = self.probe_acc[x].2;
        }}
        Some((a, {value}, moved))
    }}
    /// The dry element `j` elements from now (the moving registers `r0`,
    /// moving `s` each).
    #[allow(clippy::all)]
    fn bulk_{n}_at(&mut self, r0: &[u64; {nm}], s: &[i128; {nm}], j: i128) -> Option<([u64; 2], u128, [u64; {nm}])> {{
{set}        let x = self.bulk_{n}_dry();
        let j = 0i128;
{set}        x
    }}
    /// `{name}`'s elements after this one as a block (`repeat ... bulk`):
    /// up to `max` of them, in whole periods of `per` elements, each period's
    /// accesses costing what the `per` before them cost; the last element
    /// (and the one its `while` condition ends at) left for the step. The
    /// elements made (0: none here).
    #[allow(unused_mut, unused_variables, clippy::all)]
    pub fn bulk_{n}(&mut self, max: u64, per: u64) -> u64 {{
        if per == 0 || per > 64 {{ return 0; }}
        let r0: [u64; {nm}] = [{r0}];
        // The width each moves in on some path, from bit 0, and its own.
        let w: [u32; {nm}] = [{widths}];
        let fw: [u32; {nm}] = [{full}];
        let wr: [bool; {accs}] = [{wflags}];
        let c = r0[{q}] & {cmask};
        let mut m = max.min(c.saturating_sub(1));
        m -= m % per;
        if m == 0 {{ return 0; }}
        // How the element moves them; and whether it reaches direct memory
        // at all.
        let Some((a0, v, after)) = self.bulk_{n}_dry() else {{ return 0 }};
        let n: u64 = {bytes};
        for q in 0..{accs} {{
            if self.bulk_key(a0[q], n, wr[q]).is_none() {{ return 0; }}
        }}
        let mut s = [0i128; {nm}];
        for q in 0..{nm} {{
            let mw = {wmask};
            let d = if q == {q} {{ mw }} else {{ after[q].wrapping_sub(r0[q]) & mw }};
            s[q] = if (d >> (w[q] - 1)) & 1 == 1 {{ d as i128 - (mw as i128 + 1) }} else {{ d as i128 }};
            // From the period measured to the elements made, none carries
            // out of the width it moves in.
            let lo = (r0[q] & mw) as i128 - per as i128 * s[q];
            let hi = (r0[q] & mw) as i128 + m as i128 * s[q];
            if lo < 0 || hi < 0 || lo > mw as i128 || hi > mw as i128 || fw[q] > 64 {{ return 0; }}
        }}
        let Some((am, _, _)) = self.bulk_{n}_at(&r0, &s, -(per as i128)) else {{ return 0 }};
        let Some((a1, _, _)) = self.bulk_{n}_at(&r0, &s, 1) else {{ return 0 }};
        let Some((al, _, _)) = self.bulk_{n}_at(&r0, &s, m as i128 - 1) else {{ return 0 }};
        // The addresses move by the same each element.
        let mut d = [0i128; 2];
        for q in 0..{accs} {{
            d[q] = a1[q] as i128 - a0[q] as i128;
            if am[q] as i128 != a0[q] as i128 - per as i128 * d[q] || al[q] as i128 != a0[q] as i128 + (m as i128 - 1) * d[q] {{ return 0; }}
        }}
        let at = |q: usize, j: i128| (a0[q] as i128 + j * d[q]) as u64;
        // What each access of the period measured cost.
        let mut refk = [(0u32, 0u128); 128];
        for jj in 0..per as usize {{
            let j = jj as i128 - per as i128;
            for q in 0..{accs} {{
                match self.bulk_key(at(q, j), n, wr[q]) {{ Some(x) => refk[jj * 2 + q] = x, None => return 0 }}
            }}
        }}
        let mut made: u64 = 0;
        'run: while made < m {{
            for jj in 0..per as usize {{
                let j = (made as usize + jj) as i128;
                for q in 0..{accs} {{
                    if self.bulk_key(at(q, j), n, wr[q]) != Some(refk[jj * 2 + q]) {{ break 'run; }}
                }}
            }}
            for jj in 0..per {{
                let j = (made + jj) as i128;
                let at = |q: usize| (a0[q] as i128 + j * d[q]) as u64;
                {cont}
            }}
            for jj in 0..per {{
                let j = (made + jj) as i128;
                let at = |q: usize| (a0[q] as i128 + j * d[q]) as u64;
                {mv}
            }}
            made += per;
        }}
        if made == 0 {{ return 0; }}
{done}        made
    }}
",
            name = i.name,
            shape = k.shape,
            moved = moved.join(", "),
            r0 = r0.join(", "),
            widths = widths.join(", "),
            full = full.join(", "),
            expect = expect.join(", "),
            wflags = wflags.join(", "),
            cmask = mask(&format!("w[{q}]")),
            wmask = mask("w[q]"),
        );
        Ok(())
    }

    /// What the kernels share: `bulk_key` (what an access costs an element)
    /// and the dry code's `probe_access`.
    pub(crate) fn emit_bulk_common(&self, out: &mut String) {
        let split = if self.isa.split_cycles.is_some() { "self.split_cycles(a as u32, n as u32)" } else { "0" };
        let _ = write!(
            out,
            "    /// What an access of `n` bytes at `a` costs a kernel's element (the\n    /// split cycles, the direct memory's time): None where it is not direct.\n    #[allow(dead_code)]\n    fn bulk_key(&mut self, a: u64, n: u64, write: bool) -> Option<(u32, u128)> {{\n        let t = self.bus.direct_cost(a, n, write)?;\n        Some(({split}, t))\n    }}\n    /// The dry code's access (`kind`: the space's index, doubled, plus 1 for\n    /// a write): noted; a read reads 0.\n    #[allow(dead_code)]\n    fn probe_access(&mut self, kind: u8, a: u64, bits: u32, v: u128) -> u128 {{\n        if (self.probe_n as usize) < 4 {{ self.probe_acc[self.probe_n as usize] = (kind, bits, a, v); }}\n        self.probe_n = self.probe_n.saturating_add(1);\n        0\n    }}\n"
        );
    }
}

fn ty_bits(t: &Ty) -> u32 {
    match t {
        Ty::Bool => 1,
        Ty::UInt(n) | Ty::SInt(n) => *n,
        _ => 128,
    }
}

fn overlaps(a: &Loc, b: &Loc) -> bool {
    a.0 == b.0 && a.1 < b.1 + b.2 && b.1 < a.1 + a.2
}

/// Whether a register's value plus `off`, as a `ty`, cannot wrap.
fn fits(l: &Loc, off: &Sv, ty: &Ty) -> bool {
    let Sv::K(c) = off else { return false };
    let (w, signed) = match ty {
        Ty::UInt(n) => (*n, false),
        Ty::SInt(n) => (*n, true),
        _ => return false,
    };
    if l.2 >= 120 || w >= 120 {
        return false;
    }
    let limit: i128 = if signed { 1 << (w - 1) } else { 1 << w };
    *c >= 0 && (1i128 << l.2) - 1 + c < limit
}

/// Where the first of these values that came from something untracked was
/// made (0: none).
fn dep_at(vs: &[&Sv]) -> usize {
    vs.iter()
        .find_map(|v| match v {
            Sv::Dep(s) | Sv::Cond(_, _, s) if *s != 0 => Some(*s),
            _ => None,
        })
        .unwrap_or(0)
}

/// A constant cast to `ty`.
fn cast_k(c: i128, ty: &Ty) -> i128 {
    match ty {
        Ty::Bool => (c != 0) as i128,
        Ty::UInt(n) if *n < 127 => c & ((1i128 << n) - 1),
        Ty::SInt(n) if *n < 127 => (c << (128 - n)) >> (128 - n),
        _ => c,
    }
}

fn rc(kind: H, ty: &Ty, span: Span) -> Rc<Expr> {
    Rc::new(Expr { kind, ty: ty.clone(), span })
}

/// A constant of type `ty`, as the description would write it.
fn konst(c: i128, ty: &Ty, span: Span) -> Option<Expr> {
    let kind = match ty {
        Ty::Bool => H::Bool(c != 0),
        Ty::UInt(_) | Ty::SInt(_) => H::Int(cast_k(c, ty)),
        Ty::Enum(d) => H::Variant(*d, c as u32),
        _ => return None,
    };
    Some(Expr { kind, ty: ty.clone(), span })
}

impl Eval<'_, '_> {
    fn fail(&mut self, why: impl Into<String>) {
        if self.in_unit > 0 {
            self.unit_bad = true;
            return;
        }
        // (A path already ended: what follows is not evaluated.)
        if self.p.fail.is_some() {
            return;
        }
        let why = format!("{} in `{}`", why.into(), self.at.last().map(String::as_str).unwrap_or(""));
        if self.phase == Phase::Element {
            if self.p.fail.is_none() {
                self.p.fail = Some(why);
            }
            self.stop();
        } else if self.p.kfail.is_none() {
            self.p.kfail = Some(why);
        }
    }

    /// Where a value that depends on the element was first made: the dry
    /// code stops there when a branch on it is not one a kernel takes (an
    /// expression the paths that are not stopped do not evaluate).
    fn prov(&self, v: &Sv) -> usize {
        match v {
            Sv::Dep(s) | Sv::Cond(_, _, s) => *s,
            Sv::Load(n, _) | Sv::Elem(n, _) => self.p.load_sites.get(*n as usize - 1).copied().unwrap_or(0),
            _ => 0,
        }
    }

    /// The dry code stops where `v` was made (else at this statement).
    fn stop_on(&mut self, v: &Sv) {
        if self.in_unit > 0 {
            self.unit_bad = true;
            return;
        }
        if self.p.fail.is_some() {
            return;
        }
        let at = self.prov(v);
        if self.phase == Phase::Element && at != 0 {
            self.stops.insert(at);
            self.p.stopped = true;
        } else {
            self.stop();
        }
    }

    /// A path that ends on a branch on `v`.
    fn fail_on(&mut self, why: &str, v: &Sv) {
        if self.in_unit > 0 {
            self.unit_bad = true;
            return;
        }
        if self.p.fail.is_some() {
            return;
        }
        self.stop_on(v);
        if self.phase == Phase::Element {
            if self.p.fail.is_none() {
                self.p.fail = Some(format!("{why} in `{}`", self.at.last().map(String::as_str).unwrap_or("")));
            }
        } else if self.p.kfail.is_none() {
            self.p.kfail = Some(why.to_string());
        }
    }

    /// The dry code does not run past this statement.
    fn stop(&mut self) {
        if self.in_unit > 0 {
            self.unit_bad = true;
            return;
        }
        if self.p.fail.is_some() {
            return;
        }
        if self.phase == Phase::Element && self.site != 0 {
            self.stops.insert(self.site);
            self.p.stopped = true;
        }
    }

    /// The answer to invariant condition `key` (of `arity` answers).
    fn choose(&mut self, key: &str, arity: u32) -> u32 {
        if let Some(c) = self.p.choices.get(key) {
            return *c;
        }
        self.p.choices.insert(key.to_string(), 0);
        self.p.fresh.push((key.to_string(), arity));
        0
    }

    /// Whether a bool value holds: Some for a constant or an invariant (a
    /// choice), None for one that depends on the element.
    fn truth(&mut self, v: &Sv) -> Option<bool> {
        match v {
            Sv::K(c) => Some(*c != 0),
            Sv::InvB(k, neg, _) => Some((self.choose(k, 2) == 0) != *neg),
            Sv::Inv(k, _) => Some(self.choose(k, 2) == 0),
            _ => None,
        }
    }

    fn read_loc(&mut self, loc: Loc, place: &Expr) -> Sv {
        for (l, v) in self.p.writes.iter().rev() {
            if *l == loc {
                return v.clone();
            }
            if overlaps(l, &loc) {
                return Sv::Dep(0);
            }
        }
        if self.written.iter().any(|w| overlaps(w, &loc)) {
            Sv::Aff(loc, Box::new(Sv::K(0)), false, X(Some(Rc::new(place.clone()))))
        } else {
            Sv::Inv(format!("reg{}:{}:{}", loc.0 .0, loc.1, loc.2), X(Some(Rc::new(place.clone()))))
        }
    }

    fn write_loc(&mut self, loc: Loc, v: Sv) {
        if self.in_unit > 0 {
            self.unit_bad = true;
            return;
        }
        self.p.writes.retain(|(l, _)| !overlaps(l, &loc) || *l == loc);
        self.p.writes.retain(|(l, _)| *l != loc);
        self.p.writes.push((loc, v));
    }

    /// A value's closed form, as an expression of type `ty`.
    fn xof(&self, v: &Sv, ty: &Ty, span: Span) -> Option<Expr> {
        match v {
            Sv::K(c) => konst(*c, ty, span),
            Sv::Agg(vs) => match ty {
                Ty::Bundle(d) => {
                    let bd = self.g.bundles.get(d)?;
                    if bd.fields.len() != vs.len() {
                        return None;
                    }
                    let xs: Option<Vec<Expr>> = vs.iter().zip(&bd.fields).map(|(v, f)| self.xof(v, &f.ty, span)).collect();
                    Some(Expr { kind: H::BundleLit(*d, xs?), ty: ty.clone(), span })
                }
                Ty::Tuple(ts) if ts.len() == vs.len() => {
                    let xs: Option<Vec<Expr>> = vs.iter().zip(ts).map(|(v, (_, t))| self.xof(v, t, span)).collect();
                    Some(Expr { kind: H::Tuple(xs?), ty: ty.clone(), span })
                }
                _ => None,
            },
            other => other.x().map(|x| (**x).clone()),
        }
    }

    /// `e`'s closed form: its kind with its children's closed forms.
    fn rebuild(&self, e: &Expr, kids: &[(&Sv, &Expr)], make: impl FnOnce(Vec<Box<Expr>>) -> H) -> Option<Rc<Expr>> {
        let mut xs = Vec::new();
        for (v, k) in kids {
            xs.push(Box::new(self.xof(v, &k.ty, k.span)?));
        }
        Some(rc(make(xs), &e.ty, e.span))
    }

    fn block(&mut self, locals: &[hir::Local], env: &mut HashMap<LocalId, Sv>, b: &Block) -> Flow {
        for st in &b.stmts {
            if self.p.fail.is_some() || (self.in_unit > 0 && self.unit_bad) {
                return Flow::Next;
            }
            match self.stmt(locals, env, st) {
                Flow::Next => {}
                other => return other,
            }
        }
        Flow::Next
    }

    /// Whether a block always raises (a fault check's arm).
    fn diverges(b: &Block) -> bool {
        match b.stmts.last().map(|s| &s.kind) {
            Some(S::Raise(..)) | Some(S::Unreachable(_)) => true,
            Some(S::If(_, t, Some(e))) => Self::diverges(t) && Self::stmt_diverges(e),
            _ => false,
        }
    }
    fn stmt_diverges(s: &hir::Stmt) -> bool {
        match &s.kind {
            S::Raise(..) | S::Unreachable(_) => true,
            S::If(_, t, Some(e)) => Self::diverges(t) && Self::stmt_diverges(e),
            S::Expr(Expr { kind: H::Block(b), .. }) => Self::diverges(b),
            _ => false,
        }
    }

    /// Where a fault check on `cv` holds, the element going on when `cv`
    /// is `!neg`.
    fn held(cv: &Sv, neg: bool) -> Conv {
        match cv {
            Sv::Cond(c, _, _) => {
                if neg {
                    c.not()
                } else {
                    *c
                }
            }
            _ => Conv::Other,
        }
    }

    fn stmt(&mut self, locals: &[hir::Local], env: &mut HashMap<LocalId, Sv>, st: &hir::Stmt) -> Flow {
        if self.in_unit > 0 {
            return self.stmt_in(locals, env, st);
        }
        let outer = self.site;
        self.site = site(st);
        let f = self.stmt_in(locals, env, st);
        self.site = outer;
        f
    }

    fn stmt_in(&mut self, locals: &[hir::Local], env: &mut HashMap<LocalId, Sv>, st: &hir::Stmt) -> Flow {
        match &st.kind {
            S::Let(t, Some(e)) => {
                let v = self.expr(locals, env, e);
                self.bind(env, t, v);
                Flow::Next
            }
            S::Let(_, None) => Flow::Next,
            S::Expr(e) => {
                let _ = self.expr(locals, env, e);
                Flow::Next
            }
            S::If(c, t, e) => {
                let cv = self.expr(locals, env, c);
                match self.truth(&cv) {
                    Some(true) => self.block(locals, env, t),
                    Some(false) => match e {
                        Some(s) => self.stmt(locals, env, s),
                        None => Flow::Next,
                    },
                    None => {
                        let t_raises = Self::diverges(t);
                        let e_raises = e.as_deref().is_some_and(Self::stmt_diverges);
                        // A fault check; the dry code stops at one that need
                        // not hold between the first and the last element.
                        if self.in_unit > 0 {
                            self.unit_bad = true;
                            Flow::Next
                        } else if t_raises && !e_raises {
                            if !Self::held(&cv, true).interval() {
                                self.stop_on(&cv);
                            }
                            match e {
                                Some(s) => self.stmt(locals, env, s),
                                None => Flow::Next,
                            }
                        } else if e_raises && !t_raises {
                            if !Self::held(&cv, false).interval() {
                                self.stop_on(&cv);
                            }
                            self.block(locals, env, t)
                        } else {
                            self.fail_on("a branch on what an element loaded or where it is", &cv);
                            Flow::Next
                        }
                    }
                }
            }
            S::Match(s, arms) => {
                let sv = self.expr(locals, env, s);
                match self.arm_of(&sv, arms) {
                    Some(a) => match &arms[a].body {
                        ArmBody::Expr(x) => {
                            let _ = self.expr(locals, env, x);
                            Flow::Next
                        }
                        ArmBody::Block(b) => self.block(locals, env, b),
                    },
                    None => Flow::Next,
                }
            }
            S::For(l, lo, hi, b) => {
                if hi - lo > 64 {
                    self.fail("a long loop");
                    return Flow::Next;
                }
                for i in *lo..*hi {
                    env.insert(*l, Sv::K(i));
                    match self.block(locals, env, b) {
                        Flow::Next => {}
                        Flow::Break => break,
                        other => return other,
                    }
                }
                Flow::Next
            }
            S::Return(e) => {
                let v = match e {
                    Some(e) => self.expr(locals, env, e),
                    None => Sv::Unit,
                };
                Flow::Ret(v)
            }
            S::Assert(..) | S::Expect(_) | S::Given(_) => Flow::Next,
            S::Unreachable(_) | S::Raise(..) => {
                if self.in_unit > 0 {
                    self.unit_bad = true;
                    return Flow::Next;
                }
                Flow::Raise
            }
            S::Assign(p, v) => {
                let vv = self.expr(locals, env, v);
                self.assign(locals, env, p, vv);
                Flow::Next
            }
            S::SpaceWrite(sp, bits, a, v) => {
                if self.in_unit > 0 {
                    self.unit_bad = true;
                    return Flow::Next;
                }
                let av = self.expr(locals, env, a);
                let vv = self.expr(locals, env, v);
                self.p.mem_writes.push((*sp, av, vv, *bits));
                Flow::Next
            }
            S::Break => Flow::Break,
            S::OperandWrite(..) => {
                self.fail("it writes an operand");
                Flow::Next
            }
            _ => {
                self.fail("a statement a stream does not make");
                Flow::Next
            }
        }
    }

    fn bind(&mut self, env: &mut HashMap<LocalId, Sv>, t: &LetTarget, v: Sv) {
        match (t, v) {
            (LetTarget::Local(l), v) => {
                env.insert(*l, v);
            }
            (LetTarget::Tuple(ts), Sv::Agg(vs)) if ts.len() == vs.len() => {
                for (t, v) in ts.iter().zip(vs) {
                    self.bind(env, t, v);
                }
            }
            (LetTarget::Tuple(ts), v) => {
                for t in ts {
                    self.bind(env, t, v.clone());
                }
            }
            (LetTarget::Discard, _) => {}
        }
    }

    /// The arm a match takes: for an invariant scrutinee, a choice.
    fn arm_of(&mut self, sv: &Sv, arms: &[hir::Arm]) -> Option<usize> {
        let hit = |c: i128, p: &Pat| match p {
            Pat::Wildcard => true,
            Pat::Value(v) => *v == c,
            Pat::Range(a, b) => *a <= c && c <= *b,
            Pat::Variant(v) => *v as i128 == c,
            Pat::Bool(b) => *b as i128 == c,
        };
        match sv {
            Sv::K(c) => arms.iter().position(|a| a.patterns.iter().any(|p| hit(*c, p))),
            Sv::Inv(..) | Sv::InvB(..) => {
                let a = self.choose(&format!("match {}", sv.key()), arms.len() as u32);
                Some(a as usize)
            }
            other => {
                let other = other.clone();
                self.fail_on("a match on what an element loaded", &other);
                None
            }
        }
    }

    fn assign(&mut self, locals: &[hir::Local], env: &mut HashMap<LocalId, Sv>, p: &Expr, v: Sv) {
        match &p.kind {
            H::Local(l) => {
                env.insert(*l, v);
            }
            H::Register(_) | H::RegField(..) => {
                for loc in self.g.bulk_locs(p) {
                    self.write_loc(loc, v.clone());
                }
            }
            H::RegView(..) => {
                let locs = self.g.bulk_locs(p);
                match v {
                    Sv::Agg(vs) if vs.len() == locs.len() => {
                        for (loc, x) in locs.into_iter().zip(vs) {
                            self.write_loc(loc, x);
                        }
                    }
                    other => {
                        // (A field of a value computed whole: what it is
                        // computed from, not where.)
                        let each = match other {
                            Sv::Elem(n, _) | Sv::Load(n, _) => Sv::Elem(n, NO),
                            v if v.invariant() => Sv::Inv(format!("{}.*", v.key()), NO),
                            v => v,
                        };
                        for loc in locs {
                            self.write_loc(loc, each.clone());
                        }
                    }
                }
            }
            H::KindSelect(k, args) => match self.select_place(locals, env, *k, args) {
                Some(place) => self.assign(locals, env, &place, v),
                None => self.fail("a write to an operand kind's register chosen by the element"),
            },
            H::Field(b, _) | H::Index(b, _) | H::Slice(b, ..) | H::TupleField(b, _) => {
                // Part of a register (a bundle's field): the whole of it,
                // varying.
                let mut e = &**b;
                loop {
                    match &e.kind {
                        H::Register(_) | H::RegField(..) | H::RegView(..) => {
                            for loc in self.g.bulk_locs(e) {
                                self.write_loc(loc, Sv::Dep(0));
                            }
                            return;
                        }
                        H::Local(l) => {
                            env.insert(*l, Sv::Dep(0));
                            return;
                        }
                        H::Field(x, _) | H::Index(x, _) | H::Slice(x, ..) | H::TupleField(x, _) => e = x,
                        _ => {
                            self.fail("a write to part of something a stream does not write");
                            return;
                        }
                    }
                }
            }
            _ => self.fail("a write to a latch, the mode or a memory cell"),
        }
    }

    /// The register an operand kind's `select` rule picks for constant
    /// arguments.
    fn select_place(&mut self, locals: &[hir::Local], env: &mut HashMap<LocalId, Sv>, k: DefId, args: &[Expr]) -> Option<Expr> {
        let rule = self.g.kinds[&k].rules.iter().find(|r| r.kind == hir::RuleKind::Select)?;
        let vals: Vec<Sv> = args.iter().map(|a| self.expr(locals, env, a)).collect();
        let mut renv: HashMap<LocalId, Sv> = HashMap::new();
        for (p, v) in rule.params.iter().zip(vals) {
            renv.insert(*p, v);
        }
        let mut e = match &rule.body {
            ArmBody::Expr(x) => x.clone(),
            _ => return None,
        };
        loop {
            match &e.kind {
                H::Match(s, arms) => {
                    let sv = self.expr(&rule.locals, &mut renv, s);
                    let a = self.arm_of(&sv, arms)?;
                    e = match &arms[a].body {
                        ArmBody::Expr(x) => x.clone(),
                        _ => return None,
                    };
                }
                H::Register(_) | H::RegField(..) | H::RegView(..) => return Some(e),
                _ => return None,
            }
        }
    }

    fn expr(&mut self, locals: &[hir::Local], env: &mut HashMap<LocalId, Sv>, e: &Expr) -> Sv {
        let at = e as *const Expr as usize;
        match self.expr_in(locals, env, e) {
            Sv::Dep(0) => Sv::Dep(at),
            Sv::Cond(c, a, 0) => Sv::Cond(c, a, at),
            v => v,
        }
    }

    /// `e`'s value where its arms are chosen by an invariant `cv` in the
    /// `while` condition: both arms, merged (a branch in the closed form).
    fn merged(&mut self, e: &Expr, cv: &Sv, c: &Expr, arms: Vec<(Sv, Ty, Span)>, pats: Option<&[hir::Arm]>) -> Sv {
        let first = arms[0].0.clone();
        let same = arms.iter().all(|a| a.0 == first);
        let mut xs = Vec::new();
        for (v, t, s) in &arms {
            xs.push(self.xof(v, t, *s));
        }
        let cx = self.xof(cv, &c.ty, c.span);
        let x = match (cx, xs.iter().all(Option::is_some)) {
            (Some(cx), true) => {
                let xs: Vec<Expr> = xs.into_iter().map(Option::unwrap).collect();
                Some(match pats {
                    Some(arms) => rc(
                        H::Match(Box::new(cx), arms.iter().zip(xs).map(|(a, x)| hir::Arm { patterns: a.patterns.clone(), body: ArmBody::Expr(x), span: a.span }).collect()),
                        &e.ty,
                        e.span,
                    ),
                    None => {
                        let mut it = xs.into_iter();
                        let (a, b) = (it.next().unwrap(), it.next().unwrap());
                        rc(H::Ternary(Box::new(cx), Box::new(a), Box::new(b)), &e.ty, e.span)
                    }
                })
            }
            _ => None,
        };
        if same && !matches!(first, Sv::Agg(_)) {
            return first.with_x(x);
        }
        if arms.iter().all(|a| a.0.invariant()) {
            let keys: Vec<String> = arms.iter().map(|a| a.0.key()).collect();
            return Sv::Inv(format!("(? {} {})", cv.key(), keys.join(" ")), X(x));
        }
        if arms.iter().all(|a| a.0.invariant() || a.0.data()) {
            let n = arms.iter().find_map(|a| a.0.load()).unwrap_or(0);
            return Sv::Elem(n, X(x));
        }
        Sv::Dep(0)
    }

    fn expr_in(&mut self, locals: &[hir::Local], env: &mut HashMap<LocalId, Sv>, e: &Expr) -> Sv {
        if self.p.fail.is_some() || (self.in_unit > 0 && self.unit_bad) {
            return Sv::Dep(0);
        }
        match &e.kind {
            H::Int(c) => Sv::K(*c),
            H::Bool(b) => Sv::K(*b as i128),
            H::Str(_) => Sv::Inv("str".into(), NO),
            H::Local(l) => env.get(l).cloned().unwrap_or(Sv::Dep(0)),
            H::Const(d) => Sv::Inv(format!("const{}", d.0), NO),
            H::Variant(_, v) => Sv::K(*v as i128),
            H::Unary(op, a) => {
                let av = self.expr(locals, env, a);
                let x = self.rebuild(e, &[(&av, a)], |mut xs| H::Unary(*op, xs.remove(0)));
                let v = match (op, av) {
                    (UnaryOp::Not, Sv::K(c)) => Sv::K((c == 0) as i128),
                    (UnaryOp::Not, Sv::InvB(k, n, _)) => Sv::InvB(k, !n, NO),
                    (UnaryOp::Not, Sv::Inv(k, _)) => Sv::InvB(k, true, NO),
                    (UnaryOp::Not, Sv::Cond(c, _, at)) => Sv::Cond(c.not(), None, at),
                    (_, Sv::K(c)) => Sv::Inv(format!("({op:?} {c} {:?})", e.ty), NO),
                    (_, v) if v.invariant() => Sv::Inv(format!("({op:?} {})", v.key()), NO),
                    (_, Sv::Elem(n, _) | Sv::Load(n, _)) => Sv::Elem(n, NO),
                    (_, v) => Sv::Dep(dep_at(&[&v])),
                };
                v.with_x(x)
            }
            H::Binary(op, a, b) => {
                let av = self.expr(locals, env, a);
                let bv = self.expr(locals, env, b);
                let x = self.rebuild(e, &[(&av, a), (&bv, b)], |mut xs| {
                    let r = xs.pop().unwrap();
                    H::Binary(*op, xs.pop().unwrap(), r)
                });
                let inner = av.wraps() || bv.wraps();
                let mut v = self.binary(*op, av, bv);
                if let Sv::Aff(l, off, w, _) = &mut v {
                    *w = inner || *w || !fits(l, off, &e.ty);
                }
                v.with_x(x)
            }
            H::Ternary(c, a, b) => {
                let cv = self.expr(locals, env, c);
                if self.phase == Phase::Cond && matches!(cv, Sv::Inv(..) | Sv::InvB(..)) {
                    let av = self.expr(locals, env, a);
                    let bv = self.expr(locals, env, b);
                    return self.merged(e, &cv, c, vec![(av, a.ty.clone(), a.span), (bv, b.ty.clone(), b.span)], None);
                }
                match self.truth(&cv) {
                    Some(true) => self.expr(locals, env, a),
                    Some(false) => self.expr(locals, env, b),
                    None => {
                        let av = self.expr(locals, env, a);
                        let bv = self.expr(locals, env, b);
                        if av == bv {
                            av
                        } else if cv.data() && (av.invariant() || av.data()) && (bv.invariant() || bv.data()) {
                            // A choice of what was loaded (a value, not a
                            // way through the code).
                            let x = self.rebuild(e, &[(&cv, c), (&av, a), (&bv, b)], |mut xs| {
                                let y = xs.pop().unwrap();
                                let x = xs.pop().unwrap();
                                H::Ternary(xs.pop().unwrap(), x, y)
                            });
                            Sv::Elem(cv.load().unwrap_or(0), X(x))
                        } else {
                            // Which arm depends on where the element is: the
                            // dry code stops where that was made.
                            self.stop_on(&cv);
                            Sv::Dep(0)
                        }
                    }
                }
            }
            // A cast keeps a pointer a pointer (where it may wrap, noted), and
            // a load a load.
            H::Cast(a) => {
                let av = self.expr(locals, env, a);
                let x = self.rebuild(e, &[(&av, a)], |mut xs| H::Cast(xs.remove(0)));
                let v = match av {
                    Sv::K(c) => Sv::K(cast_k(c, &e.ty)),
                    Sv::Inv(k, _) => Sv::Inv(format!("({k} as {:?})", e.ty), NO),
                    v @ Sv::InvB(..) => Sv::Inv(format!("({} as {:?})", v.key(), e.ty), NO),
                    Sv::Aff(l, off, w, _) => {
                        let w = w || !fits(&l, &off, &e.ty);
                        Sv::Aff(l, off, w, NO)
                    }
                    other => other,
                };
                v.with_x(x)
            }
            H::Field(b, i) => {
                let bv = self.expr(locals, env, b);
                let x = self.rebuild(e, &[(&bv, b)], |mut xs| H::Field(xs.remove(0), *i));
                match bv {
                    Sv::Agg(vs) => vs.get(*i as usize).cloned().unwrap_or(Sv::Dep(0)),
                    v if v.invariant() => Sv::Inv(format!("{}.{i}", v.key()), X(x)),
                    Sv::Elem(n, _) | Sv::Load(n, _) => Sv::Elem(n, X(x)),
                    v => Sv::Dep(dep_at(&[&v])),
                }
            }
            H::TupleField(b, i) => {
                let bv = self.expr(locals, env, b);
                let x = self.rebuild(e, &[(&bv, b)], |mut xs| H::TupleField(xs.remove(0), *i));
                match bv {
                    Sv::Agg(vs) => vs.get(*i as usize).cloned().unwrap_or(Sv::Dep(0)),
                    v if v.invariant() => Sv::Inv(format!("{}.{i}", v.key()), X(x)),
                    Sv::Elem(n, _) | Sv::Load(n, _) => Sv::Elem(n, X(x)),
                    v => Sv::Dep(dep_at(&[&v])),
                }
            }
            H::Index(b, i) => {
                let bv = self.expr(locals, env, b);
                let iv = self.expr(locals, env, i);
                let x = self.rebuild(e, &[(&bv, b), (&iv, i)], |mut xs| {
                    let r = xs.pop().unwrap();
                    H::Index(xs.pop().unwrap(), r)
                });
                let v = match (&bv, &iv) {
                    (Sv::K(x), Sv::K(n)) => Sv::K(if (0..128).contains(n) { (x >> n) & 1 } else { 0 }),
                    (a, n) if a.invariant() && n.invariant() => Sv::Inv(format!("{}[{}]", a.key(), n.key()), NO),
                    (a, n) if (a.data() || a.invariant()) && (n.data() || n.invariant()) => Sv::Elem(a.load().or(n.load()).unwrap_or(0), NO),
                    _ => Sv::Dep(dep_at(&[&bv, &iv])),
                };
                v.with_x(x)
            }
            H::Slice(b, hi, lo) => {
                let bv = self.expr(locals, env, b);
                let x = self.rebuild(e, &[(&bv, b)], |mut xs| H::Slice(xs.remove(0), *hi, *lo));
                let v = match bv {
                    Sv::K(x) => Sv::K((x >> lo) & ((1i128 << (hi - lo + 1)) - 1)),
                    v if v.invariant() => Sv::Inv(format!("{}[{hi}:{lo}]", v.key()), NO),
                    Sv::Elem(n, _) | Sv::Load(n, _) => Sv::Elem(n, NO),
                    v => Sv::Dep(dep_at(&[&v])),
                };
                v.with_x(x)
            }
            H::Builtin(Builtin::Restarted, _) if self.phase == (Phase::Side { steady: true }) => Sv::K(1),
            H::Call(u, args) => {
                let vs: Vec<Sv> = args.iter().map(|a| self.expr(locals, env, a)).collect();
                let kids: Vec<(&Sv, &Expr)> = vs.iter().zip(args).collect();
                let x = self.rebuild(e, &kids, |xs| H::Call(*u, xs.into_iter().map(|b| *b).collect()));
                match self.unit(*u, vs.clone()) {
                    // Inlined: its value is what it computes; its closed form
                    // the call (and a part of it, the call's part).
                    Some(Sv::Agg(parts)) => {
                        let tys: Option<Vec<Ty>> = match &e.ty {
                            Ty::Bundle(d) => self.g.bundles.get(d).map(|bd| bd.fields.iter().map(|f| f.ty.clone()).collect()),
                            Ty::Tuple(ts) => Some(ts.iter().map(|t| t.1.clone()).collect()),
                            _ => None,
                        };
                        let bundle = matches!(e.ty, Ty::Bundle(_));
                        Sv::Agg(
                            parts
                                .into_iter()
                                .enumerate()
                                .map(|(k, p)| {
                                    let px = match (&x, &tys) {
                                        (Some(x), Some(tys)) if k < tys.len() => {
                                            let kind = if bundle { H::Field(Box::new((**x).clone()), k as u32) } else { H::TupleField(Box::new((**x).clone()), k as u32) };
                                            Some(rc(kind, &tys[k], e.span))
                                        }
                                        _ => None,
                                    };
                                    p.with_x(px)
                                })
                                .collect(),
                        )
                    }
                    Some(v) => v.with_x(x),
                    None => Self::pure_of(&format!("unit{u:?}:{:?}", e.ty), vs).with_x(x),
                }
            }
            H::Builtin(bi, args) => {
                let vs: Vec<Sv> = args.iter().map(|a| self.expr(locals, env, a)).collect();
                let kids: Vec<(&Sv, &Expr)> = vs.iter().zip(args).collect();
                let x = self.rebuild(e, &kids, |xs| H::Builtin(*bi, xs.into_iter().map(|b| *b).collect()));
                Self::pure_of(&format!("{bi:?}:{:?}", e.ty), vs).with_x(x)
            }
            H::BundleLit(_, args) | H::Tuple(args) | H::Array(args) => Sv::Agg(args.iter().map(|a| self.expr(locals, env, a)).collect()),
            H::ArrayRepeat(a, n) => {
                let v = self.expr(locals, env, a);
                Sv::Agg(vec![v; (*n).min(64) as usize])
            }
            H::Match(s, arms) => {
                let sv = self.expr(locals, env, s);
                if self.phase == Phase::Cond && matches!(sv, Sv::Inv(..) | Sv::InvB(..)) && arms.iter().all(|a| matches!(a.body, ArmBody::Expr(_))) {
                    let mut vs = Vec::new();
                    for a in arms {
                        let ArmBody::Expr(x) = &a.body else { unreachable!() };
                        vs.push((self.expr(locals, env, x), x.ty.clone(), x.span));
                    }
                    return self.merged(e, &sv, s, vs, Some(arms));
                }
                match self.arm_of(&sv, arms) {
                    Some(a) => match &arms[a].body {
                        ArmBody::Expr(x) => self.expr(locals, env, x),
                        ArmBody::Block(b) => self.block_value(locals, env, b),
                    },
                    None => Sv::Dep(0),
                }
            }
            H::If(c, t, f) => {
                let cv = self.expr(locals, env, c);
                // In the `while` condition, `if inv { a } else { b }` with one
                // expression an arm: both, as a branch in its closed form.
                if self.phase == Phase::Cond && matches!(cv, Sv::Inv(..) | Sv::InvB(..)) && t.stmts.len() == 1 && matches!(t.stmts[0].kind, S::Expr(_)) {
                    if let (S::Expr(tx), Some(fx)) = (&t.stmts[0].kind, f) {
                        let av = self.expr(locals, env, tx);
                        let bv = self.expr(locals, env, fx);
                        return self.merged(e, &cv, c, vec![(av, tx.ty.clone(), tx.span), (bv, fx.ty.clone(), fx.span)], None);
                    }
                }
                match self.truth(&cv) {
                    Some(true) => self.block_value(locals, env, t),
                    Some(false) => match f {
                        Some(x) => self.expr(locals, env, x),
                        None => Sv::Unit,
                    },
                    None => {
                        self.fail_on("a branch on what an element loaded", &cv);
                        Sv::Dep(0)
                    }
                }
            }
            H::Block(b) => self.block_value(locals, env, b),
            H::Register(_) | H::RegField(..) => {
                let locs = self.g.bulk_locs(e);
                match locs.as_slice() {
                    [l] => self.read_loc(*l, e),
                    _ => Sv::Dep(0),
                }
            }
            H::RegView(..) => {
                let locs = self.g.bulk_locs(e);
                Sv::Agg(locs.into_iter().map(|l| self.read_loc(l, e)).map(|v| if let Sv::Inv(k, _) = v { Sv::Inv(k, NO) } else { v }).collect())
            }
            H::Mode(_) => Sv::Inv("mode".into(), X(Some(Rc::new(e.clone())))),
            H::Latch(d) => Sv::Inv(format!("latch{}", d.0), X(Some(Rc::new(e.clone())))),
            H::OperandRead(i) | H::OperandRule(i, _) => Sv::Inv(format!("op{i}"), NO),
            H::OperandField(b, i) => {
                let v = self.expr(locals, env, b);
                Sv::Inv(format!("{}.f{i}", v.key()), NO)
            }
            H::OperandTail(b) | H::TailOperand(b, _) => {
                let v = self.expr(locals, env, b);
                Sv::Inv(format!("{}.tail", v.key()), NO)
            }
            H::SpaceRead(sp, bits, a) => {
                if self.in_unit > 0 {
                    self.unit_bad = true;
                    return Sv::Dep(0);
                }
                let av = self.expr(locals, env, a);
                self.p.loads += 1;
                let n = self.p.loads;
                self.p.mem_reads.push((*sp, av, *bits, e.ty.clone()));
                self.p.load_sites.push(e as *const Expr as usize);
                // (Its closed form: the local `ld<n>`.)
                Sv::Load(n, X(Some(rc(H::Local(n - 1), &e.ty, e.span))))
            }
            H::SpaceWrite(sp, bits, a, v) => {
                if self.in_unit > 0 {
                    self.unit_bad = true;
                    return Sv::Unit;
                }
                let av = self.expr(locals, env, a);
                let vv = self.expr(locals, env, v);
                self.p.mem_writes.push((*sp, av, vv, *bits));
                Sv::Unit
            }
            H::KindSelect(k, args) => match self.select_place(locals, env, *k, args) {
                Some(place) => self.expr(locals, env, &place),
                None => Sv::Dep(0),
            },
            H::FnCall(d, args) => {
                let vs: Vec<Sv> = args.iter().map(|a| self.expr(locals, env, a)).collect();
                self.call(*d, vs)
            }
            _ => {
                self.fail("an expression a stream does not make");
                Sv::Dep(0)
            }
        }
    }

    /// A pure computation (a builtin, a unit not followed) of these values.
    fn pure_of(name: &str, vs: Vec<Sv>) -> Sv {
        if vs.iter().all(Sv::invariant) {
            Sv::Inv(format!("{name}({})", vs.iter().map(Sv::key).collect::<Vec<_>>().join(",")), NO)
        } else if vs.iter().all(|v| v.invariant() || v.data()) {
            Sv::Elem(vs.iter().find_map(Sv::load).unwrap_or(0), NO)
        } else {
            Sv::Dep(dep_at(&vs.iter().collect::<Vec<_>>()))
        }
    }

    /// A unit's value for these arguments, its body followed (None where
    /// it does what the evaluation does not follow).
    fn unit(&mut self, u: hir::UnitInstId, args: Vec<Sv>) -> Option<Sv> {
        let inst = self.g.hir.units.get(u.0 as usize)?;
        if self.depth > 24 {
            return None;
        }
        self.depth += 1;
        self.in_unit += 1;
        let bad = std::mem::replace(&mut self.unit_bad, false);
        let mut env: HashMap<LocalId, Sv> = HashMap::new();
        for (p, v) in inst.params.iter().zip(args) {
            env.insert(*p, v);
        }
        let r = match self.block(&inst.locals, &mut env, &inst.body) {
            Flow::Ret(v) => Some(v),
            _ => None,
        };
        let failed = self.unit_bad;
        self.unit_bad = bad;
        self.in_unit -= 1;
        self.depth -= 1;
        if failed {
            None
        } else {
            r
        }
    }

    fn block_value(&mut self, locals: &[hir::Local], env: &mut HashMap<LocalId, Sv>, b: &Block) -> Sv {
        let n = b.stmts.len();
        for (k, st) in b.stmts.iter().enumerate() {
            if k + 1 == n {
                // (A trailing expression is lowered as part of the statement
                // holding the block: that statement is where the dry code
                // stops.)
                if let S::Expr(x) = &st.kind {
                    return self.expr(locals, env, x);
                }
            }
            match self.stmt(locals, env, st) {
                Flow::Next => {}
                Flow::Ret(v) => return v,
                _ => return Sv::Dep(0),
            }
        }
        Sv::Unit
    }

    fn call(&mut self, d: DefId, args: Vec<Sv>) -> Sv {
        let Some(f) = self.g.fns.get(&d) else { return Sv::Dep(0) };
        if f.external {
            // (Supplied outside: the dry code would call it.)
            self.stop();
            return Sv::Dep(0);
        }
        if self.depth > 24 {
            self.fail("calls too deep");
            return Sv::Dep(0);
        }
        self.depth += 1;
        self.at.push(f.name.clone());
        let mut env: HashMap<LocalId, Sv> = HashMap::new();
        for (p, v) in f.params.iter().zip(args) {
            env.insert(*p, v);
        }
        let r = match self.block(&f.locals, &mut env, &f.body) {
            Flow::Ret(v) => v,
            Flow::Raise => {
                // A fn that raises here (a check whose condition held): the
                // path raises.
                if self.phase == Phase::Element {
                    if self.p.fail.is_none() {
                        self.p.fail = Some(format!("it raises in `{}`", f.name));
                    }
                    self.p.raised = true;
                } else {
                    self.fail("it raises");
                }
                Sv::Dep(0)
            }
            _ => Sv::Unit,
        };
        self.at.pop();
        self.depth -= 1;
        r
    }

    fn binary(&mut self, op: BinaryOp, a: Sv, b: Sv) -> Sv {
        use BinaryOp as B;
        let cmp = matches!(op, B::Eq | B::Ne | B::Lt | B::Le | B::Gt | B::Ge);
        // `&&` and `||` with a side that is a choice: as it falls (not in
        // the `while` condition's closed form, where both stay).
        if matches!(op, B::And | B::Or) && !(a.invariant() && b.invariant()) && self.phase != Phase::Cond {
            for (x, y) in [(&a, &b), (&b, &a)] {
                if x.invariant() {
                    if let Some(t) = self.truth(x) {
                        return match (op, t) {
                            (B::Or, true) => Sv::K(1),
                            (B::And, false) => Sv::K(0),
                            _ => y.clone(),
                        };
                    }
                }
            }
        }
        match (&a, &b) {
            (Sv::K(x), Sv::K(y)) => Sv::K(fold(op, *x, *y)),
            _ if a.invariant() && b.invariant() => {
                // Short-circuit forms keep a choice's key.
                match (op, &a, &b) {
                    (B::And, Sv::K(0), _) | (B::And, _, Sv::K(0)) => Sv::K(0),
                    (B::Or, Sv::K(x), _) | (B::Or, _, Sv::K(x)) if *x != 0 => Sv::K(1),
                    (B::And | B::Or, Sv::K(_), x) | (B::And | B::Or, x, Sv::K(_)) => x.clone().with_x(None),
                    _ if cmp || matches!(op, B::And | B::Or) => Sv::InvB(format!("({} {op:?} {})", a.key(), b.key()), false, NO),
                    _ => Sv::Inv(format!("({} {op:?} {})", a.key(), b.key()), NO),
                }
            }
            (Sv::Aff(l, off, ..), x) | (x, Sv::Aff(l, off, ..)) if x.invariant() && matches!(op, B::Add) => Sv::Aff(*l, Box::new(self.binary(B::Add, (**off).clone(), x.clone())), false, NO),
            (Sv::Aff(l, off, ..), x) if x.invariant() && matches!(op, B::Sub) => Sv::Aff(*l, Box::new(self.binary(B::Sub, (**off).clone(), x.clone())), false, NO),
            // Masking to the register's width: it wraps there.
            (Sv::Aff(l, off, ..), Sv::K(_)) if matches!(op, B::BitAnd) => Sv::Aff(*l, off.clone(), true, NO),
            // Where the element is, compared with an invariant.
            (Sv::Aff(..), y) | (y, Sv::Aff(..)) if cmp && y.invariant() => {
                let conv = if a.wraps() || b.wraps() {
                    Conv::Other
                } else {
                    match op {
                        B::Eq => Conv::Convex,
                        B::Ne => Conv::Concave,
                        _ => Conv::Half,
                    }
                };
                Sv::Cond(conv, Some(Box::new((op, a.clone(), b.clone()))), 0)
            }
            (Sv::Cond(ca, ..), Sv::Cond(cb, ..)) if matches!(op, B::And | B::Or) => {
                let conv = match op {
                    B::And if ca.interval() && cb.interval() => Conv::Convex,
                    B::Or if matches!(ca, Conv::Half | Conv::Concave) && matches!(cb, Conv::Half | Conv::Concave) => Conv::Concave,
                    _ => Conv::Other,
                };
                Sv::Cond(conv, None, 0)
            }
            // What was loaded, with invariants: still that.
            _ if (a.data() || a.invariant()) && (b.data() || b.invariant()) => Sv::Elem(a.load().or(b.load()).unwrap_or(0), NO),
            _ => match (&a, &b) {
                (Sv::Aff(..) | Sv::Cond(..), _) | (_, Sv::Aff(..) | Sv::Cond(..)) if cmp || matches!(op, B::And | B::Or) => Sv::Cond(Conv::Other, None, dep_at(&[&a, &b])),
                _ => Sv::Dep(dep_at(&[&a, &b])),
            },
        }
    }

    /// The path's kind of stream, or why it is none.
    fn classify(&self) -> Result<Shape, String> {
        if let Some(f) = &self.p.fail {
            return Err(f.clone());
        }
        // Pointers: written registers that move by an invariant.
        let mut moved: Vec<Loc> = Vec::new();
        for (l, v) in &self.p.writes {
            match v {
                Sv::Aff(l0, off, ..) if l0 == l && off.invariant() => moved.push(*l),
                Sv::Aff(..) => return Err("a register moves by something not the same each element".into()),
                // Results: from what it loaded, or invariant.
                _ => {}
            }
        }
        let streams = |a: &Sv| matches!(a, Sv::Aff(l, off, ..) if moved.contains(l) && off.invariant());
        let mem_r: Vec<&(DefId, Sv, u32, Ty)> = self.p.mem_reads.iter().filter(|r| r.0 == self.mem).collect();
        let mem_w: Vec<&(DefId, Sv, Sv, u32)> = self.p.mem_writes.iter().filter(|w| w.0 == self.mem).collect();
        let port_r: Vec<&(DefId, Sv, u32, Ty)> = self.p.mem_reads.iter().filter(|r| r.0 != self.mem).collect();
        let port_w: Vec<&(DefId, Sv, Sv, u32)> = self.p.mem_writes.iter().filter(|w| w.0 != self.mem).collect();
        if mem_r.iter().any(|r| !streams(&r.1)) || mem_w.iter().any(|w| !streams(&w.1)) {
            return Err("a memory access at an address that does not move with the element".into());
        }
        let load_of = |v: &Sv, reads: &[&(DefId, Sv, u32, Ty)]| -> bool {
            matches!(v, Sv::Load(n, _) if (*n as usize) <= self.p.mem_reads.len() && reads.iter().any(|r| std::ptr::eq(*r, &self.p.mem_reads[*n as usize - 1])))
        };
        match (mem_r.len(), mem_w.len(), port_r.len(), port_w.len()) {
            (0, 1, 0, 0) if mem_w[0].2.invariant() => Ok(Shape::Fill),
            (0, 1, 0, 0) => Err("it writes something that changes each element without reading it".into()),
            (1, 1, 0, 0) if load_of(&mem_w[0].2, &mem_r) => Ok(Shape::Copy),
            (1, 0, 0, 0) => {
                let into_reg = self.p.writes.iter().any(|(l, v)| matches!(v, Sv::Load(..)) && !moved.contains(l));
                Ok(if into_reg { Shape::Load } else { Shape::Scan })
            }
            (2, 0, 0, 0) => Ok(Shape::Compare),
            (0, 1, 1, 0) if load_of(&mem_w[0].2, &port_r) => Ok(Shape::PortIn),
            (1, 0, 0, 1) if load_of(&port_w[0].2, &mem_r) => Ok(Shape::PortOut),
            (r, w, pr, pw) => Err(format!("{r} memory reads, {w} writes, {pr} port reads, {pw} port writes")),
        }
    }

    /// What a path tells the kernel (the element's writes `elem`, `next`'s
    /// value `nx`, the `while` condition's `cv`), or why it cannot take the
    /// path.
    fn kernel(&self, shape: Shape, elem: &[(Loc, Sv)], nx: &Sv, cv: Option<&Sv>) -> Result<Kernel, String> {
        if let Some(why) = &self.p.kfail {
            return Err(why.clone());
        }
        // The count: `next` takes one from a register, and the instruction
        // repeats while it is not zero.
        let Sv::Cond(_, Some(atom), _) = nx else { return Err("`next` is not a test of a count".into()) };
        let count = match &**atom {
            (BinaryOp::Ne, Sv::Aff(l, off, ..), Sv::K(0)) | (BinaryOp::Ne, Sv::K(0), Sv::Aff(l, off, ..)) if **off == Sv::K(-1) => *l,
            _ => return Err("`next` is not a count down to zero".into()),
        };
        let counted = self.p.writes.iter().any(|(l, v)| *l == count && matches!(v, Sv::Aff(l0, off, ..) if *l0 == count && **off == Sv::K(-1)));
        if !counted || elem.iter().any(|(l, _)| l.0 == count.0) {
            return Err("the count is not counted down once by `next` alone".into());
        }
        if self.p.writes.len() != elem.len() + 1 {
            return Err("`next` writes more than the count".into());
        }
        // The pointers; and the results, each the element's own (from what
        // it loaded and invariants), so the last element's are the
        // instruction's.
        let mut moving: Vec<(DefId, u32)> = Vec::new();
        for (l, v) in elem {
            match v {
                Sv::Aff(l0, off, ..) if l0 == l && off.invariant() && l.1 == 0 => match moving.iter_mut().find(|m| m.0 == l.0) {
                    Some(m) => m.1 = m.1.min(l.2),
                    None => moving.push((l.0, l.2)),
                },
                Sv::Aff(..) => return Err("a pointer not at bit 0 of its register".into()),
                v if v.invariant() || v.data() => {}
                _ => return Err("a result that depends on more than what the element loaded".into()),
            }
        }
        for (l, _) in elem {
            if !matches!(elem.iter().find(|(x, _)| x == l), Some((_, Sv::Aff(..)))) && moving.iter().any(|m| m.0 == l.0) {
                return Err("a pointer's register holds a result too".into());
            }
        }
        if count.1 != 0 {
            return Err("a count not at bit 0 of its register".into());
        }
        moving.push((count.0, count.2));
        let bits = match shape {
            Shape::Fill | Shape::Copy => self.p.mem_writes.iter().find(|w| w.0 == self.mem).map(|w| w.3).ok_or("no write")?,
            _ => self.p.mem_reads.iter().find(|r| r.0 == self.mem).map(|r| r.2).ok_or("no read")?,
        };
        if self.p.mem_reads.iter().any(|r| r.2 != bits) || self.p.mem_writes.iter().any(|w| w.3 != bits) {
            return Err("accesses of different sizes".into());
        }
        if bits % 8 != 0 || bits > 64 {
            return Err("an access of whole bytes, at most 8".into());
        }
        for (d, _) in &moving {
            if ty_bits(&self.g.regs[d].1.ty) > 64 {
                return Err("a register wider than 64 bits moves".into());
            }
        }
        // The `while` condition's closed form, over what the element loaded.
        let tys: Vec<Ty> = self.p.mem_reads.iter().map(|r| r.3.clone()).collect();
        let mut loads = Vec::new();
        for t in &tys {
            loads.push(self.g.rty(t).map_err(|e| e.to_string())?);
        }
        let cond = match cv {
            None => None,
            Some(v) if v.invariant() || v.data() => {
                let x = self.xof(v, &Ty::Bool, Span::default()).ok_or("a `while` condition with no closed form")?;
                let locals = Gen::bulk_load_locals(&tys);
                let cx = super::Cx { locals: &locals, fallible: true, recv: "self", ops: None, comp: None, opt: false };
                Some(self.g.expr(&cx, &x).map_err(|e| e.to_string())?)
            }
            Some(_) => return Err("a `while` condition on more than what the element loaded".into()),
        };
        if cond.is_some() && !matches!(shape, Shape::Scan | Shape::Compare | Shape::Load) {
            return Err("a `while` condition on a fill or copy".into());
        }
        Ok(Kernel { shape, bits, moving, written: Vec::new(), fns: Vec::new(), stops: Vec::new(), cond, loads })
    }
}

fn fold(op: BinaryOp, x: i128, y: i128) -> i128 {
    use BinaryOp as B;
    match op {
        B::Mul => x.wrapping_mul(y),
        B::Div if y != 0 => x / y,
        B::Rem if y != 0 => x % y,
        B::Add => x.wrapping_add(y),
        B::Sub => x.wrapping_sub(y),
        B::Shl if (0..127).contains(&y) => x << y,
        B::Shr if (0..127).contains(&y) => x >> y,
        B::BitAnd => x & y,
        B::BitXor => x ^ y,
        B::BitOr => x | y,
        B::Eq => (x == y) as i128,
        B::Ne => (x != y) as i128,
        B::Lt => (x < y) as i128,
        B::Le => (x <= y) as i128,
        B::Gt => (x > y) as i128,
        B::Ge => (x >= y) as i128,
        B::And => (x != 0 && y != 0) as i128,
        B::Or => (x != 0 || y != 0) as i128,
        _ => 0,
    }
}
