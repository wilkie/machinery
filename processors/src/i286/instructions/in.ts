import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const in_: InstructionInfo = {
  identifier: 'in',
  name: 'Input from Port',
  description:
    '`IN` transfers a data byte or data word from the port numbered by the second operand into the register (`AL` or `AX`) given as the first operand. You can access any port from 0 to 65535 by placing the port number in the `DX` register then using an `IN` instruction with `DX` as the second parameter. These I/O instructions can be shortened by using an 8-bit port I/O in the instruction. The upper 8 bits of the port address will be zero when an 8-bit port I/O is used.\n\nIntel has reserved I/O port addresses `0x00F8` through `0x00FF`; they should not be used.',
  modifies: [],
  undefined: [],
  forms: [
    // 0xE4 db - IN AL, db
    {
      modes: {
        real: {
          operation: ['AL = IO:u8[%{imm}]'],
        },
        protected: {
          operation: ['#GP if CS.RPL > IOPL', 'AL = IO:u8[%{imm}]'],
        },
      },
      opcode: [Opcodes.IN_AL_DB, 'IMM_u8'],
      operands: ['AL', 'imm'],
      operandSize: 8,
      cycles: 5,
    },
    // 0xE5 db - IN AX, db
    {
      modes: {
        real: {
          operation: ['AX = IO:u16[%{imm}]'],
        },
        protected: {
          operation: ['#GP if CS.RPL > IOPL', 'AX = IO:u16[%{imm}]'],
        },
      },
      opcode: [Opcodes.IN_AX_DB, 'IMM_u8'],
      operands: ['AX', 'imm'],
      operandSize: 16,
      cycles: 5,
    },
    // 0xEC - IN AL, DX
    {
      modes: {
        real: {
          operation: ['AL = IO:u8[DX]'],
        },
        protected: {
          operation: ['#GP if CS.RPL > IOPL', 'AL = IO:u8[DX]'],
        },
      },
      opcode: [Opcodes.IN_AL_DX],
      operands: ['AL', 'DX'],
      operandSize: 8,
      cycles: 5,
    },
    // 0xED - IN AX, DX
    {
      modes: {
        real: {
          operation: ['AX = IO:u16[DX]'],
        },
        protected: {
          operation: ['#GP if CS.RPL > IOPL', 'AX = IO:u16[DX]'],
        },
      },
      opcode: [Opcodes.IN_AX_DX],
      operands: ['AX', 'DX'],
      operandSize: 16,
      cycles: 5,
    },
  ],
};
