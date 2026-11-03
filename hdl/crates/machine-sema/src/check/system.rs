//! Checking a `system`: clocks with exact frequencies, component instances
//! and their clock bindings, bus instances with the ports attached in each
//! role and the address ranges of each slave, and profiles.

use std::collections::HashMap;

use machine_syntax::ast::{
    self, BinaryOp, BusBinding, ClockSource, ExprKind as A, SystemItem, Unit,
};
use machine_syntax::diagnostics::Diagnostic;
use machine_syntax::span::Span;

use super::{Checker, Scope};
use crate::hir::{self, HandlerKind, Level, PortKind};
use crate::program::{DefId, ItemKind, ModuleId};
use crate::ty::Ty;

fn gcd(a: u128, b: u128) -> u128 {
    if b == 0 {
        a
    } else {
        gcd(b, a % b)
    }
}

/// A positive rational, reduced.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Ratio(u128, u128);

impl Ratio {
    fn new(n: u128, d: u128) -> Option<Ratio> {
        if n == 0 || d == 0 {
            return None;
        }
        let g = gcd(n, d);
        Some(Ratio(n / g, d / g))
    }
    fn mul(self, n: u128, d: u128) -> Option<Ratio> {
        let g1 = gcd(self.0, d);
        let g2 = gcd(n, self.1);
        Ratio::new(
            (self.0 / g1).checked_mul(n / g2)?,
            (self.1 / g2).checked_mul(d / g1)?,
        )
    }
}

/// `33.333` as `33333 / 1000`.
fn decimal(text: &str) -> Option<(u128, u128)> {
    let text: String = text.chars().filter(|c| *c != '_').collect();
    let (int, frac) = text.split_once('.').unwrap_or((&text, ""));
    let mut n: u128 = 0;
    for c in int.chars().chain(frac.chars()) {
        n = n.checked_mul(10)?.checked_add(c.to_digit(10)? as u128)?;
    }
    Some((n, 10u128.checked_pow(frac.len() as u32)?))
}

/// A frequency from a literal: a frequency unit directly, a time unit as
/// its period.
fn quantity_hz(text: &str, unit: Unit) -> Option<Ratio> {
    let (n, d) = decimal(text)?;
    let v = Ratio::new(n, d)?;
    match unit {
        Unit::Hz => Some(v),
        Unit::KHz => v.mul(1_000, 1),
        Unit::MHz => v.mul(1_000_000, 1),
        Unit::GHz => v.mul(1_000_000_000, 1),
        // A period of v units is a frequency of 1/v per unit.
        Unit::Ps => Ratio::new(v.1, v.0)?.mul(1_000_000_000_000, 1),
        Unit::Ns => Ratio::new(v.1, v.0)?.mul(1_000_000_000, 1),
        Unit::Us => Ratio::new(v.1, v.0)?.mul(1_000_000, 1),
        Unit::Ms => Ratio::new(v.1, v.0)?.mul(1_000, 1),
        Unit::S => Ratio::new(v.1, v.0),
    }
}

/// The level named in a profile.
fn level(name: &str) -> Option<Level> {
    match name {
        "model" => Some(Level::Model),
        "rtl" => Some(Level::Rtl),
        _ => None,
    }
}

/// A name for a clock written inline: `osc / 12`.
fn clock_text(e: &ast::Expr) -> String {
    match &e.kind {
        A::Quantity { text, unit, .. } => format!("{text} {unit:?}"),
        A::Path(p) => p.dotted(),
        A::Int { value, .. } => value.to_string(),
        A::Binary(BinaryOp::Div, a, b) => format!("{} / {}", clock_text(a), clock_text(b)),
        A::Binary(BinaryOp::Mul, a, b) => format!("{} * {}", clock_text(a), clock_text(b)),
        _ => "clock".into(),
    }
}

struct Sys {
    module: ModuleId,
    clocks: Vec<(String, Ratio)>,
    clock_names: HashMap<String, u32>,
    insts: Vec<hir::SystemComponent>,
    inst_names: HashMap<String, u32>,
}

