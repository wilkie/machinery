//! Values the common path computes but only the rare one uses, moved there.
//!
//! The emitter keeps what it owes the machine (a field written, not yet
//! stored) as host values, and stores them on the way out: a run's exits,
//! the cold side of a branch. A value computed for that alone (the flags of
//! a compare, overwritten by the next add before anything reads them) is
//! still computed on the common path, where it was made, and passed along it
//! to every join. After the function is built:
//!
//! - `factor_params`: a join's parameter used only on rare paths, whose
//!   value every edge computes the same way (the same operations, on
//!   different inputs), is computed in the join from those inputs instead,
//!   passed in its place. The edges' computations go.
//! - `sink_cold`: a pure instruction whose results are used only in cold
//!   blocks (or by other such instructions) is copied into each cold block
//!   that uses it, and removed from where it was.
//!
//! Cold blocks: those the emitter marked (the branches it means to be rare,
//! the run's exits), and those reached only from cold ones. Nothing the
//! common path uses moves.

use cranelift_codegen::entity::{EntitySet, SecondaryMap};
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{Block, Function, Inst, InstructionData, Opcode, Type, Value, ValueDef};

/// Whether `inst` may move: it computes values and does nothing else.
fn pure(func: &Function, inst: Inst) -> bool {
    let op = func.dfg.insts[inst].opcode();
    !(op.can_load() || op.can_store() || op.is_call() || op.is_branch() || op.is_terminator() || op.can_trap() || op.other_side_effects())
        && !func.dfg.inst_results(inst).is_empty()
}

/// The blocks only rare paths reach: marked cold, or reached only from cold
/// ones.
fn cold_blocks(func: &Function) -> EntitySet<Block> {
    let mut cold: EntitySet<Block> = EntitySet::new();
    let Some(entry) = func.layout.entry_block() else { return cold };
    let mut preds: SecondaryMap<Block, Vec<Block>> = SecondaryMap::new();
    let mut succs: SecondaryMap<Block, Vec<Block>> = SecondaryMap::new();
    let mut work: Vec<Block> = Vec::new();
    for b in func.layout.blocks() {
        if func.layout.is_cold(b) {
            cold.insert(b);
            work.push(b);
        }
        if let Some(inst) = func.layout.last_inst(b) {
            for dest in func.dfg.insts[inst].branch_destination(&func.dfg.jump_tables, &func.dfg.exception_tables) {
                let t = dest.block(&func.dfg.value_lists);
                preds[t].push(b);
                succs[b].push(t);
            }
        }
    }
    // Reached only from cold blocks.
    while let Some(b) = work.pop() {
        for i in 0..succs[b].len() {
            let t = succs[b][i];
            if t != entry && !cold.contains(t) && preds[t].iter().all(|p| cold.contains(*p)) {
                cold.insert(t);
                work.push(t);
            }
        }
    }
    cold
}

/// Every use of every value (aliases resolved): the instruction and its
/// block.
fn uses_of(func: &Function) -> SecondaryMap<Value, Vec<(Inst, Block)>> {
    let mut uses: SecondaryMap<Value, Vec<(Inst, Block)>> = SecondaryMap::new();
    for b in func.layout.blocks() {
        for inst in func.layout.block_insts(b) {
            for v in func.dfg.inst_values(inst) {
                uses[func.dfg.resolve_aliases(v)].push((inst, b));
            }
        }
    }
    uses
}

/// An instruction's shape: all of it but the values it takes (None: a
/// form not compared).
fn shape(func: &Function, inst: Inst) -> Option<(Opcode, Type, i64, Option<IntCC>)> {
    use InstructionData as D;
    let d = &func.dfg.insts[inst];
    let imm = match d {
        D::Unary { .. } | D::Binary { .. } | D::Ternary { .. } | D::IntCompare { .. } => 0,
        D::UnaryImm { imm, .. } | D::BinaryImm64 { imm, .. } | D::IntCompareImm { imm, .. } => imm.bits(),
        D::BinaryImm8 { imm, .. } => *imm as i64,
        _ => return None,
    };
    Some((d.opcode(), func.dfg.ctrl_typevar(inst), imm, d.cond_code()))
}

