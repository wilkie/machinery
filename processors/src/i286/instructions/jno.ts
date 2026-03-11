import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const jno: InstructionInfo = {
  identifier: 'jno',
  name: 'Jump if No Overflow',
  description: 'Jump near if OF = 0',
  modifies: [],
  undefined: [],
  forms: [
    // 0x71 cb - JNO cb
    {
      operation: ['${RESOLVE_OF}', 'IP = (OF == 0) ? IP + %{imm} : IP'],
      opcode: [Opcodes.JNO, 'IMM_i8'],
      operands: ['rel'],
      operandSize: 8,
      distance: 'short',
      addressing: 'relative',
      cycles: 3, // 7 if jumped
    },
  ],
};
