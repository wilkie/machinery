import type { InstructionInfo } from '@machinery/core';

import { Opcodes, SystemOpcodes } from '../opcodes';

// TODO: #13 for operand 0xffff
export const lmsw: InstructionInfo = {
  identifier: 'lmsw',
  name: 'Load Machine Status Word',
  description:
    'The Machine Status Word (`LMSW` or referred to as `CR0` in modern systems) is loaded from the source operand. This instruction may be used to switch to protected mode. If so, then it must be followed by an intra-segment jump to flush the instruction queue. `LMSW` will not switch back to Real Address Mode.\n\n`LMSW` appears only in operating systems software. It does not appear in applications programs.',
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
    {
      identifier: 'value',
      name: 'Source Value',
      size: 16,
    },
  ],
  forms: [
    // 0x0F 0x01 /6 - LMSW ew
    {
      modes: {
        real: {
          operation: [
            'offset = %{DISP:u16}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'value = RAM:u16[effective_address]',
            ';; only low 4 bits are writable, PE cannot be cleared',
            'MSW = (MSW & 0xFFF0) | (value & 0x000F) | (MSW & 0x0001)',
            ';; switch to protected mode if PE was set',
            'if (MSW & 0x0001) == 1',
            ['@mode = @modes.protected'],
            'end if',
          ],
        },
        protected: {
          operation: [
            '#GP if CS.RPL != 0',
            'offset = %{DISP:u16}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'value = RAM:u16[effective_address]',
            ';; only low 4 bits are writable, PE is already 1 and cannot be cleared',
            'MSW = (MSW & 0xFFF0) | (value & 0x000F) | 0x0001',
          ],
        },
      },
      opcode: [
        Opcodes.SYSTEM,
        SystemOpcodes.SGDT_SIDT_LMSW_SMSW_LGDT_LIDT,
        'ModRM_110_110_00',
        'DISP_i16',
      ],
      operands: ['rm'],
      operandSize: 16,
      cycles: 6,
    },
    {
      modes: {
        real: {
          operation: [
            'offset = (${MOD_RM_OFFSET}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'value = RAM:u16[effective_address]',
            'MSW = (MSW & 0xFFF0) | (value & 0x000F) | (MSW & 0x0001)',
            'if (MSW & 0x0001) == 1',
            ['@mode = @modes.protected'],
            'end if',
          ],
        },
        protected: {
          operation: [
            '#GP if CS.RPL != 0',
            'offset = (${MOD_RM_OFFSET}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'value = RAM:u16[effective_address]',
            'MSW = (MSW & 0xFFF0) | (value & 0x000F) | 0x0001',
          ],
        },
      },
      opcode: [
        Opcodes.SYSTEM,
        SystemOpcodes.SGDT_SIDT_LMSW_SMSW_LGDT_LIDT,
        'ModRM_rm_110_00',
      ],
      operands: ['rm'],
      operandSize: 16,
      cycles: 6,
    },
    {
      modes: {
        real: {
          operation: [
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'value = RAM:u16[effective_address]',
            'MSW = (MSW & 0xFFF0) | (value & 0x000F) | (MSW & 0x0001)',
            'if (MSW & 0x0001) == 1',
            ['@mode = @modes.protected'],
            'end if',
          ],
        },
        protected: {
          operation: [
            '#GP if CS.RPL != 0',
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'value = RAM:u16[effective_address]',
            'MSW = (MSW & 0xFFF0) | (value & 0x000F) | 0x0001',
          ],
        },
      },
      opcode: [
        Opcodes.SYSTEM,
        SystemOpcodes.SGDT_SIDT_LMSW_SMSW_LGDT_LIDT,
        'ModRM_rm_110_01',
        'DISP_i8',
      ],
      operands: ['rm'],
      operandSize: 16,
      cycles: 6,
    },
    {
      modes: {
        real: {
          operation: [
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'value = RAM:u16[effective_address]',
            'MSW = (MSW & 0xFFF0) | (value & 0x000F) | (MSW & 0x0001)',
            'if (MSW & 0x0001) == 1',
            ['@mode = @modes.protected'],
            'end if',
          ],
        },
        protected: {
          operation: [
            '#GP if CS.RPL != 0',
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'value = RAM:u16[effective_address]',
            'MSW = (MSW & 0xFFF0) | (value & 0x000F) | 0x0001',
          ],
        },
      },
      opcode: [
        Opcodes.SYSTEM,
        SystemOpcodes.SGDT_SIDT_LMSW_SMSW_LGDT_LIDT,
        'ModRM_rm_110_10',
        'DISP_i16',
      ],
      operands: ['rm'],
      operandSize: 16,
      cycles: 6,
    },
    {
      modes: {
        real: {
          operation: [
            'value = ${MOD_RM_RM16}',
            'MSW = (MSW & 0xFFF0) | (value & 0x000F) | (MSW & 0x0001)',
            'if (MSW & 0x0001) == 1',
            ['@mode = @modes.protected'],
            'end if',
          ],
        },
        protected: {
          operation: [
            '#GP if CS.RPL != 0',
            'value = ${MOD_RM_RM16}',
            'MSW = (MSW & 0xFFF0) | (value & 0x000F) | 0x0001',
          ],
        },
      },
      opcode: [
        Opcodes.SYSTEM,
        SystemOpcodes.SGDT_SIDT_LMSW_SMSW_LGDT_LIDT,
        'ModRM_rm16_110_11',
      ],
      operands: ['rm'],
      operandSize: 16,
      cycles: 3,
    },
  ],
};
