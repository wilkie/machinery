import {
  InstructionDataTypes,
  InstructionOperandTypes,
  type InstructionInfo,
} from '@machinery/core';

import { Opcodes } from '../opcodes';

export const and: InstructionInfo = {
  identifier: 'and',
  name: 'Logical AND',
  description:
    'Performs a bitwise AND between the accumulator and the operand, storing the result in A.',
  modifies: ['SF', 'ZF', 'HF', 'PF', 'NF', 'CF'],
  macros: {
    ALU_OP: [
      'alu_result = a & b',
      'flag_op = ${FLAG_OP_LOGIC} | ${FLAG_OP_AND}',
    ],
  },
  forms: [
    {
      opcode: [Opcodes.AND_B],
      operands: ['B'],
      operandSize: 8,
      operation: ['a = A', 'b = B', '${ALU_OP}', 'A = alu_result'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.AND_C],
      operands: ['C'],
      operandSize: 8,
      operation: ['a = A', 'b = C', '${ALU_OP}', 'A = alu_result'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.AND_D],
      operands: ['D'],
      operandSize: 8,
      operation: ['a = A', 'b = D', '${ALU_OP}', 'A = alu_result'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.AND_E],
      operands: ['E'],
      operandSize: 8,
      operation: ['a = A', 'b = E', '${ALU_OP}', 'A = alu_result'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.AND_H],
      operands: ['H'],
      operandSize: 8,
      operation: ['a = A', 'b = H', '${ALU_OP}', 'A = alu_result'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.AND_L],
      operands: ['L'],
      operandSize: 8,
      operation: ['a = A', 'b = L', '${ALU_OP}', 'A = alu_result'],
      cycles: 4,
    },
    {
      opcode: [
        {
          identifier: 'Opcode_AND_xHLx',
          name: 'AND (HL) Opcode Field',
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
              match: 0b10100,
            },
          ],
        },
      ],
      operands: ['rm'],
      operandSize: 8,
      operation: ['a = A', 'b = RAM:u8[HL]', '${ALU_OP}', 'A = alu_result'],
      cycles: 7,
    },
    {
      opcode: [Opcodes.AND_A],
      operands: ['A'],
      operandSize: 8,
      operation: ['a = A', 'b = A', '${ALU_OP}', 'A = alu_result'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.AND_N, 'IMM_u8'],
      operands: ['imm'],
      operandSize: 8,
      operation: ['a = A', 'b = %{imm}', '${ALU_OP}', 'A = alu_result'],
      cycles: 7,
    },

    // AND (IX+d) / AND (IY+d)
    {
      opcode: ['DD_IX', Opcodes.AND_xHLx, 'DISP_i8'],
      operands: ['rm'],
      operandSize: 8,
      operation: ['a = A', 'b = RAM:u8[IX + %{DISP}]', '${ALU_OP}', 'A = alu_result'],
      cycles: 19,
    },
    {
      opcode: ['FD_IY', Opcodes.AND_xHLx, 'DISP_i8'],
      operands: ['rm'],
      operandSize: 8,
      operation: ['a = A', 'b = RAM:u8[IY + %{DISP}]', '${ALU_OP}', 'A = alu_result'],
      cycles: 19,
    },
  ],
};
