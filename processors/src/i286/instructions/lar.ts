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
      'ZF = 0',
      'rpl = tmp & 0x0003',
      '${RESOLVE_DESCRIPTOR}',
      'if desc_valid == 1',
      [
        ';; must be code/data segment (S=1) or valid system type',
        'valid = 0',
        'if desc_s == 1',
        ['valid = 1'],
        'end if',
        ';; TSS available: S=0, A=1, type=0b000',
        'if desc_s == 0 && desc_a == 1 && desc_type == 0b000',
        ['valid = 1'],
        'end if',
        ';; LDT: S=0, A=0, type=0b001',
        'if desc_s == 0 && desc_a == 0 && desc_type == 0b001',
        ['valid = 1'],
        'end if',
        ';; TSS busy: S=0, A=1, type=0b001',
        'if desc_s == 0 && desc_a == 1 && desc_type == 0b001',
        ['valid = 1'],
        'end if',
        ';; call gate: S=0, A=0, type=0b010',
        'if desc_s == 0 && desc_a == 0 && desc_type == 0b010',
        ['valid = 1'],
        'end if',
        ';; task gate: S=0, A=1, type=0b010',
        'if desc_s == 0 && desc_a == 1 && desc_type == 0b010',
        ['valid = 1'],
        'end if',
        'if valid == 1',
        [
          ';; conforming code: S=1, type bit 2 set, type bit 0 set — no DPL check',
          'if desc_s == 1 && (desc_type & 0b100) == 0b100 && (desc_type & 0b001) == 0b001',
          [
            'ar = (desc_a | (desc_type << 1) | (desc_s << 4) | (desc_dpl << 5) | (desc_p << 7)) << 8',
            '${MOD_RM_REG16} = ar',
            'ZF = 1',
          ],
          'end if',
          'if desc_s != 1 || (desc_type & 0b100) != 0b100 || (desc_type & 0b001) != 0b001',
          [
            'if desc_dpl >= CS.RPL && desc_dpl >= rpl',
            [
              'ar = (desc_a | (desc_type << 1) | (desc_s << 4) | (desc_dpl << 5) | (desc_p << 7)) << 8',
              '${MOD_RM_REG16} = ar',
              'ZF = 1',
            ],
            'end if',
          ],
          'end if',
        ],
        'end if',
      ],
      'end if',
    ],
  },
  locals: [
    {
      identifier: 'effective_address',
      name: 'Effective Address',
      size: 32,
    },
    {
      identifier: 'offset',
      name: 'Effective Offset',
      size: 32,
    },
    {
      identifier: 'tmp',
      name: 'Temporary Value',
      size: 16,
    },
    {
      identifier: 'index',
      name: 'Descriptor Table Index',
      size: 16,
    },
    {
      identifier: 'rpl',
      name: 'Requestor Privilege Level',
      size: 8,
    },
    {
      identifier: 'desc_type',
      name: 'Descriptor Type Field',
      size: 8,
    },
    {
      identifier: 'desc_s',
      name: 'Descriptor S Bit',
      size: 8,
    },
    {
      identifier: 'desc_a',
      name: 'Descriptor A Bit',
      size: 8,
    },
    {
      identifier: 'desc_dpl',
      name: 'Descriptor DPL',
      size: 8,
    },
    {
      identifier: 'desc_p',
      name: 'Descriptor P Bit',
      size: 8,
    },
    {
      identifier: 'desc_limit',
      name: 'Descriptor Limit',
      size: 16,
    },
    {
      identifier: 'desc_base',
      name: 'Descriptor Base',
      size: 32,
    },
    {
      identifier: 'desc_valid',
      name: 'Descriptor Valid Flag',
      size: 8,
    },
    {
      identifier: 'valid',
      name: 'Valid Type Flag',
      size: 8,
    },
    {
      identifier: 'ar',
      name: 'Access Rights Value',
      size: 16,
    },
  ],
  forms: [
    // 0x0F 0x02 /r - LAR rw, ew
    {
      modes: {
        real: {
          operation: [
            'offset = %{DISP:u16}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'tmp = RAM:u16[effective_address]',
            '${OP}',
          ],
        },
        protected: {
          operation: [
            'offset = %{DISP:u16}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'tmp = RAM:u16[effective_address]',
            '${OP}',
          ],
        },
      },
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
      modes: {
        real: {
          operation: [
            'offset = ${MOD_RM_OFFSET}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'tmp = RAM:u16[effective_address]',
            '${OP}',
          ],
        },
        protected: {
          operation: [
            'offset = ${MOD_RM_OFFSET}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'tmp = RAM:u16[effective_address]',
            '${OP}',
          ],
        },
      },
      opcode: [Opcodes.SYSTEM, SystemOpcodes.LAR, 'ModRM_rm_reg16_00'],
      operands: ['reg', 'rm'],
      operandSize: 16,
      cycles: 16,
    },
    {
      modes: {
        real: {
          operation: [
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'tmp = RAM:u16[effective_address]',
            '${OP}',
          ],
        },
        protected: {
          operation: [
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'tmp = RAM:u16[effective_address]',
            '${OP}',
          ],
        },
      },
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
      modes: {
        real: {
          operation: [
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'tmp = RAM:u16[effective_address]',
            '${OP}',
          ],
        },
        protected: {
          operation: [
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'tmp = RAM:u16[effective_address]',
            '${OP}',
          ],
        },
      },
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
      modes: {
        real: {
          operation: ['tmp = ${MOD_RM_RM16}', '${OP}'],
        },
        protected: {
          operation: ['tmp = ${MOD_RM_RM16}', '${OP}'],
        },
      },
      opcode: [Opcodes.SYSTEM, SystemOpcodes.LAR, 'ModRM_rm16_reg16_11'],
      operands: ['reg', 'rm'],
      operandSize: 16,
      cycles: 14,
    },
  ],
};
