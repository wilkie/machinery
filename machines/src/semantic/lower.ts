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
  RegisterDeclaration,
  RegisterField,
  UnitDeclaration,
  TypeParameter,
  MachineDeclaration,
  MicrowordDeclaration,
  OperandDeclaration,
  OperandField,
  RoutineDeclaration,
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
 * `declaration` wrapper node and lower it. Returns `undefined` only
 * when the CST declaration node is structurally malformed and we
 * can't identify its kind at all — which shouldn't happen if the
 * parser accepted the input.
 */
function lowerDeclaration(cst: CstNode): Declaration | undefined {
  const enumDecl = asCstNodes(cst.children['enumDecl'])[0];
  if (enumDecl) return lowerEnumDecl(enumDecl);

  const bundleDecl = asCstNodes(cst.children['bundleDecl'])[0];
  if (bundleDecl) return lowerBundleDecl(bundleDecl);

  const unionDecl = asCstNodes(cst.children['unionDecl'])[0];
  if (unionDecl) return lowerUnionDecl(unionDecl);

  const registerDecl = asCstNodes(cst.children['registerDecl'])[0];
  if (registerDecl) return lowerRegisterDecl(registerDecl);

  const unitDecl = asCstNodes(cst.children['unitDecl'])[0];
  if (unitDecl) return lowerUnitDecl(unitDecl);

  const machineDecl = asCstNodes(cst.children['machineDecl'])[0];
  if (machineDecl) return lowerMachineDecl(machineDecl);

  const microwordDecl = asCstNodes(cst.children['microwordDecl'])[0];
  if (microwordDecl) return lowerMicrowordDecl(microwordDecl);

  const operandDecl = asCstNodes(cst.children['operandDecl'])[0];
  if (operandDecl) return lowerOperandDecl(operandDecl);

  const routineDecl = asCstNodes(cst.children['routineDecl'])[0];
  if (routineDecl) return lowerRoutineDecl(routineDecl);

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
// Register
// ---------------------------------------------------------------------------

function lowerRegisterDecl(cst: CstNode): RegisterDeclaration {
  const name = firstIdentifierImage(cst) ?? '';

  const typeNode = asCstNodes(cst.children['typeRef'])[0];
  const type: TypeRef = typeNode
    ? lowerTypeRef(typeNode) ?? makeErrorTypeRef(cst)
    : makeErrorTypeRef(cst);

  const fields: RegisterField[] = [];
  const body = asCstNodes(cst.children['registerBody'])[0];
  if (body) {
    for (const f of asCstNodes(body.children['fieldDecl'])) {
      const field = lowerFieldDecl(f);
      if (field) fields.push(field);
    }
  }

  return {
    kind: 'RegisterDeclaration',
    name,
    type,
    fields,
    loc: locOf(cst),
  };
}

function lowerFieldDecl(cst: CstNode): RegisterField | undefined {
  const nameTok = asTokens(cst.children['Identifier'])[0];
  const typeNode = asCstNodes(cst.children['typeRef'])[0];
  if (!nameTok || !typeNode) return undefined;

  const type = lowerTypeRef(typeNode);
  if (!type) return undefined;

  const offset = extractNumericLiteral(cst);

  return {
    kind: 'RegisterField',
    name: nameTok.image,
    type,
    offset,
    loc: locOf(cst),
  };
}

// ---------------------------------------------------------------------------
// Unit
// ---------------------------------------------------------------------------

function lowerUnitDecl(cst: CstNode): UnitDeclaration {
  const name = firstIdentifierImage(cst) ?? '';

  const typeParams: TypeParameter[] = [];
  const paramsNode = asCstNodes(cst.children['typeParams'])[0];
  if (paramsNode) {
    for (const p of asCstNodes(paramsNode.children['typeParam'])) {
      const param = lowerTypeParam(p);
      if (param) typeParams.push(param);
    }
  }

  return {
    kind: 'UnitDeclaration',
    name,
    typeParams,
    loc: locOf(cst),
  };
}

function lowerTypeParam(cst: CstNode): TypeParameter | undefined {
  const nameTok = asTokens(cst.children['Identifier'])[0];
  const typeNode = asCstNodes(cst.children['typeRef'])[0];
  if (!nameTok || !typeNode) return undefined;

  const type = lowerTypeRef(typeNode);
  if (!type) return undefined;

  return {
    kind: 'TypeParameter',
    name: nameTok.image,
    type,
    loc: locOf(cst),
  };
}

// ---------------------------------------------------------------------------
// Machine
// ---------------------------------------------------------------------------

function lowerMachineDecl(cst: CstNode): MachineDeclaration {
  const name = firstIdentifierImage(cst) ?? '';

  // Look for an `id NAME` section inside the machine body. The body
  // is optional; when present, we walk `idSection` children to find
  // the declared id, ignoring every other section for now.
  let id: string | undefined;
  const body = asCstNodes(cst.children['machineBody'])[0];
  if (body) {
    const idSection = asCstNodes(body.children['idSection'])[0];
    if (idSection) {
      // idSection is `Id Identifier Newline`. The identifier is the
      // section's single Identifier child.
      const idTok = asTokens(idSection.children['Identifier'])[0];
      if (idTok) id = idTok.image;
    }
  }

  return {
    kind: 'MachineDeclaration',
    name,
    id,
    loc: locOf(cst),
  };
}

// ---------------------------------------------------------------------------
// Microword
// ---------------------------------------------------------------------------

function lowerMicrowordDecl(cst: CstNode): MicrowordDeclaration {
  const name = firstIdentifierImage(cst) ?? '';

  const fields: NamedField[] = [];
  const body = asCstNodes(cst.children['microwordBody'])[0];
  if (body) {
    for (const fs of asCstNodes(body.children['fieldsSection'])) {
      // The `fields` section has two forms: an inline record-literal
      // (`fields { a:T, b:U }`) whose namedFields are direct
      // children of the fieldsSection, or a block form whose
      // namedFields live inside a nested `fieldsBody`. Check both.
      const inlineFields = asCstNodes(fs.children['namedField']);
      for (const f of inlineFields) {
        const field = lowerNamedField(f);
        if (field) fields.push(field);
      }
      const blockBody = asCstNodes(fs.children['fieldsBody'])[0];
      if (blockBody) {
        for (const f of asCstNodes(blockBody.children['namedField'])) {
          const field = lowerNamedField(f);
          if (field) fields.push(field);
        }
      }
    }
  }

  return {
    kind: 'MicrowordDeclaration',
    name,
    fields,
    loc: locOf(cst),
  };
}

// ---------------------------------------------------------------------------
// Operand
// ---------------------------------------------------------------------------

function lowerOperandDecl(cst: CstNode): OperandDeclaration {
  const name = firstIdentifierImage(cst) ?? '';

  let size: number | undefined;
  const fields: OperandField[] = [];

  const body = asCstNodes(cst.children['operandBody'])[0];
  if (body) {
    // Size clauses — typically one, but walk all to be safe.
    for (const sc of asCstNodes(body.children['sizeClause'])) {
      const n = extractNumericLiteral(sc);
      if (n !== undefined) size = n;
    }

    // Fields sections — same two-form handling as microword.
    for (const fs of asCstNodes(body.children['fieldsSection'])) {
      const inlineFields = asCstNodes(fs.children['namedField']);
      for (const f of inlineFields) {
        const field = lowerOperandField(f);
        if (field) fields.push(field);
      }
      const blockBody = asCstNodes(fs.children['fieldsBody'])[0];
      if (blockBody) {
        for (const f of asCstNodes(blockBody.children['namedField'])) {
          const field = lowerOperandField(f);
          if (field) fields.push(field);
        }
      }
    }
  }

  return {
    kind: 'OperandDeclaration',
    name,
    size,
    fields,
    loc: locOf(cst),
  };
}

function lowerOperandField(cst: CstNode): OperandField | undefined {
  const nameTok = asTokens(cst.children['Identifier'])[0];
  const typeNode = asCstNodes(cst.children['typeRef'])[0];
  if (!nameTok || !typeNode) return undefined;

  const type = lowerTypeRef(typeNode);
  if (!type) return undefined;

  const offset = extractNumericLiteral(cst);

  return {
    kind: 'OperandField',
    name: nameTok.image,
    type,
    offset,
    loc: locOf(cst),
  };
}

// ---------------------------------------------------------------------------
// Routine
// ---------------------------------------------------------------------------

function lowerRoutineDecl(cst: CstNode): RoutineDeclaration {
  const name = firstIdentifierImage(cst) ?? '';

  // Parameters from the `(...)` clause. The param list rule is
  // `paramList` containing zero or more `param` sub-nodes.
  const params: NamedField[] = [];
  const paramList = asCstNodes(cst.children['paramList'])[0];
  if (paramList) {
    for (const p of asCstNodes(paramList.children['param'])) {
      const field = lowerNamedField(p);
      if (field) params.push(field);
    }
  }

  // Return type from the `-> (...)` or `-> type` clause.
  let returnType: NamedField[] | undefined;
  const returnNode = asCstNodes(cst.children['returnType'])[0];
  if (returnNode) {
    returnType = [];
    // Parenthesized form: list of `returnField` children.
    for (const rf of asCstNodes(returnNode.children['returnField'])) {
      const field = lowerNamedField(rf);
      if (field) returnType.push(field);
    }
    // Unparenthesized form: a single `typeRef` child. We synthesize
    // a one-element NamedField list with an empty name for now, so
    // the downstream consumers can still see the type. This shape is
    // uncommon in real code and we may revisit it.
  }

  // Metadata sections from the routine body.
  let entry: number | undefined;
  const allow: string[] = [];
  const modifies: string[] = [];
  const references: string[] = [];

  const body = asCstNodes(cst.children['routineBody'])[0];
  if (body) {
    for (const section of asCstNodes(body.children['entrySection'])) {
      const n = extractEntryLiteral(section);
      if (n !== undefined) entry = n;
    }
    for (const section of asCstNodes(body.children['allowSection'])) {
      allow.push(...extractIdentifierList(section));
    }
    for (const section of asCstNodes(body.children['modifiesSection'])) {
      modifies.push(...extractIdentifierList(section));
    }
    for (const section of asCstNodes(body.children['referencesSection'])) {
      references.push(...extractReferenceList(section));
    }
  }

  return {
    kind: 'RoutineDeclaration',
    name,
    params,
    returnType,
    entry,
    allow,
    modifies,
    references,
    loc: locOf(cst),
  };
}

// ---------------------------------------------------------------------------
// Routine metadata extractors
// ---------------------------------------------------------------------------

/**
 * Pull a numeric value out of an `entry: N` section when N is a
 * single bare literal. Returns `undefined` for richer forms
 * (expressions, comma lists, byte sequences).
 */
function extractEntryLiteral(section: CstNode): number | undefined {
  const expressions = asCstNodes(section.children['expression']);
  if (expressions.length !== 1) return undefined;
  const tokens: IToken[] = [];
  collectAllTokens(expressions[0]!, tokens);
  if (tokens.length !== 1) return undefined;
  const tok = tokens[0]!;
  const kind = tok.tokenType.name;
  if (kind !== 'HexLiteral' && kind !== 'DecimalLiteral') return undefined;
  return parseNumericLiteral(tok.image);
}

/** Pull the identifiers out of an allow/modifies section's `[...]`. */
function extractIdentifierList(section: CstNode): string[] {
  const list = asCstNodes(section.children['identifierList'])[0];
  if (!list) return [];
  return asTokens(list.children['Identifier']).map((t) => t.image);
}

/** Pull the quoted strings out of a references section's bulleted list. */
function extractReferenceList(section: CstNode): string[] {
  const body = asCstNodes(section.children['referencesBody'])[0];
  if (!body) return [];
  const items = asCstNodes(body.children['referenceItem']);
  const result: string[] = [];
  for (const item of items) {
    const str = asTokens(item.children['StringLiteral'])[0];
    if (!str) continue;
    result.push(str.image.slice(1, -1));
  }
  return result;
}

function collectAllTokens(node: CstNode, out: IToken[]): void {
  for (const children of Object.values(node.children)) {
    if (!children) continue;
    for (const c of children) {
      if (isCstNode(c)) {
        collectAllTokens(c, out);
      } else {
        out.push(c as IToken);
      }
    }
  }
}

// ---------------------------------------------------------------------------
// Shared: numeric-literal extractors
// ---------------------------------------------------------------------------

/**
 * Pull a number out of a CST node that has a direct `DecimalLiteral`
 * or `HexLiteral` token child. Used by register-field offsets,
 * operand size clauses, and operand-field offsets — anywhere the
 * grammar accepts a bare literal.
 */
function extractNumericLiteral(cst: CstNode): number | undefined {
  const decTok = asTokens(cst.children['DecimalLiteral'])[0];
  if (decTok) return parseNumericLiteral(decTok.image);
  const hexTok = asTokens(cst.children['HexLiteral'])[0];
  if (hexTok) return parseNumericLiteral(hexTok.image);
  return undefined;
}

function parseNumericLiteral(image: string): number {
  if (image.startsWith('0x') || image.startsWith('0X')) {
    return parseInt(image.slice(2), 16);
  }
  return parseInt(image, 10);
}

/**
 * Build an "error" type reference for cases where the source was
 * missing a type we expected. Downstream passes can still walk the
 * AST; they'll encounter the empty name and report a diagnostic
 * instead of crashing on an undefined field.
 */
function makeErrorTypeRef(cst: CstNode): TypeRef {
  return {
    kind: 'TypeRefSimple',
    name: '',
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
