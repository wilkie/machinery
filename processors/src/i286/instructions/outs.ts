import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const outs: InstructionInfo = {
  identifier: 'outs',
  name: 'Output String to Port',
  description:
    '`OUTS` transfers data from the memory byte or word at `SI` to the output port numbered by the `DX` register.\n\n`OUTS` does not allow the specification of the port number as an immediate value. The port must be addressed through the `DX` register.\n\nAfter the transfer is made, `SI` is automatically advanced. If the direction flag is 0 (`CLD` was executed), `SI` increments; if the direction flag is 1 (`STD` was executed), `SI` decrements. `SI` increments or decrements by 1 if a byte was moved; by 2 if a word was moved.\n\n`OUTS` can be preceded by the `REP` prefix for block output of `CX` bytes or words. Refer to the `REP` instruction for details of this operation.\n\nIntel reserves I/O port addresses `0x00F8` through `0x00FF`; these addresses should not be used.',
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
    // 0x6E - OUTS DX, eb
    // 0x6E - OUTSB
    {
      modes: {
        real: {
          operation: [
            'loop',
            [
              'next if REP != 0 && CX == 0',
              'effective_address = DATA_SEG_BASE + SI',
              'IO:u8[DX] = RAM:u8[effective_address]',
              'SI = SI + (DF == 1 ? -1 : 1)',
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
              'effective_address = DATA_SEG_BASE + offset',
              '#GP if offset < DATA_SEG_LIMIT_MIN',
              '#GP if offset > DATA_SEG_LIMIT_MAX',
              'IO:u8[DX] = RAM:u8[effective_address]',
              'SI = SI + (DF == 1 ? -1 : 1)',
              'CX = REP != 0 ? CX - 1 : CX',
            ],
            'repeat if REP != 0',
          ],
        },
      },
      opcode: [Opcodes.OUTS_DX_EB],
      operands: [],
      operandSize: 8,
      aliases: ['outsb'],
      cycles: 5,
    },
    // 0x6F - OUTS DX, ew
    // 0x6F - OUTSW
    {
      modes: {
        real: {
          operation: [
            'loop',
            [
              'next if REP != 0 && CX == 0',
              'offset = SI',
              'effective_address = DATA_SEG_BASE + offset',
              '${SEGMENT_LIMIT_CHECK_REAL}',
              'IO:u16[DX] = RAM:u16[effective_address]',
              'SI = SI + (DF == 1 ? -2 : 2)',
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
              'effective_address = DATA_SEG_BASE + offset',
              '#GP if (offset + 1) < DATA_SEG_LIMIT_MIN',
              '#GP if (offset + 1) > DATA_SEG_LIMIT_MAX',
              'IO:u16[DX] = RAM:u16[effective_address]',
              'SI = SI + (DF == 1 ? -2 : 2)',
              'CX = REP != 0 ? CX - 1 : CX',
            ],
            'repeat if REP != 0',
          ],
        },
      },
      opcode: [Opcodes.OUTS_DX_EW],
      operands: [],
      operandSize: 8,
      aliases: ['outsw'],
      cycles: 5,
    },
  ],
};
