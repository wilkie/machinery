import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const di: InstructionInfo = {
  identifier: 'di',
  name: 'Disable Interrupts',
  description:
    'Disables the maskable interrupt by resetting the interrupt enable flip-flops (IFF1 and IFF2).',
  modifies: [],
  forms: [
    {
      opcode: [Opcodes.DI],
      operands: [],
      operation: ['IFF1 = 0', 'IFF2 = 0'],
      cycles: 4,
    },
  ],
};
