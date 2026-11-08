//! `machine-jit`: the JIT core's code generator (`core: jit`, SPEC 12,
//! "Cores"), for machines generated from Machine descriptions.
//!
//! A run of decoded instructions is compiled into one host function of the
//! machine: for each instruction, a call of its step (`run(machine, arg)`,
//! a fn the generated machine supplies for that instruction, with the
//! instruction's own data as `arg`). A step answers 0 when it was made and
//! the run goes on, 1 when it was made and the run stops there, 2 when it
//! was not made (the run cannot go on). The function is entered at a step
//! of the run (`from`) and answers how many steps it made. The calls are made through their addresses, so the generated
//! machine supplies them as plain `extern "C"` fns of its own.
//!
//! The instructions' semantics stay in the generated machine; the compiled
//! code replaces what happens between them (dispatch, finding the next
//! instruction of the run and what decoding it left behind).

mod emit;

pub use emit::{k, n_of, Count, Em, Native, Op, Pat, Place, Slot, N, V};

use cranelift_codegen::ir::{types, AbiParam, InstBuilder};
use cranelift_codegen::settings::{self, Configurable};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Switch};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{default_libcall_names, Linkage, Module};

/// One instruction of a run: `run(machine, arg)`, the address of an
/// `extern "C"` fn.
#[derive(Clone, Copy, Debug)]
pub struct Call {
    pub run: usize,
    pub arg: usize,
}

/// A run's compiled code: its address, none yet, or none to be had (the
/// code generator failed). A copy of a machine has none: its code belongs
/// to the machine it was compiled for.
#[derive(Default, Debug)]
pub struct Code(usize);

impl Clone for Code {
    fn clone(&self) -> Self {
        Code(0)
    }
}

impl Code {
    /// The code `compile` answered (None: it failed, and is not tried again).
    pub fn new(addr: Option<usize>) -> Code {
        Code(addr.unwrap_or(usize::MAX))
    }
    pub fn addr(&self) -> Option<usize> {
        (self.0 != 0 && self.0 != usize::MAX).then_some(self.0)
    }
}

/// Compiles runs. Code lives until the engine is reset (`clear`).
pub struct Engine {
    module: Option<JITModule>,
    fbc: FunctionBuilderContext,
    count: usize,
    next: u32,
}

impl Default for Engine {
    fn default() -> Self {
        Engine { module: None, fbc: FunctionBuilderContext::new(), count: 0, next: 0 }
    }
}

/// A copy of a machine starts with no compiled code.
impl Clone for Engine {
    fn clone(&self) -> Self {
        Engine::default()
    }
}

impl std::fmt::Debug for Engine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Engine({} runs compiled)", self.count)
    }
}

/// Runs compiled before the engine is reset (and its code freed).
pub const LIMIT: usize = 16384;

fn new_module() -> Option<JITModule> {
    let mut flags = settings::builder();
    // Cranelift's own optimisation (`MACHINE_JIT_OPT=speed`): none by
    // default, the emitter folding what it knows.
    let opt = std::env::var("MACHINE_JIT_OPT").unwrap_or_else(|_| "none".to_string());
    flags.set("opt_level", &opt).ok()?;
    // The verifier checks what the emitter builds; on with
    // `MACHINE_JIT_VERIFY` (and in this crate's tests).
    let verify = cfg!(test) || std::env::var_os("MACHINE_JIT_VERIFY").is_some();
    flags.set("enable_verifier", if verify { "true" } else { "false" }).ok()?;
    let isa = cranelift_native::builder().ok()?.finish(settings::Flags::new(flags)).ok()?;
    Some(JITModule::new(JITBuilder::with_isa(isa, default_libcall_names())))
}

impl Engine {
    /// How many runs are compiled.
    pub fn count(&self) -> usize {
        self.count
    }

    /// Forget every compiled run and free its code. None may be running.
    pub fn clear(&mut self) {
        if let Some(m) = self.module.take() {
            // SAFETY: the caller has no compiled code on the stack, and drops
            // every address it was given.
            unsafe { m.free_memory() };
        }
        self.count = 0;
    }

    /// The host function for `calls`: `extern "C" fn(*mut machine, from:
    /// u32) -> u32`, entered at call `from`, answering the steps it made.
    /// None if the code generator is unavailable.
    pub fn compile(&mut self, calls: &[Call]) -> Option<usize> {
        self.compile_with(calls.len(), calls.len(), |e, k| {
            let r = e.call(calls[k].run, &[(V::K(calls[k].arg as u128), N::U64)], N::U32, false);
            e.exit_on(r);
        })
    }

