import type { InstructionInfo } from '@rawrs/architecture/Target';

import { Opcodes } from '../opcodes';

export const nop: InstructionInfo = {
  identifier: 'nop',
  name: 'No Operation',
  description:
    'Performs no operation. `NOP` is a one-byte filler instruction that takes up space but affects none of the machine context except `IP`.',
  modifies: [],
  undefined: [],
  forms: [
    // 0x90 - NOP
    {
      opcode: [Opcodes.NOP],
      operation: [],
      cycles: 3,
    },
  ],
};
