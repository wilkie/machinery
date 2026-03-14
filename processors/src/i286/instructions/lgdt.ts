import type { InstructionInfo } from '@machinery/core';

import { Opcodes, SystemOpcodes } from '../opcodes';

export const lgdt: InstructionInfo = {
  identifier: 'lgdt',
  name: 'Load Global Descriptor Table Register',
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
    // 0x0F 0x01 /2 - LGDT m
    {
      opcode: [
        Opcodes.SYSTEM,
        SystemOpcodes.SGDT_SIDT_LMSW_SMSW_LGDT_LIDT,
        'ModRM_110_010_00',
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
            'GDTR.limit = RAM:u16[effective_address]',
            'base_lo = RAM:u16[effective_address + 2]',
            'base_hi = RAM:u8[effective_address + 4]',
            'GDTR.base = (base_hi << 16) | base_lo',
          ],
        },
        protected: {
          operation: [
            '#GP if CS.RPL != 0',
            'offset = %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '#GP if (offset + 5) < ${MOD_RM_SEGMENT_LIMIT_MIN}',
            '#GP if (offset + 5) > ${MOD_RM_SEGMENT_LIMIT_MAX}',
            'GDTR.limit = RAM:u16[effective_address]',
            'base_lo = RAM:u16[effective_address + 2]',
            'base_hi = RAM:u8[effective_address + 4]',
            'GDTR.base = (base_hi << 16) | base_lo',
          ],
        },
      },
      cycles: 11,
    },
    {
      opcode: [
        Opcodes.SYSTEM,
        SystemOpcodes.SGDT_SIDT_LMSW_SMSW_LGDT_LIDT,
        'ModRM_rm_010_00',
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
            'GDTR.limit = RAM:u16[effective_address]',
            'base_lo = RAM:u16[effective_address + 2]',
            'base_hi = RAM:u8[effective_address + 4]',
            'GDTR.base = (base_hi << 16) | base_lo',
          ],
        },
        protected: {
          operation: [
            '#GP if CS.RPL != 0',
            'offset = ${MOD_RM_OFFSET}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '#GP if (offset + 5) < ${MOD_RM_SEGMENT_LIMIT_MIN}',
            '#GP if (offset + 5) > ${MOD_RM_SEGMENT_LIMIT_MAX}',
            'GDTR.limit = RAM:u16[effective_address]',
            'base_lo = RAM:u16[effective_address + 2]',
            'base_hi = RAM:u8[effective_address + 4]',
            'GDTR.base = (base_hi << 16) | base_lo',
          ],
        },
      },
      cycles: 11,
    },
    {
      opcode: [
        Opcodes.SYSTEM,
        SystemOpcodes.SGDT_SIDT_LMSW_SMSW_LGDT_LIDT,
        'ModRM_rm_010_01',
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
            'GDTR.limit = RAM:u16[effective_address]',
            'base_lo = RAM:u16[effective_address + 2]',
            'base_hi = RAM:u8[effective_address + 4]',
            'GDTR.base = (base_hi << 16) | base_lo',
          ],
        },
        protected: {
          operation: [
            '#GP if CS.RPL != 0',
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '#GP if (offset + 5) < ${MOD_RM_SEGMENT_LIMIT_MIN}',
            '#GP if (offset + 5) > ${MOD_RM_SEGMENT_LIMIT_MAX}',
            'GDTR.limit = RAM:u16[effective_address]',
            'base_lo = RAM:u16[effective_address + 2]',
            'base_hi = RAM:u8[effective_address + 4]',
            'GDTR.base = (base_hi << 16) | base_lo',
          ],
        },
      },
      cycles: 11,
    },
    {
      opcode: [
        Opcodes.SYSTEM,
        SystemOpcodes.SGDT_SIDT_LMSW_SMSW_LGDT_LIDT,
        'ModRM_rm_010_10',
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
            'GDTR.limit = RAM:u16[effective_address]',
            'base_lo = RAM:u16[effective_address + 2]',
            'base_hi = RAM:u8[effective_address + 4]',
            'GDTR.base = (base_hi << 16) | base_lo',
          ],
        },
        protected: {
          operation: [
            '#GP if CS.RPL != 0',
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '#GP if (offset + 5) < ${MOD_RM_SEGMENT_LIMIT_MIN}',
            '#GP if (offset + 5) > ${MOD_RM_SEGMENT_LIMIT_MAX}',
            'GDTR.limit = RAM:u16[effective_address]',
            'base_lo = RAM:u16[effective_address + 2]',
            'base_hi = RAM:u8[effective_address + 4]',
            'GDTR.base = (base_hi << 16) | base_lo',
          ],
        },
      },
      cycles: 11,
    },
    {
      opcode: [
        Opcodes.SYSTEM,
        SystemOpcodes.SGDT_SIDT_LMSW_SMSW_LGDT_LIDT,
        'ModRM_rm16_010_11',
      ],
      operands: ['rm'],
      operation: [
        // Raise #UD (register source not allowed)
        '#6',
      ],
      cycles: 11,
    },
  ],
};
