import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

// TODO: #13 for operand of 0xffff
export const popf: InstructionInfo = {
  identifier: 'popf',
  name: 'Pop from Stack into the Flags Register',
  description:
    'The top of the stack, pointed to by `SS:SP`, is copied into the `FLAGS` register. The stack pointer `SP` is incremented by 2 to point to the new top of stack. The flags, from the top bit (bit 15) to the bottom (bit 0), are as follows: undefined, nested task, I/O privilege level (2 bits), overflow, direction, interrupts enabled, trap, sign, zero, undefined, auxiliary carry, undefined, parity, undefined, and carry.\n\nThe I/O privilege level will be altered only when executing at privilege level 0. The interrupt enable flag will be altered only when executing at a level at least as privileged as the I/O privilege level. If you execute a `POPF` instruction with insufficient privilege, there will be no exception nor will the privileged bits be changed.',
  modifies: ['OF', 'DF', 'IF', 'TF', 'SF', 'ZF', 'AF', 'PF', 'CF'],
  undefined: [],
  locals: [
    {
      identifier: 'stack_address',
      name: 'Effective Stack Address',
      size: 32,
    },
    {
      identifier: 'value',
      name: 'Resulting Value',
      size: 16,
    },
  ],
  forms: [
    // 0x9D - POPF
    {
      opcode: [Opcodes.POPF],
      operation: [
        'stack_address = SS_BASE + SP',
        'value = RAM:u16[stack_address]',
        'SP = SP + 2',
        'FLAGS = ((FLAGS & ~0b11111111010101) | (value & 0b111111010101) | 0b10) & 0xffff',
        'flag_op = ${FLAG_OP_RESOLVED}',
        'CARRY = CF',
      ],
      cycles: 5,
    },
  ],
};
