import {
  InstructionDataTypes,
  InstructionOperandTypes,
  type InstructionInfo,
} from '@machinery/core';

import { Opcodes } from '../opcodes';

export const in_: InstructionInfo = {
  identifier: 'in',
  name: 'Input from Port',
  description: 'Reads a byte from the specified I/O port.',
  modifies: [],
  forms: [
    // IN A, (n) — port from immediate
    {
      opcode: [Opcodes.IN_A_xNx, 'PORT_u8'],
      operands: ['A', '(imm)'],
      operandSize: 8,
      operation: ['A = IO:u8[%{PORT}]'],
      cycles: 11,
    },
    // IN r, (C) — port from C register (ED prefix)
    {
      opcode: [
        Opcodes.ED_PREFIX,
        {
          identifier: 'Opcode_IN_B_xCx',
          name: 'IN B, (C) Opcode Field',
          type: InstructionDataTypes.Operand,
          size: 8,
          fields: [
            {
              identifier: 'rm',
              offset: 0,
              size: 3,
              match: 0b000,
              type: InstructionOperandTypes.Memory,
              encoding: ['C'],
            },
            {
              identifier: 'reg',
              offset: 3,
              size: 3,
              match: 0b000,
            },
            {
              identifier: 'opcode_high',
              offset: 6,
              size: 2,
              match: 0b01,
            },
          ],
        },
      ],
      operands: ['B', 'rm'],
      modifies: ['SF', 'ZF', 'HF', 'PF', 'NF'],
      operation: ['B = IO:u8[C]'],
      cycles: 12,
    },
    {
      opcode: [
        Opcodes.ED_PREFIX,
        {
          identifier: 'Opcode_IN_C_xCx',
          name: 'IN C, (C) Opcode Field',
          type: InstructionDataTypes.Operand,
          size: 8,
          fields: [
            {
              identifier: 'rm',
              offset: 0,
              size: 3,
              match: 0b000,
              type: InstructionOperandTypes.Memory,
              encoding: ['C'],
            },
            {
              identifier: 'reg',
              offset: 3,
              size: 3,
              match: 0b001,
            },
            {
              identifier: 'opcode_high',
              offset: 6,
              size: 2,
              match: 0b01,
            },
          ],
        },
      ],
      operands: ['C', 'rm'],
      modifies: ['SF', 'ZF', 'HF', 'PF', 'NF'],
      operation: ['C = IO:u8[C]'],
      cycles: 12,
    },
    {
      opcode: [
        Opcodes.ED_PREFIX,
        {
          identifier: 'Opcode_IN_D_xCx',
          name: 'IN D, (C) Opcode Field',
          type: InstructionDataTypes.Operand,
          size: 8,
          fields: [
            {
              identifier: 'rm',
              offset: 0,
              size: 3,
              match: 0b000,
              type: InstructionOperandTypes.Memory,
              encoding: ['C'],
            },
            {
              identifier: 'reg',
              offset: 3,
              size: 3,
              match: 0b010,
            },
            {
              identifier: 'opcode_high',
              offset: 6,
              size: 2,
              match: 0b01,
            },
          ],
        },
      ],
      operands: ['D', 'rm'],
      modifies: ['SF', 'ZF', 'HF', 'PF', 'NF'],
      operation: ['D = IO:u8[C]'],
      cycles: 12,
    },
    {
      opcode: [
        Opcodes.ED_PREFIX,
        {
          identifier: 'Opcode_IN_E_xCx',
          name: 'IN E, (C) Opcode Field',
          type: InstructionDataTypes.Operand,
          size: 8,
          fields: [
            {
              identifier: 'rm',
              offset: 0,
              size: 3,
              match: 0b000,
              type: InstructionOperandTypes.Memory,
              encoding: ['C'],
            },
            {
              identifier: 'reg',
              offset: 3,
              size: 3,
              match: 0b011,
            },
            {
              identifier: 'opcode_high',
              offset: 6,
              size: 2,
              match: 0b01,
            },
          ],
        },
      ],
      operands: ['E', 'rm'],
      modifies: ['SF', 'ZF', 'HF', 'PF', 'NF'],
      operation: ['E = IO:u8[C]'],
      cycles: 12,
    },
    {
      opcode: [
        Opcodes.ED_PREFIX,
        {
          identifier: 'Opcode_IN_H_xCx',
          name: 'IN H, (C) Opcode Field',
          type: InstructionDataTypes.Operand,
          size: 8,
          fields: [
            {
              identifier: 'rm',
              offset: 0,
              size: 3,
              match: 0b000,
              type: InstructionOperandTypes.Memory,
              encoding: ['C'],
            },
            {
              identifier: 'reg',
              offset: 3,
              size: 3,
              match: 0b100,
            },
            {
              identifier: 'opcode_high',
              offset: 6,
              size: 2,
              match: 0b01,
            },
          ],
        },
      ],
      operands: ['H', 'rm'],
      modifies: ['SF', 'ZF', 'HF', 'PF', 'NF'],
      operation: ['H = IO:u8[C]'],
      cycles: 12,
    },
    {
      opcode: [
        Opcodes.ED_PREFIX,
        {
          identifier: 'Opcode_IN_L_xCx',
          name: 'IN L, (C) Opcode Field',
          type: InstructionDataTypes.Operand,
          size: 8,
          fields: [
            {
              identifier: 'rm',
              offset: 0,
              size: 3,
              match: 0b000,
              type: InstructionOperandTypes.Memory,
              encoding: ['C'],
            },
            {
              identifier: 'reg',
              offset: 3,
              size: 3,
              match: 0b101,
            },
            {
              identifier: 'opcode_high',
              offset: 6,
              size: 2,
              match: 0b01,
            },
          ],
        },
      ],
      operands: ['L', 'rm'],
      modifies: ['SF', 'ZF', 'HF', 'PF', 'NF'],
      operation: ['L = IO:u8[C]'],
      cycles: 12,
    },
    {
      opcode: [
        Opcodes.ED_PREFIX,
        {
          identifier: 'Opcode_IN_A_xCx',
          name: 'IN A, (C) Opcode Field',
          type: InstructionDataTypes.Operand,
          size: 8,
          fields: [
            {
              identifier: 'rm',
              offset: 0,
              size: 3,
              match: 0b000,
              type: InstructionOperandTypes.Memory,
              encoding: ['C'],
            },
            {
              identifier: 'reg',
              offset: 3,
              size: 3,
              match: 0b111,
            },
            {
              identifier: 'opcode_high',
              offset: 6,
              size: 2,
              match: 0b01,
            },
          ],
        },
      ],
      operands: ['A', 'rm'],
      modifies: ['SF', 'ZF', 'HF', 'PF', 'NF'],
      operation: ['A = IO:u8[C]'],
      cycles: 12,
    },
    // IN (C) — affects flags only, result discarded
    {
      opcode: [
        Opcodes.ED_PREFIX,
        {
          identifier: 'Opcode_IN_xCx',
          name: 'IN (C) Opcode Field',
          type: InstructionDataTypes.Operand,
          size: 8,
          fields: [
            {
              identifier: 'rm',
              offset: 0,
              size: 3,
              match: 0b000,
              type: InstructionOperandTypes.Memory,
              encoding: ['C'],
            },
            {
              identifier: 'opcode_mid',
              offset: 3,
              size: 3,
              match: 0b110,
            },
            {
              identifier: 'opcode_high',
              offset: 6,
              size: 2,
              match: 0b01,
            },
          ],
        },
      ],
      operands: ['rm'],
      modifies: ['SF', 'ZF', 'HF', 'PF', 'NF'],
      operation: ['tmp = IO:u8[C]'],
      cycles: 12,
    },
  ],
};

