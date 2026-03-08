import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const rep: InstructionInfo = {
  identifier: 'rep',
  aliases: ['repe'],
  prefix: true,
  name: 'Repeat String Operation While Equal Prefix',
  description:
    '`REP`, `REPE`, and `REPNE` are prefix operations. These prefixes cause the string instruction that follows to be repeated `CX` times or (for `REPE` and `REPNE`) until the indicated condition in the zero flag is no longer met. Thus, `REPE` stands for "Repeat while equal," `REPNE` for "Repeat while not equal.".\n\nThe `REP` prefixes make sense only in contexts such as `INS`, `OUTS`, `MOVS`, `STOS`, `CMPS`, and `SCAS`. They cannot be applied to anything other than string operations.',
  modifies: [],
  undefined: [],
  forms: [
    {
      // 0xF3 REP
      // 0xF3 REPE
      opcode: [Opcodes.REP_PREFIX],
      operation: ['REP = 1'],
      finalize: ['REP = 0'],
    },
  ],
};
