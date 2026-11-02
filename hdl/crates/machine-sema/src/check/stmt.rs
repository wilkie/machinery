//! Statement checking for unit, isa (fn, kind rule, instruction semantics)
//! and test bodies.

use machine_syntax::ast::{self, StmtKind as A};
use machine_syntax::diagnostics::Diagnostic;
use machine_syntax::span::Span;

use super::{Checker, FnCtx};
use crate::builtins::{bits_for, Builtin};
use crate::consteval::ConstVal;
use crate::hir::{self, Block, ExprKind, LetTarget, RuleKind, Stmt, StmtKind as H};
use crate::ty::Ty;

impl Checker<'_> {
    pub fn check_block(&mut self, cx: &mut FnCtx, b: &ast::Block) -> Block {
        cx.push_scope();
        let mut stmts = Vec::new();
        let mut diverged_at: Option<Span> = None;
        let n = b.stmts.len();
        for (i, s) in b.stmts.iter().enumerate() {
            if let Some(at) = diverged_at {
                self.error_with(
                    cx.module,
                    Diagnostic::warning("W1001", s.span, "unreachable statement")
                        .with_label(at, "control flow ends here"),
                );
                diverged_at = None; // report once per block
            }
            let hs = self.check_stmt(cx, s, i + 1 == n);
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

    /// Where are we, for diagnostics?
    fn where_(&self, cx: &FnCtx) -> &'static str {
        if cx.in_test {
            "a test body"
        } else if cx.component.is_some() {
            "a model body"
        } else if cx.scope.isa.is_some() {
            "isa code"
        } else {
            "a unit"
        }
    }

    /// `is_last`: the statement closes its block, so an expression here may
    /// be the block's value and is not "unused".
    fn check_stmt(&mut self, cx: &mut FnCtx, s: &ast::Stmt, is_last: bool) -> Stmt {
        let span = s.span;
        let kind = match &s.kind {
            A::Let {
                pattern,
                mutable,
                ty,
                value,
            } => {
                let declared = match ty {
                    Some(t) => {
                        let generics = cx.generics.clone();
                        Some(self.resolve_type(cx.scope, &generics, t))
                    }
                    None => None,
                };
                let value = value
                    .as_ref()
                    .map(|v| self.check_expr(cx, v, declared.as_ref()));
                if *mutable {
                    let ast::LetPattern::Name(n) = pattern else {
                        unreachable!("the parser allows `let mut` on one name")
                    };
                    // A mutable local has a runtime type: a bare literal
                    // takes its declared type, or is an error without one.
                    let ty = match (&declared, &value) {
                        (Some(t), _) => t.clone(),
                        (None, Some(v)) if matches!(v.ty, Ty::Const(_)) => {
                            self.error_with(
                                cx.module,
                                Diagnostic::error(
                                    "E1004",
                                    n.span,
                                    "a mutable local needs a type when its value is a bare literal",
                                )
                                .with_note("write `let mut x: u8 = 0;`"),
                            );
                            Ty::Error
                        }
                        (None, Some(v)) => v.ty.clone(),
                        (None, None) => Ty::Error,
                    };
                    let value = value.map(|v| self.coerce(cx, v, &ty));
                    self.check_local_name(cx, &n.name, n.span);
                    let id = cx.declare_local(&n.name, ty, n.span, true);
                    if value.is_none() {
                        cx.unassigned.insert(id);
                    }
                    H::Let(LetTarget::Local(id), value)
                } else {
                    let value = value.expect("the parser requires a value for `let`");
                    let target = self.bind_pattern(cx, pattern, &value.ty);
                    H::Let(target, Some(value))
                }
            }
            A::Assign { target, value } => {
                if cx.scope.isa.is_none()
                    && cx.component.is_none()
                    && !self.assigns_local(cx, target)
                {
                    let where_ = self.where_(cx);
                    self.error_with(
                        cx.module,
                        Diagnostic::error("E1020", span, format!("wire drive (`=`) is not allowed in {where_}"))
                            .with_note("units are pure functions: bind values with `let` and produce results with `return`"),
                    );
                    H::Error
                } else if let Some(idx) = self.operand_target(cx, target) {
                    self.check_operand_write(cx, idx, value, span)
                } else {
                    match self.check_assign(cx, target, value) {
                        Some((place, v)) => H::Assign(place, v),
                        None => H::Error,
                    }
                }
            }
            A::Schedule { .. } => {
                let where_ = self.where_(cx);
                self.error_with(
                    cx.module,
                    Diagnostic::error("E1020", span, format!("register update (`<-`) is not allowed in {where_}"))
                        .with_note("`<-` schedules a clocked register in rtl; architectural state is assigned with `=`"),
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
                    ExprKind::SpaceWrite(def, bits, addr, value) => {
                        H::SpaceWrite(def, bits, *addr, *value)
                    }
                    kind => {
                        let he = hir::Expr { kind, ..he };
                        if !is_last
                            && !matches!(he.ty, Ty::Unit | Ty::Never | Ty::Error)
                            && !is_block_like(e)
                        {
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
                let before = cx.unassigned.clone();
                let then_b = self.check_block(cx, then_branch);
                let after_then = falls_through(&then_b).then(|| cx.unassigned.clone());
                cx.unassigned = before.clone();
                let else_s = else_branch
                    .as_ref()
                    .map(|e| Box::new(self.check_stmt(cx, e, true)));
                let after_else = match &else_s {
                    Some(e)
                        if !falls_through(&Block {
                            stmts: vec![(**e).clone()],
                            span: e.span,
                        }) =>
                    {
                        None
                    }
                    _ => Some(cx.unassigned.clone()),
                };
                cx.unassigned = join_unassigned([after_then, after_else]);
                H::If(cond, then_b, else_s)
            }
            A::Match { scrutinee, arms } => {
                let scrut = self.check_expr(cx, scrutinee, None);
                let scrut = self.settle_scrutinee(cx, scrut);
                let mut harms = Vec::new();
                let before = cx.unassigned.clone();
                let mut afters = Vec::new();
                for arm in arms {
                    cx.unassigned = before.clone();
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
                    let diverges = match &body {
                        hir::ArmBody::Block(b) => !falls_through(b),
                        hir::ArmBody::Expr(e) => matches!(e.ty, Ty::Never),
                    };
                    afters.push((!diverges).then(|| cx.unassigned.clone()));
                    harms.push(hir::Arm {
                        patterns: pats,
                        body,
                        span: arm.span,
                    });
                }
                cx.unassigned = join_unassigned(afters);
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
                    .eval_const(cx.scope, &generics, lo)
                    .and_then(ConstVal::as_int);
                let hi_v = self
                    .eval_const(cx.scope, &generics, hi)
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
                self.check_local_name(cx, &var.name, var.span);
                let id = cx.declare(&var.name, var_ty, var.span);
                // Assignments in a loop body do not count after the loop.
                let before = cx.unassigned.clone();
                let body = self.check_block(cx, body);
                cx.unassigned = before;
                cx.pop_scope();
                H::For(id, lo_v, hi_excl, body)
            }
            A::While { cond, body } if cx.component.is_some() => {
                let c = self.check_expr(cx, cond, Some(&Ty::Bool));
                cx.loop_depth += 1;
                let before = cx.unassigned.clone();
                let b = self.check_block(cx, body);
                cx.unassigned = before;
                cx.loop_depth -= 1;
                H::While(c, b)
            }
            A::Loop(body) if cx.component.is_some() => {
                cx.loop_depth += 1;
                let before = cx.unassigned.clone();
                let b = self.check_block(cx, body);
                cx.unassigned = before;
                cx.loop_depth -= 1;
                H::Loop(b)
            }
            A::Break | A::Continue if cx.component.is_some() => {
                if cx.loop_depth == 0 {
                    let what = if matches!(s.kind, A::Break) {
                        "break"
                    } else {
                        "continue"
                    };
                    self.error(
                        cx.module,
                        "E1020",
                        span,
                        format!("`{what}` outside of a loop"),
                    );
                    H::Error
                } else if matches!(s.kind, A::Break) {
                    H::Break
                } else {
                    H::Continue
                }
            }
            A::Loop(_) | A::While { .. } | A::Break | A::Continue => {
                let where_ = self.where_(cx);
                self.error_with(
                    cx.module,
                    Diagnostic::error("E1020", span, format!("loops are not allowed in {where_}"))
                        .with_note("use `for i in 0..N` with constant bounds; it is unrolled (models may loop freely)"),
                );
                H::Error
            }
            A::Emit { target, value } => match self.check_emit(cx, target, value.as_ref(), span) {
                Some(k) => k,
                None => H::Error,
            },
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
                            self.error(cx.module, "E1005", v.span, "this body returns nothing");
                            self.check_expr(cx, v, None);
                            H::Return(None)
                        } else {
                            let e = self.check_expr(cx, v, Some(&ret));
                            H::Return(Some(e))
                        }
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
            A::Halt => {
                if cx.scope.isa.is_none() || cx.in_test || cx.component.is_some() {
                    self.error(
                        cx.module,
                        "E1020",
                        span,
                        "`halt` is only allowed in isa code",
                    );
                    H::Error
                } else {
                    H::Halt
                }
            }
            A::Raise(e) => {
                if cx.scope.isa.is_none() {
                    self.error(
                        cx.module,
                        "E1020",
                        span,
                        "`raise` is only allowed in isa code",
                    );
                    H::Unreachable(None)
                } else {
                    match self.check_raise(cx, e) {
                        Some((def, args)) => H::Raise(def, args),
                        // Still diverges, so the error does not cascade into
                        // "does not return on every path".
                        None => H::Unreachable(None),
                    }
                }
            }
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
            A::NamedBlock { name, body }
                if name.name == "given" && cx.in_test && cx.scope.isa.is_some() =>
            {
                H::Given(self.check_given(cx, body))
            }
            A::Run(spec) if cx.in_test && cx.scope.isa.is_some() => match spec {
                ast::RunSpec::Instruction { name, operands } => {
                    match self.check_run_instruction(cx, name, operands, span) {
                        Some(k) => k,
                        None => H::Error,
                    }
                }
                ast::RunSpec::Bytes(e) => match self.check_array_of(cx, e, &Ty::UInt(8)) {
                    Some(items) => {
                        let mut bytes = Vec::new();
                        let mut ok = true;
                        for it in &items {
                            match it.kind {
                                ExprKind::Int(v) => bytes.push(v as u8),
                                _ => {
                                    self.error(
                                        cx.module,
                                        "E1013",
                                        it.span,
                                        "`run bytes` takes constant bytes",
                                    );
                                    ok = false;
                                }
                            }
                        }
                        if ok {
                            H::RunBytes(bytes)
                        } else {
                            H::Error
                        }
                    }
                    None => H::Error,
                },
                ast::RunSpec::Cycles { .. } | ast::RunSpec::Block(_) => {
                    self.error_with(
                        cx.module,
                        Diagnostic::error(
                            "E1020",
                            span,
                            "an isa test runs instructions, not cycles",
                        )
                        .with_note("use `run instruction Name { ... };` or `run bytes [...];`"),
                    );
                    H::Error
                }
            },
            A::NamedBlock { .. } | A::Run(_) if cx.in_test => {
                let what = match &s.kind {
                    A::NamedBlock { name, .. } => format!("`{}`", name.name),
                    _ => "`run`".to_string(),
                };
                self.error_with(
                    cx.module,
                    Diagnostic::error("E1020", span, format!("{what} needs a test target"))
                        .with_note("write `test \"name\" on <Isa or Component.level> { ... }`; component tests are not checked yet"),
                );
                H::Error
            }
            A::Error => H::Error,
            other => {
                let what = stmt_name(other);
                let where_ = self.where_(cx);
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

    /// Is this assignment target a bare instruction operand name?
    fn operand_target(&self, cx: &FnCtx, target: &ast::Expr) -> Option<u32> {
        let ast::ExprKind::Path(p) = &target.kind else {
            return None;
        };
        if p.segments.len() != 1 {
            return None;
        }
        let name = &p.segments[0].name;
        if cx.lookup_local(name).is_some() {
            return None;
        }
        cx.lookup_operand(name)
    }

    fn check_operand_write(
        &mut self,
        cx: &mut FnCtx,
        idx: u32,
        value: &ast::Expr,
        span: Span,
    ) -> H {
        let (oname, kind) = cx.operands[idx as usize].clone();
        let Some(sig) = self.kinds.get(&kind).cloned() else {
            return H::Error;
        };
        let writable = match sig.source {
            None => sig.rule(RuleKind::Select).is_some(),
            Some(_) => sig.rule(RuleKind::Write).is_some(),
        };
        if !writable {
            let msg = format!(
                "operand `{oname}` of kind `{}` is read-only (the kind has no `write` rule)",
                self.prog.item_name(kind)
            );
            self.error(cx.module, "E1037", span, msg);
            self.check_expr(cx, value, None);
            return H::Error;
        }
        let v = self.check_expr(cx, value, Some(&sig.ty));
        H::OperandWrite(idx, v)
    }

    fn bind_pattern(&mut self, cx: &mut FnCtx, pat: &ast::LetPattern, ty: &Ty) -> LetTarget {
        match pat {
            ast::LetPattern::Wildcard(_) => LetTarget::Discard,
            ast::LetPattern::Name(n) => {
                self.check_local_name(cx, &n.name, n.span);
                LetTarget::Local(cx.declare(&n.name, ty.clone(), n.span))
            }
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
        A::Halt => "halt",
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
        H::Return(_) | H::Unreachable(_) | H::Raise(..) => true,
        H::Expr(e) => matches!(e.ty, Ty::Never),
        // A `loop` with no `break` never ends.
        H::Loop(b) => !block_has_break(b),
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

/// Does the block contain a `break` for the enclosing loop (nested loops
/// have their own)?
fn block_has_break(b: &Block) -> bool {
    b.stmts.iter().any(stmt_has_break)
}

fn stmt_has_break(s: &Stmt) -> bool {
    match &s.kind {
        H::Break => true,
        H::If(_, t, e) => block_has_break(t) || e.as_ref().is_some_and(|s| stmt_has_break(s)),
        H::Match(_, arms) => arms.iter().any(|a| match &a.body {
            hir::ArmBody::Block(b) => block_has_break(b),
            hir::ArmBody::Expr(_) => false,
        }),
        H::Expr(hir::Expr {
            kind: hir::ExprKind::Block(b),
            ..
        }) => block_has_break(b),
        _ => false,
    }
}

/// Does control reach the end of this block? Not when it returns, raises,
/// or ends in `break` or `continue` (for definite assignment).
fn falls_through(b: &Block) -> bool {
    if block_diverges(b) {
        return false;
    }
    match b.stmts.last().map(|s| &s.kind) {
        Some(H::Break | H::Continue) => false,
        Some(H::If(_, t, Some(e))) => {
            falls_through(t)
                || falls_through(&Block {
                    stmts: vec![(**e).clone()],
                    span: e.span,
                })
        }
        _ => true,
    }
}

/// After branches join, a local is unassigned if it is unassigned at the
/// end of any branch that falls through. `None` is a branch that diverges.
pub(super) fn join_unassigned(
    branches: impl IntoIterator<Item = Option<std::collections::BTreeSet<hir::LocalId>>>,
) -> std::collections::BTreeSet<hir::LocalId> {
    let mut out = std::collections::BTreeSet::new();
    for b in branches.into_iter().flatten() {
        out.extend(b);
    }
    out
}

impl Checker<'_> {
    /// A `let` or `for` local may not share its name with a port, state,
    /// mem, register, register field or view, latch, mode or space visible
    /// in the body, so that `x = e;` has one meaning (E1001).
    pub(super) fn check_local_name(
        &mut self,
        cx: &FnCtx,
        name: &str,
        span: machine_syntax::span::Span,
    ) {
        let mut what = None;
        if let Some(comp) = cx.component {
            if let Some(cs) = self.comps.get(&comp) {
                what = cs.names.get(name).map(|e| match e {
                    super::CompEntry::Port(_) => "a port",
                    super::CompEntry::State(_) => "a `state`",
                    super::CompEntry::Mem(_) => "a `mem`",
                });
            }
        }
        if what.is_none() {
            if let Some(isa) = cx.scope.isa {
                what = match self.prog.lookup_in_ns(isa, name) {
                    Some(crate::program::ScopeEntry::RegField { .. }) => Some("a register field"),
                    Some(crate::program::ScopeEntry::RegView { .. }) => Some("a register view"),
                    Some(crate::program::ScopeEntry::Item(d)) => match self.prog.item(d).kind {
                        crate::program::ItemKind::Register => Some("a register"),
                        crate::program::ItemKind::Latch => Some("a latch"),
                        crate::program::ItemKind::Space => Some("an address space"),
                        crate::program::ItemKind::Mode => Some("the mode register"),
                        _ => None,
                    },
                    Some(_) => Some("the mode register"),
                    None => None,
                };
            }
        }
        if let Some(what) = what {
            self.error_with(
                cx.module,
                Diagnostic::error(
                    "E1001",
                    span,
                    format!("`{name}` is {what} of this body; a local may not share its name"),
                )
                .with_note("rename the local, so that `name = value;` has one meaning"),
            );
        }
    }

    /// Does `target` name a local (or a field or element of one)? Units
    /// may assign mutable locals; others get E1050 from `check_assign`.
    fn assigns_local(&self, cx: &FnCtx, target: &ast::Expr) -> bool {
        let mut e = target;
        loop {
            match &e.kind {
                ast::ExprKind::Field(b, _) | ast::ExprKind::Index(b, _) => e = b,
                ast::ExprKind::Path(p) if p.segments.len() == 1 => {
                    return cx.lookup_local(&p.segments[0].name).is_some();
                }
                _ => return false,
            }
        }
    }
}
