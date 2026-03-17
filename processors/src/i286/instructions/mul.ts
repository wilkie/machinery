import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const mul: InstructionInfo = {
  identifier: 'mul',
  name: 'Unsigned Multiplication',
  description:
    'If `MUL` has a byte operand, then the byte is multiplied by `AL`, and the result is left in `AX`. Carry and overflow are set to 0 if `AH` is 0; they are set to 1 otherwise. If `MUL` has a word operand, then the word is multiplied by `AX`, and the result is left in `DX:AX`. `DX` contains the high order 16 bits of the product. Carry and overflow are set to 0 if DX is 0; they are set to 1 otherwise.',
  modifies: ['OF', 'CF'],
  undefined: ['SF', 'ZF', 'AF', 'PF'],
  macros: {
    ALU8_OP: [
      'alu_result = a * b',
      'CARRY = alu_result > 0xff ? 1 : 0',
      'OF = CARRY',
      'AF = 1',
      'tmp_result = alu_result',
      // SF/ZF/PF computed from high byte of 16-bit result (undocumented 286 behavior)
      'alu_result = (alu_result >> 8) & 0xff',
      'flag_op = ${FLAG_OP_ALU} | ${FLAG_OP_NOCF} | ${FLAG_OP_NOAF} | ${FLAG_OP_NOOF} | ${FLAG_OP_8BIT}',
    ],
    ALU16_OP: [
      'alu_result = a * b',
      'CARRY = alu_result > 0xffff ? 1 : 0',
      'OF = CARRY',
      'AF = 1',
      'tmp_result = alu_result',
      // SF/ZF/PF computed from high 16 bits of 32-bit result (undocumented 286 behavior)
      'alu_result = alu_result >> 16',
      'flag_op = ${FLAG_OP_ALU} | ${FLAG_OP_NOCF} | ${FLAG_OP_NOAF} | ${FLAG_OP_NOOF} | ${FLAG_OP_16BIT}',
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
    {
      identifier: 'tmp_result',
      size: 32,
    },
  ],
  forms: [
    // 0xF6 /4 - MUL eb
    {
      modes: {
        real: {
          operation: [
            'effective_address = ${MOD_RM_SEGMENT} + %{DISP:u16}',
            'a = AL',
            'b = RAM:u8[effective_address]',
            '${ALU8_OP}',
            'AX = tmp_result',
          ],
        },
        protected: {
          operation: [
            'offset = %{DISP:u16}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'a = AL',
            'b = RAM:u8[effective_address]',
            '${ALU8_OP}',
            'AX = tmp_result',
          ],
        },
      },
      opcode: [Opcodes.ALU_LOGIC_EB, 'ModRM_110_100_00', 'DISP_i16'],
      operands: ['rm'],
      operandSize: 8,
      cycles: 16,
    },
    {
      modes: {
        real: {
          operation: [
            'effective_address = ${MOD_RM_SEGMENT} + (${MOD_RM_OFFSET}):u16',
            'a = AL',
            'b = RAM:u8[effective_address]',
            '${ALU8_OP}',
            'AX = tmp_result',
          ],
        },
        protected: {
          operation: [
            'offset = (${MOD_RM_OFFSET}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'a = AL',
            'b = RAM:u8[effective_address]',
            '${ALU8_OP}',
            'AX = tmp_result',
          ],
        },
      },
      opcode: [Opcodes.ALU_LOGIC_EB, 'ModRM_rm_100_00'],
      operands: ['rm'],
      operandSize: 8,
      cycles: 16,
    },
    {
      modes: {
        real: {
          operation: [
            'effective_address = ${MOD_RM_SEGMENT} + (${MOD_RM_OFFSET} + %{DISP}):u16',
            'a = AL',
            'b = RAM:u8[effective_address]',
            '${ALU8_OP}',
            'AX = tmp_result',
          ],
        },
        protected: {
          operation: [
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'a = AL',
            'b = RAM:u8[effective_address]',
            '${ALU8_OP}',
            'AX = tmp_result',
          ],
        },
      },
      opcode: [Opcodes.ALU_LOGIC_EB, 'ModRM_rm_100_01', 'DISP_i8'],
      operands: ['rm'],
      operandSize: 8,
      cycles: 16,
    },
    {
      modes: {
        real: {
          operation: [
            'effective_address = ${MOD_RM_SEGMENT} + (${MOD_RM_OFFSET} + %{DISP}):u16',
            'a = AL',
            'b = RAM:u8[effective_address]',
            '${ALU8_OP}',
            'AX = tmp_result',
          ],
        },
        protected: {
          operation: [
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'a = AL',
            'b = RAM:u8[effective_address]',
            '${ALU8_OP}',
            'AX = tmp_result',
          ],
        },
      },
      opcode: [Opcodes.ALU_LOGIC_EB, 'ModRM_rm_100_10', 'DISP_i16'],
      operands: ['rm'],
      operandSize: 8,
      cycles: 16,
    },
    {
      operation: [
        'a = AL',
        'b = ${MOD_RM_RM8}',
        '${ALU8_OP}',
        'AX = tmp_result',
      ],
      opcode: [Opcodes.ALU_LOGIC_EB, 'ModRM_rm8_100_11'],
      operands: ['rm'],
      operandSize: 8,
      cycles: 13,
    },
    // 0xF7 /4 - MUL ew
    {
      modes: {
        real: {
          operation: [
            'offset = %{DISP:u16}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'a = AX',
            'b = RAM:u16[effective_address]',
            '${ALU16_OP}',
            'DX = tmp_result >> 16',
            'AX = tmp_result & 0xffff',
          ],
        },
        protected: {
          operation: [
            'offset = %{DISP:u16}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'a = AX',
            'b = RAM:u16[effective_address]',
            '${ALU16_OP}',
            'DX = tmp_result >> 16',
            'AX = tmp_result & 0xffff',
          ],
        },
      },
      opcode: [Opcodes.ALU_LOGIC_EW, 'ModRM_110_100_00', 'DISP_i16'],
      operands: ['rm'],
      operandSize: 16,
      cycles: 24,
    },
    {
      modes: {
        real: {
          operation: [
            'offset = (${MOD_RM_OFFSET}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'a = AX',
            'b = RAM:u16[effective_address]',
            '${ALU16_OP}',
            'DX = tmp_result >> 16',
            'AX = tmp_result & 0xffff',
          ],
        },
        protected: {
          operation: [
            'offset = (${MOD_RM_OFFSET}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'a = AX',
            'b = RAM:u16[effective_address]',
            '${ALU16_OP}',
            'DX = tmp_result >> 16',
            'AX = tmp_result & 0xffff',
          ],
        },
      },
      opcode: [Opcodes.ALU_LOGIC_EW, 'ModRM_rm_100_00'],
      operands: ['rm'],
      operandSize: 16,
      cycles: 24,
    },
    {
      modes: {
        real: {
          operation: [
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'a = AX',
            'b = RAM:u16[effective_address]',
            '${ALU16_OP}',
            'DX = tmp_result >> 16',
            'AX = tmp_result & 0xffff',
          ],
        },
        protected: {
          operation: [
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'a = AX',
            'b = RAM:u16[effective_address]',
            '${ALU16_OP}',
            'DX = tmp_result >> 16',
            'AX = tmp_result & 0xffff',
          ],
        },
      },
      opcode: [Opcodes.ALU_LOGIC_EW, 'ModRM_rm_100_01', 'DISP_i8'],
      operands: ['rm'],
      operandSize: 16,
      cycles: 24,
    },
    {
      modes: {
        real: {
          operation: [
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'a = AX',
            'b = RAM:u16[effective_address]',
            '${ALU16_OP}',
            'DX = tmp_result >> 16',
            'AX = tmp_result & 0xffff',
          ],
        },
        protected: {
          operation: [
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'a = AX',
            'b = RAM:u16[effective_address]',
            '${ALU16_OP}',
            'DX = tmp_result >> 16',
            'AX = tmp_result & 0xffff',
          ],
        },
      },
      opcode: [Opcodes.ALU_LOGIC_EW, 'ModRM_rm_100_10', 'DISP_i16'],
      operands: ['rm'],
      operandSize: 16,
      cycles: 24,
    },
    {
      operation: [
        'a = AX',
        'b = ${MOD_RM_RM16}',
        '${ALU16_OP}',
        'DX = tmp_result >> 16',
        'AX = tmp_result & 0xffff',
      ],
      opcode: [Opcodes.ALU_LOGIC_EW, 'ModRM_rm16_100_11'],
      operands: ['rm'],
      operandSize: 16,
      cycles: 21,
    },
  ],
};
