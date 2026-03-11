import type { InstructionInfo } from '@machinery/core';

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
      opcode: [Opcodes.LD_B_xHLx],
      operands: ['B', '(HL)'],
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
      opcode: [Opcodes.LD_C_xHLx],
      operands: ['C', '(HL)'],
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
      opcode: [Opcodes.LD_D_xHLx],
      operands: ['D', '(HL)'],
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
      opcode: [Opcodes.LD_E_xHLx],
      operands: ['E', '(HL)'],
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
      opcode: [Opcodes.LD_H_xHLx],
      operands: ['H', '(HL)'],
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
      opcode: [Opcodes.LD_L_xHLx],
      operands: ['L', '(HL)'],
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
      opcode: [Opcodes.LD_xHLx_B],
      operands: ['(HL)', 'B'],
      operation: ['RAM:u8[HL] = B'],
      cycles: 7,
    },
    {
      opcode: [Opcodes.LD_xHLx_C],
      operands: ['(HL)', 'C'],
      operation: ['RAM:u8[HL] = C'],
      cycles: 7,
    },
    {
      opcode: [Opcodes.LD_xHLx_D],
      operands: ['(HL)', 'D'],
      operation: ['RAM:u8[HL] = D'],
      cycles: 7,
    },
    {
      opcode: [Opcodes.LD_xHLx_E],
      operands: ['(HL)', 'E'],
      operation: ['RAM:u8[HL] = E'],
      cycles: 7,
    },
    {
      opcode: [Opcodes.LD_xHLx_H],
      operands: ['(HL)', 'H'],
      operation: ['RAM:u8[HL] = H'],
      cycles: 7,
    },
    {
      opcode: [Opcodes.LD_xHLx_L],
      operands: ['(HL)', 'L'],
      operation: ['RAM:u8[HL] = L'],
      cycles: 7,
    },
    {
      opcode: [Opcodes.LD_xHLx_A],
      operands: ['(HL)', 'A'],
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
      opcode: [Opcodes.LD_A_xHLx],
      operands: ['A', '(HL)'],
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
      operation: ['B = %{IMM}'],
      cycles: 7,
    },
    {
      opcode: [Opcodes.LD_C_N, 'IMM_u8'],
      operands: ['C', 'imm'],
      operandSize: 8,
      operation: ['C = %{IMM}'],
      cycles: 7,
    },
    {
      opcode: [Opcodes.LD_D_N, 'IMM_u8'],
      operands: ['D', 'imm'],
      operandSize: 8,
      operation: ['D = %{IMM}'],
      cycles: 7,
    },
    {
      opcode: [Opcodes.LD_E_N, 'IMM_u8'],
      operands: ['E', 'imm'],
      operandSize: 8,
      operation: ['E = %{IMM}'],
      cycles: 7,
    },
    {
      opcode: [Opcodes.LD_H_N, 'IMM_u8'],
      operands: ['H', 'imm'],
      operandSize: 8,
      operation: ['H = %{IMM}'],
      cycles: 7,
    },
    {
      opcode: [Opcodes.LD_L_N, 'IMM_u8'],
      operands: ['L', 'imm'],
      operandSize: 8,
      operation: ['L = %{IMM}'],
      cycles: 7,
    },
    {
      opcode: [Opcodes.LD_xHLx_N, 'IMM_u8'],
      operands: ['(HL)', 'imm'],
      operandSize: 8,
      operation: ['RAM:u8[HL] = %{IMM}'],
      cycles: 10,
    },
    {
      opcode: [Opcodes.LD_A_N, 'IMM_u8'],
      operands: ['A', 'imm'],
      operandSize: 8,
      operation: ['A = %{IMM}'],
      cycles: 7,
    },

    // --- 16-bit immediate loads: LD rp, nn ---
    {
      opcode: [Opcodes.LD_BC_NN, 'IMM_u16'],
      operands: ['BC', 'imm'],
      operandSize: 16,
      operation: ['BC = %{IMM}'],
      cycles: 10,
    },
    {
      opcode: [Opcodes.LD_DE_NN, 'IMM_u16'],
      operands: ['DE', 'imm'],
      operandSize: 16,
      operation: ['DE = %{IMM}'],
      cycles: 10,
    },
    {
      opcode: [Opcodes.LD_HL_NN, 'IMM_u16'],
      operands: ['HL', 'imm'],
      operandSize: 16,
      operation: ['HL = %{IMM}'],
      cycles: 10,
    },
    {
      opcode: [Opcodes.LD_SP_NN, 'IMM_u16'],
      operands: ['SP', 'imm'],
      operandSize: 16,
      operation: ['SP = %{IMM}'],
      cycles: 10,
    },

    // --- Indirect loads ---
    // LD (BC), A / LD (DE), A
    {
      opcode: [Opcodes.LD_xBCx_A],
      operands: ['(BC)', 'A'],
      operation: ['RAM:u8[BC] = A'],
      cycles: 7,
    },
    {
      opcode: [Opcodes.LD_xDEx_A],
      operands: ['(DE)', 'A'],
      operation: ['RAM:u8[DE] = A'],
      cycles: 7,
    },
    // LD A, (BC) / LD A, (DE)
    {
      opcode: [Opcodes.LD_A_xBCx],
      operands: ['A', '(BC)'],
      operation: ['A = RAM:u8[BC]'],
      cycles: 7,
    },
    {
      opcode: [Opcodes.LD_A_xDEx],
      operands: ['A', '(DE)'],
      operation: ['A = RAM:u8[DE]'],
      cycles: 7,
    },

    // --- Direct memory loads ---
    // LD (nn), HL / LD HL, (nn)
    {
      opcode: [Opcodes.LD_xNNx_HL, 'IMM_u16'],
      operands: ['(imm)', 'HL'],
      operandSize: 16,
      operation: ['RAM:u16[%{IMM}] = HL'],
      cycles: 16,
    },
    {
      opcode: [Opcodes.LD_HL_xNNx, 'IMM_u16'],
      operands: ['HL', '(imm)'],
      operandSize: 16,
      operation: ['HL = RAM:u16[%{IMM}]'],
      cycles: 16,
    },
    // LD (nn), A / LD A, (nn)
    {
      opcode: [Opcodes.LD_xNNx_A, 'IMM_u16'],
      operands: ['(imm)', 'A'],
      operandSize: 16,
      operation: ['RAM:u8[%{IMM}] = A'],
      cycles: 13,
    },
    {
      opcode: [Opcodes.LD_A_xNNx, 'IMM_u16'],
      operands: ['A', '(imm)'],
      operandSize: 16,
      operation: ['A = RAM:u8[%{IMM}]'],
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
      opcode: [Opcodes.ED_PREFIX, EDOpcodes.LD_xNNx_BC, 'IMM_u16'],
      operands: ['(imm)', 'BC'],
      operandSize: 16,
      operation: ['RAM:u16[%{IMM}] = BC'],
      cycles: 20,
    },
    {
      opcode: [Opcodes.ED_PREFIX, EDOpcodes.LD_BC_xNNx, 'IMM_u16'],
      operands: ['BC', '(imm)'],
      operandSize: 16,
      operation: ['BC = RAM:u16[%{IMM}]'],
      cycles: 20,
    },
    {
      opcode: [Opcodes.ED_PREFIX, EDOpcodes.LD_xNNx_DE, 'IMM_u16'],
      operands: ['(imm)', 'DE'],
      operandSize: 16,
      operation: ['RAM:u16[%{IMM}] = DE'],
      cycles: 20,
    },
    {
      opcode: [Opcodes.ED_PREFIX, EDOpcodes.LD_DE_xNNx, 'IMM_u16'],
      operands: ['DE', '(imm)'],
      operandSize: 16,
      operation: ['DE = RAM:u16[%{IMM}]'],
      cycles: 20,
    },
    {
      opcode: [Opcodes.ED_PREFIX, EDOpcodes.LD_xNNx_HL_ED, 'IMM_u16'],
      operands: ['(imm)', 'HL'],
      operandSize: 16,
      operation: ['RAM:u16[%{IMM}] = HL'],
      cycles: 20,
    },
    {
      opcode: [Opcodes.ED_PREFIX, EDOpcodes.LD_HL_xNNx_ED, 'IMM_u16'],
      operands: ['HL', '(imm)'],
      operandSize: 16,
      operation: ['HL = RAM:u16[%{IMM}]'],
      cycles: 20,
    },
    {
      opcode: [Opcodes.ED_PREFIX, EDOpcodes.LD_xNNx_SP, 'IMM_u16'],
      operands: ['(imm)', 'SP'],
      operandSize: 16,
      operation: ['RAM:u16[%{IMM}] = SP'],
      cycles: 20,
    },
    {
      opcode: [Opcodes.ED_PREFIX, EDOpcodes.LD_SP_xNNx, 'IMM_u16'],
      operands: ['SP', '(imm)'],
      operandSize: 16,
      operation: ['SP = RAM:u16[%{IMM}]'],
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
      modifies: ['S', 'Z', 'H', 'P', 'N'],
      operation: ['A = I'],
      cycles: 9,
    },
    {
      opcode: [Opcodes.ED_PREFIX, EDOpcodes.LD_A_R],
      operands: ['A', 'R'],
      modifies: ['S', 'Z', 'H', 'P', 'N'],
      operation: ['A = R'],
      cycles: 9,
    },
  ],
};