/// What each edge into a block passes for one of its parameters, as a tree
/// the edges share: the same value on every edge; a pure computation of the
/// same shape on every edge, of such trees; or values the block must take.
enum Tree {
    Same(Value),
    Op { inst: Inst, result: usize, kids: Vec<Tree> },
    Leaf(Vec<Value>),
}

fn tree(func: &Function, vals: &[Value], budget: &mut usize) -> Tree {
    let vals: Vec<Value> = vals.iter().map(|v| func.dfg.resolve_aliases(*v)).collect();
    if vals.iter().all(|v| *v == vals[0]) {
        return Tree::Same(vals[0]);
    }
    let defs: Option<Vec<(Inst, usize)>> = vals
        .iter()
        .map(|v| match func.dfg.value_def(*v) {
            ValueDef::Result(i, r) if pure(func, i) => Some((i, r)),
            _ => None,
        })
        .collect();
    if let (Some(defs), true) = (defs, *budget > 0) {
        let n = func.dfg.inst_args(defs[0].0).len();
        let s0 = shape(func, defs[0].0);
        if s0.is_some() && defs.iter().all(|(i, r)| *r == defs[0].1 && func.dfg.inst_args(*i).len() == n && shape(func, *i) == s0) {
            *budget -= 1;
            let kids = (0..n).map(|a| tree(func, &defs.iter().map(|(i, _)| func.dfg.inst_args(*i)[a]).collect::<Vec<_>>(), budget)).collect();
            return Tree::Op { inst: defs[0].0, result: defs[0].1, kids };
        }
    }
    Tree::Leaf(vals)
}

fn leaves(t: &Tree, out: &mut Vec<Vec<Value>>) {
    match t {
        Tree::Leaf(vs) => {
            if !out.contains(vs) {
                out.push(vs.clone());
            }
        }
        Tree::Op { kids, .. } => kids.iter().for_each(|k| leaves(k, out)),
        Tree::Same(_) => {}
    }
}

/// `t` computed before `at`, its leaves the block's parameters `params`.
fn build(func: &mut Function, t: &Tree, at: Inst, params: &[(Vec<Value>, Value)]) -> Value {
    match t {
        Tree::Same(v) => *v,
        Tree::Leaf(vs) => params.iter().find(|(l, _)| l == vs).map(|(_, p)| *p).expect("a leaf's parameter"),
        Tree::Op { inst, result, kids } => {
            let args: Vec<Value> = kids.iter().map(|k| build(func, k, at, params)).collect();
            let c = func.dfg.clone_inst(*inst);
            let mut it = args.into_iter();
            func.dfg.map_inst_values(c, |_| it.next().expect("an argument"));
            func.layout.insert_inst(c, at);
            func.dfg.inst_results(c)[*result]
        }
    }
}

/// Uses of the values in `map` (or their aliases) changed to their
/// replacements, throughout.
fn replace(func: &mut Function, map: &SecondaryMap<Value, Option<Value>>) {
    for b in func.layout.blocks().collect::<Vec<_>>() {
        for inst in func.layout.block_insts(b).collect::<Vec<_>>() {
            let m: Vec<(Value, Value)> = func.dfg.inst_values(inst).filter_map(|v| map[func.dfg.resolve_aliases(v)].map(|r| (v, r))).collect();
            if !m.is_empty() {
                func.dfg.map_inst_values(inst, |v| m.iter().find(|(o, _)| *o == v).map(|(_, n)| *n).unwrap_or(v));
            }
        }
    }
}