    /// The host function for a run of `steps` steps, each emitted by
    /// `emit(emitter, k)`: `extern "C" fn(*mut machine, from: u32) -> u32`,
    /// entered at step `from` (one of the first `entries`), answering the
    /// steps it made. A step that falls off its end goes on to the next
    /// (what is known of the machine with it, past the entries). None if the
    /// code generator is unavailable or the code is not valid.
    pub fn compile_with(&mut self, steps: usize, entries: usize, mut emit: impl FnMut(&mut Em, usize)) -> Option<usize> {
        if self.module.is_none() {
            self.module = new_module();
        }
        let m = self.module.as_mut()?;
        let ptr = m.target_config().pointer_type();
        let cc = m.target_config().default_call_conv;
        let mut ctx = m.make_context();
        ctx.func.signature.params.push(AbiParam::new(ptr));
        ctx.func.signature.params.push(AbiParam::new(types::I32));
        ctx.func.signature.returns.push(AbiParam::new(types::I32));
        let sig = ctx.func.signature.clone();
        // A run that goes round (`Em::loop_back`) is built twice: the first
        // time says what it knows going round, and the second passes that
        // to its first step instead of storing it.
        // So are loops within steps (`Em::repeat_head`).
        let (seen, repeats) = build(&mut ctx.func, &mut self.fbc, ptr, cc, steps, entries, &mut emit, None, Vec::new());
        let plan = seen.filter(|p| !p.is_empty());
        if plan.is_some() || repeats.iter().flatten().any(|p| !p.is_empty()) {
            ctx.func = cranelift_codegen::ir::Function::with_name_signature(Default::default(), sig);
            let _ = build(&mut ctx.func, &mut self.fbc, ptr, cc, steps, entries, &mut emit, plan, repeats);
        }
        let name = format!("run{}", self.next);
        self.next += 1;
        let id = m.declare_function(&name, Linkage::Local, &ctx.func.signature).ok()?;
        if std::env::var_os("MACHINE_JIT_CLIF").is_some() {
            eprintln!("machine-jit: {name} ({steps} steps), IR:\n{}", ctx.func.display());
        }
        let dump = std::env::var_os("MACHINE_JIT_DUMP").is_some();
        if dump {
            ctx.set_disasm(true);
        }
        if let Err(err) = m.define_function(id, &mut ctx) {
            if std::env::var_os("MACHINE_JIT_DEBUG").is_some() {
                eprintln!("machine-jit: {err:?}\n{}", ctx.func.display());
            }
            m.clear_context(&mut ctx);
            return None;
        }
        if dump {
            if let Some(c) = ctx.compiled_code() {
                eprintln!("machine-jit: {name} ({steps} steps)\n{}", c.vcode.as_deref().unwrap_or(""));
            }
        }
        // For profiling: the code and the step each byte range belongs to.
        if let (Some(dir), Some(c)) = (std::env::var_os("MACHINE_JIT_BIN"), ctx.compiled_code()) {
            let dir = std::path::Path::new(&dir);
            let _ = std::fs::write(dir.join(format!("{name}.bin")), c.code_buffer());
            let map: String = c.buffer.get_srclocs_sorted().iter()
                .map(|r| format!("{:x} {:x} {}\n", r.start, r.end, r.loc.bits())).collect();
            let _ = std::fs::write(dir.join(format!("{name}.steps")), map);
        }
        m.clear_context(&mut ctx);
        m.finalize_definitions().ok()?;
        self.count += 1;
        Some(m.get_finalized_function(id) as usize)
    }
}


