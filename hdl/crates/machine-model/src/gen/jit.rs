//! The JIT core (`core: jit;`, SPEC 12, "Cores"): the trace core, with runs
//! entered often compiled to host code (`machine-jit`).
//!
//! A compiled run is a host function that makes the run's instructions
//! after its first, one after another, each as `step` would: the run's
//! checks (as the trace core makes them: epoch, pc, code writes), then
//! `step`'s prologue (reset, interrupts), the state set as decoding left it
//! and the fetch charged, the instruction's handler called directly (no
//! dispatch), `step`'s tail (faults, timing) and the devices caught up
//! (`Bus::after_step`, which `System::step` makes too). Anything else ends
//! the compiled run where it is, and stepping goes on as before.
//!
//! The machine is entered through `jit_burst`: one ordinary step, and when
//! that step entered a run, the rest of it compiled (once it has been
//! entered `HOT` times). `System::run_until` checks its stop condition
//! between bursts, so at most one run apart.

use std::fmt::Write as _;

use super::{ident, Gen, GenError, R};
use machine_sema::ty::Ty;

/// Entries into a run before it is compiled. A compile costs some 10-30 M
/// host instructions a step (the 386's), and a step compiled saves under
/// a hundred each time it runs: a run entered a few hundred times does
/// not pay for itself. At 4096, a DOS boot takes 44% fewer host
/// instructions than at 64 (the `jit` profile); at 16384 a game whose
/// frame loop runs 70 times a second (Xmas Lemmings) 11% fewer than at
/// 4096, and loops that run long lose nothing.
const HOT: u32 = 16384;

/// Steps a step of a run makes, compiled quickly (Cranelift's optimiser
/// off), before the run is compiled again optimised: the optimiser costs a
/// third more and saves a few host instructions a step.
const HOT2: u64 = 1_000_000;

/// The host instructions an outlined fn may take, inlined at a call where
/// what is known folds it (`Em::try_inline`). On the 386 benchmarks, 400
/// saves 5-6% of a run's host instructions for about 13% more compiling
/// (a memory read through a segment folds in real mode); larger budgets
/// compile more than they save.
const JIT_TRIAL: usize = 400;

impl Gen<'_> {
    pub(crate) fn jit_fields(&self) -> &'static str {
        if !self.jit {
            return "";
        }
        "    /// The JIT core: the code generator, whether the step's prologue left\n    /// the step to the instruction, and a fault to return.\n    pub jit: machine_jit::Engine,\n    pub jit_go: bool,\n    pub jit_err: Option<Fault>,\n    /// The instruction of the last step compiled code made and retired\n    /// itself (a pointer into its run), 0 when none since: `last` is set\n    /// from it when the code returns.\n    pub jit_last: u64,\n    /// The board's direct memory pointers (`Bus::jit_mem_ptrs`), as of\n    /// entering compiled code.\n    pub jit_ptrs: [usize; 8],\n    /// A fn compiled code called raised a fault, delivered: the run ends.\n    pub jit_pending: bool,\n    /// A compiled run's entry guard failed (the speculated registers\n    /// differ from what it was compiled for).\n    pub jit_guard_failed: bool,\n    /// The steps compiled code may make before it returns (a run whose\n    /// end leads to its start goes round while they last).\n    pub jit_left: u64,\n    /// A fault an outlined fn raised, for `jit_deliver_c` to deliver.\n    pub jit_fault: Option<Fault>,\n    /// `sync: quantum(...)`: when the devices are next caught up.\n    pub jit_sync_at: u128,\n    /// `repeat: bulk(n)`: the time as an element ended, a period of elements\n    /// before a kernel runs (its low half); the time the kernel's elements\n    /// took.\n    pub jit_bulk_mark: u64,\n    pub jit_bulk_dt: u128,\n    /// A kernel's dry code: it stopped (a path no kernel takes), and its\n    /// accesses (the kind, the bits, the address, a write's value).\n    pub probe_bad: bool,\n    pub probe_n: u8,\n    pub probe_acc: [(u8, u32, u64, u128); 4],\n"
    }

    pub(crate) fn jit_init(&self) -> &'static str {
        if !self.jit {
            return "";
        }
        "            jit: machine_jit::Engine::default(),\n            jit_go: false,\n            jit_err: None,\n            jit_last: 0,\n            jit_ptrs: [0; 8],\n            jit_pending: false,\n            jit_guard_failed: false,\n            jit_left: 0,\n            jit_fault: None,\n            jit_sync_at: 0,\n            jit_bulk_mark: 0,\n            jit_bulk_dt: 0,\n            probe_bad: false,\n            probe_n: 0,\n            probe_acc: [(0, 0, 0, 0); 4],\n"
    }

    /// The trace's fields for compiled code (inside `Trace`).
    pub(crate) fn jit_trace_fields(&self) -> &'static str {
        if !self.jit {
            return "";
        }
        "    /// Compiled code for the run after its first instruction, what each\n    /// of its steps was compiled with, and how often the run has been\n    /// entered.\n    pub code: machine_jit::Code,\n    pub jit_steps: Vec<JitStep>,\n    pub hits: u32,\n    /// Entries its guard turned away: past a few, it is compiled again\n    /// for the values then.\n    pub guard_fails: u32,\n    /// The steps its compiled code has made, and the tier it is compiled\n    /// at (0 quick; past `HOT2` steps a step, compiled again optimised).\n    pub exec: u64,\n    pub tier: u8,\n"
    }

    pub(crate) fn jit_trace_init(&self) -> &'static str {
        if !self.jit {
            return "";
        }
        "code: machine_jit::Code::default(), jit_steps: Vec::new(), hits: 0, guard_fails: 0, exec: 0, tier: 0, "
    }

    /// What a compiled step is compiled with (beside the trace types).
    pub(crate) fn jit_types(&self) -> &'static str {
        if !self.jit {
            return "";
        }
        "/// A compiled step's data: the step, and what checking and replaying it\n/// needs of its run (the pc it is at, its address, the run's page, stamp\n/// and slot, and whether it is the run's last).\n#[derive(Clone, Copy, Debug)]\npub struct JitStep {\n    pub step: TStep,\n    pub pc: u64,\n    pub phys: u64,\n    pub page: u32,\n    pub stamp: u32,\n    pub slot: u32,\n    pub last: bool,\n    /// The run's pc (where it starts).\n    pub start: u64,\n}\n"
    }

    /// The JIT core's methods (inside the machine's impl).
    pub(crate) fn emit_jit_fns(&mut self) -> R<()> {
        if !self.jit {
            return Ok(());
        }
        let ist = self.isa.istream.as_ref().unwrap();
        let sp = ident(&self.spaces[&ist.space].name);
        self.emit_jit_lowering()?;
        let mut shims = String::new();
        let mut fns = String::new();
        for i in self.isa.instructions.iter().filter(|i| !i.prefix) {
            let n = ident(&i.name);
            let _ = write!(
                shims,
                "    /// A compiled `{n}` step: as `step` makes it from a run.\n    extern \"C\" fn jit_step_{n}(m: *mut Self, j: *const JitStep) -> u32 {{\n        // SAFETY: compiled code calls this with the machine it was entered\n        // with and a step of one of its runs, both live for the call.\n        let (m, j) = unsafe {{ (&mut *m, &*j) }};\n        m.jit_step(j, |m, i| match i {{ Instr::{n}(ops) => m.run_{n}(ops), _ => Ok(()) }})\n    }}\n"
            );
            let _ = writeln!(fns, "            Instr::{n}(_) => Self::jit_step_{n} as *const () as usize,");
        }
        let sync_run = self.sync_run;
        let quantum = self.sync_quantum.unwrap_or(0);
        let sync_block = self.sync_block;
        let address = ident(&self.fns[&ist.address].name);
        let Some(plan) = self.trace.as_ref() else { return Ok(()) };
        let pc0n = ident(&self.regs[&plan.pc[0]].1.name);
        let _ = write!(
            self.out,
            r#"{shims}    /// The compiled step for `i`.
    fn jit_step_fn(i: &Instr) -> usize {{
        match i {{
{fns}        }}
    }}
    /// A compiled step: 0 when it was made and the run goes on, 1 when it
    /// was made and the run stops (an interrupt taken in its place, a
    /// fault, the run's end), 2 when it was not made.
    #[inline(always)]
    fn jit_step(&mut self, j: &JitStep, run: impl FnOnce(&mut Self, Instr) -> Result<(), Fault>) -> u32 {{
        if !self.jit_check(j) {{
            return 2;
        }}
        self.jit_last = 0;
        self.jit_go = false;
        let r = self.jit_prologue();
        if !self.jit_go {{
            self.bus.after_step();
            if let Err(f) = r {{ self.jit_err = Some(f); }}
            return 1;
        }}
        self.tr_rec_now = false;
        self.decoded = false;
        self.jit_restore(j);
        let phys = j.phys;
        self.tr_phys = phys;
        self.restarted = phys == self.last_start;
        self.last_start = phys;
        self.bus.{sp}_fetch_page(j.page, phys, j.step.len as u32);
        let r = run(self, j.step.instr);
        let r = self.jit_tail(phys, r);
        self.bus.after_step();
        if let Err(f) = r {{
            self.jit_err = Some(f);
            return 1;
        }}
        u32::from(self.halted || !self.tr_on)
    }}
    /// Compiles the run in `slot`.
    fn jit_compile(&mut self, slot: usize) {{
        if self.jit.count() >= machine_jit::LIMIT {{
            self.jit.clear();
            for t in self.traces.iter_mut().flatten() {{ t.code = machine_jit::Code::default(); }}
        }}
        let calls: Vec<machine_jit::Call> = match self.traces[slot].as_mut() {{
            Some(t) => {{
                let n = t.steps.len();
                t.jit_steps = t.steps.iter().enumerate().map(|(k, s)| JitStep {{
                    step: *s,
                    pc: t.pc.wrapping_add(s.off as u64),
                    phys: t.phys + s.off as u64,
                    page: t.page,
                    stamp: t.stamp,
                    slot: slot as u32,
                    last: k + 1 == n,
                    start: t.pc,
                }}).collect();
                t.jit_steps.iter().map(|j| machine_jit::Call {{
                    run: Self::jit_step_fn(&j.step.instr),
                    arg: j as *const JitStep as usize,
                }}).collect()
            }}
            None => return,
        }};
        let code = match self.traces[slot].as_ref() {{
            Some(t) => {{
                let steps = &t.jit_steps;
                let mut lowered = vec![false; steps.len()];
                let _ = &calls;
                let fetch: Vec<Option<u128>> = steps.iter().map(|j| self.bus.jit_fetch_time(j.page, j.phys, j.step.len as u32)).collect();
                let units: Vec<Option<(u64, u64, u128)>> = steps.iter().map(|j| self.bus.jit_fetch_units(j.page, j.phys, j.step.len as u32)).collect();
                // The speculated registers' values now (the run is compiled
                // for them), and the run's key.
                let spec = self.jit_spec_values();
                let key = t.key;
                // Entered at its first step only (`jit_burst`): past it, a
                // step is reached only from the one before.
                // The run keeps the time in the processor's cycles
                // (`MACHINE_JIT_FINE`: in the bus's units, as the bus does),
                // the devices' deadline as the cycles left before it.
                let bus = std::mem::offset_of!(Self, bus);
                self.jit.clock = B::jit_board()
                    .filter(|b| b.clock != 0 && b.clock < 1u128 << 64 && std::env::var_os("MACHINE_JIT_FINE").is_none())
                    .map(|b| machine_jit::Clock {{
                        now: bus + b.now,
                        scale: b.clock,
                        deadline: if {quantum}u128 != 0 {{ Some(std::mem::offset_of!(Self, jit_sync_at)) }} else {{ b.due.map(|d| bus + d) }},
                    }});
                self.jit.tier = t.tier;
                self.jit.compile_with(steps.len(), 1, |e, k| {{
                    let checked = k == 0 || lowered[k - 1];
                    // Only reached from a lowered step before it.
                    let follows = k >= 1 && lowered[k - 1];
                    lowered[k] = Self::jit_emit_step(e, &steps[k], k, checked, follows, fetch[k], units[k], &spec, &key);
                }})
            }}
            None => None,
        }};
        if let Some(t) = self.traces[slot].as_mut() {{
            t.code = machine_jit::Code::new(code);
        }}
    }}
    /// Entries into a run before it is compiled (`HOT` in the generator;
    /// `MACHINE_JIT_HOT` overrides it, for tuning).
    /// Steps a step of a run makes compiled at the quick tier before the
    /// run is compiled again optimised (`HOT2`; `MACHINE_JIT_HOT2`).
    fn jit_hot2() -> u64 {{
        static H: std::sync::OnceLock<u64> = std::sync::OnceLock::new();
        *H.get_or_init(|| std::env::var("MACHINE_JIT_HOT2").ok().and_then(|v| v.parse().ok()).unwrap_or({HOT2}))
    }}
    fn jit_hot() -> u32 {{
        static H: std::sync::OnceLock<u32> = std::sync::OnceLock::new();
        *H.get_or_init(|| std::env::var("MACHINE_JIT_HOT").ok().and_then(|v| v.parse().ok()).unwrap_or({HOT}))
    }}
    /// Runs `c`, compiled code of this machine's current run, from step
    /// `from`; the steps it made.
    fn jit_enter(&mut self, c: usize, from: u32) -> u32 {{
        // The run's code, as its next step's check would see it.
        let (page, stamp) = match self.traces[self.tr_cur as usize].as_ref() {{
            Some(t) => (t.page, t.stamp),
            None => return 0,
        }};
        if !self.jit_code_ok(page, stamp) {{
            return 0;
        }}
        self.bus.jit_mem_ptrs(&mut self.jit_ptrs);
        // `sync: quantum`: no later than an urgent device is due. Steps made
        // outside compiled code catch the devices up as the board does, not
        // as `jit_sync_quantum` does, and may have woken one (a sound card
        // started) since the catch-up was set.
        if {quantum}u128 != 0 {{
            self.jit_sync_at = self.jit_sync_at.min(self.bus.jit_urgent_due());
        }}
        // SAFETY: `c` is this machine's compiled code for its current run,
        // which is live: runs are only replaced while stepping, not while
        // compiled code runs.
        let f: extern "C" fn(*mut Self, u32) -> u32 = unsafe {{ std::mem::transmute(c) }};
        let n = f(self as *mut Self, from);
        self.jit_pending = false;
        if self.jit_guard_failed {{
            // Compiled for other values of the speculated registers: past a
            // few refusals, compiled again for the values now.
            self.jit_guard_failed = false;
            if let Some(t) = self.traces[self.tr_cur as usize].as_mut() {{
                t.guard_fails += 1;
                if t.guard_fails >= 8 {{
                    t.code = machine_jit::Code::default();
                    t.hits = 0;
                    t.guard_fails = 0;
                }}
            }}
        }}
        if {sync_run} || {sync_block} {{
            // `sync: run`: the devices, once for the run (`sync: block`:
            // as it returns, as well as where it went round).
            self.bus.after_step();
        }}
        // (As compiled code does at a run's end: a device made due, its
        // deadline to be worked out again, is caught up too.)
        if {quantum}u128 != 0
            && (self.jit_bus_time(|b| Some(b.now)) >= self.jit_sync_at
                || B::jit_board().is_some_and(|b| b.due.is_some()) && self.jit_bus_time(|b| b.due) == 0)
        {{
            self.jit_sync_quantum();
        }}
        if self.jit_last != 0 {{
            // SAFETY: a step of this machine's run, which is live.
            self.last = Some(unsafe {{ *(self.jit_last as usize as *const Instr) }});
            self.last_fault = None;
            self.jit_last = 0;
        }}
        n
    }}
    /// The pc starts a compiled run (its key holding): it made the current
    /// run, as `trace_enter` makes it, without its first step; its code and
    /// length.
    fn jit_lookup(&mut self) -> Option<(usize, u64)> {{
        let start = self.fn_{address}().ok()? as u64;
        let slot = Self::tr_slot(start);
        let pc = self.r.{pc0n} as u64;
        let ep = self.tr_epoch;
        let key = self.tkey();
        let (code, len) = match self.traces[slot].as_ref() {{
            Some(t) if t.phys == start && t.pc == pc && (t.ok_ep == ep || t.key == key) => (t.code.addr()?, t.steps.len() as u64),
            _ => return None,
        }};
        self.trace_close();
        if let Some(t) = self.traces[slot].as_mut() {{
            t.ok_ep = ep;
            t.hits = t.hits.saturating_add(1);
        }}
        let aep = self.tr_aepoch;
        let prev = self.tr_prev as usize;
        if let Some(p) = self.traces.get_mut(prev).and_then(|t| t.as_mut()) {{
            let l = TLink {{ pc, phys: start, ep: aep, kep: self.tr_kepoch, slot: slot as u32 }};
            if p.links[0].pc == pc || p.links[0].ep != aep {{ p.links[0] = l; }} else {{ p.links[1] = l; }}
        }}
        self.tr_on = true;
        self.tr_cur = slot as u32;
        self.tr_idx = 0;
        self.tr_ep0 = ep;
        self.tr_w = u64::MAX;
        Some((code, len))
    }}
    /// A run, as far as it goes (at most `budget` steps): when the pc starts
    /// a compiled run (the last run's link leads there, or it is found by
    /// address), that run from its start; else one step as `System::step`
    /// makes it, a run it entered compiled once it has been entered `HOT`
    /// times (and entered at its start, next time). How many steps were
    /// made.
    pub fn jit_burst(&mut self, budget: u64) -> Result<u64, Fault> {{
        if !self.tr_on && !self.halted && !self.in_reset {{
            let found = if self.tr_prev != u32::MAX {{ self.jit_follow() }} else {{ None }};
            let found = match found {{
                Some(f) => Some(f),
                None if !self.tr_on => self.jit_lookup(),
                None => None,
            }};
            if let Some((c, len)) = found {{
                if budget >= len {{
                    self.jit_left = budget;
                    let slot = self.tr_cur as usize;
                    let n = self.jit_enter(c, 0) as u64;
                    if let Some(e) = self.jit_err.take() {{
                        return Err(e);
                    }}
                    // Hot enough to pay for the optimiser: compiled again,
                    // at the optimised tier.
                    let again = match self.traces.get_mut(slot).and_then(|t| t.as_mut()) {{
                        Some(t) if t.tier == 0 && t.code.addr() == Some(c) => {{
                            t.exec = t.exec.saturating_add(n);
                            if t.exec >= Self::jit_hot2().saturating_mul(t.steps.len() as u64) {{
                                t.tier = 1;
                                true
                            }} else {{
                                false
                            }}
                        }}
                        _ => false,
                    }};
                    if again {{
                        self.jit_compile(slot);
                    }}
                    if n > 0 {{
                        return Ok(n);
                    }}
                }}
            }}
        }}
        let was_halted = self.halted;
        let r = self.step();
        if was_halted && self.halted {{ self.bus.halted_cycle(); }}
        self.bus.after_step();
        r?;
        let mut n = 1;
        // The step entered a run (and made its first instruction, which may
        // have been its last).
        if self.tr_idx == 1 {{
            let slot = self.tr_cur as usize;
            let start = self.last_start;
            let (code, len) = match self.traces[slot].as_mut() {{
                Some(t) if t.phys == start => {{
                    t.hits = t.hits.saturating_add(1);
                    (t.code.addr(), t.steps.len() as u64)
                }}
                _ => (None, 0),
            }};
            // (Entered at its start the next time the pc is there.)
            if code.is_none() && len > 0 && self.traces[slot].as_ref().is_some_and(|t| t.hits == Self::jit_hot()) {{
                self.jit_compile(slot);
            }}
        }}
        Ok(n)
    }}
"#
        );
        Ok(())
    }
}

