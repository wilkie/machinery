//! Typed, resolved, monomorphised intermediate representation.
//!
//! Every expression carries its [`Ty`]. Names are resolved to [`DefId`]s
//! (program items) or [`LocalId`]s (per-body locals). Const generics have
//! been substituted, so a [`UnitInst`] is a concrete function.
//!
//! Some expressions are **places**: architectural registers and their
//! fields and views, the `mode` register, memory cells of an address space,
//! and the result of an operand kind's `select` rule. A place can be read
//! like any value and assigned with [`StmtKind::Assign`]; [`is_place`] says
//! which expressions qualify.

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

    // --- architectural (isa) ---
    /// An architectural register (place).
    Register(DefId),
    /// A register field by index (place).
    RegField(DefId, u32),
    /// A register view by index (place of the view's bundle type).
    RegView(DefId, u32),
    /// The isa's `mode` register (place).
    Mode(DefId),
    /// Read of instruction operand `idx` through its kind's `read`/`select`.
    OperandRead(u32),
    /// A query rule such as `dst.is_mem` on instruction operand `idx`.
    OperandRule(u32, u32),
    /// A field of a fetched operand value.
    OperandField(Box<Expr>, u32),
    /// The tail of a fetched operand.
    OperandTail(Box<Expr>),
    /// The given operand within a tail (`m.tail.Disp16`).
    TailOperand(Box<Expr>, DefId),
    /// `space.read_uN(addr)`: `bits` wide, in the space's byte order.
    SpaceRead(DefId, u32, Box<Expr>),
    /// `space.write_uN(addr, value)`; type `()`.
    SpaceWrite(DefId, u32, Box<Expr>, Box<Expr>),
    /// `space[addr]`: one cell (place).
    SpaceCell(DefId, Box<Expr>),
    /// `kind.select(args)`: a place chosen by the kind's `select` rule.
    KindSelect(DefId, Vec<Expr>),
    /// Call of an isa `fn`.
    FnCall(DefId, Vec<Expr>),

    // --- component model ---
    /// A wire port of the enclosing component; the flag says whether it is
    /// an output (assignable).
    Port(DefId, u32, bool),
    /// A `state` of the enclosing component's model (place).
    State(DefId, u32),
    /// A `mem` of the enclosing component's model; its elements are places.
    ModelMem(DefId, u32),
    /// `port.transaction(args)` initiated by a model.
    BusCall(u32, DefId, Vec<Expr>),
    Error,
}

/// Can this expression be assigned to?
pub fn is_place(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Register(_)
        | ExprKind::RegField(..)
        | ExprKind::RegView(..)
        | ExprKind::Mode(_)
        | ExprKind::SpaceCell(..)
        | ExprKind::KindSelect(..) => true,
        ExprKind::Port(_, _, out) => *out,
        ExprKind::State(..) => true,
        ExprKind::Index(b, _) => matches!(b.kind, ExprKind::ModelMem(..)),
        ExprKind::Field(b, _) => is_place(b),
        ExprKind::Match(_, arms) => arms.iter().all(|a| match &a.body {
            ArmBody::Expr(e) => is_place(e) || matches!(e.ty, Ty::Never),
            ArmBody::Block(b) => block_is_place(b),
        }),
        ExprKind::If(_, then_b, Some(else_e)) => block_is_place(then_b) && is_place(else_e),
        ExprKind::Block(b) => block_is_place(b),
        _ => false,
    }
}

fn block_is_place(b: &Block) -> bool {
    match b.stmts.last() {
        Some(Stmt {
            kind: StmtKind::Expr(e),
            ..
        }) => is_place(e) || matches!(e.ty, Ty::Never),
        Some(Stmt {
            kind: StmtKind::Unreachable(_) | StmtKind::Raise(..),
            ..
        }) => true,
        _ => false,
    }
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

    // --- architectural (isa) ---
    /// `place = value;`
    Assign(Expr, Expr),
    /// `operand = value;` through the operand kind's `write`/`select`.
    OperandWrite(u32, Expr),
    /// `space.write_uN(addr, value);`
    SpaceWrite(DefId, u32, Expr, Expr),
    /// `raise Fault(args);`
    Raise(DefId, Vec<Expr>),
    /// `given { place = value; ... }` in an isa test.
    Given(Vec<(Expr, Expr)>),
    /// `run instruction Name { operand: value, ... };` values in the
    /// instruction's operand order.
    RunInstruction(DefId, Vec<Expr>),
    /// `run bytes [...];`
    RunBytes(Vec<u8>),

    // --- component model ---
    /// `emit port.signal(args) = value;`
    Emit {
        port: u32,
        signal: DefId,
        args: Vec<Expr>,
        value: Expr,
    },
    While(Expr, Block),
    Loop(Block),
    Break,
    Continue,
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
    /// The isa this test runs against, if any.
    pub isa: Option<DefId>,
    pub locals: Vec<Local>,
    pub body: Block,
}

