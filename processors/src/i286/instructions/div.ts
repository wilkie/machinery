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
      'if a >= (b:u16 << 8)',
      [
        ';; compute flags by performing the nonrestoring division',
        'b = b << 8',
        'a = ((a >= b) ? (a - b) | 1 : a):u16',
        'a = (((a << 1) >= b || (a & 0x8000) > 0) ? ((a << 1) - b) | 1 : a << 1):u16',
        'a = (((a << 1) >= b || (a & 0x8000) > 0) ? ((a << 1) - b) | 1 : a << 1):u16',
        'a = (((a << 1) >= b || (a & 0x8000) > 0) ? ((a << 1) - b) | 1 : a << 1):u16',
        'a = (((a << 1) >= b || (a & 0x8000) > 0) ? ((a << 1) - b) | 1 : a << 1):u16',
        'a = (((a << 1) >= b || (a & 0x8000) > 0) ? ((a << 1) - b) | 1 : a << 1):u16',
        'a = (((a << 1) >= b || (a & 0x8000) > 0) ? ((a << 1) - b) | 1 : a << 1):u16',
        ';; Set up the flag resolution as the same as CMP eb, rb with the final compare the algorithm performs',
        'a = (a & 0x7fff) >> 7',
        'b = b >> 8',
        'alu_result = a - b',
        'flag_op = ${FLAG_OP_ALU} | ${FLAG_OP_SUB} | ${FLAG_OP_8BIT}',
        '#0 if 1 == 1',
      ],
      'end if',
      'alu_result = a // b',
      'AL = alu_result',
      'AH = a % b',
      // Flags: SF/ZF/PF from remainder (AH), AF=1, CF=OF (undefined)
      'alu_result = AH',
      'flag_op = ${FLAG_OP_ALU} | ${FLAG_OP_DIV} | ${FLAG_OP_8BIT} | ${FLAG_OP_NOAF}',
      'AF = 1',
    ],
    ALU16_OP: [
      'if div_a >= (b << 16)',
      [
        ';; compute flags by performing the nonrestoring division (16-bit)',
        'div_b = b << 16',
        'div_a = ((div_a >= div_b) ? (div_a - div_b) | 1 : div_a):u32',
        'div_a = (((div_a << 1):u32 >= div_b || (div_a & 0x80000000) != 0) ? ((div_a << 1):u32 - div_b) | 1 : div_a << 1):u32',
        'div_a = (((div_a << 1):u32 >= div_b || (div_a & 0x80000000) != 0) ? ((div_a << 1):u32 - div_b) | 1 : div_a << 1):u32',
        'div_a = (((div_a << 1):u32 >= div_b || (div_a & 0x80000000) != 0) ? ((div_a << 1):u32 - div_b) | 1 : div_a << 1):u32',
        'div_a = (((div_a << 1):u32 >= div_b || (div_a & 0x80000000) != 0) ? ((div_a << 1):u32 - div_b) | 1 : div_a << 1):u32',
        'div_a = (((div_a << 1):u32 >= div_b || (div_a & 0x80000000) != 0) ? ((div_a << 1):u32 - div_b) | 1 : div_a << 1):u32',
        'div_a = (((div_a << 1):u32 >= div_b || (div_a & 0x80000000) != 0) ? ((div_a << 1):u32 - div_b) | 1 : div_a << 1):u32',
        'div_a = (((div_a << 1):u32 >= div_b || (div_a & 0x80000000) != 0) ? ((div_a << 1):u32 - div_b) | 1 : div_a << 1):u32',
        'div_a = (((div_a << 1):u32 >= div_b || (div_a & 0x80000000) != 0) ? ((div_a << 1):u32 - div_b) | 1 : div_a << 1):u32',
        'div_a = (((div_a << 1):u32 >= div_b || (div_a & 0x80000000) != 0) ? ((div_a << 1):u32 - div_b) | 1 : div_a << 1):u32',
        'div_a = (((div_a << 1):u32 >= div_b || (div_a & 0x80000000) != 0) ? ((div_a << 1):u32 - div_b) | 1 : div_a << 1):u32',
        'div_a = (((div_a << 1):u32 >= div_b || (div_a & 0x80000000) != 0) ? ((div_a << 1):u32 - div_b) | 1 : div_a << 1):u32',
        'div_a = (((div_a << 1):u32 >= div_b || (div_a & 0x80000000) != 0) ? ((div_a << 1):u32 - div_b) | 1 : div_a << 1):u32',
        'div_a = (((div_a << 1):u32 >= div_b || (div_a & 0x80000000) != 0) ? ((div_a << 1):u32 - div_b) | 1 : div_a << 1):u32',
        'div_a = (((div_a << 1):u32 >= div_b || (div_a & 0x80000000) != 0) ? ((div_a << 1):u32 - div_b) | 1 : div_a << 1):u32',
        ';; Set up the flag resolution as the same as CMP ew, rw with the final compare the algorithm performs',
        'a = (div_a & 0x7fffffff) >> 15',
        'alu_result = a - b',
        'flag_op = ${FLAG_OP_ALU} | ${FLAG_OP_SUB} | ${FLAG_OP_16BIT}',
        '#0 if 1 == 1',
      ],
      'end if',
      'alu_result = div_a // b',
      'AX = alu_result',
      'DX = div_a % b',
      // Flags: SF/ZF/PF from remainder (DX), AF=1, CF=OF (undefined)
      'alu_result = DX',
      'flag_op = ${FLAG_OP_ALU} | ${FLAG_OP_DIV} | ${FLAG_OP_16BIT} | ${FLAG_OP_NOAF}',
      'AF = 1',
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
    // 0xF6 /6 - DIV eb
    {
      modes: {
        real: {
          operation: [
            'effective_address = ${MOD_RM_SEGMENT} + %{DISP:u16}',
            'a = AX',
            'b = RAM:u8[effective_address]',
            '${ALU8_OP}',
          ],
        },
        protected: {
          operation: [
            'offset = %{DISP:u16}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'a = AX',
            'b = RAM:u8[effective_address]',
            '${ALU8_OP}',
          ],
        },
      },
      opcode: [Opcodes.ALU_LOGIC_EB, 'ModRM_110_110_00', 'DISP_i16'],
      operands: ['rm'],
      operandSize: 8,
      cycles: 20,
    },
    {
      modes: {
        real: {
          operation: [
            'effective_address = ${MOD_RM_SEGMENT} + (${MOD_RM_OFFSET}):u16',
            'a = AX',
            'b = RAM:u8[effective_address]',
            '${ALU8_OP}',
          ],
        },
        protected: {
          operation: [
            'offset = (${MOD_RM_OFFSET}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'a = AX',
            'b = RAM:u8[effective_address]',
            '${ALU8_OP}',
          ],
        },
      },
      opcode: [Opcodes.ALU_LOGIC_EB, 'ModRM_rm_110_00'],
      operands: ['rm'],
      operandSize: 8,
      cycles: 20,
    },
    {
      modes: {
        real: {
          operation: [
            'effective_address = ${MOD_RM_SEGMENT} + (${MOD_RM_OFFSET} + %{DISP}):u16',
            'a = AX',
            'b = RAM:u8[effective_address]',
            '${ALU8_OP}',
          ],
        },
        protected: {
          operation: [
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'a = AX',
            'b = RAM:u8[effective_address]',
            '${ALU8_OP}',
          ],
        },
      },
      opcode: [Opcodes.ALU_LOGIC_EB, 'ModRM_rm_110_01', 'DISP_i8'],
      operands: ['rm'],
      operandSize: 8,
      cycles: 20,
    },
    {
      modes: {
        real: {
          operation: [
            'effective_address = ${MOD_RM_SEGMENT} + (${MOD_RM_OFFSET} + %{DISP}):u16',
            'a = AX',
            'b = RAM:u8[effective_address]',
            '${ALU8_OP}',
          ],
        },
        protected: {
          operation: [
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'a = AX',
            'b = RAM:u8[effective_address]',
            '${ALU8_OP}',
          ],
        },
      },
      opcode: [Opcodes.ALU_LOGIC_EB, 'ModRM_rm_110_10', 'DISP_i16'],
      operands: ['rm'],
      operandSize: 8,
      cycles: 20,
    },
    {
      operation: [
        'a = AX',
        'b = ${MOD_RM_RM8}',
        '${ALU8_OP}',
      ],
      opcode: [Opcodes.ALU_LOGIC_EB, 'ModRM_rm8_110_11'],
      operands: ['rm'],
      operandSize: 8,
      cycles: 17,
    },
    // 0xF7 /6 - DIV ew
    {
      modes: {
        real: {
          operation: [
            'offset = %{DISP:u16}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'div_a = (DX << 16) | AX',
            'b = RAM:u16[effective_address]',
            '${ALU16_OP}',
          ],
        },
        protected: {
          operation: [
            'offset = %{DISP:u16}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'div_a = (DX << 16) | AX',
            'b = RAM:u16[effective_address]',
            '${ALU16_OP}',
          ],
        },
      },
      opcode: [Opcodes.ALU_LOGIC_EW, 'ModRM_110_110_00', 'DISP_i16'],
      operands: ['rm'],
      operandSize: 16,
      cycles: 28,
    },
    {
      modes: {
        real: {
          operation: [
            'offset = (${MOD_RM_OFFSET}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'div_a = (DX << 16) | AX',
            'b = RAM:u16[effective_address]',
            '${ALU16_OP}',
          ],
        },
        protected: {
          operation: [
            'offset = (${MOD_RM_OFFSET}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'div_a = (DX << 16) | AX',
            'b = RAM:u16[effective_address]',
            '${ALU16_OP}',
          ],
        },
      },
      opcode: [Opcodes.ALU_LOGIC_EW, 'ModRM_rm_110_00'],
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
            'b = RAM:u16[effective_address]',
            '${ALU16_OP}',
          ],
        },
        protected: {
          operation: [
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'div_a = (DX << 16) | AX',
            'b = RAM:u16[effective_address]',
            '${ALU16_OP}',
          ],
        },
      },
      opcode: [Opcodes.ALU_LOGIC_EW, 'ModRM_rm_110_01', 'DISP_i8'],
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
            'b = RAM:u16[effective_address]',
            '${ALU16_OP}',
          ],
        },
        protected: {
          operation: [
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'div_a = (DX << 16) | AX',
            'b = RAM:u16[effective_address]',
            '${ALU16_OP}',
          ],
        },
      },
      opcode: [Opcodes.ALU_LOGIC_EW, 'ModRM_rm_110_10', 'DISP_i16'],
      operands: ['rm'],
      operandSize: 16,
      cycles: 28,
    },
    {
      operation: [
        'div_a = (DX << 16) | AX',
        'b = ${MOD_RM_RM16}',
        '${ALU16_OP}',
      ],
      opcode: [Opcodes.ALU_LOGIC_EW, 'ModRM_rm16_110_11'],
      operands: ['rm'],
      operandSize: 16,
      cycles: 25,
    },
  ],
};
