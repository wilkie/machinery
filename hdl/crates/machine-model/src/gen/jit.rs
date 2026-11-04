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
        "    /// The JIT core: the code generator, whether the step's prologue left\n    /// the step to the instruction, and a fault to return.\n    pub jit: machine_jit::Engine,\n    pub jit_go: bool,\n    pub jit_err: Option<Fault>,\n    /// The instruction of the last step compiled code made and retired\n    /// itself (a pointer into its run), 0 when none since: `last` is set\n    /// from it when the code returns.\n    pub jit_last: u64,\n    /// The board's direct memory pointers (`Bus::jit_mem_ptrs`), as of\n    /// entering compiled code.\n    pub jit_ptrs: [usize; 8],\n    /// A fn compiled code called raised a fault, delivered: the run ends.\n    pub jit_pending: bool,\n"
    }

    pub(crate) fn jit_init(&self) -> &'static str {
        if !self.jit {
            return "";
        }
        "            jit: machine_jit::Engine::default(),\n            jit_go: false,\n            jit_err: None,\n            jit_last: 0,\n            jit_ptrs: [0; 8],\n            jit_pending: false,\n"
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
        let sync_run = self.sync_run;
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
                self.jit.compile_with(steps.len(), 2, |e, k| {{
                    let checked = k == 0 || lowered[k - 1];
                    // Only reached from a lowered step before it.
                    let follows = k >= 2 && lowered[k - 1];
                    lowered[k] = Self::jit_emit_step(e, &steps[k], k, checked, follows, fetch[k], units[k]);
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
        self.bus.jit_mem_ptrs(&mut self.jit_ptrs);
        // SAFETY: `c` is this machine's compiled code for its current run,
        // which is live: runs are only replaced while stepping, not while
        // compiled code runs.
        let f: extern "C" fn(*mut Self, u32) -> u32 = unsafe {{ std::mem::transmute(c) }};
        let n = f(self as *mut Self, from);
        self.jit_pending = false;
        if {sync_run} {{
            // `sync: run`: the devices, once for the run.
            self.bus.after_step();
        }}
        if self.jit_last != 0 {{
            // SAFETY: a step of this machine's run, which is live.
            self.last = Some(unsafe {{ *(self.jit_last as usize as *const Instr) }});
            self.last_fault = None;
            self.jit_last = 0;
        }}
        n
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
                    format!("        // The reset line, or leaving reset: as the interpreter's\n        // prologue makes it.\n        let rl = {lowered};\n        let ir = e.load(std::mem::offset_of!(Self, in_reset), machine_jit::N::Bool);\n        let any = e.op(machine_jit::Op::Or, rl, ir, machine_jit::N::Bool);\n        e.if_(any, |e| {{ let r = e.call(Self::jit_reset_c as *const () as usize, &[], machine_jit::N::U32, false); e.exit_on(r); }}, |_| {{}});\n"),
                    format!("    /// The prologue's reset line check: 1 when the step was the reset (or\n    /// held in it), 0 to go on (out of reset, if it was).\n    extern \"C\" fn jit_reset_c(m: *mut Self) -> u64 {{\n        // SAFETY: called by compiled code with its machine.\n        let m = unsafe {{ &mut *m }};\n        if {call} {{\n            if !m.in_reset {{ m.reset(); m.bus.processor_reset(); m.in_reset = true; }}\n            m.halted = true;\n            m.bus.after_step();\n            return 1;\n        }}\n        if m.in_reset {{ m.in_reset = false; m.halted = false; }}\n        0\n    }}\n"),
                )
            }
            _ => (String::new(), String::new()),
        };
        out.push_str(&reset_shim);
        let mut latch_resets = String::new();
        let mut latches_ok = true;
        for l in &self.isa.latches {
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
    extern "C" fn jit_code_hit_c(m: *mut Self, mem: u64, k: u64) -> u64 {{
        // SAFETY: called by compiled code with its machine.
        let m = unsafe {{ &mut *m }};
        m.bus.jit_code_hit(mem as u32, k as usize);
        0
    }}
    /// A byte read at `a` (a u64) in the istream space: in host code when it
    /// falls in the board's direct memory, else the `shim` call.
    #[allow(clippy::all)]
    fn jm_read8(e: &mut machine_jit::Em, a: machine_jit::V, shim: usize, pure: bool) -> machine_jit::V {{
        let mems = B::jit_mems();
        match B::jit_board() {{
            Some(b) if !mems.is_empty() => {{
                let a = e.op(machine_jit::Op::And, a, machine_jit::V::K(mems[0].amask as u128), machine_jit::N::U64);
                Self::jm_access(e, a, None, mems, b, shim, pure)
            }}
            _ => e.call(shim, &[(a, machine_jit::N::U64)], machine_jit::N::U8, pure),
        }}
    }}
    /// A byte write of `v` (a u8) at `a`, as `jm_read8`.
    #[allow(clippy::all)]
    fn jm_write8(e: &mut machine_jit::Em, a: machine_jit::V, v: machine_jit::V, shim: usize, pure: bool) {{
        let mems = B::jit_mems();
        match B::jit_board() {{
            Some(b) if !mems.is_empty() => {{
                let a = e.op(machine_jit::Op::And, a, machine_jit::V::K(mems[0].amask as u128), machine_jit::N::U64);
                let _ = Self::jm_access(e, a, Some(v), mems, b, shim, pure);
            }}
            _ => {{
                let v = e.cast(v, machine_jit::N::U8, machine_jit::N::U64);
                let _ = e.call(shim, &[(a, machine_jit::N::U64), (v, machine_jit::N::U64)], machine_jit::N::U64, pure);
            }}
        }}
    }}
    #[allow(clippy::all)]
    fn jm_access(e: &mut machine_jit::Em, a: machine_jit::V, v: Option<machine_jit::V>, mems: &'static [JitMem], b: JitBoard, shim: usize, pure: bool) -> machine_jit::V {{
        let Some((m, rest)) = mems.split_first() else {{
            return match v {{
                None => e.call(shim, &[(a, machine_jit::N::U64)], machine_jit::N::U8, pure),
                Some(v) => {{
                    let v = e.cast(v, machine_jit::N::U8, machine_jit::N::U64);
                    let _ = e.call(shim, &[(a, machine_jit::N::U64), (v, machine_jit::N::U64)], machine_jit::N::U64, pure);
                    machine_jit::V::K(0)
                }}
            }};
        }};
        if v.is_some() && m.write.is_none() {{
            return Self::jm_access(e, a, v, rest, b, shim, pure);
        }}
        let inside = if m.lo == 0 && m.hi >= m.amask {{
            machine_jit::V::K(1)
        }} else {{
            let ge = e.op(machine_jit::Op::Ge, a, machine_jit::V::K(m.lo as u128), machine_jit::N::U64);
            let le = e.op(machine_jit::Op::Le, a, machine_jit::V::K(m.hi as u128), machine_jit::N::U64);
            e.op(machine_jit::Op::And, ge, le, machine_jit::N::Bool)
        }};
        e.if_val(inside, machine_jit::N::U8, |e| {{
            let i = if m.mask == u64::MAX {{ a }} else {{ e.op(machine_jit::Op::And, a, machine_jit::V::K(m.mask as u128), machine_jit::N::U64) }};
            let base = e.read(&machine_jit::Place::Mem {{ off: std::mem::offset_of!(Self, jit_ptrs) + 8 * m.cells, n: machine_jit::N::U64, bump: 0 }});
            let p = e.op(machine_jit::Op::Add, base, i, machine_jit::N::U64);
            let (r, cost) = match v {{
                None => (e.load_at(p, machine_jit::N::U8), m.read),
                Some(v) => {{
                    e.store_at(p, machine_jit::N::U8, v);
                    // Recorded code written: its page's stamp moves on.
                    if let Some(c) = m.code {{
                        let cb = e.read(&machine_jit::Place::Mem {{ off: std::mem::offset_of!(Self, jit_ptrs) + 8 * c, n: machine_jit::N::U64, bump: 0 }});
                        let pg = e.op(machine_jit::Op::Shr, i, machine_jit::V::K(12), machine_jit::N::U64);
                        let off = e.op(machine_jit::Op::Shl, pg, machine_jit::V::K(2), machine_jit::N::U64);
                        let wp = e.op(machine_jit::Op::Add, cb, off, machine_jit::N::U64);
                        let word = e.load_at(wp, machine_jit::N::U32);
                        let blk = e.op(machine_jit::Op::Shr, i, machine_jit::V::K(7), machine_jit::N::U64);
                        let blk = e.op(machine_jit::Op::And, blk, machine_jit::V::K(31), machine_jit::N::U64);
                        let blk = e.cast(blk, machine_jit::N::U64, machine_jit::N::U32);
                        let bit = e.op(machine_jit::Op::Shl, machine_jit::V::K(1), blk, machine_jit::N::U32);
                        let hit = e.op(machine_jit::Op::And, word, bit, machine_jit::N::U32);
                        let hit = e.op(machine_jit::Op::Ne, hit, machine_jit::V::K(0), machine_jit::N::U32);
                        e.cold(hit, |e| {{
                            let _ = e.call(Self::jit_code_hit_c as *const () as usize, &[(machine_jit::V::K(m.mem as u128), machine_jit::N::U64), (i, machine_jit::N::U64)], machine_jit::N::U64, true);
                        }});
                    }}
                    (machine_jit::V::K(0), m.write.unwrap_or(0))
                }}
            }};
            let bus = std::mem::offset_of!(Self, bus);
            let now = e.read(&machine_jit::Place::Mem {{ off: bus + b.now, n: machine_jit::N::U128, bump: 0 }});
            let now = e.op(machine_jit::Op::Add, now, machine_jit::V::K(cost), machine_jit::N::U128);
            e.store(bus + b.now, machine_jit::N::U128, now);
            let acc = e.read(&machine_jit::Place::Mem {{ off: std::mem::offset_of!(Self, accesses), n: machine_jit::N::U8, bump: 0 }});
            let acc = e.op(machine_jit::Op::Add, acc, machine_jit::V::K(1), machine_jit::N::U8);
            e.store(std::mem::offset_of!(Self, accesses), machine_jit::N::U8, acc);
            r
        }}, |e| Self::jm_access(e, a, v, rest, b, shim, pure))
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
    fn jit_emit_step(e: &mut machine_jit::Em, j: &JitStep, k: usize, checked: bool, follows: bool, fetch: Option<u128>, units: Option<(u64, u64, u128)>) -> bool {{
        let s = &j.step;
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
{latch_resets}{reset_check}        if !follows {{
            let sd = e.load(std::mem::offset_of!(Self, shutdown), machine_jit::N::Bool);
            e.exit_if(sd, true, |e| {{ let _ = e.call(Self::jit_idle_c as *const () as usize, &[], machine_jit::N::U64, false); }});
        }}
{boundary}        // A boundary fn that takes no interrupt leaves `halted` as it was
        // (false, after a lowered step).
        let h = if follows && !{boundary_wakes} {{
            machine_jit::V::K(0)
        }} else {{
            e.read(&machine_jit::Place::Mem {{ off: std::mem::offset_of!(Self, halted), n: machine_jit::N::Bool, bump: 0 }})
        }};
        e.exit_if(h, true, |e| {{ let _ = e.call(Self::jit_idle_c as *const () as usize, &[], machine_jit::N::U64, false); }});
        }}
        e.store(std::mem::offset_of!(Self, tr_rec_now), machine_jit::N::Bool, machine_jit::V::K(0));
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
        // The board's time keeping in host code, when the board says how it
        // keeps it.
        let board = B::jit_board();
        // The bus calls (fetches, accesses) move time and the code unit.
        if let Some(b) = board {{
            let bus = std::mem::offset_of!(Self, bus);
            e.set_volatile(bus + b.now, 16);
            e.set_volatile(bus + b.code_time, 16);
            e.set_volatile(bus + b.direct_code, 8);
            e.set_volatile(bus + b.pending, 1);
        }}
        // The fetch, charged before the semantics as the bus would charge
        // it: serial (every unit), or overlapped (the unit last fetched is
        // in the buffer; the code time is what execution overlaps).
        // `timing: approximate`: the fetch as if serial, whatever the bus
        // does, charged with the cycles in one add.
        let approx_fetch: Option<u128> = if {timing_approx} {{ units.map(|(u0, u1, cost)| (u1 - u0 + 1) as u128 * cost).or(fetch) }} else {{ None }};
        let quiet = match (board, fetch, units) {{
            (Some(_), _, _) if approx_fetch.is_some() => true,
            // Serial, and nothing sees the time before the step retires: the
            // fetch is charged with it, in one add.
            (Some(b), Some(_), _) if b.serial && Self::jit_quiet(&instr) => true,
            (Some(b), Some(f), _) if b.serial => {{
                let bus = std::mem::offset_of!(Self, bus);
                let now = e.read(&machine_jit::Place::Mem {{ off: bus + b.now, n: machine_jit::N::U128, bump: 0 }});
                let now = e.op(machine_jit::Op::Add, now, machine_jit::V::K(f), machine_jit::N::U128);
                e.store(bus + b.now, machine_jit::N::U128, now);
                e.store(bus + b.code_time, machine_jit::N::U128, machine_jit::V::K(f));
                false
            }}
            (Some(b), _, Some((u0, u1, cost))) if !b.serial => {{
                let bus = std::mem::offset_of!(Self, bus);
                let dc = e.read(&machine_jit::Place::Mem {{ off: bus + b.direct_code, n: machine_jit::N::U64, bump: 0 }});
                let same = e.op(machine_jit::Op::Eq, dc, machine_jit::V::K(u0 as u128), machine_jit::N::U64);
                let same = e.cast(same, machine_jit::N::Bool, machine_jit::N::U128);
                let c = e.op(machine_jit::Op::Sub, machine_jit::V::K((u1 - u0 + 1) as u128), same, machine_jit::N::U128);
                let t = e.op(machine_jit::Op::Mul, c, machine_jit::V::K(cost), machine_jit::N::U128);
                let now = e.read(&machine_jit::Place::Mem {{ off: bus + b.now, n: machine_jit::N::U128, bump: 0 }});
                let now = e.op(machine_jit::Op::Add, now, t, machine_jit::N::U128);
                e.store(bus + b.now, machine_jit::N::U128, now);
                let ct = e.read(&machine_jit::Place::Mem {{ off: bus + b.code_time, n: machine_jit::N::U128, bump: 0 }});
                let ct = e.op(machine_jit::Op::Add, ct, t, machine_jit::N::U128);
                e.store(bus + b.code_time, machine_jit::N::U128, ct);
                e.store(bus + b.direct_code, machine_jit::N::U64, machine_jit::V::K(u1 as u128));
                false
            }}
            (Some(_), _, _) => {{
                let _ = e.call(Self::jit_fetch_c as *const () as usize, &[(machine_jit::V::K(j.page as u128), machine_jit::N::U64), (machine_jit::V::K(j.phys as u128), machine_jit::N::U64), (machine_jit::V::K(s.len as u128), machine_jit::N::U64)], machine_jit::N::U64, true);
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
        let cyc = e.cast(cyc, machine_jit::N::U32, machine_jit::N::U128);
        let x = e.op(machine_jit::Op::Mul, cyc, machine_jit::V::K(b.clock), machine_jit::N::U128);
        let now_p = machine_jit::Place::Mem {{ off: bus + b.now, n: machine_jit::N::U128, bump: 0 }};
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
        e.store(bus + b.code_time, machine_jit::N::U128, machine_jit::V::K(0));
        e.store(std::mem::offset_of!(Self, decoded), machine_jit::N::Bool, machine_jit::V::K(1));
        // An edge can wait only after a bus access (the last `after_step`
        // delivered the ones before).
        let edges = b.events && !Self::jit_quiet(&instr);
        let pending = |e: &mut machine_jit::Em| if edges {{ e.load(bus + b.pending, machine_jit::N::Bool) }} else {{ machine_jit::V::K(0) }};
        let need = match (b.due, b.idle) {{
            (Some(due), _) => {{
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
        if !{sync_run} {{
            e.cold(need, |e| {{ let _ = e.call(Self::jit_idle_c as *const () as usize, &[], machine_jit::N::U64, true); }});
        }}
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
        let h = e.read(&machine_jit::Place::Mem {{ off: std::mem::offset_of!(Self, halted), n: machine_jit::N::Bool, bump: 0 }});
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
            boundary_wakes = ist.boundary.is_some_and(|b| {
                let mut halts = false;
                machine_sema::hir::visit_block(&self.fns[&b].body, &mut |st| halts |= matches!(st.kind, machine_sema::hir::StmtKind::Halt), &mut |_| {});
                halts
            }),
            quiet_arms = quiet.iter().map(|n| format!("            Instr::{n}(_) => true,\n")).collect::<String>(),
            lowers_arms = lowered.iter().map(|n| format!("            Instr::{n}(_) => true,\n")).collect::<String>(),
        );
        self.out.push_str(&out);
        Ok(())
    }
}
