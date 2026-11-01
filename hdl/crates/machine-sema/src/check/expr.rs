//! Expression checking. Bidirectional: `check_expr` takes the expected type
//! when the context knows it, which is how unsuffixed literals and
//! compile-time constants get their widths.

use machine_syntax::ast::{self, BinaryOp, ExprKind as A, UnaryOp};
use machine_syntax::diagnostics::Diagnostic;
use machine_syntax::span::Span;

use super::{Checker, FnCtx};
use crate::builtins::{bits_for, Builtin};
use crate::consteval::{truncate, ConstVal};
use crate::hir::{self, Expr, ExprKind as H};
use crate::program::ItemKind;
use crate::ty::Ty;

fn mk(kind: H, ty: Ty, span: Span) -> Expr {
    Expr { kind, ty, span }
}

fn err(span: Span) -> Expr {
    mk(H::Error, Ty::Error, span)
}

fn const_int(v: i128, span: Span) -> Expr {
    mk(H::Int(v), Ty::Const(v), span)
}

impl Checker<'_> {
    /// Check `e`, then coerce to `expected` if given.
    pub fn check_expr(&mut self, cx: &mut FnCtx, e: &ast::Expr, expected: Option<&Ty>) -> Expr {
        let out = self.check_expr_inner(cx, e, expected);
        match expected {
            Some(t) => self.coerce(cx, out, t),
            None => out,
        }
    }

    /// Make `e` have type `target`: compile-time constants take the target
    /// width (if they fit), `!` becomes anything, everything else must match.
    pub fn coerce(&mut self, cx: &FnCtx, e: Expr, target: &Ty) -> Expr {
        if e.ty.is_error() || target.is_error() {
            return Expr {
                ty: target.clone(),
                ..e
            };
        }
        match (&e.ty, target) {
            (Ty::Never, _) => e,
            (Ty::Const(v), Ty::UInt(_) | Ty::SInt(_)) => {
                if target.fits(*v) {
                    mk(H::Int(*v), target.clone(), e.span)
                } else {
                    let msg = format!("constant {v} does not fit in `{}`", self.display(target));
                    self.error(cx.module, "E1006", e.span, msg);
                    err(e.span)
                }
            }
            (a, b) if a == b => e,
            (found, expected) => {
                self.mismatch(cx.module, e.span, expected, found);
                err(e.span)
            }
        }
    }

    /// The common type of two branches, or an error.
    pub fn join(&mut self, cx: &FnCtx, a: &Ty, b: &Ty, span: Span) -> Ty {
        match (a, b) {
            (Ty::Error, _) | (_, Ty::Error) => Ty::Error,
            (Ty::Never, t) | (t, Ty::Never) => t.clone(),
            (Ty::Const(x), Ty::Const(y)) if x == y => Ty::Const(*x),
            (Ty::Const(_), Ty::Const(_)) => {
                // A runtime choice between two compile-time values needs a
                // width, and nothing here says which.
                self.error_with(
                    cx.module,
                    Diagnostic::error("E1018", span, "cannot infer the width of these constants")
                        .with_note(
                            "give one a width suffix (`1_u8`) or annotate the enclosing `let`",
                        ),
                );
                Ty::Error
            }
            (Ty::Const(v), t @ (Ty::UInt(_) | Ty::SInt(_)))
            | (t @ (Ty::UInt(_) | Ty::SInt(_)), Ty::Const(v)) => {
                if t.fits(*v) {
                    t.clone()
                } else {
                    let msg = format!("constant {v} does not fit in `{}`", self.display(t));
                    self.error(cx.module, "E1006", span, msg);
                    Ty::Error
                }
            }
            (a, b) if a == b => a.clone(),
            (a, b) => {
                let msg = format!(
                    "branches have different types: `{}` and `{}`",
                    self.display(a),
                    self.display(b)
                );
                self.error(cx.module, "E1005", span, msg);
                Ty::Error
            }
        }
    }

    fn check_expr_inner(&mut self, cx: &mut FnCtx, e: &ast::Expr, expected: Option<&Ty>) -> Expr {
        let span = e.span;
        match &e.kind {
            A::Int { value, suffix } => {
                if *value > i128::MAX as u128 {
                    self.error(cx.module, "E1006", span, "integer literal is too large");
                    return err(span);
                }
                let v = *value as i128;
                match suffix {
                    Some((k, w)) => {
                        let ty = if *k == 'i' {
                            Ty::SInt(*w)
                        } else {
                            Ty::UInt(*w)
                        };
                        if !ty.fits(v) {
                            let msg =
                                format!("literal {v} does not fit in `{}`", self.display(&ty));
                            self.error(cx.module, "E1006", span, msg);
                            return err(span);
                        }
                        mk(H::Int(v), ty, span)
                    }
                    None => const_int(v, span),
                }
            }
            A::Bool(b) => mk(H::Bool(*b), Ty::Bool, span),
            A::Str(s) => mk(H::Str(s.clone()), Ty::Str, span),
            A::Quantity { .. } => {
                self.error(
                    cx.module,
                    "E1020",
                    span,
                    "frequency and time quantities are only allowed in `system` clocks and tests",
                );
                err(span)
            }
            A::Path(p) => self.check_path(cx, p, span),
            A::Unary(op, inner) => self.check_unary(cx, *op, inner, expected, span),
            A::Binary(op, a, b) => self.check_binary(cx, *op, a, b, expected, span),
            A::Ternary(c, a, b) => {
                let c = self.check_expr(cx, c, Some(&Ty::Bool));
                let a = self.check_expr(cx, a, expected);
                let hint = expected
                    .cloned()
                    .or_else(|| (!matches!(a.ty, Ty::Const(_))).then(|| a.ty.clone()));
                let b = self.check_expr(cx, b, hint.as_ref());
                let ty = self.join(cx, &a.ty, &b.ty, span);
                let a = self.coerce(cx, a, &ty);
                let b = self.coerce(cx, b, &ty);
                mk(H::Ternary(Box::new(c), Box::new(a), Box::new(b)), ty, span)
            }
            A::Cast(inner, ty) => self.check_cast(cx, inner, ty, span),
            A::Field(base, field) => {
                let base = self.check_expr(cx, base, None);
                self.check_field(cx, base, field, span)
            }
            A::Index(base, index) => self.check_index(cx, base, index, span),
            A::Slice(base, hi, lo) => self.check_slice(cx, base, hi, lo, span),
            A::Call {
                callee,
                generics,
                args,
            } => self.check_call(cx, callee, generics, args, expected, span),
            A::BundleLit {
                ty,
                fields,
                rest_default,
            } => self.check_bundle_lit(cx, ty, fields, *rest_default, span),
            A::Tuple(items) => self.check_tuple(cx, items, expected, span),
            A::ArrayLit(items) => self.check_array_lit(cx, items, expected, span),
            A::ArrayRepeat(item, count) => {
                let n = match self.eval_const(cx.module, &cx.generics.clone(), count) {
                    Some(ConstVal::Int(n)) if n >= 0 => n as u64,
                    Some(_) => {
                        self.error(
                            cx.module,
                            "E1013",
                            count.span,
                            "repeat count must be a non-negative integer",
                        );
                        return err(span);
                    }
                    None => return err(span),
                };
                let elem_expected = match expected {
                    Some(Ty::Array(t, len)) => {
                        if *len != n {
                            let msg = format!("expected an array of {len} elements, found {n}");
                            self.error(cx.module, "E1005", span, msg);
                        }
                        Some((**t).clone())
                    }
                    _ => None,
                };
                let item = self.check_expr(cx, item, elem_expected.as_ref());
                if matches!(item.ty, Ty::Const(_)) {
                    self.error_with(
                        cx.module,
                        Diagnostic::error(
                            "E1018",
                            item.span,
                            "cannot infer the element width of this array",
                        )
                        .with_note(
                            "give the element a width suffix (`0_u8`) or annotate the `let`",
                        ),
                    );
                    return err(span);
                }
                let ty = Ty::Array(Box::new(item.ty.clone()), n);
                mk(H::ArrayRepeat(Box::new(item), n), ty, span)
            }
            A::Match { scrutinee, arms } => {
                self.check_match_expr(cx, scrutinee, arms, expected, span)
            }
            A::If {
                cond,
                then_branch,
                else_branch,
            } => {
                let cond = self.check_expr(cx, cond, Some(&Ty::Bool));
                let then_b = self.check_block_expr(cx, then_branch);
                let Some(else_e) = else_branch else {
                    self.error_with(
                        cx.module,
                        Diagnostic::error(
                            "E1005",
                            span,
                            "`if` used as a value needs an `else` branch",
                        ),
                    );
                    return err(span);
                };
                let hint = expected
                    .cloned()
                    .or_else(|| (!matches!(then_b.1, Ty::Const(_))).then(|| then_b.1.clone()));
                let else_e = self.check_expr(cx, else_e, hint.as_ref());
                let ty = self.join(cx, &then_b.1, &else_e.ty, span);
                let else_e = self.coerce(cx, else_e, &ty);
                mk(
                    H::If(Box::new(cond), then_b.0, Some(Box::new(else_e))),
                    ty,
                    span,
                )
            }
            A::Block(b) => {
                let (block, ty) = self.check_block_expr(cx, b);
                mk(H::Block(block), ty, span)
            }
            A::RoutineCall(_) | A::Fetch(_) => {
                self.error(
                    cx.module,
                    "E1020",
                    span,
                    "`call` and `fetch` are only allowed inside routines",
                );
                err(span)
            }
            A::Range { .. } => {
                self.error(cx.module, "E1020", span, "a range is not a value here");
                err(span)
            }
            A::Wildcard => {
                self.error(
                    cx.module,
                    "E1020",
                    span,
                    "`_` is only allowed in patterns and `let` destructuring",
                );
                err(span)
            }
            A::Error => err(span),
        }
    }

    /// A block in value position: its type is that of a trailing expression
    /// statement, or `()`.
    pub fn check_block_expr(&mut self, cx: &mut FnCtx, b: &ast::Block) -> (hir::Block, Ty) {
        let block = self.check_block(cx, b);
        let ty = match block.stmts.last() {
            Some(hir::Stmt {
                kind: hir::StmtKind::Expr(e),
                ..
            }) => e.ty.clone(),
            Some(s) if super::stmt::stmt_diverges(s) => Ty::Never,
            _ => Ty::Unit,
        };
        (block, ty)
    }

    fn check_path(&mut self, cx: &mut FnCtx, p: &ast::Path, span: Span) -> Expr {
        match p.segments.len() {
            1 => {
                let name = &p.segments[0].name;
                if let Some(id) = cx.lookup_local(name) {
                    let ty = cx.locals[id as usize].ty.clone();
                    return match ty {
                        // A const-typed local is a compile-time value.
                        Ty::Const(v) => const_int(v, span),
                        ty => mk(H::Local(id), ty, span),
                    };
                }
                if let Some(v) = cx.generics.get(name) {
                    return const_int(*v, span);
                }
                match self.prog.lookup(cx.module, name) {
                    Some(def) => match self.prog.item(def).kind {
                        ItemKind::Const => match self.const_info(def) {
                            Some(info) => match info.value {
                                ConstVal::Int(v) => mk(H::Int(v), info.ty.clone(), span),
                                ConstVal::Bool(b) => mk(H::Bool(b), Ty::Bool, span),
                            },
                            None => err(span),
                        },
                        ItemKind::Unit => {
                            self.error(
                                cx.module,
                                "E1000",
                                span,
                                format!("`{name}` is a unit; call it with `{name}(...)`"),
                            );
                            err(span)
                        }
                        kind if kind.is_type() => {
                            self.error(
                                cx.module,
                                "E1000",
                                span,
                                format!("`{name}` is a type, not a value"),
                            );
                            err(span)
                        }
                        kind => {
                            self.error(
                                cx.module,
                                "E1000",
                                span,
                                format!("`{name}` is a {}, not a value", kind.describe()),
                            );
                            err(span)
                        }
                    },
                    None => {
                        if Builtin::from_name(name).is_some() {
                            self.error(
                                cx.module,
                                "E1000",
                                span,
                                format!("`{name}` is a built-in function; call it"),
                            );
                        } else {
                            self.error(cx.module, "E1000", span, format!("unknown name `{name}`"));
                        }
                        err(span)
                    }
                }
            }
            2 => {
                let (type_name, variant) = (&p.segments[0].name, &p.segments[1].name);
                let Some(def) = self.prog.lookup(cx.module, type_name) else {
                    self.error(
                        cx.module,
                        "E1000",
                        p.segments[0].span,
                        format!("unknown name `{type_name}`"),
                    );
                    return err(span);
                };
                if self.prog.item(def).kind != ItemKind::Enum {
                    self.error(
                        cx.module,
                        "E1000",
                        span,
                        format!("`{type_name}` is not an enum"),
                    );
                    return err(span);
                }
                let Some(info) = self.enum_info(def) else {
                    return err(span);
                };
                match info.variants.iter().position(|(n, _)| n == variant) {
                    Some(idx) => mk(H::Variant(def, idx as u32), Ty::Enum(def), span),
                    None => {
                        self.error(
                            cx.module,
                            "E1000",
                            p.segments[1].span,
                            format!("enum `{type_name}` has no variant `{variant}`"),
                        );
                        err(span)
                    }
                }
            }
            _ => {
                self.error(
                    cx.module,
                    "E1000",
                    span,
                    format!("unknown name `{}`", p.dotted()),
                );
                err(span)
            }
        }
    }

    fn check_unary(
        &mut self,
        cx: &mut FnCtx,
        op: UnaryOp,
        inner: &ast::Expr,
        expected: Option<&Ty>,
        span: Span,
    ) -> Expr {
        match op {
            UnaryOp::Not => {
                let inner = self.check_expr(cx, inner, Some(&Ty::Bool));
                if inner.ty.is_error() {
                    return err(span);
                }
                if let H::Bool(b) = inner.kind {
                    return mk(H::Bool(!b), Ty::Bool, span);
                }
                mk(H::Unary(op, Box::new(inner)), Ty::Bool, span)
            }
            UnaryOp::BitNot => {
                let hint = expected.filter(|t| matches!(t, Ty::UInt(_) | Ty::SInt(_)));
                let inner = self.check_expr(cx, inner, hint);
                match &inner.ty {
                    Ty::UInt(w) | Ty::SInt(w) => {
                        if let H::Int(v) = inner.kind {
                            let signed = matches!(inner.ty, Ty::SInt(_));
                            let ty = inner.ty.clone();
                            return mk(H::Int(truncate(!v, signed, *w)), ty, span);
                        }
                        let ty = inner.ty.clone();
                        mk(H::Unary(op, Box::new(inner)), ty, span)
                    }
                    Ty::Const(_) => {
                        self.error_with(
                            cx.module,
                            Diagnostic::error("E1012", span, "`~` needs an operand with a known width")
                                .with_note("add a width suffix (`0xFF_u8`) or a cast so the complement has a size"),
                        );
                        err(span)
                    }
                    Ty::Error => err(span),
                    t => {
                        let msg = format!("`~` applies to integers, not `{}`", self.display(t));
                        self.error(cx.module, "E1012", span, msg);
                        err(span)
                    }
                }
            }
            UnaryOp::Neg => {
                // No hint: a constant operand is negated first and only then
                // fitted to the context (`-128` as `i8` must work).
                let _ = expected;
                let inner = self.check_expr(cx, inner, None);
                match &inner.ty {
                    Ty::Const(v) => match v.checked_neg() {
                        Some(n) => const_int(n, span),
                        None => {
                            self.error(cx.module, "E1021", span, "constant overflow");
                            err(span)
                        }
                    },
                    Ty::SInt(_) => {
                        let ty = inner.ty.clone();
                        mk(H::Unary(op, Box::new(inner)), ty, span)
                    }
                    Ty::Error => err(span),
                    t => {
                        let msg =
                            format!("`-` applies to signed integers, not `{}`", self.display(t));
                        self.error_with(
                            cx.module,
                            Diagnostic::error("E1012", span, msg).with_note("negate an unsigned value with two's complement explicitly: `~x + 1`"),
                        );
                        err(span)
                    }
                }
            }
        }
    }

    /// Check two operands that must share a type, using `hint` for the first
    /// and the first's type for the second (or the other way round when the
    /// first is a compile-time constant).
    fn check_pair(
        &mut self,
        cx: &mut FnCtx,
        a: &ast::Expr,
        b: &ast::Expr,
        hint: Option<&Ty>,
    ) -> (Expr, Expr) {
        let ha = self.check_expr(cx, a, hint);
        if matches!(ha.ty, Ty::Const(_)) {
            let hb = self.check_expr(cx, b, hint);
            if matches!(hb.ty, Ty::Const(_) | Ty::Error) {
                (ha, hb)
            } else {
                let ty = hb.ty.clone();
                (self.coerce(cx, ha, &ty), hb)
            }
        } else if ha.ty.is_error() {
            let hb = self.check_expr(cx, b, None);
            (ha, hb)
        } else {
            let ty = ha.ty.clone();
            let hb = self.check_expr(cx, b, Some(&ty));
            (ha, hb)
        }
    }

    fn check_binary(
        &mut self,
        cx: &mut FnCtx,
        op: BinaryOp,
        a: &ast::Expr,
        b: &ast::Expr,
        expected: Option<&Ty>,
        span: Span,
    ) -> Expr {
        use BinaryOp::*;
        match op {
            Add | Sub | Mul | Div | Rem | BitAnd | BitOr | BitXor => {
                let hint = expected.filter(|t| matches!(t, Ty::UInt(_) | Ty::SInt(_)));
                let (ha, hb) = self.check_pair(cx, a, b, hint);
                if ha.ty.is_error() || hb.ty.is_error() {
                    return err(span);
                }
                if let (Ty::Const(x), Ty::Const(y)) = (&ha.ty, &hb.ty) {
                    return self.fold(cx, op, *x, *y, span);
                }
                match &ha.ty {
                    Ty::UInt(_) | Ty::SInt(_) => {
                        let ty = ha.ty.clone();
                        if let (H::Int(x), H::Int(y)) = (&ha.kind, &hb.kind) {
                            if let Some(v) = fold_in_type(op, *x, *y, &ty) {
                                return mk(H::Int(v), ty, span);
                            }
                        }
                        mk(H::Binary(op, Box::new(ha), Box::new(hb)), ty, span)
                    }
                    t => {
                        let msg = format!(
                            "operator `{}` applies to integers, not `{}`",
                            op_str(op),
                            self.display(t)
                        );
                        self.error(cx.module, "E1012", span, msg);
                        err(span)
                    }
                }
            }
            Shl | Shr => {
                let hint = expected.filter(|t| matches!(t, Ty::UInt(_) | Ty::SInt(_)));
                let ha = self.check_expr(cx, a, hint);
                let hb = self.check_expr(cx, b, None);
                if ha.ty.is_error() || hb.ty.is_error() {
                    return err(span);
                }
                let hb = match &hb.ty {
                    Ty::Const(v) if *v >= 0 => {
                        let w = bits_for(*v as u64);
                        mk(H::Int(*v), Ty::UInt(w), hb.span)
                    }
                    Ty::UInt(_) => hb,
                    t => {
                        let msg =
                            format!("shift amount must be unsigned, found `{}`", self.display(t));
                        self.error(cx.module, "E1012", hb.span, msg);
                        return err(span);
                    }
                };
                match &ha.ty {
                    Ty::Const(x) => match (&hb.kind, op) {
                        (H::Int(y), Shl) => match x.checked_shl(*y as u32) {
                            Some(v) if *y < 120 => const_int(v, span),
                            _ => {
                                self.error(cx.module, "E1021", span, "constant shift overflow");
                                err(span)
                            }
                        },
                        (H::Int(y), _) => const_int(x >> (*y).min(127) as u32, span),
                        _ => {
                            self.error_with(
                                cx.module,
                                Diagnostic::error(
                                    "E1012",
                                    span,
                                    "shifting a constant by a runtime amount needs a width",
                                )
                                .with_note("add a width suffix or cast to the constant"),
                            );
                            err(span)
                        }
                    },
                    Ty::UInt(_) | Ty::SInt(_) => {
                        let ty = ha.ty.clone();
                        mk(H::Binary(op, Box::new(ha), Box::new(hb)), ty, span)
                    }
                    t => {
                        let msg = format!(
                            "operator `{}` applies to integers, not `{}`",
                            op_str(op),
                            self.display(t)
                        );
                        self.error(cx.module, "E1012", span, msg);
                        err(span)
                    }
                }
            }
            Eq | Ne | Lt | Le | Gt | Ge => {
                let (ha, hb) = self.check_pair(cx, a, b, None);
                if ha.ty.is_error() || hb.ty.is_error() {
                    return err(span);
                }
                if let (Ty::Const(x), Ty::Const(y)) = (&ha.ty, &hb.ty) {
                    return self.fold(cx, op, *x, *y, span);
                }
                let ordering = matches!(op, Lt | Le | Gt | Ge);
                match &ha.ty {
                    Ty::UInt(_) | Ty::SInt(_) => {}
                    Ty::Bool | Ty::Enum(_) if !ordering => {}
                    t => {
                        let msg = format!(
                            "operator `{}` does not apply to `{}`",
                            op_str(op),
                            self.display(t)
                        );
                        self.error(cx.module, "E1012", span, msg);
                        return err(span);
                    }
                }
                mk(H::Binary(op, Box::new(ha), Box::new(hb)), Ty::Bool, span)
            }
            And | Or => {
                let ha = self.check_expr(cx, a, Some(&Ty::Bool));
                let hb = self.check_expr(cx, b, Some(&Ty::Bool));
                if let (H::Bool(x), H::Bool(y)) = (&ha.kind, &hb.kind) {
                    let v = if op == And { *x && *y } else { *x || *y };
                    return mk(H::Bool(v), Ty::Bool, span);
                }
                mk(H::Binary(op, Box::new(ha), Box::new(hb)), Ty::Bool, span)
            }
        }
    }

    fn fold(&mut self, cx: &FnCtx, op: BinaryOp, x: i128, y: i128, span: Span) -> Expr {
        use BinaryOp::*;
        let int = |v: Option<i128>| v;
        let r = match op {
            Add => int(x.checked_add(y)),
            Sub => int(x.checked_sub(y)),
            Mul => int(x.checked_mul(y)),
            Div => int(x.checked_div(y)),
            Rem => int(x.checked_rem(y)),
            BitAnd => Some(x & y),
            BitOr => Some(x | y),
            BitXor => Some(x ^ y),
            Eq => return mk(H::Bool(x == y), Ty::Bool, span),
            Ne => return mk(H::Bool(x != y), Ty::Bool, span),
            Lt => return mk(H::Bool(x < y), Ty::Bool, span),
            Le => return mk(H::Bool(x <= y), Ty::Bool, span),
            Gt => return mk(H::Bool(x > y), Ty::Bool, span),
            Ge => return mk(H::Bool(x >= y), Ty::Bool, span),
            Shl | Shr | And | Or => unreachable!("handled by caller"),
        };
        match r {
            Some(v) => const_int(v, span),
            None => {
                self.error(
                    cx.module,
                    "E1021",
                    span,
                    "constant overflow or division by zero",
                );
                err(span)
            }
        }
    }

    fn check_cast(
        &mut self,
        cx: &mut FnCtx,
        inner: &ast::Expr,
        ty: &ast::Type,
        span: Span,
    ) -> Expr {
        let inner = self.check_expr(cx, inner, None);
        let generics = cx.generics.clone();
        let target = self.resolve_type(cx.module, &generics, ty);
        if inner.ty.is_error() || target.is_error() {
            return err(span);
        }
        let ok = match (&inner.ty, &target) {
            (Ty::Const(v), Ty::UInt(w) | Ty::SInt(w)) => {
                let signed = matches!(target, Ty::SInt(_));
                return mk(H::Int(truncate(*v, signed, *w)), target.clone(), span);
            }
            (Ty::UInt(_) | Ty::SInt(_), Ty::UInt(_) | Ty::SInt(_)) => true,
            (Ty::Bool, Ty::UInt(1)) | (Ty::UInt(1), Ty::Bool) => true,
            (Ty::Enum(e), Ty::UInt(w)) | (Ty::UInt(w), Ty::Enum(e)) => {
                let ew = self.enum_info(*e).map(|i| i.width);
                match ew {
                    Some(ew) if ew == *w => true,
                    Some(ew) => {
                        let msg = format!(
                            "enum `{}` is {ew} bits wide; cast to or from `u{ew}` and then resize",
                            self.prog.item_name(*e)
                        );
                        self.error(cx.module, "E1011", span, msg);
                        return err(span);
                    }
                    None => return err(span),
                }
            }
            (Ty::Const(v), Ty::Enum(e)) => {
                let valid = self
                    .enum_info(*e)
                    .map(|i| i.variants.iter().any(|(_, x)| x == v))
                    .unwrap_or(false);
                if !valid {
                    self.error(
                        cx.module,
                        "E1011",
                        span,
                        format!(
                            "{v} is not a variant encoding of `{}`",
                            self.prog.item_name(*e)
                        ),
                    );
                    return err(span);
                }
                true
            }
            (a, b) if a == b => true,
            _ => false,
        };
        if !ok {
            let msg = format!(
                "cannot cast `{}` to `{}`",
                self.display(&inner.ty),
                self.display(&target)
            );
            let mut d = Diagnostic::error("E1011", span, msg);
            if matches!(
                (&inner.ty, &target),
                (Ty::Bool, Ty::UInt(_) | Ty::SInt(_)) | (Ty::UInt(_) | Ty::SInt(_), Ty::Bool)
            ) {
                d = d.with_note("`bool` converts only to and from `u1`");
            }
            self.error_with(cx.module, d);
            return err(span);
        }
        mk(H::Cast(Box::new(inner)), target, span)
    }

    fn check_field(&mut self, cx: &mut FnCtx, base: Expr, field: &ast::Ident, span: Span) -> Expr {
        match &base.ty {
            Ty::Bundle(def) => {
                let Some(info) = self.bundle_info(*def) else {
                    return err(span);
                };
                match info.fields.iter().position(|f| f.name == field.name) {
                    Some(i) => {
                        let ty = info.fields[i].ty.clone();
                        mk(H::Field(Box::new(base), i as u32), ty, span)
                    }
                    None => {
                        let msg = format!(
                            "bundle `{}` has no field `{}`",
                            self.prog.item_name(*def),
                            field.name
                        );
                        self.error(cx.module, "E1008", field.span, msg);
                        err(span)
                    }
                }
            }
            Ty::Tuple(items) => match items
                .iter()
                .position(|(n, _)| n.as_deref() == Some(field.name.as_str()))
            {
                Some(i) => {
                    let ty = items[i].1.clone();
                    mk(H::TupleField(Box::new(base), i as u32), ty, span)
                }
                None => {
                    let msg = format!(
                        "tuple `{}` has no element `{}`",
                        self.display(&base.ty),
                        field.name
                    );
                    self.error(cx.module, "E1008", field.span, msg);
                    err(span)
                }
            },
            Ty::Error => err(span),
            t => {
                let msg = format!("`{}` has no fields", self.display(t));
                self.error(cx.module, "E1008", span, msg);
                err(span)
            }
        }
    }

    fn check_index(
        &mut self,
        cx: &mut FnCtx,
        base: &ast::Expr,
        index: &ast::Expr,
        span: Span,
    ) -> Expr {
        let base = self.check_expr(cx, base, None);
        let index = self.check_expr(cx, index, None);
        if base.ty.is_error() || index.ty.is_error() {
            return err(span);
        }
        let (elem_ty, len): (Ty, u128) = match &base.ty {
            Ty::Array(t, n) => ((**t).clone(), *n as u128),
            Ty::UInt(w) | Ty::SInt(w) => (Ty::UInt(1), *w as u128),
            t => {
                let msg = format!("`{}` cannot be indexed", self.display(t));
                self.error(cx.module, "E1012", span, msg);
                return err(span);
            }
        };
        let index = match &index.ty {
            Ty::Const(v) => {
                if *v < 0 || *v as u128 >= len {
                    let msg = format!(
                        "index {v} is out of range for `{}` (0..{len})",
                        self.display(&base.ty)
                    );
                    self.error(cx.module, "E1017", index.span, msg);
                    return err(span);
                }
                let w = bits_for(len.saturating_sub(1) as u64);
                mk(H::Int(*v), Ty::UInt(w), index.span)
            }
            Ty::UInt(_) => index,
            t => {
                let msg = format!("index must be unsigned, found `{}`", self.display(t));
                self.error(cx.module, "E1012", index.span, msg);
                return err(span);
            }
        };
        mk(H::Index(Box::new(base), Box::new(index)), elem_ty, span)
    }

    fn check_slice(
        &mut self,
        cx: &mut FnCtx,
        base: &ast::Expr,
        hi: &ast::Expr,
        lo: &ast::Expr,
        span: Span,
    ) -> Expr {
        let base = self.check_expr(cx, base, None);
        let generics = cx.generics.clone();
        let hi_v = self
            .eval_const(cx.module, &generics, hi)
            .and_then(|v| v.as_int());
        let lo_v = self
            .eval_const(cx.module, &generics, lo)
            .and_then(|v| v.as_int());
        let (Some(hi_v), Some(lo_v)) = (hi_v, lo_v) else {
            return err(span);
        };
        let w = match &base.ty {
            Ty::UInt(w) | Ty::SInt(w) => *w as i128,
            Ty::Error => return err(span),
            t => {
                let msg = format!(
                    "`{}` cannot be sliced; slices apply to integers",
                    self.display(t)
                );
                self.error(cx.module, "E1012", span, msg);
                return err(span);
            }
        };
        if lo_v < 0 || hi_v < lo_v || hi_v >= w {
            let msg = format!("slice [{hi_v}:{lo_v}] is not within bits 0..{w} with hi >= lo");
            self.error(cx.module, "E1017", span, msg);
            return err(span);
        }
        let width = (hi_v - lo_v + 1) as u32;
        mk(
            H::Slice(Box::new(base), hi_v as u32, lo_v as u32),
            Ty::UInt(width),
            span,
        )
    }

    fn check_call(
        &mut self,
        cx: &mut FnCtx,
        callee: &ast::Expr,
        generics: &[ast::GenericArg],
        args: &[ast::Arg],
        expected: Option<&Ty>,
        span: Span,
    ) -> Expr {
        let A::Path(path) = &callee.kind else {
            self.error_with(
                cx.module,
                Diagnostic::error("E1020", callee.span, "only units and built-in functions can be called here")
                    .with_note("method-style calls (`port.read(...)`, `mem.write(...)`) belong to rtl and model bodies"),
            );
            return err(span);
        };
        if path.segments.len() != 1 {
            self.error(
                cx.module,
                "E1000",
                path.span,
                format!("unknown function `{}`", path.dotted()),
            );
            return err(span);
        }
        let name = &path.segments[0].name;

        if cx.lookup_local(name).is_some() {
            self.error(
                cx.module,
                "E1012",
                callee.span,
                format!("`{name}` is a local value and cannot be called"),
            );
            return err(span);
        }
        let def = self.prog.lookup(cx.module, name);
        if def.is_none() {
            if let Some(b) = Builtin::from_name(name) {
                if !generics.is_empty() {
                    self.error(
                        cx.module,
                        "E1016",
                        span,
                        "built-in functions take no generic arguments",
                    );
                    return err(span);
                }
                return self.check_builtin(cx, b, args, expected, span);
            }
            self.error(
                cx.module,
                "E1000",
                callee.span,
                format!("unknown function `{name}`"),
            );
            return err(span);
        }
        let def = def.unwrap();
        if self.prog.item(def).kind != ItemKind::Unit {
            let msg = format!(
                "`{name}` is a {}, not a unit",
                self.prog.item(def).kind.describe()
            );
            self.error(cx.module, "E1012", callee.span, msg);
            return err(span);
        }

        // Generic arguments: compile-time integers.
        let mut gen_vals = Vec::new();
        let generics_env = cx.generics.clone();
        for g in generics {
            match g {
                ast::GenericArg::Expr(e) => match self.eval_const(cx.module, &generics_env, e) {
                    Some(ConstVal::Int(v)) => gen_vals.push(v),
                    Some(ConstVal::Bool(_)) => {
                        self.error(cx.module, "E1013", e.span, "generic arguments are integers");
                        return err(span);
                    }
                    None => return err(span),
                },
                ast::GenericArg::Type(t) => {
                    self.error(
                        cx.module,
                        "E1016",
                        t.span,
                        "generic arguments are compile-time integers, not types",
                    );
                    return err(span);
                }
            }
        }
        let Some((inst, sig)) = self.instantiate_unit(def, gen_vals, span, cx.module) else {
            return err(span);
        };

        // Arguments: positional or all named.
        let named = args.iter().filter(|a| a.name.is_some()).count();
        if named != 0 && named != args.len() {
            self.error(
                cx.module,
                "E1007",
                span,
                "arguments are either all positional or all named",
            );
            return err(span);
        }
        if args.len() != sig.params.len() {
            let msg = format!(
                "unit `{name}` takes {} argument(s), {} given",
                sig.params.len(),
                args.len()
            );
            self.error(cx.module, "E1007", span, msg);
            return err(span);
        }
        let mut ordered: Vec<Option<&ast::Arg>> = vec![None; sig.params.len()];
        for (i, a) in args.iter().enumerate() {
            match &a.name {
                None => ordered[i] = Some(a),
                Some(n) => match sig.params.iter().position(|(p, _)| p == &n.name) {
                    Some(idx) => {
                        if ordered[idx].is_some() {
                            self.error(
                                cx.module,
                                "E1007",
                                n.span,
                                format!("argument `{}` given twice", n.name),
                            );
                            return err(span);
                        }
                        ordered[idx] = Some(a);
                    }
                    None => {
                        self.error(
                            cx.module,
                            "E1007",
                            n.span,
                            format!("unit `{name}` has no parameter `{}`", n.name),
                        );
                        return err(span);
                    }
                },
            }
        }
        let mut hargs = Vec::new();
        for (slot, (_, pty)) in ordered.iter().zip(&sig.params) {
            let a = slot.expect("every slot filled");
            hargs.push(self.check_expr(cx, &a.value, Some(pty)));
        }
        mk(H::Call(inst, hargs), sig.ret, span)
    }

    fn check_builtin(
        &mut self,
        cx: &mut FnCtx,
        b: Builtin,
        args: &[ast::Arg],
        _expected: Option<&Ty>,
        span: Span,
    ) -> Expr {
        if args.iter().any(|a| a.name.is_some()) {
            self.error(
                cx.module,
                "E1007",
                span,
                "built-in functions take positional arguments",
            );
            return err(span);
        }
        let arity_err = |ck: &mut Checker, want: &str| {
            let msg = format!(
                "`{}` takes {want} argument(s), {} given",
                b.name(),
                args.len()
            );
            ck.error(cx.module, "E1007", span, msg);
            err(span)
        };
        match b {
            Builtin::Parity | Builtin::Popcount | Builtin::Clz | Builtin::Ctz => {
                if args.len() != 1 {
                    return arity_err(self, "1");
                }
                let x = self.check_expr(cx, &args[0].value, None);
                let w = match &x.ty {
                    Ty::UInt(w) => *w,
                    Ty::Error => return err(span),
                    Ty::Const(_) => {
                        self.error(
                            cx.module,
                            "E1012",
                            x.span,
                            format!("`{}` needs an operand with a known width", b.name()),
                        );
                        return err(span);
                    }
                    t => {
                        let msg = format!(
                            "`{}` applies to unsigned integers, not `{}`",
                            b.name(),
                            self.display(t)
                        );
                        self.error(cx.module, "E1012", x.span, msg);
                        return err(span);
                    }
                };
                let ty = if b == Builtin::Parity {
                    Ty::UInt(1)
                } else {
                    Ty::UInt(bits_for(w as u64))
                };
                mk(H::Builtin(b, vec![x]), ty, span)
            }
            Builtin::Concat => {
                if args.len() < 2 {
                    return arity_err(self, "at least 2");
                }
                let mut total = 0u32;
                let mut hargs = Vec::new();
                for a in args {
                    let x = self.check_expr(cx, &a.value, None);
                    match &x.ty {
                        Ty::UInt(w) => total += *w,
                        Ty::Error => return err(span),
                        Ty::Const(_) => {
                            self.error(cx.module, "E1012", x.span, "`concat` operands need a known width (add a suffix such as `0_u5`)");
                            return err(span);
                        }
                        t => {
                            let msg = format!(
                                "`concat` operands must be unsigned integers, found `{}`",
                                self.display(t)
                            );
                            self.error(cx.module, "E1012", x.span, msg);
                            return err(span);
                        }
                    }
                    hargs.push(x);
                }
                if total > 128 {
                    self.error(
                        cx.module,
                        "E1004",
                        span,
                        format!("concatenation is {total} bits wide; the maximum is 128"),
                    );
                    return err(span);
                }
                mk(H::Builtin(b, hargs), Ty::UInt(total), span)
            }
            Builtin::Replicate => {
                if args.len() != 2 {
                    return arity_err(self, "2");
                }
                let x = self.check_expr(cx, &args[0].value, None);
                let generics = cx.generics.clone();
                let n = match self.eval_const(cx.module, &generics, &args[1].value) {
                    Some(ConstVal::Int(n)) if n >= 1 => n as u32,
                    Some(_) => {
                        self.error(
                            cx.module,
                            "E1013",
                            args[1].span,
                            "replication count must be a positive integer",
                        );
                        return err(span);
                    }
                    None => return err(span),
                };
                let w = match &x.ty {
                    Ty::UInt(w) => *w,
                    Ty::Error => return err(span),
                    t => {
                        let msg = format!(
                            "`replicate` applies to unsigned integers, not `{}`",
                            self.display(t)
                        );
                        self.error(cx.module, "E1012", x.span, msg);
                        return err(span);
                    }
                };
                let total = w.saturating_mul(n);
                if total > 128 {
                    self.error(
                        cx.module,
                        "E1004",
                        span,
                        format!("replication is {total} bits wide; the maximum is 128"),
                    );
                    return err(span);
                }
                let count = const_int(n as i128, args[1].span);
                mk(H::Builtin(b, vec![x, count]), Ty::UInt(total), span)
            }
            Builtin::Assert => {
                if args.is_empty() || args.len() > 2 {
                    return arity_err(self, "1 or 2");
                }
                let cond = self.check_expr(cx, &args[0].value, Some(&Ty::Bool));
                let mut hargs = vec![cond];
                if let Some(m) = args.get(1) {
                    hargs.push(self.check_expr(cx, &m.value, Some(&Ty::Str)));
                }
                mk(H::Builtin(b, hargs), Ty::Unit, span)
            }
            Builtin::Unreachable => {
                if args.len() > 1 {
                    return arity_err(self, "0 or 1");
                }
                let mut hargs = Vec::new();
                if let Some(m) = args.first() {
                    hargs.push(self.check_expr(cx, &m.value, Some(&Ty::Str)));
                }
                mk(H::Builtin(b, hargs), Ty::Never, span)
            }
            Builtin::Sync => {
                self.error(
                    cx.module,
                    "E1020",
                    span,
                    "`sync` is only meaningful in rtl bodies",
                );
                err(span)
            }
        }
    }

    fn check_bundle_lit(
        &mut self,
        cx: &mut FnCtx,
        ty_path: &ast::Path,
        fields: &[ast::FieldInit],
        rest_default: bool,
        span: Span,
    ) -> Expr {
        if ty_path.segments.len() != 1 {
            self.error(
                cx.module,
                "E1000",
                ty_path.span,
                format!("unknown bundle `{}`", ty_path.dotted()),
            );
            return err(span);
        }
        let name = &ty_path.segments[0].name;
        let Some(def) = self.prog.lookup(cx.module, name) else {
            self.error(
                cx.module,
                "E1000",
                ty_path.span,
                format!("unknown bundle `{name}`"),
            );
            return err(span);
        };
        if self.prog.item(def).kind != ItemKind::Bundle {
            let msg = format!(
                "`{name}` is a {}, not a bundle",
                self.prog.item(def).kind.describe()
            );
            self.error(cx.module, "E1012", ty_path.span, msg);
            return err(span);
        }
        let Some(info) = self.bundle_info(def) else {
            return err(span);
        };

        let mut inits: Vec<Option<Expr>> = (0..info.fields.len()).map(|_| None).collect();
        let mut ok = true;
        for f in fields {
            let Some(idx) = info.fields.iter().position(|x| x.name == f.name.name) else {
                self.error(
                    cx.module,
                    "E1008",
                    f.name.span,
                    format!("bundle `{name}` has no field `{}`", f.name.name),
                );
                ok = false;
                continue;
            };
            if inits[idx].is_some() {
                self.error(
                    cx.module,
                    "E1008",
                    f.name.span,
                    format!("field `{}` given twice", f.name.name),
                );
                ok = false;
                continue;
            }
            let fty = info.fields[idx].ty.clone();
            let value = match &f.value {
                Some(v) => self.check_expr(cx, v, Some(&fty)),
                None => {
                    // Shorthand: a local of the same name.
                    let p = ast::Path::single(f.name.clone());
                    let e = self.check_path(cx, &p, f.name.span);
                    self.coerce(cx, e, &fty)
                }
            };
            inits[idx] = Some(value);
        }
        let mut out = Vec::new();
        let mut missing = Vec::new();
        for (i, slot) in inits.into_iter().enumerate() {
            match slot {
                Some(e) => out.push(e),
                None => {
                    let field = &info.fields[i];
                    match (rest_default, field.default) {
                        (true, Some(ConstVal::Int(v))) => {
                            let e = const_int(v, span);
                            out.push(self.coerce(cx, e, &field.ty.clone()));
                        }
                        (true, Some(ConstVal::Bool(b))) => out.push(mk(H::Bool(b), Ty::Bool, span)),
                        _ => missing.push(field.name.clone()),
                    }
                }
            }
        }
        if !missing.is_empty() {
            let msg = format!(
                "missing field(s) in `{name}` literal: {}",
                missing.join(", ")
            );
            let mut d = Diagnostic::error("E1009", span, msg);
            if rest_default {
                d = d.with_note(
                    "`..default` fills only fields that declare a default (`field: T = value`)",
                );
            }
            self.error_with(cx.module, d);
            ok = false;
        }
        if !ok {
            return err(span);
        }
        mk(H::BundleLit(def, out), Ty::Bundle(def), span)
    }

    fn check_tuple(
        &mut self,
        cx: &mut FnCtx,
        items: &[ast::Arg],
        expected: Option<&Ty>,
        span: Span,
    ) -> Expr {
        let named = items.iter().filter(|a| a.name.is_some()).count();
        if named != 0 && named != items.len() {
            self.error(
                cx.module,
                "E1005",
                span,
                "a tuple is either fully named or fully positional",
            );
            return err(span);
        }
        match expected {
            Some(Ty::Tuple(exp)) => {
                if exp.len() != items.len() {
                    let msg = format!(
                        "expected a tuple of {} elements, found {}",
                        exp.len(),
                        items.len()
                    );
                    self.error(cx.module, "E1005", span, msg);
                    return err(span);
                }
                let exp_named = exp.iter().all(|(n, _)| n.is_some());
                let mut out: Vec<Option<Expr>> = (0..exp.len()).map(|_| None).collect();
                for (i, a) in items.iter().enumerate() {
                    let idx = match (&a.name, exp_named) {
                        (Some(n), true) => match exp
                            .iter()
                            .position(|(en, _)| en.as_deref() == Some(n.name.as_str()))
                        {
                            Some(idx) => idx,
                            None => {
                                let msg = format!(
                                    "tuple `{}` has no element `{}`",
                                    self.display(expected.unwrap()),
                                    n.name
                                );
                                self.error(cx.module, "E1008", n.span, msg);
                                return err(span);
                            }
                        },
                        (Some(n), false) => {
                            self.error(
                                cx.module,
                                "E1005",
                                n.span,
                                "positional tuple expected; remove the element names",
                            );
                            return err(span);
                        }
                        (None, _) => i,
                    };
                    if out[idx].is_some() {
                        self.error(cx.module, "E1008", a.span, "tuple element given twice");
                        return err(span);
                    }
                    let ety = exp[idx].1.clone();
                    out[idx] = Some(self.check_expr(cx, &a.value, Some(&ety)));
                }
                let out: Vec<Expr> = out.into_iter().map(|e| e.expect("filled")).collect();
                mk(H::Tuple(out), Ty::Tuple(exp.clone()), span)
            }
            _ => {
                let mut out = Vec::new();
                let mut tys = Vec::new();
                for a in items {
                    let e = self.check_expr(cx, &a.value, None);
                    tys.push((a.name.as_ref().map(|n| n.name.clone()), e.ty.clone()));
                    out.push(e);
                }
                mk(H::Tuple(out), Ty::Tuple(tys), span)
            }
        }
    }

    fn check_array_lit(
        &mut self,
        cx: &mut FnCtx,
        items: &[ast::Expr],
        expected: Option<&Ty>,
        span: Span,
    ) -> Expr {
        let elem_expected = match expected {
            Some(Ty::Array(t, n)) => {
                if *n as usize != items.len() {
                    let msg = format!("expected an array of {n} elements, found {}", items.len());
                    self.error(cx.module, "E1005", span, msg);
                    return err(span);
                }
                Some((**t).clone())
            }
            _ => None,
        };
        if items.is_empty() {
            self.error(
                cx.module,
                "E1018",
                span,
                "cannot infer the type of an empty array literal",
            );
            return err(span);
        }
        let mut out = Vec::new();
        let mut ty = elem_expected.clone();
        for item in items {
            let hint = ty.clone().filter(|t| !matches!(t, Ty::Const(_)));
            let e = self.check_expr(cx, item, hint.as_ref());
            ty = Some(match &ty {
                None => e.ty.clone(),
                Some(t) => self.join(cx, t, &e.ty, e.span),
            });
            out.push(e);
        }
        let elem_ty = ty.unwrap();
        if matches!(elem_ty, Ty::Const(_)) {
            self.error_with(
                cx.module,
                Diagnostic::error(
                    "E1018",
                    span,
                    "cannot infer the element width of this array",
                )
                .with_note("give one element a width suffix (`0_u8`) or annotate the `let`"),
            );
            return err(span);
        }
        let out: Vec<Expr> = out
            .into_iter()
            .map(|e| self.coerce(cx, e, &elem_ty))
            .collect();
        let n = out.len() as u64;
        mk(H::Array(out), Ty::Array(Box::new(elem_ty), n), span)
    }

    fn check_match_expr(
        &mut self,
        cx: &mut FnCtx,
        scrutinee: &ast::Expr,
        arms: &[ast::MatchArm],
        expected: Option<&Ty>,
        span: Span,
    ) -> Expr {
        let scrut = self.check_expr(cx, scrutinee, None);
        let scrut = self.settle_scrutinee(cx, scrut);
        if scrut.ty.is_error() {
            // Still check arm bodies for their own errors.
            for arm in arms {
                if let ast::ArmBody::Expr(e) = &arm.body {
                    self.check_expr(cx, e, expected);
                }
            }
            return err(span);
        }
        let mut harms = Vec::new();
        let mut ty: Option<Ty> = expected.cloned();
        let mut had_error = false;
        for arm in arms {
            let pats = self.check_patterns(cx, &arm.patterns, &scrut.ty);
            cx.push_scope();
            let (body, bty) = match &arm.body {
                ast::ArmBody::Expr(e) => {
                    let hint = ty.clone().filter(|t| !matches!(t, Ty::Const(_)));
                    let e = self.check_expr(cx, e, hint.as_ref());
                    let t = e.ty.clone();
                    (hir::ArmBody::Expr(e), t)
                }
                ast::ArmBody::Block(b) => {
                    let (b, t) = self.check_block_expr(cx, b);
                    (hir::ArmBody::Block(b), t)
                }
            };
            cx.pop_scope();
            if bty.is_error() {
                had_error = true;
            }
            ty = Some(match &ty {
                None => bty,
                Some(t) => self.join(cx, t, &bty, arm.span),
            });
            harms.push(hir::Arm {
                patterns: pats,
                body,
                span: arm.span,
            });
        }
        let ty = ty.unwrap_or(Ty::Never);
        if had_error || ty.is_error() {
            return err(span);
        }
        // Coerce expression arms to the joined type.
        for arm in &mut harms {
            if let hir::ArmBody::Expr(e) = &mut arm.body {
                let taken = std::mem::replace(e, err(span));
                *e = self.coerce(cx, taken, &ty);
            }
        }
        self.check_exhaustive(cx, &scrut.ty, &harms, span);
        mk(H::Match(Box::new(scrut), harms), ty, span)
    }

    /// A match scrutinee needs a concrete type; a compile-time constant gets
    /// the smallest unsigned width that fits.
    pub fn settle_scrutinee(&mut self, cx: &FnCtx, scrut: Expr) -> Expr {
        match &scrut.ty {
            Ty::Const(v) if *v >= 0 => {
                let ty = Ty::UInt(bits_for(*v as u64));
                self.coerce(cx, scrut, &ty)
            }
            Ty::Const(_) => {
                self.error(
                    cx.module,
                    "E1018",
                    scrut.span,
                    "cannot infer the width of this negative constant; add a cast",
                );
                err(scrut.span)
            }
            Ty::UInt(_) | Ty::SInt(_) | Ty::Bool | Ty::Enum(_) | Ty::Error => scrut,
            t => {
                let msg = format!("cannot match on a value of type `{}`", self.display(t));
                self.error(cx.module, "E1012", scrut.span, msg);
                err(scrut.span)
            }
        }
    }
}

