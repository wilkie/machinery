import type { InstructionInfo } from '@rawrs/architecture/Target';

import { Opcodes } from '../opcodes';

export const js: InstructionInfo = {
  identifier: 'js',
  name: 'Jump if Sign',
  description: 'Jump near if SF = 1',
  modifies: [],
  undefined: [],
  forms: [
    // 0x78 cb - JS cb
    {
      operation: ['${RESOLVE_SF}', 'IP = (SF == 1) ? IP + %{IMM} : IP'],
      opcode: [Opcodes.JS, 'IMM_i8'],
      operandSize: 8,
      cycles: 3, // 7 if jumped
    },
  ],
};
