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

use super::{ident, Gen, R};

/// Entries into a run before it is compiled.
const HOT: u32 = 64;

impl Gen<'_> {
    pub(crate) fn jit_fields(&self) -> &'static str {
        if !self.jit {
            return "";
        }
        "    /// The JIT core: the code generator, whether the step's prologue left\n    /// the step to the instruction, and a fault to return.\n    pub jit: machine_jit::Engine,\n    pub jit_go: bool,\n    pub jit_err: Option<Fault>,\n"
    }

    pub(crate) fn jit_init(&self) -> &'static str {
        if !self.jit {
            return "";
        }
        "            jit: machine_jit::Engine::default(),\n            jit_go: false,\n            jit_err: None,\n"
    }

    /// The trace's fields for compiled code (inside `Trace`).
    pub(crate) fn jit_trace_fields(&self) -> &'static str {
        if !self.jit {
            return "";
        }
        "    /// Compiled code for the run after its first instruction, what each\n    /// of its steps was compiled with, and how often the run has been\n    /// entered.\n    pub code: machine_jit::Code,\n    pub jit_steps: Vec<JitStep>,\n    pub hits: u32,\n"
    }

    pub(crate) fn jit_trace_init(&self) -> &'static str {
        if !self.jit {
            return "";
        }
        "code: machine_jit::Code::default(), jit_steps: Vec::new(), hits: 0, "
    }

    /// What a compiled step is compiled with (beside the trace types).
    pub(crate) fn jit_types(&self) -> &'static str {
        if !self.jit {
            return "";
        }
        "/// A compiled step's data: the step, and what checking and replaying it\n/// needs of its run (the pc it is at, its address, the run's page, stamp\n/// and slot, and whether it is the run's last).\n#[derive(Clone, Copy, Debug)]\npub struct JitStep {\n    pub step: TStep,\n    pub pc: u64,\n    pub phys: u64,\n    pub page: u32,\n    pub stamp: u32,\n    pub slot: u32,\n    pub last: bool,\n}\n"
    }

    /// The JIT core's methods (inside the machine's impl).
    pub(crate) fn emit_jit_fns(&mut self) -> R<()> {
        if !self.jit {
            return Ok(());
        }
        let ist = self.isa.istream.as_ref().unwrap();
        let sp = ident(&self.spaces[&ist.space].name);
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
                }}).collect();
                t.jit_steps.iter().map(|j| machine_jit::Call {{
                    run: Self::jit_step_fn(&j.step.instr),
                    arg: j as *const JitStep as usize,
                }}).collect()
            }}
            None => return,
        }};
        let code = self.jit.compile(&calls);
        if let Some(t) = self.traces[slot].as_mut() {{
            t.code = machine_jit::Code::new(code);
        }}
    }}
    /// Runs `c`, compiled code of this machine's current run, from step
    /// `from`; the steps it made.
    fn jit_enter(&mut self, c: usize, from: u32) -> u32 {{
        // SAFETY: `c` is this machine's compiled code for its current run,
        // which is live: runs are only replaced while stepping, not while
        // compiled code runs.
        let f: extern "C" fn(*mut Self, u32) -> u32 = unsafe {{ std::mem::transmute(c) }};
        f(self as *mut Self, from)
    }}
    /// A run, as far as it goes (at most `budget` steps): when the last one
    /// ended where a compiled run follows, that run from its start; else one
    /// step as `System::step` makes it, and when that entered a run, the
    /// rest of it compiled (once it has been entered `HOT` times). How many
    /// steps were made.
    pub fn jit_burst(&mut self, budget: u64) -> Result<u64, Fault> {{
        if !self.tr_on && self.tr_prev != u32::MAX && !self.halted && !self.in_reset {{
            if let Some((c, len)) = self.jit_follow() {{
                if budget >= len {{
                    let n = self.jit_enter(c, 0) as u64;
                    if let Some(e) = self.jit_err.take() {{
                        return Err(e);
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
            let code = match code {{
                Some(c) => Some(c),
                None if len > 0 && self.traces[slot].as_ref().is_some_and(|t| t.hits == {HOT}) => {{
                    self.jit_compile(slot);
                    self.traces[slot].as_ref().and_then(|t| t.code.addr())
                }}
                None => None,
            }};
            if let Some(c) = code {{
                if self.tr_on && len > 1 && budget >= len {{
                    n += self.jit_enter(c, 1) as u64;
                    if let Some(e) = self.jit_err.take() {{
                        return Err(e);
                    }}
                }}
            }}
        }}
        Ok(n)
    }}
"#
        );
        Ok(())
    }
}
