import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const dec: InstructionInfo = {
  identifier: 'dec',
  name: 'Decrement',
  description: 'Decrements the operand by one.',
  modifies: ['S', 'Z', 'H', 'P', 'N'],
  forms: [
    // 8-bit DEC r
    {
      opcode: [Opcodes.DEC_B],
      operands: ['B'],
      operandSize: 8,
      operation: ['B = B - 1'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.DEC_C],
      operands: ['C'],
      operandSize: 8,
      operation: ['C = C - 1'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.DEC_D],
      operands: ['D'],
      operandSize: 8,
      operation: ['D = D - 1'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.DEC_E],
      operands: ['E'],
      operandSize: 8,
      operation: ['E = E - 1'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.DEC_H],
      operands: ['H'],
      operandSize: 8,
      operation: ['H = H - 1'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.DEC_L],
      operands: ['L'],
      operandSize: 8,
      operation: ['L = L - 1'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.DEC_xHLx],
      operands: ['(HL)'],
      operandSize: 8,
      operation: ['RAM:u8[HL] = RAM:u8[HL] - 1'],
      cycles: 11,
    },
    {
      opcode: [Opcodes.DEC_A],
      operands: ['A'],
      operandSize: 8,
      operation: ['A = A - 1'],
      cycles: 4,
    },

    // 16-bit DEC rp (no flags affected)
    {
      opcode: [Opcodes.DEC_BC],
      operands: ['BC'],
      operandSize: 16,
      modifies: [],
      operation: ['BC = BC - 1'],
      cycles: 6,
    },
    {
      opcode: [Opcodes.DEC_DE],
      operands: ['DE'],
      operandSize: 16,
      modifies: [],
      operation: ['DE = DE - 1'],
      cycles: 6,
    },
    {
      opcode: [Opcodes.DEC_HL],
      operands: ['HL'],
      operandSize: 16,
      modifies: [],
      operation: ['HL = HL - 1'],
      cycles: 6,
    },
    {
      opcode: [Opcodes.DEC_SP],
      operands: ['SP'],
      operandSize: 16,
      modifies: [],
      operation: ['SP = SP - 1'],
      cycles: 6,
    },
  ],
};
