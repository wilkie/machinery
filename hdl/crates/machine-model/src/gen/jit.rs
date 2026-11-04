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
        "    /// The JIT core: the code generator, whether the step's prologue left\n    /// the step to the instruction, the instruction's result, a fault to\n    /// return, and where the instruction was.\n    pub jit: machine_jit::Engine,\n    pub jit_go: bool,\n    pub jit_res: Result<(), Fault>,\n    pub jit_err: Option<Fault>,\n    pub jit_start: u64,\n"
    }

    pub(crate) fn jit_init(&self) -> &'static str {
        if !self.jit {
            return "";
        }
        "            jit: machine_jit::Engine::default(),\n            jit_go: false,\n            jit_res: Ok(()),\n            jit_err: None,\n            jit_start: 0,\n"
    }

    /// The trace's fields for compiled code (inside `Trace`).
    pub(crate) fn jit_trace_fields(&self) -> &'static str {
        if !self.jit {
            return "";
        }
        "    /// Compiled code for the run after its first instruction, and how\n    /// often the run has been entered.\n    pub code: machine_jit::Code,\n    pub hits: u32,\n"
    }

    pub(crate) fn jit_trace_init(&self) -> &'static str {
        if !self.jit {
            return "";
        }
        "code: machine_jit::Code::default(), hits: 0, "
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
                "    extern \"C\" fn jit_run_{n}(m: *mut Self, i: *const Instr) -> u32 {{\n        // SAFETY: compiled code calls this with the machine it was entered\n        // with and a step of one of its runs, both live for the call.\n        let (m, i) = unsafe {{ (&mut *m, *i) }};\n        if let Instr::{n}(ops) = i {{ m.jit_res = m.run_{n}(ops); }}\n        0\n    }}\n"
            );
            let _ = writeln!(fns, "            Instr::{n}(_) => Self::jit_run_{n} as *const () as usize,");
        }
        let _ = write!(
            self.out,
            r#"{shims}    /// The handler a compiled step calls for `i`.
    fn jit_run_fn(i: &Instr) -> usize {{
        match i {{
{fns}        }}
    }}
    extern "C" fn jit_pre_c(m: *mut Self) -> u32 {{
        // SAFETY: as for the handlers.
        unsafe {{ &mut *m }}.jit_pre()
    }}
    extern "C" fn jit_post_c(m: *mut Self) -> u32 {{
        // SAFETY: as for the handlers.
        unsafe {{ &mut *m }}.jit_post()
    }}
    /// Before a compiled step: 0 to run it, 1 when the step ended another
    /// way (an interrupt taken), 2 when the run cannot go on.
    #[inline]
    fn jit_pre(&mut self) -> u32 {{
        if !self.trace_check() {{
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
        let (phys, _, page, len) = self.trace_restore();
        self.tr_phys = phys;
        self.restarted = phys == self.last_start;
        self.last_start = phys;
        self.bus.{sp}_fetch_page(page, phys, len as u32);
        self.jit_start = phys;
        0
    }}
    /// After a compiled step: 0 to go on, 1 to stop.
    #[inline]
    fn jit_post(&mut self) -> u32 {{
        let r = std::mem::replace(&mut self.jit_res, Ok(()));
        let r = self.jit_tail(self.jit_start, r);
        self.bus.after_step();
        if let Err(f) = r {{
            self.jit_err = Some(f);
            return 1;
        }}
        u32::from(self.halted || !self.tr_on)
    }}
    /// Compiles the run in `slot` (after its first instruction).
    fn jit_compile(&mut self, slot: usize) {{
        if self.jit.count() >= machine_jit::LIMIT {{
            self.jit.clear();
            for t in self.traces.iter_mut().flatten() {{ t.code = machine_jit::Code::default(); }}
        }}
        let calls: Vec<machine_jit::Call> = match self.traces[slot].as_ref() {{
            Some(t) => t.steps[1..].iter().map(|s| machine_jit::Call {{
                pre: Self::jit_pre_c as *const () as usize,
                run: Self::jit_run_fn(&s.instr),
                arg: &s.instr as *const Instr as usize,
                post: Self::jit_post_c as *const () as usize,
            }}).collect(),
            None => return,
        }};
        let code = self.jit.compile(&calls);
        if let Some(t) = self.traces[slot].as_mut() {{
            t.code = machine_jit::Code::new(code);
        }}
    }}
    /// One step as `System::step` makes it, then, when it entered a run, the
    /// rest of the run compiled (at most `budget` steps in all); how many
    /// steps were made.
    pub fn jit_burst(&mut self, budget: u64) -> Result<u64, Fault> {{
        let was_halted = self.halted;
        let r = self.step();
        if was_halted && self.halted {{ self.bus.halted_cycle(); }}
        self.bus.after_step();
        r?;
        let mut n = 1;
        if self.tr_on && self.tr_idx == 1 && budget > 1 {{
            let slot = self.tr_cur as usize;
            let (code, len) = match self.traces[slot].as_mut() {{
                Some(t) => {{
                    t.hits = t.hits.saturating_add(1);
                    (t.code.addr(), t.steps.len() as u64)
                }}
                None => (None, 0),
            }};
            let code = match code {{
                Some(c) => Some(c),
                None if len > 1 && self.traces[slot].as_ref().is_some_and(|t| t.hits == {HOT}) => {{
                    self.jit_compile(slot);
                    self.traces[slot].as_ref().and_then(|t| t.code.addr())
                }}
                None => None,
            }};
            if let Some(c) = code {{
                if budget - 1 >= len - 1 {{
                    // SAFETY: `c` is this machine's compiled code for the run
                    // in `slot`, which is live: runs are only replaced while
                    // stepping, not while compiled code runs.
                    let f: extern "C" fn(*mut Self) -> u32 = unsafe {{ std::mem::transmute(c) }};
                    n += f(self as *mut Self) as u64;
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
