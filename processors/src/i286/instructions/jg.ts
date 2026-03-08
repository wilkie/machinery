import type { InstructionInfo } from '@rawrs/architecture/Target';

import { Opcodes } from '../opcodes';

export const jg: InstructionInfo = {
  identifier: 'jg',
  aliases: ['jnle'],
  name: 'Jump if Greater',
  description: 'Jump near if ZF = 0 and SF = OF',
  modifies: [],
  undefined: [],
  forms: [
    // 0x7f cb - JG cb
    // 0x7f cb - JNLE cb
    {
      operation: [
        '${RESOLVE_ZF}',
        '${RESOLVE_SF}',
        '${RESOLVE_OF}',
        'IP = (ZF == 0 && SF == OF) ? IP + %{IMM} : IP',
      ],
      opcode: [Opcodes.JG, 'IMM_i8'],
      operandSize: 8,
      cycles: 3, // 7 if jumped
    },
  ],
};
