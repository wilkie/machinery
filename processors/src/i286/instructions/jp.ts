import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const jp: InstructionInfo = {
  identifier: 'jp',
  aliases: ['jpe'],
  name: 'Jump if Parity',
  description: 'Jump near if PF = 1',
  modifies: [],
  undefined: [],
  forms: [
    // 0x7A cb - JP cb
    // 0x7A cb - JPE cb
    {
      operation: ['${RESOLVE_PF}', 'IP = (PF == 1) ? IP + %{IMM} : IP'],
      opcode: [Opcodes.JP, 'IMM_i8'],
      operands: ['rel'],
      operandSize: 8,
      cycles: 3, // 7 if jumped
    },
  ],
};
