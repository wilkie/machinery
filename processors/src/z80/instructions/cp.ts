import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const cp: InstructionInfo = {
  identifier: 'cp',
  name: 'Compare',
  description:
    'Subtracts the operand from A but discards the result, only setting flags. A is not modified.',
  modifies: ['S', 'Z', 'H', 'P', 'N', 'C'],
  forms: [
    {
      opcode: [Opcodes.CP_B],
      operands: ['B'],
      operandSize: 8,
      operation: ['tmp = A - B'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.CP_C],
      operands: ['C'],
      operandSize: 8,
      operation: ['tmp = A - C'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.CP_D],
      operands: ['D'],
      operandSize: 8,
      operation: ['tmp = A - D'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.CP_E],
      operands: ['E'],
      operandSize: 8,
      operation: ['tmp = A - E'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.CP_H],
      operands: ['H'],
      operandSize: 8,
      operation: ['tmp = A - H'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.CP_L],
      operands: ['L'],
      operandSize: 8,
      operation: ['tmp = A - L'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.CP_xHLx],
      operands: ['(HL)'],
      operandSize: 8,
      operation: ['tmp = A - RAM:u8[HL]'],
      cycles: 7,
    },
    {
      opcode: [Opcodes.CP_A],
      operands: ['A'],
      operandSize: 8,
      operation: ['tmp = A - A'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.CP_N, 'IMM_u8'],
      operands: ['imm'],
      operandSize: 8,
      operation: ['tmp = A - %{IMM}'],
      cycles: 7,
    },
  ],
};
