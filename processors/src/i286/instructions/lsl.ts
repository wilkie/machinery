import type { InstructionInfo } from '@machinery/core';

import { Opcodes, SystemOpcodes } from '../opcodes';

export const lsl: InstructionInfo = {
  identifier: 'lsl',
  name: 'Load Segment Limit',
  description:
    "If the descriptor denoted by the selector in the second (memory or register) operand is visible at the `CPL`, a word that consists of the limit field of the descriptor is loaded into the left operand, which must be a register. The value is the limit field for that segment. The zero flag is set if the loading was performed (that is, if the selector is non-null, the selector index is within the descriptor table limits, the descriptor is a non-conforming segment descriptor with `DPL` >= CPL, and the descriptor `DPL` >= selector `RPL`); the zero flag is cleared otherwise.\n\nThe `LSL` instruction returns only the limit field of segments, task state segments, and local descriptor tables. The interpretation of the limit value depends on the type of segment.\n\nThe selector operand's value cannot result in a protection exception.",
  modifies: ['ZF'],
  undefined: [],
  macros: {
    OP: [
      '${RESOLVE_FLAGS}',
      // Raise #6 in real mode
      '#6',
      //"${MOD_RM_REG16} = ... read descriptor via selector in 'a' and get limit field",
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
    // 0x0F 0x03 /r - LSL rw, ew
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + %{DISP}',
        'tmp = RAM:u16[effective_address]',
        '${OP}',
      ],
      opcode: [
        Opcodes.SYSTEM,
        SystemOpcodes.LSL,
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
      opcode: [Opcodes.SYSTEM, SystemOpcodes.LSL, 'ModRM_rm_reg16_00'],
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
        SystemOpcodes.LSL,
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
        SystemOpcodes.LSL,
        'ModRM_rm_reg16_10',
        'DISP_i16',
      ],
      operands: ['reg', 'rm'],
      operandSize: 16,
      cycles: 16,
    },
    {
      operation: ['tmp = ${MOD_RM_RM16}', '${OP}'],
      opcode: [Opcodes.SYSTEM, SystemOpcodes.LSL, 'ModRM_rm16_reg16_11'],
      operands: ['reg', 'rm'],
      operandSize: 16,
      cycles: 14,
    },
  ],
};
