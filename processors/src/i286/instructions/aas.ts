import type { InstructionInfo } from '@rawrs/architecture/Target';

import { Opcodes } from '../opcodes';

export const aas: InstructionInfo = {
  identifier: 'aas',
  name: 'ASCII Adjust AL After Subtraction',
  description:
    'AAS should be executed only after a subtraction instruction which left the byte result in the `AL` register. The lower nibbles of the operands to the `SUB` instruction should have been in the range 0 through 9 (BCD digits). In this case, the `AAS` instruction will adjust `AL` to contain the correct decimal digit result. If the subtraction produced a decimal carry, the `AH` register is decremented, and the carry and auxiliary carry flags are set to 1. If there was no decimal carry, the carry and auxiliary carry flags are set to 0, and `AH` is unchanged. In any case, `AL` is left with its top nibble set to 0. To convert `AL` to an ASCII result, you can follow the `AAS` instruction with `OR AL, 30H`.\n\nThe precise definition of `AAS` is as follows: if the lower four bits of `AL` are greater than 9, or if the auxiliary carry flag is 1, then decrement `AL` by 6, `AH` by 1, and set the carry and auxiliary carry flags. Otherwise, reset the carry and auxiliary carry flags. In any case, conclude the `AAS` operation by setting the upper four bits of `AL` to zero.',
  modifies: ['AF', 'CF'],
  undefined: ['OF', 'SF', 'ZF', 'PF'],
  locals: [
    {
      identifier: 'tmp_a',
      size: 8,
    },
  ],
  forms: [
    {
      opcode: [Opcodes.AAS],
      operation: [
        // Compute the AF flag from the last ALU op
        '${RESOLVE_AF}',
        'tmp_a = (AF > 0 || (AL & 0xf) > 9) ? 0x1 : 0x0',
        // Subtract 1 from AH if AF is set or AL > 9
        'AH = tmp_a != 0 ? AH - 1 : AH',
        // Set flags
        'CARRY = tmp_a',
        'AF = tmp_a',
        // Subtract 6 from AL if AF is set or AL > 9
        'AL = (tmp_a != 0 ? (AL - 6) : AL) & 0xf',
        'alu_result = AL',
        // Reset flag operation
        'flag_op = ${FLAG_OP_NOCF} | ${FLAG_OP_NOAF}',
      ],
      cycles: 3,
    },
  ],
};
