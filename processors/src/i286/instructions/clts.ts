import type { InstructionInfo } from '@machinery/core';

import { Opcodes, SystemOpcodes } from '../opcodes';

export const clts: InstructionInfo = {
  identifier: 'clts',
  name: 'Clear Task Switch Flag',
  description:
    "`CLTS` clears the task switched flag in the Machine Status Word (`MSW`). This flag is set by the 80286 every time a task switch occurs. The `TS` flag is used to manage processor extensions as follows: every execution of a `WAIT` or an `ESC` instruction will be trapped if the `MP` flag of `MSW` is set and the task switched flag is set. Thus, if a processor extension is present and a task switch has been made since the last `ESC` instruction was begun, the processor extension's context must be saved before a new instruction can be issued. The fault routine will save the context and reset the task switched flag or place the task requesting the processor extension into a queue until the current processor extension instruction is completed.\n\n`CLTS` appears in operating systems software, not in applications programs. It is a privileged instruction that can only be executed at level 0.",
  modifies: ['MSW.TS'],
  undefined: [],
  forms: [
    // 0x0F 0x06 CLTS
    {
      opcode: [Opcodes.SYSTEM, SystemOpcodes.CLTS],
      modes: {
        real: {
          operation: ['MSW.TS = 0'],
          cycles: 2,
        },
        protected: {
          operation: ['#GP if CS.RPL != 0', 'MSW.TS = 0'],
          cycles: 2,
        },
      },
    },
  ],
};
