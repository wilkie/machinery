import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const aaa: InstructionInfo = {
  identifier: 'aaa',
  name: 'ASCII Adjust AL After Addition',
  description:
    '`AAA` should be executed only after an `ADD` instruction which leaves a byte result in the `AL` register. The lower nibbles of the operands to the `ADD` instruction should be in the range 0 through 9 (BCD digits). In this case, the `AAA` instruction will adjust `AL` to contain the correct decimal digit result. If the addition produced a decimal carry, the `AH` register is incremented, and the carry and auxiliary carry flags are set to 1. If there was no decimal carry, the carry and auxiliary carry flags are set to 0, and `AH` is unchanged. In any case, `AL` is left with its top nibble set to 0. To convert `AL` to an ASCII result, you can follow the `AAA` instruction with `OR AL, 30H`.\n\nThe precise definition of `AAA` is as follows: if the lower 4 bits of AL are greater than nine, or if the auxiliary carry flag is 1, then increment `AL` by 6, `AH` by 1, and set the carry and auxiliary carry flags.  Otherwise, reset the carry and auxiliary carry flags. In any case, conclude the `AAA` operation by setting the upper four bits of AL to zero.',
  modifies: ['AF', 'CF'],
  undefined: ['OF', 'SF', 'ZF', 'PF'],
  locals: [
    {
      identifier: 'tmp_a',
      size: 8,
    },
  ],
  forms: [
    // 0x37 - AAA
    {
      opcode: [Opcodes.AAA],
      operation: [
        // Compute the AF flag from the last ALU op
        '${RESOLVE_AF}',
        'tmp_a = (AF > 0 || (AL & 0xf) > 9) ? 0x1 : 0x0',
        // Add 1 to AH if AF is set or AL > 9
        'AH = tmp_a == 1 ? AH + 1 : AH',
        // Set flags
        'CARRY = tmp_a',
        'AF = tmp_a',
        // Add 6 to AL if AF is set or AL > 9
        'AL = (tmp_a != 0 ? (AL + 6) : AL) & 0xf',
        'alu_result = AX',
        // Reset flag operation
        'flag_op = ${FLAG_OP_NOCF} | ${FLAG_OP_NOAF}',
      ],
      cycles: 3,
    },
  ],
};
