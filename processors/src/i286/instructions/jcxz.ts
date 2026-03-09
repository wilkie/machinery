import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const jcxz: InstructionInfo = {
  identifier: 'jcxz',
  name: 'Jump if CX is zero',
  description:
    'Jump near if CX = 0.\n\n`JCXZ` differs from the other conditional jumps in that it actually tests the contents of the `CX` register for zero, rather than interrogating the flags. This instruction is useful following a conditionally repeated string operation (`REPE SCASB`, for example) or a conditional loop instruction (such as `LOOPNE`). These instructions implicitly use a limiting count in the `CX` register. Looping (repeating) ends when either the `CX` register goes to zero or the condition specified in the instruction (flags indicating equals in both of the above cases) occurs. `JCXZ` is useful when the terminations must be handled differently.',
  modifies: [],
  undefined: [],
  forms: [
    // 0xE3 cb - JCXZ cb
    {
      operation: ['IP = (CX == 0) ? IP + %{IMM} : IP'],
      opcode: [Opcodes.JCXZ, 'IMM_i8'],
      operands: ['rel'],
      operandSize: 8,
      cycles: 4, // 8 if jumped
    },
  ],
};