/// Emit a run's function into `func` (see `Engine::compile_with`); with
/// `plan`, its first step takes what the run knows going round. What it
/// knew going round, when it went round.
#[allow(clippy::too_many_arguments)]
fn build(
    func: &mut cranelift_codegen::ir::Function,
    fbc: &mut FunctionBuilderContext,
    ptr: cranelift_codegen::ir::Type,
    cc: cranelift_codegen::isa::CallConv,
    steps: usize,
    entries: usize,
    emit: &mut impl FnMut(&mut Em, usize),
    plan: Option<emit::LoopPlan>,
    repeats: Vec<Option<emit::RepeatPlan>>,
) -> (Option<emit::LoopPlan>, Vec<Option<emit::RepeatPlan>>) {
    let mut b = FunctionBuilder::new(func, fbc);
    let entry = b.create_block();
    b.append_block_params_for_function_params(entry);
    b.switch_to_block(entry);
    b.seal_block(entry);
    let machine = b.block_params(entry)[0];
    let from = b.block_params(entry)[1];
    let made_base = b.declare_var(types::I32);
    let neg = b.ins().ineg(from);
    b.def_var(made_base, neg);

    // The way out: the steps made so far.
    let out = b.create_block();
    b.append_block_param(out, types::I32);
    // A block a step, entered by `from` (past the end: none made); with a
    // plan, the first through a pad that loads what it takes.
    let blocks: Vec<_> = (0..steps).map(|_| b.create_block()).collect();
    // Going round, the first step built twice (`Em::peel_first`): entered
    // at its own copy; the first block is the back-edge's alone.
    let peel = steps >= 2 && plan.as_ref().is_some_and(|p| p.peel);
    let entry0 = if peel { Some(b.create_block()) } else { None };
    let pad = if peel { None } else { plan.as_ref().map(|_| b.create_block()) };
    // Later entries through pads too (`Em::begin_step`).
    let pads: Vec<(usize, cranelift_codegen::ir::Block)> = (1..entries.min(steps)).map(|k| (k, b.create_block())).collect();
    let mut switch = Switch::new();
    for (k, blk) in blocks.iter().enumerate().take(entries) {
        let to = match k {
            0 => entry0.or(pad).unwrap_or(*blk),
            _ => pads.iter().find(|(j, _)| *j == k).map(|(_, p)| *p).unwrap_or(*blk),
        };
        switch.set_entry(k as u128, to);
    }
    let none = b.create_block();
    switch.emit(&mut b, from, none);
    if let Some(e0) = entry0 {
        b.seal_block(e0);
    }
    b.switch_to_block(none);
    b.seal_block(none);
    let zero = b.ins().iconst(types::I32, 0);
    b.ins().jump(out, &[zero.into()]);
    let seen;
    {
        let mut e = Em::new(&mut b, ptr, cc, machine, out, made_base);
        e.first = blocks.first().copied();
        e.steps = steps;
        e.entry_pads = pads.into_iter().collect();
        e.repeat_plans = repeats;
        let mut peeled = None;
        // (Whether the plan's constants go round is decided here: built
        // once, the first step takes none, and the back-edge stores them.)
        let plan = plan.map(|mut p| {
            p.peel = peel;
            p
        });
        match (plan, pad, blocks.first().copied()) {
            (Some(plan), Some(pad), Some(first)) => e.entry_pad(pad, first, plan),
            (Some(plan), None, Some(_)) if peel => peeled = Some(plan),
            _ => {}
        }
        for (k, blk) in blocks.iter().enumerate() {
            if let (0, Some(plan), Some(e0)) = (k, peeled.take(), entry0) {
                let join = e.b.create_block();
                let mark = e.mark();
                e.peel_first(e0, *blk, join, plan, &mut |e| emit(e, 0));
                e.release(mark);
                continue;
            }
            e.begin_step(k, *blk, k < entries);
            let mark = e.mark();
            emit(&mut e, k);
            e.release(mark);
        }
        e.finish(steps);
        seen = (e.take_seen(), e.take_repeat_seen());
    }
    for blk in &blocks {
        b.seal_block(*blk);
    }
    b.switch_to_block(out);
    b.seal_block(out);
    let n = b.block_params(out)[0];
    b.ins().return_(&[n]);
    b.finalize();
    seen
}

#[cfg(test)]
mod tests {
    use super::*;

    struct M {
        log: Vec<u32>,
        stop_at: u32,
    }
    extern "C" fn run(m: *mut M, a: usize) -> u32 {
        let m = unsafe { &mut *m };
        if m.log.len() as u32 >= m.stop_at {
            return 2;
        }
        m.log.push(100 + a as u32);
        u32::from(a == 1 && m.stop_at == 1000)
    }

    #[test]
    fn a_run_calls_in_order_and_stops_when_a_step_says() {
        let mut e = Engine::default();
        let calls: Vec<Call> = (0..3).map(|k| Call { run: run as usize, arg: k }).collect();
        let f = e.compile(&calls).expect("compiled");
        let f: extern "C" fn(*mut M, u32) -> u32 = unsafe { std::mem::transmute(f) };
        let mut m = M { log: Vec::new(), stop_at: 100 };
        assert_eq!(f(&mut m, 0), 3);
        assert_eq!(m.log, vec![100, 101, 102]);
        let mut m = M { log: Vec::new(), stop_at: 2 };
        assert_eq!(f(&mut m, 0), 2, "the third was not made");
        let mut m = M { log: Vec::new(), stop_at: 1000 };
        assert_eq!(f(&mut m, 0), 2, "the second was made and stopped the run");
        let mut m = M { log: Vec::new(), stop_at: 100 };
        assert_eq!(f(&mut m, 1), 2, "entered at the second");
        assert_eq!(m.log, vec![101, 102]);
        assert_eq!(f(&mut m, 3), 0, "entered past the end");
        e.clear();
        assert_eq!(e.count(), 0);
    }

    #[repr(C)]
    #[derive(Default, Debug)]
    struct T {
        a: u8,
        b: u8,
        w: u16,
        x: u32,
        y: i32,
        out: u64,
    }

