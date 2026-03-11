import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const push: InstructionInfo = {
  identifier: 'push',
  name: 'Push to Stack',
  description:
    'Decrements SP by 2 and pushes the register pair onto the stack.',
  modifies: [],
  forms: [
    {
      opcode: [Opcodes.PUSH_BC],
      operands: ['BC'],
      operation: ['SP = SP - 2', 'RAM:u16[SP] = BC'],
      cycles: 11,
    },
    {
      opcode: [Opcodes.PUSH_DE],
      operands: ['DE'],
      operation: ['SP = SP - 2', 'RAM:u16[SP] = DE'],
      cycles: 11,
    },
    {
      opcode: [Opcodes.PUSH_HL],
      operands: ['HL'],
      operation: ['SP = SP - 2', 'RAM:u16[SP] = HL'],
      cycles: 11,
    },
    {
      opcode: [Opcodes.PUSH_AF],
      operands: ['AF'],
      operation: ['SP = SP - 2', 'RAM:u16[SP] = AF'],
      cycles: 11,
    },
  ],
};
