//! The `bulk` check of a `repeat` clause, when the description is built: is
//! the instruction's element (its semantics, once) a stream the cores may
//! run as a block?
//!
//! The element is evaluated abstractly, every fn it calls inlined. A value
//! is a constant; an invariant (the same every element: latches, operands,
//! state the element never writes); a register as the element found it
//! plus an invariant (a pointer that moves the same way each time); or an
//! element value (what it loaded, and anything computed from that).
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

use std::collections::{HashMap, HashSet};

use machine_sema::hir::{self, ArmBody, Block, Expr, ExprKind as H, LetTarget, LocalId, Pat, StmtKind as S};
use machine_sema::program::DefId;
use machine_sema::ty::Ty;
use machine_syntax::ast::{BinaryOp, UnaryOp};

use super::{Gen, GenError, R};

/// A place in a register: the register, its first bit and width.
type Loc = (DefId, u32, u32);

#[derive(Clone, Debug, PartialEq)]
enum Sv {
    K(i128),
    /// An invariant, named by where it came from; a bool one with its
    /// polarity (`!c` is `c` negated: the same choice).
    Inv(String),
    InvB(String, bool),
    /// A register as the element found it, plus an invariant (or constant).
    Aff(Loc, Box<Sv>),
    /// The nth access's value as it was loaded, and a value computed from
    /// a load.
    Load(u32),
    Elem(u32),
    Dep,
    Agg(Vec<Sv>),
    Unit,
}

