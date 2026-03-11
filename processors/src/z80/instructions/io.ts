import type { InstructionInfo } from '@machinery/core';

import { Opcodes, EDOpcodes } from '../opcodes';

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
      opcode: [Opcodes.ED_PREFIX, EDOpcodes.IN_B_xCx],
      operands: ['B', '(C)'],
      modifies: ['S', 'Z', 'H', 'P', 'N'],
      operation: ['B = IO:u8[C]'],
      cycles: 12,
    },
    {
      opcode: [Opcodes.ED_PREFIX, EDOpcodes.IN_C_xCx],
      operands: ['C', '(C)'],
      modifies: ['S', 'Z', 'H', 'P', 'N'],
      operation: ['C = IO:u8[C]'],
      cycles: 12,
    },
    {
      opcode: [Opcodes.ED_PREFIX, EDOpcodes.IN_D_xCx],
      operands: ['D', '(C)'],
      modifies: ['S', 'Z', 'H', 'P', 'N'],
      operation: ['D = IO:u8[C]'],
      cycles: 12,
    },
    {
      opcode: [Opcodes.ED_PREFIX, EDOpcodes.IN_E_xCx],
      operands: ['E', '(C)'],
      modifies: ['S', 'Z', 'H', 'P', 'N'],
      operation: ['E = IO:u8[C]'],
      cycles: 12,
    },
    {
      opcode: [Opcodes.ED_PREFIX, EDOpcodes.IN_H_xCx],
      operands: ['H', '(C)'],
      modifies: ['S', 'Z', 'H', 'P', 'N'],
      operation: ['H = IO:u8[C]'],
      cycles: 12,
    },
    {
      opcode: [Opcodes.ED_PREFIX, EDOpcodes.IN_L_xCx],
      operands: ['L', '(C)'],
      modifies: ['S', 'Z', 'H', 'P', 'N'],
      operation: ['L = IO:u8[C]'],
      cycles: 12,
    },
    {
      opcode: [Opcodes.ED_PREFIX, EDOpcodes.IN_A_xCx],
      operands: ['A', '(C)'],
      modifies: ['S', 'Z', 'H', 'P', 'N'],
      operation: ['A = IO:u8[C]'],
      cycles: 12,
    },
    // IN (C) — affects flags only, result discarded
    {
      opcode: [Opcodes.ED_PREFIX, EDOpcodes.IN_xCx],
      operands: ['(C)'],
      modifies: ['S', 'Z', 'H', 'P', 'N'],
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
      opcode: [Opcodes.ED_PREFIX, EDOpcodes.OUT_xCx_B],
      operands: ['(C)', 'B'],
      operation: ['IO:u8[C] = B'],
      cycles: 12,
    },
    {
      opcode: [Opcodes.ED_PREFIX, EDOpcodes.OUT_xCx_C],
      operands: ['(C)', 'C'],
      operation: ['IO:u8[C] = C'],
      cycles: 12,
    },
    {
      opcode: [Opcodes.ED_PREFIX, EDOpcodes.OUT_xCx_D],
      operands: ['(C)', 'D'],
      operation: ['IO:u8[C] = D'],
      cycles: 12,
    },
    {
      opcode: [Opcodes.ED_PREFIX, EDOpcodes.OUT_xCx_E],
      operands: ['(C)', 'E'],
      operation: ['IO:u8[C] = E'],
      cycles: 12,
    },
    {
      opcode: [Opcodes.ED_PREFIX, EDOpcodes.OUT_xCx_H],
      operands: ['(C)', 'H'],
      operation: ['IO:u8[C] = H'],
      cycles: 12,
    },
    {
      opcode: [Opcodes.ED_PREFIX, EDOpcodes.OUT_xCx_L],
      operands: ['(C)', 'L'],
      operation: ['IO:u8[C] = L'],
      cycles: 12,
    },
    {
      opcode: [Opcodes.ED_PREFIX, EDOpcodes.OUT_xCx_A],
      operands: ['(C)', 'A'],
      operation: ['IO:u8[C] = A'],
      cycles: 12,
    },
    // OUT (C), 0
    {
      opcode: [Opcodes.ED_PREFIX, EDOpcodes.OUT_xCx_0],
      operands: ['(C)', '0'],
      operation: ['IO:u8[C] = 0'],
      cycles: 12,
    },
  ],
};
