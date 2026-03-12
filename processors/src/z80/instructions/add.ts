import {
  InstructionDataTypes,
  InstructionOperandTypes,
  type InstructionInfo,
} from '@machinery/core';

import { Opcodes } from '../opcodes';

export const add: InstructionInfo = {
  identifier: 'add',
  name: 'Add',
  description: 'Adds the operand to the accumulator (8-bit) or to HL (16-bit).',
  modifies: ['SF', 'ZF', 'HF', 'PF', 'NF', 'CF'],
  macros: {
    ALU8_OP: ['alu_result = a + b', 'flag_op = ${FLAG_OP_ALU_ADD8}'],
    ALU16_OP: ['alu_result = a + b', 'flag_op = ${FLAG_OP_ALU_ADD16}'],
  },
  forms: [
    // ADD A, r — 8-bit register
    {
      opcode: [Opcodes.ADD_A_B],
      operands: ['A', 'B'],
      operandSize: 8,
      operation: ['a = A', 'b = B', '${ALU8_OP}', 'A = alu_result'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.ADD_A_C],
      operands: ['A', 'C'],
      operandSize: 8,
      operation: ['a = A', 'b = C', '${ALU8_OP}', 'A = alu_result'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.ADD_A_D],
      operands: ['A', 'D'],
      operandSize: 8,
      operation: ['a = A', 'b = D', '${ALU8_OP}', 'A = alu_result'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.ADD_A_E],
      operands: ['A', 'E'],
      operandSize: 8,
      operation: ['a = A', 'b = E', '${ALU8_OP}', 'A = alu_result'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.ADD_A_H],
      operands: ['A', 'H'],
      operandSize: 8,
      operation: ['a = A', 'b = H', '${ALU8_OP}', 'A = alu_result'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.ADD_A_L],
      operands: ['A', 'L'],
      operandSize: 8,
      operation: ['a = A', 'b = L', '${ALU8_OP}', 'A = alu_result'],
      cycles: 4,
    },
    {
      opcode: [
        {
          identifier: 'Opcode_ADD_A_xHLx',
          name: 'ADD A, (HL) Opcode Field',
          type: InstructionDataTypes.Operand,
          size: 8,
          fields: [
            {
              identifier: 'rm',
              offset: 0,
              size: 3,
              match: 0b110,
              type: InstructionOperandTypes.Memory,
              encoding: ['HL'],
            },
            {
              identifier: 'opcode',
              offset: 3,
              size: 5,
              match: 0b10000,
            },
          ],
        },
      ],
      operands: ['A', 'rm'],
      operandSize: 8,
      operation: ['a = A', 'b = RAM:u8[HL]', '${ALU8_OP}', 'A = alu_result'],
      cycles: 7,
    },
    {
      opcode: [Opcodes.ADD_A_A],
      operands: ['A', 'A'],
      operandSize: 8,
      operation: ['a = A', 'b = A', '${ALU8_OP}', 'A = alu_result'],
      cycles: 4,
    },

    // ADD A, n — 8-bit immediate
    {
      opcode: [Opcodes.ADD_A_N, 'IMM_u8'],
      operands: ['A', 'imm'],
      operandSize: 8,
      operation: ['a = A', 'b = %{imm}', '${ALU8_OP}', 'A = alu_result'],
      cycles: 7,
    },

    // ADD A, (IX+d) / ADD A, (IY+d)
    {
      opcode: ['DD_IX', Opcodes.ADD_A_xHLx, 'DISP_i8'],
      operands: ['A', 'rm'],
      operandSize: 8,
      operation: [
        'a = A',
        'b = RAM:u8[IX + %{DISP}]',
        '${ALU8_OP}',
        'A = alu_result',
      ],
      cycles: 19,
    },
    {
      opcode: ['FD_IY', Opcodes.ADD_A_xHLx, 'DISP_i8'],
      operands: ['A', 'rm'],
      operandSize: 8,
      operation: [
        'a = A',
        'b = RAM:u8[IY + %{DISP}]',
        '${ALU8_OP}',
        'A = alu_result',
      ],
      cycles: 19,
    },

    // ADD HL, rp — 16-bit register pair (only HF, NF, CF affected)
    {
      opcode: [Opcodes.ADD_HL_BC],
      operands: ['HL', 'BC'],
      operandSize: 16,
      modifies: ['HF', 'NF', 'CF'],
      operation: ['a = HL', 'b = BC', '${ALU16_OP}', 'HL = alu_result'],
      cycles: 11,
    },
    {
      opcode: [Opcodes.ADD_HL_DE],
      operands: ['HL', 'DE'],
      operandSize: 16,
      modifies: ['HF', 'NF', 'CF'],
      operation: ['a = HL', 'b = DE', '${ALU16_OP}', 'HL = alu_result'],
      cycles: 11,
    },
    {
      opcode: [Opcodes.ADD_HL_HL],
      operands: ['HL', 'HL'],
      operandSize: 16,
      modifies: ['HF', 'NF', 'CF'],
      operation: ['a = HL', 'b = HL', '${ALU16_OP}', 'HL = alu_result'],
      cycles: 11,
    },
    {
      opcode: [Opcodes.ADD_HL_SP],
      operands: ['HL', 'SP'],
      operandSize: 16,
      modifies: ['HF', 'NF', 'CF'],
      operation: ['a = HL', 'b = SP', '${ALU16_OP}', 'HL = alu_result'],
      cycles: 11,
    },

    // ADD IX, rp
    {
      opcode: [Opcodes.DD_PREFIX, Opcodes.ADD_HL_BC],
      operands: ['IX', 'BC'],
      operandSize: 16,
      modifies: ['HF', 'NF', 'CF'],
      operation: ['a = IX', 'b = BC', '${ALU16_OP}', 'IX = alu_result'],
      cycles: 15,
    },
    {
      opcode: [Opcodes.DD_PREFIX, Opcodes.ADD_HL_DE],
      operands: ['IX', 'DE'],
      operandSize: 16,
      modifies: ['HF', 'NF', 'CF'],
      operation: ['a = IX', 'b = DE', '${ALU16_OP}', 'IX = alu_result'],
      cycles: 15,
    },
    {
      opcode: [Opcodes.DD_PREFIX, Opcodes.ADD_HL_HL],
      operands: ['IX', 'IX'],
      operandSize: 16,
      modifies: ['HF', 'NF', 'CF'],
      operation: ['a = IX', 'b = IX', '${ALU16_OP}', 'IX = alu_result'],
      cycles: 15,
    },
    {
      opcode: [Opcodes.DD_PREFIX, Opcodes.ADD_HL_SP],
      operands: ['IX', 'SP'],
      operandSize: 16,
      modifies: ['HF', 'NF', 'CF'],
      operation: ['a = IX', 'b = SP', '${ALU16_OP}', 'IX = alu_result'],
      cycles: 15,
    },
    // ADD IY, rp
    {
      opcode: [Opcodes.FD_PREFIX, Opcodes.ADD_HL_BC],
      operands: ['IY', 'BC'],
      operandSize: 16,
      modifies: ['HF', 'NF', 'CF'],
      operation: ['a = IY', 'b = BC', '${ALU16_OP}', 'IY = alu_result'],
      cycles: 15,
    },
    {
      opcode: [Opcodes.FD_PREFIX, Opcodes.ADD_HL_DE],
      operands: ['IY', 'DE'],
      operandSize: 16,
      modifies: ['HF', 'NF', 'CF'],
      operation: ['a = IY', 'b = DE', '${ALU16_OP}', 'IY = alu_result'],
      cycles: 15,
    },
    {
      opcode: [Opcodes.FD_PREFIX, Opcodes.ADD_HL_HL],
      operands: ['IY', 'IY'],
      operandSize: 16,
      modifies: ['HF', 'NF', 'CF'],
      operation: ['a = IY', 'b = IY', '${ALU16_OP}', 'IY = alu_result'],
      cycles: 15,
    },
    {
      opcode: [Opcodes.FD_PREFIX, Opcodes.ADD_HL_SP],
      operands: ['IY', 'SP'],
      operandSize: 16,
      modifies: ['HF', 'NF', 'CF'],
      operation: ['a = IY', 'b = SP', '${ALU16_OP}', 'IY = alu_result'],
      cycles: 15,
    },
  ],
};
