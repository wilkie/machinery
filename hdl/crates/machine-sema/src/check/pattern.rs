//! Match patterns and exhaustiveness.

use machine_syntax::ast;
use machine_syntax::diagnostics::Diagnostic;
use machine_syntax::span::Span;

use super::{Checker, FnCtx};
use crate::consteval::ConstVal;
use crate::hir::{Arm, Pat};
use crate::ty::Ty;

impl Checker<'_> {
    /// Check the patterns of one arm against the scrutinee type.
    pub fn check_patterns(
        &mut self,
        cx: &mut FnCtx,
        pats: &[ast::Pattern],
        scrut: &Ty,
    ) -> Vec<Pat> {
        pats.iter()
            .map(|p| self.check_pattern(cx, p, scrut))
            .collect()
    }

    fn check_pattern(&mut self, cx: &mut FnCtx, p: &ast::Pattern, scrut: &Ty) -> Pat {
        match &p.kind {
            ast::PatternKind::Wildcard => Pat::Wildcard,
            ast::PatternKind::Range { lo, hi, inclusive } => {
                if !matches!(scrut, Ty::UInt(_) | Ty::SInt(_)) {
                    let msg = format!(
                        "range patterns apply to integers, not `{}`",
                        self.display(scrut)
                    );
                    self.error(cx.module, "E1012", p.span, msg);
                    return Pat::Wildcard;
                }
                let generics = cx.generics.clone();
                let lo_v = self
                    .eval_const(cx.module, &generics, lo)
                    .and_then(ConstVal::as_int);
                let hi_v = self
                    .eval_const(cx.module, &generics, hi)
                    .and_then(ConstVal::as_int);
                let (Some(lo_v), Some(mut hi_v)) = (lo_v, hi_v) else {
                    return Pat::Wildcard;
                };
                if !*inclusive {
                    hi_v -= 1;
                }
                if hi_v < lo_v {
                    self.error(cx.module, "E1017", p.span, "empty range pattern");
                    return Pat::Wildcard;
                }
                for v in [lo_v, hi_v] {
                    if !scrut.fits(v) {
                        let msg = format!(
                            "pattern value {v} does not fit in `{}`",
                            self.display(scrut)
                        );
                        self.error(cx.module, "E1006", p.span, msg);
                        return Pat::Wildcard;
                    }
                }
                Pat::Range(lo_v, hi_v)
            }
            ast::PatternKind::Expr(e) => match scrut {
                Ty::Enum(def) => {
                    let ast::ExprKind::Path(path) = &e.kind else {
                        self.error(
                            cx.module,
                            "E1012",
                            e.span,
                            "patterns for an enum are its variants (`Enum::Variant`)",
                        );
                        return Pat::Wildcard;
                    };
                    if path.segments.len() != 2 {
                        self.error(
                            cx.module,
                            "E1012",
                            e.span,
                            "patterns for an enum are its variants (`Enum::Variant`)",
                        );
                        return Pat::Wildcard;
                    }
                    let enum_name = self.prog.item_name(*def);
                    if path.segments[0].name != enum_name {
                        let msg = format!(
                            "expected a variant of `{enum_name}`, found `{}`",
                            path.dotted()
                        );
                        self.error(cx.module, "E1005", e.span, msg);
                        return Pat::Wildcard;
                    }
                    let Some(info) = self.enum_info(*def) else {
                        return Pat::Wildcard;
                    };
                    match info
                        .variants
                        .iter()
                        .position(|(n, _)| *n == path.segments[1].name)
                    {
                        Some(idx) => Pat::Variant(idx as u32),
                        None => {
                            let msg = format!(
                                "enum `{enum_name}` has no variant `{}`",
                                path.segments[1].name
                            );
                            self.error(cx.module, "E1000", path.segments[1].span, msg);
                            Pat::Wildcard
                        }
                    }
                }
                Ty::Bool => match &e.kind {
                    ast::ExprKind::Bool(b) => Pat::Bool(*b),
                    _ => {
                        self.error(
                            cx.module,
                            "E1012",
                            e.span,
                            "patterns for `bool` are `true` and `false`",
                        );
                        Pat::Wildcard
                    }
                },
                Ty::UInt(_) | Ty::SInt(_) => {
                    let generics = cx.generics.clone();
                    match self.eval_const(cx.module, &generics, e) {
                        Some(ConstVal::Int(v)) => {
                            if !scrut.fits(v) {
                                let msg = format!(
                                    "pattern value {v} does not fit in `{}`",
                                    self.display(scrut)
                                );
                                self.error(cx.module, "E1006", e.span, msg);
                                return Pat::Wildcard;
                            }
                            Pat::Value(v)
                        }
                        Some(ConstVal::Bool(_)) => {
                            self.error(cx.module, "E1012", e.span, "integer pattern expected");
                            Pat::Wildcard
                        }
                        None => Pat::Wildcard,
                    }
                }
                _ => Pat::Wildcard,
            },
        }
    }

    /// Report a non-exhaustive match.
    pub fn check_exhaustive(&mut self, cx: &FnCtx, scrut: &Ty, arms: &[Arm], span: Span) {
        let all: Vec<&Pat> = arms.iter().flat_map(|a| a.patterns.iter()).collect();
        if all.iter().any(|p| matches!(p, Pat::Wildcard)) {
            return;
        }
        let missing: Option<String> = match scrut {
            Ty::Enum(def) => {
                let Some(info) = self.enum_info(*def) else {
                    return;
                };
                let covered: Vec<bool> = (0..info.variants.len())
                    .map(|i| {
                        all.iter()
                            .any(|p| matches!(p, Pat::Variant(v) if *v as usize == i))
                    })
                    .collect();
                let missing: Vec<String> = info
                    .variants
                    .iter()
                    .zip(&covered)
                    .filter(|(_, c)| !**c)
                    .map(|((n, _), _)| format!("`{}::{}`", self.prog.item_name(*def), n))
                    .collect();
                if missing.is_empty() {
                    None
                } else {
                    Some(format!("variants not covered: {}", missing.join(", ")))
                }
            }
            Ty::Bool => {
                let t = all.iter().any(|p| matches!(p, Pat::Bool(true)));
                let f = all.iter().any(|p| matches!(p, Pat::Bool(false)));
                match (t, f) {
                    (true, true) => None,
                    (true, false) => Some("`false` is not covered".into()),
                    (false, true) => Some("`true` is not covered".into()),
                    (false, false) => Some("`true` and `false` are not covered".into()),
                }
            }
            Ty::UInt(w) | Ty::SInt(w) => {
                if *w > 120 {
                    Some("matches on values wider than 120 bits need a `_` arm".into())
                } else {
                    let (min, max) = match scrut {
                        Ty::UInt(w) => (0i128, (1i128 << *w) - 1),
                        _ => (-(1i128 << (*w - 1)), (1i128 << (*w - 1)) - 1),
                    };
                    let mut ranges: Vec<(i128, i128)> = all
                        .iter()
                        .filter_map(|p| match p {
                            Pat::Value(v) => Some((*v, *v)),
                            Pat::Range(a, b) => Some((*a, *b)),
                            _ => None,
                        })
                        .collect();
                    ranges.sort_unstable();
                    let mut next = min;
                    let mut gap: Option<(i128, i128)> = None;
                    for (a, b) in ranges {
                        if a > next {
                            gap = Some((next, a - 1));
                            break;
                        }
                        next = next.max(b + 1);
                    }
                    if gap.is_none() && next <= max {
                        gap = Some((next, max));
                    }
                    gap.map(|(a, b)| {
                        if a == b {
                            format!("value {a} is not covered")
                        } else {
                            format!("values {a}..={b} are not covered")
                        }
                    })
                }
            }
            _ => None,
        };
        if let Some(what) = missing {
            self.error_with(
                cx.module,
                Diagnostic::error("E1010", span, format!("non-exhaustive match: {what}"))
                    .with_note("add the missing arms or a `_ =>` arm"),
            );
        }
    }
}
