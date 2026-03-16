import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const shr: InstructionInfo = {
  identifier: 'shr',
  name: 'Shift Logical Right',
  description:
    '`SAR` and `SHR` shift the bits of the operand downward. The low-order bit is shifted into the carry flag. The effect is to divide the operand by 2. `SAR` performs a signed divide: the high-order bit remains the same. `SHR` performs an unsigned divide: the high-order bit is set to 0.\n\nThe shift is repeated the number of times indicated by the second operand, which is either an immediate number or the contents of the `CL` register. To reduce the maximum execution time, the 286 does not allow shift counts greater than 31. If a shift count greater than 31 is attempted, only the bottom five bits of the shift count are used. The 8086 uses all 8 bits of the shift count.\n\nThe overflow flag is set only if the single-shift forms of the instructions are used. For left shifts, it is set to 0 if the high bit of the answer is the same as the result carry flag (i.e., the top two bits of the original operand were the same); it is set to 1 if they are different. For SAR it is set to 0 for all single shifts. For SHR, it is set to the high-order bit of the original operand. Neither flag bit is modified when the count value is zero.',
  modifies: ['CF', 'ZF', 'PF', 'SF'],
  undefined: ['AF'],
  macros: {
    ALU8_OP: [
      'alu_result = a >>> b',
      'flag_op = ${FLAG_OP_ALU} | ${FLAG_OP_8BIT} | ${FLAG_OP_SHIFT_RIGHT}',
    ],
    ALU16_OP: [
      'alu_result = a >>> b',
      'flag_op = ${FLAG_OP_ALU} | ${FLAG_OP_16BIT} | ${FLAG_OP_SHIFT_RIGHT}',
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
    // 0xD0 /5 - SHR eb, 1
    {
      modes: {
        real: {
          operation: [
            'effective_address = ${MOD_RM_SEGMENT} + %{DISP}',
            'a = RAM:u8[effective_address]',
            'b = 1',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = alu_result',
          ],
        },
        protected: {
          operation: [
            'offset = %{DISP:u16}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'a = RAM:u8[effective_address]',
            'b = 1',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = alu_result',
          ],
        },
      },
      modifies: ['OF', 'CF', 'ZF', 'PF', 'SF'],
      opcode: [
        Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EB,
        'ModRM_110_101_00',
        'DISP_i16',
      ],
      operands: ['rm'],
      operandSize: 8,
      cycles: 7,
    },
    {
      modes: {
        real: {
          operation: [
            'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET}',
            'a = RAM:u8[effective_address]',
            'b = 1',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = alu_result',
          ],
        },
        protected: {
          operation: [
            'offset = ${MOD_RM_OFFSET}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'a = RAM:u8[effective_address]',
            'b = 1',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = alu_result',
          ],
        },
      },
      modifies: ['OF', 'CF', 'ZF', 'PF', 'SF'],
      opcode: [Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EB, 'ModRM_rm_101_00'],
      operands: ['rm'],
      operandSize: 8,
      cycles: 7,
    },
    {
      modes: {
        real: {
          operation: [
            'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
            'a = RAM:u8[effective_address]',
            'b = 1',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = alu_result',
          ],
        },
        protected: {
          operation: [
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'a = RAM:u8[effective_address]',
            'b = 1',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = alu_result',
          ],
        },
      },
      modifies: ['OF', 'CF', 'ZF', 'PF', 'SF'],
      opcode: [
        Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EB,
        'ModRM_rm_101_01',
        'DISP_i8',
      ],
      operands: ['rm'],
      operandSize: 8,
      cycles: 7,
    },
    {
      modes: {
        real: {
          operation: [
            'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
            'a = RAM:u8[effective_address]',
            'b = 1',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = alu_result',
          ],
        },
        protected: {
          operation: [
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'a = RAM:u8[effective_address]',
            'b = 1',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = alu_result',
          ],
        },
      },
      modifies: ['OF', 'CF', 'ZF', 'PF', 'SF'],
      opcode: [
        Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EB,
        'ModRM_rm_101_10',
        'DISP_i16',
      ],
      operands: ['rm'],
      operandSize: 8,
      cycles: 7,
    },
    {
      operation: [
        'a = ${MOD_RM_RM8}',
        'b = 1',
        '${ALU8_OP}',
        '${MOD_RM_RM8} = alu_result',
      ],
      modifies: ['OF', 'CF', 'ZF', 'PF', 'SF'],
      opcode: [Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EB, 'ModRM_rm8_101_11'],
      operands: ['rm'],
      operandSize: 8,
      cycles: 2,
    },
    // 0xD2 /5 - SHR eb, CL
    {
      modes: {
        real: {
          operation: [
            'next if (CL & 0x1f) == 0',
            'effective_address = ${MOD_RM_SEGMENT} + %{DISP}',
            'a = RAM:u8[effective_address]',
            'b = CL & 0x1f',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = alu_result',
          ],
        },
        protected: {
          operation: [
            'next if (CL & 0x1f) == 0',
            'offset = %{DISP:u16}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'a = RAM:u8[effective_address]',
            'b = CL & 0x1f',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = alu_result',
          ],
        },
      },
      undefined: ['AF', 'OF'],
      opcode: [
        Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EB_CL,
        'ModRM_110_101_00',
        'DISP_i16',
      ],
      operands: ['rm', 'CL'],
      operandSize: 8,
      cycles: 8,
    },
    {
      modes: {
        real: {
          operation: [
            'next if (CL & 0x1f) == 0',
            'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET}',
            'a = RAM:u8[effective_address]',
            'b = CL & 0x1f',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = alu_result',
          ],
        },
        protected: {
          operation: [
            'next if (CL & 0x1f) == 0',
            'offset = ${MOD_RM_OFFSET}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'a = RAM:u8[effective_address]',
            'b = CL & 0x1f',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = alu_result',
          ],
        },
      },
      undefined: ['AF', 'OF'],
      opcode: [Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EB_CL, 'ModRM_rm_101_00'],
      operands: ['rm', 'CL'],
      operandSize: 8,
      cycles: 8,
    },
    {
      modes: {
        real: {
          operation: [
            'next if (CL & 0x1f) == 0',
            'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
            'a = RAM:u8[effective_address]',
            'b = CL & 0x1f',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = alu_result',
          ],
        },
        protected: {
          operation: [
            'next if (CL & 0x1f) == 0',
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'a = RAM:u8[effective_address]',
            'b = CL & 0x1f',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = alu_result',
          ],
        },
      },
      undefined: ['AF', 'OF'],
      opcode: [
        Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EB_CL,
        'ModRM_rm_101_01',
        'DISP_i8',
      ],
      operands: ['rm', 'CL'],
      operandSize: 8,
      cycles: 8,
    },
    {
      modes: {
        real: {
          operation: [
            'next if (CL & 0x1f) == 0',
            'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
            'a = RAM:u8[effective_address]',
            'b = CL & 0x1f',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = alu_result',
          ],
        },
        protected: {
          operation: [
            'next if (CL & 0x1f) == 0',
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'a = RAM:u8[effective_address]',
            'b = CL & 0x1f',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = alu_result',
          ],
        },
      },
      undefined: ['AF', 'OF'],
      opcode: [
        Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EB_CL,
        'ModRM_rm_101_10',
        'DISP_i16',
      ],
      operands: ['rm', 'CL'],
      operandSize: 8,
      cycles: 8,
    },
    {
      operation: [
        'next if (CL & 0x1f) == 0',
        'a = ${MOD_RM_RM8}',
        'b = CL & 0x1f',
        '${ALU8_OP}',
        '${MOD_RM_RM8} = alu_result',
      ],
      undefined: ['AF', 'OF'],
      opcode: [Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EB_CL, 'ModRM_rm8_101_11'],
      operands: ['rm', 'CL'],
      operandSize: 8,
      cycles: 5,
    },
    // 0xC0 /5 - SHR eb, db
    {
      modes: {
        real: {
          operation: [
            'next if (%{imm} & 0x1f) == 0',
            'effective_address = ${MOD_RM_SEGMENT} + %{DISP}',
            'a = RAM:u8[effective_address]',
            'b = %{imm} & 0x1f',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = alu_result',
          ],
        },
        protected: {
          operation: [
            'next if (%{imm} & 0x1f) == 0',
            'offset = %{DISP:u16}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'a = RAM:u8[effective_address]',
            'b = %{imm} & 0x1f',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = alu_result',
          ],
        },
      },
      undefined: ['AF', 'OF'],
      opcode: [
        Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EB_DB,
        'ModRM_110_101_00',
        'DISP_i16',
        'IMM_u8',
      ],
      operands: ['rm', 'imm'],
      operandSize: 8,
      cycles: 8,
    },
    {
      modes: {
        real: {
          operation: [
            'next if (%{imm} & 0x1f) == 0',
            'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET}',
            'a = RAM:u8[effective_address]',
            'b = %{imm} & 0x1f',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = alu_result',
          ],
        },
        protected: {
          operation: [
            'next if (%{imm} & 0x1f) == 0',
            'offset = ${MOD_RM_OFFSET}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'a = RAM:u8[effective_address]',
            'b = %{imm} & 0x1f',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = alu_result',
          ],
        },
      },
      undefined: ['AF', 'OF'],
      opcode: [
        Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EB_DB,
        'ModRM_rm_101_00',
        'IMM_u8',
      ],
      operands: ['rm', 'imm'],
      operandSize: 8,
      cycles: 8,
    },
    {
      modes: {
        real: {
          operation: [
            'next if (%{imm} & 0x1f) == 0',
            'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
            'a = RAM:u8[effective_address]',
            'b = %{imm} & 0x1f',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = alu_result',
          ],
        },
        protected: {
          operation: [
            'next if (%{imm} & 0x1f) == 0',
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'a = RAM:u8[effective_address]',
            'b = %{imm} & 0x1f',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = alu_result',
          ],
        },
      },
      undefined: ['AF', 'OF'],
      opcode: [
        Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EB_DB,
        'ModRM_rm_101_01',
        'DISP_i8',
        'IMM_u8',
      ],
      operands: ['rm', 'imm'],
      operandSize: 8,
      cycles: 8,
    },
    {
      modes: {
        real: {
          operation: [
            'next if (%{imm} & 0x1f) == 0',
            'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
            'a = RAM:u8[effective_address]',
            'b = %{imm} & 0x1f',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = alu_result',
          ],
        },
        protected: {
          operation: [
            'next if (%{imm} & 0x1f) == 0',
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'a = RAM:u8[effective_address]',
            'b = %{imm} & 0x1f',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = alu_result',
          ],
        },
      },
      undefined: ['AF', 'OF'],
      opcode: [
        Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EB_DB,
        'ModRM_rm_101_10',
        'DISP_i16',
        'IMM_u8',
      ],
      operands: ['rm', 'imm'],
      operandSize: 8,
      cycles: 8,
    },
    {
      operation: [
        'next if (%{imm} & 0x1f) == 0',
        'a = ${MOD_RM_RM8}',
        'b = %{imm} & 0x1f',
        '${ALU8_OP}',
        '${MOD_RM_RM8} = alu_result',
      ],
      undefined: ['AF', 'OF'],
      opcode: [
        Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EB_DB,
        'ModRM_rm8_101_11',
        'IMM_u8',
      ],
      operands: ['rm', 'imm'],
      operandSize: 8,
      cycles: 5,
    },
    // 0xD1 /5 - SHR ew, 1
    {
      modes: {
        real: {
          operation: [
            'offset = %{DISP:u16}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'a = RAM:u16[effective_address]',
            'b = 1',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = alu_result',
          ],
        },
        protected: {
          operation: [
            'offset = %{DISP:u16}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'a = RAM:u16[effective_address]',
            'b = 1',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = alu_result',
          ],
        },
      },
      modifies: ['OF', 'CF', 'ZF', 'PF', 'SF'],
      opcode: [
        Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EW,
        'ModRM_110_101_00',
        'DISP_i16',
      ],
      operands: ['rm'],
      operandSize: 16,
      cycles: 7,
    },
    {
      modes: {
        real: {
          operation: [
            'offset = ${MOD_RM_OFFSET}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'a = RAM:u16[effective_address]',
            'b = 1',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = alu_result',
          ],
        },
        protected: {
          operation: [
            'offset = ${MOD_RM_OFFSET}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'a = RAM:u16[effective_address]',
            'b = 1',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = alu_result',
          ],
        },
      },
      modifies: ['OF', 'CF', 'ZF', 'PF', 'SF'],
      opcode: [Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EW, 'ModRM_rm_101_00'],
      operands: ['rm'],
      operandSize: 16,
      cycles: 7,
    },
    {
      modes: {
        real: {
          operation: [
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'a = RAM:u16[effective_address]',
            'b = 1',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = alu_result',
          ],
        },
        protected: {
          operation: [
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'a = RAM:u16[effective_address]',
            'b = 1',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = alu_result',
          ],
        },
      },
      modifies: ['OF', 'CF', 'ZF', 'PF', 'SF'],
      opcode: [
        Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EW,
        'ModRM_rm_101_01',
        'DISP_i8',
      ],
      operands: ['rm'],
      operandSize: 16,
      cycles: 7,
    },
    {
      modes: {
        real: {
          operation: [
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'a = RAM:u16[effective_address]',
            'b = 1',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = alu_result',
          ],
        },
        protected: {
          operation: [
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'a = RAM:u16[effective_address]',
            'b = 1',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = alu_result',
          ],
        },
      },
      modifies: ['OF', 'CF', 'ZF', 'PF', 'SF'],
      opcode: [
        Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EW,
        'ModRM_rm_101_10',
        'DISP_i16',
      ],
      operands: ['rm'],
      operandSize: 16,
      cycles: 7,
    },
    {
      operation: [
        'a = ${MOD_RM_RM16}',
        'b = 1',
        '${ALU16_OP}',
        '${MOD_RM_RM16} = alu_result',
      ],
      modifies: ['OF', 'CF', 'ZF', 'PF', 'SF'],
      opcode: [Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EW, 'ModRM_rm16_101_11'],
      operands: ['rm'],
      operandSize: 16,
      cycles: 2,
    },
    // 0xD3 /5 - SHR ew, CL
    {
      modes: {
        real: {
          operation: [
            'next if (CL & 0x1f) == 0',
            'offset = %{DISP:u16}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'a = RAM:u16[effective_address]',
            'b = CL & 0x1f',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = alu_result',
          ],
        },
        protected: {
          operation: [
            'next if (CL & 0x1f) == 0',
            'offset = %{DISP:u16}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'a = RAM:u16[effective_address]',
            'b = CL & 0x1f',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = alu_result',
          ],
        },
      },
      undefined: ['AF', 'OF'],
      opcode: [
        Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EW_CL,
        'ModRM_110_101_00',
        'DISP_i16',
      ],
      operands: ['rm', 'CL'],
      operandSize: 16,
      cycles: 8,
    },
    {
      modes: {
        real: {
          operation: [
            'next if (CL & 0x1f) == 0',
            'offset = ${MOD_RM_OFFSET}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'a = RAM:u16[effective_address]',
            'b = CL & 0x1f',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = alu_result',
          ],
        },
        protected: {
          operation: [
            'next if (CL & 0x1f) == 0',
            'offset = ${MOD_RM_OFFSET}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'a = RAM:u16[effective_address]',
            'b = CL & 0x1f',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = alu_result',
          ],
        },
      },
      undefined: ['AF', 'OF'],
      opcode: [Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EW_CL, 'ModRM_rm_101_00'],
      operands: ['rm', 'CL'],
      operandSize: 16,
      cycles: 8,
    },
    {
      modes: {
        real: {
          operation: [
            'next if (CL & 0x1f) == 0',
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'a = RAM:u16[effective_address]',
            'b = CL & 0x1f',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = alu_result',
          ],
        },
        protected: {
          operation: [
            'next if (CL & 0x1f) == 0',
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'a = RAM:u16[effective_address]',
            'b = CL & 0x1f',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = alu_result',
          ],
        },
      },
      undefined: ['AF', 'OF'],
      opcode: [
        Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EW_CL,
        'ModRM_rm_101_01',
        'DISP_i8',
      ],
      operands: ['rm', 'CL'],
      operandSize: 16,
      cycles: 8,
    },
    {
      modes: {
        real: {
          operation: [
            'next if (CL & 0x1f) == 0',
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'a = RAM:u16[effective_address]',
            'b = CL & 0x1f',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = alu_result',
          ],
        },
        protected: {
          operation: [
            'next if (CL & 0x1f) == 0',
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'a = RAM:u16[effective_address]',
            'b = CL & 0x1f',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = alu_result',
          ],
        },
      },
      undefined: ['AF', 'OF'],
      opcode: [
        Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EW_CL,
        'ModRM_rm_101_10',
        'DISP_i16',
      ],
      operands: ['rm', 'CL'],
      operandSize: 16,
      cycles: 8,
    },
    {
      operation: [
        'next if (CL & 0x1f) == 0',
        'a = ${MOD_RM_RM16}',
        'b = CL & 0x1f',
        '${ALU16_OP}',
        '${MOD_RM_RM16} = alu_result',
      ],
      undefined: ['AF', 'OF'],
      opcode: [Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EW_CL, 'ModRM_rm16_101_11'],
      operands: ['rm', 'CL'],
      operandSize: 16,
      cycles: 5,
    },
    // 0xC1 /5 - SHR ew, db
    {
      modes: {
        real: {
          operation: [
            'next if (%{imm} & 0x1f) == 0',
            'offset = %{DISP:u16}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'a = RAM:u16[effective_address]',
            'b = %{imm} & 0x1f',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = alu_result',
          ],
        },
        protected: {
          operation: [
            'next if (%{imm} & 0x1f) == 0',
            'offset = %{DISP:u16}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'a = RAM:u16[effective_address]',
            'b = %{imm} & 0x1f',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = alu_result',
          ],
        },
      },
      undefined: ['AF', 'OF'],
      opcode: [
        Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EW_DB,
        'ModRM_110_101_00',
        'DISP_i16',
        'IMM_u8',
      ],
      operands: ['rm', 'imm'],
      operandSize: 16,
      cycles: 8,
    },
    {
      modes: {
        real: {
          operation: [
            'next if (%{imm} & 0x1f) == 0',
            'offset = ${MOD_RM_OFFSET}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'a = RAM:u16[effective_address]',
            'b = %{imm} & 0x1f',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = alu_result',
          ],
        },
        protected: {
          operation: [
            'next if (%{imm} & 0x1f) == 0',
            'offset = ${MOD_RM_OFFSET}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'a = RAM:u16[effective_address]',
            'b = %{imm} & 0x1f',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = alu_result',
          ],
        },
      },
      undefined: ['AF', 'OF'],
      opcode: [
        Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EW_DB,
        'ModRM_rm_101_00',
        'IMM_u8',
      ],
      operands: ['rm', 'imm'],
      operandSize: 16,
      cycles: 8,
    },
    {
      modes: {
        real: {
          operation: [
            'next if (%{imm} & 0x1f) == 0',
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'a = RAM:u16[effective_address]',
            'b = %{imm} & 0x1f',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = alu_result',
          ],
        },
        protected: {
          operation: [
            'next if (%{imm} & 0x1f) == 0',
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'a = RAM:u16[effective_address]',
            'b = %{imm} & 0x1f',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = alu_result',
          ],
        },
      },
      undefined: ['AF', 'OF'],
      opcode: [
        Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EW_DB,
        'ModRM_rm_101_01',
        'DISP_i8',
        'IMM_u8',
      ],
      operands: ['rm', 'imm'],
      operandSize: 16,
      cycles: 8,
    },
    {
      modes: {
        real: {
          operation: [
            'next if (%{imm} & 0x1f) == 0',
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'a = RAM:u16[effective_address]',
            'b = %{imm} & 0x1f',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = alu_result',
          ],
        },
        protected: {
          operation: [
            'next if (%{imm} & 0x1f) == 0',
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'a = RAM:u16[effective_address]',
            'b = %{imm} & 0x1f',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = alu_result',
          ],
        },
      },
      undefined: ['AF', 'OF'],
      opcode: [
        Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EW_DB,
        'ModRM_rm_101_10',
        'DISP_i16',
        'IMM_u8',
      ],
      operands: ['rm', 'imm'],
      operandSize: 16,
      cycles: 8,
    },
    {
      operation: [
        'next if (%{imm} & 0x1f) == 0',
        'a = ${MOD_RM_RM16}',
        'b = %{imm} & 0x1f',
        '${ALU16_OP}',
        '${MOD_RM_RM16} = alu_result',
      ],
      undefined: ['AF', 'OF'],
      opcode: [
        Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EW_DB,
        'ModRM_rm16_101_11',
        'IMM_u8',
      ],
      operands: ['rm', 'imm'],
      operandSize: 16,
      cycles: 5,
    },
  ],
};
