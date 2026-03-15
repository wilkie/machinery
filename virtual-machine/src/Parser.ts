import type { MacrosInfo } from '@machinery/core';
import nearley from 'nearley';

import { StatementNode } from './ast';
import grammar from './grammar';
import type { Token } from './Token';
import Tokenizer from './Tokenizer';
import type { LocalsInfo } from './types';

class ParseError extends Error {
  /** The source code that failed to parse */
  readonly code: string;
  /** The token stream produced by the tokenizer */
  readonly tokens: Token[];
  /** The token where the error occurred (if available) */
  readonly token?: Token;
  /** Index into the token array where the error occurred */
  readonly tokenIndex?: number;

  constructor(
    message: string,
    code: string,
    tokens: Token[],
    token?: Token,
    tokenIndex?: number,
  ) {
    super(message);
    this.name = 'ParseError';
    this.code = code;
    this.tokens = tokens;
    this.token = token;
    this.tokenIndex = tokenIndex;
  }
}

class Parser {
  readonly tokenizer: Tokenizer;

  constructor(macros?: MacrosInfo, locals?: LocalsInfo) {
    this.tokenizer = new Tokenizer(macros, locals);
  }

  parse(code: string, macros?: MacrosInfo, locals?: LocalsInfo): StatementNode {
    const parser = new nearley.Parser(nearley.Grammar.fromCompiled(grammar));
    const tokens = this.tokenizer.tokenize(code, macros, locals);

    try {
      // @ts-expect-error TS2345 - The type definitions expect a string, but we're totally
      // able to pass an array of tokens
      parser.feed(tokens);
    } catch (e) {
      throw this.formatError(e as Error, code, tokens);
    }

    if (parser.results.length === 0) {
      throw new ParseError(
        this.unexpectedEndMessage(code, tokens),
        code,
        tokens,
      );
    }

    if (parser.results.length > 1) {
      console.warn(
        `Ambiguous parse: ${parser.results.length} results for: ${code}`,
      );
    }

    return parser.results[0];
  }

  private formatError(err: Error, code: string, tokens: Token[]): ParseError {
    // Nearley format: "Syntax error at index N\nUnexpected {"type":"...","value":"..."}"
    const indexMatch = err.message.match(/at index (\d+)/);
    const tokenIndex = indexMatch ? parseInt(indexMatch[1]) : undefined;
    const failToken =
      tokenIndex !== undefined ? tokens[tokenIndex] : undefined;

    // Build a context window around the failure point
    const contextTokens = this.tokenContext(tokens, tokenIndex);

    let message = 'Parse error';
    if (failToken) {
      message += `: unexpected ${failToken.type} '${failToken.value}'`;
    }
    message += `\n  code: ${code}`;
    if (contextTokens) {
      message += `\n  near: ${contextTokens}`;
    }

    // Extract expected token types from nearley's message
    const expectedTypes = this.extractExpected(err.message);
    if (expectedTypes.length > 0) {
      message += `\n  expected: ${expectedTypes.join(', ')}`;
    }

    return new ParseError(message, code, tokens, failToken, tokenIndex);
  }

  /** Human-readable labels for token types shown in expected lists */
  private static readonly TOKEN_LABELS: Record<string, string> = {
    identifier: 'identifier',
    number: 'number',
    system: 'system identifier (@...)',
    assignment: '=',
    comparison: 'comparison (==, !=, <, >, <=, >=)',
    terminator: ';',
    left_paren: '(',
    right_paren: ')',
    op_add: '+/-',
    op_mul: '*//',
    op_and: '&',
    op_or: '|',
    op_xor: '^',
    op_shift: '<</>>/>>>',
    op_rotate: 'rotate (~>[n]/<~[n])',
    unary_operator: '~',
    unary_logic_operator: '!',
    logical_operator: '&&/||',
    if: "'if'",
    end: "'end'",
    loop: "'loop'",
    repeat: "'repeat'",
    next: "'next'",
    raise: '#',
    ternary_if: '?',
    ternary_else: ':',
    dot: '.',
    comment: 'comment (;;)',
    local: 'local (%{...})',
    macro_start: 'macro (${...})',
  };

  /** Token types to hide from expected lists (internal grammar plumbing) */
  private static readonly HIDDEN_TYPES = new Set([
    'macro_end', 'list_start', 'list_end', 'list_delimiter',
    'ternary_else',
  ]);

  private extractExpected(nearleyMessage: string): string[] {
    // Nearley lists expected tokens as: "A token matching x=>x.type==="foo""
    const matches = nearleyMessage.matchAll(/x\.type\s*===\s*"([^"]+)"/g);
    const types = new Set<string>();
    for (const m of matches) {
      if (!Parser.HIDDEN_TYPES.has(m[1])) {
        types.add(Parser.TOKEN_LABELS[m[1]] || m[1]);
      }
    }
    return [...types];
  }

  private unexpectedEndMessage(code: string, tokens: Token[]): string {
    const last = tokens[tokens.length - 1];
    let message = `Parse error: unexpected end of input`;
    message += `\n  code: ${code}`;
    if (last) {
      message += `\n  last token: ${last.type} '${last.value}'`;
    }
    message += `\n  (missing 'end if', 'repeat', or terminator?)`;
    return message;
  }

  private tokenContext(
    tokens: Token[],
    index?: number,
  ): string | undefined {
    if (index === undefined || tokens.length === 0) return undefined;
    const start = Math.max(0, index - 3);
    const end = Math.min(tokens.length, index + 4);
    return tokens
      .slice(start, end)
      .map((t, i) => {
        const repr = `${t.value}`;
        return i + start === index ? `>>>${repr}<<<` : repr;
      })
      .join(' ');
  }
}

export { ParseError };
export default Parser;
