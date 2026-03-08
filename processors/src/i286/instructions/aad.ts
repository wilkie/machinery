import type { InstructionInfo } from '@rawrs/architecture/Target';

import { Opcodes } from '../opcodes';

export const aad: InstructionInfo = {
  identifier: 'aad',
  name: 'ASCII Adjust AX Before Division',
  description:
    '`AAD` is used to prepare two unpacked BCD digits (least significant in `AL`, most significant in `AH`) for a division operation which will yield an unpacked result. This is accomplished by setting `AL` to `AL + (10 x AH)`, and then setting `AH` to 0. This leaves `AX` equal to the binary equivalent of the original unpacked 2~digit number.',
  modifies: ['SF', 'ZF', 'PF'],
  undefined: ['OF', 'AF', 'CF'],
  forms: [
    {
      opcode: [Opcodes.AAD, 'IMM_u8'],
      operation: [
        'AL = AL + (AH * IMM)',
        'AH = 0',
        'alu_result = AX',
        // Reset flag operation
        'flag_op = ${FLAG_OP_ALU} | ${FLAG_OP_8BIT}',
      ],
      cycles: 14,
    },
  ],
};
