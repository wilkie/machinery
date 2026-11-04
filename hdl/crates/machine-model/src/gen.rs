//! Rust code generation from the isa HIR.
//!
//! Value representation: `u<N>` is the smallest native unsigned type that
//! holds N bits, kept masked to N bits after every operation that can
//! overflow; `i<N>` is the smallest native signed type, kept sign-extended.
//! `bool` is `bool`. Enums are their backing integer with a module of named
//! constants. Bundles are structs. Operands are structs with a `tail` enum.
//! Places (registers, fields, views, memory cells, `select` results) have
//! read and write forms; assignments to compound place expressions (`match`
//! of places) push the write into the branches.

use std::collections::HashMap;
use std::fmt::Write as _;

mod forms;
mod jit;
mod system;
mod trace;

use machine_sema::builtins::Builtin;
use machine_sema::hir::{
    self, ArmBody, Block, Expr, ExprKind as H, LetTarget, Local, Pat, Stmt, StmtKind as S,
};
use machine_sema::program::{DefId, Program};
use machine_sema::ty::Ty;
use machine_syntax::ast::{BinaryOp, UnaryOp};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenError {
    pub message: String,
}

impl GenError {
    pub fn new(message: impl Into<String>) -> GenError {
        GenError {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for GenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for GenError {}

type R<T> = Result<T, GenError>;

const KEYWORDS: &[&str] = &[
    "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn", "for",
    "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return",
    "self", "Self", "static", "struct", "super", "trait", "true", "type", "unsafe", "use", "where",
    "while", "async", "await", "dyn", "abstract", "become", "box", "do", "final", "macro",
    "override", "priv", "typeof", "unsized", "virtual", "yield", "try", "gen",
];

/// A Rust-safe identifier.
fn ident(name: &str) -> String {
    if KEYWORDS.contains(&name) {
        format!("{name}_")
    } else if name.contains('<') {
        // A family instance (SPEC 11.2d): `rm<16>` is `rm_16`.
        name.replace('<', "_").replace('>', "")
    } else {
        name.to_string()
    }
}

/// Native width and type name for an N-bit unsigned or signed integer.
fn native(bits: u32, signed: bool) -> (u32, &'static str) {
    let w = match bits {
        0..=8 => 8,
        9..=16 => 16,
        17..=32 => 32,
        33..=64 => 64,
        _ => 128,
    };
    let name = match (w, signed) {
        (8, false) => "u8",
        (16, false) => "u16",
        (32, false) => "u32",
        (64, false) => "u64",
        (128, false) => "u128",
        (8, true) => "i8",
        (16, true) => "i16",
        (32, true) => "i32",
        (64, true) => "i64",
        _ => "i128",
    };
    (w, name)
}

fn mask_literal(bits: u32, ty: &str) -> String {
    if bits >= 128 {
        format!("{ty}::MAX")
    } else {
        format!("{:#x}{ty}", (1u128 << bits) - 1)
    }
}

/// How the current body returns and what it can see.
struct Cx<'a> {
    locals: &'a [Local],
    /// The body returns `Result<_, Fault>`.
    fallible: bool,
    /// The receiver expression for architectural state (`self` or `m`).
    recv: &'a str,
    /// Instruction operand bindings in scope (instruction semantics).
    ops: Option<&'a hir::Instruction>,
    /// The component whose model body this is (state, ports, `ctx`).
    comp: Option<&'a hir::ComponentInfo>,
    /// A transaction handler: values are returned as `Some(..)` and
    /// `decline` returns `None`.
    opt: bool,
}

/// A component's `memo fn` bookkeeping while it is emitted (SPEC 8.3b).
#[derive(Clone, Debug, Default)]
pub(crate) struct MemoCur {
    pub states: HashMap<u32, u32>,
    pub mems: HashMap<u32, u8>,
}

pub struct Gen<'a> {
    prog: &'a Program,
    hir: &'a hir::Hir,
    isa: &'a hir::IsaInfo,
    enums: HashMap<DefId, &'a hir::EnumDef>,
    bundles: HashMap<DefId, &'a hir::BundleDef>,
    regs: HashMap<DefId, (usize, &'a hir::Register)>,
    spaces: HashMap<DefId, &'a hir::Space>,
    operands: HashMap<DefId, &'a hir::Operand>,
    kinds: HashMap<DefId, &'a hir::OperandKind>,
    faults: HashMap<DefId, &'a hir::Fault>,
    fns: HashMap<DefId, &'a hir::FnBody>,
    out: String,
    /// Emitting the istream's `next` fn, whose reads are code fetches.
    in_istream_next: std::cell::Cell<bool>,
    /// The component being emitted, when it has `memo fn`s: its versioned
    /// state (index to bit) and memories (index to number).
    pub(crate) memo_cur: std::cell::RefCell<Option<MemoCur>>,
    /// Emitting a recording copy: reads of versioned state are recorded.
    pub(crate) memo_rec: std::cell::Cell<bool>,
    /// The trace core (`core: trace;`), when the profile asks for it; and
    /// the JIT core (`core: jit;`), which builds on it.
    trace: Option<trace::TracePlan>,
    jit: bool,
    /// Operand kinds with forms, and the forms of the copy being emitted
    /// (operand index, form).
    forms: HashMap<DefId, forms::FormKind>,
    form_ctx: std::cell::RefCell<Vec<(u32, u8)>>,
}

pub fn generate(prog: &Program, hir: &hir::Hir, isa_def: DefId) -> R<String> {
    let mut g = Gen::new(prog, hir, isa_def)?;
    g.emit_all()?;
    Ok(g.out)
}

/// The interpreter of the system's processor, then the system: component
/// models, buses with their address decoders, time and signals.
pub fn generate_system(
    prog: &Program,
    hir: &hir::Hir,
    sys_def: DefId,
    profile: Option<&str>,
) -> R<String> {
    let sys = hir
        .systems
        .iter()
        .find(|s| s.def == sys_def)
        .ok_or_else(|| GenError::new("system was not checked (errors earlier)"))?;
    let proc_idx = system::processor(hir, sys)?;
    let comp = system::comp_info(hir, sys.components[proc_idx].component)?;
    let mut g = Gen::new(prog, hir, comp.isa.unwrap())?;
    // The profile's core: the trace core is planned before the processor is
    // emitted (assignments bump its epoch).
    let prof = profile.and_then(|n| sys.profiles.iter().find(|p| p.name == n));
    if prof.is_some_and(|p| matches!(p.core, hir::Core::Trace | hir::Core::Jit)) {
        if !prof.is_some_and(|p| p.direct_memory) {
            return Err(GenError::new("`core: trace` and `core: jit` need `memory: direct` (code pages are counted in direct memory)"));
        }
        g.trace = Some(g.plan_trace()?);
        g.jit = prof.is_some_and(|p| p.core == hir::Core::Jit);
    }
    g.emit_all()?;
    g.emit_system(sys, proc_idx, profile)?;
    Ok(g.out)
}

impl<'a> Gen<'a> {
    fn new(prog: &'a Program, hir: &'a hir::Hir, isa_def: DefId) -> R<Gen<'a>> {
        let isa = hir
            .isas
            .iter()
            .find(|i| i.def == isa_def)
            .ok_or_else(|| GenError::new("isa was not checked (errors earlier)"))?;
        Ok(Gen {
            prog,
            hir,
            isa,
            enums: hir.enums.iter().map(|e| (e.def, e)).collect(),
            bundles: hir.bundles.iter().map(|b| (b.def, b)).collect(),
            regs: isa
                .registers
                .iter()
                .enumerate()
                .map(|(i, r)| (r.def, (i, r)))
                .collect(),
            spaces: isa.spaces.iter().map(|s| (s.def, s)).collect(),
            operands: isa.operands.iter().map(|o| (o.def, o)).collect(),
            kinds: isa.kinds.iter().map(|k| (k.def, k)).collect(),
            faults: isa.faults.iter().map(|f| (f.def, f)).collect(),
            fns: isa.fns.iter().map(|f| (f.def, f)).collect(),
            out: String::new(),
            in_istream_next: std::cell::Cell::new(false),
            memo_cur: std::cell::RefCell::new(None),
            memo_rec: std::cell::Cell::new(false),
            trace: None,
            jit: false,
            forms: HashMap::new(),
            form_ctx: std::cell::RefCell::new(Vec::new()),
        })
    }
}

impl<'a> Gen<'a> {
    fn line(&mut self, s: &str) {
        self.out.push_str(s);
        self.out.push('\n');
    }

    // -- types -----------------------------------------------------------------

    fn rty(&self, ty: &Ty) -> R<String> {
        Ok(match ty {
            Ty::UInt(n) => native(*n, false).1.to_string(),
            Ty::SInt(n) => native(*n, true).1.to_string(),
            Ty::Bool => "bool".into(),
            Ty::Enum(d) => {
                let e = self
                    .enums
                    .get(d)
                    .ok_or_else(|| GenError::new("unresolved enum"))?;
                native(e.width, false).1.to_string()
            }
            Ty::Bundle(d) => ident(&self.prog.item_name(*d)),
            Ty::Union(_) => {
                return Err(GenError::new(
                    "unions are not supported by the interpreter backend yet",
                ))
            }
            Ty::Array(t, n) => format!("[{}; {n}]", self.rty(t)?),
            Ty::Tuple(items) => {
                let parts: R<Vec<String>> = items.iter().map(|(_, t)| self.rty(t)).collect();
                let parts = parts?;
                if parts.len() == 1 {
                    format!("({},)", parts[0])
                } else {
                    format!("({})", parts.join(", "))
                }
            }
            Ty::Const(_) => "i128".into(),
            Ty::Operand(d) => ident(&self.prog.item_name(*d)),
            Ty::OperandTail(d) => format!("{}_Tail", ident(&self.prog.item_name(*d))),
            Ty::Str => "&'static str".into(),
            Ty::Unit => "()".into(),
            Ty::Never => "()".into(),
            Ty::Error => return Err(GenError::new("expression with an error type")),
        })
    }

    /// The integer shape of a type: `(bits, signed, native type)`.
    fn int_shape(&self, ty: &Ty) -> Option<(u32, bool, &'static str)> {
        match ty {
            Ty::UInt(n) => Some((*n, false, native(*n, false).1)),
            Ty::SInt(n) => Some((*n, true, native(*n, true).1)),
            Ty::Enum(d) => self
                .enums
                .get(d)
                .map(|e| (e.width, false, native(e.width, false).1)),
            Ty::Bool => Some((1, false, "bool")),
            Ty::Const(_) => Some((128, true, "i128")),
            _ => None,
        }
    }

    /// Normalise `text` (of type `ty`) to its declared width.
    fn fix(&self, text: String, ty: &Ty) -> String {
        match ty {
            Ty::UInt(n) => {
                let (w, name) = native(*n, false);
                if *n == w {
                    text
                } else {
                    format!("(({text}) & {})", mask_literal(*n, name))
                }
            }
            Ty::SInt(n) => {
                let (w, _) = native(*n, true);
                if *n == w {
                    text
                } else {
                    let k = w - n;
                    format!("((({text}) << {k}) >> {k})")
                }
            }
            _ => text,
        }
    }

    fn int_literal(&self, v: i128, ty: &Ty) -> R<String> {
        Ok(match ty {
            Ty::UInt(_) | Ty::Enum(_) => {
                let (_, _, name) = self.int_shape(ty).unwrap();
                format!("{v}{name}")
            }
            Ty::SInt(_) => {
                let (_, _, name) = self.int_shape(ty).unwrap();
                if v < 0 {
                    format!("({v}{name})")
                } else {
                    format!("{v}{name}")
                }
            }
            Ty::Const(_) => {
                if v < 0 {
                    format!("({v}i128)")
                } else {
                    format!("{v}i128")
                }
            }
            Ty::Bool => (if v != 0 { "true" } else { "false" }).to_string(),
            other => {
                return Err(GenError::new(format!(
                    "integer literal of non-integer type {other:?}"
                )))
            }
        })
    }

    fn local_name(&self, cx: &Cx, id: hir::LocalId) -> String {
        let l = &cx.locals[id as usize];
        format!("l{}_{}", id, ident(&l.name))
    }

    // -- expressions ----------------------------------------------------------------

    fn expr(&self, cx: &Cx, e: &Expr) -> R<String> {
        let q = if cx.fallible { "?" } else { "" };
        Ok(match &e.kind {
            H::Int(v) => self.int_literal(*v, &e.ty)?,
            H::Bool(b) => b.to_string(),
            H::Str(s) => format!("{s:?}"),
            H::Local(id) => self.local_name(cx, *id),
            H::Const(_) => return Err(GenError::new("unexpanded constant")),
            H::Variant(d, idx) => {
                let en = self.enums[d];
                format!(
                    "{}::{}",
                    ident(&en.name),
                    ident(&en.variants[*idx as usize].0)
                )
            }
            H::Unary(op, a) => {
                let at = self.expr(cx, a)?;
                match op {
                    UnaryOp::Not => format!("(!{at})"),
                    UnaryOp::BitNot => self.fix(format!("(!{at})"), &e.ty),
                    UnaryOp::Neg => self.fix(format!("({at}).wrapping_neg()"), &e.ty),
                }
            }
            H::Binary(op, a, b) => self.binary(cx, *op, a, b, &e.ty)?,
            H::Ternary(c, a, b) => format!(
                "(if {} {{ {} }} else {{ {} }})",
                self.expr(cx, c)?,
                self.expr(cx, a)?,
                self.expr(cx, b)?
            ),
            H::Cast(inner) => self.cast(cx, inner, &e.ty)?,
            H::Field(base, i) => {
                let Ty::Bundle(d) = &base.ty else {
                    return Err(GenError::new("field of non-bundle"));
                };
                let bd = self.bundles[d];
                format!(
                    "{}.{}",
                    self.expr(cx, base)?,
                    ident(&bd.fields[*i as usize].name)
                )
            }
            H::TupleField(base, i) => format!("{}.{i}", self.expr(cx, base)?),
            H::Index(base, idx) if self.memo_rec.get() && matches!(base.kind, H::ModelMem(_, mi) if self.memo_mem(mi).is_some()) => {
                let H::ModelMem(_, mi) = base.kind else { unreachable!() };
                let m = self.memo_mem(mi).unwrap();
                let bt = self.expr(cx, base)?;
                let it = self.expr(cx, idx)?;
                format!("({{ let i = ({it}) as usize; {}.memo_rec.cell({m}, i); {bt}[i] }})", cx.recv)
            }
            H::Index(base, idx) => {
                let bt = self.expr(cx, base)?;
                let it = self.expr(cx, idx)?;
                match &base.ty {
                    // The index first: it may call into the component.
                    Ty::Array(..) => format!("({{ let i = ({it}) as usize; {bt}[i] }})"),
                    _ => format!("((({bt}).checked_shr(({it}) as u32).unwrap_or(0) & 1) as u8)"),
                }
            }
            H::Slice(base, hi, lo) => {
                let bt = self.expr(cx, base)?;
                let (_, _, out_ty) = self.int_shape(&e.ty).unwrap();
                let width = hi - lo + 1;
                let (_, src_ty) = match &base.ty {
                    Ty::UInt(n) => native(*n, false),
                    Ty::SInt(n) => native(*n, true),
                    _ => return Err(GenError::new("slice of non-integer")),
                };
                let mask = mask_literal(width, src_ty.trim_start_matches('i').to_string().as_str());
                let mask = if src_ty.starts_with('i') {
                    format!("{} as {src_ty}", mask)
                } else {
                    mask
                };
                format!("(((({bt}) >> {lo}) & ({mask})) as {out_ty})")
            }
            H::Call(inst, args) => {
                let name = self.unit_fn_name(*inst);
                let a: R<Vec<String>> = args.iter().map(|x| self.expr(cx, x)).collect();
                format!("{name}({})", a?.join(", "))
            }
            H::Builtin(b, args) => self.builtin(cx, *b, args, &e.ty)?,
            H::BundleLit(d, fields) => {
                let bd = self.bundles[d];
                let mut parts = Vec::new();
                for (f, v) in bd.fields.iter().zip(fields) {
                    parts.push(format!("{}: {}", ident(&f.name), self.expr(cx, v)?));
                }
                format!("{} {{ {} }}", ident(&bd.name), parts.join(", "))
            }
            H::Tuple(items) => {
                let a: R<Vec<String>> = items.iter().map(|x| self.expr(cx, x)).collect();
                let a = a?;
                if a.len() == 1 {
                    format!("({},)", a[0])
                } else {
                    format!("({})", a.join(", "))
                }
            }
            H::Array(items) => {
                let a: R<Vec<String>> = items.iter().map(|x| self.expr(cx, x)).collect();
                format!("[{}]", a?.join(", "))
            }
            H::ArrayRepeat(v, n) => format!("[{}; {n}]", self.expr(cx, v)?),
            H::Match(scrut, arms) => {
                let mut s = format!("(match {} {{\n", self.expr(cx, scrut)?);
                for arm in arms {
                    let pats = self.patterns(&arm.patterns, &scrut.ty)?;
                    let body = match &arm.body {
                        ArmBody::Expr(x) => self.expr(cx, x)?,
                        ArmBody::Block(b) => self.block_expr(cx, b)?,
                    };
                    let _ = writeln!(s, "{pats} => {body},");
                }
                if self.needs_catch_all(arms, &scrut.ty) {
                    s.push_str("_ => unreachable!(\"value outside the type's encodings\"),\n");
                }
                s.push_str("})");
                s
            }
            H::If(c, then_b, else_e) => {
                let ct = self.expr(cx, c)?;
                let tb = self.block_expr(cx, then_b)?;
                match else_e {
                    Some(el) => format!("(if {ct} {tb} else {{ {} }})", self.expr(cx, el)?),
                    None => format!("(if {ct} {tb})"),
                }
            }
            H::Block(b) => self.block_expr(cx, b)?,

            // -- architectural --
            H::Register(d) => format!("{}.r.{}", cx.recv, ident(&self.regs[d].1.name)),
            H::RegField(d, i) => self.reg_field_read(cx, *d, *i),
            H::RegView(d, i) => {
                let (_, r) = self.regs[d];
                let view = &r.views[*i as usize];
                let Ty::Bundle(bd) = &view.ty else {
                    unreachable!()
                };
                let bd = self.bundles[bd];
                let mut parts = Vec::new();
                for (f, ri) in bd.fields.iter().zip(&view.mapping) {
                    parts.push(format!(
                        "{}: {}",
                        ident(&f.name),
                        self.reg_field_read(cx, *d, *ri)
                    ));
                }
                format!("{} {{ {} }}", ident(&bd.name), parts.join(", "))
            }
            H::Mode(_) => format!("{}.mode", cx.recv),
            H::Latch(d) => format!("{}.latch_{}", cx.recv, ident(&self.prog.item_name(*d))),
            H::OperandRead(i) => {
                let instr = cx
                    .ops
                    .ok_or_else(|| GenError::new("operand outside an instruction"))?;
                let b = &instr.operands[*i as usize];
                let kind = self.kinds[&b.kind];
                let op = format!("ops.{}", ident(&b.name));
                if let Some(f) = self.form_of(*i) {
                    format!("{}.kind_{}_read_f{f}({op}){q}", cx.recv, ident(&kind.name))
                } else if kind.source.is_some() {
                    format!("{}.kind_{}_read({op}){q}", cx.recv, ident(&kind.name))
                } else if kind.rules.iter().any(|r| r.kind == hir::RuleKind::Value) {
                    format!("{}.kind_{}_value({op})", cx.recv, ident(&kind.name))
                } else {
                    format!("{}.kind_{}_select_read({op})", cx.recv, ident(&kind.name))
                }
            }
            H::OperandRule(i, rule) => {
                let instr = cx
                    .ops
                    .ok_or_else(|| GenError::new("operand outside an instruction"))?;
                let b = &instr.operands[*i as usize];
                let kind = self.kinds[&b.kind];
                let r = &kind.rules[*rule as usize];
                // In a copy for a form, `is_mem` is the form's.
                let suffix = match self.form_of(*i) {
                    Some(f) if r.kind == hir::RuleKind::IsMem => format!("_f{f}"),
                    _ => String::new(),
                };
                format!(
                    "{}.kind_{}_{}{suffix}(ops.{}){q}",
                    cx.recv,
                    ident(&kind.name),
                    ident(&r.name),
                    ident(&b.name)
                )
            }
            H::OperandField(base, i) => {
                let Ty::Operand(d) = &base.ty else {
                    unreachable!()
                };
                let o = self.operands[d];
                format!(
                    "{}.{}",
                    self.expr(cx, base)?,
                    ident(&o.fields[*i as usize].name)
                )
            }
            H::OperandTail(base) => format!("{}.tail", self.expr(cx, base)?),
            H::TailOperand(base, d) => {
                format!("{}.as_{}()", self.expr(cx, base)?, self.prog.item_name(*d))
            }
            H::SpaceRead(d, bits, addr) => {
                let sp = self.spaces[d];
                let (_, _, out_ty) = self.int_shape(&e.ty).unwrap();
                let op = if self.in_istream_next.get()
                    && self.isa.istream.as_ref().is_some_and(|i| i.space == *d)
                {
                    "fetch"
                } else {
                    "read"
                };
                // Data accesses are counted for fault delivery (SPEC 11.2c),
                // and may cost a split (`split =>`).
                let count = if op == "read" {
                    format!(
                        "{}.accesses = {}.accesses.wrapping_add(1); {}",
                        cx.recv,
                        cx.recv,
                        self.split_charge(cx, *d, *bits)
                    )
                } else {
                    String::new()
                };
                format!(
                    "{{ let a = ({}) as u64; {count}{}.bus.{}_{op}(a, {bits}) as {out_ty} }}",
                    self.expr(cx, addr)?,
                    cx.recv,
                    ident(&sp.name)
                )
            }
            H::SpaceCell(d, addr) => {
                let sp = self.spaces[d];
                let (_, _, out_ty) = self.int_shape(&e.ty).unwrap();
                format!(
                    "{{ let a = ({}) as u64; {1}.accesses = {1}.accesses.wrapping_add(1); {1}.bus.{2}_read(a, {3}) as {out_ty} }}",
                    self.expr(cx, addr)?,
                    cx.recv,
                    ident(&sp.name),
                    sp.cell_bits
                )
            }
            H::SpaceWrite(d, bits, addr, v) => {
                let sp = self.spaces[d];
                format!(
                    "{{ let a = ({}) as u64; let v = ({}) as u128; {2}.accesses = {2}.accesses.wrapping_add(1); {4}{2}.bus.{3}_write(a, {bits}, v); }}",
                    self.expr(cx, addr)?,
                    self.expr(cx, v)?,
                    cx.recv,
                    ident(&sp.name),
                    self.split_charge(cx, *d, *bits)
                )
            }
            H::KindSelect(k, args) => {
                let kind = self.kinds[k];
                let a: R<Vec<String>> = args.iter().map(|x| self.expr(cx, x)).collect();
                format!(
                    "{}.kind_{}_select_read({})",
                    cx.recv,
                    ident(&kind.name),
                    a?.join(", ")
                )
            }
            H::FnCall(d, args) => {
                let f = self
                    .fns
                    .get(d)
                    .ok_or_else(|| GenError::new("call of a fn from another isa"))?;
                // In the istream's `next`, its address goes through the
                // window (`istream { window ...; }`).
                if args.is_empty()
                    && self.in_istream_next.get()
                    && self.isa.istream.as_ref().is_some_and(|i| i.address == *d && i.window.is_some())
                {
                    return Ok(format!("{}.istream_address()?", cx.recv));
                }
                let a: R<Vec<String>> = args.iter().map(|x| self.expr(cx, x)).collect();
                if f.external {
                    // Supplied by the bus side: a processor model handler,
                    // or a default in `Flat`.
                    let a = a?;
                    let mut s = String::from("{ ");
                    for (i, arg) in a.iter().enumerate() {
                        let _ = write!(s, "let a{i} = {arg}; ");
                    }
                    let names: Vec<String> = (0..a.len()).map(|i| format!("a{i}")).collect();
                    let _ = write!(
                        s,
                        "{}.bus.ext_{}({}) }}",
                        cx.recv,
                        ident(&f.name),
                        names.join(", ")
                    );
                    return Ok(s);
                }
                if !cx.fallible {
                    return Err(GenError::new(format!(
                        "fn `{}` called from a context that cannot fault",
                        f.name
                    )));
                }
                // Arguments first, in temporaries, so nested `&mut self`
                // calls do not overlap the call's own borrow.
                let a = a?;
                let mut s = String::from("{ ");
                for (i, arg) in a.iter().enumerate() {
                    let _ = write!(s, "let a{i} = {arg}; ");
                }
                let names: Vec<String> = (0..a.len()).map(|i| format!("a{i}")).collect();
                let _ = write!(
                    s,
                    "{}.fn_{}({})? }}",
                    cx.recv,
                    ident(&f.name),
                    names.join(", ")
                );
                s
            }
            H::State(_, i) => {
                let c = cx
                    .comp
                    .ok_or_else(|| GenError::new("state outside a component"))?;
                let m = c.model.as_ref().unwrap();
                let name = ident(&m.states[*i as usize].name);
                match self.memo_var(*i) {
                    Some(k) if self.memo_rec.get() => {
                        format!("({{ {0}.memo_rec.var({k}); {0}.{name} }})", cx.recv)
                    }
                    _ => format!("{}.{name}", cx.recv),
                }
            }
            H::ModelMem(_, i) => {
                let c = cx
                    .comp
                    .ok_or_else(|| GenError::new("mem outside a component"))?;
                let m = c.model.as_ref().unwrap();
                format!("{}.{}", cx.recv, ident(&m.mems[*i as usize].name))
            }
            H::BusCall(port, t, args) => {
                let c = cx
                    .comp
                    .ok_or_else(|| GenError::new("bus call outside a component"))?;
                // Arguments first, in temporaries: one may call into the
                // component, which borrows the context too.
                let a: R<Vec<String>> = args.iter().map(|x| self.expr(cx, x)).collect();
                let mut s = String::from("{ ");
                let mut names = Vec::new();
                for (k, arg) in a?.iter().enumerate() {
                    let _ = write!(s, "let b{k} = {arg}; ");
                    names.push(format!("b{k}"));
                }
                let _ = write!(
                    s,
                    "ctx.{}_{}({}) }}",
                    ident(&c.ports[*port as usize].name),
                    ident(&self.prog.item_name(*t)),
                    names.join(", ")
                );
                s
            }
            H::PortClocks(port, n) => {
                let c = cx
                    .comp
                    .ok_or_else(|| GenError::new("port outside a component"))?;
                format!("ctx.clocks_{}(({}) as u32)", ident(&c.ports[*port as usize].name), self.expr(cx, n)?)
            }
            H::ModelFnCall(i, args) => {
                let c = cx
                    .comp
                    .ok_or_else(|| GenError::new("model fn call outside a component"))?;
                let f = &c.model.as_ref().unwrap().fns[*i as usize];
                // Arguments first, so they may read the component.
                let mut s = String::from("{ ");
                let mut names = Vec::new();
                for (k, a) in args.iter().enumerate() {
                    let _ = write!(s, "let a{k} = {}; ", self.expr(cx, a)?);
                    names.push(format!(", a{k}"));
                }
                let rec = if self.memo_rec.get() { "_rec" } else { "" };
                let _ = write!(
                    s,
                    "{}.fn_{}{rec}(&mut *ctx{}) }}",
                    cx.recv,
                    ident(&f.name),
                    names.concat()
                );
                s
            }
            H::SignalLevel(port, sig, args) => {
                let c = cx
                    .comp
                    .ok_or_else(|| GenError::new("signal read outside a component"))?;
                let a: R<Vec<String>> = args.iter().map(|x| self.expr(cx, x)).collect();
                format!(
                    "ctx.level_{}_{}({})",
                    ident(&c.ports[*port as usize].name),
                    ident(&self.prog.item_name(*sig)),
                    a?.join(", ")
                )
            }
            H::Port(_, i, _) => {
                let c = cx
                    .comp
                    .ok_or_else(|| GenError::new("port outside a component"))?;
                format!("ctx.read_{}()", ident(&c.ports[*i as usize].name))
            }
            H::Error => return Err(GenError::new("error expression")),
        })
    }

