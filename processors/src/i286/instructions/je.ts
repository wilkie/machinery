import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const je: InstructionInfo = {
  identifier: 'je',
  aliases: ['jz'],
  name: 'Jump if Equal',
  description: 'Jump near if ZF = 1',
  modifies: [],
  undefined: [],
  forms: [
    // 0x74 cb - JE cb
    // 0x74 cb - JZ cb
    {
      operation: ['${RESOLVE_ZF}', 'IP = (ZF == 1) ? IP + %{IMM} : IP'],
      opcode: [Opcodes.JE, 'IMM_i8'],
      operands: ['rel'],
      operandSize: 8,
      cycles: 3, // 7 if jumped
    },
  ],
};
