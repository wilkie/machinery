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
      'AF = 1',
      ';; detect overflow',
      'if (((a & 0x8000) > 0) ? ~a : a):u16 >= (((((b & 0x80) > 0) ? ~b + 1 : b) & 0xff) << 7):u16',
      [
        ';; compute division manually since CPU bugs allow -0x80',
        'a = (AX & 0x8000) > 0 ? ~a : a',
        'offset = b:u8',
        'b = ((((((b & 0x80) > 0) ? ~b + 1 : b) & 0xff) << 8) - 1)',
        'a = a << 1',
        'a = a - ((a > b) ? b : 0)',
        'a = a << 1',
        'a = a - ((a > b) ? b : 0)',
        'a = a << 1',
        'a = a - ((a > b) ? b : 0)',
        'a = a << 1',
        'a = a - ((a > b) ? b : 0)',
        'a = a << 1',
        'a = a - ((a > b) ? b : 0)',
        'a = a << 1',
        'a = a - ((a > b) ? b : 0)',
        'a = a << 1',
        'a = a - ((a > b) ? b : 0)',
        'a = a << 1',
        'a = a - ((a > b) ? b : 0)',
        'b = (a >> 8) & 0xff',
        'a = a & 0xff',
        'alu_result = ((AX & 0x8000) > 0 ? ~b : b) & 0xff',

        ';; compute the carry flag as r < d but with the computed partially restored remainder',
        'CARRY = (((((offset & 0x80) > 0) ? ~b : b) & 0xff) < offset:u8) ? 1 : 0',
        'CF = CARRY',
        'OF = CARRY',
        'flag_op = ${FLAG_OP_RESOLVED}',

        'if (AX & 0x8000) > 0',
        [
          'b = (b + 1) & 0xff',
        ],
        'end if',
        'if b == (((offset & 0x80) > 0 ? ~offset + 1 : offset) & 0xff)',
        [
          'b = 0',
          'a = (a + 1) & 0xff',
          'alu_result = 0',
        ],
        'end if',
        'if (AX & 0x8000) > 0',
        [
          'b = (~b + 1) & 0xff',
        ],
        'end if',
        'ZF = alu_result == 0x0 ? 1 : 0',
        'SF = alu_result & 0x80 > 0 ? 1 : 0',
        'PF = ROM.PARITY[alu_result]',
        'if ((AX & 0x8000) >> 8) != (offset & 0x80)',
        [
          'a = (~a + 1) & 0xff',
          'if a != 0x80',
          [
            '#DE',
          ],
          'end if',
        ],
        'else',
        [
          '#DE',
        ],
        'end if',

        ';; Special case CPU bug where computed quotients of -128 is for some reason always allowed',
        'AL = a',
        'AH = b',
        'alu_result = AH',
        'b = offset',
        // Flags: SF/ZF/PF from remainder (AH), AF=1, CF=OF (undefined)
        // However, CF/OF is just whether or not AH is less than b
        'flag_op = ${FLAG_OP_ALU} | ${FLAG_OP_DIV} | ${FLAG_OP_8BIT} | ${FLAG_OP_SIGNED} | ${FLAG_OP_NOAF}',
      ],
      'else',
      [
        ';; normal case',
        'AL = a:i16 // b:i8',
        'AH = a:i16 % b:i8',
        'alu_result = AH',
        // Flags: SF/ZF/PF from remainder (AH), AF=1, CF=OF (undefined)
        // However, CF/OF is just whether or not AH is less than b
        'flag_op = ${FLAG_OP_ALU} | ${FLAG_OP_DIV} | ${FLAG_OP_8BIT} | ${FLAG_OP_SIGNED} | ${FLAG_OP_NOAF}',
      ],
      'end if',
    ],
    ALU16_OP: [
      'AF = 1',
      ';; detect overflow',
      'if (((DX & 0x8000) > 0) ? ~div_a : div_a):u32 >= (((((b & 0x8000) > 0) ? ~b + 1 : b) & 0xffff) << 15):u32',
      [
        ';; compute division manually since CPU bugs allow -0x8000',
        'div_a = (DX & 0x8000) > 0 ? ~div_a : div_a',
        'offset = b:u16',
        'div_b = ((((((b & 0x8000) > 0) ? ~b + 1 : b) & 0xffff) << 16) - 1):u32',

        'div_a = div_a << 1',
        'div_a = div_a - ((div_a > div_b) ? div_b : 0)',
        'div_a = div_a << 1',
        'div_a = div_a - ((div_a > div_b) ? div_b : 0)',
        'div_a = div_a << 1',
        'div_a = div_a - ((div_a > div_b) ? div_b : 0)',
        'div_a = div_a << 1',
        'div_a = div_a - ((div_a > div_b) ? div_b : 0)',
        'div_a = div_a << 1',
        'div_a = div_a - ((div_a > div_b) ? div_b : 0)',
        'div_a = div_a << 1',
        'div_a = div_a - ((div_a > div_b) ? div_b : 0)',
        'div_a = div_a << 1',
        'div_a = div_a - ((div_a > div_b) ? div_b : 0)',
        'div_a = div_a << 1',
        'div_a = div_a - ((div_a > div_b) ? div_b : 0)',
        'div_a = div_a << 1',
        'div_a = div_a - ((div_a > div_b) ? div_b : 0)',
        'div_a = div_a << 1',
        'div_a = div_a - ((div_a > div_b) ? div_b : 0)',
        'div_a = div_a << 1',
        'div_a = div_a - ((div_a > div_b) ? div_b : 0)',
        'div_a = div_a << 1',
        'div_a = div_a - ((div_a > div_b) ? div_b : 0)',
        'div_a = div_a << 1',
        'div_a = div_a - ((div_a > div_b) ? div_b : 0)',
        'div_a = div_a << 1',
        'div_a = div_a - ((div_a > div_b) ? div_b : 0)',
        'div_a = div_a << 1',
        'div_a = div_a - ((div_a > div_b) ? div_b : 0)',
        'div_a = div_a << 1',
        'div_a = div_a - ((div_a > div_b) ? div_b : 0)',

        'b = (div_a >> 16) & 0xffff',
        'a = div_a & 0xffff',
        'alu_result = ((DX & 0x8000) > 0 ? ~b : b) & 0xffff',

        ';; compute the carry flag as r < d but with the computed partially restored remainder',
        'CARRY = (((((offset & 0x8000) > 0) ? ~b : b) & 0xffff) < offset:u16) ? 1 : 0',
        'CF = CARRY',
        'OF = CARRY',
        'flag_op = ${FLAG_OP_RESOLVED}',

        'if (DX & 0x8000) > 0',
        [
          'b = (b + 1) & 0xffff',
        ],
        'end if',
        'if b == (((offset & 0x8000) > 0 ? ~offset + 1 : offset) & 0xffff)',
        [
          'b = 0',
          'a = (a + 1) & 0xffff',
          'alu_result = 0',
        ],
        'end if',
        'if (DX & 0x8000) > 0',
        [
          'b = (~b + 1) & 0xffff',
        ],
        'end if',
        'ZF = alu_result == 0x0 ? 1 : 0',
        'SF = alu_result & 0x8000 > 0 ? 1 : 0',
        'PF = ROM.PARITY[alu_result & 0xff]',
        'if (DX & 0x8000) != (offset & 0x8000)',
        [
          'a = (~a + 1) & 0xffff',
          'if a != 0x8000',
          [
            '#DE',
          ],
          'end if',
        ],
        'else',
        [
          '#DE',
        ],
        'end if',

        ';; Special case CPU bug where computed quotients of -0x8000 is for some reason always allowed',
        'AX = a',
        'DX = b',
        'alu_result = DX',
        'b = offset',
        // Flags: SF/ZF/PF from remainder (AH), AF=1, CF=OF (undefined)
        // However, CF/OF is just whether or not AH is less than b
        'flag_op = ${FLAG_OP_ALU} | ${FLAG_OP_DIV} | ${FLAG_OP_16BIT} | ${FLAG_OP_SIGNED} | ${FLAG_OP_NOAF}',
      ],
      'else',
      [
        ';; normal case',
        'alu_result = div_a:i32 // b:i16',
        'AX = alu_result',
        'DX = div_a:i32 % b:i16',
        'alu_result = DX',
        // Flags: SF/ZF/PF from remainder (AH), AF=1, CF=OF (undefined)
        // However, CF/OF is just whether or not AH is less than b
        'flag_op = ${FLAG_OP_ALU} | ${FLAG_OP_DIV} | ${FLAG_OP_16BIT} | ${FLAG_OP_SIGNED} | ${FLAG_OP_NOAF}',
      ],
      'end if',
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
            'effective_address = ${MOD_RM_SEGMENT} + %{DISP:u16}',
            'a = AX',
            'b = RAM:i8[effective_address]',
            '${ALU8_OP}',
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
            'effective_address = ${MOD_RM_SEGMENT} + (${MOD_RM_OFFSET}):u16',
            'a = AX',
            'b = RAM:i8[effective_address]',
            '${ALU8_OP}',
          ],
        },
        protected: {
          operation: [
            'offset = (${MOD_RM_OFFSET}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'a = AX',
            'b = RAM:i8[effective_address]',
            '${ALU8_OP}',
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
            'effective_address = ${MOD_RM_SEGMENT} + (${MOD_RM_OFFSET} + %{DISP}):u16',
            'a = AX',
            'b = RAM:i8[effective_address]',
            '${ALU8_OP}',
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
            'effective_address = ${MOD_RM_SEGMENT} + (${MOD_RM_OFFSET} + %{DISP}):u16',
            'a = AX',
            'b = RAM:i8[effective_address]',
            '${ALU8_OP}',
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
            'offset = (${MOD_RM_OFFSET}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'div_a = (DX << 16) | AX',
            'b = RAM:i16[effective_address]',
            '${ALU16_OP}',
          ],
        },
        protected: {
          operation: [
            'offset = (${MOD_RM_OFFSET}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'div_a = (DX << 16) | AX',
            'b = RAM:i16[effective_address]',
            '${ALU16_OP}',
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
      ],
      opcode: [Opcodes.ALU_LOGIC_EW, 'ModRM_rm16_111_11'],
      operands: ['rm'],
      operandSize: 16,
      cycles: 25,
    },
  ],
};