    fn reg_field_read(&self, cx: &Cx, d: DefId, i: u32) -> String {
        let (_, r) = self.regs[&d];
        let f = &r.fields[i as usize];
        let (_, _, out_ty) = self.int_shape(&f.ty).unwrap_or((8, false, "u8"));
        let (_, rty) = match &r.ty {
            Ty::UInt(n) => native(*n, false),
            _ => (8, "u8"),
        };
        let width = match &f.ty {
            Ty::UInt(n) | Ty::SInt(n) => *n,
            Ty::Bool => 1,
            Ty::Enum(e) => self.enums[e].width,
            _ => 8,
        };
        let raw = format!(
            "((({}.r.{}) >> {}) & {})",
            cx.recv,
            ident(&r.name),
            f.offset,
            mask_literal(width, rty)
        );
        match &f.ty {
            Ty::Bool => format!("({raw} != 0)"),
            Ty::SInt(n) => {
                let (w, _) = native(*n, true);
                let k = w - n;
                format!("((({raw} as {out_ty}) << {k}) >> {k})")
            }
            _ => format!("({raw} as {out_ty})"),
        }
    }

    fn binary(&self, cx: &Cx, op: BinaryOp, a: &Expr, b: &Expr, ty: &Ty) -> R<String> {
        use BinaryOp::*;
        let at = self.expr(cx, a)?;
        let bt = self.expr(cx, b)?;
        Ok(match op {
            Add => self.fix(format!("({at}).wrapping_add({bt})"), ty),
            Sub => self.fix(format!("({at}).wrapping_sub({bt})"), ty),
            Mul => self.fix(format!("({at}).wrapping_mul({bt})"), ty),
            Div => format!("(({at}) / ({bt}))"),
            Rem => format!("(({at}) % ({bt}))"),
            BitAnd => format!("(({at}) & ({bt}))"),
            BitOr => format!("(({at}) | ({bt}))"),
            BitXor => format!("(({at}) ^ ({bt}))"),
            Shl => self.fix(format!("({at}).checked_shl(({bt}) as u32).unwrap_or(0)"), ty),
            Shr => match ty {
                Ty::SInt(_) => format!("{{ let s = {at}; s.checked_shr(({bt}) as u32).unwrap_or(if s < 0 {{ -1 }} else {{ 0 }}) }}"),
                _ => format!("({at}).checked_shr(({bt}) as u32).unwrap_or(0)"),
            },
            Eq => format!("(({at}) == ({bt}))"),
            Ne => format!("(({at}) != ({bt}))"),
            Lt => format!("(({at}) < ({bt}))"),
            Le => format!("(({at}) <= ({bt}))"),
            Gt => format!("(({at}) > ({bt}))"),
            Ge => format!("(({at}) >= ({bt}))"),
            And => format!("(({at}) && ({bt}))"),
            Or => format!("(({at}) || ({bt}))"),
        })
    }

    fn cast(&self, cx: &Cx, inner: &Expr, target: &Ty) -> R<String> {
        let it = self.expr(cx, inner)?;
        Ok(match (&inner.ty, target) {
            (Ty::Bool, Ty::UInt(_)) => format!("(({it}) as u8)"),
            (Ty::UInt(_), Ty::Bool) => format!("(({it}) != 0)"),
            (_, Ty::UInt(_) | Ty::SInt(_) | Ty::Enum(_)) => {
                let (_, _, name) = self.int_shape(target).unwrap();
                self.fix(format!("(({it}) as {name})"), target)
            }
            (a, b) if a == b => it,
            _ => return Err(GenError::new(format!("unsupported cast to {target:?}"))),
        })
    }

    fn builtin(&self, cx: &Cx, b: Builtin, args: &[Expr], ty: &Ty) -> R<String> {
        let a: R<Vec<String>> = args.iter().map(|x| self.expr(cx, x)).collect();
        let a = a?;
        Ok(match b {
            Builtin::Parity => format!("((({}).count_ones() & 1) as u8)", a[0]),
            Builtin::Popcount => {
                let (_, _, t) = self.int_shape(ty).unwrap();
                format!("(({}).count_ones() as {t})", a[0])
            }
            Builtin::Clz => {
                let (n, _, _) = self.int_shape(&args[0].ty).unwrap();
                let (w, _) = native(n, false);
                let (_, _, t) = self.int_shape(ty).unwrap();
                format!("((({}).leading_zeros() - {}) as {t})", a[0], w - n)
            }
            Builtin::Ctz => {
                let (n, _, _) = self.int_shape(&args[0].ty).unwrap();
                let (_, _, t) = self.int_shape(ty).unwrap();
                format!("((({}).trailing_zeros().min({n})) as {t})", a[0])
            }
            Builtin::Concat => {
                let (_, _, t) = self.int_shape(ty).unwrap();
                let mut shift = 0u32;
                let mut parts = Vec::new();
                for (x, xt) in a.iter().zip(args).rev() {
                    let (n, _, _) = self.int_shape(&xt.ty).unwrap();
                    parts.push(format!("((({x}) as {t}) << {shift})"));
                    shift += n;
                }
                format!("({})", parts.join(" | "))
            }
            Builtin::Replicate => {
                let (_, _, t) = self.int_shape(ty).unwrap();
                let (n, _, _) = self.int_shape(&args[0].ty).unwrap();
                let H::Int(count) = args[1].kind else {
                    return Err(GenError::new("replicate count"));
                };
                let parts: Vec<String> = (0..count)
                    .map(|i| format!("((({}) as {t}) << {})", a[0], i as u32 * n))
                    .collect();
                format!("({})", parts.join(" | "))
            }
            Builtin::Assert => {
                if a.len() == 2 {
                    format!("assert!({}, {})", a[0], a[1])
                } else {
                    format!("assert!({})", a[0])
                }
            }
            Builtin::Unreachable => {
                if a.is_empty() {
                    "unreachable!()".into()
                } else {
                    format!("unreachable!({})", a[0])
                }
            }
            Builtin::Sync => return Err(GenError::new("`sync` in an isa")),
            Builtin::Restarted => format!("{}.restarted", cx.recv),
            Builtin::Schedule => return Err(GenError::new("`schedule` is lowered by sema")),
        })
    }

    fn patterns(&self, pats: &[Pat], scrut_ty: &Ty) -> R<String> {
        let mut parts = Vec::new();
        for p in pats {
            parts.push(match p {
                Pat::Wildcard => "_".to_string(),
                Pat::Value(v) => match scrut_ty {
                    Ty::Bool => (if *v != 0 { "true" } else { "false" }).to_string(),
                    _ => v.to_string(),
                },
                Pat::Range(a, b) => format!("{a}..={b}"),
                Pat::Variant(i) => {
                    let Ty::Enum(d) = scrut_ty else {
                        unreachable!()
                    };
                    let en = self.enums[d];
                    format!(
                        "{}::{}",
                        ident(&en.name),
                        ident(&en.variants[*i as usize].0)
                    )
                }
                Pat::Bool(b) => b.to_string(),
            });
        }
        Ok(parts.join(" | "))
    }

    /// Rust needs a catch-all unless the match covers the native type.
    fn needs_catch_all(&self, arms: &[hir::Arm], scrut_ty: &Ty) -> bool {
        let pats: Vec<&Pat> = arms.iter().flat_map(|a| a.patterns.iter()).collect();
        if pats.iter().any(|p| matches!(p, Pat::Wildcard)) {
            return false;
        }
        match scrut_ty {
            Ty::Bool => false,
            Ty::UInt(n) => native(*n, false).0 != *n,
            Ty::SInt(n) => native(*n, true).0 != *n,
            _ => true,
        }
    }

    fn unit_fn_name(&self, inst: hir::UnitInstId) -> String {
        let u = &self.hir.units[inst.0 as usize];
        let mut name = format!("unit_{}", ident(&self.prog.item_name(u.def)));
        for a in &u.args {
            let _ = write!(name, "_{a}");
        }
        name
    }

    // -- blocks and statements -------------------------------------------------------

    /// A block in value position: `{ stmts; tail }`.
    fn block_expr(&self, cx: &Cx, b: &Block) -> R<String> {
        let mut s = String::from("{\n");
        let n = b.stmts.len();
        for (i, st) in b.stmts.iter().enumerate() {
            if i + 1 == n {
                if let S::Expr(e) = &st.kind {
                    s.push_str(&self.expr(cx, e)?);
                    s.push('\n');
                    continue;
                }
            }
            s.push_str(&self.stmt(cx, st)?);
        }
        s.push('}');
        Ok(s)
    }

    fn block_stmts(&self, cx: &Cx, b: &Block) -> R<String> {
        let mut s = String::new();
        for st in &b.stmts {
            s.push_str(&self.stmt(cx, st)?);
        }
        Ok(s)
    }

    fn let_target(&self, cx: &Cx, t: &LetTarget) -> String {
        match t {
            LetTarget::Local(id) if cx.locals[*id as usize].mutable => {
                format!("mut {}", self.local_name(cx, *id))
            }
            LetTarget::Local(id) => self.local_name(cx, *id),
            LetTarget::Discard => "_".into(),
            LetTarget::Tuple(items) => {
                let parts: Vec<String> = items.iter().map(|i| self.let_target(cx, i)).collect();
                if parts.len() == 1 {
                    format!("({},)", parts[0])
                } else {
                    format!("({})", parts.join(", "))
                }
            }
        }
    }

    fn stmt(&self, cx: &Cx, s: &Stmt) -> R<String> {
        Ok(match &s.kind {
            S::Let(t, Some(e)) => {
                format!("let {} = {};\n", self.let_target(cx, t), self.expr(cx, e)?)
            }
            S::Let(t, None) => {
                let LetTarget::Local(id) = t else {
                    return Err(GenError::new("an uninitialised `let` binds one local"));
                };
                format!(
                    "let {}: {};\n",
                    self.let_target(cx, t),
                    self.rty(&cx.locals[*id as usize].ty)?
                )
            }
            S::Expr(e) => match e.ty {
                Ty::Unit | Ty::Never => format!("{};\n", self.expr(cx, e)?),
                _ => format!("let _ = {};\n", self.expr(cx, e)?),
            },
            S::If(c, then_b, else_s) => {
                let mut t = format!(
                    "if {} {{\n{}}}",
                    self.expr(cx, c)?,
                    self.block_stmts(cx, then_b)?
                );
                if let Some(el) = else_s {
                    match &el.kind {
                        S::If(..) => {
                            let _ = write!(t, " else {}", self.stmt(cx, el)?);
                        }
                        S::Expr(Expr {
                            kind: H::Block(b), ..
                        }) => {
                            let _ = write!(t, " else {{\n{}}}\n", self.block_stmts(cx, b)?);
                        }
                        _ => {
                            let _ = write!(t, " else {{\n{}}}\n", self.stmt(cx, el)?);
                        }
                    }
                } else {
                    t.push('\n');
                }
                t
            }
            S::Match(scrut, arms) => {
                let mut t = format!("match {} {{\n", self.expr(cx, scrut)?);
                for arm in arms {
                    let pats = self.patterns(&arm.patterns, &scrut.ty)?;
                    let body = match &arm.body {
                        ArmBody::Block(b) => self.block_stmts(cx, b)?,
                        ArmBody::Expr(e) => format!("{};\n", self.expr(cx, e)?),
                    };
                    let _ = write!(t, "{pats} => {{\n{body}}}\n");
                }
                if self.needs_catch_all(arms, &scrut.ty) {
                    t.push_str("_ => unreachable!(\"value outside the type's encodings\"),\n");
                }
                t.push_str("}\n");
                t
            }
            S::For(id, lo, hi, body) => {
                let name = self.local_name(cx, *id);
                let ty = self.rty(&cx.locals[*id as usize].ty)?;
                // Inclusive: the loop variable's type holds `hi - 1`, which
                // may be its largest value (`for i in 0..256` over a `u8`).
                if hi <= lo {
                    String::new()
                } else {
                    format!(
                        "for {name} in ({lo} as {ty})..=({} as {ty}) {{\n{}}}\n",
                        hi - 1,
                        self.block_stmts(cx, body)?
                    )
                }
            }
            S::Return(Some(e)) if cx.opt => format!("return Some({});\n", self.expr(cx, e)?),
            S::Decline => "return None;\n".into(),
            S::Return(Some(e)) => {
                if cx.fallible {
                    format!("return Ok({});\n", self.expr(cx, e)?)
                } else {
                    format!("return {};\n", self.expr(cx, e)?)
                }
            }
            S::Return(None) => {
                if cx.fallible {
                    "return Ok(());\n".into()
                } else {
                    "return;\n".into()
                }
            }
            S::Assert(c, msg) => match msg {
                Some(m) => format!("assert!({}, {:?});\n", self.expr(cx, c)?, m),
                None => format!("assert!({});\n", self.expr(cx, c)?),
            },
            S::Unreachable(msg) => match msg {
                Some(m) => format!("unreachable!({m:?});\n"),
                None => "unreachable!();\n".into(),
            },
            S::Assign(place, v) => self.assign(cx, place, &self.expr(cx, v)?)?,
            S::OperandWrite(i, v) => {
                let instr = cx
                    .ops
                    .ok_or_else(|| GenError::new("operand outside an instruction"))?;
                let b = &instr.operands[*i as usize];
                let kind = self.kinds[&b.kind];
                let op = format!("ops.{}", ident(&b.name));
                // The value first: it may itself borrow the machine.
                let vt = self.expr(cx, v)?;
                if let Some(f) = self.form_of(*i).filter(|_| self.forms.get(&b.kind).is_some_and(|k| k.write_forms)) {
                    format!(
                        "{{ let v = {vt}; {}.kind_{}_write_f{f}({op}, v)?; }}\n",
                        cx.recv,
                        ident(&kind.name)
                    )
                } else if kind.source.is_some() {
                    format!(
                        "{{ let v = {vt}; {}.kind_{}_write({op}, v)?; }}\n",
                        cx.recv,
                        ident(&kind.name)
                    )
                } else {
                    format!(
                        "{{ let v = {vt}; {}.kind_{}_select_write({op}, v); }}\n",
                        cx.recv,
                        ident(&kind.name)
                    )
                }
            }
            S::SpaceWrite(d, bits, addr, v) => {
                let sp = self.spaces[d];
                format!(
                    "{{ let a = ({}) as u64; let v = ({}) as u128; {2}.accesses = {2}.accesses.wrapping_add(1); {4}{2}.bus.{3}_write(a, {bits}, v); }}\n",
                    self.expr(cx, addr)?,
                    self.expr(cx, v)?,
                    cx.recv,
                    ident(&sp.name),
                    self.split_charge(cx, *d, *bits)
                )
            }
            S::Raise(d, args) => {
                let f = self.faults[d];
                let a: R<Vec<String>> = args.iter().map(|x| self.expr(cx, x)).collect();
                let a = a?;
                if a.is_empty() {
                    format!("return Err(Fault::{});\n", ident(&f.name))
                } else {
                    format!("return Err(Fault::{}({}));\n", ident(&f.name), a.join(", "))
                }
            }
            S::Halt => format!("{}.halted = true;\n", cx.recv),
            S::Expect(_) | S::Given(_) | S::RunInstruction(..) | S::RunBytes(_) => {
                return Err(GenError::new("test statement outside a test"))
            }
            S::Emit {
                port,
                signal,
                args,
                value,
            } => {
                let c = cx
                    .comp
                    .ok_or_else(|| GenError::new("emit outside a component"))?;
                let mut a: Vec<String> = Vec::new();
                for x in args {
                    a.push(self.expr(cx, x)?);
                }
                a.push(self.expr(cx, value)?);
                format!(
                    "ctx.emit_{}_{}({});\n",
                    ident(&c.ports[*port as usize].name),
                    ident(&self.prog.item_name(*signal)),
                    a.join(", ")
                )
            }
            S::While(c, b) => format!(
                "while {} {{\n{}}}\n",
                self.expr(cx, c)?,
                self.block_stmts(cx, b)?
            ),
            S::Loop(b) => format!("loop {{\n{}}}\n", self.block_stmts(cx, b)?),
            S::Break => "break;\n".into(),
            S::Continue => "continue;\n".into(),
            S::Error => return Err(GenError::new("error statement")),
        })
    }

