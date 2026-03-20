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
        'if %{imm} == 0',
        [
          ';; Generally, the cpu flags are set assuming the ALU result is 0',
          ';; Yet, real hardware seems to stage the quotient result as the value of AL',
          ';; then it invokes the ALU non-restoring division for one step which computes a 0 in bit 0',
          ';; and assigns that 0 to the first bit of the result and pre-computes flags as it goes',
          ';; So the flags are actually reflecting the 16-bit zero-extended AL with bit 0 cleared',
          'flag_op = ${FLAG_OP_ALU} | ${FLAG_OP_16BIT} | ${FLAG_OP_LOGIC}',
          'alu_result = AL & 0xFE',
          '#0 if 1 == 1',
        ],
        'end if',
        'if %{imm} == 10',
        [
          'AX = ROM.AAM_0XA[AL]',
          'alu_result = AX',
          'flag_op = ${FLAG_OP_ALU} | ${FLAG_OP_8BIT} | ${FLAG_OP_LOGIC}',
        ],
        'end if',
        'if %{imm} != 10',
        [
          'flag_op = ${FLAG_OP_ALU} | ${FLAG_OP_8BIT} | ${FLAG_OP_LOGIC}',
          'AH = AL // imm',
          'AL = AL % imm',
          'alu_result = AL',
        ],
        'end if',
      ],
      cycles: 16,
    },
  ],
};
