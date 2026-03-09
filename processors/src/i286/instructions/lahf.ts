import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const lahf: InstructionInfo = {
  identifier: 'lahf',
  name: 'Load Flags into AH Register',
  description:
    'The low byte of the flags word is transferred to `AH`. The bits, from MSB to LSB, are as follows: sign, zero, indeterminate, auxiliary carry, indeterminate, parity, indeterminate, and carry.',
  modifies: [],
  undefined: [],
  forms: [
    // 0x9F - LAHF
    {
      opcode: [Opcodes.LAHF],
      operands: [],
      operation: ['${RESOLVE_FLAGS}', 'AH = FLAGS & 0xff'],
      cycles: 2,
    },
  ],
};