    /// Versioned state's bit, if `memo fn`s read state `i`.
    fn memo_var(&self, i: u32) -> Option<u32> {
        self.memo_cur.borrow().as_ref().and_then(|m| m.states.get(&i).copied())
    }

    /// Versioned memory's number, if `memo fn`s read mem `i`.
    fn memo_mem(&self, i: u32) -> Option<u8> {
        self.memo_cur.borrow().as_ref().and_then(|m| m.mems.get(&i).copied())
    }

    /// The place expression itself (never recorded as a read). An indexed
    /// array stays a place here (`base[i]`), where a read evaluates the
    /// index into a temporary first.
    fn lvalue(&self, cx: &Cx, e: &Expr) -> R<String> {
        let rec = self.memo_rec.replace(false);
        let r = match &e.kind {
            H::Index(base, idx) if matches!(base.ty, Ty::Array(..)) => {
                self.memo_rec.set(rec);
                let it = self.expr(cx, idx);
                self.memo_rec.set(false);
                let bt = self.lvalue(cx, base);
                match (bt, it) {
                    (Ok(b), Ok(i)) => Ok(format!("{b}[({i}) as usize]")),
                    (Err(x), _) | (_, Err(x)) => Err(x),
                }
            }
            H::Field(base, i) if Self::is_lvalue(base) => {
                let Ty::Bundle(d) = &base.ty else { unreachable!() };
                let bd = self.bundles[d];
                self.lvalue(cx, base).map(|b| format!("{b}.{}", ident(&bd.fields[*i as usize].name)))
            }
            _ => self.expr(cx, e),
        };
        self.memo_rec.set(rec);
        r
    }

    /// Statements that store `rhs` (an expression text) into `place`, and
    /// stamp versioned state with a new version (SPEC 8.3b).
    fn assign(&self, cx: &Cx, place: &Expr, rhs: &str) -> R<String> {
        let bump = self.trace_bump(cx, place);
        let s = self.assign_plain(cx, place, rhs)?;
        Ok(if bump.is_empty() { s } else { format!("{s}{bump}") })
    }

    fn assign_plain(&self, cx: &Cx, place: &Expr, rhs: &str) -> R<String> {
        fn root(e: &Expr) -> Option<&Expr> {
            match &e.kind {
                H::State(..) | H::ModelMem(..) => Some(e),
                H::Index(b, _) | H::Field(b, _) | H::TupleField(b, _) => root(b),
                _ => None,
            }
        }
        let r = cx.recv;
        match (&place.kind, root(place).map(|e| &e.kind)) {
            // A whole state: a new version only when the value changes.
            (H::State(_, i), _) if self.memo_var(*i).is_some() => {
                let k = self.memo_var(*i).unwrap();
                let p = self.lvalue(cx, place)?;
                Ok(format!("{{ let v = {rhs}; if {p} != v {{ let e = {r}.memo_bump(); {r}.memo_ver[{k}] = e; }} {p} = v; }}\n"))
            }
            (_, Some(H::State(_, i))) if self.memo_var(*i).is_some() => {
                let k = self.memo_var(*i).unwrap();
                let inner = self.assign_inner(cx, place, rhs)?;
                Ok(format!("{inner}{{ let e = {r}.memo_bump(); {r}.memo_ver[{k}] = e; }}\n"))
            }
            // A cell of a memory: its block.
            (H::Index(b, idx), Some(H::ModelMem(_, i))) if matches!(b.kind, H::ModelMem(..)) && self.memo_mem(*i).is_some() => {
                let m = self.memo_mem(*i).unwrap();
                Ok(format!(
                    "{{ let v = {rhs}; let i = ({}) as usize; {}[i] = v; let e = {r}.memo_bump(); {r}.memo_blocks[{m}][i >> rt::memo::BLOCK_SHIFT] = e; }}\n",
                    self.expr(cx, idx)?,
                    self.lvalue(cx, b)?
                ))
            }
            // A cell of a direct memory, with the trace core: its page's
            // write count (code pages are checked against it).
            (H::Index(b, idx), Some(H::ModelMem(_, i))) if matches!(b.kind, H::ModelMem(..)) && self.stamped(cx, *i).is_some() => {
                let m = self.stamped(cx, *i).unwrap();
                Ok(format!(
                    "{{ let v = {rhs}; let i = ({}) as usize; {}[i] = v; if {r}.{m}_code[i >> 12] & (1u32 << ((i >> 7) & 31)) != 0 {{ {r}.{m}_stamps[i >> 12] = {r}.{m}_stamps[i >> 12].wrapping_add(1); {r}.{m}_code[i >> 12] = 0; {r}.{m}_writes = {r}.{m}_writes.wrapping_add(1); }} }}\n",
                    self.expr(cx, idx)?,
                    self.lvalue(cx, b)?
                ))
            }
            (_, Some(H::ModelMem(_, i))) if self.stamped(cx, *i).is_some() => {
                let m = self.stamped(cx, *i).unwrap();
                let inner = self.assign_inner(cx, place, rhs)?;
                Ok(format!("{inner}for c in {r}.{m}_stamps.iter_mut() {{ *c = c.wrapping_add(1); }} for c in {r}.{m}_code.iter_mut() {{ *c = 0; }} {r}.{m}_writes = {r}.{m}_writes.wrapping_add(1);\n"))
            }
            (_, Some(H::ModelMem(_, i))) if self.memo_mem(*i).is_some() => {
                let m = self.memo_mem(*i).unwrap();
                let inner = self.assign_inner(cx, place, rhs)?;
                Ok(format!("{inner}{{ let e = {r}.memo_bump(); {r}.memo_all[{m}] = e; }}\n"))
            }
            _ => self.assign_inner(cx, place, rhs),
        }
    }

    /// With the trace core, a component's direct memory counts writes per
    /// page: its name, when memory `i` of the component in `cx` is one.
    fn stamped(&self, cx: &Cx, i: u32) -> Option<String> {
        self.trace.as_ref()?;
        let m = cx.comp?.model.as_ref()?.mems.get(i as usize)?;
        m.direct.as_ref().map(|_| ident(&m.name))
    }

    fn assign_inner(&self, cx: &Cx, place: &Expr, rhs: &str) -> R<String> {
        Ok(match &place.kind {
            H::Port(_, i, true) => {
                let c = cx
                    .comp
                    .ok_or_else(|| GenError::new("port outside a component"))?;
                format!(
                    "{{ let v = {rhs}; ctx.write_{}(v); }}\n",
                    ident(&c.ports[*i as usize].name)
                )
            }
            H::Index(b, idx) if matches!(b.ty, Ty::Array(..)) && Self::is_lvalue(place) => {
                format!(
                    "{{ let v = {rhs}; let i = ({}) as usize; {}[i] = v; }}\n",
                    self.expr(cx, idx)?,
                    self.lvalue(cx, b)?
                )
            }
            H::State(..) | H::Local(_) => format!("{} = {rhs};\n", self.lvalue(cx, place)?),
            H::Register(_) | H::Mode(_) | H::Latch(_) => {
                format!("{} = {rhs};\n", self.expr(cx, place)?)
            }
            H::Field(base, i) => {
                if Self::is_lvalue(base) {
                    let Ty::Bundle(d) = &base.ty else {
                        unreachable!()
                    };
                    let bd = self.bundles[d];
                    format!(
                        "{}.{} = {rhs};\n",
                        self.lvalue(cx, base)?,
                        ident(&bd.fields[*i as usize].name)
                    )
                } else {
                    let Ty::Bundle(d) = &base.ty else {
                        unreachable!()
                    };
                    let bd = self.bundles[d];
                    let read = self.expr(cx, base)?;
                    let inner = self.assign(cx, base, "tmp")?;
                    format!(
                        "{{ let mut tmp = {read}; tmp.{} = {rhs}; {inner}}}\n",
                        ident(&bd.fields[*i as usize].name)
                    )
                }
            }
            H::RegField(d, i) => self.reg_field_write(cx, *d, *i, rhs),
            H::RegView(d, i) => {
                let (_, r) = self.regs[d];
                let view = &r.views[*i as usize];
                let Ty::Bundle(bd) = &view.ty else {
                    unreachable!()
                };
                let bd = self.bundles[bd];
                let mut s = format!("{{ let v = {rhs};\n");
                for (f, ri) in bd.fields.iter().zip(&view.mapping) {
                    s.push_str(&self.reg_field_write(
                        cx,
                        *d,
                        *ri,
                        &format!("v.{}", ident(&f.name)),
                    ));
                }
                s.push_str("}\n");
                s
            }
            H::SpaceCell(d, addr) => {
                let sp = self.spaces[d];
                format!(
                    "{{ let a = ({}) as u64; let v = ({rhs}) as u128; {1}.accesses = {1}.accesses.wrapping_add(1); {1}.bus.{2}_write(a, {3}, v); }}\n",
                    self.expr(cx, addr)?,
                    cx.recv,
                    ident(&sp.name),
                    sp.cell_bits
                )
            }
            H::KindSelect(k, args) => {
                let kind = self.kinds[k];
                let a: R<Vec<String>> = args.iter().map(|x| self.expr(cx, x)).collect();
                let a = a?;
                let mut t = format!("{{ let v = {rhs}; ");
                for (i, arg) in a.iter().enumerate() {
                    let _ = write!(t, "let s{i} = {arg}; ");
                }
                let names: Vec<String> = (0..a.len()).map(|i| format!("s{i}")).collect();
                let _ = writeln!(
                    t,
                    "{}.kind_{}_select_write({}, v); }}",
                    cx.recv,
                    ident(&kind.name),
                    names.join(", ")
                );
                t
            }
            H::Match(scrut, arms) => {
                let mut t = format!("{{ let v = {rhs}; match {} {{\n", self.expr(cx, scrut)?);
                for arm in arms {
                    let pats = self.patterns(&arm.patterns, &scrut.ty)?;
                    let body = match &arm.body {
                        ArmBody::Expr(e) => self.assign(cx, e, "v")?,
                        ArmBody::Block(b) => self.assign_block(cx, b, "v")?,
                    };
                    let _ = write!(t, "{pats} => {{\n{body}}}\n");
                }
                if self.needs_catch_all(arms, &scrut.ty) {
                    t.push_str("_ => unreachable!(\"value outside the type's encodings\"),\n");
                }
                t.push_str("} }\n");
                t
            }
            H::If(c, then_b, Some(else_e)) => {
                format!(
                    "{{ let v = {rhs}; if {} {{\n{}}} else {{\n{}}} }}\n",
                    self.expr(cx, c)?,
                    self.assign_block(cx, then_b, "v")?,
                    self.assign(cx, else_e, "v")?
                )
            }
            H::Block(b) => format!("{{ let v = {rhs};\n{}}}\n", self.assign_block(cx, b, "v")?),
            _ => return Err(GenError::new("assignment to a non-place")),
        })
    }

    fn assign_block(&self, cx: &Cx, b: &Block, rhs: &str) -> R<String> {
        let mut s = String::new();
        let n = b.stmts.len();
        for (i, st) in b.stmts.iter().enumerate() {
            if i + 1 == n {
                if let S::Expr(e) = &st.kind {
                    s.push_str(&self.assign(cx, e, rhs)?);
                    continue;
                }
            }
            s.push_str(&self.stmt(cx, st)?);
        }
        Ok(s)
    }

    /// Is `e` a direct Rust lvalue (`self.r.X`, `self.r.CS.base`, `self.mode`)?
    fn is_lvalue(e: &Expr) -> bool {
        match &e.kind {
            H::Register(_) | H::Mode(_) | H::Latch(_) | H::State(..) | H::Local(_) => true,
            H::Index(b, _) => {
                matches!(b.kind, H::ModelMem(..))
                    || (Self::is_lvalue(b) && matches!(b.ty, Ty::Array(..)))
            }
            H::Field(b, _) => Self::is_lvalue(b),
            _ => false,
        }
    }

    fn reg_field_write(&self, cx: &Cx, d: DefId, i: u32, rhs: &str) -> String {
        let (_, r) = self.regs[&d];
        let f = &r.fields[i as usize];
        let (_, rty) = match &r.ty {
            Ty::UInt(n) => native(*n, false),
            _ => (8, "u8"),
        };
        let width = match &f.ty {
            Ty::UInt(n) | Ty::SInt(n) => *n,
            Ty::Bool => 1,
            Ty::Enum(e) => self.enums[e].width,
            _ => 8,
        };
        let mask = mask_literal(width, rty);
        let reg = format!("{}.r.{}", cx.recv, ident(&r.name));
        format!(
            "{{ let fv = (({rhs}) as {rty}) & {mask}; {reg} = ({reg} & !({mask} << {off})) | (fv << {off}); }}\n",
            off = f.offset
        )
    }

    /// `place` as a `rt::PlaceRef` expression, or `None` text when it is
    /// not an architectural place (memory, values).
    fn place_ref_text(&self, cx: &Cx, e: &Expr, optional: bool) -> R<String> {
        let some = |s: String| if optional { format!("Some({s})") } else { s };
        Ok(match &e.kind {
            H::Register(d) => some(format!("rt::PlaceRef::Reg({})", self.regs[d].0)),
            H::RegField(d, i) => some(format!("rt::PlaceRef::Field({}, {i})", self.regs[d].0)),
            H::RegView(d, i) => some(format!("rt::PlaceRef::View({}, {i})", self.regs[d].0)),
            H::Mode(_) => some("rt::PlaceRef::Mode".into()),
            H::KindSelect(k, args) => {
                let kind = self.kinds[k];
                let a: R<Vec<String>> = args.iter().map(|x| self.expr(cx, x)).collect();
                some(format!(
                    "{}.kind_{}_select_place({})",
                    cx.recv,
                    ident(&kind.name),
                    a?.join(", ")
                ))
            }
            H::Match(scrut, arms) => {
                let mut t = format!("(match {} {{\n", self.expr(cx, scrut)?);
                for arm in arms {
                    let pats = self.patterns(&arm.patterns, &scrut.ty)?;
                    let body = match &arm.body {
                        ArmBody::Expr(x) => self.place_ref_text(cx, x, optional)?,
                        ArmBody::Block(b) => self.place_block(cx, b, optional)?,
                    };
                    let _ = writeln!(t, "{pats} => {body},");
                }
                if self.needs_catch_all(arms, &scrut.ty) {
                    t.push_str("_ => unreachable!(\"value outside the type's encodings\"),\n");
                }
                t.push_str("})");
                t
            }
            H::If(c, then_b, Some(else_e)) => format!(
                "(if {} {} else {{ {} }})",
                self.expr(cx, c)?,
                self.place_block(cx, then_b, optional)?,
                self.place_ref_text(cx, else_e, optional)?
            ),
            H::Block(b) => self.place_block(cx, b, optional)?,
            _ if optional => "None".into(),
            _ => return Err(GenError::new("`select` body is not a place")),
        })
    }

    fn place_block(&self, cx: &Cx, b: &Block, optional: bool) -> R<String> {
        let mut s = String::from("{\n");
        let n = b.stmts.len();
        for (i, st) in b.stmts.iter().enumerate() {
            if i + 1 == n {
                if let S::Expr(e) = &st.kind {
                    s.push_str(&self.place_ref_text(cx, e, optional)?);
                    s.push('\n');
                    continue;
                }
            }
            s.push_str(&self.stmt(cx, st)?);
        }
        s.push('}');
        Ok(s)
    }

    // -- items ---------------------------------------------------------------------------

