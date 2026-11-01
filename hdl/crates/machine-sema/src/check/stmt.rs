//! Statement checking for unit and test bodies.

use machine_syntax::ast::{self, StmtKind as A};
use machine_syntax::diagnostics::Diagnostic;
use machine_syntax::span::Span;

use super::{Checker, FnCtx};
use crate::builtins::{bits_for, Builtin};
use crate::consteval::ConstVal;
use crate::hir::{self, Block, ExprKind, LetTarget, Stmt, StmtKind as H};
use crate::ty::Ty;

impl Checker<'_> {
    pub fn check_block(&mut self, cx: &mut FnCtx, b: &ast::Block) -> Block {
        cx.push_scope();
        let mut stmts = Vec::new();
        let mut diverged_at: Option<Span> = None;
        for s in &b.stmts {
            if let Some(at) = diverged_at {
                self.error_with(
                    cx.module,
                    Diagnostic::warning("W1001", s.span, "unreachable statement")
                        .with_label(at, "control flow ends here"),
                );
                diverged_at = None; // report once per block
            }
            let hs = self.check_stmt(cx, s);
            if stmt_diverges(&hs) {
                diverged_at = Some(hs.span);
            }
            stmts.push(hs);
        }
        cx.pop_scope();
        Block {
            stmts,
            span: b.span,
        }
    }

    fn check_stmt(&mut self, cx: &mut FnCtx, s: &ast::Stmt) -> Stmt {
        let span = s.span;
        let kind = match &s.kind {
            A::Let { pattern, ty, value } => {
                let declared = match ty {
                    Some(t) => {
                        let generics = cx.generics.clone();
                        Some(self.resolve_type(cx.module, &generics, t))
                    }
                    None => None,
                };
                let value = self.check_expr(cx, value, declared.as_ref());
                let target = self.bind_pattern(cx, pattern, &value.ty);
                H::Let(target, value)
            }
            A::Assign { .. } | A::Schedule { .. } => {
                let what = if matches!(s.kind, A::Assign { .. }) {
                    "wire drive (`=`)"
                } else {
                    "register update (`<-`)"
                };
                let where_ = if cx.in_test { "a test body" } else { "a unit" };
                self.error_with(
                    cx.module,
                    Diagnostic::error("E1020", span, format!("{what} is not allowed in {where_}"))
                        .with_note("units are pure functions: bind values with `let` and produce results with `return`"),
                );
                H::Error
            }
            A::Expr(e) => {
                let he = self.check_expr(cx, e, None);
                match he.kind {
                    ExprKind::Builtin(Builtin::Assert, mut args) => {
                        let msg = if args.len() == 2 {
                            args.pop().and_then(|m| match m.kind {
                                ExprKind::Str(s) => Some(s),
                                _ => None,
                            })
                        } else {
                            None
                        };
                        let cond = args.remove(0);
                        H::Assert(cond, msg)
                    }
                    ExprKind::Builtin(Builtin::Unreachable, args) => {
                        let msg = args.into_iter().next().and_then(|m| match m.kind {
                            ExprKind::Str(s) => Some(s),
                            _ => None,
                        });
                        H::Unreachable(msg)
                    }
                    kind => {
                        let he = hir::Expr { kind, ..he };
                        if !matches!(he.ty, Ty::Unit | Ty::Never | Ty::Error) && !is_block_like(e) {
                            self.warning(
                                cx.module,
                                "W1002",
                                span,
                                "the value of this expression is unused",
                            );
                        }
                        H::Expr(he)
                    }
                }
            }
            A::If {
                cond,
                then_branch,
                else_branch,
            } => {
                let cond = self.check_expr(cx, cond, Some(&Ty::Bool));
                let then_b = self.check_block(cx, then_branch);
                let else_s = else_branch
                    .as_ref()
                    .map(|e| Box::new(self.check_stmt(cx, e)));
                H::If(cond, then_b, else_s)
            }
            A::Match { scrutinee, arms } => {
                let scrut = self.check_expr(cx, scrutinee, None);
                let scrut = self.settle_scrutinee(cx, scrut);
                let mut harms = Vec::new();
                for arm in arms {
                    let pats = if scrut.ty.is_error() {
                        Vec::new()
                    } else {
                        self.check_patterns(cx, &arm.patterns, &scrut.ty)
                    };
                    cx.push_scope();
                    let body = match &arm.body {
                        ast::ArmBody::Block(b) => hir::ArmBody::Block(self.check_block(cx, b)),
                        ast::ArmBody::Expr(e) => {
                            let he = self.check_expr(cx, e, None);
                            if !matches!(he.ty, Ty::Unit | Ty::Never | Ty::Error) {
                                self.warning(
                                    cx.module,
                                    "W1002",
                                    e.span,
                                    "the value of this match arm is unused",
                                );
                            }
                            hir::ArmBody::Expr(he)
                        }
                    };
                    cx.pop_scope();
                    harms.push(hir::Arm {
                        patterns: pats,
                        body,
                        span: arm.span,
                    });
                }
                if !scrut.ty.is_error() {
                    self.check_exhaustive(cx, &scrut.ty, &harms, span);
                }
                H::Match(scrut, harms)
            }
            A::For { var, range, body } => {
                let ast::ExprKind::Range { lo, hi, inclusive } = &range.kind else {
                    self.error(
                        cx.module,
                        "E1013",
                        range.span,
                        "`for` iterates over a constant range: `for i in 0..N`",
                    );
                    return Stmt {
                        kind: H::Error,
                        span,
                    };
                };
                let generics = cx.generics.clone();
                let lo_v = self
                    .eval_const(cx.module, &generics, lo)
                    .and_then(ConstVal::as_int);
                let hi_v = self
                    .eval_const(cx.module, &generics, hi)
                    .and_then(ConstVal::as_int);
                let (Some(lo_v), Some(hi_v)) = (lo_v, hi_v) else {
                    return Stmt {
                        kind: H::Error,
                        span,
                    };
                };
                let hi_excl = if *inclusive { hi_v + 1 } else { hi_v };
                if lo_v < 0 {
                    self.error(
                        cx.module,
                        "E1013",
                        lo.span,
                        "`for` ranges start at or above 0",
                    );
                    return Stmt {
                        kind: H::Error,
                        span,
                    };
                }
                let max = (hi_excl - 1).max(lo_v).max(0);
                let var_ty = Ty::UInt(bits_for(max as u64));
                cx.push_scope();
                let id = cx.declare(&var.name, var_ty, var.span);
                let body = self.check_block(cx, body);
                cx.pop_scope();
                H::For(id, lo_v, hi_excl, body)
            }
            A::Loop(_) | A::While { .. } | A::Break | A::Continue => {
                let where_ = if cx.in_test { "test bodies" } else { "units" };
                self.error_with(
                    cx.module,
                    Diagnostic::error("E1020", span, format!("loops are not allowed in {where_}"))
                        .with_note("use `for i in 0..N` with constant bounds; it is unrolled"),
                );
                H::Error
            }
            A::Return(value) => match cx.ret.clone() {
                None => {
                    self.error(
                        cx.module,
                        "E1020",
                        span,
                        "`return` is not allowed in a test body",
                    );
                    H::Error
                }
                Some(ret) => match value {
                    Some(v) => {
                        if matches!(ret, Ty::Unit) {
                            self.error(cx.module, "E1005", v.span, "this unit returns nothing");
                        }
                        let e = self.check_expr(cx, v, Some(&ret));
                        H::Return(Some(e))
                    }
                    None => {
                        if !matches!(ret, Ty::Unit | Ty::Error) {
                            let msg =
                                format!("`return` needs a value of type `{}`", self.display(&ret));
                            self.error(cx.module, "E1005", span, msg);
                        }
                        H::Return(None)
                    }
                },
            },
            A::NamedBlock { name, body } if name.name == "expect" && cx.in_test => {
                let mut checks = Vec::new();
                for st in &body.stmts {
                    match &st.kind {
                        A::Expr(e) => checks.push(self.check_expr(cx, e, Some(&Ty::Bool))),
                        A::Error => {}
                        _ => self.error(
                            cx.module,
                            "E1020",
                            st.span,
                            "`expect` contains boolean expressions only",
                        ),
                    }
                }
                H::Expect(checks)
            }
            A::NamedBlock { .. } | A::Run(_) if cx.in_test => {
                let what = match &s.kind {
                    A::NamedBlock { name, .. } => format!("`{}`", name.name),
                    _ => "`run`".to_string(),
                };
                self.error_with(
                    cx.module,
                    Diagnostic::error("E1020", span, format!("{what} needs a test target"))
                        .with_note("write `test \"name\" on <Isa or Component.level> { ... }`; targeted tests are not checked yet"),
                );
                H::Error
            }
            A::Error => H::Error,
            other => {
                let what = stmt_name(other);
                let where_ = if cx.in_test { "a test body" } else { "a unit" };
                self.error(
                    cx.module,
                    "E1020",
                    span,
                    format!("`{what}` is not allowed in {where_}"),
                );
                H::Error
            }
        };
        Stmt { kind, span }
    }

    fn bind_pattern(&mut self, cx: &mut FnCtx, pat: &ast::LetPattern, ty: &Ty) -> LetTarget {
        match pat {
            ast::LetPattern::Wildcard(_) => LetTarget::Discard,
            ast::LetPattern::Name(n) => LetTarget::Local(cx.declare(&n.name, ty.clone(), n.span)),
            ast::LetPattern::Tuple(items) => match ty {
                Ty::Tuple(elems) => {
                    if elems.len() != items.len() {
                        let msg = format!(
                            "pattern has {} elements but the value is a tuple of {}",
                            items.len(),
                            elems.len()
                        );
                        self.error(cx.module, "E1005", pattern_span(pat), msg);
                        for p in items {
                            self.bind_pattern(cx, p, &Ty::Error);
                        }
                        return LetTarget::Discard;
                    }
                    let targets = items
                        .iter()
                        .zip(elems)
                        .map(|(p, (_, t))| self.bind_pattern(cx, p, t))
                        .collect();
                    LetTarget::Tuple(targets)
                }
                Ty::Error => {
                    for p in items {
                        self.bind_pattern(cx, p, &Ty::Error);
                    }
                    LetTarget::Discard
                }
                t => {
                    let msg = format!("cannot destructure `{}` as a tuple", self.display(t));
                    self.error(cx.module, "E1005", pattern_span(pat), msg);
                    for p in items {
                        self.bind_pattern(cx, p, &Ty::Error);
                    }
                    LetTarget::Discard
                }
            },
        }
    }
}

