//! The abstract evaluation of one way through an element: every fn and unit
//! it calls inlined, a branch on an invariant answered as the path's choice.

use std::collections::{HashMap, HashSet};

use machine_sema::builtins::Builtin;
use machine_sema::hir::{self, ArmBody, Block, Expr, ExprKind as H, LetTarget, LocalId, Pat, StmtKind as S};
use machine_sema::program::DefId;
use machine_sema::ty::Ty;
use machine_syntax::ast::{BinaryOp, UnaryOp};
use machine_syntax::span::Span;

use super::val::{cast_k, fits, fold, konst, made_of, overlaps, rc, Conv, Kind, Loc, Val};
use super::Site;
use crate::gen::Gen;

/// How a path left a block.
pub(super) enum Flow {
    Next,
    Ret(Val),
    Raise,
    Break,
}

/// What the evaluation is of.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Phase {
    /// The element: a path's ways through it, its stops.
    Element,
    /// The `next` clause and the timing entry, for the kernel only. In the
    /// timing entry `restarted()` holds (the elements after the first).
    Side { steady: bool },
    /// The `while` condition, for the kernel only: a branch on an invariant
    /// is kept a branch (both arms, in its closed form).
    Cond,
}

/// One way through the element.
#[derive(Default)]
pub(super) struct Path {
    /// Each invariant condition's answer, by key (arm index for a match).
    pub choices: HashMap<String, u32>,
    /// The conditions met this run with no answer given: answered 0, to be
    /// tried otherwise.
    pub fresh: Vec<(String, u32)>,
    pub writes: Vec<(Loc, Val)>,
    /// Accesses: the space, the address, the bits (a read: the value's
    /// type; a write: the value).
    pub mem_reads: Vec<(DefId, Val, u32, Ty)>,
    pub mem_writes: Vec<(DefId, Val, Val, u32)>,
    /// The expression each load is.
    pub load_sites: Vec<Site>,
    /// Why the path is not a stream.
    pub fail: Option<String>,
    /// It raised (the dry code raises too).
    pub raised: bool,
    /// It reached where the dry code stops.
    pub stopped: bool,
    /// Outside the element (`next`, `while`, the timing entry): why the
    /// kernel cannot take the path.
    pub kfail: Option<String>,
}

/// Where the dry code stops for a branch: at the statement being
/// evaluated, or where the value branched on was made.
pub(super) enum At<'v> {
    Here,
    Made(&'v Val),
}

pub(super) struct Eval<'g, 'a> {
    pub g: &'g Gen<'a>,
    /// The locations the element (and `next`) may write, anywhere (a
    /// register read that none of them overlaps is an invariant).
    pub written: Vec<Loc>,
    /// The istream's space (memory; the others are ports).
    pub mem: DefId,
    pub p: Path,
    depth: u32,
    /// The fns being evaluated (for saying where a path stops).
    at: Vec<String>,
    pub phase: Phase,
    /// The statement being evaluated; where the dry code stops (on any
    /// path so far).
    site: Site,
    pub stops: HashSet<Site>,
    /// Inside a unit (inlined); and whether it did what the evaluation does
    /// not follow (it is then a call whose value is computed from its
    /// arguments).
    in_unit: u32,
    unit_bad: bool,
}

impl<'g, 'a> Eval<'g, 'a> {
    pub(super) fn new(g: &'g Gen<'a>, written: Vec<Loc>, mem: DefId, choices: HashMap<String, u32>, name: &str, stops: HashSet<Site>) -> Self {
        Eval { g, written, mem, p: Path { choices, ..Default::default() }, depth: 0, at: vec![name.to_string()], phase: Phase::Side { steady: true }, site: Site::NONE, stops, in_unit: 0, unit_bad: false }
    }

    // -- what the evaluation does not follow ------------------------------------

    /// Nothing more of this path (or unit) is evaluated.
    fn ended(&self) -> bool {
        self.p.fail.is_some() || (self.in_unit > 0 && self.unit_bad)
    }

    /// The dry code stops here: the path is not one the kernel takes. (In
    /// a unit: the unit is not followed. Past the path's end: nothing.)
    pub(super) fn stop(&mut self, at: At) {
        if self.in_unit > 0 {
            self.unit_bad = true;
            return;
        }
        if self.p.fail.is_some() || self.phase != Phase::Element {
            return;
        }
        let made = match at {
            At::Made(v) => self.prov(v),
            At::Here => Site::NONE,
        };
        let s = if made.is_none() { self.site } else { made };
        if !s.is_none() {
            self.stops.insert(s);
            self.p.stopped = true;
        }
    }

