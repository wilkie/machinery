import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const div: InstructionInfo = {
  identifier: 'div',
  name: 'Unsigned Division',
  description:
    '`DIV` performs an unsigned divide. The dividend is implicit; only the divisor is given as an operand. If the source operand is a BYTE operand, divide `AX` by the byte. The quotient is stored in `AL`, and the remainder is stored in `AH`. If the source operand is a WORD operand, divide `DX:AX` by the word. The high-order 16 bits of the dividend are kept in `DX`. The quotient is stored in `AX`, and the remainder is stored in `DX`. Non-integral quotients are truncated towards 0. The remainder is always less than the dividend.',
  modifies: [],
  undefined: ['OF', 'CF', 'SF', 'ZF', 'AF', 'PF'],
  macros: {
    ALU8_OP: [
      '#0 if b == 0',
      'alu_result = a // b',
      'flag_op = ${FLAG_OP_ALU} | ${FLAG_OP_16BIT}',
      '#0 if alu_result:i8 < -128 || alu_result:i8 > 127',
    ],
    ALU16_OP: [
      '#0 if b == 0',
      'alu_result = div_a // b',
      'flag_op = ${FLAG_OP_ALU} | ${FLAG_OP_16BIT}',
      '#0 if alu_result:i16 < -32768 || alu_result:i16 > 32767',
    ],
  },
  locals: [
    {
      identifier: 'effective_address',
      name: 'Effective Address',
      size: 32,
    },
  ],
  forms: [
    // 0xF6 /6 - DIV eb
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + %{DISP}',
        'a = AX',
        'b = RAM:u8[effective_address]',
        '${ALU8_OP}',
        'AL = alu_result',
        'AH = a % b',
      ],
      opcode: [Opcodes.ALU_LOGIC_EB, 'ModRM_110_110_00', 'DISP_i16'],
      operandSize: 8,
      cycles: 20,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET}',
        'a = AX',
        'b = RAM:u8[effective_address]',
        '${ALU8_OP}',
        'AL = alu_result',
        'AH = a % b',
      ],
      opcode: [Opcodes.ALU_LOGIC_EB, 'ModRM_rm_110_00'],
      operandSize: 8,
      cycles: 20,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'a = AX',
        'b = RAM:u8[effective_address]',
        '${ALU8_OP}',
        'AL = alu_result',
        'AH = a % b',
      ],
      opcode: [Opcodes.ALU_LOGIC_EB, 'ModRM_rm_110_01', 'DISP_i8'],
      operandSize: 8,
      cycles: 20,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'a = AX',
        'b = RAM:u8[effective_address]',
        '${ALU8_OP}',
        'AL = alu_result',
        'AH = a % b',
      ],
      opcode: [Opcodes.ALU_LOGIC_EB, 'ModRM_rm_110_10', 'DISP_i16'],
      operandSize: 8,
      cycles: 20,
    },
    {
      operation: [
        'a = AX',
        'b = ${MOD_RM_RM8}',
        '${ALU8_OP}',
        'AL = alu_result',
        'AH = a % b',
      ],
      opcode: [Opcodes.ALU_LOGIC_EB, 'ModRM_rm_110_11'],
      operandSize: 8,
      cycles: 17,
    },
    // 0xF7 /6 - DIV ew
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + %{DISP}',
        'div_a = (DX << 16) | AX',
        'b = RAM:u16[effective_address]',
        '${ALU16_OP}',
        'AX = alu_result',
        'DX = div_a % b',
      ],
      opcode: [Opcodes.ALU_LOGIC_EW, 'ModRM_110_110_00', 'DISP_i16'],
      operandSize: 16,
      cycles: 28,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET}',
        'div_a = (DX << 16) | AX',
        'b = RAM:u16[effective_address]',
        '${ALU16_OP}',
        'AX = alu_result',
        'DX = div_a % b',
      ],
      opcode: [Opcodes.ALU_LOGIC_EW, 'ModRM_rm_110_00'],
      operandSize: 16,
      cycles: 28,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'div_a = (DX << 16) | AX',
        'b = RAM:u16[effective_address]',
        '${ALU16_OP}',
        'AX = alu_result',
        'DX = div_a % b',
      ],
      opcode: [Opcodes.ALU_LOGIC_EW, 'ModRM_rm_110_01', 'DISP_i8'],
      operandSize: 16,
      cycles: 28,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'div_a = (DX << 16) | AX',
        'b = RAM:u16[effective_address]',
        '${ALU16_OP}',
        'AX = alu_result',
        'DX = div_a % b',
      ],
      opcode: [Opcodes.ALU_LOGIC_EW, 'ModRM_rm_110_10', 'DISP_i16'],
      operandSize: 16,
      cycles: 28,
    },
    {
      operation: [
        'div_a = (DX << 16) | AX',
        'b = ${MOD_RM_RM16}',
        '${ALU16_OP}',
        'AX = alu_result',
        'DX = div_a % b',
      ],
      opcode: [Opcodes.ALU_LOGIC_EW, 'ModRM_rm_110_11'],
      operandSize: 16,
      cycles: 25,
    },
  ],
};
