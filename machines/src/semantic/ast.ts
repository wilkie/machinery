// Typed AST for .machine source files.
//
// This module defines the abstract syntax tree produced by the lowering
// pass that runs over a parser CST. Every later semantic pass (resolver,
// type checker, structural rules, dataflow) walks this tree — not the
// CST — so its shape is chosen for analysis convenience rather than for
// parsing.
//
// Design conventions:
//
// - **Every node has a `kind` discriminator** and a `loc` source position,
//   so TypeScript narrowing works uniformly and diagnostics can point at
//   the right place.
// - **Field names are semantic** (`name`, `type`, `variants`), not
//   grammar-shaped (`Identifier`, `children`, `bundleBody`).
// - **Optional fields are optional**, not zero-or-one arrays. Arrays are
//   used only for genuinely-repeated structure.
// - **Source locations are always populated**, even for synthesized nodes
//   (use the closest real token's location). Downstream diagnostic quality
//   depends on this.
//
// Coverage is incremental: this first slice covers `file`, `enum`,
// `bundle`, and `union` declarations end to end. The other declaration
// kinds, expressions, statements, and type references will land in
// follow-up slices, extending the discriminated unions below.

// ---------------------------------------------------------------------------
// Source locations
// ---------------------------------------------------------------------------

/** A half-open `[start, end)` range in the source, in absolute offsets
 *  plus line/column pairs for human-readable diagnostics. */
export interface SourceLocation {
  /** Byte offset of the first character of the node. */
  startOffset: number;
  /** Byte offset one past the last character of the node. */
  endOffset: number;
  /** 1-based line number of the first character. */
  startLine: number;
  /** 1-based column of the first character. */
  startColumn: number;
  /** 1-based line number of the last character. */
  endLine: number;
  /** 1-based column of the last character. */
  endColumn: number;
}

// ---------------------------------------------------------------------------
// File — the root node
// ---------------------------------------------------------------------------

/** Top-level AST node: one file's worth of declarations in source order. */
export interface File {
  kind: 'File';
  declarations: Declaration[];
  loc: SourceLocation;
}

// ---------------------------------------------------------------------------
// Declarations
// ---------------------------------------------------------------------------

/** A top-level declaration. Every kind has `kind`, `name`, and `loc` at
 *  a minimum; most add their own structural fields. */
export type Declaration =
  | EnumDeclaration
  | BundleDeclaration
  | UnionDeclaration
  | RegisterDeclaration
  | UnitDeclaration
  | MachineDeclaration
  | MicrowordDeclaration
  | OperandDeclaration
  | RoutineDeclaration;

// ---- Enum ----

export interface EnumDeclaration {
  kind: 'EnumDeclaration';
  name: string;
  variants: EnumVariant[];
  loc: SourceLocation;
}

export interface EnumVariant {
  kind: 'EnumVariant';
  name: string;
  loc: SourceLocation;
}

// ---- Bundle ----

export interface BundleDeclaration {
  kind: 'BundleDeclaration';
  name: string;
  fields: NamedField[];
  loc: SourceLocation;
}

// ---- Union ----

export interface UnionDeclaration {
  kind: 'UnionDeclaration';
  name: string;
  arms: NamedField[];
  loc: SourceLocation;
}

// ---- Register ----

export interface RegisterDeclaration {
  kind: 'RegisterDeclaration';
  name: string;
  /** The register's overall type (e.g., `u16`, `seg16`). */
  type: TypeRef;
  /** Named bit fields of the register, in source order. Empty for
   *  registers without a body. */
  fields: RegisterField[];
  loc: SourceLocation;
}

export interface RegisterField {
  kind: 'RegisterField';
  name: string;
  type: TypeRef;
  /** Bit offset from the `@ N` clause. `undefined` when the source
   *  omits the offset (rare but legal). */
  offset: number | undefined;
  loc: SourceLocation;
}

// ---- Unit ----

/**
 * A `unit` declaration. First-slice coverage captures name and type
 * parameters only; the body (combinational statements, `wires in/out`
 * sections, etc.) is parsed but not yet lowered — it requires
 * statement/expression AST support which lands in a follow-up slice.
 */
export interface UnitDeclaration {
  kind: 'UnitDeclaration';
  name: string;
  typeParams: TypeParameter[];
  loc: SourceLocation;
}

export interface TypeParameter {
  kind: 'TypeParameter';
  name: string;
  /** The constraint type (e.g. `Width` in `<W:Width>`). */
  type: TypeRef;
  loc: SourceLocation;
}

// ---- Machine ----

/**
 * A `machine` declaration. First-slice coverage captures name and the
 * optional `id` tag; every other machine-body section (registers,
 * wires in/out, default, instances, rom, and statement-level body
 * content) requires downstream AST support and lands in follow-up
 * slices.
 */
