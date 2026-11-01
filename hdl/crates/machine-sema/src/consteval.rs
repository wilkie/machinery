//! Compile-time evaluation of integer and boolean expressions: widths,
//! array lengths, generic arguments, enum variant values, match patterns.

use machine_syntax::ast::{BinaryOp, Expr, ExprKind, UnaryOp};
use machine_syntax::span::Span;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConstVal {
    Int(i128),
    Bool(bool),
}

impl ConstVal {
    pub fn as_int(self) -> Option<i128> {
        match self {
            ConstVal::Int(v) => Some(v),
            ConstVal::Bool(_) => None,
        }
    }
}

/// Why an expression could not be evaluated.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConstError {
    pub span: Span,
    pub message: String,
    /// True when a diagnostic has already been emitted for the underlying
    /// failure (for example a `const` item that failed to evaluate), so the
    /// caller should not report again.
    pub reported: bool,
}

/// The environment a const expression is evaluated in. Implemented by the
/// checker, which knows generic parameters, `const` items and enum variants.
pub trait ConstEnv {
    /// Value of a single name (generic parameter or `const` item).
    fn lookup_name(&mut self, name: &str, span: Span) -> Result<ConstVal, ConstError>;
    /// Value of a two-segment path `Enum::Variant` (the variant's encoding).
    fn lookup_variant(
        &mut self,
        enum_name: &str,
        variant: &str,
        span: Span,
    ) -> Result<ConstVal, ConstError>;
    /// Width in bits of a cast target (`as u<N>`), for truncation.
    fn cast_width(&mut self, ty: &machine_syntax::ast::Type) -> Result<(bool, u32), ConstError>;
}

pub fn eval(expr: &Expr, env: &mut dyn ConstEnv) -> Result<ConstVal, ConstError> {
    let err = |msg: &str| ConstError {
        span: expr.span,
        message: msg.to_string(),
        reported: false,
    };
    match &expr.kind {
        ExprKind::Int { value, .. } => {
            if *value > i128::MAX as u128 {
                return Err(err("constant too large"));
            }
            Ok(ConstVal::Int(*value as i128))
        }
        ExprKind::Bool(b) => Ok(ConstVal::Bool(*b)),
        ExprKind::Path(p) => match p.segments.len() {
            1 => env.lookup_name(&p.segments[0].name, p.span),
            2 => env.lookup_variant(&p.segments[0].name, &p.segments[1].name, p.span),
            _ => Err(err("not a constant")),
        },
        ExprKind::Unary(op, inner) => {
            let v = eval(inner, env)?;
            match (op, v) {
                (UnaryOp::Neg, ConstVal::Int(v)) => v
                    .checked_neg()
                    .map(ConstVal::Int)
                    .ok_or_else(|| err("overflow")),
                (UnaryOp::BitNot, ConstVal::Int(v)) => Ok(ConstVal::Int(!v)),
                (UnaryOp::Not, ConstVal::Bool(b)) => Ok(ConstVal::Bool(!b)),
                _ => Err(err("operator does not apply to this constant")),
            }
        }
        ExprKind::Binary(op, a, b) => {
            let a = eval(a, env)?;
            let b = eval(b, env)?;
            binary(*op, a, b).ok_or_else(|| {
                err("invalid constant operation (overflow, division by zero or type mismatch)")
            })
        }
        ExprKind::Ternary(c, a, b) => match eval(c, env)? {
            ConstVal::Bool(true) => eval(a, env),
            ConstVal::Bool(false) => eval(b, env),
            ConstVal::Int(_) => Err(err("condition must be a boolean constant")),
        },
        ExprKind::Cast(inner, ty) => {
            let v = eval(inner, env)?
                .as_int()
                .ok_or_else(|| err("only integers can be cast"))?;
            let (signed, w) = env.cast_width(ty)?;
            Ok(ConstVal::Int(truncate(v, signed, w)))
        }
        ExprKind::Block(b) if b.stmts.is_empty() => Err(err("empty block is not a constant")),
        _ => Err(err("not a constant expression")),
    }
}

fn binary(op: BinaryOp, a: ConstVal, b: ConstVal) -> Option<ConstVal> {
    use ConstVal::*;
    Some(match (op, a, b) {
        (BinaryOp::Add, Int(a), Int(b)) => Int(a.checked_add(b)?),
        (BinaryOp::Sub, Int(a), Int(b)) => Int(a.checked_sub(b)?),
        (BinaryOp::Mul, Int(a), Int(b)) => Int(a.checked_mul(b)?),
        (BinaryOp::Div, Int(a), Int(b)) => Int(a.checked_div(b)?),
        (BinaryOp::Rem, Int(a), Int(b)) => Int(a.checked_rem(b)?),
        (BinaryOp::Shl, Int(a), Int(b)) => Int(a.checked_shl(u32::try_from(b).ok()?)?),
        (BinaryOp::Shr, Int(a), Int(b)) => Int(a.checked_shr(u32::try_from(b).ok()?)?),
        (BinaryOp::BitAnd, Int(a), Int(b)) => Int(a & b),
        (BinaryOp::BitOr, Int(a), Int(b)) => Int(a | b),
        (BinaryOp::BitXor, Int(a), Int(b)) => Int(a ^ b),
        (BinaryOp::Eq, Int(a), Int(b)) => Bool(a == b),
        (BinaryOp::Ne, Int(a), Int(b)) => Bool(a != b),
        (BinaryOp::Lt, Int(a), Int(b)) => Bool(a < b),
        (BinaryOp::Le, Int(a), Int(b)) => Bool(a <= b),
        (BinaryOp::Gt, Int(a), Int(b)) => Bool(a > b),
        (BinaryOp::Ge, Int(a), Int(b)) => Bool(a >= b),
        (BinaryOp::Eq, Bool(a), Bool(b)) => Bool(a == b),
        (BinaryOp::Ne, Bool(a), Bool(b)) => Bool(a != b),
        (BinaryOp::And, Bool(a), Bool(b)) => Bool(a && b),
        (BinaryOp::Or, Bool(a), Bool(b)) => Bool(a || b),
        _ => return None,
    })
}

/// Truncate (or sign-extend) `v` to a `w`-bit value of the given signedness.
pub fn truncate(v: i128, signed: bool, w: u32) -> i128 {
    if w >= 128 {
        return v;
    }
    let mask = (1i128 << w) - 1;
    let u = v & mask;
    if signed && w > 0 && (u >> (w - 1)) & 1 == 1 {
        u - (1i128 << w)
    } else {
        u
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncation() {
        assert_eq!(truncate(0x1FF, false, 8), 0xFF);
        assert_eq!(truncate(0xFF, true, 8), -1);
        assert_eq!(truncate(0x7F, true, 8), 127);
        assert_eq!(truncate(-1, false, 4), 15);
    }
}
