import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const std: InstructionInfo = {
  identifier: 'std',
  name: 'Set Direction Flag',
  description:
    'The direction flag is set to 1. This causes all subsequent string operations to decrement the index registers (`SI` and/or `DI`) on which they operate.',
  modifies: ['DF'],
  undefined: [],
  forms: [
    // 0xFD - STD
    {
      opcode: [Opcodes.STD],
      operands: [],
      operation: ['DF = 1'],
      cycles: 2,
    },
  ],
};
