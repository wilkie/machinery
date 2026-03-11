import type { InstructionInfo } from '@machinery/core';

import { Opcodes, SystemOpcodes } from '../opcodes';

export const lar: InstructionInfo = {
  identifier: 'lar',
  name: 'Load Access Rights Byte',
  description:
    '`LAR` expects the second operand (memory or register word) to contain a selector. If the associated descriptor is visible at the current privilege level and at the selector `RPL`, then the access rights byte of the descriptor is loaded into the high byte of the first (register) operand, and the low byte is set to zero. The zero flag is set if the loading was performed (i.e., the selector index is within the table limit, descriptor `DPL` >= `CPL`, and descriptor `DPL` >= selector `RPL`); the zero flag is cleared otherwise.\n\nSelector operands cannot cause protection exceptions.',
  modifies: ['ZF'],
  undefined: [],
  macros: {
    OP: [
      '${RESOLVE_FLAGS}',
      // Raise #6 in real mode
      '#6',
      //"${MOD_RM_REG16} = ... read descriptor via selector in 'a' and get AR byte",
      //ZF = 1 if read else 0
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
    // 0x0F 0x02 /r - LAR rw, ew
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + %{DISP}',
        'tmp = RAM:u16[effective_address]',
        '${OP}',
      ],
      opcode: [
        Opcodes.SYSTEM,
        SystemOpcodes.LAR,
        'ModRM_110_reg16_00',
        'DISP_i16',
      ],
      operands: ['reg', 'rm'],
      operandSize: 16,
      cycles: 16,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET}',
        'tmp = RAM:u16[effective_address]',
        '${OP}',
      ],
      opcode: [Opcodes.SYSTEM, SystemOpcodes.LAR, 'ModRM_rm_reg16_00'],
      operands: ['reg', 'rm'],
      operandSize: 16,
      cycles: 16,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'tmp = RAM:u16[effective_address]',
        '${OP}',
      ],
      opcode: [
        Opcodes.SYSTEM,
        SystemOpcodes.LAR,
        'ModRM_rm_reg16_01',
        'DISP_i8',
      ],
      operands: ['reg', 'rm'],
      operandSize: 16,
      cycles: 16,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'tmp = RAM:u16[effective_address]',
        '${OP}',
      ],
      opcode: [
        Opcodes.SYSTEM,
        SystemOpcodes.LAR,
        'ModRM_rm_reg16_10',
        'DISP_i16',
      ],
      operands: ['reg', 'rm'],
      operandSize: 16,
      cycles: 16,
    },
    {
      operation: ['tmp = ${MOD_RM_RM16}', '${OP}'],
      opcode: [Opcodes.SYSTEM, SystemOpcodes.LAR, 'ModRM_rm16_reg16_11'],
      operands: ['reg', 'rm'],
      operandSize: 16,
      cycles: 14,
    },
  ],
};
