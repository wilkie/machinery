import type { Target } from '@machinery/core';

export interface Syntax {
  name: string;
  operandOrder: 'dest-first' | 'source-first';
  registerFormat(name: string): string;
  memoryFormat: { brackets: [string, string]; sizePrefix: boolean };
  generateTokenizer(target: Target): string;
  generateGrammarRules(target: Target): string;
}
