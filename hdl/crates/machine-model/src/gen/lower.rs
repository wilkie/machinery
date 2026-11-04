//! Lowering for the JIT core: the isa's semantics as code that builds a
//! compiled run's host code (`machine_jit::Em`), SPEC 12 "Cores".
//!
//! For each isa fn, operand kind rule and instruction this writes a Rust
//! fn of the machine that, given the emitter and what is known when a run
//! is compiled (an instruction's operands, the latches and pc its step was
//! decoded with), emits the host code for it. It follows the interpreter's
//! code (`Gen::expr`, `Gen::stmt`) rule for rule: values in native types,
//! wrapping arithmetic brought back to the declared width, Rust's shift and
//! cast semantics. The emitter folds whatever is known, so an operand's
//! choice of register, a latch's prefix, a constant condition cost the
//! compiled code nothing.
//!
//! Anything outside what it lowers (faults, loops, bundles, memories other
//! than through the bus, `repeats()`...) is an error here, and the
//! instruction (or fn) keeps calling its interpreter handler.

use std::collections::HashSet;
use std::fmt::Write as _;

use machine_sema::builtins::Builtin;
use machine_sema::hir::{self, ArmBody, Block, Expr, ExprKind as H, LetTarget, Local, LocalId, Pat, StmtKind as S};
use machine_sema::program::DefId;
use machine_sema::ty::Ty;
use machine_syntax::ast::{BinaryOp, UnaryOp};

use super::{ident, native, Gen, GenError, R};

/// What a lowering sees.
#[derive(Clone, Copy)]
pub(crate) struct Lx<'a> {
    pub locals: &'a [Local],
    /// The instruction whose operands are in scope (`ops`).
    pub ops: Option<&'a hir::Instruction>,
    /// In a kind's rule: the `from` operand's local (`src`).
    pub src: Option<LocalId>,
}

