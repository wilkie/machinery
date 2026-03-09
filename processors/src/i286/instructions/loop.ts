import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const loop: InstructionInfo = {
  identifier: 'loop',
  name: 'Loop Control with CX Counter',
  description:
    '`LOOP` decrements `CX` and short near jumps if `CX` is not zero.\n\n`LOOP` first decrements the `CX` register without changing any of the flags. Then, conditions are checked for the form of `LOOP` being used. If the conditions are met, then an intra-segment jump is made. The destination to `LOOP` is in the range from 126 (decimal) bytes before the instruction to 127 bytes beyond the instruction.\n\nThe `LOOP` instructions are intended to provide iteration control and to combine loop index management with conditional branching. To use the `LOOP` instruction you load an unsigned iteration count into `CX`, then code the `LOOP` at the end of a series of instructions to be iterated. The destination of `LOOP` is a label that points to the beginning of the iteration.',
  modifies: [],
  undefined: [],
  forms: [
    // 0xE2 cb - LOOP cb
    {
      operation: ['CX = CX - 1', 'IP = (CX != 0) ? IP + %{IMM} : IP'],
      opcode: [Opcodes.LOOP, 'IMM_i8'],
      operands: ['rel'],
      operandSize: 16,
      cycles: 4, // 8 if jumped
    },
  ],
};
