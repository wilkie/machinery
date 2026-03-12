import type { InstructionInfo } from '@machinery/core';

import { Opcodes, EDOpcodes } from '../opcodes';

export const daa: InstructionInfo = {
  identifier: 'daa',
  name: 'Decimal Adjust Accumulator',
  description:
    'Adjusts A so that a correct BCD result is obtained after an addition or subtraction.',
  modifies: ['SF', 'ZF', 'HF', 'PF', 'CF'],
  forms: [
    {
      opcode: [Opcodes.DAA],
      operands: [],
      operation: [
        '${RESOLVE_CF}',
        // Resolve HF inline: (a ^ b ^ (alu_result & 0xFF)) >> 4 & 1
        // Must mask alu_result to 8 bits since it may be stored as 32-bit negative
        'HF = flag_op < ${FLAG_OP_RESOLVED} ? ((a ^ b ^ (alu_result & 0xFF)) >> 4) & 0x1 : HF',
        '${RESOLVE_NF}',
        'tmp = A',
        // After addition (NF=0):
        //   low nibble > 9 or HF: add 0x06
        //   A > 0x99 or CF: add 0x60, CF=1
        // After subtraction (NF=1):
        //   HF: subtract 0x06
        //   CF: subtract 0x60
        'a = (NF == 0) ? (((A & 0x0F) > 0x09 || HF == 1) ? 0x06 : 0x00) : ((HF == 1) ? 0x06 : 0x00)',
        'b = (NF == 0) ? ((A > 0x99 || CF == 1) ? 0x60 : 0x00) : ((CF == 1) ? 0x60 : 0x00)',
        'A = (NF == 0) ? A + a + b : A - a - b',
        'CF = (CF == 1 || ((NF == 0) && (tmp > 0x99))) ? 1 : 0',
        'HF = (NF == 0) ? (((tmp & 0x0F) > 0x09) ? 1 : 0) : ((((tmp ^ A) & 0x10) > 0) ? 1 : 0)',
        'SF = (A & 0x80) > 0 ? 1 : 0',
        'ZF = (A & 0xFF) == 0 ? 1 : 0',
        'PF = ROM.PARITY[A & 0xFF]',
        'flag_op = ${FLAG_OP_RESOLVED}',
      ],
      cycles: 4,
    },
  ],
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
