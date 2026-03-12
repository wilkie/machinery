import {
  InstructionDataTypes,
  InstructionOperandTypes,
  type InstructionInfo,
} from '@machinery/core';

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
      operation: ['PC = %{imm}'],
      cycles: 10,
    },
    // JP cc, nn — conditional
    {
      opcode: [Opcodes.JP_NZ_NN, 'IMM_u16'],
      operands: ['imm'],
      operandSize: 16,
      aliases: ['jp nz'],
      addressing: 'absolute',
      operation: ['${RESOLVE_ZF}', 'PC = (ZF == 0) ? %{imm} : PC'],
      cycles: 10,
    },
    {
      opcode: [Opcodes.JP_Z_NN, 'IMM_u16'],
      operands: ['imm'],
      operandSize: 16,
      aliases: ['jp z'],
      addressing: 'absolute',
      operation: ['${RESOLVE_ZF}', 'PC = (ZF == 1) ? %{imm} : PC'],
      cycles: 10,
    },
    {
      opcode: [Opcodes.JP_NC_NN, 'IMM_u16'],
      operands: ['imm'],
      operandSize: 16,
      aliases: ['jp nc'],
      addressing: 'absolute',
      operation: ['${RESOLVE_CF}', 'PC = (CF == 0) ? %{imm} : PC'],
      cycles: 10,
    },
    {
      opcode: [Opcodes.JP_C_NN, 'IMM_u16'],
      operands: ['imm'],
      operandSize: 16,
      aliases: ['jp c'],
      addressing: 'absolute',
      operation: ['${RESOLVE_CF}', 'PC = (CF == 1) ? %{imm} : PC'],
      cycles: 10,
    },
    {
      opcode: [Opcodes.JP_PO_NN, 'IMM_u16'],
      operands: ['imm'],
      operandSize: 16,
      aliases: ['jp po'],
      addressing: 'absolute',
      operation: ['${RESOLVE_PF}', 'PC = (PF == 0) ? %{imm} : PC'],
      cycles: 10,
    },
    {
      opcode: [Opcodes.JP_PE_NN, 'IMM_u16'],
      operands: ['imm'],
      operandSize: 16,
      aliases: ['jp pe'],
      addressing: 'absolute',
      operation: ['${RESOLVE_PF}', 'PC = (PF == 1) ? %{imm} : PC'],
      cycles: 10,
    },
    {
      opcode: [Opcodes.JP_P_NN, 'IMM_u16'],
      operands: ['imm'],
      operandSize: 16,
      aliases: ['jp p'],
      addressing: 'absolute',
      operation: ['${RESOLVE_SF}', 'PC = (SF == 0) ? %{imm} : PC'],
      cycles: 10,
    },
    {
      opcode: [Opcodes.JP_M_NN, 'IMM_u16'],
      operands: ['imm'],
      operandSize: 16,
      aliases: ['jp m'],
      addressing: 'absolute',
      operation: ['${RESOLVE_SF}', 'PC = (SF == 1) ? %{imm} : PC'],
      cycles: 10,
    },
    // JP (HL) — indirect
    {
      opcode: [
        {
          identifier: 'Opcode_JP_xHLx',
          name: 'JP (HL) Opcode Field',
          type: InstructionDataTypes.Operand,
          size: 8,
          fields: [
            {
              identifier: 'opcode_low',
              offset: 0,
              size: 3,
              match: 0b001,
            },
            {
              identifier: 'rm',
              offset: 3,
              size: 3,
              match: 0b101,
              type: InstructionOperandTypes.Memory,
              encoding: ['HL'],
            },
            {
              identifier: 'opcode_high',
              offset: 6,
              size: 2,
              match: 0b11,
            },
          ],
        },
      ],
      operands: ['rm'],
      addressing: 'absolute',
      operation: ['PC = HL'],
      cycles: 4,
    },
    // JP (IX)
    {
      opcode: [
        Opcodes.DD_PREFIX,
        {
          identifier: 'Opcode_JP_xIXx',
          name: 'JP (IX) Opcode Field',
          type: InstructionDataTypes.Operand,
          size: 8,
          fields: [
            {
              identifier: 'opcode_low',
              offset: 0,
              size: 3,
              match: 0b001,
            },
            {
              identifier: 'rm',
              offset: 3,
              size: 3,
              match: 0b101,
              type: InstructionOperandTypes.Memory,
              encoding: ['IX'],
            },
            {
              identifier: 'opcode_high',
              offset: 6,
              size: 2,
              match: 0b11,
            },
          ],
        },
      ],
      operands: ['rm'],
      addressing: 'absolute',
      operation: ['PC = IX'],
      cycles: 8,
    },
    // JP (IY)
    {
      opcode: [
        Opcodes.FD_PREFIX,
        {
          identifier: 'Opcode_JP_xIYx',
          name: 'JP (IY) Opcode Field',
          type: InstructionDataTypes.Operand,
          size: 8,
          fields: [
            {
              identifier: 'opcode_low',
              offset: 0,
              size: 3,
              match: 0b001,
            },
            {
              identifier: 'rm',
              offset: 3,
              size: 3,
              match: 0b101,
              type: InstructionOperandTypes.Memory,
              encoding: ['IY'],
            },
            {
              identifier: 'opcode_high',
              offset: 6,
              size: 2,
              match: 0b11,
            },
          ],
        },
      ],
      operands: ['rm'],
      addressing: 'absolute',
      operation: ['PC = IY'],
      cycles: 8,
    },
  ],
};
