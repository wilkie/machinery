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
    /// Declared `let mut`: assignable.
    pub mutable: bool,
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
    /// A `latch` of the isa (place).
    Latch(DefId),

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
    /// `port.signal(args)` read as a value: the signal's current level.
    SignalLevel(u32, DefId, Vec<Expr>),
    /// Call of a model fn of the enclosing component, by index into
    /// [`ModelBody::fns`].
    ModelFnCall(u32, Vec<Expr>),
    Error,
}

/// Can this expression be assigned to?
pub fn is_place(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Register(_)
        | ExprKind::RegField(..)
        | ExprKind::RegView(..)
        | ExprKind::Mode(_)
        | ExprKind::Latch(_)
        | ExprKind::SpaceCell(..)
        | ExprKind::KindSelect(..) => true,
        ExprKind::Port(_, _, out) => *out,
        ExprKind::State(..) => true,
        // An element of a `mem`, or of any array-typed place.
        ExprKind::Index(b, _) => {
            matches!(b.kind, ExprKind::ModelMem(..))
                || (matches!(b.ty, Ty::Array(..)) && is_place(b))
        }
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
    /// `None` for `let mut x: T;` (assigned later, before any read).
    Let(LetTarget, Option<Expr>),
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
    /// `halt;`
    Halt,
    /// `decline;` in a transaction handler.
    Decline,

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
    /// A fault raised while delivering another contributory one is a
    /// double fault (`istream { double ...; }`).
    pub contributory: bool,
    /// The 386's page fault: a page fault or a contributory fault raised
    /// while delivering it is a double fault (a page fault raised while
    /// delivering a contributory one is not).
    pub page_fault: bool,
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
    /// An `extern fn`: no body; the processor model supplies it.
    pub external: bool,
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
    /// Extra decoder clocks on this operand's first byte (`decode <Name>
    /// => n;` in a timing table, SPEC 11.2c).
    pub decode: u8,
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
    /// `value(n) => expr`: the operand is a value carried by an encoding
    /// field (an ALU operation, a condition code).
    Value,
    /// `name => expr`: a pure query over the fetched operand (`disp8`).
    Query,
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
    /// What the rule yields.
    pub ty: Ty,
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
    /// An encoding field fed to the kind's `select` or `value` rule: field
    /// `field` of the fetched operand at `step`, or of the opcode pattern
    /// at `step`.
    Field { step: u32, field: u32 },
    /// The whole fetched operand at `step` (a `from` kind).
    From { step: u32 },
}

/// One opcode byte: the bits under `mask` must equal `value`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OpcodeByte {
    pub value: u8,
    pub mask: u8,
}

/// A named run of bits in an opcode pattern.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpcodeField {
    pub name: String,
    /// Index of the byte within this opcode step.
    pub byte: u32,
    pub shift: u32,
    pub width: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OperandBinding {
    pub name: String,
    pub kind: DefId,
    pub source: BindSource,
}

#[derive(Clone, Debug, PartialEq)]
pub enum EncodingStep {
    /// Fixed and pattern bytes; fields bound to operands.
    Opcode {
        bytes: Vec<OpcodeByte>,
        fields: Vec<OpcodeField>,
    },
    /// Fetch an operand. `constraints` are `(field, value)` pairs the
    /// fetched operand must match for this instruction to apply (group
    /// opcodes such as `C6 /0`).
    Fetch {
        operand: DefId,
        constraints: Vec<(u32, i128)>,
    },
}

impl EncodingStep {
    pub fn is_opcode(&self) -> bool {
        matches!(self, EncodingStep::Opcode { .. })
    }
}