impl Gen<'_> {
    /// `machine_jit::N` of a type.
    pub(crate) fn jn(&self, ty: &Ty) -> R<&'static str> {
        Ok(match ty {
            Ty::Bool => "machine_jit::N::Bool",
            Ty::UInt(n) => nname(native(*n, false).0, false),
            Ty::SInt(n) => nname(native(*n, true).0, true),
            Ty::Enum(d) => nname(native(self.enums[d].width, false).0, false),
            Ty::Const(_) => "machine_jit::N::I128",
            _ => return Err(GenError::new(format!("lowering: a value of type {ty:?}"))),
        })
    }

    fn jwidth(&self, ty: &Ty) -> R<u32> {
        Ok(match ty {
            Ty::Bool => 8,
            Ty::UInt(n) | Ty::SInt(n) => native(*n, ty_signed(ty)).0,
            Ty::Enum(d) => native(self.enums[d].width, false).0,
            Ty::Const(_) => 128,
            _ => return Err(GenError::new("lowering: width")),
        })
    }

    /// `text` (a value of `ty`) brought to its declared width, as `fix`.
    fn jfix(&self, text: String, ty: &Ty) -> R<String> {
        Ok(match ty {
            Ty::UInt(n) if native(*n, false).0 != *n => {
                let n_ = self.jn(ty)?;
                format!("{{ let v = {text}; e.op(machine_jit::Op::And, v, machine_jit::V::K({:#x}), {n_}) }}", (1u128 << n) - 1)
            }
            Ty::SInt(n) if native(*n, true).0 != *n => {
                let n_ = self.jn(ty)?;
                let k = native(*n, true).0 - n;
                format!("{{ let v = {text}; let v = e.op(machine_jit::Op::Shl, v, machine_jit::V::K({k}), {n_}); e.op(machine_jit::Op::Shr, v, machine_jit::V::K({k}), {n_}) }}")
            }
            _ => text,
        })
    }

    /// A known value of `ty`: `v`'s bits in its native width.
    fn jconst(&self, v: i128, ty: &Ty) -> R<String> {
        let w = self.jwidth(ty)?;
        let bits = if w >= 128 { v as u128 } else { (v as u128) & ((1u128 << w) - 1) };
        Ok(format!("machine_jit::V::K({bits:#x})"))
    }

    /// The place of a register, its field, a latch or the mode, with the
    /// epochs a write to it bumps.
    fn jplace_reg(&self, e: &Expr) -> R<String> {
        let bump = self.jbump(e);
        Ok(match &e.kind {
            H::Register(d) => {
                let (_, r) = self.regs[d];
                let Ty::UInt(n) = &r.ty else { return Err(GenError::new("lowering: a register that is not an integer")) };
                format!("machine_jit::Place::Mem {{ off: std::mem::offset_of!(Self, r.{}), n: {}, bump: {bump} }}", ident(&r.name), nname(native(*n, false).0, false))
            }
            H::RegField(d, i) => {
                let (_, r) = self.regs[d];
                let Ty::UInt(rn) = &r.ty else { return Err(GenError::new("lowering: a field of a register that is not an integer")) };
                let f = &r.fields[*i as usize];
                let width = match &f.ty {
                    Ty::UInt(n) | Ty::SInt(n) => *n,
                    Ty::Bool => 1,
                    Ty::Enum(e) => self.enums[e].width,
                    _ => return Err(GenError::new("lowering: a register field's type")),
                };
                format!(
                    "machine_jit::Place::Bits {{ off: std::mem::offset_of!(Self, r.{}), reg: {}, shift: {}, width: {width}, n: {}, bump: {bump} }}",
                    ident(&r.name),
                    nname(native(*rn, false).0, false),
                    f.offset,
                    self.jn(&f.ty)?
                )
            }
            H::Latch(d) => format!("machine_jit::Place::Mem {{ off: std::mem::offset_of!(Self, latch_{}), n: {}, bump: 0 }}", ident(&self.prog.item_name(*d)), self.jn(&e.ty)?),
            H::Mode(_) => format!("machine_jit::Place::Mem {{ off: std::mem::offset_of!(Self, mode), n: {}, bump: {bump} }}", self.jn(&e.ty)?),
            _ => return Err(GenError::new("lowering: not a register place")),
        })
    }

    /// The epochs a write to `place` bumps (1: the epoch, 2: the address
    /// epoch, 4: the key epoch), as `trace_bump` writes them.
    fn jbump(&self, place: &Expr) -> u8 {
        let text = self.trace_bump_text(place);
        (text.contains("tr_epoch =") as u8) | ((text.contains("tr_aepoch =") as u8) << 1) | ((text.contains("tr_kepoch =") as u8) << 2)
    }

    /// A place expression as code making a `machine_jit::Place`.
    pub(crate) fn jplace(&self, cx: &Lx, e: &Expr) -> R<String> {
        Ok(match &e.kind {
            H::Register(_) | H::RegField(..) | H::Latch(_) | H::Mode(_) => self.jplace_reg(e)?,
            H::Local(id) if Some(*id) != cx.src => format!("machine_jit::Place::Local(l{id})"),
            H::KindSelect(k, args) => {
                let kind = self.kinds[k];
                self.jrule_needed(kind, &[hir::RuleKind::Select])?;
                let mut s = String::from("{ ");
                let mut names = Vec::new();
                for (i, a) in args.iter().enumerate() {
                    let _ = write!(s, "let s{i} = {}; ", self.jexpr(cx, a)?);
                    names.push(format!("s{i}"));
                }
                let _ = write!(s, "Self::jk_{}_select(e, {}) }}", ident(&kind.name), names.join(", "));
                s
            }
            H::Match(scrut, arms) => {
                let n = self.jn(&scrut.ty)?;
                let mut s = format!("{{ let sc = {}; let arms = vec![", self.jexpr(cx, scrut)?);
                for arm in arms {
                    let p = match &arm.body {
                        ArmBody::Expr(x) => self.jplace(cx, x)?,
                        ArmBody::Block(b) => self.jplace_block(cx, b)?,
                    };
                    let _ = write!(s, "({}, {p}), ", self.jpats_of(&arm.patterns, &scrut.ty)?);
                }
                let _ = write!(s, "]; e.place_switch(sc, {n}, arms) }}");
                s
            }
            H::If(c, then_b, Some(else_e)) => format!(
                "{{ let c = {}; let arms = vec![(vec![machine_jit::Pat::Is(1)], {}), (vec![machine_jit::Pat::Any], {})]; e.place_switch(c, machine_jit::N::Bool, arms) }}",
                self.jexpr(cx, c)?,
                self.jplace_block(cx, then_b)?,
                self.jplace(cx, else_e)?
            ),
            H::Block(b) => self.jplace_block(cx, b)?,
            _ => return Err(GenError::new(format!("lowering: a place {:?}", std::mem::discriminant(&e.kind)))),
        })
    }

    fn jplace_block(&self, cx: &Lx, b: &Block) -> R<String> {
        match b.stmts.as_slice() {
            [st] => match &st.kind {
                S::Expr(e) => self.jplace(cx, e),
                _ => Err(GenError::new("lowering: a place block with statements")),
            },
            _ => Err(GenError::new("lowering: a place block with statements")),
        }
    }

    fn jpats(&self, pats: &[Pat]) -> R<String> {
        let mut parts = Vec::new();
        for p in pats {
            parts.push(match p {
                Pat::Wildcard => "machine_jit::Pat::Any".to_string(),
                Pat::Value(v) => format!("machine_jit::Pat::Is({v})"),
                Pat::Range(a, b) => format!("machine_jit::Pat::Range({a}, {b})"),
                Pat::Bool(b) => format!("machine_jit::Pat::Is({})", *b as i128),
                Pat::Variant(_) => return Err(GenError::new("lowering: an enum pattern without its type")),
            });
        }
        Ok(format!("vec![{}]", parts.join(", ")))
    }

    fn jpats_of(&self, pats: &[Pat], scrut_ty: &Ty) -> R<String> {
        if let Ty::Enum(d) = scrut_ty {
            let en = self.enums[d];
            let mut parts = Vec::new();
            for p in pats {
                parts.push(match p {
                    Pat::Variant(i) => format!("machine_jit::Pat::Is(({}::{}) as i128)", ident(&en.name), ident(&en.variants[*i as usize].0)),
                    Pat::Wildcard => "machine_jit::Pat::Any".to_string(),
                    Pat::Value(v) => format!("machine_jit::Pat::Is({v})"),
                    Pat::Range(a, b) => format!("machine_jit::Pat::Range({a}, {b})"),
                    Pat::Bool(b) => format!("machine_jit::Pat::Is({})", *b as i128),
                });
            }
            return Ok(format!("vec![{}]", parts.join(", ")));
        }
        self.jpats(pats)
    }

    /// An expression as code making its `machine_jit::V` (with `e` the
    /// emitter in scope).
    pub(crate) fn jexpr(&self, cx: &Lx, e: &Expr) -> R<String> {
        let n = || self.jn(&e.ty);
        Ok(match &e.kind {
            H::Int(v) => self.jconst(*v, &e.ty)?,
            H::Bool(b) => format!("machine_jit::V::K({})", *b as u8),
            H::Local(id) if Some(*id) != cx.src => format!("e.get(l{id})"),
            H::Variant(d, idx) => {
                let en = self.enums[d];
                format!("machine_jit::V::K(({}::{}) as u128)", ident(&en.name), ident(&en.variants[*idx as usize].0))
            }
            H::Unary(op, a) => {
                let at = self.jexpr(cx, a)?;
                match op {
                    UnaryOp::Not => format!("{{ let a = {at}; e.not(a, machine_jit::N::Bool) }}"),
                    UnaryOp::BitNot => self.jfix(format!("{{ let a = {at}; e.not(a, {}) }}", n()?), &e.ty)?,
                    UnaryOp::Neg => self.jfix(format!("{{ let a = {at}; e.neg(a, {}) }}", n()?), &e.ty)?,
                }
            }
            H::Binary(op, a, b) => self.jbinary(cx, *op, a, b, &e.ty)?,
            H::Ternary(c, a, b) => format!(
                "{{ let c = {}; e.if_val(c, {}, |e| {}, |e| {}) }}",
                self.jexpr(cx, c)?,
                n()?,
                self.jexpr(cx, a)?,
                self.jexpr(cx, b)?
            ),
            H::Cast(inner) => self.jcast(cx, inner, &e.ty)?,
            H::Index(base, idx) if matches!(base.ty, Ty::UInt(_) | Ty::SInt(_)) => {
                // `(base.checked_shr(idx as u32).unwrap_or(0) & 1) as u8`:
                // past the width 0 even for a signed base.
                let w = self.jwidth(&base.ty)?;
                let un = nname(w, false);
                format!(
                    "{{ let b = {}; let i = {}; let b = e.cast(b, {}, {un}); let s = e.op(machine_jit::Op::Shr, b, i, {un}); let s = e.op(machine_jit::Op::And, s, machine_jit::V::K(1), {un}); e.cast(s, {un}, machine_jit::N::U8) }}",
                    self.jexpr(cx, base)?,
                    self.jexpr(cx, idx)?,
                    self.jn(&base.ty)?
                )
            }
            H::Slice(base, hi, lo) => {
                let bn = self.jn(&base.ty)?;
                let width = hi - lo + 1;
                let mask = if width >= 128 { u128::MAX } else { (1u128 << width) - 1 };
                let w = self.jwidth(&base.ty)?;
                let mask = if w >= 128 { mask } else { mask & ((1u128 << w) - 1) };
                format!(
                    "{{ let b = {}; let s = e.op(machine_jit::Op::Shr, b, machine_jit::V::K({lo}), {bn}); let s = e.op(machine_jit::Op::And, s, machine_jit::V::K({mask:#x}), {bn}); e.cast(s, {bn}, {}) }}",
                    self.jexpr(cx, base)?,
                    n()?
                )
            }
            H::Builtin(b, args) => self.jbuiltin(cx, *b, args, &e.ty)?,
            H::Match(scrut, arms) => {
                let sn = self.jn(&scrut.ty)?;
                let mut tail = "machine_jit::V::K(0)".to_string();
                for arm in arms.iter().rev() {
                    let body = match &arm.body {
                        ArmBody::Expr(x) => self.jexpr(cx, x)?,
                        ArmBody::Block(b) => self.jblock_expr(cx, b)?,
                    };
                    tail = format!(
                        "{{ let t = e.test(sc, {sn}, &{}); e.if_val(t, {}, |e| {body}, |e| {tail}) }}",
                        self.jpats_of(&arm.patterns, &scrut.ty)?,
                        n()?
                    );
                }
                format!("{{ let sc = {}; {tail} }}", self.jexpr(cx, scrut)?)
            }
            H::If(c, then_b, else_e) => {
                let ct = self.jexpr(cx, c)?;
                match (else_e, &e.ty) {
                    (Some(el), Ty::Unit | Ty::Never) => format!(
                        "{{ let c = {ct}; e.if_(c, |e| {{ {}; }}, |e| {{ {}; }}); machine_jit::V::K(0) }}",
                        self.jblock_expr(cx, then_b)?,
                        self.jexpr(cx, el)?
                    ),
                    (Some(el), _) => format!(
                        "{{ let c = {ct}; e.if_val(c, {}, |e| {}, |e| {}) }}",
                        n()?,
                        self.jblock_expr(cx, then_b)?,
                        self.jexpr(cx, el)?
                    ),
                    (None, _) => format!("{{ let c = {ct}; e.if_(c, |e| {{ {}; }}, |_| {{}}); machine_jit::V::K(0) }}", self.jblock_expr(cx, then_b)?),
                }
            }
            H::Block(b) => self.jblock_expr(cx, b)?,
            H::Register(_) | H::RegField(..) | H::Latch(_) | H::Mode(_) => format!("e.read(&{})", self.jplace_reg(e)?),
            H::OperandRead(i) => {
                let instr = cx.ops.ok_or_else(|| GenError::new("lowering: an operand outside an instruction"))?;
                let b = &instr.operands[*i as usize];
                let kind = self.kinds[&b.kind];
                let k = ident(&kind.name);
                let op = format!("ops.{}", ident(&b.name));
                self.jrule_needed(kind, &[hir::RuleKind::Read, hir::RuleKind::Value, hir::RuleKind::Select])?;
                if kind.source.is_some() {
                    format!("Self::jk_{k}_read(e, &{op})")
                } else if kind.rules.iter().any(|r| r.kind == hir::RuleKind::Value) {
                    format!("{{ let a = machine_jit::k({op}); Self::jk_{k}_value(e, a) }}")
                } else {
                    format!("{{ let a = machine_jit::k({op}); let p = Self::jk_{k}_select(e, a); e.read(&p) }}")
                }
            }
            H::OperandRule(i, rule) => {
                let instr = cx.ops.ok_or_else(|| GenError::new("lowering: an operand outside an instruction"))?;
                let b = &instr.operands[*i as usize];
                let kind = self.kinds[&b.kind];
                let r = &kind.rules[*rule as usize];
                if !self.jit_rules.borrow().contains(&(kind.def, *rule)) {
                    return Err(GenError::new("lowering: a kind rule that does not lower"));
                }
                let op = format!("ops.{}", ident(&b.name));
                if kind.source.is_some() {
                    format!("Self::jk_{}_{}(e, &{op})", ident(&kind.name), ident(&r.name))
                } else {
                    format!("{{ let a = machine_jit::k({op}); Self::jk_{}_{}(e, a) }}", ident(&kind.name), ident(&r.name))
                }
            }
            H::OperandField(base, i) => {
                let Ty::Operand(d) = &base.ty else { unreachable!() };
                let o = self.operands[d];
                match &base.kind {
                    H::Local(id) if Some(*id) == cx.src => {
                        format!("machine_jit::k(src.{})", ident(&o.fields[*i as usize].name))
                    }
                    _ => return Err(GenError::new("lowering: a field of an operand other than the rule's")),
                }
            }
            H::SpaceRead(d, bits, addr) => self.jspace_read(cx, *d, *bits, addr, &e.ty)?,
            H::SpaceCell(d, addr) if self.isa.split_cycles.is_none() => {
                let sp = self.spaces[d];
                self.jspace_read(cx, *d, sp.cell_bits, addr, &e.ty)?
            }
            H::SpaceWrite(d, bits, addr, v) => format!("{{ {} machine_jit::V::K(0) }}", self.jspace_write(cx, *d, *bits, addr, v)?),
            H::KindSelect(..) => format!("{{ let p = {}; e.read(&p) }}", self.jplace(cx, e)?),
            H::FnCall(d, args) => {
                let f = self.fns.get(d).ok_or_else(|| GenError::new("lowering: a fn of another isa"))?;
                if f.external || !self.jit_fns.borrow().contains(d) {
                    return Err(GenError::new(format!("lowering: fn `{}` is not lowered", f.name)));
                }
                let mut s = String::from("{ ");
                let mut names = Vec::new();
                for (i, a) in args.iter().enumerate() {
                    let _ = write!(s, "let a{i} = {}; ", self.jexpr(cx, a)?);
                    names.push(format!(", a{i}"));
                }
                let _ = write!(s, "Self::jf_{}(e{}) }}", ident(&f.name), names.concat());
                s
            }
            _ => return Err(GenError::new(format!("lowering: an expression {:?}", std::mem::discriminant(&e.kind)))),
        })
    }

    fn jbinary(&self, cx: &Lx, op: BinaryOp, a: &Expr, b: &Expr, ty: &Ty) -> R<String> {
        use BinaryOp::*;
        let at = self.jexpr(cx, a)?;
        let bt = self.jexpr(cx, b)?;
        // Short-circuit: the right side only when it decides.
        match op {
            And => return Ok(format!("{{ let a = {at}; e.if_val(a, machine_jit::N::Bool, |e| {bt}, |_| machine_jit::V::K(0)) }}")),
            Or => return Ok(format!("{{ let a = {at}; e.if_val(a, machine_jit::N::Bool, |_| machine_jit::V::K(1), |e| {bt}) }}")),
            _ => {}
        }
        let (o, n, fix) = match op {
            Add => ("Add", self.jn(ty)?, true),
            Sub => ("Sub", self.jn(ty)?, true),
            Mul => ("Mul", self.jn(ty)?, true),
            BitAnd => ("And", self.jn(ty)?, false),
            BitOr => ("Or", self.jn(ty)?, false),
            BitXor => ("Xor", self.jn(ty)?, false),
            Shl => ("Shl", self.jn(ty)?, true),
            Shr => ("Shr", self.jn(ty)?, false),
            Eq => ("Eq", self.jn(&a.ty)?, false),
            Ne => ("Ne", self.jn(&a.ty)?, false),
            Lt => ("Lt", self.jn(&a.ty)?, false),
            Le => ("Le", self.jn(&a.ty)?, false),
            Gt => ("Gt", self.jn(&a.ty)?, false),
            Ge => ("Ge", self.jn(&a.ty)?, false),
            _ => return Err(GenError::new("lowering: division")),
        };
        let text = format!("{{ let a = {at}; let b = {bt}; e.op(machine_jit::Op::{o}, a, b, {n}) }}");
        if fix {
            self.jfix(text, ty)
        } else {
            Ok(text)
        }
    }

    fn jcast(&self, cx: &Lx, inner: &Expr, target: &Ty) -> R<String> {
        let it = self.jexpr(cx, inner)?;
        Ok(match (&inner.ty, target) {
            (Ty::Bool, Ty::UInt(_)) => format!("{{ let a = {it}; e.cast(a, machine_jit::N::Bool, {}) }}", self.jn(target)?),
            (Ty::UInt(_), Ty::Bool) => format!("{{ let a = {it}; e.op(machine_jit::Op::Ne, a, machine_jit::V::K(0), {}) }}", self.jn(&inner.ty)?),
            (_, Ty::UInt(_) | Ty::SInt(_) | Ty::Enum(_)) => {
                let from = self.jn(&inner.ty)?;
                self.jfix(format!("{{ let a = {it}; e.cast(a, {from}, {}) }}", self.jn(target)?), target)?
            }
            (a, b) if a == b => it,
            _ => return Err(GenError::new("lowering: a cast")),
        })
    }

    fn jbuiltin(&self, cx: &Lx, b: Builtin, args: &[Expr], ty: &Ty) -> R<String> {
        let mut pre = String::new();
        for (i, a) in args.iter().enumerate() {
            let _ = write!(pre, "let x{i} = {}; ", self.jexpr(cx, a)?);
        }
        let an = |i: usize| self.jn(&args[i].ty);
        Ok(match b {
            Builtin::Parity => format!("{{ {pre}let c = e.count(machine_jit::Count::Ones, x0, {}); let c = e.op(machine_jit::Op::And, c, machine_jit::V::K(1), machine_jit::N::U32); e.cast(c, machine_jit::N::U32, machine_jit::N::U8) }}", an(0)?),
            Builtin::Popcount => format!("{{ {pre}let c = e.count(machine_jit::Count::Ones, x0, {}); e.cast(c, machine_jit::N::U32, {}) }}", an(0)?, self.jn(ty)?),
            Builtin::Clz => {
                let Ty::UInt(n) = &args[0].ty else { return Err(GenError::new("lowering: clz of a signed value")) };
                let w = native(*n, false).0;
                format!("{{ {pre}let c = e.count(machine_jit::Count::Leading, x0, {}); let c = e.op(machine_jit::Op::Sub, c, machine_jit::V::K({}), machine_jit::N::U32); e.cast(c, machine_jit::N::U32, {}) }}", an(0)?, w - n, self.jn(ty)?)
            }
            Builtin::Ctz => {
                let (Ty::UInt(n) | Ty::SInt(n)) = &args[0].ty else { return Err(GenError::new("lowering: ctz")) };
                format!("{{ {pre}let c = e.count(machine_jit::Count::Trailing, x0, {}); let lt = e.op(machine_jit::Op::Lt, c, machine_jit::V::K({n}), machine_jit::N::U32); let c = e.select(lt, c, machine_jit::V::K({n}), machine_jit::N::U32); e.cast(c, machine_jit::N::U32, {}) }}", an(0)?, self.jn(ty)?)
            }
            Builtin::Concat => {
                let t = self.jn(ty)?;
                let mut s = format!("{{ {pre}let acc = machine_jit::V::K(0); ");
                let mut shift = 0u32;
                for (i, xt) in args.iter().enumerate().rev() {
                    let (Ty::UInt(n) | Ty::SInt(n)) = &xt.ty else {
                        return Err(GenError::new("lowering: concat of a non-integer"));
                    };
                    let _ = write!(s, "let p = e.cast(x{i}, {}, {t}); let p = e.op(machine_jit::Op::Shl, p, machine_jit::V::K({shift}), {t}); let acc = e.op(machine_jit::Op::Or, acc, p, {t}); ", self.jn(&xt.ty)?);
                    shift += n;
                }
                s.push_str("acc }");
                s
            }
            Builtin::Restarted => "e.read(&machine_jit::Place::Mem { off: std::mem::offset_of!(Self, restarted), n: machine_jit::N::Bool, bump: 0 })".to_string(),
            _ => return Err(GenError::new(format!("lowering: builtin {}", b.name()))),
        })
    }

    /// The first of `kinds` that `kind` has must have lowered.
    fn jrule_needed(&self, kind: &hir::OperandKind, kinds: &[hir::RuleKind]) -> R<()> {
        for want in kinds {
            if let Some(ri) = kind.rules.iter().position(|r| r.kind == *want) {
                return if self.jit_rules.borrow().contains(&(kind.def, ri as u32)) {
                    Ok(())
                } else {
                    Err(GenError::new(format!("lowering: kind `{}`'s rule does not lower", kind.name)))
                };
            }
        }
        Err(GenError::new(format!("lowering: kind `{}` has no such rule", kind.name)))
    }

    fn jbits(bits: u32) -> R<u32> {
        match bits {
            8 | 16 | 32 | 64 => Ok(bits),
            _ => Err(GenError::new("lowering: an access of an odd width")),
        }
    }

    fn jspace_read(&self, cx: &Lx, d: DefId, bits: u32, addr: &Expr, ty: &Ty) -> R<String> {
        if bits > 64 {
            return Err(GenError::new("lowering: an access wider than 64 bits"));
        }
        let sp = ident(&self.spaces[&d].name);
        let pure = self.isa.split_cycles.is_none();
        let bits = Self::jbits(bits)?;
        Ok(format!(
            "{{ let a = {}; let a = e.cast(a, {}, machine_jit::N::U64); e.call(Self::jit_sp_{sp}_read{bits} as *const () as usize, &[(a, machine_jit::N::U64)], {}, {pure}) }}",
            self.jexpr(cx, addr)?,
            self.jn(&addr.ty)?,
            self.jn(ty)?
        ))
    }

    fn jspace_write(&self, cx: &Lx, d: DefId, bits: u32, addr: &Expr, v: &Expr) -> R<String> {
        if bits > 64 {
            return Err(GenError::new("lowering: an access wider than 64 bits"));
        }
        let sp = ident(&self.spaces[&d].name);
        let pure = self.isa.split_cycles.is_none();
        let bits = Self::jbits(bits)?;
        Ok(format!(
            "let a = {}; let a = e.cast(a, {}, machine_jit::N::U64); let v = {}; let v = e.cast(v, {}, machine_jit::N::U64); let _ = e.call(Self::jit_sp_{sp}_write{bits} as *const () as usize, &[(a, machine_jit::N::U64), (v, machine_jit::N::U64)], machine_jit::N::U64, {pure});",
            self.jexpr(cx, addr)?,
            self.jn(&addr.ty)?,
            self.jexpr(cx, v)?,
            self.jn(&v.ty)?
        ))
    }

    /// A block as a value: its statements, then its tail (scoped: its
    /// locals are released after).
    fn jblock_expr(&self, cx: &Lx, b: &Block) -> R<String> {
        let mut s = String::from("{ let mark = e.mark(); ");
        let n = b.stmts.len();
        let mut tail = "machine_jit::V::K(0)".to_string();
        for (i, st) in b.stmts.iter().enumerate() {
            if i + 1 == n {
                if let S::Expr(x) = &st.kind {
                    tail = self.jexpr(cx, x)?;
                    continue;
                }
            }
            s.push_str(&self.jstmt(cx, st)?);
        }
        let _ = write!(s, "let v = {tail}; e.release(mark); v }}");
        Ok(s)
    }

    pub(crate) fn jblock(&self, cx: &Lx, b: &Block) -> R<String> {
        let mut s = String::from("{ let mark = e.mark(); ");
        for st in &b.stmts {
            s.push_str(&self.jstmt(cx, st)?);
        }
        s.push_str("e.release(mark); }\n");
        Ok(s)
    }

    fn jstmt(&self, cx: &Lx, st: &hir::Stmt) -> R<String> {
        Ok(match &st.kind {
            S::Let(LetTarget::Local(id), Some(x)) => {
                format!("let l{id} = {{ let v = {}; e.local({}, v) }};\n", self.jexpr(cx, x)?, self.jn(&cx.locals[*id as usize].ty)?)
            }
            S::Let(LetTarget::Discard, Some(x)) => format!("let _ = {};\n", self.jexpr(cx, x)?),
            S::Let(LetTarget::Local(id), None) => format!("let l{id} = e.local({}, machine_jit::V::K(0));\n", self.jn(&cx.locals[*id as usize].ty)?),
            S::Let(..) => return Err(GenError::new("lowering: a tuple let")),
            S::Expr(x) => format!("let _ = {};\n", self.jexpr(cx, x)?),
            S::If(c, then_b, else_s) => {
                let els = match else_s {
                    None => String::new(),
                    Some(el) => match &el.kind {
                        S::Expr(Expr { kind: H::Block(b), .. }) => self.jblock(cx, b)?,
                        _ => self.jstmt(cx, el)?,
                    },
                };
                format!("{{ let c = {}; e.if_(c, |e| {}, |e| {{ {els} }}); }}\n", self.jexpr(cx, c)?, self.jblock(cx, then_b)?)
            }
            S::Match(scrut, arms) => {
                let sn = self.jn(&scrut.ty)?;
                let mut tail = String::new();
                for arm in arms.iter().rev() {
                    let body = match &arm.body {
                        ArmBody::Block(b) => self.jblock(cx, b)?,
                        ArmBody::Expr(x) => format!("{{ let _ = {}; }}", self.jexpr(cx, x)?),
                    };
                    tail = format!("{{ let t = e.test(sc, {sn}, &{}); e.if_(t, |e| {body}, |e| {{ {tail} }}); }}", self.jpats_of(&arm.patterns, &scrut.ty)?);
                }
                format!("{{ let sc = {}; {tail} }}\n", self.jexpr(cx, scrut)?)
            }
            S::For(id, lo, hi, body) => {
                let n = self.jn(&cx.locals[*id as usize].ty)?;
                let w = self.jwidth(&cx.locals[*id as usize].ty)?;
                let mask = if w >= 128 { u128::MAX } else { (1u128 << w) - 1 };
                format!("for i in ({lo}i128)..({hi}i128) {{ let l{id} = e.local({n}, machine_jit::V::K((i as u128) & {mask:#x})); {} }}\n", self.jblock(cx, body)?)
            }
            S::Return(Some(x)) => format!("{{ let v = {}; e.set(ret, v); e.ret(); }}\n", self.jexpr(cx, x)?),
            S::Return(None) => "e.ret();\n".to_string(),
            S::Assign(place, v) => format!("{{ let v = {}; let p = {}; e.write(&p, v); }}\n", self.jexpr(cx, v)?, self.jplace(cx, place)?),
            S::OperandWrite(i, v) => {
                let instr = cx.ops.ok_or_else(|| GenError::new("lowering: an operand outside an instruction"))?;
                let b = &instr.operands[*i as usize];
                let kind = self.kinds[&b.kind];
                let k = ident(&kind.name);
                let op = format!("ops.{}", ident(&b.name));
                self.jrule_needed(kind, &[hir::RuleKind::Write, hir::RuleKind::Select])?;
                let vt = self.jexpr(cx, v)?;
                if kind.source.is_some() {
                    format!("{{ let v = {vt}; Self::jk_{k}_write(e, &{op}, v); }}\n")
                } else {
                    format!("{{ let v = {vt}; let a = machine_jit::k({op}); let p = Self::jk_{k}_select(e, a); e.write(&p, v); }}\n")
                }
            }
            S::SpaceWrite(d, bits, addr, v) => format!("{{ {} }}\n", self.jspace_write(cx, *d, *bits, addr, v)?),
            S::Halt => "e.store(std::mem::offset_of!(Self, halted), machine_jit::N::Bool, machine_jit::V::K(1));\n".to_string(),
            _ => return Err(GenError::new(format!("lowering: a statement {:?}", std::mem::discriminant(&st.kind)))),
        })
    }

    /// The isa's fns that lower, each as `jf_<name>(e, args...) -> V`
    /// (inlined where called): the fns whose bodies lower, given the others
    /// that do.
    pub(crate) fn jit_lower_fns(&self) -> String {
        let mut ok: HashSet<DefId> = self.fns.values().filter(|f| !f.external && !self.jit_recursive(f.def)).map(|f| f.def).collect();
        loop {
            *self.jit_fns.borrow_mut() = ok.clone();
            let before = ok.len();
            ok.retain(|d| self.jfn(self.fns[d]).is_ok());
            if ok.len() == before {
                break;
            }
        }
        *self.jit_fns.borrow_mut() = ok.clone();
        let mut fns: Vec<&&hir::FnBody> = self.fns.values().filter(|f| ok.contains(&f.def)).collect();
        fns.sort_by(|a, b| a.name.cmp(&b.name));
        let mut s = String::new();
        for f in fns {
            if let Ok(t) = self.jfn(f) {
                s.push_str(&t);
            }
        }
        s
    }

    fn jfn(&self, f: &hir::FnBody) -> R<String> {
        let cx = Lx { locals: &f.locals, ops: None, src: None };
        let mut params = String::new();
        let mut bind = String::new();
        for (i, p) in f.params.iter().enumerate() {
            let _ = write!(params, ", a{i}: machine_jit::V");
            let _ = write!(bind, "let l{p} = e.local({}, a{i}); ", self.jn(&f.locals[*p as usize].ty)?);
        }
        let rn = match &f.ret {
            Ty::Unit => "machine_jit::N::U8",
            t => self.jn(t)?,
        };
        let body = self.jblock(&cx, &f.body)?;
        Ok(format!(
            "    /// Lowered `fn {}`.\n    #[allow(unused_variables, unused_mut, clippy::all)]\n    fn jf_{}(e: &mut machine_jit::Em{params}) -> machine_jit::V {{\n        let ret = e.local({rn}, machine_jit::V::K(0));\n        e.region_begin();\n        {bind}{body}        e.region_end();\n        e.get(ret)\n    }}\n",
            f.name,
            ident(&f.name)
        ))
    }

    /// Whether an instruction's semantics (or a fn they call) reach a bus
    /// space: then time must be kept in order around them.
    pub(crate) fn jtouches_bus(&self, i: &hir::Instruction) -> bool {
        let Some(b) = &i.semantics else { return false };
        let mut seen: HashSet<DefId> = HashSet::new();
        let mut todo: Vec<&Block> = vec![b];
        let mut found = false;
        while let Some(b) = todo.pop() {
            let mut calls = Vec::new();
            hir::visit_block(b, &mut |_| {}, &mut |x| match &x.kind {
                H::SpaceRead(..) | H::SpaceWrite(..) | H::SpaceCell(..) => found = true,
                H::FnCall(f, _) => calls.push(*f),
                _ => {}
            });
            hir::visit_block(b, &mut |st| found |= matches!(st.kind, S::SpaceWrite(..)), &mut |_| {});
            for f in calls {
                if seen.insert(f) {
                    if let Some(fb) = self.fns.get(&f) {
                        todo.push(&fb.body);
                    }
                }
            }
        }
        // A kind rule may reach memory too (a ModRM operand's read).
        for b in &i.operands {
            let kind = self.kinds[&b.kind];
            for r in &kind.rules {
                let mut hit = false;
                let mut on = |x: &Expr| hit |= matches!(x.kind, H::SpaceRead(..) | H::SpaceWrite(..) | H::SpaceCell(..) | H::FnCall(..));
                match &r.body {
                    ArmBody::Expr(x) => hir::visit_expr(x, &mut |_| {}, &mut on),
                    ArmBody::Block(b) => hir::visit_block(b, &mut |_| {}, &mut on),
                }
                found |= hit;
            }
        }
        found
    }

    /// Whether `d` can reach a call of itself (it would be inlined forever).
    fn jit_recursive(&self, d: DefId) -> bool {
        let mut seen = HashSet::new();
        let mut todo = vec![d];
        let mut first = true;
        while let Some(f) = todo.pop() {
            if f == d && !first {
                return true;
            }
            first = false;
            if !seen.insert(f) && f != d {
                continue;
            }
            let Some(body) = self.fns.get(&f) else { continue };
            let mut out = Vec::new();
            hir::visit_block(&body.body, &mut |_| {}, &mut |x| {
                if let H::FnCall(f, _) = &x.kind {
                    out.push(*f);
                }
            });
            for c in out {
                if c == d {
                    return true;
                }
                if !seen.contains(&c) {
                    todo.push(c);
                }
            }
        }
        false
    }

    /// The kinds' rules, each as `jk_<kind>_<rule>`: a select's place, a
    /// value, a `from` kind's read, write and queries (over `src`, the
    /// fetched operand).
    pub(crate) fn jit_lower_kinds(&self) -> String {
        let mut s = String::new();
        let mut kinds: Vec<&&hir::OperandKind> = self.kinds.values().collect();
        kinds.sort_by(|a, b| a.name.cmp(&b.name));
        let mut ok = HashSet::new();
        for kind in kinds {
            for (ri, r) in kind.rules.iter().enumerate() {
                match self.jrule(kind, r) {
                    Ok(t) => {
                        s.push_str(&t);
                        ok.insert((kind.def, ri as u32));
                    }
                    Err(e) if std::env::var_os("MACHINE_JIT_LOWERING").is_some() => eprintln!("{}.{}: {:?}", kind.name, r.name, e),
                    Err(_) => {}
                }
            }
        }
        *self.jit_rules.borrow_mut() = ok;
        s
    }

    fn jrule(&self, kind: &hir::OperandKind, r: &hir::KindRule) -> R<String> {
        let src = kind.source.and(r.params.first().copied());
        let cx = Lx { locals: &r.locals, ops: None, src };
        let k = ident(&kind.name);
        let mut params = String::new();
        let mut bind = String::new();
        if let Some(d) = kind.source {
            let _ = write!(params, ", src: &{}", ident(&self.prog.item_name(d)));
        }
        for (i, p) in r.params.iter().enumerate() {
            if Some(*p) == src {
                continue;
            }
            let _ = write!(params, ", a{i}: machine_jit::V");
            let _ = write!(bind, "let l{p} = e.local({}, a{i}); ", self.jn(&r.locals[*p as usize].ty)?);
        }
        let attrs = "    #[allow(unused_variables, unused_mut, clippy::all)]\n";
        Ok(match r.kind {
            hir::RuleKind::Select => {
                let body = match &r.body {
                    ArmBody::Expr(x) => self.jplace(&cx, x)?,
                    ArmBody::Block(b) => self.jplace_block(&cx, b)?,
                };
                format!("{attrs}    fn jk_{k}_select(e: &mut machine_jit::Em{params}) -> machine_jit::Place {{\n        {bind}{body}\n    }}\n")
            }
            hir::RuleKind::Write => {
                // `write(v) => body`: the value is the rule's last parameter.
                let body = match &r.body {
                    ArmBody::Expr(x) => format!("let _ = {};", self.jexpr(&cx, x)?),
                    ArmBody::Block(b) => self.jblock(&cx, b)?,
                };
                format!("{attrs}    fn jk_{k}_write(e: &mut machine_jit::Em{params}) {{\n        {bind}{body}\n    }}\n")
            }
            _ => {
                let body = match &r.body {
                    ArmBody::Expr(x) => self.jexpr(&cx, x)?,
                    ArmBody::Block(b) => self.jblock_expr(&cx, b)?,
                };
                let name = match r.kind {
                    hir::RuleKind::Value => "value".to_string(),
                    hir::RuleKind::Read => "read".to_string(),
                    _ => ident(&r.name),
                };
                format!("{attrs}    fn jk_{k}_{name}(e: &mut machine_jit::Em{params}) -> machine_jit::V {{\n        {bind}{body}\n    }}\n")
            }
        })
    }

    /// An instruction's timing entry and semantics as `jt_<name>(e, ops)`
    /// and `jx_<name>(e, ops)`, when both lower.
    pub(crate) fn jinstr(&self, i: &hir::Instruction) -> R<String> {
        let n = ident(&i.name);
        let ops_ty = Self::ops_struct_name(i);
        let timing = match &i.timing {
            Some(t) => {
                let mut uses_repeats = false;
                hir::visit_expr(t, &mut |_| {}, &mut |x| uses_repeats |= matches!(&x.kind, H::Builtin(Builtin::Repeats, _)));
                if uses_repeats {
                    return Err(GenError::new("lowering: a timing entry after the instruction"));
                }
                let cx = Lx { locals: &i.timing_locals, ops: Some(i), src: None };
                let t = self.jexpr(&cx, t)?;
                format!("{{ let v = {t}; let _ = ops; v }}")
            }
            None => "machine_jit::V::K(0)".to_string(),
        };
        let cx = Lx { locals: &i.locals, ops: Some(i), src: None };
        let body = match &i.semantics {
            Some(b) => self.jblock(&cx, b)?,
            None => String::new(),
        };
        let attrs = "    #[allow(unused_variables, unused_mut, clippy::all)]\n";
        Ok(format!(
            "{attrs}    fn jt_{n}(e: &mut machine_jit::Em, ops: &{ops_ty}) -> machine_jit::V {{\n        {timing}\n    }}\n{attrs}    fn jx_{n}(e: &mut machine_jit::Em, ops: &{ops_ty}) {{\n        let ret = e.local(machine_jit::N::U8, machine_jit::V::K(0));\n        e.region_begin();\n        {body}        e.region_end();\n    }}\n"
        ))
    }
}

fn ty_signed(ty: &Ty) -> bool {
    matches!(ty, Ty::SInt(_))
}

fn nname(w: u32, signed: bool) -> &'static str {
    match (w, signed) {
        (8, false) => "machine_jit::N::U8",
        (16, false) => "machine_jit::N::U16",
        (32, false) => "machine_jit::N::U32",
        (64, false) => "machine_jit::N::U64",
        (_, false) => "machine_jit::N::U128",
        (8, true) => "machine_jit::N::I8",
        (16, true) => "machine_jit::N::I16",
        (32, true) => "machine_jit::N::I32",
        (64, true) => "machine_jit::N::I64",
        (_, true) => "machine_jit::N::I128",
    }
}
