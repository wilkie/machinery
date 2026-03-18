import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const lock: InstructionInfo = {
  identifier: 'lock',
  prefix: true,
  name: 'Assert LOCK# Signal Prefix',
  description:
    'The `LOCK` prefix causes the processor to assert the bus `LOCK#` signal during execution of the instruction that follows it. In a multiprocessor environment, this signal ensures that the processor has exclusive use of any shared memory while `LOCK#` is asserted.\n\nThe `LOCK` prefix can only be prepended to the following instructions: `ADC`, `ADD`, `AND`, `BTC`, `BTR`, `BTS`, `CMPXCHG`, `DEC`, `INC`, `NEG`, `NOT`, `OR`, `SBB`, `SUB`, `XCHG`, and `XOR`.',
  modifies: [],
  undefined: [],
  forms: [
    {
      // 0xF0 LOCK
      opcode: [Opcodes.LOCK_PREFIX],
      operands: [],
      modes: {
        real: {
          operation: [],
        },
        protected: {
          operation: ['#GP if CS.RPL > IOPL'],
        },
      },
    },
  ],
};
