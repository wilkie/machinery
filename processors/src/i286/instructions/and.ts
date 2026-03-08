import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const and: InstructionInfo = {
  identifier: 'and',
  name: 'Logical AND',
  description:
    'Each bit of the result is a 1 if both corresponding bits of the operands were 1; it is 0 otherwise.',
  modifies: ['OF', 'SF', 'ZF', 'PF', 'CF'],
  undefined: ['AF'],
  macros: {
    ALU8_OP: [
      // OF = 0, CF = 0
      'alu_result = a & b',
      'flag_op = ${FLAG_OP_LOGIC} | ${FLAG_OP_8BIT}',
    ],
    ALU16_OP: [
      // OF = 0, CF = 0
      'alu_result = a & b',
      'flag_op = ${FLAG_OP_LOGIC} | ${FLAG_OP_16BIT}',
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
    // 0x20 /r - AND eb, rb
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + %{DISP}',
        'a = RAM:u8[effective_address]',
        'b = ${MOD_RM_REG8}',
        '${ALU8_OP}',
        'RAM:u8[effective_address] = alu_result',
      ],
      opcode: [Opcodes.AND_EB_RB, 'ModRM_110_reg_00', 'DISP_i16'],
      operandSize: 8,
      cycles: 7,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET}',
        'a = RAM:u8[effective_address]',
        'b = ${MOD_RM_REG8}',
        '${ALU8_OP}',
        'RAM:u8[effective_address] = alu_result',
      ],
      opcode: [Opcodes.AND_EB_RB, 'ModRM_rm_reg_00'],
      operandSize: 8,
      cycles: 7,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'a = RAM:u8[effective_address]',
        'b = ${MOD_RM_REG8}',
        '${ALU8_OP}',
        'RAM:u8[effective_address] = alu_result',
      ],
      opcode: [Opcodes.AND_EB_RB, 'ModRM_rm_reg_01', 'DISP_i8'],
      operandSize: 8,
      cycles: 7,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'a = RAM:u8[effective_address]',
        'b = ${MOD_RM_REG8}',
        '${ALU8_OP}',
        'RAM:u8[effective_address] = alu_result',
      ],
      opcode: [Opcodes.AND_EB_RB, 'ModRM_rm_reg_10', 'DISP_i16'],
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
      opcode: [Opcodes.AND_EB_RB, 'ModRM_rm_reg_11'],
      operandSize: 8,
      cycles: 2,
    },
    // 0x21 /r - AND ew, rw
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + %{DISP}',
        'a = RAM:u16[effective_address]',
        'b = ${MOD_RM_REG16}',
        '${ALU16_OP}',
        'RAM:u16[effective_address] = alu_result',
      ],
      opcode: [Opcodes.AND_EW_RW, 'ModRM_110_reg_00', 'DISP_i16'],
      operandSize: 16,
      cycles: 7,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET}',
        'a = RAM:u16[effective_address]',
        'b = ${MOD_RM_REG16}',
        '${ALU16_OP}',
        'RAM:u16[effective_address] = alu_result',
      ],
      opcode: [Opcodes.AND_EW_RW, 'ModRM_rm_reg_00'],
      operandSize: 16,
      cycles: 7,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'a = RAM:u16[effective_address]',
        'b = ${MOD_RM_REG16}',
        '${ALU16_OP}',
        'RAM:u16[effective_address] = alu_result',
      ],
      opcode: [Opcodes.AND_EW_RW, 'ModRM_rm_reg_01', 'DISP_i8'],
      operandSize: 16,
      cycles: 7,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'a = RAM:u16[effective_address]',
        'b = ${MOD_RM_REG16}',
        '${ALU16_OP}',
        'RAM:u16[effective_address] = alu_result',
      ],
      opcode: [Opcodes.AND_EW_RW, 'ModRM_rm_reg_10', 'DISP_i16'],
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
      opcode: [Opcodes.AND_EW_RW, 'ModRM_rm_reg_11'],
      operandSize: 16,
      cycles: 2,
    },
    // 0x22 /r - AND rb, eb
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + %{DISP}',
        'a = ${MOD_RM_REG8}',
        'b = RAM:u8[effective_address]',
        '${ALU8_OP}',
        '${MOD_RM_REG8} = alu_result',
      ],
      opcode: [Opcodes.AND_RB_EB, 'ModRM_110_reg_00', 'DISP_i16'],
      operandSize: 8,
      cycles: 7,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET}',
        'a = ${MOD_RM_REG8}',
        'b = RAM:u8[effective_address]',
        '${ALU8_OP}',
        '${MOD_RM_REG8} = alu_result',
      ],
      opcode: [Opcodes.AND_RB_EB, 'ModRM_rm_reg_00'],
      operandSize: 8,
      cycles: 7,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'a = ${MOD_RM_REG8}',
        'b = RAM:u8[effective_address]',
        '${ALU8_OP}',
        '${MOD_RM_REG8} = alu_result',
      ],
      opcode: [Opcodes.AND_RB_EB, 'ModRM_rm_reg_01', 'DISP_i8'],
      operandSize: 8,
      cycles: 7,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'a = ${MOD_RM_REG8}',
        'b = RAM:u8[effective_address]',
        '${ALU8_OP}',
        '${MOD_RM_REG8} = alu_result',
      ],
      opcode: [Opcodes.AND_RB_EB, 'ModRM_rm_reg_10', 'DISP_i16'],
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
      opcode: [Opcodes.AND_RB_EB, 'ModRM_rm_reg_11'],
      operandSize: 8,
      cycles: 2,
    },
    // 0x23 /r - AND rw, ew
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + %{DISP}',
        'a = ${MOD_RM_REG16}',
        'b = RAM:u16[effective_address]',
        '${ALU16_OP}',
        '${MOD_RM_REG16} = alu_result',
      ],
      opcode: [Opcodes.AND_RW_EW, 'ModRM_110_reg_00', 'DISP_i16'],
      operandSize: 16,
      cycles: 7,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET}',
        'a = ${MOD_RM_REG16}',
        'b = RAM:u16[effective_address]',
        '${ALU16_OP}',
        '${MOD_RM_REG16} = alu_result',
      ],
      opcode: [Opcodes.AND_RW_EW, 'ModRM_rm_reg_00'],
      operandSize: 16,
      cycles: 7,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'a = ${MOD_RM_REG16}',
        'b = RAM:u16[effective_address]',
        '${ALU16_OP}',
        '${MOD_RM_REG16} = alu_result',
      ],
      opcode: [Opcodes.AND_RW_EW, 'ModRM_rm_reg_01', 'DISP_i8'],
      operandSize: 16,
      cycles: 7,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'a = ${MOD_RM_REG16}',
        'b = RAM:u16[effective_address]',
        '${ALU16_OP}',
        '${MOD_RM_REG16} = alu_result',
      ],
      opcode: [Opcodes.AND_RW_EW, 'ModRM_rm_reg_10', 'DISP_i16'],
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
      opcode: [Opcodes.AND_RW_EW, 'ModRM_rm_reg_11'],
      operandSize: 16,
      cycles: 2,
    },
    // 0x24 db - AND AL, db
    {
      operation: ['a = AL', 'b = %{IMM}', '${ALU8_OP}', 'AL = alu_result'],
      opcode: [Opcodes.AND_AL_DB, 'IMM_i8'],
      operandSize: 8,
      cycles: 3,
    },
    // 0x25 dw - AND AX, dw
    {
      operation: ['a = AX', 'b = %{IMM}', '${ALU16_OP}', 'AX = alu_result'],
      opcode: [Opcodes.AND_AX_DW, 'IMM_u16'],
      operandSize: 16,
      cycles: 3,
    },
    // 0x80 /4 db - AND eb, db
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + %{DISP}',
        'a = RAM:u8[effective_address]',
        'b = %{IMM}',
        '${ALU8_OP}',
        'RAM:u8[effective_address] = alu_result',
      ],
      opcode: [Opcodes.ALU_EB_DB, 'ModRM_110_100_00', 'DISP_i16', 'IMM_i8'],
      operandSize: 8,
      cycles: 7,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET}',
        'a = RAM:u8[effective_address]',
        'b = %{IMM}',
        '${ALU8_OP}',
        'RAM:u8[effective_address] = alu_result',
      ],
      opcode: [Opcodes.ALU_EB_DB, 'ModRM_rm_100_00', 'IMM_i8'],
      operandSize: 8,
      cycles: 7,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'a = RAM:u8[effective_address]',
        'b = %{IMM}',
        '${ALU8_OP}',
        'RAM:u8[effective_address] = alu_result',
      ],
      opcode: [Opcodes.ALU_EB_DB, 'ModRM_rm_100_01', 'DISP_i8', 'IMM_i8'],
      operandSize: 8,
      cycles: 7,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'a = RAM:u8[effective_address]',
        'b = %{IMM}',
        '${ALU8_OP}',
        'RAM:u8[effective_address] = alu_result',
      ],
      opcode: [Opcodes.ALU_EB_DB, 'ModRM_rm_100_10', 'DISP_i16', 'IMM_i8'],
      operandSize: 8,
      cycles: 7,
    },
    {
      operation: [
        'a = ${MOD_RM_RM8}',
        'b = %{IMM}',
        '${ALU8_OP}',
        '${MOD_RM_RM8} = alu_result',
      ],
      opcode: [Opcodes.ALU_EB_DB, 'ModRM_rm_100_11', 'IMM_i8'],
      operandSize: 8,
      cycles: 3,
    },
    // 0x81 /4 dw - AND ew, dw
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + %{DISP}',
        'a = RAM:u16[effective_address]',
        'b = %{IMM}',
        '${ALU16_OP}',
        'RAM:u16[effective_address] = alu_result',
      ],
      opcode: [Opcodes.ALU_EW_DW, 'ModRM_110_100_00', 'DISP_i16', 'IMM_u16'],
      operandSize: 16,
      cycles: 7,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET}',
        'a = RAM:u16[effective_address]',
        'b = %{IMM}',
        '${ALU16_OP}',
        'RAM:u16[effective_address] = alu_result',
      ],
      opcode: [Opcodes.ALU_EW_DW, 'ModRM_rm_100_00', 'IMM_u16'],
      operandSize: 16,
      cycles: 7,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'a = RAM:u16[effective_address]',
        'b = %{IMM}',
        '${ALU16_OP}',
        'RAM:u16[effective_address] = alu_result',
      ],
      opcode: [Opcodes.ALU_EW_DW, 'ModRM_rm_100_01', 'DISP_i8', 'IMM_u16'],
      operandSize: 16,
      cycles: 7,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'a = RAM:u16[effective_address]',
        'b = %{IMM}',
        '${ALU16_OP}',
        'RAM:u16[effective_address] = alu_result',
      ],
      opcode: [Opcodes.ALU_EW_DW, 'ModRM_rm_100_10', 'DISP_i16', 'IMM_u16'],
      operandSize: 16,
      cycles: 7,
    },
    {
      operation: [
        'a = ${MOD_RM_RM16}',
        'b = %{IMM}',
        '${ALU16_OP}',
        '${MOD_RM_RM16} = alu_result',
      ],
      opcode: [Opcodes.ALU_EW_DW, 'ModRM_rm_100_11', 'IMM_u16'],
      operandSize: 16,
      cycles: 3,
    },
    // 0x83 /4 db - AND ew, db
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + %{DISP}',
        'a = RAM:u16[effective_address]',
        'b = %{IMM}',
        '${ALU16_OP}',
        'RAM:u16[effective_address] = alu_result',
      ],
      opcode: [Opcodes.ALU_EW_DB, 'ModRM_110_100_00', 'DISP_i16', 'IMM_i8'],
      operandSize: 16,
      cycles: 7,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET}',
        'a = RAM:u16[effective_address]',
        'b = %{IMM}',
        '${ALU16_OP}',
        'RAM:u16[effective_address] = alu_result',
      ],
      opcode: [Opcodes.ALU_EW_DB, 'ModRM_rm_100_00', 'IMM_i8'],
      operandSize: 16,
      cycles: 7,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'a = RAM:u16[effective_address]',
        'b = %{IMM}',
        '${ALU16_OP}',
        'RAM:u16[effective_address] = alu_result',
      ],
      opcode: [Opcodes.ALU_EW_DB, 'ModRM_rm_100_01', 'DISP_i8', 'IMM_i8'],
      operandSize: 16,
      cycles: 7,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'a = RAM:u16[effective_address]',
        'b = %{IMM}',
        '${ALU16_OP}',
        'RAM:u16[effective_address] = alu_result',
      ],
      opcode: [Opcodes.ALU_EW_DB, 'ModRM_rm_100_10', 'DISP_i16', 'IMM_i8'],
      operandSize: 16,
      cycles: 7,
    },
    {
      operation: [
        'a = ${MOD_RM_RM16}',
        'b = %{IMM}',
        '${ALU16_OP}',
        '${MOD_RM_RM16} = alu_result',
      ],
      opcode: [Opcodes.ALU_EW_DB, 'ModRM_rm_100_11', 'IMM_i8'],
      operandSize: 16,
      cycles: 3,
    },
  ],
};
