import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

// #13 for offset of 0xffff
export const test: InstructionInfo = {
  identifier: 'test',
  name: 'Logical Compare',
  description:
    '`TEST` computes the bit-wise logical `AND` of the two operands given. Each bit of the result is 1 if both of the corresponding bits of the operands are 1; each bit is 0 otherwise. The result of the operation is discarded; only the flags are modified.',
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
    // 0x84 /r - TEST eb, rb
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + %{DISP}',
        'a = RAM:u8[effective_address]',
        'b = ${MOD_RM_REG8}',
        '${ALU8_OP}',
      ],
      opcode: [Opcodes.TEST_EB_RB, 'ModRM_110_reg8_00', 'DISP_i16'],
      operands: ['rm', 'reg'],
      operandSize: 8,
      cycles: 6,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET}',
        'a = RAM:u8[effective_address]',
        'b = ${MOD_RM_REG8}',
        '${ALU8_OP}',
      ],
      opcode: [Opcodes.TEST_EB_RB, 'ModRM_rm_reg8_00'],
      operands: ['rm', 'reg'],
      operandSize: 8,
      cycles: 6,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'a = RAM:u8[effective_address]',
        'b = ${MOD_RM_REG8}',
        '${ALU8_OP}',
      ],
      opcode: [Opcodes.TEST_EB_RB, 'ModRM_rm_reg8_01', 'DISP_i8'],
      operands: ['rm', 'reg'],
      operandSize: 8,
      cycles: 6,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'a = RAM:u8[effective_address]',
        'b = ${MOD_RM_REG8}',
        '${ALU8_OP}',
      ],
      opcode: [Opcodes.TEST_EB_RB, 'ModRM_rm_reg8_10', 'DISP_i16'],
      operands: ['rm', 'reg'],
      operandSize: 8,
      cycles: 6,
    },
    {
      operation: ['a = ${MOD_RM_RM8}', 'b = ${MOD_RM_REG8}', '${ALU8_OP}'],
      opcode: [Opcodes.TEST_EB_RB, 'ModRM_rm8_reg8_11'],
      operands: ['rm', 'reg'],
      operandSize: 8,
      cycles: 2,
    },
    // 0x85 /r - TEST ew, rw
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + %{DISP}',
        'a = RAM:u16[effective_address]',
        'b = ${MOD_RM_REG16}',
        '${ALU16_OP}',
      ],
      opcode: [Opcodes.TEST_EW_RW, 'ModRM_110_reg16_00', 'DISP_i16'],
      operands: ['rm', 'reg'],
      operandSize: 16,
      cycles: 6,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET}',
        'a = RAM:u16[effective_address]',
        'b = ${MOD_RM_REG16}',
        '${ALU16_OP}',
      ],
      opcode: [Opcodes.TEST_EW_RW, 'ModRM_rm_reg16_00'],
      operands: ['rm', 'reg'],
      operandSize: 16,
      cycles: 6,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'a = RAM:u16[effective_address]',
        'b = ${MOD_RM_REG16}',
        '${ALU16_OP}',
      ],
      opcode: [Opcodes.TEST_EW_RW, 'ModRM_rm_reg16_01', 'DISP_i8'],
      operands: ['rm', 'reg'],
      operandSize: 16,
      cycles: 6,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'a = RAM:u16[effective_address]',
        'b = ${MOD_RM_REG16}',
        '${ALU16_OP}',
      ],
      opcode: [Opcodes.TEST_EW_RW, 'ModRM_rm_reg16_10', 'DISP_i16'],
      operands: ['rm', 'reg'],
      operandSize: 16,
      cycles: 6,
    },
    {
      operation: ['a = ${MOD_RM_RM16}', 'b = ${MOD_RM_REG16}', '${ALU16_OP}'],
      opcode: [Opcodes.TEST_EW_RW, 'ModRM_rm16_reg16_11'],
      operands: ['rm', 'reg'],
      operandSize: 16,
      cycles: 2,
    },
    // 0xA8 db - TEST AL, db
    {
      operation: ['a = AL', 'b = %{imm}', '${ALU8_OP}'],
      opcode: [Opcodes.TEST_AL_DB, 'IMM_u8'],
      operands: ['AL', 'imm'],
      operandSize: 8,
      encodingPriority: 1,
      cycles: 3,
    },
    // 0xA9 dw - TEST AX, dw
    {
      operation: ['a = AX', 'b = %{imm}', '${ALU16_OP}'],
      opcode: [Opcodes.TEST_AX_DW, 'IMM_u16'],
      operands: ['AX', 'imm'],
      operandSize: 16,
      encodingPriority: 1,
      cycles: 3,
    },
    // 0xF6 /0 db - TEST eb, db
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + %{DISP}',
        'a = RAM:u8[effective_address]',
        'b = %{imm}',
        '${ALU8_OP}',
      ],
      opcode: [Opcodes.ALU_LOGIC_EB, 'ModRM_110_000_00', 'DISP_i16', 'IMM_u8'],
      operands: ['rm', 'imm'],
      operandSize: 8,
      cycles: 6,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET}',
        'a = RAM:u8[effective_address]',
        'b = %{imm}',
        '${ALU8_OP}',
      ],
      opcode: [Opcodes.ALU_LOGIC_EB, 'ModRM_rm_000_00', 'IMM_u8'],
      operands: ['rm', 'imm'],
      operandSize: 8,
      cycles: 6,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'a = RAM:u8[effective_address]',
        'b = %{imm}',
        '${ALU8_OP}',
      ],
      opcode: [Opcodes.ALU_LOGIC_EB, 'ModRM_rm_000_01', 'DISP_i8', 'IMM_u8'],
      operands: ['rm', 'imm'],
      operandSize: 8,
      cycles: 6,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'a = RAM:u8[effective_address]',
        'b = %{imm}',
        '${ALU8_OP}',
      ],
      opcode: [Opcodes.ALU_LOGIC_EB, 'ModRM_rm_000_10', 'DISP_i16', 'IMM_u8'],
      operands: ['rm', 'imm'],
      operandSize: 8,
      cycles: 6,
    },
    {
      operation: ['a = ${MOD_RM_RM8}', 'b = %{imm}', '${ALU8_OP}'],
      opcode: [Opcodes.ALU_LOGIC_EB, 'ModRM_rm8_000_11', 'IMM_u8'],
      operands: ['rm', 'imm'],
      operandSize: 8,
      cycles: 3,
    },
    // 0xF7 /0 dw - TEST ew, dw
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + %{DISP}',
        'a = RAM:u16[effective_address]',
        'b = %{imm}',
        '${ALU16_OP}',
      ],
      opcode: [Opcodes.ALU_LOGIC_EW, 'ModRM_110_000_00', 'DISP_i16', 'IMM_u16'],
      operands: ['rm', 'imm'],
      operandSize: 16,
      cycles: 6,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET}',
        'a = RAM:u16[effective_address]',
        'b = %{imm}',
        '${ALU16_OP}',
      ],
      opcode: [Opcodes.ALU_LOGIC_EW, 'ModRM_rm_000_00', 'IMM_u16'],
      operands: ['rm', 'imm'],
      operandSize: 16,
      cycles: 6,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'a = RAM:u16[effective_address]',
        'b = %{imm}',
        '${ALU16_OP}',
      ],
      opcode: [Opcodes.ALU_LOGIC_EW, 'ModRM_rm_000_01', 'DISP_i8', 'IMM_u16'],
      operands: ['rm', 'imm'],
      operandSize: 16,
      cycles: 6,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'a = RAM:u16[effective_address]',
        'b = %{imm}',
        '${ALU16_OP}',
      ],
      opcode: [Opcodes.ALU_LOGIC_EW, 'ModRM_rm_000_10', 'DISP_i16', 'IMM_u16'],
      operands: ['rm', 'imm'],
      operandSize: 16,
      cycles: 6,
    },
    {
      operation: ['a = ${MOD_RM_RM16}', 'b = %{imm}', '${ALU16_OP}'],
      opcode: [Opcodes.ALU_LOGIC_EW, 'ModRM_rm16_000_11', 'IMM_u16'],
      operands: ['rm', 'imm'],
      operandSize: 16,
      cycles: 3,
    },
  ],
};
