// CST → AST lowering pass.
//
// This module translates the Chevrotain CST produced by the parser into
// the typed AST defined in `./ast.ts`. Each CST node kind has a dedicated
// `lowerX(cst)` function that returns the corresponding AST node.
//
// Lowering is purely structural: no name resolution, no type checking,
// no cross-node analysis. Every lowering function is local — it reads
// the CST node's children, extracts the fields it needs, and constructs
// the AST node. All inter-node analysis lives in later semantic-phase
// slices.
//
// Coverage: first slice handles `file`, `enum`, `bundle`, `union`. Other
// declaration kinds land incrementally.

import type { CstNode, CstElement, IToken } from 'chevrotain';
import type {
  File,
  Declaration,
  EnumDeclaration,
  EnumVariant,
  BundleDeclaration,
  UnionDeclaration,
  NamedField,
  TypeRef,
  SourceLocation,
} from './ast.js';

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/**
 * Lower a `file` CST node (produced by `parse()` / `machineParser.file()`)
 * into a typed `File` AST. Declarations are returned in source order;
 * unknown or malformed declarations are skipped silently at this layer
 * (the parser will have produced diagnostics for them already).
 */
export function lowerFile(cst: CstNode): File {
  const declarations: Declaration[] = [];
  for (const child of asCstNodes(cst.children['declaration'])) {
    const lowered = lowerDeclaration(child);
    if (lowered) declarations.push(lowered);
  }
  return {
    kind: 'File',
    declarations,
    loc: locOf(cst),
  };
}

// ---------------------------------------------------------------------------
// Declaration dispatch
// ---------------------------------------------------------------------------

/**
 * Pick the matching declaration-kind sub-rule from the generic
 * `declaration` wrapper node and lower it. Returns `undefined` when the
 * declaration is of a kind we haven't lowered yet (e.g., `routine`,
 * `unit`, `machine`) — those are deferred to later slices, not errors.
 */
function lowerDeclaration(cst: CstNode): Declaration | undefined {
  const enumDecl = asCstNodes(cst.children['enumDecl'])[0];
  if (enumDecl) return lowerEnumDecl(enumDecl);

  const bundleDecl = asCstNodes(cst.children['bundleDecl'])[0];
  if (bundleDecl) return lowerBundleDecl(bundleDecl);

  const unionDecl = asCstNodes(cst.children['unionDecl'])[0];
  if (unionDecl) return lowerUnionDecl(unionDecl);

  // Other declaration kinds (registerDecl, unitDecl, machineDecl,
  // microwordDecl, operandDecl, routineDecl) are not yet lowered.
  // They're valid in the CST but return undefined here, which
  // `lowerFile` skips. When lowering support lands for each, add a
  // branch above.
  return undefined;
}

// ---------------------------------------------------------------------------
// Enum
// ---------------------------------------------------------------------------

function lowerEnumDecl(cst: CstNode): EnumDeclaration {
  const name = firstIdentifierImage(cst) ?? '';
  const variants: EnumVariant[] = [];

  const body = asCstNodes(cst.children['enumBody'])[0];
  if (body) {
    for (const v of asCstNodes(body.children['enumVariant'])) {
      // `enumVariant` accepts either Identifier or Fetch as a variant
      // name (the latter is a contextual-keyword accommodation for enum
      // members like `ExecuteState.fetch`). Try both token kinds.
      const tok =
        asTokens(v.children['Identifier'])[0] ??
        asTokens(v.children['Fetch'])[0];
      if (!tok) continue;
      variants.push({
        kind: 'EnumVariant',
        name: tok.image,
        loc: locOf(v),
      });
    }
  }

  return {
    kind: 'EnumDeclaration',
    name,
    variants,
    loc: locOf(cst),
  };
}

// ---------------------------------------------------------------------------
// Bundle
// ---------------------------------------------------------------------------

function lowerBundleDecl(cst: CstNode): BundleDeclaration {
  const name = firstIdentifierImage(cst) ?? '';
  const fields: NamedField[] = [];

  const body = asCstNodes(cst.children['bundleBody'])[0];
  if (body) {
    for (const f of asCstNodes(body.children['namedField'])) {
      const field = lowerNamedField(f);
      if (field) fields.push(field);
    }
  }

  return {
    kind: 'BundleDeclaration',
    name,
    fields,
    loc: locOf(cst),
  };
}

// ---------------------------------------------------------------------------
// Union
// ---------------------------------------------------------------------------

function lowerUnionDecl(cst: CstNode): UnionDeclaration {
  const name = firstIdentifierImage(cst) ?? '';
  const arms: NamedField[] = [];

  const body = asCstNodes(cst.children['unionBody'])[0];
  if (body) {
    for (const f of asCstNodes(body.children['namedField'])) {
      const field = lowerNamedField(f);
      if (field) arms.push(field);
    }
  }

  return {
    kind: 'UnionDeclaration',
    name,
    arms,
    loc: locOf(cst),
  };
}

