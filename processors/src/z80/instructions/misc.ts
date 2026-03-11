import type { InstructionInfo } from '@machinery/core';

import { Opcodes, EDOpcodes } from '../opcodes';

export const daa: InstructionInfo = {
  identifier: 'daa',
  name: 'Decimal Adjust Accumulator',
  description:
    'Adjusts A so that a correct BCD result is obtained after an addition or subtraction.',
  modifies: ['S', 'Z', 'H', 'P', 'C'],
  forms: [{ opcode: [Opcodes.DAA], operands: [], operation: [], cycles: 4 }],
};

export const cpl: InstructionInfo = {
  identifier: 'cpl',
  name: 'Complement Accumulator',
  description: "Inverts all bits of A (one's complement).",
  modifies: ['H', 'N'],
  forms: [
    { opcode: [Opcodes.CPL], operands: [], operation: ['A = ~A'], cycles: 4 },
  ],
};

export const neg: InstructionInfo = {
  identifier: 'neg',
  name: 'Negate Accumulator',
  description: "Negates A (two's complement).",
  modifies: ['S', 'Z', 'H', 'P', 'N', 'C'],
  forms: [
    {
      opcode: [Opcodes.ED_PREFIX, EDOpcodes.NEG],
      operands: [],
      operation: ['A = 0 - A'],
      cycles: 8,
    },
  ],
};

export const scf: InstructionInfo = {
  identifier: 'scf',
  name: 'Set Carry Flag',
  description: 'Sets the carry flag to 1.',
  modifies: ['H', 'N', 'C'],
  forms: [
    { opcode: [Opcodes.SCF], operands: [], operation: ['CF = 1'], cycles: 4 },
  ],
};

export const ccf: InstructionInfo = {
  identifier: 'ccf',
  name: 'Complement Carry Flag',
  description: 'Inverts the carry flag.',
  modifies: ['H', 'N', 'C'],
  forms: [
    { opcode: [Opcodes.CCF], operands: [], operation: ['CF = ~CF'], cycles: 4 },
  ],
};

export const im: InstructionInfo = {
  identifier: 'im',
  name: 'Set Interrupt Mode',
  description: 'Sets the interrupt mode (0, 1, or 2).',
  modifies: [],
  forms: [
    {
      opcode: [Opcodes.ED_PREFIX, EDOpcodes.IM_0],
      operands: ['0'],
      operation: [],
      cycles: 8,
    },
    {
      opcode: [Opcodes.ED_PREFIX, EDOpcodes.IM_1],
      operands: ['1'],
      operation: [],
      cycles: 8,
    },
    {
      opcode: [Opcodes.ED_PREFIX, EDOpcodes.IM_2],
      operands: ['2'],
      operation: [],
      cycles: 8,
    },
  ],
};