    /// The path ends here, not a stream (and the dry code stops). Outside
    /// the element: the kernel cannot take the path.
    pub(super) fn end(&mut self, why: impl Into<String>, at: At) {
        if self.in_unit > 0 {
            self.unit_bad = true;
            return;
        }
        if self.p.fail.is_some() {
            return;
        }
        let why = format!("{} in `{}`", why.into(), self.at.last().map(String::as_str).unwrap_or(""));
        if self.phase == Phase::Element {
            self.stop(at);
            self.p.fail = Some(why);
        } else if self.p.kfail.is_none() {
            self.p.kfail = Some(why);
        }
    }

    /// Where a value that depends on the element was first made: where the
    /// dry code stops for a branch on it (an expression the paths the kernel
    /// takes do not evaluate).
    fn prov(&self, v: &Val) -> Site {
        match v.load() {
            Some(n) => self.p.load_sites.get(n as usize - 1).copied().unwrap_or(Site::NONE),
            None => v.made(),
        }
    }

    // -- choices ----------------------------------------------------------------

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
    fn truth(&mut self, v: &Val) -> Option<bool> {
        match &v.kind {
            Kind::Const(c) => Some(*c != 0),
            Kind::Bool(k, neg) => Some((self.choose(k.as_str(), 2) == 0) != *neg),
            Kind::Inv(k) => Some(self.choose(k.as_str(), 2) == 0),
            _ => None,
        }
    }

    // -- registers ----------------------------------------------------------------

    fn read_loc(&mut self, loc: Loc, place: &Expr) -> Val {
        for (l, v) in self.p.writes.iter().rev() {
            if *l == loc {
                return v.clone();
            }
            if overlaps(l, &loc) {
                return Val::dep(Site::NONE);
            }
        }
        let x = Some(std::rc::Rc::new(place.clone()));
        if self.written.iter().any(|w| overlaps(w, &loc)) {
            Val::ptr(loc, Val::k(0), false).closed(x)
        } else {
            Val::inv(format!("reg{}:{}:{}", loc.0 .0, loc.1, loc.2)).closed(x)
        }
    }

    fn write_loc(&mut self, loc: Loc, v: Val) {
        if self.in_unit > 0 {
            self.unit_bad = true;
            return;
        }
        self.p.writes.retain(|(l, _)| !overlaps(l, &loc) || *l == loc);
        self.p.writes.retain(|(l, _)| *l != loc);
        self.p.writes.push((loc, v));
    }

    // -- closed forms ---------------------------------------------------------------

