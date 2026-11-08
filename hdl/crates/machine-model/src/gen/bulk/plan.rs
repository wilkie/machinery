//! An instruction's paths, followed one by one: the `bulk` claim, and what
//! the kernel learns from them.

use std::collections::{HashMap, HashSet};

use machine_sema::hir::{self, ArmBody, Block, Expr, ExprKind as H, LocalId, StmtKind as S};
use machine_sema::program::DefId;
use machine_sema::ty::Ty;
use machine_syntax::ast::BinaryOp;
use machine_syntax::span::Span;

use super::eval::{Eval, Flow, Phase};
use super::val::{ty_bits, Kind, Loc, Val};
use super::{Kernel, Shape, Site};
use crate::gen::{Gen, GenError, R};

/// The search over an element's choices: each condition met for the first
/// time is answered 0; each other answer is tried later, with the answers
/// given before it (those after it may not be met on that path).
pub(super) struct Search {
    todo: Vec<HashMap<String, u32>>,
    pub runs: usize,
}

impl Search {
    pub(super) fn new() -> Search {
        Search { todo: vec![HashMap::new()], runs: 0 }
    }
    /// The next path's answers.
    pub(super) fn next(&mut self) -> Option<HashMap<String, u32>> {
        let c = self.todo.pop()?;
        self.runs += 1;
        Some(c)
    }
    /// A path run with `given`, meeting `fresh` (in order) for the first
    /// time: each's other answers to try.
    pub(super) fn explored(&mut self, given: &HashMap<String, u32>, fresh: Vec<(String, u32)>) {
        let mut before = given.clone();
        for (key, arity) in fresh {
            for alt in 1..arity {
                let mut c = before.clone();
                c.insert(key.clone(), alt);
                self.todo.push(c);
            }
            before.insert(key, 0);
        }
    }
}

const MAX_PATHS: usize = 4096;