// ---------------------------------------------------------------------------
// Shared: namedField (`name: type`)
// ---------------------------------------------------------------------------

function lowerNamedField(cst: CstNode): NamedField | undefined {
  const nameTok = asTokens(cst.children['Identifier'])[0];
  const typeNode = asCstNodes(cst.children['typeRef'])[0];
  if (!nameTok || !typeNode) return undefined;

  const type = lowerTypeRef(typeNode);
  if (!type) return undefined;

  return {
    kind: 'NamedField',
    name: nameTok.image,
    type,
    loc: locOf(cst),
  };
}

// ---------------------------------------------------------------------------
// typeRef
// ---------------------------------------------------------------------------

function lowerTypeRef(cst: CstNode): TypeRef | undefined {
  const nameTok = asTokens(cst.children['Identifier'])[0];
  if (!nameTok) return undefined;

  // First-slice coverage: only the bare-identifier form is lowered.
  // Parameterized types (`u{W+1}`, `Local{8}`) and array suffixes
  // (`MicroOp[]`) are valid in the CST but lower to `TypeRefSimple`
  // for now, discarding the parameterization. When the AST grows
  // richer typeRef kinds (see ast.ts), extend this function to
  // produce them.
  return {
    kind: 'TypeRefSimple',
    name: nameTok.image,
    loc: locOf(cst),
  };
}

// ---------------------------------------------------------------------------
// CST element helpers
// ---------------------------------------------------------------------------

/** First `Identifier` token image on a CST node, or undefined. Used for
 *  the common "declaration name is the first identifier" pattern. */
function firstIdentifierImage(cst: CstNode): string | undefined {
  return asTokens(cst.children['Identifier'])[0]?.image;
}

/**
 * Extract a source location from a CST node by walking its leftmost
 * token for the start and rightmost token for the end. Chevrotain
 * doesn't give us a nice `loc` field on CST nodes out of the box, so
 * we compute it here from the node's constituent tokens. The walk is
 * depth-first left-to-right for start and right-to-left for end.
 */
function locOf(cst: CstNode): SourceLocation {
  const first = firstToken(cst);
  const last = lastToken(cst);
  // If the node has no tokens at all (shouldn't happen for well-formed
  // CSTs, but the type allows it), return a zero-span placeholder so
  // downstream code never sees `undefined` for `loc`.
  if (!first || !last) {
    return {
      startOffset: 0,
      endOffset: 0,
      startLine: 1,
      startColumn: 1,
      endLine: 1,
      endColumn: 1,
    };
  }
  return {
    startOffset: first.startOffset ?? 0,
    endOffset: (last.endOffset ?? last.startOffset ?? 0) + 1,
    startLine: first.startLine ?? 1,
    startColumn: first.startColumn ?? 1,
    endLine: last.endLine ?? first.startLine ?? 1,
    endColumn: (last.endColumn ?? first.startColumn ?? 1),
  };
}

function firstToken(node: CstNode): IToken | undefined {
  for (const child of iterChildren(node)) {
    if (isToken(child)) return child;
    const inner = firstToken(child);
    if (inner) return inner;
  }
  return undefined;
}

function lastToken(node: CstNode): IToken | undefined {
  const all = [...iterChildren(node)];
  for (let i = all.length - 1; i >= 0; i--) {
    const child = all[i]!;
    if (isToken(child)) return child;
    const inner = lastToken(child);
    if (inner) return inner;
  }
  return undefined;
}

/** Yield every direct child of a CST node in source order (across all
 *  name buckets, interleaved by source offset). */
function* iterChildren(node: CstNode): Generator<CstElement> {
  const all: CstElement[] = [];
  for (const children of Object.values(node.children)) {
    if (children) all.push(...children);
  }
  all.sort((a, b) => tokenOffset(a) - tokenOffset(b));
  for (const c of all) yield c;
}

function tokenOffset(el: CstElement): number {
  if (isToken(el)) return el.startOffset ?? 0;
  const t = firstToken(el as CstNode);
  return t?.startOffset ?? 0;
}

function isToken(el: CstElement): el is IToken {
  return (el as CstNode).children === undefined;
}

function isCstNode(el: CstElement): el is CstNode {
  return (el as CstNode).children !== undefined;
}

function asCstNodes(children: CstElement[] | undefined): CstNode[] {
  if (!children) return [];
  return children.filter(isCstNode);
}

function asTokens(children: CstElement[] | undefined): IToken[] {
  if (!children) return [];
  return children.filter((c): c is IToken => !isCstNode(c));
}
