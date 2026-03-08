import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const cld: InstructionInfo = {
  identifier: 'cld',
  name: 'Clear Direction Flag',
  description:
    '`CLD` clears the direction flag. No other flags or registers are affected. After `CLD` is executed, string operations will increment the index registers (`SI` and/or `DI`) that they use.',
  modifies: ['DF'],
  undefined: [],
  forms: [
    // 0xFC - CLD
    {
      opcode: [Opcodes.CLD],
      operation: ['DF = 0'],
      cycles: 2,
    },
  ],
};
