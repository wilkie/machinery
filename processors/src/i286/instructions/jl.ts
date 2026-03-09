import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const jl: InstructionInfo = {
  identifier: 'jl',
  aliases: ['jnge'],
  name: 'Jump if Less',
  description: 'Jump near if SF != OF',
  modifies: [],
  undefined: [],
  forms: [
    // 0x7c cb - JL cb
    // 0x7c cb - JNGE cb
    {
      operation: [
        '${RESOLVE_SF}',
        '${RESOLVE_OF}',
        'IP = (SF != OF) ? IP + %{IMM} : IP',
      ],
      opcode: [Opcodes.JL, 'IMM_i8'],
      operands: ['rel'],
      operandSize: 8,
      cycles: 3, // 7 if jumped
    },
  ],
};
