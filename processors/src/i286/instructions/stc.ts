import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const stc: InstructionInfo = {
  identifier: 'stc',
  name: 'Set Carry Flag',
  description: 'The carry flag is set to 1.',
  modifies: ['CF'],
  undefined: [],
  forms: [
    // 0xF9 - STC
    {
      opcode: [Opcodes.STC],
      operation: ['CF = 1', 'CARRY = 1', 'flag_op = flag_op | ${FLAG_OP_NOCF}'],
      cycles: 2,
    },
  ],
};
