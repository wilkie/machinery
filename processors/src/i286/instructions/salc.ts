import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const salc: InstructionInfo = {
  identifier: 'salc',
  name: 'Set AL from Carry Flag',
  description:
    'Undocumented instruction. Sets `AL` to `0xFF` if the carry flag is set, or `0x00` if the carry flag is clear.',
  modifies: [],
  undefined: [],
  forms: [
    // 0xD6 - SALC
    {
      opcode: [Opcodes.SALC],
      operands: [],
      operation: ['${RESOLVE_FLAGS}', 'AL = CF > 0 ? 0xFF : 0x00'],
      cycles: 3,
    },
  ],
};