impl Gen<'_> {
    /// Whether instruction `i`'s element is a stream on some path (`bulk`),
    /// and which; an error naming why not. With it, its kernel, or why it
    /// has none.
    pub(crate) fn bulk_plan(&self, i: &hir::Instruction) -> R<(Shape, Result<Kernel, String>)> {
        let Some(sem) = &i.semantics else {
            return Err(GenError::new(format!("`{}`: `bulk` with no semantics", i.name)));
        };
        let Some(rep) = &i.repeat else {
            return Err(GenError::new(format!("`{}`: `bulk` with no repeat", i.name)));
        };
        let mem = self.isa.istream.as_ref().map(|s| s.space).ok_or_else(|| GenError::new("`bulk` needs an istream"))?;
        let mut side: Vec<&Expr> = vec![&rep.next];
        side.extend(rep.cond.iter());
        let (written, fns) = self.bulk_written(sem, &side);
        let mut search = Search::new();
        let mut reasons: Vec<String> = Vec::new();
        let mut shape = None;
        let mut stops: HashSet<Site> = HashSet::new();
        // What the kernel's paths share, and why there is none.
        let mut kern: Option<Kernel> = None;
        let mut why: Option<String> = None;
        let mut regs_written: Vec<DefId> = Vec::new();
        let trace = std::env::var_os("MACHINE_BULK").is_some();
        while let Some(choices) = search.next() {
            if search.runs > MAX_PATHS {
                why.get_or_insert_with(|| "too many paths to follow".into());
                break;
            }
            let mut e = Eval::new(self, written.clone(), mem, choices.clone(), &i.name, std::mem::take(&mut stops));
            // The timing entry as the element starts, the element, and what
            // the repeat makes of it.
            let t0 = i.timing.as_ref().map(|t| e.expr(&i.timing_locals, &mut HashMap::new(), t));
            e.phase = Phase::Element;
            let mut env: HashMap<LocalId, Val> = HashMap::new();
            if matches!(e.block(&i.locals, &mut env, sem), Flow::Raise) && e.p.fail.is_none() {
                e.p.fail = Some("it raises".into());
                e.p.raised = true;
            }
            let class = e.classify();
            let elem_writes = e.p.writes.clone();
            let accesses = (e.p.mem_reads.len(), e.p.mem_writes.len());
            e.phase = Phase::Side { steady: false };
            let nx = e.expr(&rep.locals, &mut HashMap::new(), &rep.next);
            e.phase = Phase::Cond;
            let cv = rep.cond.as_ref().map(|c| e.expr(&rep.locals, &mut HashMap::new(), c));
            e.phase = Phase::Side { steady: true };
            let t1 = i.timing.as_ref().map(|t| e.expr(&i.timing_locals, &mut HashMap::new(), t));
            search.explored(&choices, std::mem::take(&mut e.p.fresh));
            if trace {
                eprintln!("bulk path {}: writes {:?} reads {:?} stores {:?} fail {:?} raised {} stopped {}", i.name, e.p.writes, e.p.mem_reads, e.p.mem_writes, e.p.fail, e.p.raised, e.p.stopped);
            }
            for (l, _) in &elem_writes {
                if !regs_written.contains(&l.0) {
                    regs_written.push(l.0);
                }
            }
            match &class {
                Ok(s) => {
                    shape.get_or_insert(*s);
                }
                Err(w) => {
                    if !reasons.contains(w) {
                        reasons.push(w.clone());
                    }
                }
            }
            // A path the dry code stops on, or that raises: the kernel need
            // not take it. Any other it must.
            if !(e.p.stopped || e.p.raised) {
                let k = match class {
                    Ok(s @ (Shape::Fill | Shape::Copy | Shape::Scan | Shape::Compare | Shape::Load)) => {
                        let timing_ok = [&t0, &t1].iter().all(|t| t.as_ref().is_none_or(Val::invariant));
                        if !timing_ok {
                            Err("its time depends on the element".to_string())
                        } else if (e.p.mem_reads.len(), e.p.mem_writes.len()) != accesses {
                            Err("`next`, `while` or the timing entry reach memory".to_string())
                        } else {
                            e.kernel(s, &elem_writes, &nx, cv.as_ref())
                        }
                    }
                    Ok(s) => Err(format!("a {s:?} path")),
                    Err(w) => Err(format!("a path the dry code finishes that is not a stream ({w})")),
                };
                match (k, &mut kern) {
                    (Ok(k), None) => kern = Some(k),
                    (Ok(k), Some(have)) => {
                        if let Err(w) = have.merge(k) {
                            why.get_or_insert(w);
                        }
                    }
                    (Err(w), _) => {
                        why.get_or_insert(w);
                    }
                }
            }
            stops = std::mem::take(&mut e.stops);
        }
        let Some(s) = shape else {
            return Err(GenError::new(format!(
                "`{}`: `repeat ... bulk`, but its element is not a fill, copy, scan, compare, load or port stream on any path ({})",
                i.name,
                reasons.join("; ")
            )));
        };
        let k = match (why, kern) {
            (Some(w), _) => Err(w),
            (None, None) => Err("no path the dry code finishes".into()),
            (None, Some(mut k)) => {
                // A register the element moves on one path and not another
                // (or the count): it is saved round the dry code all the same.
                if k.moving.iter().map(|m| m.0).collect::<HashSet<_>>().len() != k.moving.len() {
                    Err("a register both moved and counted".into())
                } else {
                    for (d, _) in &k.moving {
                        if !regs_written.contains(d) {
                            regs_written.push(*d);
                        }
                    }
                    k.written = regs_written;
                    k.fns = fns;
                    let mut stops: Vec<Site> = stops.into_iter().collect();
                    stops.sort_by_key(|s| (s.span.start, s.span.end));
                    k.stops = stops;
                    Ok(k)
                }
            }
        };
        Ok((s, k))
    }

    /// The register locations the element may write (its semantics, the
    /// exprs in `also`, and the fns they call; an operand kind's selected
    /// registers all of them), and the fns the element may call.
    fn bulk_written(&self, b: &Block, also: &[&Expr]) -> (Vec<Loc>, Vec<DefId>) {
        let out = std::cell::RefCell::new(Vec::new());
        let mut seen: HashSet<DefId> = HashSet::new();
        let mut fns: Vec<DefId> = Vec::new();
        let stack = std::cell::RefCell::new(Vec::<DefId>::new());
        let mut blocks: Vec<&Block> = vec![b];
        let place_locs = |p: &Expr, out: &mut Vec<Loc>| {
            let mut e = p;
            loop {
                match &e.kind {
                    H::Register(_) | H::RegField(..) | H::RegView(..) => {
                        out.extend(self.bulk_locs(e));
                        return;
                    }
                    H::KindSelect(k, _) => {
                        for r in &self.kinds[k].rules {
                            if r.kind == hir::RuleKind::Select {
                                let mut on = |x: &Expr| out.extend(self.bulk_locs(x));
                                match &r.body {
                                    ArmBody::Expr(x) => hir::visit_expr(x, &mut |_| {}, &mut on),
                                    ArmBody::Block(bl) => hir::visit_block(bl, &mut |_| {}, &mut on),
                                }
                            }
                        }
                        return;
                    }
                    H::Field(b, _) | H::Index(b, _) | H::Slice(b, ..) | H::TupleField(b, _) => e = b,
                    _ => return,
                }
            }
        };
        let mut side_fns: Vec<DefId> = Vec::new();
        for x in also {
            hir::visit_expr(
                x,
                &mut |_| {},
                &mut |x| {
                    if let H::FnCall(g, _) = &x.kind {
                        side_fns.push(*g);
                    }
                },
            );
        }
        // The element's fns first (the dry code calls them), then `next`'s
        // and `while`'s.
        for phase in 0..2 {
            if phase == 1 {
                stack.borrow_mut().extend(side_fns.iter().copied());
            }
            loop {
                let Some(blk) = blocks.pop() else {
                    let Some(f) = stack.borrow_mut().pop() else { break };
                    if seen.insert(f) {
                        if let Some(fb) = self.fns.get(&f) {
                            if !fb.external {
                                if phase == 0 {
                                    fns.push(f);
                                }
                                blocks.push(&fb.body);
                            }
                        }
                    }
                    continue;
                };
                hir::visit_block(
                    blk,
                    &mut |st| {
                        if let S::Assign(p, _) = &st.kind {
                            place_locs(p, &mut out.borrow_mut());
                        }
                    },
                    &mut |x| {
                        if let H::FnCall(g, _) = &x.kind {
                            stack.borrow_mut().push(*g);
                        }
                    },
                );
            }
        }
        (out.into_inner(), fns)
    }

    /// The locations a register place covers (a view: each field it maps).
    pub(super) fn bulk_locs(&self, e: &Expr) -> Vec<Loc> {
        match &e.kind {
            H::Register(d) => match self.regs.get(d) {
                Some((_, r)) => vec![(*d, 0, ty_bits(&r.ty))],
                None => Vec::new(),
            },
            H::RegField(d, i) => match self.regs.get(d) {
                Some((_, r)) => {
                    let f = &r.fields[*i as usize];
                    vec![(*d, f.offset, ty_bits(&f.ty))]
                }
                None => Vec::new(),
            },
            H::RegView(d, v) => match self.regs.get(d) {
                Some((_, r)) => r.views[*v as usize]
                    .mapping
                    .iter()
                    .map(|i| {
                        let f = &r.fields[*i as usize];
                        (*d, f.offset, ty_bits(&f.ty))
                    })
                    .collect(),
                None => Vec::new(),
            },
            _ => Vec::new(),
        }
    }

    /// The locals standing for the values an element loads (`ld1`, `ld2`,
    /// ...), of these types.
    pub(super) fn bulk_load_locals(tys: &[Ty]) -> Vec<hir::Local> {
        tys.iter().enumerate().map(|(n, t)| hir::Local { name: format!("ld{}", n + 1), ty: t.clone(), span: Span::default(), mutable: false }).collect()
    }
}

