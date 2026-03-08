import type { InstructionInfo } from '@rawrs/architecture/Target';

import { Opcodes, SystemOpcodes } from '../opcodes';

// TODO: allow in real mode since they are used in the initialization of protected mode
export const lgdt: InstructionInfo = {
  identifier: 'lgdt',
  name: 'Load Global Descriptor Table Register',
  description:
    'The Global or the Interrupt Descriptor Table Register is loaded from the six bytes of memory pointed to by the effective address operand. The `LIMIT` field of the descriptor table register loads from the first word; the next three bytes go to the `BASE` field of the register; the last byte is ignored.\n\n`LGDT` and `LIDT` appear in operating systems software; they are not used in application programs.\n\nThese are the only instructions that directly load a physical memory address in 286 protected mode.',
  modifies: [],
  undefined: [],
  forms: [
    // 0x0F 0x01 /2 - LGDT ew
    {
      opcode: [
        Opcodes.SYSTEM,
        SystemOpcodes.SGDT_SIDT_LMSW_SMSW_LGDT_LIDT,
        'ModRM_110_010_00',
        'DISP_i16',
      ],
      operation: [
        // Raise #6 in real mode
        '#6',
      ],
      cycles: 11,
    },
    {
      opcode: [
        Opcodes.SYSTEM,
        SystemOpcodes.SGDT_SIDT_LMSW_SMSW_LGDT_LIDT,
        'ModRM_rm_010_00',
      ],
      operation: [
        // Raise #6 in real mode
        '#6',
      ],
      cycles: 11,
    },
    {
      opcode: [
        Opcodes.SYSTEM,
        SystemOpcodes.SGDT_SIDT_LMSW_SMSW_LGDT_LIDT,
        'ModRM_rm_010_01',
        'DISP_i8',
      ],
      operation: [
        // Raise #6 in real mode
        '#6',
      ],
      cycles: 11,
    },
    {
      opcode: [
        Opcodes.SYSTEM,
        SystemOpcodes.SGDT_SIDT_LMSW_SMSW_LGDT_LIDT,
        'ModRM_rm_010_10',
        'DISP_i16',
      ],
      operation: [
        // Raise #6 in real mode
        '#6',
      ],
      cycles: 11,
    },
    {
      opcode: [
        Opcodes.SYSTEM,
        SystemOpcodes.SGDT_SIDT_LMSW_SMSW_LGDT_LIDT,
        'ModRM_rm_010_11',
      ],
      operation: [
        // Raise #UD (register source not allowed)
        '#6',
      ],
      cycles: 11,
    },
  ],
};
