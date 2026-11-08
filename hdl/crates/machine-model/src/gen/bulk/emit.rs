//! The kernels' Rust: the dry element, its `while` condition, and the
//! element as `rt::bulk::run` sees it.

use std::fmt::Write as _;

use machine_sema::hir::{self, Expr, ExprKind as H};
use machine_sema::program::DefId;

use super::val::ty_bits;
use super::{Kernel, Shape};
use crate::gen::{ident, Gen, R};

impl Gen<'_> {
    /// The bytes past which the isa's split charge (`split => ...`) repeats:
    /// the address read only through `address & (2^b - 1)` (2^b, the
    /// largest such), not read (1), no split charge (1); 0 otherwise (a
    /// kernel's elements are then checked one by one).
    pub(crate) fn split_period(&self) -> u64 {
        let Some(t) = &self.isa.split_cycles else { return 1 };
        let Some(&addr) = t.params.first() else { return 0 };
        let is_addr = |x: &Expr| matches!(x.kind, H::Local(l) if l == addr);
        let mut uses = 0usize;
        let mut masked = 0usize;
        let mut period = 1u64;
        let mut bad = false;
        hir::visit_expr(&t.expr, &mut |_| {}, &mut |x| {
            if is_addr(x) {
                uses += 1;
            }
            if let H::Binary(machine_syntax::ast::BinaryOp::BitAnd, a, b) = &x.kind {
                let (v, k) = if is_addr(a) { (a, b) } else { (b, a) };
                if is_addr(v) {
                    match k.kind {
                        H::Int(m) if m >= 0 && m < 1 << 32 && (m as u64 + 1).is_power_of_two() => {
                            masked += 1;
                            period = period.max(m as u64 + 1);
                        }
                        _ => bad = true,
                    }
                }
            }
        });
        if bad || uses != masked {
            return 0;
        }
        period
    }

    /// Instruction `i`'s kernel: its dry code (`probe_<name>`), the element
    /// as the runtime's kernel loop sees it (`Bulk_<name>`, an
    /// `rt::bulk::Element`, in `items`), and `bulk_<name>`, which runs it.
    /// (Every kernel's stops are in `bulk_stops`: the dry fns are shared.)
    pub(crate) fn emit_bulk_kernel(&self, i: &hir::Instruction, k: &Kernel, out: &mut String, items: &mut String) -> R<()> {
        let n = ident(&i.name);
        let Some(sem) = &i.semantics else { return Ok(()) };
        let ops = Self::ops_struct_name(i);
        // The dry element.
        let cx = crate::gen::Cx { locals: &i.locals, fallible: true, recv: "self", ops: Some(i), comp: None, opt: false };
        let body = self.lowering_dry(|g| g.block_stmts(&cx, sem))?;
        let _ = write!(out, "    /// `{name}`'s element, dry: its accesses noted (`probe_access`), not made.\n    #[allow(unused_variables, unused_parens, unreachable_code, clippy::all)]\n    fn probe_{n}(&mut self, ops: {ops}) -> Result<(), Fault> {{\n{body}Ok(())\n}}\n", name = i.name);
        // The dry element as the machine is, the registers it may write kept.
        let reg = |d: &DefId| -> String { format!("r.{}", ident(&self.regs[d].1.name)) };
        let mut save = String::new();
        let mut restore = String::new();
        for (x, d) in k.written.iter().enumerate() {
            let _ = writeln!(save, "        let s{x} = self.{}.clone();", reg(d));
            let _ = writeln!(restore, "        self.{} = s{x};", reg(d));
        }
        let mi = self.isa.spaces.iter().position(|s| Some(s.def) == self.isa.istream.as_ref().map(|i| i.space)).unwrap_or(0) as u8;
        let (kind, writes): (&str, &[bool]) = match k.shape {
            Shape::Fill => ("Fill", &[true]),
            Shape::Copy => ("Copy", &[false, true]),
            Shape::Compare => ("Read", &[false, false]),
            _ => ("Read", &[false]),
        };
        let expect: Vec<String> = writes.iter().map(|w| format!("({}u8, {})", (mi << 1) | *w as u8, k.bits)).collect();
        let moved: Vec<String> = k.moving.iter().map(|(d, _)| format!("self.{} as u64", reg(d))).collect();
        let value = if k.shape == Shape::Fill { "self.probe_acc[0].3" } else { "0" };
        let _ = write!(
            out,
            "    /// `{name}`'s element, dry, as the machine is (`rt::bulk::Element::dry`).
    #[allow(clippy::all)]
    fn bulk_{n}_dry(&mut self) -> Option<rt::bulk::Dry> {{
{save}        self.probe_bad = false;
        self.probe_n = 0;
        let r = self.probe_{n}({ops} {{}});
        let mut moved = [0u64; rt::bulk::MOVING];
        for (x, v) in [{moved}].into_iter().enumerate() {{ moved[x] = v; }}
{restore}        let expect = [{expect}];
        if r.is_err() || self.probe_bad || self.probe_n as usize != expect.len() {{ return None; }}
        let mut addrs = [0u64; 2];
        for (x, e) in expect.iter().enumerate() {{
            if (self.probe_acc[x].0, self.probe_acc[x].1) != *e {{ return None; }}
            addrs[x] = self.probe_acc[x].2;
        }}
        Some(rt::bulk::Dry {{ addrs, value: {value}, moved }})
    }}
    /// `{name}`'s elements after this one as a block (`repeat ... bulk`):
    /// up to `max`, in whole periods of `per` (`rt::bulk::run`). The
    /// elements made.
    pub fn bulk_{n}(&mut self, max: u64, per: u64) -> u64 {{
        rt::bulk::run(&mut Bulk_{n}(self), max, per)
    }}
",
            name = i.name,
            moved = moved.join(", "),
            expect = expect.join(", "),
        );
        // The `while` condition, on what an element reads.
        let cont = match &k.cond {
            Some(c) => {
                let params: Vec<String> = k.loads.iter().enumerate().map(|(x, t)| format!("l{x}_ld{}: {t}", x + 1)).collect();
                let args: Vec<String> = k.loads.iter().enumerate().map(|(x, t)| format!("read[{x}] as {t}")).collect();
                let _ = write!(out, "    /// `{name}`'s `while` condition, on the values an element reads.\n    #[allow(unused_variables, unused_parens, clippy::all)]\n    fn bulk_{n}_cont(&mut self, {}) -> Result<bool, Fault> {{\n        Ok({c})\n    }}\n", params.join(", "), name = i.name);
                format!("self.0.bulk_{n}_cont({}).unwrap_or(false)", args.join(", "))
            }
            None => "true".into(),
        };
        // The element, for the kernel loop.
        let mut widths = vec!["0".to_string(); 4];
        let mut full = vec!["0".to_string(); 4];
        let mut get = String::new();
        let mut place = String::new();
        let mut commit = String::new();
        let cx0 = crate::gen::Cx { locals: &[], fallible: true, recv: "self.0", ops: None, comp: None, opt: false };
        for (x, (d, w)) in k.moving.iter().enumerate() {
            let ty = &self.regs[d].1.ty;
            let rty = self.rty(ty)?;
            widths[x] = w.to_string();
            full[x] = ty_bits(ty).to_string();
            let _ = write!(get, "v[{x}] = self.0.{} as u64; ", reg(d));
            let _ = writeln!(place, "        self.0.{} = v[{x}] as {rty};", reg(d));
            let at = Expr { kind: H::Register(*d), ty: ty.clone(), span: sem.span };
            let _ = writeln!(commit, "        {}", self.assign(&cx0, &at, &format!("v[{x}] as {rty}"))?);
        }
        let isa = ident(&self.isa.name);
        let _ = write!(
            items,
            "/// `{name}`'s element, as its kernel runs it (`repeat ... bulk`).
#[allow(non_camel_case_types)]
pub struct Bulk_{n}<'a, B: Bus>(pub &'a mut {isa}<B>);
impl<B: Bus> rt::bulk::Element for Bulk_{n}<'_, B> {{
    const SHAPE: rt::bulk::Shape = rt::bulk::Shape {{
        kind: rt::bulk::Kind::{kind},
        moving: {nm},
        widths: [{widths}],
        full: [{full}],
        accesses: {accs},
        writes: [{w0}, {w1}],
        bytes: {bytes},
        cond: {has_cond},
        cost_period: {cost_period},
    }};
    fn moving(&mut self) -> [u64; rt::bulk::MOVING] {{
        let mut v = [0u64; rt::bulk::MOVING];
        {get}
        v
    }}
    fn place(&mut self, v: &[u64; rt::bulk::MOVING]) {{
{place}    }}
    #[allow(unused_parens, clippy::all)]
    fn commit(&mut self, v: &[u64; rt::bulk::MOVING]) {{
{commit}    }}
    fn dry(&mut self) -> Option<rt::bulk::Dry> {{
        self.0.bulk_{n}_dry()
    }}
    #[allow(unused_variables)]
    fn cont(&mut self, read: [u128; 2]) -> bool {{
        {cont}
    }}
    fn cost(&mut self, a: u64, n: u64, write: bool) -> Option<(u32, u128)> {{
        self.0.bulk_key(a, n, write)
    }}
    fn peek(&mut self, a: u64, n: u64) -> Option<u128> {{
        self.0.bus.direct_peek(a, n)
    }}
    fn poke(&mut self, a: u64, n: u64, v: u128) -> bool {{
        self.0.bus.direct_poke(a, n, v)
    }}
    fn span(&mut self, a: u64, len: u64, write: bool) -> Option<(u32, usize)> {{
        self.0.bus.direct_span(a, len, write)
    }}
    fn cells(&mut self, mem: u32) -> &mut [u8] {{
        self.0.bus.direct_cells(mem)
    }}
}}
",
            name = i.name,
            nm = k.moving.len(),
            widths = widths.join(", "),
            full = full.join(", "),
            accs = writes.len(),
            w0 = writes[0],
            w1 = writes.get(1).copied().unwrap_or(false),
            bytes = k.bits / 8,
            has_cond = k.cond.is_some(),
            cost_period = self.split_period(),
        );
        Ok(())
    }

    /// What the kernels share: `bulk_key` (what an access costs an element)
    /// and the dry code's `probe_access`.
    pub(crate) fn emit_bulk_common(&self, out: &mut String) {
        let split = if self.isa.split_cycles.is_some() { "self.split_cycles(a as u32, n as u32)" } else { "0" };
        let _ = write!(
            out,
            "    /// What an access of `n` bytes at `a` costs a kernel's element (the\n    /// split cycles, the direct memory's time): None where it is not direct.\n    #[allow(dead_code)]\n    fn bulk_key(&mut self, a: u64, n: u64, write: bool) -> Option<(u32, u128)> {{\n        let t = self.bus.direct_cost(a, n, write)?;\n        Some(({split}, t))\n    }}\n    /// The dry code's access (`kind`: the space's index, doubled, plus 1 for\n    /// a write): noted; a read reads 0.\n    #[allow(dead_code)]\n    fn probe_access(&mut self, kind: u8, a: u64, bits: u32, v: u128) -> u128 {{\n        if (self.probe_n as usize) < 4 {{ self.probe_acc[self.probe_n as usize] = (kind, bits, a, v); }}\n        self.probe_n = self.probe_n.saturating_add(1);\n        0\n    }}\n"
        );
    }
}
