import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const lods: InstructionInfo = {
  identifier: 'lods',
  name: 'Load String Operand',
  description:
    '`LODS` loads the `AL` or `AX` register with the memory byte or word at `SI`. After the transfer is made, `SI` is automatically advanced. If the direction flag is 0 (`CLD` was executed), `SI` increments; if the direction flag is 1 (`STD` was executed), `SI` decrements. `SI` increments or decrements by 1 if a byte was moved; by 2 if a word was moved.',
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
    // 0xAC LODS mb
    // 0xAC LODSB
    {
      operation: [
        'loop',
        [
          'next if REP != 0 && CX == 0',
          'effective_address = (DATA_SEG_BASE == 0xffff ? DS_BASE : DATA_SEG_BASE) + SI',
          'AL = RAM:u8[effective_address]',
          'SI = SI + (DF == 1 ? -1 : 1)',
          'CX = REP != 0 ? CX - 1 : CX',
        ],
        'repeat if REP != 0',
      ],
      opcode: [Opcodes.LODS_MB],
      operands: [],
      operandSize: 8,
      aliases: ['lodsb'],
      cycles: 5,
    },
    // 0xAD LODS mw
    // 0xAD LODSW
    {
      operation: [
        'loop',
        [
          'next if REP != 0 && CX == 0',
          'effective_address = (DATA_SEG_BASE == 0xffff ? DS_BASE : DATA_SEG_BASE) + SI',
          'AX = RAM:u16[effective_address]',
          'SI = SI + (DF == 1 ? -2 : 2)',
          'CX = REP != 0 ? CX - 1 : CX',
        ],
        'repeat if REP != 0',
      ],
      opcode: [Opcodes.LODS_MW],
      operands: [],
      operandSize: 16,
      aliases: ['lodsw'],
      cycles: 5,
    },
  ],
};
