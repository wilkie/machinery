import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

// #13 for offset of 0xffff
export const xlat: InstructionInfo = {
  identifier: 'xlat',
  name: 'Table Look-up Translation',
  description:
    'When `XLAT` is executed, `AL` should be the unsigned index into a table addressed by `DS:BX`. `XLAT` changes the `AL` register from the table index into the table entry. `BX` is unchanged.',
  modifies: [],
  undefined: [],
  locals: [
    {
      identifier: 'effective_address',
      name: 'Effective Address',
      size: 32,
    },
    {
      identifier: 'offset',
      name: 'Effective Offset',
      size: 32,
    },
  ],
  forms: [
    // 0xD7 - XLAT mb
    // 0xD7 - XLAT
    {
      opcode: [Opcodes.XLAT],
      operands: [],
      modes: {
        real: {
          operation: ['AL = RAM:u8[DS_BASE + BX + AL]'],
        },
        protected: {
          operation: [
            'offset = BX + AL',
            'effective_address = DS_BASE + offset',
            '#GP if offset < DS_LIMIT_MIN',
            '#GP if offset > DS_LIMIT_MAX',
            'AL = RAM:u8[effective_address]',
          ],
        },
      },
      cycles: 5,
    },
  ],
};