impl Sv {
    fn invariant(&self) -> bool {
        matches!(self, Sv::K(_) | Sv::Inv(_) | Sv::InvB(..))
    }
    fn key(&self) -> String {
        match self {
            Sv::K(c) => c.to_string(),
            Sv::Inv(k) => k.clone(),
            Sv::InvB(k, neg) => if *neg { format!("!{k}") } else { k.clone() },
            other => format!("{other:?}"),
        }
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
    mem_reads: Vec<(DefId, Sv, u32)>,
    mem_writes: Vec<(DefId, Sv, Sv)>,
    loads: u32,
    fail: Option<String>,
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

struct Eval<'g, 'a> {
    g: &'g Gen<'a>,
    /// The locations the element may write, anywhere (a register read that
    /// none of them overlaps is an invariant).
    written: Vec<Loc>,
    mem: DefId,
    p: Path,
    depth: u32,
    /// The fns being evaluated (for saying where a path stops).
    at: Vec<String>,
}

const MAX_PATHS: usize = 256;

impl Gen<'_> {
    /// Whether instruction `i`'s element is a stream on some path (`bulk`),
    /// and which; an error naming why not.
    pub(crate) fn bulk_shape(&self, i: &hir::Instruction) -> R<Shape> {
        let Some(sem) = &i.semantics else {
            return Err(GenError::new(format!("`{}`: `bulk` with no semantics", i.name)));
        };
        let mem = self.isa.istream.as_ref().map(|s| s.space).ok_or_else(|| GenError::new("`bulk` needs an istream"))?;
        let written = self.bulk_written(sem);
        let mut todo: Vec<HashMap<String, u32>> = vec![HashMap::new()];
        let mut reasons: Vec<String> = Vec::new();
        let mut runs = 0;
        while let Some(choices) = todo.pop() {
            runs += 1;
            if runs > MAX_PATHS {
                break;
            }
            let mut e = Eval { g: self, written: written.clone(), mem, p: Path { choices: choices.clone(), ..Default::default() }, depth: 0, at: vec![i.name.clone()] };
            let mut env: HashMap<LocalId, Sv> = HashMap::new();
            let flow = e.block(&i.locals, &mut env, sem);
            for (key, arity) in std::mem::take(&mut e.p.fresh) {
                for alt in 1..arity {
                    let mut c = choices.clone();
                    for (k, v) in &e.p.choices {
                        c.entry(k.clone()).or_insert(*v);
                    }
                    c.insert(key.clone(), alt);
                    todo.push(c);
                }
            }
            if matches!(flow, Flow::Raise) && e.p.fail.is_none() {
                e.p.fail = Some("it raises".into());
            }
            if std::env::var_os("MACHINE_BULK").is_some() {
                eprintln!("bulk path {}: writes {:?} reads {:?} stores {:?} fail {:?}", i.name, e.p.writes, e.p.mem_reads, e.p.mem_writes, e.p.fail);
            }
            match e.classify() {
                Ok(s) => return Ok(s),
                Err(why) => {
                    if !reasons.contains(&why) {
                        reasons.push(why);
                    }
                }
            }
        }
        Err(GenError::new(format!(
            "`{}`: `repeat ... bulk`, but its element is not a fill, copy, scan, compare, load or port stream on any path ({})",
            i.name,
            reasons.join("; ")
        )))
    }

    /// The register locations the element may write (its semantics and the
    /// fns they call; an operand kind's selected registers all of them).
    fn bulk_written(&self, b: &Block) -> Vec<Loc> {
        let out = std::cell::RefCell::new(Vec::new());
        let mut seen: HashSet<DefId> = HashSet::new();
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
        loop {
            let Some(blk) = blocks.pop() else {
                let Some(f) = stack.borrow_mut().pop() else { break };
                if seen.insert(f) {
                    if let Some(fb) = self.fns.get(&f) {
                        if !fb.external {
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
        out.into_inner()
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

impl Eval<'_, '_> {
    fn fail(&mut self, why: impl Into<String>) {
        if self.p.fail.is_none() {
            self.p.fail = Some(format!("{} in `{}`", why.into(), self.at.last().map(String::as_str).unwrap_or("")));
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
            Sv::InvB(k, neg) => Some((self.choose(k, 2) == 0) != *neg),
            Sv::Inv(k) => Some(self.choose(k, 2) == 0),
            _ => None,
        }
    }

    fn read_loc(&mut self, loc: Loc) -> Sv {
        for (l, v) in self.p.writes.iter().rev() {
            if *l == loc {
                return v.clone();
            }
            if overlaps(l, &loc) {
                return Sv::Dep;
            }
        }
        if self.written.iter().any(|w| overlaps(w, &loc)) {
            Sv::Aff(loc, Box::new(Sv::K(0)))
        } else {
            Sv::Inv(format!("reg{}:{}:{}", loc.0 .0, loc.1, loc.2))
        }
    }

    fn write_loc(&mut self, loc: Loc, v: Sv) {
        self.p.writes.retain(|(l, _)| !overlaps(l, &loc) || *l == loc);
        self.p.writes.retain(|(l, _)| *l != loc);
        self.p.writes.push((loc, v));
    }

    fn block(&mut self, locals: &[hir::Local], env: &mut HashMap<LocalId, Sv>, b: &Block) -> Flow {
        for st in &b.stmts {
            if self.p.fail.is_some() {
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

    fn stmt(&mut self, locals: &[hir::Local], env: &mut HashMap<LocalId, Sv>, st: &hir::Stmt) -> Flow {
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
                        if t_raises && !e_raises {
                            match e {
                                Some(s) => self.stmt(locals, env, s),
                                None => Flow::Next,
                            }
                        } else if e_raises && !t_raises {
                            self.block(locals, env, t)
                        } else {
                            self.fail("a branch on what an element loaded or where it is");
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
            S::Unreachable(_) | S::Raise(..) => Flow::Raise,
            S::Assign(p, v) => {
                let vv = self.expr(locals, env, v);
                self.assign(locals, env, p, vv);
                Flow::Next
            }
            S::SpaceWrite(sp, _, a, v) => {
                let av = self.expr(locals, env, a);
                let vv = self.expr(locals, env, v);
                self.p.mem_writes.push((*sp, av, vv));
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
            Sv::Inv(k) | Sv::InvB(k, _) => {
                let a = self.choose(&format!("match {k}"), arms.len() as u32);
                Some(a as usize)
            }
            _ => {
                self.fail("a match on what an element loaded");
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
                        for loc in locs {
                            self.write_loc(loc, other.clone());
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
                                self.write_loc(loc, Sv::Dep);
                            }
                            return;
                        }
                        H::Local(l) => {
                            env.insert(*l, Sv::Dep);
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
        if self.p.fail.is_some() {
            return Sv::Dep;
        }
        match &e.kind {
            H::Int(c) => Sv::K(*c),
            H::Bool(b) => Sv::K(*b as i128),
            H::Str(_) => Sv::Inv("str".into()),
            H::Local(l) => env.get(l).cloned().unwrap_or(Sv::Dep),
            H::Const(d) => Sv::Inv(format!("const{}", d.0)),
            H::Variant(_, v) => Sv::K(*v as i128),
            H::Unary(op, a) => {
                let av = self.expr(locals, env, a);
                match (op, av) {
                    (UnaryOp::Not, Sv::K(c)) => Sv::K((c == 0) as i128),
                    (UnaryOp::Not, Sv::InvB(k, n)) => Sv::InvB(k, !n),
                    (UnaryOp::Not, Sv::Inv(k)) => Sv::InvB(k, true),
                    (_, Sv::K(c)) => Sv::Inv(format!("({op:?} {c})")),
                    (_, v) if v.invariant() => Sv::Inv(format!("({op:?} {})", v.key())),
                    (_, Sv::Elem(n) | Sv::Load(n)) => Sv::Elem(n),
                    _ => Sv::Dep,
                }
            }
            H::Binary(op, a, b) => {
                let av = self.expr(locals, env, a);
                let bv = self.expr(locals, env, b);
                self.binary(*op, av, bv)
            }
            H::Ternary(c, a, b) => {
                let cv = self.expr(locals, env, c);
                match self.truth(&cv) {
                    Some(true) => self.expr(locals, env, a),
                    Some(false) => self.expr(locals, env, b),
                    None => {
                        let av = self.expr(locals, env, a);
                        let bv = self.expr(locals, env, b);
                        if av == bv { av } else { Sv::Dep }
                    }
                }
            }
            // A cast keeps a pointer a pointer (it wraps where the register
            // does).
            H::Cast(a) => self.expr(locals, env, a),
            H::Field(b, i) => match self.expr(locals, env, b) {
                Sv::Agg(vs) => vs.get(*i as usize).cloned().unwrap_or(Sv::Dep),
                v if v.invariant() => Sv::Inv(format!("{}.{i}", v.key())),
                Sv::Elem(n) | Sv::Load(n) => Sv::Elem(n),
                _ => Sv::Dep,
            },
            H::TupleField(b, i) => match self.expr(locals, env, b) {
                Sv::Agg(vs) => vs.get(*i as usize).cloned().unwrap_or(Sv::Dep),
                v if v.invariant() => Sv::Inv(format!("{}.{i}", v.key())),
                Sv::Elem(n) | Sv::Load(n) => Sv::Elem(n),
                _ => Sv::Dep,
            },
            H::Index(b, i) => {
                let bv = self.expr(locals, env, b);
                let iv = self.expr(locals, env, i);
                match (bv, iv) {
                    (Sv::K(x), Sv::K(n)) => Sv::K(if (0..128).contains(&n) { (x >> n) & 1 } else { 0 }),
                    (a, n) if a.invariant() && n.invariant() => Sv::Inv(format!("{}[{}]", a.key(), n.key())),
                    (Sv::Elem(n) | Sv::Load(n), _) => Sv::Elem(n),
                    _ => Sv::Dep,
                }
            }
            H::Slice(b, hi, lo) => match self.expr(locals, env, b) {
                Sv::K(x) => Sv::K((x >> lo) & ((1i128 << (hi - lo + 1)) - 1)),
                v if v.invariant() => Sv::Inv(format!("{}[{hi}:{lo}]", v.key())),
                Sv::Elem(n) | Sv::Load(n) => Sv::Elem(n),
                _ => Sv::Dep,
            },
            H::Call(_, args) | H::Builtin(_, args) => {
                let vs: Vec<Sv> = args.iter().map(|a| self.expr(locals, env, a)).collect();
                Self::pure_of(&format!("{:?}", e.kind).chars().take(24).collect::<String>(), vs)
            }
            H::BundleLit(_, args) | H::Tuple(args) | H::Array(args) => Sv::Agg(args.iter().map(|a| self.expr(locals, env, a)).collect()),
            H::ArrayRepeat(a, n) => {
                let v = self.expr(locals, env, a);
                Sv::Agg(vec![v; (*n).min(64) as usize])
            }
            H::Match(s, arms) => {
                let sv = self.expr(locals, env, s);
                match self.arm_of(&sv, arms) {
                    Some(a) => match &arms[a].body {
                        ArmBody::Expr(x) => self.expr(locals, env, x),
                        ArmBody::Block(b) => self.block_value(locals, env, b),
                    },
                    None => Sv::Dep,
                }
            }
            H::If(c, t, f) => {
                let cv = self.expr(locals, env, c);
                match self.truth(&cv) {
                    Some(true) => self.block_value(locals, env, t),
                    Some(false) => match f {
                        Some(x) => self.expr(locals, env, x),
                        None => Sv::Unit,
                    },
                    None => {
                        self.fail("a branch on what an element loaded");
                        Sv::Dep
                    }
                }
            }
            H::Block(b) => self.block_value(locals, env, b),
            H::Register(_) | H::RegField(..) => {
                let locs = self.g.bulk_locs(e);
                match locs.as_slice() {
                    [l] => self.read_loc(*l),
                    _ => Sv::Dep,
                }
            }
            H::RegView(..) => Sv::Agg(self.g.bulk_locs(e).into_iter().map(|l| self.read_loc(l)).collect()),
            H::Mode(_) => Sv::Inv("mode".into()),
            H::Latch(d) => Sv::Inv(format!("latch{}", d.0)),
            H::OperandRead(i) | H::OperandRule(i, _) => Sv::Inv(format!("op{i}")),
            H::OperandField(b, i) => {
                let v = self.expr(locals, env, b);
                Sv::Inv(format!("{}.f{i}", v.key()))
            }
            H::OperandTail(b) | H::TailOperand(b, _) => {
                let v = self.expr(locals, env, b);
                Sv::Inv(format!("{}.tail", v.key()))
            }
            H::SpaceRead(sp, bits, a) => {
                let av = self.expr(locals, env, a);
                self.p.loads += 1;
                let n = self.p.loads;
                self.p.mem_reads.push((*sp, av, *bits));
                Sv::Load(n)
            }
            H::SpaceWrite(sp, _, a, v) => {
                let av = self.expr(locals, env, a);
                let vv = self.expr(locals, env, v);
                self.p.mem_writes.push((*sp, av, vv));
                Sv::Unit
            }
            H::KindSelect(k, args) => match self.select_place(locals, env, *k, args) {
                Some(place) => self.expr(locals, env, &place),
                None => Sv::Dep,
            },
            H::FnCall(d, args) => {
                let vs: Vec<Sv> = args.iter().map(|a| self.expr(locals, env, a)).collect();
                self.call(*d, vs)
            }
            _ => {
                self.fail("an expression a stream does not make");
                Sv::Dep
            }
        }
    }

    /// A pure computation (a unit, a builtin) of these values.
    fn pure_of(name: &str, vs: Vec<Sv>) -> Sv {
        if vs.iter().all(Sv::invariant) {
            Sv::Inv(format!("{name}({})", vs.iter().map(Sv::key).collect::<Vec<_>>().join(",")))
        } else if let Some(n) = vs.iter().find_map(|v| if let Sv::Elem(n) | Sv::Load(n) = v { Some(*n) } else { None }) {
            Sv::Elem(n)
        } else {
            Sv::Dep
        }
    }

    fn block_value(&mut self, locals: &[hir::Local], env: &mut HashMap<LocalId, Sv>, b: &Block) -> Sv {
        let n = b.stmts.len();
        for (k, st) in b.stmts.iter().enumerate() {
            if k + 1 == n {
                if let S::Expr(x) = &st.kind {
                    return self.expr(locals, env, x);
                }
            }
            match self.stmt(locals, env, st) {
                Flow::Next => {}
                Flow::Ret(v) => return v,
                _ => return Sv::Dep,
            }
        }
        Sv::Unit
    }

    fn call(&mut self, d: DefId, args: Vec<Sv>) -> Sv {
        let Some(f) = self.g.fns.get(&d) else { return Sv::Dep };
        if f.external {
            return Sv::Dep;
        }
        if self.depth > 24 {
            self.fail("calls too deep");
            return Sv::Dep;
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
                self.fail("it raises");
                Sv::Dep
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
        // `&&` and `||` with a side that is a choice: as it falls.
        if matches!(op, B::And | B::Or) && !(a.invariant() && b.invariant()) {
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
                    (B::And | B::Or, Sv::K(_), x) | (B::And | B::Or, x, Sv::K(_)) => x.clone(),
                    _ if cmp || matches!(op, B::And | B::Or) => Sv::InvB(format!("({} {op:?} {})", a.key(), b.key()), false),
                    _ => Sv::Inv(format!("({} {op:?} {})", a.key(), b.key())),
                }
            }
            (Sv::Aff(l, off), x) | (x, Sv::Aff(l, off)) if x.invariant() && matches!(op, B::Add) => Sv::Aff(*l, Box::new(self.binary(B::Add, (**off).clone(), x.clone()))),
            (Sv::Aff(l, off), x) if x.invariant() && matches!(op, B::Sub) => Sv::Aff(*l, Box::new(self.binary(B::Sub, (**off).clone(), x.clone()))),
            // Masking to the register's width: it wraps there.
            (Sv::Aff(..), Sv::K(_)) if matches!(op, B::BitAnd) => a.clone(),
            _ => match (&a, &b) {
                (Sv::Elem(n) | Sv::Load(n), _) | (_, Sv::Elem(n) | Sv::Load(n)) => Sv::Elem(*n),
                _ => Sv::Dep,
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
                Sv::Aff(l0, off) if l0 == l && off.invariant() => moved.push(*l),
                Sv::Aff(..) => return Err("a register moves by something not the same each element".into()),
                // Results: from what it loaded, or invariant.
                _ => {}
            }
        }
        let streams = |a: &Sv| matches!(a, Sv::Aff(l, off) if moved.contains(l) && off.invariant());
        let mem_r: Vec<&(DefId, Sv, u32)> = self.p.mem_reads.iter().filter(|r| r.0 == self.mem).collect();
        let mem_w: Vec<&(DefId, Sv, Sv)> = self.p.mem_writes.iter().filter(|w| w.0 == self.mem).collect();
        let port_r: Vec<&(DefId, Sv, u32)> = self.p.mem_reads.iter().filter(|r| r.0 != self.mem).collect();
        let port_w: Vec<&(DefId, Sv, Sv)> = self.p.mem_writes.iter().filter(|w| w.0 != self.mem).collect();
        if mem_r.iter().any(|r| !streams(&r.1)) || mem_w.iter().any(|w| !streams(&w.1)) {
            return Err("a memory access at an address that does not move with the element".into());
        }
        let load_of = |v: &Sv, reads: &[&(DefId, Sv, u32)]| -> bool {
            matches!(v, Sv::Load(n) if (*n as usize) <= self.p.mem_reads.len() && reads.iter().any(|r| std::ptr::eq(*r, &self.p.mem_reads[*n as usize - 1])))
        };
        match (mem_r.len(), mem_w.len(), port_r.len(), port_w.len()) {
            (0, 1, 0, 0) if mem_w[0].2.invariant() => Ok(Shape::Fill),
            (0, 1, 0, 0) => Err("it writes something that changes each element without reading it".into()),
            (1, 1, 0, 0) if load_of(&mem_w[0].2, &mem_r) => Ok(Shape::Copy),
            (1, 0, 0, 0) => {
                let into_reg = self.p.writes.iter().any(|(l, v)| matches!(v, Sv::Load(_)) && !moved.contains(l));
                Ok(if into_reg { Shape::Load } else { Shape::Scan })
            }
            (2, 0, 0, 0) => Ok(Shape::Compare),
            (0, 1, 1, 0) if load_of(&mem_w[0].2, &port_r) => Ok(Shape::PortIn),
            (1, 0, 0, 1) if load_of(&port_w[0].2, &mem_r) => Ok(Shape::PortOut),
            (r, w, pr, pw) => Err(format!("{r} memory reads, {w} writes, {pr} port reads, {pw} port writes")),
        }
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