    fn run_t(t: &mut T, emit: impl FnMut(&mut Em, usize)) -> u32 {
        let mut e = Engine::default();
        let f = e.compile_with(1, 1, emit).expect("compiled");
        let f: extern "C" fn(*mut T, u32) -> u32 = unsafe { std::mem::transmute(f) };
        f(t, 0)
    }

    const A: Place = Place::Mem { off: std::mem::offset_of!(T, a), n: N::U8, bump: 0 };
    const W: Place = Place::Mem { off: std::mem::offset_of!(T, w), n: N::U16, bump: 0 };
    const X: Place = Place::Mem { off: std::mem::offset_of!(T, x), n: N::U32, bump: 0 };
    const Y: Place = Place::Mem { off: std::mem::offset_of!(T, y), n: N::I32, bump: 0 };
    const OUT: Place = Place::Mem { off: std::mem::offset_of!(T, out), n: N::U64, bump: 0 };

    #[test]
    fn locals_merge_across_a_computed_branch() {
        // let l = 5; if a > 10 { l = a + 1 } else { l = 7 }; out = l * 3 (in u64)
        for (a, want) in [(20u8, 63u64), (3, 21)] {
            let mut t = T { a, ..T::default() };
            let n = run_t(&mut t, |e, _| {
                let l = e.local(N::U8, V::K(5));
                let av = e.read(&A);
                let c = e.op(Op::Gt, av, V::K(10), N::U8);
                e.if_(
                    c,
                    |e| {
                        let av = e.read(&A);
                        let v = e.op(Op::Add, av, V::K(1), N::U8);
                        e.set(l, v)
                    },
                    |e| e.set(l, V::K(7)),
                );
                let v = e.get(l);
                let v = e.cast(v, N::U8, N::U64);
                let v = e.op(Op::Mul, v, V::K(3), N::U64);
                e.write(&OUT, v);
            });
            assert_eq!(n, 1);
            assert_eq!(t.out, want, "a = {a}");
        }
    }

    #[test]
    fn a_local_known_in_one_branch_and_unchanged_in_the_other() {
        for (a, want) in [(1u8, 9u64), (0, 4)] {
            let mut t = T { a, ..T::default() };
            run_t(&mut t, |e, _| {
                let l = e.local(N::U64, V::K(4));
                let av = e.read(&A);
                e.if_(av, |e| e.set(l, V::K(9)), |_| {});
                let v = e.get(l);
                e.write(&OUT, v);
            });
            assert_eq!(t.out, want);
        }
    }

    #[test]
    fn bits_places_and_folding() {
        let mut t = T { w: 0xABCD, ..T::default() };
        run_t(&mut t, |e, _| {
            // The high byte of w, as a u8; known arithmetic folds.
            let hi = Place::Bits { off: std::mem::offset_of!(T, w), reg: N::U16, shift: 8, width: 8, n: N::U8, bump: 0 };
            let v = e.read(&hi);
            let k = e.op(Op::Add, V::K(250), V::K(10), N::U8);
            assert_eq!(k, V::K(4), "u8 arithmetic wraps when folded");
            let v = e.op(Op::Xor, v, k, N::U8);
            e.write(&A, v);
            // Bit 0 of w set as a one-bit field.
            let lo = Place::Bits { off: std::mem::offset_of!(T, w), reg: N::U16, shift: 0, width: 1, n: N::Bool, bump: 0 };
            e.write(&lo, V::K(0));
            let w = e.read(&W);
            let w = e.cast(w, N::U16, N::U64);
            e.write(&OUT, w);
        });
        assert_eq!(t.a, 0xAB ^ 4);
        assert_eq!(t.out, 0xABCC);
    }

