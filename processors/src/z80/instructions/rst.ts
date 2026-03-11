import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const rst: InstructionInfo = {
  identifier: 'rst',
  name: 'Restart',
  description:
    'Pushes PC onto the stack and jumps to one of 8 fixed addresses (0x00, 0x08, ..., 0x38).',
  modifies: [],
  forms: [
    {
      opcode: [Opcodes.RST_00],
      operands: ['0x00'],
      operation: ['SP = SP - 2', 'RAM:u16[SP] = PC', 'PC = 0x00'],
      cycles: 11,
    },
    {
      opcode: [Opcodes.RST_08],
      operands: ['0x08'],
      operation: ['SP = SP - 2', 'RAM:u16[SP] = PC', 'PC = 0x08'],
      cycles: 11,
    },
    {
      opcode: [Opcodes.RST_10],
      operands: ['0x10'],
      operation: ['SP = SP - 2', 'RAM:u16[SP] = PC', 'PC = 0x10'],
      cycles: 11,
    },
    {
      opcode: [Opcodes.RST_18],
      operands: ['0x18'],
      operation: ['SP = SP - 2', 'RAM:u16[SP] = PC', 'PC = 0x18'],
      cycles: 11,
    },
    {
      opcode: [Opcodes.RST_20],
      operands: ['0x20'],
      operation: ['SP = SP - 2', 'RAM:u16[SP] = PC', 'PC = 0x20'],
      cycles: 11,
    },
    {
      opcode: [Opcodes.RST_28],
      operands: ['0x28'],
      operation: ['SP = SP - 2', 'RAM:u16[SP] = PC', 'PC = 0x28'],
      cycles: 11,
    },
    {
      opcode: [Opcodes.RST_30],
      operands: ['0x30'],
      operation: ['SP = SP - 2', 'RAM:u16[SP] = PC', 'PC = 0x30'],
      cycles: 11,
    },
    {
      opcode: [Opcodes.RST_38],
      operands: ['0x38'],
      operation: ['SP = SP - 2', 'RAM:u16[SP] = PC', 'PC = 0x38'],
      cycles: 11,
    },
  ],
};