impl Kernel {
    /// Another path's kernel, taken in: the same kinds of access, `while`
    /// condition and count; each register moving in the narrowest width
    /// any path moves it in.
    fn merge(&mut self, k: Kernel) -> Result<(), String> {
        if k.shape != self.shape || k.bits != self.bits || k.loads != self.loads {
            return Err("paths of different kinds or sizes".into());
        }
        if k.cond != self.cond {
            return Err("paths whose `while` conditions differ".into());
        }
        let (mut a, b) = (std::mem::take(&mut self.moving), k.moving);
        // The count stays last.
        let (ac, bc) = (a.pop().unwrap(), b.last().copied().unwrap());
        if ac.0 != bc.0 {
            return Err("counts in different registers".into());
        }
        for (d, w) in &b[..b.len() - 1] {
            match a.iter_mut().find(|x| x.0 == *d) {
                Some(x) => x.1 = x.1.min(*w),
                None => a.push((*d, *w)),
            }
        }
        a.push((ac.0, ac.1.min(bc.1)));
        self.moving = a;
        Ok(())
    }
}

impl Eval<'_, '_> {
    /// The path's kind of stream, or why it is none.
    pub(super) fn classify(&self) -> Result<Shape, String> {
        if let Some(f) = &self.p.fail {
            return Err(f.clone());
        }
        // Pointers: written registers that move by an invariant.
        let mut moved: Vec<Loc> = Vec::new();
        for (l, v) in &self.p.writes {
            match &v.kind {
                Kind::Ptr { loc, off, .. } if loc == l && off.invariant() => moved.push(*l),
                Kind::Ptr { .. } => return Err("a register moves by something not the same each element".into()),
                // Results: from what it loaded, or invariant.
                _ => {}
            }
        }
        let streams = |a: &Val| matches!(&a.kind, Kind::Ptr { loc, off, .. } if moved.contains(loc) && off.invariant());
        let mem = self.mem;
        let reads: Vec<usize> = (0..self.p.mem_reads.len()).filter(|n| self.p.mem_reads[*n].0 == mem).collect();
        let port_reads: Vec<usize> = (0..self.p.mem_reads.len()).filter(|n| self.p.mem_reads[*n].0 != mem).collect();
        let writes: Vec<&(DefId, Val, Val, u32)> = self.p.mem_writes.iter().filter(|w| w.0 == mem).collect();
        let port_writes: Vec<&(DefId, Val, Val, u32)> = self.p.mem_writes.iter().filter(|w| w.0 != mem).collect();
        if reads.iter().any(|n| !streams(&self.p.mem_reads[*n].1)) || writes.iter().any(|w| !streams(&w.1)) {
            return Err("a memory access at an address that does not move with the element".into());
        }
        // The value of one of these reads, as it was loaded.
        let loaded = |v: &Val, of: &[usize]| matches!(v.kind, Kind::Data { load, loaded: true } if of.contains(&(load as usize - 1)));
        match (reads.len(), writes.len(), port_reads.len(), port_writes.len()) {
            (0, 1, 0, 0) if writes[0].2.invariant() => Ok(Shape::Fill),
            (0, 1, 0, 0) => Err("it writes something that changes each element without reading it".into()),
            (1, 1, 0, 0) if loaded(&writes[0].2, &reads) => Ok(Shape::Copy),
            (1, 0, 0, 0) => {
                let into_reg = self.p.writes.iter().any(|(l, v)| matches!(v.kind, Kind::Data { loaded: true, .. }) && !moved.contains(l));
                Ok(if into_reg { Shape::Load } else { Shape::Scan })
            }
            (2, 0, 0, 0) => Ok(Shape::Compare),
            (0, 1, 1, 0) if loaded(&writes[0].2, &port_reads) => Ok(Shape::PortIn),
            (1, 0, 0, 1) if loaded(&port_writes[0].2, &reads) => Ok(Shape::PortOut),
            (r, w, pr, pw) => Err(format!("{r} memory reads, {w} writes, {pr} port reads, {pw} port writes")),
        }
    }