    fn emit_all(&mut self) -> R<()> {
        let isa_name = ident(&self.isa.name);
        self.line("// Generated by machine-model from a Machine isa description. Do not edit.");
        self.line("#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals, unused_variables, unused_mut, unused_parens, unused_imports, unused_braces, unused_labels, dead_code, unreachable_code, unreachable_patterns, unused_comparisons, unused_assignments, clippy::all)]");
        self.line("use machine_runtime as rt;");
        self.line("");
        self.emit_trace_types()?;
        self.forms = self.plan_forms();

        // Enums.
        for e in &self.hir.enums {
            let (_, t) = native(e.width, false);
            let mut s = format!("pub mod {} {{\n", ident(&e.name));
            for (n, v) in &e.variants {
                let _ = writeln!(s, "    pub const {}: {t} = {v};", ident(n));
            }
            s.push_str("}\n");
            self.out.push_str(&s);
        }
        // Bundles.
        if self.hir.bundles.iter().any(|b| b.name == "Schedule") {
            self.out.push_str(SCHEDULE_THEN);
        }
        for b in &self.hir.bundles {
            // Rust derives `Default` only for arrays of up to 32 elements.
            fn big(t: &Ty) -> bool {
                match t {
                    Ty::Array(e, n) => *n > 32 || big(e),
                    Ty::Tuple(items) => items.iter().any(|(_, t)| big(t)),
                    _ => false,
                }
            }
            let manual = b.fields.iter().any(|f| big(&f.ty));
            let mut s = format!(
                "#[derive(Clone, Copy, Debug, PartialEq{})]\npub struct {} {{\n",
                if manual { "" } else { ", Default" },
                ident(&b.name)
            );
            for f in &b.fields {
                let _ = writeln!(s, "    pub {}: {},", ident(&f.name), self.rty(&f.ty)?);
            }
            s.push_str("}\n");
            if manual {
                let mut init = String::new();
                for f in &b.fields {
                    let _ = write!(init, "{}: {}, ", ident(&f.name), self.default_value(&f.ty)?);
                }
                let _ = writeln!(
                    s,
                    "impl Default for {0} {{ fn default() -> {0} {{ {0} {{ {init}}} }} }}",
                    ident(&b.name)
                );
            }
            self.out.push_str(&s);
        }
        // Units.
        for (i, u) in self.hir.units.iter().enumerate() {
            let name = self.unit_fn_name(hir::UnitInstId(i as u32));
            let cx = Cx {
                locals: &u.locals,
                fallible: false,
                recv: "self",
                ops: None,
                comp: None,
                opt: false,
            };
            let mut params = Vec::new();
            for p in &u.params {
                params.push(format!(
                    "{}: {}",
                    self.local_name(&cx, *p),
                    self.rty(&u.locals[*p as usize].ty)?
                ));
            }
            let body = self.block_stmts(&cx, &u.body)?;
            let s = format!(
                "pub fn {name}({}) -> {} {{\n{body}}}\n",
                params.join(", "),
                self.rty(&u.ret)?
            );
            self.out.push_str(&s);
        }

        // Faults.
        let mut s = String::from("#[derive(Clone, Debug, PartialEq)]\npub enum Fault {\n");
        for f in &self.isa.faults {
            if f.params.is_empty() {
                let _ = writeln!(s, "    {},", ident(&f.name));
            } else {
                let tys: R<Vec<String>> = f.params.iter().map(|(_, t)| self.rty(t)).collect();
                let _ = writeln!(s, "    {}({}),", ident(&f.name), tys?.join(", "));
            }
        }
        s.push_str(
            "}\nimpl Fault {\n    pub fn class(&self) -> rt::FaultClass {\n        match self {\n",
        );
        for f in &self.isa.faults {
            let pat = if f.params.is_empty() {
                String::new()
            } else {
                "(..)".into()
            };
            let class = match f.class {
                hir::FaultClass::Fault => "Fault",
                hir::FaultClass::Trap => "Trap",
                hir::FaultClass::Abort => "Abort",
            };
            let _ = writeln!(
                s,
                "            Fault::{}{pat} => rt::FaultClass::{class},",
                ident(&f.name)
            );
        }
        if self.isa.faults.is_empty() {
            s.push_str("            _ => rt::FaultClass::Fault,\n");
        }
        // Whether a fault is contributory (two make a double fault).
        s.push_str("        }\n    }\n    /// A fault raised while delivering another contributory one makes a double fault.\n    pub fn contributory(&self) -> bool {\n        match self {\n");
        for f in &self.isa.faults {
            let pat = if f.params.is_empty() {
                String::new()
            } else {
                "(..)".into()
            };
            let _ = writeln!(
                s,
                "            Fault::{}{pat} => {},",
                ident(&f.name),
                f.contributory
            );
        }
        if self.isa.faults.is_empty() {
            s.push_str("            _ => false,\n");
        }
        // Whether a fault is the page fault (`page_fault: true`).
        s.push_str("        }\n    }\n    /// The page fault: it or a contributory fault while delivering it makes a double fault.\n    pub fn page_fault(&self) -> bool {\n        match self {\n");
        for f in &self.isa.faults {
            let pat = if f.params.is_empty() {
                String::new()
            } else {
                "(..)".into()
            };
            let _ = writeln!(
                s,
                "            Fault::{}{pat} => {},",
                ident(&f.name),
                f.page_fault
            );
        }
        if self.isa.faults.is_empty() {
            s.push_str("            _ => false,\n");
        }
        // The error code a fault pushes (`pushes_error_code`): its first
        // parameter, or 0 without one.
        s.push_str("        }\n    }\n    /// The error code this fault pushes, if it pushes one.\n    pub fn error_code(&self) -> Option<u16> {\n        match self {\n");
        for f in &self.isa.faults {
            let arm = match (f.pushes_error_code, f.params.is_empty()) {
                (false, true) => format!("            Fault::{} => None,", ident(&f.name)),
                (false, false) => format!("            Fault::{}(..) => None,", ident(&f.name)),
                (true, true) => format!("            Fault::{} => Some(0),", ident(&f.name)),
                (true, false) => {
                    let rest = if f.params.len() > 1 { ", .." } else { "" };
                    format!(
                        "            Fault::{}(e{rest}) => Some(*e as u16),",
                        ident(&f.name)
                    )
                }
            };
            let _ = writeln!(s, "{arm}");
        }
        if self.isa.faults.is_empty() {
            s.push_str("            _ => None,\n");
        }
        s.push_str("        }\n    }\n    /// The interrupt vector the isa assigns to this fault.\n    pub fn vector(&self) -> Option<u8> {\n        match self {\n");
        for f in &self.isa.faults {
            let pat = if f.params.is_empty() {
                String::new()
            } else {
                "(..)".into()
            };
            match f.vector {
                Some(v) => {
                    let _ = writeln!(
                        s,
                        "            Fault::{}{pat} => Some({v}),",
                        ident(&f.name)
                    );
                }
                None => {
                    let _ = writeln!(s, "            Fault::{}{pat} => None,", ident(&f.name));
                }
            }
        }
        if self.isa.faults.is_empty() {
            s.push_str("            _ => None,\n");
        }
        s.push_str("        }\n    }\n}\n");
        self.out.push_str(&s);

        for o in &self.isa.operands {
            self.emit_operand(o)?;
        }

        // Registers.
        let mut s =
            String::from("#[derive(Clone, Copy, Debug, PartialEq, Default)]\npub struct Regs {\n");
        for r in &self.isa.registers {
            let _ = writeln!(s, "    pub {}: {},", ident(&r.name), self.rty(&r.ty)?);
        }
        s.push_str("}\n");
        self.out.push_str(&s);

        // Instruction operand structs and the Instr enum.
        for i in &self.isa.instructions {
            self.emit_ops_struct(i)?;
        }
        let mut s = String::from("#[derive(Clone, Copy, Debug, PartialEq)]\npub enum Instr {\n");
        for i in self.isa.instructions.iter().filter(|i| !i.prefix) {
            let _ = writeln!(s, "    {}({}),", ident(&i.name), Self::ops_struct_name(i));
        }
        s.push_str("}\n");
        self.out.push_str(&s);

        // The machine.
        let mode_ty = match self.isa.mode {
            Some(m) => native(self.enums[&m].width, false).1,
            None => "u8",
        };
        // The bus: how the machine reaches its address spaces.
        let mut s = String::from("/// The isa's address spaces as the machine reaches them: plain memory\n/// ([`Flat`]) or a system's buses.\npub trait Bus {\n    /// Whether `retire` looks at its schedule: without, the interpreter\n    /// doesn't build one.\n    const SCHEDULE: bool = true;\n");
        for sp in &self.isa.spaces {
            let n = ident(&sp.name);
            let _ = writeln!(s, "    fn {n}_read(&mut self, addr: u64, bits: u32) -> u128;\n    fn {n}_write(&mut self, addr: u64, bits: u32, value: u128);");
        }
        s.push_str("    /// An instruction (or an interrupt taken at a boundary) finished, having spent\n    /// `cycles` of execution beyond its bus transfers, to `schedule`.\n    #[allow(unused_variables)]\n    fn retire(&mut self, cycles: u32, schedule: Schedule) {}\n    /// The processor shut down (a fault while delivering a double fault);\n    /// it stays halted until reset.\n    fn shutdown(&mut self) {}\n    /// The processor was reset alone (`istream { reset }`): the\n    /// processor model's state with it.\n    fn processor_reset(&mut self) {}\n    /// After each step, as `System::step` makes it: the devices caught up\n    /// (the JIT core's compiled steps make it themselves).\n    fn after_step(&mut self) {}\n    /// A halted processor's step: one cycle of its clock.\n    fn halted_cycle(&mut self) {}\n");
        if let Some(ist) = &self.isa.istream {
            let n = ident(&self.spaces[&ist.space].name);
            let _ = writeln!(s, "    /// An instruction-stream fetch (a code bus cycle); a read by default.\n    fn {n}_fetch(&mut self, addr: u64, bits: u32) -> u128 {{ self.{n}_read(addr, bits) }}");
            let _ = writeln!(s, "    /// The trace core: the page of code memory `addr` is in, when its writes\n    /// are counted; its write count; and the fetches of `n` bytes from\n    /// `addr` as the istream makes them, their values not needed.\n    #[allow(unused_variables)]\n    fn {n}_code_page(&mut self, addr: u64) -> Option<u32> {{ None }}\n    /// Writes to code memory so far, all pages.\n    fn {n}_code_writes(&mut self) -> u64 {{ 0 }}\n    /// `n` bytes at `addr` (in `page`) hold code: writes to them count.\n    #[allow(unused_variables)]\n    fn {n}_code_mark(&mut self, page: u32, addr: u64, n: u32) {{}}\n    #[allow(unused_variables)]\n    fn {n}_code_stamp(&mut self, page: u32) -> u32 {{ 0 }}\n    fn {n}_fetch_skip(&mut self, addr: u64, n: u32) {{ for i in 0..n {{ self.{n}_fetch(addr.wrapping_add(i as u64), 8); }} }}\n    /// The same, for bytes in code page `page`.\n    fn {n}_fetch_page(&mut self, page: u32, addr: u64, n: u32) {{ let _ = page; self.{n}_fetch_skip(addr, n) }}");
        }
        let mut ext_flat = String::new();
        for f in self.isa.fns.iter().filter(|f| f.external) {
            let cx = Cx {
                locals: &f.locals,
                fallible: false,
                recv: "self",
                ops: None,
                comp: None,
                opt: false,
            };
            let mut ps = String::new();
            for p in &f.params {
                let _ = write!(
                    ps,
                    ", {}: {}",
                    self.local_name(&cx, *p),
                    self.rty(&f.locals[*p as usize].ty)?
                );
            }
            let ret = self.rty(&f.ret)?;
            let _ = writeln!(s, "    /// `extern fn {}`: supplied by the processor model.\n    fn ext_{}(&mut self{ps}) -> {ret};", f.name, ident(&f.name));
            let _ = writeln!(
                ext_flat,
                "    fn ext_{}(&mut self{ps}) -> {ret} {{ Default::default() }}",
                ident(&f.name)
            );
        }
        s.push_str("}\n/// Every space as plain memory: the standalone interpreter, tests and vectors.\n#[derive(Clone, Debug)]\npub struct Flat {\n");
        for sp in &self.isa.spaces {
            let _ = writeln!(s, "    pub {}: rt::Memory,", ident(&sp.name));
        }
        s.push_str("}\nimpl Flat {\n    pub fn new() -> Flat {\n        Flat {\n");
        for sp in &self.isa.spaces {
            let _ = writeln!(
                s,
                "            {}: rt::Memory::new({}, {}),",
                ident(&sp.name),
                sp.cell_bits,
                sp.little_endian
            );
        }
        s.push_str("        }\n    }\n}\nimpl Default for Flat { fn default() -> Flat { Flat::new() } }\nimpl Bus for Flat {\n");
        for sp in &self.isa.spaces {
            let n = ident(&sp.name);
            let _ = writeln!(s, "    fn {n}_read(&mut self, addr: u64, bits: u32) -> u128 {{ self.{n}.read(addr, bits) }}\n    fn {n}_write(&mut self, addr: u64, bits: u32, value: u128) {{ self.{n}.write(addr, bits, value) }}");
        }
        s.push_str(&ext_flat);
        s.push_str("}\n");
        self.out.push_str(&s);

        let mut latch_fields = String::new();
        for l in &self.isa.latches {
            let _ = writeln!(latch_fields, "    pub latch_{}: {},", ident(&l.name), self.rty(&l.ty)?);
        }
        let (tr_hot, tr_cold) = self.trace_fields();
        latch_fields.push_str(tr_hot);
        let mut s = format!(
            "#[derive(Clone, Debug)]\n/// Laid out as declared (`repr(C)`): the latches and the per-step state\n/// first, then the registers, then the bus. Left to the compiler, the\n/// order shifted as fields were added, and with it the speed: five unused\n/// fields cost a fifth of the 386's.\n#[repr(C)]\npub struct {isa_name}<B: Bus = Flat> {{\n{latch_fields}    /// Bytes decoded this step, and the extra decoder clocks on each.\n    pub fetched: u8,\n    /// The istream's window this step (`istream {{ window ...; }}`): from\n    /// byte `win_from`, `win_len` bytes at consecutive addresses from\n    /// `win_base`.\n    pub win_from: u8,\n    pub win_len: u32,\n    pub win_base: u64,\n    /// Execution cycles of the last step beyond its bus transfers (the\n    /// timing table: the instruction, its prefixes, a delivered fault).\n    pub cycles: u32,\n    /// Prefix cycles accumulated while decoding.\n    pub prefix_cycles: u32,\n    /// Of `cycles`, the instruction's own timing-table entry.\n    pub entry_cycles: u32,\n    /// The decoder matched an instruction's opcode bytes during this step\n    /// (so an undefined-opcode fault is a real one, not a gap in the\n    /// description).\n    pub known_opcode: bool,\n    /// Set by `halt`; `step` does nothing while halted.\n    pub halted: bool,\n    /// This step decoded an instruction (`last` is it), and its schedule.\n    pub decoded: bool,\n    /// Data accesses made this step (a fault's delivery follows those the\n    /// instruction made).\n    pub accesses: u8,\n    /// Where the last step started, and whether this one started there too.\n    pub last_start: u64,\n    pub restarted: bool,\n    /// Held in reset (`istream {{ reset ...; }}`).\n    pub in_reset: bool,\n    /// Shut down: a fault while delivering a double fault.\n    pub shutdown: bool,\n    pub mode: {mode_ty},\n    pub decode_at: [u8; 16],\n    /// The last instruction decoded by `step`.\n    pub last: Option<Instr>,\n    /// A fault raised and delivered during the last `step`/`run`.\n    pub last_fault: Option<Fault>,\n"

        );
        s.push_str("    pub r: Regs,\n    pub run_sched: Schedule,\n");
        s.push_str(tr_cold);
        s.push_str(self.jit_fields());
        s.push_str("    pub bus: B,\n");
        s.push_str("}\n");
        self.out.push_str(&s);

        let mut s = format!(
            "impl {isa_name}<Flat> {{\n    /// A machine whose spaces are plain memory.\n    pub fn new() -> Self {{ Self::with_bus(Flat::new()) }}\n}}\nimpl<B: Bus> {isa_name}<B> {{\n    pub fn with_bus(bus: B) -> Self {{\n        let mut m = Self {{\n            r: Regs::default(),\n            mode: 0,\n            halted: false,\n            last: None,\n            known_opcode: false,\n            last_fault: None,\n            cycles: 0,\n            prefix_cycles: 0,\n            entry_cycles: 0,\n            last_start: u64::MAX,\n            restarted: false,\n            fetched: 0,\n            decode_at: [0; 16],\n            decoded: false,\n            in_reset: false,\n            run_sched: Schedule::default(),\n            win_from: 0,\n            win_len: 0,\n            win_base: 0,\n            accesses: 0,\n            shutdown: false,\n            bus,\n"
        );
        for l in &self.isa.latches {
            let _ = writeln!(
                s,
                "            latch_{}: Default::default(),",
                ident(&l.name)
            );
        }
        s.push_str(&self.trace_init());
        s.push_str(self.jit_init());
        s.push_str("        };\n        m.reset_latches();\n        m\n    }\n");
        // Reset: architectural state to zero, then the isa's `fn reset()`.
        let reset_fn = self
            .isa
            .fns
            .iter()
            .find(|f| f.name == "reset" && f.params.is_empty());
        let _ = write!(
            s,
            "    /// Processor reset: architectural state to zero{}.\n    pub fn reset(&mut self) {{\n        self.r = Regs::default();\n        self.mode = 0;\n        self.halted = false;\n        self.shutdown = false;\n        self.last = None;\n        self.last_fault = None;\n        self.reset_latches();\n{}    }}\n",
            if reset_fn.is_some() { ", then the isa's `fn reset()`" } else { "" },
            if reset_fn.is_some() { "        self.fn_reset().expect(\"fn reset() raised a fault\");\n" } else { "" }
        );
        s.push_str("    /// Per-instruction state back to its reset values.\n    pub fn reset_latches(&mut self) {\n");
        for l in &self.isa.latches {
            let cx = Cx {
                locals: &[],
                fallible: false,
                recv: "self",
                ops: None,
                comp: None,
                opt: false,
            };
            let _ = writeln!(
                s,
                "        self.latch_{} = {};",
                ident(&l.name),
                self.expr(&cx, &l.init)?
            );
        }
        s.push_str("    }\n");
        self.out.push_str(&s);

        for f in &self.isa.fns {
            self.emit_fn(f)?;
        }
        for k in &self.isa.kinds {
            self.emit_kind(k)?;
        }
        for i in &self.isa.instructions {
            self.emit_exec(i)?;
        }
        self.emit_decoder()?;
        self.emit_instr_info()?;
        self.line("}");
        self.line(&format!(
            "impl Default for {isa_name}<Flat> {{ fn default() -> Self {{ Self::new() }} }}"
        ));

        self.emit_tests()?;
        self.emit_vectors()?;
        Ok(())
    }

    fn emit_operand(&mut self, o: &hir::Operand) -> R<()> {
        let name = ident(&o.name);
        let tail_name = format!("{name}_Tail");
        let mut s = format!(
            "#[derive(Clone, Copy, Debug, PartialEq)]\npub enum {tail_name} {{\n    None,\n"
        );
        for t in &o.tail_set {
            let tn = self.prog.item_name(*t);
            let _ = writeln!(s, "    {tn}({tn}),", tn = ident(&tn));
        }
        s.push_str("}\n");
        let _ = writeln!(s, "impl {tail_name} {{");
        for t in &o.tail_set {
            let tn = ident(&self.prog.item_name(*t));
            let _ = writeln!(
                s,
                "    pub fn as_{tn}(&self) -> {tn} {{ match self {{ {tail_name}::{tn}(v) => *v, other => panic!(\"operand {name}: tail is {{other:?}}, not {tn}\") }} }}"
            );
        }
        s.push_str("}\n");

        let _ = writeln!(
            s,
            "#[derive(Clone, Copy, Debug, PartialEq)]\npub struct {name} {{"
        );
        for f in &o.fields {
            let _ = writeln!(s, "    pub {}: {},", ident(&f.name), self.rty(&f.ty)?);
        }
        let _ = writeln!(s, "    pub tail: {tail_name},\n}}");
        let _ = writeln!(s, "impl {name} {{\n    pub const SIZE: usize = {};", o.size);
        let _ = writeln!(
            s,
            "    pub fn from_bits(bits: u64) -> {name} {{\n        {name} {{"
        );
        for f in &o.fields {
            let (n, signed, t) = self
                .int_shape(&f.ty)
                .ok_or_else(|| GenError::new("operand field type"))?;
            let raw = format!("((bits >> {}) & {})", f.offset, mask_literal(n, "u64"));
            let v = if f.ty == Ty::Bool {
                format!("({raw} != 0)")
            } else if signed {
                let (w, _) = native(n, true);
                let k = w - n;
                format!("((({raw} as {t}) << {k}) >> {k})")
            } else {
                format!("({raw} as {t})")
            };
            let _ = writeln!(s, "            {}: {v},", ident(&f.name));
        }
        let _ = writeln!(
            s,
            "            tail: {tail_name}::None,\n        }}\n    }}"
        );
        let _ = writeln!(s, "    pub fn tail_sel(&self) -> u8 {{");
        let cx = Cx {
            locals: &o.tail_locals,
            fallible: false,
            recv: "self",
            ops: None,
            comp: None,
            opt: false,
        };
        for (i, f) in o.fields.iter().enumerate() {
            if i < o.tail_locals.len() {
                let _ = writeln!(
                    s,
                    "        let {} = self.{};",
                    self.local_name(&cx, i as u32),
                    ident(&f.name)
                );
            }
        }
        match &o.tail {
            Some(t) if tail_is_local(t, o.fields.len()) => {
                let _ = writeln!(s, "        {}", self.tail_expr(&cx, t, &o.tail_set)?);
            }
            // A tail that reads the machine (SPEC 11.2d) is the machine's
            // `tail_of_` method; here, for the encoders, there is none.
            _ => s.push_str("        0\n"),
        }
        s.push_str("    }\n}\n");
        self.out.push_str(&s);
        Ok(())
    }

    /// The operand `o`'s tail, chosen with the machine's state at hand:
    /// its fields, and `parent` when it is another operand's tail.
    fn emit_tail_of(&mut self, o: &hir::Operand) -> R<()> {
        let on = ident(&o.name);
        let cx = Cx {
            locals: &o.tail_locals,
            fallible: true,
            recv: "self",
            ops: None,
            comp: None,
            opt: false,
        };
        let parent = tail_parent(o).map(|p| ident(&self.prog.item_name(p)));
        let pparam = match &parent {
            Some(p) => format!(", parent: &{p}"),
            None => String::new(),
        };
        let mut s = format!("    #[allow(unused_variables)]\n    pub fn tail_of_{on}(&mut self, o: &{on}{pparam}) -> Result<u8, Fault> {{\n");
        for (i, f) in o.fields.iter().enumerate() {
            if i < o.tail_locals.len() {
                let _ = writeln!(
                    s,
                    "        let {} = o.{};",
                    self.local_name(&cx, i as u32),
                    ident(&f.name)
                );
            }
        }
        if parent.is_some() {
            let _ = writeln!(
                s,
                "        let {} = parent;",
                self.local_name(&cx, o.fields.len() as u32)
            );
        }
        match &o.tail {
            Some(t) => {
                let _ = writeln!(s, "        Ok({})", self.tail_expr(&cx, t, &o.tail_set)?);
            }
            None => s.push_str("        Ok(0)\n"),
        }
        s.push_str("    }\n");
        self.out.push_str(&s);
        Ok(())
    }

    fn tail_expr(&self, cx: &Cx, t: &hir::TailExpr, set: &[DefId]) -> R<String> {
        Ok(match t {
            hir::TailExpr::Operand(d) => {
                format!("{}u8", set.iter().position(|x| x == d).unwrap() + 1)
            }
            hir::TailExpr::If(c, a, b) => {
                let bt = match b {
                    Some(b) => self.tail_expr(cx, b, set)?,
                    None => "0u8".into(),
                };
                format!(
                    "if {} {{ {} }} else {{ {bt} }}",
                    self.expr(cx, c)?,
                    self.tail_expr(cx, a, set)?
                )
            }
        })
    }

    fn emit_fn(&mut self, f: &hir::FnBody) -> R<()> {
        if f.external {
            return Ok(());
        }
        let next = self.isa.istream.as_ref().is_some_and(|i| i.next == f.def);
        self.in_istream_next.set(next);
        let r = self.emit_fn_body(f);
        self.in_istream_next.set(false);
        r
    }

