import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const wait: InstructionInfo = {
  identifier: 'wait',
  name: 'Wait Until Busy Pin is Inactive',
  description:
    '`WAIT` suspends execution of instructions until the `BUSY` pin is inactive (high). The `BUSY` pin is driven by the 80287 numeric processor extension. `WAIT` is issued to ensure that the numeric instruction being executed is complete. and to check for a possible numeric fault.',
  modifies: [],
  undefined: [],
  forms: [
    // 0x9B - WAIT
    {
      opcode: [Opcodes.WAIT],
      operands: [],
      operation: [
        // No-op
      ],
      cycles: 3,
    },
  ],
};
