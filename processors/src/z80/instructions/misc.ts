import type { InstructionInfo } from '@machinery/core';

import { Opcodes, EDOpcodes } from '../opcodes';

export const daa: InstructionInfo = {
  identifier: 'daa',
  name: 'Decimal Adjust Accumulator',
  description:
    'Adjusts A so that a correct BCD result is obtained after an addition or subtraction.',
  modifies: ['SF', 'ZF', 'HF', 'PF', 'CF'],
  forms: [{ opcode: [Opcodes.DAA], operands: [], operation: [], cycles: 4 }],
};

export const cpl: InstructionInfo = {
  identifier: 'cpl',
  name: 'Complement Accumulator',
  description: "Inverts all bits of A (one's complement).",
  modifies: ['HF', 'NF'],
  forms: [
    {
      opcode: [Opcodes.CPL],
      operands: [],
      operation: [
        'A = ~A',
        'HF = 1',
        'NF = 1',
        'flag_op = ${FLAG_OP_RESOLVED}',
      ],
      cycles: 4,
    },
  ],
};

export const neg: InstructionInfo = {
  identifier: 'neg',
  name: 'Negate Accumulator',
  description: "Negates A (two's complement).",
  modifies: ['SF', 'ZF', 'HF', 'PF', 'NF', 'CF'],
  forms: [
    {
      opcode: [Opcodes.ED_PREFIX, EDOpcodes.NEG],
      operands: [],
      operation: [
        'a = 0',
        'b = A',
        'alu_result = a - b',
        'flag_op = ${FLAG_OP_ALU_SUB8}',
        'A = alu_result',
      ],
      cycles: 8,
    },
  ],
};

export const scf: InstructionInfo = {
  identifier: 'scf',
  name: 'Set Carry Flag',
  description: 'Sets the carry flag to 1.',
  modifies: ['HF', 'NF', 'CF'],
  forms: [
    {
      opcode: [Opcodes.SCF],
      operands: [],
      operation: [
        'CF = 1',
        'HF = 0',
        'NF = 0',
        'flag_op = ${FLAG_OP_RESOLVED}',
      ],
      cycles: 4,
    },
  ],
};

export const ccf: InstructionInfo = {
  identifier: 'ccf',
  name: 'Complement Carry Flag',
  description: 'Inverts the carry flag.',
  modifies: ['HF', 'NF', 'CF'],
  forms: [
    {
      opcode: [Opcodes.CCF],
      operands: [],
      operation: [
        '${RESOLVE_CF}',
        'HF = CF',
        'CF = ~CF',
        'NF = 0',
        'flag_op = ${FLAG_OP_RESOLVED}',
      ],
      cycles: 4,
    },
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
