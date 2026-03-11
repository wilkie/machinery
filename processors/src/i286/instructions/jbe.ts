import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const jbe: InstructionInfo = {
  identifier: 'jbe',
  aliases: ['jna'],
  name: 'Jump if Below or Equal',
  description: 'Jump near if CF = 1 or ZF = 1',
  modifies: [],
  undefined: [],
  forms: [
    // 0x72 cb - JBE cb
    // 0x72 cb - JNA cb
    {
      operation: [
        '${RESOLVE_CF}',
        '${RESOLVE_ZF}',
        'IP = (CARRY == 1 || ZF == 1) ? IP + %{IMM} : IP',
      ],
      opcode: [Opcodes.JBE, 'IMM_i8'],
      operands: ['rel'],
      operandSize: 8,
      distance: 'short',
      addressing: 'relative',
      cycles: 3, // 7 if jumped
    },
  ],
};
