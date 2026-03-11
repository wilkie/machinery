import type { InstructionInfo } from '@machinery/core';

import { Opcodes, EDOpcodes } from '../opcodes';

export const ldi: InstructionInfo = {
  identifier: 'ldi',
  name: 'Load and Increment',
  description:
    'Copies (HL) to (DE), then increments HL and DE, and decrements BC.',
  modifies: ['H', 'P', 'N'],
  forms: [
    {
      opcode: [Opcodes.ED_PREFIX, EDOpcodes.LDI],
      operands: [],
      operation: [
        'RAM:u8[DE] = RAM:u8[HL]',
        'HL = HL + 1',
        'DE = DE + 1',
        'BC = BC - 1',
      ],
      cycles: 16,
    },
  ],
};

export const ldd: InstructionInfo = {
  identifier: 'ldd',
  name: 'Load and Decrement',
  description:
    'Copies (HL) to (DE), then decrements HL and DE, and decrements BC.',
  modifies: ['H', 'P', 'N'],
  forms: [
    {
      opcode: [Opcodes.ED_PREFIX, EDOpcodes.LDD],
      operands: [],
      operation: [
        'RAM:u8[DE] = RAM:u8[HL]',
        'HL = HL - 1',
        'DE = DE - 1',
        'BC = BC - 1',
      ],
      cycles: 16,
    },
  ],
};

export const ldir: InstructionInfo = {
  identifier: 'ldir',
  name: 'Load, Increment, and Repeat',
  description: 'Repeats LDI until BC reaches 0.',
  modifies: ['H', 'P', 'N'],
  forms: [
    {
      opcode: [Opcodes.ED_PREFIX, EDOpcodes.LDIR],
      operands: [],
      operation: [
        'RAM:u8[DE] = RAM:u8[HL]',
        'HL = HL + 1',
        'DE = DE + 1',
        'BC = BC - 1',
        'if (BC != 0) PC = PC - 2',
      ],
      cycles: 21,
    },
  ],
};

export const lddr: InstructionInfo = {
  identifier: 'lddr',
  name: 'Load, Decrement, and Repeat',
  description: 'Repeats LDD until BC reaches 0.',
  modifies: ['H', 'P', 'N'],
  forms: [
    {
      opcode: [Opcodes.ED_PREFIX, EDOpcodes.LDDR],
      operands: [],
      operation: [
        'RAM:u8[DE] = RAM:u8[HL]',
        'HL = HL - 1',
        'DE = DE - 1',
        'BC = BC - 1',
        'if (BC != 0) PC = PC - 2',
      ],
      cycles: 21,
    },
  ],
};

export const cpi: InstructionInfo = {
  identifier: 'cpi',
  name: 'Compare and Increment',
  description: 'Compares A with (HL), increments HL, and decrements BC.',
  modifies: ['S', 'Z', 'H', 'P', 'N'],
  forms: [
    {
      opcode: [Opcodes.ED_PREFIX, EDOpcodes.CPI],
      operands: [],
      operation: ['tmp = A - RAM:u8[HL]', 'HL = HL + 1', 'BC = BC - 1'],
      cycles: 16,
    },
  ],
};

export const cpd: InstructionInfo = {
  identifier: 'cpd',
  name: 'Compare and Decrement',
  description: 'Compares A with (HL), decrements HL, and decrements BC.',
  modifies: ['S', 'Z', 'H', 'P', 'N'],
  forms: [
    {
      opcode: [Opcodes.ED_PREFIX, EDOpcodes.CPD],
      operands: [],
      operation: ['tmp = A - RAM:u8[HL]', 'HL = HL - 1', 'BC = BC - 1'],
      cycles: 16,
    },
  ],
};

export const cpir: InstructionInfo = {
  identifier: 'cpir',
  name: 'Compare, Increment, and Repeat',
  description: 'Repeats CPI until BC=0 or a match is found.',
  modifies: ['S', 'Z', 'H', 'P', 'N'],
  forms: [
    {
      opcode: [Opcodes.ED_PREFIX, EDOpcodes.CPIR],
      operands: [],
      operation: [
        'tmp = A - RAM:u8[HL]',
        'HL = HL + 1',
        'BC = BC - 1',
        'if (BC != 0 && tmp != 0) PC = PC - 2',
      ],
      cycles: 21,
    },
  ],
};

export const cpdr: InstructionInfo = {
  identifier: 'cpdr',
  name: 'Compare, Decrement, and Repeat',
  description: 'Repeats CPD until BC=0 or a match is found.',
  modifies: ['S', 'Z', 'H', 'P', 'N'],
  forms: [
    {
      opcode: [Opcodes.ED_PREFIX, EDOpcodes.CPDR],
      operands: [],
      operation: [
        'tmp = A - RAM:u8[HL]',
        'HL = HL - 1',
        'BC = BC - 1',
        'if (BC != 0 && tmp != 0) PC = PC - 2',
      ],
      cycles: 21,
    },
  ],
};

