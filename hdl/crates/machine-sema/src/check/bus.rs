//! Checking a `bus`: roles, wires, transaction and signal signatures.
//!
//! The wire-level role bodies of transactions (the cycle sequences) are the
//! accurate simulator's concern and are not checked yet; this pass produces
//! what components need to talk to the bus at the transaction level.

use std::collections::HashMap;

use machine_syntax::ast::{self, BusItem, WireAttr};
use machine_syntax::diagnostics::Diagnostic;

use super::{Checker, FnCtx, Scope};
use crate::hir::{self, ExprKind as H, WireDriver};
use crate::program::{DefId, ItemKind};
use crate::ty::Ty;
use crate::Skipped;

impl Checker<'_> {
    pub(super) fn check_bus(&mut self, bus: DefId) -> Option<hir::BusInfo> {
        let module = self.prog.item(bus).module;
        let scope = Scope {
            module,
            isa: None,
            bus: Some(bus),
        };
        let decl = self.prog.bus_decl(bus);
        let name = decl.name.name.clone();
        let empty = HashMap::new();
        let mut ok = true;

        for id in self.prog.bus_items_of(bus, ItemKind::Enum) {
            self.enum_info(id);
        }

        // Roles.
        let mut roles: Vec<String> = Vec::new();
        let mut roles_seen = false;
        let mut clock = None;
        for item in &decl.items {
            match item {
                BusItem::Roles(rs) => {
                    if roles_seen {
                        self.error(
                            module,
                            "E1040",
                            decl.name.span,
                            "a bus declares `roles` once",
                        );
                        ok = false;
                    }
                    roles_seen = true;
                    for r in rs {
                        if roles.contains(&r.name) {
                            self.error(
                                module,
                                "E1001",
                                r.span,
                                format!("duplicate role `{}`", r.name),
                            );
                            ok = false;
                        } else {
                            roles.push(r.name.clone());
                        }
                    }
                }
                BusItem::Clock(c) => {
                    if clock.replace(c.name.clone()).is_some() {
                        self.error(module, "E1040", c.span, "a bus has one clock");
                        ok = false;
                    }
                }
                BusItem::Params(ps) if !ps.is_empty() => {
                    self.error(
                        module,
                        "E1040",
                        ps[0].span,
                        "bus parameters are not supported yet; write the widths out",
                    );
                    ok = false;
                }
                _ => {}
            }
        }
        if roles.is_empty() {
            self.error_with(
                module,
                Diagnostic::error("E1040", decl.name.span, format!("bus `{name}` declares no `roles`"))
                    .with_note("the first role initiates transactions; the others respond: `roles { Master; Slave; }`"),
            );
            return None;
        }

        // Wires.
        let mut wires: Vec<hir::BusWire> = Vec::new();
        for item in &decl.items {
            let BusItem::Wires(ws) = item else { continue };
            for w in ws {
                let ty = self.resolve_type(scope, &empty, &w.ty);
                let wire_ok = match &ty {
                    Ty::Bool | Ty::UInt(_) => true,
                    Ty::Array(elem, _) => matches!(**elem, Ty::Bool | Ty::UInt(_)),
                    Ty::Enum(_) => true,
                    Ty::Error => false,
                    _ => {
                        let msg = format!(
                            "bus wires are `bool`, `u<N>`, enums or arrays of those, not `{}`",
                            self.display(&ty)
                        );
                        self.error(module, "E1040", w.ty.span, msg);
                        false
                    }
                };
                let mut driver: Option<WireDriver> = None;
                let mut active_low = false;
                for a in &w.attrs {
                    let d = match a {
                        WireAttr::DrivenBy(role) => {
                            match roles.iter().position(|r| *r == role.name) {
                                Some(i) => Some(WireDriver::Role(i as u32)),
                                None => {
                                    self.error(
                                        module,
                                        "E1040",
                                        role.span,
                                        format!("`{}` is not a role of bus `{name}`", role.name),
                                    );
                                    ok = false;
                                    // Placeholder so the wire is not also reported as driverless.
                                    Some(WireDriver::Shared)
                                }
                            }
                        }
                        WireAttr::Shared => Some(WireDriver::Shared),
                        WireAttr::WiredAnd => Some(WireDriver::WiredAnd),
                        WireAttr::WiredOr => Some(WireDriver::WiredOr),
                        WireAttr::ActiveLow => {
                            active_low = true;
                            None
                        }
                        WireAttr::Other(n, _) => {
                            self.error_with(
                                module,
                                Diagnostic::error("E1040", n.span, format!("unknown wire attribute `{}`", n.name))
                                    .with_note("attributes are `driven_by <Role>`, `shared`, `wired_and`, `wired_or`, `active_low`"),
                            );
                            ok = false;
                            Some(WireDriver::Shared)
                        }
                    };
                    if let Some(d) = d {
                        if driver.replace(d).is_some() {
                            self.error(
                                module,
                                "E1040",
                                w.span,
                                format!(
                                    "wire `{}` has more than one driver attribute",
                                    w.name.name
                                ),
                            );
                            ok = false;
                        }
                    }
                }
                let Some(driver) = driver else {
                    self.error_with(
                        module,
                        Diagnostic::error("E1040", w.span, format!("wire `{}` needs a driver attribute", w.name.name))
                            .with_note("`driven_by <Role>` for a single driver, `shared` for a tristate bus, `wired_and`/`wired_or` for open-collector lines"),
                    );
                    ok = false;
                    continue;
                };
                if wires.iter().any(|x| x.name == w.name.name) {
                    self.error(
                        module,
                        "E1001",
                        w.name.span,
                        format!("duplicate wire `{}`", w.name.name),
                    );
                    ok = false;
                }
                if !wire_ok {
                    ok = false;
                }
                wires.push(hir::BusWire {
                    name: w.name.name.clone(),
                    ty,
                    driver,
                    active_low,
                });
            }
        }

        // Transactions.
        let mut transactions = Vec::new();
        let mut has_role_bodies = false;
        for id in self.prog.bus_items_of(bus, ItemKind::Transaction) {
            let t = self.prog.transaction_decl(id);
            let mut params = Vec::new();
            for p in &t.params {
                let ty = self.resolve_type(scope, &empty, &p.ty);
                ok &= !ty.is_error();
                params.push((p.name.name.clone(), ty));
            }
            let ret = match &t.ret {
                None => None,
                Some(r) => {
                    let ty = self.resolve_type(scope, &empty, r);
                    match &ty {
                        Ty::Tuple(items) if items.iter().all(|(n, _)| n.is_some()) => Some(ty),
                        Ty::Error => {
                            ok = false;
                            None
                        }
                        _ => {
                            self.error_with(
                                module,
                                Diagnostic::error("E1040", r.span, "a transaction returns a named tuple")
                                    .with_note("write `-> (data: u8)`; the names become the handler's return fields"),
                            );
                            ok = false;
                            None
                        }
                    }
                }
            };
            for rb in &t.roles {
                if !roles.contains(&rb.role.name) {
                    self.error(
                        module,
                        "E1040",
                        rb.role.span,
                        format!("`{}` is not a role of bus `{name}`", rb.role.name),
                    );
                    ok = false;
                }
                has_role_bodies = true;
            }
            transactions.push(hir::TransactionSig {
                def: id,
                name: t.name.name.clone(),
                params,
                ret,
            });
        }
        if has_role_bodies {
            self.skipped.push(Skipped {
                module,
                kind: "bus protocol",
                name: name.clone(),
            });
        }

        // Signals: a wire (or element of a wire array) by name.
        let mut signals = Vec::new();
        for id in self.prog.bus_items_of(bus, ItemKind::Signal) {
            let s = self.prog.signal_decl(id);
            let mut cx = FnCtx::new(scope, HashMap::new(), None);
            let wire_locals: Vec<u32> = wires
                .iter()
                .map(|w| cx.declare(&w.name, w.ty.clone(), s.span))
                .collect();
            let mut params = Vec::new();
            let mut param_ids = Vec::new();
            for p in &s.params {
                let ty = self.resolve_type(scope, &empty, &p.ty);
                ok &= !ty.is_error();
                param_ids.push(cx.declare(&p.name.name, ty.clone(), p.name.span));
                params.push((p.name.name.clone(), ty));
            }
            let expr = self.check_expr(&mut cx, &s.value, Some(&Ty::Bool));
            // The signal's driver is that of the wire it reads.
            let root = match &expr.kind {
                H::Local(l) => Some(*l),
                H::Index(b, _) => match &b.kind {
                    H::Local(l) => Some(*l),
                    _ => None,
                },
                _ => None,
            };
            let driver = match root.and_then(|l| wire_locals.iter().position(|w| *w == l)) {
                Some(wi) => wires[wi].driver,
                None => {
                    if !expr.ty.is_error() {
                        self.error_with(
                            module,
                            Diagnostic::error(
                                "E1040",
                                s.value.span,
                                "a signal is a `bool` wire or one element of a wire array",
                            )
                            .with_note("`signal interrupt(line: u2) = irq[line];`"),
                        );
                    }
                    ok = false;
                    WireDriver::Shared
                }
            };
            signals.push(hir::SignalInfo {
                def: id,
                name: s.name.name.clone(),
                params,
                locals: cx.locals,
                param_ids,
                expr,
                driver,
            });
        }

        if !ok {
            return None;
        }
        Some(hir::BusInfo {
            def: bus,
            name,
            clock,
            roles,
            wires,
            transactions,
            signals,
        })
    }

    /// Parse `Bus::Role` from a port type.
    pub(super) fn resolve_bus_role(
        &mut self,
        scope: Scope,
        ty: &ast::Type,
    ) -> Option<(DefId, u32)> {
        let module = scope.module;
        let ast::TypeKind::Named(path, args) = &ty.kind else {
            self.error(module, "E1042", ty.span, "a bus port's type is `Bus::Role`");
            return None;
        };
        if path.segments.len() != 2 || !args.is_empty() {
            self.error(module, "E1042", ty.span, "a bus port's type is `Bus::Role`");
            return None;
        }
        let (bname, rname) = (&path.segments[0], &path.segments[1]);
        let Some(bus) = self.lookup_item(scope, &bname.name) else {
            self.error(
                module,
                "E1000",
                bname.span,
                format!("unknown bus `{}`", bname.name),
            );
            return None;
        };
        if self.prog.item(bus).kind != ItemKind::Bus {
            self.error(
                module,
                "E1042",
                bname.span,
                format!(
                    "`{}` is a {}, not a bus",
                    bname.name,
                    self.prog.item(bus).kind.describe()
                ),
            );
            return None;
        }
        let info = self.buses.get(&bus).cloned()?;
        match info.roles.iter().position(|r| *r == rname.name) {
            Some(i) => Some((bus, i as u32)),
            None => {
                let msg = format!(
                    "bus `{}` has no role `{}` (roles: {})",
                    bname.name,
                    rname.name,
                    info.roles.join(", ")
                );
                self.error(module, "E1042", rname.span, msg);
                None
            }
        }
    }
}
