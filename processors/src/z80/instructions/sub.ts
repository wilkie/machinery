import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const sub: InstructionInfo = {
  identifier: 'sub',
  name: 'Subtract',
  description: 'Subtracts the operand from the accumulator.',
  modifies: ['S', 'Z', 'H', 'P', 'N', 'C'],
  forms: [
    {
      opcode: [Opcodes.SUB_B],
      operands: ['B'],
      operandSize: 8,
      operation: ['A = A - B'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.SUB_C],
      operands: ['C'],
      operandSize: 8,
      operation: ['A = A - C'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.SUB_D],
      operands: ['D'],
      operandSize: 8,
      operation: ['A = A - D'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.SUB_E],
      operands: ['E'],
      operandSize: 8,
      operation: ['A = A - E'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.SUB_H],
      operands: ['H'],
      operandSize: 8,
      operation: ['A = A - H'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.SUB_L],
      operands: ['L'],
      operandSize: 8,
      operation: ['A = A - L'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.SUB_xHLx],
      operands: ['(HL)'],
      operandSize: 8,
      operation: ['A = A - RAM:u8[HL]'],
      cycles: 7,
    },
    {
      opcode: [Opcodes.SUB_A],
      operands: ['A'],
      operandSize: 8,
      operation: ['A = A - A'],
      cycles: 4,
    },

    // SUB n
    {
      opcode: [Opcodes.SUB_N, 'IMM_u8'],
      operands: ['imm'],
      operandSize: 8,
      operation: ['A = A - %{IMM}'],
      cycles: 7,
    },
  ],
};
