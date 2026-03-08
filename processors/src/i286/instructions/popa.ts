import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

// TODO: #13 for offset of 0xffff (though not sure how... maybe if SP is 0xffff)
export const popa: InstructionInfo = {
  identifier: 'popa',
  name: 'Pop All General Registers',
  description:
    '`POPA` pops the eight general registers given in the description above, except that the `SP` value is discarded instead of loaded into `SP`. `POPA` reverses a previous `PUSHA`, restoring the general registers to their values before `PUSHA` was executed. The first register popped is `DI`.',
  modifies: [],
  undefined: [],
  locals: [
    {
      identifier: 'stack_address',
      name: 'Effective Stack Address',
      size: 32,
    },
  ],
  forms: [
    // 0x61 - POPA
    {
      opcode: [Opcodes.POPA],
      operation: [
        'SP = SP + 0x10',
        'stack_address = SS_BASE + SP',
        'AX = RAM:u16[stack_address - 0x02]',
        'CX = RAM:u16[stack_address - 0x04]',
        'DX = RAM:u16[stack_address - 0x06]',
        'BX = RAM:u16[stack_address - 0x08]',
        'BP = RAM:u16[stack_address - 0x0c]',
        'SI = RAM:u16[stack_address - 0x0e]',
        'DI = RAM:u16[stack_address - 0x10]',
      ],
      cycles: 19,
    },
  ],
};
