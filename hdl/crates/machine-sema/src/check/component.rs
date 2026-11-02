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

/// A model fn's `(name, parameters, return type)`.
pub type ModelFnSig = (String, Vec<(String, Ty)>, Ty);

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
    /// Model fns: `(name, parameters, return type)`, by declaration order.
    pub fns: Vec<ModelFnSig>,
    pub fn_names: HashMap<String, u32>,
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
        let mut fns: Vec<ModelFnSig> = Vec::new();
        let mut fn_names: HashMap<String, u32> = HashMap::new();
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
                    S::Fn(d) => {
                        let mut ps = Vec::new();
                        for p in &d.params {
                            let t = self.resolve_type(scope, &empty, &p.ty);
                            ok &= !t.is_error();
                            ps.push((p.name.name.clone(), t));
                        }
                        let ret = match &d.ret {
                            Some(r) => self.resolve_type(scope, &empty, r),
                            None => Ty::Unit,
                        };
                        if names.contains_key(&d.name.name) || fn_names.contains_key(&d.name.name) {
                            self.error(
                                module,
                                "E1001",
                                d.name.span,
                                format!(
                                    "`{}` is already a port, state, mem or fn of `{name}`",
                                    d.name.name
                                ),
                            );
                            ok = false;
                        } else {
                            fn_names.insert(d.name.name.clone(), fns.len() as u32);
                        }
                        fns.push((d.name.name.clone(), ps, ret));
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
                fns: fns.clone(),
                fn_names,
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
            let mut hfns = Vec::new();
            for s in &body.stmts {
                match &s.kind {
                    S::State { .. } | S::Mem { .. } | S::Error => {}
                    S::Fn(d) => {
                        let idx = hfns.len();
                        hfns.push(self.check_model_fn(comp, scope, idx, d));
                    }
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
                            Diagnostic::error("E1043", s.span, "a `model` body contains `state`, `mem`, `fn` and `on` handlers")
                                .with_note("put logic inside a handler: `on reset { ... }`, `on tick(n: u32) { ... }`, `on port.txn(...) -> (...) { ... }`"),
                        );
                        ok = false;
                    }
                }
            }
            if let Some(isa) = isa {
                ok &= self.check_space_bindings(module, isa, &handlers, decl.name.span);
            }
            ok &= self.check_model_fn_graph(module, &mut hfns, &handlers);
            model = Some(hir::ModelBody {
                init_locals: init_cx.locals,
                states: hstates,
                mems: hmems,
                handlers,
                fns: hfns,
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
                    // `-> u32`: the most cycles that may pass before the
                    // device must run again (tick scheduling).
                    let want_ret = match &decl_ret {
                        None => Ty::Unit,
                        Some(Ty::UInt(32)) | Some(Ty::Error) => Ty::UInt(32),
                        Some(_) => {
                            self.error_with(
                                module,
                                Diagnostic::error("E1041", span, "`on tick` returns nothing, or `u32`: the most cycles before it must run again")
                                    .with_note("write `on tick(n: u32) -> u32 { ...; return cycles_until_next_event; }`"),
                            );
                            return None;
                        }
                    };
                    let pname = decl_params
                        .first()
                        .map(|p| p.0.clone())
                        .unwrap_or_else(|| "n".into());
                    (
                        HandlerKind::Tick { clock: clock_port },
                        Some(vec![(pname, Ty::UInt(32))]),
                        want_ret,
                    )
                }
                [single]
                    if matches!(
                        cs.names.get(&single.name),
                        Some(CompEntry::Port(i))
                            if matches!(cs.ports[*i as usize].kind, PortKind::In(_) | PortKind::AsyncIn(_))
                    ) =>
                {
                    let Some(CompEntry::Port(pi)) = cs.names.get(&single.name).copied() else {
                        unreachable!()
                    };
                    let (PortKind::In(ty) | PortKind::AsyncIn(ty)) =
                        cs.ports[pi as usize].kind.clone()
                    else {
                        unreachable!()
                    };
                    if clock.is_some() || ret.is_some() {
                        self.error(module, "E1041", span, "an input handler is `on <in port>(value: T) { ... }`: no clock, no return type");
                        return None;
                    }
                    (
                        HandlerKind::Input { port: pi },
                        Some(vec![("value".into(), ty)]),
                        Ty::Unit,
                    )
                }
                [single] if self.isa_extern(cs.isa, &single.name).is_some() => {
                    let f = self.isa_extern(cs.isa, &single.name).unwrap();
                    if clock.is_some() {
                        self.error(
                            module,
                            "E1041",
                            span,
                            "an `extern fn` handler takes no `@ clock`",
                        );
                        return None;
                    }
                    let sig = self.fn_sig(f)?;
                    (
                        HandlerKind::Extern { def: f },
                        Some(sig.params.clone()),
                        sig.ret.clone(),
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

    /// The body of model fn `idx`: the component's state, ports and effects
    /// are in scope, as in a handler.
    fn check_model_fn(
        &mut self,
        comp: DefId,
        scope: Scope,
        idx: usize,
        d: &ast::FnDecl,
    ) -> hir::ModelFn {
        let cs = self.comps.get(&comp).cloned().expect("component scope");
        let (_, params, ret) = cs.fns[idx].clone();
        let mut cx = FnCtx::new(scope, HashMap::new(), Some(ret.clone()));
        cx.component = Some(comp);
        let mut ids = Vec::new();
        for (p, (n, t)) in d.params.iter().zip(&params) {
            if cx.lookup_local(n).is_some() {
                self.error(
                    scope.module,
                    "E1001",
                    p.name.span,
                    format!("duplicate parameter `{n}`"),
                );
            }
            ids.push(cx.declare(n, t.clone(), p.name.span));
        }
        let body = self.check_block(&mut cx, &d.body);
        if !matches!(ret, Ty::Unit | Ty::Error) && !super::stmt::block_diverges(&body) {
            let msg = format!("fn `{}` does not return a value on every path", d.name.name);
            self.error(scope.module, "E1015", d.name.span, msg);
        }
        hir::ModelFn {
            name: d.name.name.clone(),
            locals: cx.locals,
            params: ids,
            ret,
            body,
            span: d.name.span,
            starts_transaction: false,
        }
    }

    /// The model fn call graph: no recursion (E1053), which fns start a
    /// transaction (directly or through a call), no such call from a
    /// transaction handler (E1048), and fns that could be units (W1005).
    fn check_model_fn_graph(
        &mut self,
        module: crate::program::ModuleId,
        fns: &mut [hir::ModelFn],
        handlers: &[hir::Handler],
    ) -> bool {
        let mut ok = true;
        // Per fn: the fns it calls, whether it starts a transaction itself,
        // and whether it touches state or has an effect itself.
        let mut calls: Vec<Vec<u32>> = Vec::new();
        let mut direct_tx = Vec::new();
        let mut direct_touch = Vec::new();
        for f in fns.iter() {
            let (mut c, mut tx, mut touch, mut emits) = (Vec::new(), false, false, false);
            hir::visit_block(
                &f.body,
                &mut |s| {
                    if matches!(s.kind, hir::StmtKind::Emit { .. }) {
                        emits = true;
                    }
                },
                &mut |e| match &e.kind {
                    H::ModelFnCall(i, _) => c.push(*i),
                    H::BusCall(..) => {
                        tx = true;
                        touch = true;
                    }
                    H::State(..) | H::ModelMem(..) | H::Port(..) | H::SignalLevel(..) => {
                        touch = true
                    }
                    _ => {}
                },
            );
            calls.push(c);
            direct_tx.push(tx);
            direct_touch.push(touch || emits);
        }

        // Recursion: a depth-first search for a cycle through each fn.
        let n = fns.len();
        let mut reported = vec![false; n];
        for start in 0..n {
            let mut stack = vec![(start, 0usize)];
            let mut on_path = vec![false; n];
            on_path[start] = true;
            while let Some((f, next)) = stack.pop() {
                if next < calls[f].len() {
                    stack.push((f, next + 1));
                    let g = calls[f][next] as usize;
                    if g == start {
                        if !reported[start] {
                            reported[start] = true;
                            let msg = format!(
                                "fn `{}` calls itself (directly or through other fns)",
                                fns[start].name
                            );
                            self.error_with(
                                module,
                                Diagnostic::error("E1053", fns[start].span, msg)
                                    .with_note("recursion is not allowed; every call graph in the language is finite"),
                            );
                            ok = false;
                        }
                    } else if !on_path[g] {
                        on_path[g] = true;
                        stack.push((g, 0));
                    }
                } else {
                    on_path[f] = false;
                }
            }
        }

        // Transitive effects, to a fixed point (the graph may have cycles
        // that were reported above).
        let mut tx = direct_tx;
        let mut touch = direct_touch;
        loop {
            let mut changed = false;
            for f in 0..n {
                for &g in &calls[f] {
                    let g = g as usize;
                    if tx[g] && !tx[f] {
                        tx[f] = true;
                        changed = true;
                    }
                    if touch[g] && !touch[f] {
                        touch[f] = true;
                        changed = true;
                    }
                }
            }
            if !changed {
                break;
            }
        }
        for (f, t) in fns.iter_mut().zip(&tx) {
            f.starts_transaction = *t;
        }

        // A transaction handler may not reach a transaction through a fn.
        for h in handlers {
            if !matches!(h.kind, HandlerKind::Transaction { .. }) {
                continue;
            }
            let mut bad: Vec<(u32, Span)> = Vec::new();
            hir::visit_block(&h.body, &mut |_| {}, &mut |e| {
                if let H::ModelFnCall(i, _) = &e.kind {
                    if tx[*i as usize] {
                        bad.push((*i, e.span));
                    }
                }
            });
            for (i, span) in bad {
                let msg = format!(
                    "this calls `{}`, which starts a transaction",
                    fns[i as usize].name
                );
                self.error_with(
                    module,
                    Diagnostic::error("E1048", span, msg)
                        .with_note("a transaction handler runs inside the initiator's transaction; it may `emit` signals, and a device that masters a bus does so from `on tick`"),
                );
                ok = false;
            }
        }

        // Fns that could be units.
        for (f, t) in fns.iter().zip(&touch) {
            if !*t {
                let msg = format!(
                    "fn `{}` touches no state and has no effect; it could be a `unit`",
                    f.name
                );
                self.error_with(
                    module,
                    Diagnostic::warning("W1005", f.span, msg)
                        .with_note("a unit is shared with the rtl level; a model fn is not"),
                );
            }
        }
        ok
    }

    /// An `extern fn` of the component's isa, by name.
    fn isa_extern(&self, isa: Option<DefId>, name: &str) -> Option<DefId> {
        let isa = isa?;
        self.prog
            .isa_items_of(isa, ItemKind::Fn)
            .find(|f| self.prog.item_name(*f) == name && self.prog.fn_decl(*f).external)
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
        for f in &info.fns {
            if f.external
                && !handlers
                    .iter()
                    .any(|h| h.kind == HandlerKind::Extern { def: f.def })
            {
                let msg = format!(
                    "the model does not supply the isa's `extern fn {}`: add `on {}(...) {{ ... }}`",
                    f.name, f.name
                );
                self.error(module, "E1041", span, msg);
                ok = false;
            }
        }
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
            if let Some(sig) = info.signals.iter().find(|s| s.name == method.name) {
                // Read as a value: the signal's current level.
                let Some(ordered) = self.order_args(
                    cx,
                    &format!("signal `{}`", sig.name),
                    &sig.params,
                    args,
                    span,
                ) else {
                    return err;
                };
                let mut hargs = Vec::new();
                for (a, (_, pty)) in ordered.iter().zip(&sig.params) {
                    hargs.push(self.check_expr(cx, &a.value, Some(pty)));
                }
                return hir::Expr {
                    kind: H::SignalLevel(port, sig.def, hargs),
                    ty: Ty::Bool,
                    span,
                };
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
