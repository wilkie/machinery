// Top-level parse entry point.
//
// Wires the lexer through the parser and returns the CST plus any
// errors encountered along the way. Also provides a `getDeclarations`
// helper for quickly listing the top-level declarations in a file,
// which is the main thing the skeleton grammar can answer today.

import type {
  CstNode,
  CstElement,
  IToken,
  ILexingError,
  IRecognitionException,
} from 'chevrotain';
import { lex } from '../lexer/lex.js';
import { machineParser } from './parser.js';

export interface ParseResult {
  /** The root CST node, or undefined if lexing failed. */
  cst: CstNode | undefined;
  lexErrors: readonly ILexingError[];
  parseErrors: readonly IRecognitionException[];
}

/**
 * Parse a `.machine` source string. On lexer failure, returns the lex
 * errors without running the parser. On parse failure with recovery,
 * returns a partial CST alongside the parser's error list.
 */
export function parse(source: string): ParseResult {
  const lexResult = lex(source);
  if (lexResult.errors.length > 0) {
    return {
      cst: undefined,
      lexErrors: lexResult.errors,
      parseErrors: [],
    };
  }
  machineParser.input = lexResult.tokens;
  const cst = machineParser.file();
  return {
    cst,
    lexErrors: [],
    parseErrors: machineParser.errors,
  };
}

/** The nine top-level declaration kinds recognized by the skeleton parser. */
export type DeclarationKind =
  | 'registerDecl'
  | 'enumDecl'
  | 'bundleDecl'
  | 'unionDecl'
  | 'unitDecl'
  | 'machineDecl'
  | 'microwordDecl'
  | 'operandDecl'
  | 'routineDecl';

export interface TopLevelDecl {
  kind: DeclarationKind;
  name: string;
}

const DECLARATION_KINDS: readonly DeclarationKind[] = [
  'registerDecl',
  'enumDecl',
  'bundleDecl',
  'unionDecl',
  'unitDecl',
  'machineDecl',
  'microwordDecl',
  'operandDecl',
  'routineDecl',
];

/**
 * Walk a `file` CST and return the list of top-level declarations in
 * source order. Each entry is the kind (which top-level grammar rule
 * matched) and the declared name.
 *
 * The skeleton parser captures kind and name for every declaration;
 * richer per-construct information (enum variants, register fields,
 * routine bodies) comes from dedicated accessors like `getEnums`.
 */
export function getDeclarations(file: CstNode): TopLevelDecl[] {
  const result: TopLevelDecl[] = [];
  const declNodes = asCstNodes(file.children['declaration']);
  for (const decl of declNodes) {
    for (const kind of DECLARATION_KINDS) {
      const matched = asCstNodes(decl.children[kind]);
      if (matched.length === 0) continue;
      const inner = matched[0];
      if (!inner) continue;
      const nameTokens = asTokens(inner.children['Identifier']);
      const nameToken = nameTokens[0];
      if (!nameToken) continue;
      result.push({ kind, name: nameToken.image });
      break;
    }
  }
  return result;
}

export interface EnumInfo {
  name: string;
  variants: string[];
}

/**
 * Walk a `file` CST and return every enum declaration as a plain data
 * object, with the enum's name and its variants in source order. Enums
 * without a body are reported with an empty variants list.
 */
export function getEnums(file: CstNode): EnumInfo[] {
  const result: EnumInfo[] = [];
  const declNodes = asCstNodes(file.children['declaration']);
  for (const decl of declNodes) {
    const enumDecls = asCstNodes(decl.children['enumDecl']);
    for (const enumDecl of enumDecls) {
      const nameToken = asTokens(enumDecl.children['Identifier'])[0];
      if (!nameToken) continue;
      const variants: string[] = [];
      const body = asCstNodes(enumDecl.children['enumBody'])[0];
      if (body) {
        const variantNodes = asCstNodes(body.children['enumVariant']);
        for (const v of variantNodes) {
          const variantToken = asTokens(v.children['Identifier'])[0];
          if (variantToken) variants.push(variantToken.image);
        }
      }
      result.push({ name: nameToken.image, variants });
    }
  }
  return result;
}

export interface BundleField {
  name: string;
  type: string;
}

export interface BundleInfo {
  name: string;
  fields: BundleField[];
}

/**
 * Walk a `file` CST and return every bundle declaration as a plain data
 * object, with its fields in source order. Bundles without a body are
 * reported with an empty fields list.
 */
export function getBundles(file: CstNode): BundleInfo[] {
  const result: BundleInfo[] = [];
  const declNodes = asCstNodes(file.children['declaration']);
  for (const decl of declNodes) {
    const bundleDecls = asCstNodes(decl.children['bundleDecl']);
    for (const bundleDecl of bundleDecls) {
      const nameToken = asTokens(bundleDecl.children['Identifier'])[0];
      if (!nameToken) continue;
      const body = asCstNodes(bundleDecl.children['bundleBody'])[0];
      const fields = body ? extractNamedFields(body) : [];
      result.push({ name: nameToken.image, fields });
    }
  }
  return result;
}

