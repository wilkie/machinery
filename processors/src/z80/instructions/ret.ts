import type { InstructionInfo } from '@machinery/core';

import { Opcodes, EDOpcodes } from '../opcodes';

export const ret: InstructionInfo = {
  identifier: 'ret',
  name: 'Return from Subroutine',
  description: 'Pops the return address from the stack and jumps to it.',
  modifies: [],
  macros: {
    RET_OP: ['PC = RAM:u16[SP]', 'SP = SP + 2'],
  },
  forms: [
    // RET — unconditional
    {
      opcode: [Opcodes.RET],
      operands: [],
      operation: ['${RET_OP}'],
      cycles: 10,
    },
    // RET cc — conditional
    {
      opcode: [Opcodes.RET_NZ],
      operands: [],
      aliases: ['ret nz'],
      operation: ['if (ZF == 0) { ${RET_OP} }'],
      cycles: 11,
    },
    {
      opcode: [Opcodes.RET_Z],
      operands: [],
      aliases: ['ret z'],
      operation: ['if (ZF == 1) { ${RET_OP} }'],
      cycles: 11,
    },
    {
      opcode: [Opcodes.RET_NC],
      operands: [],
      aliases: ['ret nc'],
      operation: ['if (CF == 0) { ${RET_OP} }'],
      cycles: 11,
    },
    {
      opcode: [Opcodes.RET_C],
      operands: [],
      aliases: ['ret c'],
      operation: ['if (CF == 1) { ${RET_OP} }'],
      cycles: 11,
    },
    {
      opcode: [Opcodes.RET_PO],
      operands: [],
      aliases: ['ret po'],
      operation: ['if (PF == 0) { ${RET_OP} }'],
      cycles: 11,
    },
    {
      opcode: [Opcodes.RET_PE],
      operands: [],
      aliases: ['ret pe'],
      operation: ['if (PF == 1) { ${RET_OP} }'],
      cycles: 11,
    },
    {
      opcode: [Opcodes.RET_P],
      operands: [],
      aliases: ['ret p'],
      operation: ['if (SF == 0) { ${RET_OP} }'],
      cycles: 11,
    },
    {
      opcode: [Opcodes.RET_M],
      operands: [],
      aliases: ['ret m'],
      operation: ['if (SF == 1) { ${RET_OP} }'],
      cycles: 11,
    },
  ],
};

export const reti: InstructionInfo = {
  identifier: 'reti',
  name: 'Return from Interrupt',
  description:
    'Pops the return address from the stack. Used to signal the I/O device that the interrupt is complete.',
  modifies: [],
  forms: [
    {
      opcode: [Opcodes.ED_PREFIX, EDOpcodes.RETI],
      operands: [],
      operation: ['PC = RAM:u16[SP]', 'SP = SP + 2'],
      cycles: 14,
    },
  ],
};

export const retn: InstructionInfo = {
  identifier: 'retn',
  name: 'Return from Non-Maskable Interrupt',
  description:
    'Pops the return address from the stack and restores IFF1 from IFF2.',
  modifies: [],
  forms: [
    {
      opcode: [Opcodes.ED_PREFIX, EDOpcodes.RETN],
      operands: [],
      operation: ['IFF1 = IFF2', 'PC = RAM:u16[SP]', 'SP = SP + 2'],
      cycles: 14,
    },
  ],
};
