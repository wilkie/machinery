import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

// TODO: #13 for operand of 0xffff
export const leave: InstructionInfo = {
  identifier: 'leave',
  name: 'High Level Procedure Exit',
  description:
    "`LEAVE` is the complementary operation to `ENTER`; it reverses the effects of that instruction. By copying `BP` to `SP`, `LEAVE` releases the stack space used by a procedure for its dynamics and display. The old frame pointer is now popped into `BP`, restoring the caller's frame, and a subsequent `RET nn` instruction will follow the back-link and remove any arguments pushed on the stack for the exiting procedure.",
  modifies: [],
  undefined: [],
  forms: [
    // 0xC9 - LEAVE
    {
      operation: ['SP = BP + 2', 'BP = RAM:u16[SS_BASE + BP]'],
      opcode: [Opcodes.LEAVE],
      operands: [],
      operandSize: 16,
      cycles: 5,
    },
  ],
};
