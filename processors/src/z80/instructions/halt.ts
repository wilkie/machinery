import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const halt: InstructionInfo = {
  identifier: 'halt',
  name: 'Halt',
  description:
    'Suspends CPU operation until an interrupt or reset is received. The processor continues to refresh DRAM.',
  modifies: [],
  forms: [
    {
      opcode: [Opcodes.HALT],
      operands: [],
      operation: ['HALTED = 1'],
      cycles: 4,
    },
  ],
};