    #[test]
    fn bit_fields_wait_to_be_put_in_their_field() {
        // Flags in x: bit 0 and bit 6 written (computed, then known), bit 0
        // read back as written, x read whole, then a branch that writes bit
        // 6 on one side only; x as stored at the end.
        for a in [0u8, 1, 2, 3] {
            let mut t = T { a, x: 0xF0F0_0F0F, ..T::default() };
            run_t(&mut t, |e, _| {
                let off = std::mem::offset_of!(T, x);
                let b0 = Place::Bits { off, reg: N::U32, shift: 0, width: 1, n: N::Bool, bump: 0 };
                let b6 = Place::Bits { off, reg: N::U32, shift: 6, width: 1, n: N::Bool, bump: 0 };
                let nib = Place::Bits { off, reg: N::U32, shift: 4, width: 4, n: N::U8, bump: 0 };
                let av = e.read(&A);
                let odd = e.op(Op::And, av, V::K(1), N::U8);
                let odd = e.op(Op::Ne, odd, V::K(0), N::U8);
                e.write(&b0, odd);
                e.write(&b6, V::K(1));
                e.write(&b0, V::K(0));
                e.write(&b0, odd);
                // Read back: as written, and a field the bits overlap.
                let r = e.read(&b0);
                let r = e.cast(r, N::Bool, N::U8);
                e.write(&A, r);
                let n = e.read(&nib);
                let n = e.cast(n, N::U8, N::U64);
                e.write(&OUT, n);
                let two = e.op(Op::And, av, V::K(2), N::U8);
                e.if_(two, |e| e.write(&b6, V::K(0)), |_| {});
            });
            let odd = (a & 1) as u32;
            let six = if a & 2 != 0 { 0 } else { 1 };
            assert_eq!(t.a, odd as u8, "a = {a}");
            assert_eq!(t.out, 0x4, "the nibble with bit 6 set, a = {a}");
            assert_eq!(t.x, (0xF0F0_0F0F & !0x41) | odd | (six << 6), "a = {a}");
        }
    }

    #[test]
    fn a_run_goes_round_and_counts_its_steps() {
        // Step 0: a += 1. Step 1: out += 1, then round again while a < 5.
        // From step 0: five passes, ten steps; from step 1: one step
        // first, then four passes.
        for (from, a0, steps, out) in [(0u32, 0u8, 10u32, 5u64), (1, 1, 9, 5)] {
            let mut t = T { a: a0, x: 0xF0F0_0000, ..T::default() };
            let mut e = Engine::default();
            let f = e
                .compile_with(2, 2, |e, k| {
                    if k == 0 {
                        let a = e.read(&A);
                        let a = e.op(Op::Add, a, V::K(1), N::U8);
                        e.write(&A, a);
                        // Bit 0 of x: a's low bit (a piece going round).
                        let b0 = Place::Bits { off: std::mem::offset_of!(T, x), reg: N::U32, shift: 0, width: 1, n: N::Bool, bump: 0 };
                        let odd = e.op(Op::And, a, V::K(1), N::U8);
                        let odd = e.op(Op::Ne, odd, V::K(0), N::U8);
                        e.write(&b0, odd);
                    } else {
                        let o = e.read(&OUT);
                        let o = e.op(Op::Add, o, V::K(1), N::U64);
                        e.write(&OUT, o);
                        let a = e.read(&A);
                        let more = e.op(Op::Lt, a, V::K(5), N::U8);
                        let made = e.made_through();
                        assert!(matches!(made, V::D(_)));
                        e.loop_back(more, |_| {});
                    }
                })
                .expect("compiled");
            let f: extern "C" fn(*mut T, u32) -> u32 = unsafe { std::mem::transmute(f) };
            assert_eq!(f(&mut t, from), steps, "from {from}");
            assert_eq!((t.a, t.out), (5, out), "from {from}");
            assert_eq!(t.x, 0xF0F0_0001, "from {from}");
        }
    }

    #[test]
    fn going_round_the_first_step_knows_the_constants_owed() {
        // Step 0: a += b. Step 1: b = 7 (owed, a constant), out += 1, round
        // again while a < 40. Entered with b = 1: the first pass adds 1 (b
        // as the machine holds it), the rest 7 (b as the back-edge knows
        // it); b is stored when the run returns.
        let mut t = T { a: 0, b: 1, ..T::default() };
        let mut e = Engine::default();
        let bp = Place::Mem { off: std::mem::offset_of!(T, b), n: N::U8, bump: 0 };
        let f = e
            .compile_with(2, 1, |e, k| {
                if k == 0 {
                    let a = e.read(&A);
                    let b = e.read(&bp);
                    let a = e.op(Op::Add, a, b, N::U8);
                    e.write(&A, a);
                } else {
                    e.write(&bp, V::K(7));
                    let o = e.read(&OUT);
                    let o = e.op(Op::Add, o, V::K(1), N::U64);
                    e.write(&OUT, o);
                    let a = e.read(&A);
                    let more = e.op(Op::Lt, a, V::K(40), N::U8);
                    e.loop_back(more, |_| {});
                }
            })
            .expect("compiled");
        let f: extern "C" fn(*mut T, u32) -> u32 = unsafe { std::mem::transmute(f) };
        // 1, 8, 15, 22, 29, 36, 43: seven passes.
        assert_eq!(f(&mut t, 0), 14);
        assert_eq!((t.a, t.b, t.out), (43, 7, 7));
    }

