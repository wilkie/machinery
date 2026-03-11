import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const jb: InstructionInfo = {
  identifier: 'jb',
  aliases: ['jc', 'jnae'],
  name: 'Jump if Below',
  description: 'Jump near if CF = 1',
  modifies: [],
  undefined: [],
  forms: [
    // 0x72 cb - JB cb
    // 0x72 cb - JC cb
    // 0x72 cb - JNAE cb
    {
      operation: ['${RESOLVE_CF}', 'IP = (CARRY == 1) ? IP + %{IMM} : IP'],
      opcode: [Opcodes.JB, 'IMM_i8'],
      operands: ['rel'],
      operandSize: 8,
      distance: 'short',
      addressing: 'relative',
      cycles: 3, // 7 if jumped
    },
  ],
};
