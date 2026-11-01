//! Abstract syntax of Machine.
//!
//! The AST is a faithful tree of the source after parsing: names are
//! unresolved strings, widths are expressions, nothing is monomorphised.
//! Every node carries a [`Span`]. Semantic analysis (a later crate) consumes
//! this tree and produces the typed IR; nothing here is "resolved".
//!
//! Naming convention: a `*Decl` is a top-level declaration; an `*Item` is a
//! declaration nested inside a body (`rtl`, `model`, `isa`, `bus`, `system`).

use crate::span::Span;

// ---------------------------------------------------------------------------
// Common
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub struct Ident {
    pub name: String,
    pub span: Span,
}

/// `a::b::c` or `a.b.c` style multi-segment name. Which separator was used
/// is implied by where the path appears (`::` in expressions and types, `.`
/// in system wiring and test targets); the segments alone are kept.
#[derive(Clone, Debug, PartialEq)]
pub struct Path {
    pub segments: Vec<Ident>,
    pub span: Span,
}

impl Path {
    pub fn single(ident: Ident) -> Path {
        let span = ident.span;
        Path {
            segments: vec![ident],
            span,
        }
    }
    pub fn last(&self) -> &Ident {
        self.segments.last().expect("path has at least one segment")
    }
    pub fn dotted(&self) -> String {
        self.segments
            .iter()
            .map(|s| s.name.as_str())
            .collect::<Vec<_>>()
            .join(".")
    }
}

/// Doc comments attached to a declaration, one entry per `///` line.
pub type Docs = Vec<String>;