    #[test]
    fn a_run_of_one_step_going_round_stores_what_it_owes() {
        // One step (not built twice): a += 1, b = 7 (an owed constant), round
        // while a < 5. b must reach the machine.
        let mut t = T { a: 0, b: 1, ..T::default() };
        let mut e = Engine::default();
        let bp = Place::Mem { off: std::mem::offset_of!(T, b), n: N::U8, bump: 0 };
        let f = e
            .compile_with(1, 1, |e, _| {
                let a = e.read(&A);
                let a = e.op(Op::Add, a, V::K(1), N::U8);
                e.write(&A, a);
                e.write(&bp, V::K(7));
                let more = e.op(Op::Lt, a, V::K(5), N::U8);
                e.loop_back(more, |_| {});
            })
            .expect("compiled");
        let f: extern "C" fn(*mut T, u32) -> u32 = unsafe { std::mem::transmute(f) };
        assert_eq!(f(&mut t, 0), 5);
        assert_eq!((t.a, t.b), (5, 7));
    }

    extern "C" fn sees(t: *mut T) -> u64 {
        // SAFETY: the test's machine.
        let t = unsafe { &mut *t };
        // a is the code's to store later (left alone); b was stored.
        t.out = (t.a as u64) << 8 | t.b as u64;
        t.w = t.b as u16 + 1;
        0
    }

    #[test]
    fn a_call_that_leaves_fields_alone() {
        let mut t = T { a: 1, b: 2, ..T::default() };
        run_t(&mut t, |e, _| {
            let ao = std::mem::offset_of!(T, a);
            let bo = std::mem::offset_of!(T, b);
            e.write(&A, V::K(7));
            let b = Place::Mem { off: bo, n: N::U8, bump: 0 };
            e.write(&b, V::K(9));
            let _ = e.call_fx(sees as *const () as usize, &[], N::U64, &[(ao, 1)], &[(bo, 1)]);
            // Still known: a (owed) and b (read); w is the call's.
            assert_eq!(e.read(&A), V::K(7));
            assert_eq!(e.read(&b), V::K(9));
            assert!(matches!(e.read(&W), V::D(_)));
        });
        assert_eq!(t.out, 1 << 8 | 9, "the call saw a as it was, b as written");
        assert_eq!((t.a, t.b, t.w), (7, 9, 10));
    }

    #[test]
    fn one_of_two_constants_stays_one_of_two() {
        // t = a ? 5 : 9 (in u128); out = (100 - t) as u64, w = (t > 6) as u16.
        for (a, out, w) in [(1u8, 95u64, 0u16), (0, 91, 1)] {
            let mut t = T { a, ..T::default() };
            run_t(&mut t, |e, _| {
                let av = e.read(&A);
                let c = e.op(Op::Ne, av, V::K(0), N::U8);
                let tv = e.select(c, V::K(5), V::K(9), N::U128);
                let d = e.op(Op::Sub, V::K(100), tv, N::U128);
                let d = e.cast(d, N::U128, N::U64);
                e.write(&OUT, d);
                let g = e.op(Op::Gt, tv, V::K(6), N::U128);
                let g = e.cast(g, N::Bool, N::U16);
                e.write(&W, g);
            });
            assert_eq!((t.out, t.w), (out, w), "a = {a}");
        }
    }

    #[test]
    fn a_lazy_flag_is_computed_where_it_is_needed() {
        // Bit 0 of x: whether a is odd (lazy); leave when a > 5 (the flag is
        // stored on the way out), else bit 0 is written again, from a
        // different value.
        for (a, made, bit) in [(7u8, 0u32, 1u32), (3, 1, 0), (9, 0, 1), (2, 1, 1)] {
            let mut t = T { a, x: 0x100, ..T::default() };
            let n = run_t(&mut t, |e, _| {
                let b0 = Place::Bits { off: std::mem::offset_of!(T, x), reg: N::U32, shift: 0, width: 1, n: N::Bool, bump: 0 };
                let av = e.read(&A);
                let odd = e.op(Op::And, av, V::K(1), N::U8);
                let odd = e.op(Op::Ne, odd, V::K(0), N::U8);
                e.write(&b0, odd);
                let big = e.op(Op::Gt, av, V::K(5), N::U8);
                e.exit_if(big, false, |_| {});
                let even = e.op(Op::Eq, odd, V::K(0), N::Bool);
                e.write(&b0, even);
            });
            assert_eq!((n, t.x), (made, 0x100 | bit), "a = {a}");
        }
    }

