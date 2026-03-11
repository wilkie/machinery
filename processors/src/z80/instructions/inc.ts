import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const inc: InstructionInfo = {
  identifier: 'inc',
  name: 'Increment',
  description: 'Increments the operand by one.',
  modifies: ['S', 'Z', 'H', 'P', 'N'],
  forms: [
    // 8-bit INC r
    {
      opcode: [Opcodes.INC_B],
      operands: ['B'],
      operandSize: 8,
      operation: ['B = B + 1'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.INC_C],
      operands: ['C'],
      operandSize: 8,
      operation: ['C = C + 1'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.INC_D],
      operands: ['D'],
      operandSize: 8,
      operation: ['D = D + 1'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.INC_E],
      operands: ['E'],
      operandSize: 8,
      operation: ['E = E + 1'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.INC_H],
      operands: ['H'],
      operandSize: 8,
      operation: ['H = H + 1'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.INC_L],
      operands: ['L'],
      operandSize: 8,
      operation: ['L = L + 1'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.INC_xHLx],
      operands: ['(HL)'],
      operandSize: 8,
      operation: ['RAM:u8[HL] = RAM:u8[HL] + 1'],
      cycles: 11,
    },
    {
      opcode: [Opcodes.INC_A],
      operands: ['A'],
      operandSize: 8,
      operation: ['A = A + 1'],
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
