import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const jge: InstructionInfo = {
  identifier: 'jge',
  aliases: ['jnl'],
  name: 'Jump if Greater or Equal',
  description: 'Jump near if SF = OF',
  modifies: [],
  undefined: [],
  forms: [
    // 0x7d cb - JGE cb
    // 0x7d cb - JNL cb
    {
      operation: [
        '${RESOLVE_SF}',
        '${RESOLVE_OF}',
        'IP = (SF == OF) ? IP + %{IMM} : IP',
      ],
      opcode: [Opcodes.JGE, 'IMM_i8'],
      operandSize: 8,
      cycles: 3, // 7 if jumped
    },
  ],
};
