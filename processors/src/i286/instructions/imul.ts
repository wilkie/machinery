import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const imul: InstructionInfo = {
  identifier: 'imul',
  name: 'Signed Multiplication',
  description:
    '`IMUL` performs signed multiplication. If `IMUL` has a single byte source operand, then the source is multiplied by `AL` and the 16-bit signed result is left in `AX`. Carry and overflow are set to 0 if `AH` is a sign extension of `AL`; they are set to 1 otherwise.\n\nIf `IMUL` has a single word source operand, then the source operand is multiplied by `AX` and the 32-bit signed result is left in `DX:AX`. `DX` contains the high-order 16 bits of the product. Carry and overflow are set to 0 if `DX` is a sign extension of `AX`; they are set to 1 otherwise.\n\nIf `IMUL` has three operands, then the second operand (an effective address word) is multiplied by the third operand (an immediate word), and the 16 bits of the result are placed in the first operand (a word register). Carry and overflow are set to 0 if the result fits in a signed word (between - 32768 and +32767, inclusive); they are set to 1 otherwise.\n\n**Note**: The low 16 bits of the product of a 16-bit signed multiply are the same as those of an unsigned multiply. The three operand `IMUL` instruction can be used for unsigned operands as well.',
  modifies: ['OF', 'CF'],
  undefined: ['SF', 'ZF', 'AF', 'PF'],
  macros: {
    ALU8_OP: [
      'alu_result = a:i8 * b:i8',
      'CARRY = (alu_result & 0xff00) == (0x1fe * (alu_result & 0x80)) ? 0x0 : 0x1',
      'OF = CARRY',
      'flag_op = ${FLAG_OP_RESOLVED}',
    ],
    ALU16_OP: [
      'alu_result = a:i16 * b:i16',
      'CARRY = (alu_result & 0xffff0000) == (0x1fffe * (alu_result & 0x8000)) ? 0x0 : 0x1',
      'OF = CARRY',
      'flag_op = ${FLAG_OP_RESOLVED}',
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
    // 0xF6 /5 - IMUL eb
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + %{DISP}',
        'a = AL:i8',
        'b = RAM:i8[effective_address]',
        '${ALU8_OP}',
        'AX = alu_result',
      ],
      opcode: [Opcodes.ALU_LOGIC_EB, 'ModRM_110_101_00', 'DISP_i16'],
      operandSize: 8,
      cycles: 16,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET}',
        'a = AL:i8',
        'b = RAM:i8[effective_address]',
        '${ALU8_OP}',
        'AX = alu_result',
      ],
      opcode: [Opcodes.ALU_LOGIC_EB, 'ModRM_rm_101_00'],
      operandSize: 8,
      cycles: 16,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'a = AL:i8',
        'b = RAM:i8[effective_address]',
        '${ALU8_OP}',
        'AX = alu_result',
      ],
      opcode: [Opcodes.ALU_LOGIC_EB, 'ModRM_rm_101_01', 'DISP_i8'],
      operandSize: 8,
      cycles: 16,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'a = AL:i8',
        'b = RAM:i8[effective_address]',
        '${ALU8_OP}',
        'AX = alu_result',
      ],
      opcode: [Opcodes.ALU_LOGIC_EB, 'ModRM_rm_101_10', 'DISP_i16'],
      operandSize: 8,
      cycles: 16,
    },
    {
      operation: [
        'a = AL:i8',
        'b = ${MOD_RM_RM8:i8}',
        '${ALU8_OP}',
        'AX = alu_result',
      ],
      opcode: [Opcodes.ALU_LOGIC_EB, 'ModRM_rm_101_11'],
      operandSize: 8,
      cycles: 13,
    },
    // 0xF7 /5 - IMUL ew
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + %{DISP}',
        'a = AX',
        'b = RAM:u16[effective_address]',
        '${ALU16_OP}',
        'DX = alu_result >> 16',
        'AX = alu_result & 0xffff',
      ],
      opcode: [Opcodes.ALU_LOGIC_EW, 'ModRM_110_101_00', 'DISP_i16'],
      operandSize: 16,
      cycles: 24,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET}',
        'a = AX',
        'b = RAM:u16[effective_address]',
        '${ALU16_OP}',
        'DX = alu_result >> 16',
        'AX = alu_result & 0xffff',
      ],
      opcode: [Opcodes.ALU_LOGIC_EW, 'ModRM_rm_101_00'],
      operandSize: 16,
      cycles: 24,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'a = AX',
        'b = RAM:u16[effective_address]',
        '${ALU16_OP}',
        'DX = alu_result >> 16',
        'AX = alu_result & 0xffff',
      ],
      opcode: [Opcodes.ALU_LOGIC_EW, 'ModRM_rm_101_01', 'DISP_i8'],
      operandSize: 16,
      cycles: 24,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'a = AX',
        'b = RAM:u16[effective_address]',
        '${ALU16_OP}',
        'DX = alu_result >> 16',
        'AX = alu_result & 0xffff',
      ],
      opcode: [Opcodes.ALU_LOGIC_EW, 'ModRM_rm_101_10', 'DISP_i16'],
      operandSize: 16,
      cycles: 24,
    },
    {
      operation: [
        'a = AX',
        'b = ${MOD_RM_RM16}',
        '${ALU16_OP}',
        'DX = alu_result >> 16',
        'AX = alu_result & 0xffff',
      ],
      opcode: [Opcodes.ALU_LOGIC_EW, 'ModRM_rm_101_11'],
      operandSize: 16,
      cycles: 21,
    },
    // 0x6B /r db - IMUL rw, db
    // 0x6B /r db - IMUL rw, ew, db
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + %{DISP}',
        'a = RAM:u16[effective_address]',
        'b = %{IMM}',
        '${ALU16_OP}',
        '${MOD_RM_REG16} = alu_result',
      ],
      opcode: [Opcodes.IMUL_EW_DB, 'ModRM_110_reg_00', 'DISP_i16', 'IMM_i8'],
      operandSize: 16,
      cycles: 24,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET}',
        'a = RAM:u16[effective_address]',
        'b = %{IMM}',
        '${ALU16_OP}',
        '${MOD_RM_REG16} = alu_result',
      ],
      opcode: [Opcodes.IMUL_EW_DB, 'ModRM_rm_reg_00', 'IMM_i8'],
      operandSize: 16,
      cycles: 24,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'a = RAM:u16[effective_address]',
        'b = %{IMM}',
        '${ALU16_OP}',
        '${MOD_RM_REG16} = alu_result',
      ],
      opcode: [Opcodes.IMUL_EW_DB, 'ModRM_rm_reg_01', 'DISP_i8', 'IMM_i8'],
      operandSize: 16,
      cycles: 24,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'a = RAM:u16[effective_address]',
        'b = %{IMM}',
        '${ALU16_OP}',
        '${MOD_RM_REG16} = alu_result',
      ],
      opcode: [Opcodes.IMUL_EW_DB, 'ModRM_rm_reg_10', 'DISP_i16', 'IMM_i8'],
      operandSize: 16,
      cycles: 24,
    },
    {
      operation: [
        'a = ${MOD_RM_RM16}',
        'b = %{IMM}',
        '${ALU16_OP}',
        '${MOD_RM_REG16} = alu_result',
      ],
      opcode: [Opcodes.IMUL_EW_DB, 'ModRM_rm_reg_11', 'IMM_i8'],
      operandSize: 16,
      cycles: 21,
    },
    // 0x69 /r dw - IMUL rw, ew, dw
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + %{DISP}',
        'a = RAM:u16[effective_address]',
        'b = %{IMM}',
        '${ALU16_OP}',
        '${MOD_RM_REG16} = alu_result',
      ],
      opcode: [Opcodes.IMUL_EW_DW, 'ModRM_110_reg_00', 'DISP_i16', 'IMM_i16'],
      operandSize: 16,
      cycles: 24,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET}',
        'a = RAM:u16[effective_address]',
        'b = %{IMM}',
        '${ALU16_OP}',
        '${MOD_RM_REG16} = alu_result',
      ],
      opcode: [Opcodes.IMUL_EW_DW, 'ModRM_rm_reg_00', 'IMM_i16'],
      operandSize: 16,
      cycles: 24,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'a = RAM:u16[effective_address]',
        'b = %{IMM}',
        '${ALU16_OP}',
        '${MOD_RM_REG16} = alu_result',
      ],
      opcode: [Opcodes.IMUL_EW_DW, 'ModRM_rm_reg_01', 'DISP_i8', 'IMM_i16'],
      operandSize: 16,
      cycles: 24,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'a = RAM:u16[effective_address]',
        'b = %{IMM}',
        '${ALU16_OP}',
        '${MOD_RM_REG16} = alu_result',
      ],
      opcode: [Opcodes.IMUL_EW_DW, 'ModRM_rm_reg_10', 'DISP_i16', 'IMM_i16'],
      operandSize: 16,
      cycles: 24,
    },
    {
      operation: [
        'a = ${MOD_RM_RM16}',
        'b = %{IMM}',
        '${ALU16_OP}',
        '${MOD_RM_REG16} = alu_result',
      ],
      opcode: [Opcodes.IMUL_EW_DW, 'ModRM_rm_reg_11', 'IMM_i16'],
      operandSize: 16,
      cycles: 21,
    },
  ],
};