/// Parameters used only on rare paths whose values every edge computes the
/// same way: computed in the block from what the edges compute them from.
pub(crate) fn factor_params(func: &mut Function) {
    for _ in 0..8 {
        let cold = cold_blocks(func);
        let uses = uses_of(func);
        let blocks: Vec<Block> = func.layout.blocks().collect();
        // The edges into each block (a jump table's: left alone).
        let mut into: SecondaryMap<Block, Vec<(Inst, usize)>> = SecondaryMap::new();
        let mut tabled: EntitySet<Block> = EntitySet::new();
        for &b in &blocks {
            if let Some(inst) = func.layout.last_inst(b) {
                let data = &func.dfg.insts[inst];
                let table = matches!(data, InstructionData::BranchTable { .. });
                for (di, d) in data.branch_destination(&func.dfg.jump_tables, &func.dfg.exception_tables).iter().enumerate() {
                    let t = d.block(&func.dfg.value_lists);
                    if table {
                        tabled.insert(t);
                    } else {
                        into[t].push((inst, di));
                    }
                }
            }
        }
        let mut roots: SecondaryMap<Value, Option<Value>> = SecondaryMap::new();
        let mut changed = false;
        for &j in blocks.iter().rev() {
            let params: Vec<Value> = func.dfg.block_params(j).to_vec();
            if params.is_empty() || tabled.contains(j) || into[j].is_empty() {
                continue;
            }
            let edges = into[j].clone();
            let arg = |func: &Function, (inst, di): (Inst, usize), ix: usize| -> Option<Value> {
                let d = func.dfg.insts[inst].branch_destination(&func.dfg.jump_tables, &func.dfg.exception_tables)[di];
                d.args(&func.dfg.value_lists).nth(ix).and_then(|a| a.as_value())
            };
            let mut factored: Vec<(usize, Tree)> = Vec::new();
            for (ix, &p) in params.iter().enumerate() {
                if !uses[p].iter().all(|(_, ub)| cold.contains(*ub)) {
                    continue;
                }
                let Some(vals) = edges.iter().map(|e| arg(func, *e, ix)).collect::<Option<Vec<Value>>>() else { continue };
                let mut budget = 64;
                let t = tree(func, &vals, &mut budget);
                if !matches!(t, Tree::Leaf(_)) {
                    factored.push((ix, t));
                }
            }
            if factored.is_empty() {
                continue;
            }
            let Some(at) = func.layout.first_inst(j) else { continue };
            // The leaves: the block's new parameters, each passed by its
            // edges.
            let mut ls: Vec<Vec<Value>> = Vec::new();
            for (_, t) in &factored {
                leaves(t, &mut ls);
            }
            let mut ps: Vec<(Vec<Value>, Value)> = Vec::new();
            for vs in ls {
                let ty = func.dfg.value_type(vs[0]);
                let np = func.dfg.append_block_param(j, ty);
                for (e, v) in edges.iter().zip(&vs) {
                    let dfg = &mut func.dfg;
                    dfg.insts[e.0].branch_destination_mut(&mut dfg.jump_tables, &mut dfg.exception_tables)[e.1].append_argument(*v, &mut dfg.value_lists);
                }
                ps.push((vs, np));
            }
            // The parameters computed in the block.
            for (ix, t) in &factored {
                roots[params[*ix]] = Some(build(func, t, at, &ps));
            }
            // The old parameters and their arguments go (from the last).
            let mut ixs: Vec<usize> = factored.iter().map(|(ix, _)| *ix).collect();
            ixs.sort_unstable_by(|a, b| b.cmp(a));
            for ix in ixs {
                for e in &edges {
                    let dfg = &mut func.dfg;
                    dfg.insts[e.0].branch_destination_mut(&mut dfg.jump_tables, &mut dfg.exception_tables)[e.1].remove(ix, &mut dfg.value_lists);
                }
                func.dfg.remove_block_param(params[ix]);
            }
            changed = true;
        }
        if !changed {
            break;
        }
        // (A root may be another's parameter: followed to the end.)
        for v in roots.keys().collect::<Vec<_>>() {
            let mut r = roots[v];
            let mut n = 0;
            while let Some(x) = r.and_then(|x| roots[x]) {
                r = Some(x);
                n += 1;
                if n > 64 {
                    break;
                }
            }
            roots[v] = r;
        }
        replace(func, &roots);
    }
}

