import {
  InstructionDataTypes,
  InstructionOperandTypes,
  type InstructionInfo,
} from '@machinery/core';

import { Opcodes } from '../opcodes';

export const inc: InstructionInfo = {
  identifier: 'inc',
  name: 'Increment',
  description: 'Increments the operand by one.',
  modifies: ['SF', 'ZF', 'HF', 'PF', 'NF'],
  macros: {
    ALU_OP: ['alu_result = a + b', 'flag_op = ${FLAG_OP_ALU_INC8}'],
  },
  forms: [
    // 8-bit INC r
    {
      opcode: [Opcodes.INC_B],
      operands: ['B'],
      operandSize: 8,
      operation: ['a = B', 'b = 1', '${ALU_OP}', 'B = alu_result'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.INC_C],
      operands: ['C'],
      operandSize: 8,
      operation: ['a = C', 'b = 1', '${ALU_OP}', 'C = alu_result'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.INC_D],
      operands: ['D'],
      operandSize: 8,
      operation: ['a = D', 'b = 1', '${ALU_OP}', 'D = alu_result'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.INC_E],
      operands: ['E'],
      operandSize: 8,
      operation: ['a = E', 'b = 1', '${ALU_OP}', 'E = alu_result'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.INC_H],
      operands: ['H'],
      operandSize: 8,
      operation: ['a = H', 'b = 1', '${ALU_OP}', 'H = alu_result'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.INC_L],
      operands: ['L'],
      operandSize: 8,
      operation: ['a = L', 'b = 1', '${ALU_OP}', 'L = alu_result'],
      cycles: 4,
    },
    {
      opcode: [
        {
          identifier: 'Opcode_INC_xHLx',
          name: 'INC (HL) Opcode Field',
          type: InstructionDataTypes.Operand,
          size: 8,
          fields: [
            {
              identifier: 'opcode_low',
              offset: 0,
              size: 3,
              match: 0b100,
            },
            {
              identifier: 'rm',
              offset: 3,
              size: 3,
              match: 0b110,
              type: InstructionOperandTypes.Memory,
              encoding: ['HL'],
            },
            {
              identifier: 'opcode_high',
              offset: 6,
              size: 2,
              match: 0b00,
            },
          ],
        },
      ],
      operands: ['rm'],
      operandSize: 8,
      operation: [
        'a = RAM:u8[HL]',
        'b = 1',
        '${ALU_OP}',
        'RAM:u8[HL] = alu_result',
      ],
      cycles: 11,
    },
    {
      opcode: [Opcodes.INC_A],
      operands: ['A'],
      operandSize: 8,
      operation: ['a = A', 'b = 1', '${ALU_OP}', 'A = alu_result'],
      cycles: 4,
    },

    // 16-bit INC rp (no flags affected)
    {
      opcode: [Opcodes.INC_BC],
      operands: ['BC'],
      operandSize: 16,
      modifies: [],
      operation: ['BC = BC + 1'],
      cycles: 6,
    },
    {
      opcode: [Opcodes.INC_DE],
      operands: ['DE'],
      operandSize: 16,
      modifies: [],
      operation: ['DE = DE + 1'],
      cycles: 6,
    },
    {
      opcode: [Opcodes.INC_HL],
      operands: ['HL'],
      operandSize: 16,
      modifies: [],
      operation: ['HL = HL + 1'],
      cycles: 6,
    },
    {
      opcode: [Opcodes.INC_SP],
      operands: ['SP'],
      operandSize: 16,
      modifies: [],
      operation: ['SP = SP + 1'],
      cycles: 6,
    },
  ],
};
