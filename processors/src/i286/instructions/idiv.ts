import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const idiv: InstructionInfo = {
  identifier: 'idiv',
  name: 'Signed Division',
  description:
    '`IDIV` performs a signed divide. The dividend is implicit; only the divisor is given as an operand. If the source operand is a BYTE operand, divide `AX` by the byte. The quotient is stored in `AL`, and the remainder is stored in `AH`. If the source operand is a WORD operand, divide `DX:AX` by the word. The high-order 16 bits of the dividend are in `DX`. The quotient is stored in `AX`, and the remainder is stored in `DX`. Non-integral quotients are truncated towards 0. The remainder has the same sign as the dividend and always has less magnitude than the dividend.',
  modifies: [],
  undefined: ['OF', 'CF', 'SF', 'ZF', 'AF', 'PF'],
  macros: {
    ALU8_OP: [
      '#0 if b == 0',
      'alu_result = a:i16 // b:i8',
      'AH = a:i16 % b:i8',
      'flag_op = ${FLAG_OP_ALU} | ${FLAG_OP_16BIT}',
      '#0 if alu_result:i32 > 127 || alu_result:i32 < -128',
    ],
    ALU16_OP: [
      '#0 if b == 0',
      'alu_result = div_a:i32 // b:i16',
      'DX = div_a:i32 % b:i16',
      'flag_op = ${FLAG_OP_ALU} | ${FLAG_OP_16BIT}',
      '#0 if alu_result:i32 > 32767 || alu_result:i32 < -32768',
    ],
  },
  locals: [
    {
      identifier: 'effective_address',
      name: 'Effective Address',
      size: 32,
    },
    {
      identifier: 'offset',
      name: 'Effective Offset',
      size: 32,
    },
  ],
  forms: [
    // 0xF6 /7 - IDIV eb
    {
      modes: {
        real: {
          operation: [
            'effective_address = ${MOD_RM_SEGMENT} + %{DISP}',
            'a = AX',
            'b = RAM:i8[effective_address]',
            '${ALU8_OP}',
            'AL = alu_result',
          ],
        },
        protected: {
          operation: [
            'offset = %{DISP:u16}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'a = AX',
            'b = RAM:i8[effective_address]',
            '${ALU8_OP}',
            'AL = alu_result',
          ],
        },
      },
      opcode: [Opcodes.ALU_LOGIC_EB, 'ModRM_110_111_00', 'DISP_i16'],
      operands: ['rm'],
      operandSize: 8,
      cycles: 20,
    },
    {
      modes: {
        real: {
          operation: [
            'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET}',
            'a = AX',
            'b = RAM:i8[effective_address]',
            '${ALU8_OP}',
            'AL = alu_result',
          ],
        },
        protected: {
          operation: [
            'offset = ${MOD_RM_OFFSET}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'a = AX',
            'b = RAM:i8[effective_address]',
            '${ALU8_OP}',
            'AL = alu_result',
          ],
        },
      },
      opcode: [Opcodes.ALU_LOGIC_EB, 'ModRM_rm_111_00'],
      operands: ['rm'],
      operandSize: 8,
      cycles: 20,
    },
    {
      modes: {
        real: {
          operation: [
            'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
            'a = AX',
            'b = RAM:i8[effective_address]',
            '${ALU8_OP}',
            'AL = alu_result',
          ],
        },
        protected: {
          operation: [
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'a = AX',
            'b = RAM:i8[effective_address]',
            '${ALU8_OP}',
            'AL = alu_result',
          ],
        },
      },
      opcode: [Opcodes.ALU_LOGIC_EB, 'ModRM_rm_111_01', 'DISP_i8'],
      operands: ['rm'],
      operandSize: 8,
      cycles: 20,
    },
    {
      modes: {
        real: {
          operation: [
            'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
            'a = AX',
            'b = RAM:i8[effective_address]',
            '${ALU8_OP}',
            'AL = alu_result',
          ],
        },
        protected: {
          operation: [
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'a = AX',
            'b = RAM:i8[effective_address]',
            '${ALU8_OP}',
            'AL = alu_result',
          ],
        },
      },
      opcode: [Opcodes.ALU_LOGIC_EB, 'ModRM_rm_111_10', 'DISP_i16'],
      operands: ['rm'],
      operandSize: 8,
      cycles: 20,
    },
    {
      operation: [
        'a = AX',
        'b = ${MOD_RM_RM8}',
        '${ALU8_OP}',
        'AL = alu_result',
      ],
      opcode: [Opcodes.ALU_LOGIC_EB, 'ModRM_rm8_111_11'],
      operands: ['rm'],
      operandSize: 8,
      cycles: 17,
    },
    // 0xF7 /7 - IDIV ew
    {
      modes: {
        real: {
          operation: [
            'offset = %{DISP:u16}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'div_a = (DX << 16) | AX',
            'b = RAM:i16[effective_address]',
            '${ALU16_OP}',
            'AX = alu_result',
          ],
        },
        protected: {
          operation: [
            'offset = %{DISP:u16}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'div_a = (DX << 16) | AX',
            'b = RAM:i16[effective_address]',
            '${ALU16_OP}',
            'AX = alu_result',
          ],
        },
      },
      opcode: [Opcodes.ALU_LOGIC_EW, 'ModRM_110_111_00', 'DISP_i16'],
      operands: ['rm'],
      operandSize: 16,
      cycles: 28,
    },
    {
      modes: {
        real: {
          operation: [
            'offset = ${MOD_RM_OFFSET}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'div_a = (DX << 16) | AX',
            'b = RAM:i16[effective_address]',
            '${ALU16_OP}',
            'AX = alu_result',
          ],
        },
        protected: {
          operation: [
            'offset = ${MOD_RM_OFFSET}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'div_a = (DX << 16) | AX',
            'b = RAM:i16[effective_address]',
            '${ALU16_OP}',
            'AX = alu_result',
          ],
        },
      },
      opcode: [Opcodes.ALU_LOGIC_EW, 'ModRM_rm_111_00'],
      operands: ['rm'],
      operandSize: 16,
      cycles: 28,
    },
    {
      modes: {
        real: {
          operation: [
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'div_a = (DX << 16) | AX',
            'b = RAM:i16[effective_address]',
            '${ALU16_OP}',
            'AX = alu_result',
          ],
        },
        protected: {
          operation: [
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'div_a = (DX << 16) | AX',
            'b = RAM:i16[effective_address]',
            '${ALU16_OP}',
            'AX = alu_result',
          ],
        },
      },
      opcode: [Opcodes.ALU_LOGIC_EW, 'ModRM_rm_111_01', 'DISP_i8'],
      operands: ['rm'],
      operandSize: 16,
      cycles: 28,
    },
    {
      modes: {
        real: {
          operation: [
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'div_a = (DX << 16) | AX',
            'b = RAM:i16[effective_address]',
            '${ALU16_OP}',
            'AX = alu_result',
          ],
        },
        protected: {
          operation: [
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'div_a = (DX << 16) | AX',
            'b = RAM:i16[effective_address]',
            '${ALU16_OP}',
            'AX = alu_result',
          ],
        },
      },
      opcode: [Opcodes.ALU_LOGIC_EW, 'ModRM_rm_111_10', 'DISP_i16'],
      operands: ['rm'],
      operandSize: 16,
      cycles: 28,
    },
    {
      operation: [
        'div_a = (DX << 16) | AX',
        'b = ${MOD_RM_RM16}',
        '${ALU16_OP}',
        'AX = alu_result',
      ],
      opcode: [Opcodes.ALU_LOGIC_EW, 'ModRM_rm16_111_11'],
      operands: ['rm'],
      operandSize: 16,
      cycles: 25,
    },
  ],
};
