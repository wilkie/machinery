import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const cbw: InstructionInfo = {
  identifier: 'cbw',
  name: 'Convert Byte to Word',
  description:
    '`CBW` converts the signed byte in `AL` to a signed word in `AX`. It does so by extending the top bit of `AL` into all of the bits of `AH`.',
  modifies: [],
  undefined: [],
  forms: [
    // 0x98 - CBW
    {
      opcode: [Opcodes.CBW],
      operands: [],
      operation: ['AH = (AL & 0x80) > 0 ? 0xff : 0x00'],
      cycles: 2,
    },
  ],
};
