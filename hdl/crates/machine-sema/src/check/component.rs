//! Checking a `component`: ports, the `isa` reference, and the `model`
//! body. The `rtl` body is not checked yet.

use std::collections::HashMap;
use std::rc::Rc;

use machine_syntax::ast::{self, ExprKind as A, PortDir, StmtKind as S};
use machine_syntax::diagnostics::Diagnostic;
use machine_syntax::span::Span;

use super::{Checker, FnCtx, Scope};
use crate::hir::{self, ExprKind as H, HandlerKind, PortKind, WireDriver};
use crate::program::{DefId, ItemKind};
use crate::ty::Ty;
use crate::Skipped;

/// What a bare name denotes inside a component's bodies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompEntry {
    Port(u32),
    State(u32),
    Mem(u32),
}

/// The names a component's bodies can see, plus what the checker needs to
/// type them.
#[derive(Debug)]
pub struct CompScope {
    pub ports: Vec<hir::Port>,
    pub states: Vec<(String, Ty)>,
    pub mems: Vec<(String, Ty, u64)>,
    pub names: HashMap<String, CompEntry>,
    /// The processor's isa, whose spaces a model body binds.
    pub isa: Option<DefId>,
}

impl Checker<'_> {
    pub(super) fn check_component(&mut self, comp: DefId) -> Option<hir::ComponentInfo> {
        let module = self.prog.item(comp).module;
        let scope = Scope {
            module,
            isa: None,
            bus: None,
        };
        let decl = self.prog.component_decl(comp);
        let name = decl.name.name.clone();
        let empty = HashMap::new();
        let mut ok = true;

        if !decl.generics.is_empty() {
            self.error(
                module,
                "E1042",
                decl.generics[0].span,
                "generic components are not supported yet",
            );
            ok = false;
        }

        // The isa reference.
        let mut isa = None;
        if let Some(p) = &decl.isa {
            let target = if p.segments.len() == 1 {
                self.lookup_item(scope, &p.segments[0].name)
            } else {
                None
            };
            match target {
                Some(d) if self.prog.item(d).kind == ItemKind::Isa => isa = Some(d),
                _ => {
                    self.error(
                        module,
                        "E1000",
                        p.span,
                        format!("`{}` is not an isa", p.dotted()),
                    );
                    ok = false;
                }
            }
        }

        // Ports.
        let mut ports: Vec<hir::Port> = Vec::new();
        let mut names: HashMap<String, CompEntry> = HashMap::new();
        for p in &decl.ports {
            let kind = match &p.kind {
                ast::PortKind::Wire { dir, ty } => {
                    let t = self.resolve_type(scope, &empty, ty);
                    ok &= !t.is_error();
                    match dir {
                        PortDir::In => PortKind::In(t),
                        PortDir::Out => PortKind::Out(t),
                        PortDir::AsyncIn => PortKind::AsyncIn(t),
                    }
                }
                ast::PortKind::Bus(ty) => match self.resolve_bus_role(scope, ty) {
                    Some((bus, role)) => PortKind::Bus { bus, role },
                    None => {
                        ok = false;
                        continue;
                    }
                },
                ast::PortKind::Clock => PortKind::Clock,
            };
            if names
                .insert(p.name.name.clone(), CompEntry::Port(ports.len() as u32))
                .is_some()
            {
                self.error(
                    module,
                    "E1001",
                    p.name.span,
                    format!("duplicate port `{}`", p.name.name),
                );
                ok = false;
            }
            ports.push(hir::Port {
                name: p.name.name.clone(),
                kind,
                span: p.span,
            });
        }

        // Model state and memories: collected before any body so handlers
        // may appear in any order.
        let mut states: Vec<(String, Ty)> = Vec::new();
        let mut mems: Vec<(String, Ty, u64)> = Vec::new();
        let mut state_decls: Vec<(&ast::Stmt, Ty)> = Vec::new();
        if let Some(model) = &decl.model {
            for s in &model.stmts {
                match &s.kind {
                    S::State { name: n, ty, .. } => {
                        let t = self.resolve_type(scope, &empty, ty);
                        ok &= !t.is_error();
                        if names
                            .insert(n.name.clone(), CompEntry::State(states.len() as u32))
                            .is_some()
                        {
                            self.error(
                                module,
                                "E1001",
                                n.span,
                                format!("`{}` is already a port or state of `{name}`", n.name),
                            );
                            ok = false;
                        }
                        states.push((n.name.clone(), t.clone()));
                        state_decls.push((s, t));
                    }
                    S::Mem {
                        name: n,
                        ty,
                        options,
                        ..
                    } => {
                        let t = self.resolve_type(scope, &empty, ty);
                        let (elem, len) = match &t {
                            Ty::Array(e, n) => ((**e).clone(), *n),
                            Ty::Error => {
                                ok = false;
                                (Ty::Error, 0)
                            }
                            other => {
                                let msg = format!(
                                    "a `mem` is an array type, found `{}`",
                                    self.display(other)
                                );
                                self.error(module, "E1043", ty.span, msg);
                                ok = false;
                                (Ty::Error, 0)
                            }
                        };
                        for o in options {
                            if let ast::MemOption::Other(k, _) = o {
                                self.error(
                                    module,
                                    "E1043",
                                    k.span,
                                    format!("unknown `mem` option `{}`", k.name),
                                );
                                ok = false;
                            }
                        }
                        if names
                            .insert(n.name.clone(), CompEntry::Mem(mems.len() as u32))
                            .is_some()
                        {
                            self.error(
                                module,
                                "E1001",
                                n.span,
                                format!("`{}` is already a port or state of `{name}`", n.name),
                            );
                            ok = false;
                        }
                        mems.push((n.name.clone(), elem, len));
                    }
                    _ => {}
                }
            }
        }

        self.comps.insert(
            comp,
            Rc::new(CompScope {
                ports: ports.clone(),
                states: states.clone(),
                mems: mems.clone(),
                names,
                isa,
            }),
        );

        // Model body.
        let mut model = None;
        if let Some(body) = &decl.model {
            let mut init_cx = FnCtx::new(scope, HashMap::new(), None);
            init_cx.component = Some(comp);
            let mut hstates = Vec::new();
            for (s, t) in &state_decls {
                let S::State { name: n, init, .. } = &s.kind else {
                    unreachable!()
                };
                let init = init
                    .as_ref()
                    .map(|e| self.check_expr(&mut init_cx, e, Some(t)));
                hstates.push(hir::ModelState {
                    name: n.name.clone(),
                    ty: t.clone(),
                    init,
                });
            }
            let hmems = mems
                .iter()
                .map(|(n, e, l)| hir::ModelMem {
                    name: n.clone(),
                    elem: e.clone(),
                    len: *l,
                })
                .collect();
            let mut handlers = Vec::new();
            for s in &body.stmts {
                match &s.kind {
                    S::State { .. } | S::Mem { .. } | S::Error => {}
                    S::On {
                        target,
                        clock,
                        params,
                        ret,
                        body,
                    } => {
                        if let Some(h) = self.check_handler(
                            comp,
                            scope,
                            target,
                            clock,
                            params.as_deref(),
                            ret.as_ref(),
                            body,
                            s.span,
                        ) {
                            handlers.push(h);
                        } else {
                            ok = false;
                        }
                    }
                    _ => {
                        self.error_with(
                            module,
                            Diagnostic::error("E1043", s.span, "a `model` body contains `state`, `mem` and `on` handlers")
                                .with_note("put logic inside a handler: `on reset { ... }`, `on tick(n: u32) { ... }`, `on port.txn(...) -> (...) { ... }`"),
                        );
                        ok = false;
                    }
                }
            }
            if let Some(isa) = isa {
                ok &= self.check_space_bindings(module, isa, &handlers, decl.name.span);
            }
            model = Some(hir::ModelBody {
                init_locals: init_cx.locals,
                states: hstates,
                mems: hmems,
                handlers,
            });
        }

        if decl.rtl.is_some() {
            self.skipped.push(Skipped {
                module,
                kind: "rtl",
                name: name.clone(),
            });
        }
        if ok && decl.rtl.is_none() && decl.model.is_none() && isa.is_none() {
            self.warning(
                module,
                "W1004",
                decl.name.span,
                format!("component `{name}` has neither an `rtl` nor a `model` body"),
            );
        }

        if !ok {
            return None;
        }
        Some(hir::ComponentInfo {
            def: comp,
            name,
            isa,
            ports,
            model,
            has_rtl: decl.rtl.is_some(),
        })
    }

    /// One `on ...` handler of a model body.
    #[allow(clippy::too_many_arguments)]
    fn check_handler(
        &mut self,
        comp: DefId,
        scope: Scope,
        target: &ast::Path,
        clock: &Option<ast::Ident>,
        params: Option<&[ast::Param]>,
        ret: Option<&ast::Type>,
        body: &ast::Block,
        span: Span,
    ) -> Option<hir::Handler> {
        let module = scope.module;
        let cs = self.comps.get(&comp).cloned()?;
        let empty = HashMap::new();

        // Declared parameter and return types.
        let mut decl_params = Vec::new();
        for p in params.unwrap_or(&[]) {
            let t = self.resolve_type(scope, &empty, &p.ty);
            decl_params.push((p.name.name.clone(), t, p.name.span));
        }
        let decl_ret = ret.map(|r| self.resolve_type(scope, &empty, r));

        let (kind, expected_params, expected_ret): (HandlerKind, Option<Vec<(String, Ty)>>, Ty) =
            match target.segments.as_slice() {
                [single] if single.name == "reset" => {
                    if clock.is_some() || params.is_some() || ret.is_some() {
                        self.error(
                            module,
                            "E1041",
                            span,
                            "`on reset` takes no clock, parameters or return type",
                        );
                        return None;
                    }
                    (HandlerKind::Reset, Some(Vec::new()), Ty::Unit)
                }
                [single] if single.name == "tick" => {
                    let clocks: Vec<u32> = cs
                        .ports
                        .iter()
                        .enumerate()
                        .filter(|(_, p)| matches!(p.kind, PortKind::Clock))
                        .map(|(i, _)| i as u32)
                        .collect();
                    let clock_port = match clock {
                        Some(c) => match cs.names.get(&c.name) {
                            Some(CompEntry::Port(i))
                                if matches!(cs.ports[*i as usize].kind, PortKind::Clock) =>
                            {
                                Some(*i)
                            }
                            _ => {
                                self.error(
                                    module,
                                    "E1041",
                                    c.span,
                                    format!("`{}` is not a clock port of this component", c.name),
                                );
                                return None;
                            }
                        },
                        None => match clocks.as_slice() {
                            [] => None,
                            [one] => Some(*one),
                            _ => {
                                let names: Vec<&str> = clocks
                                    .iter()
                                    .map(|i| cs.ports[*i as usize].name.as_str())
                                    .collect();
                                let msg = format!("this component has several clocks; say which: `on tick @ <clock> (n: u32)` (one of {})", names.join(", "));
                                self.error(module, "E1041", span, msg);
                                return None;
                            }
                        },
                    };
                    if ret.is_some() {
                        self.error(module, "E1041", span, "`on tick` returns nothing");
                        return None;
                    }
                    let pname = decl_params
                        .first()
                        .map(|p| p.0.clone())
                        .unwrap_or_else(|| "n".into());
                    (
                        HandlerKind::Tick { clock: clock_port },
                        Some(vec![(pname, Ty::UInt(32))]),
                        Ty::Unit,
                    )
                }
                [port, event]
                    if !cs.names.contains_key(&port.name)
                        && self.isa_space(cs.isa, &port.name).is_some() =>
                {
                    let sp = self.isa_space(cs.isa, &port.name).unwrap();
                    let parsed = event
                        .name
                        .strip_prefix("read_u")
                        .map(|b| (false, b))
                        .or_else(|| event.name.strip_prefix("write_u").map(|b| (true, b)))
                        .and_then(|(w, b)| b.parse::<u32>().ok().map(|b| (w, b)));
                    let Some((write, bits)) =
                        parsed.filter(|(_, b)| *b > 0 && *b <= 128 && b % sp.cell_bits == 0)
                    else {
                        let msg = format!(
                            "a space handler is `on {0}.read_uN(addr) -> uN` or `on {0}.write_uN(addr, data: uN)`, N a multiple of the {1}-bit cell; found `{2}`",
                            sp.name, sp.cell_bits, event.name
                        );
                        self.error(module, "E1041", event.span, msg);
                        return None;
                    };
                    if clock.is_some() {
                        self.error(module, "E1041", span, "a space handler takes no `@ clock`");
                        return None;
                    }
                    if write {
                        (
                            HandlerKind::SpaceWrite {
                                space: sp.def,
                                bits,
                            },
                            Some(vec![
                                ("addr".into(), sp.addr.clone()),
                                ("data".into(), Ty::UInt(bits)),
                            ]),
                            Ty::Unit,
                        )
                    } else {
                        (
                            HandlerKind::SpaceRead {
                                space: sp.def,
                                bits,
                            },
                            Some(vec![("addr".into(), sp.addr.clone())]),
                            Ty::UInt(bits),
                        )
                    }
                }
                [port, event] => {
                    let Some(CompEntry::Port(pi)) = cs.names.get(&port.name).copied() else {
                        self.error(
                            module,
                            "E1041",
                            port.span,
                            format!("`{}` is not a port of this component", port.name),
                        );
                        return None;
                    };
                    let PortKind::Bus { bus, role } = cs.ports[pi as usize].kind else {
                        self.error(module, "E1041", port.span, format!("`{}` is not a bus port; handlers respond to bus transactions and signals", port.name));
                        return None;
                    };
                    if clock.is_some() {
                        self.error(
                            module,
                            "E1041",
                            span,
                            "a bus handler runs on the bus clock; it takes no `@ clock`",
                        );
                        return None;
                    }
                    let info = self.buses.get(&bus).cloned()?;
                    if let Some(t) = info.transactions.iter().find(|t| t.name == event.name) {
                        if role == 0 {
                            let msg = format!(
                            "role `{}` initiates `{}`; only responding roles handle it (call it with `{}.{}(...)` instead)",
                            info.roles[0], event.name, port.name, event.name
                        );
                            self.error(module, "E1041", span, msg);
                            return None;
                        }
                        let mut want_ret: Vec<(Option<String>, Ty)> = match &t.ret {
                            Some(Ty::Tuple(items)) => items.clone(),
                            _ => Vec::new(),
                        };
                        want_ret.push((Some("wait".into()), Ty::UInt(8)));
                        (
                            HandlerKind::Transaction {
                                port: pi,
                                transaction: t.def,
                            },
                            Some(t.params.clone()),
                            Ty::Tuple(want_ret),
                        )
                    } else if let Some(s) = info.signals.iter().find(|s| s.name == event.name) {
                        if s.driver == WireDriver::Role(role) {
                            let msg = format!("role `{}` drives `{}`; it emits the signal rather than observing it", info.roles[role as usize], event.name);
                            self.error(module, "E1041", span, msg);
                            return None;
                        }
                        if ret.is_some() {
                            self.error(module, "E1041", span, "a signal handler returns nothing");
                            return None;
                        }
                        let mut want = s.params.clone();
                        want.push(("value".into(), Ty::Bool));
                        (
                            HandlerKind::Signal {
                                port: pi,
                                signal: s.def,
                            },
                            Some(want),
                            Ty::Unit,
                        )
                    } else {
                        let msg = format!(
                            "bus `{}` has no transaction or signal `{}`",
                            info.name, event.name
                        );
                        self.error(module, "E1041", event.span, msg);
                        return None;
                    }
                }
                _ => {
                    self.error_with(
                    module,
                    Diagnostic::error("E1041", target.span, format!("unknown handler `{}`", target.dotted()))
                        .with_note("handlers are `on reset`, `on tick [@ clock] (n: u32)`, `on <bus port>.<transaction>(...) -> (..., wait: u8)` and `on <bus port>.<signal>(..., value: bool)`"),
                );
                    return None;
                }
            };

        // Compare the declared signature with the expected one.
        if let Some(want) = &expected_params {
            let got: Vec<(String, Ty)> = decl_params
                .iter()
                .map(|(n, t, _)| (n.clone(), t.clone()))
                .collect();
            let same = got.len() == want.len()
                && got
                    .iter()
                    .zip(want)
                    .all(|(g, w)| g.0 == w.0 && (g.1 == w.1 || g.1.is_error()));
            if !same {
                let render = |ck: &Checker, ps: &[(String, Ty)]| -> String {
                    ps.iter()
                        .map(|(n, t)| format!("{n}: {}", ck.display(t)))
                        .collect::<Vec<_>>()
                        .join(", ")
                };
                let msg = format!(
                    "handler `{}` takes ({}), but the protocol defines ({})",
                    target.dotted(),
                    render(self, &got),
                    render(self, want)
                );
                self.error(module, "E1041", span, msg);
                return None;
            }
        }
        let ret_ty = match &decl_ret {
            Some(t) if *t == expected_ret || t.is_error() => t.clone(),
            None if expected_ret == Ty::Unit => Ty::Unit,
            Some(t) => {
                let msg = format!(
                    "handler `{}` returns `{}`, but the protocol requires `{}`",
                    target.dotted(),
                    self.display(t),
                    self.display(&expected_ret)
                );
                self.error(module, "E1041", span, msg);
                return None;
            }
            None => {
                let msg = format!(
                    "handler `{}` must return `{}`",
                    target.dotted(),
                    self.display(&expected_ret)
                );
                self.error(module, "E1041", span, msg);
                return None;
            }
        };

        // The body.
        let mut cx = FnCtx::new(scope, HashMap::new(), Some(ret_ty.clone()));
        cx.component = Some(comp);
        cx.responding = matches!(kind, HandlerKind::Transaction { .. });
        let mut param_ids = Vec::new();
        for (n, t, sp) in &decl_params {
            param_ids.push(cx.declare(n, t.clone(), *sp));
        }
        let hbody = self.check_block(&mut cx, body);
        if !matches!(ret_ty, Ty::Unit | Ty::Error) && !super::stmt::block_diverges(&hbody) {
            let msg = format!(
                "handler `{}` does not return a value on every path",
                target.dotted()
            );
            self.error(module, "E1015", span, msg);
        }
        Some(hir::Handler {
            kind,
            locals: cx.locals,
            params: param_ids,
            ret: ret_ty,
            body: hbody,
            span,
        })
    }

    /// A space of the component's isa, by name.
    fn isa_space(&self, isa: Option<DefId>, name: &str) -> Option<hir::Space> {
        let isa = isa?;
        self.isas
            .iter()
            .find(|i| i.def == isa)?
            .spaces
            .iter()
            .find(|s| s.name == name)
            .cloned()
    }

    /// A processor's model body binds every isa space to its buses: at least
    /// the one-cell read and write; wider accesses without a handler are
    /// composed from cells in the space's byte order.
    fn check_space_bindings(
        &mut self,
        module: crate::program::ModuleId,
        isa: DefId,
        handlers: &[hir::Handler],
        span: Span,
    ) -> bool {
        let Some(info) = self.isas.iter().find(|i| i.def == isa).cloned() else {
            return true;
        };
        let mut ok = true;
        for sp in &info.spaces {
            for write in [false, true] {
                let found = handlers.iter().any(|h| match h.kind {
                    HandlerKind::SpaceRead { space, bits } => {
                        !write && space == sp.def && bits == sp.cell_bits
                    }
                    HandlerKind::SpaceWrite { space, bits } => {
                        write && space == sp.def && bits == sp.cell_bits
                    }
                    _ => false,
                });
                if !found {
                    let msg = if write {
                        format!("the model does not bind writes to space `{0}`: add `on {0}.write_u{1}(addr: ..., data: u{1}) {{ ... }}`", sp.name, sp.cell_bits)
                    } else {
                        format!("the model does not bind reads of space `{0}`: add `on {0}.read_u{1}(addr: ...) -> u{1} {{ ... }}`", sp.name, sp.cell_bits)
                    };
                    self.error(module, "E1041", span, msg);
                    ok = false;
                }
            }
        }
        ok
    }

    /// `emit port.signal(args) = value;`
    pub(super) fn check_emit(
        &mut self,
        cx: &mut FnCtx,
        target: &ast::Expr,
        value: Option<&ast::Expr>,
        span: Span,
    ) -> Option<hir::StmtKind> {
        let Some(comp) = cx.component else {
            self.error(
                cx.module,
                "E1020",
                span,
                "`emit` is only allowed in a model body",
            );
            return None;
        };
        let cs = self.comps.get(&comp).cloned()?;
        let (callee, args): (&ast::Expr, &[ast::Arg]) = match &target.kind {
            A::Call { callee, args, .. } => (callee, args),
            _ => (target, &[]),
        };
        let (port_name, sig_name) = match &callee.kind {
            A::Field(base, f) => match &base.kind {
                A::Path(p) if p.segments.len() == 1 => (&p.segments[0], f),
                _ => {
                    self.error(
                        cx.module,
                        "E1044",
                        target.span,
                        "`emit` names a bus port and a signal: `emit io.interrupt(0) = level;`",
                    );
                    return None;
                }
            },
            _ => {
                self.error(
                    cx.module,
                    "E1044",
                    target.span,
                    "`emit` names a bus port and a signal: `emit io.interrupt(0) = level;`",
                );
                return None;
            }
        };
        let Some(CompEntry::Port(pi)) = cs.names.get(&port_name.name).copied() else {
            self.error(
                cx.module,
                "E1044",
                port_name.span,
                format!("`{}` is not a port of this component", port_name.name),
            );
            return None;
        };
        let PortKind::Bus { bus, role } = cs.ports[pi as usize].kind else {
            self.error(
                cx.module,
                "E1044",
                port_name.span,
                format!("`{}` is not a bus port", port_name.name),
            );
            return None;
        };
        let info = self.buses.get(&bus).cloned()?;
        let Some(sig) = info.signals.iter().find(|s| s.name == sig_name.name) else {
            self.error(
                cx.module,
                "E1044",
                sig_name.span,
                format!("bus `{}` has no signal `{}`", info.name, sig_name.name),
            );
            return None;
        };
        match sig.driver {
            WireDriver::Role(r) if r != role => {
                let msg = format!(
                    "signal `{}` is driven by role `{}`; this port is a `{}`",
                    sig.name, info.roles[r as usize], info.roles[role as usize]
                );
                self.error(cx.module, "E1044", span, msg);
                return None;
            }
            _ => {}
        }
        let ordered = self.order_args(
            cx,
            &format!("signal `{}`", sig.name),
            &sig.params,
            args,
            target.span,
        )?;
        let mut hargs = Vec::new();
        for (a, (_, t)) in ordered.iter().zip(&sig.params) {
            hargs.push(self.check_expr(cx, &a.value, Some(t)));
        }
        let Some(value) = value else {
            self.error_with(
                cx.module,
                Diagnostic::error(
                    "E1044",
                    span,
                    "`emit` sets a signal level: `emit port.signal(args) = <bool>;`",
                ),
            );
            return None;
        };
        let v = self.check_expr(cx, value, Some(&Ty::Bool));
        Some(hir::StmtKind::Emit {
            port: pi,
            signal: sig.def,
            args: hargs,
            value: v,
        })
    }

    /// `port.transaction(args)` from the initiating role.
    pub(super) fn bus_call(
        &mut self,
        cx: &mut FnCtx,
        port: u32,
        method: &ast::Ident,
        args: &[ast::Arg],
        span: Span,
    ) -> hir::Expr {
        let err = hir::Expr {
            kind: H::Error,
            ty: Ty::Error,
            span,
        };
        let Some(comp) = cx.component else { return err };
        let Some(cs) = self.comps.get(&comp).cloned() else {
            return err;
        };
        let PortKind::Bus { bus, role } = cs.ports[port as usize].kind else {
            return err;
        };
        let Some(info) = self.buses.get(&bus).cloned() else {
            return err;
        };
        let Some(t) = info.transactions.iter().find(|t| t.name == method.name) else {
            if info.signals.iter().any(|s| s.name == method.name) {
                self.error(
                    cx.module,
                    "E1044",
                    method.span,
                    format!(
                        "`{}` is a signal; set it with `emit {}.{}(...) = level;`",
                        method.name, cs.ports[port as usize].name, method.name
                    ),
                );
            } else {
                self.error(
                    cx.module,
                    "E1041",
                    method.span,
                    format!("bus `{}` has no transaction `{}`", info.name, method.name),
                );
            }
            return err;
        };
        if cx.responding {
            self.error_with(
                cx.module,
                Diagnostic::error(
                    "E1048",
                    span,
                    "a transaction handler cannot start a transaction",
                )
                .with_note("it runs inside the initiator's transaction; it may `emit` signals, and a device that masters a bus does so from `on tick`"),
            );
            return err;
        }
        if role != 0 {
            let msg = format!(
                "only the initiating role `{}` calls `{}`; this port is a `{}` and handles it with `on {}.{}(...)`",
                info.roles[0], t.name, info.roles[role as usize], cs.ports[port as usize].name, t.name
            );
            self.error(cx.module, "E1041", span, msg);
            return err;
        }
        let Some(ordered) = self.order_args(
            cx,
            &format!("transaction `{}`", t.name),
            &t.params,
            args,
            span,
        ) else {
            return err;
        };
        let mut hargs = Vec::new();
        for (a, (_, pty)) in ordered.iter().zip(&t.params) {
            hargs.push(self.check_expr(cx, &a.value, Some(pty)));
        }
        let ty = t.ret.clone().unwrap_or(Ty::Unit);
        hir::Expr {
            kind: H::BusCall(port, t.def, hargs),
            ty,
            span,
        }
    }
}
