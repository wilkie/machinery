import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const hlt: InstructionInfo = {
  identifier: 'hlt',
  name: 'Halt',
  description:
    'Successful execution of `HLT` causes the 80286 to cease executing instructions and to enter a `HALT` state. Execution resumes only upon receipt of an enabled interrupt or a reset. If an interrupt is used to resume program execution after `HLT`, the saved `CS:IP` value will point to the instruction that follows `HLT`.',
  modifies: ['DF'],
  undefined: [],
  forms: [
    // 0xF4 - HLT
    {
      opcode: [Opcodes.HLT],
      operands: [],
      operation: [
        '${RESOLVE_FLAGS}',
        'HALTED = 1',
        'HALTED_CS = CS',
        'HALTED_IP = IP',
        ';; switch to halted mode',
        '@mode = @modes.halted',
      ],
      cycles: 2,
    },
  ],
};
