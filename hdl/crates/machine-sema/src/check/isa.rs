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
        });
    }

    /// Two instructions may not share leading opcode bytes (equal or one a
    /// prefix of the other) up to their first operand fetch.
    fn check_opcode_conflicts(&mut self, scope: Scope, instructions: &[hir::Instruction]) {
        let lead = |i: &hir::Instruction| -> Vec<u8> {
            let mut bytes = Vec::new();
            for step in &i.encoding {
                match step {
                    hir::EncodingStep::Opcode(b) => bytes.extend_from_slice(b),
                    hir::EncodingStep::Fetch { .. } => break,
                }
            }
            bytes
        };
        for (a, ia) in instructions.iter().enumerate() {
            let la = lead(ia);
            for ib in &instructions[..a] {
                let lb = lead(ib);
                if la.starts_with(&lb) || lb.starts_with(&la) {
                    let span = self.prog.item(ia.def).span;
                    let msg = format!(
                        "opcode {} of `{}` conflicts with {} of `{}`",
                        hex_bytes(&la),
                        ia.name,
                        hex_bytes(&lb),
                        ib.name
                    );
                    self.error_with(
                        scope.module,
                        Diagnostic::error("E1045", span, msg)
                            .with_label(self.prog.item(ib.def).span, "the other instruction"),
                    );
                }
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
        let (Some(space), Some(address), Some(next)) = (space, address, next) else {
            return None;
        };
        ok.then_some(hir::Istream {
            space,
            address,
            next,
            undefined,
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
                        .with_note("rules are `select(n)`, `read`, `write(v)` and `is_mem`"),
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
                RuleKind::Select if params.len() != 1 => {
                    self.error(
                        module,
                        "E1037",
                        r.span,
                        "`select` takes exactly one parameter: the encoding field",
                    );
                    ok = false;
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
            if has(RuleKind::Select) {
                self.error(
                    module,
                    "E1037",
                    decl.name.span,
                    "a kind with `from` is built from an operand and cannot have `select`",
                );
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
            if !has(RuleKind::Select) {
                self.error(
                    module,
                    "E1037",
                    decl.name.span,
                    format!(
                        "kind `{}` has no `from` operand, so it needs a `select` rule",
                        decl.name.name
                    ),
                );
                ok = false;
            }
            if has(RuleKind::Read) || has(RuleKind::Write) || has(RuleKind::IsMem) {
                self.error(
                    module,
                    "E1037",
                    decl.name.span,
                    "a `select` kind is a register; it has no `read`, `write` or `is_mem` rules",
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
                RuleKind::Select | RuleKind::Read => Some(sig.ty.clone()),
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
                    let (hb, bty) = self.check_block_expr(&mut cx, b);
                    if let Some(want) = &ret {
                        if *kind != RuleKind::Write
                            && bty != *want
                            && !bty.is_error()
                            && !matches!(bty, Ty::Never)
                        {
                            self.mismatch(module, b.span, want, &bty);
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

    fn check_instruction(&mut self, scope: Scope, id: DefId) -> Option<hir::Instruction> {
        let decl = self.prog.instruction_decl(id);
        let module = scope.module;
        let sig = self.instr_sigs.get(&id)?.clone();
        let empty = HashMap::new();
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

        // Binding slots: which kind each encoding select/fetch produces.
        let mut select_bindings: Vec<(DefId, u32, u32)> = Vec::new(); // (kind, step, field)
        let mut fetch_steps: Vec<(DefId, u32)> = Vec::new(); // (operand, step)

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
                        format!("instruction `{}` has `{key}` twice", decl.name.name),
                    );
                    ok = false;
                }
                seen.push(key);
            }
            match item {
                InstructionItem::Mnemonic(s) => mnemonic = Some(s.value.clone()),
                InstructionItem::Encoding(items) => {
                    let mut steps = Vec::new();
                    for ei in items {
                        let step = steps.len() as u32;
                        if ei.name.name == "opcode" {
                            let Some(arg) = &ei.arg else {
                                self.error(
                                    module,
                                    "E1032",
                                    ei.span,
                                    "`opcode` takes a byte or a list of bytes",
                                );
                                ok = false;
                                continue;
                            };
                            let byte_exprs: Vec<&ast::Expr> = match &arg.kind {
                                A::ArrayLit(items) => items.iter().collect(),
                                _ => vec![arg],
                            };
                            let mut bytes = Vec::new();
                            for b in byte_exprs {
                                match self.eval_const(scope, &empty, b) {
                                    Some(ConstVal::Int(v)) if (0..=255).contains(&v) => {
                                        bytes.push(v as u8)
                                    }
                                    Some(_) => {
                                        self.error(
                                            module,
                                            "E1032",
                                            b.span,
                                            "opcode bytes are 0..=255",
                                        );
                                        ok = false;
                                        bytes.push(0);
                                    }
                                    None => {
                                        ok = false;
                                        bytes.push(0);
                                    }
                                }
                            }
                            steps.push(hir::EncodingStep::Opcode(bytes));
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
                                    .with_note("encoding items are `opcode <byte>` and operand names, optionally with `{ field: kind }` bindings"),
                            );
                            ok = false;
                            continue;
                        };
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
                        let mut selects = Vec::new();
                        for f in &ei.fields {
                            let Some(fi) =
                                oinfo.fields.iter().position(|of| of.name == f.name.name)
                            else {
                                self.error(
                                    module,
                                    "E1032",
                                    f.name.span,
                                    format!(
                                        "operand `{}` has no field `{}`",
                                        oinfo.name, f.name.name
                                    ),
                                );
                                ok = false;
                                continue;
                            };
                            let kind_name = match f.value.as_ref().map(|v| &v.kind) {
                                Some(A::Path(p)) if p.segments.len() == 1 => {
                                    p.segments[0].name.clone()
                                }
                                _ => {
                                    self.error(
                                        module,
                                        "E1032",
                                        f.span,
                                        "a field binding names an operand kind: `reg: r8`",
                                    );
                                    ok = false;
                                    continue;
                                }
                            };
                            let Some(kdef) = self
                                .lookup_item(scope, &kind_name)
                                .filter(|d| self.prog.item(*d).kind == ItemKind::OperandKind)
                            else {
                                self.error(
                                    module,
                                    "E1033",
                                    f.span,
                                    format!("`{kind_name}` is not an operand kind"),
                                );
                                ok = false;
                                continue;
                            };
                            let Some(ksig) = self.kinds.get(&kdef).cloned() else {
                                ok = false;
                                continue;
                            };
                            let Some((_, (_, _, params))) = ksig.rule(RuleKind::Select) else {
                                let msg = format!("kind `{kind_name}` is built `from` an operand; it binds to a fetched operand, not to a field");
                                self.error(module, "E1033", f.span, msg);
                                ok = false;
                                continue;
                            };
                            if params[0].1 != oinfo.fields[fi].ty {
                                let msg = format!(
                                    "field `{}` is `{}` but `{kind_name}.select` takes `{}`",
                                    f.name.name,
                                    self.display(&oinfo.fields[fi].ty),
                                    self.display(&params[0].1)
                                );
                                self.error(module, "E1033", f.span, msg);
                                ok = false;
                            }
                            // Which operand binding this select feeds is
                            // resolved below; record the slot now.
                            select_bindings.push((kdef, step, fi as u32));
                            selects.push((fi as u32, u32::MAX));
                        }
                        fetch_steps.push((odef, step));
                        steps.push(hir::EncodingStep::Fetch {
                            operand: odef,
                            selects,
                        });
                    }
                    if !steps
                        .iter()
                        .any(|s| matches!(s, hir::EncodingStep::Opcode(b) if !b.is_empty()))
                    {
                        self.error(
                            module,
                            "E1032",
                            decl.name.span,
                            format!("instruction `{}` has no `opcode`", decl.name.name),
                        );
                        ok = false;
                    }
                    encoding = Some(steps);
                }
                InstructionItem::Operands(_) => {} // in the signature
                InstructionItem::Flags(sets) => {
                    for set in sets {
                        let target = match set.name.name.as_str() {
                            "modifies" => &mut flags.modifies,
                            "undefined" => &mut flags.undefined,
                            "reads" => &mut flags.reads,
                            other => {
                                self.error(module, "E1034", set.name.span, format!("unknown flag set `{other}`; use `modifies`, `undefined` or `reads`"));
                                ok = false;
                                continue;
                            }
                        };
                        let Some(A::ArrayLit(names)) = set.value.as_ref().map(|v| &v.kind) else {
                            self.error(
                                module,
                                "E1034",
                                set.span,
                                "a flag set is a list: `modifies: [CF, ZF]`",
                            );
                            ok = false;
                            continue;
                        };
                        for n in names {
                            let A::Path(p) = &n.kind else {
                                self.error(module, "E1034", n.span, "flags are register fields");
                                ok = false;
                                continue;
                            };
                            let fname = p.last().name.clone();
                            match self.lookup(scope, &fname) {
                                Some(ScopeEntry::RegField { reg, index }) => {
                                    target.push((reg, index))
                                }
                                _ => {
                                    self.error(
                                        module,
                                        "E1034",
                                        n.span,
                                        format!("`{fname}` is not a register field"),
                                    );
                                    ok = false;
                                }
                            }
                        }
                    }
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
                            self.error(
                                module,
                                "E1032",
                                it.name.span,
                                format!(
                                    "unknown timing attribute `{}`; use `cycles`",
                                    it.name.name
                                ),
                            );
                            ok = false;
                            continue;
                        }
                        let Some(v) = &it.value else {
                            self.error(module, "E1032", it.span, "`cycles` takes an expression");
                            ok = false;
                            continue;
                        };
                        let mut cx = FnCtx::new(scope, HashMap::new(), None);
                        cx.operands = sig.operands.clone();
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
                    let mut cx = FnCtx::new(scope, HashMap::new(), None);
                    cx.operands = sig.operands.clone();
                    requires = Some(self.check_expr(&mut cx, e, Some(&Ty::Bool)));
                }
                InstructionItem::Other(name, _) => {
                    self.error(
                        module,
                        "E1032",
                        name.span,
                        format!("unknown instruction item `{}`", name.name),
                    );
                    ok = false;
                }
            }
        }

        if mnemonic.is_none() {
            self.error(
                module,
                "E1032",
                decl.name.span,
                format!("instruction `{}` needs a `mnemonic`", decl.name.name),
            );
            ok = false;
        }
        let Some(mut encoding) = encoding else {
            self.error(
                module,
                "E1032",
                decl.name.span,
                format!("instruction `{}` needs an `encoding`", decl.name.name),
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
        let mut used_selects = vec![false; select_bindings.len()];
        let mut used_fetches = vec![false; fetch_steps.len()];
        for (oi, (oname, kdef)) in sig.operands.iter().enumerate() {
            let Some(ksig) = self.kinds.get(kdef).cloned() else {
                ok = false;
                continue;
            };
            let source = match ksig.source {
                None => {
                    let candidates: Vec<usize> = select_bindings
                        .iter()
                        .enumerate()
                        .filter(|(i, (k, _, _))| *k == *kdef && !used_selects[*i])
                        .map(|(i, _)| i)
                        .collect();
                    match candidates.as_slice() {
                        [i] => {
                            used_selects[*i] = true;
                            let (_, step, field) = select_bindings[*i];
                            // Record the binding index in the step.
                            if let hir::EncodingStep::Fetch { selects, .. } =
                                &mut encoding[step as usize]
                            {
                                for s in selects.iter_mut() {
                                    if s.0 == field {
                                        s.1 = oi as u32;
                                    }
                                }
                            }
                            hir::BindSource::Select { step, field }
                        }
                        [] => {
                            let msg = format!(
                                "operand `{oname}` of kind `{}` is not bound by the encoding",
                                self.prog.item_name(*kdef)
                            );
                            self.error_with(
                                module,
                                Diagnostic::error("E1033", decl.name.span, msg).with_note(format!(
                                    "add `{{ field: {} }}` to a fetched operand in `encoding`",
                                    self.prog.item_name(*kdef)
                                )),
                            );
                            ok = false;
                            continue;
                        }
                        _ => {
                            let msg = format!("operand `{oname}` is ambiguous: several encoding fields bind kind `{}`", self.prog.item_name(*kdef));
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
                                "operand `{oname}` of kind `{}` needs a fetched `{}` in the encoding",
                                self.prog.item_name(*kdef),
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
        for (i, used) in used_selects.iter().enumerate() {
            if !used {
                let (k, _, _) = select_bindings[i];
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
        let place = self.check_expr(cx, target, None);
        if place.ty.is_error() {
            self.check_expr(cx, value, None);
            return None;
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

fn hex_bytes(b: &[u8]) -> String {
    let parts: Vec<String> = b.iter().map(|x| format!("{x:#04x}")).collect();
    format!("[{}]", parts.join(", "))
}
