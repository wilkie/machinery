import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const jle: InstructionInfo = {
  identifier: 'jle',
  aliases: ['jng'],
  name: 'Jump if Less or Equal',
  description: 'Jump near if ZF = 1 or SF != OF',
  modifies: [],
  undefined: [],
  forms: [
    // 0x7e cb - JLE cb
    // 0x7e cb - JNG cb
    {
      operation: [
        '${RESOLVE_ZF}',
        '${RESOLVE_SF}',
        '${RESOLVE_OF}',
        'IP = (ZF == 1 || SF != OF) ? IP + %{IMM} : IP',
      ],
      opcode: [Opcodes.JLE, 'IMM_i8'],
      operands: ['rel'],
      operandSize: 8,
      cycles: 3, // 7 if jumped
    },
  ],
};
