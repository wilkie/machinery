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
//! Devices that master buses (a DMA controller, a bridge such as the AT's
//! bus controller) get a context holding the devices they can reach, so
//! their transactions run the responders' handlers too; a transaction on
//! a port that is not attached finds nothing (open bus).

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
    pub(super) fn default_value(&self, ty: &Ty) -> R<String> {
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
        // `memo fn`s (SPEC 8.3b): number the state and memories they read.
        let memo = model.memo.as_ref();
        *self.memo_cur.borrow_mut() = memo.map(|m| crate::gen::MemoCur {
            states: m.states.iter().enumerate().map(|(k, i)| (*i, k as u32)).collect(),
            mems: m.mems.iter().enumerate().map(|(k, i)| (*i, k as u8)).collect(),
        });
        if memo.is_some_and(|m| m.states.len() > 64 || m.mems.len() > 255) {
            return Err(GenError::new(format!("`{}`: memo fns read more than 64 state variables", comp.name)));
        }

        // The ports trait.
        let mut t = format!(
            "/// What a `{name}` model reaches through its ports. Every method has a\n/// default (nothing attached: open bus, low levels, emits go nowhere), so a\n/// hand-written context overrides only what it observes.\npub trait {name}Ports {{\n"
        );
        for p in &comp.ports {
            let wire = match &p.kind {
                PortKind::In(ty) | PortKind::AsyncIn(ty) => Some((ty, false)),
                PortKind::Out(ty) => Some((ty, true)),
                _ => None,
            };
            if let Some((ty, out)) = wire {
                let rt = self.rty(ty)?;
                let _ = writeln!(
                    t,
                    "    /// The level on wire port `{0}`.\n    fn read_{1}(&mut self) -> {rt} {{ Default::default() }}",
                    p.name,
                    ident(&p.name)
                );
                if out {
                    let _ = writeln!(
                        t,
                        "    /// Drive output `{0}`.\n    #[allow(unused_variables)]\n    fn write_{1}(&mut self, value: {rt}) {{}}",
                        p.name,
                        ident(&p.name)
                    );
                }
            }
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

        // The struct, with the compiler's view of its state: what writes
        // each piece (SPEC 8.3b).
        let mut st = format!("/// The `{}` model. Its state, by what writes it:\n", comp.name);
        for u in &model.state_use {
            let who = if u.writers.is_empty() { "the host only".to_string() } else { u.writers.join(", ") };
            let memo = if u.memo_reads { "; read by memo fns" } else { "" };
            let _ = writeln!(st, "/// - `{}`{}: {who}{memo}", u.name, if u.mem { " (mem)" } else { "" });
        }
        st.push_str("#[derive(Clone, Debug)]\n");
        let _ = writeln!(st, "pub struct {name} {{");
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
        let mut memo_init = String::new();
        if let Some(mi) = memo {
            let _ = writeln!(st, "    /// `memo fn`s: the version counter, the versions of the state and\n    /// memory blocks they read, what the running call reads, and each\n    /// fn's calls by arguments.\n    pub memo_epoch: u64,\n    pub memo_ver: [u64; {}],\n    pub memo_blocks: Vec<Vec<u64>>,\n    pub memo_all: Vec<u64>,\n    pub memo_rec: rt::memo::Rec,", mi.states.len());
            let _ = write!(memo_init, "            memo_epoch: 0,\n            memo_ver: [0; {}],\n            memo_blocks: vec![", mi.states.len());
            for &m in &mi.mems {
                let len = model.mems[m as usize].len;
                let _ = write!(memo_init, "vec![0; {}], ", len.div_ceil(64));
            }
            let _ = writeln!(memo_init, "],\n            memo_all: vec![0; {}],\n            memo_rec: rt::memo::Rec::default(),", mi.mems.len());
            for &f in &mi.fns {
                let f = &model.fns[f as usize];
                let key: R<Vec<String>> = f.params.iter().map(|p| self.rty(&f.locals[*p as usize].ty)).collect();
                let _ = writeln!(st, "    pub memo_{}: std::collections::HashMap<({},), rt::memo::Entry>,", ident(&f.name), key?.join(", "));
                let _ = writeln!(memo_init, "            memo_{}: std::collections::HashMap::new(),", ident(&f.name));
            }
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
        st.push_str(&memo_init);
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
        if let Some(mi) = memo {
            let mut clears = String::new();
            for &f in &mi.fns {
                let _ = write!(clears, " self.memo_{}.clear();", ident(&model.fns[f as usize].name));
            }
            let _ = write!(
                st,
                "    /// A new version for a write to state a `memo fn` reads.\n    #[inline]\n    pub fn memo_bump(&mut self) -> u64 {{\n        self.memo_epoch += 1;\n        self.memo_epoch\n    }}\n    /// Forget every `memo fn` call: after the embedding program writes\n    /// state or memory directly.\n    pub fn memo_clear(&mut self) {{{clears} }}\n"
            );
        }

        for (fi, f) in model.fns.iter().enumerate() {
            let is_memo = memo.is_some_and(|m| m.fns.contains(&(fi as u32)));
            let rec_copy = memo.is_some_and(|m| m.reach.contains(&(fi as u32)));
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
            let (ret, tail) = match f.ret {
                Ty::Unit => (String::new(), ""),
                ref t => (format!(" -> {}", self.rty(t)?), "        #[allow(unreachable_code)]\n        { unreachable!(\"fn fell off its end\") }\n"),
            };
            let fname = ident(&f.name);
            if !is_memo {
                let body = self.block_stmts(&cx, &f.body)?;
                let _ = write!(
                    st,
                    "    /// `fn {}` of the model.\n    pub fn fn_{fname}<P: {name}Ports>(&mut self, ctx: &mut P{ps}){ret} {{\n{body}{tail}    }}\n",
                    f.name
                );
            }
            if rec_copy {
                self.memo_rec.set(true);
                let body = self.block_stmts(&cx, &f.body);
                self.memo_rec.set(false);
                let body = body?;
                let _ = write!(
                    st,
                    "    /// `fn {}`, recording what it reads for a `memo fn`.\n    #[allow(dead_code)]\n    fn fn_{fname}_rec<P: {name}Ports>(&mut self, ctx: &mut P{ps}){ret} {{\n{body}{tail}    }}\n",
                    f.name
                );
            }
            if is_memo {
                let args: Vec<String> = f.params.iter().map(|p| self.local_name(&cx, *p)).collect();
                let key = format!("({},)", args.join(", "));
                let pass: String = args.iter().map(|a| format!(", {a}")).collect();
                let _ = write!(
                    st,
                    "    /// `memo fn {0}` of the model: skipped while what its last call with\n    /// these arguments read is unchanged.\n    pub fn fn_{fname}<P: {name}Ports>(&mut self, ctx: &mut P{ps}) {{\n        let key = {key};\n        if let Some(e) = self.memo_{fname}.get(&key) {{\n            let (ver, blocks, all) = (&self.memo_ver, &self.memo_blocks, &self.memo_all);\n            if e.valid(|k| ver[k as usize], |m| (&blocks[m as usize][..], all[m as usize])) {{\n                return;\n            }}\n        }}\n        let outer = std::mem::take(&mut self.memo_rec);\n        self.fn_{fname}_rec(ctx{pass});\n        let rec = std::mem::replace(&mut self.memo_rec, outer);\n        self.memo_{fname}.insert(key, rt::memo::Entry {{ at: self.memo_epoch, vars: rec.vars, ranges: rec.ranges }});\n    }}\n",
                    f.name
                );
            }
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
                HandlerKind::SpaceFetch { space, bits } => {
                    format!("{}_fetch_u{bits}", ident(&self.spaces[space].name))
                }
                HandlerKind::Extern { def } => format!("ext_{}", ident(&self.prog.item_name(*def))),
                HandlerKind::Input { port } => {
                    format!("on_{}", ident(&comp.ports[*port as usize].name))
                }
                HandlerKind::Retire => "on_retire".to_string(),
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
        *self.memo_cur.borrow_mut() = None;
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
            let decodes = self
                .bus(b.bus)?
                .transactions
                .iter()
                .any(|t| t.params.iter().any(|(n, _)| n == "addr"));
            if decodes && b.slaves.iter().filter(|(_, r)| r.is_empty()).count() > 1 {
                return Err(GenError::new(format!(
                    "bus `{}` has several slaves without address ranges",
                    b.name
                )));
            }
        }
        let cpu = &insts[proc_idx];
        let cpu_name = ident(&cpu.name);
        // A processor model with `on retire` owns its timing (SPEC 11.2c):
        // its own transactions are charged only their wait states.
        let cpu_retire = cpu
            .model
            .handlers
            .iter()
            .any(|h| h.kind == HandlerKind::Retire);
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
            "\n/// The system `{}`.\npub mod system {{\n    use super::*;\n\n    /// `e / p` for times: in 64 bits when both fit (an elapsed time is\n    /// short), else in 128.\n    #[inline(always)]\n    pub fn div_time(e: clock::Time, p: clock::Time) -> clock::Time {{\n        if (e | p) >> 64 == 0 {{ (e as u64 / p as u64) as clock::Time }} else {{ e / p }}\n    }}\n\n    /// The time unit is `BASE_NUM / BASE_DEN` seconds.\n    pub const BASE_NUM: u128 = {};\n    pub const BASE_DEN: u128 = {};\n    /// Clock periods in time units.\n    pub mod clock {{\n        /// A time: a count of the system's time unit. 128 bits, so that time\n        /// stays exact over any run however fine the unit the clocks need.\n        pub type Time = u128;\n",
            sys.name, sys.base_num, sys.base_den
        );
        for (i, c) in sys.clocks.iter().enumerate() {
            let _ = writeln!(
                o,
                "        /// `{}`: {} / {} Hz\n        pub const C{i}: Time = {};",
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
        // Wires: one level per output port; an edge when it changes.
        for i in &insts {
            for p in &i.comp.ports {
                if let PortKind::Out(t) = &p.kind {
                    let f = format!("w_{}_{}", ident(&i.name), ident(&p.name));
                    let _ = writeln!(sig_fields, "        pub {f}: {},", self.rty(t)?);
                    let _ = writeln!(sig_init, "                {f}: {},", self.default_value(t)?);
                    let _ = writeln!(events, "        {f},");
                }
            }
        }
        let _ = write!(
            o,
            "    #[allow(non_camel_case_types)]\n    #[derive(Clone, Copy, Debug, PartialEq)]\n    pub enum Event {{\n{events}    }}\n    /// Time and signal levels.\n    #[derive(Clone, Debug)]\n    pub struct Signals {{\n        /// Time in units of `BASE_NUM / BASE_DEN` seconds.\n        pub now: clock::Time,\n{sig_fields}        /// Edges not yet delivered.\n        pub events: Vec<Event>,\n    }}\n    impl Signals {{\n        pub fn new() -> Signals {{\n            Signals {{\n                now: 0,\n{sig_init}                events: Vec::new(),\n            }}\n        }}\n    }}\n"
        );

        // Tick marks: the last time each device's clock was caught up to.
        let mut marks = String::new();
        // (inst, handler clock port, system clock)
        let mut ticks: Vec<(usize, Option<u32>, u32)> = Vec::new();
        // Tick handlers that return their next deadline.
        let mut scheduled: Vec<(usize, Option<u32>)> = Vec::new();
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
                    "        pub {}_{}: clock::Time,",
                    ident(&i.name),
                    method_tick(i.comp, clock)
                );
                if h.ret == Ty::UInt(32) {
                    let _ = writeln!(
                        marks,
                        "        /// When `{}`'s `{}` must next run.\n        pub due_{}_{}: clock::Time,",
                        i.name,
                        method_tick(i.comp, clock),
                        ident(&i.name),
                        method_tick(i.comp, clock)
                    );
                    scheduled.push((ii, clock));
                }
                ticks.push((ii, clock, bound));
            }
        }
        let _ = write!(
            o,
            "    #[derive(Clone, Debug, Default)]\n    pub struct Marks {{\n{marks}    }}\n"
        );

        // Who masters a bus: the processor, and at most one device.
        // Only a bus with transactions has anything to initiate: the
        // first role of a signals-only bus (an interrupt controller on its
        // request lines) is not a master in this sense.
        let mut has_txns = Vec::new();
        for b in &sys.buses {
            has_txns.push(!self.bus(b.bus)?.transactions.is_empty());
        }
        let masters_any = |ii: usize| -> bool {
            sys.buses
                .iter()
                .zip(&has_txns)
                .any(|(b, t)| *t && b.masters.iter().any(|(i, _)| *i as usize == ii))
        };
        let dev_masters: Vec<usize> = (0..insts.len())
            .filter(|&ii| ii != proc_idx && masters_any(ii))
            .collect();
        // What a master can reach: the devices on the buses it masters, and
        // through any of them that master buses themselves (a bridge), the
        // devices on those, never itself or the processor. Its context
        // holds those, each `None` when the context is built inside another
        // one that cannot lend it (a cycle through the bridge).
        let mut reach: Vec<Vec<usize>> = vec![Vec::new(); insts.len()];
        for (ii, out) in reach.iter_mut().enumerate() {
            if ii != proc_idx && !dev_masters.contains(&ii) {
                continue;
            }
            let mut todo = vec![ii];
            let mut seen = vec![ii];
            while let Some(m) = todo.pop() {
                for (b, t) in sys.buses.iter().zip(&has_txns) {
                    if !*t || !b.masters.iter().any(|(i, _)| *i as usize == m) {
                        continue;
                    }
                    for ((si, _), _) in &b.slaves {
                        let si = *si as usize;
                        if si == proc_idx || seen.contains(&si) {
                            continue;
                        }
                        seen.push(si);
                        out.push(si);
                        if dev_masters.contains(&si) {
                            todo.push(si);
                        }
                    }
                }
            }
            out.sort();
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
        // The context for `ii`, built from `src`: the board (`from` None,
        // owning every device) or the context of master `from`.
        let ctx_for = |ii: usize, src: &str, from: Option<usize>| -> String {
            let n = ident(&insts[ii].name);
            let deref = from.is_some();
            if full(ii) {
                let mut s = format!(
                    "Ctx_{n} {{ sig: {}, marks: {}",
                    field(src, deref, "sig"),
                    field(src, deref, "marks")
                );
                for &jj in &reach[ii] {
                    let jn = ident(&insts[jj].name);
                    let v = match from {
                        None => format!("Some(&mut {src}.{jn})"),
                        Some(x) if reach[x].contains(&jj) => format!("{src}.{jn}.as_deref_mut()"),
                        Some(_) => "None".to_string(),
                    };
                    let _ = write!(s, ", {jn}: {v}");
                }
                s.push_str(" }");
                s
            } else {
                format!("Ctx_{n} {{ sig: {} }}", field(src, deref, "sig"))
            }
        };
        let ticking = |ii: usize| ticks.iter().any(|t| t.0 == ii);
        // Statements that make device `ii` due (its scheduled ticks run at
        // the next sync), from a source holding the marks.
        let make_due = |ii: usize, src: &str, deref: bool| -> String {
            let n = ident(&insts[ii].name);
            let m = if deref {
                format!("(*{src}.marks)")
            } else {
                format!("{src}.marks")
            };
            scheduled
                .iter()
                .filter(|(i, _)| *i == ii)
                .map(|(_, clock)| {
                    format!(" {m}.due_{n}_{} = 0;", method_tick(insts[ii].comp, *clock))
                })
                .collect()
        };
        // Statements that catch device `ii` up, from the board.
        let catch_from = |ii: usize, src: &str, _deref: bool| -> String {
            if !ticking(ii) {
                return String::new();
            }
            let n = ident(&insts[ii].name);
            if full(ii) {
                format!(
                    "{{ let mut c = {}; catch_up_{n}(&mut {src}.{n}, &mut c); }} ",
                    ctx_for(ii, src, None)
                )
            } else {
                format!("catch_up_{n}(&mut {src}.{n}, &mut {src}.sig, &mut {src}.marks); ")
            }
        };
        // The same inside master `from`'s context, for a device bound to `dev`.
        // A master whose reach that context cannot lend entirely (it is
        // inside a bridge the master's transfers would cross) is not caught
        // up here: it was before the bridge was entered, and is again after
        // the instruction.
        let catch_in = |ii: usize, from: usize| -> String {
            if !ticking(ii) {
                return String::new();
            }
            let n = ident(&insts[ii].name);
            if full(ii)
                && reach[ii]
                    .iter()
                    .any(|j| *j == from || !reach[from].contains(j))
            {
                return String::new();
            }
            if full(ii) {
                format!(
                    "{{ let mut c = {}; catch_up_{n}(dev, &mut c); }} ",
                    ctx_for(ii, "self", Some(from))
                )
            } else {
                format!("catch_up_{n}(dev, &mut *self.sig, &mut *self.marks); ")
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
                for &jj in &reach[ii] {
                    let j = &insts[jj];
                    let _ = writeln!(
                        fields,
                        "        pub {}: Option<&'a mut {}>,",
                        ident(&j.name),
                        ident(&j.comp.name)
                    );
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
                        let base = if ii == proc_idx && cpu_retire {
                            0
                        } else {
                            base
                        };
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
                            // Entering a bridge: first catch up the masters behind
                            // it, while this context can still lend what their
                            // transfers reach.
                            let mut behind = String::new();
                            if full(si) {
                                for &m in &reach[si] {
                                    if m != ii && full(m) && ticking(m) && reach[ii].contains(&m) {
                                        let c = catch_in(m, ii);
                                        if !c.is_empty() {
                                            let mn = ident(&insts[m].name);
                                            // Only when it is due: by its own
                                            // deadline it does nothing before.
                                            let mine: Vec<&(usize, Option<u32>, u32)> = ticks.iter().filter(|t| t.0 == m).collect();
                                            let gate = if mine.iter().all(|t| scheduled.contains(&(m, t.1))) {
                                                let dues: Vec<String> = mine.iter().map(|t| format!("self.sig.now >= self.marks.due_{mn}_{}", method_tick(insts[m].comp, t.1))).collect();
                                                dues.join(" || ")
                                            } else {
                                                "true".to_string()
                                            };
                                            let _ = write!(behind, "if {gate} {{ if let Some(dev) = self.{mn}.as_deref_mut() {{ {c}}} }} ");
                                        }
                                    }
                                }
                            }
                            let catch = format!("{behind}let Some(dev) = self.{dn}.as_deref_mut() else {{ self.sig.now += {base} * {period}; return {open}; }}; {}", catch_in(si, ii));
                            // The responder may have changed what it schedules:
                            // it is due again after the access.
                            let call = format!(
                                "{{ let r = {{ let mut c = {}; dev.on_{}_{}(&mut c, {}) }};{} r }}",
                                ctx_for(si, "self", Some(ii)),
                                ident(&dev.comp.ports[*sp as usize].name),
                                ident(&tr.name),
                                args.join(", "),
                                make_due(si, "self", true)
                            );
                            let Some(a) = addr.clone() else {
                                // Not decoded: offered to each responder that
                                // handles it, in binding order, until one
                                // does not decline.
                                if handler {
                                    let _ = writeln!(
                                        body,
                                        "        {{\n            {catch}if let Some(r) = {call} {{\n                self.sig.now += ({base} + r.{n_ret} as clock::Time) * {period};\n                return {result};\n            }}\n        }}"
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
                                "        if {cond} {{\n            {catch}match {call} {{\n                Some(r) => {{\n                    self.sig.now += ({base} + r.{n_ret} as clock::Time) * {period};\n                    return {result};\n                }}\n                None => {{\n                    self.sig.now += {base} * {period};\n                    return {open};\n                }}\n            }}\n        }}"
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
            for (pi, p) in i.comp.ports.iter().enumerate() {
                let pn = ident(&p.name);
                match &p.kind {
                    PortKind::Out(t) => {
                        let f = format!("w_{n}_{pn}");
                        let rt = self.rty(t)?;
                        let _ = writeln!(
                            m,
                            "        fn read_{pn}(&mut self) -> {rt} {{ self.sig.{f} }}"
                        );
                        let _ = writeln!(m, "        fn write_{pn}(&mut self, value: {rt}) {{ if self.sig.{f} != value {{ self.sig.{f} = value; self.sig.events.push(Event::{f}); }} }}");
                    }
                    PortKind::In(t) | PortKind::AsyncIn(t) => {
                        let rt = self.rty(t)?;
                        let src = sys
                            .wires
                            .iter()
                            .find(|w| w.target == (ii as u32, pi as u32))
                            .map(|w| &w.source);
                        let body = match src {
                            Some(hir::WireSource::Port((si, sp))) => {
                                format!(
                                    "self.sig.w_{}_{}",
                                    ident(&insts[*si as usize].name),
                                    ident(&insts[*si as usize].comp.ports[*sp as usize].name)
                                )
                            }
                            Some(hir::WireSource::Const(v)) => match t {
                                Ty::Bool => format!("{}", *v != 0),
                                _ => format!("{v} as {rt}"),
                            },
                            None => self.default_value(t)?,
                        };
                        let _ = writeln!(m, "        fn read_{pn}(&mut self) -> {rt} {{ {body} }}");
                    }
                    _ => {}
                }
            }
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
                let sched = scheduled.contains(&(ii, *clock));
                let (call_pre, after) = if sched {
                    (
                        "let r = ",
                        format!("\n                {{m}}.due_{n}_{mt} = {{m}}.{n}_{mt}.saturating_add((r as clock::Time).saturating_mul(p));"),
                    )
                } else {
                    ("", String::new())
                };
                if full(ii) {
                    let after = after.replace("{m}", "c.marks");
                    let _ = write!(
                        body,
                        "        {{\n            let p = clock::C{sc};\n            let mut k = div_time(c.sig.now - c.marks.{n}_{mt}, p);\n            c.marks.{n}_{mt} += k * p;\n            while k > 0 {{\n                let s = k.min(u32::MAX as clock::Time);\n                k -= s;\n                {call_pre}dev.{mt}(&mut *c, s as u32);{after}\n            }}\n        }}\n"
                    );
                } else {
                    let after = after.replace("{m}", "marks");
                    let _ = write!(
                        body,
                        "        {{\n            let p = clock::C{sc};\n            let mut k = div_time(sig.now - marks.{n}_{mt}, p);\n            marks.{n}_{mt} += k * p;\n            while k > 0 {{\n                let s = k.min(u32::MAX as clock::Time);\n                k -= s;\n                let mut c = Ctx_{n} {{ sig: &mut *sig }};\n                {call_pre}dev.{mt}(&mut c, s as u32);{after}\n            }}\n        }}\n"
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
        let ctx_cpu = |b: &str| ctx_for(proc_idx, b, None);

        // The board.
        let mut fields = String::from("        pub sig: Signals,\n        pub marks: Marks,\n        /// Time the current instruction spent fetching code (overlapped\n        /// with its execution at retire).\n        pub code_time: clock::Time,\n        /// The bus unit code was last fetched from directly (`memory: direct`).\n        pub direct_code: u64,\n");
        let mut init = String::from(
            "            sig: Signals::new(),\n            marks: Marks::default(),\n            code_time: 0,\n            direct_code: u64::MAX,\n",
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
        // Direct memory (`memory: direct`): the processor's accesses in the
        // ranges of slaves whose memory declares `direct` read and write
        // the cells, charged a transfer's clocks for each bus-width unit
        // they touch; code is charged once a unit, as a one-unit buffer.
        let mut direct = String::new();
        let mut direct_hooks = (String::new(), String::new(), String::new());
        if prof.is_some_and(|p| p.direct_memory) {
            if cpu_retire {
                return Err(GenError::new(format!(
                    "profile `{}`: `memory: direct` bypasses the processor model's accesses, but `{}` times instructions itself (`on retire`)",
                    prof.unwrap().name,
                    cpu.name
                )));
            }
            let msp = self.isa.istream.as_ref().map(|i| self.spaces[&i.space]);
            let mut reads = String::new();
            let mut writes = String::new();
            let mut fetches = String::new();
            if let Some(msp) = msp.filter(|s| s.cell_bits == 8 && s.little_endian) {
                for b in sys.buses.iter().filter(|b| b.masters.iter().any(|(m, _)| *m as usize == proc_idx)) {
                    let info = self.bus(b.bus)?;
                    let tr = |name: &str| info.transactions.iter().find(|t| t.name == name);
                    let (Some(rd), Some(wr)) = (tr("read"), tr("write")) else { continue };
                    let (Some(rbase), Some(wbase)) = (rd.base_cycles, wr.base_cycles) else { continue };
                    let mut unit_bytes = 1u64;
                    if let Some(Ty::Tuple(items)) = &rd.ret {
                        if let Some((_, Ty::UInt(w))) = items.iter().find(|(n, _)| n.as_deref() == Some("data")) {
                            unit_bytes = (*w as u64 / 8).max(1);
                        }
                    }
                    // The `space` value that is the isa's memory space, by name.
                    let mem_value = rd.params.iter().find(|(n, _)| n == "space").and_then(|(_, t)| match t {
                        Ty::Enum(e) => self.enums[e].variants.iter().find(|(v, _)| v.eq_ignore_ascii_case(&msp.name)).map(|(_, x)| *x),
                        _ => None,
                    });
                    let period = format!("clock::C{}", b.clock);
                    for ((si, sp), ranges) in &b.slaves {
                        let dev = &insts[*si as usize];
                        let Some(m) = dev.model.mems.iter().find(|m| m.direct.as_ref().is_some_and(|d| d.port == *sp)) else { continue };
                        let d = m.direct.as_ref().unwrap();
                        let cells = format!("self.{}.{}", ident(&dev.name), ident(&m.name));
                        let index = |a: &str| if d.mask == u128::MAX { format!("({a}) as usize") } else { format!("(({a}) & {:#x}) as usize", d.mask) };
                        for r in ranges {
                            if let Some(v) = r.space {
                                if Some(v) != mem_value { continue; }
                            }
                            if d.mask == u128::MAX && r.hi >= m.len as u128 { continue; }
                            let cond = format!("addr >= {:#x} && last <= {:#x}", r.lo, r.hi);
                            let units = format!("(last / {unit_bytes} - addr / {unit_bytes} + 1) as clock::Time");
                            let _ = writeln!(reads, "        if {cond} {{ let mut v: u128 = 0; for i in 0..n {{ v |= ({cells}[{}] as u128) << (8 * i); }} self.sig.now += {units} * {} * {period}; return Some(v); }}", index("addr + i"), rbase as u128 + d.wait);
                            if !d.read_only {
                                let _ = writeln!(writes, "        if {cond} {{ for i in 0..n {{ {cells}[{}] = (value >> (8 * i)) as u8; }} self.sig.now += {units} * {} * {period}; return true; }}", index("addr + i"), wbase as u128 + d.wait);
                            }
                            let _ = writeln!(fetches, "        if addr >= {:#x} && addr <= {:#x} {{ let u = addr / {unit_bytes}; if u != self.direct_code {{ self.direct_code = u; let t = {} * {period}; self.sig.now += t; self.code_time += t; }} return Some({cells}[{}]); }}", r.lo, r.hi, rbase as u128 + d.wait, index("addr"));
                        }
                    }
                }
                let amask = mask_literal(match &msp.addr { Ty::UInt(b) => *b, _ => 64 }, "u64");
                direct_hooks = (
                    format!("        if let Some(v) = self.direct_read(addr & {amask}, (bits / 8) as u64) {{ return v; }}\n"),
                    format!("        if self.direct_write(addr & {amask}, (bits / 8) as u64, value) {{ return; }}\n"),
                    format!("        if bits == 8 {{ if let Some(b) = self.direct_fetch(addr & {amask}) {{ return b as u128; }} }}\n"),
                );
            }
            let _ = write!(
                direct,
                "        /// `memory: direct`: `n` bytes at `addr` from a memory that\n        /// declares `direct`, else None.\n        #[inline]\n        #[allow(unused_variables)]\n        fn direct_read(&mut self, addr: u64, n: u64) -> Option<u128> {{\n        let last = addr.wrapping_add(n - 1);\n{reads}        None\n        }}\n        #[inline]\n        #[allow(unused_variables)]\n        fn direct_write(&mut self, addr: u64, n: u64, value: u128) -> bool {{\n        let last = addr.wrapping_add(n - 1);\n{writes}        false\n        }}\n        #[inline]\n        #[allow(unused_variables)]\n        fn direct_fetch(&mut self, addr: u64) -> Option<u8> {{\n{fetches}        self.direct_code = u64::MAX;\n        None\n        }}\n"
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
                let is_code = self.isa.istream.as_ref().is_some_and(|i| i.space == sp.def);
                let hook = match (is_code, write) {
                    (false, _) => "",
                    (true, false) => direct_hooks.0.as_str(),
                    (true, true) => direct_hooks.1.as_str(),
                };
                if write {
                    let _ = write!(
                        bus_impl,
                        "    fn {n}_write(&mut self, addr: u64, bits: u32, value: u128) {{\n{hook}        let mut ctx = {};\n        let addr = addr & {amask};\n        match bits {{\n{arms}            _ => {{\n                let n = bits / {cell};\n                for i in 0..n {{\n                    let a = (addr + i as u64) & {amask};\n                    let v = (value >> ({shift})) as {cty};\n                    self.{cpu_name}.{n}_write_u{cell}(&mut ctx, a as {aty}, v);\n                }}\n            }}\n        }}\n    }}\n",
                        ctx_cpu("self")
                    );
                } else {
                    let _ = write!(
                        bus_impl,
                        "    fn {n}_read(&mut self, addr: u64, bits: u32) -> u128 {{\n{hook}        let mut ctx = {};\n        let addr = addr & {amask};\n        match bits {{\n{arms}            _ => {{\n                let n = bits / {cell};\n                let mut v: u128 = 0;\n                for i in 0..n {{\n                    let a = (addr + i as u64) & {amask};\n                    v |= (self.{cpu_name}.{n}_read_u{cell}(&mut ctx, a as {aty}) as u128) << ({shift});\n                }}\n                v\n            }}\n        }}\n    }}\n",
                        ctx_cpu("self")
                    );
                }
            }
        }
        // Instruction fetches: the fetch handler (else the read handler),
        // timed as code; and retire, which overlaps execution with them.
        if let Some(ist) = &self.isa.istream {
            let sp = self.spaces[&ist.space];
            let n = ident(&sp.name);
            let (abits, aty) = match &sp.addr {
                Ty::UInt(b) => (*b, native(*b, false).1),
                _ => return Err(GenError::new("space address type")),
            };
            let amask = mask_literal(abits, "u64");
            let cell = sp.cell_bits;
            let fetch_bits: Vec<u32> = cpu
                .model
                .handlers
                .iter()
                .filter_map(|h| match h.kind {
                    HandlerKind::SpaceFetch { space, bits } if space == sp.def => Some(bits),
                    _ => None,
                })
                .collect();
            let mut arms = String::new();
            for b in &fetch_bits {
                let _ = writeln!(arms, "            {b} => self.{cpu_name}.{n}_fetch_u{b}(&mut ctx, addr as {aty}) as u128,");
            }
            let cell_fetch = if fetch_bits.contains(&cell) {
                format!("self.{cpu_name}.{n}_fetch_u{cell}(&mut ctx, a as {aty})")
            } else {
                format!("self.{cpu_name}.{n}_read_u{cell}(&mut ctx, a as {aty})")
            };
            let shift = if sp.little_endian {
                format!("i * {cell}")
            } else {
                format!("(n - 1 - i) * {cell}")
            };
            let _ = write!(
                bus_impl,
                "    fn {n}_fetch(&mut self, addr: u64, bits: u32) -> u128 {{\n{}        let t0 = self.sig.now;\n        let v = {{\n            let mut ctx = {};\n            let addr = addr & {amask};\n            match bits {{\n{arms}                _ => {{\n                    let n = bits / {cell};\n                    let mut v: u128 = 0;\n                    for i in 0..n {{\n                        let a = (addr + i as u64) & {amask};\n                        v |= ({cell_fetch} as u128) << ({shift});\n                    }}\n                    v\n                }}\n            }}\n        }};\n        self.code_time += self.sig.now - t0;\n        v\n    }}\n",
                direct_hooks.2,
                ctx_cpu("self")
            );
            if cpu_retire {
                let _ = write!(
                    bus_impl,
                    "    /// The processor model times the instruction from its schedule.\n    fn retire(&mut self, _cycles: u32, schedule: Schedule) {{\n        let c = {{\n            let mut ctx = {};\n            self.{cpu_name}.on_retire(&mut ctx, schedule)\n        }};\n        self.sig.now += c as clock::Time * clock::C{cpu_clock};\n        self.code_time = 0;\n    }}\n",
                    ctx_cpu("self")
                );
            } else {
                let _ = write!(
                bus_impl,
                "    /// The board times instructions from their cycles: no schedule.\n    const SCHEDULE: bool = false;\n    /// The instruction's execution overlaps its code fetches: charge what\n    /// exceeds them.\n    fn retire(&mut self, cycles: u32, _schedule: Schedule) {{\n        let x = cycles as clock::Time * clock::C{cpu_clock};\n        if x > self.code_time {{\n            self.sig.now += x - self.code_time;\n        }}\n        self.code_time = 0;\n    }}\n"
            );
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
            "    /// Every component model, the time and the signal levels: the processor's bus.\n    #[derive(Clone, Debug)]\n    pub struct Board {{\n{fields}    }}\n    impl Board {{\n        pub fn new() -> Board {{\n            Board {{\n{init}            }}\n        }}\n{direct}    }}\n    impl Bus for Board {{\n{bus_impl}    }}\n"
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
                ctx_for(ii, "b", None)
            );
        }
        let mut sync = String::new();
        for (ii, inst) in insts.iter().enumerate() {
            let c = catch_from(ii, "b", false);
            if c.is_empty() {
                continue;
            }
            let mine: Vec<&(usize, Option<u32>, u32)> =
                ticks.iter().filter(|t| t.0 == ii).collect();
            let all_scheduled = mine.iter().all(|t| scheduled.contains(&(ii, t.1)));
            if all_scheduled {
                let n = ident(&inst.name);
                let dues: Vec<String> = mine
                    .iter()
                    .map(|t| {
                        format!(
                            "b.sig.now >= b.marks.due_{n}_{}",
                            method_tick(inst.comp, t.1)
                        )
                    })
                    .collect();
                let _ = writeln!(sync, "            if {} {{ {c}}}", dues.join(" || "));
            } else {
                let _ = writeln!(sync, "            {c}");
            }
        }
        let mut all = String::new();
        for ii in 0..insts.len() {
            let c = catch_from(ii, "b", false);
            if !c.is_empty() {
                let _ = writeln!(all, "            {c}");
            }
        }
        // Waking a device: catch it up before an edge reaches it, and make
        // it due after, so it recomputes its deadline.
        let wake = |ii: usize| -> (String, String) {
            let before = catch_from(ii, "b", false);
            let n = ident(&insts[ii].name);
            let after: String = scheduled
                .iter()
                .filter(|(i, _)| *i == ii)
                .map(|(_, clock)| {
                    format!(
                        " b.marks.due_{n}_{} = 0;",
                        method_tick(insts[ii].comp, *clock)
                    )
                })
                .collect();
            (before, after)
        };
        // Host entry points: a model's fns with a live context.
        let mut with = String::new();
        for (ii, i) in insts.iter().enumerate() {
            let n = ident(&i.name);
            let cname = ident(&i.comp.name);
            let catch = catch_from(ii, "b", false);
            let ctx = ctx_for(ii, "b", None);
            let due = make_due(ii, "b", false);
            let _ = write!(
                with,
                "        /// Call into `{n}` with a live context: how the embedding program\n        /// reaches its model fns (host input). The device is caught up first,\n        /// made due after, and the signal edges it raises are delivered.\n        pub fn with_{n}<R>(&mut self, f: impl FnOnce(&mut {cname}, &mut Ctx_{n}<'_>) -> R) -> R {{\n            let r = {{\n                let b = &mut self.{cpu_name}.bus;\n                {catch}\n                let r = {{ let mut c = {ctx}; f(&mut b.{n}, &mut c) }};\n               {due}\n                r\n            }};\n            self.deliver();\n            r\n        }}\n"
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
                let (before, after) = wake(ii as usize);
                let _ = writeln!(
                    calls,
                    "                    {before}{{ let mut c = {}; b.{n}.on_{}_{}(&mut c, {arg}v); }}{after}",
                    ctx_for(ii as usize, "b", None),
                    ident(&i.comp.ports[pi as usize].name),
                    ident(&self.prog.item_name(*sd))
                );
            }
            let _ = writeln!(
                deliver,
                "                Event::{f}(i, v) => {{\n{calls}                }}"
            );
        }
        for (si, src) in insts.iter().enumerate() {
            for (sp, p) in src.comp.ports.iter().enumerate() {
                if !matches!(p.kind, PortKind::Out(_)) {
                    continue;
                }
                let f = format!("w_{}_{}", ident(&src.name), ident(&p.name));
                let mut calls = String::new();
                for w in &sys.wires {
                    if w.source != hir::WireSource::Port((si as u32, sp as u32)) {
                        continue;
                    }
                    let (ti, tp) = w.target;
                    let t = &insts[ti as usize];
                    if !t
                        .model
                        .handlers
                        .iter()
                        .any(|h| h.kind == HandlerKind::Input { port: tp })
                    {
                        continue;
                    }
                    let (before, after) = wake(ti as usize);
                    let _ = writeln!(
                        calls,
                        "                    {before}{{ let v = b.sig.{f}; let mut c = {}; b.{}.on_{}(&mut c, v); }}{after}",
                        ctx_for(ti as usize, "b", None),
                        ident(&t.name),
                        ident(&t.comp.ports[tp as usize].name)
                    );
                }
                let _ = writeln!(
                    deliver,
                    "                Event::{f} => {{\n{calls}                }}"
                );
            }
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
        pub fn now(&self) -> clock::Time {{
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
{with}        /// Catch every device up to the present, scheduled or not (before
        /// the host reads a device's state directly), and deliver edges.
        pub fn catch_up_all(&mut self) {{
            let b = &mut self.{cpu_name}.bus;
{all}            self.deliver();
        }}
        /// Catch up every device that is due (a scheduled `on tick` runs
        /// only when its deadline has passed) and deliver signal edges.
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
