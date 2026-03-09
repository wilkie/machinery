import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const sti: InstructionInfo = {
  identifier: 'sti',
  name: 'Set Interrupt Enable Flag',
  description:
    'The interrupts-enabled flag is set to 1. The 80286 will now respond to external interrupts after executing the `STI` instruction.',
  modifies: ['IF'],
  undefined: [],
  forms: [
    // 0xFB - STI
    {
      opcode: [Opcodes.STI],
      operands: [],
      operation: ['${RESOLVE_FLAGS}', 'IF = 1'],
      cycles: 2,
    },
  ],
};