    #[test]
    fn a_branch_knows_what_it_tested() {
        // b (one bit) == 1: on that side b is known 1, on the other 0, so
        // `b + 5` folds on both sides; a known field is not reloaded.
        for b in [0u8, 1] {
            let mut t = T { b, ..T::default() };
            let bo = std::mem::offset_of!(T, b);
            let bp = Place::Mem { off: bo, n: N::U8, bump: 0 };
            run_t(&mut t, |e, _| {
                e.declare_bits(bo, N::U8, 1);
                let v = e.read(&bp);
                let c = e.op(Op::Eq, v, V::K(1), N::U8);
                let out = e.local(N::U64, V::K(0));
                e.if_(
                    c,
                    |e| {
                        let v = e.read(&bp);
                        assert_eq!(v, V::K(1));
                        e.set(out, V::K(6));
                    },
                    |e| {
                        let v = e.read(&bp);
                        assert_eq!(v, V::K(0));
                        e.set(out, V::K(5));
                    },
                );
                let o = e.get(out);
                e.write(&OUT, o);
                // Past an exit on b != 1 the code knows b is 1.
                let v = e.read(&bp);
                let ne = e.op(Op::Ne, v, V::K(1), N::U8);
                e.exit_if(ne, false, |_| {});
                assert_eq!(e.read(&bp), V::K(1));
            });
            assert_eq!(t.out, 5 + b as u64, "b = {b}");
        }
    }

    #[test]
    fn past_an_exit_on_either_both_are_known() {
        // Leave when a == 3 or b == 4: past it, neither holds, and for a
        // one-bit field that says what it is.
        let mut t = T { a: 1, b: 0, ..T::default() };
        let (ao, bo) = (std::mem::offset_of!(T, a), std::mem::offset_of!(T, b));
        let n = run_t(&mut t, |e, _| {
            e.declare_bits(ao, N::U8, 1);
            let a = e.read(&A);
            let b = e.read(&Place::Mem { off: bo, n: N::U8, bump: 0 });
            let ca = e.op(Op::Eq, a, V::K(0), N::U8);
            let cb = e.op(Op::Eq, b, V::K(4), N::U8);
            let either = e.op(Op::Or, ca, cb, N::Bool);
            e.exit_if(either, false, |_| {});
            assert_eq!(e.read(&A), V::K(1), "a is not 0: one bit, so 1");
            e.write(&OUT, V::K(9));
        });
        assert_eq!((n, t.out), (1, 9));
    }

    #[test]
    fn a_step_repeats_within_itself() {
        // One step: a += 1 while a < 10, each time round a step made; the
        // local counts the times round, from a known start.
        for (a0, steps, out) in [(0u8, 10u32, 10u64), (9, 1, 1), (20, 1, 1)] {
            let mut t = T { a: a0, ..T::default() };
            let n = run_t(&mut t, |e, _| {
                let times = e.local(N::U64, V::K(0));
                let head = e.repeat_head();
                let a = e.read(&A);
                let a = e.op(Op::Add, a, V::K(1), N::U8);
                e.write(&A, a);
                let c = e.get(times);
                let c = e.op(Op::Add, c, V::K(1), N::U64);
                e.set(times, c);
                let more = e.op(Op::Lt, a, V::K(10), N::U8);
                e.repeat_tail(head, more);
                let c = e.get(times);
                e.write(&OUT, c);
            });
            assert_eq!((n, t.out), (steps, out), "a = {a0}");
        }
    }

    #[test]
    fn a_loop_within_a_step_carries_what_it_knows() {
        // b written before the loop (owed, unchanged inside); a counted in
        // a register round the loop; w a constant written each time round;
        // out counts the times round. All stored right at the end.
        for (a0, out) in [(0u8, 10u64), (9, 1)] {
            let mut t = T { a: a0, b: 1, ..T::default() };
            let bp = Place::Mem { off: std::mem::offset_of!(T, b), n: N::U8, bump: 0 };
            let n = run_t(&mut t, |e, _| {
                e.write(&bp, V::K(7));
                e.write(&W, V::K(3));
                let times = e.local(N::U64, V::K(0));
                let head = e.repeat_head();
                e.write(&W, V::K(3));
                let a = e.read(&A);
                let a = e.op(Op::Add, a, V::K(1), N::U8);
                e.write(&A, a);
                let c = e.get(times);
                let c = e.op(Op::Add, c, V::K(1), N::U64);
                e.set(times, c);
                let more = e.op(Op::Lt, a, V::K(10), N::U8);
                e.repeat_tail(head, more);
                let c = e.get(times);
                e.write(&OUT, c);
            });
            assert_eq!((n as u64, t.out, t.a, t.b, t.w), (out, out, 10.max(a0 + 1), 7, 3), "a = {a0}");
        }
    }

