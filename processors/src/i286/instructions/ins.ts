import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const ins: InstructionInfo = {
  identifier: 'ins',
  name: 'Input String from Port',
  description:
    '`INS` transfers data from the input port numbered by the `DX` register to the memory byte or word at `ES:DI`. The memory operand must be addressable from the `ES` register; no segment override is possible.\n\n`INS` does not allow the specification of the port number as an immediate value. The port must be addressed through the `DX` register.\n\nAfter the transfer is made, `DI` is automatically advanced. If the direction flag is 0 (`CLD` was executed), `DI` increments; if the direction flag is 1 (`STD` was executed), `DI` decrements. `DI` increments or decrements by 1 if a byte was moved; by 2 if a word was moved.\n\n`INS` can be preceded by the `REP` prefix for block input of `CX` bytes or words. Refer to the `REP` instruction for details of this operation.\n\nIntel has reserved I/O port addresses `0x00F8` through `0x00FF`; they should not be used.',
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
    // 0x6C - INS eb, DX
    // 0x6C - INSB
    {
      operation: [
        'loop',
        [
          'next if REP != 0 && CX == 0',
          'effective_address = ES_BASE + DI',
          'RAM:u8[effective_address] = IO.read(1, DX)',
          'DI = DI + (DF == 1 ? -1 : 1)',
          'CX = REP != 0 ? CX - 1 : CX',
        ],
        'repeat if REP != 0',
      ],
      opcode: [Opcodes.INS_EB_DX],
      operands: [],
      operandSize: 8,
      aliases: ['insb'],
      cycles: 5,
    },
    // 0x6D - INS ew, DX
    // 0x6D - INSW
    {
      operation: [
        'loop',
        [
          'next if REP != 0 && CX == 0',
          'effective_address = ES_BASE + DI',
          'RAM:u16[effective_address] = IO.read(2, DX)',
          'DI = DI + (DF == 1 ? -2 : 2)',
          'CX = REP != 0 ? CX - 1 : CX',
        ],
        'repeat if REP != 0',
      ],
      opcode: [Opcodes.INS_EW_DX],
      operands: [],
      operandSize: 8,
      aliases: ['insw'],
      cycles: 5,
    },
  ],
};