    fn emit_fn_body(&mut self, f: &hir::FnBody) -> R<()> {
        let cx = Cx {
            locals: &f.locals,
            fallible: true,
            recv: "self",
            ops: None,
            comp: None,
            opt: false,
        };
        let mut params = vec!["&mut self".to_string()];
        for p in &f.params {
            params.push(format!(
                "{}: {}",
                self.local_name(&cx, *p),
                self.rty(&f.locals[*p as usize].ty)?
            ));
        }
        let tail = if f.ret == Ty::Unit { "Ok(())\n" } else { "" };
        let name = ident(&f.name);
        let params = params.join(", ");
        let ret = self.rty(&f.ret)?;
        // Guards first: the leading `if`s that only leave the function
        // (`if CR0.PG == 0 { return lin; }`). When the rest is more than a
        // few statements, the guards go in an inlined wrapper and the rest
        // in a function of its own, so that the common early way out costs
        // no call.
        // Plain `let`s may come before and between them; the locals they
        // bind are passed on to the rest.
        let lead = f
            .body
            .stmts
            .iter()
            .take_while(|st| Self::is_guard(st) || matches!(st.kind, S::Let(LetTarget::Local(_), Some(_))))
            .count();
        // The head ends after the first run of guards: the cheap way out.
        let guards = match f.body.stmts[..lead].iter().position(Self::is_guard) {
            Some(i) => i + f.body.stmts[i..lead].iter().take_while(|st| Self::is_guard(st)).count(),
            None => 0,
        };
        let rest = &f.body.stmts[guards..];
        if guards > 0 && Self::weight(rest) > 4 {
            let mut params = params.clone();
            let mut bound: Vec<hir::LocalId> = f.params.clone();
            for st in &f.body.stmts[..guards] {
                if let S::Let(LetTarget::Local(id), _) = st.kind {
                    params.push_str(&format!(
                        ", {}: {}",
                        self.local_name(&cx, id),
                        self.rty(&cx.locals[id as usize].ty)?
                    ));
                    bound.push(id);
                }
            }
            let mut head = String::new();
            for st in &f.body.stmts[..guards] {
                head.push_str(&self.stmt(&cx, st)?);
            }
            let mut body = String::new();
            for st in rest {
                body.push_str(&self.stmt(&cx, st)?);
            }
            let args: Vec<String> = bound.iter().map(|p| self.local_name(&cx, *p)).collect();
            let outer = f.params.iter().map(|p| -> R<String> {
                Ok(format!("{}: {}", self.local_name(&cx, *p), self.rty(&cx.locals[*p as usize].ty)?))
            });
            let outer: Vec<String> = std::iter::once(Ok("&mut self".to_string())).chain(outer).collect::<R<_>>()?;
            let s = format!(
                "    #[inline]\n    pub fn fn_{name}({}) -> Result<{ret}, Fault> {{\n{head}self.fn_{name}_rest({})\n}}\n    #[inline(never)]\n    #[allow(clippy::too_many_arguments)]\n    fn fn_{name}_rest({params}) -> Result<{ret}, Fault> {{\n{body}{tail}}}\n",
                outer.join(", "),
                args.join(", ")
            );
            self.out.push_str(&s);
            return Ok(());
        }
        let body = self.block_stmts(&cx, &f.body)?;
        let inline = if Self::weight(&f.body.stmts) <= 3 { "    #[inline]\n" } else { "" };
        let s = format!("{inline}    pub fn fn_{name}({params}) -> Result<{ret}, Fault> {{\n{body}{tail}}}\n");
        self.out.push_str(&s);
        Ok(())
    }

    /// `if c { ...; return/raise }` with no else: a way out of the function.
    fn is_guard(st: &Stmt) -> bool {
        match &st.kind {
            S::If(_, b, None) => matches!(
                b.stmts.last().map(|s| &s.kind),
                Some(S::Return(_) | S::Raise(..))
            ),
            _ => false,
        }
    }

    /// Roughly how many statements run: nested blocks count their own.
    fn weight(stmts: &[Stmt]) -> usize {
        stmts
            .iter()
            .map(|st| match &st.kind {
                S::If(_, b, e) => {
                    1 + Self::weight(&b.stmts) + e.as_ref().map_or(0, |e| Self::weight(std::slice::from_ref(&**e)))
                }
                S::Match(_, arms) => 1 + 2 * arms.len(),
                S::For(_, _, _, b) => 8 + Self::weight(&b.stmts),
                _ => 1,
            })
            .sum()
    }

    fn rule_expr(k: &hir::OperandKind, rule: &hir::KindRule) -> Expr {
        match &rule.body {
            ArmBody::Expr(e) => e.clone(),
            ArmBody::Block(b) => Expr {
                kind: H::Block(b.clone()),
                ty: k.ty.clone(),
                span: b.span,
            },
        }
    }

    fn emit_kind(&mut self, k: &hir::OperandKind) -> R<()> {
        let kname = ident(&k.name);
        let vty = self.rty(&k.ty)?;
        for rule in &k.rules {
            let cx = Cx {
                locals: &rule.locals,
                fallible: true,
                recv: "self",
                ops: None,
                comp: None,
                opt: false,
            };
            let pure = Cx {
                fallible: false,
                ..cx
            };
            let mut params = Vec::new();
            for p in &rule.params {
                params.push(format!(
                    "{}: {}",
                    self.local_name(&cx, *p),
                    self.rty(&rule.locals[*p as usize].ty)?
                ));
            }
            let ps = params.join(", ");
            match rule.kind {
                hir::RuleKind::Select => {
                    let body_expr = Self::rule_expr(k, rule);
                    let read = self.expr(&pure, &body_expr)?;
                    let write = self.assign(&pure, &body_expr, "value")?;
                    let place = self.place_ref_text(&pure, &body_expr, false)?;
                    let _ = write!(
                        self.out,
                        "    pub fn kind_{kname}_select_read(&self, {ps}) -> {vty} {{\n{read}\n}}\n    pub fn kind_{kname}_select_write(&mut self, {ps}, value: {vty}) {{\n{write}}}\n    pub fn kind_{kname}_select_place(&self, {ps}) -> rt::PlaceRef {{\n{place}\n}}\n"
                    );
                    let (n, _, pt) = self
                        .int_shape(&rule.locals[rule.params[0] as usize].ty)
                        .ok_or_else(|| GenError::new("select param type"))?;
                    let _ = write!(
                        self.out,
                        "    pub fn find_{kname}_select(&self, target: rt::PlaceRef) -> {pt} {{\n        for n in 0..(1u64 << {n}) {{ let n = n as {pt}; if self.kind_{kname}_select_place(n) == target {{ return n; }} }}\n        panic!(\"operand kind {kname} cannot denote {{target:?}}\")\n    }}\n"
                    );
                }
                hir::RuleKind::Value => {
                    let body_expr = Self::rule_expr(k, rule);
                    let v = self.expr(&pure, &body_expr)?;
                    let (n, _, pt) = self
                        .int_shape(&rule.locals[rule.params[0] as usize].ty)
                        .ok_or_else(|| GenError::new("value param type"))?;
                    let _ = write!(
                        self.out,
                        "    pub fn kind_{kname}_value(&self, {ps}) -> {vty} {{\n{v}\n}}\n    pub fn find_{kname}_value(&self, target: {vty}) -> {pt} {{\n        for n in 0..(1u64 << {n}) {{ let n = n as {pt}; if self.kind_{kname}_value(n) == target {{ return n; }} }}\n        panic!(\"operand kind {kname} cannot carry {{target:?}}\")\n    }}\n"
                    );
                }
                hir::RuleKind::Read => {
                    let body = match &rule.body {
                        ArmBody::Expr(e) => format!("Ok({})\n", self.expr(&cx, e)?),
                        ArmBody::Block(b) => format!("Ok({})\n", self.block_expr(&cx, b)?),
                    };
                    let _ = write!(self.out, "    pub fn kind_{kname}_read(&mut self, {ps}) -> Result<{vty}, Fault> {{\n{body}}}\n");
                    let body_expr = Self::rule_expr(k, rule);
                    let place = self.place_ref_text(&cx, &body_expr, true)?;
                    let _ = write!(
                        self.out,
                        "    pub fn kind_{kname}_place_of(&mut self, {ps}) -> Result<Option<rt::PlaceRef>, Fault> {{\n        Ok({place})\n    }}\n"
                    );
                    if let Some(src) = k.source {
                        let on = ident(&self.prog.item_name(src));
                        let pname = self.local_name(&cx, rule.params[0]);
                        let _ = write!(
                            self.out,
                            "    pub fn find_{kname}_place(&mut self, target: rt::PlaceRef) -> Result<{on}, Fault> {{\n        for bits in 0..(1u64 << ({on}::SIZE * 8)) {{\n            let {pname} = {on}::from_bits(bits);\n            if {pname}.tail_sel() != 0 {{ continue; }}\n            if self.kind_{kname}_place_of({pname})? == Some(target) {{ return Ok({pname}); }}\n        }}\n        panic!(\"operand kind {kname} cannot address {{target:?}}\")\n    }}\n"
                        );
                        if self.is_pure_read(&rule.body) {
                            let _ = write!(
                                self.out,
                                "    pub fn find_{kname}_value(&mut self, value: {vty}) -> Result<{on}, Fault> {{\n        for bits in 0..(1u64 << ({on}::SIZE * 8)) {{\n            let {pname} = {on}::from_bits(bits);\n            if {pname}.tail_sel() != 0 {{ continue; }}\n            if self.kind_{kname}_read({pname})? == value {{ return Ok({pname}); }}\n        }}\n        panic!(\"operand kind {kname} cannot carry the value {{value:?}}\")\n    }}\n"
                            );
                        }
                    }
                }
                hir::RuleKind::Write => {
                    let body = match &rule.body {
                        ArmBody::Expr(e) => format!("{};\n", self.expr(&cx, e)?),
                        ArmBody::Block(b) => self.block_stmts(&cx, b)?,
                    };
                    let _ = write!(self.out, "    pub fn kind_{kname}_write(&mut self, {ps}) -> Result<(), Fault> {{\n{body}Ok(())\n}}\n");
                }
                hir::RuleKind::IsMem => {
                    let body = match &rule.body {
                        ArmBody::Expr(e) => format!("Ok({})\n", self.expr(&cx, e)?),
                        ArmBody::Block(b) => format!("Ok({})\n", self.block_expr(&cx, b)?),
                    };
                    let _ = write!(self.out, "    pub fn kind_{kname}_is_mem(&mut self, {ps}) -> Result<bool, Fault> {{\n{body}}}\n");
                }
                hir::RuleKind::Query => {
                    let body = match &rule.body {
                        ArmBody::Expr(e) => format!("Ok({})\n", self.expr(&cx, e)?),
                        ArmBody::Block(b) => format!("Ok({})\n", self.block_expr(&cx, b)?),
                    };
                    let _ = write!(self.out, "    pub fn kind_{kname}_{}(&mut self, {ps}) -> Result<{}, Fault> {{\n{body}}}\n", ident(&rule.name), self.rty(&rule.ty)?);
                }
            }
        }
        self.emit_kind_forms(k)
    }

    /// A `read` rule that touches no memory and calls no fn can be inverted
    /// by value.
    fn is_pure_read(&self, body: &ArmBody) -> bool {
        fn pure_expr(e: &Expr) -> bool {
            match &e.kind {
                H::SpaceRead(..) | H::SpaceCell(..) | H::FnCall(..) | H::SpaceWrite(..) => false,
                H::Unary(_, a)
                | H::Cast(a)
                | H::Field(a, _)
                | H::TupleField(a, _)
                | H::OperandField(a, _)
                | H::OperandTail(a)
                | H::TailOperand(a, _) => pure_expr(a),
                H::Binary(_, a, b) | H::Index(a, b) => pure_expr(a) && pure_expr(b),
                H::Slice(a, ..) => pure_expr(a),
                H::Ternary(a, b, c) => pure_expr(a) && pure_expr(b) && pure_expr(c),
                H::Call(_, args)
                | H::Builtin(_, args)
                | H::BundleLit(_, args)
                | H::Tuple(args)
                | H::Array(args)
                | H::KindSelect(_, args) => args.iter().all(pure_expr),
                H::ArrayRepeat(a, _) => pure_expr(a),
                H::Match(s, arms) => {
                    pure_expr(s)
                        && arms.iter().all(|a| match &a.body {
                            ArmBody::Expr(e) => pure_expr(e),
                            ArmBody::Block(b) => pure_block(b),
                        })
                }
                H::If(c, t, e) => {
                    pure_expr(c) && pure_block(t) && e.as_ref().is_none_or(|e| pure_expr(e))
                }
                H::Block(b) => pure_block(b),
                _ => true,
            }
        }
        fn pure_block(b: &Block) -> bool {
            b.stmts.iter().all(|s| match &s.kind {
                S::Let(_, Some(e)) | S::Expr(e) => pure_expr(e),
                S::Let(_, None) => true,
                S::Return(e) => e.as_ref().is_none_or(pure_expr),
                S::Match(s, arms) => {
                    pure_expr(s)
                        && arms.iter().all(|a| match &a.body {
                            ArmBody::Expr(e) => pure_expr(e),
                            ArmBody::Block(b) => pure_block(b),
                        })
                }
                _ => false,
            })
        }
        match body {
            ArmBody::Expr(e) => pure_expr(e),
            ArmBody::Block(b) => pure_block(b),
        }
    }

    fn ops_struct_name(i: &hir::Instruction) -> String {
        format!("Ops_{}", ident(&i.name))
    }

    /// The Rust type of an operand slot in an `Ops_*` struct.
    fn ops_field_type(&self, i: &hir::Instruction, b: &hir::OperandBinding) -> R<String> {
        Ok(match &b.source {
            hir::BindSource::From { .. } => {
                let kind = self.kinds[&b.kind];
                ident(&self.prog.item_name(kind.source.unwrap()))
            }
            hir::BindSource::Field { step, field } => match &i.encoding[*step as usize] {
                hir::EncodingStep::Fetch { operand, .. } => {
                    self.rty(&self.operands[operand].fields[*field as usize].ty)?
                }
                hir::EncodingStep::Opcode { fields, .. } => {
                    self.rty(&Ty::UInt(fields[*field as usize].width))?
                }
            },
        })
    }

    fn emit_ops_struct(&mut self, i: &hir::Instruction) -> R<()> {
        let mut s = format!(
            "#[derive(Clone, Copy, Debug, PartialEq)]\npub struct {} {{\n",
            Self::ops_struct_name(i)
        );
        for b in &i.operands {
            let _ = writeln!(
                s,
                "    pub {}: {},",
                ident(&b.name),
                self.ops_field_type(i, b)?
            );
        }
        s.push_str("}\n");
        self.out.push_str(&s);
        Ok(())
    }

    fn emit_exec(&mut self, i: &hir::Instruction) -> R<()> {
        let cx = Cx {
            locals: &i.locals,
            fallible: true,
            recv: "self",
            ops: Some(i),
            comp: None,
            opt: false,
        };
        let body = match &i.semantics {
            Some(b) => self.block_stmts(&cx, b)?,
            None => String::new(),
        };
        let _ = write!(
            self.out,
            "    /// {}\n    pub fn exec_{}(&mut self, ops: {}) -> Result<(), Fault> {{\n{body}Ok(())\n}}\n",
            i.description.join(" "),
            ident(&i.name),
            Self::ops_struct_name(i)
        );
        self.emit_exec_forms(i)
    }

