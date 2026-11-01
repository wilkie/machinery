//! Resolved types.

use std::fmt;

use crate::program::DefId;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Ty {
    /// `u<N>`
    UInt(u32),
    /// `i<N>`
    SInt(u32),
    Bool,
    /// An `enum` or `mode` item.
    Enum(DefId),
    Bundle(DefId),
    Union(DefId),
    /// `[T; N]`
    Array(Box<Ty>, u64),
    /// Positional or named tuple. Named tuples are unit return types.
    Tuple(Vec<(Option<String>, Ty)>),
    /// A compile-time integer with a known value. Unsuffixed literals and
    /// const-parameter expressions have this type; it coerces to any integer
    /// type the value fits in.
    Const(i128),
    /// A fetched instruction operand (`operand ModRM { ... }`).
    Operand(DefId),
    /// The `tail` of an operand: whichever operand followed it, or none.
    OperandTail(DefId),
    /// String literals (metadata and assertion messages only).
    Str,
    /// The type of statements-as-expressions (`assert(...)`).
    Unit,
    /// The type of `unreachable(...)` and `raise`: unifies with anything.
    Never,
    /// Produced after an error; suppresses cascades.
    Error,
}

impl Ty {
    pub fn is_integer(&self) -> bool {
        matches!(self, Ty::UInt(_) | Ty::SInt(_) | Ty::Const(_))
    }

    pub fn is_error(&self) -> bool {
        matches!(self, Ty::Error)
    }

    /// Does a compile-time value fit in this integer type?
    pub fn fits(&self, v: i128) -> bool {
        match self {
            Ty::UInt(w) => v >= 0 && (*w >= 128 || v < (1i128 << *w)),
            Ty::SInt(w) => {
                let w = *w as i128;
                if w >= 128 {
                    true
                } else {
                    v >= -(1i128 << (w - 1)) && v < (1i128 << (w - 1))
                }
            }
            Ty::Const(_) => true,
            _ => false,
        }
    }
}

/// Renders a type for diagnostics; needs the program for names of
/// user-defined types.
pub struct TyDisplay<'a> {
    pub ty: &'a Ty,
    pub names: &'a dyn Fn(DefId) -> String,
}

impl fmt::Display for TyDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_ty(self.ty, self.names, f)
    }
}

pub fn fmt_ty(ty: &Ty, names: &dyn Fn(DefId) -> String, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match ty {
        Ty::UInt(w) => write!(f, "u{w}"),
        Ty::SInt(w) => write!(f, "i{w}"),
        Ty::Bool => write!(f, "bool"),
        Ty::Enum(d) | Ty::Bundle(d) | Ty::Union(d) | Ty::Operand(d) => write!(f, "{}", names(*d)),
        Ty::OperandTail(d) => write!(f, "tail of {}", names(*d)),
        Ty::Array(t, n) => {
            write!(f, "[")?;
            fmt_ty(t, names, f)?;
            write!(f, "; {n}]")
        }
        Ty::Tuple(items) => {
            write!(f, "(")?;
            for (i, (name, t)) in items.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                if let Some(n) = name {
                    write!(f, "{n}: ")?;
                }
                fmt_ty(t, names, f)?;
            }
            write!(f, ")")
        }
        Ty::Const(v) => write!(f, "const {v}"),
        Ty::Str => write!(f, "str"),
        Ty::Unit => write!(f, "()"),
        Ty::Never => write!(f, "!"),
        Ty::Error => write!(f, "<error>"),
    }
}
