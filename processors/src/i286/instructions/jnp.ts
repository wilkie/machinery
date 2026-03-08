import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const jnp: InstructionInfo = {
  identifier: 'jnp',
  aliases: ['jpo'],
  name: 'Jump if No Parity',
  description: 'Jump near if PF = 1',
  modifies: [],
  undefined: [],
  forms: [
    // 0x7B cb - JNP cb
    // 0x7B cb - JPO cb
    {
      operation: ['${RESOLVE_PF}', 'IP = (PF == 0) ? IP + %{IMM} : IP'],
      opcode: [Opcodes.JNP, 'IMM_i8'],
      operandSize: 8,
      cycles: 3, // 7 if jumped
    },
  ],
};