    fn emit_decoder(&mut self) -> R<()> {
        let Some(istream) = &self.isa.istream else {
            if self.isa.instructions.is_empty() {
                return Ok(());
            }
            return Err(GenError::new(format!(
                "isa `{}` has instructions but no `istream`; add `istream {{ space ...; address ...; next ...; }}`",
                self.isa.name
            )));
        };
        let next = ident(&self.fns[&istream.next].name);
        let address = ident(&self.fns[&istream.address].name);
        let space = ident(&self.spaces[&istream.space].name);
        let cell = self.spaces[&istream.space].cell_bits;
        let undefined = match istream.undefined {
            Some(f) => format!("return Err(Fault::{})", ident(&self.faults[&f].name)),
            None => "panic!(\"undefined opcode\")".into(),
        };
        let _ = writeln!(
            self.out,
            "    pub fn istream_next(&mut self) -> Result<u8, Fault> {{ self.fetched = self.fetched.wrapping_add(1); Ok(self.fn_{next}()? as u8) }}"
        );
        if let Some(w) = istream.window {
            // Byte `fetched` of the step: within the window it is the
            // window's base plus how far the stream has come; past it the
            // address fn runs and opens a new window.
            let window = ident(&self.fns[&w].name);
            let addr_ty = self.rty(&self.fns[&istream.address].ret)?;
            let _ = writeln!(
                self.out,
                "    /// The istream's address through its window: translated once a window.\n    #[inline]\n    pub fn istream_address(&mut self) -> Result<{addr_ty}, Fault> {{\n        let k = self.fetched.wrapping_sub(self.win_from) as u32;\n        if self.fetched >= self.win_from && k < self.win_len {{\n            return Ok(self.win_base.wrapping_add(k as u64) as {addr_ty});\n        }}\n        let a = self.fn_{address}()?;\n        self.win_len = self.fn_{window}()?;\n        self.win_from = self.fetched;\n        self.win_base = a as u64;\n        Ok(a)\n    }}"
            );
        }

        // Operand fetchers.
        for o in &self.isa.operands {
            let on = ident(&o.name);
            let mut s = format!(
                "    pub fn fetch_{on}(&mut self{pparam}) -> Result<{on}, Fault> {{\n        let mut bits: u64 = 0;\n        for i in 0..{on}::SIZE {{ bits |= (self.istream_next()? as u64) << (8 * i); }}\n        let mut o = {on}::from_bits(bits);\n{mark}        match self.tail_of_{on}(&o{parg})? {{\n",
                pparam = match tail_parent(o) {
                    Some(p) => format!(", parent: &{}", ident(&self.prog.item_name(p))),
                    None => String::new(),
                },
                parg = if tail_parent(o).is_some() { ", parent" } else { "" },
                mark = if o.decode > 0 {
                    format!("        // `decode {on} => {0}`: the decoder spends {0} more clock(s) on its first byte.\n        if let Some(d) = self.decode_at.get_mut((self.fetched as usize).wrapping_sub({on}::SIZE as usize)) {{ *d = {0}; }}\n", o.decode)
                } else {
                    String::new()
                },
            );
            for (k, t) in o.tail_set.iter().enumerate() {
                let tn = ident(&self.prog.item_name(*t));
                let _ = writeln!(
                    s,
                    "            {} => {{ let t = self.fetch_{tn}({})?; o.tail = {on}_Tail::{tn}(t); }}",
                    k + 1,
                    if self.operands.get(t).is_some_and(|x| tail_parent(x).is_some()) { "&o" } else { "" }
                );
            }
            s.push_str("            _ => {}\n        }\n        Ok(o)\n    }\n");
            self.out.push_str(&s);
            self.emit_tail_of(o)?;
        }

        // A trie over concrete leading opcode bytes.
        #[derive(Default)]
        struct Node {
            children: std::collections::BTreeMap<u8, Node>,
            leaves: Vec<usize>,
        }
        fn insert(node: &mut Node, lead: &[hir::OpcodeByte], idx: usize) {
            match lead.split_first() {
                None => node.leaves.push(idx),
                Some((b, rest)) => {
                    for v in 0..=255u8 {
                        if v & b.mask == b.value {
                            insert(node.children.entry(v).or_default(), rest, idx);
                        }
                    }
                }
            }
        }
        let mut root = Node::default();
        for (idx, i) in self.isa.instructions.iter().enumerate() {
            let lead: Vec<hir::OpcodeByte> = i
                .encoding
                .iter()
                .take_while(|s| s.is_opcode())
                .flat_map(|s| match s {
                    hir::EncodingStep::Opcode { bytes, .. } => bytes.clone(),
                    _ => Vec::new(),
                })
                .collect();
            insert(&mut root, &lead, idx);
        }

        fn emit_node(g: &Gen, node: &Node, depth: usize, undefined: &str) -> R<String> {
            let mut out = String::new();
            if !node.leaves.is_empty() {
                out.push_str(&g.emit_leaf(&node.leaves, undefined)?);
                return Ok(out);
            }
            let _ = writeln!(
                out,
                "let b{depth} = self.istream_next()?;\nmatch b{depth} {{"
            );
            // Group bytes whose subtrees generate identical code.
            let mut groups: Vec<(String, Vec<u8>)> = Vec::new();
            for (byte, child) in &node.children {
                let code = emit_node(g, child, depth + 1, undefined)?;
                match groups.iter_mut().find(|(c, _)| *c == code) {
                    Some((_, bytes)) => bytes.push(*byte),
                    None => groups.push((code, vec![*byte])),
                }
            }
            for (code, bytes) in groups {
                let pats: Vec<String> = bytes.iter().map(|b| format!("{b:#04x}")).collect();
                let _ = write!(out, "{} => {{\n{code}}}\n", pats.join(" | "));
            }
            let _ = writeln!(out, "_ => {undefined},\n}}");
            Ok(out)
        }

        let body = emit_node(self, &root, 0, &undefined)?;
        let _ = write!(self.out, "    pub fn decode(&mut self) -> Result<Instr, Fault> {{\n'decode: loop {{\n{body}}}\n}}\n");
        // The same decoder, running each instruction where it is decoded
        // (`run_<instr>`): one dispatch instead of a decode and an exec.
        let mut run = String::new();
        for line in body.lines() {
            match line.strip_prefix("return Ok(Instr::").and_then(|l| l.strip_suffix("));")) {
                Some(rest) => {
                    let (name, ops) = rest.split_once('(').ok_or_else(|| GenError::new("decoder arm"))?;
                    let _ = writeln!(run, "return self.run_{name}({ops});");
                }
                None => {
                    run.push_str(line);
                    run.push('\n');
                }
            }
        }
        let _ = write!(self.out, "    /// Decodes and runs one instruction (prefixes included).\n    pub fn decode_run(&mut self) -> Result<(), Fault> {{\n'decode: loop {{\n{run}}}\n}}\n");
        self.emit_trace_fns()?;
        self.emit_jit_fns()?;

        let mut s = String::from(
            "    pub fn exec(&mut self, i: Instr) -> Result<(), Fault> {\n        match i {\n",
        );
        for i in self.isa.instructions.iter().filter(|i| !i.prefix) {
            let _ = writeln!(
                s,
                "            Instr::{n}(ops) => self.exec_{n}(ops),",
                n = ident(&i.name)
            );
        }
        if self.isa.instructions.iter().all(|i| i.prefix) {
            s.push_str("            _ => Ok(()),\n");
        }
        s.push_str("        }\n    }\n");
        let deliver = match (istream.fault, istream.double) {
            (Some(f), None) => {
                let args = if self.fns[&f].params.len() == 4 {
                    "v, restart, f.error_code().is_some(), f.error_code().unwrap_or(0)"
                } else {
                    "v, restart"
                };
                format!("if let Some(v) = f.vector() {{ let restart = f.class() == rt::FaultClass::Fault; self.fn_{}({args})?; self.cycles += self.fault_cycles(v); self.last_fault = Some(f); return Ok(()); }}", ident(&self.fns[&f].name))
            }
            // With a double fault: a contributory fault raised while
            // delivering a contributory one becomes the double fault, any
            // other is delivered in its place, and a fault while delivering
            // the double fault shuts the processor down.
            (Some(f), Some(df)) => {
                let args = if self.fns[&f].params.len() == 4 {
                    "v, restart, cur.error_code().is_some(), cur.error_code().unwrap_or(0)"
                } else {
                    "v, restart"
                };
                let dfn = ident(&self.faults[&df].name);
                format!("if let Some(v0) = f.vector() {{\n                    let mut cur = f;\n                    let mut v = v0;\n                    let mut tries = 0;\n                    loop {{\n                        let restart = cur.class() == rt::FaultClass::Fault;\n                        match self.fn_{}({args}) {{\n                            Ok(()) => {{ self.cycles += self.fault_cycles(v); self.last_fault = Some(cur); return Ok(()); }}\n                            Err(g) => {{\n                                tries += 1;\n                                if matches!(cur, Fault::{dfn}) || tries >= 3 {{\n                                    self.shutdown = true;\n                                    self.halted = true;\n                                    self.bus.shutdown();\n                                    self.last_fault = Some(cur);\n                                    return Ok(());\n                                }}\n                                cur = if (cur.contributory() && g.contributory()) || (cur.page_fault() && (g.contributory() || g.page_fault())) {{ Fault::{dfn} }} else {{ g }};\n                                v = match cur.vector() {{ Some(v) => v, None => return Err(cur) }};\n                            }}\n                        }}\n                    }}\n                }}", ident(&self.fns[&f].name))
            }
            _ => String::new(),
        };
        let _ = write!(
            s,
            "    /// After an instruction: a fault with a vector is delivered through the isa's `fault` fn,\n    /// told whether the fault restarts the instruction. The isa, not the interpreter, decides\n    /// what restarting means (state is never rolled back implicitly).\n    fn finish(&mut self, r: Result<(), Fault>) -> Result<(), Fault> {{\n        match r {{\n            Ok(()) => Ok(()),\n            Err(f) => {{\n                {deliver}\n                Err(f)\n            }}\n        }}\n    }}\n"
        );
        s.push_str(
            "    /// Execute one decoded instruction (an instruction boundary: latches reset) with fault delivery.\n    pub fn run(&mut self, i: Instr) -> Result<(), Fault> {\n        self.reset_latches();\n        self.last_fault = None;\n        let r = self.exec(i);\n        self.finish(r)\n    }\n",
        );
        let boundary = match istream.boundary {
            Some(b) => format!(
                "        // The isa's boundary fn may take an interrupt in place of the\n        // next instruction (and wakes a halted processor when it does).\n        match self.fn_{}() {{\n            Ok(true) => {{ self.halted = false; self.cycles = self.interrupt_cycles(); let mut s = Schedule::default(); if B::SCHEDULE {{ s = self.interrupt_schedule(); s.start = self.fn_{1}().map(|a| a as u64).unwrap_or(u64::MAX); s.next = s.start; s.room = {2}; }} self.bus.retire(self.cycles, s); return Ok(()); }}\n            Ok(false) => {{}}\n            Err(f) => return self.finish(Err(f)),\n        }}\n",
                ident(&self.fns[&b].name),
                ident(&self.fns[&istream.address].name),
                match istream.limit {
                    Some(f) => format!("self.fn_{}().unwrap_or(u32::MAX)", ident(&self.fns[&f].name)),
                    None => "u32::MAX".to_string(),
                }
            ),
            None => String::new(),
        };
        // A lock prefix before an instruction that may not take one.
        // How far the stream reaches past where execution continues.
        let room = match istream.limit {
            Some(f) => format!(
                "        sched.room = self.fn_{}().unwrap_or(u32::MAX);\n",
                ident(&self.fns[&f].name)
            ),
            None => "        sched.room = u32::MAX;\n".to_string(),
        };
        // A delivered fault: its `schedules` entry after the accesses the
        // instruction made, else its cycles as work after the last step.
        let deliver_schedule = if self.isa.fault_schedule.is_some()
            || self
                .isa
                .instructions
                .iter()
                .any(|i| i.fault_schedule.is_some())
        {
            "        if let Some(v) = self.last_fault.as_ref().and_then(|f| f.vector()) {\n            let f = self.fault_schedule_for(cur, v);\n            sched = schedule_then(sched, performed, f);\n        }\n".to_string()
        } else {
            "        let _ = (performed, cur);\n        let k = (sched.steps as usize).min(8);\n        sched.work[k] = sched.work[k].saturating_add(self.cycles.saturating_sub(before) as u16);\n".to_string()
        };
        // `istream { reset }`: while the line is up the processor is held
        // in reset; when it falls it starts as after reset.
        let reset_check = match istream.reset {
            Some(f) => {
                let fd = &self.fns[&f];
                let call = if fd.external {
                    format!("self.bus.ext_{}()", ident(&fd.name))
                } else {
                    format!("self.fn_{}().unwrap_or(false)", ident(&fd.name))
                };
                format!("        if {call} {{\n            if !self.in_reset {{ self.reset(); self.bus.processor_reset(); self.in_reset = true; }}\n            self.halted = true;\n            return Ok(());\n        }}\n        if self.in_reset {{ self.in_reset = false; self.halted = false; }}\n")
            }
            None => String::new(),
        };
        // The stream's first byte is at `start`: the window opens there.
        let seed = match istream.window {
            Some(w) => format!(
                "        if start != u64::MAX {{ self.win_from = 1; self.win_base = start; self.win_len = self.fn_{}().unwrap_or(0); }}\n",
                ident(&self.fns[&w].name)
            ),
            None => String::new(),
        };
        // Without a `schedules` table an instruction's schedule is its
        // timing entry as work: computed once.
        // Where the instruction is, and how it runs: decoded, or (the trace
        // core) the next of a run being replayed, or the first of a run kept
        // here, else decoded and recorded.
        let (find, _run) = if self.trace.is_some() {
            (
                format!("        self.tr_rec_now = false;\n        self.decoded = false;\n        let mut done = if self.tr_on {{ self.trace_go() }} else {{ None }};\n        if done.is_none() && self.tr_prev != u32::MAX {{ done = self.trace_follow(); }}\n        let (start, r) = match done {{\n            Some(x) => x,\n            None => {{\n        let start = self.fn_{address}().map(|a| a as u64).unwrap_or(u64::MAX);\n{seed}        let entered = if start != u64::MAX {{ self.trace_enter(start) }} else {{ None }};\n        match entered {{\n            Some(x) => x,\n            None => {{\n        if start != u64::MAX {{ self.trace_note(start); }}\n        self.restarted = start == self.last_start;\n        self.last_start = start;\n        (start, self.decode_run())\n            }}\n        }}\n            }}\n        }};\n        let mut sched = Schedule::default();\n"),
                String::new(),
            )
        } else {
            (
                format!("        let start = self.fn_{address}().map(|a| a as u64).unwrap_or(u64::MAX);\n{seed}        self.restarted = start == self.last_start;\n        self.last_start = start;\n        let mut sched = Schedule::default();\n        self.decoded = false;\n        let r = self.decode_run();\n"),
                String::new(),
            )
        };
        let entry = if self.isa.instructions.iter().any(|i| i.schedule.is_some()) {
            "sched = self.run_sched;"
        } else {
            "sched.work[0] = self.entry_cycles as u16;"
        };
        let prologue = format!("        self.reset_latches();\n        self.known_opcode = false;\n        self.last_fault = None;\n        self.cycles = 0;\n        self.prefix_cycles = 0;\n        self.entry_cycles = 0;\n        self.fetched = 0;\n        self.win_len = 0;\n        self.decode_at = [0; 16];\n        self.accesses = 0;\n{reset_check}        if self.shutdown {{ return Ok(()); }}\n{boundary}        if self.halted {{ return Ok(()); }}\n");
        let tail = format!("        let cur: Option<Instr> = if self.decoded {{ self.last }} else {{ None }};\n        if self.decoded {{ {entry} }}\n        let before = self.cycles;\n        let performed = self.accesses;\n        let r = self.finish(r);\n        if B::SCHEDULE {{\n{deliver_schedule}        sched.length = self.fetched;\n        sched.decode = self.decode_at;\n        sched.start = start;\n        sched.next = self.fn_{address}().map(|a| a as u64).unwrap_or(u64::MAX);\n{room}        sched.halted = self.halted;\n        }} else {{\n        let _ = (before, performed, cur);\n        }}\n        self.bus.retire(self.cycles, sched);\n        r\n");
        s.push_str(&format!(
            "    /// Decode and execute one instruction from the instruction stream.\n    pub fn step(&mut self) -> Result<(), Fault> {{\n{prologue}{find}{tail}    }}\n"
        ));
        // The JIT core: the step's prologue and tail, for a compiled
        // instruction (the prologue sets `jit_go` when it leaves the step to
        // the instruction).
        if self.jit {
            s.push_str(&format!(
                "    /// `step`'s prologue, for the JIT core: `jit_go` when the instruction is to run.\n    #[inline]\n    fn jit_prologue(&mut self) -> Result<(), Fault> {{\n{prologue}        self.jit_go = true;\n        Ok(())\n    }}\n    /// `step`'s tail, for the JIT core.\n    #[inline]\n    fn jit_tail(&mut self, start: u64, r: Result<(), Fault>) -> Result<(), Fault> {{\n        let mut sched = Schedule::default();\n{tail}    }}\n"
            ));
        }
        let _ = write!(
            s,
            "    /// Place `bytes` at the instruction stream address and execute one instruction.\n    pub fn run_bytes(&mut self, bytes: &[u8]) -> Result<(), Fault> {{\n        let a = self.fn_{address}()? as u64;\n        for (i, b) in bytes.iter().enumerate() {{ self.bus.{space}_write(a + i as u64, {cell}, *b as u128); }}\n        self.step()\n    }}\n"
        );
        self.out.push_str(&s);
        Ok(())
    }

    /// Code for a trie leaf: one instruction, or several told apart by
    /// constraints on the operand they all fetch first.
    fn emit_leaf(&self, leaves: &[usize], undefined: &str) -> R<String> {
        // Decode conditions (SPEC 11.2d): test them first, in declaration
        // order, then decode within the group whose condition holds.
        if leaves
            .iter()
            .any(|&i| self.isa.instructions[i].when.is_some())
        {
            let mut groups: Vec<(Option<String>, Vec<usize>)> = Vec::new();
            for &i in leaves {
                let ins = &self.isa.instructions[i];
                let cond = match &ins.when {
                    Some(w) => {
                        let cx = Cx {
                            locals: &ins.when_locals,
                            fallible: true,
                            recv: "self",
                            ops: None,
                            comp: None,
                            opt: false,
                        };
                        Some(self.expr(&cx, w)?)
                    }
                    None => None,
                };
                match groups.iter_mut().find(|(c, _)| *c == cond) {
                    Some((_, g)) => g.push(i),
                    None => groups.push((cond, vec![i])),
                }
            }
            let mut chain = Vec::new();
            let mut otherwise = None;
            for (cond, g) in groups {
                let body = self.emit_leaf_plain(&g, undefined)?;
                match cond {
                    Some(c) => chain.push(format!("if {c} {{\n{body}}}")),
                    None => otherwise = Some(body),
                }
            }
            let tail = otherwise.unwrap_or_else(|| format!("{undefined}\n"));
            return Ok(format!("{} else {{\n{tail}}}\n", chain.join(" else ")));
        }
        self.emit_leaf_plain(leaves, undefined)
    }

    /// A leaf's instructions told apart by their first operand's field
    /// constraints.
    fn emit_leaf_plain(&self, leaves: &[usize], undefined: &str) -> R<String> {
        let first = &self.isa.instructions[leaves[0]];
        // A prefix is not an instruction: the opcode that follows decides.
        let mut out = if first.prefix {
            String::new()
        } else {
            String::from("self.known_opcode = true;\n")
        };
        let first_fetch = first.encoding.iter().position(|s| !s.is_opcode());
        let constrained = leaves.iter().any(|i| {
            self.isa.instructions[*i]
                .encoding
                .iter()
                .any(|s| matches!(s, hir::EncodingStep::Fetch { constraints, .. } if !constraints.is_empty()))
        });
        if leaves.len() == 1 && !constrained {
            out.push_str(&self.emit_instr_tail(first, None)?);
            return Ok(out);
        }
        let Some(ff) = first_fetch else {
            return Err(GenError::new(format!(
                "instruction `{}` shares its opcode but fetches no operand",
                first.name
            )));
        };
        let hir::EncodingStep::Fetch { operand, .. } = &first.encoding[ff] else {
            unreachable!()
        };
        let _ = writeln!(
            out,
            "let f{ff} = self.fetch_{}()?;",
            ident(&self.prog.item_name(*operand))
        );
        let o = self.operands[operand];
        let mut chain = Vec::new();
        for &idx in leaves {
            let i = &self.isa.instructions[idx];
            let pos = i.encoding.iter().position(|s| !s.is_opcode()).unwrap();
            let hir::EncodingStep::Fetch { constraints, .. } = &i.encoding[pos] else {
                unreachable!()
            };
            let conds: Vec<String> = constraints
                .iter()
                .map(|(f, v)| {
                    format!(
                        "(f{ff}.{} as i128) == {v}",
                        ident(&o.fields[*f as usize].name)
                    )
                })
                .collect();
            let cond = if conds.is_empty() {
                "true".to_string()
            } else {
                conds.join(" && ")
            };
            chain.push(format!(
                "if {cond} {{\n{}}}",
                self.emit_instr_tail(i, Some((pos, format!("f{ff}"))))?
            ));
        }
        let _ = writeln!(out, "{} else {{ {undefined} }}", chain.join(" else "));
        Ok(out)
    }

    /// Fetch the remaining operands of `i` and build its `Instr`, or run a
    /// prefix and continue decoding.
    fn emit_instr_tail(
        &self,
        i: &hir::Instruction,
        prefetched: Option<(usize, String)>,
    ) -> R<String> {
        let mut out = String::new();
        let mut fetched: Vec<Option<String>> = vec![None; i.encoding.len()];
        let mut opcode_base: Vec<u32> = vec![0; i.encoding.len()];
        let mut lead = 0u32;
        for (si, step) in i.encoding.iter().enumerate() {
            match step {
                hir::EncodingStep::Opcode { bytes, .. } => {
                    opcode_base[si] = lead;
                    lead += bytes.len() as u32;
                }
                hir::EncodingStep::Fetch { operand, .. } => {
                    if let Some((p, var)) = &prefetched {
                        if *p == si {
                            fetched[si] = Some(var.clone());
                            continue;
                        }
                    }
                    let var = format!("f{si}");
                    let _ = writeln!(
                        out,
                        "let {var} = self.fetch_{}()?;",
                        ident(&self.prog.item_name(*operand))
                    );
                    fetched[si] = Some(var);
                }
            }
        }
        let mut fields = Vec::new();
        for b in &i.operands {
            let v = match &b.source {
                hir::BindSource::From { step } => fetched[*step as usize].clone().unwrap(),
                hir::BindSource::Field { step, field } => match &i.encoding[*step as usize] {
                    hir::EncodingStep::Fetch { operand, .. } => {
                        let o = self.operands[operand];
                        format!(
                            "{}.{}",
                            fetched[*step as usize].clone().unwrap(),
                            ident(&o.fields[*field as usize].name)
                        )
                    }
                    hir::EncodingStep::Opcode { fields: of, .. } => {
                        let f = &of[*field as usize];
                        let g = opcode_base[*step as usize] + f.byte;
                        let t = self.rty(&Ty::UInt(f.width))?;
                        format!(
                            "((b{g} >> {}) & {}) as {t}",
                            f.shift,
                            mask_literal(f.width, "u8")
                        )
                    }
                },
            };
            fields.push(format!("{}: {v}", ident(&b.name)));
        }
        let ops = format!("{} {{ {} }}", Self::ops_struct_name(i), fields.join(", "));
        if i.prefix {
            let cost = match &i.timing {
                Some(t) => {
                    let cx = Cx {
                        locals: &i.timing_locals,
                        fallible: true,
                        recv: "self",
                        ops: Some(i),
                        comp: None,
                        opt: false,
                    };
                    format!("self.prefix_cycles += ({}) as u32;\n", self.expr(&cx, t)?)
                }
                None => String::new(),
            };
            let _ = writeln!(
                out,
                "let ops = {ops};\n{cost}self.exec_{}(ops)?;\ncontinue 'decode;",
                ident(&i.name)
            );
        } else {
            let _ = writeln!(out, "return Ok(Instr::{}({ops}));", ident(&i.name));
        }
        Ok(out)
    }

