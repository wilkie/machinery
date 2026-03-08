import type { InstructionInfo } from '@rawrs/architecture/Target';

import { Opcodes, SystemOpcodes } from '../opcodes';

export const sldt: InstructionInfo = {
  identifier: 'sldt',
  name: 'Store Local Descriptor Table Register',
  description:
    'The Local Descriptor Table register is stored in the 2-byte register or memory location indicated by the effective address operand. This register is a selector that points into the Global Descriptor Table.\n\n`SLDT` appears only in operating systems software. It is not used in applications programs.',
  modifies: [],
  undefined: [],
  forms: [
    // 0x0F 0x00 /0 - SLDT ew
    {
      opcode: [
        Opcodes.SYSTEM,
        SystemOpcodes.LTR_STR_LLDT_SLDT_VERR_VERW,
        'ModRM_110_000_00',
        'DISP_i16',
      ],
      operation: [
        // Raise #6 in real mode
        '#6',
      ],
      cycles: 3,
    },
    {
      opcode: [
        Opcodes.SYSTEM,
        SystemOpcodes.LTR_STR_LLDT_SLDT_VERR_VERW,
        'ModRM_rm_000_00',
      ],
      operation: [
        // Raise #6 in real mode
        '#6',
      ],
      cycles: 3,
    },
    {
      opcode: [
        Opcodes.SYSTEM,
        SystemOpcodes.LTR_STR_LLDT_SLDT_VERR_VERW,
        'ModRM_rm_000_01',
        'DISP_i8',
      ],
      operation: [
        // Raise #6 in real mode
        '#6',
      ],
      cycles: 3,
    },
    {
      opcode: [
        Opcodes.SYSTEM,
        SystemOpcodes.LTR_STR_LLDT_SLDT_VERR_VERW,
        'ModRM_rm_000_10',
        'DISP_i16',
      ],
      operation: [
        // Raise #6 in real mode
        '#6',
      ],
      cycles: 3,
    },
    {
      opcode: [
        Opcodes.SYSTEM,
        SystemOpcodes.LTR_STR_LLDT_SLDT_VERR_VERW,
        'ModRM_rm_000_11',
      ],
      operation: [
        // Raise #6 in real mode
        '#6',
      ],
      cycles: 2,
    },
  ],
};
