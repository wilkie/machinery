//! `machine-syntax`: the front end of the Machine hardware description
//! language.
//!
//! ```text
//! source text ──lex──▶ tokens ──parse──▶ ast::File + Diagnostics
//! ```
//!
//! The language is specified in `hdl/SPEC.md`. This crate knows nothing about
//! meaning: names are unresolved, widths are expressions, and every construct
//! the grammar admits is accepted whether or not it is legal in context.
//! Semantic analysis lives in `machine-sema`.

pub mod ast;
pub mod diagnostics;
pub mod lexer;
pub mod parser;
pub mod span;
pub mod token;

pub use diagnostics::{Diagnostic, Diagnostics, Severity};
pub use parser::{parse, parse_expr_str, Parsed};
pub use span::{SourceFile, Span};