    /// `undefined_mask`, `mnemonic` and `name` per instruction.
    fn emit_instr_info(&mut self) -> R<()> {
        let instrs: Vec<&hir::Instruction> =
            self.isa.instructions.iter().filter(|i| !i.prefix).collect();
        // undefined_mask
        let mut s = String::from(
            "    /// Bits of register `reg` (declaration index) that the instruction leaves undefined.\n    pub fn undefined_mask(&mut self, i: &Instr, reg: usize) -> u128 {\n        let r: Result<u128, Fault> = (|| { let mut m: u128 = 0; match *i {\n",
        );
        for i in &instrs {
            let cx = Cx {
                locals: &i.flags.locals,
                fallible: true,
                recv: "self",
                ops: Some(i),
                comp: None,
                opt: false,
            };
            let mut arm = String::new();
            for f in &i.flags.undefined {
                let (ri, r) = self.regs[&f.reg];
                let field = &r.fields[f.field as usize];
                let width = self.width_of_field(&field.ty);
                let mask = format!("({}u128 << {})", (1u128 << width) - 1, field.offset);
                match &f.cond {
                    Some(c) => {
                        let _ = writeln!(
                            arm,
                            "if reg == {ri} && {} {{ m |= {mask}; }}",
                            self.expr(&cx, c)?
                        );
                    }
                    None => {
                        let _ = writeln!(arm, "if reg == {ri} {{ m |= {mask}; }}");
                    }
                }
            }
            let _ = writeln!(
                s,
                "            Instr::{}(ops) => {{\n{arm}}}",
                ident(&i.name)
            );
        }
        if instrs.is_empty() {
            s.push_str("            _ => {}\n");
        }
        s.push_str("        } Ok(m) })();\n        r.unwrap_or(0)\n    }\n");

        // timing
        s.push_str("    /// Execution cycles of `i` beyond its bus transfers (its timing entry, on the state\n    /// before it runs; 0 without one).\n    pub fn timing_of(&mut self, i: &Instr) -> u32 {\n        let r: Result<u32, Fault> = (|| Ok(match *i {\n");
        let mut sched = String::from("    /// The schedule of `i` (its timing entry, on the state before it runs; a plain\n    /// count is all work before any step).\n    pub fn schedule_of(&mut self, i: &Instr) -> Schedule {\n        let r: Result<Schedule, Fault> = (|| Ok(match *i {\n");
        let any_schedule = instrs.iter().any(|i| i.schedule.is_some());
        let lock_part = match self.isa.istream.as_ref().map(|i| (i.lock, i.undefined)) {
            Some((Some(l), Some(u))) => format!(
                "        if self.latch_{} == 1 && !self.lock_ok(&i) {{ return Err(Fault::{}); }}\n",
                ident(&self.prog.item_name(l)),
                ident(&self.faults[&u].name)
            ),
            _ => String::new(),
        };
        let mut run_fns = String::new();
        let mut has = String::from("    /// Whether `i` has a timing entry.\n    pub fn has_timing(i: &Instr) -> bool {\n        match i {\n");
        let mut feats = String::from("    /// For fitting timing: whether each memory-capable operand of `i` is in\n    /// memory, and its address work (an `ea` query) when it has one.\n    pub fn timing_features(&mut self, i: &Instr) -> Vec<(&'static str, i64)> {\n        match *i {\n");
        for i in &instrs {
            let text = match &i.timing {
                Some(t) => {
                    let cx = Cx {
                        locals: &i.timing_locals,
                        fallible: true,
                        recv: "self",
                        ops: Some(i),
                        comp: None,
                        opt: false,
                    };
                    format!("({}) as u32", self.expr(&cx, t)?)
                }
                None => "0".into(),
            };
            // Its schedule: the `schedules` entry, else its cycles as work.
            let stext = match &i.schedule {
                Some(t) => {
                    let cx = Cx {
                        locals: &i.schedule_locals,
                        fallible: true,
                        recv: "self",
                        ops: Some(i),
                        comp: None,
                        opt: false,
                    };
                    let e = self.expr(&cx, t)?;
                    if matches!(t.ty, Ty::Bundle(_)) {
                        e
                    } else {
                        format!(
                            "{{ let mut s = Schedule::default(); s.work[0] = ({e}) as u16; s }}"
                        )
                    }
                }
                None if i.timing.is_some() => {
                    format!("{{ let mut s = Schedule::default(); s.work[0] = ({text}) as u16; s }}")
                }
                None => "Schedule::default()".into(),
            };
            let _ = writeln!(
                sched,
                "            Instr::{}(ops) => {stext},",
                ident(&i.name)
            );
            let _ = writeln!(s, "            Instr::{}(ops) => {text},", ident(&i.name));
            {
                let n = ident(&i.name);
                let ops_ty = Self::ops_struct_name(i);
                let sched_part = if any_schedule {
                    "        if B::SCHEDULE { self.run_sched = self.schedule_of(&i); }\n"
                } else {
                    ""
                };
                let call = self.exec_call(i);
                let rec = if self.trace.is_some() { "        if self.tr_rec_now { self.trace_record(i); }\n" } else { "" };
                let _ = write!(
                    run_fns,
                    "    #[inline]\n    #[allow(unused_variables, clippy::redundant_closure_call)]\n    fn run_{n}(&mut self, ops: {ops_ty}) -> Result<(), Fault> {{\n        let i = Instr::{n}(ops);\n        self.last = Some(i);\n        self.decoded = true;\n{rec}{sched_part}        self.entry_cycles = {{ let r: Result<u32, Fault> = (|| Ok({text}))(); r.unwrap_or(0) }};\n        self.cycles = self.entry_cycles + self.prefix_cycles;\n{lock_part}        {call}\n    }}\n"
                );
            }
            let _ = writeln!(
                has,
                "            Instr::{}(_) => {},",
                ident(&i.name),
                i.timing.is_some()
            );
            let mut fs = Vec::new();
            for b in &i.operands {
                let kind = self.kinds[&b.kind];
                if kind.rules.iter().any(|r| r.kind == hir::RuleKind::IsMem) {
                    fs.push(format!(
                        "({:?}, self.kind_{}_is_mem(ops.{}).unwrap_or(false) as i64)",
                        format!("{}.is_mem", b.name),
                        ident(&kind.name),
                        ident(&b.name)
                    ));
                    if kind
                        .rules
                        .iter()
                        .any(|r| r.kind == hir::RuleKind::Query && r.name == "ea")
                    {
                        fs.push(format!(
                            "({:?}, if self.kind_{1}_is_mem(ops.{2}).unwrap_or(false) {{ self.kind_{1}_ea(ops.{2}).map(|v| v as i64).unwrap_or(0) }} else {{ 0 }})",
                            format!("{}.ea", b.name),
                            ident(&kind.name),
                            ident(&b.name)
                        ));
                    }
                }
            }
            let _ = writeln!(
                feats,
                "            Instr::{}(ops) => vec![{}],",
                ident(&i.name),
                fs.join(", ")
            );
        }
        if instrs.is_empty() {
            s.push_str("            _ => 0,\n");
            has.push_str("            _ => false,\n");
            sched.push_str("            _ => Schedule::default(),\n");
            feats.push_str("            _ => Vec::new(),\n");
        }
        s.push_str("        }))();\n        r.unwrap_or(0)\n    }\n");
        has.push_str("        }\n    }\n");
        sched.push_str("        }))();\n        r.unwrap_or_default()\n    }\n");
        s.push_str(&sched);
        s.push_str(&run_fns);
        feats.push_str("        }\n    }\n");
        s.push_str(&has);
        s.push_str(&feats);
        for (name, t) in [
            ("fault_cycles", &self.isa.fault_cycles),
            ("interrupt_cycles", &self.isa.interrupt_cycles),
        ] {
            let mut params = String::new();
            if name == "fault_cycles" {
                params = match t {
                    Some(t) if !t.params.is_empty() => {
                        let cx = Cx {
                            locals: &t.locals,
                            fallible: true,
                            recv: "self",
                            ops: None,
                            comp: None,
                            opt: false,
                        };
                        format!("{}: u8", self.local_name(&cx, t.params[0]))
                    }
                    _ => "_vector: u8".into(),
                };
            }
            let body = match t {
                Some(t) => {
                    let cx = Cx {
                        locals: &t.locals,
                        fallible: true,
                        recv: "self",
                        ops: None,
                        comp: None,
                        opt: false,
                    };
                    format!("let r: Result<u32, Fault> = (|| Ok(({}) as u32))();\n        r.unwrap_or(0)", self.expr(&cx, &t.expr)?)
                }
                None => "0".into(),
            };
            let _ = write!(s, "    /// Timing table: `{}`.\n    pub fn {name}(&mut self, {params}) -> u32 {{\n        {body}\n    }}\n", name.trim_end_matches("_cycles"));
        }
        if let Some(t) = &self.isa.split_cycles {
            let cx = Cx {
                locals: &t.locals,
                fallible: true,
                recv: "self",
                ops: None,
                comp: None,
                opt: false,
            };
            let _ = write!(
                s,
                "    /// Timing table: `split`, the cycles a data access adds.\n    #[allow(unused_variables)]\n    pub fn split_cycles(&mut self, {}: u32, {}: u32) -> u32 {{\n        let r: Result<u32, Fault> = (|| Ok(({}) as u32))();\n        r.unwrap_or(0)\n    }}\n",
                self.local_name(&cx, t.params[0]),
                self.local_name(&cx, t.params[1]),
                self.expr(&cx, &t.expr)?
            );
        }
        // The interrupt's schedule: its `schedules` entry, else its cycles as work.
        let sbody = match &self.isa.interrupt_schedule {
            Some(t) => {
                let cx = Cx {
                    locals: &t.locals,
                    fallible: true,
                    recv: "self",
                    ops: None,
                    comp: None,
                    opt: false,
                };
                let e = self.expr(&cx, &t.expr)?;
                if matches!(t.expr.ty, Ty::Bundle(_)) {
                    format!("let r: Result<Schedule, Fault> = (|| Ok({e}))();\n        r.unwrap_or_default()")
                } else {
                    format!("let r: Result<u32, Fault> = (|| Ok(({e}) as u32))();\n        let mut s = Schedule::default();\n        s.work[0] = r.unwrap_or(0) as u16;\n        s")
                }
            }
            None => "let mut s = Schedule::default();\n        s.work[0] = self.interrupt_cycles() as u16;\n        s".into(),
        };
        let _ = write!(s, "    /// The schedule of an interrupt taken at a boundary.\n    pub fn interrupt_schedule(&mut self) -> Schedule {{\n        {sbody}\n    }}\n");
        // A fault's delivery schedule, when the `schedules` table has one:
        // the instruction's own `fault <Name>` entry, else the isa's.
        let any_fault_schedule =
            self.isa.fault_schedule.is_some() || instrs.iter().any(|i| i.fault_schedule.is_some());
        if any_fault_schedule {
            let mut arms = String::new();
            for i in &instrs {
                let Some(t) = &i.fault_schedule else { continue };
                let cx = Cx {
                    locals: &t.locals,
                    fallible: true,
                    recv: "self",
                    ops: Some(i),
                    comp: None,
                    opt: false,
                };
                let vn = self.local_name(&cx, t.params[0]);
                let e = self.expr(&cx, &t.expr)?;
                let body = if matches!(t.expr.ty, Ty::Bundle(_)) {
                    e
                } else {
                    format!("{{ let mut s = Schedule::default(); s.work[0] = ({e}) as u16; s }}")
                };
                let _ = writeln!(arms, "            Some(Instr::{}(ops)) => {{ let {vn}: u8 = v; let r: Result<Schedule, Fault> = (|| Ok({body}))(); r.unwrap_or_default() }}", ident(&i.name));
            }
            let _ = write!(s, "    /// The schedule of delivering fault `v` that `i` raised.\n    #[allow(unused_variables)]\n    pub fn fault_schedule_for(&mut self, i: Option<Instr>, v: u8) -> Schedule {{\n        match i {{\n{arms}            _ => self.fault_schedule(v),\n        }}\n    }}\n");
        }
        if !any_fault_schedule {
        } else if self.isa.fault_schedule.is_none() {
            let _ = write!(s, "    /// The schedule of delivering a fault: its timing cost as work.\n    pub fn fault_schedule(&mut self, v: u8) -> Schedule {{\n        let mut s = Schedule::default();\n        s.work[0] = self.fault_cycles(v) as u16;\n        s\n    }}\n");
        } else if let Some(t) = &self.isa.fault_schedule {
            let cx = Cx {
                locals: &t.locals,
                fallible: true,
                recv: "self",
                ops: None,
                comp: None,
                opt: false,
            };
            let param = match t.params.first() {
                Some(p) => self.local_name(&cx, *p),
                None => "_vector".into(),
            };
            let e = self.expr(&cx, &t.expr)?;
            let body = if matches!(t.expr.ty, Ty::Bundle(_)) {
                format!("let r: Result<Schedule, Fault> = (|| Ok({e}))();\n        r.unwrap_or_default()")
            } else {
                format!("let r: Result<u32, Fault> = (|| Ok(({e}) as u32))();\n        let mut s = Schedule::default();\n        s.work[0] = r.unwrap_or(0) as u16;\n        s")
            };
            let _ = write!(s, "    /// The schedule of delivering fault `{param}`, after the faulting instruction's.\n    pub fn fault_schedule(&mut self, {param}: u8) -> Schedule {{\n        {body}\n    }}\n");
        }

        // lock: whether `i` may follow a lock prefix
        s.push_str("    /// Whether `i` may follow a lock prefix (its `lock` item).\n    #[allow(unused_variables)]\n    pub fn lock_ok(&mut self, i: &Instr) -> bool {\n        let r: Result<bool, Fault> = (|| Ok(match *i {\n");
        for i in &instrs {
            let text = match &i.lock {
                Some(e) => {
                    let cx = Cx {
                        locals: &i.lock_locals,
                        fallible: true,
                        recv: "self",
                        ops: Some(i),
                        comp: None,
                        opt: false,
                    };
                    self.expr(&cx, e)?
                }
                None => "false".into(),
            };
            let _ = writeln!(s, "            Instr::{}(ops) => {text},", ident(&i.name));
        }
        if instrs.is_empty() {
            s.push_str("            _ => false,\n");
        }
        s.push_str("        }))();\n        r.unwrap_or(false)\n    }\n");

        // mnemonic
        s.push_str("    pub fn mnemonic(&mut self, i: &Instr) -> &'static str {\n        let r: Result<&'static str, Fault> = (|| Ok(match *i {\n");
        for i in &instrs {
            let text = match &i.mnemonic {
                Some(hir::Mnemonic::Fixed(m)) => format!("{m:?}"),
                Some(hir::Mnemonic::Expr { locals, expr }) => {
                    let cx = Cx {
                        locals,
                        fallible: true,
                        recv: "self",
                        ops: Some(i),
                        comp: None,
                        opt: false,
                    };
                    self.expr(&cx, expr)?
                }
                None => format!("{:?}", i.name),
            };
            let _ = writeln!(s, "            Instr::{}(ops) => {text},", ident(&i.name));
        }
        if instrs.is_empty() {
            s.push_str("            _ => \"?\",\n");
        }
        s.push_str("        }))();\n        r.unwrap_or(\"?\")\n    }\n");

        // name
        s.push_str("    pub fn instr_name(i: &Instr) -> &'static str {\n        match i {\n");
        for i in &instrs {
            let _ = writeln!(
                s,
                "            Instr::{}(_) => {:?},",
                ident(&i.name),
                i.name
            );
        }
        if instrs.is_empty() {
            s.push_str("            _ => \"?\",\n");
        }
        s.push_str("        }\n    }\n");
        self.out.push_str(&s);
        Ok(())
    }

    fn width_of_field(&self, ty: &Ty) -> u32 {
        match ty {
            Ty::UInt(n) | Ty::SInt(n) => *n,
            Ty::Bool => 1,
            Ty::Enum(e) => self.enums[e].width,
            _ => 8,
        }
    }

    fn emit_tests(&mut self) -> R<()> {
        let isa_name = ident(&self.isa.name);
        let tests: Vec<&hir::TestBody> = self
            .hir
            .tests
            .iter()
            .filter(|t| t.isa == Some(self.isa.def) || t.isa.is_none())
            .collect();
        if tests.is_empty() {
            return Ok(());
        }
        self.line("#[cfg(test)]\nmod tests {\n    use super::*;");
        for (k, t) in tests.iter().enumerate() {
            let module = self.prog.item(t.def).module;
            let file = &self.prog.modules[module].file;
            let cx = Cx {
                locals: &t.locals,
                fallible: true,
                recv: "m",
                ops: None,
                comp: None,
                opt: false,
            };
            let mut body = format!("let mut m = <{isa_name}>::new();\n");
            for s in &t.body.stmts {
                match &s.kind {
                    S::Given(pairs) => {
                        for (place, v) in pairs {
                            body.push_str(&self.assign(&cx, place, &self.expr(&cx, v)?)?);
                        }
                    }
                    S::Expect(checks) => {
                        for c in checks {
                            let src = file
                                .slice(c.span)
                                .replace('"', "'")
                                .replace('{', "{{")
                                .replace('}', "}}");
                            let _ = writeln!(
                                body,
                                "assert!({}, \"expect failed: {}\");",
                                self.expr(&cx, c)?,
                                src
                            );
                        }
                    }
                    S::RunBytes(bytes) => {
                        let bs: Vec<String> = bytes.iter().map(|b| format!("{b:#04x}")).collect();
                        let _ = writeln!(body, "m.run_bytes(&[{}])?;", bs.join(", "));
                    }
                    S::RunInstruction(def, values) => {
                        let instr = self
                            .isa
                            .instructions
                            .iter()
                            .find(|i| i.def == *def)
                            .ok_or_else(|| GenError::new("instruction not generated"))?;
                        let mut fields = Vec::new();
                        for (b, v) in instr.operands.iter().zip(values) {
                            let kind = self.kinds[&b.kind];
                            let kname = ident(&kind.name);
                            let is_place = matches!(
                                v.kind,
                                H::Register(_) | H::RegField(..) | H::RegView(..) | H::Mode(_)
                            );
                            let has = |rk: hir::RuleKind| kind.rules.iter().any(|r| r.kind == rk);
                            let val = if kind.source.is_some() {
                                if is_place {
                                    format!(
                                        "m.find_{kname}_place({})?",
                                        self.place_ref_text(&cx, v, false)?
                                    )
                                } else if self.is_pure_read(
                                    &kind
                                        .rules
                                        .iter()
                                        .find(|r| r.kind == hir::RuleKind::Read)
                                        .unwrap()
                                        .body,
                                ) {
                                    format!("m.find_{kname}_value({})?", self.expr(&cx, v)?)
                                } else {
                                    return Err(GenError::new(format!(
                                        "test {:?}: operand `{}` of kind `{}` must be given as a register; its `read` touches memory",
                                        t.name, b.name, kind.name
                                    )));
                                }
                            } else if has(hir::RuleKind::Value) {
                                format!("m.find_{kname}_value({})", self.expr(&cx, v)?)
                            } else if is_place {
                                format!(
                                    "m.find_{kname}_select({})",
                                    self.place_ref_text(&cx, v, false)?
                                )
                            } else {
                                return Err(GenError::new(format!(
                                    "test {:?}: operand `{}` of kind `{}` is a register selector; give a register",
                                    t.name, b.name, kind.name
                                )));
                            };
                            let _ = writeln!(body, "let op_{} = {val};", ident(&b.name));
                            fields.push(format!("{n}: op_{n}", n = ident(&b.name)));
                        }
                        let _ = writeln!(
                            body,
                            "m.run(Instr::{}({} {{ {} }}))?;",
                            ident(&instr.name),
                            Self::ops_struct_name(instr),
                            fields.join(", ")
                        );
                    }
                    _ => body.push_str(&self.stmt(&cx, s)?),
                }
            }
            let fname = format!(
                "t{k}_{}",
                t.name
                    .chars()
                    .map(|c| if c.is_ascii_alphanumeric() {
                        c.to_ascii_lowercase()
                    } else {
                        '_'
                    })
                    .collect::<String>()
            );
            let _ = write!(
                self.out,
                "    fn body_{k}() -> Result<(), Fault> {{\n{body}Ok(())\n}}\n    #[test]\n    fn {fname}() {{ body_{k}().unwrap(); }}\n"
            );
        }
        self.line("}");
        Ok(())
    }

    /// The cycles a data access of `bits` in space `d` adds (`split =>`):
    /// charged for the instruction stream's space only.
    fn split_charge(&self, cx: &Cx, d: DefId, bits: u32) -> String {
        let data_space = self.isa.istream.as_ref().is_some_and(|i| i.space == d);
        if self.isa.split_cycles.is_none() || !data_space {
            return String::new();
        }
        format!(
            "{{ let c = {0}.split_cycles(a as u32, {1}); {0}.cycles = {0}.cycles.wrapping_add(c); }} ",
            cx.recv,
            bits / 8
        )
    }

    /// The vector runner: one module per `vectors ... on <this isa>`.
    fn emit_vectors(&mut self) -> R<()> {
        let vs: Vec<&hir::VectorsInfo> = self
            .hir
            .vectors
            .iter()
            .filter(|v| v.isa == self.isa.def)
            .collect();
        let Some(v) = vs.first() else { return Ok(()) };
        let isa_name = ident(&self.isa.name);
        let mem = ident(&self.spaces[&v.memory].name);
        let harness = v.harness_cycles;
        let undefined = match self.isa.istream.as_ref().and_then(|s| s.undefined) {
            Some(f) => format!("Fault::{}", ident(&self.faults[&f].name)),
            None => {
                return Err(GenError::new(
                    "vectors need the isa's `istream` to name an `undefined` fault",
                ))
            }
        };
        let glob = std::path::Path::new(&v.glob);
        let glob_abs = glob
            .canonicalize()
            .ok()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|| {
                // The directory may exist even if the pattern does not.
                match (
                    glob.parent().and_then(|d| d.canonicalize().ok()),
                    glob.file_name(),
                ) {
                    (Some(d), Some(f)) => d.join(f).to_string_lossy().into_owned(),
                    _ => v.glob.clone(),
                }
            });

