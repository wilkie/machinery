import type { InstructionInfo } from '@machinery/core';

import { Opcodes, EDOpcodes } from '../opcodes';

export const rlca: InstructionInfo = {
  identifier: 'rlca',
  name: 'Rotate Left Circular Accumulator',
  description: 'Rotates A left by one bit. Bit 7 goes to carry and bit 0.',
  modifies: ['HF', 'NF', 'CF'],
  forms: [
    {
      opcode: [Opcodes.RLCA],
      operands: [],
      operation: [
        'CF = A >> 7',
        'A = (A << 1) | CF',
        'HF = 0',
        'NF = 0',
        'flag_op = ${FLAG_OP_RESOLVED}',
      ],
      cycles: 4,
    },
  ],
};

export const rrca: InstructionInfo = {
  identifier: 'rrca',
  name: 'Rotate Right Circular Accumulator',
  description: 'Rotates A right by one bit. Bit 0 goes to carry and bit 7.',
  modifies: ['HF', 'NF', 'CF'],
  forms: [
    {
      opcode: [Opcodes.RRCA],
      operands: [],
      operation: [
        'CF = A & 1',
        'A = (A >> 1) | (CF << 7)',
        'HF = 0',
        'NF = 0',
        'flag_op = ${FLAG_OP_RESOLVED}',
      ],
      cycles: 4,
    },
  ],
};

export const rla: InstructionInfo = {
  identifier: 'rla',
  name: 'Rotate Left Accumulator through Carry',
  description:
    'Rotates A left through carry. Old carry goes to bit 0, bit 7 goes to carry.',
  modifies: ['HF', 'NF', 'CF'],
  forms: [
    {
      opcode: [Opcodes.RLA],
      operands: [],
      operation: [
        '${RESOLVE_CF}',
        'tmp = CF',
        'CF = A >> 7',
        'A = (A << 1) | tmp',
        'HF = 0',
        'NF = 0',
        'flag_op = ${FLAG_OP_RESOLVED}',
      ],
      cycles: 4,
    },
  ],
};

export const rra: InstructionInfo = {
  identifier: 'rra',
  name: 'Rotate Right Accumulator through Carry',
  description:
    'Rotates A right through carry. Old carry goes to bit 7, bit 0 goes to carry.',
  modifies: ['HF', 'NF', 'CF'],
  forms: [
    {
      opcode: [Opcodes.RRA],
      operands: [],
      operation: [
        '${RESOLVE_CF}',
        'tmp = CF',
        'CF = A & 1',
        'A = (A >> 1) | (tmp << 7)',
        'HF = 0',
        'NF = 0',
        'flag_op = ${FLAG_OP_RESOLVED}',
      ],
      cycles: 4,
    },
  ],
};

export const rrd: InstructionInfo = {
  identifier: 'rrd',
  name: 'Rotate Right Decimal',
  description:
    'Rotates the low nibble of (HL) into the low nibble of A, the low nibble of A into the high nibble of (HL), and the high nibble of (HL) into the low nibble of (HL).',
  modifies: ['SF', 'ZF', 'HF', 'PF', 'NF'],
  forms: [
    {
      opcode: [Opcodes.ED_PREFIX, EDOpcodes.RRD],
      operands: [],
      operation: [],
      cycles: 18,
    },
  ],
};

export const rld: InstructionInfo = {
  identifier: 'rld',
  name: 'Rotate Left Decimal',
  description:
    'Rotates the high nibble of (HL) into the low nibble of A, the low nibble of A into the low nibble of (HL), and the low nibble of (HL) into the high nibble of (HL).',
  modifies: ['SF', 'ZF', 'HF', 'PF', 'NF'],
  forms: [
    {
      opcode: [Opcodes.ED_PREFIX, EDOpcodes.RLD],
      operands: [],
      operation: [],
      cycles: 18,
    },
  ],
};
