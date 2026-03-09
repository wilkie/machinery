import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const jns: InstructionInfo = {
  identifier: 'jns',
  name: 'Jump if Not Sign',
  description: 'Jump near if SF = 0',
  modifies: [],
  undefined: [],
  forms: [
    // 0x79 cb - JNS cb
    {
      operation: ['${RESOLVE_SF}', 'IP = (SF == 0) ? IP + %{IMM} : IP'],
      opcode: [Opcodes.JNS, 'IMM_i8'],
      operands: ['rel'],
      operandSize: 8,
      cycles: 3, // 7 if jumped
    },
  ],
};
