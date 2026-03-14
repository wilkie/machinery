import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

// TODO: segment moves
// TODO: protected mode (cannot mov to CS... on real mode, it seems possible even if undocumented)
// TODO: #13 for operand 0xffff
// TODO: #UD for FS, GS perhaps, if emulating a real 286
// TODO: #UD for abnormal segment selectors (reg/rm: 110 and 111)
export const stos: InstructionInfo = {
  identifier: 'stos',
  name: 'Store String Data',
  description:
    'STOS transfers the contents the `AL` or `AX` register to the memory byte or word at `ES:DI`. The operand must be addressable from the `ES` register, no segment override is possible.\n\nAfter the transfer is made, `DI` is automatically advanced. If the direction flag is 0 (`CLD` was executed), `DI` increments; if the direction flag is 1 (`STD` was executed), `DI` decrements. `DI` increments or decrements by 1 if a byte was moved; by 2 if a word was moved.\n\n`STOS` can be preceded by the `REP` prefix for a block fill of `CX` bytes or words. Refer to the `REP` instruction for details of this operation.',
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
    // 0xAA STOS mb
    // 0xAA STOSB
    {
      modes: {
        real: {
          operation: [
            'loop',
            [
              'next if REP != 0 && CX == 0',
              'RAM:u8[ES_BASE + DI] = AL',
              'DI = DI + (DF == 1 ? -1 : 1)',
              'CX = REP != 0 ? CX - 1 : CX',
            ],
            'repeat if REP != 0',
          ],
        },
        protected: {
          operation: [
            'loop',
            [
              'next if REP != 0 && CX == 0',
              'offset = DI',
              'effective_address = ES_BASE + offset',
              '#GP if offset < ES_LIMIT_MIN',
              '#GP if offset > ES_LIMIT_MAX',
              'RAM:u8[ES_BASE + DI] = AL',
              'DI = DI + (DF == 1 ? -1 : 1)',
              'CX = REP != 0 ? CX - 1 : CX',
            ],
            'repeat if REP != 0',
          ],
        },
      },
      opcode: [Opcodes.STOS_MB],
      operands: [],
      operandSize: 8,
      aliases: ['stosb'],
      cycles: 3,
    },
    // 0xAB STOS mw
    // 0xAB STOSW
    {
      modes: {
        real: {
          operation: [
            'loop',
            [
              'next if REP != 0 && CX == 0',
              'RAM:u16[ES_BASE + DI] = AX',
              'DI = DI + (DF == 1 ? -2 : 2)',
              'CX = REP != 0 ? CX - 1 : CX',
            ],
            'repeat if REP != 0',
          ],
        },
        protected: {
          operation: [
            'loop',
            [
              'next if REP != 0 && CX == 0',
              'offset = DI',
              'effective_address = ES_BASE + offset',
              '#GP if (offset + 1) < ES_LIMIT_MIN',
              '#GP if (offset + 1) > ES_LIMIT_MAX',
              'RAM:u16[ES_BASE + DI] = AX',
              'DI = DI + (DF == 1 ? -2 : 2)',
              'CX = REP != 0 ? CX - 1 : CX',
            ],
            'repeat if REP != 0',
          ],
        },
      },
      opcode: [Opcodes.STOS_MW],
      operands: [],
      operandSize: 16,
      aliases: ['stosw'],
      cycles: 3,
    },
  ],
};
