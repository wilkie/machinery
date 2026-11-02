//! Checking an `isa`: address spaces, registers, mode, faults, operands,
//! operand kinds, functions and instructions.
//!
//! Order matters: signatures (types of everything a body might name) are
//! resolved before any body is checked, so bodies may refer to each other
//! freely. Within one isa the passes run: spaces, registers, mode, faults,
//! operands (including tails), operand-kind signatures, then the bodies of
//! kind rules, functions and instructions.

use std::collections::HashMap;
use std::rc::Rc;

use machine_syntax::ast::{self, ExprKind as A, InstructionItem};
use machine_syntax::diagnostics::Diagnostic;
use machine_syntax::span::Span;

use super::{Checker, FnCtx, InstrSig, KindSig, Scope};
use crate::consteval::ConstVal;
use crate::hir::{self, ExprKind as H, RuleKind};
use crate::program::{DefId, ItemKind, ScopeEntry};
use crate::ty::Ty;

impl Checker<'_> {
    pub(super) fn check_isa(&mut self, isa: DefId) {
        let module = self.prog.item(isa).module;
        let scope = Scope {
            module,
            isa: Some(isa),
            bus: None,
        };
        let name = self.prog.item_name(isa);

        // Nested types and constants are lazy; force them so their errors
        // appear in a predictable order.
        for id in self.prog.isa_items_of(isa, ItemKind::Enum) {
            self.enum_info(id);
        }
        for id in self.prog.isa_items_of(isa, ItemKind::Const) {
            self.const_info(id);
        }
        for id in self.prog.isa_items_of(isa, ItemKind::Bundle) {
            self.bundle_info(id);
        }

        let mut spaces = Vec::new();
        for id in self.prog.isa_items_of(isa, ItemKind::Space) {
            if let Some(s) = self.check_space(scope, id) {
                let rc = Rc::new(s);
                self.spaces.insert(id, rc.clone());
                spaces.push((*rc).clone());
            }
        }

        let mut registers = Vec::new();
        for id in self.prog.isa_items_of(isa, ItemKind::Register) {
            if let Some(r) = self.check_register(scope, id) {
                let rc = Rc::new(r);
                self.registers.insert(id, rc.clone());
                registers.push((*rc).clone());
            }
        }

        let mut mode = None;
        let modes: Vec<DefId> = self.prog.isa_items_of(isa, ItemKind::Mode).collect();
        if modes.len() > 1 {
            let span = self.prog.item(modes[1]).span;
            self.error(module, "E1001", span, "an isa has at most one `mode`");
        }
        if let Some(&m) = modes.first() {
            if self.enum_info(m).is_some() {
                mode = Some(m);
                self.isa_modes.insert(isa, m);
            }
        }

        let mut latches = Vec::new();
        for id in self.prog.isa_items_of(isa, ItemKind::Latch) {
            if let Some(l) = self.check_latch(scope, id) {
                let rc = Rc::new(l);
                self.latches.insert(id, rc.clone());
                latches.push((*rc).clone());
            }
        }

        let mut faults = Vec::new();
        for id in self.prog.isa_items_of(isa, ItemKind::Fault) {
            if let Some(f) = self.check_fault(scope, id) {
                let rc = Rc::new(f);
                self.faults.insert(id, rc.clone());
                faults.push((*rc).clone());
            }
        }

        let mut operands = Vec::new();
        for id in self.prog.isa_items_of(isa, ItemKind::Operand) {
            if let Some(o) = self.check_operand(scope, id) {
                let rc = Rc::new(o);
                self.operands.insert(id, rc.clone());
                operands.push((*rc).clone());
            }
        }

        // Kind signatures first, bodies after.
        let kind_ids: Vec<DefId> = self.prog.isa_items_of(isa, ItemKind::OperandKind).collect();
        for &id in &kind_ids {
            if let Some(sig) = self.kind_sig(scope, id) {
                self.kinds.insert(id, Rc::new(sig));
            }
        }
        for id in self.prog.isa_items_of(isa, ItemKind::Fn) {
            self.fn_sig(id);
        }
        // Instruction signatures (operand lists) for tests and dispatch.
        let instr_ids: Vec<DefId> = self.prog.isa_items_of(isa, ItemKind::Instruction).collect();
        for &id in &instr_ids {
            let sig = self.instr_sig(scope, id);
            self.instr_sigs.insert(id, Rc::new(sig));
        }

        let mut kinds = Vec::new();
        for &id in &kind_ids {
            if let Some(k) = self.check_kind_bodies(scope, id) {
                kinds.push(k);
            }
        }

        let mut fns = Vec::new();
        for id in self.prog.isa_items_of(isa, ItemKind::Fn) {
            if let Some(f) = self.check_fn(scope, id) {
                if f.name == "reset" && (!f.params.is_empty() || f.ret != Ty::Unit) {
                    let span = self.prog.item(id).span;
                    self.error(
                        scope.module,
                        "E1046",
                        span,
                        "`fn reset()` is the processor's reset: it takes no parameters and returns nothing",
                    );
                }
                fns.push(f);
            }
        }

        let mut instructions = Vec::new();
        for &id in &instr_ids {
            if let Some(i) = self.check_instruction(scope, id) {
                instructions.push(i);
            }
        }
        self.check_opcode_conflicts(scope, &instructions);

        let istream = self.check_istream(scope, isa);

        self.isas.push(hir::IsaInfo {
            def: isa,
            name,
            spaces,
            registers,
            mode,
            faults,
            fns,
            operands,
            kinds,
            instructions,
            istream,
            latches,
            fault_cycles: None,
            interrupt_cycles: None,
        });
    }

    /// No byte sequence may decode to two instructions (or prefixes). Two
    /// leading opcode patterns overlap when some concrete bytes match both;
    /// overlapping instructions must then fetch the same operand first and
    /// be told apart by disjoint field constraints on it.
    fn check_opcode_conflicts(&mut self, scope: Scope, instructions: &[hir::Instruction]) {
        fn lead(i: &hir::Instruction) -> Vec<hir::OpcodeByte> {
            let mut bytes = Vec::new();
            for step in &i.encoding {
                match step {
                    hir::EncodingStep::Opcode { bytes: b, .. } => bytes.extend_from_slice(b),
                    hir::EncodingStep::Fetch { .. } => break,
                }
            }
            bytes
        }
        fn first_fetch(i: &hir::Instruction) -> Option<(DefId, &[(u32, i128)])> {
            i.encoding.iter().find_map(|s| match s {
                hir::EncodingStep::Fetch {
                    operand,
                    constraints,
                } => Some((*operand, constraints.as_slice())),
                _ => None,
            })
        }
        let overlap = |a: &[hir::OpcodeByte], b: &[hir::OpcodeByte]| {
            a.iter()
                .zip(b)
                .all(|(x, y)| (x.value ^ y.value) & x.mask & y.mask == 0)
        };
        for (ai, ia) in instructions.iter().enumerate() {
            let la = lead(ia);
            for ib in &instructions[..ai] {
                let lb = lead(ib);
                if !overlap(&la, &lb) {
                    continue;
                }
                let distinguished = la.len() == lb.len()
                    && !ia.prefix
                    && !ib.prefix
                    && match (first_fetch(ia), first_fetch(ib)) {
                        (Some((oa, ca)), Some((ob, cb))) if oa == ob => ca
                            .iter()
                            .any(|(fa, va)| cb.iter().any(|(fb, vb)| fa == fb && va != vb)),
                        _ => false,
                    };
                if distinguished {
                    continue;
                }
                let span = self.prog.item(ia.def).span;
                let msg = format!(
                    "opcode {} of `{}` overlaps {} of `{}`",
                    pattern_text(&la),
                    ia.name,
                    pattern_text(&lb),
                    ib.name
                );
                self.error_with(
                    scope.module,
                    Diagnostic::error("E1045", span, msg)
                        .with_label(self.prog.item(ib.def).span, "the other instruction")
                        .with_note("instructions sharing opcode bytes must fetch the same operand and constrain one of its fields to different values, e.g. `ModRM { reg: 0 }`"),
                );
            }
        }
    }

    /// `istream { space; address; next; undefined; }`
    fn check_istream(&mut self, scope: Scope, isa: DefId) -> Option<hir::Istream> {
        let decl = self.prog.isa_decl(isa);
        let module = scope.module;
        let decls: Vec<&ast::IstreamDecl> = decl
            .items
            .iter()
            .filter_map(|i| match i {
                ast::IsaItem::Istream(d) => Some(d),
                _ => None,
            })
            .collect();
        if decls.len() > 1 {
            self.error(module, "E1046", decls[1].span, "an isa has one `istream`");
        }
        let d = *decls.first()?;
        let mut ok = true;
        let space = match self.lookup_item(scope, &d.space.name) {
            Some(s) if self.prog.item(s).kind == ItemKind::Space => Some(s),
            _ => {
                self.error(
                    module,
                    "E1046",
                    d.space.span,
                    format!("`{}` is not an address space", d.space.name),
                );
                None
            }
        };
        let space_info = space.and_then(|s| self.spaces.get(&s).cloned());
        let fn_item = |ck: &mut Self, p: &ast::Path| -> Option<DefId> {
            match ck.lookup_item(scope, &p.last().name) {
                Some(f) if ck.prog.item(f).kind == ItemKind::Fn => Some(f),
                _ => {
                    ck.error(
                        module,
                        "E1046",
                        p.span,
                        format!("`{}` is not an isa `fn`", p.dotted()),
                    );
                    None
                }
            }
        };
        let address = fn_item(self, &d.address);
        let next = fn_item(self, &d.next);
        if let (Some(f), Some(sp)) = (address, &space_info) {
            if let Some(sig) = self.fn_sig(f) {
                if !sig.params.is_empty() || sig.ret != sp.addr {
                    let msg = format!(
                        "`address` is a `fn () -> {}` (the space's address type)",
                        self.display(&sp.addr)
                    );
                    self.error(module, "E1046", d.address.span, msg);
                    ok = false;
                }
            }
        }
        if let (Some(f), Some(sp)) = (next, &space_info) {
            if let Some(sig) = self.fn_sig(f) {
                if !sig.params.is_empty() || sig.ret != Ty::UInt(sp.cell_bits) {
                    let msg = format!(
                        "`next` is a `fn () -> u{}` (one cell of the space)",
                        sp.cell_bits
                    );
                    self.error(module, "E1046", d.next.span, msg);
                    ok = false;
                }
            }
        }
        let undefined = match &d.undefined {
            None => None,
            Some(p) => match self.lookup_item(scope, &p.last().name) {
                Some(f) if self.prog.item(f).kind == ItemKind::Fault => {
                    if self
                        .faults
                        .get(&f)
                        .map(|i| !i.params.is_empty())
                        .unwrap_or(false)
                    {
                        self.error(
                            module,
                            "E1046",
                            p.span,
                            "the `undefined` fault takes no arguments",
                        );
                        ok = false;
                    }
                    Some(f)
                }
                _ => {
                    self.error(
                        module,
                        "E1046",
                        p.span,
                        format!("`{}` is not a fault", p.dotted()),
                    );
                    ok = false;
                    None
                }
            },
        };
        let fault = match &d.fault {
            None => None,
            Some(p) => match fn_item(self, p) {
                Some(f) => {
                    if let Some(sig) = self.fn_sig(f) {
                        let good = sig.params.len() == 2
                            && sig.params[0].1 == Ty::UInt(8)
                            && sig.params[1].1 == Ty::Bool
                            && sig.ret == Ty::Unit;
                        if !good {
                            self.error(module, "E1046", p.span, "`fault` is a `fn (vector: u8, restart: bool)` that delivers the fault");
                            ok = false;
                        }
                    }
                    Some(f)
                }
                None => {
                    ok = false;
                    None
                }
            },
        };
        let boundary = match &d.boundary {
            None => None,
            Some(p) => match fn_item(self, p) {
                Some(f) => {
                    if let Some(sig) = self.fn_sig(f) {
                        if !sig.params.is_empty() || sig.ret != Ty::Bool {
                            self.error(module, "E1046", p.span, "`boundary` is a `fn () -> bool`: run at each instruction boundary, `true` when it took an interrupt");
                            ok = false;
                        }
                    }
                    Some(f)
                }
                None => {
                    ok = false;
                    None
                }
            },
        };
        let (Some(space), Some(address), Some(next)) = (space, address, next) else {
            return None;
        };
        ok.then_some(hir::Istream {
            space,
            address,
            next,
            undefined,
            fault,
            boundary,
        })
    }

    // -- spaces ------------------------------------------------------------------

    fn check_space(&mut self, scope: Scope, id: DefId) -> Option<hir::Space> {
        let decl = self.prog.space_decl(id);
        let empty = HashMap::new();
        let addr = self.resolve_type(scope, &empty, &decl.addr);
        let cell = self.resolve_type(scope, &empty, &decl.cell);
        let mut ok = true;
        if !matches!(addr, Ty::UInt(_) | Ty::Error) {
            let msg = format!(
                "address type must be `u<N>`, found `{}`",
                self.display(&addr)
            );
            self.error(scope.module, "E1003", decl.addr.span, msg);
            ok = false;
        }
        let cell_bits = match cell {
            Ty::UInt(w) => w,
            Ty::Error => return None,
            t => {
                let msg = format!("cell type must be `u<N>`, found `{}`", self.display(&t));
                self.error(scope.module, "E1003", decl.cell.span, msg);
                return None;
            }
        };
        let mut little_endian = true;
        for a in &decl.attrs {
            match a.name.as_str() {
                "little_endian" => little_endian = true,
                "big_endian" => little_endian = false,
                other => {
                    self.error(
                        scope.module,
                        "E1031",
                        a.span,
                        format!("unknown address space attribute `{other}`"),
                    );
                    ok = false;
                }
            }
        }
        if addr.is_error() || !ok {
            return None;
        }
        Some(hir::Space {
            def: id,
            name: decl.name.name.clone(),
            addr,
            cell_bits,
            little_endian,
        })
    }

    // -- registers -------------------------------------------------------------------

    fn check_register(&mut self, scope: Scope, id: DefId) -> Option<hir::Register> {
        let decl = self.prog.register_decl(id);
        let module = scope.module;
        let empty = HashMap::new();
        let ty = self.resolve_type(scope, &empty, &decl.ty);
        if ty.is_error() {
            return None;
        }
        let reg_width = self.width_of(&ty);
        let mut ok = true;
        let mut fields = Vec::new();
        if !decl.fields.is_empty() && !matches!(ty, Ty::UInt(_)) {
            let msg = format!(
                "only `u<N>` registers have bit fields; `{}` is `{}`",
                decl.name.name,
                self.display(&ty)
            );
            self.error(module, "E1003", decl.ty.span, msg);
            ok = false;
        }
        for f in &decl.fields {
            let fty = self.resolve_type(scope, &empty, &f.ty);
            let offset = match self.eval_const(scope, &empty, &f.offset) {
                Some(ConstVal::Int(v)) if v >= 0 => v as u32,
                Some(_) => {
                    self.error(
                        module,
                        "E1013",
                        f.offset.span,
                        "field offset must be a non-negative integer",
                    );
                    ok = false;
                    0
                }
                None => {
                    ok = false;
                    0
                }
            };
            if let (Some(fw), Some(rw)) = (self.width_of(&fty), reg_width) {
                if offset + fw > rw {
                    let msg = format!("field `{}` ({fw} bits at offset {offset}) does not fit in the {rw}-bit register", f.name.name);
                    self.error(module, "E1017", f.span, msg);
                    ok = false;
                }
            }
            if fty.is_error() {
                ok = false;
            }
            fields.push(hir::RegField {
                name: f.name.name.clone(),
                ty: fty,
                offset,
            });
        }
        let mut views = Vec::new();
        for v in &decl.views {
            let vty = self.resolve_type(scope, &empty, &v.ty);
            let Ty::Bundle(bdef) = vty else {
                if !vty.is_error() {
                    let msg = format!(
                        "a view's type must be a bundle, found `{}`",
                        self.display(&vty)
                    );
                    self.error(module, "E1035", v.ty.span, msg);
                }
                ok = false;
                continue;
            };
            let Some(binfo) = self.bundle_info(bdef) else {
                ok = false;
                continue;
            };
            let mut mapping = vec![u32::MAX; binfo.fields.len()];
            for m in &v.mapping {
                let Some(bi) = binfo.fields.iter().position(|bf| bf.name == m.name.name) else {
                    let msg = format!(
                        "bundle `{}` has no field `{}`",
                        self.prog.item_name(bdef),
                        m.name.name
                    );
                    self.error(module, "E1008", m.name.span, msg);
                    ok = false;
                    continue;
                };
                let Some(A::Path(p)) = m.value.as_ref().map(|e| &e.kind) else {
                    self.error(
                        module,
                        "E1035",
                        m.span,
                        "a view maps each bundle field to a field of this register: `cf: CF`",
                    );
                    ok = false;
                    continue;
                };
                let target = p.segments.last().unwrap();
                let Some(ri) = fields.iter().position(|rf| rf.name == target.name) else {
                    let msg = format!(
                        "register `{}` has no field `{}`",
                        decl.name.name, target.name
                    );
                    self.error(module, "E1035", p.span, msg);
                    ok = false;
                    continue;
                };
                if fields[ri].ty != binfo.fields[bi].ty {
                    let msg = format!(
                        "view field `{}` is `{}` but register field `{}` is `{}`",
                        m.name.name,
                        self.display(&binfo.fields[bi].ty),
                        target.name,
                        self.display(&fields[ri].ty)
                    );
                    self.error(module, "E1035", m.span, msg);
                    ok = false;
                }
                mapping[bi] = ri as u32;
            }
            let missing: Vec<&str> = binfo
                .fields
                .iter()
                .zip(&mapping)
                .filter(|(_, m)| **m == u32::MAX)
                .map(|(f, _)| f.name.as_str())
                .collect();
            if !missing.is_empty() {
                let msg = format!(
                    "view `{}` does not map bundle field(s): {}",
                    v.name.name,
                    missing.join(", ")
                );
                self.error(module, "E1035", v.span, msg);
                ok = false;
            }
            views.push(hir::RegView {
                name: v.name.name.clone(),
                ty: Ty::Bundle(bdef),
                mapping,
            });
        }
        if !ok {
            return None;
        }
        Some(hir::Register {
            def: id,
            name: decl.name.name.clone(),
            ty,
            fields,
            views,
        })
    }

    // -- faults -----------------------------------------------------------------------

    fn check_fault(&mut self, scope: Scope, id: DefId) -> Option<hir::Fault> {
        let decl = self.prog.fault_decl(id);
        let module = scope.module;
        let empty = HashMap::new();
        let mut ok = true;
        let mut params = Vec::new();
        for p in &decl.params {
            let ty = self.resolve_type(scope, &empty, &p.ty);
            ok &= !ty.is_error();
            params.push((p.name.name.clone(), ty));
        }
        let mut vector = None;
        let mut class = hir::FaultClass::Fault;
        let mut pushes_error_code = false;
        let mut seen_class = false;
        for a in &decl.attrs {
            let Some(value) = &a.value else {
                self.error(
                    module,
                    "E1038",
                    a.span,
                    "fault attributes take values: `vector: 13`",
                );
                ok = false;
                continue;
            };
            match a.name.name.as_str() {
                "vector" => match self.eval_const(scope, &empty, value) {
                    Some(ConstVal::Int(v)) if v >= 0 => vector = Some(v),
                    Some(_) => {
                        self.error(
                            module,
                            "E1038",
                            value.span,
                            "`vector` must be a non-negative integer",
                        );
                        ok = false;
                    }
                    None => ok = false,
                },
                "class" => {
                    seen_class = true;
                    let word = match &value.kind {
                        A::Path(p) if p.segments.len() == 1 => Some(p.segments[0].name.as_str()),
                        _ => None,
                    };
                    class = match word {
                        Some("Fault") => hir::FaultClass::Fault,
                        Some("Trap") => hir::FaultClass::Trap,
                        Some("Abort") => hir::FaultClass::Abort,
                        _ => {
                            self.error(
                                module,
                                "E1038",
                                value.span,
                                "`class` is one of `Fault`, `Trap`, `Abort`",
                            );
                            ok = false;
                            hir::FaultClass::Fault
                        }
                    };
                }
                "pushes_error_code" => match self.eval_const(scope, &empty, value) {
                    Some(ConstVal::Bool(b)) => pushes_error_code = b,
                    Some(_) => {
                        self.error(
                            module,
                            "E1038",
                            value.span,
                            "`pushes_error_code` is `true` or `false`",
                        );
                        ok = false;
                    }
                    None => ok = false,
                },
                other => {
                    self.error(
                        module,
                        "E1038",
                        a.name.span,
                        format!("unknown fault attribute `{other}`"),
                    );
                    ok = false;
                }
            }
        }
        if !seen_class {
            self.warning(
                module,
                "W1003",
                decl.name.span,
                format!(
                    "fault `{}` does not state its `class`; assuming `Fault`",
                    decl.name.name
                ),
            );
        }
        if !ok {
            return None;
        }
        Some(hir::Fault {
            def: id,
            name: decl.name.name.clone(),
            params,
            vector,
            class,
            pushes_error_code,
        })
    }

    // -- operands ----------------------------------------------------------------------

    fn check_operand(&mut self, scope: Scope, id: DefId) -> Option<hir::Operand> {
        let decl = self.prog.operand_decl(id);
        let module = scope.module;
        let empty = HashMap::new();
        let mut ok = true;
        let size = match &decl.size {
            Some(e) => match self.eval_const(scope, &empty, e) {
                Some(ConstVal::Int(n)) if n >= 1 => n as u64,
                Some(_) => {
                    self.error(
                        module,
                        "E1036",
                        e.span,
                        "operand size is a positive number of bytes",
                    );
                    ok = false;
                    1
                }
                None => {
                    ok = false;
                    1
                }
            },
            None => {
                self.error(
                    module,
                    "E1036",
                    decl.name.span,
                    format!("operand `{}` needs a `size` in bytes", decl.name.name),
                );
                ok = false;
                1
            }
        };
        let mut fields = Vec::new();
        for f in &decl.fields {
            let ty = self.resolve_type(scope, &empty, &f.ty);
            let offset = match self.eval_const(scope, &empty, &f.offset) {
                Some(ConstVal::Int(v)) if v >= 0 => v as u32,
                _ => {
                    self.error(
                        module,
                        "E1013",
                        f.offset.span,
                        "field offset must be a non-negative integer",
                    );
                    ok = false;
                    0
                }
            };
            if let Some(w) = self.width_of(&ty) {
                if (offset + w) as u64 > size * 8 {
                    let msg = format!(
                        "field `{}` ({w} bits at {offset}) does not fit in {size} byte(s)",
                        f.name.name
                    );
                    self.error(module, "E1017", f.span, msg);
                    ok = false;
                }
            }
            if fields
                .iter()
                .any(|x: &hir::OperandField| x.name == f.name.name)
            {
                self.error(
                    module,
                    "E1001",
                    f.name.span,
                    format!("duplicate field `{}`", f.name.name),
                );
                ok = false;
            }
            ok &= !ty.is_error();
            fields.push(hir::OperandField {
                name: f.name.name.clone(),
                ty,
                offset,
            });
        }

        // The tail expression sees the operand's fields as locals.
        let mut tail = None;
        let mut tail_locals = Vec::new();
        let mut tail_set = Vec::new();
        if let Some(block) = &decl.tail {
            let mut cx = FnCtx::new(scope, HashMap::new(), None);
            for f in &fields {
                cx.declare(&f.name, f.ty.clone(), decl.span);
            }
            let stmts: Vec<&ast::Stmt> = block
                .stmts
                .iter()
                .filter(|s| !matches!(s.kind, ast::StmtKind::Error))
                .collect();
            if stmts.len() != 1 {
                self.error(
                    module,
                    "E1036",
                    block.span,
                    "`tail` is a single expression: an operand name or an `if` chain of them",
                );
                ok = false;
            } else {
                match self.check_tail_stmt(&mut cx, stmts[0], &mut tail_set) {
                    Some(t) => tail = Some(t),
                    None => ok = false,
                }
            }
            tail_locals = cx.locals;
        }
        if !ok {
            return None;
        }
        Some(hir::Operand {
            def: id,
            name: decl.name.name.clone(),
            size,
            fields,
            tail_locals,
            tail,
            tail_set,
        })
    }

    /// A tail branch as the parser produces it: an `if` without `else` is a
    /// statement, an `if` with one or a bare name is an expression statement.
    fn check_tail_stmt(
        &mut self,
        cx: &mut FnCtx,
        s: &ast::Stmt,
        set: &mut Vec<DefId>,
    ) -> Option<hir::TailExpr> {
        match &s.kind {
            ast::StmtKind::Expr(e) => self.check_tail_expr(cx, e, set),
            ast::StmtKind::If {
                cond,
                then_branch,
                else_branch,
            } => {
                let cond = self.check_expr(cx, cond, Some(&Ty::Bool));
                let then_e = self.tail_block(cx, then_branch, set);
                let else_e = match else_branch {
                    Some(else_s) => Some(Box::new(self.check_tail_stmt(cx, else_s, set)?)),
                    None => None,
                };
                Some(hir::TailExpr::If(cond, Box::new(then_e?), else_e))
            }
            _ => {
                self.error(
                    cx.module,
                    "E1036",
                    s.span,
                    "`tail` is a single expression: an operand name or an `if` chain of them",
                );
                None
            }
        }
    }

    fn check_tail_expr(
        &mut self,
        cx: &mut FnCtx,
        e: &ast::Expr,
        set: &mut Vec<DefId>,
    ) -> Option<hir::TailExpr> {
        match &e.kind {
            A::Path(p) if p.segments.len() == 1 => {
                let name = &p.segments[0].name;
                match self.lookup_item(cx.scope, name) {
                    Some(def) if self.prog.item(def).kind == ItemKind::Operand => {
                        if !set.contains(&def) {
                            set.push(def);
                        }
                        Some(hir::TailExpr::Operand(def))
                    }
                    _ => {
                        self.error(
                            cx.module,
                            "E1036",
                            e.span,
                            format!("`{name}` is not an operand"),
                        );
                        None
                    }
                }
            }
            A::If {
                cond,
                then_branch,
                else_branch,
            } => {
                let cond = self.check_expr(cx, cond, Some(&Ty::Bool));
                let then_e = self.tail_block(cx, then_branch, set);
                let else_e = match else_branch {
                    Some(else_e) => match &else_e.kind {
                        A::Block(b) => self.tail_block(cx, b, set).map(Box::new),
                        _ => self.check_tail_expr(cx, else_e, set).map(Box::new),
                    },
                    None => None,
                };
                let then_e = then_e?;
                if else_branch.is_some() && else_e.is_none() {
                    return None;
                }
                Some(hir::TailExpr::If(cond, Box::new(then_e), else_e))
            }
            A::Block(b) => self.tail_block(cx, b, set),
            _ => {
                self.error(
                    cx.module,
                    "E1036",
                    e.span,
                    "`tail` branches name an operand (`Disp8`) or nest another `if`",
                );
                None
            }
        }
    }

    fn tail_block(
        &mut self,
        cx: &mut FnCtx,
        b: &ast::Block,
        set: &mut Vec<DefId>,
    ) -> Option<hir::TailExpr> {
        if b.stmts.len() != 1 {
            self.error(
                cx.module,
                "E1036",
                b.span,
                "a `tail` branch is a single operand name or `if`",
            );
            return None;
        }
        self.check_tail_stmt(cx, &b.stmts[0], set)
    }

    // -- operand kinds -------------------------------------------------------------------

    fn kind_sig(&mut self, scope: Scope, id: DefId) -> Option<KindSig> {
        let decl = self.prog.operand_kind_decl(id);
        let module = scope.module;
        let empty = HashMap::new();
        let ty = self.resolve_type(scope, &empty, &decl.ty);
        let mut ok = !ty.is_error();
        let source = match &decl.from {
            Some((_, path)) => {
                let name = path.last().name.clone();
                match self.lookup_item(scope, &name) {
                    Some(def) if self.prog.item(def).kind == ItemKind::Operand => Some(def),
                    _ => {
                        self.error(
                            module,
                            "E1037",
                            path.span,
                            format!("`{name}` is not an operand"),
                        );
                        ok = false;
                        None
                    }
                }
            }
            None => None,
        };
        let mut rules = Vec::new();
        for r in &decl.rules {
            let kind = match r.name.name.as_str() {
                "select" => RuleKind::Select,
                "value" => RuleKind::Value,
                "read" => RuleKind::Read,
                "write" => RuleKind::Write,
                "is_mem" => RuleKind::IsMem,
                other => {
                    self.error_with(
                        module,
                        Diagnostic::error(
                            "E1037",
                            r.name.span,
                            format!("unknown operand kind rule `{other}`"),
                        )
                        .with_note(
                            "rules are `select(n)`, `value(n)`, `read`, `write(v)` and `is_mem`",
                        ),
                    );
                    ok = false;
                    continue;
                }
            };
            if rules.iter().any(|(_, k, _)| *k == kind) {
                self.error(
                    module,
                    "E1001",
                    r.name.span,
                    format!("rule `{}` given twice", r.name.name),
                );
                ok = false;
            }
            let mut params = Vec::new();
            for p in &r.params {
                let pty = self.resolve_type(scope, &empty, &p.ty);
                ok &= !pty.is_error();
                params.push((p.name.name.clone(), pty));
            }
            match kind {
                RuleKind::Select | RuleKind::Value => {
                    if params.len() != 1 {
                        let msg = format!(
                            "`{}` takes exactly one parameter: the encoding field",
                            r.name.name
                        );
                        self.error(module, "E1037", r.span, msg);
                        ok = false;
                    } else if !matches!(params[0].1, Ty::UInt(_) | Ty::Error) {
                        let msg = format!(
                            "an encoding field is `u<N>`, not `{}`",
                            self.display(&params[0].1)
                        );
                        self.error(module, "E1037", r.params[0].span, msg);
                        ok = false;
                    }
                }
                RuleKind::Write => {
                    if params.len() != 1 {
                        self.error(
                            module,
                            "E1037",
                            r.span,
                            "`write` takes exactly one parameter: the value",
                        );
                        ok = false;
                    } else if params[0].1 != ty && !params[0].1.is_error() {
                        let msg = format!(
                            "`write` parameter is `{}` but the kind's type is `{}`",
                            self.display(&params[0].1),
                            self.display(&ty)
                        );
                        self.error(module, "E1037", r.params[0].span, msg);
                        ok = false;
                    }
                }
                RuleKind::Read | RuleKind::IsMem if !params.is_empty() => {
                    self.error(
                        module,
                        "E1037",
                        r.span,
                        format!("`{}` takes no parameters", r.name.name),
                    );
                    ok = false;
                }
                _ => {}
            }
            rules.push((r.name.name.clone(), kind, params));
        }
        let has = |k: RuleKind| rules.iter().any(|(_, rk, _)| *rk == k);
        if source.is_some() {
            if has(RuleKind::Select) || has(RuleKind::Value) {
                self.error(module, "E1037", decl.name.span, "a kind with `from` is built from an operand and cannot have `select` or `value`");
                ok = false;
            }
            if !has(RuleKind::Read) {
                self.error(
                    module,
                    "E1037",
                    decl.name.span,
                    format!("kind `{}` needs a `read` rule", decl.name.name),
                );
                ok = false;
            }
        } else {
            let field_rules = has(RuleKind::Select) as u32 + has(RuleKind::Value) as u32;
            if field_rules == 0 {
                self.error_with(
                    module,
                    Diagnostic::error(
                        "E1037",
                        decl.name.span,
                        format!("kind `{}` has no `from` operand, so it needs a `select` or `value` rule", decl.name.name),
                    )
                    .with_note("`select(n)` names a register by an encoding field; `value(n)` turns the field into a value"),
                );
                ok = false;
            }
            if field_rules == 2 {
                self.error(
                    module,
                    "E1037",
                    decl.name.span,
                    "a kind has either `select` or `value`, not both",
                );
                ok = false;
            }
            if has(RuleKind::Read) || has(RuleKind::Write) || has(RuleKind::IsMem) {
                self.error(
                    module,
                    "E1037",
                    decl.name.span,
                    "a field kind has no `read`, `write` or `is_mem` rules",
                );
                ok = false;
            }
        }
        ok.then_some(KindSig { ty, source, rules })
    }

    fn check_kind_bodies(&mut self, scope: Scope, id: DefId) -> Option<hir::OperandKind> {
        let sig = self.kinds.get(&id)?.clone();
        let decl = self.prog.operand_kind_decl(id);
        let module = scope.module;
        let mut rules = Vec::new();
        for r in &decl.rules {
            let Some((_, (_, kind, params))) = sig
                .rules
                .iter()
                .enumerate()
                .find(|(_, (n, _, _))| *n == r.name.name)
            else {
                continue;
            };
            let ret = match kind {
                RuleKind::Select | RuleKind::Read | RuleKind::Value => Some(sig.ty.clone()),
                RuleKind::IsMem => Some(Ty::Bool),
                RuleKind::Write => Some(Ty::Unit),
            };
            let mut cx = FnCtx::new(scope, HashMap::new(), ret.clone());
            let mut param_ids = Vec::new();
            if let (Some((src_name, _)), Some(src_def)) = (&decl.from, sig.source) {
                param_ids.push(cx.declare(&src_name.name, Ty::Operand(src_def), src_name.span));
            }
            for (p, (name, ty)) in r.params.iter().zip(params) {
                param_ids.push(cx.declare(name, ty.clone(), p.name.span));
            }
            let body = match &r.body {
                ast::ArmBody::Expr(e) => {
                    let expected = match kind {
                        RuleKind::Write => None,
                        _ => ret.as_ref(),
                    };
                    let he = self.check_expr(&mut cx, e, expected);
                    if *kind == RuleKind::Select && !he.ty.is_error() && !hir::is_place(&he) {
                        self.error_with(
                            module,
                            Diagnostic::error("E1030", e.span, "`select` must yield a place (a register or register field)")
                                .with_note("every branch of the expression must be assignable, e.g. `match n { 0 => AL, 1 => CL }`"),
                        );
                    }
                    hir::ArmBody::Expr(he)
                }
                ast::ArmBody::Block(b) => {
                    let expected = match kind {
                        RuleKind::Write => None,
                        _ => ret.as_ref(),
                    };
                    let (mut hb, bty) = self.check_block_expr(&mut cx, b, expected);
                    if let Some(want) = &ret {
                        if *kind != RuleKind::Write
                            && bty != *want
                            && !bty.is_error()
                            && !matches!(bty, Ty::Never)
                        {
                            let tail_ty = bty.clone();
                            self.coerce_block_tail(&cx, &mut hb, want);
                            if !matches!(tail_ty, Ty::Const(_)) {
                                self.mismatch(module, b.span, want, &tail_ty);
                            }
                        }
                    }
                    hir::ArmBody::Block(hb)
                }
            };
            rules.push(hir::KindRule {
                name: r.name.name.clone(),
                kind: *kind,
                locals: cx.locals,
                params: param_ids,
                body,
            });
        }
        Some(hir::OperandKind {
            def: id,
            name: decl.name.name.clone(),
            ty: sig.ty.clone(),
            source: sig.source,
            rules,
        })
    }

    // -- latches ------------------------------------------------------------------------------

    fn check_latch(&mut self, scope: Scope, id: DefId) -> Option<hir::Latch> {
        let decl = self.prog.latch_decl(id);
        let empty = HashMap::new();
        let ty = self.resolve_type(scope, &empty, &decl.ty);
        if ty.is_error() {
            return None;
        }
        let mut cx = FnCtx::new(scope, HashMap::new(), None);
        let init = self.check_expr(&mut cx, &decl.init, Some(&ty));
        if init.ty.is_error() {
            return None;
        }
        // A latch initialiser is evaluated at every instruction boundary; it
        // may read architectural state (`latch start_ip: u16 = IP;`) but
        // must not fault.
        if contains_call(&init) {
            self.error(
                scope.module,
                "E1047",
                decl.init.span,
                "a latch initialiser cannot call functions; it runs at every instruction boundary",
            );
            return None;
        }
        Some(hir::Latch {
            def: id,
            name: decl.name.name.clone(),
            ty,
            init,
        })
    }

    // -- functions ----------------------------------------------------------------------------

    fn check_fn(&mut self, scope: Scope, id: DefId) -> Option<hir::FnBody> {
        let sig = self.fn_sig(id)?;
        let decl = self.prog.fn_decl(id);
        let module = scope.module;
        let mut cx = FnCtx::new(scope, HashMap::new(), Some(sig.ret.clone()));
        let mut params = Vec::new();
        for (p, (name, ty)) in decl.params.iter().zip(&sig.params) {
            if cx.lookup_local(name).is_some() {
                self.error(
                    module,
                    "E1001",
                    p.name.span,
                    format!("duplicate parameter `{name}`"),
                );
            }
            params.push(cx.declare(name, ty.clone(), p.name.span));
        }
        if decl.external {
            return Some(hir::FnBody {
                def: id,
                name: decl.name.name.clone(),
                locals: cx.locals,
                params,
                ret: sig.ret.clone(),
                body: hir::Block {
                    stmts: Vec::new(),
                    span: decl.body.span,
                },
                external: true,
            });
        }
        let body = self.check_block(&mut cx, &decl.body);
        if !matches!(sig.ret, Ty::Unit | Ty::Error) && !super::stmt::block_diverges(&body) {
            let msg = format!(
                "fn `{}` does not return a value on every path",
                decl.name.name
            );
            self.error_with(
                module,
                Diagnostic::error("E1015", decl.name.span, msg)
                    .with_note("every path must end in `return`, `raise` or `unreachable(...)`"),
            );
        }
        Some(hir::FnBody {
            def: id,
            name: decl.name.name.clone(),
            locals: cx.locals,
            params,
            ret: sig.ret.clone(),
            body,
            external: false,
        })
    }

    // -- instructions ---------------------------------------------------------------------------

    fn instr_sig(&mut self, scope: Scope, id: DefId) -> InstrSig {
        let decl = self.prog.instruction_decl(id);
        let module = scope.module;
        let mut operands = Vec::new();
        for item in &decl.items {
            let InstructionItem::Operands(ops) = item else {
                continue;
            };
            for o in ops {
                let kind_name = o.kind.last().name.clone();
                let Some(kdef) = self
                    .lookup_item(scope, &kind_name)
                    .filter(|d| self.prog.item(*d).kind == ItemKind::OperandKind)
                else {
                    self.error(
                        module,
                        "E1033",
                        o.kind.span,
                        format!("`{kind_name}` is not an operand kind"),
                    );
                    continue;
                };
                if operands
                    .iter()
                    .any(|(n, _): &(String, DefId)| *n == o.name.name)
                {
                    self.error(
                        module,
                        "E1001",
                        o.name.span,
                        format!("duplicate operand `{}`", o.name.name),
                    );
                    continue;
                }
                operands.push((o.name.name.clone(), kdef));
            }
        }
        InstrSig { operands }
    }

    /// Parse an opcode argument (byte, pattern string, or list of those)
    /// into bytes and named fields.
    fn parse_opcode(
        &mut self,
        scope: Scope,
        arg: &ast::Expr,
    ) -> Option<(Vec<hir::OpcodeByte>, Vec<hir::OpcodeField>)> {
        let module = scope.module;
        let empty = HashMap::new();
        let parts: Vec<&ast::Expr> = match &arg.kind {
            A::ArrayLit(items) => items.iter().collect(),
            _ => vec![arg],
        };
        let mut bytes = Vec::new();
        let mut fields: Vec<hir::OpcodeField> = Vec::new();
        for part in parts {
            if let A::Str(pattern) = &part.kind {
                let bits: Vec<char> = pattern.chars().filter(|c| *c != '_' && *c != ' ').collect();
                if bits.is_empty() || !bits.len().is_multiple_of(8) {
                    self.error_with(
                        module,
                        Diagnostic::error("E1032", part.span, "an opcode pattern is a multiple of 8 bits")
                            .with_note("`0`/`1` are fixed bits, `-` is don't-care, letters are fields: \"00ooo000\""),
                    );
                    return None;
                }
                for chunk in bits.chunks(8) {
                    let byte_index = bytes.len() as u32;
                    let mut value = 0u8;
                    let mut mask = 0u8;
                    for (i, c) in chunk.iter().enumerate() {
                        let bit = 7 - i as u32;
                        match c {
                            '0' => mask |= 1 << bit,
                            '1' => {
                                mask |= 1 << bit;
                                value |= 1 << bit;
                            }
                            '-' => {}
                            c if c.is_ascii_alphabetic() => {
                                let name = c.to_string();
                                match fields.iter_mut().find(|f| f.name == name) {
                                    Some(f) => {
                                        if f.byte != byte_index || f.shift != bit + 1 {
                                            let msg = format!("field `{name}` must be one contiguous run of bits within a byte");
                                            self.error(module, "E1032", part.span, msg);
                                            return None;
                                        }
                                        f.shift = bit;
                                        f.width += 1;
                                    }
                                    None => fields.push(hir::OpcodeField {
                                        name,
                                        byte: byte_index,
                                        shift: bit,
                                        width: 1,
                                    }),
                                }
                            }
                            other => {
                                let msg = format!("unexpected `{other}` in opcode pattern");
                                self.error(module, "E1032", part.span, msg);
                                return None;
                            }
                        }
                    }
                    bytes.push(hir::OpcodeByte { value, mask });
                }
                continue;
            }
            match self.eval_const(scope, &empty, part) {
                Some(ConstVal::Int(v)) if (0..=255).contains(&v) => bytes.push(hir::OpcodeByte {
                    value: v as u8,
                    mask: 0xFF,
                }),
                Some(_) => {
                    self.error(module, "E1032", part.span, "opcode bytes are 0..=255");
                    return None;
                }
                None => return None,
            }
        }
        Some((bytes, fields))
    }

    /// Resolve the kind named by a field binding and check it accepts a
    /// field of type `field_ty`.
    fn field_kind(
        &mut self,
        scope: Scope,
        binding: &ast::FieldInit,
        field_ty: &Ty,
    ) -> Option<DefId> {
        let module = scope.module;
        let kind_name = match binding.value.as_ref().map(|v| &v.kind) {
            Some(A::Path(p)) if p.segments.len() == 1 => p.segments[0].name.clone(),
            _ => {
                self.error(module, "E1032", binding.span, "a field binding names an operand kind (`reg: r8`) or constrains the field (`reg: 0`)");
                return None;
            }
        };
        let Some(kdef) = self
            .lookup_item(scope, &kind_name)
            .filter(|d| self.prog.item(*d).kind == ItemKind::OperandKind)
        else {
            self.error(
                module,
                "E1033",
                binding.span,
                format!("`{kind_name}` is not an operand kind"),
            );
            return None;
        };
        let ksig = self.kinds.get(&kdef).cloned()?;
        let rule = ksig
            .rule(RuleKind::Select)
            .or_else(|| ksig.rule(RuleKind::Value));
        let Some((_, (rname, _, params))) = rule else {
            let msg = format!("kind `{kind_name}` is built `from` an operand; it binds to a fetched operand, not to a field");
            self.error(module, "E1033", binding.span, msg);
            return None;
        };
        if params[0].1 != *field_ty {
            let msg = format!(
                "field `{}` is `{}` but `{kind_name}.{rname}` takes `{}`",
                binding.name.name,
                self.display(field_ty),
                self.display(&params[0].1)
            );
            self.error(module, "E1033", binding.span, msg);
            return None;
        }
        Some(kdef)
    }

    fn check_instruction(&mut self, scope: Scope, id: DefId) -> Option<hir::Instruction> {
        let decl = self.prog.instruction_decl(id);
        let module = scope.module;
        let sig = self.instr_sigs.get(&id)?.clone();
        let what = if decl.prefix { "prefix" } else { "instruction" };
        let mut ok = true;

        let mut mnemonic = None;
        let mut encoding: Option<Vec<hir::EncodingStep>> = None;
        let mut flags = hir::FlagSets::default();
        let mut semantics = None;
        let mut locals = Vec::new();
        let mut timing = None;
        let mut timing_locals = Vec::new();
        let mut implements = None;
        let mut references = Vec::new();
        let mut allowed_in = Vec::new();
        let mut requires = None;
        let mut seen: Vec<&'static str> = Vec::new();

        // Field bindings: (kind, step, field) for opcode and operand fields;
        // fetched operands: (operand, step).
        let mut field_bindings: Vec<(DefId, u32, u32)> = Vec::new();
        let mut fetch_steps: Vec<(DefId, u32)> = Vec::new();

        let ops_cx = |sig: &InstrSig| {
            let mut cx = FnCtx::new(scope, HashMap::new(), None);
            cx.operands = sig.operands.clone();
            cx
        };

        for item in &decl.items {
            let key = match item {
                InstructionItem::Mnemonic(_) => "mnemonic",
                InstructionItem::Encoding(_) => "encoding",
                InstructionItem::Operands(_) => "operands",
                InstructionItem::Flags(_) => "flags",
                InstructionItem::Semantics(_) => "semantics",
                InstructionItem::Timing(_) => "timing",
                InstructionItem::Implements(_) => "implements",
                InstructionItem::References(_) => "references",
                InstructionItem::AllowedIn(_) => "allowed_in",
                InstructionItem::Requires(_) => "requires",
                InstructionItem::Other(..) => "other",
            };
            if key != "other" {
                if seen.contains(&key) {
                    self.error(
                        module,
                        "E1001",
                        decl.name.span,
                        format!("{what} `{}` has `{key}` twice", decl.name.name),
                    );
                    ok = false;
                }
                seen.push(key);
            }
            match item {
                InstructionItem::Mnemonic(e) => {
                    if let A::Str(s) = &e.kind {
                        mnemonic = Some(hir::Mnemonic::Fixed(s.clone()));
                    } else {
                        let mut cx = ops_cx(&sig);
                        let he = self.check_expr(&mut cx, e, Some(&Ty::Str));
                        if he.ty.is_error() {
                            ok = false;
                        } else {
                            mnemonic = Some(hir::Mnemonic::Expr {
                                locals: cx.locals,
                                expr: he,
                            });
                        }
                    }
                }
                InstructionItem::Encoding(items) => {
                    let mut steps = Vec::new();
                    let wrote_opcode = items.iter().any(|ei| ei.name.name == "opcode");
                    for ei in items {
                        let step = steps.len() as u32;
                        if ei.name.name == "opcode" {
                            let Some(arg) = &ei.arg else {
                                self.error(
                                    module,
                                    "E1032",
                                    ei.span,
                                    "`opcode` takes a byte, a pattern string, or a list of those",
                                );
                                ok = false;
                                continue;
                            };
                            let Some((bytes, fields)) = self.parse_opcode(scope, arg) else {
                                ok = false;
                                continue;
                            };
                            let mut bound = vec![false; fields.len()];
                            for b in &ei.fields {
                                let Some(fi) = fields.iter().position(|f| f.name == b.name.name)
                                else {
                                    self.error(
                                        module,
                                        "E1032",
                                        b.name.span,
                                        format!(
                                            "the opcode pattern has no field `{}`",
                                            b.name.name
                                        ),
                                    );
                                    ok = false;
                                    continue;
                                };
                                bound[fi] = true;
                                match self.field_kind(scope, b, &Ty::UInt(fields[fi].width)) {
                                    Some(k) => field_bindings.push((k, step, fi as u32)),
                                    None => ok = false,
                                }
                            }
                            for (f, b) in fields.iter().zip(&bound) {
                                if !b {
                                    let msg = format!("opcode field `{}` is not bound to an operand kind; use `-` for don't-care bits", f.name);
                                    self.error(module, "E1032", arg.span, msg);
                                    ok = false;
                                }
                            }
                            steps.push(hir::EncodingStep::Opcode { bytes, fields });
                            continue;
                        }
                        // An operand to fetch.
                        let Some(odef) = self
                            .lookup_item(scope, &ei.name.name)
                            .filter(|d| self.prog.item(*d).kind == ItemKind::Operand)
                        else {
                            self.error_with(
                                module,
                                Diagnostic::error("E1032", ei.name.span, format!("`{}` is not an encoding item", ei.name.name))
                                    .with_note("encoding items are `opcode <byte or pattern>` and operand names, optionally with `{ field: kind }` bindings or `{ field: 0 }` constraints"),
                            );
                            ok = false;
                            continue;
                        };
                        if decl.prefix {
                            self.error(
                                module,
                                "E1048",
                                ei.span,
                                "a prefix is opcode bytes only; it fetches no operands",
                            );
                            ok = false;
                        }
                        if ei.arg.is_some() {
                            self.error(
                                module,
                                "E1032",
                                ei.span,
                                "an operand fetch takes no argument",
                            );
                            ok = false;
                        }
                        let Some(oinfo) = self.operands.get(&odef).cloned() else {
                            ok = false;
                            continue;
                        };
                        let mut constraints = Vec::new();
                        for f in &ei.fields {
                            let Some(fi) =
                                oinfo.fields.iter().position(|of| of.name == f.name.name)
                            else {
                                let msg = format!(
                                    "operand `{}` has no field `{}`",
                                    oinfo.name, f.name.name
                                );
                                self.error(module, "E1032", f.name.span, msg);
                                ok = false;
                                continue;
                            };
                            let fty = oinfo.fields[fi].ty.clone();
                            if let Some(A::Int { value, .. }) = f.value.as_ref().map(|v| &v.kind) {
                                let v = *value as i128;
                                if !fty.fits(v) {
                                    let msg = format!(
                                        "constraint {v} does not fit field `{}` (`{}`)",
                                        f.name.name,
                                        self.display(&fty)
                                    );
                                    self.error(module, "E1006", f.span, msg);
                                    ok = false;
                                    continue;
                                }
                                constraints.push((fi as u32, v));
                                continue;
                            }
                            match self.field_kind(scope, f, &fty) {
                                Some(k) => field_bindings.push((k, step, fi as u32)),
                                None => ok = false,
                            }
                        }
                        fetch_steps.push((odef, step));
                        steps.push(hir::EncodingStep::Fetch {
                            operand: odef,
                            constraints,
                        });
                    }
                    if !wrote_opcode {
                        self.error(
                            module,
                            "E1032",
                            decl.name.span,
                            format!("{what} `{}` has no `opcode`", decl.name.name),
                        );
                        ok = false;
                    }
                    if let Some(pos) = steps.iter().position(|s| !s.is_opcode()) {
                        if steps[pos..].iter().any(|s| s.is_opcode()) {
                            self.error(
                                module,
                                "E1032",
                                decl.name.span,
                                "opcode bytes after an operand fetch are not supported",
                            );
                            ok = false;
                        }
                    }
                    encoding = Some(steps);
                }
                InstructionItem::Operands(_) => {} // in the signature
                InstructionItem::Flags(sets) => {
                    let mut cx = ops_cx(&sig);
                    for set in sets {
                        let mut target = Vec::new();
                        for entry in &set.entries {
                            let fname = entry.flag.last().name.clone();
                            let Some(ScopeEntry::RegField { reg, index }) =
                                self.lookup(scope, &fname)
                            else {
                                self.error(
                                    module,
                                    "E1034",
                                    entry.flag.span,
                                    format!("`{fname}` is not a register field"),
                                );
                                ok = false;
                                continue;
                            };
                            let cond = entry
                                .cond
                                .as_ref()
                                .map(|c| self.check_expr(&mut cx, c, Some(&Ty::Bool)));
                            if cond.as_ref().is_some_and(|c| c.ty.is_error()) {
                                ok = false;
                            }
                            target.push(hir::FlagUse {
                                reg,
                                field: index,
                                cond,
                            });
                        }
                        match set.name.name.as_str() {
                            "modifies" => flags.modifies = target,
                            "undefined" => flags.undefined = target,
                            "reads" => flags.reads = target,
                            other => {
                                let msg = format!("unknown flag set `{other}`; use `modifies`, `undefined` or `reads`");
                                self.error(module, "E1034", set.name.span, msg);
                                ok = false;
                            }
                        }
                    }
                    flags.locals = cx.locals;
                }
                InstructionItem::Semantics(block) => {
                    let mut cx = FnCtx::new(scope, HashMap::new(), Some(Ty::Unit));
                    cx.operands = sig.operands.clone();
                    let b = self.check_block(&mut cx, block);
                    locals = cx.locals;
                    semantics = Some(b);
                }
                InstructionItem::Timing(items) => {
                    for it in items {
                        if it.name.name != "cycles" {
                            let msg = format!(
                                "unknown timing attribute `{}`; use `cycles`",
                                it.name.name
                            );
                            self.error(module, "E1032", it.name.span, msg);
                            ok = false;
                            continue;
                        }
                        let Some(v) = &it.value else {
                            self.error(module, "E1032", it.span, "`cycles` takes an expression");
                            ok = false;
                            continue;
                        };
                        let mut cx = ops_cx(&sig);
                        cx.in_timing = true;
                        let e = self.check_expr(&mut cx, v, Some(&Ty::UInt(32)));
                        timing_locals = cx.locals;
                        timing = Some(e);
                    }
                }
                InstructionItem::Implements(p) => implements = Some(p.dotted()),
                InstructionItem::References(refs) => {
                    references = refs.iter().map(|s| s.value.clone()).collect()
                }
                InstructionItem::AllowedIn(modes) => {
                    let Some(&mode_def) = self.isa_modes.get(&scope.isa.unwrap()) else {
                        self.error(
                            module,
                            "E1032",
                            decl.name.span,
                            "`allowed_in` needs the isa to declare a `mode`",
                        );
                        ok = false;
                        continue;
                    };
                    let mut cx = FnCtx::new(scope, HashMap::new(), None);
                    for m in modes {
                        let e = self.check_expr(&mut cx, m, Some(&Ty::Enum(mode_def)));
                        match e.kind {
                            H::Variant(_, idx) => allowed_in.push(idx),
                            H::Error => ok = false,
                            _ => {
                                self.error(
                                    module,
                                    "E1032",
                                    m.span,
                                    "`allowed_in` lists mode variants",
                                );
                                ok = false;
                            }
                        }
                    }
                }
                InstructionItem::Requires(e) => {
                    let mut cx = ops_cx(&sig);
                    requires = Some(self.check_expr(&mut cx, e, Some(&Ty::Bool)));
                }
                InstructionItem::Other(name, _) => {
                    self.error(
                        module,
                        "E1032",
                        name.span,
                        format!("unknown {what} item `{}`", name.name),
                    );
                    ok = false;
                }
            }
        }

        if !seen.contains(&"mnemonic") && !decl.prefix {
            self.error(
                module,
                "E1032",
                decl.name.span,
                format!("instruction `{}` needs a `mnemonic`", decl.name.name),
            );
            ok = false;
        }
        let Some(encoding) = encoding else {
            self.error(
                module,
                "E1032",
                decl.name.span,
                format!("{what} `{}` needs an `encoding`", decl.name.name),
            );
            return None;
        };
        if !ok {
            // Binding diagnostics would only restate problems already
            // reported in the encoding or operand list.
            return None;
        }

        // Bind operands to the encoding.
        let mut bindings = Vec::new();
        let mut used_fields = vec![false; field_bindings.len()];
        let mut used_fetches = vec![false; fetch_steps.len()];
        for (oname, kdef) in &sig.operands {
            let Some(ksig) = self.kinds.get(kdef).cloned() else {
                ok = false;
                continue;
            };
            let kname = self.prog.item_name(*kdef);
            let source = match ksig.source {
                None => {
                    let candidates: Vec<usize> = field_bindings
                        .iter()
                        .enumerate()
                        .filter(|(i, (k, _, _))| *k == *kdef && !used_fields[*i])
                        .map(|(i, _)| i)
                        .collect();
                    match candidates.as_slice() {
                        [i] => {
                            used_fields[*i] = true;
                            let (_, step, field) = field_bindings[*i];
                            hir::BindSource::Field { step, field }
                        }
                        [] => {
                            let msg = format!(
                                "operand `{oname}` of kind `{kname}` is not bound by the encoding"
                            );
                            self.error_with(
                                module,
                                Diagnostic::error("E1033", decl.name.span, msg).with_note(format!(
                                    "bind an encoding field to it: `{{ field: {kname} }}`"
                                )),
                            );
                            ok = false;
                            continue;
                        }
                        _ => {
                            let msg = format!("operand `{oname}` is ambiguous: several encoding fields bind kind `{kname}`");
                            self.error(module, "E1033", decl.name.span, msg);
                            ok = false;
                            continue;
                        }
                    }
                }
                Some(src) => {
                    let candidates: Vec<usize> = fetch_steps
                        .iter()
                        .enumerate()
                        .filter(|(i, (o, _))| *o == src && !used_fetches[*i])
                        .map(|(i, _)| i)
                        .collect();
                    match candidates.as_slice() {
                        [i] => {
                            used_fetches[*i] = true;
                            hir::BindSource::From {
                                step: fetch_steps[*i].1,
                            }
                        }
                        [] => {
                            let msg = format!(
                                "operand `{oname}` of kind `{kname}` needs a fetched `{}` in the encoding",
                                self.prog.item_name(src)
                            );
                            self.error(module, "E1033", decl.name.span, msg);
                            ok = false;
                            continue;
                        }
                        _ => {
                            let msg = format!("operand `{oname}` is ambiguous: the encoding fetches `{}` more than once", self.prog.item_name(src));
                            self.error(module, "E1033", decl.name.span, msg);
                            ok = false;
                            continue;
                        }
                    }
                }
            };
            bindings.push(hir::OperandBinding {
                name: oname.clone(),
                kind: *kdef,
                source,
            });
        }
        for (i, used) in used_fields.iter().enumerate() {
            if !used {
                let (k, _, _) = field_bindings[i];
                let msg = format!(
                    "the encoding binds kind `{}` but no operand has that kind",
                    self.prog.item_name(k)
                );
                self.error(module, "E1033", decl.name.span, msg);
                ok = false;
            }
        }

        if !ok {
            return None;
        }
        Some(hir::Instruction {
            def: id,
            name: decl.name.name.clone(),
            prefix: decl.prefix,
            mnemonic,
            description: decl.docs.clone(),
            encoding,
            operands: bindings,
            flags,
            locals,
            semantics,
            timing_locals,
            timing,
            implements,
            references,
            allowed_in,
            requires,
        })
    }

    // -- helpers used by expression/statement checking -----------------------------------------

    /// Resolve a `raise` target: `Fault(args)` or `Isa::Fault(args)`.
    pub(super) fn check_raise(
        &mut self,
        cx: &mut FnCtx,
        e: &ast::Expr,
    ) -> Option<(DefId, Vec<hir::Expr>)> {
        let (callee, args): (&ast::Expr, &[ast::Arg]) = match &e.kind {
            A::Call {
                callee,
                args,
                generics,
            } => {
                if !generics.is_empty() {
                    self.error(
                        cx.module,
                        "E1038",
                        e.span,
                        "faults take no generic arguments",
                    );
                    return None;
                }
                (callee, args)
            }
            _ => (e, &[]),
        };
        let A::Path(p) = &callee.kind else {
            self.error(
                cx.module,
                "E1038",
                e.span,
                "`raise` names a fault: `raise GP(0);`",
            );
            return None;
        };
        let def = self.resolve_isa_path(cx, p);
        let Some(def) = def else {
            self.error(
                cx.module,
                "E1000",
                p.span,
                format!("unknown fault `{}`", p.dotted()),
            );
            return None;
        };
        if self.prog.item(def).kind != ItemKind::Fault {
            self.error(
                cx.module,
                "E1038",
                p.span,
                format!(
                    "`{}` is a {}, not a fault",
                    p.dotted(),
                    self.prog.item(def).kind.describe()
                ),
            );
            return None;
        }
        let info = self.faults.get(&def).cloned()?;
        if args.len() != info.params.len() {
            let msg = format!(
                "fault `{}` takes {} argument(s), {} given",
                info.name,
                info.params.len(),
                args.len()
            );
            self.error(cx.module, "E1007", e.span, msg);
            return None;
        }
        let mut hargs = Vec::new();
        for (a, (_, ty)) in args.iter().zip(&info.params) {
            hargs.push(self.check_expr(cx, &a.value, Some(ty)));
        }
        Some((def, hargs))
    }

    /// Resolve `Name` or `Isa::Name` to an isa item.
    pub(super) fn resolve_isa_path(&mut self, cx: &FnCtx, p: &ast::Path) -> Option<DefId> {
        match p.segments.len() {
            1 => self.lookup_item(cx.scope, &p.segments[0].name),
            2 => {
                let isa = self.lookup_item(cx.scope, &p.segments[0].name)?;
                if self.prog.item(isa).kind != ItemKind::Isa {
                    return None;
                }
                match self.prog.lookup_in_ns(isa, &p.segments[1].name) {
                    Some(ScopeEntry::Item(d)) => Some(d),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// Validate `given { place = value; }` assignments and build the HIR.
    pub(super) fn check_given(
        &mut self,
        cx: &mut FnCtx,
        body: &ast::Block,
    ) -> Vec<(hir::Expr, hir::Expr)> {
        let mut out = Vec::new();
        for s in &body.stmts {
            match &s.kind {
                ast::StmtKind::Assign { target, value } => {
                    if let Some(pair) = self.check_assign(cx, target, value) {
                        out.push(pair);
                    }
                }
                ast::StmtKind::Error => {}
                _ => self.error(
                    cx.module,
                    "E1020",
                    s.span,
                    "`given` contains `place = value;` assignments only",
                ),
            }
        }
        out
    }

    /// Check `place = value;` where the place is architectural state. Returns
    /// `None` after reporting.
    pub(super) fn check_assign(
        &mut self,
        cx: &mut FnCtx,
        target: &ast::Expr,
        value: &ast::Expr,
    ) -> Option<(hir::Expr, hir::Expr)> {
        // Assigning a whole unassigned local is not a read of it.
        let whole_local = match &target.kind {
            ast::ExprKind::Path(p) if p.segments.len() == 1 => cx.lookup_local(&p.segments[0].name),
            _ => None,
        };
        let pending = whole_local.filter(|id| cx.unassigned.remove(id));
        let place = self.check_expr(cx, target, None);
        if let Some(id) = pending {
            cx.unassigned.insert(id);
        }
        if place.ty.is_error() {
            self.check_expr(cx, value, None);
            return None;
        }
        if let Some(id) = place_local(&place) {
            let local = &cx.locals[id as usize];
            if !local.mutable {
                let name = local.name.clone();
                self.error_with(
                    cx.module,
                    Diagnostic::error("E1050", target.span, format!("`{name}` is not mutable"))
                        .with_note(format!("declare it `let mut {name}` to assign it; a parameter is copied into a `let mut` local first")),
                );
                self.check_expr(cx, value, None);
                return None;
            }
            let ty = place.ty.clone();
            let v = self.check_expr(cx, value, Some(&ty));
            if pending.is_some() {
                cx.unassigned.remove(&id);
            }
            return Some((place, v));
        }
        if !hir::is_place(&place) {
            self.error_with(
                cx.module,
                Diagnostic::error("E1030", target.span, "this is not assignable")
                    .with_note("assignable places are registers, register fields and views, `mode`, memory cells (`mem[addr]`) and operands"),
            );
            self.check_expr(cx, value, None);
            return None;
        }
        let ty = place.ty.clone();
        let v = self.check_expr(cx, value, Some(&ty));
        Some((place, v))
    }

    /// `run instruction Name { operand: value, ... }`
    /// `vectors "glob" on Isa { format moo; memory mem; reg ax = AX; ... }`
    pub(super) fn check_vectors(&mut self, id: DefId) -> Option<hir::VectorsInfo> {
        let module = self.prog.item(id).module;
        let ast::Decl::Vectors(d) = self.prog.decl(id) else {
            unreachable!()
        };
        let isa = match d.target.segments.as_slice() {
            [one] => self
                .prog
                .lookup(module, &one.name)
                .filter(|i| self.prog.item(*i).kind == ItemKind::Isa),
            _ => None,
        };
        let Some(isa) = isa else {
            self.error(
                module,
                "E1049",
                d.target.span,
                format!(
                    "`{}` is not an isa; vectors run on an isa",
                    d.target.dotted()
                ),
            );
            return None;
        };
        if d.items.is_empty() {
            self.error_with(
                module,
                Diagnostic::error("E1049", d.span, "a `vectors` declaration needs a body saying how to read the vectors")
                    .with_note("`vectors \"dir/*.MOO.gz\" on Isa { format moo; memory mem; reg ax = AX; ... }`"),
            );
            return None;
        }
        let scope = Scope {
            module,
            isa: Some(isa),
            bus: None,
        };
        let mut ok = true;
        let mut format = None;
        let mut memory = None;
        let mut regs: Vec<hir::VectorReg> = Vec::new();
        let mut open: Vec<(DefId, i128)> = Vec::new();
        let mut harness_cycles: u32 = 0;
        const MOO_REGS: &[&str] = &[
            "ax", "bx", "cx", "dx", "cs", "ss", "ds", "es", "sp", "bp", "si", "di", "ip", "flags",
        ];
        for item in &d.items {
            match item {
                ast::VectorsItem::Format(f) => {
                    if f.name != "moo" {
                        self.error(
                            module,
                            "E1049",
                            f.span,
                            format!("unknown vector format `{}`; supported: `moo`", f.name),
                        );
                        ok = false;
                    }
                    format = Some(f.name.clone());
                }
                ast::VectorsItem::HarnessCycles { value, span } => {
                    match self.eval_const(scope, &HashMap::new(), value) {
                        Some(ConstVal::Int(v)) if (0..=u32::MAX as i128).contains(&v) => {
                            harness_cycles = v as u32
                        }
                        Some(_) => {
                            self.error(module, "E1049", *span, "`harness_cycles` is a cycle count");
                            ok = false;
                        }
                        None => ok = false,
                    }
                }
                ast::VectorsItem::Open { space, value, span } => {
                    let Some(sp) = self
                        .lookup_item(scope, &space.name)
                        .filter(|d| self.prog.item(*d).kind == ItemKind::Space)
                    else {
                        self.error(
                            module,
                            "E1049",
                            space.span,
                            format!(
                                "`{}` is not an address space of `{}`",
                                space.name,
                                d.target.dotted()
                            ),
                        );
                        ok = false;
                        continue;
                    };
                    let cell = self.spaces.get(&sp).map(|i| i.cell_bits).unwrap_or(8);
                    match self.eval_const(scope, &HashMap::new(), value) {
                        Some(ConstVal::Int(v)) if Ty::UInt(cell).fits(v) => open.push((sp, v)),
                        Some(_) => {
                            self.error(
                                module,
                                "E1049",
                                *span,
                                format!("an open-bus value is one u{cell} cell"),
                            );
                            ok = false;
                        }
                        None => ok = false,
                    }
                }
                ast::VectorsItem::Memory(m) => match self.lookup_item(scope, &m.name) {
                    Some(sp) if self.prog.item(sp).kind == ItemKind::Space => memory = Some(sp),
                    _ => {
                        self.error(
                            module,
                            "E1049",
                            m.span,
                            format!(
                                "`{}` is not an address space of `{}`",
                                m.name,
                                d.target.dotted()
                            ),
                        );
                        ok = false;
                    }
                },
                ast::VectorsItem::Reg { name, place, span }
                | ast::VectorsItem::RegCustom {
                    name,
                    read: place,
                    span,
                    ..
                } => {
                    if !MOO_REGS.contains(&name.name.as_str()) {
                        let msg = format!(
                            "`{}` is not a register of the vector format (registers: {})",
                            name.name,
                            MOO_REGS.join(", ")
                        );
                        self.error(module, "E1049", name.span, msg);
                        ok = false;
                        continue;
                    }
                    if regs.iter().any(|r| r.name == name.name) {
                        self.error(
                            module,
                            "E1001",
                            name.span,
                            format!("register `{}` mapped twice", name.name),
                        );
                        ok = false;
                        continue;
                    }
                    let mut cx = FnCtx::new(scope, HashMap::new(), None);
                    let value = cx.declare("value", Ty::UInt(16), *span);
                    let load = match item {
                        ast::VectorsItem::Reg { place, .. } => {
                            let assign = ast::Stmt {
                                kind: ast::StmtKind::Assign {
                                    target: place.clone(),
                                    value: ast::Expr {
                                        kind: A::Path(ast::Path::single(ast::Ident {
                                            name: "value".into(),
                                            span: *span,
                                        })),
                                        span: *span,
                                    },
                                },
                                span: *span,
                            };
                            ast::Block {
                                stmts: vec![assign],
                                span: *span,
                            }
                        }
                        ast::VectorsItem::RegCustom { load, .. } => ast::Block {
                            stmts: vec![(**load).clone()],
                            span: load.span,
                        },
                        _ => unreachable!(),
                    };
                    let errors_before = self.diags[module].error_count();
                    let load = self.check_block(&mut cx, &load);
                    if self.diags[module].error_count() > errors_before {
                        ok = false;
                        continue;
                    }
                    let read = self.check_expr(&mut cx, place, Some(&Ty::UInt(16)));
                    if read.ty.is_error() {
                        ok = false;
                        continue;
                    }
                    let masked_reg = match &read.kind {
                        H::Register(r) => Some(*r),
                        _ => None,
                    };
                    regs.push(hir::VectorReg {
                        name: name.name.clone(),
                        locals: cx.locals,
                        value,
                        load,
                        read,
                        masked_reg,
                    });
                }
            }
        }
        let Some(format) = format else {
            self.error(module, "E1049", d.span, "`vectors` needs `format moo;`");
            return None;
        };
        let Some(memory) = memory else {
            self.error(
                module,
                "E1049",
                d.span,
                "`vectors` needs `memory <space>;` for the vectors' RAM",
            );
            return None;
        };
        if !ok {
            return None;
        }
        let base = std::path::Path::new(&self.prog.modules[module].file.name)
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_default();
        let glob = base.join(&d.glob.value).to_string_lossy().into_owned();
        Some(hir::VectorsInfo {
            def: id,
            isa,
            glob,
            format,
            memory,
            open,
            regs,
            harness_cycles,
        })
    }

    pub(super) fn check_run_instruction(
        &mut self,
        cx: &mut FnCtx,
        name: &ast::Path,
        operands: &[ast::FieldInit],
        span: Span,
    ) -> Option<hir::StmtKind> {
        let Some(def) = self.resolve_isa_path(cx, name) else {
            self.error(
                cx.module,
                "E1000",
                name.span,
                format!("unknown instruction `{}`", name.dotted()),
            );
            return None;
        };
        if self.prog.item(def).kind != ItemKind::Instruction {
            self.error(
                cx.module,
                "E1000",
                name.span,
                format!("`{}` is not an instruction", name.dotted()),
            );
            return None;
        }
        let sig = self.instr_sigs.get(&def).cloned()?;
        let mut values: Vec<Option<hir::Expr>> = (0..sig.operands.len()).map(|_| None).collect();
        let mut ok = true;
        for f in operands {
            let Some(idx) = sig.operands.iter().position(|(n, _)| *n == f.name.name) else {
                self.error(
                    cx.module,
                    "E1008",
                    f.name.span,
                    format!(
                        "instruction `{}` has no operand `{}`",
                        name.dotted(),
                        f.name.name
                    ),
                );
                ok = false;
                continue;
            };
            let Some(v) = &f.value else {
                self.error(
                    cx.module,
                    "E1008",
                    f.span,
                    "give each operand a value: `dst: AL`",
                );
                ok = false;
                continue;
            };
            let kty = self
                .kinds
                .get(&sig.operands[idx].1)
                .map(|k| k.ty.clone())
                .unwrap_or(Ty::Error);
            values[idx] = Some(self.check_expr(cx, v, Some(&kty)));
        }
        let missing: Vec<&str> = sig
            .operands
            .iter()
            .zip(&values)
            .filter(|(_, v)| v.is_none())
            .map(|((n, _), _)| n.as_str())
            .collect();
        if !missing.is_empty() {
            self.error(
                cx.module,
                "E1008",
                span,
                format!("missing operand(s): {}", missing.join(", ")),
            );
            ok = false;
        }
        if !ok {
            return None;
        }
        Some(hir::StmtKind::RunInstruction(
            def,
            values.into_iter().map(|v| v.unwrap()).collect(),
        ))
    }
}

/// `[0x80, 00--0000]`-style rendering of an opcode pattern.
fn pattern_text(b: &[hir::OpcodeByte]) -> String {
    let parts: Vec<String> = b
        .iter()
        .map(|x| {
            if x.mask == 0xFF {
                format!("{:#04x}", x.value)
            } else {
                (0..8)
                    .rev()
                    .map(|i| {
                        if x.mask >> i & 1 == 0 {
                            '-'
                        } else if x.value >> i & 1 == 1 {
                            '1'
                        } else {
                            '0'
                        }
                    })
                    .collect()
            }
        })
        .collect();
    format!("[{}]", parts.join(", "))
}

/// Does an expression call an isa `fn` or touch memory (which may fault)?
fn contains_call(e: &hir::Expr) -> bool {
    match &e.kind {
        H::FnCall(..) | H::SpaceRead(..) | H::SpaceCell(..) | H::SpaceWrite(..) => true,
        H::Unary(_, a)
        | H::Cast(a)
        | H::Field(a, _)
        | H::TupleField(a, _)
        | H::Slice(a, ..)
        | H::ArrayRepeat(a, _) => contains_call(a),
        H::Binary(_, a, b) | H::Index(a, b) => contains_call(a) || contains_call(b),
        H::Ternary(a, b, c) => contains_call(a) || contains_call(b) || contains_call(c),
        H::Call(_, args)
        | H::Builtin(_, args)
        | H::BundleLit(_, args)
        | H::Tuple(args)
        | H::Array(args)
        | H::KindSelect(_, args) => args.iter().any(contains_call),
        H::Match(..) | H::If(..) | H::Block(_) => true,
        _ => false,
    }
}

/// The local at the root of a place: `x`, `x.f`, `x[i].f`.
fn place_local(e: &hir::Expr) -> Option<hir::LocalId> {
    match &e.kind {
        hir::ExprKind::Local(id) => Some(*id),
        hir::ExprKind::Field(b, _) | hir::ExprKind::Index(b, _) => place_local(b),
        _ => None,
    }
}

impl Checker<'_> {
    /// `timing on Isa { Name => cycles; ... }`: each entry is the execution
    /// cycles of an instruction (or prefix) beyond its bus transfers, in the
    /// context of its flag conditions (operands, registers and latches on
    /// the state before it runs); `fault` and `interrupt` are delivery.
    pub(super) fn check_timing(&mut self, id: DefId) {
        let module = self.prog.item(id).module;
        let ast::Decl::Timing(d) = self.prog.decl(id) else {
            unreachable!()
        };
        let isa = match d.target.segments.as_slice() {
            [one] => self
                .prog
                .lookup(module, &one.name)
                .filter(|i| self.prog.item(*i).kind == ItemKind::Isa),
            _ => None,
        };
        let Some(isa) = isa else {
            self.error(
                module,
                "E1055",
                d.target.span,
                format!(
                    "`{}` is not an isa; a timing table belongs to one",
                    d.target.dotted()
                ),
            );
            return;
        };
        let Some(ii) = self.isas.iter().position(|i| i.def == isa) else {
            return;
        };
        let scope = Scope {
            module,
            isa: Some(isa),
            bus: None,
        };
        let mut seen: Vec<String> = Vec::new();
        for e in &d.entries {
            if seen.contains(&e.name.name) {
                self.error(
                    module,
                    "E1055",
                    e.name.span,
                    format!("`{}` already has an entry", e.name.name),
                );
                continue;
            }
            seen.push(e.name.name.clone());
            if e.name.name == "fault" || e.name.name == "interrupt" {
                let mut cx = FnCtx::new(scope, HashMap::new(), None);
                cx.in_timing = true;
                // A fault's cost may depend on which fault it is.
                let params = if e.name.name == "fault" {
                    vec![cx.declare("vector", Ty::UInt(8), e.name.span)]
                } else {
                    Vec::new()
                };
                let expr = self.check_expr(&mut cx, &e.value, Some(&Ty::UInt(32)));
                let t = Some(hir::TimingExpr {
                    locals: cx.locals,
                    params,
                    expr,
                });
                if e.name.name == "fault" {
                    self.isas[ii].fault_cycles = t;
                } else {
                    self.isas[ii].interrupt_cycles = t;
                }
                continue;
            }
            let Some(j) = self.isas[ii]
                .instructions
                .iter()
                .position(|i| i.name == e.name.name)
            else {
                let msg = format!("isa `{}` has no instruction `{}` (entries are instructions, prefixes, `fault` and `interrupt`)", d.target.dotted(), e.name.name);
                self.error(module, "E1055", e.name.span, msg);
                continue;
            };
            if self.isas[ii].instructions[j].timing.is_some() {
                let msg = format!("`{}` has timing in its declaration already", e.name.name);
                self.error(module, "E1055", e.name.span, msg);
                continue;
            }
            let def = self.isas[ii].instructions[j].def;
            let Some(sig) = self.instr_sigs.get(&def).cloned() else {
                continue;
            };
            let mut cx = FnCtx::new(scope, HashMap::new(), None);
            cx.operands = sig.operands.clone();
            cx.in_timing = true;
            let expr = self.check_expr(&mut cx, &e.value, Some(&Ty::UInt(32)));
            let ins = &mut self.isas[ii].instructions[j];
            ins.timing_locals = cx.locals;
            ins.timing = Some(expr);
        }
    }
}