    #[test]
    fn shifts_past_the_width_and_signed_values() {
        for (x, y) in [(3u32, -64i32), (40, 7), (31, i32::MIN)] {
            let mut t = T { x, y, ..T::default() };
            run_t(&mut t, |e, _| {
                let one = V::K(1);
                let xv = e.read(&X);
                let s = e.op(Op::Shl, one, xv, N::U32);
                let yv = e.read(&Y);
                let r = e.op(Op::Shr, yv, xv, N::I32);
                let lt = e.op(Op::Lt, yv, V::K(0), N::I32);
                let r = e.cast(r, N::I32, N::I64);
                let r = e.cast(r, N::I64, N::U64);
                let s = e.cast(s, N::U32, N::U64);
                let lt = e.cast(lt, N::Bool, N::U64);
                let lt = e.op(Op::Shl, lt, V::K(63), N::U64);
                let v = e.op(Op::Xor, r, s, N::U64);
                let v = e.op(Op::Xor, v, lt, N::U64);
                e.write(&OUT, v);
            });
            let r = y.checked_shr(x).unwrap_or(if y < 0 { -1 } else { 0 }) as i64 as u64;
            let s = 1u32.checked_shl(x).unwrap_or(0) as u64;
            let want = r ^ s ^ (((y < 0) as u64) << 63);
            assert_eq!(t.out, want, "x = {x}, y = {y}");
        }
    }

    #[test]
    fn exits_count_the_steps_made() {
        let mut e = Engine::default();
        let f = e
            .compile_with(3, 3, |e, k| {
                let av = e.read(&A);
                let stop = e.op(Op::Eq, av, V::K(k as u128), N::U8);
                e.exit_if(stop, false, |_| {});
                let v = e.op(Op::Add, av, V::K(0), N::U8);
                let v = e.cast(v, N::U8, N::U64);
                let o = e.read(&OUT);
                let o = e.op(Op::Add, o, v, N::U64);
                e.write(&OUT, o);
            })
            .expect("compiled");
        let f: extern "C" fn(*mut T, u32) -> u32 = unsafe { std::mem::transmute(f) };
        let mut t = T { a: 2, ..T::default() };
        assert_eq!(f(&mut t, 0), 2, "stopped at the third");
        assert_eq!(t.out, 4);
        let mut t = T { a: 9, ..T::default() };
        assert_eq!(f(&mut t, 1), 2, "from the second, all made");
        assert_eq!(t.out, 18);
    }

    #[repr(C)]
    #[derive(Default, Debug)]
    struct W {
        a: u128,
        b: u128,
        q: u128,
        r: u128,
    }

    #[test]
    fn wide_division_by_a_call() {
        let a_ = Place::Mem { off: std::mem::offset_of!(W, a), n: N::I128, bump: 0 };
        let b_ = Place::Mem { off: std::mem::offset_of!(W, b), n: N::I128, bump: 0 };
        let mut e = Engine::default();
        let f = e
            .compile_with(1, 1, |e, _| {
                let a = e.read(&a_);
                let b = e.read(&b_);
                let q = e.op(Op::Div, a, b, N::I128);
                let r = e.op(Op::Rem, a, b, N::I128);
                e.store(std::mem::offset_of!(W, q), N::I128, q);
                e.store(std::mem::offset_of!(W, r), N::I128, r);
            })
            .expect("compiled");
        let f: extern "C" fn(*mut W, u32) -> u32 = unsafe { std::mem::transmute(f) };
        for (a, b) in [(-(1i128 << 100) - 12345, 7i128), (i64::MIN as i128, -1), (99, -100), (1i128 << 120, (1i128 << 64) + 3)] {
            let mut w = W { a: a as u128, b: b as u128, ..W::default() };
            assert_eq!(f(&mut w, 0), 1);
            assert_eq!((w.q as i128, w.r as i128), (a / b, a % b), "{a} / {b}");
        }
    }

    #[test]
    fn a_trial_inline_is_kept_or_falls_back() {
        // out = a * 3 + 1: inlined when it fits, else "called" (a different
        // computation, to tell them apart), with a local live across.
        for (budget, want) in [(1000usize, 3 * 7 + 1), (2, 7 + 100)] {
            let mut t = T { a: 7, ..T::default() };
            run_t(&mut t, |e, _| {
                let keep = e.local(N::U64, V::K(5));
                let av = e.read(&A);
                let av = e.cast(av, N::U8, N::U64);
                let r = e.try_inline(
                    budget,
                    |e| {
                        let mut x = av;
                        // Enough work to pass a small budget.
                        for _ in 0..3 {
                            x = e.op(Op::Add, x, av, N::U64);
                        }
                        let x = e.op(Op::Sub, x, av, N::U64);
                        e.op(Op::Add, x, V::K(1), N::U64)
                    },
                    |e| e.op(Op::Add, av, V::K(100), N::U64),
                );
                let k = e.get(keep);
                let r = e.op(Op::Add, r, k, N::U64);
                let r = e.op(Op::Sub, r, V::K(5), N::U64);
                e.write(&OUT, r);
            });
            assert_eq!(t.out, want, "budget {budget}");
        }
    }
}
