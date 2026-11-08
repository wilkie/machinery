//! The values the evaluation computes: what kind of thing each is (the same
//! every element, where the element is, what it loaded), and how it is
//! computed (its closed form), kept apart.

use std::rc::Rc;

use machine_sema::hir::{Expr, ExprKind as H};
use machine_sema::ty::Ty;
use machine_syntax::ast::BinaryOp;
use machine_syntax::span::Span;

use super::Site;
use machine_sema::program::DefId;

/// A place in a register: the register, its first bit and width.
pub(super) type Loc = (DefId, u32, u32);

/// What an invariant is, by where it came from (`CR0.PG == 0` the same key
/// wherever it is tested): a condition met twice gets the same answer.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) struct Key(Rc<str>);

impl Key {
    pub(super) fn new(s: impl Into<String>) -> Key {
        Key(Rc::from(s.into()))
    }
    pub(super) fn as_str(&self) -> &str {
        &self.0
    }
}

/// Where a condition on where the element is holds, over a run of elements
/// whose pointers move without wrapping: on an interval of them, the rest
/// being one too (a comparison of a pointer with an invariant); on an
/// interval (a conjunction of those, an equality); outside one (a
/// disjunction, an inequality); or anywhere.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Conv {
    Half,
    Convex,
    Concave,
    Other,
}

