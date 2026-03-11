import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const out: InstructionInfo = {
  identifier: 'out',
  name: 'Output to Port',
  description:
    '`OUT` transfers a data byte or data word from the register (`AL` or `AX`) given as the second operand to the output port numbered by the first operand. You can output to any port from 0-65535 by placing the port number in the `DX` register then using an `OUT` instruction with `DX` as the first operand. If the instruction contains an 8-bit port ID, that value is zero-extended to 16 bits.\n\nIntel reserves I/O port addresses `0x00F8` through `0x00FF`; these addresses should not be used.',
  modifies: [],
  undefined: [],
  forms: [
    // 0xE6 db - OUT db, AL
    {
      operation: ['IO:u8[%{imm}] = AL'],
      opcode: [Opcodes.OUT_DB_AL, 'IMM_u8'],
      operands: ['imm', 'AL'],
      operandSize: 8,
      cycles: 5,
    },
    // 0xE7 db - OUT db, AX
    {
      operation: ['IO:u16[%{imm}] = AX'],
      opcode: [Opcodes.OUT_DB_AX, 'IMM_u8'],
      operands: ['imm', 'AX'],
      operandSize: 16,
      cycles: 5,
    },
    // 0xEE - OUT DX, AL
    {
      operation: ['IO:u8[DX] = AL'],
      opcode: [Opcodes.OUT_DX_AL],
      operands: ['DX', 'AL'],
      operandSize: 8,
      cycles: 5,
    },
    // 0xEF - OUT DX, AX
    {
      operation: ['IO:u16[DX] = AX'],
      opcode: [Opcodes.OUT_DX_AX],
      operands: ['DX', 'AX'],
      operandSize: 16,
      cycles: 5,
    },
  ],
};
