import type { InstructionInfo } from '@machinery/core';

import { Opcodes, SystemOpcodes } from '../opcodes';

export const sidt: InstructionInfo = {
  identifier: 'sidt',
  name: 'Store Interrupt Descriptor Table Register',
  description:
    'The contents of the descriptor table register are copied to six bytes of memory indicated by the operand. The `LIMIT` field of the register goes to the first word at the effective address; the next three bytes get the `BASE` field of the register; and the last byte is undefined.\n\n`SGDT` and `SIDT` appear only in operating systems software; they are not used in applications programs.',
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
    // 0x0F 0x01 /1 - SIDT m
    {
      opcode: [
        Opcodes.SYSTEM,
        SystemOpcodes.SGDT_SIDT_LMSW_SMSW_LGDT_LIDT,
        'ModRM_110_001_00',
        'DISP_i16',
      ],
      operands: ['rm'],
      modes: {
        real: {
          operation: [
            'offset = %{DISP:u16}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '#GP if offset == 0xffff',
            '#GP if (offset + 2) == 0xffff',
            '#GP if (offset + 4) == 0xffff',
            'RAM:u16[effective_address] = IDTR.limit',
            'RAM:u16[effective_address + 2] = IDTR.base',
            'RAM:u8[effective_address + 4] = IDTR.base >> 16',
            'RAM:u8[effective_address + 5] = 0xFF',
          ],
        },
        protected: {
          operation: [
            'offset = %{DISP:u16}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '#GP if (offset + 5) < ${MOD_RM_SEGMENT_LIMIT_MIN}',
            '#GP if (offset + 5) > ${MOD_RM_SEGMENT_LIMIT_MAX}',
            'RAM:u16[effective_address] = IDTR.limit',
            'RAM:u16[effective_address + 2] = IDTR.base',
            'RAM:u8[effective_address + 4] = IDTR.base >> 16',
            'RAM:u8[effective_address + 5] = 0xFF',
          ],
        },
      },
      cycles: 12,
    },
    {
      opcode: [
        Opcodes.SYSTEM,
        SystemOpcodes.SGDT_SIDT_LMSW_SMSW_LGDT_LIDT,
        'ModRM_rm_001_00',
      ],
      operands: ['rm'],
      modes: {
        real: {
          operation: [
            'offset = ${MOD_RM_OFFSET}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '#GP if offset == 0xffff',
            '#GP if (offset + 2) == 0xffff',
            '#GP if (offset + 4) == 0xffff',
            'RAM:u16[effective_address] = IDTR.limit',
            'RAM:u16[effective_address + 2] = IDTR.base',
            'RAM:u8[effective_address + 4] = IDTR.base >> 16',
            'RAM:u8[effective_address + 5] = 0xFF',
          ],
        },
        protected: {
          operation: [
            'offset = ${MOD_RM_OFFSET}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '#GP if (offset + 5) < ${MOD_RM_SEGMENT_LIMIT_MIN}',
            '#GP if (offset + 5) > ${MOD_RM_SEGMENT_LIMIT_MAX}',
            'RAM:u16[effective_address] = IDTR.limit',
            'RAM:u16[effective_address + 2] = IDTR.base',
            'RAM:u8[effective_address + 4] = IDTR.base >> 16',
            'RAM:u8[effective_address + 5] = 0xFF',
          ],
        },
      },
      cycles: 12,
    },
    {
      opcode: [
        Opcodes.SYSTEM,
        SystemOpcodes.SGDT_SIDT_LMSW_SMSW_LGDT_LIDT,
        'ModRM_rm_001_01',
        'DISP_i8',
      ],
      operands: ['rm'],
      modes: {
        real: {
          operation: [
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '#GP if offset == 0xffff',
            '#GP if (offset + 2) == 0xffff',
            '#GP if (offset + 4) == 0xffff',
            'RAM:u16[effective_address] = IDTR.limit',
            'RAM:u16[effective_address + 2] = IDTR.base',
            'RAM:u8[effective_address + 4] = IDTR.base >> 16',
            'RAM:u8[effective_address + 5] = 0xFF',
          ],
        },
        protected: {
          operation: [
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '#GP if (offset + 5) < ${MOD_RM_SEGMENT_LIMIT_MIN}',
            '#GP if (offset + 5) > ${MOD_RM_SEGMENT_LIMIT_MAX}',
            'RAM:u16[effective_address] = IDTR.limit',
            'RAM:u16[effective_address + 2] = IDTR.base',
            'RAM:u8[effective_address + 4] = IDTR.base >> 16',
            'RAM:u8[effective_address + 5] = 0xFF',
          ],
        },
      },
      cycles: 12,
    },
    {
      opcode: [
        Opcodes.SYSTEM,
        SystemOpcodes.SGDT_SIDT_LMSW_SMSW_LGDT_LIDT,
        'ModRM_rm_001_10',
        'DISP_i16',
      ],
      operands: ['rm'],
      modes: {
        real: {
          operation: [
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '#GP if offset == 0xffff',
            '#GP if (offset + 2) == 0xffff',
            '#GP if (offset + 4) == 0xffff',
            'RAM:u16[effective_address] = IDTR.limit',
            'RAM:u16[effective_address + 2] = IDTR.base',
            'RAM:u8[effective_address + 4] = IDTR.base >> 16',
            'RAM:u8[effective_address + 5] = 0xFF',
          ],
        },
        protected: {
          operation: [
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '#GP if (offset + 5) < ${MOD_RM_SEGMENT_LIMIT_MIN}',
            '#GP if (offset + 5) > ${MOD_RM_SEGMENT_LIMIT_MAX}',
            'RAM:u16[effective_address] = IDTR.limit',
            'RAM:u16[effective_address + 2] = IDTR.base',
            'RAM:u8[effective_address + 4] = IDTR.base >> 16',
            'RAM:u8[effective_address + 5] = 0xFF',
          ],
        },
      },
      cycles: 12,
    },
    {
      opcode: [
        Opcodes.SYSTEM,
        SystemOpcodes.SGDT_SIDT_LMSW_SMSW_LGDT_LIDT,
        'ModRM_rm16_001_11',
      ],
      operands: ['rm'],
      operation: [
        // Raise #UD (register destination not allowed)
        '#6',
      ],
      cycles: 12,
    },
  ],
};
