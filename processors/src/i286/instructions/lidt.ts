import type { InstructionInfo } from '@machinery/core';

import { Opcodes, SystemOpcodes } from '../opcodes';

export const lidt: InstructionInfo = {
  identifier: 'lidt',
  name: 'Load Interrupt Descriptor Table Register',
  description:
    'The Global or the Interrupt Descriptor Table Register is loaded from the six bytes of memory pointed to by the effective address operand. The `LIMIT` field of the descriptor table register loads from the first word; the next three bytes go to the `BASE` field of the register; the last byte is ignored.\n\n`LGDT` and `LIDT` appear in operating systems software; they are not used in application programs.\n\nThese are the only instructions that directly load a physical memory address in 286 protected mode.',
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
      identifier: 'base_lo',
      name: 'Base Address Low Word',
      size: 16,
    },
    {
      identifier: 'base_hi',
      name: 'Base Address High Byte',
      size: 16,
    },
  ],
  forms: [
    // 0x0F 0x01 /3 - LIDT m
    {
      opcode: [
        Opcodes.SYSTEM,
        SystemOpcodes.SGDT_SIDT_LMSW_SMSW_LGDT_LIDT,
        'ModRM_110_011_00',
        'DISP_i16',
      ],
      operands: ['rm'],
      modes: {
        real: {
          operation: [
            'offset = %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '#GP if offset == 0xffff',
            '#GP if (offset + 2) == 0xffff',
            '#GP if (offset + 4) == 0xffff',
            'IDTR.limit = RAM:u16[effective_address]',
            'base_lo = RAM:u16[effective_address + 2]',
            'base_hi = RAM:u8[effective_address + 4]',
            'IDTR.base = (base_hi << 16) | base_lo',
          ],
        },
        // TODO: #GP if CPL != 0
        protected: {
          operation: [
            'offset = %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '#GP if (offset + 5) < ${MOD_RM_SEGMENT_LIMIT_MIN}',
            '#GP if (offset + 5) > ${MOD_RM_SEGMENT_LIMIT_MAX}',
            'IDTR.limit = RAM:u16[effective_address]',
            'base_lo = RAM:u16[effective_address + 2]',
            'base_hi = RAM:u8[effective_address + 4]',
            'IDTR.base = (base_hi << 16) | base_lo',
          ],
        },
      },
      cycles: 12,
    },
    {
      opcode: [
        Opcodes.SYSTEM,
        SystemOpcodes.SGDT_SIDT_LMSW_SMSW_LGDT_LIDT,
        'ModRM_rm_011_00',
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
            'IDTR.limit = RAM:u16[effective_address]',
            'base_lo = RAM:u16[effective_address + 2]',
            'base_hi = RAM:u8[effective_address + 4]',
            'IDTR.base = (base_hi << 16) | base_lo',
          ],
        },
        protected: {
          operation: [
            'offset = ${MOD_RM_OFFSET}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '#GP if (offset + 5) < ${MOD_RM_SEGMENT_LIMIT_MIN}',
            '#GP if (offset + 5) > ${MOD_RM_SEGMENT_LIMIT_MAX}',
            'IDTR.limit = RAM:u16[effective_address]',
            'base_lo = RAM:u16[effective_address + 2]',
            'base_hi = RAM:u8[effective_address + 4]',
            'IDTR.base = (base_hi << 16) | base_lo',
          ],
        },
      },
      cycles: 12,
    },
    {
      opcode: [
        Opcodes.SYSTEM,
        SystemOpcodes.SGDT_SIDT_LMSW_SMSW_LGDT_LIDT,
        'ModRM_rm_011_01',
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
            'IDTR.limit = RAM:u16[effective_address]',
            'base_lo = RAM:u16[effective_address + 2]',
            'base_hi = RAM:u8[effective_address + 4]',
            'IDTR.base = (base_hi << 16) | base_lo',
          ],
        },
        protected: {
          operation: [
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '#GP if (offset + 5) < ${MOD_RM_SEGMENT_LIMIT_MIN}',
            '#GP if (offset + 5) > ${MOD_RM_SEGMENT_LIMIT_MAX}',
            'IDTR.limit = RAM:u16[effective_address]',
            'base_lo = RAM:u16[effective_address + 2]',
            'base_hi = RAM:u8[effective_address + 4]',
            'IDTR.base = (base_hi << 16) | base_lo',
          ],
        },
      },
      cycles: 12,
    },
    {
      opcode: [
        Opcodes.SYSTEM,
        SystemOpcodes.SGDT_SIDT_LMSW_SMSW_LGDT_LIDT,
        'ModRM_rm_011_10',
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
            'IDTR.limit = RAM:u16[effective_address]',
            'base_lo = RAM:u16[effective_address + 2]',
            'base_hi = RAM:u8[effective_address + 4]',
            'IDTR.base = (base_hi << 16) | base_lo',
          ],
        },
        protected: {
          operation: [
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '#GP if (offset + 5) < ${MOD_RM_SEGMENT_LIMIT_MIN}',
            '#GP if (offset + 5) > ${MOD_RM_SEGMENT_LIMIT_MAX}',
            'IDTR.limit = RAM:u16[effective_address]',
            'base_lo = RAM:u16[effective_address + 2]',
            'base_hi = RAM:u8[effective_address + 4]',
            'IDTR.base = (base_hi << 16) | base_lo',
          ],
        },
      },
      cycles: 12,
    },
    {
      opcode: [
        Opcodes.SYSTEM,
        SystemOpcodes.SGDT_SIDT_LMSW_SMSW_LGDT_LIDT,
        'ModRM_rm16_011_11',
      ],
      operands: ['rm'],
      operation: [
        // Raise #UD (register source not allowed)
        '#6',
      ],
      cycles: 12,
    },
  ],
};
