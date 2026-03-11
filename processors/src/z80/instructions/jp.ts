import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const jp: InstructionInfo = {
  identifier: 'jp',
  name: 'Jump',
  description: 'Unconditional or conditional jump to an absolute address.',
  modifies: [],
  forms: [
    // JP nn — unconditional
    {
      opcode: [Opcodes.JP_NN, 'IMM_u16'],
      operands: ['imm'],
      operandSize: 16,
      addressing: 'absolute',
      operation: ['PC = %{IMM}'],
      cycles: 10,
    },
    // JP cc, nn — conditional
    {
      opcode: [Opcodes.JP_NZ_NN, 'IMM_u16'],
      operands: ['imm'],
      operandSize: 16,
      aliases: ['jp nz'],
      addressing: 'absolute',
      operation: ['if (ZF == 0) PC = %{IMM}'],
      cycles: 10,
    },
    {
      opcode: [Opcodes.JP_Z_NN, 'IMM_u16'],
      operands: ['imm'],
      operandSize: 16,
      aliases: ['jp z'],
      addressing: 'absolute',
      operation: ['if (ZF == 1) PC = %{IMM}'],
      cycles: 10,
    },
    {
      opcode: [Opcodes.JP_NC_NN, 'IMM_u16'],
      operands: ['imm'],
      operandSize: 16,
      aliases: ['jp nc'],
      addressing: 'absolute',
      operation: ['if (CF == 0) PC = %{IMM}'],
      cycles: 10,
    },
    {
      opcode: [Opcodes.JP_C_NN, 'IMM_u16'],
      operands: ['imm'],
      operandSize: 16,
      aliases: ['jp c'],
      addressing: 'absolute',
      operation: ['if (CF == 1) PC = %{IMM}'],
      cycles: 10,
    },
    {
      opcode: [Opcodes.JP_PO_NN, 'IMM_u16'],
      operands: ['imm'],
      operandSize: 16,
      aliases: ['jp po'],
      addressing: 'absolute',
      operation: ['if (PF == 0) PC = %{IMM}'],
      cycles: 10,
    },
    {
      opcode: [Opcodes.JP_PE_NN, 'IMM_u16'],
      operands: ['imm'],
      operandSize: 16,
      aliases: ['jp pe'],
      addressing: 'absolute',
      operation: ['if (PF == 1) PC = %{IMM}'],
      cycles: 10,
    },
    {
      opcode: [Opcodes.JP_P_NN, 'IMM_u16'],
      operands: ['imm'],
      operandSize: 16,
      aliases: ['jp p'],
      addressing: 'absolute',
      operation: ['if (SF == 0) PC = %{IMM}'],
      cycles: 10,
    },
    {
      opcode: [Opcodes.JP_M_NN, 'IMM_u16'],
      operands: ['imm'],
      operandSize: 16,
      aliases: ['jp m'],
      addressing: 'absolute',
      operation: ['if (SF == 1) PC = %{IMM}'],
      cycles: 10,
    },
    // JP (HL) — indirect
    {
      opcode: [Opcodes.JP_xHLx],
      operands: ['(HL)'],
      addressing: 'absolute',
      operation: ['PC = HL'],
      cycles: 4,
    },
  ],
};
