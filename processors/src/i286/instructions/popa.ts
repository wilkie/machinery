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
    {
      identifier: 'offset',
      name: 'Effective Offset',
      size: 16,
    },
  ],
  forms: [
    // 0x61 - POPA
    {
      opcode: [Opcodes.POPA],
      operands: [],
      modes: {
        real: {
          operation: [
            'offset = SP',
            '#GP if offset == 0xffff',
            '#GP if (offset + 2) == 0xffff',
            '#GP if (offset + 4) == 0xffff',
            '#GP if (offset + 6) == 0xffff',
            '#GP if (offset + 8) == 0xffff',
            '#GP if (offset + 0xa) == 0xffff',
            '#GP if (offset + 0xc) == 0xffff',
            '#GP if (offset + 0xe) == 0xffff',
            'DI = RAM:u16[SS_BASE + (SP):u16]',
            'SI = RAM:u16[SS_BASE + (SP + 0x02):u16]',
            'BP = RAM:u16[SS_BASE + (SP + 0x04):u16]',
            'BX = RAM:u16[SS_BASE + (SP + 0x08):u16]',
            'DX = RAM:u16[SS_BASE + (SP + 0x0a):u16]',
            'CX = RAM:u16[SS_BASE + (SP + 0x0c):u16]',
            'AX = RAM:u16[SS_BASE + (SP + 0x0e):u16]',
            'SP = SP + 0x10',
          ],
        },
        protected: {
          operation: [
            'offset = SP',
            '#GP if offset < SS_LIMIT_MIN',
            '#GP if (offset + 0x0f) > SS_LIMIT_MAX',
            'DI = RAM:u16[SS_BASE + (SP):u16]',
            'SI = RAM:u16[SS_BASE + (SP + 0x02):u16]',
            'BP = RAM:u16[SS_BASE + (SP + 0x04):u16]',
            'BX = RAM:u16[SS_BASE + (SP + 0x08):u16]',
            'DX = RAM:u16[SS_BASE + (SP + 0x0a):u16]',
            'CX = RAM:u16[SS_BASE + (SP + 0x0c):u16]',
            'AX = RAM:u16[SS_BASE + (SP + 0x0e):u16]',
            'SP = SP + 0x10',
          ],
        },
      },
      cycles: 19,
    },
  ],
};
