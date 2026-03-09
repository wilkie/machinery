import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const scas: InstructionInfo = {
  identifier: 'scas',
  name: 'Compare String Data',
  description:
    '`SCAS` subtracts the memory byte or word at `ES:DI` from the `AL` or `AX` register. The result is discarded; only the flags are set. The operand must be addressable from the `ES` register; no segment override is possible.\n\nAfter the comparison is made, `DI` is automatically advanced. If the direction flag is 0 (`CLD` was executed), `DI` increments; if the direction flag is 1 (`STD` was executed), `DI` decrements. `DI` increments or decrements by 1 if bytes were compared; by 2 if words were compared.\n\n`SCAS` can be preceded by the `REPE` or `REPNE` prefix for a block search of `CX` bytes or words. Refer to the `REP` instruction for details of this operation.',
  modifies: ['OF', 'SF', 'ZF', 'AF', 'PF', 'CF'],
  undefined: [],
  macros: {
    ALU8_OP: [
      'alu_result = a - b',
      'flag_op = ${FLAG_OP_ALU} | ${FLAG_OP_SUB} | ${FLAG_OP_8BIT}',
    ],
    ALU16_OP: [
      'alu_result = a - b',
      'flag_op = ${FLAG_OP_ALU} | ${FLAG_OP_SUB} | ${FLAG_OP_16BIT}',
    ],
  },
  forms: [
    // 0xAE SCAS mb
    // 0xAE SCASB
    {
      operation: [
        'loop',
        [
          'next if REP != 0 && CX == 0',
          'a = AL',
          'b = RAM:u8[ES_BASE + DI]',
          '${ALU8_OP}',
          'DI = DI + (DF == 1 ? -1 : 1)',
          'CX = REP != 0 ? CX - 1 : CX',
        ],
        'repeat if REP != 0 && ((alu_result & 0xff) == 0 ? 0 : 1) == REP_CHECK',
      ],
      opcode: [Opcodes.SCAS_MB],
      operands: [],
      operandSize: 8,
      cycles: 7,
    },
    // 0xAF SCAS mw
    // 0xAF SCASW
    {
      operation: [
        'loop',
        [
          'next if REP != 0 && CX == 0',
          'a = AX',
          'b = RAM:u16[ES_BASE + DI]',
          '${ALU16_OP}',
          'DI = DI + (DF == 1 ? -2 : 2)',
          'CX = REP != 0 ? CX - 1 : CX',
        ],
        'repeat if REP != 0 && ((alu_result & 0xffff) == 0 ? 0 : 1) == REP_CHECK',
      ],
      opcode: [Opcodes.SCAS_MW],
      operands: [],
      operandSize: 16,
      cycles: 7,
    },
  ],
};