        let mut open = String::new();
        for (sp, v) in &v.open {
            let _ = writeln!(
                open,
                "        m.bus.{}.set_open_bus({v});",
                ident(&self.spaces[sp].name)
            );
        }
        let mut load = String::new();
        let mut read = String::new();
        let mut masked = String::new();
        let mut names = Vec::new();
        for r in &v.regs {
            let cx = Cx {
                locals: &r.locals,
                fallible: true,
                recv: "m",
                ops: None,
                comp: None,
                opt: false,
            };
            let value_name = self.local_name(&cx, r.value);
            let _ = writeln!(
                load,
                "            {:?} => {{ let {value_name}: u{} = *v as u{}; {}}}",
                r.name,
                r.bits,
                r.bits,
                self.block_stmts(&cx, &r.load)?
            );
            let _ = writeln!(
                read,
                "            {:?} => ({}) as u32,",
                r.name,
                self.expr(&cx, &r.read)?
            );
            if let Some(reg) = r.masked_reg {
                let _ = writeln!(
                    masked,
                    "            {:?} => Some({}),",
                    r.name, self.regs[&reg].0
                );
            }
            names.push(format!("{:?}", r.name));
        }
        let names = names.join(", ");
        let _ = write!(
            self.out,
            r#"
#[cfg(feature = "vectors")]
pub mod vectors {{
    use super::*;
    use rt::vectors::{{CpuState, MooTest}};

    /// Where the vectors were found when this crate was generated.
    pub const GLOB: &str = {glob_abs:?};
    pub const REGS: &[&str] = &[{names}];

    fn load(m: &mut {isa_name}, st: &CpuState) -> Result<(), Fault> {{
{open}        for (name, v) in &st.regs {{
            match *name {{
{load}            _ => {{}}
            }}
        }}
        for (a, b) in &st.ram {{ m.bus.{mem}.write_cell(*a as u64, *b as u128); }}
        Ok(())
    }}

    fn read(m: &mut {isa_name}, name: &str) -> Result<u32, Fault> {{
        Ok(match name {{
{read}            _ => 0,
        }})
    }}

    fn masked_reg(name: &str) -> Option<usize> {{
        match name {{
{masked}            _ => None,
        }}
    }}

    pub enum Outcome {{
        Pass,
        Fail(Vec<String>),
        /// The first instruction does not decode: not described yet.
        Unimplemented,
        /// Everything matches except stored copies of flags the instruction
        /// leaves undefined (a fault frame pushed after a DIV, say): the
        /// hardware's values come from state the description does not model.
        UndefinedLeak(Vec<String>),
    }}

    pub fn run_test(t: &MooTest) -> Outcome {{
        let mut m = <{isa_name}>::new();
        if let Err(f) = load(&mut m, &t.initial) {{
            return Outcome::Fail(vec![format!("fault while loading the initial state: {{f:?}}")]);
        }}
        let mut first: Option<Instr> = None;
        let mut delivered: Option<Fault> = None;
        let mut trace: Vec<String> = Vec::new();
        for n in 0..4096 {{
            if m.halted {{ break; }}
            let before: Vec<String> = REGS.iter().filter(|r| **r == "cs" || **r == "ip").map(|r| format!("{{r}}={{:04x}}", read(&mut m, r).unwrap_or(0))).collect();
            let r = m.step();
            if n == 0 && !m.known_opcode && (r.is_err() || m.last_fault.is_some()) {{ return Outcome::Unimplemented; }}
            if first.is_none() {{ first = m.last; }}
            if let Some(f) = &m.last_fault {{ delivered = Some(f.clone()); }}
            if trace.len() < 6 {{ trace.push(format!("step {{n}}: {{}} -> {{}}{{}}", before.join(" "), m.last.map(|i| <{isa_name}>::instr_name(&i)).unwrap_or("?"), m.last_fault.as_ref().map(|f| format!(", delivered {{f:?}}")).unwrap_or_default())); }}
            if let Err(f) = r {{
                let at = first.map(|i| <{isa_name}>::instr_name(&i)).unwrap_or("?");
                return Outcome::Fail(vec![format!("undelivered fault {{f:?}} (after {{at}})")]);
            }}
        }}
        let _ = {undefined};
        let mut errors = Vec::new();
        if !m.halted {{ errors.push("did not halt".to_string()); }}
        // Flag conditions describe the state before the instruction
        // (`undefined: [OF if CL != 1]` must not see `shl cl, cl`'s result).
        let mut pre = <{isa_name}>::new();
        let _ = load(&mut pre, &t.initial);
        pre.reset_latches();
        for name in REGS {{
            let Some(expected) = t.final_.reg(name).or_else(|| t.initial.reg(name)) else {{ continue }};
            let actual = match read(&mut m, name) {{ Ok(v) => v, Err(f) => {{ errors.push(format!("{{name}}: fault while reading: {{f:?}}")); continue; }} }};
            let mut mask = match (masked_reg(name), &first) {{
                (Some(r), Some(i)) => pre.undefined_mask(i, r) as u32,
                _ => 0,
            }};
            // Bits the corpus itself marks undefined (MOO `RM32`).
            for (n, u) in &t.final_.undefined {{ if n == name {{ mask |= *u; }} }}
            if actual & !mask != expected & !mask {{
                errors.push(format!("{{name}}: expected {{expected:#06x}}, got {{actual:#06x}} (undefined bits {{mask:#06x}})"));
            }}
        }}
        let regs_ok = errors.is_empty();
        // Flag bits the executed instruction leaves undefined, for recognising
        // a stored copy of them (a fault frame) in memory.
        let mut leak_mask: u32 = 0;
        if let Some(i) = &first {{
            for name in REGS {{
                if let Some(r) = masked_reg(name) {{ leak_mask |= pre.undefined_mask(i, r) as u32; }}
            }}
        }}
        let mut leaks = 0usize;
        let mut addrs: Vec<u32> = t.initial.ram.iter().chain(t.final_.ram.iter()).map(|(a, _)| *a).collect();
        addrs.sort_unstable();
        addrs.dedup();
        for a in addrs {{
            let expected = t.final_.ram.iter().chain(t.initial.ram.iter()).find(|(x, _)| *x == a).map(|(_, v)| *v).unwrap_or(0);
            let actual = m.bus.{mem}.read_cell(a as u64) as u8;
            if actual != expected {{
                let diff = actual ^ expected;
                if delivered.is_some() && t.exception.is_some() && (diff & !(leak_mask as u8) == 0 || diff & !((leak_mask >> 8) as u8) == 0) {{ leaks += 1; }}
                errors.push(format!("ram[{{a:#07x}}]: expected {{expected:#04x}}, got {{actual:#04x}}"));
            }}
        }}
        if regs_ok && !errors.is_empty() && leaks == errors.len() {{
            return Outcome::UndefinedLeak(errors);
        }}
        if errors.is_empty() {{ Outcome::Pass }} else {{
            if let Some(i) = first {{ errors.insert(0, format!("decoded as {{}} ({{}})", <{isa_name}>::instr_name(&i), m.mnemonic(&i))); }}
            if let Some(f) = delivered {{ errors.insert(1, format!("delivered {{f:?}}; vector expects {{:?}}", t.exception)); }}
            else if let Some(v) = t.exception {{ errors.insert(1, format!("vector expects exception {{v}}, none was raised")); }}
            for (k, line) in trace.iter().take(6).enumerate() {{ errors.insert(2 + k, line.clone()); }}
            Outcome::Fail(errors)
        }}
    }}

    #[derive(Default, Debug, Clone)]
    pub struct Report {{
        pub total: usize,
        pub passed: usize,
        pub failed: usize,
        pub exceptions: usize,
        pub revoked: usize,
        pub unimplemented: usize,
        pub leaks: usize,
    }}

    /// Run every vector file matching `GLOB` (or `dir`) whose name starts with
    /// one of `filter` (all when empty). Prints one line per file and up to
    /// `max_failures` failure details. Returns whether nothing failed.
    pub fn run(dir: Option<&str>, filter: &[String], max_failures: usize, quiet: bool) -> bool {{
        let pattern = match dir {{
            Some(d) => format!("{{}}/{{}}", d.trim_end_matches('/'), std::path::Path::new(GLOB).file_name().and_then(|f| f.to_str()).unwrap_or("*")),
            None => GLOB.to_string(),
        }};
        let files = rt::vectors::glob(&pattern);
        if files.is_empty() {{
            eprintln!("no vector files match {{pattern}}");
            return false;
        }}
        let revoked = rt::vectors::revocations(std::path::Path::new(&pattern).parent().unwrap_or(std::path::Path::new(".")));
        let mut all = Report::default();
        let mut shown = 0usize;
        let mut files_run = 0usize;
        for path in files {{
            let fname = path.file_name().and_then(|f| f.to_str()).unwrap_or("").to_string();
            if !filter.is_empty() && !filter.iter().any(|p| fname.starts_with(p.as_str())) {{ continue; }}
            let tests = match rt::vectors::read_moo(&path) {{ Ok(t) => t, Err(e) => {{ eprintln!("{{e}}"); all.failed += 1; continue; }} }};
            let mut r = Report::default();
            for t in &tests {{
                r.total += 1;
                if t.hash.as_ref().is_some_and(|h| revoked.contains(h)) {{ r.revoked += 1; continue; }}
                if t.exception.is_some() {{ r.exceptions += 1; }}
                match run_test(t) {{
                    Outcome::Pass => r.passed += 1,
                    Outcome::Unimplemented => r.unimplemented += 1,
                    Outcome::UndefinedLeak(_) => r.leaks += 1,
                    Outcome::Fail(errors) => {{
                        r.failed += 1;
                        if shown < max_failures {{
                            shown += 1;
                            let bytes: Vec<String> = t.bytes.iter().map(|b| format!("{{b:02x}}")).collect();
                            println!("FAIL {{fname}} #{{}} `{{}}` [{{}}]", t.index, t.name, bytes.join(" "));
                            for e in errors {{ println!("    {{e}}"); }}
                        }}
                    }}
                }}
            }}
            files_run += 1;
            let status = if r.failed > 0 {{ "FAIL" }} else if r.passed == 0 {{ "skip" }} else {{ "ok  " }};
            if !quiet || r.failed > 0 {{
                println!("{{status}} {{fname:<14}} {{:>5}} passed {{:>5}} failed {{:>5}} unimplemented {{:>4}} leak {{:>4}} exc {{:>3}} revoked", r.passed, r.failed, r.unimplemented, r.leaks, r.exceptions, r.revoked);
            }}
            all.total += r.total; all.passed += r.passed; all.failed += r.failed;
            all.exceptions += r.exceptions; all.revoked += r.revoked; all.unimplemented += r.unimplemented; all.leaks += r.leaks;
        }}
        println!("{{files_run}} file(s), {{}} vectors: {{}} passed, {{}} failed, {{}} unimplemented, {{}} undefined-flag leaks, {{}} revoked ({{}} with exceptions)", all.total, all.passed, all.failed, all.unimplemented, all.leaks, all.revoked, all.exceptions);
        all.failed == 0
    }}

    /// Cycles of each trace that belong to the capture rig.
    pub const HARNESS_CYCLES: u32 = {harness};

    /// One vector's timing: the table's prediction against the trace.
    #[derive(Debug, Clone)]
    pub struct TimingSample {{
        pub name: &'static str,
        pub features: Vec<(&'static str, i64)>,
        /// Of `predicted`, the cycles that are not the instruction's table
        /// entry: its prefixes and split accesses (what a fit must not
        /// attribute to the entry).
        pub dynamic: i64,
        /// Execution cycles the timing table gives, summed over the steps.
        pub predicted: i64,
        /// The trace's cycles outside bus cycles, less the rig's.
        pub measured: i64,
        pub fault: bool,
        /// The vector of a delivered fault.
        pub fault_vector: Option<u8>,
        /// Every step had a timing entry.
        pub timed: bool,
    }}

    /// Run a vector and compare its timing, or `None` without a trace or
    /// when it does not run cleanly to the rig's HALT.
    pub fn timing_sample(t: &MooTest) -> Option<TimingSample> {{
        let c = t.timing.as_ref()?;
        let mut m = <{isa_name}>::new();
        load(&mut m, &t.initial).ok()?;
        let mut pre = <{isa_name}>::new();
        let _ = load(&mut pre, &t.initial);
        pre.reset_latches();
        let mut first: Option<Instr> = None;
        let mut started = false;
        let mut predicted: i64 = 0;
        let mut fault = false;
        let mut fault_vector: Option<u8> = None;
        let mut timed = true;
        let mut dynamic: i64 = 0;
        for _ in 0..4096 {{
            if m.halted {{ break; }}
            if m.step().is_err() {{ return None; }}
            // The rig's HALT is the harness's, not the instruction's.
            if m.halted && started {{ break; }}
            if !started {{
                started = true;
                first = m.last;
                // Faulted before an instruction decoded (an undefined
                // opcode): only the delivery is timed.
                if first.is_none() {{
                    return Some(TimingSample {{ name: "(fault while decoding)", features: Vec::new(), dynamic: 0, predicted: m.cycles as i64, measured: c.execution(HARNESS_CYCLES), fault: true, fault_vector: m.last_fault.as_ref().and_then(|f| f.vector()), timed: true }});
                }}
            }}
            if let Some(i) = &m.last {{
                if !<{isa_name}>::has_timing(i) {{ timed = false; }}
                // What this step charged beyond its table entry.
                dynamic += m.cycles as i64 - m.entry_cycles as i64;
            }}
            fault |= m.last_fault.is_some();
            if let Some(f) = &m.last_fault {{ fault_vector = f.vector(); }}
            predicted += m.cycles as i64;
        }}
        if !m.halted {{ return None; }}
        let i = first?;
        Some(TimingSample {{
            name: <{isa_name}>::instr_name(&i),
            features: pre.timing_features(&i),
            dynamic,
            predicted,
            measured: c.execution(HARNESS_CYCLES),
            fault,
            fault_vector,
            timed,
        }})
    }}

    #[derive(Default, Debug, Clone)]
    struct TimingTally {{ n: usize, exact: usize, near: usize, abs: i64 }}
    impl TimingTally {{
        fn add(&mut self, s: &TimingSample) {{
            let e = (s.predicted - s.measured).abs();
            self.n += 1;
            if e == 0 {{ self.exact += 1; }}
            if e <= 1 {{ self.near += 1; }}
            self.abs += e;
        }}
        fn line(&self) -> String {{
            if self.n == 0 {{ return "no timed vectors".into(); }}
            format!("{{:>6}} vectors  {{:5.1}}% exact  {{:5.1}}% within 1  mean error {{:.2}}",
                self.n, 100.0 * self.exact as f64 / self.n as f64, 100.0 * self.near as f64 / self.n as f64, self.abs as f64 / self.n as f64)
        }}
    }}

    /// The timing report (`--timing`), or suggested table entries (`--fit`).
    pub fn run_timing(dir: Option<&str>, filter: &[String], fit: bool, quiet: bool) -> bool {{
        let pattern = match dir {{
            Some(d) => format!("{{}}/{{}}", d.trim_end_matches('/'), std::path::Path::new(GLOB).file_name().and_then(|f| f.to_str()).unwrap_or("*")),
            None => GLOB.to_string(),
        }};
        let files = rt::vectors::glob(&pattern);
        if files.is_empty() {{ eprintln!("no vector files match {{pattern}}"); return false; }}
        let revoked = rt::vectors::revocations(std::path::Path::new(&pattern).parent().unwrap_or(std::path::Path::new(".")));
        let mut all = TimingTally::default();
        let mut by_name: std::collections::BTreeMap<&'static str, TimingTally> = Default::default();
        let mut untimed: std::collections::BTreeMap<&'static str, usize> = Default::default();
        // For fitting: per (instruction, features) the measured cycles of
        // vectors that did not fault; per instruction the excess of
        // faulting ones over the table.
        let mut hist: std::collections::BTreeMap<(&'static str, String), std::collections::BTreeMap<i64, usize>> = Default::default();
        let mut fault_excess: std::collections::BTreeMap<Option<u8>, std::collections::BTreeMap<i64, usize>> = Default::default();
        for path in files {{
            let fname = path.file_name().and_then(|f| f.to_str()).unwrap_or("").to_string();
            if !filter.is_empty() && !filter.iter().any(|p| fname.starts_with(p.as_str())) {{ continue; }}
            let tests = match rt::vectors::read_moo(&path) {{ Ok(t) => t, Err(e) => {{ eprintln!("{{e}}"); continue; }} }};
            let mut file = TimingTally::default();
            for t in &tests {{
                if t.hash.as_ref().is_some_and(|h| revoked.contains(h)) {{ continue; }}
                let Some(s) = timing_sample(t) else {{ continue }};
                if !s.timed && !fit {{ *untimed.entry(s.name).or_default() += 1; continue; }}
                if fit {{
                    if s.fault {{
                        *fault_excess.entry(s.fault_vector).or_default().entry(s.measured - s.predicted).or_default() += 1;
                    }} else {{
                        let key = s.features.iter().map(|(n, v)| format!("{{n}}={{v}}")).collect::<Vec<_>>().join(" ");
                        *hist.entry((s.name, key)).or_default().entry(s.measured - s.dynamic).or_default() += 1;
                    }}
                }}
                file.add(&s);
                by_name.entry(s.name).or_default().add(&s);
                all.add(&s);
            }}
            if !quiet && !fit {{ println!("{{fname:<14}} {{}}", file.line()); }}
        }}
        if fit {{
            println!("// Suggested timing entries: the most common measured cycles per form,");
            println!("// with the share of vectors it explains. Below 80% the cycles depend on");
            println!("// data or on alignment the table cannot see; write those by hand.");
            let mut last = "";
            for ((name, key), h) in &hist {{
                let n: usize = h.values().sum();
                let (mode, count) = h.iter().max_by_key(|(_, c)| **c).map(|(v, c)| (*v, *c)).unwrap();
                let share = 100.0 * count as f64 / n as f64;
                if *name != last {{ println!("// {{name}}"); last = name; }}
                let flag = if share < 80.0 {{ "  <- data-dependent?" }} else {{ "" }};
                let spread: Vec<String> = h.iter().map(|(v, c)| format!("{{v}}x{{c}}")).collect();
                println!("//   [{{key}}] {{mode}} ({{share:.0}}% of {{n}}){{flag}}   {{}}", spread.join(" "));
            }}
            for (v, h) in &fault_excess {{
                let (mode, count) = h.iter().max_by_key(|(_, c)| **c).map(|(v, c)| (*v, *c)).unwrap();
                let n: usize = h.values().sum();
                let spread: Vec<String> = h.iter().map(|(v, c)| format!("{{v}}x{{c}}")).collect();
                println!("// fault {{v:?}}: exceeds the table by {{mode}} most often ({{:.0}}% of {{n}})   {{}}", 100.0 * count as f64 / n as f64, spread.join(" "));
            }}
            return true;
        }}
        let mut worst: Vec<(&&'static str, &TimingTally)> = by_name.iter().filter(|(_, t)| t.n > 0).collect();
        worst.sort_by(|a, b| (b.1.abs as f64 / b.1.n as f64).partial_cmp(&(a.1.abs as f64 / a.1.n as f64)).unwrap());
        if !worst.is_empty() && worst[0].1.abs > 0 {{
            println!("worst instructions:");
            for (n, t) in worst.iter().take(12) {{ if t.abs > 0 {{ println!("  {{n:<22}} {{}}", t.line()); }} }}
        }}
        if !untimed.is_empty() {{
            let names: Vec<String> = untimed.iter().map(|(n, c)| format!("{{n}} ({{c}})")).collect();
            println!("no timing entry: {{}}", names.join(", "));
        }}
        println!("timing: {{}}", all.line());
        true
    }}

    /// Command-line entry used by the generated `vectors` binary.
    pub fn main_entry() -> bool {{
        let args: Vec<String> = std::env::args().skip(1).collect();
        let mut dir = None;
        let mut filter = Vec::new();
        let mut max_failures = 20usize;
        let mut quiet = false;
        let mut timing = false;
        let mut fit = false;
        let mut i = 0;
        while i < args.len() {{
            match args[i].as_str() {{
                "--dir" => {{ dir = args.get(i + 1).cloned(); i += 1; }}
                "--filter" => {{ if let Some(f) = args.get(i + 1) {{ filter.extend(f.split(',').filter(|s| !s.is_empty()).map(|s| s.to_string())); }} i += 1; }}
                "--max-failures" => {{ max_failures = args.get(i + 1).and_then(|s| s.parse().ok()).unwrap_or(20); i += 1; }}
                "--quiet" => quiet = true,
                "--timing" => timing = true,
                "--fit" => fit = true,
                other => {{ eprintln!("unknown argument {{other}}"); return false; }}
            }}
            i += 1;
        }}
        if timing || fit {{ return run_timing(dir.as_deref(), &filter, fit, quiet); }}
        run(dir.as_deref(), &filter, max_failures, quiet)
    }}
}}
"#
        );
        Ok(())
    }
}

/// Joining a fault's delivery onto the faulting instruction's schedule
/// (SPEC 11.2c): the instruction's schedule up to the first access it did
/// not make (where it stopped), then the delivery's steps; past 8 steps,
/// the rest is work at the end.
const SCHEDULE_THEN: &str = r#"/// `s` up to the `performed` accesses its instruction made, then `f`.
pub fn schedule_then(s: Schedule, performed: u8, f: Schedule) -> Schedule {
    let mut work: Vec<u16> = vec![s.work[0]];
    let mut steps: Vec<(u8, u8)> = Vec::new();
    let mut seen = 0u8;
    for i in 0..(s.steps as usize).min(8) {
        let mut count = s.count[i];
        if s.step[i] == 1 {
            // The instruction stopped at the access it did not make.
            if seen >= performed {
                break;
            }
            let full = count;
            count = count.min(performed - seen);
            seen += count;
            if count < full {
                steps.push((1, count));
                work.push(0);
                break;
            }
        }
        steps.push((s.step[i], count));
        work.push(s.work[i + 1]);
    }
    let w = work.last_mut().unwrap();
    *w = w.saturating_add(f.work[0]);
    for i in 0..(f.steps as usize).min(8) {
        steps.push((f.step[i], f.count[i]));
        work.push(f.work[i + 1]);
    }
    let mut out = s;
    out.work = [0; 9];
    out.step = [0; 8];
    out.count = [0; 8];
    let n = steps.len().min(8);
    for i in 0..n {
        out.step[i] = steps[i].0;
        out.count[i] = steps[i].1;
    }
    for (i, w) in work.iter().enumerate() {
        let k = i.min(8);
        out.work[k] = out.work[k].saturating_add(*w);
    }
    out.steps = n as u8;
    out
}
"#;

/// Whether a tail reads only its operand's fields (locals below `fields`)
/// and constants, so that the operand struct can compute it.
fn tail_is_local(t: &hir::TailExpr, fields: usize) -> bool {
    match t {
        hir::TailExpr::Operand(_) => true,
        hir::TailExpr::If(c, a, b) => {
            let mut ok = true;
            hir::visit_expr(c, &mut |_| {}, &mut |e| match &e.kind {
                hir::ExprKind::Local(l) if (*l as usize) < fields => {}
                hir::ExprKind::Local(_)
                | hir::ExprKind::Register(_)
                | hir::ExprKind::Latch(_)
                | hir::ExprKind::Mode(_)
                | hir::ExprKind::FnCall(..) => ok = false,
                _ => {}
            });
            ok && tail_is_local(a, fields) && b.as_ref().is_none_or(|b| tail_is_local(b, fields))
        }
    }
}

/// The operand whose tail `o` is, when `o`'s tail reads it as `parent`.
fn tail_parent(o: &hir::Operand) -> Option<DefId> {
    let l = o.tail_locals.get(o.fields.len())?;
    match (&l.name[..], &l.ty) {
        ("parent", Ty::Operand(p)) => Some(*p),
        _ => None,
    }
}