impl Checker<'_> {
    pub(super) fn check_system(&mut self, sys_id: DefId) -> Option<hir::SystemInfo> {
        let module = self.prog.item(sys_id).module;
        let scope = Scope {
            module,
            isa: None,
            bus: None,
        };
        let decl = self.prog.system_decl(sys_id);
        let mut ok = true;
        let mut sys = Sys {
            module,
            clocks: Vec::new(),
            clock_names: HashMap::new(),
            insts: Vec::new(),
            inst_names: HashMap::new(),
        };

        // Clocks, in order: a derived clock names earlier ones.
        for item in &decl.items {
            let SystemItem::Clock(c) = item else { continue };
            let e = match &c.source {
                ClockSource::Frequency(e) | ClockSource::Derived(e) => e,
            };
            let Some(f) = self.clock_freq(&sys, e) else {
                ok = false;
                continue;
            };
            if sys
                .clock_names
                .insert(c.name.name.clone(), sys.clocks.len() as u32)
                .is_some()
            {
                self.error(
                    module,
                    "E1049",
                    c.name.span,
                    format!("duplicate clock `{}`", c.name.name),
                );
                ok = false;
            }
            sys.clocks.push((c.name.name.clone(), f));
        }

        // Component instances and their clocks.
        for item in &decl.items {
            let SystemItem::Component(ci) = item else {
                continue;
            };
            let comp = match &ci.component.kind {
                ast::TypeKind::Named(p, args) if args.is_empty() && p.segments.len() == 1 => {
                    self.lookup_item(scope, &p.segments[0].name)
                }
                _ => None,
            };
            let Some(comp) = comp.filter(|d| self.prog.item(*d).kind == ItemKind::Component) else {
                self.error(
                    module,
                    "E1049",
                    ci.component.span,
                    "expected a (non-generic) component",
                );
                ok = false;
                continue;
            };
            let Some(cs) = self.comps.get(&comp).cloned() else {
                // The component itself has errors.
                ok = false;
                continue;
            };
            let clock_ports: Vec<u32> = cs
                .ports
                .iter()
                .enumerate()
                .filter(|(_, p)| matches!(p.kind, PortKind::Clock))
                .map(|(i, _)| i as u32)
                .collect();
            let mut clocks = Vec::new();
            if let Some(b) = &ci.clocks {
                let named = b.bindings.iter().any(|a| a.name.is_some());
                if named && b.bindings.iter().any(|a| a.name.is_none()) {
                    self.error(
                        module,
                        "E1049",
                        b.span,
                        "bind clocks all by name or all by position",
                    );
                    ok = false;
                    continue;
                }
                let want = clock_ports.len().max(1);
                if b.bindings.len() != want {
                    let msg = format!(
                        "component `{}` has {} clock{}, but {} {} bound",
                        self.prog.item_name(comp),
                        want,
                        if want == 1 { "" } else { "s" },
                        b.bindings.len(),
                        if b.bindings.len() == 1 { "is" } else { "are" }
                    );
                    self.error(module, "E1049", b.span, msg);
                    ok = false;
                    continue;
                }
                for (i, a) in b.bindings.iter().enumerate() {
                    let port = match &a.name {
                        Some(n) => match clock_ports
                            .iter()
                            .find(|p| cs.ports[**p as usize].name == n.name)
                        {
                            Some(p) => Some(*p),
                            None => {
                                self.error(
                                    module,
                                    "E1049",
                                    n.span,
                                    format!(
                                        "`{}` is not a clock of `{}`",
                                        n.name,
                                        self.prog.item_name(comp)
                                    ),
                                );
                                ok = false;
                                continue;
                            }
                        },
                        None => clock_ports.get(i).copied(),
                    };
                    let Some(ck) = self.clock_ref(&mut sys, &a.value) else {
                        ok = false;
                        continue;
                    };
                    clocks.push((port, ck));
                }
            } else if self.component_ticks(comp) {
                let msg = format!(
                    "component `{}` has an `on tick` handler; bind its clock with `@ <clock>`",
                    self.prog.item_name(comp)
                );
                self.error(module, "E1049", ci.span, msg);
                ok = false;
            }
            if sys
                .inst_names
                .insert(ci.name.name.clone(), sys.insts.len() as u32)
                .is_some()
            {
                self.error(
                    module,
                    "E1049",
                    ci.name.span,
                    format!("duplicate component `{}`", ci.name.name),
                );
                ok = false;
            }
            sys.insts.push(hir::SystemComponent {
                name: ci.name.name.clone(),
                component: comp,
                clocks,
            });
        }

        // Buses.
        let mut buses = Vec::new();
        let mut attached: HashMap<hir::InstPort, String> = HashMap::new();
        for item in &decl.items {
            let SystemItem::Bus(bi) = item else { continue };
            match self.check_bus_inst(scope, &mut sys, bi, &mut attached) {
                Some(b) => buses.push(b),
                None => ok = false,
            }
        }

        // Wires and profiles.
        let mut wires: Vec<hir::SystemWire> = Vec::new();
        let mut profiles = Vec::new();
        for item in &decl.items {
            match item {
                SystemItem::Wire {
                    target,
                    source,
                    span,
                } => match self.check_wire(scope, &sys, target, source, *span, &wires) {
                    Some(w) => wires.push(w),
                    None => ok = false,
                },
                SystemItem::Profile(p) => {
                    let mut default = Level::Model;
                    let mut overrides = Vec::new();
                    let mut direct_memory = false;
                    for e in &p.entries {
                        if e.target.segments.len() == 1 && e.target.segments[0].name == "memory" {
                            match e.level.name.as_str() {
                                "direct" => direct_memory = true,
                                "bus" => direct_memory = false,
                                other => {
                                    self.error(
                                        module,
                                        "E1049",
                                        e.level.span,
                                        format!("unknown memory access `{other}` (expected `direct` or `bus`)"),
                                    );
                                    ok = false;
                                }
                            }
                            continue;
                        }
                        let Some(lv) = level(&e.level.name) else {
                            self.error(
                                module,
                                "E1049",
                                e.level.span,
                                format!(
                                    "unknown level `{}` (expected `model` or `rtl`)",
                                    e.level.name
                                ),
                            );
                            ok = false;
                            continue;
                        };
                        let first = &e.target.segments[0].name;
                        if e.target.segments.len() == 1 && first == "default" {
                            default = lv;
                        } else if sys.inst_names.contains_key(first) {
                            overrides.push((e.target.dotted(), lv));
                        } else {
                            self.error(
                                module,
                                "E1049",
                                e.target.span,
                                format!("`{first}` is not a component of this system"),
                            );
                            ok = false;
                        }
                    }
                    profiles.push(hir::Profile {
                        name: p.name.name.clone(),
                        direct_memory,
                        default,
                        overrides,
                    });
                }
                _ => {}
            }
        }

        // The time base: the largest unit in which every period is an
        // integer. Periods are hz_den / hz_num seconds, so the unit is
        // gcd(hz_den...) / lcm(hz_num...).
        let mut base_num: u128 = 0;
        let mut base_den: u128 = 1;
        for (_, f) in &sys.clocks {
            base_num = gcd(base_num, f.1);
            let g = gcd(base_den, f.0);
            match (base_den / g).checked_mul(f.0) {
                Some(v) => base_den = v,
                None => {
                    self.error(
                        module,
                        "E1049",
                        decl.name.span,
                        "the clocks of this system have no common time unit that fits 128 bits",
                    );
                    return None;
                }
            }
        }
        if base_num == 0 {
            base_num = 1;
        }
        let mut clocks = Vec::new();
        for (name, f) in &sys.clocks {
            // period / base = (f.1 / f.0) * (base_den / base_num)
            let p = (f.1 / base_num).checked_mul(base_den / f.0);
            let Some(p) = p.and_then(|p| u64::try_from(p).ok()) else {
                let msg =
                    format!("clock `{name}` has a period too long for the system's time unit");
                self.error(module, "E1049", decl.name.span, msg);
                return None;
            };
            clocks.push(hir::SystemClock {
                name: name.clone(),
                hz_num: f.0,
                hz_den: f.1,
                period: p,
            });
        }

        if !ok {
            return None;
        }
        Some(hir::SystemInfo {
            def: sys_id,
            name: decl.name.name.clone(),
            base_num,
            base_den,
            clocks,
            components: sys.insts,
            buses,
            wires,
            profiles,
        })
    }

    fn component_ticks(&self, comp: DefId) -> bool {
        self.components
            .iter()
            .find(|c| c.def == comp)
            .and_then(|c| c.model.as_ref())
            .is_some_and(|m| {
                m.handlers
                    .iter()
                    .any(|h| matches!(h.kind, HandlerKind::Tick { .. }))
            })
    }

    /// A frequency expression: a quantity, an earlier clock, or either
    /// multiplied or divided by an integer.
    fn clock_freq(&mut self, sys: &Sys, e: &ast::Expr) -> Option<Ratio> {
        let bad = |ck: &mut Checker, span: Span| {
            ck.error_with(
                sys.module,
                Diagnostic::error("E1049", span, "expected a clock frequency")
                    .with_note("a clock is a frequency (`8 MHz`), a period (`70 ns`), or another clock times or divided by an integer (`bus_clk / 4`)"),
            );
            None
        };
        match &e.kind {
            A::Quantity { text, unit, .. } => match quantity_hz(text, *unit) {
                Some(f) => Some(f),
                None => bad(self, e.span),
            },
            A::Path(p) if p.segments.len() == 1 => match sys.clock_names.get(&p.segments[0].name) {
                Some(i) => Some(sys.clocks[*i as usize].1),
                None => {
                    self.error(
                        sys.module,
                        "E1049",
                        e.span,
                        format!("unknown clock `{}`", p.segments[0].name),
                    );
                    None
                }
            },
            A::Binary(op @ (BinaryOp::Div | BinaryOp::Mul), a, b) => {
                let f = self.clock_freq(sys, a)?;
                let A::Int { value, .. } = b.kind else {
                    return bad(self, b.span);
                };
                let r = match op {
                    BinaryOp::Div => f.mul(1, value),
                    _ => f.mul(value, 1),
                };
                match r {
                    Some(r) => Some(r),
                    None => bad(self, e.span),
                }
            }
            _ => bad(self, e.span),
        }
    }

    /// A clock in a binding: a name, or an expression that makes a new
    /// anonymous clock.
    fn clock_ref(&mut self, sys: &mut Sys, e: &ast::Expr) -> Option<u32> {
        if let A::Path(p) = &e.kind {
            if p.segments.len() == 1 {
                if let Some(i) = sys.clock_names.get(&p.segments[0].name) {
                    return Some(*i);
                }
            }
        }
        let f = self.clock_freq(sys, e)?;
        let name = clock_text(e);
        if let Some(i) = sys.clocks.iter().position(|(n, _)| *n == name) {
            return Some(i as u32);
        }
        sys.clocks.push((name, f));
        Some(sys.clocks.len() as u32 - 1)
    }

    /// `inst.port` naming a wire port of an instance: its index and type,
    /// and whether it is an output.
    fn inst_wire_port(&mut self, sys: &Sys, p: &ast::Path) -> Option<(hir::InstPort, Ty, bool)> {
        let [inst, port] = p.segments.as_slice() else {
            self.error(sys.module, "E1049", p.span, "expected `<component>.<port>`");
            return None;
        };
        let Some(&ii) = sys.inst_names.get(&inst.name) else {
            self.error(
                sys.module,
                "E1049",
                inst.span,
                format!("`{}` is not a component of this system", inst.name),
            );
            return None;
        };
        let comp = sys.insts[ii as usize].component;
        let cs = self.comps.get(&comp).cloned()?;
        let Some(pi) = cs.ports.iter().position(|q| q.name == port.name) else {
            self.error(
                sys.module,
                "E1049",
                port.span,
                format!(
                    "`{}` has no port `{}`",
                    self.prog.item_name(comp),
                    port.name
                ),
            );
            return None;
        };
        match &cs.ports[pi].kind {
            PortKind::In(t) | PortKind::AsyncIn(t) => Some(((ii, pi as u32), t.clone(), false)),
            PortKind::Out(t) => Some(((ii, pi as u32), t.clone(), true)),
            _ => {
                self.error(
                    sys.module,
                    "E1049",
                    port.span,
                    format!(
                        "`{}.{}` is not a wire port (`in` or `out`)",
                        inst.name, port.name
                    ),
                );
                None
            }
        }
    }

    /// `wire inst.in_port = other.out_port;` or `= <constant>;`.
    fn check_wire(
        &mut self,
        scope: Scope,
        sys: &Sys,
        target: &ast::Path,
        source: &ast::Expr,
        span: Span,
        done: &[hir::SystemWire],
    ) -> Option<hir::SystemWire> {
        let module = sys.module;
        let (tp, tty, t_out) = self.inst_wire_port(sys, target)?;
        if t_out {
            self.error(
                module,
                "E1049",
                target.span,
                format!(
                    "`{}` is an output; a wire drives an input port",
                    target.dotted()
                ),
            );
            return None;
        }
        if done.iter().any(|w| w.target == tp) {
            self.error(
                module,
                "E1049",
                span,
                format!("`{}` is already driven by another wire", target.dotted()),
            );
            return None;
        }
        // An output port of another instance: `inst.port`, which parses as a
        // field of a path.
        let as_path = match &source.kind {
            A::Path(p) => Some(p.clone()),
            A::Field(base, f) => match &base.kind {
                A::Path(b) if b.segments.len() == 1 => Some(ast::Path {
                    segments: vec![b.segments[0].clone(), f.clone()],
                    span: source.span,
                }),
                _ => None,
            },
            _ => None,
        };
        if let Some(p) = &as_path {
            if p.segments.len() == 2 {
                let (sp, sty, s_out) = self.inst_wire_port(sys, p)?;
                if !s_out {
                    self.error(
                        module,
                        "E1049",
                        source.span,
                        format!(
                            "`{}` is an input; a wire is driven by an output port",
                            p.dotted()
                        ),
                    );
                    return None;
                }
                if sty != tty {
                    let msg = format!(
                        "`{}` is `{}` but `{}` is `{}`",
                        p.dotted(),
                        self.display(&sty),
                        target.dotted(),
                        self.display(&tty)
                    );
                    self.error(module, "E1049", source.span, msg);
                    return None;
                }
                return Some(hir::SystemWire {
                    target: tp,
                    source: hir::WireSource::Port(sp),
                });
            }
        }
        // A constant strap.
        let empty = HashMap::new();
        let v = self.eval_const(scope, &empty, source)?;
        let value = match (v, &tty) {
            (crate::consteval::ConstVal::Bool(b), Ty::Bool) => b as i128,
            (crate::consteval::ConstVal::Int(n), Ty::UInt(w))
                if n >= 0 && (*w >= 127 || n < (1i128 << w)) =>
            {
                n
            }
            _ => {
                let msg = format!(
                    "this constant does not fit `{}` (`{}`)",
                    target.dotted(),
                    self.display(&tty)
                );
                self.error(module, "E1049", source.span, msg);
                return None;
            }
        };
        Some(hir::SystemWire {
            target: tp,
            source: hir::WireSource::Const(value),
        })
    }

    /// `inst.port` naming a bus port of an instance.
    fn inst_port(&mut self, sys: &Sys, p: &ast::Path) -> Option<(hir::InstPort, DefId, u32)> {
        let [inst, port] = p.segments.as_slice() else {
            self.error(
                sys.module,
                "E1049",
                p.span,
                "expected `<component>.<bus port>`",
            );
            return None;
        };
        let Some(&ii) = sys.inst_names.get(&inst.name) else {
            self.error(
                sys.module,
                "E1049",
                inst.span,
                format!("`{}` is not a component of this system", inst.name),
            );
            return None;
        };
        let comp = sys.insts[ii as usize].component;
        let cs = self.comps.get(&comp).cloned()?;
        let Some(pi) = cs.ports.iter().position(|q| q.name == port.name) else {
            self.error(
                sys.module,
                "E1049",
                port.span,
                format!(
                    "`{}` has no port `{}`",
                    self.prog.item_name(comp),
                    port.name
                ),
            );
            return None;
        };
        let PortKind::Bus { bus, role } = cs.ports[pi].kind else {
            self.error(
                sys.module,
                "E1049",
                port.span,
                format!("`{}.{}` is not a bus port", inst.name, port.name),
            );
            return None;
        };
        Some(((ii, pi as u32), bus, role))
    }

    fn check_bus_inst(
        &mut self,
        scope: Scope,
        sys: &mut Sys,
        bi: &ast::BusInst,
        attached: &mut HashMap<hir::InstPort, String>,
    ) -> Option<hir::SystemBus> {
        let module = sys.module;
        let bus = match &bi.protocol.kind {
            ast::TypeKind::Named(p, args) if args.is_empty() && p.segments.len() == 1 => {
                self.lookup_item(scope, &p.segments[0].name)
            }
            _ => None,
        };
        let Some(bus) = bus.filter(|d| self.prog.item(*d).kind == ItemKind::Bus) else {
            self.error(module, "E1049", bi.protocol.span, "expected a bus protocol");
            return None;
        };
        let info = self.buses.get(&bus).cloned()?;
        let Some(clock) = bi.clock.as_ref() else {
            self.error(
                module,
                "E1049",
                bi.name.span,
                format!(
                    "bus `{}` needs a clock: `bus {}: {} @ <clock> {{ ... }}`",
                    bi.name.name, bi.name.name, info.name
                ),
            );
            return None;
        };
        let Some(&clock) = sys.clock_names.get(&clock.name) else {
            self.error(
                module,
                "E1049",
                clock.span,
                format!("unknown clock `{}`", clock.name),
            );
            return None;
        };

        // The decoder reads `addr`, and `space` when ranges name one.
        let space_enum: Option<DefId> = info.transactions.iter().find_map(|t| {
            t.params.iter().find_map(|(n, ty)| match ty {
                Ty::Enum(d) if n == "space" => Some(*d),
                _ => None,
            })
        });

        let mut ok = true;
        let mut masters = Vec::new();
        let mut slaves: Vec<(hir::InstPort, Vec<hir::AddressRange>)> = Vec::new();
        let mut bridges = Vec::new();
        for b in &bi.bindings {
            match b {
                BusBinding::Master(p) | BusBinding::Slave { port: p, .. } => {
                    let is_master = matches!(b, BusBinding::Master(_));
                    let Some((ip, pbus, role)) = self.inst_port(sys, p) else {
                        ok = false;
                        continue;
                    };
                    if pbus != bus {
                        let msg = format!(
                            "`{}` is a port of bus `{}`, not `{}`",
                            p.dotted(),
                            self.prog.item_name(pbus),
                            info.name
                        );
                        self.error(module, "E1049", p.span, msg);
                        ok = false;
                        continue;
                    }
                    if is_master != (role == 0) {
                        let msg = format!(
                            "`{}` is in role `{}`; `master` attaches the initiating role `{}`, `slave` a responding one",
                            p.dotted(),
                            info.roles[role as usize],
                            info.roles[0]
                        );
                        self.error(module, "E1049", p.span, msg);
                        ok = false;
                        continue;
                    }
                    if let Some(other) = attached.insert(ip, bi.name.name.clone()) {
                        self.error(
                            module,
                            "E1049",
                            p.span,
                            format!("`{}` is already attached to bus `{other}`", p.dotted()),
                        );
                        ok = false;
                        continue;
                    }
                    if is_master {
                        masters.push(ip);
                        continue;
                    }
                    let BusBinding::Slave { ranges, .. } = b else {
                        unreachable!()
                    };
                    let mut rs = Vec::new();
                    for r in ranges {
                        let space = match &r.space {
                            None => None,
                            Some(s) => {
                                let Some(e) = space_enum.and_then(|d| self.enum_info(d)) else {
                                    let msg = format!("`{}` names an address space, but the transactions of `{}` have no `space` enum parameter", s.name, info.name);
                                    self.error(module, "E1049", s.span, msg);
                                    ok = false;
                                    continue;
                                };
                                match e
                                    .variants
                                    .iter()
                                    .find(|(n, _)| n.eq_ignore_ascii_case(&s.name))
                                {
                                    Some((_, v)) => Some(*v),
                                    None => {
                                        let names: Vec<&str> =
                                            e.variants.iter().map(|(n, _)| n.as_str()).collect();
                                        self.error(
                                            module,
                                            "E1049",
                                            s.span,
                                            format!(
                                                "unknown space `{}` (one of {})",
                                                s.name,
                                                names.join(", ")
                                            ),
                                        );
                                        ok = false;
                                        continue;
                                    }
                                }
                            }
                        };
                        let A::Range { lo, hi, inclusive } = &r.range.kind else {
                            self.error(
                                module,
                                "E1049",
                                r.range.span,
                                "expected an address range `lo..=hi`",
                            );
                            ok = false;
                            continue;
                        };
                        let empty = HashMap::new();
                        let lo_v = self.eval_const(scope, &empty, lo).and_then(|v| v.as_int());
                        let hi_v = self.eval_const(scope, &empty, hi).and_then(|v| v.as_int());
                        let (Some(lo_v), Some(mut hi_v)) = (lo_v, hi_v) else {
                            ok = false;
                            continue;
                        };
                        if !inclusive {
                            hi_v -= 1;
                        }
                        if lo_v < 0 || hi_v < lo_v {
                            self.error(module, "E1049", r.range.span, "empty address range");
                            ok = false;
                            continue;
                        }
                        let new = hir::AddressRange {
                            space,
                            lo: lo_v as u128,
                            hi: hi_v as u128,
                        };
                        let clash = slaves
                            .iter()
                            .flat_map(|(_, v)| v.iter())
                            .chain(rs.iter())
                            .any(|o: &hir::AddressRange| {
                                (o.space.is_none() || new.space.is_none() || o.space == new.space)
                                    && o.lo <= new.hi
                                    && new.lo <= o.hi
                            });
                        if clash {
                            self.error(
                                module,
                                "E1049",
                                r.span,
                                "this range overlaps another slave's on the same bus",
                            );
                            ok = false;
                            continue;
                        }
                        rs.push(new);
                    }
                    // A transaction without `addr` is not decoded: it goes to
                    // the slave that handles it (an interrupt acknowledge).
                    if !rs.is_empty() {
                        for t in &info.transactions {
                            let Some((_, aty)) = t.params.iter().find(|(n, _)| n == "addr") else {
                                continue;
                            };
                            if !matches!(aty, Ty::UInt(_)) {
                                let msg = format!("address ranges decode `addr` as an unsigned number, but `{}.{}` declares it otherwise", info.name, t.name);
                                self.error(module, "E1049", p.span, msg);
                                ok = false;
                            }
                            if rs.iter().any(|r| r.space.is_some())
                                && !t.params.iter().any(|(n, _)| n == "space")
                            {
                                let msg = format!("these ranges name address spaces, but `{}.{}` has no `space` parameter", info.name, t.name);
                                self.error(module, "E1049", p.span, msg);
                                ok = false;
                            }
                        }
                    }
                    slaves.push((ip, rs));
                }
                BusBinding::Bridge(p) => bridges.push(p.dotted()),
                BusBinding::Line { name, .. } => {
                    if !info.wires.iter().any(|w| w.name == name.name) {
                        self.error(
                            module,
                            "E1049",
                            name.span,
                            format!("bus `{}` has no wire `{}`", info.name, name.name),
                        );
                        ok = false;
                    }
                }
            }
        }
        if !ok {
            return None;
        }
        Some(hir::SystemBus {
            name: bi.name.name.clone(),
            bus,
            clock,
            masters,
            slaves,
            bridges,
        })
    }
}
