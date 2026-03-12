import {
  InstructionDataTypes,
  InstructionOperandTypes,
  type InstructionInfo,
} from '@machinery/core';

import { Opcodes } from '../opcodes';

export const sub: InstructionInfo = {
  identifier: 'sub',
  name: 'Subtract',
  description: 'Subtracts the operand from the accumulator.',
  modifies: ['SF', 'ZF', 'HF', 'PF', 'NF', 'CF'],
  macros: {
    ALU_OP: ['alu_result = a - b', 'flag_op = ${FLAG_OP_ALU_SUB8}'],
  },
  forms: [
    {
      opcode: [Opcodes.SUB_B],
      operands: ['B'],
      operandSize: 8,
      operation: ['a = A', 'b = B', '${ALU_OP}', 'A = alu_result'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.SUB_C],
      operands: ['C'],
      operandSize: 8,
      operation: ['a = A', 'b = C', '${ALU_OP}', 'A = alu_result'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.SUB_D],
      operands: ['D'],
      operandSize: 8,
      operation: ['a = A', 'b = D', '${ALU_OP}', 'A = alu_result'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.SUB_E],
      operands: ['E'],
      operandSize: 8,
      operation: ['a = A', 'b = E', '${ALU_OP}', 'A = alu_result'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.SUB_H],
      operands: ['H'],
      operandSize: 8,
      operation: ['a = A', 'b = H', '${ALU_OP}', 'A = alu_result'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.SUB_L],
      operands: ['L'],
      operandSize: 8,
      operation: ['a = A', 'b = L', '${ALU_OP}', 'A = alu_result'],
      cycles: 4,
    },
    {
      opcode: [
        {
          identifier: 'Opcode_SUB_xHLx',
          name: 'SUB (HL) Opcode Field',
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
              match: 0b10010,
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
      opcode: [Opcodes.SUB_A],
      operands: ['A'],
      operandSize: 8,
      operation: ['a = A', 'b = A', '${ALU_OP}', 'A = alu_result'],
      cycles: 4,
    },

    // SUB n
    {
      opcode: [Opcodes.SUB_N, 'IMM_u8'],
      operands: ['imm'],
      operandSize: 8,
      operation: ['a = A', 'b = %{imm}', '${ALU_OP}', 'A = alu_result'],
      cycles: 7,
    },

    // SUB (IX+d) / SUB (IY+d)
    {
      opcode: ['DD_IX', Opcodes.SUB_xHLx, 'DISP_i8'],
      operands: ['rm'],
      operandSize: 8,
      operation: ['a = A', 'b = RAM:u8[IX + %{DISP}]', '${ALU_OP}', 'A = alu_result'],
      cycles: 19,
    },
    {
      opcode: ['FD_IY', Opcodes.SUB_xHLx, 'DISP_i8'],
      operands: ['rm'],
      operandSize: 8,
      operation: ['a = A', 'b = RAM:u8[IY + %{DISP}]', '${ALU_OP}', 'A = alu_result'],
      cycles: 19,
    },
  ],
};