export interface UnionArm {
  name: string;
  type: string;
}

export interface UnionInfo {
  name: string;
  arms: UnionArm[];
}

export interface OperandField {
  name: string;
  type: string;
  /** Bit offset from `@ N` on a field line, or undefined if omitted. */
  offset: number | undefined;
}

export interface OperandInfo {
  name: string;
  /** Byte count from the `size: N` clause, or undefined if omitted. */
  size: number | undefined;
  fields: OperandField[];
}

/**
 * Walk a `file` CST and return every operand declaration as a plain data
 * object. Captures the operand's name, its `size: N` value (if any), and
 * its `fields` section contents with bit offsets (if any). The
 * `description` and `fetch` sections are parsed structurally but their
 * contents are opaque until expression and statement grammars land.
 */
export function getOperands(file: CstNode): OperandInfo[] {
  const result: OperandInfo[] = [];
  const declNodes = asCstNodes(file.children['declaration']);
  for (const decl of declNodes) {
    const operandDecls = asCstNodes(decl.children['operandDecl']);
    for (const opDecl of operandDecls) {
      const nameToken = asTokens(opDecl.children['Identifier'])[0];
      if (!nameToken) continue;
      const body = asCstNodes(opDecl.children['operandBody'])[0];

      let size: number | undefined;
      const fields: OperandField[] = [];

      if (body) {
        // Size: look at any sizeClause sub-nodes (usually exactly one).
        const sizeClauses = asCstNodes(body.children['sizeClause']);
        for (const sc of sizeClauses) {
          const decTokens = asTokens(sc.children['DecimalLiteral']);
          const hexTokens = asTokens(sc.children['HexLiteral']);
          const literal = decTokens[0] ?? hexTokens[0];
          if (literal) {
            size = parseNumericLiteral(literal.image);
          }
        }

        // Fields: same traversal as microwords, but keep offsets.
        const fieldsSections = asCstNodes(body.children['fieldsSection']);
        for (const fs of fieldsSections) {
          const fieldsBodyNode = asCstNodes(fs.children['fieldsBody'])[0];
          const container = fieldsBodyNode ?? fs;
          fields.push(...extractNamedFieldsWithOffsets(container));
        }
      }

      result.push({ name: nameToken.image, size, fields });
    }
  }
  return result;
}

/**
 * Like `extractNamedFields` but also pulls the optional `@ offset` that
 * the grammar allows on every namedField. Used by operand walking; the
 * other container walkers (bundle / union / microword) drop the offset.
 */
function extractNamedFieldsWithOffsets(container: CstNode): OperandField[] {
  const result: OperandField[] = [];
  const fieldNodes = asCstNodes(container.children['namedField']);
  for (const f of fieldNodes) {
    const fieldName = asTokens(f.children['Identifier'])[0]?.image;
    const typeNode = asCstNodes(f.children['typeRef'])[0];
    const typeName = typeNode
      ? asTokens(typeNode.children['Identifier'])[0]?.image
      : undefined;
    if (!fieldName || !typeName) continue;
    const decTokens = asTokens(f.children['DecimalLiteral']);
    const hexTokens = asTokens(f.children['HexLiteral']);
    const literal = decTokens[0] ?? hexTokens[0];
    const offset = literal ? parseNumericLiteral(literal.image) : undefined;
    result.push({ name: fieldName, type: typeName, offset });
  }
  return result;
}

export interface MicrowordInfo {
  name: string;
  fields: BundleField[];
}

/**
 * Walk a `file` CST and return every microword declaration as a plain
 * data object, with its name and `fields` section contents in source
 * order. Only the `fields` section is interpreted today — description,
 * ready, and effect are parsed structurally but their contents are
 * opaque until expression grammar lands.
 *
 * Microwords with no body, no `fields` section, or an empty inline
 * `fields {}` all return with an empty fields list.
 */
export function getMicrowords(file: CstNode): MicrowordInfo[] {
  const result: MicrowordInfo[] = [];
  const declNodes = asCstNodes(file.children['declaration']);
  for (const decl of declNodes) {
    const microwordDecls = asCstNodes(decl.children['microwordDecl']);
    for (const mwDecl of microwordDecls) {
      const nameToken = asTokens(mwDecl.children['Identifier'])[0];
      if (!nameToken) continue;
      const body = asCstNodes(mwDecl.children['microwordBody'])[0];
      const fields: BundleField[] = [];
      if (body) {
        const fieldsSections = asCstNodes(body.children['fieldsSection']);
        for (const fs of fieldsSections) {
          // Block form: fields live inside a fieldsBody sub-node.
          const fieldsBodyNode = asCstNodes(fs.children['fieldsBody'])[0];
          if (fieldsBodyNode) {
            fields.push(...extractNamedFields(fieldsBodyNode));
          } else {
            // Inline form: namedFields are direct children of fieldsSection.
            fields.push(...extractNamedFields(fs));
          }
        }
      }
      result.push({ name: nameToken.image, fields });
    }
  }
  return result;
}