/// A flag (register field), optionally only when a condition over the
/// instruction's operands holds.
#[derive(Clone, Debug, PartialEq)]
pub struct FlagUse {
    pub reg: DefId,
    pub field: u32,
    pub cond: Option<Expr>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct FlagSets {
    pub modifies: Vec<FlagUse>,
    pub undefined: Vec<FlagUse>,
    pub reads: Vec<FlagUse>,
    /// Locals used by the conditions.
    pub locals: Vec<Local>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Mnemonic {
    Fixed(String),
    /// A string-valued expression over the operands.
    Expr {
        locals: Vec<Local>,
        expr: Expr,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Instruction {
    pub def: DefId,
    pub name: String,
    /// A `prefix`: its semantics run during decoding and decoding continues.
    pub prefix: bool,
    pub mnemonic: Option<Mnemonic>,
    pub description: Vec<String>,
    pub encoding: Vec<EncodingStep>,
    pub operands: Vec<OperandBinding>,
    pub flags: FlagSets,
    pub locals: Vec<Local>,
    pub semantics: Option<Block>,
    pub timing_locals: Vec<Local>,
    pub timing: Option<Expr>,
    /// Its `schedules` table entry (SPEC 11.2c): a `Schedule`, or `u32`
    /// cycles meaning `schedule(n)`.
    pub schedule_locals: Vec<Local>,
    pub schedule: Option<Expr>,
    /// `when <cond>;` in the encoding (SPEC 11.2d): a decode condition over
    /// latches, registers and pure fns, checked after the bytes match.
    pub when_locals: Vec<Local>,
    pub when: Option<Expr>,
    /// `lock` / `lock if cond`: when the instruction may follow a lock
    /// prefix (`None`: never).
    pub lock_locals: Vec<Local>,
    pub lock: Option<Expr>,
    /// `fault <Name> =>` in a `schedules` table: delivering a fault this
    /// instruction raises, with `vector` (its first param) in scope.
    pub fault_schedule: Option<TimingExpr>,
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
    pub istream: Option<Istream>,
    pub latches: Vec<Latch>,
    /// Cycles to deliver a fault, and to take an interrupt at an
    /// instruction boundary (`fault =>`, `interrupt =>` in a timing table).
    pub fault_cycles: Option<TimingExpr>,
    pub interrupt_cycles: Option<TimingExpr>,
    /// `interrupt =>` and `fault =>` (with `vector`) in a `schedules` table.
    pub interrupt_schedule: Option<TimingExpr>,
    pub fault_schedule: Option<TimingExpr>,
}

/// A timing expression with its locals.
#[derive(Clone, Debug, PartialEq)]
pub struct TimingExpr {
    pub locals: Vec<Local>,
    /// `vector: u8` for `fault`.
    pub params: Vec<LocalId>,
    pub expr: Expr,
}

/// Per-instruction state, reset to `init` at every instruction boundary.
#[derive(Clone, Debug, PartialEq)]
pub struct Latch {
    pub def: DefId,
    pub name: String,
    pub ty: Ty,
    pub init: Expr,
}

/// How one register of a vector format maps onto the isa.
#[derive(Clone, Debug, PartialEq)]
pub struct VectorReg {
    /// The value's width: 16 for the 286 file, 32 for the 386's.
    pub bits: u32,
    pub name: String,
    pub locals: Vec<Local>,
    /// The local holding the vector's value during `load`.
    pub value: LocalId,
    pub load: Block,
    pub read: Expr,
    /// The register read by `read`, if it is a whole register: its
    /// instruction-undefined flag fields are masked when comparing.
    pub masked_reg: Option<DefId>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VectorsInfo {
    pub def: DefId,
    pub isa: DefId,
    /// Glob resolved against the declaring file's directory.
    pub glob: String,
    pub format: String,
    pub memory: DefId,
    /// Spaces with nothing attached in the test environment: `(space,
    /// value every read returns)`.
    pub open: Vec<(DefId, i128)>,
    pub regs: Vec<VectorReg>,
    /// Cycles of each trace that belong to the capture rig (`harness_cycles`).
    pub harness_cycles: u32,
}

/// Where instruction bytes come from.
#[derive(Clone, Debug, PartialEq)]
pub struct Istream {
    pub space: DefId,
    /// `fn () -> <space address type>`
    pub address: DefId,
    /// `fn () -> <space cell type>`
    pub next: DefId,
    /// The fault raised for an undecodable opcode.
    pub undefined: Option<DefId>,
    /// `fn (vector: u8)` that delivers a raised fault, or `None` to leave
    /// faults to the caller of `step`.
    pub fault: Option<DefId>,
    /// `fn () -> bool` run at each instruction boundary; `true` means it
    /// took an interrupt in place of the next instruction.
    pub boundary: Option<DefId>,
    /// `fn () -> u32`: bytes the stream holds from the current address
    /// before its segment ends (a processor model stops prefetching there).
    pub limit: Option<DefId>,
    /// The fault delivered when a contributory fault is raised while one is
    /// delivered; a fault while delivering it shuts the processor down.
    pub double: Option<DefId>,
    /// The latch a lock prefix sets (`istream { lock locked; }`).
    pub lock: Option<DefId>,
}

/// A resolved `enum`/`mode`, exported for backends.
#[derive(Clone, Debug, PartialEq)]
pub struct EnumDef {
    pub def: DefId,
    pub name: String,
    pub width: u32,
    pub variants: Vec<(String, i128)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BundleFieldDef {
    pub name: String,
    pub ty: Ty,
    pub offset: Option<u32>,
}

/// A resolved `bundle`, exported for backends.
#[derive(Clone, Debug, PartialEq)]
pub struct BundleDef {
    pub def: DefId,
    pub name: String,
    pub fields: Vec<BundleFieldDef>,
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
    /// Bus cycles without wait states, counted from the initiating role's
    /// `cycle`s; `None` when the body is absent or not straight-line.
    pub base_cycles: Option<u32>,
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
    /// A processor's binding of an isa space read of `bits` to its buses.
    SpaceRead {
        space: DefId,
        bits: u32,
    },
    /// A processor's binding of an isa space write of `bits`.
    SpaceWrite {
        space: DefId,
        bits: u32,
    },
    /// An instruction-stream fetch from the istream's space (a code bus
    /// cycle); without one, fetches use the read handler.
    SpaceFetch {
        space: DefId,
        bits: u32,
    },
    /// A processor's implementation of an isa `extern fn`.
    Extern {
        def: DefId,
    },
    /// The level driven into an input port changed.
    Input {
        port: u32,
    },
    /// `on retire(s: Schedule) -> u32`: a processor model's instruction
    /// timing. Called after each instruction with its schedule; returns the
    /// clocks it took (SPEC 11.2c).
    Retire,
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

/// A `fn` in a model body (SPEC 8.3a).
#[derive(Clone, Debug, PartialEq)]
pub struct ModelFn {
    pub name: String,
    pub locals: Vec<Local>,
    pub params: Vec<LocalId>,
    pub ret: Ty,
    pub body: Block,
    pub span: Span,
    /// It, or a fn it calls, starts a bus transaction.
    pub starts_transaction: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ModelBody {
    /// Locals used by state initialisers.
    pub init_locals: Vec<Local>,
    pub states: Vec<ModelState>,
    pub mems: Vec<ModelMem>,
    pub handlers: Vec<Handler>,
    pub fns: Vec<ModelFn>,
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

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// A clock of a system, with its exact frequency.
#[derive(Clone, Debug, PartialEq)]
pub struct SystemClock {
    /// The declared name, or the expression text for a clock written
    /// inline in a binding (`osc / 12`).
    pub name: String,
    /// Frequency in Hz as a reduced fraction `hz_num / hz_den`.
    pub hz_num: u128,
    pub hz_den: u128,
    /// The clock period in the system's base time unit (an integer).
    pub period: u64,
}

/// A component instance and the system clock of each of its clocks.
#[derive(Clone, Debug, PartialEq)]
pub struct SystemComponent {
    pub name: String,
    pub component: DefId,
    /// `(clock port index, system clock index)`; the port is `None` for a
    /// component's implicit single clock.
    pub clocks: Vec<(Option<u32>, u32)>,
}

/// One address range a slave answers, inclusive.
#[derive(Clone, Debug, PartialEq)]
pub struct AddressRange {
    /// The value of the transactions' `space` parameter it applies to, or
    /// `None` for every space.
    pub space: Option<i128>,
    pub lo: u128,
    pub hi: u128,
}

/// A bus port of an instance: `(instance index, port index)`.
pub type InstPort = (u32, u32);

#[derive(Clone, Debug, PartialEq)]
pub struct SystemBus {
    pub name: String,
    pub bus: DefId,
    pub clock: u32,
    /// The ports in the initiating role, in binding order.
    pub masters: Vec<InstPort>,
    /// Responding ports and the ranges each answers.
    pub slaves: Vec<(InstPort, Vec<AddressRange>)>,
    /// `bridge` targets (not generated yet).
    pub bridges: Vec<String>,
}

/// What drives a wire into an input port.
#[derive(Clone, Debug, PartialEq)]
pub enum WireSource {
    /// An output port of an instance.
    Port(InstPort),
    /// A constant strap (`wire pic1.sp = true;`).
    Const(i128),
}

/// `wire target = source;`: an input port and what drives it.
#[derive(Clone, Debug, PartialEq)]
pub struct SystemWire {
    pub target: InstPort,
    pub source: WireSource,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    Model,
    Rtl,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Profile {
    pub name: String,
    pub default: Level,
    /// Instance paths (`vga`, `vga.crtc`) and their level.
    pub overrides: Vec<(String, Level)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SystemInfo {
    pub def: DefId,
    pub name: String,
    /// The base time unit in seconds, `base_num / base_den`: the largest
    /// unit in which every clock period is an integer.
    pub base_num: u128,
    pub base_den: u128,
    pub clocks: Vec<SystemClock>,
    pub components: Vec<SystemComponent>,
    pub buses: Vec<SystemBus>,
    /// `wire` items: what drives each connected input port.
    pub wires: Vec<SystemWire>,
    pub profiles: Vec<Profile>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Hir {
    pub units: Vec<UnitInst>,
    pub tests: Vec<TestBody>,
    pub isas: Vec<IsaInfo>,
    pub buses: Vec<BusInfo>,
    pub components: Vec<ComponentInfo>,
    /// Every resolved enum and bundle, by definition order.
    pub enums: Vec<EnumDef>,
    pub bundles: Vec<BundleDef>,
    pub vectors: Vec<VectorsInfo>,
    pub systems: Vec<SystemInfo>,
}

// ---------------------------------------------------------------------------
// Traversal
// ---------------------------------------------------------------------------

/// Visit every statement and expression of a block, outermost first.
pub fn visit_block(b: &Block, on_stmt: &mut dyn FnMut(&Stmt), on_expr: &mut dyn FnMut(&Expr)) {
    for s in &b.stmts {
        visit_stmt(s, on_stmt, on_expr);
    }
}

pub fn visit_stmt(s: &Stmt, on_stmt: &mut dyn FnMut(&Stmt), on_expr: &mut dyn FnMut(&Expr)) {
    on_stmt(s);
    let e = |x: &Expr, on_stmt: &mut dyn FnMut(&Stmt), on_expr: &mut dyn FnMut(&Expr)| {
        visit_expr(x, on_stmt, on_expr)
    };
    match &s.kind {
        StmtKind::Let(_, v) => {
            if let Some(v) = v {
                e(v, on_stmt, on_expr);
            }
        }
        StmtKind::Expr(x) | StmtKind::OperandWrite(_, x) => e(x, on_stmt, on_expr),
        StmtKind::If(c, t, el) => {
            e(c, on_stmt, on_expr);
            visit_block(t, on_stmt, on_expr);
            if let Some(el) = el {
                visit_stmt(el, on_stmt, on_expr);
            }
        }
        StmtKind::Match(x, arms) => {
            e(x, on_stmt, on_expr);
            for a in arms {
                visit_arm(a, on_stmt, on_expr);
            }
        }
        StmtKind::For(_, _, _, b) | StmtKind::Loop(b) => visit_block(b, on_stmt, on_expr),
        StmtKind::While(c, b) => {
            e(c, on_stmt, on_expr);
            visit_block(b, on_stmt, on_expr);
        }
        StmtKind::Return(v) => {
            if let Some(v) = v {
                e(v, on_stmt, on_expr);
            }
        }
        StmtKind::Assert(c, _) => e(c, on_stmt, on_expr),
        StmtKind::Expect(xs) | StmtKind::Raise(_, xs) | StmtKind::RunInstruction(_, xs) => {
            for x in xs {
                e(x, on_stmt, on_expr);
            }
        }
        StmtKind::Assign(a, b) | StmtKind::SpaceWrite(_, _, a, b) => {
            e(a, on_stmt, on_expr);
            e(b, on_stmt, on_expr);
        }
        StmtKind::Given(pairs) => {
            for (a, b) in pairs {
                e(a, on_stmt, on_expr);
                e(b, on_stmt, on_expr);
            }
        }
        StmtKind::Emit { args, value, .. } => {
            for x in args {
                e(x, on_stmt, on_expr);
            }
            e(value, on_stmt, on_expr);
        }
        StmtKind::Unreachable(_)
        | StmtKind::RunBytes(_)
        | StmtKind::Halt
        | StmtKind::Decline
        | StmtKind::Break
        | StmtKind::Continue
        | StmtKind::Error => {}
    }
}

fn visit_arm(a: &Arm, on_stmt: &mut dyn FnMut(&Stmt), on_expr: &mut dyn FnMut(&Expr)) {
    match &a.body {
        ArmBody::Expr(x) => visit_expr(x, on_stmt, on_expr),
        ArmBody::Block(b) => visit_block(b, on_stmt, on_expr),
    }
}

pub fn visit_expr(x: &Expr, on_stmt: &mut dyn FnMut(&Stmt), on_expr: &mut dyn FnMut(&Expr)) {
    on_expr(x);
    let all = |xs: &[Expr], on_stmt: &mut dyn FnMut(&Stmt), on_expr: &mut dyn FnMut(&Expr)| {
        for c in xs {
            visit_expr(c, on_stmt, on_expr);
        }
    };
    match &x.kind {
        ExprKind::Unary(_, a)
        | ExprKind::Cast(a)
        | ExprKind::Field(a, _)
        | ExprKind::TupleField(a, _)
        | ExprKind::Slice(a, _, _)
        | ExprKind::ArrayRepeat(a, _)
        | ExprKind::OperandField(a, _)
        | ExprKind::OperandTail(a)
        | ExprKind::TailOperand(a, _)
        | ExprKind::SpaceRead(_, _, a)
        | ExprKind::SpaceCell(_, a) => visit_expr(a, on_stmt, on_expr),
        ExprKind::Binary(_, a, b) | ExprKind::Index(a, b) | ExprKind::SpaceWrite(_, _, a, b) => {
            visit_expr(a, on_stmt, on_expr);
            visit_expr(b, on_stmt, on_expr);
        }
        ExprKind::Ternary(a, b, c) => {
            visit_expr(a, on_stmt, on_expr);
            visit_expr(b, on_stmt, on_expr);
            visit_expr(c, on_stmt, on_expr);
        }
        ExprKind::Call(_, xs)
        | ExprKind::Builtin(_, xs)
        | ExprKind::BundleLit(_, xs)
        | ExprKind::Tuple(xs)
        | ExprKind::Array(xs)
        | ExprKind::KindSelect(_, xs)
        | ExprKind::FnCall(_, xs)
        | ExprKind::BusCall(_, _, xs)
        | ExprKind::SignalLevel(_, _, xs)
        | ExprKind::ModelFnCall(_, xs) => all(xs, on_stmt, on_expr),
        ExprKind::Match(s, arms) => {
            visit_expr(s, on_stmt, on_expr);
            for a in arms {
                visit_arm(a, on_stmt, on_expr);
            }
        }
        ExprKind::If(c, t, el) => {
            visit_expr(c, on_stmt, on_expr);
            visit_block(t, on_stmt, on_expr);
            if let Some(el) = el {
                visit_expr(el, on_stmt, on_expr);
            }
        }
        ExprKind::Block(b) => visit_block(b, on_stmt, on_expr),
        ExprKind::Int(_)
        | ExprKind::Bool(_)
        | ExprKind::Str(_)
        | ExprKind::Local(_)
        | ExprKind::Const(_)
        | ExprKind::Variant(..)
        | ExprKind::Register(_)
        | ExprKind::RegField(..)
        | ExprKind::RegView(..)
        | ExprKind::Mode(_)
        | ExprKind::OperandRead(_)
        | ExprKind::OperandRule(..)
        | ExprKind::Latch(_)
        | ExprKind::Port(..)
        | ExprKind::State(..)
        | ExprKind::ModelMem(..)
        | ExprKind::Error => {}
    }
}
