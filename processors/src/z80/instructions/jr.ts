import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const jr: InstructionInfo = {
  identifier: 'jr',
  name: 'Jump Relative',
  description:
    'Unconditional or conditional jump by a signed 8-bit offset from the current PC.',
  modifies: [],
  forms: [
    // JR e — unconditional
    {
      opcode: [Opcodes.JR, 'REL_i8'],
      operands: ['rel'],
      addressing: 'relative',
      distance: 'short',
      operation: ['PC = PC + %{rel}'],
      cycles: 12,
    },
    // JR cc, e — conditional
    {
      opcode: [Opcodes.JR_NZ, 'REL_i8'],
      operands: ['rel'],
      aliases: ['jr nz'],
      addressing: 'relative',
      distance: 'short',
      operation: ['${RESOLVE_ZF}', 'PC = (ZF == 0) ? PC + %{rel} : PC'],
      cycles: 12,
    },
    {
      opcode: [Opcodes.JR_Z, 'REL_i8'],
      operands: ['rel'],
      aliases: ['jr z'],
      addressing: 'relative',
      distance: 'short',
      operation: ['${RESOLVE_ZF}', 'PC = (ZF == 1) ? PC + %{rel} : PC'],
      cycles: 12,
    },
    {
      opcode: [Opcodes.JR_NC, 'REL_i8'],
      operands: ['rel'],
      aliases: ['jr nc'],
      addressing: 'relative',
      distance: 'short',
      operation: ['${RESOLVE_CF}', 'PC = (CF == 0) ? PC + %{rel} : PC'],
      cycles: 12,
    },
    {
      opcode: [Opcodes.JR_C, 'REL_i8'],
      operands: ['rel'],
      aliases: ['jr c'],
      addressing: 'relative',
      distance: 'short',
      operation: ['${RESOLVE_CF}', 'PC = (CF == 1) ? PC + %{rel} : PC'],
      cycles: 12,
    },
  ],
};