    /// A value's closed form, as an expression of type `ty`.
    pub(super) fn xof(&self, v: &Val, ty: &Ty, span: Span) -> Option<Expr> {
        match &v.kind {
            Kind::Const(c) => konst(*c, ty, span),
            Kind::Agg(vs) => match ty {
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
            _ => v.closed.as_ref().map(|x| (**x).clone()),
        }
    }

    /// `e`'s closed form: its kind with its children's closed forms.
    fn rebuild(&self, e: &Expr, kids: &[(&Val, &Expr)], make: impl FnOnce(Vec<Box<Expr>>) -> H) -> Option<std::rc::Rc<Expr>> {
        let mut xs = Vec::new();
        for (v, k) in kids {
            xs.push(Box::new(self.xof(v, &k.ty, k.span)?));
        }
        Some(rc(make(xs), &e.ty, e.span))
    }

    // -- statements -----------------------------------------------------------------

    pub(super) fn block(&mut self, locals: &[hir::Local], env: &mut HashMap<LocalId, Val>, b: &Block) -> Flow {
        for st in &b.stmts {
            if self.ended() {
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
    fn held(cv: &Val, neg: bool) -> Conv {
        match &cv.kind {
            Kind::Cond { conv, .. } => {
                if neg {
                    conv.not()
                } else {
                    *conv
                }
            }
            _ => Conv::Other,
        }
    }

    fn stmt(&mut self, locals: &[hir::Local], env: &mut HashMap<LocalId, Val>, st: &hir::Stmt) -> Flow {
        if self.in_unit > 0 {
            return self.stmt_in(locals, env, st);
        }
        let outer = self.site;
        self.site = Site::stmt(st);
        let f = self.stmt_in(locals, env, st);
        self.site = outer;
        f
    }

    fn stmt_in(&mut self, locals: &[hir::Local], env: &mut HashMap<LocalId, Val>, st: &hir::Stmt) -> Flow {
        match &st.kind {
            S::Let(t, Some(e)) => {
                let v = self.expr(locals, env, e);
                Self::bind(env, t, v);
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
                        if self.in_unit > 0 {
                            self.unit_bad = true;
                            return Flow::Next;
                        }
                        // A fault check (one arm always raises): the path
                        // takes the other; the dry code stops at one that
                        // need not hold between the first and the last
                        // element.
                        let t_raises = Self::diverges(t);
                        let e_raises = e.as_deref().is_some_and(Self::stmt_diverges);
                        if t_raises && !e_raises {
                            if !Self::held(&cv, true).interval() {
                                self.stop(At::Made(&cv));
                            }
                            match e {
                                Some(s) => self.stmt(locals, env, s),
                                None => Flow::Next,
                            }
                        } else if e_raises && !t_raises {
                            if !Self::held(&cv, false).interval() {
                                self.stop(At::Made(&cv));
                            }
                            self.block(locals, env, t)
                        } else {
                            self.end("a branch on what an element loaded or where it is", At::Made(&cv));
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
                    self.end("a long loop", At::Here);
                    return Flow::Next;
                }
                for i in *lo..*hi {
                    env.insert(*l, Val::k(i));
                    match self.block(locals, env, b) {
                        Flow::Next => {}
                        Flow::Break => break,
                        other => return other,
                    }
                }
                Flow::Next
            }
            S::Return(e) => Flow::Ret(match e {
                Some(e) => self.expr(locals, env, e),
                None => Val::unit(),
            }),
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
                self.space_write(locals, env, *sp, *bits, a, v);
                Flow::Next
            }
            S::Break => Flow::Break,
            S::OperandWrite(..) => {
                self.end("it writes an operand", At::Here);
                Flow::Next
            }
            _ => {
                self.end("a statement a stream does not make", At::Here);
                Flow::Next
            }
        }
    }

    fn space_write(&mut self, locals: &[hir::Local], env: &mut HashMap<LocalId, Val>, sp: DefId, bits: u32, a: &Expr, v: &Expr) {
        if self.in_unit > 0 {
            self.unit_bad = true;
            return;
        }
        let av = self.expr(locals, env, a);
        let vv = self.expr(locals, env, v);
        self.p.mem_writes.push((sp, av, vv, bits));
    }

    fn bind(env: &mut HashMap<LocalId, Val>, t: &LetTarget, v: Val) {
        match (t, v) {
            (LetTarget::Local(l), v) => {
                env.insert(*l, v);
            }
            (LetTarget::Tuple(ts), Val { kind: Kind::Agg(vs), .. }) if ts.len() == vs.len() => {
                for (t, v) in ts.iter().zip(vs) {
                    Self::bind(env, t, v);
                }
            }
            (LetTarget::Tuple(ts), v) => {
                for t in ts {
                    Self::bind(env, t, v.clone());
                }
            }
            (LetTarget::Discard, _) => {}
        }
    }

    /// The arm a match takes: for an invariant scrutinee, a choice.
    fn arm_of(&mut self, sv: &Val, arms: &[hir::Arm]) -> Option<usize> {
        let hit = |c: i128, p: &Pat| match p {
            Pat::Wildcard => true,
            Pat::Value(v) => *v == c,
            Pat::Range(a, b) => *a <= c && c <= *b,
            Pat::Variant(v) => *v as i128 == c,
            Pat::Bool(b) => *b as i128 == c,
        };
        match &sv.kind {
            Kind::Const(c) => arms.iter().position(|a| a.patterns.iter().any(|p| hit(*c, p))),
            Kind::Inv(_) | Kind::Bool(..) => Some(self.choose(&format!("match {}", sv.key()), arms.len() as u32) as usize),
            _ => {
                self.end("a match on what an element loaded", At::Made(sv));
                None
            }
        }
    }

    fn assign(&mut self, locals: &[hir::Local], env: &mut HashMap<LocalId, Val>, p: &Expr, v: Val) {
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
                match v.kind {
                    Kind::Agg(vs) if vs.len() == locs.len() => {
                        for (loc, x) in locs.into_iter().zip(vs) {
                            self.write_loc(loc, x);
                        }
                    }
                    _ => {
                        // (A field of a value computed whole: what it is
                        // computed from, not how.)
                        let each = match &v.kind {
                            Kind::Data { load, .. } => Val::data(*load, false),
                            _ if v.invariant() => Val::inv(format!("{}.*", v.key())),
                            _ => Val { closed: None, ..v.clone() },
                        };
                        for loc in locs {
                            self.write_loc(loc, each.clone());
                        }
                    }
                }
            }
            H::KindSelect(k, args) => match self.select_place(locals, env, *k, args) {
                Some(place) => self.assign(locals, env, &place, v),
                None => self.end("a write to an operand kind's register chosen by the element", At::Here),
            },
            H::Field(b, _) | H::Index(b, _) | H::Slice(b, ..) | H::TupleField(b, _) => {
                // Part of a register (a bundle's field): the whole of it,
                // varying.
                let mut e = &**b;
                loop {
                    match &e.kind {
                        H::Register(_) | H::RegField(..) | H::RegView(..) => {
                            for loc in self.g.bulk_locs(e) {
                                self.write_loc(loc, Val::dep(Site::NONE));
                            }
                            return;
                        }
                        H::Local(l) => {
                            env.insert(*l, Val::dep(Site::NONE));
                            return;
                        }
                        H::Field(x, _) | H::Index(x, _) | H::Slice(x, ..) | H::TupleField(x, _) => e = x,
                        _ => {
                            self.end("a write to part of something a stream does not write", At::Here);
                            return;
                        }
                    }
                }
            }
            _ => self.end("a write to a latch, the mode or a memory cell", At::Here),
        }
    }

    /// The register an operand kind's `select` rule picks for constant
    /// arguments.
    fn select_place(&mut self, locals: &[hir::Local], env: &mut HashMap<LocalId, Val>, k: DefId, args: &[Expr]) -> Option<Expr> {
        let rule = self.g.kinds[&k].rules.iter().find(|r| r.kind == hir::RuleKind::Select)?;
        let vals: Vec<Val> = args.iter().map(|a| self.expr(locals, env, a)).collect();
        let mut renv: HashMap<LocalId, Val> = HashMap::new();
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

    // -- expressions --------------------------------------------------------------

    pub(super) fn expr(&mut self, locals: &[hir::Local], env: &mut HashMap<LocalId, Val>, e: &Expr) -> Val {
        let mut v = self.expr_in(locals, env, e);
        // What this expression made untracked is made here.
        match &mut v.kind {
            Kind::Dep(s) | Kind::Cond { made: s, .. } if s.is_none() => *s = Site::expr(e),
            _ => {}
        }
        v
    }

    /// `e`'s value where its arms are chosen by an invariant `cv` in the
    /// `while` condition: both arms, merged (a branch in its closed form).
    fn merged(&mut self, e: &Expr, cv: &Val, c: &Expr, arms: Vec<(Val, Ty, Span)>, pats: Option<&[hir::Arm]>) -> Val {
        let first = arms[0].0.clone();
        let same = arms.iter().all(|a| a.0 == first);
        let xs: Option<Vec<Expr>> = arms.iter().map(|(v, t, s)| self.xof(v, t, *s)).collect();
        let x = match (self.xof(cv, &c.ty, c.span), xs) {
            (Some(cx), Some(xs)) => Some(match pats {
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
            }),
            _ => None,
        };
        if same && !matches!(first.kind, Kind::Agg(_)) {
            return first.closed(x);
        }
        if arms.iter().all(|a| a.0.invariant()) {
            let keys: Vec<String> = arms.iter().map(|a| a.0.key()).collect();
            return Val::inv(format!("(? {} {})", cv.key(), keys.join(" "))).closed(x);
        }
        if arms.iter().all(|a| a.0.invariant() || a.0.is_data()) {
            return Val::data(arms.iter().find_map(|a| a.0.load()).unwrap_or(0), false).closed(x);
        }
        Val::dep(Site::NONE)
    }

    fn expr_in(&mut self, locals: &[hir::Local], env: &mut HashMap<LocalId, Val>, e: &Expr) -> Val {
        if self.ended() {
            return Val::dep(Site::NONE);
        }
        let cond = self.phase == Phase::Cond;
        match &e.kind {
            H::Int(c) => Val::k(*c),
            H::Bool(b) => Val::k(*b as i128),
            H::Str(_) => Val::inv("str"),
            H::Local(l) => env.get(l).cloned().unwrap_or(Val::dep(Site::NONE)),
            H::Const(d) => Val::inv(format!("const{}", d.0)),
            H::Variant(_, v) => Val::k(*v as i128),
            H::Unary(op, a) => {
                let av = self.expr(locals, env, a);
                let x = self.rebuild(e, &[(&av, a)], |mut xs| H::Unary(*op, xs.remove(0)));
                let v = match (op, &av.kind) {
                    (UnaryOp::Not, Kind::Const(c)) => Val::k((*c == 0) as i128),
                    (UnaryOp::Not, Kind::Bool(k, n)) => Val::boolean(k.as_str(), !n),
                    (UnaryOp::Not, Kind::Inv(k)) => Val::boolean(k.as_str(), true),
                    (UnaryOp::Not, Kind::Cond { conv, made, .. }) => Val::cond(conv.not(), None, *made),
                    (_, Kind::Const(c)) => Val::inv(format!("({op:?} {c} {:?})", e.ty)),
                    _ if av.invariant() => Val::inv(format!("({op:?} {})", av.key())),
                    (_, Kind::Data { load, .. }) => Val::data(*load, false),
                    _ => Val::dep(made_of(&[&av])),
                };
                v.closed(x)
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
                if let Kind::Ptr { loc, off, wraps } = &mut v.kind {
                    *wraps = inner || *wraps || !fits(loc, off, &e.ty);
                }
                v.closed(x)
            }
            H::Ternary(c, a, b) => {
                let cv = self.expr(locals, env, c);
                if cond && matches!(cv.kind, Kind::Inv(_) | Kind::Bool(..)) {
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
                        let plain = |v: &Val| v.invariant() || v.is_data();
                        if av == bv {
                            av
                        } else if cv.is_data() && plain(&av) && plain(&bv) {
                            // A choice of what was loaded (a value, not a way
                            // through the code).
                            let x = self.rebuild(e, &[(&cv, c), (&av, a), (&bv, b)], |mut xs| {
                                let y = xs.pop().unwrap();
                                let x = xs.pop().unwrap();
                                H::Ternary(xs.pop().unwrap(), x, y)
                            });
                            Val::data(cv.load().unwrap_or(0), false).closed(x)
                        } else {
                            // Which arm depends on where the element is: the
                            // dry code stops where that was made.
                            self.stop(At::Made(&cv));
                            Val::dep(Site::NONE)
                        }
                    }
                }
            }
            // A cast keeps a pointer a pointer (where it may wrap, noted), and
            // a load a load.
            H::Cast(a) => {
                let av = self.expr(locals, env, a);
                let x = self.rebuild(e, &[(&av, a)], |mut xs| H::Cast(xs.remove(0)));
                let v = match &av.kind {
                    Kind::Const(c) => Val::k(cast_k(*c, &e.ty)),
                    Kind::Inv(k) => Val::inv(format!("({} as {:?})", k.as_str(), e.ty)),
                    Kind::Bool(..) => Val::inv(format!("({} as {:?})", av.key(), e.ty)),
                    Kind::Ptr { loc, off, wraps } => Val::ptr(*loc, (**off).clone(), *wraps || !fits(loc, off, &e.ty)),
                    _ => Val { closed: None, ..av },
                };
                v.closed(x)
            }
            H::Field(b, i) | H::TupleField(b, i) => {
                let bv = self.expr(locals, env, b);
                let tuple = matches!(e.kind, H::TupleField(..));
                let x = self.rebuild(e, &[(&bv, b)], |mut xs| if tuple { H::TupleField(xs.remove(0), *i) } else { H::Field(xs.remove(0), *i) });
                match &bv.kind {
                    Kind::Agg(vs) => vs.get(*i as usize).cloned().unwrap_or(Val::dep(Site::NONE)),
                    _ if bv.invariant() => Val::inv(format!("{}.{i}", bv.key())).closed(x),
                    Kind::Data { load, .. } => Val::data(*load, false).closed(x),
                    _ => Val::dep(made_of(&[&bv])),
                }
            }
            H::Index(b, i) => {
                let bv = self.expr(locals, env, b);
                let iv = self.expr(locals, env, i);
                let x = self.rebuild(e, &[(&bv, b), (&iv, i)], |mut xs| {
                    let r = xs.pop().unwrap();
                    H::Index(xs.pop().unwrap(), r)
                });
                let plain = |v: &Val| v.invariant() || v.is_data();
                let v = match (bv.konst(), iv.konst()) {
                    (Some(x), Some(n)) => Val::k(if (0..128).contains(&n) { (x >> n) & 1 } else { 0 }),
                    _ if bv.invariant() && iv.invariant() => Val::inv(format!("{}[{}]", bv.key(), iv.key())),
                    _ if plain(&bv) && plain(&iv) => Val::data(bv.load().or(iv.load()).unwrap_or(0), false),
                    _ => Val::dep(made_of(&[&bv, &iv])),
                };
                v.closed(x)
            }
            H::Slice(b, hi, lo) => {
                let bv = self.expr(locals, env, b);
                let x = self.rebuild(e, &[(&bv, b)], |mut xs| H::Slice(xs.remove(0), *hi, *lo));
                let v = match &bv.kind {
                    Kind::Const(x) => Val::k((x >> lo) & ((1i128 << (hi - lo + 1)) - 1)),
                    _ if bv.invariant() => Val::inv(format!("{}[{hi}:{lo}]", bv.key())),
                    Kind::Data { load, .. } => Val::data(*load, false),
                    _ => Val::dep(made_of(&[&bv])),
                };
                v.closed(x)
            }
            H::Builtin(Builtin::Restarted, _) if self.phase == (Phase::Side { steady: true }) => Val::k(1),
            H::Call(u, args) => self.unit_call(locals, env, e, *u, args),
            H::Builtin(bi, args) => {
                let vs: Vec<Val> = args.iter().map(|a| self.expr(locals, env, a)).collect();
                let kids: Vec<(&Val, &Expr)> = vs.iter().zip(args).collect();
                let x = self.rebuild(e, &kids, |xs| H::Builtin(*bi, xs.into_iter().map(|b| *b).collect()));
                Self::pure_of(&format!("{bi:?}:{:?}", e.ty), &vs).closed(x)
            }
            H::BundleLit(_, args) | H::Tuple(args) | H::Array(args) => Kind::Agg(args.iter().map(|a| self.expr(locals, env, a)).collect()).into(),
            H::ArrayRepeat(a, n) => {
                let v = self.expr(locals, env, a);
                Kind::Agg(vec![v; (*n).min(64) as usize]).into()
            }
            H::Match(s, arms) => {
                let sv = self.expr(locals, env, s);
                if cond && matches!(sv.kind, Kind::Inv(_) | Kind::Bool(..)) && arms.iter().all(|a| matches!(a.body, ArmBody::Expr(_))) {
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
                    None => Val::dep(Site::NONE),
                }
            }
            H::If(c, t, f) => {
                let cv = self.expr(locals, env, c);
                // In the `while` condition, `if inv { a } else { b }`, an
                // expression each arm: both, a branch in its closed form.
                if cond && matches!(cv.kind, Kind::Inv(_) | Kind::Bool(..)) && t.stmts.len() == 1 {
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
                        None => Val::unit(),
                    },
                    None => {
                        self.end("a branch on what an element loaded", At::Made(&cv));
                        Val::dep(Site::NONE)
                    }
                }
            }
            H::Block(b) => self.block_value(locals, env, b),
            H::Register(_) | H::RegField(..) => match self.g.bulk_locs(e).as_slice() {
                [l] => self.read_loc(*l, e),
                _ => Val::dep(Site::NONE),
            },
            H::RegView(..) => {
                let locs = self.g.bulk_locs(e);
                // (Each field's closed form is not the view's.)
                Kind::Agg(locs.into_iter().map(|l| Val { closed: None, ..self.read_loc(l, e) }).collect()).into()
            }
            H::Mode(_) => Val::inv("mode").closed(Some(std::rc::Rc::new(e.clone()))),
            H::Latch(d) => Val::inv(format!("latch{}", d.0)).closed(Some(std::rc::Rc::new(e.clone()))),
            H::OperandRead(i) | H::OperandRule(i, _) => Val::inv(format!("op{i}")),
            H::OperandField(b, i) => {
                let v = self.expr(locals, env, b);
                Val::inv(format!("{}.f{i}", v.key()))
            }
            H::OperandTail(b) | H::TailOperand(b, _) => {
                let v = self.expr(locals, env, b);
                Val::inv(format!("{}.tail", v.key()))
            }
            H::SpaceRead(sp, bits, a) => {
                if self.in_unit > 0 {
                    self.unit_bad = true;
                    return Val::dep(Site::NONE);
                }
                let av = self.expr(locals, env, a);
                self.p.mem_reads.push((*sp, av, *bits, e.ty.clone()));
                self.p.load_sites.push(Site::expr(e));
                let n = self.p.mem_reads.len() as u32;
                // (Its closed form: the local `ld<n>`.)
                Val::data(n, true).closed(Some(rc(H::Local(n - 1), &e.ty, e.span)))
            }
            H::SpaceWrite(sp, bits, a, v) => {
                self.space_write(locals, env, *sp, *bits, a, v);
                Val::unit()
            }
            H::KindSelect(k, args) => match self.select_place(locals, env, *k, args) {
                Some(place) => self.expr(locals, env, &place),
                None => Val::dep(Site::NONE),
            },
            H::FnCall(d, args) => {
                let vs: Vec<Val> = args.iter().map(|a| self.expr(locals, env, a)).collect();
                self.call(*d, vs)
            }
            _ => {
                self.end("an expression a stream does not make", At::Here);
                Val::dep(Site::NONE)
            }
        }
    }

