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
    /// In a timing entry: a fault would make the entry 0 (the interpreter
    /// evaluates it so), so nothing that raises lowers.
    pub timing: bool,
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

    /// Whether values of `ty` are aggregates (records, tuples): lowered as
    /// their scalar leaves.
    pub(crate) fn jagg_ty(ty: &Ty) -> bool {
        matches!(ty, Ty::Bundle(_) | Ty::Tuple(_))
    }

    /// The scalar leaves of `ty`, in declaration order.
    pub(crate) fn jleaves(&self, ty: &Ty) -> R<Vec<Ty>> {
        Ok(match ty {
            Ty::Bundle(d) => {
                let mut out = Vec::new();
                for f in &self.bundles[d].fields {
                    out.extend(self.jleaves(&f.ty)?);
                }
                out
            }
            Ty::Tuple(items) => {
                let mut out = Vec::new();
                for (_, t) in items {
                    out.extend(self.jleaves(t)?);
                }
                out
            }
            Ty::Array(..) | Ty::Union(_) | Ty::Operand(_) | Ty::OperandTail(_) | Ty::Str => {
                return Err(GenError::new(format!("lowering: a value of type {ty:?}")))
            }
            t => vec![t.clone()],
        })
    }

    /// `[N, ...]` of `ty`'s leaves.
    fn jleaf_ns(&self, ty: &Ty) -> R<String> {
        let ns: R<Vec<&str>> = self.jleaves(ty)?.iter().map(|t| self.jn(t)).collect();
        Ok(format!("[{}]", ns?.join(", ")))
    }

    /// The leaves field `i` of record (or tuple) type `ty` covers.
    fn jfield_range(&self, ty: &Ty, i: u32) -> R<(usize, usize)> {
        let items: Vec<Ty> = match ty {
            Ty::Bundle(d) => self.bundles[d].fields.iter().map(|f| f.ty.clone()).collect(),
            Ty::Tuple(items) => items.iter().map(|(_, t)| t.clone()).collect(),
            _ => return Err(GenError::new("lowering: a field of a non-record")),
        };
        let mut at = 0;
        for (k, t) in items.iter().enumerate() {
            let n = self.jleaves(t)?.len();
            if k == i as usize {
                return Ok((at, n));
            }
            at += n;
        }
        Err(GenError::new("lowering: no such field"))
    }

    /// The places of a record register's leaves (`path` within `Regs`).
    fn jreg_leaf_places(&self, path: &str, ty: &Ty, bump: u8, out: &mut Vec<String>) -> R<()> {
        match ty {
            Ty::Bundle(d) => {
                for f in &self.bundles[d].fields {
                    self.jreg_leaf_places(&format!("{path}.{}", ident(&f.name)), &f.ty, bump, out)?;
                }
            }
            Ty::Tuple(_) | Ty::Array(..) => return Err(GenError::new("lowering: a register of this shape")),
            t => out.push(format!("machine_jit::Place::Mem {{ off: std::mem::offset_of!(Self, {path}), n: {}, bump: {bump} }}", self.jn(t)?)),
        }
        Ok(())
    }

    /// An aggregate place's leaves, as code making `Vec<Place>`.
    fn jagg_place(&self, cx: &Lx, e: &Expr) -> R<String> {
        Ok(match &e.kind {
            H::Register(d) => {
                let (_, r) = self.regs[d];
                let mut v = Vec::new();
                self.jreg_leaf_places(&format!("r.{}", ident(&r.name)), &r.ty, self.jbump(e), &mut v)?;
                format!("vec![{}]", v.join(", "))
            }
            H::RegView(d, i) => {
                let (_, r) = self.regs[d];
                let view = &r.views[*i as usize];
                let mut v = Vec::new();
                for ri in &view.mapping {
                    let fe = Expr { kind: H::RegField(*d, *ri), ty: r.fields[*ri as usize].ty.clone(), span: e.span };
                    v.push(self.jplace_reg(&fe)?);
                }
                format!("vec![{}]", v.join(", "))
            }
            H::Local(id) if Some(*id) != cx.src => format!("l{id}.iter().map(|s| machine_jit::Place::Local(*s)).collect::<Vec<_>>()"),
            H::Field(base, i) | H::TupleField(base, i) => {
                let (at, n) = self.jfield_range(&base.ty, *i)?;
                format!("{{ let p = {}; p[{at}..{}].to_vec() }}", self.jagg_place(cx, base)?, at + n)
            }
            H::Index(base, idx) if matches!(base.ty, Ty::Array(..)) => {
                // The index once, for every leaf.
                let H::Register(d) = &base.kind else { return Err(GenError::new("lowering: an index into a non-register array")) };
                let one = self.jelem_places_with(e, *d, "idx")?;
                format!("{{ let idx = {{ let i = {}; e.cast(i, {}, machine_jit::N::U64) }}; vec![{}] }}", self.jexpr(cx, idx)?, self.jn(&idx.ty)?, one.join(", "))
            }
            _ => return Err(GenError::new("lowering: an aggregate place")),
        })
    }

    /// The places of an element of register array `d`, one per leaf of the
    /// element, at the index in variable `idx` (a `V`, a u64).
    fn jelem_places_with(&self, whole: &Expr, d: DefId, idx: &str) -> R<Vec<String>> {
        let (_, r) = self.regs[&d];
        let Ty::Array(elem, count) = &r.ty else { return Err(GenError::new("lowering: an index into a non-array")) };
        let bump = self.jbump(whole);
        fn walk(g: &Gen, path: &str, ty: &Ty, v: &mut Vec<(String, Ty)>) {
            match ty {
                Ty::Bundle(b) => {
                    for f in &g.bundles[b].fields {
                        let p = if path.is_empty() { ident(&f.name) } else { format!("{path}.{}", ident(&f.name)) };
                        walk(g, &p, &f.ty, v);
                    }
                }
                t => v.push((path.to_string(), t.clone())),
            }
        }
        let mut offs = Vec::new();
        walk(self, "", elem, &mut offs);
        let elem_ty = match &**elem {
            Ty::Bundle(bd) => ident(&self.prog.item_name(*bd)),
            t => self.rty(t)?,
        };
        let mut out = Vec::new();
        for (path, t) in offs {
            if matches!(t, Ty::Array(..) | Ty::Tuple(_)) {
                return Err(GenError::new("lowering: an array element of this shape"));
            }
            let off = if path.is_empty() { "0".to_string() } else { format!("std::mem::offset_of!({elem_ty}, {path})") };
            out.push(format!(
                "machine_jit::Place::Elem {{ base: std::mem::offset_of!(Self, r.{}), stride: std::mem::size_of::<{elem_ty}>(), count: {count}, off: {off}, index: {idx}, n: {}, bump: {bump} }}",
                ident(&r.name),
                self.jn(&t)?
            ));
        }
        Ok(out)
    }

    /// Whether `e` names an aggregate place (read leaf by leaf).
    fn jis_agg_place(e: &Expr) -> bool {
        match &e.kind {
            H::Register(_) | H::RegView(..) | H::Local(_) => true,
            H::Field(b, _) | H::TupleField(b, _) => Self::jis_agg_place(b),
            H::Index(b, _) => matches!(b.kind, H::Register(_)) && matches!(b.ty, Ty::Array(..)),
            _ => false,
        }
    }

    /// An aggregate value, as code making its leaves (`Vec<V>`).
    pub(crate) fn jagg(&self, cx: &Lx, e: &Expr) -> R<String> {
        if !Self::jagg_ty(&e.ty) {
            return Ok(format!("vec![{}]", self.jexpr(cx, e)?));
        }
        let ns = self.jleaf_ns(&e.ty)?;
        Ok(match &e.kind {
            H::Local(id) if Some(*id) != cx.src => format!("l{id}.iter().map(|s| e.get(*s)).collect::<Vec<_>>()"),
            _ if Self::jis_agg_place(e) => format!("{{ let p = {}; p.iter().map(|p| e.read(p)).collect::<Vec<_>>() }}", self.jagg_place(cx, e)?),
            H::Field(base, i) | H::TupleField(base, i) => {
                let (at, n) = self.jfield_range(&base.ty, *i)?;
                format!("{{ let a = {}; a[{at}..{}].to_vec() }}", self.jagg(cx, base)?, at + n)
            }
            H::BundleLit(_, items) | H::Tuple(items) => {
                let mut s = String::from("{ let mut a: Vec<machine_jit::V> = Vec::new(); ");
                for x in items {
                    let _ = write!(s, "let x = {}; a.extend(x); ", self.jagg(cx, x)?);
                }
                s.push_str("a }");
                s
            }
            H::FnCall(..) | H::Call(..) => self.jcall(cx, e)?,
            H::OperandRead(i) => {
                let instr = cx.ops.ok_or_else(|| GenError::new("lowering: an operand outside an instruction"))?;
                let b = &instr.operands[*i as usize];
                let kind = self.kinds[&b.kind];
                self.jrule_needed(kind, &[hir::RuleKind::Read, hir::RuleKind::Value])?;
                let op = format!("ops.{}", ident(&b.name));
                if kind.source.is_some() {
                    format!("Self::jk_{}_read(e, &{op})", ident(&kind.name))
                } else {
                    format!("{{ let a = machine_jit::k({op}); Self::jk_{}_value(e, a) }}", ident(&kind.name))
                }
            }
            H::If(c, then_b, Some(else_e)) => format!(
                "{{ let ns = {ns}; let r: Vec<machine_jit::Slot> = ns.iter().map(|n| e.local(*n, machine_jit::V::K(0))).collect(); let c = {}; e.if_(c, |e| {{ let v = {}; for (s, x) in r.iter().zip(v) {{ e.set(*s, x); }} }}, |e| {{ let v = {}; for (s, x) in r.iter().zip(v) {{ e.set(*s, x); }} }}); r.iter().map(|s| e.get(*s)).collect::<Vec<_>>() }}",
                self.jexpr(cx, c)?,
                self.jagg_block(cx, then_b)?,
                self.jagg(cx, else_e)?
            ),
            H::Ternary(c, a, b) => format!(
                "{{ let ns = {ns}; let r: Vec<machine_jit::Slot> = ns.iter().map(|n| e.local(*n, machine_jit::V::K(0))).collect(); let c = {}; e.if_(c, |e| {{ let v = {}; for (s, x) in r.iter().zip(v) {{ e.set(*s, x); }} }}, |e| {{ let v = {}; for (s, x) in r.iter().zip(v) {{ e.set(*s, x); }} }}); r.iter().map(|s| e.get(*s)).collect::<Vec<_>>() }}",
                self.jexpr(cx, c)?,
                self.jagg(cx, a)?,
                self.jagg(cx, b)?
            ),
            H::Match(scrut, arms) => {
                let sn = self.jn(&scrut.ty)?;
                let mut tail = "{}".to_string();
                for arm in arms.iter().rev() {
                    let body = match &arm.body {
                        ArmBody::Expr(x) => self.jagg(cx, x)?,
                        ArmBody::Block(b) => self.jagg_block(cx, b)?,
                    };
                    tail = format!("{{ let t = e.test(sc, {sn}, &{}); e.if_(t, |e| {{ let v = {body}; for (s, x) in r.iter().zip(v) {{ e.set(*s, x); }} }}, |e| {tail}); }}", self.jpats_of(&arm.patterns, &scrut.ty)?);
                }
                format!("{{ let ns = {ns}; let r: Vec<machine_jit::Slot> = ns.iter().map(|n| e.local(*n, machine_jit::V::K(0))).collect(); let sc = {}; {tail} r.iter().map(|s| e.get(*s)).collect::<Vec<_>>() }}", self.jexpr(cx, scrut)?)
            }
            H::Block(b) => self.jagg_block(cx, b)?,
            _ => return Err(GenError::new(format!("lowering: an aggregate {:?}", std::mem::discriminant(&e.kind)))),
        })
    }

    fn jagg_block(&self, cx: &Lx, b: &Block) -> R<String> {
        let mut s = String::from("{ let mark = e.mark(); ");
        let n = b.stmts.len();
        let mut tail = None;
        for (i, st) in b.stmts.iter().enumerate() {
            if i + 1 == n {
                if let S::Expr(x) = &st.kind {
                    tail = Some(self.jagg(cx, x)?);
                    continue;
                }
            }
            s.push_str(&self.jstmt(cx, st)?);
        }
        let tail = tail.ok_or_else(|| GenError::new("lowering: an aggregate block without a value"))?;
        let _ = write!(s, "let v = {tail}; e.release(mark); v }}");
        Ok(s)
    }

    /// A value of an operand type (the fetched operand's struct, known when
    /// compiling), as code making it.
    fn joperand(&self, cx: &Lx, e: &Expr) -> R<String> {
        Ok(match &e.kind {
            H::Local(id) if Some(*id) == cx.src => "(*src)".to_string(),
            H::Local(id) => format!("l{id}"),
            H::OperandTail(base) => format!("({}).tail", self.joperand(cx, base)?),
            // The tail the decoder fetched for this operand. In a branch the
            // compiled code cannot take (the decoder chose the tail by the
            // same condition), another tail is a trap.
            H::TailOperand(base, d) => {
                let Ty::OperandTail(owner) = &base.ty else { return Err(GenError::new("lowering: a tail of a non-tail")) };
                let name = self.prog.item_name(*d);
                format!(
                    "(match ({}) {{ {}_Tail::{name}(v) => v, _ => {{ e.trap(); {}::from_bits(0) }} }})",
                    self.joperand(cx, base)?,
                    ident(&self.prog.item_name(*owner)),
                    ident(&name)
                )
            }
            _ => return Err(GenError::new("lowering: an operand value")),
        })
    }

    /// A call of an isa fn or a unit, whatever its type (`V`, `Vec<V>`, or
    /// nothing for a unit fn).
    fn jcall(&self, cx: &Lx, e: &Expr) -> R<String> {
        let (name, params, args): (String, Vec<Ty>, &Vec<Expr>) = match &e.kind {
            H::FnCall(d, args) if self.jit_outlined.borrow().contains(d) => {
                let f = self.fns[d];
                if cx.timing && self.jit_raisers.borrow().contains(d) {
                    return Err(GenError::new("lowering: a timing entry that may raise"));
                }
                // The arguments once; then the fn inlined if what is known
                // folds it small (`try_inline`), else called.
                let mut s = String::from("{ ");
                let mut inline_args = Vec::new();
                let mut flat = String::from("let mut av: Vec<(machine_jit::V, machine_jit::N)> = Vec::new(); ");
                for (i, (a, p)) in args.iter().zip(&f.params).enumerate() {
                    let t = &f.locals[*p as usize].ty;
                    if Self::jagg_ty(t) {
                        let _ = write!(s, "let a{i} = {}; ", self.jagg(cx, a)?);
                        inline_args.push(format!(", a{i}.clone()"));
                        let _ = write!(flat, "for (x, n) in a{i}.iter().copied().zip({}) {{ let x = e.cast(x, n, machine_jit::N::U64); av.push((x, machine_jit::N::U64)); }} ", self.jleaf_ns(t)?);
                    } else {
                        let _ = write!(s, "let a{i} = {}; ", self.jexpr(cx, a)?);
                        inline_args.push(format!(", a{i}"));
                        let _ = write!(flat, "let x = e.cast(a{i}, {}, machine_jit::N::U64); av.push((x, machine_jit::N::U64)); ", self.jn(t)?);
                    }
                }
                let rn = match &f.ret {
                    Ty::Unit | Ty::Never => "machine_jit::N::U8",
                    t => self.jn(t)?,
                };
                // A fault the fn raised: delivered once the code has stored
                // what it owes, and the run ends.
                let pending = if self.jit_raisers.borrow().contains(d) {
                    "let p = e.load(std::mem::offset_of!(Self, jit_pending), machine_jit::N::Bool); e.exit_if(p, true, |e| { let _ = e.call(Self::jit_deliver_c as *const () as usize, &[], machine_jit::N::U64, false); }); "
                } else {
                    ""
                };
                // The registers the fn leaves alone stay in host values.
                let invoke = match self.jit_call_ranges(*d) {
                    Some((kept, read)) => format!("e.call_fx(Self::jit_call_{} as *const () as usize, &av, {rn}, &[{kept}], &[{read}])", ident(&f.name)),
                    None => format!("e.call(Self::jit_call_{} as *const () as usize, &av, {rn}, false)", ident(&f.name)),
                };
                let call = format!("{flat}let r = {invoke}; {pending}r");
                if self.jit_fns.borrow().contains(d) {
                    let _ = write!(s, "e.try_inline(Self::jit_trial_budget(), |e| Self::jf_{}(e{}), |e| {{ {call} }}) }}", ident(&f.name), inline_args.concat());
                } else {
                    let _ = write!(s, "{call} }}");
                }
                return Ok(s);
            }
            // An external fn: the wire level the board keeps (a load), else
            // a call of the bus's handler.
            H::FnCall(d, args) if self.fns.get(d).is_some_and(|f| f.external) => {
                let f = self.fns[d];
                if Self::jagg_ty(&f.ret) || cx.timing {
                    return Err(GenError::new("lowering: an external fn here"));
                }
                let rn = match &f.ret {
                    Ty::Unit | Ty::Never => "machine_jit::N::U8",
                    t => self.jn(t)?,
                };
                let mut s = String::from("{ let mut av: Vec<(machine_jit::V, machine_jit::N)> = Vec::new(); ");
                for (a, p) in args.iter().zip(&f.params) {
                    let t = &f.locals[*p as usize].ty;
                    if Self::jagg_ty(t) {
                        return Err(GenError::new("lowering: an external fn's record argument"));
                    }
                    let _ = write!(s, "let x = {}; let x = e.cast(x, {}, machine_jit::N::U64); av.push((x, machine_jit::N::U64)); ", self.jexpr(cx, a)?, self.jn(t)?);
                }
                let _ = write!(
                    s,
                    "match B::jit_ext({:?}) {{ Some(Ok(off)) if av.is_empty() => e.load(std::mem::offset_of!(Self, bus) + off, {rn}), Some(Err(v)) if av.is_empty() => machine_jit::V::K(v), _ => e.call(Self::jit_ext_{} as *const () as usize, &av, {rn}, true) }} }}",
                    f.name,
                    ident(&f.name)
                );
                return Ok(s);
            }
            H::FnCall(d, args) => {
                let f = self.fns.get(d).ok_or_else(|| GenError::new("lowering: a fn of another isa"))?;
                if f.external || !self.jit_fns.borrow().contains(d) {
                    return Err(GenError::new(format!("lowering: fn `{}` is not lowered", f.name)));
                }
                if cx.timing && self.jit_raisers.borrow().contains(d) {
                    return Err(GenError::new("lowering: a timing entry that may raise"));
                }
                (format!("jf_{}", ident(&f.name)), f.params.iter().map(|p| f.locals[*p as usize].ty.clone()).collect(), args)
            }
            H::Call(inst, args) => {
                let u = &self.hir.units[inst.0 as usize];
                if !self.jit_units.borrow().contains(&inst.0) {
                    return Err(GenError::new("lowering: a unit that is not lowered"));
                }
                (format!("j{}", self.unit_fn_name(*inst)), u.params.iter().map(|p| u.locals[*p as usize].ty.clone()).collect(), args)
            }
            _ => return Err(GenError::new("lowering: not a call")),
        };
        let mut s = String::from("{ ");
        let mut names = Vec::new();
        for (i, (a, t)) in args.iter().zip(&params).enumerate() {
            let v = if matches!(t, Ty::Operand(_)) {
                self.joperand(cx, a)?
            } else if Self::jagg_ty(t) {
                self.jagg(cx, a)?
            } else {
                self.jexpr(cx, a)?
            };
            let _ = write!(s, "let a{i} = {v}; ");
            names.push(format!(", a{i}"));
        }
        let _ = write!(s, "Self::{name}(e{}) }}", names.concat());
        Ok(s)
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
            H::Field(base, i) | H::TupleField(base, i) if Self::jagg_ty(&base.ty) && !Self::jagg_ty(&e.ty) => {
                let (at, _) = self.jfield_range(&base.ty, *i)?;
                format!("{{ let p = {}; p[{at}].clone() }}", self.jagg_place(cx, base)?)
            }
            H::Index(base, idx) if matches!(base.ty, Ty::Array(..)) && !Self::jagg_ty(&e.ty) => {
                let H::Register(d) = &base.kind else { return Err(GenError::new("lowering: an index into a non-register array")) };
                let p = self.jelem_places_with(e, *d, "idx")?;
                format!("{{ let idx = {{ let i = {}; e.cast(i, {}, machine_jit::N::U64) }}; {} }}", self.jexpr(cx, idx)?, self.jn(&idx.ty)?, p[0])
            }
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
            H::Index(base, _) if matches!(base.ty, Ty::Array(..)) => format!("{{ let p = {}; e.read(&p) }}", self.jplace(cx, e)?),
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
                format!("machine_jit::k(({}).{})", self.joperand(cx, base)?, ident(&o.fields[*i as usize].name))
            }
            H::SpaceRead(d, bits, addr) => self.jspace_read(cx, *d, *bits, addr, &e.ty)?,
            H::SpaceCell(d, addr) if self.isa.split_cycles.is_none() => {
                let sp = self.spaces[d];
                self.jspace_read(cx, *d, sp.cell_bits, addr, &e.ty)?
            }
            H::SpaceWrite(d, bits, addr, v) => format!("{{ {} machine_jit::V::K(0) }}", self.jspace_write(cx, *d, *bits, addr, v)?),
            H::KindSelect(..) => format!("{{ let p = {}; e.read(&p) }}", self.jplace(cx, e)?),
            H::FnCall(..) | H::Call(..) => self.jcall(cx, e)?,
            H::Field(base, i) | H::TupleField(base, i) if Self::jagg_ty(&base.ty) => {
                let (at, _) = self.jfield_range(&base.ty, *i)?;
                if Self::jis_agg_place(base) {
                    format!("{{ let p = {}; e.read(&p[{at}]) }}", self.jagg_place(cx, base)?)
                } else {
                    format!("{{ let a = {}; a[{at}] }}", self.jagg(cx, base)?)
                }
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
            Div => ("Div", self.jn(ty)?, false),
            Rem => ("Rem", self.jn(ty)?, false),
            _ => unreachable!(),
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
        match b {
            Builtin::Unreachable => return Ok("{ e.trap(); machine_jit::V::K(0) }".to_string()),
            Builtin::Assert => {
                return Ok(format!("{{ let c = {}; let bad = e.not(c, machine_jit::N::Bool); e.if_(bad, |e| e.trap(), |_| {{}}); machine_jit::V::K(0) }}", self.jexpr(cx, &args[0])?))
            }
            _ => {}
        }
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
            // After the instruction: the istream's address is where the
            // step started (`phys`, known when compiling).
            Builtin::Repeats => {
                let ist = self.isa.istream.as_ref().ok_or_else(|| GenError::new("lowering: repeats() without an istream"))?;
                let f = self.fns[&ist.address];
                if !self.jit_fns.borrow().contains(&ist.address) {
                    return Err(GenError::new("lowering: the istream's address fn does not lower"));
                }
                format!("{{ let a = Self::jf_{}(e); let a = e.cast(a, {}, machine_jit::N::U64); e.op(machine_jit::Op::Eq, a, machine_jit::V::K(phys as u128), machine_jit::N::U64) }}", ident(&f.name), self.jn(&f.ret)?)
            }
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

    /// Whether `d` is the istream's space: its accesses may be the board's
    /// direct memory (the split charge, if any, made with them).
    fn jdirect_space(&self, d: DefId) -> bool {
        self.isa.istream.as_ref().is_some_and(|i| i.space == d)
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
        // A byte of the istream space: in host code when the board's memory
        // is direct.
        if self.jdirect_space(d) {
            let nb = nname_bits(bits);
            return Ok(format!(
                "{{ let a = {}; let a = e.cast(a, {}, machine_jit::N::U64); let v = Self::jm_read(e, a, {}, Self::jit_sp_{sp}_read{bits} as *const () as usize, {pure}); e.cast(v, {nb}, {}) }}",
                self.jexpr(cx, addr)?,
                self.jn(&addr.ty)?,
                bits / 8,
                self.jn(ty)?
            ));
        }
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
        if self.jdirect_space(d) {
            let nb = nname_bits(bits);
            return Ok(format!(
                "let a = {}; let a = e.cast(a, {}, machine_jit::N::U64); let v = {}; let v = e.cast(v, {}, {nb}); Self::jm_write(e, a, v, {}, Self::jit_sp_{sp}_write{bits} as *const () as usize, {pure});",
                self.jexpr(cx, addr)?,
                self.jn(&addr.ty)?,
                self.jexpr(cx, v)?,
                self.jn(&v.ty)?,
                bits / 8
            ));
        }
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

    /// A tuple `let`'s targets from `tv` (the value's leaves), from leaf
    /// `at`.
    fn jlet_tuple(&self, t: &LetTarget, ty: &Ty, at: &mut usize, s: &mut String) -> R<()> {
        match t {
            LetTarget::Tuple(items) => {
                let Ty::Tuple(tys) = ty else { return Err(GenError::new("lowering: a tuple let of a non-tuple")) };
                for (it, (_, tt)) in items.iter().zip(tys) {
                    self.jlet_tuple(it, tt, at, s)?;
                }
            }
            LetTarget::Discard => *at += self.jleaves(ty)?.len(),
            LetTarget::Local(id) => {
                let n = self.jleaves(ty)?.len();
                if Self::jagg_ty(ty) {
                    let _ = writeln!(s, "let l{id}: Vec<machine_jit::Slot> = {}.iter().zip(tv[{}..{}].iter()).map(|(n, x)| e.local(*n, *x)).collect();", self.jleaf_ns(ty)?, *at, *at + n);
                } else {
                    let _ = writeln!(s, "let l{id} = e.local({}, tv[{}]);", self.jn(ty)?, *at);
                }
                *at += n;
            }
        }
        Ok(())
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
            S::Let(LetTarget::Local(id), Some(x)) if matches!(cx.locals[*id as usize].ty, Ty::Operand(_)) => {
                format!("let l{id} = {};\n", self.joperand(cx, x)?)
            }
            S::Let(LetTarget::Local(id), Some(x)) if Self::jagg_ty(&cx.locals[*id as usize].ty) => {
                let ns = self.jleaf_ns(&cx.locals[*id as usize].ty)?;
                format!("let l{id}: Vec<machine_jit::Slot> = {{ let v = {}; {ns}.iter().zip(v).map(|(n, x)| e.local(*n, x)).collect() }};\n", self.jagg(cx, x)?)
            }
            S::Let(LetTarget::Local(id), Some(x)) => {
                format!("let l{id} = {{ let v = {}; e.local({}, v) }};\n", self.jexpr(cx, x)?, self.jn(&cx.locals[*id as usize].ty)?)
            }
            S::Let(t @ LetTarget::Tuple(_), Some(x)) => {
                let mut s = format!("let tv = {};\n", self.jagg(cx, x)?);
                let mut at = 0;
                self.jlet_tuple(t, &x.ty, &mut at, &mut s)?;
                s
            }
            S::Let(LetTarget::Discard, Some(x)) => format!("let _ = {};\n", self.jexpr(cx, x)?),
            S::Let(LetTarget::Local(id), None) if Self::jagg_ty(&cx.locals[*id as usize].ty) => {
                format!("let l{id}: Vec<machine_jit::Slot> = {}.iter().map(|n| e.local(*n, machine_jit::V::K(0))).collect();\n", self.jleaf_ns(&cx.locals[*id as usize].ty)?)
            }
            S::Let(LetTarget::Local(id), None) => format!("let l{id} = e.local({}, machine_jit::V::K(0));\n", self.jn(&cx.locals[*id as usize].ty)?),
            S::Let(..) => return Err(GenError::new("lowering: a tuple let")),
            S::Expr(x) if matches!(x.kind, H::FnCall(..) | H::Call(..)) => format!("let _ = {};\n", self.jcall(cx, x)?),
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
            S::Return(Some(x)) if Self::jagg_ty(&x.ty) => format!("{{ let v = {}; for (s, x) in ret.iter().zip(v) {{ e.set(*s, x); }} e.ret(); }}\n", self.jagg(cx, x)?),
            S::Return(Some(x)) => format!("{{ let v = {}; e.set(ret, v); e.ret(); }}\n", self.jexpr(cx, x)?),
            S::Return(None) => "e.ret();\n".to_string(),
            S::Assign(place, v) if Self::jagg_ty(&place.ty) => format!("{{ let v = {}; let p = {}; for (p, x) in p.iter().zip(v) {{ e.write(p, x); }} }}\n", self.jagg(cx, v)?, self.jagg_place(cx, place)?),
            S::Assign(place, v) => format!("{{ let v = {}; let p = {}; e.write(&p, v); }}\n", self.jexpr(cx, v)?, self.jplace(cx, place)?),
            S::Raise(d, args) => {
                if cx.timing {
                    return Err(GenError::new("lowering: a raise in a timing entry"));
                }
                let f = self.faults[d];
                let mut s = String::from("{ ");
                let mut list = Vec::new();
                for (i, (a, (_, t))) in args.iter().zip(&f.params).enumerate() {
                    let _ = write!(s, "let x{i} = {}; let x{i} = e.cast(x{i}, {}, machine_jit::N::U64); ", self.jexpr(cx, a)?, self.jn(t)?);
                    list.push(format!("(x{i}, machine_jit::N::U64)"));
                }
                let _ = writeln!(s, "let _ = e.call(Self::jit_raise_{} as *const () as usize, &[{}], machine_jit::N::U64, false); e.exit(true); }}", ident(&f.name), list.join(", "));
                s
            }
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
            S::Unreachable(_) => "e.trap();\n".to_string(),
            S::Assert(c, _) => format!("{{ let c = {}; let bad = e.not(c, machine_jit::N::Bool); e.if_(bad, |e| e.trap(), |_| {{}}); }}\n", self.jexpr(cx, c)?),
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

    /// What does not lower, and why (`MACHINE_JIT_LOWERING`), after the
    /// last pass.
    pub(crate) fn jit_report(&self) {
        if std::env::var_os("MACHINE_JIT_LOWERING").is_none() {
            return;
        }
        let mut fns: Vec<&&hir::FnBody> = self.fns.values().filter(|f| !f.external && !self.jit_fns.borrow().contains(&f.def)).collect();
        fns.sort_by(|a, b| a.name.cmp(&b.name));
        for f in fns {
            if let Err(e) = self.jfn(f) {
                eprintln!("fn {}: {}", f.name, e.message);
            }
        }
        let mut kinds: Vec<&&hir::OperandKind> = self.kinds.values().collect();
        kinds.sort_by(|a, b| a.name.cmp(&b.name));
        for k in kinds {
            for r in &k.rules {
                if let Err(e) = self.jrule(k, r) {
                    eprintln!("kind {}.{}: {}", k.name, r.name, e.message);
                }
            }
        }
        for (i, u) in self.hir.units.iter().enumerate() {
            if !self.jit_units.borrow().contains(&(i as u32)) {
                if let Err(e) = self.junit(i as u32) {
                    eprintln!("unit {}: {}", self.prog.item_name(u.def), e.message);
                }
            }
        }
    }

    fn jfn(&self, f: &hir::FnBody) -> R<String> {
        self.jbody_fn(&format!("jf_{}", ident(&f.name)), &format!("fn {}", f.name), &f.locals, &f.params, &f.ret, &f.body)
    }

    /// An inlined body (a fn's, a unit's): parameters bound to locals, its
    /// returns joined, its value (`V`, or `Vec<V>` for an aggregate).
    fn jbody_fn(&self, name: &str, what: &str, locals: &[Local], params: &[LocalId], ret: &Ty, body: &Block) -> R<String> {
        let cx = Lx { locals, ops: None, src: None, timing: false };
        let mut ps = String::new();
        let mut bind = String::new();
        for (i, p) in params.iter().enumerate() {
            let t = &locals[*p as usize].ty;
            match t {
                Ty::Operand(d) => {
                    let _ = write!(ps, ", a{i}: {}", ident(&self.prog.item_name(*d)));
                    let _ = write!(bind, "let l{p} = a{i}; ");
                }
                t if Self::jagg_ty(t) => {
                    let _ = write!(ps, ", a{i}: Vec<machine_jit::V>");
                    let _ = write!(bind, "let l{p}: Vec<machine_jit::Slot> = {}.iter().zip(a{i}).map(|(n, x)| e.local(*n, x)).collect(); ", self.jleaf_ns(t)?);
                }
                t => {
                    let _ = write!(ps, ", a{i}: machine_jit::V");
                    let _ = write!(bind, "let l{p} = e.local({}, a{i}); ", self.jn(t)?);
                }
            }
        }
        let body = self.jblock(&cx, body)?;
        let attrs = "    #[allow(unused_variables, unused_mut, clippy::all)]\n";
        Ok(if Self::jagg_ty(ret) {
            format!(
                "    /// Lowered `{what}`.\n{attrs}    fn {name}(e: &mut machine_jit::Em{ps}) -> Vec<machine_jit::V> {{\n        let ret: Vec<machine_jit::Slot> = {}.iter().map(|n| e.local(*n, machine_jit::V::K(0))).collect();\n        e.region_begin();\n        {bind}{body}        e.region_end();\n        ret.iter().map(|s| e.get(*s)).collect()\n    }}\n",
                self.jleaf_ns(ret)?
            )
        } else {
            let rn = match ret {
                Ty::Unit | Ty::Never => "machine_jit::N::U8",
                t => self.jn(t)?,
            };
            format!(
                "    /// Lowered `{what}`.\n{attrs}    fn {name}(e: &mut machine_jit::Em{ps}) -> machine_jit::V {{\n        let ret = e.local({rn}, machine_jit::V::K(0));\n        e.region_begin_ret(ret);\n        {bind}{body}        e.region_end();\n        e.get(ret)\n    }}\n"
            )
        })
    }

    /// The units that lower, each as `junit_<name>_<args>`.
    pub(crate) fn jit_lower_units(&self) -> String {
        let mut ok: HashSet<u32> = (0..self.hir.units.len() as u32).collect();
        loop {
            *self.jit_units.borrow_mut() = ok.clone();
            let before = ok.len();
            ok.retain(|i| self.junit(*i).is_ok());
            if ok.len() == before {
                break;
            }
        }
        *self.jit_units.borrow_mut() = ok.clone();
        let mut ids: Vec<u32> = ok.into_iter().collect();
        ids.sort();
        let mut s = String::new();
        for i in ids {
            if let Ok(t) = self.junit(i) {
                s.push_str(&t);
            }
        }
        s
    }

    fn junit(&self, i: u32) -> R<String> {
        let u = &self.hir.units[i as usize];
        let name = format!("j{}", self.unit_fn_name(hir::UnitInstId(i)));
        self.jbody_fn(&name, &self.unit_fn_name(hir::UnitInstId(i)), &u.locals, &u.params, &u.ret, &u.body)
    }

    /// The fns too big to inline at every call (their HIR and what they
    /// would inline, past `JIT_INLINE`): called through `jit_call_<name>`.
    /// Only fns whose parameters are scalars or records and whose result is
    /// a scalar (or nothing) can be.
    pub(crate) fn jit_find_outlined(&self) {
        const JIT_INLINE: usize = 300;
        let mut cost: std::collections::HashMap<DefId, usize> = std::collections::HashMap::new();
        let mut out: HashSet<DefId> = HashSet::new();
        let mut defs: Vec<DefId> = self.fns.keys().copied().collect();
        defs.sort();
        // Callees first: a few rounds settle any order.
        for _ in 0..8 {
            for d in &defs {
                let f = self.fns[d];
                let n = std::cell::Cell::new(0usize);
                let calls = std::cell::RefCell::new(Vec::new());
                hir::visit_block(&f.body, &mut |_| n.set(n.get() + 1), &mut |x| {
                    n.set(n.get() + 1);
                    match &x.kind {
                        H::FnCall(g, _) => calls.borrow_mut().push(*g),
                        H::Call(u, _) => n.set(n.get() + 4 * self.hir.units[u.0 as usize].body.stmts.len()),
                        _ => {}
                    }
                });
                let mut c = n.get();
                for g in calls.into_inner() {
                    if !out.contains(&g) && g != *d {
                        c += cost.get(&g).copied().unwrap_or(0);
                    }
                }
                cost.insert(*d, c);
                let ok_params = f.params.iter().all(|p| {
                    let t = &f.locals[*p as usize].ty;
                    !matches!(t, Ty::Operand(_) | Ty::OperandTail(_)) && self.jleaves(t).is_ok()
                });
                let ok_ret = !Self::jagg_ty(&f.ret) && self.jleaves(&f.ret).is_ok() || matches!(f.ret, Ty::Unit | Ty::Never);
                if c > JIT_INLINE && ok_params && ok_ret && !f.external {
                    out.insert(*d);
                }
            }
        }
        if std::env::var_os("MACHINE_JIT_LOWERING").is_some() {
            let mut v: Vec<(&str, usize, bool)> = defs.iter().map(|d| (self.fns[d].name.as_str(), cost[d], out.contains(d))).collect();
            v.sort_by_key(|x| std::cmp::Reverse(x.1));
            for (n, c, o) in v.iter().take(30) {
                eprintln!("cost {n}: {c}{}", if *o { " (called)" } else { "" });
            }
        }
        *self.jit_outlined.borrow_mut() = out;
    }

    /// A Rust value of `ty` rebuilt from u64 leaves `a{at}..` (for a call
    /// into the interpreter).
    fn jrebuild(&self, ty: &Ty, at: &mut usize) -> R<String> {
        Ok(match ty {
            Ty::Bundle(d) => {
                let bd = self.bundles[d];
                let mut parts = Vec::new();
                for f in &bd.fields {
                    parts.push(format!("{}: {}", ident(&f.name), self.jrebuild(&f.ty, at)?));
                }
                format!("{} {{ {} }}", ident(&bd.name), parts.join(", "))
            }
            Ty::Tuple(items) => {
                let mut parts = Vec::new();
                for (_, t) in items {
                    parts.push(self.jrebuild(t, at)?);
                }
                format!("({},)", parts.join(", "))
            }
            Ty::Bool => {
                *at += 1;
                format!("(a{} != 0)", *at - 1)
            }
            t => {
                *at += 1;
                format!("(a{} as {})", *at - 1, self.rty(t)?)
            }
        })
    }

    /// The shims of the outlined fns: the interpreter's fn, its fault
    /// delivered as the step's tail would deliver it (`jit_pending` then
    /// ends the run).
    pub(crate) fn jit_outlined_shims(&self) -> R<String> {
        let mut out = String::from("    /// A field's size (for `call_fx`'s ranges).\n    #[allow(dead_code)]\n    fn jit_fsize<T>(_: impl for<'a> Fn(&'a Self) -> &'a T) -> usize {\n        std::mem::size_of::<T>()\n    }\n    /// The fault an outlined fn raised (`jit_fault`), delivered as the\n    /// step's tail would deliver it (`jit_pending` then ends the run).\n    extern \"C\" fn jit_deliver_c(m: *mut Self) -> u64 {\n        // SAFETY: called by compiled code with its machine.\n        let m = unsafe { &mut *m };\n        let Some(f) = m.jit_fault.take() else { return 0 };\n        if m.jit_last != 0 {\n            // SAFETY: the step's instruction, in its run.\n            m.last = Some(unsafe { *(m.jit_last as usize as *const Instr) });\n            m.jit_last = 0;\n        }\n        m.decoded = true;\n        m.last_fault = None;\n        let r = m.jit_tail(m.tr_phys, Err(f));\n        m.bus.after_step();\n        if let Err(f) = r {\n            m.jit_err = Some(f);\n        }\n        0\n    }\n");
        let mut defs: Vec<DefId> = self.jit_outlined.borrow().iter().copied().collect();
        defs.sort();
        for d in defs {
            let f = self.fns[&d];
            let mut at = 0;
            let mut args = Vec::new();
            for p in &f.params {
                args.push(self.jrebuild(&f.locals[*p as usize].ty, &mut at)?);
            }
            let ps: String = (0..at).map(|i| format!(", a{i}: u64")).collect();
            let ok = match &f.ret {
                Ty::Unit | Ty::Never => "0".to_string(),
                _ => "v as u64".to_string(),
            };
            let _ = write!(
                out,
                "    #[allow(clippy::all, unused_variables)]\n    extern \"C\" fn jit_call_{n}(m: *mut Self{ps}) -> u64 {{\n        // SAFETY: called by compiled code with its machine.\n        let m = unsafe {{ &mut *m }};\n        match m.fn_{n}({}) {{\n            Ok(v) => {ok},\n            Err(f) => {{\n                // Delivered by `jit_deliver_c`, once the code has stored\n                // what it owes.\n                m.jit_fault = Some(f);\n                m.jit_pending = true;\n                0\n            }}\n        }}\n    }}\n",
                args.join(", "),
                n = ident(&f.name)
            );
        }
        Ok(out)
    }

    /// Calls of the bus's external fns, for compiled code.
    pub(crate) fn jit_ext_shims(&self) -> R<String> {
        let mut out = String::new();
        let mut fns: Vec<&&hir::FnBody> = self.fns.values().filter(|f| f.external).collect();
        fns.sort_by(|a, b| a.name.cmp(&b.name));
        for f in fns {
            if Self::jagg_ty(&f.ret) || f.params.iter().any(|p| Self::jagg_ty(&f.locals[*p as usize].ty)) {
                continue;
            }
            let mut ps = String::new();
            let mut args = Vec::new();
            for (i, p) in f.params.iter().enumerate() {
                let _ = write!(ps, ", a{i}: u64");
                args.push(match &f.locals[*p as usize].ty {
                    Ty::Bool => format!("a{i} != 0"),
                    t => format!("a{i} as {}", self.rty(t)?),
                });
            }
            let ret = match &f.ret {
                Ty::Unit | Ty::Never => format!("m.bus.ext_{}({}); 0", ident(&f.name), args.join(", ")),
                _ => format!("m.bus.ext_{}({}) as u64", ident(&f.name), args.join(", ")),
            };
            let _ = write!(out, "    #[allow(clippy::all)]\n    extern \"C\" fn jit_ext_{}(m: *mut Self{ps}) -> u64 {{\n        // SAFETY: called by compiled code with its machine.\n        let m = unsafe {{ &mut *m }};\n        {ret}\n    }}\n", ident(&f.name));
        }
        Ok(out)
    }

    /// The registers fn `d` (and the fns it calls) reads and writes; None
    /// when it may touch any (an operand kind's, an external fn).
    pub(crate) fn jit_reg_fx(&self, d: DefId) -> Option<(HashSet<DefId>, HashSet<DefId>)> {
        fn root(e: &Expr) -> Option<DefId> {
            match &e.kind {
                H::Register(d) | H::RegField(d, _) | H::RegView(d, _) | H::Latch(d) => Some(*d),
                H::Field(b, _) | H::Index(b, _) | H::Slice(b, ..) | H::TupleField(b, _) => root(b),
                _ => None,
            }
        }
        let reads = std::cell::RefCell::new(HashSet::new());
        let writes = std::cell::RefCell::new(HashSet::new());
        let all = std::cell::Cell::new(false);
        let mut seen: HashSet<DefId> = HashSet::new();
        let stack = std::cell::RefCell::new(vec![d]);
        loop {
            let Some(f) = stack.borrow_mut().pop() else { break };
            if !seen.insert(f) {
                continue;
            }
            let Some(fb) = self.fns.get(&f) else {
                all.set(true);
                continue;
            };
            if fb.external {
                all.set(true);
                continue;
            }
            hir::visit_block(
                &fb.body,
                &mut |st| match &st.kind {
                    S::Assign(p, _) => {
                        if let Some(r) = root(p) {
                            writes.borrow_mut().insert(r);
                        }
                    }
                    S::OperandWrite(..) => all.set(true),
                    _ => {}
                },
                &mut |x| match &x.kind {
                    H::Register(r) | H::RegField(r, _) | H::RegView(r, _) | H::Latch(r) => {
                        reads.borrow_mut().insert(*r);
                    }
                    H::FnCall(g, _) => stack.borrow_mut().push(*g),
                    H::KindSelect(..) | H::OperandField(..) | H::OperandTail(..) | H::TailOperand(..) => all.set(true),
                    _ => {}
                },
            );
        }
        (!all.get()).then(|| (reads.into_inner(), writes.into_inner()))
    }

    /// For a call of fn `d`: the native registers and latches it leaves
    /// alone, and those it only reads, as `call_fx`'s byte ranges (Rust);
    /// the trace core's bookkeeping, which no isa fn touches, is left alone
    /// too (but the epochs: a register's write bumps them).
    fn jit_call_ranges(&self, d: DefId) -> Option<(String, String)> {
        let (reads, writes) = self.jit_reg_fx(d)?;
        let mut kept = String::new();
        let mut read = String::new();
        for f in ["tr_on", "tr_prev", "tr_idx", "tr_ep0", "tr_w", "tr_rec_now", "tr_phys", "jit_left", "jit_last", "jit_guard_failed", "last_start"] {
            let _ = write!(kept, "(std::mem::offset_of!(Self, {f}), Self::jit_fsize(|m: &Self| &m.{f})), ");
        }
        for l in &self.isa.latches {
            let Ok(n) = self.jn(&l.ty) else { continue };
            let range = format!("(std::mem::offset_of!(Self, latch_{}), {n}.width() as usize / 8), ", ident(&l.name));
            if writes.contains(&l.def) {
                continue;
            }
            if reads.contains(&l.def) {
                read.push_str(&range);
            } else {
                kept.push_str(&range);
            }
        }
        let mut regs: Vec<(&DefId, &(usize, &hir::Register))> = self.regs.iter().collect();
        regs.sort_by_key(|(d, _)| **d);
        for (rd, (_, r)) in regs {
            let Ok(n) = self.jn(&r.ty) else { continue };
            let range = format!("(std::mem::offset_of!(Self, r.{}), {n}.width() as usize / 8), ", ident(&r.name));
            if writes.contains(rd) {
                continue;
            }
            if reads.contains(rd) {
                read.push_str(&range);
            } else {
                kept.push_str(&range);
            }
        }
        Some((kept, read))
    }

    /// The fns that may raise a fault (themselves or a fn they call).
    pub(crate) fn jit_find_raisers(&self) {
        let mut raisers: HashSet<DefId> = HashSet::new();
        loop {
            let before = raisers.len();
            for f in self.fns.values() {
                if raisers.contains(&f.def) {
                    continue;
                }
                let hit = std::cell::Cell::new(false);
                hir::visit_block(&f.body, &mut |st| hit.set(hit.get() | matches!(st.kind, S::Raise(..))), &mut |x| {
                    if let H::FnCall(g, _) = &x.kind {
                        hit.set(hit.get() | raisers.contains(g));
                    }
                });
                if hit.get() {
                    raisers.insert(f.def);
                }
            }
            if raisers.len() == before {
                break;
            }
        }
        *self.jit_raisers.borrow_mut() = raisers;
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
                if let Ok(t) = self.jrule(kind, r) {
                    s.push_str(&t);
                    ok.insert((kind.def, ri as u32));
                }
            }
        }
        *self.jit_rules.borrow_mut() = ok;
        s
    }

    fn jrule(&self, kind: &hir::OperandKind, r: &hir::KindRule) -> R<String> {
        let src = kind.source.and(r.params.first().copied());
        let cx = Lx { locals: &r.locals, ops: None, src, timing: false };
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
            _ if Self::jagg_ty(&r.ty) => {
                let body = match &r.body {
                    ArmBody::Expr(x) => self.jagg(&cx, x)?,
                    ArmBody::Block(b) => self.jagg_block(&cx, b)?,
                };
                let name = match r.kind {
                    hir::RuleKind::Value => "value".to_string(),
                    hir::RuleKind::Read => "read".to_string(),
                    _ => ident(&r.name),
                };
                format!("{attrs}    fn jk_{k}_{name}(e: &mut machine_jit::Em{params}) -> Vec<machine_jit::V> {{\n        {bind}{body}\n    }}\n")
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
    /// Whether `i`'s timing entry is evaluated after it runs (`repeats()`).
    pub(crate) fn jtiming_after(i: &hir::Instruction) -> bool {
        let mut after = false;
        if let Some(t) = &i.timing {
            hir::visit_expr(t, &mut |_| {}, &mut |x| after |= matches!(&x.kind, H::Builtin(Builtin::Repeats, _)));
        }
        after
    }

    pub(crate) fn jinstr(&self, i: &hir::Instruction) -> R<String> {
        let n = ident(&i.name);
        let ops_ty = Self::ops_struct_name(i);
        let timing = match &i.timing {
            Some(t) => {
                let cx = Lx { locals: &i.timing_locals, ops: Some(i), src: None, timing: true };
                let t = self.jexpr(&cx, t)?;
                format!("{{ let v = {t}; let _ = ops; v }}")
            }
            None => "machine_jit::V::K(0)".to_string(),
        };
        let cx = Lx { locals: &i.locals, ops: Some(i), src: None, timing: false };
        let body = match &i.semantics {
            Some(b) => self.jblock(&cx, b)?,
            None => String::new(),
        };
        let attrs = "    #[allow(unused_variables, unused_mut, clippy::all)]\n";
        Ok(format!(
            "{attrs}    fn jt_{n}(e: &mut machine_jit::Em, ops: &{ops_ty}, phys: u64) -> machine_jit::V {{\n        {timing}\n    }}\n{attrs}    fn jx_{n}(e: &mut machine_jit::Em, ops: &{ops_ty}) {{\n        let ret = e.local(machine_jit::N::U8, machine_jit::V::K(0));\n        e.region_begin();\n        {body}        e.region_end();\n    }}\n"
        ))
    }
}

fn ty_signed(ty: &Ty) -> bool {
    matches!(ty, Ty::SInt(_))
}

fn nname_bits(bits: u32) -> &'static str {
    nname(native(bits, false).0, false)
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
