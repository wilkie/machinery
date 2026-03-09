import type { InstructionInfo } from '@machinery/core';

import { Opcodes, SystemOpcodes } from '../opcodes';

// TODO: protected mode
export const ltr: InstructionInfo = {
  identifier: 'ltr',
  name: 'Load Task Register',
  description:
    'The Task Register is loaded from the source register or memory location given by the operand. The loaded `TSS` is marked busy. A task switch operation does not occur.\n\n`LTR` appears only in operating systems software. It is not used in applications programs.',
  modifies: [],
  undefined: [],
  macros: {
    OP: [
      '${RESOLVE_FLAGS}',
      // Raise #6 in real mode
      '#6',
      // TR = tmp (protected mode)
    ],
  },
  locals: [
    {
      identifier: 'effective_address',
      name: 'Effective Address',
      size: 32,
    },
    {
      identifier: 'tmp',
      name: 'Temporary Value',
      size: 16,
    },
  ],
  forms: [
    // 0x0F 0x00 /3 - LTR ew
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + %{DISP}',
        'tmp = RAM:u16[effective_address]',
        '${OP}',
      ],
      opcode: [
        Opcodes.SYSTEM,
        SystemOpcodes.LTR_STR_LLDT_SLDT_VERR_VERW,
        'ModRM_110_011_00',
        'DISP_i16',
      ],
      operands: ['rm'],
      operandSize: 16,
      cycles: 19,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET}',
        'tmp = RAM:u16[effective_address]',
        '${OP}',
      ],
      opcode: [
        Opcodes.SYSTEM,
        SystemOpcodes.LTR_STR_LLDT_SLDT_VERR_VERW,
        'ModRM_rm_011_00',
      ],
      operands: ['rm'],
      operandSize: 16,
      cycles: 19,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'tmp = RAM:u16[effective_address]',
        '${OP}',
      ],
      opcode: [
        Opcodes.SYSTEM,
        SystemOpcodes.LTR_STR_LLDT_SLDT_VERR_VERW,
        'ModRM_rm_011_01',
        'DISP_i8',
      ],
      operands: ['rm'],
      operandSize: 16,
      cycles: 19,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'tmp = RAM:u16[effective_address]',
        '${OP}',
      ],
      opcode: [
        Opcodes.SYSTEM,
        SystemOpcodes.LTR_STR_LLDT_SLDT_VERR_VERW,
        'ModRM_rm_011_10',
        'DISP_i16',
      ],
      operands: ['rm'],
      operandSize: 16,
      cycles: 19,
    },
    {
      operation: ['tmp = ${MOD_RM_RM16}', '${OP}'],
      opcode: [
        Opcodes.SYSTEM,
        SystemOpcodes.LTR_STR_LLDT_SLDT_VERR_VERW,
        'ModRM_rm16_011_11',
      ],
      operands: ['rm'],
      operandSize: 16,
      cycles: 17,
    },
  ],
};
