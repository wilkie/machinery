import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

// #13 for offset of 0xffff
export const or: InstructionInfo = {
  identifier: 'or',
  name: 'Logical OR',
  description:
    'This instruction computes the inclusive OR of the two operands. Each bit of the result is 0 if both corresponding bits of the operands are 0; each bit is 1 otherwise. The result is placed in the; first operand.',
  modifies: ['OF', 'SF', 'ZF', 'PF', 'CF'],
  undefined: ['AF'],
  macros: {
    ALU8_OP: [
      // OF = 0, CF = 0
      'alu_result = a | b',
      'flag_op = ${FLAG_OP_LOGIC} | ${FLAG_OP_8BIT}',
    ],
    ALU16_OP: [
      // OF = 0, CF = 0
      'alu_result = a | b',
      'flag_op = ${FLAG_OP_LOGIC} | ${FLAG_OP_16BIT}',
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
    // 0x08 /r - OR eb, rb
    {
      modes: {
        real: {
          operation: [
            'effective_address = ${MOD_RM_SEGMENT} + %{DISP}',
            'a = RAM:u8[effective_address]',
            'b = ${MOD_RM_REG8}',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = alu_result',
          ],
        },
        protected: {
          operation: [
            'offset = %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'a = RAM:u8[effective_address]',
            'b = ${MOD_RM_REG8}',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = alu_result',
          ],
        },
      },
      opcode: [Opcodes.OR_EB_RB, 'ModRM_110_reg8_00', 'DISP_i16'],
      operands: ['rm', 'reg'],
      operandSize: 8,
      cycles: 7,
    },
    {
      modes: {
        real: {
          operation: [
            'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET}',
            'a = RAM:u8[effective_address]',
            'b = ${MOD_RM_REG8}',
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
            'b = ${MOD_RM_REG8}',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = alu_result',
          ],
        },
      },
      opcode: [Opcodes.OR_EB_RB, 'ModRM_rm_reg8_00'],
      operands: ['rm', 'reg'],
      operandSize: 8,
      cycles: 7,
    },
    {
      modes: {
        real: {
          operation: [
            'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
            'a = RAM:u8[effective_address]',
            'b = ${MOD_RM_REG8}',
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
            'b = ${MOD_RM_REG8}',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = alu_result',
          ],
        },
      },
      opcode: [Opcodes.OR_EB_RB, 'ModRM_rm_reg8_01', 'DISP_i8'],
      operands: ['rm', 'reg'],
      operandSize: 8,
      cycles: 7,
    },
    {
      modes: {
        real: {
          operation: [
            'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
            'a = RAM:u8[effective_address]',
            'b = ${MOD_RM_REG8}',
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
            'b = ${MOD_RM_REG8}',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = alu_result',
          ],
        },
      },
      opcode: [Opcodes.OR_EB_RB, 'ModRM_rm_reg8_10', 'DISP_i16'],
      operands: ['rm', 'reg'],
      operandSize: 8,
      cycles: 7,
    },
    {
      operation: [
        'a = ${MOD_RM_RM8}',
        'b = ${MOD_RM_REG8}',
        '${ALU8_OP}',
        '${MOD_RM_RM8} = alu_result',
      ],
      opcode: [Opcodes.OR_EB_RB, 'ModRM_rm8_reg8_11'],
      operands: ['rm', 'reg'],
      operandSize: 8,
      cycles: 2,
    },
    // 0x09 /r - OR ew, rw
    {
      modes: {
        real: {
          operation: [
            'offset = %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'a = RAM:u16[effective_address]',
            'b = ${MOD_RM_REG16}',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = alu_result',
          ],
        },
        protected: {
          operation: [
            'offset = %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'a = RAM:u16[effective_address]',
            'b = ${MOD_RM_REG16}',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = alu_result',
          ],
        },
      },
      opcode: [Opcodes.OR_EW_RW, 'ModRM_110_reg16_00', 'DISP_i16'],
      operands: ['rm', 'reg'],
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
            'b = ${MOD_RM_REG16}',
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
            'b = ${MOD_RM_REG16}',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = alu_result',
          ],
        },
      },
      opcode: [Opcodes.OR_EW_RW, 'ModRM_rm_reg16_00'],
      operands: ['rm', 'reg'],
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
            'b = ${MOD_RM_REG16}',
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
            'b = ${MOD_RM_REG16}',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = alu_result',
          ],
        },
      },
      opcode: [Opcodes.OR_EW_RW, 'ModRM_rm_reg16_01', 'DISP_i8'],
      operands: ['rm', 'reg'],
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
            'b = ${MOD_RM_REG16}',
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
            'b = ${MOD_RM_REG16}',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = alu_result',
          ],
        },
      },
      opcode: [Opcodes.OR_EW_RW, 'ModRM_rm_reg16_10', 'DISP_i16'],
      operands: ['rm', 'reg'],
      operandSize: 16,
      cycles: 7,
    },
    {
      operation: [
        'a = ${MOD_RM_RM16}',
        'b = ${MOD_RM_REG16}',
        '${ALU16_OP}',
        '${MOD_RM_RM16} = alu_result',
      ],
      opcode: [Opcodes.OR_EW_RW, 'ModRM_rm16_reg16_11'],
      operands: ['rm', 'reg'],
      operandSize: 16,
      cycles: 2,
    },
    // 0x0A /r - OR rb, eb
    {
      modes: {
        real: {
          operation: [
            'effective_address = ${MOD_RM_SEGMENT} + %{DISP}',
            'a = ${MOD_RM_REG8}',
            'b = RAM:u8[effective_address]',
            '${ALU8_OP}',
            '${MOD_RM_REG8} = alu_result',
          ],
        },
        protected: {
          operation: [
            'offset = %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'a = ${MOD_RM_REG8}',
            'b = RAM:u8[effective_address]',
            '${ALU8_OP}',
            '${MOD_RM_REG8} = alu_result',
          ],
        },
      },
      opcode: [Opcodes.OR_RB_EB, 'ModRM_110_reg8_00', 'DISP_i16'],
      operands: ['reg', 'rm'],
      operandSize: 8,
      cycles: 7,
    },
    {
      modes: {
        real: {
          operation: [
            'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET}',
            'a = ${MOD_RM_REG8}',
            'b = RAM:u8[effective_address]',
            '${ALU8_OP}',
            '${MOD_RM_REG8} = alu_result',
          ],
        },
        protected: {
          operation: [
            'offset = ${MOD_RM_OFFSET}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'a = ${MOD_RM_REG8}',
            'b = RAM:u8[effective_address]',
            '${ALU8_OP}',
            '${MOD_RM_REG8} = alu_result',
          ],
        },
      },
      opcode: [Opcodes.OR_RB_EB, 'ModRM_rm_reg8_00'],
      operands: ['reg', 'rm'],
      operandSize: 8,
      cycles: 7,
    },
    {
      modes: {
        real: {
          operation: [
            'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
            'a = ${MOD_RM_REG8}',
            'b = RAM:u8[effective_address]',
            '${ALU8_OP}',
            '${MOD_RM_REG8} = alu_result',
          ],
        },
        protected: {
          operation: [
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'a = ${MOD_RM_REG8}',
            'b = RAM:u8[effective_address]',
            '${ALU8_OP}',
            '${MOD_RM_REG8} = alu_result',
          ],
        },
      },
      opcode: [Opcodes.OR_RB_EB, 'ModRM_rm_reg8_01', 'DISP_i8'],
      operands: ['reg', 'rm'],
      operandSize: 8,
      cycles: 7,
    },
    {
      modes: {
        real: {
          operation: [
            'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
            'a = ${MOD_RM_REG8}',
            'b = RAM:u8[effective_address]',
            '${ALU8_OP}',
            '${MOD_RM_REG8} = alu_result',
          ],
        },
        protected: {
          operation: [
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'a = ${MOD_RM_REG8}',
            'b = RAM:u8[effective_address]',
            '${ALU8_OP}',
            '${MOD_RM_REG8} = alu_result',
          ],
        },
      },
      opcode: [Opcodes.OR_RB_EB, 'ModRM_rm_reg8_10', 'DISP_i16'],
      operands: ['reg', 'rm'],
      operandSize: 8,
      cycles: 7,
    },
    {
      operation: [
        'a = ${MOD_RM_REG8}',
        'b = ${MOD_RM_RM8}',
        '${ALU8_OP}',
        '${MOD_RM_REG8} = alu_result',
      ],
      opcode: [Opcodes.OR_RB_EB, 'ModRM_rm8_reg8_11'],
      operands: ['reg', 'rm'],
      operandSize: 8,
      cycles: 2,
    },
    // 0x0B /r - OR rw, ew
    {
      modes: {
        real: {
          operation: [
            'offset = %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'a = ${MOD_RM_REG16}',
            'b = RAM:u16[effective_address]',
            '${ALU16_OP}',
            '${MOD_RM_REG16} = alu_result',
          ],
        },
        protected: {
          operation: [
            'offset = %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'a = ${MOD_RM_REG16}',
            'b = RAM:u16[effective_address]',
            '${ALU16_OP}',
            '${MOD_RM_REG16} = alu_result',
          ],
        },
      },
      opcode: [Opcodes.OR_RW_EW, 'ModRM_110_reg16_00', 'DISP_i16'],
      operands: ['reg', 'rm'],
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
            'a = ${MOD_RM_REG16}',
            'b = RAM:u16[effective_address]',
            '${ALU16_OP}',
            '${MOD_RM_REG16} = alu_result',
          ],
        },
        protected: {
          operation: [
            'offset = ${MOD_RM_OFFSET}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'a = ${MOD_RM_REG16}',
            'b = RAM:u16[effective_address]',
            '${ALU16_OP}',
            '${MOD_RM_REG16} = alu_result',
          ],
        },
      },
      opcode: [Opcodes.OR_RW_EW, 'ModRM_rm_reg16_00'],
      operands: ['reg', 'rm'],
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
            'a = ${MOD_RM_REG16}',
            'b = RAM:u16[effective_address]',
            '${ALU16_OP}',
            '${MOD_RM_REG16} = alu_result',
          ],
        },
        protected: {
          operation: [
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'a = ${MOD_RM_REG16}',
            'b = RAM:u16[effective_address]',
            '${ALU16_OP}',
            '${MOD_RM_REG16} = alu_result',
          ],
        },
      },
      opcode: [Opcodes.OR_RW_EW, 'ModRM_rm_reg16_01', 'DISP_i8'],
      operands: ['reg', 'rm'],
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
            'a = ${MOD_RM_REG16}',
            'b = RAM:u16[effective_address]',
            '${ALU16_OP}',
            '${MOD_RM_REG16} = alu_result',
          ],
        },
        protected: {
          operation: [
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'a = ${MOD_RM_REG16}',
            'b = RAM:u16[effective_address]',
            '${ALU16_OP}',
            '${MOD_RM_REG16} = alu_result',
          ],
        },
      },
      opcode: [Opcodes.OR_RW_EW, 'ModRM_rm_reg16_10', 'DISP_i16'],
      operands: ['reg', 'rm'],
      operandSize: 16,
      cycles: 7,
    },
    {
      operation: [
        'a = ${MOD_RM_REG16}',
        'b = ${MOD_RM_RM16}',
        '${ALU16_OP}',
        '${MOD_RM_REG16} = alu_result',
      ],
      opcode: [Opcodes.OR_RW_EW, 'ModRM_rm16_reg16_11'],
      operands: ['reg', 'rm'],
      operandSize: 16,
      cycles: 2,
    },
    // 0x0C db - OR AL, db
    {
      operation: ['a = AL', 'b = %{imm}', '${ALU8_OP}', 'AL = alu_result'],
      opcode: [Opcodes.OR_AL_DB, 'IMM_i8'],
      operands: ['AL', 'imm'],
      operandSize: 8,
      encodingPriority: 1,
      cycles: 3,
    },
    // 0x0D dw - OR AX, dw
    {
      operation: ['a = AX', 'b = %{imm}', '${ALU16_OP}', 'AX = alu_result'],
      opcode: [Opcodes.OR_AX_DW, 'IMM_u16'],
      operands: ['AX', 'imm'],
      operandSize: 16,
      encodingPriority: 1,
      cycles: 3,
    },
    // 0x80 /1 db - OR eb, db
    {
      modes: {
        real: {
          operation: [
            'effective_address = ${MOD_RM_SEGMENT} + %{DISP}',
            'a = RAM:u8[effective_address]',
            'b = %{imm}',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = alu_result',
          ],
        },
        protected: {
          operation: [
            'offset = %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'a = RAM:u8[effective_address]',
            'b = %{imm}',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = alu_result',
          ],
        },
      },
      opcode: [Opcodes.ALU_EB_DB, 'ModRM_110_001_00', 'DISP_i16', 'IMM_i8'],
      operands: ['rm', 'imm'],
      operandSize: 8,
      cycles: 7,
    },
    {
      modes: {
        real: {
          operation: [
            'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET}',
            'a = RAM:u8[effective_address]',
            'b = %{imm}',
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
            'b = %{imm}',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = alu_result',
          ],
        },
      },
      opcode: [Opcodes.ALU_EB_DB, 'ModRM_rm_001_00', 'IMM_i8'],
      operands: ['rm', 'imm'],
      operandSize: 8,
      cycles: 7,
    },
    {
      modes: {
        real: {
          operation: [
            'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
            'a = RAM:u8[effective_address]',
            'b = %{imm}',
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
            'b = %{imm}',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = alu_result',
          ],
        },
      },
      opcode: [Opcodes.ALU_EB_DB, 'ModRM_rm_001_01', 'DISP_i8', 'IMM_i8'],
      operands: ['rm', 'imm'],
      operandSize: 8,
      cycles: 7,
    },
    {
      modes: {
        real: {
          operation: [
            'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
            'a = RAM:u8[effective_address]',
            'b = %{imm}',
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
            'b = %{imm}',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = alu_result',
          ],
        },
      },
      opcode: [Opcodes.ALU_EB_DB, 'ModRM_rm_001_10', 'DISP_i16', 'IMM_i8'],
      operands: ['rm', 'imm'],
      operandSize: 8,
      cycles: 7,
    },
    {
      operation: [
        'a = ${MOD_RM_RM8}',
        'b = %{imm}',
        '${ALU8_OP}',
        '${MOD_RM_RM8} = alu_result',
      ],
      opcode: [Opcodes.ALU_EB_DB, 'ModRM_rm8_001_11', 'IMM_i8'],
      operands: ['rm', 'imm'],
      operandSize: 8,
      cycles: 3,
    },
    // 0x81 /1 dw - OR ew, dw
    {
      modes: {
        real: {
          operation: [
            'offset = %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'a = RAM:u16[effective_address]',
            'b = %{imm}',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = alu_result',
          ],
        },
        protected: {
          operation: [
            'offset = %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'a = RAM:u16[effective_address]',
            'b = %{imm}',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = alu_result',
          ],
        },
      },
      opcode: [Opcodes.ALU_EW_DW, 'ModRM_110_001_00', 'DISP_i16', 'IMM_u16'],
      operands: ['rm', 'imm'],
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
            'b = %{imm}',
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
            'b = %{imm}',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = alu_result',
          ],
        },
      },
      opcode: [Opcodes.ALU_EW_DW, 'ModRM_rm_001_00', 'IMM_u16'],
      operands: ['rm', 'imm'],
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
            'b = %{imm}',
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
            'b = %{imm}',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = alu_result',
          ],
        },
      },
      opcode: [Opcodes.ALU_EW_DW, 'ModRM_rm_001_01', 'DISP_i8', 'IMM_u16'],
      operands: ['rm', 'imm'],
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
            'b = %{imm}',
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
            'b = %{imm}',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = alu_result',
          ],
        },
      },
      opcode: [Opcodes.ALU_EW_DW, 'ModRM_rm_001_10', 'DISP_i16', 'IMM_u16'],
      operands: ['rm', 'imm'],
      operandSize: 16,
      cycles: 7,
    },
    {
      operation: [
        'a = ${MOD_RM_RM16}',
        'b = %{imm}',
        '${ALU16_OP}',
        '${MOD_RM_RM16} = alu_result',
      ],
      opcode: [Opcodes.ALU_EW_DW, 'ModRM_rm16_001_11', 'IMM_u16'],
      operands: ['rm', 'imm'],
      operandSize: 16,
      cycles: 3,
    },
    // 0x83 /1 db - OR ew, db
    {
      modes: {
        real: {
          operation: [
            'offset = %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'a = RAM:u16[effective_address]',
            'b = %{imm}',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = alu_result',
          ],
        },
        protected: {
          operation: [
            'offset = %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'a = RAM:u16[effective_address]',
            'b = %{imm}',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = alu_result',
          ],
        },
      },
      opcode: [Opcodes.ALU_EW_DB, 'ModRM_110_001_00', 'DISP_i16', 'IMM_i8'],
      operands: ['rm', 'imm'],
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
            'b = %{imm}',
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
            'b = %{imm}',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = alu_result',
          ],
        },
      },
      opcode: [Opcodes.ALU_EW_DB, 'ModRM_rm_001_00', 'IMM_i8'],
      operands: ['rm', 'imm'],
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
            'b = %{imm}',
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
            'b = %{imm}',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = alu_result',
          ],
        },
      },
      opcode: [Opcodes.ALU_EW_DB, 'ModRM_rm_001_01', 'DISP_i8', 'IMM_i8'],
      operands: ['rm', 'imm'],
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
            'b = %{imm}',
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
            'b = %{imm}',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = alu_result',
          ],
        },
      },
      opcode: [Opcodes.ALU_EW_DB, 'ModRM_rm_001_10', 'DISP_i16', 'IMM_i8'],
      operands: ['rm', 'imm'],
      operandSize: 16,
      cycles: 7,
    },
    {
      operation: [
        'a = ${MOD_RM_RM16}',
        'b = %{imm}',
        '${ALU16_OP}',
        '${MOD_RM_RM16} = alu_result',
      ],
      opcode: [Opcodes.ALU_EW_DB, 'ModRM_rm16_001_11', 'IMM_i8'],
      operands: ['rm', 'imm'],
      operandSize: 16,
      cycles: 3,
    },
  ],
};
