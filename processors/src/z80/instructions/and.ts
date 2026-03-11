import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const and: InstructionInfo = {
  identifier: 'and',
  name: 'Logical AND',
  description:
    'Performs a bitwise AND between the accumulator and the operand, storing the result in A.',
  modifies: ['S', 'Z', 'H', 'P', 'N', 'C'],
  forms: [
    {
      opcode: [Opcodes.AND_B],
      operands: ['B'],
      operandSize: 8,
      operation: ['A = A & B'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.AND_C],
      operands: ['C'],
      operandSize: 8,
      operation: ['A = A & C'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.AND_D],
      operands: ['D'],
      operandSize: 8,
      operation: ['A = A & D'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.AND_E],
      operands: ['E'],
      operandSize: 8,
      operation: ['A = A & E'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.AND_H],
      operands: ['H'],
      operandSize: 8,
      operation: ['A = A & H'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.AND_L],
      operands: ['L'],
      operandSize: 8,
      operation: ['A = A & L'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.AND_xHLx],
      operands: ['(HL)'],
      operandSize: 8,
      operation: ['A = A & RAM:u8[HL]'],
      cycles: 7,
    },
    {
      opcode: [Opcodes.AND_A],
      operands: ['A'],
      operandSize: 8,
      operation: ['A = A & A'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.AND_N, 'IMM_u8'],
      operands: ['imm'],
      operandSize: 8,
      operation: ['A = A & %{IMM}'],
      cycles: 7,
    },
  ],
};
