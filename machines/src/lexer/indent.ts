// Indent post-processor.
//
// Chevrotain's built-in lexer doesn't know about indentation. This pass
// walks the raw token stream produced by `machineLexer.tokenize()` and
// injects synthetic INDENT/OUTDENT tokens at the start of each line whose
// indentation differs from the previous non-blank line.
//
// The injected stream lets the parser treat the .machine language as if
// it were brace-delimited: INDENT opens a block, OUTDENT closes one, and
// the grammar doesn't need to look at columns anywhere.

import { createTokenInstance, type IToken, type TokenType } from 'chevrotain';
import {
  Newline,
  Indent,
  Outdent,
  LBrace,
  RBrace,
  LParen,
  RParen,
  LBracket,
  RBracket,
} from './tokens.js';

const OPEN_BRACKETS = new Set<TokenType>([LBrace, LParen, LBracket]);
const CLOSE_BRACKETS = new Set<TokenType>([RBrace, RParen, RBracket]);

/**
 * Inject INDENT/OUTDENT tokens into a raw token stream.
 *
 * Rules:
 *
 *   * Indentation is the column of the first non-whitespace token on a
 *     line, measured as `startColumn - 1` (Chevrotain columns are 1-based).
 *   * A larger indent than the current stack top emits one INDENT; the new
 *     level is pushed onto the stack.
 *   * A smaller indent pops the stack and emits one OUTDENT per pop until
 *     the stack top is <= the new indent.
 *   * Newlines inside any bracket (`{`, `(`, `[`) are dropped entirely, so
 *     record literals and parenthesized expressions can span multiple lines
 *     without being treated as separate statements.
 *   * Blank lines and comment-only lines do not affect indentation — the
 *     raw lexer skips comments and horizontal whitespace, so the only
 *     token produced for such a line is a `Newline`, which leaves the
 *     indent stack alone.
 *   * At end-of-input, the indent stack is drained — one OUTDENT per
 *     pushed level — so blocks opened but not closed by a dedent get
 *     terminated cleanly.
 *
 * Mis-aligned indentation (one that doesn't match any prior level) is
 * tolerated at this layer: the processor stops popping when it would
 * cross the new indent. The parser will later see tokens at an unexpected
 * nesting depth and report a contextual error.
 */
export function injectIndentation(rawTokens: IToken[]): IToken[] {
  const result: IToken[] = [];
  const stack: number[] = [0];
  let bracketDepth = 0;
  let atLineStart = true;
  let lastToken: IToken | undefined;

  for (const tok of rawTokens) {
    if (tok.tokenType === Newline) {
      if (bracketDepth === 0) {
        result.push(tok);
        atLineStart = true;
      }
      // Inside brackets: drop the newline; line is a continuation.
      continue;
    }

    // Compute indent BEFORE updating bracket depth. This matters when
    // an open bracket like `(` or `{` is itself the first non-newline
    // token on a line — e.g. an empty-parens no-op `()` as a statement
    // body, or a record literal `{ ... }` standing alone. Updating
    // bracketDepth first would skip the indent check (which gates on
    // `bracketDepth === 0`) and silently lose the INDENT/OUTDENT that
    // the line's column otherwise implies.
    if (atLineStart && bracketDepth === 0) {
      const indent = (tok.startColumn ?? 1) - 1;
      const top = stack[stack.length - 1] ?? 0;

      if (indent > top) {
        stack.push(indent);
        result.push(synthesize(Indent, tok));
      } else if (indent < top) {
        while (stack.length > 1 && (stack[stack.length - 1] ?? 0) > indent) {
          stack.pop();
          result.push(synthesize(Outdent, tok));
        }
      }
      // indent === stack top: same level, no INDENT/OUTDENT.
    }

    // Update bracket depth AFTER the indent check so opening brackets
    // at line start are counted as real tokens for indentation, but
    // newlines inside the bracket region are still suppressed.
    if (OPEN_BRACKETS.has(tok.tokenType)) {
      bracketDepth++;
    } else if (CLOSE_BRACKETS.has(tok.tokenType)) {
      bracketDepth = Math.max(0, bracketDepth - 1);
    }

    // Any real token clears the "at line start" flag, even inside
    // brackets (where we skip the indent check entirely). Without this
    // reset, a closing bracket that drops bracketDepth back to 0 would
    // re-enter the indent check and emit spurious INDENT/OUTDENT tokens
    // against the bracket's own column.
    atLineStart = false;

    result.push(tok);
    lastToken = tok;
  }

  // Drain remaining indents at EOF so the parser sees a balanced stream.
  while (stack.length > 1) {
    stack.pop();
    result.push(synthesize(Outdent, lastToken));
  }

  return result;
}

/**
 * Construct a zero-width synthetic token at the position of `anchor`.
 * Used for INDENT/OUTDENT, which don't correspond to real source characters
 * but need valid location info so error messages can point at the right
 * place.
 */
function synthesize(tokenType: TokenType, anchor: IToken | undefined): IToken {
  const startOffset = anchor?.startOffset ?? 0;
  const startLine = anchor?.startLine ?? 1;
  const startColumn = anchor?.startColumn ?? 1;
  return createTokenInstance(
    tokenType,
    '',
    startOffset,
    startOffset,
    startLine,
    startLine,
    startColumn,
    startColumn,
  );
}
