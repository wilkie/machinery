import type { InstructionInfo } from '@machinery/core';
import { InstructionDataTypes, InstructionOperandTypes } from '@machinery/core';

import { Opcodes, EDOpcodes } from '../opcodes';

export const ld: InstructionInfo = {
  identifier: 'ld',
  name: 'Load',
  description: 'Loads the source operand into the destination operand.',
  modifies: [],
  forms: [
    // --- 8-bit register to register: LD r, r' ---
    // 0x40-0x7F block (excluding 0x76 = HALT and (HL),(HL))
    // LD B, r
    {
      opcode: [Opcodes.LD_B_B],
      operands: ['B', 'B'],
      operation: [],
      cycles: 4,
    },
    {
      opcode: [Opcodes.LD_B_C],
      operands: ['B', 'C'],
      operation: ['B = C'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.LD_B_D],
      operands: ['B', 'D'],
      operation: ['B = D'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.LD_B_E],
      operands: ['B', 'E'],
      operation: ['B = E'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.LD_B_H],
      operands: ['B', 'H'],
      operation: ['B = H'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.LD_B_L],
      operands: ['B', 'L'],
      operation: ['B = L'],
      cycles: 4,
    },
    {
      opcode: [
        {
          identifier: 'Opcode_LD_B_xHLx',
          name: 'LD B, (HL) Opcode Field',
          type: InstructionDataTypes.Operand,
          size: 8,
          fields: [
            {
              identifier: 'opcode',
              offset: 3,
              size: 5,
              match: 0b01000,
            },
            {
              identifier: 'rm',
              offset: 0,
              size: 3,
              match: 0b110,
              type: InstructionOperandTypes.Memory,
              encoding: ['HL'],
            },
          ],
        },
      ],
      operands: ['B', 'rm'],
      operation: ['B = RAM:u8[HL]'],
      cycles: 7,
    },
    {
      opcode: [Opcodes.LD_B_A],
      operands: ['B', 'A'],
      operation: ['B = A'],
      cycles: 4,
    },
    // LD C, r
    {
      opcode: [Opcodes.LD_C_B],
      operands: ['C', 'B'],
      operation: ['C = B'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.LD_C_C],
      operands: ['C', 'C'],
      operation: [],
      cycles: 4,
    },
    {
      opcode: [Opcodes.LD_C_D],
      operands: ['C', 'D'],
      operation: ['C = D'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.LD_C_E],
      operands: ['C', 'E'],
      operation: ['C = E'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.LD_C_H],
      operands: ['C', 'H'],
      operation: ['C = H'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.LD_C_L],
      operands: ['C', 'L'],
      operation: ['C = L'],
      cycles: 4,
    },
    {
      opcode: [
        {
          identifier: 'Opcode_LD_C_xHLx',
          name: 'LD C, (HL) Opcode Field',
          type: InstructionDataTypes.Operand,
          size: 8,
          fields: [
            {
              identifier: 'opcode',
              offset: 3,
              size: 5,
              match: 0b01001,
            },
            {
              identifier: 'rm',
              offset: 0,
              size: 3,
              match: 0b110,
              type: InstructionOperandTypes.Memory,
              encoding: ['HL'],
            },
          ],
        },
      ],
      operands: ['C', 'rm'],
      operation: ['C = RAM:u8[HL]'],
      cycles: 7,
    },
    {
      opcode: [Opcodes.LD_C_A],
      operands: ['C', 'A'],
      operation: ['C = A'],
      cycles: 4,
    },
    // LD D, r
    {
      opcode: [Opcodes.LD_D_B],
      operands: ['D', 'B'],
      operation: ['D = B'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.LD_D_C],
      operands: ['D', 'C'],
      operation: ['D = C'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.LD_D_D],
      operands: ['D', 'D'],
      operation: [],
      cycles: 4,
    },
    {
      opcode: [Opcodes.LD_D_E],
      operands: ['D', 'E'],
      operation: ['D = E'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.LD_D_H],
      operands: ['D', 'H'],
      operation: ['D = H'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.LD_D_L],
      operands: ['D', 'L'],
      operation: ['D = L'],
      cycles: 4,
    },
    {
      opcode: [
        {
          identifier: 'Opcode_LD_D_xHLx',
          name: 'LD D, (HL) Opcode Field',
          type: InstructionDataTypes.Operand,
          size: 8,
          fields: [
            {
              identifier: 'opcode',
              offset: 3,
              size: 5,
              match: 0b01010,
            },
            {
              identifier: 'rm',
              offset: 0,
              size: 3,
              match: 0b110,
              type: InstructionOperandTypes.Memory,
              encoding: ['HL'],
            },
          ],
        },
      ],
      operands: ['D', 'rm'],
      operation: ['D = RAM:u8[HL]'],
      cycles: 7,
    },
    {
      opcode: [Opcodes.LD_D_A],
      operands: ['D', 'A'],
      operation: ['D = A'],
      cycles: 4,
    },
    // LD E, r
    {
      opcode: [Opcodes.LD_E_B],
      operands: ['E', 'B'],
      operation: ['E = B'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.LD_E_C],
      operands: ['E', 'C'],
      operation: ['E = C'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.LD_E_D],
      operands: ['E', 'D'],
      operation: ['E = D'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.LD_E_E],
      operands: ['E', 'E'],
      operation: [],
      cycles: 4,
    },
    {
      opcode: [Opcodes.LD_E_H],
      operands: ['E', 'H'],
      operation: ['E = H'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.LD_E_L],
      operands: ['E', 'L'],
      operation: ['E = L'],
      cycles: 4,
    },
    {
      opcode: [
        {
          identifier: 'Opcode_LD_E_xHLx',
          name: 'LD E, (HL) Opcode Field',
          type: InstructionDataTypes.Operand,
          size: 8,
          fields: [
            {
              identifier: 'opcode',
              offset: 3,
              size: 5,
              match: 0b01011,
            },
            {
              identifier: 'rm',
              offset: 0,
              size: 3,
              match: 0b110,
              type: InstructionOperandTypes.Memory,
              encoding: ['HL'],
            },
          ],
        },
      ],
      operands: ['E', 'rm'],
      operation: ['E = RAM:u8[HL]'],
      cycles: 7,
    },
    {
      opcode: [Opcodes.LD_E_A],
      operands: ['E', 'A'],
      operation: ['E = A'],
      cycles: 4,
    },
    // LD H, r
    {
      opcode: [Opcodes.LD_H_B],
      operands: ['H', 'B'],
      operation: ['H = B'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.LD_H_C],
      operands: ['H', 'C'],
      operation: ['H = C'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.LD_H_D],
      operands: ['H', 'D'],
      operation: ['H = D'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.LD_H_E],
      operands: ['H', 'E'],
      operation: ['H = E'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.LD_H_H],
      operands: ['H', 'H'],
      operation: [],
      cycles: 4,
    },
    {
      opcode: [Opcodes.LD_H_L],
      operands: ['H', 'L'],
      operation: ['H = L'],
      cycles: 4,
    },
    {
      opcode: [
        {
          identifier: 'Opcode_LD_H_xHLx',
          name: 'LD H, (HL) Opcode Field',
          type: InstructionDataTypes.Operand,
          size: 8,
          fields: [
            {
              identifier: 'opcode',
              offset: 3,
              size: 5,
              match: 0b01100,
            },
            {
              identifier: 'rm',
              offset: 0,
              size: 3,
              match: 0b110,
              type: InstructionOperandTypes.Memory,
              encoding: ['HL'],
            },
          ],
        },
      ],
      operands: ['H', 'rm'],
      operation: ['H = RAM:u8[HL]'],
      cycles: 7,
    },
    {
      opcode: [Opcodes.LD_H_A],
      operands: ['H', 'A'],
      operation: ['H = A'],
      cycles: 4,
    },
    // LD L, r
    {
      opcode: [Opcodes.LD_L_B],
      operands: ['L', 'B'],
      operation: ['L = B'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.LD_L_C],
      operands: ['L', 'C'],
      operation: ['L = C'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.LD_L_D],
      operands: ['L', 'D'],
      operation: ['L = D'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.LD_L_E],
      operands: ['L', 'E'],
      operation: ['L = E'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.LD_L_H],
      operands: ['L', 'H'],
      operation: ['L = H'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.LD_L_L],
      operands: ['L', 'L'],
      operation: [],
      cycles: 4,
    },
    {
      opcode: [
        {
          identifier: 'Opcode_LD_L_xHLx',
          name: 'LD L, (HL) Opcode Field',
          type: InstructionDataTypes.Operand,
          size: 8,
          fields: [
            {
              identifier: 'opcode',
              offset: 3,
              size: 5,
              match: 0b01101,
            },
            {
              identifier: 'rm',
              offset: 0,
              size: 3,
              match: 0b110,
              type: InstructionOperandTypes.Memory,
              encoding: ['HL'],
            },
          ],
        },
      ],
      operands: ['L', 'rm'],
      operation: ['L = RAM:u8[HL]'],
      cycles: 7,
    },
    {
      opcode: [Opcodes.LD_L_A],
      operands: ['L', 'A'],
      operation: ['L = A'],
      cycles: 4,
    },
    // LD (HL), r
    {
      opcode: [
        {
          identifier: 'Opcode_LD_xHLx_B',
          name: 'LD (HL), B Opcode Field',
          type: InstructionDataTypes.Operand,
          size: 8,
          fields: [
            {
              identifier: 'opcode',
              offset: 3,
              size: 5,
              match: 0b01110,
            },
            {
              identifier: 'rm',
              offset: 0,
              size: 3,
              match: 0b000,
              type: InstructionOperandTypes.Memory,
              encoding: ['HL'],
            },
          ],
        },
      ],
      operands: ['rm', 'B'],
      operation: ['RAM:u8[HL] = B'],
      cycles: 7,
    },
    {
      opcode: [
        {
          identifier: 'Opcode_LD_xHLx_C',
          name: 'LD (HL), C Opcode Field',
          type: InstructionDataTypes.Operand,
          size: 8,
          fields: [
            {
              identifier: 'opcode',
              offset: 3,
              size: 5,
              match: 0b01110,
            },
            {
              identifier: 'rm',
              offset: 0,
              size: 3,
              match: 0b001,
              type: InstructionOperandTypes.Memory,
              encoding: ['HL'],
            },
          ],
        },
      ],
      operands: ['rm', 'C'],
      operation: ['RAM:u8[HL] = C'],
      cycles: 7,
    },
    {
      opcode: [
        {
          identifier: 'Opcode_LD_xHLx_D',
          name: 'LD (HL), D Opcode Field',
          type: InstructionDataTypes.Operand,
          size: 8,
          fields: [
            {
              identifier: 'opcode',
              offset: 3,
              size: 5,
              match: 0b01110,
            },
            {
              identifier: 'rm',
              offset: 0,
              size: 3,
              match: 0b010,
              type: InstructionOperandTypes.Memory,
              encoding: ['HL'],
            },
          ],
        },
      ],
      operands: ['rm', 'D'],
      operation: ['RAM:u8[HL] = D'],
      cycles: 7,
    },
    {
      opcode: [
        {
          identifier: 'Opcode_LD_xHLx_E',
          name: 'LD (HL), E Opcode Field',
          type: InstructionDataTypes.Operand,
          size: 8,
          fields: [
            {
              identifier: 'opcode',
              offset: 3,
              size: 5,
              match: 0b01110,
            },
            {
              identifier: 'rm',
              offset: 0,
              size: 3,
              match: 0b011,
              type: InstructionOperandTypes.Memory,
              encoding: ['HL'],
            },
          ],
        },
      ],
      operands: ['rm', 'E'],
      operation: ['RAM:u8[HL] = E'],
      cycles: 7,
    },
    {
      opcode: [
        {
          identifier: 'Opcode_LD_xHLx_H',
          name: 'LD (HL), H Opcode Field',
          type: InstructionDataTypes.Operand,
          size: 8,
          fields: [
            {
              identifier: 'opcode',
              offset: 3,
              size: 5,
              match: 0b01110,
            },
            {
              identifier: 'rm',
              offset: 0,
              size: 3,
              match: 0b100,
              type: InstructionOperandTypes.Memory,
              encoding: ['HL'],
            },
          ],
        },
      ],
      operands: ['rm', 'H'],
      operation: ['RAM:u8[HL] = H'],
      cycles: 7,
    },
    {
      opcode: [
        {
          identifier: 'Opcode_LD_xHLx_L',
          name: 'LD (HL), L Opcode Field',
          type: InstructionDataTypes.Operand,
          size: 8,
          fields: [
            {
              identifier: 'opcode',
              offset: 3,
              size: 5,
              match: 0b01110,
            },
            {
              identifier: 'rm',
              offset: 0,
              size: 3,
              match: 0b101,
              type: InstructionOperandTypes.Memory,
              encoding: ['HL'],
            },
          ],
        },
      ],
      operands: ['rm', 'L'],
      operation: ['RAM:u8[HL] = L'],
      cycles: 7,
    },
    {
      opcode: [
        {
          identifier: 'Opcode_LD_xHLx_A',
          name: 'LD (HL), A Opcode Field',
          type: InstructionDataTypes.Operand,
          size: 8,
          fields: [
            {
              identifier: 'opcode',
              offset: 3,
              size: 5,
              match: 0b01110,
            },
            {
              identifier: 'rm',
              offset: 0,
              size: 3,
              match: 0b111,
              type: InstructionOperandTypes.Memory,
              encoding: ['HL'],
            },
          ],
        },
      ],
      operands: ['rm', 'A'],
      operation: ['RAM:u8[HL] = A'],
      cycles: 7,
    },
    // LD A, r
    {
      opcode: [Opcodes.LD_A_B],
      operands: ['A', 'B'],
      operation: ['A = B'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.LD_A_C],
      operands: ['A', 'C'],
      operation: ['A = C'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.LD_A_D],
      operands: ['A', 'D'],
      operation: ['A = D'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.LD_A_E],
      operands: ['A', 'E'],
      operation: ['A = E'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.LD_A_H],
      operands: ['A', 'H'],
      operation: ['A = H'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.LD_A_L],
      operands: ['A', 'L'],
      operation: ['A = L'],
      cycles: 4,
    },
    {
      opcode: [
        {
          identifier: 'Opcode_LD_A_xHLx',
          name: 'LD A, (HL) Opcode Field',
          type: InstructionDataTypes.Operand,
          size: 8,
          fields: [
            {
              identifier: 'opcode',
              offset: 3,
              size: 5,
              match: 0b01111,
            },
            {
              identifier: 'rm',
              offset: 0,
              size: 3,
              match: 0b110,
              type: InstructionOperandTypes.Memory,
              encoding: ['HL'],
            },
          ],
        },
      ],
      operands: ['A', 'rm'],
      operation: ['A = RAM:u8[HL]'],
      cycles: 7,
    },
    {
      opcode: [Opcodes.LD_A_A],
      operands: ['A', 'A'],
      operation: [],
      cycles: 4,
    },

    // --- 8-bit immediate loads: LD r, n ---
    {
      opcode: [Opcodes.LD_B_N, 'IMM_u8'],
      operands: ['B', 'imm'],
      operandSize: 8,
      operation: ['B = %{imm}'],
      cycles: 7,
    },
    {
      opcode: [Opcodes.LD_C_N, 'IMM_u8'],
      operands: ['C', 'imm'],
      operandSize: 8,
      operation: ['C = %{imm}'],
      cycles: 7,
    },
    {
      opcode: [Opcodes.LD_D_N, 'IMM_u8'],
      operands: ['D', 'imm'],
      operandSize: 8,
      operation: ['D = %{imm}'],
      cycles: 7,
    },
    {
      opcode: [Opcodes.LD_E_N, 'IMM_u8'],
      operands: ['E', 'imm'],
      operandSize: 8,
      operation: ['E = %{imm}'],
      cycles: 7,
    },
    {
      opcode: [Opcodes.LD_H_N, 'IMM_u8'],
      operands: ['H', 'imm'],
      operandSize: 8,
      operation: ['H = %{imm}'],
      cycles: 7,
    },
    {
      opcode: [Opcodes.LD_L_N, 'IMM_u8'],
      operands: ['L', 'imm'],
      operandSize: 8,
      operation: ['L = %{imm}'],
      cycles: 7,
    },
    {
      opcode: [
        {
          identifier: 'Opcode_LD_xHLx_N',
          name: 'LD (HL), N Opcode Field',
          type: InstructionDataTypes.Operand,
          size: 8,
          fields: [
            {
              identifier: 'opcode',
              offset: 3,
              size: 5,
              match: 0b00110,
            },
            {
              identifier: 'rm',
              offset: 0,
              size: 3,
              match: 0b110,
              type: InstructionOperandTypes.Memory,
              encoding: ['HL'],
            },
          ],
        },
        'IMM_u8',
      ],
      operands: ['rm', 'imm'],
      operandSize: 8,
      operation: ['RAM:u8[HL] = %{imm}'],
      cycles: 10,
    },
    {
      opcode: [Opcodes.LD_A_N, 'IMM_u8'],
      operands: ['A', 'imm'],
      operandSize: 8,
      operation: ['A = %{imm}'],
      cycles: 7,
    },

    // --- 16-bit immediate loads: LD rp, nn ---
    {
      opcode: [Opcodes.LD_BC_NN, 'IMM_u16'],
      operands: ['BC', 'imm'],
      operandSize: 16,
      operation: ['BC = %{imm}'],
      cycles: 10,
    },
    {
      opcode: [Opcodes.LD_DE_NN, 'IMM_u16'],
      operands: ['DE', 'imm'],
      operandSize: 16,
      operation: ['DE = %{imm}'],
      cycles: 10,
    },
    {
      opcode: [Opcodes.LD_HL_NN, 'IMM_u16'],
      operands: ['HL', 'imm'],
      operandSize: 16,
      operation: ['HL = %{imm}'],
      cycles: 10,
    },
    {
      opcode: [Opcodes.LD_SP_NN, 'IMM_u16'],
      operands: ['SP', 'imm'],
      operandSize: 16,
      operation: ['SP = %{imm}'],
      cycles: 10,
    },

    // --- Indirect loads ---
    // LD (BC), A / LD (DE), A
    {
      opcode: [
        {
          identifier: 'Opcode_LD_xBCx_A',
          name: 'LD (BC), A Opcode Field',
          type: InstructionDataTypes.Operand,
          size: 8,
          fields: [
            {
              identifier: 'opcode',
              offset: 3,
              size: 5,
              match: 0b00000,
            },
            {
              identifier: 'rm',
              offset: 0,
              size: 3,
              match: 0b010,
              type: InstructionOperandTypes.Memory,
              encoding: ['BC'],
            },
          ],
        },
      ],
      operands: ['rm', 'A'],
      operation: ['RAM:u8[BC] = A'],
      cycles: 7,
    },
    {
      opcode: [
        {
          identifier: 'Opcode_LD_xDEx_A',
          name: 'LD (DE), A Opcode Field',
          type: InstructionDataTypes.Operand,
          size: 8,
          fields: [
            {
              identifier: 'opcode',
              offset: 3,
              size: 5,
              match: 0b00010,
            },
            {
              identifier: 'rm',
              offset: 0,
              size: 3,
              match: 0b010,
              type: InstructionOperandTypes.Memory,
              encoding: ['DE'],
            },
          ],
        },
      ],
      operands: ['rm', 'A'],
      operation: ['RAM:u8[DE] = A'],
      cycles: 7,
    },
    // LD A, (BC) / LD A, (DE)
    {
      opcode: [
        {
          identifier: 'Opcode_LD_A_xBCx',
          name: 'LD A, (BC) Opcode Field',
          type: InstructionDataTypes.Operand,
          size: 8,
          fields: [
            {
              identifier: 'opcode',
              offset: 3,
              size: 5,
              match: 0b00001,
            },
            {
              identifier: 'rm',
              offset: 0,
              size: 3,
              match: 0b010,
              type: InstructionOperandTypes.Memory,
              encoding: ['BC'],
            },
          ],
        },
      ],
      operands: ['A', 'rm'],
      operation: ['A = RAM:u8[BC]'],
      cycles: 7,
    },
    {
      opcode: [
        {
          identifier: 'Opcode_LD_A_xDEx',
          name: 'LD A, (DE) Opcode Field',
          type: InstructionDataTypes.Operand,
          size: 8,
          fields: [
            {
              identifier: 'opcode',
              offset: 3,
              size: 5,
              match: 0b00011,
            },
            {
              identifier: 'rm',
              offset: 0,
              size: 3,
              match: 0b010,
              type: InstructionOperandTypes.Memory,
              encoding: ['DE'],
            },
          ],
        },
      ],
      operands: ['A', 'rm'],
      operation: ['A = RAM:u8[DE]'],
      cycles: 7,
    },

    // --- Direct memory loads ---
    // LD (nn), HL / LD HL, (nn)
    {
      opcode: [Opcodes.LD_xNNx_HL, 'IMM_MEM_u16'],
      operands: ['mem', 'HL'],
      operandSize: 16,
      operation: ['RAM:u16[%{mem}] = HL'],
      cycles: 16,
    },
    {
      opcode: [Opcodes.LD_HL_xNNx, 'IMM_MEM_u16'],
      operands: ['HL', 'mem'],
      operandSize: 16,
      operation: ['HL = RAM:u16[%{mem}]'],
      cycles: 16,
    },
    // LD (nn), A / LD A, (nn)
    {
      opcode: [Opcodes.LD_xNNx_A, 'IMM_MEM_u16'],
      operands: ['mem', 'A'],
      operandSize: 16,
      operation: ['RAM:u8[%{mem}] = A'],
      cycles: 13,
    },
    {
      opcode: [Opcodes.LD_A_xNNx, 'IMM_MEM_u16'],
      operands: ['A', 'mem'],
      operandSize: 16,
      operation: ['A = RAM:u8[%{mem}]'],
      cycles: 13,
    },

    // --- LD SP, HL ---
    {
      opcode: [Opcodes.LD_SP_HL],
      operands: ['SP', 'HL'],
      operation: ['SP = HL'],
      cycles: 6,
    },

    // --- ED-prefixed 16-bit direct memory loads ---
    // LD (nn), rp / LD rp, (nn) — ED prefix
    {
      opcode: [Opcodes.ED_PREFIX, EDOpcodes.LD_xNNx_BC, 'IMM_MEM_u16'],
      operands: ['mem', 'BC'],
      operandSize: 16,
      operation: ['RAM:u16[%{mem}] = BC'],
      cycles: 20,
    },
    {
      opcode: [Opcodes.ED_PREFIX, EDOpcodes.LD_BC_xNNx, 'IMM_MEM_u16'],
      operands: ['BC', 'mem'],
      operandSize: 16,
      operation: ['BC = RAM:u16[%{mem}]'],
      cycles: 20,
    },
    {
      opcode: [Opcodes.ED_PREFIX, EDOpcodes.LD_xNNx_DE, 'IMM_MEM_u16'],
      operands: ['mem', 'DE'],
      operandSize: 16,
      operation: ['RAM:u16[%{mem}] = DE'],
      cycles: 20,
    },
    {
      opcode: [Opcodes.ED_PREFIX, EDOpcodes.LD_DE_xNNx, 'IMM_MEM_u16'],
      operands: ['DE', 'mem'],
      operandSize: 16,
      operation: ['DE = RAM:u16[%{mem}]'],
      cycles: 20,
    },
    {
      opcode: [Opcodes.ED_PREFIX, EDOpcodes.LD_xNNx_HL_ED, 'IMM_MEM_u16'],
      operands: ['mem', 'HL'],
      operandSize: 16,
      operation: ['RAM:u16[%{mem}] = HL'],
      cycles: 20,
    },
    {
      opcode: [Opcodes.ED_PREFIX, EDOpcodes.LD_HL_xNNx_ED, 'IMM_MEM_u16'],
      operands: ['HL', 'mem'],
      operandSize: 16,
      operation: ['HL = RAM:u16[%{mem}]'],
      cycles: 20,
    },
    {
      opcode: [Opcodes.ED_PREFIX, EDOpcodes.LD_xNNx_SP, 'IMM_MEM_u16'],
      operands: ['mem', 'SP'],
      operandSize: 16,
      operation: ['RAM:u16[%{mem}] = SP'],
      cycles: 20,
    },
    {
      opcode: [Opcodes.ED_PREFIX, EDOpcodes.LD_SP_xNNx, 'IMM_MEM_u16'],
      operands: ['SP', 'mem'],
      operandSize: 16,
      operation: ['SP = RAM:u16[%{mem}]'],
      cycles: 20,
    },

    // --- Special register loads (ED prefix) ---
    {
      opcode: [Opcodes.ED_PREFIX, EDOpcodes.LD_I_A],
      operands: ['I', 'A'],
      operation: ['I = A'],
      cycles: 9,
    },
    {
      opcode: [Opcodes.ED_PREFIX, EDOpcodes.LD_R_A],
      operands: ['R', 'A'],
      operation: ['R = A'],
      cycles: 9,
    },
    {
      opcode: [Opcodes.ED_PREFIX, EDOpcodes.LD_A_I],
      operands: ['A', 'I'],
      modifies: ['SF', 'ZF', 'HF', 'PF', 'NF'],
      operation: ['A = I'],
      cycles: 9,
    },
    {
      opcode: [Opcodes.ED_PREFIX, EDOpcodes.LD_A_R],
      operands: ['A', 'R'],
      modifies: ['SF', 'ZF', 'HF', 'PF', 'NF'],
      operation: ['A = R'],
      cycles: 9,
    },

    // --- IX/IY indexed forms (DD/FD prefix) ---

    // LD r, (IX+d) / LD r, (IY+d)
    {
      opcode: ['DD_IX', Opcodes.LD_B_xHLx, 'DISP_i8'],
      operands: ['B', 'rm'],
      operation: ['B = RAM:u8[IX + %{DISP}]'],
      cycles: 19,
    },
    {
      opcode: ['FD_IY', Opcodes.LD_B_xHLx, 'DISP_i8'],
      operands: ['B', 'rm'],
      operation: ['B = RAM:u8[IY + %{DISP}]'],
      cycles: 19,
    },
    {
      opcode: ['DD_IX', Opcodes.LD_C_xHLx, 'DISP_i8'],
      operands: ['C', 'rm'],
      operation: ['C = RAM:u8[IX + %{DISP}]'],
      cycles: 19,
    },
    {
      opcode: ['FD_IY', Opcodes.LD_C_xHLx, 'DISP_i8'],
      operands: ['C', 'rm'],
      operation: ['C = RAM:u8[IY + %{DISP}]'],
      cycles: 19,
    },
    {
      opcode: ['DD_IX', Opcodes.LD_D_xHLx, 'DISP_i8'],
      operands: ['D', 'rm'],
      operation: ['D = RAM:u8[IX + %{DISP}]'],
      cycles: 19,
    },
    {
      opcode: ['FD_IY', Opcodes.LD_D_xHLx, 'DISP_i8'],
      operands: ['D', 'rm'],
      operation: ['D = RAM:u8[IY + %{DISP}]'],
      cycles: 19,
    },
    {
      opcode: ['DD_IX', Opcodes.LD_E_xHLx, 'DISP_i8'],
      operands: ['E', 'rm'],
      operation: ['E = RAM:u8[IX + %{DISP}]'],
      cycles: 19,
    },
    {
      opcode: ['FD_IY', Opcodes.LD_E_xHLx, 'DISP_i8'],
      operands: ['E', 'rm'],
      operation: ['E = RAM:u8[IY + %{DISP}]'],
      cycles: 19,
    },
    {
      opcode: ['DD_IX', Opcodes.LD_H_xHLx, 'DISP_i8'],
      operands: ['H', 'rm'],
      operation: ['H = RAM:u8[IX + %{DISP}]'],
      cycles: 19,
    },
    {
      opcode: ['FD_IY', Opcodes.LD_H_xHLx, 'DISP_i8'],
      operands: ['H', 'rm'],
      operation: ['H = RAM:u8[IY + %{DISP}]'],
      cycles: 19,
    },
    {
      opcode: ['DD_IX', Opcodes.LD_L_xHLx, 'DISP_i8'],
      operands: ['L', 'rm'],
      operation: ['L = RAM:u8[IX + %{DISP}]'],
      cycles: 19,
    },
    {
      opcode: ['FD_IY', Opcodes.LD_L_xHLx, 'DISP_i8'],
      operands: ['L', 'rm'],
      operation: ['L = RAM:u8[IY + %{DISP}]'],
      cycles: 19,
    },
    {
      opcode: ['DD_IX', Opcodes.LD_A_xHLx, 'DISP_i8'],
      operands: ['A', 'rm'],
      operation: ['A = RAM:u8[IX + %{DISP}]'],
      cycles: 19,
    },
    {
      opcode: ['FD_IY', Opcodes.LD_A_xHLx, 'DISP_i8'],
      operands: ['A', 'rm'],
      operation: ['A = RAM:u8[IY + %{DISP}]'],
      cycles: 19,
    },

    // LD (IX+d), r / LD (IY+d), r
    {
      opcode: ['DD_IX', Opcodes.LD_xHLx_B, 'DISP_i8'],
      operands: ['rm', 'B'],
      operation: ['RAM:u8[IX + %{DISP}] = B'],
      cycles: 19,
    },
    {
      opcode: ['FD_IY', Opcodes.LD_xHLx_B, 'DISP_i8'],
      operands: ['rm', 'B'],
      operation: ['RAM:u8[IY + %{DISP}] = B'],
      cycles: 19,
    },
    {
      opcode: ['DD_IX', Opcodes.LD_xHLx_C, 'DISP_i8'],
      operands: ['rm', 'C'],
      operation: ['RAM:u8[IX + %{DISP}] = C'],
      cycles: 19,
    },
    {
      opcode: ['FD_IY', Opcodes.LD_xHLx_C, 'DISP_i8'],
      operands: ['rm', 'C'],
      operation: ['RAM:u8[IY + %{DISP}] = C'],
      cycles: 19,
    },
    {
      opcode: ['DD_IX', Opcodes.LD_xHLx_D, 'DISP_i8'],
      operands: ['rm', 'D'],
      operation: ['RAM:u8[IX + %{DISP}] = D'],
      cycles: 19,
    },
    {
      opcode: ['FD_IY', Opcodes.LD_xHLx_D, 'DISP_i8'],
      operands: ['rm', 'D'],
      operation: ['RAM:u8[IY + %{DISP}] = D'],
      cycles: 19,
    },
    {
      opcode: ['DD_IX', Opcodes.LD_xHLx_E, 'DISP_i8'],
      operands: ['rm', 'E'],
      operation: ['RAM:u8[IX + %{DISP}] = E'],
      cycles: 19,
    },
    {
      opcode: ['FD_IY', Opcodes.LD_xHLx_E, 'DISP_i8'],
      operands: ['rm', 'E'],
      operation: ['RAM:u8[IY + %{DISP}] = E'],
      cycles: 19,
    },
    {
      opcode: ['DD_IX', Opcodes.LD_xHLx_H, 'DISP_i8'],
      operands: ['rm', 'H'],
      operation: ['RAM:u8[IX + %{DISP}] = H'],
      cycles: 19,
    },
    {
      opcode: ['FD_IY', Opcodes.LD_xHLx_H, 'DISP_i8'],
      operands: ['rm', 'H'],
      operation: ['RAM:u8[IY + %{DISP}] = H'],
      cycles: 19,
    },
    {
      opcode: ['DD_IX', Opcodes.LD_xHLx_L, 'DISP_i8'],
      operands: ['rm', 'L'],
      operation: ['RAM:u8[IX + %{DISP}] = L'],
      cycles: 19,
    },
    {
      opcode: ['FD_IY', Opcodes.LD_xHLx_L, 'DISP_i8'],
      operands: ['rm', 'L'],
      operation: ['RAM:u8[IY + %{DISP}] = L'],
      cycles: 19,
    },
    {
      opcode: ['DD_IX', Opcodes.LD_xHLx_A, 'DISP_i8'],
      operands: ['rm', 'A'],
      operation: ['RAM:u8[IX + %{DISP}] = A'],
      cycles: 19,
    },
    {
      opcode: ['FD_IY', Opcodes.LD_xHLx_A, 'DISP_i8'],
      operands: ['rm', 'A'],
      operation: ['RAM:u8[IY + %{DISP}] = A'],
      cycles: 19,
    },

    // LD (IX+d), n / LD (IY+d), n
    {
      opcode: ['DD_IX', Opcodes.LD_xHLx_N, 'DISP_i8', 'IMM_u8'],
      operands: ['rm', 'imm'],
      operandSize: 8,
      operation: ['RAM:u8[IX + %{DISP}] = %{imm}'],
      cycles: 19,
    },
    {
      opcode: ['FD_IY', Opcodes.LD_xHLx_N, 'DISP_i8', 'IMM_u8'],
      operands: ['rm', 'imm'],
      operandSize: 8,
      operation: ['RAM:u8[IY + %{DISP}] = %{imm}'],
      cycles: 19,
    },

    // --- IX/IY 16-bit loads (Category A) ---

    // LD IX, nn / LD IY, nn
    {
      opcode: [Opcodes.DD_PREFIX, Opcodes.LD_HL_NN, 'IMM_u16'],
      operands: ['IX', 'imm'],
      operandSize: 16,
      operation: ['IX = %{imm}'],
      cycles: 14,
    },
    {
      opcode: [Opcodes.FD_PREFIX, Opcodes.LD_HL_NN, 'IMM_u16'],
      operands: ['IY', 'imm'],
      operandSize: 16,
      operation: ['IY = %{imm}'],
      cycles: 14,
    },

    // LD (nn), IX / LD (nn), IY
    {
      opcode: [Opcodes.DD_PREFIX, Opcodes.LD_xNNx_HL, 'IMM_MEM_u16'],
      operands: ['mem', 'IX'],
      operandSize: 16,
      operation: ['RAM:u16[%{mem}] = IX'],
      cycles: 20,
    },
    {
      opcode: [Opcodes.FD_PREFIX, Opcodes.LD_xNNx_HL, 'IMM_MEM_u16'],
      operands: ['mem', 'IY'],
      operandSize: 16,
      operation: ['RAM:u16[%{mem}] = IY'],
      cycles: 20,
    },

    // LD IX, (nn) / LD IY, (nn)
    {
      opcode: [Opcodes.DD_PREFIX, Opcodes.LD_HL_xNNx, 'IMM_MEM_u16'],
      operands: ['IX', 'mem'],
      operandSize: 16,
      operation: ['IX = RAM:u16[%{mem}]'],
      cycles: 20,
    },
    {
      opcode: [Opcodes.FD_PREFIX, Opcodes.LD_HL_xNNx, 'IMM_MEM_u16'],
      operands: ['IY', 'mem'],
      operandSize: 16,
      operation: ['IY = RAM:u16[%{mem}]'],
      cycles: 20,
    },

    // LD SP, IX / LD SP, IY
    {
      opcode: [Opcodes.DD_PREFIX, Opcodes.LD_SP_HL],
      operands: ['SP', 'IX'],
      operation: ['SP = IX'],
      cycles: 10,
    },
    {
      opcode: [Opcodes.FD_PREFIX, Opcodes.LD_SP_HL],
      operands: ['SP', 'IY'],
      operation: ['SP = IY'],
      cycles: 10,
    },
  ],
};
