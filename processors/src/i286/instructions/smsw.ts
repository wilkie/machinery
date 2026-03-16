import type { InstructionInfo } from '@machinery/core';

import { Opcodes, SystemOpcodes } from '../opcodes';

// TODO: #13 for operand 0xffff
export const smsw: InstructionInfo = {
  identifier: 'smsw',
  name: 'Store Machine Status Word',
  description:
    'The Machine Status Word (`MSW` or referred to as `CR0` in modern systems) is stored in the 2-byte register or memory location indicated by the effective address operand.',
  modifies: [],
  undefined: [],
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
  ],
  forms: [
    // 0x0F 0x01 /4 - SMSW ew
    {
      modes: {
        real: {
          operation: [
            'offset = %{DISP:u16}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'RAM:u16[effective_address] = MSW',
          ],
        },
        protected: {
          operation: [
            'offset = %{DISP:u16}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'RAM:u16[effective_address] = MSW',
          ],
        },
      },
      opcode: [
        Opcodes.SYSTEM,
        SystemOpcodes.SGDT_SIDT_LMSW_SMSW_LGDT_LIDT,
        'ModRM_110_100_00',
        'DISP_i16',
      ],
      operands: ['rm'],
      operandSize: 16,
      cycles: 3,
    },
    {
      modes: {
        real: {
          operation: [
            'offset = ${MOD_RM_OFFSET}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'RAM:u16[effective_address] = MSW',
          ],
        },
        protected: {
          operation: [
            'offset = ${MOD_RM_OFFSET}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'RAM:u16[effective_address] = MSW',
          ],
        },
      },
      opcode: [
        Opcodes.SYSTEM,
        SystemOpcodes.SGDT_SIDT_LMSW_SMSW_LGDT_LIDT,
        'ModRM_rm_100_00',
      ],
      operands: ['rm'],
      operandSize: 16,
      cycles: 3,
    },
    {
      modes: {
        real: {
          operation: [
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'RAM:u16[effective_address] = MSW',
          ],
        },
        protected: {
          operation: [
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'RAM:u16[effective_address] = MSW',
          ],
        },
      },
      opcode: [
        Opcodes.SYSTEM,
        SystemOpcodes.SGDT_SIDT_LMSW_SMSW_LGDT_LIDT,
        'ModRM_rm_100_01',
        'DISP_i8',
      ],
      operands: ['rm'],
      operandSize: 16,
      cycles: 3,
    },
    {
      modes: {
        real: {
          operation: [
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'RAM:u16[effective_address] = MSW',
          ],
        },
        protected: {
          operation: [
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'RAM:u16[effective_address] = MSW',
          ],
        },
      },
      opcode: [
        Opcodes.SYSTEM,
        SystemOpcodes.SGDT_SIDT_LMSW_SMSW_LGDT_LIDT,
        'ModRM_rm_100_10',
        'DISP_i16',
      ],
      operands: ['rm'],
      operandSize: 16,
      cycles: 3,
    },
    {
      operation: ['${MOD_RM_RM16} = MSW', 'RAM:u16[effective_address] = MSW'],
      opcode: [
        Opcodes.SYSTEM,
        SystemOpcodes.SGDT_SIDT_LMSW_SMSW_LGDT_LIDT,
        'ModRM_rm16_100_11',
      ],
      operands: ['rm'],
      operandSize: 16,
      cycles: 2,
    },
  ],
};
