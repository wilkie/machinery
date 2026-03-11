import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const ja: InstructionInfo = {
  identifier: 'ja',
  name: 'Jump if Above',
  description: 'Jump near if CF = 0 and ZF = 0',
  modifies: [],
  undefined: [],
  forms: [
    // 0x77 cb - JA cb
    {
      operation: [
        '${RESOLVE_CF}',
        '${RESOLVE_ZF}',
        'IP = (CARRY == 0 && ZF == 0) ? IP + %{IMM} : IP',
      ],
      opcode: [Opcodes.JA, 'IMM_i8'],
      operands: ['rel'],
      operandSize: 8,
      distance: 'short',
      addressing: 'relative',
      cycles: 3, // 7 if jumped
    },
  ],
};