/// The common path's values used only on rare paths, moved there.
pub(crate) fn sink_cold(func: &mut Function) {
    let cold = cold_blocks(func);
    let blocks: Vec<Block> = func.layout.blocks().collect();
    if !blocks.iter().any(|b| cold.contains(*b)) {
        return;
    }
    let uses = uses_of(func);
    let mut order: SecondaryMap<Inst, u32> = SecondaryMap::new();
    let mut hot: Vec<Inst> = Vec::new();
    let mut n = 0;
    for &b in &blocks {
        for inst in func.layout.block_insts(b) {
            order[inst] = n;
            n += 1;
            if !cold.contains(b) && pure(func, inst) {
                hot.push(inst);
            }
        }
    }
    // The pure instructions on the common path used only on rare paths (or
    // by others so used).
    let mut moves: EntitySet<Inst> = EntitySet::new();
    loop {
        let mut grew = false;
        for &inst in hot.iter().rev() {
            if moves.contains(inst) {
                continue;
            }
            if func.dfg.inst_results(inst).iter().all(|r| uses[*r].iter().all(|(u, ub)| cold.contains(*ub) || moves.contains(*u))) {
                moves.insert(inst);
                grew = true;
            }
        }
        if !grew {
            break;
        }
    }
    let moved: Vec<Inst> = hot.iter().copied().filter(|i| moves.contains(*i)).collect();
    if moved.is_empty() {
        return;
    }
    let mut def: SecondaryMap<Value, Option<Inst>> = SecondaryMap::new();
    for &i in &moved {
        for r in func.dfg.inst_results(i) {
            def[*r] = Some(i);
        }
    }
    // Each cold block that uses a moved value gets its own copies, at its
    // start, in the order they were made.
    for b in blocks.iter().copied().filter(|b| cold.contains(*b)) {
        let mut need: EntitySet<Inst> = EntitySet::new();
        let mut list: Vec<Inst> = Vec::new();
        let mut stack: Vec<Inst> = func.layout.block_insts(b).flat_map(|i| func.dfg.inst_values(i).collect::<Vec<_>>()).filter_map(|v| def[func.dfg.resolve_aliases(v)]).collect();
        while let Some(i) = stack.pop() {
            if need.insert(i) {
                list.push(i);
                stack.extend(func.dfg.inst_args(i).iter().filter_map(|v| def[func.dfg.resolve_aliases(*v)]));
            }
        }
        if list.is_empty() {
            continue;
        }
        list.sort_by_key(|i| order[*i]);
        let Some(first) = func.layout.first_inst(b) else { continue };
        let insts: Vec<Inst> = func.layout.block_insts(b).collect();
        // A value moved here, to its copy (an alias of one too).
        let mut copy: Vec<(Value, Value)> = Vec::new();
        let swap = |func: &Function, copy: &[(Value, Value)], inst: Inst| -> Vec<(Value, Value)> {
            func.dfg
                .inst_values(inst)
                .filter_map(|v| {
                    let r = func.dfg.resolve_aliases(v);
                    copy.iter().find(|(o, _)| *o == r).map(|(_, n)| (v, *n))
                })
                .collect()
        };
        for i in list {
            let c = func.dfg.clone_inst(i);
            let m = swap(func, &copy, c);
            func.dfg.map_inst_values(c, |v| m.iter().find(|(o, _)| *o == v).map(|(_, n)| *n).unwrap_or(v));
            for (old, new) in func.dfg.inst_results(i).to_vec().into_iter().zip(func.dfg.inst_results(c).to_vec()) {
                copy.push((old, new));
            }
            func.layout.insert_inst(c, first);
        }
        for inst in insts {
            let m = swap(func, &copy, inst);
            if !m.is_empty() {
                func.dfg.map_inst_values(inst, |v| m.iter().find(|(o, _)| *o == v).map(|(_, n)| *n).unwrap_or(v));
            }
        }
    }
    for i in moved {
        func.layout.remove_inst(i);
    }
}