impl Gen<'_> {
    /// The lowered JIT (`gen/lower.rs`): the isa's fns, kind rules and the
    /// instructions that lower, their shims, and `jit_emit_step`, which
    /// emits a step's host code: the run's check, `step`'s prologue, the
    /// state as decoding left it, the timing entry and semantics, all with
    /// what the step was decoded with as constants; then a call that
    /// retires it. A step whose instruction does not lower is a call of
    /// its fused step fn (`jit_step_<name>`), as before.
    pub(crate) fn emit_jit_lowering(&mut self) -> R<()> {
        let Some(plan) = self.trace.clone() else { return Ok(()) };
        let ist = self.isa.istream.as_ref().unwrap();
        let sp = ident(&self.spaces[&ist.space].name);
        let mut out = String::new();
        // Kind rules and fns may use each other: a second pass sees what
        // the first lowered.
        self.jit_find_raisers();
        self.jit_find_outlined();
        out.push_str(&self.jit_outlined_shims()?);
        out.push_str(&self.jit_ext_shims()?);
        let _ = self.jit_lower_units();
        let _ = self.jit_lower_kinds();
        let _ = self.jit_lower_fns();
        out.push_str(&self.jit_lower_units());
        out.push_str(&self.jit_lower_kinds());
        out.push_str(&self.jit_lower_fns());
        self.jit_report();
        // A fault raised by lowered code: delivered as the step's tail
        // delivers it, then the run ends there.
        for f in &self.isa.faults {
            let n = ident(&f.name);
            let mut ps = String::new();
            let mut args = Vec::new();
            for (i, (_, t)) in f.params.iter().enumerate() {
                let _ = write!(ps, ", a{i}: u64");
                args.push(match t {
                    Ty::Bool => format!("a{i} != 0"),
                    t => format!("a{i} as {}", self.rty(t)?),
                });
            }
            let ctor = if args.is_empty() { format!("Fault::{n}") } else { format!("Fault::{n}({})", args.join(", ")) };
            let _ = write!(
                out,
                "    #[allow(clippy::all)]\n    extern \"C\" fn jit_raise_{n}(m: *mut Self{ps}) -> u64 {{\n        // SAFETY: called by compiled code with its machine; `jit_last` is\n        // the step's instruction, in its run.\n        let m = unsafe {{ &mut *m }};\n        if m.jit_last != 0 {{\n            m.last = Some(unsafe {{ *(m.jit_last as usize as *const Instr) }});\n            m.jit_last = 0;\n        }}\n        m.decoded = true;\n        m.last_fault = None;\n        let r = m.jit_tail(m.tr_phys, Err({ctor}));\n        m.bus.after_step();\n        if let Err(f) = r {{\n            m.jit_err = Some(f);\n        }}\n        1\n    }}\n"
            );
        }
        // The instructions that lower.
        let mut lowered = Vec::new();
        let mut quiet = Vec::new();
        // Instructions that repeat (`repeat ... interruptible`): they may run
        // again from where they started (`repeat: bulk`).
        let mut repeaters = Vec::new();
        // `repeat: bulk(n)`'s kernels: their dry code (and the fns it calls,
        // dry), and the kernels.
        let kernel_names: Vec<String> = self.bulk_kernels.iter().map(|(at, _)| ident(&self.isa.instructions[*at].name)).collect();
        if !self.bulk_kernels.is_empty() {
            let mut ks = String::new();
            self.emit_bulk_common(&mut ks);
            let mut fns: Vec<machine_sema::program::DefId> = Vec::new();
            for (_, k) in &self.bulk_kernels {
                for f in k.fns() {
                    if !fns.contains(f) {
                        fns.push(*f);
                    }
                }
            }
            let dry: R<Vec<String>> = self.lowering_dry(|g| fns.iter().map(|f| g.fn_body_text(g.fns[f])).collect());
            for t in dry? {
                ks.push_str(&t);
            }
            let mut items = String::new();
            for (at, k) in &self.bulk_kernels {
                self.emit_bulk_kernel(&self.isa.instructions[*at], k, &mut ks, &mut items)?;
            }
            self.bulk_items.push_str(&items);
            let f = self.isa.faults.first().ok_or_else(|| GenError::new("`bulk` kernels need a fault"))?;
            let args = if f.params.is_empty() { String::new() } else { format!("({})", vec!["Default::default()"; f.params.len()].join(", ")) };
            let _ = write!(ks, "    /// Where the dry code stops: `probe_bad`, and a fault to leave by.\n    #[allow(dead_code)]\n    fn probe_stop<T>(&mut self) -> Result<T, Fault> {{\n        self.probe_bad = true;\n        Err(Fault::{}{args})\n    }}\n", ident(&f.name));
            self.out.push_str(&ks);
        }
        for i in self.isa.instructions.iter().filter(|i| !i.prefix) {
            match self.jinstr(i) {
                Ok(t) => {
                    out.push_str(&t);
                    lowered.push(ident(&i.name));
                    if i.repeat.as_ref().is_some_and(|r| r.interruptible) {
                        repeaters.push(ident(&i.name));
                    }
                    if !self.jtouches_bus(i) {
                        quiet.push(ident(&i.name));
                    }
                }
                Err(e) if std::env::var_os("MACHINE_JIT_LOWERING").is_some() => eprintln!("{}: {:?}", i.name, e),
                Err(_) => {}
            }
        }
        // The space shims a lowered access calls.
        let cx = super::Cx { locals: &[], fallible: false, recv: "m", ops: None, comp: None, opt: false };
        for s in &self.isa.spaces {
            let n = ident(&s.name);
            for bits in [8u32, 16, 32, 64] {
                let split = self.split_charge(&cx, s.def, bits);
                let _ = write!(
                    out,
                    "    extern \"C\" fn jit_sp_{n}_read{bits}(m: *mut Self, a: u64) -> u64 {{\n        // SAFETY: called by compiled code with its machine.\n        let m = unsafe {{ &mut *m }};\n        m.accesses = m.accesses.wrapping_add(1); {split}m.bus.{n}_read(a, {bits}) as u64\n    }}\n    extern \"C\" fn jit_sp_{n}_write{bits}(m: *mut Self, a: u64, v: u64) -> u64 {{\n        // SAFETY: as for the reads.\n        let m = unsafe {{ &mut *m }};\n        m.accesses = m.accesses.wrapping_add(1); {split}m.bus.{n}_write(a, {bits}, v as u128);\n        0\n    }}\n"
                );
            }
        }
        // The prologue in host code: when the isa resets through the
        // istream, or its boundary fn does not lower, steps are not lowered.
        let jf = self.jit_fns.borrow().clone();
        // The reset line: an external fn (a load or a call) or a lowered one.
        let reset_ok = ist.reset.is_none_or(|r| self.fns[&r].external || jf.contains(&r));
        let gate_lowers = reset_ok && ist.boundary.is_none_or(|b| jf.contains(&b));
        let (reset_check, reset_shim) = match ist.reset {
            Some(r) if gate_lowers => {
                let f = self.fns[&r];
                let call = if f.external {
                    format!("m.bus.ext_{}()", ident(&f.name))
                } else {
                    format!("m.fn_{}().unwrap_or(false)", ident(&f.name))
                };
                let line = super::lower::Lx { locals: &[], ops: None, src: None, timing: false };
                let call_expr = machine_sema::hir::Expr { kind: machine_sema::hir::ExprKind::FnCall(r, Vec::new()), ty: Ty::Bool, span: Default::default() };
                let lowered = self.jexpr(&line, &call_expr)?;
                (
                    format!("        // The reset line, or leaving reset: as the interpreter's\n        // prologue makes it.\n        let rl = {lowered};\n        let ir = e.load(std::mem::offset_of!(Self, in_reset), machine_jit::N::Bool);\n        let any = e.op(machine_jit::Op::Or, rl, ir, machine_jit::N::Bool);\n        e.if_(any, |e| {{ let r = e.call(Self::jit_reset_c as *const () as usize, &[], machine_jit::N::U32, false); e.exit_with(r); }}, |_| {{}});\n"),
                    format!("    /// The prologue's reset line check: 1 when the step was the reset (or\n    /// held in it), 0 to go on (out of reset, if it was).\n    extern \"C\" fn jit_reset_c(m: *mut Self) -> u64 {{\n        // SAFETY: called by compiled code with its machine.\n        let m = unsafe {{ &mut *m }};\n        if {call} {{\n            if !m.in_reset {{ m.reset(); m.bus.processor_reset(); m.in_reset = true; }}\n            m.halted = true;\n            m.bus.after_step();\n            return 1;\n        }}\n        // Out of reset: the step is made by the interpreter, which sees it\n        // so too (the run is left: what the code knew may not hold).\n        if m.in_reset {{ m.in_reset = false; m.halted = false; }}\n        2\n    }}\n"),
                )
            }
            _ => (String::new(), String::new()),
        };
        out.push_str(&reset_shim);
        let mut latch_resets = String::new();
        let mut latches_ok = true;
        for l in &self.latches {
            let lx = super::lower::Lx { locals: &[], ops: None, src: None, timing: false };
            match self.jexpr(&lx, &l.init) {
                Ok(v) => {
                    let _ = writeln!(
                        latch_resets,
                        "        {{ let v = {v}; e.write(&machine_jit::Place::Mem {{ off: std::mem::offset_of!(Self, latch_{}), n: {}, bump: 0 }}, v); }}",
                        ident(&l.name),
                        self.jn(&l.ty)?
                    );
                }
                Err(_) => latches_ok = false,
            }
        }
        let lowers = latches_ok;
        // A step decoded with the lock prefix: the interpreter checks it.
        let locked = match ist.lock {
            Some(l) => format!("s.l_{} == 1", ident(&self.prog.item_name(l))),
            None => "false".to_string(),
        };
        let boundary = match ist.boundary {
            _ if !lowers || !gate_lowers => String::new(),
            Some(b) => format!(
                "        // Where the fn returns true the step leaves the run, in place.\n        e.exit_on_true_return(Self::jit_interrupt_c as *const () as usize, true);\n        let taken = Self::jf_{}(e);\n        e.exit_if(taken, true, |e| {{ let _ = e.call(Self::jit_interrupt_c as *const () as usize, &[], machine_jit::N::U64, false); }});\n",
                ident(&self.fns[&b].name)
            ),
            None => String::new(),
        };
        let opcode = match ist.opcode {
            _ if !lowers => String::new(),
            Some(h) if jf.contains(&h) => format!("        for _ in 0..s.opcodes {{ let _ = Self::jf_{}(e); }}\n", ident(&self.fns[&h].name)),
            Some(_) => "        for _ in 0..s.opcodes { let _ = e.call(Self::jit_opcode_c as *const () as usize, &[], machine_jit::N::U64, false); }\n".to_string(),
            None => String::new(),
        };
        let opcode_shim = match ist.opcode {
            Some(h) => format!("    extern \"C\" fn jit_opcode_c(m: *mut Self) -> u64 {{\n        // SAFETY: called by compiled code with its machine.\n        let m = unsafe {{ &mut *m }};\n        let _ = m.fn_{}();\n        0\n    }}\n", ident(&self.fns[&h].name)),
            None => String::new(),
        };
        let opcodes_store = if ist.opcode.is_some() { "        e.store(std::mem::offset_of!(Self, opcodes), machine_jit::N::U8, machine_jit::V::K(s.opcodes as u128));\n" } else { "" };
        let mut restore = String::new();
        for l in &self.latches {
            let n = ident(&l.name);
            let _ = writeln!(restore, "        e.write(&machine_jit::Place::Mem {{ off: std::mem::offset_of!(Self, latch_{n}), n: {}, bump: 0 }}, machine_jit::k(s.l_{n}));", self.jn(&l.ty)?);
        }
        for d in &plan.pc {
            let (_, r) = self.regs[d];
            let n = ident(&r.name);
            let Ty::UInt(bits) = &r.ty else { return Ok(()) };
            let _ = writeln!(
                restore,
                "        e.write(&machine_jit::Place::Mem {{ off: std::mem::offset_of!(Self, r.{n}), n: {}, bump: 0 }}, machine_jit::k(s.pc_{n}));",
                self.jn(&Ty::UInt(*bits))?
            );
        }
        let (_, pc0) = self.regs[&plan.pc[0]];
        let Ty::UInt(pcbits) = &pc0.ty else { return Ok(()) };
        let pc_n = self.jn(&Ty::UInt(*pcbits))?;
        let pc_mask = if super::native(*pcbits, false).0 >= 64 { u64::MAX } else { (1u64 << super::native(*pcbits, false).0) - 1 };
        let mut arms = String::new();
        for n in &lowered {
            let after = self.isa.instructions.iter().find(|i| ident(&i.name) == *n).is_some_and(Self::jtiming_after);
            let _ = writeln!(arms, "            Instr::{n}(ops) => Some((Box::new(move |e: &mut machine_jit::Em| Self::jt_{n}(e, &ops, phys)) as Box<dyn FnOnce(&mut machine_jit::Em) -> machine_jit::V>, Box::new(move |e: &mut machine_jit::Em| Self::jx_{n}(e, &ops)) as Box<dyn FnOnce(&mut machine_jit::Em)>, {after})),");
        }
        let _ = write!(
            out,
            r#"{opcode_shim}    /// The step's retirement, for a lowered step: `last`, `step`'s tail,
    /// the devices caught up, then the run's code checked for writes (the
    /// next step's check). 0 to go on, 1 to stop.
    /// With `len`, the step's code fetch is charged first (an instruction
    /// that reaches no bus space: nothing sees the time between).
    extern "C" fn jit_retire_c(m: *mut Self, i: u64, phys: u64, page: u64, stamp: u64, len: u64) -> u64 {{
        // SAFETY: called by compiled code with its machine and a step of
        // one of its runs, live for the call.
        let (m, i) = unsafe {{ (&mut *m, *(i as usize as *const Instr)) }};
        m.jit_last = 0;
        if len != 0 {{
            m.bus.{sp}_fetch_page(page as u32, phys, len as u32);
        }}
        m.last = Some(i);
        m.decoded = true;
        m.last_fault = None;
        let r = m.jit_tail(phys, Ok(()));
        m.bus.after_step();
        if let Err(f) = r {{
            m.jit_err = Some(f);
            return 1;
        }}
        if !m.jit_code_ok(page as u32, stamp as u32) {{
            return 1;
        }}
        u64::from(m.halted || !m.tr_on)
    }}
    /// Code writes since the run's page was last checked: its stamp still
    /// holds (else the run stops, as `trace_check` stops it).
    fn jit_code_ok(&mut self, page: u32, stamp: u32) -> bool {{
        let w = self.bus.{sp}_code_writes();
        if w != self.tr_w {{
            // (The run whose code asks: the current one, its stamp its own,
            // which a check may have moved on since it was compiled.)
            let _ = stamp;
            if !self.tr_fresh(self.tr_cur as usize, page) {{
                self.tr_on = false;
                self.tr_prev = u32::MAX;
                return false;
            }}
            self.tr_w = w;
        }}
        true
    }}
    extern "C" fn jit_code_c(m: *mut Self, page: u64, stamp: u64) -> u64 {{
        // SAFETY: called by compiled code with its machine.
        let m = unsafe {{ &mut *m }};
        if m.jit_code_ok(page as u32, stamp as u32) {{ 0 }} else {{ 2 }}
    }}
    extern "C" fn jit_code_hit_c(m: *mut Self, mem: u64, k: u64) -> u64 {{
        // SAFETY: called by compiled code with its machine.
        let m = unsafe {{ &mut *m }};
        m.bus.jit_code_hit(mem as u32, k as usize);
        0
    }}
    /// The native type of an access of `n` bytes.
    fn jm_n(n: u32) -> machine_jit::N {{
        match n {{ 1 => machine_jit::N::U8, 2 => machine_jit::N::U16, 4 => machine_jit::N::U32, _ => machine_jit::N::U64 }}
    }}
    /// A read of `n` bytes at `a` (a u64) in the istream space: in host code
    /// when it falls in the board's direct memory, else the `shim` call
    /// (which charges the access and any split itself).
    #[allow(clippy::all)]
    fn jm_read(e: &mut machine_jit::Em, a: machine_jit::V, n: u32, shim: usize, pure: bool) -> machine_jit::V {{
        let mems = B::jit_mems();
        match B::jit_board() {{
            Some(b) if !mems.is_empty() => {{
                let a = e.op(machine_jit::Op::And, a, machine_jit::V::K(mems[0].amask as u128), machine_jit::N::U64);
                Self::jm_access(e, a, n, None, mems, b, shim, pure)
            }}
            _ => e.call(shim, &[(a, machine_jit::N::U64)], Self::jm_n(n), pure),
        }}
    }}
    /// A write of `v` (`n` bytes) at `a`, as `jm_read`.
    #[allow(clippy::all)]
    fn jm_write(e: &mut machine_jit::Em, a: machine_jit::V, v: machine_jit::V, n: u32, shim: usize, pure: bool) {{
        let mems = B::jit_mems();
        match B::jit_board() {{
            Some(b) if !mems.is_empty() => {{
                let a = e.op(machine_jit::Op::And, a, machine_jit::V::K(mems[0].amask as u128), machine_jit::N::U64);
                let _ = Self::jm_access(e, a, n, Some(v), mems, b, shim, pure);
            }}
            _ => {{
                let v = e.cast(v, Self::jm_n(n), machine_jit::N::U64);
                let _ = e.call(shim, &[(a, machine_jit::N::U64), (v, machine_jit::N::U64)], machine_jit::N::U64, pure);
            }}
        }}
    }}
    #[allow(clippy::all)]
    /// A time `t` (fine units) in the processor's cycles, when the run
    /// keeps its clock in them (`machine_jit::Clock`) and `t` is whole
    /// cycles.
    fn jit_cyc(e: &machine_jit::Em, b: &JitBoard, t: u128) -> Option<u128> {{
        (e.clocked() && b.clock != 0 && t % b.clock == 0).then(|| t / b.clock)
    }}
    fn jm_access(e: &mut machine_jit::Em, a: machine_jit::V, n: u32, v: Option<machine_jit::V>, mems: &'static [JitMem], b: JitBoard, shim: usize, pure: bool) -> machine_jit::V {{
        let nn = Self::jm_n(n);
        let Some((m, rest)) = mems.split_first() else {{
            // By the bus: the shim counts the access and its split cycles;
            // the processor's state is not the bus's to change.
            let touched = [(std::mem::offset_of!(Self, cycles), 4), (std::mem::offset_of!(Self, accesses), 1)];
            return match v {{
                None if pure => e.call(shim, &[(a, machine_jit::N::U64)], nn, pure),
                None => e.call_reading(shim, &[(a, machine_jit::N::U64)], nn, &touched),
                Some(v) => {{
                    let v = e.cast(v, nn, machine_jit::N::U64);
                    if pure {{
                        let _ = e.call(shim, &[(a, machine_jit::N::U64), (v, machine_jit::N::U64)], machine_jit::N::U64, pure);
                    }} else {{
                        let _ = e.call_touching(shim, &[(a, machine_jit::N::U64), (v, machine_jit::N::U64)], machine_jit::N::U64, &touched);
                    }}
                    machine_jit::V::K(0)
                }}
            }};
        }};
        if v.is_some() && m.write.is_none() {{
            return Self::jm_access(e, a, n, v, rest, b, shim, pure);
        }}
        // The ranges of the same memory at the same costs (one split by
        // a hole) share the access: one body for all of them. Only the
        // first memory's: the others (ROM, as a rule) go by the bus.
        let same = |x: &JitMem| x.amask == m.amask && x.mask == m.mask && x.read == m.read && x.write == m.write && x.unit == m.unit && x.cells == m.cells && x.code == m.code && x.mem == m.mem;
        let group: Vec<&JitMem> = mems.iter().filter(|x| same(x)).collect();
        // Within a range, and (a mirrored memory) not wrapping inside the
        // access.
        let last = e.op(machine_jit::Op::Add, a, machine_jit::V::K((n - 1) as u128), machine_jit::N::U64);
        let mut inside = machine_jit::V::K(0);
        for g in &group {{
            let one = if g.lo == 0 && g.hi >= g.amask && n == 1 {{
                machine_jit::V::K(1)
            }} else {{
                let ge = e.op(machine_jit::Op::Ge, a, machine_jit::V::K(g.lo as u128), machine_jit::N::U64);
                let le = e.op(machine_jit::Op::Le, last, machine_jit::V::K(g.hi as u128), machine_jit::N::U64);
                e.op(machine_jit::Op::And, ge, le, machine_jit::N::Bool)
            }};
            inside = e.op(machine_jit::Op::Or, inside, one, machine_jit::N::Bool);
        }}
        if m.mask != u64::MAX && n > 1 {{
            let i = e.op(machine_jit::Op::And, a, machine_jit::V::K(m.mask as u128), machine_jit::N::U64);
            let il = e.op(machine_jit::Op::Add, i, machine_jit::V::K((n - 1) as u128), machine_jit::N::U64);
            let fits = e.op(machine_jit::Op::Le, il, machine_jit::V::K(m.mask as u128), machine_jit::N::U64);
            inside = e.op(machine_jit::Op::And, inside, fits, machine_jit::N::Bool);
        }}
        e.if_val(inside, nn, |e| {{
            // The access, as the shim counts it, and its split.
            let acc = e.read(&machine_jit::Place::Mem {{ off: std::mem::offset_of!(Self, accesses), n: machine_jit::N::U8, bump: 0 }});
            let acc = e.op(machine_jit::Op::Add, acc, machine_jit::V::K(1), machine_jit::N::U8);
            e.store(std::mem::offset_of!(Self, accesses), machine_jit::N::U8, acc);
            let a32 = e.cast(a, machine_jit::N::U64, machine_jit::N::U32);
            let sc = Self::jit_split(e, a32, machine_jit::V::K(n as u128));
            if sc != machine_jit::V::K(0) {{
                let cy = e.read(&machine_jit::Place::Mem {{ off: std::mem::offset_of!(Self, cycles), n: machine_jit::N::U32, bump: 0 }});
                let cy = e.op(machine_jit::Op::Add, cy, sc, machine_jit::N::U32);
                e.store(std::mem::offset_of!(Self, cycles), machine_jit::N::U32, cy);
            }}
            let i = if m.mask == u64::MAX {{ a }} else {{ e.op(machine_jit::Op::And, a, machine_jit::V::K(m.mask as u128), machine_jit::N::U64) }};
            let base = e.read(&machine_jit::Place::Mem {{ off: std::mem::offset_of!(Self, jit_ptrs) + 8 * m.cells, n: machine_jit::N::U64, bump: 0 }});
            let p = e.op(machine_jit::Op::Add, base, i, machine_jit::N::U64);
            let (r, cost) = match v {{
                None => (e.load_at(p, nn), m.read),
                Some(v) => {{
                    e.store_at(p, nn, v);
                    // Recorded code written (its first or last byte's block):
                    // its page's stamp moves on.
                    if let Some(c) = m.code {{
                        let cb = e.read(&machine_jit::Place::Mem {{ off: std::mem::offset_of!(Self, jit_ptrs) + 8 * c, n: machine_jit::N::U64, bump: 0 }});
                        let mut ends = vec![i];
                        if n > 1 {{
                            ends.push(e.op(machine_jit::Op::Add, i, machine_jit::V::K((n - 1) as u128), machine_jit::N::U64));
                        }}
                        for k in ends {{
                            let pg = e.op(machine_jit::Op::Shr, k, machine_jit::V::K(12), machine_jit::N::U64);
                            let off = e.op(machine_jit::Op::Shl, pg, machine_jit::V::K(2), machine_jit::N::U64);
                            let wp = e.op(machine_jit::Op::Add, cb, off, machine_jit::N::U64);
                            let word = e.load_at(wp, machine_jit::N::U32);
                            let blk = e.op(machine_jit::Op::Shr, k, machine_jit::V::K(7), machine_jit::N::U64);
                            let blk = e.op(machine_jit::Op::And, blk, machine_jit::V::K(31), machine_jit::N::U64);
                            let blk = e.cast(blk, machine_jit::N::U64, machine_jit::N::U32);
                            let bit = e.op(machine_jit::Op::Shl, machine_jit::V::K(1), blk, machine_jit::N::U32);
                            let hit = e.op(machine_jit::Op::And, word, bit, machine_jit::N::U32);
                            let hit = e.op(machine_jit::Op::Ne, hit, machine_jit::V::K(0), machine_jit::N::U32);
                            e.cold(hit, |e| {{
                                let _ = e.call(Self::jit_code_hit_c as *const () as usize, &[(machine_jit::V::K(m.mem as u128), machine_jit::N::U64), (k, machine_jit::N::U64)], machine_jit::N::U64, true);
                            }});
                            e.note_code_write();
                        }}
                    }}
                    (machine_jit::V::K(0), m.write.unwrap_or(0))
                }}
            }};
            // The bus units it spans, each at its cost.
            // (`n` bytes span ceil(n / unit) units, or one more: a choice of
            // two.)
            // (In cycles when the run counts them: `jit_cyc`.)
            let (two, n_of) = if m.unit == 1 {{
                (None, n as u128)
            }} else {{
                let sh = m.unit.trailing_zeros() as u128;
                let u1 = e.op(machine_jit::Op::Shr, last, machine_jit::V::K(sh), machine_jit::N::U64);
                let u0 = e.op(machine_jit::Op::Shr, a, machine_jit::V::K(sh), machine_jit::N::U64);
                let d = e.op(machine_jit::Op::Sub, u1, u0, machine_jit::N::U64);
                let lo = (n as u64).div_ceil(m.unit) as u128;
                let fewer = e.op(machine_jit::Op::Eq, d, machine_jit::V::K(lo - 1), machine_jit::N::U64);
                (Some(fewer), lo)
            }};
            match (two, Self::jit_cyc(e, &b, n_of * cost), Self::jit_cyc(e, &b, (n_of + 1) * cost)) {{
                (None, Some(c), _) => e.tick(machine_jit::V::K(c)),
                (Some(fewer), Some(c0), Some(c1)) => {{
                    let t = e.select(fewer, machine_jit::V::K(c0), machine_jit::V::K(c1), machine_jit::N::U64);
                    e.tick(t);
                }}
                _ => {{
                    let t = match two {{
                        None => machine_jit::V::K(n_of * cost),
                        Some(fewer) => e.select(fewer, machine_jit::V::K(n_of * cost), machine_jit::V::K((n_of + 1) * cost), machine_jit::N::U128),
                    }};
                    let bus = std::mem::offset_of!(Self, bus);
                    let now = e.read(&machine_jit::Place::Mem {{ off: bus + b.now, n: machine_jit::N::U128, bump: 0 }});
                    let now = e.op(machine_jit::Op::Add, now, t, machine_jit::N::U128);
                    e.store(bus + b.now, machine_jit::N::U128, now);
                }}
            }}
            r
        }}, |e| Self::jm_access(e, a, n, v, &[], b, shim, pure))
    }}
    /// `step`'s prologue, for a lowered step whose gate does not lower: 0
    /// when the instruction is to run, else 1 (the step was made another
    /// way: reset, shut down, an interrupt, halted).
    extern "C" fn jit_gate_c(m: *mut Self) -> u64 {{
        // SAFETY: called by compiled code with its machine.
        let m = unsafe {{ &mut *m }};
        m.jit_go = false;
        let r = m.jit_prologue();
        if !m.jit_go {{
            m.bus.after_step();
            if let Err(f) = r {{
                m.jit_err = Some(f);
            }}
            return 1;
        }}
        0
    }}
    /// A step made by its prologue alone (shut down, halted).
    extern "C" fn jit_idle_c(m: *mut Self) -> u64 {{
        // SAFETY: called by compiled code with its machine.
        let m = unsafe {{ &mut *m }};
        m.bus.after_step();
        1
    }}
    /// `sync: quantum(...)`: the devices caught up, and the next time they
    /// are: when one is due, but not before the quantum has passed.
    extern "C" fn jit_quantum_c(m: *mut Self) -> u64 {{
        // SAFETY: called by compiled code with its machine.
        let m = unsafe {{ &mut *m }};
        m.jit_sync_quantum();
        1
    }}
    /// `repeat: bulk`: of `max` elements, the whole periods (of `per`
    /// elements, each taking the time since `jit_bulk_mark`, the period's
    /// end at `now`, the time's low half) that end before the devices'
    /// deadline: the elements after them run one by one, the batch ending
    /// at the first that reaches it.
    #[allow(dead_code)]
    fn jit_due_cap(&self, max: u64, now_lo: u64, per: u64) -> u64 {{
        let Some(b) = B::jit_board() else {{ return max }};
        let dl = if {quantum}u128 != 0 {{
            self.jit_sync_at
        }} else if b.due.is_some() {{
            self.jit_bus_time(|b| b.due)
        }} else {{
            return max;
        }};
        let now = self.jit_bus_time(|b| Some(b.now));
        let p = now_lo.wrapping_sub(self.jit_bulk_mark) as u128;
        if p == 0 {{
            return max;
        }}
        if dl <= now {{
            return 0;
        }}
        let periods = (dl - now - 1) / p;
        let els = periods.saturating_mul(per as u128);
        max.min(u64::try_from(els).unwrap_or(u64::MAX))
    }}
    fn jit_sync_quantum(&mut self) {{
        self.bus.after_step();
        let now = self.jit_bus_time(|b| Some(b.now));
        let due = self.jit_bus_time(|b| b.due);
        // The others wait for the quantum; a device that raises signals
        // or masters a bus keeps its deadline.
        self.jit_sync_at = due.max(now.saturating_add({quantum}u128)).min(self.bus.jit_urgent_due());
    }}
    /// A time the board keeps (`JitBoard`'s offsets): `now`, or `due`
    /// (0 when the board has none: always due).
    fn jit_bus_time(&self, f: impl Fn(&JitBoard) -> Option<usize>) -> u128 {{
        let Some(off) = B::jit_board().and_then(|b| f(&b)) else {{ return 0 }};
        // SAFETY: `off` is a `u128` field's offset within the bus (the
        // board's own `jit_board`).
        unsafe {{ ((&self.bus as *const B as *const u8).add(off) as *const u128).read_unaligned() }}
    }}
    /// An interrupt taken at the boundary, in place of the step.
    extern "C" fn jit_interrupt_c(m: *mut Self) -> u64 {{
        // SAFETY: called by compiled code with its machine.
        let m = unsafe {{ &mut *m }};
        m.halted = false;
        m.cycles = m.interrupt_cycles();
        m.bus.retire(m.cycles, Schedule::default());
        m.bus.after_step();
        1
    }}
    extern "C" fn jit_fetch_c(m: *mut Self, page: u64, phys: u64, len: u64) -> u64 {{
        // SAFETY: called by compiled code with its machine.
        let m = unsafe {{ &mut *m }};
        m.bus.{sp}_fetch_page(page as u32, phys, len as u32);
        0
    }}
    /// Step `k` of a run (`j`), lowered when its instruction lowers;
    /// `checked` when the step before it checked the run's code after it.
    /// Whether it was lowered.
    #[allow(unused_variables, clippy::all)]
    /// `follows`: the step is reached only from a lowered step before it,
    /// which retired with the processor running, not shut down, and the
    /// epoch as it was unless its writes bumped it.
    fn jit_emit_step(e: &mut machine_jit::Em, j: &JitStep, k: usize, checked: bool, follows: bool, fetch: Option<u128>, units: Option<(u64, u64, u128)>, spec: &[u128], key: &TKey) -> bool {{
        let s = &j.step;
        if k == 0 {{
            Self::jit_declare_bits(e);
        }}
        if B::SCHEDULE || !{lowers} {{
            let r = e.call(Self::jit_step_fn(&s.instr), &[(machine_jit::V::K(j as *const JitStep as usize as u128), machine_jit::N::U64)], machine_jit::N::U32, false);
            e.exit_on(r);
            return false;
        }}
        let instr = s.instr;
        // Is the instruction lowered? (Its timing is emitted after the
        // prologue, so only the shape is asked here.)
        if !Self::jit_lowers(&instr) || {locked} {{
            let r = e.call(Self::jit_step_fn(&s.instr), &[(machine_jit::V::K(j as *const JitStep as usize as u128), machine_jit::N::U64)], machine_jit::N::U32, false);
            e.exit_on(r);
            return false;
        }}
        e.set_epochs([std::mem::offset_of!(Self, tr_epoch), std::mem::offset_of!(Self, tr_aepoch), std::mem::offset_of!(Self, tr_kepoch)]);
        // The run's check: epoch and pc (the code, after the step before).
        let bumped = e.take_bumped();
        let c1 = if follows && !bumped {{
            machine_jit::V::K(0)
        }} else {{
            let ep = e.load(std::mem::offset_of!(Self, tr_epoch), machine_jit::N::U64);
            let ep0 = e.load(std::mem::offset_of!(Self, tr_ep0), machine_jit::N::U64);
            e.op(machine_jit::Op::Ne, ep, ep0, machine_jit::N::U64)
        }};
        let pc = e.read(&machine_jit::Place::Mem {{ off: std::mem::offset_of!(Self, r.{pc0n}), n: {pc_n}, bump: 0 }});
        let c2 = e.op(machine_jit::Op::Ne, pc, machine_jit::V::K((j.pc & {pc_mask:#x}) as u128), {pc_n});
        let bad = e.op(machine_jit::Op::Or, c1, c2, machine_jit::N::Bool);
        e.exit_if(bad, false, |e| {{
            e.store(std::mem::offset_of!(Self, tr_on), machine_jit::N::Bool, machine_jit::V::K(0));
            e.store(std::mem::offset_of!(Self, tr_prev), machine_jit::N::U32, machine_jit::V::K(j.slot as u128));
        }});
        if !checked {{
            let r = e.call(Self::jit_code_c as *const () as usize, &[(machine_jit::V::K(j.page as u128), machine_jit::N::U64), (machine_jit::V::K(j.stamp as u128), machine_jit::N::U64)], machine_jit::N::U32, true);
            e.exit_on(r);
        }}
        // Speculation: where the run is entered, the speculated registers
        // must hold what it was compiled for; then (the epoch unchanged:
        // a write to one would have ended the run) they, and the key's
        // registers, are known values.
        let places = Self::jit_spec_places();
        if k == 0 {{
            for (i, (off, n)) in places.iter().enumerate() {{
                // (Known going round: the run's copy of this step reached
                // from its end checks nothing.)
                let v = e.read(&machine_jit::Place::Mem {{ off: *off, n: *n, bump: 0 }});
                let bad = e.op(machine_jit::Op::Ne, v, machine_jit::V::K(spec[i]), *n);
                e.exit_if(bad, false, |e| e.store(std::mem::offset_of!(Self, jit_guard_failed), machine_jit::N::Bool, machine_jit::V::K(1)));
            }}
        }}
        let assume = |e: &mut machine_jit::Em| {{
            for (i, (off, n)) in places.iter().enumerate() {{
                e.assume(*off, *n, spec[i]);
            }}
            Self::jit_key_assume(e, key);
            // Past the run's check: the run goes on.
            e.assume(std::mem::offset_of!(Self, tr_on), machine_jit::N::Bool, 1);
            // Between instructions no code time is owed (each retires it).
            if let Some(b) = B::jit_board() {{
                if !b.serial {{
                    e.assume(std::mem::offset_of!(Self, bus) + b.code_time, machine_jit::N::U128, 0);
                }}
            }}
        }};
        assume(e);
        // `step`'s prologue.
        e.store(std::mem::offset_of!(Self, cycles), machine_jit::N::U32, machine_jit::V::K(0));
        e.store(std::mem::offset_of!(Self, accesses), machine_jit::N::U8, machine_jit::V::K(0));
        e.store(std::mem::offset_of!(Self, win_len), machine_jit::N::U32, machine_jit::V::K(0));
        if !{gate_lowers} {{
            // The prologue's gate as the interpreter makes it (the reset
            // line, shutdown, the boundary fn, halted).
            let r = e.call(Self::jit_gate_c as *const () as usize, &[], machine_jit::N::U32, false);
            e.exit_on(r);
        }} else {{
{latch_resets}        // `events: block`: the reset line and interrupts are recognised
        // where a run is entered or goes round, not past a compiled step.
        let events_here = !({events_block} && follows);
        if events_here {{
{reset_check}        }}
        if !follows {{
            let sd = e.load(std::mem::offset_of!(Self, shutdown), machine_jit::N::Bool);
            e.exit_if(sd, true, |e| {{ let _ = e.call(Self::jit_idle_c as *const () as usize, &[], machine_jit::N::U64, false); }});
        }}
        if events_here {{
{boundary}        }}
        // A boundary fn that takes no interrupt leaves `halted` as it was
        // (false, after a lowered step).
        let h = if follows && !{boundary_wakes} {{
            machine_jit::V::K(0)
        }} else {{
            e.read(&machine_jit::Place::Mem {{ off: std::mem::offset_of!(Self, halted), n: machine_jit::N::Bool, bump: 0 }})
        }};
        e.exit_if(h, true, |e| {{ let _ = e.call(Self::jit_idle_c as *const () as usize, &[], machine_jit::N::U64, false); }});
        // Past the check: the processor runs.
        e.assume(std::mem::offset_of!(Self, halted), machine_jit::N::Bool, 0);
        }}
        // Past the gate, the step is the instruction's: what an interrupt's
        // path (which left the run) forgot holds again.
        assume(e);
        let code0 = e.code_writes();
        e.store(std::mem::offset_of!(Self, tr_rec_now), machine_jit::N::Bool, machine_jit::V::K(0));
        // The state as decoding left it.
        e.store(std::mem::offset_of!(Self, fetched), machine_jit::N::U8, machine_jit::V::K(s.len as u128));
        e.store(std::mem::offset_of!(Self, known_opcode), machine_jit::N::Bool, machine_jit::V::K(s.known as u128));
        e.store(std::mem::offset_of!(Self, prefix_cycles), machine_jit::N::U32, machine_jit::V::K(s.prefix_cycles as u128));
        // `repeat: bulk(n)`: an instruction that runs again (its pc set back
        // to it: a repeated string instruction) runs again within the step,
        // up to n times, each time as a step after its gate: decoded (the
        // latches and the pc as decoding left them), fetched, timed, and the
        // run's code checked; interrupts and devices are seen past the
        // batch.
        let bulk = {repeat_bulk}u32 != 0 && Self::jit_may_repeat(&instr);
        let times = if bulk {{ Some(e.local(machine_jit::N::U32, machine_jit::V::K(0))) }} else {{ None }};
        let head = if bulk {{ e.repeat_head() }} else {{ None }};
        if bulk {{
            // (What the run's checks found holds round the loop: the epoch
            // is checked as it goes round.)
            assume(e);
            e.assume(std::mem::offset_of!(Self, halted), machine_jit::N::Bool, 0);
        }}
{restore}        e.store(std::mem::offset_of!(Self, tr_idx), machine_jit::N::U32, machine_jit::V::K(k as u128 + 1));
{opcodes_store}{opcode}        if j.last {{
            e.store(std::mem::offset_of!(Self, tr_on), machine_jit::N::Bool, machine_jit::V::K(0));
            e.store(std::mem::offset_of!(Self, tr_prev), machine_jit::N::U32, machine_jit::V::K(j.slot as u128));
        }}
        e.store(std::mem::offset_of!(Self, tr_phys), machine_jit::N::U64, machine_jit::V::K(j.phys as u128));
        let ls = e.read(&machine_jit::Place::Mem {{ off: std::mem::offset_of!(Self, last_start), n: machine_jit::N::U64, bump: 0 }});
        let rs = e.op(machine_jit::Op::Eq, ls, machine_jit::V::K(j.phys as u128), machine_jit::N::U64);
        e.store(std::mem::offset_of!(Self, restarted), machine_jit::N::Bool, rs);
        e.store(std::mem::offset_of!(Self, last_start), machine_jit::N::U64, machine_jit::V::K(j.phys as u128));
        // The board's time keeping in host code, when the board says how it
        // keeps it.
        let board = B::jit_board();
        // The bus calls (fetches, accesses) move time and the code unit.
        if let Some(b) = board {{
            let bus = std::mem::offset_of!(Self, bus);
            e.set_volatile(bus + b.now, 16);
            // (The code time and unit change only with the processor's own
            // fetches: the fetch shim says so.)
            e.set_volatile(bus + b.pending, 1);
        }}
        // The fetch, charged before the semantics as the bus would charge
        // it: serial (every unit), or overlapped (the unit last fetched is
        // in the buffer; the code time is what execution overlaps).
        // `timing: approximate`: the fetch as if serial, whatever the bus
        // does, charged with the cycles in one add.
        let approx_fetch: Option<u128> = if {timing_approx} {{ units.map(|(u0, u1, cost)| (u1 - u0 + 1) as u128 * cost).or(fetch) }} else {{ None }};
        // The fetch's charge in cycles, overlapped (`jit_cyc`): what the
        // retire's execution overlaps.
        let mut ct_cyc: Option<machine_jit::V> = None;
        let quiet = match (board, fetch, units) {{
            (Some(_), _, _) if approx_fetch.is_some() => true,
            // Serial, and nothing sees the time before the step retires: the
            // fetch is charged with it, in one add.
            (Some(b), Some(_), _) if b.serial && Self::jit_quiet(&instr) => true,
            (Some(b), Some(f), _) if b.serial => {{
                let bus = std::mem::offset_of!(Self, bus);
                if let Some(c) = Self::jit_cyc(e, &b, f) {{
                    e.tick(machine_jit::V::K(c));
                }} else {{
                    let now = e.read(&machine_jit::Place::Mem {{ off: bus + b.now, n: machine_jit::N::U128, bump: 0 }});
                    let now = e.op(machine_jit::Op::Add, now, machine_jit::V::K(f), machine_jit::N::U128);
                    e.store(bus + b.now, machine_jit::N::U128, now);
                }}
                e.store(bus + b.code_time, machine_jit::N::U128, machine_jit::V::K(f));
                false
            }}
            (Some(b), _, Some((u0, u1, cost))) if !b.serial => {{
                let bus = std::mem::offset_of!(Self, bus);
                let dc = e.read(&machine_jit::Place::Mem {{ off: bus + b.direct_code, n: machine_jit::N::U64, bump: 0 }});
                let same = e.op(machine_jit::Op::Eq, dc, machine_jit::V::K(u0 as u128), machine_jit::N::U64);
                // The units not in the buffer, each `cost`: a choice of two.
                let n = (u1 - u0 + 1) as u128;
                let t = e.select(same, machine_jit::V::K((n - 1) * cost), machine_jit::V::K(n * cost), machine_jit::N::U128);
                let tc = match (Self::jit_cyc(e, &b, (n - 1) * cost), Self::jit_cyc(e, &b, n * cost)) {{
                    (Some(c0), Some(c1)) => Some(e.select(same, machine_jit::V::K(c0), machine_jit::V::K(c1), machine_jit::N::U64)),
                    _ => None,
                }};
                if let Some(tc) = tc {{
                    e.tick(tc);
                }} else {{
                    let now = e.read(&machine_jit::Place::Mem {{ off: bus + b.now, n: machine_jit::N::U128, bump: 0 }});
                    let now = e.op(machine_jit::Op::Add, now, t, machine_jit::N::U128);
                    e.store(bus + b.now, machine_jit::N::U128, now);
                }}
                let ct = e.read(&machine_jit::Place::Mem {{ off: bus + b.code_time, n: machine_jit::N::U128, bump: 0 }});
                // (The step's code time starts at none: then its cycles are
                // the fetch's.)
                if ct == machine_jit::V::K(0) {{
                    ct_cyc = tc;
                }}
                let ct = e.op(machine_jit::Op::Add, ct, t, machine_jit::N::U128);
                e.store(bus + b.code_time, machine_jit::N::U128, ct);
                e.store(bus + b.direct_code, machine_jit::N::U64, machine_jit::V::K(u1 as u128));
                false
            }}
            (Some(b), _, _) => {{
                let _ = e.call_touching(Self::jit_fetch_c as *const () as usize, &[(machine_jit::V::K(j.page as u128), machine_jit::N::U64), (machine_jit::V::K(j.phys as u128), machine_jit::N::U64), (machine_jit::V::K(s.len as u128), machine_jit::N::U64)], machine_jit::N::U64, &[(std::mem::offset_of!(Self, bus) + b.code_time, 16), (std::mem::offset_of!(Self, bus) + b.direct_code, 8)]);
                false
            }}
            (None, _, _) => {{
                let q = Self::jit_quiet(&instr);
                if !q {{
                    let _ = e.call(Self::jit_fetch_c as *const () as usize, &[(machine_jit::V::K(j.page as u128), machine_jit::N::U64), (machine_jit::V::K(j.phys as u128), machine_jit::N::U64), (machine_jit::V::K(s.len as u128), machine_jit::N::U64)], machine_jit::N::U64, true);
                }}
                q
            }}
        }};
        // The instruction: its timing entry on the state before it, then
        // its semantics.
        // `last`, as `run_<name>` sets it before the instruction runs (a
        // fault delivered from the semantics sees it).
        e.store(std::mem::offset_of!(Self, jit_last), machine_jit::N::U64, machine_jit::V::K(&s.instr as *const Instr as usize as u128));
        let Some((timing, body, after)) = Self::jit_instr(instr, j.phys) else {{ unreachable!() }};
        let mut entry = |e: &mut machine_jit::Em, timing: Box<dyn FnOnce(&mut machine_jit::Em) -> machine_jit::V>| {{
            let t = timing(e);
            e.store(std::mem::offset_of!(Self, entry_cycles), machine_jit::N::U32, t);
            let c = e.op(machine_jit::Op::Add, t, machine_jit::V::K(s.prefix_cycles as u128), machine_jit::N::U32);
            e.store(std::mem::offset_of!(Self, cycles), machine_jit::N::U32, c);
        }};
        if after {{
            // The entry asks how the step ended: the prefixes' cycles while
            // it runs, as `run_<name>` keeps them.
            e.store(std::mem::offset_of!(Self, cycles), machine_jit::N::U32, machine_jit::V::K(s.prefix_cycles as u128));
            body(e);
            entry(e, timing);
        }} else {{
            entry(e, timing);
            body(e);
        }}
        let Some(b) = board else {{
            let r = e.call(Self::jit_retire_c as *const () as usize, &[
                (machine_jit::V::K(&s.instr as *const Instr as usize as u128), machine_jit::N::U64),
                (machine_jit::V::K(j.phys as u128), machine_jit::N::U64),
                (machine_jit::V::K(j.page as u128), machine_jit::N::U64),
                (machine_jit::V::K(j.stamp as u128), machine_jit::N::U64),
                (machine_jit::V::K(if quiet {{ s.len as u128 }} else {{ 0 }}), machine_jit::N::U64),
            ], machine_jit::N::U32, false);
            e.exit_on(r);
            return true;
        }};
        // Retired in host code: the time (serial: the fetch for a step that
        // reached no bus space, then the cycles, the code unit and time
        // starting over; overlapped: what execution adds beyond the
        // fetches), `last` (when the code returns), the devices when one is
        // due or an edge waits, the run's code, and whether the run goes on.
        let bus = std::mem::offset_of!(Self, bus);
        let cyc = e.read(&machine_jit::Place::Mem {{ off: std::mem::offset_of!(Self, cycles), n: machine_jit::N::U32, bump: 0 }});
        let now_p = machine_jit::Place::Mem {{ off: bus + b.now, n: machine_jit::N::U128, bump: 0 }};
        // In cycles, when the run counts them (`jit_cyc`) and the fetch is
        // whole cycles: no time in the machine (it holds it when it must).
        let cyc64 = e.cast(cyc, machine_jit::N::U32, machine_jit::N::U64);
        let ticks = if let Some(f) = approx_fetch {{
            Self::jit_cyc(e, &b, f).map(|fc| e.op(machine_jit::Op::Add, cyc64, machine_jit::V::K(fc), machine_jit::N::U64))
        }} else if b.serial {{
            Self::jit_cyc(e, &b, if quiet {{ fetch.unwrap_or(0) }} else {{ 0 }}).map(|fc| e.op(machine_jit::Op::Add, cyc64, machine_jit::V::K(fc), machine_jit::N::U64))
        }} else {{
            ct_cyc.map(|cc| {{
                let over = e.op(machine_jit::Op::Gt, cyc64, cc, machine_jit::N::U64);
                let d = e.op(machine_jit::Op::Sub, cyc64, cc, machine_jit::N::U64);
                e.select(over, d, machine_jit::V::K(0), machine_jit::N::U64)
            }})
        }};
        let now = if let Some(t) = ticks {{
            if approx_fetch.is_some() || b.serial {{
                e.store(bus + b.direct_code, machine_jit::N::U64, machine_jit::V::K(u64::MAX as u128));
            }}
            e.tick(t);
            None
        }} else {{
        let cyc = e.cast(cyc, machine_jit::N::U32, machine_jit::N::U128);
        let x = e.op(machine_jit::Op::Mul, cyc, machine_jit::V::K(b.clock), machine_jit::N::U128);
        let now = e.read(&now_p);
        let now = if let Some(f) = approx_fetch {{
            let t = e.op(machine_jit::Op::Add, x, machine_jit::V::K(f), machine_jit::N::U128);
            e.store(bus + b.direct_code, machine_jit::N::U64, machine_jit::V::K(u64::MAX as u128));
            e.op(machine_jit::Op::Add, now, t, machine_jit::N::U128)
        }} else if b.serial {{
            let t = if quiet {{ e.op(machine_jit::Op::Add, x, machine_jit::V::K(fetch.unwrap_or(0)), machine_jit::N::U128) }} else {{ x }};
            e.store(bus + b.direct_code, machine_jit::N::U64, machine_jit::V::K(u64::MAX as u128));
            e.op(machine_jit::Op::Add, now, t, machine_jit::N::U128)
        }} else {{
            let ct = e.read(&machine_jit::Place::Mem {{ off: bus + b.code_time, n: machine_jit::N::U128, bump: 0 }});
            let over = e.op(machine_jit::Op::Gt, x, ct, machine_jit::N::U128);
            let d = e.op(machine_jit::Op::Sub, x, ct, machine_jit::N::U128);
            let add = e.select(over, d, machine_jit::V::K(0), machine_jit::N::U128);
            e.op(machine_jit::Op::Add, now, add, machine_jit::N::U128)
        }};
        e.store(bus + b.now, machine_jit::N::U128, now);
        Some(now)
        }};
        e.store(bus + b.code_time, machine_jit::N::U128, machine_jit::V::K(0));
        e.store(std::mem::offset_of!(Self, decoded), machine_jit::N::Bool, machine_jit::V::K(1));
        // An edge can wait only after a bus access (the last `after_step`
        // delivered the ones before).
        let edges = b.events && !Self::jit_quiet(&instr);
        let pending = |e: &mut machine_jit::Em| if edges {{ e.load(bus + b.pending, machine_jit::N::Bool) }} else {{ machine_jit::V::K(0) }};
        // (Made only where it is checked: it may read the time.)
        let need = |e: &mut machine_jit::Em| match (b.due, b.idle) {{
            // (Counted in cycles: the clock's deadline is `due`.)
            (Some(_), _) if now.is_none() && {quantum}u128 == 0 => {{
                let late = e.late();
                let p = pending(e);
                e.op(machine_jit::Op::Or, late, p, machine_jit::N::Bool)
            }}
            (Some(due), _) => {{
                let now = match now {{ Some(n) => n, None => e.read(&now_p) }};
                let d = e.load(bus + due, machine_jit::N::U128);
                let late = e.op(machine_jit::Op::Ge, now, d, machine_jit::N::U128);
                let p = pending(e);
                e.op(machine_jit::Op::Or, late, p, machine_jit::N::Bool)
            }}
            (None, true) => pending(e),
            (None, false) => machine_jit::V::K(1),
        }};
        // The devices change no processor field: a rare path.
        // `sync: run`: the devices are caught up when the run returns.
        // The run's code: changed only by what this step called or wrote
        // (direct writes that hit code note it), or by devices (checked as
        // they run, below).
        let code_check = |e: &mut machine_jit::Em| {{
        if !b.writes.is_empty() {{
                let mut w = machine_jit::V::K(0);
                for off in b.writes {{
                    let x = e.load(bus + off, machine_jit::N::U64);
                    w = e.op(machine_jit::Op::Add, w, x, machine_jit::N::U64);
                }}
                let tw = e.load(std::mem::offset_of!(Self, tr_w), machine_jit::N::U64);
                let changed = e.op(machine_jit::Op::Ne, w, tw, machine_jit::N::U64);
                e.cold(changed, |e| {{
                    let r = e.call(Self::jit_code_c as *const () as usize, &[(machine_jit::V::K(j.page as u128), machine_jit::N::U64), (machine_jit::V::K(j.stamp as u128), machine_jit::N::U64)], machine_jit::N::U32, true);
                    let bad = e.op(machine_jit::Op::Ne, r, machine_jit::V::K(0), machine_jit::N::U32);
                    e.exit_if(bad, true, |_| {{}});
                }});
            }}
        }};
        if e.code_writes() != code0 {{
            code_check(e);
        }}
        if let Some(times) = times {{
            // A fill or a copy with a kernel: past the second element (the
            // first may be timed apart), the time is noted; a period of
            // elements later, the kernel makes what it can of the rest, in
            // whole periods, each taking the time the one measured took.
            let kern = Self::jit_bulk_kernel(&instr);
            let per = B::jit_mems().iter().map(|m| m.unit).max().unwrap_or(1).max(1);
            if kern != 0 && ({repeat_bulk} as u64) > per + 3 {{
                let t = e.get(times);
                let t = e.op(machine_jit::Op::Add, t, machine_jit::V::K(1), machine_jit::N::U32);
                let is_mark = e.op(machine_jit::Op::Eq, t, machine_jit::V::K(2), machine_jit::N::U32);
                e.cold(is_mark, |e| {{
                    let now = e.read(&now_p);
                    let now_lo = e.cast(now, machine_jit::N::U128, machine_jit::N::U64);
                    e.store(std::mem::offset_of!(Self, jit_bulk_mark), machine_jit::N::U64, now_lo);
                }});
                let pc = e.read(&machine_jit::Place::Mem {{ off: std::mem::offset_of!(Self, r.{pc0n}), n: {pc_n}, bump: 0 }});
                let again = e.op(machine_jit::Op::Eq, pc, machine_jit::V::K((j.pc & {pc_mask:#x}) as u128), {pc_n});
                let at = e.op(machine_jit::Op::Eq, t, machine_jit::V::K(2 + per as u128), machine_jit::N::U32);
                let go = e.op(machine_jit::Op::And, at, again, machine_jit::N::Bool);
                e.cold(go, |e| {{
                    let now = e.read(&now_p);
                    let now_lo = e.cast(now, machine_jit::N::U128, machine_jit::N::U64);
                    let left = e.op(machine_jit::Op::Sub, machine_jit::V::K({repeat_bulk} as u128), t, machine_jit::N::U32);
                    let left = e.cast(left, machine_jit::N::U32, machine_jit::N::U64);
                    let r = e.call(kern, &[(left, machine_jit::N::U64), (now_lo, machine_jit::N::U64), (machine_jit::V::K(per as u128), machine_jit::N::U64)], machine_jit::N::U64, false);
                    let dt = e.load(std::mem::offset_of!(Self, jit_bulk_dt), machine_jit::N::U128);
                    let n0 = e.read(&now_p);
                    let n1 = e.op(machine_jit::Op::Add, n0, dt, machine_jit::N::U128);
                    e.store(bus + b.now, machine_jit::N::U128, n1);
                    let r = e.cast(r, machine_jit::N::U64, machine_jit::N::U32);
                    let tt = e.get(times);
                    let tt = e.op(machine_jit::Op::Add, tt, r, machine_jit::N::U32);
                    e.set(times, tt);
                    e.add_made(r);
                    // (Its writes may have hit the run's code.)
                    code_check(e);
                }});
            }}
            let pc = e.read(&machine_jit::Place::Mem {{ off: std::mem::offset_of!(Self, r.{pc0n}), n: {pc_n}, bump: 0 }});
            let again = e.op(machine_jit::Op::Eq, pc, machine_jit::V::K((j.pc & {pc_mask:#x}) as u128), {pc_n});
            let ep = e.load(std::mem::offset_of!(Self, tr_epoch), machine_jit::N::U64);
            let ep0 = e.load(std::mem::offset_of!(Self, tr_ep0), machine_jit::N::U64);
            let same = e.op(machine_jit::Op::Eq, ep, ep0, machine_jit::N::U64);
            let h = e.read(&machine_jit::Place::Mem {{ off: std::mem::offset_of!(Self, halted), n: machine_jit::N::Bool, bump: 0 }});
            let running = e.not(h, machine_jit::N::Bool);
            let t = e.get(times);
            let t = e.op(machine_jit::Op::Add, t, machine_jit::V::K(1), machine_jit::N::U32);
            e.set(times, t);
            // `repeat: bulk`: until a device is due or an edge waits (the
            // run then ends here, and they are seen); `bulk(n)`: n times.
            let more = if {repeat_due} {{
                let late = if e.clocked() {{
                    e.late()
                }} else if {quantum}u128 != 0 {{
                    let now = e.read(&now_p);
                    let at = e.read(&machine_jit::Place::Mem {{ off: std::mem::offset_of!(Self, jit_sync_at), n: machine_jit::N::U128, bump: 0 }});
                    e.op(machine_jit::Op::Ge, now, at, machine_jit::N::U128)
                }} else if let Some(due) = b.due {{
                    let now = e.read(&now_p);
                    let d = e.load(bus + due, machine_jit::N::U128);
                    e.op(machine_jit::Op::Ge, now, d, machine_jit::N::U128)
                }} else {{
                    machine_jit::V::K(0)
                }};
                let p = pending(e);
                let stop = e.op(machine_jit::Op::Or, late, p, machine_jit::N::Bool);
                e.not(stop, machine_jit::N::Bool)
            }} else {{
                e.op(machine_jit::Op::Lt, t, machine_jit::V::K({repeat_bulk} as u128), machine_jit::N::U32)
            }};
            let c = e.op(machine_jit::Op::And, again, same, machine_jit::N::Bool);
            let c = e.op(machine_jit::Op::And, c, running, machine_jit::N::Bool);
            let c = e.op(machine_jit::Op::And, c, more, machine_jit::N::Bool);
            e.repeat_tail(head, c);
        }}
        if !{sync_run} && {quantum}u128 == 0 && (!{sync_block} || j.last) {{
            // (`sync: block`: at the run's last step only, where it goes
            // round or returns.)
            let need = need(e);
            e.cold(need, |e| {{
                let _ = e.call(Self::jit_idle_c as *const () as usize, &[], machine_jit::N::U64, true);
                code_check(e);
            }});
        }}
        // `sync: quantum(...)`: the devices at the run's last step (where it
        // goes round or returns), once the quantum since the last catch-up
        // has passed or one is due.
        if {quantum}u128 != 0 && j.last {{
            // (Counted in cycles: the clock's deadline is the catch-up.)
            let late = match now {{
                None => e.late(),
                Some(now) => {{
                    let at = e.read(&machine_jit::Place::Mem {{ off: std::mem::offset_of!(Self, jit_sync_at), n: machine_jit::N::U128, bump: 0 }});
                    e.op(machine_jit::Op::Ge, now, at, machine_jit::N::U128)
                }}
            }};
            // An edge waiting is delivered at once, and a device the step
            // touched (made due: its deadline to be worked out again) is
            // caught up.
            let p = pending(e);
            let late = e.op(machine_jit::Op::Or, late, p, machine_jit::N::Bool);
            let late = match b.due {{
                Some(due) => {{
                    let d = e.load(bus + due, machine_jit::N::U128);
                    let z = e.op(machine_jit::Op::Eq, d, machine_jit::V::K(0), machine_jit::N::U128);
                    e.op(machine_jit::Op::Or, late, z, machine_jit::N::Bool)
                }}
                None => late,
            }};
            e.cold(late, |e| {{
                // (It sets when the devices are next due: the code forgets the
                // time it knew.)
                let _ = e.call_reading(Self::jit_quantum_c as *const () as usize, &[], machine_jit::N::U64, &[(std::mem::offset_of!(Self, jit_sync_at), 16)]);
                code_check(e);
            }});
        }}
        let h = e.read(&machine_jit::Place::Mem {{ off: std::mem::offset_of!(Self, halted), n: machine_jit::N::Bool, bump: 0 }});
        // The run's end, leading back to its start: round again, as the
        // caller would enter it (by its link: the epoch unchanged, so the
        // key and the code's page hold), while the steps allowed last.
        // Not with `sync: run` (the devices wait for the run's end).
        if j.last && !{sync_run} {{
            let pc = e.read(&machine_jit::Place::Mem {{ off: std::mem::offset_of!(Self, r.{pc0n}), n: {pc_n}, bump: 0 }});
            let again = e.op(machine_jit::Op::Eq, pc, machine_jit::V::K((j.start & {pc_mask:#x}) as u128), {pc_n});
            let ep = e.load(std::mem::offset_of!(Self, tr_epoch), machine_jit::N::U64);
            let ep0 = e.load(std::mem::offset_of!(Self, tr_ep0), machine_jit::N::U64);
            let same = e.op(machine_jit::Op::Eq, ep, ep0, machine_jit::N::U64);
            let made = e.made_through();
            let made = e.cast(made, machine_jit::N::U32, machine_jit::N::U64);
            let need = e.op(machine_jit::Op::Add, made, machine_jit::V::K(e.steps() as u128), machine_jit::N::U64);
            let left = e.load(std::mem::offset_of!(Self, jit_left), machine_jit::N::U64);
            let fits = e.op(machine_jit::Op::Le, need, left, machine_jit::N::U64);
            let running = e.not(h, machine_jit::N::Bool);
            let c = e.op(machine_jit::Op::And, again, same, machine_jit::N::Bool);
            let c = e.op(machine_jit::Op::And, c, fits, machine_jit::N::Bool);
            let c = e.op(machine_jit::Op::And, c, running, machine_jit::N::Bool);
            e.loop_back(c, |e| {{
                e.store(std::mem::offset_of!(Self, tr_on), machine_jit::N::Bool, machine_jit::V::K(1));
                e.store(std::mem::offset_of!(Self, tr_idx), machine_jit::N::U32, machine_jit::V::K(0));
            }});
        }}
        let on = e.read(&machine_jit::Place::Mem {{ off: std::mem::offset_of!(Self, tr_on), n: machine_jit::N::Bool, bump: 0 }});
        let off = e.not(on, machine_jit::N::Bool);
        let stop = e.op(machine_jit::Op::Or, h, off, machine_jit::N::Bool);
        e.exit_if(stop, true, |_| {{}});
        true
    }}
    /// A lowered instruction: its timing entry and its semantics, to emit,
    /// and whether the entry follows the semantics (`repeats()`).
    #[allow(unused_variables, clippy::all)]
    fn jit_instr(i: Instr, phys: u64) -> Option<(Box<dyn FnOnce(&mut machine_jit::Em) -> machine_jit::V>, Box<dyn FnOnce(&mut machine_jit::Em)>, bool)> {{
        match i {{
{arms}            _ => None,
        }}
    }}
    /// Whether `i`'s instruction reaches no bus space (its fetch is charged
    /// with its retirement).
    fn jit_quiet(i: &Instr) -> bool {{
        match i {{
{quiet_arms}            _ => false,
        }}
    }}
    /// Whether `i`'s instruction may set the pc back to itself (and so run
    /// again).
    #[allow(dead_code)]
    fn jit_may_repeat(i: &Instr) -> bool {{
        match i {{
{repeat_arms}            _ => false,
        }}
    }}
    /// The shim of `i`'s kernel (`bulk_<name>`: its elements as a block),
    /// 0 when it has none.
    #[allow(dead_code)]
    fn jit_bulk_kernel(i: &Instr) -> usize {{
        match i {{
{kernel_arms}            _ => 0,
        }}
    }}
    /// Whether `i`'s instruction lowers.
    fn jit_lowers(i: &Instr) -> bool {{
        match i {{
{lowers_arms}            _ => false,
        }}
    }}
"#,
            pc0n = ident(&pc0.name),
            sync_run = self.sync_run,
            timing_approx = self.timing_approx,
            events_block = self.events_block,
            quantum = self.sync_quantum.unwrap_or(0),
            sync_block = self.sync_block,
            repeat_bulk = self.repeat_bulk,
            repeat_due = self.repeat_due,
            boundary_wakes = ist.boundary.is_some_and(|b| {
                let mut halts = false;
                machine_sema::hir::visit_block(&self.fns[&b].body, &mut |st| halts |= matches!(st.kind, machine_sema::hir::StmtKind::Halt), &mut |_| {});
                halts
            }),
            quiet_arms = quiet.iter().map(|n| format!("            Instr::{n}(_) => true,\n")).collect::<String>(),
            repeat_arms = repeaters.iter().map(|n| format!("            Instr::{n}(_) => true,\n")).collect::<String>(),
            kernel_arms = kernel_names.iter().map(|n| format!("            Instr::{n}(_) => Self::jit_bulk_{n}_c as *const () as usize,\n")).collect::<String>(),
            lowers_arms = lowered.iter().map(|n| format!("            Instr::{n}(_) => true,\n")).collect::<String>(),
        );
        // The kernels' shims: the elements made (and their time, in
        // `jit_bulk_dt`: whole periods, each the time of the one measured).
        // (`repeat: bulk`: no further than the devices' deadline.)
        let cap = if self.repeat_due { "m.jit_due_cap(max, now, per)" } else { "max" };
        for n in &kernel_names {
            let _ = write!(self.out, "    #[allow(clippy::all)]\n    extern \"C\" fn jit_bulk_{n}_c(m: *mut Self, max: u64, now: u64, per: u64) -> u64 {{\n        // SAFETY: called by compiled code with its machine.\n        let m = unsafe {{ &mut *m }};\n        let made = m.bulk_{n}({cap}, per);\n        m.jit_bulk_dt = (made / per.max(1)) as u128 * now.wrapping_sub(m.jit_bulk_mark) as u128;\n        made\n    }}\n");
        }
        // The split charge of a data access (`split =>`), lowered.
        let split = match &self.isa.split_cycles {
            Some(t) => {
                let lx = super::lower::Lx { locals: &t.locals, ops: None, src: None, timing: true };
                let mut bind = String::new();
                for (i, p) in t.params.iter().enumerate() {
                    let _ = write!(bind, "let l{p} = e.local(machine_jit::N::U32, {}); ", if i == 0 { "a" } else { "w" });
                }
                match self.jexpr(&lx, &t.expr) {
                    Ok(x) => format!("{bind}{{ let v = {x}; e.cast(v, {}, machine_jit::N::U32) }}", self.jn(&t.expr.ty).unwrap_or("machine_jit::N::U32")),
                    Err(_) => "{ let _ = (a, w); e.trap(); machine_jit::V::K(0) }".to_string(),
                }
            }
            None => "{ let _ = (a, w); machine_jit::V::K(0) }".to_string(),
        };
        let _ = write!(self.out, "    /// The split charge of a data access at `a` of `w` bytes.\n    #[allow(unused_variables, clippy::all)]\n    fn jit_split(e: &mut machine_jit::Em, a: machine_jit::V, w: machine_jit::V) -> machine_jit::V {{\n        {split}\n    }}\n");
        // The speculated registers (the trace plan's), and the key's
        // registers' leaves.
        let mut places = Vec::new();
        let mut values = Vec::new();
        for d in &plan.spec {
            let (_, r) = self.regs[d];
            let n = ident(&r.name);
            places.push(format!("(std::mem::offset_of!(Self, r.{n}), {})", self.jn(&r.ty)?));
            values.push(format!("machine_jit::Native::bits(self.r.{n})"));
        }
        let mut key_assume = String::new();
        for d in &plan.key {
            let (_, r) = self.regs[d];
            let n = ident(&r.name);
            fn walk(g: &Gen, path: &str, ty: &Ty, out: &mut Vec<(String, Ty)>) {
                match ty {
                    Ty::Bundle(b) => {
                        for f in &g.bundles[b].fields {
                            walk(g, &format!("{path}.{}", ident(&f.name)), &f.ty, out);
                        }
                    }
                    t => out.push((path.to_string(), t.clone())),
                }
            }
            let mut leaves = Vec::new();
            walk(self, "", &r.ty, &mut leaves);
            for (path, t) in leaves {
                if matches!(t, Ty::Array(..) | Ty::Tuple(_)) {
                    continue;
                }
                let _ = writeln!(key_assume, "        e.assume(std::mem::offset_of!(Self, r.{n}{path}), {}, machine_jit::Native::bits(key.r_{n}{path}));", self.jn(&t)?);
            }
        }
        if plan.key_mode {
            let _ = writeln!(key_assume, "        e.assume(std::mem::offset_of!(Self, mode), {}, machine_jit::Native::bits(key.mode));", self.jn(&match self.isa.mode { Some(m) => Ty::Enum(m), None => Ty::UInt(8) })?);
        }
        let _ = write!(
            self.out,
            "    /// The registers compiled runs speculate on: where they are.\n    fn jit_spec_places() -> [(usize, machine_jit::N); {n}] {{\n        [{}]\n    }}\n    /// Their values now.\n    fn jit_spec_values(&self) -> [u128; {n}] {{\n        [{}]\n    }}\n    /// The run's key: its registers hold these values in the run.\n    #[allow(unused_variables)]\n    fn jit_key_assume(e: &mut machine_jit::Em, key: &TKey) {{\n{key_assume}    }}\n",
            places.join(", "),
            values.join(", "),
            n = plan.spec.len()
        );
        // Registers and latches narrower than their native type: a branch
        // on one of them knows more (a one-bit field not 1 is 0).
        let mut narrow = String::new();
        let mut regs: Vec<_> = self.regs.values().collect();
        regs.sort_by_key(|(i, _)| *i);
        for (_, r) in regs {
            if let (Ty::UInt(b), Ok(n)) = (&r.ty, self.jn(&r.ty)) {
                if *b < super::native(*b, false).0 {
                    let _ = writeln!(narrow, "        e.declare_bits(std::mem::offset_of!(Self, r.{}), {n}, {b});", ident(&r.name));
                }
            }
        }
        for l in &self.latches {
            if let (Ty::UInt(b), Ok(n)) = (&l.ty, self.jn(&l.ty)) {
                if *b < super::native(*b, false).0 {
                    let _ = writeln!(narrow, "        e.declare_bits(std::mem::offset_of!(Self, latch_{}), {n}, {b});", ident(&l.name));
                }
            }
        }
        let _ = write!(self.out, "    /// Registers and latches narrower than their native type.\n    #[allow(unused_variables)]\n    fn jit_declare_bits(e: &mut machine_jit::Em) {{\n{narrow}    }}\n");
        let _ = write!(self.out, "    /// The host instructions an outlined fn may take inlined (`JIT_TRIAL`\n    /// in the generator; `MACHINE_JIT_TRIAL` overrides it, for tuning).\n    #[allow(dead_code)]\n    fn jit_trial_budget() -> usize {{\n        static B: std::sync::OnceLock<usize> = std::sync::OnceLock::new();\n        *B.get_or_init(|| std::env::var(\"MACHINE_JIT_TRIAL\").ok().and_then(|v| v.parse().ok()).unwrap_or({JIT_TRIAL}))\n    }}\n");
        self.out.push_str(&out);
        Ok(())
    }
}
