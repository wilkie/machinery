import type { InstructionInfo } from '@machinery/core';

import { Opcodes, EDOpcodes } from '../opcodes';

export const adc: InstructionInfo = {
  identifier: 'adc',
  name: 'Add with Carry',
  description:
    'Adds the operand and the carry flag to the accumulator (8-bit) or to HL (16-bit).',
  modifies: ['S', 'Z', 'H', 'P', 'N', 'C'],
  forms: [
    // ADC A, r
    {
      opcode: [Opcodes.ADC_A_B],
      operands: ['A', 'B'],
      operandSize: 8,
      operation: ['A = A + B + CF'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.ADC_A_C],
      operands: ['A', 'C'],
      operandSize: 8,
      operation: ['A = A + C + CF'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.ADC_A_D],
      operands: ['A', 'D'],
      operandSize: 8,
      operation: ['A = A + D + CF'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.ADC_A_E],
      operands: ['A', 'E'],
      operandSize: 8,
      operation: ['A = A + E + CF'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.ADC_A_H],
      operands: ['A', 'H'],
      operandSize: 8,
      operation: ['A = A + H + CF'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.ADC_A_L],
      operands: ['A', 'L'],
      operandSize: 8,
      operation: ['A = A + L + CF'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.ADC_A_xHLx],
      operands: ['A', '(HL)'],
      operandSize: 8,
      operation: ['A = A + RAM:u8[HL] + CF'],
      cycles: 7,
    },
    {
      opcode: [Opcodes.ADC_A_A],
      operands: ['A', 'A'],
      operandSize: 8,
      operation: ['A = A + A + CF'],
      cycles: 4,
    },

    // ADC A, n
    {
      opcode: [Opcodes.ADC_A_N, 'IMM_u8'],
      operands: ['A', 'imm'],
      operandSize: 8,
      operation: ['A = A + %{IMM} + CF'],
      cycles: 7,
    },

    // ADC HL, rp (ED prefix)
    {
      opcode: [Opcodes.ED_PREFIX, EDOpcodes.ADC_HL_BC],
      operands: ['HL', 'BC'],
      operandSize: 16,
      operation: ['HL = HL + BC + CF'],
      cycles: 15,
    },
    {
      opcode: [Opcodes.ED_PREFIX, EDOpcodes.ADC_HL_DE],
      operands: ['HL', 'DE'],
      operandSize: 16,
      operation: ['HL = HL + DE + CF'],
      cycles: 15,
    },
    {
      opcode: [Opcodes.ED_PREFIX, EDOpcodes.ADC_HL_HL],
      operands: ['HL', 'HL'],
      operandSize: 16,
      operation: ['HL = HL + HL + CF'],
      cycles: 15,
    },
    {
      opcode: [Opcodes.ED_PREFIX, EDOpcodes.ADC_HL_SP],
      operands: ['HL', 'SP'],
      operandSize: 16,
      operation: ['HL = HL + SP + CF'],
      cycles: 15,
    },
  ],
};
