import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const nop: InstructionInfo = {
  identifier: 'nop',
  name: 'No Operation',
  description: 'The CPU performs no operation during this machine cycle.',
  modifies: [],
  forms: [
    {
      opcode: [Opcodes.NOP],
      operands: [],
      operation: [],
      cycles: 4,
    },
  ],
};