impl Conv {
    pub(super) fn not(self) -> Conv {
        match self {
            Conv::Convex => Conv::Concave,
            Conv::Concave => Conv::Convex,
            c => c,
        }
    }
    /// It holds on an interval of elements: holding at the first and the
    /// last, it holds between.
    pub(super) fn interval(self) -> bool {
        matches!(self, Conv::Half | Conv::Convex)
    }
    pub(super) fn and(self, b: Conv) -> Conv {
        if self.interval() && b.interval() {
            Conv::Convex
        } else {
            Conv::Other
        }
    }
    pub(super) fn or(self, b: Conv) -> Conv {
        let outside = |c: Conv| matches!(c, Conv::Half | Conv::Concave);
        if outside(self) && outside(b) {
            Conv::Concave
        } else {
            Conv::Other
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) enum Kind {
    Const(i128),
    /// The same every element; a condition, with whether it is negated
    /// (`!c` is `c`'s choice).
    Inv(Key),
    Bool(Key, bool),
    /// A register as the element found it, plus an invariant (or constant):
    /// a pointer; and whether it may have wrapped (overflowed its type).
    Ptr { loc: Loc, off: Box<Val>, wraps: bool },
    /// A condition on where the element is; the comparison, when it is one;
    /// the expression that made it.
    Cond { conv: Conv, cmp: Option<Box<(BinaryOp, Val, Val)>>, made: Site },
    /// What the element loaded (its nth load; `loaded`: that value itself),
    /// or computed from that and invariants alone.
    Data { load: u32, loaded: bool },
    /// Anything else, and the expression that made it.
    Dep(Site),
    Agg(Vec<Val>),
    Unit,
}

/// A value: its kind, and its closed form where it has one (a description
/// expression over the state the element starts in and the values it
/// loaded, the locals `ld<n>`). Two values are equal when their kinds are.
#[derive(Clone)]
pub(super) struct Val {
    pub kind: Kind,
    pub closed: Option<Rc<Expr>>,
}

impl PartialEq for Val {
    fn eq(&self, o: &Val) -> bool {
        self.kind == o.kind
    }
}

impl std::fmt::Debug for Val {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.kind.fmt(f)
    }
}

impl From<Kind> for Val {
    fn from(kind: Kind) -> Val {
        Val { kind, closed: None }
    }
}

impl Val {
    pub(super) fn k(c: i128) -> Val {
        Kind::Const(c).into()
    }
    pub(super) fn inv(key: impl Into<String>) -> Val {
        Kind::Inv(Key::new(key)).into()
    }
    pub(super) fn boolean(key: impl Into<String>, neg: bool) -> Val {
        Kind::Bool(Key::new(key), neg).into()
    }
    pub(super) fn data(load: u32, loaded: bool) -> Val {
        Kind::Data { load, loaded }.into()
    }
    pub(super) fn dep(made: Site) -> Val {
        Kind::Dep(made).into()
    }
    pub(super) fn unit() -> Val {
        Kind::Unit.into()
    }
    pub(super) fn ptr(loc: Loc, off: Val, wraps: bool) -> Val {
        Kind::Ptr { loc, off: Box::new(off), wraps }.into()
    }
    pub(super) fn cond(conv: Conv, cmp: Option<(BinaryOp, Val, Val)>, made: Site) -> Val {
        Kind::Cond { conv, cmp: cmp.map(Box::new), made }.into()
    }
    /// With this closed form (where there is one).
    pub(super) fn closed(mut self, x: Option<Rc<Expr>>) -> Val {
        if x.is_some() && !matches!(self.kind, Kind::Const(_) | Kind::Agg(_) | Kind::Unit) {
            self.closed = x;
        }
        self
    }
    pub(super) fn konst(&self) -> Option<i128> {
        match self.kind {
            Kind::Const(c) => Some(c),
            _ => None,
        }
    }
    pub(super) fn invariant(&self) -> bool {
        matches!(self.kind, Kind::Const(_) | Kind::Inv(_) | Kind::Bool(..))
    }
    /// What the element loaded, and what it computed from that alone.
    pub(super) fn is_data(&self) -> bool {
        matches!(self.kind, Kind::Data { .. })
    }
    pub(super) fn load(&self) -> Option<u32> {
        match self.kind {
            Kind::Data { load, .. } => Some(load),
            _ => None,
        }
    }
    pub(super) fn wraps(&self) -> bool {
        matches!(self.kind, Kind::Ptr { wraps: true, .. })
    }
    /// An invariant's key (with its polarity), or a description of the
    /// value.
    pub(super) fn key(&self) -> String {
        match &self.kind {
            Kind::Const(c) => c.to_string(),
            Kind::Inv(k) => k.as_str().to_string(),
            Kind::Bool(k, neg) => {
                if *neg {
                    format!("!{}", k.as_str())
                } else {
                    k.as_str().to_string()
                }
            }
            other => format!("{other:?}"),
        }
    }
    /// Where something untracked in it was made.
    pub(super) fn made(&self) -> Site {
        match &self.kind {
            Kind::Dep(s) | Kind::Cond { made: s, .. } => *s,
            _ => Site::NONE,
        }
    }
}

/// Where the first of these values that came from something untracked was
/// made.
pub(super) fn made_of(vs: &[&Val]) -> Site {
    vs.iter().map(|v| v.made()).find(|s| !s.is_none()).unwrap_or(Site::NONE)
}

pub(super) fn ty_bits(t: &Ty) -> u32 {
    match t {
        Ty::Bool => 1,
        Ty::UInt(n) | Ty::SInt(n) => *n,
        _ => 128,
    }
}

pub(super) fn overlaps(a: &Loc, b: &Loc) -> bool {
    a.0 == b.0 && a.1 < b.1 + b.2 && b.1 < a.1 + a.2
}

/// Whether a register's value plus `off`, as a `ty`, cannot wrap.
pub(super) fn fits(l: &Loc, off: &Val, ty: &Ty) -> bool {
    let Some(c) = off.konst() else { return false };
    let (w, signed) = match ty {
        Ty::UInt(n) => (*n, false),
        Ty::SInt(n) => (*n, true),
        _ => return false,
    };
    if l.2 >= 120 || w >= 120 {
        return false;
    }
    let limit: i128 = if signed { 1 << (w - 1) } else { 1 << w };
    c >= 0 && (1i128 << l.2) - 1 + c < limit
}

/// A constant cast to `ty`.
pub(super) fn cast_k(c: i128, ty: &Ty) -> i128 {
    match ty {
        Ty::Bool => (c != 0) as i128,
        Ty::UInt(n) if *n < 127 => c & ((1i128 << n) - 1),
        Ty::SInt(n) if *n < 127 => (c << (128 - n)) >> (128 - n),
        _ => c,
    }
}

pub(super) fn rc(kind: H, ty: &Ty, span: Span) -> Rc<Expr> {
    Rc::new(Expr { kind, ty: ty.clone(), span })
}

/// A constant of type `ty`, as the description would write it.
pub(super) fn konst(c: i128, ty: &Ty, span: Span) -> Option<Expr> {
    let kind = match ty {
        Ty::Bool => H::Bool(c != 0),
        Ty::UInt(_) | Ty::SInt(_) => H::Int(cast_k(c, ty)),
        Ty::Enum(d) => H::Variant(*d, c as u32),
        _ => return None,
    };
    Some(Expr { kind, ty: ty.clone(), span })
}

pub(super) fn fold(op: BinaryOp, x: i128, y: i128) -> i128 {
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
