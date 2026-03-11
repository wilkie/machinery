import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const djnz: InstructionInfo = {
  identifier: 'djnz',
  name: 'Decrement and Jump if Not Zero',
  description:
    'Decrements B and jumps by a signed offset if B is not zero. Used as a loop counter.',
  modifies: [],
  forms: [
    {
      opcode: [Opcodes.DJNZ, 'REL_i8'],
      operands: ['rel'],
      addressing: 'relative',
      distance: 'short',
      operation: ['B = B - 1', 'if (B != 0) PC = PC + %{REL}'],
      cycles: 13,
    },
  ],
};
