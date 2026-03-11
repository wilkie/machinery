import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const aam: InstructionInfo = {
  identifier: 'aam',
  name: 'ASCII Adjust AX After Multiply',
  description:
    '`AAM` should be used only after executing a `MUL` instruction between two unpacked BCD digits, leaving the result in the `AX` register. Since the result is less than one hundred, it is contained entirely in the `AL` register. `AAM` unpacks the `AL` result by dividing `AL` by ten, leaving the quotient (most significant digit) in `AH`, and the remainder (least significant digit) in `AL`.',
  modifies: ['SF', 'ZF', 'PF'],
  undefined: ['OF', 'AF', 'CF'],
  forms: [
    {
      opcode: [Opcodes.AAM, 'IMM_u8'],
      operands: ['imm'],
      operation: [
        'AH = AL // imm',
        'AL = AL % imm',
        'alu_result = AL',
        // Reset flag operation
        'flag_op = ${FLAG_OP_ALU} | ${FLAG_OP_8BIT}',
      ],
      cycles: 16,
    },
    {
      // No-operand form: AAM with implicit base 10
      opcode: [Opcodes.AAM, 0x0a],
      operands: [],
      operation: [
        'AH = AL // 10',
        'AL = AL % 10',
        'alu_result = AL',
        'flag_op = ${FLAG_OP_ALU} | ${FLAG_OP_8BIT}',
      ],
      cycles: 16,
    },
  ],
};
