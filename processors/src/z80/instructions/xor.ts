import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const xor: InstructionInfo = {
  identifier: 'xor',
  name: 'Logical Exclusive OR',
  description:
    'Performs a bitwise XOR between the accumulator and the operand, storing the result in A.',
  modifies: ['S', 'Z', 'H', 'P', 'N', 'C'],
  forms: [
    {
      opcode: [Opcodes.XOR_B],
      operands: ['B'],
      operandSize: 8,
      operation: ['A = A ^ B'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.XOR_C],
      operands: ['C'],
      operandSize: 8,
      operation: ['A = A ^ C'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.XOR_D],
      operands: ['D'],
      operandSize: 8,
      operation: ['A = A ^ D'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.XOR_E],
      operands: ['E'],
      operandSize: 8,
      operation: ['A = A ^ E'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.XOR_H],
      operands: ['H'],
      operandSize: 8,
      operation: ['A = A ^ H'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.XOR_L],
      operands: ['L'],
      operandSize: 8,
      operation: ['A = A ^ L'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.XOR_xHLx],
      operands: ['(HL)'],
      operandSize: 8,
      operation: ['A = A ^ RAM:u8[HL]'],
      cycles: 7,
    },
    {
      opcode: [Opcodes.XOR_A],
      operands: ['A'],
      operandSize: 8,
      operation: ['A = A ^ A'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.XOR_N, 'IMM_u8'],
      operands: ['imm'],
      operandSize: 8,
      operation: ['A = A ^ %{IMM}'],
      cycles: 7,
    },
  ],
};
