import type { InstructionInfo } from '@rawrs/architecture/Target';

import { Opcodes } from '../opcodes';

// #13 for offset of 0xffff
export const xlat: InstructionInfo = {
  identifier: 'xlat',
  name: 'Table Look-up Translation',
  description:
    'When `XLAT` is executed, `AL` should be the unsigned index into a table addressed by `DS:BX`. `XLAT` changes the `AL` register from the table index into the table entry. `BX` is unchanged.',
  modifies: [],
  undefined: [],
  forms: [
    // 0xD7 - XLAT mb
    // 0xD7 - XLAT
    {
      opcode: [Opcodes.XLAT],
      operation: ['AL = RAM:u8[DS_BASE + BX + AL]'],
      cycles: 5,
    },
  ],
};
