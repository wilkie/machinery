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
  | UnionDeclaration;
// NOTE: Other declaration kinds (`register`, `unit`, `machine`,
// `microword`, `operand`, `routine`) extend this union in follow-up
// slices.

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

// ---- Shared field node ----

/** A `name: type` field used by bundles (as fields) and unions (as arms).
 *  Operand and microword fields land here too in a later slice. */
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
