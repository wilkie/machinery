import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

// TODO: Some kind of shut down or fatal error when SP is 1, 3, or 5?? (why not any value less than 16?)
// TODO: #13 for if SP is some odd number between 7 and 15.
export const pusha: InstructionInfo = {
  identifier: 'pusha',
  name: 'Push All General Registers',
  description:
    '`PUSHA` saves the registers, in this order, `AX`, `CX`, `DX`, `BX`, original `SP`, `BP`, `SI`, and `DI` on the stack. The stack pointer `SP` is decremented by 16 to hold the 8 word values. Since the registers are pushed onto the stack in the order in which they were given, they will appear in the 16 new stack bytes in the reverse order. The last register pushed is `DI`.',
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
    // 0x60 - PUSHA
    {
      opcode: [Opcodes.PUSHA],
      operands: [],
      modes: {
        real: {
          operation: [
            'offset = SP - 0x10',
            '#GP if offset == 0xffff',
            '#GP if (offset + 2) == 0xffff',
            '#GP if (offset + 4) == 0xffff',
            '#GP if (offset + 6) == 0xffff',
            '#GP if (offset + 8) == 0xffff',
            '#GP if (offset + 0xa) == 0xffff',
            '#GP if (offset + 0xc) == 0xffff',
            '#GP if (offset + 0xe) == 0xffff',
            'RAM:u16[SS_BASE + (SP - 0x02):u16] = AX',
            'RAM:u16[SS_BASE + (SP - 0x04):u16] = CX',
            'RAM:u16[SS_BASE + (SP - 0x06):u16] = DX',
            'RAM:u16[SS_BASE + (SP - 0x08):u16] = BX',
            'RAM:u16[SS_BASE + (SP - 0x0a):u16] = SP',
            'RAM:u16[SS_BASE + (SP - 0x0c):u16] = BP',
            'RAM:u16[SS_BASE + (SP - 0x0e):u16] = SI',
            'RAM:u16[SS_BASE + (SP - 0x10):u16] = DI',
            'SP = SP - 0x10',
          ],
        },
        protected: {
          operation: [
            'offset = SP - 0x10',
            '#GP if offset < SS_LIMIT_MIN',
            '#GP if (offset + 0x0f) > SS_LIMIT_MAX',
            'RAM:u16[SS_BASE + (SP - 0x02):u16] = AX',
            'RAM:u16[SS_BASE + (SP - 0x04):u16] = CX',
            'RAM:u16[SS_BASE + (SP - 0x06):u16] = DX',
            'RAM:u16[SS_BASE + (SP - 0x08):u16] = BX',
            'RAM:u16[SS_BASE + (SP - 0x0a):u16] = SP',
            'RAM:u16[SS_BASE + (SP - 0x0c):u16] = BP',
            'RAM:u16[SS_BASE + (SP - 0x0e):u16] = SI',
            'RAM:u16[SS_BASE + (SP - 0x10):u16] = DI',
            'SP = SP - 0x10',
          ],
        },
      },
      cycles: 17,
    },
  ],
};
