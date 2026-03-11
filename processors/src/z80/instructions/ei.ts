import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const ei: InstructionInfo = {
  identifier: 'ei',
  name: 'Enable Interrupts',
  description:
    'Sets the interrupt enable flip-flops (IFF1 and IFF2) to enable maskable interrupts.',
  modifies: [],
  forms: [
    {
      opcode: [Opcodes.EI],
      operands: [],
      operation: ['IFF1 = 1', 'IFF2 = 1'],
      cycles: 4,
    },
  ],
};
