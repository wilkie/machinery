//! The model backend for a `system`: one Rust struct per component model,
//! a ports trait per component (the transactions it starts and the signals
//! it drives), and the system that links them.
//!
//! Shape of the generated system, for one processor:
//!
//! - `Board` owns every component model (the processor's too), the signal
//!   levels and the time. It implements the interpreter's `Bus`, so the
//!   processor is `Isa<Board>` and every isa space access runs the
//!   processor model's space handlers, whose bus calls decode the address
//!   and run the responding device's handler.
//! - Time is an integer count of the system's base unit. A transaction
//!   charges its bus's clock period times (base cycles + the responder's
//!   `wait`).
//! - Devices catch up lazily: before a transaction reaches a device, and
//!   after every instruction, its `on tick` runs with the cycles of its
//!   clock that elapsed since the last call.
//! - `emit` sets a signal level at once and queues an edge; edges are
//!   delivered to the observing ports after each instruction.
//!
//! Only the processor starts transactions in this backend: a device's
//! handlers may `emit`, and a transaction a device starts finds nothing
//! attached (open bus).

use std::fmt::Write as _;

use machine_sema::hir::{self, HandlerKind, PortKind, SystemInfo, WireDriver};
use machine_sema::program::DefId;
use machine_sema::ty::Ty;

use super::{ident, mask_literal, native, Cx, Gen, GenError, R};

pub(super) fn comp_info(hir: &hir::Hir, def: DefId) -> R<&hir::ComponentInfo> {
    hir.components
        .iter()
        .find(|c| c.def == def)
        .ok_or_else(|| GenError::new("component was not checked (errors earlier)"))
}

/// The index of the system's one processor instance.
pub(super) fn processor(hir: &hir::Hir, sys: &SystemInfo) -> R<usize> {
    let mut found = Vec::new();
    for (i, c) in sys.components.iter().enumerate() {
        if comp_info(hir, c.component)?.isa.is_some() {
            found.push(i);
        }
    }
    match found.as_slice() {
        [one] => Ok(*one),
        [] => Err(GenError::new(format!(
            "system `{}` has no processor; the model backend runs a system around one processor",
            sys.name
        ))),
        _ => Err(GenError::new(format!(
            "system `{}` has several processors; the model backend supports one",
            sys.name
        ))),
    }
}

fn method_tick(comp: &hir::ComponentInfo, clock: Option<u32>) -> String {
    match clock {
        None => "on_tick".into(),
        Some(p) => format!("on_tick_{}", ident(&comp.ports[p as usize].name)),
    }
}

/// A signal's level array: one `bool` per value of its (single, small)
/// parameter.
fn signal_lines(g: &Gen, s: &hir::SignalInfo) -> R<(usize, Option<String>)> {
    match s.params.as_slice() {
        [] => Ok((1, None)),
        [(_, ty)] => {
            let bits = match ty {
                Ty::UInt(b) => *b,
                Ty::Enum(d) => g.enums[d].width,
                _ => 99,
            };
            if bits > 8 {
                return Err(GenError::new(format!(
                    "signal `{}`: the model backend supports signals with no parameter or one of at most 8 bits",
                    s.name
                )));
            }
            Ok((1usize << bits, Some(g.rty(ty)?)))
        }
        _ => Err(GenError::new(format!(
            "signal `{}`: the model backend supports signals with at most one parameter",
            s.name
        ))),
    }
}

struct Inst<'h> {
    name: String,
    comp: &'h hir::ComponentInfo,
    model: &'h hir::ModelBody,
    sys: &'h hir::SystemComponent,
}

impl<'a> Gen<'a> {
    fn default_value(&self, ty: &Ty) -> R<String> {
        Ok(match ty {
            Ty::Bool => "false".into(),
            Ty::Bundle(_) => format!("{}::default()", self.rty(ty)?),
            Ty::Array(t, n) => format!("[{}; {n}]", self.default_value(t)?),
            Ty::Tuple(items) => {
                let parts: R<Vec<String>> =
                    items.iter().map(|(_, t)| self.default_value(t)).collect();
                format!("({},)", parts?.join(", "))
            }
            _ => "0".into(),
        })
    }

    /// The value an unanswered read returns: all ones.
    fn open_value(&self, ty: &Ty) -> R<String> {
        Ok(match ty {
            Ty::UInt(b) => mask_literal(*b, native(*b, false).1),
            Ty::Enum(d) => {
                let w = self.enums[d].width;
                mask_literal(w, native(w, false).1)
            }
            Ty::Bool => "true".into(),
            other => self.default_value(other)?,
        })
    }