    /// What a path tells the kernel (the element's writes `elem`, `next`'s
    /// value `nx`, the `while` condition's `cv`), or why it cannot take the
    /// path.
    pub(super) fn kernel(&self, shape: Shape, elem: &[(Loc, Val)], nx: &Val, cv: Option<&Val>) -> Result<Kernel, String> {
        if let Some(why) = &self.p.kfail {
            return Err(why.clone());
        }
        // The count: `next` takes one from a register, and the instruction
        // repeats while it is not zero.
        let Kind::Cond { cmp: Some(cmp), .. } = &nx.kind else { return Err("`next` is not a test of a count".into()) };
        let down = |v: &Val| match &v.kind {
            Kind::Ptr { loc, off, .. } if off.konst() == Some(-1) => Some(*loc),
            _ => None,
        };
        let count = match &**cmp {
            (BinaryOp::Ne, a, b) if b.konst() == Some(0) && down(a).is_some() => down(a).unwrap(),
            (BinaryOp::Ne, a, b) if a.konst() == Some(0) && down(b).is_some() => down(b).unwrap(),
            _ => return Err("`next` is not a count down to zero".into()),
        };
        let counted = self.p.writes.iter().any(|(l, v)| *l == count && down(v) == Some(count));
        if !counted || elem.iter().any(|(l, _)| l.0 == count.0) {
            return Err("the count is not counted down once by `next` alone".into());
        }
        if self.p.writes.len() != elem.len() + 1 {
            return Err("`next` writes more than the count".into());
        }
        if count.1 != 0 {
            return Err("a count not at bit 0 of its register".into());
        }
        // The pointers (from bit 0 of their registers); and the results,
        // each the element's own (from what it loaded and invariants), so
        // the last element's are the instruction's.
        let mut moving: Vec<(DefId, u32)> = Vec::new();
        for (l, v) in elem {
            match &v.kind {
                Kind::Ptr { loc, off, .. } if loc == l && off.invariant() && l.1 == 0 => match moving.iter_mut().find(|m| m.0 == l.0) {
                    Some(m) => m.1 = m.1.min(l.2),
                    None => moving.push((l.0, l.2)),
                },
                Kind::Ptr { .. } => return Err("a pointer not at bit 0 of its register".into()),
                _ if v.invariant() || v.is_data() => {}
                _ => return Err("a result that depends on more than what the element loaded".into()),
            }
        }
        for (l, v) in elem {
            if !matches!(v.kind, Kind::Ptr { .. }) && moving.iter().any(|m| m.0 == l.0) {
                return Err("a pointer's register holds a result too".into());
            }
        }
        moving.push((count.0, count.2));
        for (d, _) in &moving {
            if ty_bits(&self.g.regs[d].1.ty) > 64 {
                return Err("a register wider than 64 bits moves".into());
            }
        }
        let bits = match shape {
            Shape::Fill | Shape::Copy => self.p.mem_writes.iter().find(|w| w.0 == self.mem).map(|w| w.3).ok_or("no write")?,
            _ => self.p.mem_reads.iter().find(|r| r.0 == self.mem).map(|r| r.2).ok_or("no read")?,
        };
        if self.p.mem_reads.iter().any(|r| r.2 != bits) || self.p.mem_writes.iter().any(|w| w.3 != bits) {
            return Err("accesses of different sizes".into());
        }
        if bits % 8 != 0 || bits > 64 {
            return Err("an access of whole bytes, at most 8".into());
        }
        // The `while` condition's closed form, over what the element loaded.
        let tys: Vec<Ty> = self.p.mem_reads.iter().map(|r| r.3.clone()).collect();
        let loads = tys.iter().map(|t| self.g.rty(t).map_err(|e| e.to_string())).collect::<Result<Vec<_>, _>>()?;
        let cond = match cv {
            None => None,
            Some(v) if v.invariant() || v.is_data() => {
                let x = self.xof(v, &Ty::Bool, Span::default()).ok_or("a `while` condition with no closed form")?;
                let locals = Gen::bulk_load_locals(&tys);
                let cx = crate::gen::Cx { locals: &locals, fallible: true, recv: "self", ops: None, comp: None, opt: false };
                Some(self.g.expr(&cx, &x).map_err(|e| e.to_string())?)
            }
            Some(_) => return Err("a `while` condition on more than what the element loaded".into()),
        };
        if cond.is_some() && !matches!(shape, Shape::Scan | Shape::Compare | Shape::Load) {
            return Err("a `while` condition on a fill or copy".into());
        }
        Ok(Kernel { shape, bits, moving, written: Vec::new(), fns: Vec::new(), stops: Vec::new(), cond, loads })
    }
}