#[cfg(test)]
mod tests {
    use cranelift_codegen::ir::{types, AbiParam, Function, InstBuilder, Signature, UserFuncName};
    use cranelift_codegen::isa::CallConv;
    use cranelift_codegen::verifier::verify_function;
    use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};

    /// `fn(x) -> i32`: a value made from x on the common path, used only on
    /// the cold one (x > 10: return it), else return x.
    fn rare_use(join: bool) -> Function {
        let mut sig = Signature::new(CallConv::SystemV);
        sig.params.push(AbiParam::new(types::I32));
        sig.returns.push(AbiParam::new(types::I32));
        let mut f = Function::with_name_signature(UserFuncName::default(), sig);
        let mut ctx = FunctionBuilderContext::new();
        let mut b = FunctionBuilder::new(&mut f, &mut ctx);
        let entry = b.create_block();
        let cold = b.create_block();
        let hot = b.create_block();
        let mid = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let x = b.block_params(entry)[0];
        if join {
            // Two edges into `mid`, each passing x * 7 + 1 of its own x.
            let other = b.create_block();
            let c = b.ins().icmp_imm(cranelift_codegen::ir::condcodes::IntCC::Equal, x, 3);
            b.append_block_param(mid, types::I32);
            b.append_block_param(mid, types::I32);
            let y = b.ins().imul_imm(x, 7);
            let y = b.ins().iadd_imm(y, 1);
            b.ins().brif(c, other, &[], mid, &[x.into(), y.into()]);
            b.switch_to_block(other);
            let x2 = b.ins().iadd_imm(x, 0);
            let y2 = b.ins().imul_imm(x2, 7);
            let y2 = b.ins().iadd_imm(y2, 1);
            b.ins().jump(mid, &[x2.into(), y2.into()]);
            b.switch_to_block(mid);
            let (x, y) = (b.block_params(mid)[0], b.block_params(mid)[1]);
            let big = b.ins().icmp_imm(cranelift_codegen::ir::condcodes::IntCC::SignedGreaterThan, x, 10);
            b.ins().brif(big, cold, &[], hot, &[]);
            b.set_cold_block(cold);
            b.switch_to_block(cold);
            b.ins().return_(&[y]);
            b.switch_to_block(hot);
            b.ins().return_(&[x]);
        } else {
            let y = b.ins().imul_imm(x, 7);
            let y = b.ins().iadd_imm(y, 1);
            let big = b.ins().icmp_imm(cranelift_codegen::ir::condcodes::IntCC::SignedGreaterThan, x, 10);
            b.ins().brif(big, cold, &[], hot, &[]);
            b.set_cold_block(cold);
            b.switch_to_block(cold);
            b.ins().return_(&[y]);
            b.switch_to_block(hot);
            b.ins().return_(&[x]);
            let _ = mid;
        }
        b.seal_all_blocks();
        b.finalize();
        f
    }

    fn multiplies(f: &Function, b: cranelift_codegen::ir::Block) -> usize {
        f.layout.block_insts(b).filter(|i| f.dfg.insts[*i].opcode() == cranelift_codegen::ir::Opcode::ImulImm).count()
    }

    #[test]
    fn a_value_used_only_on_the_rare_path_moves_there() {
        let mut f = rare_use(false);
        let entry = f.layout.entry_block().unwrap();
        assert_eq!(multiplies(&f, entry), 1);
        super::sink_cold(&mut f);
        verify_function(&f, &cranelift_codegen::settings::Flags::new(cranelift_codegen::settings::builder())).expect("valid");
        assert_eq!(multiplies(&f, entry), 0, "moved off the common path:\n{}", f.display());
        assert_eq!(f.layout.blocks().map(|b| multiplies(&f, b)).sum::<usize>(), 1);
    }

    #[test]
    fn a_parameter_both_edges_compute_alike_is_computed_where_needed() {
        let mut f = rare_use(true);
        super::factor_params(&mut f);
        super::sink_cold(&mut f);
        verify_function(&f, &cranelift_codegen::settings::Flags::new(cranelift_codegen::settings::builder())).expect("valid");
        // The edges no longer multiply: only the cold block does.
        let cold: Vec<_> = f.layout.blocks().filter(|b| f.layout.is_cold(*b)).collect();
        let total: usize = f.layout.blocks().map(|b| multiplies(&f, b)).sum();
        let in_cold: usize = cold.iter().map(|b| multiplies(&f, *b)).sum();
        assert_eq!((total, in_cold), (1, 1), "{}", f.display());
    }
}
