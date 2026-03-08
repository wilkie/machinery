import type { InstructionInfo } from '@rawrs/architecture/Target';

import { Opcodes } from '../opcodes';

export const sahf: InstructionInfo = {
  identifier: 'sahf',
  name: 'Store AH into Flags',
  description:
    'The flags listed above are loaded with values from the AH register, from bits 7, 6, 4, 2, and 0, respectively.',
  modifies: ['SF', 'ZF', 'AF', 'PF', 'CF'],
  undefined: [],
  forms: [
    // 0x9E - SAHF
    {
      opcode: [Opcodes.SAHF],
      operation: [
        '${RESOLVE_FLAGS}',
        'FLAGS = (FLAGS & ~0b11010101) | (AH & 0b11010101)',
        'CARRY = CF',
      ],
      cycles: 2,
    },
  ],
};
