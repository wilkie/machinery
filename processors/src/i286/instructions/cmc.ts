import type { InstructionInfo } from '@rawrs/architecture/Target';

import { Opcodes } from '../opcodes';

export const cmc: InstructionInfo = {
  identifier: 'cmc',
  name: 'Complement Carry Flag',
  description:
    '`CMC` reverses the setting of the carry flag. No other flags are affected.',
  modifies: ['CF'],
  undefined: [],
  forms: [
    // 0xF5 - CMC
    {
      opcode: [Opcodes.CMC],
      operation: [
        'CF = ~CF',
        'CARRY = CF',
        'flag_op = flag_op | ${FLAG_OP_NOCF}',
      ],
      cycles: 2,
    },
  ],
};