fn op_str(op: BinaryOp) -> &'static str {
    use BinaryOp::*;
    match op {
        Mul => "*",
        Div => "/",
        Rem => "%",
        Add => "+",
        Sub => "-",
        Shl => "<<",
        Shr => ">>",
        BitAnd => "&",
        BitXor => "^",
        BitOr => "|",
        Eq => "==",
        Ne => "!=",
        Lt => "<",
        Le => "<=",
        Gt => ">",
        Ge => ">=",
        And => "&&",
        Or => "||",
    }
}

/// Fold an operation on two literals already typed as `ty`, with the type's
/// wrap-around semantics.
fn fold_in_type(op: BinaryOp, x: i128, y: i128, ty: &Ty) -> Option<i128> {
    use BinaryOp::*;
    let (signed, w) = match ty {
        Ty::UInt(w) => (false, *w),
        Ty::SInt(w) => (true, *w),
        _ => return None,
    };
    let r = match op {
        Add => x.wrapping_add(y),
        Sub => x.wrapping_sub(y),
        Mul => x.wrapping_mul(y),
        Div => x.checked_div(y)?,
        Rem => x.checked_rem(y)?,
        BitAnd => x & y,
        BitOr => x | y,
        BitXor => x ^ y,
        _ => return None,
    };
    Some(truncate(r, signed, w))
}
