//! `repeat ... bulk`: the check, when the description is built, that an
//! instruction's element (its semantics, once) is a stream the cores may
//! run as a block; and, for a fill, copy, scan, compare or load, the kernel
//! that does (SPEC 11.2).
//!
//! The element is evaluated abstractly (`eval`), every fn and unit it
//! calls inlined. A value (`val`) is a constant; an invariant (the same
//! every element: latches, operands, state the element never writes); a
//! register as the element found it plus an invariant (a pointer that moves
//! the same way each time); or what the element loaded (and anything
//! computed from that and invariants alone).
//!
//! A branch on an invariant splits the evaluation (`plan`'s search): each
//! path is a case the runtime would tell apart (real or protected mode,
//! paging, the address size). A condition is keyed by what it tests, so
//! `CR0.PG == 0` gets the same answer everywhere in a path. A branch on an
//! element value where one arm always raises is a fault check (a segment
//! limit): the path takes the other. Any other branch on an element value
//! ends the path: not a stream.
//!
//! A path is a stream when its pointers move by invariants, nothing else
//! but its results changes, and its accesses are one of: a fill (one write
//! of an invariant), a copy (a read, and a write of it), a scan (one read,
//! results from it), a compare (two reads), a load (one read into a
//! register), a port-in (a port read written to memory) or a port-out (a
//! memory read written to a port). `bulk` claims at least one path is.
//!
//! The kernel (`emit`; the loop is `rt::bulk::run`) runs the element's own
//! code dry (its accesses noted, not made) at a few elements: how the
//! registers move, where it reads and writes, whether it faults. The paths
//! are told apart where the code tells them apart, by running it; the dry
//! code stops where a path stops being one the kernel can take (a branch on
//! where the element is that is not a fault check holding on an interval of
//! elements; anything not a stream): at the expression that first made the
//! value branched on, which the paths it takes do not evaluate. So every
//! path the dry code finishes is one the kernel takes (its fault checks,
//! holding at the first and the last element, hold between), or it raised.
//! When some path is neither, the instruction has no kernel.
//!
//! A scan's or compare's `while` condition (REPE, REPNE) depends on what
//! the element loaded: the kernel tests it on each element's loaded values
//! as its closed form (the condition with the element's results in place,
//! a branch on an invariant kept a branch), and stops before the element
//! that ends the repeat, which the step runs. The results (the flags) are
//! each element's own, from what it loaded, so that element's are the
//! instruction's.

mod emit;
mod eval;
mod plan;
mod val;

use machine_sema::hir::{Expr, Stmt};
use machine_sema::program::DefId;
use machine_syntax::span::Span;

use crate::gen::{Gen, R};

/// A statement or expression of the description (where the dry code may
/// stop), and where it is.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Site {
    at: usize,
    pub span: Span,
}

impl PartialEq for Site {
    fn eq(&self, o: &Site) -> bool {
        self.at == o.at
    }
}
impl Eq for Site {}
impl std::hash::Hash for Site {
    fn hash<H: std::hash::Hasher>(&self, h: &mut H) {
        self.at.hash(h)
    }
}

impl Site {
    pub(crate) const NONE: Site = Site { at: 0, span: Span::DUMMY };
    pub(crate) fn stmt(s: &Stmt) -> Site {
        Site { at: s as *const Stmt as usize, span: s.span }
    }
    pub(crate) fn expr(e: &Expr) -> Site {
        Site { at: e as *const Expr as usize, span: e.span }
    }
    pub(crate) fn is_none(&self) -> bool {
        self.at == 0
    }
}

/// A stream's kind (`bulk`'s claim).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Shape {
    Fill,
    Copy,
    Scan,
    Compare,
    Load,
    PortIn,
    PortOut,
}

/// An instruction's kernel: what its paths tell the kernel loop.
#[derive(Clone, Debug)]
pub(crate) struct Kernel {
    shape: Shape,
    /// The bits of each access.
    bits: u32,
    /// The registers that move: each whole register, and the width (from
    /// bit 0) it moves in on some path; the count last (counted down by
    /// one, the instruction repeating while it is not zero).
    moving: Vec<(DefId, u32)>,
    /// The registers the element may write, on any path, before it ends
    /// (kept round the dry code).
    written: Vec<DefId>,
    /// The fns the element may call (the dry code has its own of each).
    fns: Vec<DefId>,
    /// Where its dry code stops.
    stops: Vec<Site>,
    /// The `while` condition's closed form (Rust, over `l<n>_ld<n+1>`, the
    /// values loaded), and those values' types.
    cond: Option<String>,
    loads: Vec<String>,
}

impl Kernel {
    pub(crate) fn fns(&self) -> &[DefId] {
        &self.fns
    }
    pub(crate) fn stops(&self) -> &[Site] {
        &self.stops
    }
}

/// What the generator makes of an instruction's `bulk` (for tests and
/// `MACHINE_BULK`).
#[derive(Clone, Debug)]
pub struct BulkReport {
    pub instruction: String,
    /// The stream's kind (`Fill`, `Copy`, ...).
    pub shape: String,
    /// Its kernel (what moves, its `while` condition), or why it has none.
    pub kernel: Result<String, String>,
    /// Where the kernel's dry code stops: spans in the source.
    pub stops: Vec<Span>,
}

impl Gen<'_> {
    /// Every `bulk` instruction's plan.
    pub(crate) fn bulk_report(&self) -> R<Vec<BulkReport>> {
        let mut out = Vec::new();
        for i in &self.isa.instructions {
            if !i.repeat.as_ref().is_some_and(|r| r.bulk) {
                continue;
            }
            let (shape, k) = self.bulk_plan(i)?;
            out.push(BulkReport {
                instruction: i.name.clone(),
                shape: format!("{shape:?}"),
                stops: k.as_ref().map(|k| k.stops.iter().map(|s| s.span).collect()).unwrap_or_default(),
                kernel: k.map(|k| format!("{:?} of {} bits, moving {}, while {}", k.shape, k.bits, k.moving.len() - 1, k.cond.as_deref().unwrap_or("-"))),
            });
        }
        Ok(out)
    }
}