/// A string literal node.
#[derive(Clone, Debug, PartialEq)]
pub struct StrLit {
    pub value: String,
    pub span: Span,
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub enum TypeKind {
    /// `u<N>` / `uN`
    Unsigned(Box<Expr>),
    /// `i<N>` / `iN`
    Signed(Box<Expr>),
    Bool,
    /// The kind of const width parameters: `const W: width`.
    Width,
    /// A named type, possibly with generic arguments: `Flags`, `Alu<8>`,
    /// `PeriphBus::Slave`.
    Named(Path, Vec<GenericArg>),
    /// `[T; N]`
    Array(Box<Type>, Box<Expr>),
    /// `(T, U)` or a named tuple `(result: u8, raw: u9)`.
    Tuple(Vec<TupleElem>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct TupleElem {
    pub name: Option<Ident>,
    pub ty: Type,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Type {
    pub kind: TypeKind,
    pub span: Span,
}

/// A generic argument is syntactically an expression (`8`, `W + 1`) or a
/// type; the parser cannot always tell, so a bare path is kept as a type and
/// anything else as an expression.
#[derive(Clone, Debug, PartialEq)]
pub enum GenericArg {
    Type(Type),
    Expr(Expr),
}

#[derive(Clone, Debug, PartialEq)]
pub struct GenericParam {
    pub name: Ident,
    /// `const W: width` → `Width`; `T: Type`-style params are not in the
    /// language yet, so this is always a const param today.
    pub kind: Type,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Param {
    pub name: Ident,
    pub ty: Type,
    pub span: Span,
}

// ---------------------------------------------------------------------------
// Expressions
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnaryOp {
    /// `!` (bool)
    Not,
    /// `~` (bitwise)
    BitNot,
    /// `-`
    Neg,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryOp {
    Mul,
    Div,
    Rem,
    Add,
    Sub,
    Shl,
    Shr,
    BitAnd,
    BitXor,
    BitOr,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}

/// A physical unit on a number literal, used in `system` clocks and in
/// tests: `33.333 MHz`, `70 ns`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unit {
    Hz,
    KHz,
    MHz,
    GHz,
    Ps,
    Ns,
    Us,
    Ms,
    S,
}

impl Unit {
    pub fn from_word(s: &str) -> Option<Unit> {
        Some(match s {
            "Hz" => Unit::Hz,
            "kHz" | "KHz" => Unit::KHz,
            "MHz" => Unit::MHz,
            "GHz" => Unit::GHz,
            "ps" => Unit::Ps,
            "ns" => Unit::Ns,
            "us" => Unit::Us,
            "ms" => Unit::Ms,
            "s" => Unit::S,
            _ => return None,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FieldInit {
    pub name: Ident,
    /// `None` for shorthand `Flags { cf, pf: x }`.
    pub value: Option<Expr>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MatchArm {
    pub patterns: Vec<Pattern>,
    pub body: ArmBody,
    pub span: Span,
}

/// A match arm is either an expression (`0 => q0,`) or a block of
/// statements (`0 => { reload <- x; }`). Which is legal depends on whether
/// the `match` is in expression or statement position; the parser accepts
/// both and semantic analysis checks.
#[derive(Clone, Debug, PartialEq)]
pub enum ArmBody {
    Expr(Expr),
    Block(Block),
}

#[derive(Clone, Debug, PartialEq)]
pub enum PatternKind {
    Wildcard,
    /// An integer or enum-variant path or constant name.
    Expr(Expr),
    /// `lo..=hi` (inclusive) or `lo..hi`.
    Range {
        lo: Expr,
        hi: Expr,
        inclusive: bool,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Pattern {
    pub kind: PatternKind,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ExprKind {
    Int {
        value: u128,
        suffix: Option<(char, u32)>,
    },
    Bool(bool),
    Str(String),
    /// `33.333 MHz`, `70 ns`. `value` keeps the literal as text for exact
    /// rational handling later; `f64` is for convenience.
    Quantity {
        value: f64,
        text: String,
        unit: Unit,
    },
    /// A name or namespaced path: `x`, `AluOp::Add`, `I286Isa::UD`.
    Path(Path),
    Unary(UnaryOp, Box<Expr>),
    Binary(BinaryOp, Box<Expr>, Box<Expr>),
    /// `c ? a : b`
    Ternary(Box<Expr>, Box<Expr>, Box<Expr>),
    Cast(Box<Expr>, Type),
    /// `a.f`
    Field(Box<Expr>, Ident),
    /// `a[i]`
    Index(Box<Expr>, Box<Expr>),
    /// `a[hi:lo]`
    Slice(Box<Expr>, Box<Expr>, Box<Expr>),
    /// `f(args)`, `Alu<8>(args)`, `io.respond(data: v, wait: 0)`. Arguments
    /// may be named.
    Call {
        callee: Box<Expr>,
        generics: Vec<GenericArg>,
        args: Vec<Arg>,
    },
    /// `Flags { cf: 1, .. }`
    BundleLit {
        ty: Path,
        fields: Vec<FieldInit>,
        /// `..default`
        rest_default: bool,
    },
    /// `(a, b)` or named `(result: x, raw: y)`. A one-element unnamed tuple
    /// is just a parenthesised expression and is not produced.
    Tuple(Vec<Arg>),
    /// `[a, b, c]`
    ArrayLit(Vec<Expr>),
    /// `[x; n]`
    ArrayRepeat(Box<Expr>, Box<Expr>),
    Match {
        scrutinee: Box<Expr>,
        arms: Vec<MatchArm>,
    },
    If {
        cond: Box<Expr>,
        then_branch: Block,
        else_branch: Option<Box<Expr>>,
    },
    /// A `{ ... }` block in expression position (the tail of an if/else).
    Block(Block),
    /// `call routine(args)` inside a routine: runs the callee to completion.
    RoutineCall(Box<Expr>),
    /// `fetch Operand` inside a routine.
    Fetch(Path),
    /// `a..b` / `a..=b` (only legal in `for` heads, address ranges, patterns).
    Range {
        lo: Box<Expr>,
        hi: Box<Expr>,
        inclusive: bool,
    },
    /// `_` in a destructuring position.
    Wildcard,
    /// Placeholder produced after a parse error so the tree stays total.
    Error,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Arg {
    pub name: Option<Ident>,
    pub value: Expr,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

// ---------------------------------------------------------------------------
// Statements
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub span: Span,
}

/// Left-hand side of `let`: a name or a tuple destructuring.
#[derive(Clone, Debug, PartialEq)]
pub enum LetPattern {
    Name(Ident),
    Tuple(Vec<LetPattern>),
    Wildcard(Span),
}

/// `@commit(on_retire)` and similar annotations on a register write.
#[derive(Clone, Debug, PartialEq)]
pub struct Annotation {
    pub name: Ident,
    pub args: Vec<Expr>,
    pub span: Span,
}

/// Which clock a register or routine belongs to: `@ clk`.
pub type ClockRef = Option<Ident>;

#[derive(Clone, Debug, PartialEq)]
pub enum MemOption {
    Ports(Expr),
    Read(Ident),
    Write(Ident),
    Init(Expr),
    Other(Ident, Option<Expr>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum RunSpec {
    /// `run instruction Path { operands };`
    Instruction {
        name: Path,
        operands: Vec<FieldInit>,
    },
    /// `run bytes [..];`
    Bytes(Expr),
    /// `run { stmts }`
    Block(Block),
    /// `run N cycles (of clk)?;`
    Cycles { count: Expr, clock: Option<Ident> },
}

#[derive(Clone, Debug, PartialEq)]
pub enum StmtKind {
    Let {
        pattern: LetPattern,
        ty: Option<Type>,
        value: Expr,
    },
    /// `lhs = rhs;` drive a wire (or an architectural write in `isa` code).
    Assign {
        target: Expr,
        value: Expr,
    },
    /// `lhs <- rhs @annotations;` schedule a register.
    Schedule {
        target: Expr,
        value: Expr,
        annotations: Vec<Annotation>,
    },
    Expr(Expr),
    If {
        cond: Expr,
        then_branch: Block,
        else_branch: Option<Box<Stmt>>,
    },
    Match {
        scrutinee: Expr,
        arms: Vec<MatchArm>,
    },
    For {
        var: Ident,
        range: Expr,
        body: Block,
    },
    Loop(Block),
    While {
        cond: Expr,
        body: Block,
    },
    Break,
    Continue,
    Return(Option<Expr>),
    /// `default { ... }`
    Default(Block),
    /// `on <target> (@ clock)? (params)? (-> ret)? { ... }`: an rtl clock
    /// block (`on tick { }`), or a model handler (`on io.read(addr: u8) ->
    /// (...)`, `on tick @ pixel (n: u32)`).
    On {
        target: Path,
        clock: ClockRef,
        params: Option<Vec<Param>>,
        ret: Option<Type>,
        body: Block,
    },
    /// `cycle { ... }` in a routine.
    Cycle(Block),
    /// `wait until cond;`
    WaitUntil(Expr),
    /// `retire;`
    Retire,
    /// `halt;` in isa code: stop until an interrupt or reset.
    Halt,
    /// `fault Path(args)?;`
    Fault(Expr),
    /// `raise Fault(args);` in isa code.
    Raise(Expr),
    /// `start routine(args);` in a sequencer.
    Start(Expr),
    /// `dispatch e via Path { otherwise => call; }`
    Dispatch {
        scrutinee: Expr,
        table: Path,
        otherwise: Option<Expr>,
    },
    /// `emit port.signal(args) (= value)?;`
    Emit {
        target: Expr,
        value: Option<Expr>,
    },
    /// `reg name: T @ clk = init;`
    Reg {
        docs: Docs,
        name: Ident,
        ty: Type,
        clock: ClockRef,
        init: Option<Expr>,
    },
    /// `wire name: T;` (optionally `= expr`)
    Wire {
        name: Ident,
        ty: Type,
        init: Option<Expr>,
    },
    /// `state name: T = init;` (model body)
    State {
        name: Ident,
        ty: Type,
        init: Option<Expr>,
    },
    /// `mem name: [T; N] { options }`
    Mem {
        docs: Docs,
        name: Ident,
        ty: Type,
        options: Vec<MemOption>,
    },
    /// `instance name: Comp<..> @ clk { port = expr, ... };`
    Instance {
        name: Ident,
        component: Type,
        clock: ClockRef,
        connections: Vec<FieldInit>,
    },
    Routine(RoutineDecl),
    Sequencer(SequencerDecl),
    /// A `unit` nested in an rtl body.
    Unit(UnitDecl),
    /// `macro name(params) { ... }`
    Macro {
        docs: Docs,
        name: Ident,
        params: Vec<Param>,
        body: Block,
    },
    /// `architectural { assigns; at retire; }`
    Architectural {
        mappings: Vec<(Expr, Expr)>,
        at: Option<Ident>,
    },
    /// `at retire;` inside `architectural`.
    At(Ident),
    // --- test and conformance statements ---
    /// `given { assigns }` / `drive { assigns }` / `expect { exprs }` /
    /// `clocks { name: freq; }`: a named block of statements.
    NamedBlock {
        name: Ident,
        body: Block,
    },
    Run(RunSpec),
    /// `stimulus name(args);` / `stimulus vectors "glob";`
    Stimulus(Expr),
    /// `compare <what> (within N cycles of clk)?;` — kept as raw words plus
    /// optional tolerance until the conformance sub-language settles.
    Compare {
        words: Vec<Ident>,
        signals: Option<Vec<Expr>>,
        within: Option<(Expr, Ident)>,
    },
    /// `report timing;`
    Report(Ident),
    // --- bus slave-side protocol statements (draft notation) ---
    /// `begins when cond;`
    BeginsWhen(Expr),
    /// `responds with (names) { ... }`
    RespondsWith {
        names: Vec<Ident>,
        body: Block,
    },
    /// `hold wire = value for n cycles;`
    Hold {
        target: Expr,
        value: Expr,
        cycles: Expr,
    },
    /// `drive wire = value on the sample cycle;`
    Drive {
        target: Expr,
        value: Expr,
        when: Vec<Ident>,
    },
    /// `sample wire on the first cycle;`
    Sample {
        target: Expr,
        when: Vec<Ident>,
    },
    /// Produced after a parse error; the parser skips to a synchronisation
    /// point and continues.
    Error,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

// ---------------------------------------------------------------------------
// Routines and sequencers
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub struct RoutineDecl {
    pub docs: Docs,
    /// `istream routine ...`
    pub istream: bool,
    pub name: Ident,
    pub params: Vec<Param>,
    pub ret: Option<Type>,
    pub clock: ClockRef,
    pub body: Block,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SequencerDecl {
    pub docs: Docs,
    pub name: Ident,
    pub clock: ClockRef,
    pub body: Block,
    pub span: Span,
}

// ---------------------------------------------------------------------------
// Top-level declarations
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub struct File {
    pub decls: Vec<Decl>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Decl {
    Use(UseDecl),
    Enum(EnumDecl),
    Bundle(BundleDecl),
    Union(UnionDecl),
    Const(ConstDecl),
    Unit(UnitDecl),
    Component(ComponentDecl),
    Bus(BusDecl),
    Isa(IsaDecl),
    System(SystemDecl),
    Test(TestDecl),
    Vectors(VectorsDecl),
    Conformance(ConformanceDecl),
    /// Placeholder for an unparseable declaration.
    Error(Span),
}

impl Decl {
    pub fn span(&self) -> Span {
        match self {
            Decl::Use(d) => d.span,
            Decl::Enum(d) => d.span,
            Decl::Bundle(d) => d.span,
            Decl::Union(d) => d.span,
            Decl::Const(d) => d.span,
            Decl::Unit(d) => d.span,
            Decl::Component(d) => d.span,
            Decl::Bus(d) => d.span,
            Decl::Isa(d) => d.span,
            Decl::System(d) => d.span,
            Decl::Test(d) => d.span,
            Decl::Vectors(d) => d.span,
            Decl::Conformance(d) => d.span,
            Decl::Error(s) => *s,
        }
    }

    pub fn keyword(&self) -> &'static str {
        match self {
            Decl::Use(_) => "use",
            Decl::Enum(_) => "enum",
            Decl::Bundle(_) => "bundle",
            Decl::Union(_) => "union",
            Decl::Const(_) => "const",
            Decl::Unit(_) => "unit",
            Decl::Component(_) => "component",
            Decl::Bus(_) => "bus",
            Decl::Isa(_) => "isa",
            Decl::System(_) => "system",
            Decl::Test(_) => "test",
            Decl::Vectors(_) => "vectors",
            Decl::Conformance(_) => "conformance",
            Decl::Error(_) => "<error>",
        }
    }
}

/// `use alu::{AluOp, Alu};` or `use alu::AluOp;`
#[derive(Clone, Debug, PartialEq)]
pub struct UseDecl {
    pub path: Path,
    /// Empty when the path itself names the single imported item.
    pub items: Vec<Ident>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EnumVariant {
    pub docs: Docs,
    pub name: Ident,
    pub value: Option<Expr>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EnumDecl {
    pub docs: Docs,
    pub name: Ident,
    pub repr: Type,
    pub variants: Vec<EnumVariant>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BundleField {
    pub docs: Docs,
    pub name: Ident,
    pub ty: Type,
    /// `@ offset`
    pub offset: Option<Expr>,
    /// `= default`
    pub default: Option<Expr>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BundleDecl {
    pub docs: Docs,
    pub name: Ident,
    pub fields: Vec<BundleField>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UnionVariant {
    pub docs: Docs,
    pub name: Ident,
    pub payload: Option<Type>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UnionDecl {
    pub docs: Docs,
    pub name: Ident,
    pub variants: Vec<UnionVariant>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ConstDecl {
    pub docs: Docs,
    pub name: Ident,
    pub ty: Type,
    pub value: Expr,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UnitDecl {
    pub docs: Docs,
    pub name: Ident,
    pub generics: Vec<GenericParam>,
    pub params: Vec<Param>,
    pub ret: Type,
    pub body: Block,
    pub span: Span,
}

// --- component ---

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PortDir {
    In,
    Out,
    /// `async in`
    AsyncIn,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PortKind {
    Wire {
        dir: PortDir,
        ty: Type,
    },
    /// `bus name: Protocol::Role;`
    Bus(Type),
    /// `clock name;`
    Clock,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Port {
    pub docs: Docs,
    pub name: Ident,
    pub kind: PortKind,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ComponentDecl {
    pub docs: Docs,
    pub name: Ident,
    pub generics: Vec<GenericParam>,
    /// `isa Name;`
    pub isa: Option<Path>,
    pub ports: Vec<Port>,
    pub rtl: Option<Block>,
    pub model: Option<Block>,
    pub span: Span,
}

// --- bus ---

#[derive(Clone, Debug, PartialEq)]
pub enum WireAttr {
    DrivenBy(Ident),
    Shared,
    WiredAnd,
    WiredOr,
    ActiveLow,
    Other(Ident, Option<Expr>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct BusWire {
    pub docs: Docs,
    pub name: Ident,
    pub ty: Type,
    pub attrs: Vec<WireAttr>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RoleBody {
    pub role: Ident,
    pub body: Block,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TransactionDecl {
    pub docs: Docs,
    pub name: Ident,
    pub params: Vec<Param>,
    pub ret: Option<Type>,
    pub roles: Vec<RoleBody>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SignalDecl {
    pub docs: Docs,
    pub name: Ident,
    pub params: Vec<Param>,
    pub value: Expr,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum BusItem {
    Clock(Ident),
    Params(Vec<GenericParam>),
    Wires(Vec<BusWire>),
    Roles(Vec<Ident>),
    Enum(EnumDecl),
    Transaction(TransactionDecl),
    Signal(SignalDecl),
}

#[derive(Clone, Debug, PartialEq)]
pub struct BusDecl {
    pub docs: Docs,
    pub name: Ident,
    pub items: Vec<BusItem>,
    pub span: Span,
}

// --- isa ---

#[derive(Clone, Debug, PartialEq)]
pub struct SpaceDecl {
    pub docs: Docs,
    pub name: Ident,
    pub addr: Type,
    pub cell: Type,
    pub attrs: Vec<Ident>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RegisterField {
    pub docs: Docs,
    pub name: Ident,
    pub ty: Type,
    pub offset: Expr,
    pub span: Span,
}

/// `view arith: ArithFlags { cf: CF, ... }` — a bundle-typed alias over
/// register fields.
#[derive(Clone, Debug, PartialEq)]
pub struct RegisterView {
    pub docs: Docs,
    pub name: Ident,
    pub ty: Type,
    pub mapping: Vec<FieldInit>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RegisterDecl {
    pub docs: Docs,
    pub name: Ident,
    pub ty: Type,
    pub fields: Vec<RegisterField>,
    pub views: Vec<RegisterView>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FaultDecl {
    pub docs: Docs,
    pub name: Ident,
    pub params: Vec<Param>,
    pub attrs: Vec<FieldInit>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FnDecl {
    pub docs: Docs,
    pub name: Ident,
    pub params: Vec<Param>,
    pub ret: Option<Type>,
    pub body: Block,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OperandField {
    pub name: Ident,
    pub ty: Type,
    pub offset: Expr,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OperandDecl {
    pub docs: Docs,
    pub name: Ident,
    pub size: Option<Expr>,
    pub fields: Vec<OperandField>,
    /// `tail { ... }`: which further operands follow, as a block whose
    /// value is an operand name.
    pub tail: Option<Block>,
    pub span: Span,
}

/// One `name => expr;` or `name(params) => block;` rule in an `operand_kind`.
#[derive(Clone, Debug, PartialEq)]
pub struct KindRule {
    pub name: Ident,
    pub params: Vec<Param>,
    pub body: ArmBody,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OperandKindDecl {
    pub docs: Docs,
    pub name: Ident,
    pub ty: Type,
    /// `from m: ModRM`: the source operand and the name it is bound to in
    /// the rules.
    pub from: Option<(Ident, Path)>,
    pub rules: Vec<KindRule>,
    pub span: Span,
}

/// An item of an `encoding { }` block: `opcode 0x00;`, `modrm { reg: r8, rm: rm8 }`,
/// `imm8;`, `prefix 0x66;`.
#[derive(Clone, Debug, PartialEq)]
pub struct EncodingItem {
    pub name: Ident,
    pub arg: Option<Expr>,
    pub fields: Vec<FieldInit>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OperandBinding {
    pub name: Ident,
    pub kind: Path,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum InstructionItem {
    /// A string, or a string-valued expression over the operands.
    Mnemonic(Expr),
    Encoding(Vec<EncodingItem>),
    Operands(Vec<OperandBinding>),
    /// `flags { modifies: [..]; undefined: [..]; reads: [..]; }`
    /// `flags { modifies: [CF, ZF]; undefined: [AF if is_logic(op)]; }`
    Flags(Vec<FlagSet>),
    Semantics(Block),
    Timing(Vec<FieldInit>),
    Implements(Path),
    References(Vec<StrLit>),
    AllowedIn(Vec<Expr>),
    Requires(Expr),
    Other(Ident, Option<Expr>),
}

/// One entry of a flag set: a register field, optionally conditional.
#[derive(Clone, Debug, PartialEq)]
pub struct FlagEntry {
    pub flag: Path,
    pub cond: Option<Expr>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FlagSet {
    pub name: Ident,
    pub entries: Vec<FlagEntry>,
    pub span: Span,
}

/// `latch name: T = init;`: per-instruction state, reset at every
/// instruction boundary (prefix effects).
#[derive(Clone, Debug, PartialEq)]
pub struct LatchDecl {
    pub docs: Docs,
    pub name: Ident,
    pub ty: Type,
    pub init: Expr,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InstructionDecl {
    pub docs: Docs,
    /// Declared with `prefix` instead of `instruction`.
    pub prefix: bool,
    pub name: Ident,
    pub items: Vec<InstructionItem>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum IsaItem {
    Space(SpaceDecl),
    Register(RegisterDecl),
    /// `mode Name : repr { A, B }` — an enum that is also a register.
    Mode(EnumDecl),
    Fault(FaultDecl),
    Fn(FnDecl),
    Operand(OperandDecl),
    OperandKind(OperandKindDecl),
    Instruction(InstructionDecl),
    Enum(EnumDecl),
    Bundle(BundleDecl),
    Const(ConstDecl),
    /// `istream { space mem; address code_address; next next_byte; undefined UD; }`
    Istream(IstreamDecl),
    Latch(LatchDecl),
}

/// Where instruction bytes come from: the space they live in, the function
/// giving the address of the next byte, the function that consumes one, and
/// the fault raised for an undecodable opcode.
#[derive(Clone, Debug, PartialEq)]
pub struct IstreamDecl {
    pub space: Ident,
    pub address: Path,
    pub next: Path,
    pub undefined: Option<Path>,
    /// `fault deliver;`: the `fn (vector: u8)` that delivers a raised fault
    /// (after rollback for `Fault`-class faults).
    pub fault: Option<Path>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IsaDecl {
    pub docs: Docs,
    pub name: Ident,
    pub items: Vec<IsaItem>,
    pub span: Span,
}

// --- system ---

/// How a component is bound to clocks at instantiation: `@ clk`,
/// `@ (osc / 12)`, `@ (bus_clk: bus_clk, tick: tick)`, `@ (bclk, pixel)`.
#[derive(Clone, Debug, PartialEq)]
pub struct ClockBinding {
    pub bindings: Vec<Arg>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ClockSource {
    /// `clock name: 8 MHz;`
    Frequency(Expr),
    /// `clock name = expr;`
    Derived(Expr),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClockDecl {
    pub docs: Docs,
    pub name: Ident,
    pub source: ClockSource,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ComponentInst {
    pub docs: Docs,
    pub name: Ident,
    pub component: Type,
    pub clocks: Option<ClockBinding>,
    pub span: Span,
}

/// `io 0x3B0..=0x3DF`, `mem 0xA0000..=0xBFFFF`, or a bare range.
#[derive(Clone, Debug, PartialEq)]
pub struct AddressRange {
    pub space: Option<Ident>,
    pub range: Expr,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum BusBinding {
    Master(Path),
    Slave {
        port: Path,
        ranges: Vec<AddressRange>,
    },
    Bridge(Path),
    /// `irq 0 = source;`
    Line {
        name: Ident,
        index: Expr,
        source: Expr,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct BusInst {
    pub docs: Docs,
    pub name: Ident,
    pub protocol: Type,
    pub clock: ClockRef,
    pub bindings: Vec<BusBinding>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProfileEntry {
    /// `default` or a component path like `vga.crtc`.
    pub target: Path,
    pub level: Ident,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProfileDecl {
    pub docs: Docs,
    pub name: Ident,
    pub entries: Vec<ProfileEntry>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SystemItem {
    Clock(ClockDecl),
    Component(ComponentInst),
    Bus(BusInst),
    /// `wire cpu.intr = pic.intr;`
    Wire {
        target: Path,
        source: Expr,
        span: Span,
    },
    Profile(ProfileDecl),
}

#[derive(Clone, Debug, PartialEq)]
pub struct SystemDecl {
    pub docs: Docs,
    pub name: Ident,
    pub items: Vec<SystemItem>,
    pub span: Span,
}

// --- verification ---

#[derive(Clone, Debug, PartialEq)]
pub struct TestDecl {
    pub docs: Docs,
    pub name: StrLit,
    /// `on I286Isa`, `on Timer.rtl`
    pub target: Option<Path>,
    pub body: Block,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VectorsDecl {
    pub docs: Docs,
    pub glob: StrLit,
    pub target: Path,
    /// `{ format moo; memory mem; reg ax = AX; ... }`
    pub items: Vec<VectorsItem>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum VectorsItem {
    /// `format moo;`
    Format(Ident),
    /// `memory mem;`: the address space vector RAM addresses refer to.
    Memory(Ident),
    /// `open io = 0xFF;`: in the test environment nothing answers on this
    /// space; reads return the value and writes are discarded.
    Open {
        space: Ident,
        value: Expr,
        span: Span,
    },
    /// `reg ax = AX;`: load assigns the place, compare reads it.
    Reg {
        name: Ident,
        place: Expr,
        span: Span,
    },
    /// `reg cs { load => <stmt using `value`>; read => <expr>; }`
    RegCustom {
        name: Ident,
        load: Box<Stmt>,
        read: Expr,
        span: Span,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct ConformanceDecl {
    pub docs: Docs,
    pub target: Path,
    pub body: Block,
    pub span: Span,
}
