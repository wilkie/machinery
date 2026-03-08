import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const repne: InstructionInfo = {
  identifier: 'repne',
  prefix: true,
  name: 'Repeat String Operation While Not Equal Prefix',
  description: '',
  modifies: [],
  undefined: [],
  forms: [
    {
      // 0xF2 REPNE
      opcode: [Opcodes.REPNE_PREFIX],
      operation: ['REP = 1', 'REP_CHECK = 1'],
      finalize: ['REP = 0', 'REP_CHECK = 0'],
    },
  ],
};
