//! `machine-jit`: the JIT core's code generator (`core: jit`, SPEC 12,
//! "Cores"), for machines generated from Machine descriptions.
//!
//! A run of decoded instructions is compiled into one host function of the
//! machine: for each instruction, a call that prepares it (`pre`), a call
//! that runs it (`run`, the instruction's own handler) and a call that
//! finishes it (`post`). `pre` answers 0 to go on, 1 when the step ended
//! another way (an interrupt was taken: it counts as a step), 2 when the run
//! cannot go on (nothing was done); `post` answers 0 to go on, else 1. The
//! function answers how many steps it made. The calls are made through
//! their addresses, so the generated machine supplies them as plain
//! `extern "C"` fns of its own.
//!
//! This is the first form of the core: the instructions' semantics stay in
//! the generated handlers, and the compiled code replaces what happens
//! between them (dispatch, finding the next instruction of the run).

use cranelift_codegen::ir::{types, AbiParam, InstBuilder, MemFlags};
use cranelift_codegen::settings::{self, Configurable};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{default_libcall_names, Linkage, Module};

/// One instruction of a run: `pre(machine)`, then `run(machine, arg)`, then
/// `post(machine)`; addresses of `extern "C"` fns.
#[derive(Clone, Copy, Debug)]
pub struct Call {
    pub pre: usize,
    pub run: usize,
    pub arg: usize,
    pub post: usize,
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
    flags.set("opt_level", "none").ok()?;
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

    /// The host function for `calls`: `extern "C" fn(*mut machine) -> u32`,
    /// the steps it made. None if the code generator is unavailable.
    pub fn compile(&mut self, calls: &[Call]) -> Option<usize> {
        if self.module.is_none() {
            self.module = new_module();
        }
        let m = self.module.as_mut()?;
        let ptr = m.target_config().pointer_type();
        let mut ctx = m.make_context();
        ctx.func.signature.params.push(AbiParam::new(ptr));
        ctx.func.signature.returns.push(AbiParam::new(types::I32));
        let mut pre_sig = m.make_signature();
        pre_sig.params.push(AbiParam::new(ptr));
        pre_sig.returns.push(AbiParam::new(types::I32));
        let mut run_sig = m.make_signature();
        run_sig.params.push(AbiParam::new(ptr));
        run_sig.params.push(AbiParam::new(ptr));
        run_sig.returns.push(AbiParam::new(types::I32));
        {
            let mut b = FunctionBuilder::new(&mut ctx.func, &mut self.fbc);
            let pre_ref = b.import_signature(pre_sig);
            let run_ref = b.import_signature(run_sig);
            let entry = b.create_block();
            b.append_block_params_for_function_params(entry);
            b.switch_to_block(entry);
            b.seal_block(entry);
            let machine = b.block_params(entry)[0];
            // The way out: the steps made so far.
            let out = b.create_block();
            b.append_block_param(out, types::I32);
            let _ = MemFlags::trusted();
            for (k, c) in calls.iter().enumerate() {
                let f = b.ins().iconst(ptr, c.pre as i64);
                let call = b.ins().call_indirect(pre_ref, f, &[machine]);
                let r = b.inst_results(call)[0];
                // 1: a step made another way; 2: none.
                let go = b.create_block();
                let not_go = b.create_block();
                b.ins().brif(r, not_go, &[], go, &[]);
                b.switch_to_block(not_go);
                b.seal_block(not_go);
                let one = b.ins().icmp_imm(cranelift_codegen::ir::condcodes::IntCC::Equal, r, 1);
                let here = b.ins().iconst(types::I32, k as i64);
                let after = b.ins().iconst(types::I32, k as i64 + 1);
                let n = b.ins().select(one, after, here);
                b.ins().jump(out, &[n.into()]);
                b.switch_to_block(go);
                b.seal_block(go);
                let f = b.ins().iconst(ptr, c.run as i64);
                let a = b.ins().iconst(ptr, c.arg as i64);
                b.ins().call_indirect(run_ref, f, &[machine, a]);
                let f = b.ins().iconst(ptr, c.post as i64);
                let call = b.ins().call_indirect(pre_ref, f, &[machine]);
                let r = b.inst_results(call)[0];
                let next = b.create_block();
                let stop = b.create_block();
                b.ins().brif(r, stop, &[], next, &[]);
                b.switch_to_block(stop);
                b.seal_block(stop);
                let n = b.ins().iconst(types::I32, k as i64 + 1);
                b.ins().jump(out, &[n.into()]);
                b.switch_to_block(next);
                b.seal_block(next);
            }
            let n = b.ins().iconst(types::I32, calls.len() as i64);
            b.ins().jump(out, &[n.into()]);
            b.switch_to_block(out);
            b.seal_block(out);
            let n = b.block_params(out)[0];
            b.ins().return_(&[n]);
            b.finalize();
        }
        let name = format!("run{}", self.next);
        self.next += 1;
        let id = m.declare_function(&name, Linkage::Local, &ctx.func.signature).ok()?;
        m.define_function(id, &mut ctx).ok()?;
        m.clear_context(&mut ctx);
        m.finalize_definitions().ok()?;
        self.count += 1;
        Some(m.get_finalized_function(id) as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct M {
        log: Vec<u32>,
        stop_at: u32,
    }
    extern "C" fn pre(m: *mut M) -> u32 {
        let m = unsafe { &mut *m };
        m.log.push(1);
        0
    }
    extern "C" fn run(m: *mut M, a: usize) -> u32 {
        let m = unsafe { &mut *m };
        m.log.push(100 + a as u32);
        0
    }
    extern "C" fn post(m: *mut M) -> u32 {
        let m = unsafe { &mut *m };
        m.log.push(2);
        u32::from(m.log.len() as u32 >= m.stop_at)
    }

    #[test]
    fn a_run_calls_in_order_and_stops_when_post_says() {
        let mut e = Engine::default();
        let calls: Vec<Call> = (0..3).map(|k| Call { pre: pre as usize, run: run as usize, arg: k, post: post as usize }).collect();
        let f = e.compile(&calls).expect("compiled");
        let f: extern "C" fn(*mut M) -> u32 = unsafe { std::mem::transmute(f) };
        let mut m = M { log: Vec::new(), stop_at: 100 };
        assert_eq!(f(&mut m), 3);
        assert_eq!(m.log, vec![1, 100, 2, 1, 101, 2, 1, 102, 2]);
        let mut m = M { log: Vec::new(), stop_at: 6 };
        assert_eq!(f(&mut m), 2, "post stopped it after the second");
        e.clear();
        assert_eq!(e.count(), 0);
    }
}
