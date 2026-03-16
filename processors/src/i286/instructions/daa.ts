import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const daa: InstructionInfo = {
  identifier: 'daa',
  name: 'Decimal Adjust AL After Addition',
  description:
    '`DAA` should be executed only after an `ADD` instruction which leaves a two-BCD-digit byte result in the `AL` register. The `ADD` operands should consist of two packed BCD digits. In this case, the `DAA` instruction will adjust `AL` to contain the correct two-digit packed decimal result.\n\nThe precise definition of `DAA` is as follows:\n\n1. If the lower 4 bits of `AL` are greater than nine, or if the auxiliary carry flag is 1, then increment `AL` by 6, and set the auxiliary carry flag. Otherwise, reset the auxiliary carry flag.\n2. If `AL` is now greater than 0x9F, or if the carry flag is set, then increment `AL` by 0x60, and set the carry flag. Otherwise, clear the carry flag.',
  modifies: ['SF', 'ZF', 'AF', 'PF', 'CF'],
  undefined: ['OF'],
  locals: [
    {
      identifier: 'tmp_a',
      size: 8,
    },
    {
      identifier: 'tmp_b',
      size: 8,
    },
  ],
  forms: [
    // 0x27 - DAA
    {
      opcode: [Opcodes.DAA],
      operands: [],
      operation: [
        // Compute the AF flag from the last ALU op
        '${RESOLVE_FLAGS}',
        // Add 6 to AL if AF is set or AL > 9
        'tmp_a = (AF > 0 || (AL & 0xf) > 9) ? 0x1 : 0x0',
        // This is the new AF flag
        'AF = tmp_a',
        // Add 0x60 to AL if CF is set or old AL > 0x99 (accounts for the aux-carry of AL > 9)
        'tmp_b = (AL > 0x99 || CF > 0) ? 0x1 : 0x0',
        // This is the new CF flag
        'CARRY = tmp_b',
        // Retain for OF flag (undefined, but still computed on real hardware)
        'a = AL',
        'b = (tmp_a != 0 ? 0x6 : 0x0) + (tmp_b != 0 ? 0x60 : 0x0)',
        // Perform alterations
        'alu_result = a + b',
        'AL = alu_result',
        // Reset flag operation
        'flag_op = ${FLAG_OP_NOCF} | ${FLAG_OP_NOAF}',
      ],
      cycles: 3,
    },
  ],
};
