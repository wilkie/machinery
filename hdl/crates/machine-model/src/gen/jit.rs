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
use machine_sema::ty::Ty;

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
        let code = match self.traces[slot].as_ref() {{
            Some(t) => {{
                let steps = &t.jit_steps;
                let mut lowered = vec![false; steps.len()];
                let _ = &calls;
                self.jit.compile_with(steps.len(), 2, |e, k| {{
                    let checked = k == 0 || lowered[k - 1];
                    lowered[k] = Self::jit_emit_step(e, &steps[k], k, checked);
                }})
            }}
            None => None,
        }};
        if let Some(t) = self.traces[slot].as_mut() {{
            t.code = machine_jit::Code::new(code);
        }}
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
        let _ = self.jit_lower_kinds();
        let _ = self.jit_lower_fns();
        out.push_str(&self.jit_lower_kinds());
        out.push_str(&self.jit_lower_fns());
        // The instructions that lower.
        let mut lowered = Vec::new();
        let mut quiet = Vec::new();
        for i in self.isa.instructions.iter().filter(|i| !i.prefix) {
            match self.jinstr(i) {
                Ok(t) => {
                    out.push_str(&t);
                    lowered.push(ident(&i.name));
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
        let gate_lowers = ist.reset.is_none() && ist.boundary.is_none_or(|b| jf.contains(&b));
        let mut latch_resets = String::new();
        let mut latches_ok = true;
        for l in &self.isa.latches {
            let lx = super::lower::Lx { locals: &[], ops: None, src: None };
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
        let lowers = gate_lowers && latches_ok && ist.lock.is_none();
        let boundary = match ist.boundary {
            _ if !lowers => String::new(),
            Some(b) => format!(
                "        let taken = Self::jf_{}(e);\n        e.exit_if(taken, true, |e| {{ let _ = e.call(Self::jit_interrupt_c as *const () as usize, &[], machine_jit::N::U64, false); }});\n",
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
        for l in &self.isa.latches {
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
            let _ = writeln!(arms, "            Instr::{n}(ops) => {{ let t = Self::jt_{n}(e, &ops); Some((t, Box::new(move |e: &mut machine_jit::Em| Self::jx_{n}(e, &ops)) as Box<dyn FnOnce(&mut machine_jit::Em)>)) }}");
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
            if self.bus.{sp}_code_stamp(page) != stamp {{
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
    /// A step made by its prologue alone (shut down, halted).
    extern "C" fn jit_idle_c(m: *mut Self) -> u64 {{
        // SAFETY: called by compiled code with its machine.
        let m = unsafe {{ &mut *m }};
        m.bus.after_step();
        1
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
    fn jit_emit_step(e: &mut machine_jit::Em, j: &JitStep, k: usize, checked: bool) -> bool {{
        let s = &j.step;
        if B::SCHEDULE || !{lowers} {{
            let r = e.call(Self::jit_step_fn(&s.instr), &[(machine_jit::V::K(j as *const JitStep as usize as u128), machine_jit::N::U64)], machine_jit::N::U32, false);
            e.exit_on(r);
            return false;
        }}
        let instr = s.instr;
        // Is the instruction lowered? (Its timing is emitted after the
        // prologue, so only the shape is asked here.)
        if !Self::jit_lowers(&instr) {{
            let r = e.call(Self::jit_step_fn(&s.instr), &[(machine_jit::V::K(j as *const JitStep as usize as u128), machine_jit::N::U64)], machine_jit::N::U32, false);
            e.exit_on(r);
            return false;
        }}
        e.set_epochs([std::mem::offset_of!(Self, tr_epoch), std::mem::offset_of!(Self, tr_aepoch), std::mem::offset_of!(Self, tr_kepoch)]);
        // The run's check: epoch and pc (the code, after the step before).
        let ep = e.load(std::mem::offset_of!(Self, tr_epoch), machine_jit::N::U64);
        let ep0 = e.load(std::mem::offset_of!(Self, tr_ep0), machine_jit::N::U64);
        let pc = e.read(&machine_jit::Place::Mem {{ off: std::mem::offset_of!(Self, r.{pc0n}), n: {pc_n}, bump: 0 }});
        let c1 = e.op(machine_jit::Op::Ne, ep, ep0, machine_jit::N::U64);
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
        // `step`'s prologue.
        e.store(std::mem::offset_of!(Self, cycles), machine_jit::N::U32, machine_jit::V::K(0));
        e.store(std::mem::offset_of!(Self, accesses), machine_jit::N::U8, machine_jit::V::K(0));
        e.store(std::mem::offset_of!(Self, win_len), machine_jit::N::U32, machine_jit::V::K(0));
{latch_resets}        let sd = e.load(std::mem::offset_of!(Self, shutdown), machine_jit::N::Bool);
        e.exit_if(sd, true, |e| {{ let _ = e.call(Self::jit_idle_c as *const () as usize, &[], machine_jit::N::U64, false); }});
{boundary}        let h = e.read(&machine_jit::Place::Mem {{ off: std::mem::offset_of!(Self, halted), n: machine_jit::N::Bool, bump: 0 }});
        e.exit_if(h, true, |e| {{ let _ = e.call(Self::jit_idle_c as *const () as usize, &[], machine_jit::N::U64, false); }});
        e.store(std::mem::offset_of!(Self, tr_rec_now), machine_jit::N::Bool, machine_jit::V::K(0));
        e.store(std::mem::offset_of!(Self, decoded), machine_jit::N::Bool, machine_jit::V::K(0));
        // The state as decoding left it.
        e.store(std::mem::offset_of!(Self, fetched), machine_jit::N::U8, machine_jit::V::K(s.len as u128));
        e.store(std::mem::offset_of!(Self, known_opcode), machine_jit::N::Bool, machine_jit::V::K(s.known as u128));
        e.store(std::mem::offset_of!(Self, prefix_cycles), machine_jit::N::U32, machine_jit::V::K(s.prefix_cycles as u128));
{restore}        e.store(std::mem::offset_of!(Self, tr_idx), machine_jit::N::U32, machine_jit::V::K(k as u128 + 1));
{opcodes_store}{opcode}        if j.last {{
            e.store(std::mem::offset_of!(Self, tr_on), machine_jit::N::Bool, machine_jit::V::K(0));
            e.store(std::mem::offset_of!(Self, tr_prev), machine_jit::N::U32, machine_jit::V::K(j.slot as u128));
        }}
        e.store(std::mem::offset_of!(Self, tr_phys), machine_jit::N::U64, machine_jit::V::K(j.phys as u128));
        let ls = e.load(std::mem::offset_of!(Self, last_start), machine_jit::N::U64);
        let rs = e.op(machine_jit::Op::Eq, ls, machine_jit::V::K(j.phys as u128), machine_jit::N::U64);
        e.store(std::mem::offset_of!(Self, restarted), machine_jit::N::Bool, rs);
        e.store(std::mem::offset_of!(Self, last_start), machine_jit::N::U64, machine_jit::V::K(j.phys as u128));
        let quiet = Self::jit_quiet(&instr);
        if !quiet {{
            let _ = e.call(Self::jit_fetch_c as *const () as usize, &[(machine_jit::V::K(j.page as u128), machine_jit::N::U64), (machine_jit::V::K(j.phys as u128), machine_jit::N::U64), (machine_jit::V::K(s.len as u128), machine_jit::N::U64)], machine_jit::N::U64, true);
        }}
        // The instruction: its timing entry on the state before it, then
        // its semantics.
        let Some((t, body)) = Self::jit_instr(e, instr) else {{ unreachable!() }};
        e.store(std::mem::offset_of!(Self, entry_cycles), machine_jit::N::U32, t);
        let c = e.op(machine_jit::Op::Add, t, machine_jit::V::K(s.prefix_cycles as u128), machine_jit::N::U32);
        e.store(std::mem::offset_of!(Self, cycles), machine_jit::N::U32, c);
        body(e);
        let r = e.call(Self::jit_retire_c as *const () as usize, &[
            (machine_jit::V::K(&s.instr as *const Instr as usize as u128), machine_jit::N::U64),
            (machine_jit::V::K(j.phys as u128), machine_jit::N::U64),
            (machine_jit::V::K(j.page as u128), machine_jit::N::U64),
            (machine_jit::V::K(j.stamp as u128), machine_jit::N::U64),
            (machine_jit::V::K(if quiet {{ s.len as u128 }} else {{ 0 }}), machine_jit::N::U64),
        ], machine_jit::N::U32, false);
        e.exit_on(r);
        true
    }}
    /// A lowered instruction: its timing entry (emitted now) and its
    /// semantics (to emit).
    #[allow(unused_variables, clippy::all)]
    fn jit_instr(e: &mut machine_jit::Em, i: Instr) -> Option<(machine_jit::V, Box<dyn FnOnce(&mut machine_jit::Em)>)> {{
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
    /// Whether `i`'s instruction lowers.
    fn jit_lowers(i: &Instr) -> bool {{
        match i {{
{lowers_arms}            _ => false,
        }}
    }}
"#,
            pc0n = ident(&pc0.name),
            quiet_arms = quiet.iter().map(|n| format!("            Instr::{n}(_) => true,\n")).collect::<String>(),
            lowers_arms = lowered.iter().map(|n| format!("            Instr::{n}(_) => true,\n")).collect::<String>(),
        );
        self.out.push_str(&out);
        Ok(())
    }
}
