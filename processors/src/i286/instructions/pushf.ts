import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

// TODO: determine what the machine should do when SP <= 1 (286 documentation says 'shut down')
export const pushf: InstructionInfo = {
  identifier: 'pushf',
  name: 'Push Flags Register onto the Stack',
  description:
    'The stack pointer `SP` is decremented by 2, and the `FLAGS` register is copied to the new top of stack, which is pointed to by `SS:SP`. The flags, from the top bit (15) to the bottom bit (0), are as follows: undefined, nested task, I/O privilege level (2 bits), overflow, direction, interrupts enabled, trap, sign, zero, undefined, auxiliary carry, undefined, parity, undefined, and carry.',
  modifies: [],
  undefined: [],
  locals: [
    {
      identifier: 'stack_address',
      name: 'Effective Stack Address',
      size: 32,
    },
    {
      identifier: 'tmp',
      name: 'Temporary stack pointer',
      size: 16,
    },
  ],
  forms: [
    // 0x9C - PUSHF
    {
      opcode: [Opcodes.PUSHF],
      operands: [],
      operation: [
        '${RESOLVE_FLAGS}',
        'tmp = SP - 2',
        'stack_address = SS_BASE + tmp',
        // FLAGS bit 1 is always set
        'RAM:u16[stack_address] = FLAGS | 0b10',
        'SP = tmp',
      ],
      cycles: 3,
    },
  ],
};