#[cfg(test)]
mod tests {
    use super::Search;
    use std::collections::HashMap;

    /// A tree of choices: `a`; under a=0, `b` (3 ways); under a=1, `c`;
    /// under b=2, `d`. Its leaves are the paths.
    fn walk(c: &HashMap<String, u32>) -> (Vec<(String, u32)>, String) {
        let mut fresh = Vec::new();
        let mut ask = |k: &str, n: u32| -> u32 {
            match c.get(k) {
                Some(v) => *v,
                None => {
                    fresh.push((k.to_string(), n));
                    0
                }
            }
        };
        let leaf = if ask("a", 2) == 0 {
            match ask("b", 3) {
                2 => format!("b2d{}", ask("d", 2)),
                b => format!("b{b}"),
            }
        } else {
            format!("c{}", ask("c", 2))
        };
        (fresh, leaf)
    }

    #[test]
    fn the_search_meets_every_path_once() {
        let mut s = Search::new();
        let mut leaves = Vec::new();
        while let Some(c) = s.next() {
            let (fresh, leaf) = walk(&c);
            s.explored(&c, fresh);
            leaves.push(leaf);
        }
        leaves.sort();
        assert_eq!(leaves, ["b0", "b1", "b2d0", "b2d1", "c0", "c1"]);
    }
}
