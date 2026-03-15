import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const popf: InstructionInfo = {
  identifier: 'popf',
  name: 'Pop from Stack into the Flags Register',
  description:
    'The top of the stack, pointed to by `SS:SP`, is copied into the `FLAGS` register. The stack pointer `SP` is incremented by 2 to point to the new top of stack. The flags, from the top bit (bit 15) to the bottom (bit 0), are as follows: undefined, nested task, I/O privilege level (2 bits), overflow, direction, interrupts enabled, trap, sign, zero, undefined, auxiliary carry, undefined, parity, undefined, and carry.\n\nThe I/O privilege level will be altered only when executing at privilege level 0. The interrupt enable flag will be altered only when executing at a level at least as privileged as the I/O privilege level. If you execute a `POPF` instruction with insufficient privilege, there will be no exception nor will the privileged bits be changed.',
  modifies: ['OF', 'DF', 'IF', 'TF', 'SF', 'ZF', 'AF', 'PF', 'CF'],
  undefined: [],
  macros: {
    // Mask for flags that are always changeable (CF, PF, AF, ZF, SF, TF, DF, OF, NT)
    // Excludes IF (bit 9) and IOPL (bits 12-13) which are privilege-gated
    POPF_BASE_MASK: 0b100111111010101,
  },
  locals: [
    {
      identifier: 'stack_address',
      name: 'Effective Stack Address',
      size: 32,
    },
    {
      identifier: 'offset',
      name: 'Effective Offset',
      size: 32,
    },
    {
      identifier: 'value',
      name: 'Resulting Value',
      size: 16,
    },
    {
      identifier: 'mask',
      name: 'Changeable Flags Mask',
      size: 16,
    },
  ],
  forms: [
    // 0x9D - POPF
    {
      opcode: [Opcodes.POPF],
      operands: [],
      modes: {
        real: {
          operation: [
            'offset = SP',
            'stack_address = SS_BASE + offset',
            '#GP if offset == 0xffff',
            'value = RAM:u16[stack_address]',
            'SP = SP + 2',
            // FLAGS bit 1 is always set
            'FLAGS = ((FLAGS & ~0b11111111010101) | (value & 0b111111010101) | 0b10) & 0xffff',
            'flag_op = ${FLAG_OP_RESOLVED}',
            'CARRY = CF',
          ],
        },
        protected: {
          operation: [
            'offset = SP',
            'stack_address = SS_BASE + offset',
            '#GP if (offset + 1) < SS_LIMIT_MIN',
            '#GP if (offset + 1) > SS_LIMIT_MAX',
            'value = RAM:u16[stack_address]',
            'SP = SP + 2',
            ';; start with base mask (no IF, no IOPL)',
            'mask = ${POPF_BASE_MASK}',
            ';; IF changeable if CPL <= IOPL',
            'if CS.RPL <= IOPL',
            ['mask = mask | 0b1000000000'],
            'end if',
            ';; IOPL changeable only at CPL 0',
            'if CS.RPL == 0',
            ['mask = mask | 0b11000000000000'],
            'end if',
            'FLAGS = ((FLAGS & ~mask) | (value & mask) | 0b10) & 0xffff',
            'flag_op = ${FLAG_OP_RESOLVED}',
            'CARRY = CF',
          ],
        },
      },
      cycles: 5,
    },
  ],
};
