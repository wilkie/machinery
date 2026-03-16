import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const cli: InstructionInfo = {
  identifier: 'cli',
  name: 'Clear Interrupt Flag',
  description:
    '`CLI` clears the interrupt enable flag if the current privilege level is at least as privileged as `I0PL`. No other flags are affected. External interrupts will not be recognized at the end of the `CLI` instruction or thereafter until the interrupt flag is set.',
  modifies: ['IF'],
  undefined: [],
  forms: [
    // 0xFA - CLI
    {
      opcode: [Opcodes.CLI],
      operands: [],
      modes: {
        real: {
          operation: ['${RESOLVE_FLAGS}', 'IF = 0'],
        },
        protected: {
          operation: ['${RESOLVE_FLAGS}', '#GP if CS.RPL > IOPL', 'IF = 0'],
        },
      },
      cycles: 3,
    },
  ],
};
