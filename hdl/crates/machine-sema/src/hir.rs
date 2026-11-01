//! Typed, resolved, monomorphised intermediate representation.
//!
//! Every expression carries its [`Ty`]. Names are resolved to [`DefId`]s
//! (program items) or [`LocalId`]s (per-body locals). Const generics have
//! been substituted, so a [`UnitInst`] is a concrete function.

use machine_syntax::ast::{BinaryOp, UnaryOp};
use machine_syntax::span::Span;

use crate::builtins::Builtin;
use crate::program::DefId;
use crate::ty::Ty;

pub type LocalId = u32;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct UnitInstId(pub u32);

#[derive(Clone, Debug, PartialEq)]
pub struct Local {
    pub name: String,
    pub ty: Ty,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Expr {
    pub kind: ExprKind,
    pub ty: Ty,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ExprKind {
    Int(i128),
    Bool(bool),
    Str(String),
    Local(LocalId),
    Const(DefId),
    /// Enum variant by index into the enum's variant list.
    Variant(DefId, u32),
    Unary(UnaryOp, Box<Expr>),
    Binary(BinaryOp, Box<Expr>, Box<Expr>),
    Ternary(Box<Expr>, Box<Expr>, Box<Expr>),
    Cast(Box<Expr>),
    /// Bundle field by index.
    Field(Box<Expr>, u32),
    /// Tuple element by index.
    TupleField(Box<Expr>, u32),
    Index(Box<Expr>, Box<Expr>),
    /// `a[hi:lo]` with constant bounds.
    Slice(Box<Expr>, u32, u32),
    Call(UnitInstId, Vec<Expr>),
    Builtin(Builtin, Vec<Expr>),
    /// Fields in declaration order.
    BundleLit(DefId, Vec<Expr>),
    Tuple(Vec<Expr>),
    Array(Vec<Expr>),
    ArrayRepeat(Box<Expr>, u64),
    Match(Box<Expr>, Vec<Arm>),
    If(Box<Expr>, Block, Option<Box<Expr>>),
    Block(Block),
    Error,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Pat {
    Wildcard,
    /// Integer or bool-as-integer value.
    Value(i128),
    /// Inclusive range.
    Range(i128, i128),
    Variant(u32),
    Bool(bool),
}

#[derive(Clone, Debug, PartialEq)]
pub enum ArmBody {
    Expr(Expr),
    Block(Block),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Arm {
    pub patterns: Vec<Pat>,
    pub body: ArmBody,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub span: Span,
}

/// Destructuring target of a `let`.
#[derive(Clone, Debug, PartialEq)]
pub enum LetTarget {
    Local(LocalId),
    Tuple(Vec<LetTarget>),
    Discard,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum StmtKind {
    Let(LetTarget, Expr),
    Expr(Expr),
    If(Expr, Block, Option<Box<Stmt>>),
    Match(Expr, Vec<Arm>),
    /// `for local in lo..hi` with constant bounds (exclusive `hi`).
    For(LocalId, i128, i128, Block),
    Return(Option<Expr>),
    Assert(Expr, Option<String>),
    Unreachable(Option<String>),
    /// `expect { e; e; }` in a test: each expression is a `bool`.
    Expect(Vec<Expr>),
    Error,
}

/// A monomorphised unit.
#[derive(Clone, Debug, PartialEq)]
pub struct UnitInst {
    pub def: DefId,
    /// Const generic arguments in declaration order.
    pub args: Vec<i128>,
    pub locals: Vec<Local>,
    /// Locals holding the parameters, in order.
    pub params: Vec<LocalId>,
    pub ret: Ty,
    pub body: Block,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TestBody {
    pub def: DefId,
    pub name: String,
    pub locals: Vec<Local>,
    pub body: Block,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Hir {
    pub units: Vec<UnitInst>,
    pub tests: Vec<TestBody>,
}
