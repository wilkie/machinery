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
}

pub fn generate(prog: &Program, hir: &hir::Hir, isa_def: DefId) -> R<String> {
    let isa = hir
        .isas
        .iter()
        .find(|i| i.def == isa_def)
        .ok_or_else(|| GenError::new("isa was not checked (errors earlier)"))?;
    let mut g = Gen {
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
    };
    g.emit_all()?;
    Ok(g.out)
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
            Ty::Bundle(d) => self.prog.item_name(*d),
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
            Ty::Operand(d) => self.prog.item_name(*d),
            Ty::OperandTail(d) => format!("{}_Tail", self.prog.item_name(*d)),
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
            H::Index(base, idx) => {
                let bt = self.expr(cx, base)?;
                let it = self.expr(cx, idx)?;
                match &base.ty {
                    Ty::Array(..) => format!("{bt}[({it}) as usize]"),
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
            H::OperandRead(i) => {
                let instr = cx
                    .ops
                    .ok_or_else(|| GenError::new("operand outside an instruction"))?;
                let b = &instr.operands[*i as usize];
                let kind = self.kinds[&b.kind];
                let op = format!("ops.{}", ident(&b.name));
                if kind.source.is_some() {
                    format!("{}.kind_{}_read({op}){q}", cx.recv, ident(&kind.name))
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
                let rname = &kind.rules[*rule as usize].name;
                format!(
                    "{}.kind_{}_{}(ops.{}){q}",
                    cx.recv,
                    ident(&kind.name),
                    ident(rname),
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
                format!(
                    "{{ let a = ({}) as u64; {}.{}.read(a, {bits}) as {out_ty} }}",
                    self.expr(cx, addr)?,
                    cx.recv,
                    ident(&sp.name)
                )
            }
            H::SpaceCell(d, addr) => {
                let sp = self.spaces[d];
                let (_, _, out_ty) = self.int_shape(&e.ty).unwrap();
                format!(
                    "{{ let a = ({}) as u64; {}.{}.read_cell(a) as {out_ty} }}",
                    self.expr(cx, addr)?,
                    cx.recv,
                    ident(&sp.name)
                )
            }
            H::SpaceWrite(..) => return Err(GenError::new("space write in expression position")),
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
                let a: R<Vec<String>> = args.iter().map(|x| self.expr(cx, x)).collect();
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
            H::Port(..) | H::State(..) | H::ModelMem(..) | H::BusCall(..) => {
                return Err(GenError::new(
                    "component model constructs are not part of an isa interpreter",
                ))
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
            S::Let(t, e) => format!("let {} = {};\n", self.let_target(cx, t), self.expr(cx, e)?),
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
                format!(
                    "for {name} in ({lo} as {ty})..({hi} as {ty}) {{\n{}}}\n",
                    self.block_stmts(cx, body)?
                )
            }
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
                let vt = self.expr(cx, v)?;
                if kind.source.is_some() {
                    format!(
                        "{}.kind_{}_write({op}, {vt})?;\n",
                        cx.recv,
                        ident(&kind.name)
                    )
                } else {
                    format!(
                        "{}.kind_{}_select_write({op}, {vt});\n",
                        cx.recv,
                        ident(&kind.name)
                    )
                }
            }
            S::SpaceWrite(d, bits, addr, v) => {
                let sp = self.spaces[d];
                format!(
                    "{{ let a = ({}) as u64; let v = ({}) as u128; {}.{}.write(a, {bits}, v); }}\n",
                    self.expr(cx, addr)?,
                    self.expr(cx, v)?,
                    cx.recv,
                    ident(&sp.name)
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
            S::Expect(_) | S::Given(_) | S::RunInstruction(..) | S::RunBytes(_) => {
                return Err(GenError::new("test statement outside a test"))
            }
            S::Emit { .. } | S::While(..) | S::Loop(_) | S::Break | S::Continue => {
                return Err(GenError::new("component model statement in an isa"))
            }
            S::Error => return Err(GenError::new("error statement")),
        })
    }

    /// Statements that store `rhs` (an expression text) into `place`.
    fn assign(&self, cx: &Cx, place: &Expr, rhs: &str) -> R<String> {
        Ok(match &place.kind {
            H::Register(_) | H::Mode(_) => format!("{} = {rhs};\n", self.expr(cx, place)?),
            H::Field(base, i) => {
                if Self::is_lvalue(base) {
                    let Ty::Bundle(d) = &base.ty else {
                        unreachable!()
                    };
                    let bd = self.bundles[d];
                    format!(
                        "{}.{} = {rhs};\n",
                        self.expr(cx, base)?,
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
                    "{{ let a = ({}) as u64; let v = ({rhs}) as u128; {}.{}.write_cell(a, v); }}\n",
                    self.expr(cx, addr)?,
                    cx.recv,
                    ident(&sp.name)
                )
            }
            H::KindSelect(k, args) => {
                let kind = self.kinds[k];
                let a: R<Vec<String>> = args.iter().map(|x| self.expr(cx, x)).collect();
                format!(
                    "{}.kind_{}_select_write({}, {rhs});\n",
                    cx.recv,
                    ident(&kind.name),
                    a?.join(", ")
                )
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
            H::Register(_) | H::Mode(_) => true,
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
        self.line("#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals, unused_variables, unused_mut, unused_parens, unused_imports, dead_code, unreachable_code, clippy::all)]");
        self.line("use machine_runtime as rt;");
        self.line("");

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
        for b in &self.hir.bundles {
            let mut s = format!(
                "#[derive(Clone, Copy, Debug, PartialEq, Default)]\npub struct {} {{\n",
                ident(&b.name)
            );
            for f in &b.fields {
                let _ = writeln!(s, "    pub {}: {},", ident(&f.name), self.rty(&f.ty)?);
            }
            s.push_str("}\n");
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
        s.push_str("        }\n    }\n}\n");
        self.out.push_str(&s);

        // Operands.
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

        // The machine.
        let mode_ty = match self.isa.mode {
            Some(m) => native(self.enums[&m].width, false).1,
            None => "u8",
        };
        let mut s = format!("#[derive(Clone, Debug)]\npub struct {isa_name} {{\n    pub r: Regs,\n    pub mode: {mode_ty},\n");
        for sp in &self.isa.spaces {
            let _ = writeln!(s, "    pub {}: rt::Memory,", ident(&sp.name));
        }
        s.push_str("}\n");
        self.out.push_str(&s);

        let mut s = format!("impl {isa_name} {{\n    pub fn new() -> Self {{\n        Self {{\n            r: Regs::default(),\n            mode: 0,\n");
        for sp in &self.isa.spaces {
            let _ = writeln!(
                s,
                "            {}: rt::Memory::new({}, {}),",
                ident(&sp.name),
                sp.cell_bits,
                sp.little_endian
            );
        }
        s.push_str("        }\n    }\n");
        self.out.push_str(&s);

        for f in &self.isa.fns {
            self.emit_fn(f)?;
        }
        for k in &self.isa.kinds {
            self.emit_kind(k)?;
        }
        for i in &self.isa.instructions {
            self.emit_instruction(i)?;
        }
        self.emit_decoder()?;
        self.line("}");
        self.line(&format!(
            "impl Default for {isa_name} {{ fn default() -> Self {{ Self::new() }} }}"
        ));

        self.emit_tests()?;
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
        // tail_sel: 0 = none, k = k-th operand of the tail set.
        let _ = writeln!(s, "    pub fn tail_sel(&self) -> u8 {{");
        let cx = Cx {
            locals: &o.tail_locals,
            fallible: false,
            recv: "self",
            ops: None,
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
            Some(t) => {
                let _ = writeln!(s, "        {}", self.tail_expr(&cx, t, &o.tail_set)?);
            }
            None => s.push_str("        0\n"),
        }
        s.push_str("    }\n}\n");
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
        let cx = Cx {
            locals: &f.locals,
            fallible: true,
            recv: "self",
            ops: None,
        };
        let mut params = vec!["&mut self".to_string()];
        for p in &f.params {
            params.push(format!(
                "{}: {}",
                self.local_name(&cx, *p),
                self.rty(&f.locals[*p as usize].ty)?
            ));
        }
        let body = self.block_stmts(&cx, &f.body)?;
        let tail = if f.ret == Ty::Unit { "Ok(())\n" } else { "" };
        let s = format!(
            "    pub fn fn_{}({}) -> Result<{}, Fault> {{\n{body}{tail}}}\n",
            ident(&f.name),
            params.join(", "),
            self.rty(&f.ret)?
        );
        self.out.push_str(&s);
        Ok(())
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
                    let pure = Cx {
                        fallible: false,
                        ..cx
                    };
                    let body_expr = match &rule.body {
                        ArmBody::Expr(e) => e.clone(),
                        ArmBody::Block(b) => Expr {
                            kind: H::Block(b.clone()),
                            ty: k.ty.clone(),
                            span: b.span,
                        },
                    };
                    let read = self.expr(&pure, &body_expr)?;
                    let write = self.assign(&pure, &body_expr, "value")?;
                    let place = self.place_ref_text(&pure, &body_expr, false)?;
                    let _ = write!(
                        self.out,
                        "    pub fn kind_{kname}_select_read(&self, {ps}) -> {vty} {{\n{read}\n}}\n    pub fn kind_{kname}_select_write(&mut self, {ps}, value: {vty}) {{\n{write}}}\n    pub fn kind_{kname}_select_place(&self, {ps}) -> rt::PlaceRef {{\n{place}\n}}\n"
                    );
                    // Inversion for tests: which selector denotes a place?
                    let (n, _, pt) = self
                        .int_shape(&rule.locals[rule.params[0] as usize].ty)
                        .ok_or_else(|| GenError::new("select param type"))?;
                    let _ = write!(
                        self.out,
                        "    pub fn find_{kname}_select(&self, target: rt::PlaceRef) -> {pt} {{\n        for n in 0..(1u64 << {n}) {{ let n = n as {pt}; if self.kind_{kname}_select_place(n) == target {{ return n; }} }}\n        panic!(\"operand kind {kname} cannot denote {{target:?}}\")\n    }}\n"
                    );
                }
                hir::RuleKind::Read => {
                    let body = match &rule.body {
                        ArmBody::Expr(e) => format!("Ok({})\n", self.expr(&cx, e)?),
                        ArmBody::Block(b) => format!("Ok({})\n", self.block_expr(&cx, b)?),
                    };
                    let _ = write!(self.out, "    pub fn kind_{kname}_read(&mut self, {ps}) -> Result<{vty}, Fault> {{\n{body}}}\n");
                    // Place inversion for tests.
                    let body_expr = match &rule.body {
                        ArmBody::Expr(e) => e.clone(),
                        ArmBody::Block(b) => Expr {
                            kind: H::Block(b.clone()),
                            ty: k.ty.clone(),
                            span: b.span,
                        },
                    };
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
            }
        }
        Ok(())
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
                S::Let(_, e) | S::Expr(e) => pure_expr(e),
                S::If(c, t, e) => {
                    pure_expr(c)
                        && pure_block(t)
                        && e.as_ref().is_none_or(|s| {
                            pure_block(&Block {
                                stmts: vec![(**s).clone()],
                                span: s.span,
                            })
                        })
                }
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

    fn emit_instruction(&mut self, i: &hir::Instruction) -> R<()> {
        let ops_name = Self::ops_struct_name(i);
        // Operand struct: fetched operand or selector field.
        let mut s =
            format!("}}\n#[derive(Clone, Copy, Debug, PartialEq)]\npub struct {ops_name} {{\n");
        for b in &i.operands {
            let ty = match &b.source {
                hir::BindSource::From { .. } => {
                    let kind = self.kinds[&b.kind];
                    ident(&self.prog.item_name(kind.source.unwrap()))
                }
                hir::BindSource::Select { step, field } => {
                    let hir::EncodingStep::Fetch { operand, .. } = &i.encoding[*step as usize]
                    else {
                        unreachable!()
                    };
                    let o = self.operands[operand];
                    self.rty(&o.fields[*field as usize].ty)?
                }
            };
            let _ = writeln!(s, "    pub {}: {ty},", ident(&b.name));
        }
        let _ = writeln!(s, "}}\nimpl {} {{", ident(&self.isa.name));
        self.out.push_str(&s);

        let cx = Cx {
            locals: &i.locals,
            fallible: true,
            recv: "self",
            ops: Some(i),
        };
        let body = match &i.semantics {
            Some(b) => self.block_stmts(&cx, b)?,
            None => String::new(),
        };
        let _ = write!(
            self.out,
            "    /// {}\n    pub fn exec_{}(&mut self, ops: {ops_name}) -> Result<(), Fault> {{\n{body}Ok(())\n}}\n",
            i.description.join(" "),
            ident(&i.name)
        );
        Ok(())
    }

    fn emit_decoder(&mut self) -> R<()> {
        // Instr enum.
        let mut s = String::from("}\n#[derive(Clone, Copy, Debug, PartialEq)]\npub enum Instr {\n");
        for i in &self.isa.instructions {
            let _ = writeln!(s, "    {}({}),", ident(&i.name), Self::ops_struct_name(i));
        }
        let _ = writeln!(s, "}}\nimpl {} {{", ident(&self.isa.name));
        self.out.push_str(&s);

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
        let undefined = match istream.undefined {
            Some(f) => format!("Err(Fault::{})", ident(&self.faults[&f].name)),
            None => "panic!(\"undefined opcode\")".into(),
        };
        let _ = writeln!(
            self.out,
            "    pub fn istream_next(&mut self) -> Result<u8, Fault> {{ Ok(self.fn_{next}()? as u8) }}"
        );

        // Fetch functions.
        for o in &self.isa.operands {
            let on = ident(&o.name);
            let mut s = format!(
                "    pub fn fetch_{on}(&mut self) -> Result<{on}, Fault> {{\n        let mut bits: u64 = 0;\n        for i in 0..{on}::SIZE {{ bits |= (self.istream_next()? as u64) << (8 * i); }}\n        let mut o = {on}::from_bits(bits);\n        match o.tail_sel() {{\n",
            );
            for (k, t) in o.tail_set.iter().enumerate() {
                let tn = ident(&self.prog.item_name(*t));
                let _ = writeln!(
                    s,
                    "            {} => o.tail = {on}_Tail::{tn}(self.fetch_{tn}()?),",
                    k + 1
                );
            }
            s.push_str("            _ => {}\n        }\n        Ok(o)\n    }\n");
            self.out.push_str(&s);
        }

        // Decoder: a trie over leading opcode bytes.
        struct Node {
            children: Vec<(u8, Node)>,
            leaf: Option<usize>,
        }
        let mut root = Node {
            children: Vec::new(),
            leaf: None,
        };
        for (idx, i) in self.isa.instructions.iter().enumerate() {
            let mut node = &mut root;
            let mut seen_fetch = false;
            for step in &i.encoding {
                match step {
                    hir::EncodingStep::Opcode(bytes) => {
                        if seen_fetch {
                            return Err(GenError::new(format!("instruction `{}`: opcode bytes after an operand fetch are not supported yet", i.name)));
                        }
                        for &b in bytes {
                            let pos = match node.children.iter().position(|(k, _)| *k == b) {
                                Some(p) => p,
                                None => {
                                    node.children.push((
                                        b,
                                        Node {
                                            children: Vec::new(),
                                            leaf: None,
                                        },
                                    ));
                                    node.children.len() - 1
                                }
                            };
                            node = &mut node.children[pos].1;
                        }
                    }
                    hir::EncodingStep::Fetch { .. } => seen_fetch = true,
                }
            }
            node.leaf = Some(idx);
        }

        fn emit_node(
            g: &Gen,
            node: &Node,
            depth: usize,
            out: &mut String,
            undefined: &str,
        ) -> R<()> {
            if let Some(idx) = node.leaf {
                let i = &g.isa.instructions[idx];
                // Fetch operands in encoding order.
                let mut fetched: Vec<Option<String>> = vec![None; i.encoding.len()];
                for (si, step) in i.encoding.iter().enumerate() {
                    if let hir::EncodingStep::Fetch { operand, .. } = step {
                        let var = format!("f{si}");
                        let _ = writeln!(
                            out,
                            "let {var} = self.fetch_{}()?;",
                            ident(&g.prog.item_name(*operand))
                        );
                        fetched[si] = Some(var);
                    }
                }
                let mut fields = Vec::new();
                for b in &i.operands {
                    let v = match &b.source {
                        hir::BindSource::From { step } => fetched[*step as usize].clone().unwrap(),
                        hir::BindSource::Select { step, field } => {
                            let hir::EncodingStep::Fetch { operand, .. } =
                                &i.encoding[*step as usize]
                            else {
                                unreachable!()
                            };
                            let o = g.operands[operand];
                            format!(
                                "{}.{}",
                                fetched[*step as usize].clone().unwrap(),
                                ident(&o.fields[*field as usize].name)
                            )
                        }
                    };
                    fields.push(format!("{}: {v}", ident(&b.name)));
                }
                let _ = writeln!(
                    out,
                    "Ok(Instr::{}({} {{ {} }}))",
                    ident(&i.name),
                    Gen::ops_struct_name(i),
                    fields.join(", ")
                );
                return Ok(());
            }
            let _ = writeln!(
                out,
                "let b{depth} = self.istream_next()?;\nmatch b{depth} {{"
            );
            for (byte, child) in &node.children {
                let _ = writeln!(out, "{byte:#04x} => {{");
                emit_node(g, child, depth + 1, out, undefined)?;
                out.push_str("}\n");
            }
            let _ = writeln!(out, "_ => {undefined},\n}}");
            Ok(())
        }

        let mut body = String::new();
        emit_node(self, &root, 0, &mut body, &undefined)?;
        let _ = write!(
            self.out,
            "    pub fn decode(&mut self) -> Result<Instr, Fault> {{\n{body}}}\n"
        );

        // exec dispatch, run (with rollback), step, run_bytes.
        let mut s = String::from(
            "    pub fn exec(&mut self, i: Instr) -> Result<(), Fault> {\n        match i {\n",
        );
        for i in &self.isa.instructions {
            let _ = writeln!(
                s,
                "            Instr::{n}(ops) => self.exec_{n}(ops),",
                n = ident(&i.name)
            );
        }
        if self.isa.instructions.is_empty() {
            s.push_str("            _ => Ok(()),\n");
        }
        s.push_str("        }\n    }\n");
        s.push_str(
            "    /// Execute one decoded instruction with fault rollback: a `Fault`-class fault restores the registers.\n    pub fn run(&mut self, i: Instr) -> Result<(), Fault> {\n        let saved_r = self.r;\n        let saved_mode = self.mode;\n        let r = self.exec(i);\n        if let Err(f) = &r {\n            if f.class() == rt::FaultClass::Fault { self.r = saved_r; self.mode = saved_mode; }\n        }\n        r\n    }\n",
        );
        s.push_str(
            "    /// Decode and execute one instruction from the instruction stream.\n    pub fn step(&mut self) -> Result<(), Fault> {\n        let saved_r = self.r;\n        let saved_mode = self.mode;\n        let r = self.decode().and_then(|i| self.exec(i));\n        if let Err(f) = &r {\n            if f.class() == rt::FaultClass::Fault { self.r = saved_r; self.mode = saved_mode; }\n        }\n        r\n    }\n",
        );
        let _ = write!(
            s,
            "    /// Place `bytes` at the instruction stream address and execute one instruction.\n    pub fn run_bytes(&mut self, bytes: &[u8]) -> Result<(), Fault> {{\n        let a = self.fn_{address}()? as u64;\n        for (i, b) in bytes.iter().enumerate() {{ self.{space}.write_cell(a + i as u64, *b as u128); }}\n        self.step()\n    }}\n"
        );
        self.out.push_str(&s);
        Ok(())
    }

    fn emit_tests(&mut self) -> R<()> {
        let isa_name = ident(&self.isa.name);
        let tests: Vec<&hir::TestBody> = self
            .hir
            .tests
            .iter()
            .filter(|t| t.isa == Some(self.isa.def))
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
            };
            let mut body = format!("let mut m = {isa_name}::new();\n");
            for s in &t.body.stmts {
                match &s.kind {
                    S::Given(pairs) => {
                        for (place, v) in pairs {
                            body.push_str(&self.assign(&cx, place, &self.expr(&cx, v)?)?);
                        }
                    }
                    S::Expect(checks) => {
                        for c in checks {
                            let src = file.slice(c.span).replace('"', "'");
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
                            // Each lookup borrows `m` mutably; hoist them.
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
}
