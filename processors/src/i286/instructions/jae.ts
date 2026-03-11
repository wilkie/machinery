import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const jae: InstructionInfo = {
  identifier: 'jae',
  name: 'Jump if Above or Equal',
  description: 'Jump near if CF = 0',
  modifies: [],
  undefined: [],
  forms: [
    // 0x73 cb - JAE cb
    {
      operation: ['${RESOLVE_CF}', 'IP = (CARRY == 0) ? IP + %{IMM} : IP'],
      opcode: [Opcodes.JAE, 'IMM_i8'],
      operands: ['rel'],
      operandSize: 8,
      distance: 'short',
      addressing: 'relative',
      cycles: 3, // 7 if jumped
    },
  ],
};
