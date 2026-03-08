import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const cwd: InstructionInfo = {
  identifier: 'cwd',
  name: 'Convert Word to Doubleword',
  description:
    '`CWD` converts the signed word in `AX` to a signed doubleword in `DX:AX`. It does so by extending the top bit of `AX` into all the bits of `DX`.',
  modifies: [],
  undefined: [],
  forms: [
    // 0x99 - CWD
    {
      opcode: [Opcodes.CWD],
      operation: ['DX = (AX & 0x8000) > 0 ? 0xffff : 0x0'],
      cycles: 2,
    },
  ],
};
