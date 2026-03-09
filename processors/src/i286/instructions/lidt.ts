import type { InstructionInfo } from '@machinery/core';

import { Opcodes, SystemOpcodes } from '../opcodes';

// TODO: allow in real mode since they are used in the initialization of protected mode
export const lidt: InstructionInfo = {
  identifier: 'lidt',
  name: 'Load Interrupt Descriptor Table Register',
  description:
    'The Global or the Interrupt Descriptor Table Register is loaded from the six bytes of memory pointed to by the effective address operand. The `LIMIT` field of the descriptor table register loads from the first word; the next three bytes go to the `BASE` field of the register; the last byte is ignored.\n\n`LGDT` and `LIDT` appear in operating systems software; they are not used in application programs.\n\nThese are the only instructions that directly load a physical memory address in 286 protected mode.',
  modifies: [],
  undefined: [],
  forms: [
    // 0x0F 0x01 /3 - LIDT ew
    {
      opcode: [
        Opcodes.SYSTEM,
        SystemOpcodes.SGDT_SIDT_LMSW_SMSW_LGDT_LIDT,
        'ModRM_110_011_00',
        'DISP_i16',
      ],
      operands: ['rm'],
      operation: [
        // Raise #6 in real mode
        '#6',
      ],
      cycles: 12,
    },
    {
      opcode: [
        Opcodes.SYSTEM,
        SystemOpcodes.SGDT_SIDT_LMSW_SMSW_LGDT_LIDT,
        'ModRM_rm_011_00',
      ],
      operands: ['rm'],
      operation: [
        // Raise #6 in real mode
        '#6',
      ],
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
      operation: [
        // Raise #6 in real mode
        '#6',
      ],
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
      operation: [
        // Raise #6 in real mode
        '#6',
      ],
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
