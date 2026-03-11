import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const pop: InstructionInfo = {
  identifier: 'pop',
  name: 'Pop from Stack',
  description:
    'Pops a 16-bit value from the stack into the register pair and increments SP by 2.',
  modifies: [],
  forms: [
    {
      opcode: [Opcodes.POP_BC],
      operands: ['BC'],
      operation: ['BC = RAM:u16[SP]', 'SP = SP + 2'],
      cycles: 10,
    },
    {
      opcode: [Opcodes.POP_DE],
      operands: ['DE'],
      operation: ['DE = RAM:u16[SP]', 'SP = SP + 2'],
      cycles: 10,
    },
    {
      opcode: [Opcodes.POP_HL],
      operands: ['HL'],
      operation: ['HL = RAM:u16[SP]', 'SP = SP + 2'],
      cycles: 10,
    },
    {
      opcode: [Opcodes.POP_AF],
      operands: ['AF'],
      operation: ['AF = RAM:u16[SP]', 'SP = SP + 2'],
      cycles: 10,
    },
  ],
};