    /// A pure computation (a builtin, a unit not followed) of these values.
    fn pure_of(name: &str, vs: &[Val]) -> Val {
        if vs.iter().all(Val::invariant) {
            Val::inv(format!("{name}({})", vs.iter().map(Val::key).collect::<Vec<_>>().join(",")))
        } else if vs.iter().all(|v| v.invariant() || v.is_data()) {
            Val::data(vs.iter().find_map(Val::load).unwrap_or(0), false)
        } else {
            Val::dep(made_of(&vs.iter().collect::<Vec<_>>()))
        }
    }

    /// A unit's call: its body followed for its value (its closed form the
    /// call, a part of it the call's part); else computed from its
    /// arguments.
    fn unit_call(&mut self, locals: &[hir::Local], env: &mut HashMap<LocalId, Val>, e: &Expr, u: hir::UnitInstId, args: &[Expr]) -> Val {
        let vs: Vec<Val> = args.iter().map(|a| self.expr(locals, env, a)).collect();
        let kids: Vec<(&Val, &Expr)> = vs.iter().zip(args).collect();
        let x = self.rebuild(e, &kids, |xs| H::Call(u, xs.into_iter().map(|b| *b).collect()));
        match self.unit(u, vs.clone()) {
            Some(Val { kind: Kind::Agg(parts), .. }) => {
                let tys: Option<Vec<Ty>> = match &e.ty {
                    Ty::Bundle(d) => self.g.bundles.get(d).map(|bd| bd.fields.iter().map(|f| f.ty.clone()).collect()),
                    Ty::Tuple(ts) => Some(ts.iter().map(|t| t.1.clone()).collect()),
                    _ => None,
                };
                let bundle = matches!(e.ty, Ty::Bundle(_));
                let parts = parts
                    .into_iter()
                    .enumerate()
                    .map(|(k, p)| {
                        let px = match (&x, &tys) {
                            (Some(x), Some(tys)) if k < tys.len() => {
                                let whole = Box::new((**x).clone());
                                Some(rc(if bundle { H::Field(whole, k as u32) } else { H::TupleField(whole, k as u32) }, &tys[k], e.span))
                            }
                            _ => None,
                        };
                        p.closed(px)
                    })
                    .collect();
                Kind::Agg(parts).into()
            }
            Some(v) => v.closed(x),
            None => Self::pure_of(&format!("unit{u:?}:{:?}", e.ty), &vs).closed(x),
        }
    }