    fn bus(&self, def: DefId) -> R<&'a hir::BusInfo> {
        self.hir
            .buses
            .iter()
            .find(|b| b.def == def)
            .ok_or_else(|| GenError::new("bus was not checked (errors earlier)"))
    }

    /// The parameter list of a transaction or signal, as Rust.
    fn params_text(&self, ps: &[(String, Ty)]) -> R<String> {
        let v: R<Vec<String>> = ps
            .iter()
            .map(|(n, t)| Ok(format!("{}: {}", ident(n), self.rty(t)?)))
            .collect();
        Ok(v?.join(", "))
    }

    /// The signals a component may drive through a bus port.
    fn drivable(info: &'a hir::BusInfo, role: u32) -> impl Iterator<Item = &'a hir::SignalInfo> {
        info.signals
            .iter()
            .filter(move |s| !matches!(s.driver, WireDriver::Role(r) if r != role))
    }

    /// The ports trait and the model struct of one component.
    fn emit_component(&mut self, comp: &hir::ComponentInfo) -> R<()> {
        let name = ident(&comp.name);
        let model = comp.model.as_ref().unwrap();

        // The ports trait.
        let mut t = format!(
            "/// What a `{name}` model reaches through its ports. Every method has a\n/// default (nothing attached: open bus, low levels, emits go nowhere), so a\n/// hand-written context overrides only what it observes.\npub trait {name}Ports {{\n"
        );
        for p in &comp.ports {
            let PortKind::Bus { bus, role } = p.kind else {
                continue;
            };
            let info = self.bus(bus)?;
            if role == 0 {
                for tr in &info.transactions {
                    let (ret, open) = match &tr.ret {
                        Some(r @ Ty::Tuple(items)) => {
                            let vals: R<Vec<String>> =
                                items.iter().map(|(_, t)| self.open_value(t)).collect();
                            (self.rty(r)?, format!("({},)", vals?.join(", ")))
                        }
                        _ => ("()".into(), "()".into()),
                    };
                    let _ = writeln!(
                        t,
                        "    #[allow(unused_variables)]\n    fn {}_{}(&mut self, {}) -> {ret} {{ {open} }}",
                        ident(&p.name),
                        ident(&tr.name),
                        self.params_text(&tr.params)?
                    );
                }
            }
            for s in &info.signals {
                let _ = writeln!(
                    t,
                    "    /// The current level of signal `{}`.\n    #[allow(unused_variables)]\n    fn level_{}_{}(&mut self, {}) -> bool {{ false }}",
                    s.name,
                    ident(&p.name),
                    ident(&s.name),
                    self.params_text(&s.params)?
                );
            }
            for s in Self::drivable(info, role) {
                let mut ps = self.params_text(&s.params)?;
                if !ps.is_empty() {
                    ps.push_str(", ");
                }
                let _ = writeln!(
                    t,
                    "    #[allow(unused_variables)]\n    fn emit_{}_{}(&mut self, {ps}value: bool) {{}}",
                    ident(&p.name),
                    ident(&s.name)
                );
            }
        }
        t.push_str("}\n");
        self.out.push_str(&t);

        // The struct.
        let mut st = format!("#[derive(Clone, Debug)]\npub struct {name} {{\n");
        for s in &model.states {
            let _ = writeln!(st, "    pub {}: {},", ident(&s.name), self.rty(&s.ty)?);
        }
        for m in &model.mems {
            let _ = writeln!(
                st,
                "    pub {}: Vec<{}>,",
                ident(&m.name),
                self.rty(&m.elem)?
            );
        }
        st.push_str("}\n");
        let _ = write!(
            st,
            "impl {name} {{\n    pub fn new() -> {name} {{\n        let mut s = {name} {{\n"
        );
        for s in &model.states {
            let _ = writeln!(
                st,
                "            {}: {},",
                ident(&s.name),
                self.default_value(&s.ty)?
            );
        }
        for m in &model.mems {
            let _ = writeln!(
                st,
                "            {}: vec![{}; {}],",
                ident(&m.name),
                self.default_value(&m.elem)?,
                m.len
            );
        }
        st.push_str("        };\n");
        let cx = Cx {
            locals: &model.init_locals,
            fallible: false,
            recv: "s",
            ops: None,
            comp: Some(comp),
            opt: false,
        };
        for s in &model.states {
            if let Some(init) = &s.init {
                let _ = writeln!(
                    st,
                    "        s.{} = {};",
                    ident(&s.name),
                    self.expr(&cx, init)?
                );
            }
        }
        st.push_str("        s\n    }\n");

        for f in &model.fns {
            let cx = Cx {
                locals: &f.locals,
                fallible: false,
                recv: "self",
                ops: None,
                comp: Some(comp),
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
            let body = self.block_stmts(&cx, &f.body)?;
            let (ret, tail) = match f.ret {
                Ty::Unit => (String::new(), ""),
                ref t => (format!(" -> {}", self.rty(t)?), "        #[allow(unreachable_code)]\n        { unreachable!(\"fn fell off its end\") }\n"),
            };
            let _ = write!(
                st,
                "    /// `fn {}` of the model.\n    pub fn fn_{}<P: {name}Ports>(&mut self, ctx: &mut P{ps}){ret} {{\n{body}{tail}    }}\n",
                f.name,
                ident(&f.name)
            );
        }
        for h in &model.handlers {
            // A transaction handler returns `None` when it declines.
            let opt = matches!(h.kind, HandlerKind::Transaction { .. });
            let cx = Cx {
                locals: &h.locals,
                fallible: false,
                recv: "self",
                ops: None,
                comp: Some(comp),
                opt,
            };
            let mname = match &h.kind {
                HandlerKind::Reset => "on_reset".to_string(),
                HandlerKind::Tick { clock } => method_tick(comp, *clock),
                HandlerKind::Transaction { port, transaction } => format!(
                    "on_{}_{}",
                    ident(&comp.ports[*port as usize].name),
                    ident(&self.prog.item_name(*transaction))
                ),
                HandlerKind::Signal { port, signal } => format!(
                    "on_{}_{}",
                    ident(&comp.ports[*port as usize].name),
                    ident(&self.prog.item_name(*signal))
                ),
                HandlerKind::SpaceRead { space, bits } => {
                    format!("{}_read_u{bits}", ident(&self.spaces[space].name))
                }
                HandlerKind::SpaceWrite { space, bits } => {
                    format!("{}_write_u{bits}", ident(&self.spaces[space].name))
                }
                HandlerKind::Extern { def } => format!("ext_{}", ident(&self.prog.item_name(*def))),
            };
            let mut ps = String::new();
            for p in &h.params {
                let _ = write!(
                    ps,
                    ", {}: {}",
                    self.local_name(&cx, *p),
                    self.rty(&h.locals[*p as usize].ty)?
                );
            }
            let body = self.block_stmts(&cx, &h.body)?;
            let (ret, tail) = match h.ret {
                Ty::Unit => (String::new(), ""),
                ref t if opt => (format!(" -> Option<{}>", self.rty(t)?), "        #[allow(unreachable_code)]\n        { unreachable!(\"handler fell off its end\") }\n"),
                ref t => (format!(" -> {}", self.rty(t)?), "        #[allow(unreachable_code)]\n        { unreachable!(\"handler fell off its end\") }\n"),
            };
            let _ = write!(
                st,
                "    pub fn {mname}<P: {name}Ports>(&mut self, ctx: &mut P{ps}){ret} {{\n{body}{tail}    }}\n"
            );
        }
        st.push_str("}\n");
        self.out.push_str(&st);
        Ok(())
    }

    pub(super) fn emit_system(
        &mut self,
        sys: &'a SystemInfo,
        proc_idx: usize,
        profile: Option<&str>,
    ) -> R<()> {
        let hir = self.hir;
        // What this backend generates.
        if sys.wires > 0 {
            return Err(GenError::new(format!(
                "system `{}`: `wire` connections are not generated yet",
                sys.name
            )));
        }
        let prof = match profile {
            None => None,
            Some(p) => Some(sys.profiles.iter().find(|q| q.name == p).ok_or_else(|| {
                let names: Vec<&str> = sys.profiles.iter().map(|q| q.name.as_str()).collect();
                GenError::new(format!(
                    "system `{}` has no profile `{p}` (profiles: {})",
                    sys.name,
                    names.join(", ")
                ))
            })?),
        };
        let mut insts: Vec<Inst> = Vec::new();
        for c in &sys.components {
            let comp = comp_info(hir, c.component)?;
            if let Some(p) = prof {
                if let Some((path, _)) = p.overrides.iter().find(|(path, _)| {
                    path.contains('.') && path.starts_with(&format!("{}.", c.name))
                }) {
                    return Err(GenError::new(format!(
                        "profile `{}`: levels for sub-components (`{path}`) are not generated yet",
                        p.name
                    )));
                }
                let level = p
                    .overrides
                    .iter()
                    .find(|(path, _)| *path == c.name)
                    .map(|(_, l)| *l)
                    .unwrap_or(p.default);
                if level != hir::Level::Model {
                    return Err(GenError::new(format!(
                        "profile `{}` runs `{}` at the rtl level; the model backend generates model bodies only",
                        p.name, c.name
                    )));
                }
            }
            let Some(model) = comp.model.as_ref() else {
                return Err(GenError::new(format!(
                    "`{}` ({}) has no model body; running rtl in the model backend is not supported yet",
                    c.name, comp.name
                )));
            };
            insts.push(Inst {
                name: c.name.clone(),
                comp,
                model,
                sys: c,
            });
        }
        for b in &sys.buses {
            if !b.bridges.is_empty() {
                return Err(GenError::new(format!(
                    "bus `{}`: bridges are not generated yet",
                    b.name
                )));
            }
            if b.slaves.iter().filter(|(_, r)| r.is_empty()).count() > 1 {
                return Err(GenError::new(format!(
                    "bus `{}` has several slaves without address ranges",
                    b.name
                )));
            }
        }
        let cpu = &insts[proc_idx];
        let cpu_name = ident(&cpu.name);
        let cpu_clock = match cpu.sys.clocks.first() {
            Some((_, c)) => *c,
            None => {
                return Err(GenError::new(format!(
                    "the processor `{}` needs a clock (`component {}: {} @ <clock>;`): a halted processor waits in cycles of it",
                    cpu.name, cpu.name, cpu.comp.name
                )))
            }
        };
        let isa_name = ident(&self.isa.name);

        // Component models, once per component.
        let mut done: Vec<DefId> = Vec::new();
        for i in &insts {
            if !done.contains(&i.comp.def) {
                done.push(i.comp.def);
                self.emit_component(i.comp)?;
            }
        }

        let mut o = String::new();
        let _ = write!(
            o,
            "\n/// The system `{}`.\npub mod system {{\n    use super::*;\n\n    /// The time unit is `BASE_NUM / BASE_DEN` seconds.\n    pub const BASE_NUM: u128 = {};\n    pub const BASE_DEN: u128 = {};\n    /// Clock periods in time units.\n    pub mod clock {{\n",
            sys.name, sys.base_num, sys.base_den
        );
        for (i, c) in sys.clocks.iter().enumerate() {
            let _ = writeln!(
                o,
                "        /// `{}`: {} / {} Hz\n        pub const C{i}: u64 = {};",
                c.name, c.hz_num, c.hz_den, c.period
            );
        }
        o.push_str("    }\n");

        // Signals: one level array per bus instance and signal.
        let mut sig_fields = String::new();
        let mut sig_init = String::new();
        let mut events = String::new();
        // (bus index, signal def) -> (field, len, param type)
        let mut sig_names: Vec<(usize, DefId, String, Option<String>)> = Vec::new();
        for (bi, b) in sys.buses.iter().enumerate() {
            let info = self.bus(b.bus)?;
            for s in &info.signals {
                let (n, pty) = signal_lines(self, s)?;
                let f = format!("{}_{}", ident(&b.name), ident(&s.name));
                let _ = writeln!(sig_fields, "        pub {f}: [bool; {n}],");
                let _ = writeln!(sig_init, "                {f}: [false; {n}],");
                let _ = writeln!(events, "        {f}(usize, bool),");
                sig_names.push((bi, s.def, f, pty));
            }
        }
        let _ = write!(
            o,
            "    #[allow(non_camel_case_types)]\n    #[derive(Clone, Copy, Debug, PartialEq)]\n    pub enum Event {{\n{events}    }}\n    /// Time and signal levels.\n    #[derive(Clone, Debug)]\n    pub struct Signals {{\n        /// Time in units of `BASE_NUM / BASE_DEN` seconds.\n        pub now: u64,\n{sig_fields}        /// Edges not yet delivered.\n        pub events: Vec<Event>,\n    }}\n    impl Signals {{\n        pub fn new() -> Signals {{\n            Signals {{\n                now: 0,\n{sig_init}                events: Vec::new(),\n            }}\n        }}\n    }}\n"
        );

        // Tick marks: the last time each device's clock was caught up to.
        let mut marks = String::new();
        // (inst, handler clock port, system clock)
        let mut ticks: Vec<(usize, Option<u32>, u32)> = Vec::new();
        for (ii, i) in insts.iter().enumerate() {
            for h in &i.model.handlers {
                let HandlerKind::Tick { clock } = h.kind else {
                    continue;
                };
                let bound = i
                    .sys
                    .clocks
                    .iter()
                    .find(|(p, _)| {
                        *p == clock || (clock.is_some() && p.is_none() && i.sys.clocks.len() == 1)
                    })
                    .map(|(_, c)| *c)
                    .ok_or_else(|| {
                        GenError::new(format!(
                            "`{}`: its tick clock is not bound in the system",
                            i.name
                        ))
                    })?;
                let _ = writeln!(
                    marks,
                    "        pub {}_{}: u64,",
                    ident(&i.name),
                    method_tick(i.comp, clock)
                );
                ticks.push((ii, clock, bound));
            }
        }
        let _ = write!(
            o,
            "    #[derive(Clone, Debug, Default)]\n    pub struct Marks {{\n{marks}    }}\n"
        );

        // Who masters a bus: the processor, and at most one device.
        let masters_any = |ii: usize| -> bool {
            sys.buses
                .iter()
                .any(|b| b.masters.iter().any(|(i, _)| *i as usize == ii))
        };
        let dev_masters: Vec<usize> = (0..insts.len())
            .filter(|&ii| ii != proc_idx && masters_any(ii))
            .collect();
        if dev_masters.len() > 1 {
            let names: Vec<&str> = dev_masters
                .iter()
                .map(|&i| insts[i].name.as_str())
                .collect();
            return Err(GenError::new(format!(
                "several devices master buses ({}); the model backend supports one besides the processor",
                names.join(", ")
            )));
        }
        if ticks.iter().any(|t| t.0 == proc_idx) {
            return Err(GenError::new(
                "an `on tick` handler on the processor model is not generated yet",
            ));
        }
        // A master's context holds every other device, so its bus calls can
        // reach them; any other device's context holds only the signals.
        let full = |ii: usize| ii == proc_idx || dev_masters.contains(&ii);
        let field = |src: &str, deref: bool, f: &str| -> String {
            if deref {
                format!("&mut *{src}.{f}")
            } else {
                format!("&mut {src}.{f}")
            }
        };
        let ctx_for = |ii: usize, src: &str, deref: bool| -> String {
            let n = ident(&insts[ii].name);
            if full(ii) {
                let mut s = format!(
                    "Ctx_{n} {{ sig: {}, marks: {}",
                    field(src, deref, "sig"),
                    field(src, deref, "marks")
                );
                for (jj, j) in insts.iter().enumerate() {
                    if jj != proc_idx && jj != ii {
                        let jn = ident(&j.name);
                        let _ = write!(s, ", {jn}: {}", field(src, deref, &jn));
                    }
                }
                s.push_str(" }");
                s
            } else {
                format!("Ctx_{n} {{ sig: {} }}", field(src, deref, "sig"))
            }
        };
        let ticking = |ii: usize| ticks.iter().any(|t| t.0 == ii);
        // Statements that catch device `ii` up, from a source holding it.
        let catch_from = |ii: usize, src: &str, deref: bool| -> String {
            if !ticking(ii) {
                return String::new();
            }
            let n = ident(&insts[ii].name);
            if full(ii) {
                format!(
                    "{{ let mut c = {}; catch_up_{n}({}, &mut c); }} ",
                    ctx_for(ii, src, deref),
                    field(src, deref, &n)
                )
            } else {
                format!(
                    "catch_up_{n}({}, {}, {}); ",
                    field(src, deref, &n),
                    field(src, deref, "sig"),
                    field(src, deref, "marks")
                )
            }
        };
        // The bus a port is attached to.
        let attached = |ii: usize, pi: usize| -> Option<usize> {
            let ip = (ii as u32, pi as u32);
            sys.buses
                .iter()
                .position(|b| b.masters.contains(&ip) || b.slaves.iter().any(|(s, _)| *s == ip))
        };

        // Signal methods for an instance: emit through its ports, and read
        // the levels of the signals on its buses.
        let signal_methods = |g: &Gen<'a>, ii: usize| -> R<String> {
            let i = &insts[ii];
            let mut s = String::new();
            for (pi, p) in i.comp.ports.iter().enumerate() {
                let PortKind::Bus { bus, role } = p.kind else {
                    continue;
                };
                let info = g.bus(bus)?;
                let at = attached(ii, pi);
                let line = |sg: &hir::SignalInfo| -> (String, String) {
                    let idx = match sg.params.first() {
                        Some((n, _)) => format!("{} as usize", ident(n)),
                        None => "0".into(),
                    };
                    let f = at
                        .and_then(|bi| {
                            sig_names
                                .iter()
                                .find(|(b, d, _, _)| *b == bi && *d == sg.def)
                                .map(|x| x.2.clone())
                        })
                        .unwrap_or_default();
                    (f, idx)
                };
                for sg in Self::drivable(info, role) {
                    let mut ps = g.params_text(&sg.params)?;
                    if !ps.is_empty() {
                        ps.push_str(", ");
                    }
                    let (f, idx) = line(sg);
                    let body = if at.is_none() {
                        String::new()
                    } else {
                        format!("let i = {idx}; if self.sig.{f}[i] != value {{ self.sig.{f}[i] = value; self.sig.events.push(Event::{f}(i, value)); }}")
                    };
                    let _ = writeln!(
                        s,
                        "        fn emit_{}_{}(&mut self, {ps}value: bool) {{ {body} }}",
                        ident(&p.name),
                        ident(&sg.name)
                    );
                }
                for sg in &info.signals {
                    let (f, idx) = line(sg);
                    let body = if at.is_none() {
                        "false".to_string()
                    } else {
                        format!("self.sig.{f}[{idx}]")
                    };
                    let _ = writeln!(
                        s,
                        "        fn level_{}_{}(&mut self, {}) -> bool {{ {body} }}",
                        ident(&p.name),
                        ident(&sg.name),
                        g.params_text(&sg.params)?
                    );
                }
            }
            Ok(s)
        };

        // One context per instance.
        for (ii, i) in insts.iter().enumerate() {
            let cname = ident(&i.comp.name);
            let n = ident(&i.name);
            let mut fields = String::from("        pub sig: &'a mut Signals,\n");
            if full(ii) {
                fields.push_str("        pub marks: &'a mut Marks,\n");
                for (jj, j) in insts.iter().enumerate() {
                    if jj != proc_idx && jj != ii {
                        let _ = writeln!(
                            fields,
                            "        pub {}: &'a mut {},",
                            ident(&j.name),
                            ident(&j.comp.name)
                        );
                    }
                }
            }
            let mut m = String::new();
            for (pi, p) in i.comp.ports.iter().enumerate() {
                let PortKind::Bus { bus, role: 0 } = p.kind else {
                    continue;
                };
                let info = self.bus(bus)?;
                let inst_bus = sys
                    .buses
                    .iter()
                    .find(|b| b.masters.contains(&(ii as u32, pi as u32)));
                for tr in &info.transactions {
                    let ret_items: Vec<(Option<String>, Ty)> = match &tr.ret {
                        Some(Ty::Tuple(items)) => items.clone(),
                        _ => Vec::new(),
                    };
                    let ret = match &tr.ret {
                        Some(r) => self.rty(r)?,
                        None => "()".into(),
                    };
                    let open: R<Vec<String>> =
                        ret_items.iter().map(|(_, t)| self.open_value(t)).collect();
                    let open = if ret_items.is_empty() {
                        "()".to_string()
                    } else {
                        format!("({},)", open?.join(", "))
                    };
                    let mut body = String::new();
                    if let (Some(b), true) = (inst_bus, full(ii)) {
                        let base = tr.base_cycles.ok_or_else(|| {
                            GenError::new(format!(
                                "transaction `{}.{}`: the base cycle count cannot be derived (the initiating role's body is missing or not straight-line)",
                                info.name, tr.name
                            ))
                        })?;
                        let period = format!("clock::C{}", b.clock);
                        let addr = tr
                            .params
                            .iter()
                            .find(|(n, _)| n == "addr")
                            .map(|(n, _)| ident(n));
                        let space = tr
                            .params
                            .iter()
                            .find(|(n, _)| n == "space")
                            .map(|(n, _)| ident(n));
                        let args: Vec<String> = tr.params.iter().map(|(n, _)| ident(n)).collect();
                        let n_ret = ret_items.len();
                        let pass: Vec<String> = (0..n_ret).map(|k| format!("r.{k}")).collect();
                        let result = if n_ret == 0 {
                            "()".to_string()
                        } else {
                            format!("({},)", pass.join(", "))
                        };
                        // Ranged slaves first, then the one without ranges;
                        // never the master's own ports.
                        let mut order: Vec<&(hir::InstPort, Vec<hir::AddressRange>)> =
                            b.slaves.iter().filter(|(_, r)| !r.is_empty()).collect();
                        order.extend(b.slaves.iter().filter(|(_, r)| r.is_empty()));
                        order.retain(|((si, _), _)| *si as usize != ii);
                        for ((si, sp), ranges) in order {
                            let si = *si as usize;
                            let dev = &insts[si];
                            let dn = ident(&dev.name);
                            let handler = dev.model.handlers.iter().any(|h| matches!(h.kind, HandlerKind::Transaction { port, transaction } if port == *sp && transaction == tr.def));
                            let catch = catch_from(si, "self", true);
                            let call = format!(
                                "{{ let mut c = {}; self.{dn}.on_{}_{}(&mut c, {}) }}",
                                ctx_for(si, "self", true),
                                ident(&dev.comp.ports[*sp as usize].name),
                                ident(&tr.name),
                                args.join(", ")
                            );
                            let Some(a) = addr.clone() else {
                                // Not decoded: offered to each responder that
                                // handles it, in binding order, until one
                                // does not decline.
                                if handler {
                                    let _ = writeln!(
                                        body,
                                        "        {{\n            {catch}if let Some(r) = {call} {{\n                self.sig.now += ({base} + r.{n_ret} as u64) * {period};\n                return {result};\n            }}\n        }}"
                                    );
                                }
                                continue;
                            };
                            let cond = if ranges.is_empty() {
                                "true".to_string()
                            } else {
                                let parts: Vec<String> = ranges
                                    .iter()
                                    .map(|r| {
                                        let sp = match (r.space, &space) {
                                            (Some(v), Some(s)) => format!("{s} as i128 == {v} && "),
                                            _ => String::new(),
                                        };
                                        format!("({sp}({a} as u128) >= {:#x} && ({a} as u128) <= {:#x})", r.lo, r.hi)
                                    })
                                    .collect();
                                parts.join(" || ")
                            };
                            if !handler {
                                // Selected, but the device does not answer this transaction.
                                let _ = writeln!(body, "        if {cond} {{ self.sig.now += {base} * {period}; return {open}; }}");
                                continue;
                            }
                            let _ = writeln!(
                                body,
                                "        if {cond} {{\n            {catch}match {call} {{\n                Some(r) => {{\n                    self.sig.now += ({base} + r.{n_ret} as u64) * {period};\n                    return {result};\n                }}\n                None => {{\n                    self.sig.now += {base} * {period};\n                    return {open};\n                }}\n            }}\n        }}"
                            );
                        }
                        let _ = writeln!(body, "        self.sig.now += {base} * {period};");
                    }
                    let _ = write!(
                        m,
                        "        fn {}_{}(&mut self, {}) -> {ret} {{\n{body}        {open}\n        }}\n",
                        ident(&p.name),
                        ident(&tr.name),
                        self.params_text(&tr.params)?
                    );
                }
            }
            m.push_str(&signal_methods(self, ii)?);
            let _ = write!(o, "    pub struct Ctx_{n}<'a> {{\n{fields}    }}\n    impl {cname}Ports for Ctx_{n}<'_> {{\n{m}    }}\n");
        }

        // Catch-up functions: run a device's clocks up to the present.
        for (ii, i) in insts.iter().enumerate() {
            let mine: Vec<&(usize, Option<u32>, u32)> =
                ticks.iter().filter(|t| t.0 == ii).collect();
            if mine.is_empty() {
                continue;
            }
            let n = ident(&i.name);
            let cname = ident(&i.comp.name);
            let mut body = String::new();
            for (_, clock, sc) in mine {
                let mt = method_tick(i.comp, *clock);
                if full(ii) {
                    let _ = write!(
                        body,
                        "        {{\n            let p = clock::C{sc};\n            let mut k = (c.sig.now - c.marks.{n}_{mt}) / p;\n            c.marks.{n}_{mt} += k * p;\n            while k > 0 {{\n                let s = k.min(u32::MAX as u64);\n                k -= s;\n                dev.{mt}(&mut *c, s as u32);\n            }}\n        }}\n"
                    );
                } else {
                    let _ = write!(
                        body,
                        "        {{\n            let p = clock::C{sc};\n            let mut k = (sig.now - marks.{n}_{mt}) / p;\n            marks.{n}_{mt} += k * p;\n            while k > 0 {{\n                let s = k.min(u32::MAX as u64);\n                k -= s;\n                let mut c = Ctx_{n} {{ sig: &mut *sig }};\n                dev.{mt}(&mut c, s as u32);\n            }}\n        }}\n"
                    );
                }
            }
            let sig = if full(ii) {
                format!("c: &mut Ctx_{n}<'_>")
            } else {
                "sig: &mut Signals, marks: &mut Marks".to_string()
            };
            let _ = write!(o, "    /// Run `{n}`'s clocks up to the present.\n    pub fn catch_up_{n}(dev: &mut {cname}, {sig}) {{\n{body}    }}\n");
        }
        let ctx_cpu = |b: &str| ctx_for(proc_idx, b, false);

        // The board.
        let mut fields = String::from("        pub sig: Signals,\n        pub marks: Marks,\n");
        let mut init = String::from(
            "            sig: Signals::new(),\n            marks: Marks::default(),\n",
        );
        for i in &insts {
            let _ = writeln!(
                fields,
                "        pub {}: {},",
                ident(&i.name),
                ident(&i.comp.name)
            );
            let _ = writeln!(
                init,
                "            {}: {}::new(),",
                ident(&i.name),
                ident(&i.comp.name)
            );
        }
        let mut bus_impl = String::new();
        for sp in &self.isa.spaces {
            let n = ident(&sp.name);
            let (abits, aty) = match &sp.addr {
                Ty::UInt(b) => (*b, native(*b, false).1),
                _ => return Err(GenError::new("space address type")),
            };
            let amask = mask_literal(abits, "u64");
            let cell = sp.cell_bits;
            for write in [false, true] {
                let mut arms = String::new();
                for h in &cpu.model.handlers {
                    match h.kind {
                        HandlerKind::SpaceRead { space, bits } if !write && space == sp.def => {
                            let _ = writeln!(arms, "            {bits} => self.{cpu_name}.{n}_read_u{bits}(&mut ctx, addr as {aty}) as u128,");
                        }
                        HandlerKind::SpaceWrite { space, bits } if write && space == sp.def => {
                            let (_, vty) = native(bits, false);
                            let _ = writeln!(arms, "            {bits} => self.{cpu_name}.{n}_write_u{bits}(&mut ctx, addr as {aty}, value as {vty}),");
                        }
                        _ => {}
                    }
                }
                let (_, cty) = native(cell, false);
                let shift = if sp.little_endian {
                    format!("i * {cell}")
                } else {
                    format!("(n - 1 - i) * {cell}")
                };
                if write {
                    let _ = write!(
                        bus_impl,
                        "    fn {n}_write(&mut self, addr: u64, bits: u32, value: u128) {{\n        let mut ctx = {};\n        let addr = addr & {amask};\n        match bits {{\n{arms}            _ => {{\n                let n = bits / {cell};\n                for i in 0..n {{\n                    let a = (addr + i as u64) & {amask};\n                    let v = (value >> ({shift})) as {cty};\n                    self.{cpu_name}.{n}_write_u{cell}(&mut ctx, a as {aty}, v);\n                }}\n            }}\n        }}\n    }}\n",
                        ctx_cpu("self")
                    );
                } else {
                    let _ = write!(
                        bus_impl,
                        "    fn {n}_read(&mut self, addr: u64, bits: u32) -> u128 {{\n        let mut ctx = {};\n        let addr = addr & {amask};\n        match bits {{\n{arms}            _ => {{\n                let n = bits / {cell};\n                let mut v: u128 = 0;\n                for i in 0..n {{\n                    let a = (addr + i as u64) & {amask};\n                    v |= (self.{cpu_name}.{n}_read_u{cell}(&mut ctx, a as {aty}) as u128) << ({shift});\n                }}\n                v\n            }}\n        }}\n    }}\n",
                        ctx_cpu("self")
                    );
                }
            }
        }
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
            let mut args = String::new();
            for p in &f.params {
                let n = self.local_name(&cx, *p);
                let _ = write!(ps, ", {n}: {}", self.rty(&f.locals[*p as usize].ty)?);
                let _ = write!(args, ", {n}");
            }
            let _ = write!(
                bus_impl,
                "    fn ext_{0}(&mut self{ps}) -> {1} {{\n        let mut ctx = {2};\n        self.{cpu_name}.ext_{0}(&mut ctx{args})\n    }}\n",
                ident(&f.name),
                self.rty(&f.ret)?,
                ctx_cpu("self")
            );
        }
        let _ = write!(
            o,
            "    /// Every component model, the time and the signal levels: the processor's bus.\n    #[derive(Clone, Debug)]\n    pub struct Board {{\n{fields}    }}\n    impl Board {{\n        pub fn new() -> Board {{\n            Board {{\n{init}            }}\n        }}\n    }}\n    impl Bus for Board {{\n{bus_impl}    }}\n"
        );

        // Reset, sync and signal delivery.
        let mut reset = String::new();
        for (ii, i) in insts.iter().enumerate() {
            if !i
                .model
                .handlers
                .iter()
                .any(|h| h.kind == HandlerKind::Reset)
            {
                continue;
            }
            let n = ident(&i.name);
            let _ = writeln!(
                reset,
                "        {{ let mut c = {}; b.{n}.on_reset(&mut c); }}",
                ctx_for(ii, "b", false)
            );
        }
        let mut sync = String::new();
        for ii in 0..insts.len() {
            let c = catch_from(ii, "b", false);
            if !c.is_empty() {
                let _ = writeln!(sync, "            {c}");
            }
        }
        // Host entry points: a model's fns with a live context.
        let mut with = String::new();
        for (ii, i) in insts.iter().enumerate() {
            let n = ident(&i.name);
            let cname = ident(&i.comp.name);
            let catch = catch_from(ii, "b", false);
            let ctx = ctx_for(ii, "b", false);
            let _ = write!(
                with,
                "        /// Call into `{n}` with a live context: how the embedding program\n        /// reaches its model fns (host input). The device is caught up first,\n        /// and signal edges it raises are delivered after.\n        pub fn with_{n}<R>(&mut self, f: impl FnOnce(&mut {cname}, &mut Ctx_{n}<'_>) -> R) -> R {{\n            let r = {{\n                let b = &mut self.{cpu_name}.bus;\n                {catch}\n                let mut c = {ctx};\n                f(&mut b.{n}, &mut c)\n            }};\n            self.deliver();\n            r\n        }}\n"
            );
        }
        let mut deliver = String::new();
        for (bi, sd, f, pty) in &sig_names {
            let b = &sys.buses[*bi];
            let mut calls = String::new();
            let ports: Vec<hir::InstPort> = b
                .masters
                .iter()
                .copied()
                .chain(b.slaves.iter().map(|(p, _)| *p))
                .collect();
            for (ii, pi) in ports {
                let i = &insts[ii as usize];
                let has = i.model.handlers.iter().any(|h| matches!(h.kind, HandlerKind::Signal { port, signal } if port == pi && signal == *sd));
                if !has {
                    continue;
                }
                let n = ident(&i.name);
                let arg = match pty {
                    Some(t) => format!("i as {t}, "),
                    None => String::new(),
                };
                let _ = writeln!(
                    calls,
                    "                    {{ let mut c = {}; b.{n}.on_{}_{}(&mut c, {arg}v); }}",
                    ctx_for(ii as usize, "b", false),
                    ident(&i.comp.ports[pi as usize].name),
                    ident(&self.prog.item_name(*sd))
                );
            }
            let _ = writeln!(
                deliver,
                "                Event::{f}(i, v) => {{\n{calls}                }}"
            );
        }
        let _ = write!(
            o,
            r#"    /// The system around its processor `{cpu_name}`.
    #[derive(Clone, Debug)]
    pub struct System {{
        pub {cpu_name}: {isa_name}<Board>,
    }}
    impl System {{
        /// A system after reset.
        pub fn new() -> System {{
            let mut s = System {{ {cpu_name}: {isa_name}::with_bus(Board::new()) }};
            s.reset();
            s
        }}
        pub fn board(&mut self) -> &mut Board {{
            &mut self.{cpu_name}.bus
        }}
        /// Time in units of `BASE_NUM / BASE_DEN` seconds.
        pub fn now(&self) -> u64 {{
            self.{cpu_name}.bus.sig.now
        }}
        pub fn seconds(&self) -> f64 {{
            self.now() as f64 * BASE_NUM as f64 / BASE_DEN as f64
        }}
        /// Every model's `on reset`, then the processor's.
        pub fn reset(&mut self) {{
            let b = &mut self.{cpu_name}.bus;
{reset}            self.{cpu_name}.reset();
            self.deliver();
        }}
        /// One instruction (or an interrupt taken at the boundary), then
        /// every device caught up to the present. A halted processor
        /// spends one cycle of its clock waiting.
        pub fn step(&mut self) -> Result<(), Fault> {{
            let was_halted = self.{cpu_name}.halted;
            let r = self.{cpu_name}.step();
            if was_halted && self.{cpu_name}.halted {{
                self.{cpu_name}.bus.sig.now += clock::C{cpu_clock};
            }}
            self.sync();
            r
        }}
        /// Run until the processor halts or `max` steps have run; returns
        /// how many ran.
        pub fn run(&mut self, max: u64) -> Result<u64, Fault> {{
            self.run_until(max, |s| s.{cpu_name}.halted)
        }}
        /// Run until `stop` holds (checked before each step) or `max` steps
        /// have run; returns how many ran.
        pub fn run_until(&mut self, max: u64, mut stop: impl FnMut(&mut System) -> bool) -> Result<u64, Fault> {{
            for n in 0..max {{
                if stop(self) {{
                    return Ok(n);
                }}
                self.step()?;
            }}
            Ok(max)
        }}
{with}        /// Catch every device up to the present and deliver signal edges.
        pub fn sync(&mut self) {{
            let b = &mut self.{cpu_name}.bus;
{sync}            self.deliver();
        }}
        fn deliver(&mut self) {{
            let b = &mut self.{cpu_name}.bus;
            let mut rounds = 0;
            while !b.sig.events.is_empty() {{
                rounds += 1;
                assert!(rounds < 1000, "signal edges did not settle");
                for e in std::mem::take(&mut b.sig.events) {{
                    match e {{
{deliver}                    }}
                }}
            }}
        }}
    }}
    impl Default for System {{
        fn default() -> System {{
            System::new()
        }}
    }}
}}
"#
        );
        self.out.push_str(&o);
        Ok(())
    }
}