export interface MachineDeclaration {
  kind: 'MachineDeclaration';
  name: string;
  /** The `id NAME` tag from the machine body, if present. Used by the
   *  executionUnit to identify which processor it represents. */
  id: string | undefined;
  loc: SourceLocation;
}

// ---- Microword ----

/**
 * A `microword` declaration. Captures the variant name and its field
 * list. The `description`, `ready`, `terminal`, and `effect` sections
 * are parsed but not yet lowered — they contain expressions / prose /
 * statement blocks that require follow-up slices.
 */
export interface MicrowordDeclaration {
  kind: 'MicrowordDeclaration';
  name: string;
  /** Typed fields from the `fields` section, in source order. May be
   *  empty when the microword has no per-instance parameters
   *  (`fields {}` case). */
  fields: NamedField[];
  loc: SourceLocation;
}

// ---- Operand ----

/**
 * An `operand` declaration. Captures name, size, and bit-offset
 * fields. The `description` and `fetch` sections are parsed but not
 * yet lowered.
 */
export interface OperandDeclaration {
  kind: 'OperandDeclaration';
  name: string;
  /** Byte count from the `size: N` clause. `undefined` if omitted. */
  size: number | undefined;
  /** The operand's bit-layout fields, in source order. */
  fields: OperandField[];
  loc: SourceLocation;
}

export interface OperandField {
  kind: 'OperandField';
  name: string;
  type: TypeRef;
  /** Bit offset from `@ N`, or undefined when the field has no
   *  explicit offset. */
  offset: number | undefined;
  loc: SourceLocation;
}

// ---- Routine ----

/**
 * A `routine` declaration. Captures name, parameter list, return
 * type, and author-written metadata (entry, allow, modifies,
 * references). The `description` block and `micro` body are parsed
 * but not yet lowered — they contain prose and statement blocks
 * respectively.
 */
export interface RoutineDeclaration {
  kind: 'RoutineDeclaration';
  name: string;
  /** Parameter list from `(...)`. Empty for routines with no params. */
  params: NamedField[];
  /** Return type fields from `-> (...)`. Undefined when the routine
   *  has no return-type clause at all. An empty array represents
   *  the `-> ()` form (explicit no-return). */
  returnType: NamedField[] | undefined;
  /** Opcode value from an `entry: N` clause, when N is a single bare
   *  numeric literal. More complex entry forms (expressions, group
   *  constraints, byte sequences) are not yet lowered; this field is
   *  undefined in those cases. */
  entry: number | undefined;
  /** Identifiers from the `allow: [...]` policy list. */
  allow: string[];
  /** Identifiers from the `modifies: [...]` list. */
  modifies: string[];
  /** Quoted strings (with quotes stripped) from the `references`
   *  bullet list. */
  references: string[];
  loc: SourceLocation;
}

// ---- Shared field node ----

/** A `name: type` field used by bundles (as fields), unions (as arms),
 *  microword field sections, and routine parameter / return lists. */
export interface NamedField {
  kind: 'NamedField';
  name: string;
  type: TypeRef;
  loc: SourceLocation;
}

// ---------------------------------------------------------------------------
// Type references
// ---------------------------------------------------------------------------

/** A source-level type reference. First-slice form is just an identifier;
 *  later slices will extend this with parameterized types (`u{W+1}`,
 *  `Local{8}`) and array suffixes (`MicroOp[]`). */
export type TypeRef = TypeRefSimple;

export interface TypeRefSimple {
  kind: 'TypeRefSimple';
  name: string;
  loc: SourceLocation;
}

// ---------------------------------------------------------------------------
// Narrowing helpers
// ---------------------------------------------------------------------------
// Small discriminator predicates so callers don't have to remember the
// exact string literals when filtering declaration lists.

export function isEnumDeclaration(d: Declaration): d is EnumDeclaration {
  return d.kind === 'EnumDeclaration';
}

export function isBundleDeclaration(d: Declaration): d is BundleDeclaration {
  return d.kind === 'BundleDeclaration';
}

export function isUnionDeclaration(d: Declaration): d is UnionDeclaration {
  return d.kind === 'UnionDeclaration';
}

export function isRegisterDeclaration(
  d: Declaration,
): d is RegisterDeclaration {
  return d.kind === 'RegisterDeclaration';
}

export function isUnitDeclaration(d: Declaration): d is UnitDeclaration {
  return d.kind === 'UnitDeclaration';
}

export function isMachineDeclaration(
  d: Declaration,
): d is MachineDeclaration {
  return d.kind === 'MachineDeclaration';
}

export function isMicrowordDeclaration(
  d: Declaration,
): d is MicrowordDeclaration {
  return d.kind === 'MicrowordDeclaration';
}

export function isOperandDeclaration(
  d: Declaration,
): d is OperandDeclaration {
  return d.kind === 'OperandDeclaration';
}

export function isRoutineDeclaration(
  d: Declaration,
): d is RoutineDeclaration {
  return d.kind === 'RoutineDeclaration';
}
