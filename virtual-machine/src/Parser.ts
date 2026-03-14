import type { MacrosInfo } from '@machinery/core';
import nearley from 'nearley';

import { StatementNode } from './ast';
import grammar from './grammar';
import Tokenizer from './Tokenizer';
import type { LocalsInfo } from './types';

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
      console.log('offending code:', code);
      console.log('tokens:', JSON.stringify(tokens, null, 2));
      throw e;
    }

    return parser.results[0];
  }
}

export default Parser;
