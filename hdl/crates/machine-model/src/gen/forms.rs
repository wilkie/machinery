//! Operand forms (SPEC 11, "Operand forms"): instructions specialised by
//! their operands' forms, chosen once where they are decoded.
//!
//! A CISC operand kind often behaves in one of a few ways, chosen by fields
//! of the operand: the 386's `rm` is a register when `m.mod == 3`, else
//! memory, and its `read` and `write` rules begin by testing that. The
//! fields are known once the instruction is decoded, but the test is made
//! again at every access, in rules shared by every instruction.
//!
//! The generator finds such kinds: a `read` rule whose body is an `if` on a
//! condition of the operand's fields alone. The kind gets a form fn (the
//! condition's value), and each rule a copy per form with the `if` decided:
//! `read` and `write` when they test that condition, `is_mem` when it is the
//! condition or its negation. Each instruction with such operands (up to
//! three) gets a copy of its semantics per combination of their forms,
//! calling those copies (`exec_<name>_f<n>`), and `run_<name>` picks the
//! copy from the operands. Each copy does exactly what the instruction
//! does for operands of those forms; the tests are made once per
//! instruction instead of once per access, and each copy's code is its own.

use std::collections::HashMap;
use std::fmt::Write as _;

use machine_sema::hir::{self, ArmBody, Expr, ExprKind as H};
use machine_sema::program::DefId;
use machine_syntax::ast::BinaryOp;

use super::{ident, Cx, Gen, R};

/// Most form operands an instruction is specialised over (copies: 2^n).
const MAX_FORM_OPERANDS: usize = 3;

/// A kind with forms: form 1 is its condition true.
#[derive(Clone, Debug)]
pub(crate) struct FormKind {
    /// `is_mem` in form 1, when it follows from the condition.
    pub is_mem_when_true: Option<bool>,
    /// `write` tests the same condition.
    pub write_forms: bool,
}

/// The `if` a rule body is: its condition and two branches.
fn top_if(b: &ArmBody) -> Option<(&Expr, &hir::Block, &Expr)> {
    let e = match b {
        ArmBody::Expr(e) => e,
        ArmBody::Block(b) if b.stmts.len() == 1 => match &b.stmts[0].kind {
            hir::StmtKind::Expr(e) => e,
            _ => return None,
        },
        _ => return None,
    };
    match &e.kind {
        H::If(c, t, Some(f)) => Some((c, t, f)),
        _ => None,
    }
}

/// Is `e` a function of the rule's parameters' fields alone?
fn of_fields(e: &Expr) -> bool {
    match &e.kind {
        H::Int(_) | H::Bool(_) | H::Const(_) | H::Variant(..) | H::Local(_) => true,
        H::OperandField(a, _) | H::OperandTail(a) | H::TailOperand(a, _) | H::Field(a, _) | H::TupleField(a, _) | H::Cast(a) | H::Unary(_, a) | H::Slice(a, ..) => of_fields(a),
        H::Binary(_, a, b) => of_fields(a) && of_fields(b),
        _ => false,
    }
}

