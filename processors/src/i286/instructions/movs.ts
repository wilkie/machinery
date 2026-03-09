import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

// TODO: segment moves
// TODO: protected mode (cannot mov to CS... on real mode, it seems possible even if undocumented)
// TODO: #13 for operand 0xffff
// TODO: #UD for FS, GS perhaps, if emulating a real 286
// TODO: #UD for abnormal segment selectors (reg/rm: 110 and 111)
export const movs: InstructionInfo = {
  identifier: 'movs',
  aliases: ['movsb', 'movsw'],
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
  ],
  forms: [
    // 0xA4 MOVS mb, mb
    // 0xA4 MOVSB
    {
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
      opcode: [Opcodes.MOVS_MB_MB],
      operands: [],
      operandSize: 8,
      cycles: 5,
    },
    // 0xA5 MOVS mw, mw
    // 0xA5 MOVSW
    {
      operation: [
        'loop',
        [
          'next if REP != 0 && CX == 0',
          'effective_address = (DATA_SEG_BASE == 0xffff ? DS_BASE : DATA_SEG_BASE) + SI',
          'RAM:u16[ES_BASE + DI] = RAM:u16[effective_address]',
          'SI = SI + (DF == 1 ? -2 : 2)',
          'DI = DI + (DF == 1 ? -2 : 2)',
          'CX = REP != 0 ? CX - 1 : CX',
        ],
        'repeat if REP != 0',
      ],
      opcode: [Opcodes.MOVS_MW_MW],
      operands: [],
      operandSize: 16,
      cycles: 5,
    },
  ],
};
