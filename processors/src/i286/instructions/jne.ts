import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const jne: InstructionInfo = {
  identifier: 'jne',
  aliases: ['jnz'],
  name: 'Jump if Not Equal',
  description: 'Jump near if ZF = 0',
  modifies: [],
  undefined: [],
  forms: [
    // 0x75 cb - JNE cb
    // 0x75 cb - JNZ cb
    {
      operation: ['${RESOLVE_ZF}', 'IP = (ZF == 0) ? IP + %{IMM} : IP'],
      opcode: [Opcodes.JNE, 'IMM_i8'],
      operands: ['rel'],
      operandSize: 8,
      cycles: 3, // 7 if jumped
    },
  ],
};