export const out: InstructionInfo = {
  identifier: 'out',
  name: 'Output to Port',
  description: 'Writes a byte to the specified I/O port.',
  modifies: [],
  forms: [
    // OUT (n), A — port from immediate
    {
      opcode: [Opcodes.OUT_xNx_A, 'PORT_u8'],
      operands: ['(imm)', 'A'],
      operandSize: 8,
      operation: ['IO:u8[%{PORT}] = A'],
      cycles: 11,
    },
    // OUT (C), r — port from C register (ED prefix)
    {
      opcode: [
        Opcodes.ED_PREFIX,
        {
          identifier: 'Opcode_OUT_xCx_B',
          name: 'OUT (C), B Opcode Field',
          type: InstructionDataTypes.Operand,
          size: 8,
          fields: [
            {
              identifier: 'rm',
              offset: 0,
              size: 3,
              match: 0b001,
              type: InstructionOperandTypes.Memory,
              encoding: ['C'],
            },
            {
              identifier: 'reg',
              offset: 3,
              size: 3,
              match: 0b000,
            },
            {
              identifier: 'opcode_high',
              offset: 6,
              size: 2,
              match: 0b01,
            },
          ],
        },
      ],
      operands: ['rm', 'B'],
      operation: ['IO:u8[C] = B'],
      cycles: 12,
    },
    {
      opcode: [
        Opcodes.ED_PREFIX,
        {
          identifier: 'Opcode_OUT_xCx_C',
          name: 'OUT (C), C Opcode Field',
          type: InstructionDataTypes.Operand,
          size: 8,
          fields: [
            {
              identifier: 'rm',
              offset: 0,
              size: 3,
              match: 0b001,
              type: InstructionOperandTypes.Memory,
              encoding: ['C'],
            },
            {
              identifier: 'reg',
              offset: 3,
              size: 3,
              match: 0b001,
            },
            {
              identifier: 'opcode_high',
              offset: 6,
              size: 2,
              match: 0b01,
            },
          ],
        },
      ],
      operands: ['rm', 'C'],
      operation: ['IO:u8[C] = C'],
      cycles: 12,
    },
    {
      opcode: [
        Opcodes.ED_PREFIX,
        {
          identifier: 'Opcode_OUT_xCx_D',
          name: 'OUT (C), D Opcode Field',
          type: InstructionDataTypes.Operand,
          size: 8,
          fields: [
            {
              identifier: 'rm',
              offset: 0,
              size: 3,
              match: 0b001,
              type: InstructionOperandTypes.Memory,
              encoding: ['C'],
            },
            {
              identifier: 'reg',
              offset: 3,
              size: 3,
              match: 0b010,
            },
            {
              identifier: 'opcode_high',
              offset: 6,
              size: 2,
              match: 0b01,
            },
          ],
        },
      ],
      operands: ['rm', 'D'],
      operation: ['IO:u8[C] = D'],
      cycles: 12,
    },
    {
      opcode: [
        Opcodes.ED_PREFIX,
        {
          identifier: 'Opcode_OUT_xCx_E',
          name: 'OUT (C), E Opcode Field',
          type: InstructionDataTypes.Operand,
          size: 8,
          fields: [
            {
              identifier: 'rm',
              offset: 0,
              size: 3,
              match: 0b001,
              type: InstructionOperandTypes.Memory,
              encoding: ['C'],
            },
            {
              identifier: 'reg',
              offset: 3,
              size: 3,
              match: 0b011,
            },
            {
              identifier: 'opcode_high',
              offset: 6,
              size: 2,
              match: 0b01,
            },
          ],
        },
      ],
      operands: ['rm', 'E'],
      operation: ['IO:u8[C] = E'],
      cycles: 12,
    },
    {
      opcode: [
        Opcodes.ED_PREFIX,
        {
          identifier: 'Opcode_OUT_xCx_H',
          name: 'OUT (C), H Opcode Field',
          type: InstructionDataTypes.Operand,
          size: 8,
          fields: [
            {
              identifier: 'rm',
              offset: 0,
              size: 3,
              match: 0b001,
              type: InstructionOperandTypes.Memory,
              encoding: ['C'],
            },
            {
              identifier: 'reg',
              offset: 3,
              size: 3,
              match: 0b100,
            },
            {
              identifier: 'opcode_high',
              offset: 6,
              size: 2,
              match: 0b01,
            },
          ],
        },
      ],
      operands: ['rm', 'H'],
      operation: ['IO:u8[C] = H'],
      cycles: 12,
    },
    {
      opcode: [
        Opcodes.ED_PREFIX,
        {
          identifier: 'Opcode_OUT_xCx_L',
          name: 'OUT (C), L Opcode Field',
          type: InstructionDataTypes.Operand,
          size: 8,
          fields: [
            {
              identifier: 'rm',
              offset: 0,
              size: 3,
              match: 0b001,
              type: InstructionOperandTypes.Memory,
              encoding: ['C'],
            },
            {
              identifier: 'reg',
              offset: 3,
              size: 3,
              match: 0b101,
            },
            {
              identifier: 'opcode_high',
              offset: 6,
              size: 2,
              match: 0b01,
            },
          ],
        },
      ],
      operands: ['rm', 'L'],
      operation: ['IO:u8[C] = L'],
      cycles: 12,
    },
    {
      opcode: [
        Opcodes.ED_PREFIX,
        {
          identifier: 'Opcode_OUT_xCx_A',
          name: 'OUT (C), A Opcode Field',
          type: InstructionDataTypes.Operand,
          size: 8,
          fields: [
            {
              identifier: 'rm',
              offset: 0,
              size: 3,
              match: 0b001,
              type: InstructionOperandTypes.Memory,
              encoding: ['C'],
            },
            {
              identifier: 'reg',
              offset: 3,
              size: 3,
              match: 0b111,
            },
            {
              identifier: 'opcode_high',
              offset: 6,
              size: 2,
              match: 0b01,
            },
          ],
        },
      ],
      operands: ['rm', 'A'],
      operation: ['IO:u8[C] = A'],
      cycles: 12,
    },
    // OUT (C), 0
    {
      opcode: [
        Opcodes.ED_PREFIX,
        {
          identifier: 'Opcode_OUT_xCx_0',
          name: 'OUT (C), 0 Opcode Field',
          type: InstructionDataTypes.Operand,
          size: 8,
          fields: [
            {
              identifier: 'rm',
              offset: 0,
              size: 3,
              match: 0b001,
              type: InstructionOperandTypes.Memory,
              encoding: ['C'],
            },
            {
              identifier: 'reg',
              offset: 3,
              size: 3,
              match: 0b110,
            },
            {
              identifier: 'opcode_high',
              offset: 6,
              size: 2,
              match: 0b01,
            },
          ],
        },
      ],
      operands: ['rm', '0'],
      operation: ['IO:u8[C] = 0'],
      cycles: 12,
    },
  ],
};
