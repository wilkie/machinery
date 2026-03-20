import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

// TODO: segment moves
// TODO: protected mode (cannot mov to CS... on real mode, it seems possible even if undocumented)
// TODO: #13 for operand 0xffff
// TODO: #UD for FS, GS perhaps, if emulating a real 286
// TODO: #UD for abnormal segment selectors (reg/rm: 110 and 111)
export const movs: InstructionInfo = {
  identifier: 'movs',
  name: 'Move Data from String to String',
  description:
    '`MOVS` copies the byte or word at `[SI]` to the byte or word at `ES:[DI]`. The destination operand must be addressable from the `ES` register; no segment override is possible. A segment override may be used for the source operand.\n\nAfter the data movement is made, both `SI` and `DI` are automatically advanced. If the direction flag is 0 (`CLD` was executed), the registers increment; if the direction flag is `1` (`STD` was executed), the registers decrement. The registers increment or decrement by `1` if a byte was moved; by `2` if a word was moved.\n\n`MOVS` can be preceded by the `REP` prefix for block movement of `CX` bytes or words. Refer to the `REP` instruction for details of this operation.',
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
    {
      identifier: 'value',
      name: 'Source Value',
      size: 16,
    },
  ],
  forms: [
    // 0xA4 MOVS mb, mb
    // 0xA4 MOVSB
    {
      modes: {
        real: {
          operation: [
            'loop',
            [
              'next if REP != 0 && CX == 0',
              'effective_address = (DATA_SEG_BASE == 0xffff ? DS_BASE : DATA_SEG_BASE) + SI',
              'RAM:u8[ES_BASE + DI] = RAM:u8[effective_address]',
              'SI = SI + (DF == 1 ? -1 : 1)',
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
              'offset = SI',
              'effective_address = (DATA_SEG_BASE == 0xffff ? DS_BASE : DATA_SEG_BASE) + offset',
              '#GP if offset < (DATA_SEG_BASE == 0xffff ? DS_LIMIT_MIN : DATA_SEG_LIMIT_MIN)',
              '#GP if offset > (DATA_SEG_BASE == 0xffff ? DS_LIMIT_MAX : DATA_SEG_LIMIT_MAX)',
              'offset = DI',
              'effective_address = ES_BASE + offset',
              '#GP if offset < ES_LIMIT_MIN',
              '#GP if offset > ES_LIMIT_MAX',
              'effective_address = (DATA_SEG_BASE == 0xffff ? DS_BASE : DATA_SEG_BASE) + SI',
              'RAM:u8[ES_BASE + DI] = RAM:u8[effective_address]',
              'SI = SI + (DF == 1 ? -1 : 1)',
              'DI = DI + (DF == 1 ? -1 : 1)',
              'CX = REP != 0 ? CX - 1 : CX',
            ],
            'repeat if REP != 0',
          ],
        },
      },
      opcode: [Opcodes.MOVS_MB_MB],
      operands: [],
      operandSize: 8,
      aliases: ['movsb'],
      cycles: 5,
    },
    // 0xA5 MOVS mw, mw
    // 0xA5 MOVSW
    {
      modes: {
        real: {
          operation: [
            'loop',
            [
              'next if REP != 0 && CX == 0',
              'offset = SI',
              'SI = SI + (DF == 1 ? -2 : 2)',
              'CX = REP != 0 ? CX - 1 : CX',
              'effective_address = (DATA_SEG_BASE == 0xffff ? DS_BASE : DATA_SEG_BASE) + offset',
              '${SEGMENT_LIMIT_CHECK_REAL}',
              'value = RAM:u16[effective_address]',
              'offset = DI',
              'DI = DI + (DF == 1 ? -2 : 2)',
              'CX = REP != 0 ? CX - 1 : CX',
              'effective_address = ES_BASE + offset',
              '${SEGMENT_LIMIT_CHECK_REAL}',
              'RAM:u16[effective_address] = value',
              'CX = REP != 0 ? CX + 1 : CX',
            ],
            'repeat if REP != 0',
          ],
        },
        protected: {
          operation: [
            'loop',
            [
              'next if REP != 0 && CX == 0',
              'offset = SI',
              'effective_address = (DATA_SEG_BASE == 0xffff ? DS_BASE : DATA_SEG_BASE) + offset',
              '#GP if (offset + 1) < (DATA_SEG_BASE == 0xffff ? DS_LIMIT_MIN : DATA_SEG_LIMIT_MIN)',
              '#GP if (offset + 1) > (DATA_SEG_BASE == 0xffff ? DS_LIMIT_MAX : DATA_SEG_LIMIT_MAX)',
              'offset = DI',
              'effective_address = ES_BASE + offset',
              '#GP if (offset + 1) < ES_LIMIT_MIN',
              '#GP if (offset + 1) > ES_LIMIT_MAX',
              'effective_address = (DATA_SEG_BASE == 0xffff ? DS_BASE : DATA_SEG_BASE) + SI',
              'RAM:u16[ES_BASE + DI] = RAM:u16[effective_address]',
              'SI = SI + (DF == 1 ? -2 : 2)',
              'DI = DI + (DF == 1 ? -2 : 2)',
              'CX = REP != 0 ? CX - 1 : CX',
            ],
            'repeat if REP != 0',
          ],
        },
      },
      opcode: [Opcodes.MOVS_MW_MW],
      operands: [],
      operandSize: 16,
      aliases: ['movsw'],
      cycles: 5,
    },
  ],
};
