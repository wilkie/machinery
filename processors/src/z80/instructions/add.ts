import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const add: InstructionInfo = {
  identifier: 'add',
  name: 'Add',
  description: 'Adds the operand to the accumulator (8-bit) or to HL (16-bit).',
  modifies: ['S', 'Z', 'H', 'P', 'N', 'C'],
  forms: [
    // ADD A, r — 8-bit register
    {
      opcode: [Opcodes.ADD_A_B],
      operands: ['A', 'B'],
      operandSize: 8,
      operation: ['A = A + B'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.ADD_A_C],
      operands: ['A', 'C'],
      operandSize: 8,
      operation: ['A = A + C'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.ADD_A_D],
      operands: ['A', 'D'],
      operandSize: 8,
      operation: ['A = A + D'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.ADD_A_E],
      operands: ['A', 'E'],
      operandSize: 8,
      operation: ['A = A + E'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.ADD_A_H],
      operands: ['A', 'H'],
      operandSize: 8,
      operation: ['A = A + H'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.ADD_A_L],
      operands: ['A', 'L'],
      operandSize: 8,
      operation: ['A = A + L'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.ADD_A_xHLx],
      operands: ['A', '(HL)'],
      operandSize: 8,
      operation: ['A = A + RAM:u8[HL]'],
      cycles: 7,
    },
    {
      opcode: [Opcodes.ADD_A_A],
      operands: ['A', 'A'],
      operandSize: 8,
      operation: ['A = A + A'],
      cycles: 4,
    },

    // ADD A, n — 8-bit immediate
    {
      opcode: [Opcodes.ADD_A_N, 'IMM_u8'],
      operands: ['A', 'imm'],
      operandSize: 8,
      operation: ['A = A + %{IMM}'],
      cycles: 7,
    },

    // ADD HL, rp — 16-bit register pair
    {
      opcode: [Opcodes.ADD_HL_BC],
      operands: ['HL', 'BC'],
      operandSize: 16,
      modifies: ['H', 'N', 'C'],
      operation: ['HL = HL + BC'],
      cycles: 11,
    },
    {
      opcode: [Opcodes.ADD_HL_DE],
      operands: ['HL', 'DE'],
      operandSize: 16,
      modifies: ['H', 'N', 'C'],
      operation: ['HL = HL + DE'],
      cycles: 11,
    },
    {
      opcode: [Opcodes.ADD_HL_HL],
      operands: ['HL', 'HL'],
      operandSize: 16,
      modifies: ['H', 'N', 'C'],
      operation: ['HL = HL + HL'],
      cycles: 11,
    },
    {
      opcode: [Opcodes.ADD_HL_SP],
      operands: ['HL', 'SP'],
      operandSize: 16,
      modifies: ['H', 'N', 'C'],
      operation: ['HL = HL + SP'],
      cycles: 11,
    },
  ],
};