// ---------------------------------------------------------------------------
// ISA
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub struct Space {
    pub def: DefId,
    pub name: String,
    pub addr: Ty,
    /// Width of one cell in bits.
    pub cell_bits: u32,
    pub little_endian: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RegField {
    pub name: String,
    pub ty: Ty,
    pub offset: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RegView {
    pub name: String,
    /// Always a bundle.
    pub ty: Ty,
    /// For each bundle field (in declaration order), the register field it
    /// aliases.
    pub mapping: Vec<u32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Register {
    pub def: DefId,
    pub name: String,
    pub ty: Ty,
    pub fields: Vec<RegField>,
    pub views: Vec<RegView>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FaultClass {
    /// Restores the instruction pointer and discards the instruction's
    /// architectural writes.
    Fault,
    /// Reported after the instruction commits.
    Trap,
    /// Not restartable.
    Abort,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Fault {
    pub def: DefId,
    pub name: String,
    pub params: Vec<(String, Ty)>,
    pub vector: Option<i128>,
    pub class: FaultClass,
    pub pushes_error_code: bool,
}

/// An isa `fn`: imperative over architectural state.
#[derive(Clone, Debug, PartialEq)]
pub struct FnBody {
    pub def: DefId,
    pub name: String,
    pub locals: Vec<Local>,
    pub params: Vec<LocalId>,
    pub ret: Ty,
    pub body: Block,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OperandField {
    pub name: String,
    pub ty: Ty,
    pub offset: u32,
}

/// Which operand follows, decided by the operand's own fields.
#[derive(Clone, Debug, PartialEq)]
pub enum TailExpr {
    Operand(DefId),
    If(Expr, Box<TailExpr>, Option<Box<TailExpr>>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Operand {
    pub def: DefId,
    pub name: String,
    /// Size in bytes.
    pub size: u64,
    pub fields: Vec<OperandField>,
    /// Locals used by the tail expression (the operand's fields).
    pub tail_locals: Vec<Local>,
    pub tail: Option<TailExpr>,
    /// Every operand that can appear in the tail.
    pub tail_set: Vec<DefId>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuleKind {
    /// `select(n) => place`: the operand is a register chosen by an
    /// encoding field.
    Select,
    /// `read => value`
    Read,
    /// `write(v) => ...`
    Write,
    /// `is_mem => bool`
    IsMem,
}

#[derive(Clone, Debug, PartialEq)]
pub struct KindRule {
    pub name: String,
    pub kind: RuleKind,
    pub locals: Vec<Local>,
    /// The `from` source operand local (if any) followed by the rule's own
    /// parameters.
    pub params: Vec<LocalId>,
    pub body: ArmBody,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OperandKind {
    pub def: DefId,
    pub name: String,
    /// The value type of operands of this kind.
    pub ty: Ty,
    /// `from name: Operand`
    pub source: Option<DefId>,
    pub rules: Vec<KindRule>,
}

/// How an instruction operand is obtained from the encoding.
#[derive(Clone, Debug, PartialEq)]
pub enum BindSource {
    /// `kind.select(field)` of the fetched operand at `step`.
    Select { step: u32, field: u32 },
    /// The whole fetched operand at `step` (a `from` kind).
    From { step: u32 },
}

#[derive(Clone, Debug, PartialEq)]
pub struct OperandBinding {
    pub name: String,
    pub kind: DefId,
    pub source: BindSource,
}

#[derive(Clone, Debug, PartialEq)]
pub enum EncodingStep {
    Opcode(Vec<u8>),
    /// Fetch an operand; `selects` maps its fields to operand bindings.
    Fetch {
        operand: DefId,
        selects: Vec<(u32, u32)>,
    },
}

/// A register field reference `(register, field index)`.
pub type FlagRef = (DefId, u32);

#[derive(Clone, Debug, Default, PartialEq)]
pub struct FlagSets {
    pub modifies: Vec<FlagRef>,
    pub undefined: Vec<FlagRef>,
    pub reads: Vec<FlagRef>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Instruction {
    pub def: DefId,
    pub name: String,
    pub mnemonic: Option<String>,
    pub description: Vec<String>,
    pub encoding: Vec<EncodingStep>,
    pub operands: Vec<OperandBinding>,
    pub flags: FlagSets,
    pub locals: Vec<Local>,
    pub semantics: Option<Block>,
    pub timing_locals: Vec<Local>,
    pub timing: Option<Expr>,
    pub implements: Option<String>,
    pub references: Vec<String>,
    /// Variant indices of the isa's mode enum.
    pub allowed_in: Vec<u32>,
    pub requires: Option<Expr>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IsaInfo {
    pub def: DefId,
    pub name: String,
    pub spaces: Vec<Space>,
    pub registers: Vec<Register>,
    /// The `mode` enum item, if declared.
    pub mode: Option<DefId>,
    pub faults: Vec<Fault>,
    pub fns: Vec<FnBody>,
    pub operands: Vec<Operand>,
    pub kinds: Vec<OperandKind>,
    pub instructions: Vec<Instruction>,
}

// ---------------------------------------------------------------------------
// Buses and components
// ---------------------------------------------------------------------------

/// Who may drive a bus wire.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WireDriver {
    /// Exactly one role drives it (by index into the bus's roles).
    Role(u32),
    /// Tristate: one driver at a time, chosen by the protocol.
    Shared,
    /// Open-collector, idles high; any role may pull it low.
    WiredAnd,
    /// Open-emitter, idles low; any role may pull it high.
    WiredOr,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BusWire {
    pub name: String,
    pub ty: Ty,
    pub driver: WireDriver,
    pub active_low: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TransactionSig {
    pub def: DefId,
    pub name: String,
    pub params: Vec<(String, Ty)>,
    /// A named tuple, or `None` for a transaction that returns nothing.
    pub ret: Option<Ty>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SignalInfo {
    pub def: DefId,
    pub name: String,
    pub params: Vec<(String, Ty)>,
    /// Wires first (one local per bus wire, in order), then the parameters.
    pub locals: Vec<Local>,
    pub param_ids: Vec<LocalId>,
    /// The wire (or wire-array element) the signal is, as a `bool`.
    pub expr: Expr,
    /// The driver of the underlying wire.
    pub driver: WireDriver,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BusInfo {
    pub def: DefId,
    pub name: String,
    pub clock: Option<String>,
    /// The first role initiates transactions; the others respond.
    pub roles: Vec<String>,
    pub wires: Vec<BusWire>,
    pub transactions: Vec<TransactionSig>,
    pub signals: Vec<SignalInfo>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PortKind {
    In(Ty),
    Out(Ty),
    AsyncIn(Ty),
    Bus { bus: DefId, role: u32 },
    Clock,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Port {
    pub name: String,
    pub kind: PortKind,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ModelState {
    pub name: String,
    pub ty: Ty,
    pub init: Option<Expr>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ModelMem {
    pub name: String,
    pub elem: Ty,
    pub len: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum HandlerKind {
    Reset,
    /// `on tick`; `clock` is the clock port, or `None` for a component's
    /// implicit single clock.
    Tick {
        clock: Option<u32>,
    },
    /// A responding role's handler for a transaction on a bus port.
    Transaction {
        port: u32,
        transaction: DefId,
    },
    /// An observing role's handler for a signal change on a bus port.
    Signal {
        port: u32,
        signal: DefId,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Handler {
    pub kind: HandlerKind,
    pub locals: Vec<Local>,
    pub params: Vec<LocalId>,
    pub ret: Ty,
    pub body: Block,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ModelBody {
    /// Locals used by state initialisers.
    pub init_locals: Vec<Local>,
    pub states: Vec<ModelState>,
    pub mems: Vec<ModelMem>,
    pub handlers: Vec<Handler>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ComponentInfo {
    pub def: DefId,
    pub name: String,
    pub isa: Option<DefId>,
    pub ports: Vec<Port>,
    pub model: Option<ModelBody>,
    /// The rtl body exists but is not checked yet.
    pub has_rtl: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Hir {
    pub units: Vec<UnitInst>,
    pub tests: Vec<TestBody>,
    pub isas: Vec<IsaInfo>,
    pub buses: Vec<BusInfo>,
    pub components: Vec<ComponentInfo>,
}
