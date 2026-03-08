import type { InstructionInfo } from '@rawrs/architecture/Target';

import { Opcodes } from '../opcodes';

export const das: InstructionInfo = {
  identifier: 'das',
  name: 'Decimal Adjust AL After Subtraction',
  description:
    '`DAS` should be executed only after a subtraction instruction which leaves a two-BCD-digit byte result in the `AL` register. The operands should consist of two packed BCD digits. In this case, the `DAS` instruction will adjust `AL` to contain the correct packed two-digit decimal result.\n\nThe precise definition of `DAS` is as follows:\n\n1. If the lower four bits of `AL` are greater than 9, or if the auxiliary carry flag is 1, then decrement `AL` by 6, and set the auxiliary carry flag. Otherwise, reset the auxiliary carry flag.\n2. If `AL` is now greater than 0x9F, or if the carry flag is set, then decrement `AL` by 0x60, and set the carry flag. Otherwise, clear the carry flag.',
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
    // 0x2F - DAS
    {
      opcode: [Opcodes.DAS],
      operation: [
        // Compute the AF flag from the last ALU op
        '${RESOLVE_FLAGS}',
        // Subtract 6 from AL if AF is set or AL > 9
        'tmp_a = (AF > 0 || (AL & 0xf) > 9) ? 0x1 : 0x0',
        // This is the new AF flag
        'AF = tmp_a',
        // Subtract 0x60 from AL if CF is set or old AL > 0x99 (accounts for the aux-carry of AL > 9)
        'tmp_b = (AL > 0x99 || CF > 0) ? 0x1 : 0x0',
        // This is the new CF flag
        'CARRY = tmp_b',
        // Perform alterations
        'AL = tmp_a != 0 ? AL - 6 : AL',
        'AL = tmp_b != 0 ? AL - 0x60 : AL',
        'alu_result = AL',
        // Reset flag operation
        'flag_op = ${FLAG_OP_NOCF} | ${FLAG_OP_NOAF}',
      ],
      cycles: 3,
    },
  ],
};
