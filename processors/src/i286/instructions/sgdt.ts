import type { InstructionInfo } from '@rawrs/architecture/Target';

import { Opcodes, SystemOpcodes } from '../opcodes';

// TODO: allow in real mode since they are used in the initialization of protected mode
export const sgdt: InstructionInfo = {
  identifier: 'sgdt',
  name: 'Store Global Descriptor Table Register',
  description:
    'The contents of the descriptor table register are copied to six bytes of memory indicated by the operand. The `LIMIT` field of the register goes to the first word at the effective address; the next three bytes get the `BASE` field of the register; and the last byte is undefined.\n\n`SGDT` and `SIDT` appear only in operating systems software; they are not used in applications programs.',
  modifies: [],
  undefined: [],
  forms: [
    // 0x0F 0x01 /0 - SGDT m
    {
      opcode: [
        Opcodes.SYSTEM,
        SystemOpcodes.SGDT_SIDT_LMSW_SMSW_LGDT_LIDT,
        'ModRM_110_000_00',
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
        'ModRM_rm_000_00',
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
        'ModRM_rm_000_01',
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
        'ModRM_rm_000_10',
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
        'ModRM_rm_000_11',
      ],
      operation: [
        // Raise #UD (register source not allowed)
        '#6',
      ],
      cycles: 11,
    },
  ],
};
