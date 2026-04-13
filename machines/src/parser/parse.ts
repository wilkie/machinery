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