export interface FieldInfo {
  name: string;
  type: string;
  /** Bit offset from `@ N`, or undefined if the field had no `@` clause. */
  offset: number | undefined;
}

export interface RegisterInfo {
  name: string;
  /** The register's overall type, e.g. `u16` or `seg16`. */
  type: string;
  fields: FieldInfo[];
}

/**
 * Walk a `file` CST and return every top-level register declaration as a
 * plain data object, with the register's overall type and any bit fields
 * in source order. Registers without a body return with an empty fields
 * list; fields without an `@ N` offset clause return with `offset: undefined`.
 */
export function getRegisters(file: CstNode): RegisterInfo[] {
  const result: RegisterInfo[] = [];
  const declNodes = asCstNodes(file.children['declaration']);
  for (const decl of declNodes) {
    const registerDecls = asCstNodes(decl.children['registerDecl']);
    for (const registerDecl of registerDecls) {
      const nameToken = asTokens(registerDecl.children['Identifier'])[0];
      if (!nameToken) continue;
      const typeNode = asCstNodes(registerDecl.children['typeRef'])[0];
      const typeName = typeNode
        ? asTokens(typeNode.children['Identifier'])[0]?.image
        : undefined;
      if (!typeName) continue;

      const fields: FieldInfo[] = [];
      const body = asCstNodes(registerDecl.children['registerBody'])[0];
      if (body) {
        const fieldNodes = asCstNodes(body.children['fieldDecl']);
        for (const f of fieldNodes) {
          const fieldNameToken = asTokens(f.children['Identifier'])[0];
          const fieldTypeNode = asCstNodes(f.children['typeRef'])[0];
          const fieldTypeToken = fieldTypeNode
            ? asTokens(fieldTypeNode.children['Identifier'])[0]
            : undefined;
          if (!fieldNameToken || !fieldTypeToken) continue;
          const offset = parseFieldOffset(f);
          fields.push({
            name: fieldNameToken.image,
            type: fieldTypeToken.image,
            offset,
          });
        }
      }
      result.push({ name: nameToken.image, type: typeName, fields });
    }
  }
  return result;
}

/**
 * Pull the bit offset out of a fieldDecl CST node. Returns the parsed
 * numeric value (handling both decimal and hex literals) or `undefined`
 * when the field had no `@ N` clause.
 */
function parseFieldOffset(fieldDecl: CstNode): number | undefined {
  const decTokens = asTokens(fieldDecl.children['DecimalLiteral']);
  const hexTokens = asTokens(fieldDecl.children['HexLiteral']);
  const literal = decTokens[0] ?? hexTokens[0];
  if (!literal) return undefined;
  return parseNumericLiteral(literal.image);
}

function parseNumericLiteral(image: string): number {
  if (image.startsWith('0x') || image.startsWith('0X')) {
    return parseInt(image.slice(2), 16);
  }
  return parseInt(image, 10);
}

/**
 * Walk a `file` CST and return every union declaration as a plain data
 * object, with its arms in source order. Unions without a body are
 * reported with an empty arms list.
 */
export function getUnions(file: CstNode): UnionInfo[] {
  const result: UnionInfo[] = [];
  const declNodes = asCstNodes(file.children['declaration']);
  for (const decl of declNodes) {
    const unionDecls = asCstNodes(decl.children['unionDecl']);
    for (const unionDecl of unionDecls) {
      const nameToken = asTokens(unionDecl.children['Identifier'])[0];
      if (!nameToken) continue;
      const body = asCstNodes(unionDecl.children['unionBody'])[0];
      const arms = body ? extractNamedFields(body) : [];
      result.push({ name: nameToken.image, arms });
    }
  }
  return result;
}

/**
 * Shared helper: pull a list of `Identifier ':' typeRef` entries out of a
 * bundle or union body CST node. Fields are returned in source order.
 */
function extractNamedFields(body: CstNode): { name: string; type: string }[] {
  const result: { name: string; type: string }[] = [];
  const fieldNodes = asCstNodes(body.children['namedField']);
  for (const f of fieldNodes) {
    const fieldName = asTokens(f.children['Identifier'])[0]?.image;
    const typeNode = asCstNodes(f.children['typeRef'])[0];
    const typeName = typeNode
      ? asTokens(typeNode.children['Identifier'])[0]?.image
      : undefined;
    if (fieldName && typeName) {
      result.push({ name: fieldName, type: typeName });
    }
  }
  return result;
}

// ---- CST-element narrowing helpers ---------------------------------------
// CstElement is `CstNode | IToken`; the two are distinguished by whether
// the value has a `children` field. These helpers pick one or the other
// out of a CstElement[] array for safe iteration.

function isCstNode(x: CstElement): x is CstNode {
  return (x as CstNode).children !== undefined;
}

function asCstNodes(children: CstElement[] | undefined): CstNode[] {
  if (!children) return [];
  return children.filter(isCstNode);
}

function asTokens(children: CstElement[] | undefined): IToken[] {
  if (!children) return [];
  return children.filter((x): x is IToken => !isCstNode(x));
}
