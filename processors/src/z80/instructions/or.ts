import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const or: InstructionInfo = {
  identifier: 'or',
  name: 'Logical OR',
  description:
    'Performs a bitwise OR between the accumulator and the operand, storing the result in A.',
  modifies: ['S', 'Z', 'H', 'P', 'N', 'C'],
  forms: [
    {
      opcode: [Opcodes.OR_B],
      operands: ['B'],
      operandSize: 8,
      operation: ['A = A | B'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.OR_C],
      operands: ['C'],
      operandSize: 8,
      operation: ['A = A | C'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.OR_D],
      operands: ['D'],
      operandSize: 8,
      operation: ['A = A | D'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.OR_E],
      operands: ['E'],
      operandSize: 8,
      operation: ['A = A | E'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.OR_H],
      operands: ['H'],
      operandSize: 8,
      operation: ['A = A | H'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.OR_L],
      operands: ['L'],
      operandSize: 8,
      operation: ['A = A | L'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.OR_xHLx],
      operands: ['(HL)'],
      operandSize: 8,
      operation: ['A = A | RAM:u8[HL]'],
      cycles: 7,
    },
    {
      opcode: [Opcodes.OR_A],
      operands: ['A'],
      operandSize: 8,
      operation: ['A = A | A'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.OR_N, 'IMM_u8'],
      operands: ['imm'],
      operandSize: 8,
      operation: ['A = A | %{IMM}'],
      cycles: 7,
    },
  ],
};
