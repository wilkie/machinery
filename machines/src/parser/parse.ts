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
 * The skeleton parser only captures kind and name; richer metadata
 * (parameters, return types, bodies) requires real body grammars which
 * haven't landed yet.
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