impl Gen<'_> {
    fn rule(k: &hir::OperandKind, kind: hir::RuleKind) -> Option<&hir::KindRule> {
        k.rules.iter().find(|r| r.kind == kind)
    }

    fn rule_cx<'c>(rule: &'c hir::KindRule) -> Cx<'c> {
        Cx { locals: &rule.locals, fallible: true, recv: "self", ops: None, comp: None, opt: false }
    }

    /// The kinds with forms.
    pub(crate) fn plan_forms(&self) -> HashMap<DefId, FormKind> {
        let mut out = HashMap::new();
        for k in &self.isa.kinds {
            if k.source.is_none() {
                continue;
            }
            let Some(read) = Self::rule(k, hir::RuleKind::Read) else { continue };
            let Some((c, _, _)) = top_if(&read.body) else { continue };
            if !of_fields(c) {
                continue;
            }
            let cx = Self::rule_cx(read);
            let Ok(ct) = self.expr(&cx, c) else { continue };
            let write_forms = Self::rule(k, hir::RuleKind::Write).is_some_and(|w| {
                top_if(&w.body).is_some_and(|(c2, _, _)| self.expr(&Self::rule_cx(w), c2).ok().as_deref() == Some(ct.as_str()))
            });
            // `is_mem`: the condition (true in form 1), or its negation.
            let is_mem_when_true = Self::rule(k, hir::RuleKind::IsMem).and_then(|m| {
                let ArmBody::Expr(e) = &m.body else { return None };
                let mcx = Self::rule_cx(m);
                let et = self.expr(&mcx, e).ok()?;
                if et == ct {
                    return Some(true);
                }
                match (&e.kind, &c.kind) {
                    (H::Binary(BinaryOp::Ne, a, b), H::Binary(BinaryOp::Eq, a2, b2)) | (H::Binary(BinaryOp::Eq, a, b), H::Binary(BinaryOp::Ne, a2, b2)) => {
                        let same = self.expr(&mcx, a).ok()? == self.expr(&cx, a2).ok()? && self.expr(&mcx, b).ok()? == self.expr(&cx, b2).ok()?;
                        same.then_some(false)
                    }
                    _ => None,
                }
            });
            out.insert(k.def, FormKind { is_mem_when_true, write_forms });
        }
        out
    }

    /// A kind's form fn and its rules' copies per form.
    pub(crate) fn emit_kind_forms(&mut self, k: &hir::OperandKind) -> R<()> {
        let Some(fk) = self.forms.get(&k.def).cloned() else { return Ok(()) };
        let kname = ident(&k.name);
        let vty = self.rty(&k.ty)?;
        let read = Self::rule(k, hir::RuleKind::Read).unwrap();
        let (c, t, f) = top_if(&read.body).unwrap();
        let cx = Self::rule_cx(read);
        let p = self.local_name(&cx, read.params[0]);
        let pty = self.rty(&read.locals[read.params[0] as usize].ty)?;
        let ct = self.expr(&cx, c)?;
        let tt = self.block_expr(&cx, t)?;
        let ft = self.expr(&cx, f)?;
        let _ = write!(
            self.out,
            "    /// `{0}`'s form: 1 when `{0}` is the first branch of its rules.\n    #[inline]\n    pub fn kind_{kname}_form(&self, {p}: {pty}) -> u8 {{ ({ct}) as u8 }}\n    #[inline]\n    pub fn kind_{kname}_read_f1(&mut self, {p}: {pty}) -> Result<{vty}, Fault> {{ Ok({tt}) }}\n    #[inline]\n    pub fn kind_{kname}_read_f0(&mut self, {p}: {pty}) -> Result<{vty}, Fault> {{ Ok({ft}) }}\n",
            k.name
        );
        if let Some(w) = Self::rule(k, hir::RuleKind::Write) {
            let wcx = Self::rule_cx(w);
            let ps: R<Vec<String>> = w.params.iter().map(|q| Ok(format!("{}: {}", self.local_name(&wcx, *q), self.rty(&w.locals[*q as usize].ty)?))).collect();
            let ps = ps?.join(", ");
            let names: Vec<String> = w.params.iter().map(|q| self.local_name(&wcx, *q)).collect();
            if fk.write_forms {
                let (_, wt, wf) = top_if(&w.body).unwrap();
                let wtt = self.block_stmts(&wcx, wt)?;
                let wft = match &wf.kind {
                    H::Block(b) => self.block_stmts(&wcx, b)?,
                    _ => format!("{};\n", self.expr(&wcx, wf)?),
                };
                let _ = write!(
                    self.out,
                    "    #[inline]\n    pub fn kind_{kname}_write_f1(&mut self, {ps}) -> Result<(), Fault> {{\n{wtt}Ok(())\n}}\n    #[inline]\n    pub fn kind_{kname}_write_f0(&mut self, {ps}) -> Result<(), Fault> {{\n{wft}Ok(())\n}}\n"
                );
            } else {
                for n in 0..2 {
                    let _ = writeln!(self.out, "    #[inline]\n    pub fn kind_{kname}_write_f{n}(&mut self, {ps}) -> Result<(), Fault> {{ self.kind_{kname}_write({}) }}", names.join(", "));
                }
            }
        }
        if Self::rule(k, hir::RuleKind::IsMem).is_some() {
            for n in 0..2u8 {
                let body = match fk.is_mem_when_true {
                    Some(v) => format!("Ok({})", if n == 1 { v } else { !v }),
                    None => format!("self.kind_{kname}_is_mem({p})"),
                };
                let _ = writeln!(self.out, "    #[inline]\n    #[allow(unused_variables)]\n    pub fn kind_{kname}_is_mem_f{n}(&mut self, {p}: {pty}) -> Result<bool, Fault> {{ {body} }}");
            }
        }
        Ok(())
    }

    /// The operands of `i` with forms, by index (none when more than the
    /// copies allow).
    pub(crate) fn form_operands(&self, i: &hir::Instruction) -> Vec<u32> {
        let v: Vec<u32> = i.operands.iter().enumerate().filter(|(_, b)| self.forms.contains_key(&b.kind)).map(|(k, _)| k as u32).collect();
        if v.len() > MAX_FORM_OPERANDS {
            Vec::new()
        } else {
            v
        }
    }

    /// The copies of `i`'s semantics per form.
    pub(crate) fn emit_exec_forms(&mut self, i: &hir::Instruction) -> R<()> {
        let fo = self.form_operands(i);
        if fo.is_empty() {
            return Ok(());
        }
        let cx = Cx { locals: &i.locals, fallible: true, recv: "self", ops: Some(i), comp: None, opt: false };
        for n in 0..(1u32 << fo.len()) {
            *self.form_ctx.borrow_mut() = fo.iter().enumerate().map(|(b, k)| (*k, ((n >> b) & 1) as u8)).collect();
            let body = match &i.semantics {
                Some(b) => self.block_stmts(&cx, b),
                None => Ok(String::new()),
            };
            self.form_ctx.borrow_mut().clear();
            let body = body?;
            let _ = write!(self.out, "    #[inline]\n    #[allow(unused_variables)]\n    pub fn exec_{}_f{n}(&mut self, ops: {}) -> Result<(), Fault> {{\n{body}Ok(())\n}}\n", ident(&i.name), Self::ops_struct_name(i));
        }
        Ok(())
    }

    /// How `run_<name>` runs `i`'s semantics: the copy for its operands'
    /// forms, else the semantics.
    pub(crate) fn exec_call(&self, i: &hir::Instruction) -> String {
        if i.repeat.is_some() {
            return format!("self.repeat_{}(ops)", ident(&i.name));
        }
        self.exec_element_call(i)
    }

    /// The call of `i`'s semantics alone (one element, when it repeats).
    pub(crate) fn exec_element_call(&self, i: &hir::Instruction) -> String {
        let n = ident(&i.name);
        let fo = self.form_operands(i);
        if fo.is_empty() {
            return format!("self.exec_{n}(ops)");
        }
        let mut sel = String::from("0u32");
        for (b, k) in fo.iter().enumerate() {
            let op = &i.operands[*k as usize];
            let kind = self.kinds[&op.kind];
            let _ = write!(sel, " | ((self.kind_{}_form(ops.{}) as u32) << {b})", ident(&kind.name), ident(&op.name));
        }
        let mut arms = String::new();
        for v in 0..(1u32 << fo.len()) {
            let _ = write!(arms, "{v} => self.exec_{n}_f{v}(ops), ");
        }
        format!("match {sel} {{ {arms}_ => unreachable!() }}")
    }

    /// The form an operand access is specialised to, in a copy.
    pub(crate) fn form_of(&self, idx: u32) -> Option<u8> {
        self.form_ctx.borrow().iter().find(|(k, _)| *k == idx).map(|(_, f)| *f)
    }
}
