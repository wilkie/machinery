import type { InstructionInfo } from '@rawrs/architecture/Target';

import { Opcodes } from '../opcodes';

export const clc: InstructionInfo = {
  identifier: 'clc',
  name: 'Clear Carry Flag',
  description:
    '`CLC` sets the carry flag to zero. No other flags or registers are affected.',
  modifies: ['CF'],
  undefined: [],
  forms: [
    // 0xF8 - CLC
    {
      opcode: [Opcodes.CLC],
      operation: ['CF = 0', 'CARRY = 0', 'flag_op = flag_op | ${FLAG_OP_NOCF}'],
      cycles: 2,
    },
  ],
};
