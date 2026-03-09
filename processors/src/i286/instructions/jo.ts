import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const jo: InstructionInfo = {
  identifier: 'jo',
  name: 'Jump if Overflow',
  description: 'Jump near if OF = 1',
  modifies: [],
  undefined: [],
  forms: [
    // 0x70 cb - JO cb
    {
      operation: ['${RESOLVE_OF}', 'IP = (OF == 1) ? IP + %{IMM} : IP'],
      opcode: [Opcodes.JO, 'IMM_i8'],
      operands: ['rel'],
      operandSize: 8,
      cycles: 3, // 7 if jumped
    },
  ],
};
