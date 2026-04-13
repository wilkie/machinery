// Top-level tokenization entry point.
//
// Combines the raw Chevrotain lexer with the indent post-processor to
// produce a token stream suitable for a parser that treats INDENT and
// OUTDENT as ordinary block delimiters.

import type { IToken, ILexingError } from 'chevrotain';
import { machineLexer } from './tokens.js';
import { injectIndentation } from './indent.js';

export interface LexResult {
  /** Tokens in program order, with INDENT/OUTDENT injected. */
  tokens: IToken[];
  /** Any lexer errors (malformed tokens, unrecognized characters). */
  errors: ILexingError[];
}

/**
 * Tokenize a `.machine` source string. On lexer error, the returned token
 * stream is the partial result produced before the error; callers should
 * check `errors.length === 0` before attempting to parse.
 */
export function lex(source: string): LexResult {
  const raw = machineLexer.tokenize(source);
  if (raw.errors.length > 0) {
    return { tokens: raw.tokens, errors: raw.errors };
  }
  const tokens = injectIndentation(raw.tokens);
  return { tokens, errors: [] };
}