    /// A unit's value for these arguments, its body followed (None where it
    /// does what the evaluation does not follow).
    fn unit(&mut self, u: hir::UnitInstId, args: Vec<Val>) -> Option<Val> {
        let inst = self.g.hir.units.get(u.0 as usize)?;
        if self.depth > 24 {
            return None;
        }
        self.depth += 1;
        self.in_unit += 1;
        let outer = std::mem::replace(&mut self.unit_bad, false);
        let mut env: HashMap<LocalId, Val> = HashMap::new();
        for (p, v) in inst.params.iter().zip(args) {
            env.insert(*p, v);
        }
        let r = match self.block(&inst.locals, &mut env, &inst.body) {
            Flow::Ret(v) => Some(v),
            _ => None,
        };
        let bad = std::mem::replace(&mut self.unit_bad, outer);
        self.in_unit -= 1;
        self.depth -= 1;
        if bad {
            None
        } else {
            r
        }
    }

    fn block_value(&mut self, locals: &[hir::Local], env: &mut HashMap<LocalId, Val>, b: &Block) -> Val {
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
                _ => return Val::dep(Site::NONE),
            }
        }
        Val::unit()
    }

    fn call(&mut self, d: DefId, args: Vec<Val>) -> Val {
        let Some(f) = self.g.fns.get(&d) else { return Val::dep(Site::NONE) };
        if f.external {
            // (Supplied outside: the dry code would call it.)
            self.stop(At::Here);
            return Val::dep(Site::NONE);
        }
        if self.depth > 24 {
            self.end("calls too deep", At::Here);
            return Val::dep(Site::NONE);
        }
        self.depth += 1;
        self.at.push(f.name.clone());
        let mut env: HashMap<LocalId, Val> = HashMap::new();
        for (p, v) in f.params.iter().zip(args) {
            env.insert(*p, v);
        }
        let r = match self.block(&f.locals, &mut env, &f.body) {
            Flow::Ret(v) => v,
            Flow::Raise => {
                // A fn that raises here (a check whose condition held): the
                // path raises (and so does the dry code).
                if self.phase == Phase::Element {
                    if self.p.fail.is_none() {
                        self.p.fail = Some(format!("it raises in `{}`", f.name));
                    }
                    self.p.raised = true;
                } else {
                    self.end("it raises", At::Here);
                }
                Val::dep(Site::NONE)
            }
            _ => Val::unit(),
        };
        self.at.pop();
        self.depth -= 1;
        r
    }

    fn binary(&mut self, op: BinaryOp, a: Val, b: Val) -> Val {
        use BinaryOp as B;
        let cmp = matches!(op, B::Eq | B::Ne | B::Lt | B::Le | B::Gt | B::Ge);
        let logic = matches!(op, B::And | B::Or);
        // `&&` and `||` with a side that is a choice: as it falls (not in
        // the `while` condition's closed form, where both stay).
        if logic && !(a.invariant() && b.invariant()) && self.phase != Phase::Cond {
            for (x, y) in [(&a, &b), (&b, &a)] {
                if x.invariant() {
                    if let Some(t) = self.truth(x) {
                        return match (op, t) {
                            (B::Or, true) => Val::k(1),
                            (B::And, false) => Val::k(0),
                            _ => y.clone(),
                        };
                    }
                }
            }
        }
        if let (Some(x), Some(y)) = (a.konst(), b.konst()) {
            return Val::k(fold(op, x, y));
        }
        if a.invariant() && b.invariant() {
            // Short-circuit forms keep a choice's key.
            return match (op, a.konst(), b.konst()) {
                (B::And, Some(0), _) | (B::And, _, Some(0)) => Val::k(0),
                (B::Or, Some(x), _) | (B::Or, _, Some(x)) if x != 0 => Val::k(1),
                (B::And | B::Or, Some(_), _) => Val { closed: None, ..b },
                (B::And | B::Or, _, Some(_)) => Val { closed: None, ..a },
                _ if cmp || logic => Val::boolean(format!("({} {op:?} {})", a.key(), b.key()), false),
                _ => Val::inv(format!("({} {op:?} {})", a.key(), b.key())),
            };
        }
        match (&a.kind, &b.kind) {
            // A pointer moved by an invariant; masked to its register's
            // width (it wraps there).
            (Kind::Ptr { loc, off, .. }, _) if b.invariant() && op == B::Add => return Val::ptr(*loc, self.binary(B::Add, (**off).clone(), b.clone()), false),
            (_, Kind::Ptr { loc, off, .. }) if a.invariant() && op == B::Add => return Val::ptr(*loc, self.binary(B::Add, (**off).clone(), a.clone()), false),
            (Kind::Ptr { loc, off, .. }, _) if b.invariant() && op == B::Sub => return Val::ptr(*loc, self.binary(B::Sub, (**off).clone(), b.clone()), false),
            (Kind::Ptr { loc, off, .. }, Kind::Const(_)) if op == B::BitAnd => return Val::ptr(*loc, (**off).clone(), true),
            _ => {}
        }
        // Where the element is, compared with an invariant.
        let ptr = |v: &Val| matches!(v.kind, Kind::Ptr { .. });
        if cmp && ((ptr(&a) && b.invariant()) || (ptr(&b) && a.invariant())) {
            let conv = if a.wraps() || b.wraps() {
                Conv::Other
            } else {
                match op {
                    B::Eq => Conv::Convex,
                    B::Ne => Conv::Concave,
                    _ => Conv::Half,
                }
            };
            return Val::cond(conv, Some((op, a, b)), Site::NONE);
        }
        if let (Kind::Cond { conv: ca, .. }, Kind::Cond { conv: cb, .. }) = (&a.kind, &b.kind) {
            if logic {
                return Val::cond(if op == B::And { ca.and(*cb) } else { ca.or(*cb) }, None, Site::NONE);
            }
        }
        // What was loaded, with invariants: still that.
        let plain = |v: &Val| v.invariant() || v.is_data();
        if plain(&a) && plain(&b) {
            return Val::data(a.load().or(b.load()).unwrap_or(0), false);
        }
        let place = |v: &Val| matches!(v.kind, Kind::Ptr { .. } | Kind::Cond { .. });
        if (cmp || logic) && (place(&a) || place(&b)) {
            return Val::cond(Conv::Other, None, made_of(&[&a, &b]));
        }
        Val::dep(made_of(&[&a, &b]))
    }
}
