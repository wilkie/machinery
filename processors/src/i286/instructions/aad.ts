import type { InstructionInfo } from '@machinery/core';

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
      operands: ['imm'],
      operation: [
        'a = AL',
        'b = (AH * imm):u8',
        'AL = AL + (AH * imm)',
        'AH = 0',
        'alu_result = AL',
        // CF = carry from 8-bit addition; OF = CF on 286
        'CARRY = (a + b) > 0xFF ? 1 : 0',
        'CF = CARRY',
        'OF = CARRY',
        'flag_op = ${FLAG_OP_ALU} | ${FLAG_OP_8BIT} | ${FLAG_OP_NOCF} | ${FLAG_OP_NOOF}',
      ],
      cycles: 14,
    },
    {
      // No-operand form: AAD with implicit base 10
      opcode: [Opcodes.AAD, 0x0a],
      operands: [],
      operation: [
        'a = AL',
        'b = (AH * 10):u8',
        'AL = AL + (AH * 10)',
        'AH = 0',
        'alu_result = AL',
        'CARRY = (a + b) > 0xFF ? 1 : 0',
        'CF = CARRY',
        'OF = CARRY',
        'flag_op = ${FLAG_OP_ALU} | ${FLAG_OP_8BIT} | ${FLAG_OP_NOCF} | ${FLAG_OP_NOOF}',
      ],
      cycles: 14,
    },
  ],
};