fn pattern_span(p: &ast::LetPattern) -> Span {
    match p {
        ast::LetPattern::Wildcard(s) => *s,
        ast::LetPattern::Name(n) => n.span,
        ast::LetPattern::Tuple(items) => items.first().map(pattern_span).unwrap_or(Span::DUMMY),
    }
}

fn is_block_like(e: &ast::Expr) -> bool {
    matches!(
        e.kind,
        ast::ExprKind::Match { .. } | ast::ExprKind::If { .. } | ast::ExprKind::Block(_)
    )
}

fn stmt_name(k: &ast::StmtKind) -> &'static str {
    match k {
        A::Default(_) => "default",
        A::On { .. } => "on",
        A::Cycle(_) => "cycle",
        A::WaitUntil(_) => "wait until",
        A::Retire => "retire",
        A::Fault(_) => "fault",
        A::Raise(_) => "raise",
        A::Start(_) => "start",
        A::Dispatch { .. } => "dispatch",
        A::Emit { .. } => "emit",
        A::Reg { .. } => "reg",
        A::Wire { .. } => "wire",
        A::State { .. } => "state",
        A::Mem { .. } => "mem",
        A::Instance { .. } => "instance",
        A::Routine(_) => "routine",
        A::Sequencer(_) => "sequencer",
        A::Unit(_) => "unit",
        A::Macro { .. } => "macro",
        A::Architectural { .. } => "architectural",
        A::At(_) => "at",
        A::NamedBlock { .. } => "named block",
        A::Run(_) => "run",
        A::Stimulus(_) => "stimulus",
        A::Compare { .. } => "compare",
        A::Report(_) => "report",
        A::BeginsWhen(_) => "begins when",
        A::RespondsWith { .. } => "responds with",
        A::Hold { .. } => "hold",
        A::Drive { .. } => "drive",
        A::Sample { .. } => "sample",
        _ => "statement",
    }
}

/// Does control flow never continue past this statement?
pub fn stmt_diverges(s: &Stmt) -> bool {
    match &s.kind {
        H::Return(_) | H::Unreachable(_) => true,
        H::Expr(e) => matches!(e.ty, Ty::Never),
        H::If(_, then_b, Some(else_s)) => block_diverges(then_b) && stmt_diverges(else_s),
        H::Match(_, arms) => {
            !arms.is_empty()
                && arms.iter().all(|a| match &a.body {
                    hir::ArmBody::Block(b) => block_diverges(b),
                    hir::ArmBody::Expr(e) => matches!(e.ty, Ty::Never),
                })
        }
        _ => false,
    }
}

pub fn block_diverges(b: &Block) -> bool {
    b.stmts.iter().any(stmt_diverges)
}
