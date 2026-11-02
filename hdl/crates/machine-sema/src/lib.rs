//! `machine-sema`: semantic analysis for Machine.
//!
//! ```text
//! ast::File (per module) ──collect──▶ Program (items, scopes)
//!                         ──check────▶ hir (typed, resolved, monomorphised) + Diagnostics
//! ```
//!
//! What is checked today: `enum`, `bundle`, `union`, `const`, `unit` (with
//! const generics) and target-less `test` blocks. Declarations of other
//! kinds (`component`, `bus`, `isa`, `system`, targeted tests, `vectors`,
//! `conformance`) are collected into the program's scopes so names resolve,
//! but their bodies are not yet checked; [`Checked::skipped`] lists them.
//!
//! Diagnostic codes in this crate are `E1xxx` / `W1xxx`.

pub mod builtins;
pub mod check;
pub mod consteval;
pub mod hir;
pub mod program;
pub mod ty;

use machine_syntax::diagnostics::Diagnostics;
use machine_syntax::SourceFile;

pub use program::{DefId, ModuleId, Program};
pub use ty::Ty;

/// A body the checker recognised but does not analyse yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skipped {
    pub module: ModuleId,
    pub kind: &'static str,
    pub name: String,
}

pub struct Checked {
    pub program: Program,
    pub hir: hir::Hir,
    /// Diagnostics per module, in module order.
    pub diagnostics: Vec<Diagnostics>,
    pub skipped: Vec<Skipped>,
}

impl Checked {
    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(|d| d.has_errors())
    }
}

/// Parse and check a set of source files that form one program. Files refer
/// to each other through `use <stem>::{...}`.
pub fn check_files(mut files: Vec<SourceFile>) -> Checked {
    files.push(SourceFile::new(program::PRELUDE_NAME, program::PRELUDE));
    let (program, mut diagnostics) = Program::collect(files);
    let (hir, skipped) = check::check_program(&program, &mut diagnostics);
    Checked {
        program,
        hir,
        diagnostics,
        skipped,
    }
}
