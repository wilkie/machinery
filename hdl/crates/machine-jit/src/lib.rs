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

use cranelift_codegen::ir::condcodes::IntCC;
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

    /// The host function for `calls`: `extern "C" fn(*mut machine, from:
    /// u32) -> u32`, entered at call `from`, answering the steps it made.
    /// None if the code generator is unavailable.
    pub fn compile(&mut self, calls: &[Call]) -> Option<usize> {
        if self.module.is_none() {
            self.module = new_module();
        }
        let m = self.module.as_mut()?;
        let ptr = m.target_config().pointer_type();
        let mut ctx = m.make_context();
        ctx.func.signature.params.push(AbiParam::new(ptr));
        ctx.func.signature.params.push(AbiParam::new(types::I32));
        ctx.func.signature.returns.push(AbiParam::new(types::I32));
        let mut run_sig = m.make_signature();
        run_sig.params.push(AbiParam::new(ptr));
        run_sig.params.push(AbiParam::new(ptr));
        run_sig.returns.push(AbiParam::new(types::I32));
        {
            let mut b = FunctionBuilder::new(&mut ctx.func, &mut self.fbc);
            let run_ref = b.import_signature(run_sig);
            let entry = b.create_block();
            b.append_block_params_for_function_params(entry);
            b.switch_to_block(entry);
            b.seal_block(entry);
            let machine = b.block_params(entry)[0];
            let from = b.block_params(entry)[1];
            // The way out: the steps made so far.
            let out = b.create_block();
            b.append_block_param(out, types::I32);
            // A block a step, entered by `from` (past the end: none made).
            let steps: Vec<_> = calls.iter().map(|_| b.create_block()).collect();
            let mut switch = Switch::new();
            for (k, blk) in steps.iter().enumerate() {
                switch.set_entry(k as u128, *blk);
            }
            let none = b.create_block();
            switch.emit(&mut b, from, none);
            b.switch_to_block(none);
            b.seal_block(none);
            let zero = b.ins().iconst(types::I32, 0);
            b.ins().jump(out, &[zero.into()]);
            for (k, c) in calls.iter().enumerate() {
                b.switch_to_block(steps[k]);
                let f = b.ins().iconst(ptr, c.run as i64);
                let a = b.ins().iconst(ptr, c.arg as i64);
                let call = b.ins().call_indirect(run_ref, f, &[machine, a]);
                let r = b.inst_results(call)[0];
                let next = b.create_block();
                let stop = b.create_block();
                b.ins().brif(r, stop, &[], next, &[]);
                // 1: made, then stop; 2: not made.
                b.switch_to_block(stop);
                b.seal_block(stop);
                let made = b.ins().icmp_imm(IntCC::Equal, r, 1);
                let here = b.ins().iconst(types::I32, k as i64);
                let after = b.ins().iconst(types::I32, k as i64 + 1);
                let n = b.ins().select(made, after, here);
                let n = b.ins().isub(n, from);
                b.ins().jump(out, &[n.into()]);
                b.switch_to_block(next);
                b.seal_block(next);
                let to = steps.get(k + 1).copied().unwrap_or(out);
                if to == out {
                    let n = b.ins().iconst(types::I32, calls.len() as i64);
                    let n = b.ins().isub(n, from);
                    b.ins().jump(out, &[n.into()]);
                } else {
                    b.ins().jump(to, &[]);
                }
            }
            for blk in &steps {
                b.seal_block(*blk);
            }
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
}