export const ini: InstructionInfo = {
  identifier: 'ini',
  name: 'Input and Increment',
  description:
    'Reads a byte from port (C) into (HL), increments HL, and decrements B.',
  modifies: ['Z', 'N'],
  forms: [
    {
      opcode: [Opcodes.ED_PREFIX, EDOpcodes.INI],
      operands: [],
      operation: ['RAM:u8[HL] = IO:u8[C]', 'HL = HL + 1', 'B = B - 1'],
      cycles: 16,
    },
  ],
};

export const ind: InstructionInfo = {
  identifier: 'ind',
  name: 'Input and Decrement',
  description:
    'Reads a byte from port (C) into (HL), decrements HL, and decrements B.',
  modifies: ['Z', 'N'],
  forms: [
    {
      opcode: [Opcodes.ED_PREFIX, EDOpcodes.IND],
      operands: [],
      operation: ['RAM:u8[HL] = IO:u8[C]', 'HL = HL - 1', 'B = B - 1'],
      cycles: 16,
    },
  ],
};

export const inir: InstructionInfo = {
  identifier: 'inir',
  name: 'Input, Increment, and Repeat',
  description: 'Repeats INI until B reaches 0.',
  modifies: ['Z', 'N'],
  forms: [
    {
      opcode: [Opcodes.ED_PREFIX, EDOpcodes.INIR],
      operands: [],
      operation: [
        'RAM:u8[HL] = IO:u8[C]',
        'HL = HL + 1',
        'B = B - 1',
        'if (B != 0) PC = PC - 2',
      ],
      cycles: 21,
    },
  ],
};

export const indr: InstructionInfo = {
  identifier: 'indr',
  name: 'Input, Decrement, and Repeat',
  description: 'Repeats IND until B reaches 0.',
  modifies: ['Z', 'N'],
  forms: [
    {
      opcode: [Opcodes.ED_PREFIX, EDOpcodes.INDR],
      operands: [],
      operation: [
        'RAM:u8[HL] = IO:u8[C]',
        'HL = HL - 1',
        'B = B - 1',
        'if (B != 0) PC = PC - 2',
      ],
      cycles: 21,
    },
  ],
};

export const outi: InstructionInfo = {
  identifier: 'outi',
  name: 'Output and Increment',
  description: 'Outputs (HL) to port (C), increments HL, and decrements B.',
  modifies: ['Z', 'N'],
  forms: [
    {
      opcode: [Opcodes.ED_PREFIX, EDOpcodes.OUTI],
      operands: [],
      operation: ['IO:u8[C] = RAM:u8[HL]', 'HL = HL + 1', 'B = B - 1'],
      cycles: 16,
    },
  ],
};

export const outd: InstructionInfo = {
  identifier: 'outd',
  name: 'Output and Decrement',
  description: 'Outputs (HL) to port (C), decrements HL, and decrements B.',
  modifies: ['Z', 'N'],
  forms: [
    {
      opcode: [Opcodes.ED_PREFIX, EDOpcodes.OUTD],
      operands: [],
      operation: ['IO:u8[C] = RAM:u8[HL]', 'HL = HL - 1', 'B = B - 1'],
      cycles: 16,
    },
  ],
};

export const otir: InstructionInfo = {
  identifier: 'otir',
  name: 'Output, Increment, and Repeat',
  description: 'Repeats OUTI until B reaches 0.',
  modifies: ['Z', 'N'],
  forms: [
    {
      opcode: [Opcodes.ED_PREFIX, EDOpcodes.OTIR],
      operands: [],
      operation: [
        'IO:u8[C] = RAM:u8[HL]',
        'HL = HL + 1',
        'B = B - 1',
        'if (B != 0) PC = PC - 2',
      ],
      cycles: 21,
    },
  ],
};

export const otdr: InstructionInfo = {
  identifier: 'otdr',
  name: 'Output, Decrement, and Repeat',
  description: 'Repeats OUTD until B reaches 0.',
  modifies: ['Z', 'N'],
  forms: [
    {
      opcode: [Opcodes.ED_PREFIX, EDOpcodes.OTDR],
      operands: [],
      operation: [
        'IO:u8[C] = RAM:u8[HL]',
        'HL = HL - 1',
        'B = B - 1',
        'if (B != 0) PC = PC - 2',
      ],
      cycles: 21,
    },
  ],
};
