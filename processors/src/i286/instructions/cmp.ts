import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

// #13 for offset 0xffff
export const cmp: InstructionInfo = {
  identifier: 'cmp',
  name: 'Compare Two Operands',
  description:
    '`CMP` subtracts the second operand from the first operand, but it does not place the result anywhere. Only the flags are changed by this instruction. `CMP` is usually followed by a conditional jump instruction. See the `J*` instructions for the list of signed and unsigned flag tests provided by the CPU.\n\nIf a word operand is compared to an immediate byte value, the byte value is first sign-extended.',
  modifies: ['OF', 'SF', 'ZF', 'AF', 'PF', 'CF'],
  undefined: [],
  macros: {
    ALU8_OP: [
      'alu_result = a - b',
      'flag_op = ${FLAG_OP_ALU} | ${FLAG_OP_SUB} | ${FLAG_OP_8BIT}',
    ],
    ALU16_OP: [
      'alu_result = a - b',
      'flag_op = ${FLAG_OP_ALU} | ${FLAG_OP_SUB} | ${FLAG_OP_16BIT}',
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
    // 0x38 /r - CMP eb, rb
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + %{DISP}',
        'a = RAM:u8[effective_address]',
        'b = ${MOD_RM_REG8}',
        '${ALU8_OP}',
      ],
      opcode: [Opcodes.CMP_EB_RB, 'ModRM_110_reg8_00', 'DISP_i16'],
      operands: ['rm', 'reg'],
      operandSize: 8,
      cycles: 7,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET}',
        'a = RAM:u8[effective_address]',
        'b = ${MOD_RM_REG8}',
        '${ALU8_OP}',
      ],
      opcode: [Opcodes.CMP_EB_RB, 'ModRM_rm_reg8_00'],
      operands: ['rm', 'reg'],
      operandSize: 8,
      cycles: 7,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'a = RAM:u8[effective_address]',
        'b = ${MOD_RM_REG8}',
        '${ALU8_OP}',
      ],
      opcode: [Opcodes.CMP_EB_RB, 'ModRM_rm_reg8_01', 'DISP_i8'],
      operands: ['rm', 'reg'],
      operandSize: 8,
      cycles: 7,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'a = RAM:u8[effective_address]',
        'b = ${MOD_RM_REG8}',
        '${ALU8_OP}',
      ],
      opcode: [Opcodes.CMP_EB_RB, 'ModRM_rm_reg8_10', 'DISP_i16'],
      operands: ['rm', 'reg'],
      operandSize: 8,
      cycles: 7,
    },
    {
      operation: ['a = ${MOD_RM_RM8}', 'b = ${MOD_RM_REG8}', '${ALU8_OP}'],
      opcode: [Opcodes.CMP_EB_RB, 'ModRM_rm8_reg8_11'],
      operands: ['rm', 'reg'],
      operandSize: 8,
      cycles: 2,
    },
    // 0x39 /r - CMP ew, rw
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + %{DISP}',
        'a = RAM:u16[effective_address]',
        'b = ${MOD_RM_REG16}',
        '${ALU16_OP}',
      ],
      opcode: [Opcodes.CMP_EW_RW, 'ModRM_110_reg16_00', 'DISP_i16'],
      operands: ['rm', 'reg'],
      operandSize: 16,
      cycles: 7,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET}',
        'a = RAM:u16[effective_address]',
        'b = ${MOD_RM_REG16}',
        '${ALU16_OP}',
      ],
      opcode: [Opcodes.CMP_EW_RW, 'ModRM_rm_reg16_00'],
      operands: ['rm', 'reg'],
      operandSize: 16,
      cycles: 7,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'a = RAM:u16[effective_address]',
        'b = ${MOD_RM_REG16}',
        '${ALU16_OP}',
      ],
      opcode: [Opcodes.CMP_EW_RW, 'ModRM_rm_reg16_01', 'DISP_i8'],
      operands: ['rm', 'reg'],
      operandSize: 16,
      cycles: 7,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'a = RAM:u16[effective_address]',
        'b = ${MOD_RM_REG16}',
        '${ALU16_OP}',
      ],
      opcode: [Opcodes.CMP_EW_RW, 'ModRM_rm_reg16_10', 'DISP_i16'],
      operands: ['rm', 'reg'],
      operandSize: 16,
      cycles: 7,
    },
    {
      operation: ['a = ${MOD_RM_RM16}', 'b = ${MOD_RM_REG16}', '${ALU16_OP}'],
      opcode: [Opcodes.CMP_EW_RW, 'ModRM_rm16_reg16_11'],
      operands: ['rm', 'reg'],
      operandSize: 16,
      cycles: 2,
    },
    // 0x3A /r - CMP rb, eb
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + %{DISP}',
        'a = ${MOD_RM_REG8}',
        'b = RAM:u8[effective_address]',
        '${ALU8_OP}',
      ],
      opcode: [Opcodes.CMP_RB_EB, 'ModRM_110_reg8_00', 'DISP_i16'],
      operands: ['reg', 'rm'],
      operandSize: 8,
      cycles: 6,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET}',
        'a = ${MOD_RM_REG8}',
        'b = RAM:u8[effective_address]',
        '${ALU8_OP}',
      ],
      opcode: [Opcodes.CMP_RB_EB, 'ModRM_rm_reg8_00'],
      operands: ['reg', 'rm'],
      operandSize: 8,
      cycles: 6,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'a = ${MOD_RM_REG8}',
        'b = RAM:u8[effective_address]',
        '${ALU8_OP}',
      ],
      opcode: [Opcodes.CMP_RB_EB, 'ModRM_rm_reg8_01', 'DISP_i8'],
      operands: ['reg', 'rm'],
      operandSize: 8,
      cycles: 6,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'a = ${MOD_RM_REG8}',
        'b = RAM:u8[effective_address]',
        '${ALU8_OP}',
      ],
      opcode: [Opcodes.CMP_RB_EB, 'ModRM_rm_reg8_10', 'DISP_i16'],
      operands: ['reg', 'rm'],
      operandSize: 8,
      cycles: 6,
    },
    {
      operation: ['a = ${MOD_RM_REG8}', 'b = ${MOD_RM_RM8}', '${ALU8_OP}'],
      opcode: [Opcodes.CMP_RB_EB, 'ModRM_rm8_reg8_11'],
      operands: ['reg', 'rm'],
      operandSize: 8,
      cycles: 2,
    },
    // 0x3B /r - CMP rw, ew
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + %{DISP}',
        'a = ${MOD_RM_REG16}',
        'b = RAM:u16[effective_address]',
        '${ALU16_OP}',
      ],
      opcode: [Opcodes.CMP_RW_EW, 'ModRM_110_reg16_00', 'DISP_i16'],
      operands: ['reg', 'rm'],
      operandSize: 16,
      cycles: 6,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET}',
        'a = ${MOD_RM_REG16}',
        'b = RAM:u16[effective_address]',
        '${ALU16_OP}',
      ],
      opcode: [Opcodes.CMP_RW_EW, 'ModRM_rm_reg16_00'],
      operands: ['reg', 'rm'],
      operandSize: 16,
      cycles: 6,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'a = ${MOD_RM_REG16}',
        'b = RAM:u16[effective_address]',
        '${ALU16_OP}',
      ],
      opcode: [Opcodes.CMP_RW_EW, 'ModRM_rm_reg16_01', 'DISP_i8'],
      operands: ['reg', 'rm'],
      operandSize: 16,
      cycles: 6,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'a = ${MOD_RM_REG16}',
        'b = RAM:u16[effective_address]',
        '${ALU16_OP}',
      ],
      opcode: [Opcodes.CMP_RW_EW, 'ModRM_rm_reg16_10', 'DISP_i16'],
      operands: ['reg', 'rm'],
      operandSize: 16,
      cycles: 6,
    },
    {
      operation: ['a = ${MOD_RM_REG16}', 'b = ${MOD_RM_RM16}', '${ALU16_OP}'],
      opcode: [Opcodes.CMP_RW_EW, 'ModRM_rm16_reg16_11'],
      operands: ['reg', 'rm'],
      operandSize: 16,
      cycles: 2,
    },
    // 0x3C db - CMP AL, db
    {
      operation: ['a = AL', 'b = %{IMM}', '${ALU8_OP}'],
      opcode: [Opcodes.CMP_AL_DB, 'IMM_u8'],
      operands: ['AL', 'imm'],
      operandSize: 8,
      cycles: 3,
    },
    // 0x3D dw - CMP AX, dw
    {
      operation: ['a = AX', 'b = %{IMM}', '${ALU16_OP}'],
      opcode: [Opcodes.CMP_AX_DW, 'IMM_u16'],
      operands: ['AX', 'imm'],
      operandSize: 16,
      cycles: 3,
    },
    // 0x80 /7 db - CMP eb, db
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + %{DISP}',
        'a = RAM:u8[effective_address]',
        'b = %{IMM}',
        '${ALU8_OP}',
      ],
      opcode: [Opcodes.ALU_EB_DB, 'ModRM_110_111_00', 'DISP_i16', 'IMM_u8'],
      operands: ['rm', 'imm'],
      operandSize: 8,
      cycles: 6,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET}',
        'a = RAM:u8[effective_address]',
        'b = %{IMM}',
        '${ALU8_OP}',
      ],
      opcode: [Opcodes.ALU_EB_DB, 'ModRM_rm_111_00', 'IMM_u8'],
      operands: ['rm', 'imm'],
      operandSize: 8,
      cycles: 6,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'a = RAM:u8[effective_address]',
        'b = %{IMM}',
        '${ALU8_OP}',
      ],
      opcode: [Opcodes.ALU_EB_DB, 'ModRM_rm_111_01', 'DISP_i8', 'IMM_u8'],
      operands: ['rm', 'imm'],
      operandSize: 8,
      cycles: 6,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'a = RAM:u8[effective_address]',
        'b = %{IMM}',
        '${ALU8_OP}',
      ],
      opcode: [Opcodes.ALU_EB_DB, 'ModRM_rm_111_10', 'DISP_i16', 'IMM_u8'],
      operands: ['rm', 'imm'],
      operandSize: 8,
      cycles: 6,
    },
    {
      operation: ['a = ${MOD_RM_RM8}', 'b = %{IMM}', '${ALU8_OP}'],
      opcode: [Opcodes.ALU_EB_DB, 'ModRM_rm8_111_11', 'IMM_u8'],
      operands: ['rm', 'imm'],
      operandSize: 8,
      cycles: 3,
    },
    // 0x81 /7 dw - CMP ew, dw
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + %{DISP}',
        'a = RAM:u16[effective_address]',
        'b = %{IMM}',
        '${ALU16_OP}',
      ],
      opcode: [Opcodes.ALU_EW_DW, 'ModRM_110_111_00', 'DISP_i16', 'IMM_u16'],
      operands: ['rm', 'imm'],
      operandSize: 16,
      cycles: 6,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET}',
        'a = RAM:u16[effective_address]',
        'b = %{IMM}',
        '${ALU16_OP}',
      ],
      opcode: [Opcodes.ALU_EW_DW, 'ModRM_rm_111_00', 'IMM_u16'],
      operands: ['rm', 'imm'],
      operandSize: 16,
      cycles: 6,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'a = RAM:u16[effective_address]',
        'b = %{IMM}',
        '${ALU16_OP}',
      ],
      opcode: [Opcodes.ALU_EW_DW, 'ModRM_rm_111_01', 'DISP_i8', 'IMM_u16'],
      operands: ['rm', 'imm'],
      operandSize: 16,
      cycles: 6,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'a = RAM:u16[effective_address]',
        'b = %{IMM}',
        '${ALU16_OP}',
      ],
      opcode: [Opcodes.ALU_EW_DW, 'ModRM_rm_111_10', 'DISP_i16', 'IMM_u16'],
      operands: ['rm', 'imm'],
      operandSize: 16,
      cycles: 6,
    },
    {
      operation: ['a = ${MOD_RM_RM16}', 'b = %{IMM}', '${ALU16_OP}'],
      opcode: [Opcodes.ALU_EW_DW, 'ModRM_rm16_111_11', 'IMM_u16'],
      operands: ['rm', 'imm'],
      operandSize: 16,
      cycles: 3,
    },
    // 0x83 /7 db - CMP ew, db
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + %{DISP}',
        'a = RAM:u16[effective_address]',
        'b = %{IMM}',
        '${ALU16_OP}',
      ],
      opcode: [Opcodes.ALU_EW_DB, 'ModRM_110_111_00', 'DISP_i16', 'IMM_i8'],
      operands: ['rm', 'imm'],
      operandSize: 16,
      cycles: 6,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET}',
        'a = RAM:u16[effective_address]',
        'b = %{IMM}',
        '${ALU16_OP}',
      ],
      opcode: [Opcodes.ALU_EW_DB, 'ModRM_rm_111_00', 'IMM_i8'],
      operands: ['rm', 'imm'],
      operandSize: 16,
      cycles: 6,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'a = RAM:u16[effective_address]',
        'b = %{IMM}',
        '${ALU16_OP}',
      ],
      opcode: [Opcodes.ALU_EW_DB, 'ModRM_rm_111_01', 'DISP_i8', 'IMM_i8'],
      operands: ['rm', 'imm'],
      operandSize: 16,
      cycles: 6,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'a = RAM:u16[effective_address]',
        'b = %{IMM}',
        '${ALU16_OP}',
      ],
      opcode: [Opcodes.ALU_EW_DB, 'ModRM_rm_111_10', 'DISP_i16', 'IMM_i8'],
      operands: ['rm', 'imm'],
      operandSize: 16,
      cycles: 6,
    },
    {
      operation: ['a = ${MOD_RM_RM16}', 'b = %{IMM}', '${ALU16_OP}'],
      opcode: [Opcodes.ALU_EW_DB, 'ModRM_rm16_111_11', 'IMM_i8'],
      operands: ['rm', 'imm'],
      operandSize: 16,
      cycles: 3,
    },
  ],
};
