import type { MacrosInfo } from '@machinery/core';
import moo from 'moo';

import type { Token } from './Token';
import type { LocalsInfo } from './types';

/**
 * This tokenizes a target macro or operation.
 */
class Tokenizer {
  /** The tokenizer internal processor */
  protected lexer: moo.Lexer;
  /** The set of macros that are expanded when found */
  protected macros: MacrosInfo;
  /** A set of locals to use by default */
  protected locals: LocalsInfo;

  constructor(macros?: MacrosInfo, locals?: LocalsInfo) {
    this.macros = macros || {};
    this.locals = locals || {};

    this.lexer = moo.compile({
      // if
      if: /if/,
      // next
      next: /next/,
      // loop
      loop: /loop/,
      // repeat
      repeat: /repeat/,
      // end (end if, etc)
      end: /end/,
      // . (as a delimiter between identifiers: foo.bar)
      dot: /\./,
      // A list is an array indexer generally found within a macro expansion. (${[FOO,BAR][RM]})
      list_start: /\[/,
      list_end: /\](?::(?:u|i)\d+)?/,
      list_delimiter: /,/,
      // Starts a comment (until a terminator)
      comment: {
        match: /;;.+?\s;\s/,
        value: (data) => data.substring(2, data.length - 3),
      },
      // Ends a statement
      terminator: /;/,
      // A macro starts with a dollar sign and is encapsulated in curly braces
      // ex: '${RESOLVE_CF}'
      macro: {
        match: /\$\{[a-zA-Z_][a-zA-Z0-9_]*(?::(?:u|i)\d+)?\}/,
        value: (data) => data.substring(2, data.length - 1),
      },
      // More complicated macros (like choice macros) are going to be handled by the parser
      macro_start: /\$\{/,
      macro_end: /\}/,
      // A local is a variable that needs to exist just in the context of the operation
      // ex: %{imm}
      local: {
        match: /%\{[a-zA-Z_][a-zA-Z0-9_]*\}/,
        value: (data) => data.substring(2, data.length - 1),
      },
      // Any constant value that represents an integer number
      // ex: '-42', '0xface'
      number: [
        /[+-]?0b[0-1]+(?::(?:u|i)\d+)?/,
        /[+-]?0x[0-9a-fA-F]+(?::(?:u|i)\d+)?/,
        /[+-]?[0-9]+(?::(?:u|i)\d+)?/,
      ],
      // A system identifier is something specifically named that starts with an at (@)
      system: /@[a-zA-Z_][a-zA-Z0-9_]*'?(?::(?:u|i)\d+)?/,
      // An identifier is something specifically named, like a register or local value
      // ex: 'AX'
      identifier: /[a-zA-Z_][a-zA-Z0-9_]*'?(?::(?:u|i)\d+)?/,
      // Operators that may appear as symbols
      logical_operator: /&&|[|][|]/,
      op_or: /\|/,
      op_xor: /\^/,
      op_and: /&/,
      op_shift: />>>|>>|<</,
      op_add: /[+-]/,
      op_mul: /\/\/|[*/%]/,
      op_rotate: /~>\[\d+\]|<~\[\d+\]/,
      // An assignment operator which will write a value to the left-hand side
      comparison: /==|!=|>=|<=|>|</,
      assignment: /=/,
      unary_operator: /[~]/,
      unary_logic_operator: /[!]/,
      ternary_if: /\?/,
      ternary_else: /\s:/,
      raise: /#/,
      // Parentheses
      left_paren: /\(/,
      right_paren: /\)/,
      // And then whitespace, which we will more or less ignore
      space: {
        match: /\s+/,
        lineBreaks: true,
      },
    });
  }

  tokenize(code: string, macros?: MacrosInfo, locals?: LocalsInfo): Token[] {
    this.lexer.reset(code);

    return Array.from(this.lexer)
      .filter((token) => token.type !== 'space')
      .map((token) => {
        if (token.type === 'macro') {
          const coercion = token.value.includes(':')
            ? token.value.split(':')[1]
            : undefined;
          token.value = coercion ? token.value.split(':')[0] : token.value;

          let macro = macros?.[token.value] || this.macros[token.value];

          if (macro === undefined) {
            // Error! Macro not found
            throw new Error(`Macro ${token.value} not found`);
          }

          if (Array.isArray(macro) && coercion) {
            // You cannot apply a type coercion to a multi-line macro
            throw new Error(
              `Macro ${token.value} is a multi-line operation and cannot have '${coercion}' applied.`,
            );
          }

          if (coercion && typeof macro === 'string') {
            if (macro.endsWith('}')) {
              macro = `${macro.slice(0, macro.length - 1)}:${coercion}}`;
            } else {
              macro = `${macro}:${coercion}`;
            }
          }

          let ret: Token[] = [];
          const operations = Array.isArray(macro)
            ? (macro as unknown[]).flat(Infinity)
            : [macro];
          for (const operation of operations) {
            ret = ret.concat(
              this.tokenize(
                (operation as string) + (Array.isArray(macro) ? ' ; ' : ''),
                macros,
                locals,
              ),
            );
          }
          return ret;
        } else if (token.type === 'local' || token.type === 'identifier') {
          const local =
            locals?.[token.value]?.identifier ||
            this.locals[token.value]?.identifier;

          if (local === undefined) {
            // Error! Local not found
            //throw new Error(`Local ${token.value} not found`);
          }

          if (typeof local === 'number') {
            return {
              type: 'number',
              value: local,
            } as Token;
          }
        }

        const coercion =
          token.type !== 'comment' && token.value.includes(':')
            ? token.value.split(':')[1]
            : undefined;
        token.value = coercion ? token.value.split(':')[0] : token.value;

        return {
          type: token.type,
          value:
            token.type === 'number'
              ? token.value.startsWith('0b')
                ? parseInt(token.value.substring(2), 2)
                : parseInt(token.value)
              : token.value,
          coercion,
        } as Token;
      })
      .flat();
  }
}

export default Tokenizer;
