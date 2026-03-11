import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const cmps: InstructionInfo = {
  identifier: 'cmps',
  name: 'Compare String Operands',
  description:
    '`CMPS` compares the byte or word pointed to by `SI` with the byte or word pointed to by `DI` by performing the subtraction `[SI] - [DI]`. The result is not placed anywhere; only the flags reflect the result of the subtraction. The types of the operands to `CMPS` determine whether bytes or words are compared. The segment addressability of the first `(SI)` operand determines whether a segment override byte will be produced or whether the default segment register `DS` is used. The second `(DI)` operand must be addressible from the `ES` register; no segment override is possible.\n\nAfter the comparison is made, both `SI` and `DI` are automatically advanced. If the direction flag is 0 (`CLD` was executed), the registers increment; if the direction flag is 1 (`STD` was executed), the registers decrement. The registers increment or decrement by 1 if a byte was moved; by 2 if a word was moved.\n\n`CMPS` can be preceded by the `REPE` or `REPNE` prefix for block comparison of `CX` bytes or words. Refer to the `REP` instruction for details of this operation.',
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
  locals: [
    {
      identifier: 'effective_address',
      name: 'Effective Address',
      size: 32,
    },
  ],
  forms: [
    // 0xA6 CMPS mb, mb
    // 0xA6 CMPSB
    {
      operation: [
        'loop',
        [
          'next if REP != 0 && CX == 0',
          'effective_address = (DATA_SEG_BASE == 0xffff ? ES_BASE : DATA_SEG_BASE) + SI',
          'a = RAM:u8[effective_address]',
          'b = RAM:u8[ES_BASE + DI]',
          '${ALU8_OP}',
          'DI = DI + (DF == 1 ? -1 : 1)',
          'SI = SI + (DF == 1 ? -1 : 1)',
          'CX = REP != 0 ? CX - 1 : CX',
        ],
        'repeat if REP != 0 && ((alu_result & 0xff) == 0 ? 0 : 1) == REP_CHECK',
      ],
      opcode: [Opcodes.CMPS_MB_MB],
      operands: [],
      operandSize: 8,
      aliases: ['cmpsb'],
      cycles: 7,
    },
    // 0xA7 CMPS mw, mw
    // 0xA7 CMPSW
    {
      operation: [
        'loop',
        [
          'next if REP != 0 && CX == 0',
          'effective_address = (DATA_SEG_BASE == 0xffff ? ES_BASE : DATA_SEG_BASE) + SI',
          'a = RAM:u16[effective_address]',
          'b = RAM:u16[ES_BASE + DI]',
          '${ALU16_OP}',
          'DI = DI + (DF == 1 ? -2 : 2)',
          'SI = SI + (DF == 1 ? -2 : 2)',
          'CX = REP != 0 ? CX - 1 : CX',
        ],
        'repeat if REP != 0 && ((alu_result & 0xffff) == 0 ? 0 : 1) == REP_CHECK',
      ],
      opcode: [Opcodes.CMPS_MW_MW],
      operands: [],
      operandSize: 16,
      aliases: ['cmpsw'],
      cycles: 7,
    },
  ],
};
