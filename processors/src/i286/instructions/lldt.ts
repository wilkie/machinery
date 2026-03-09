import type { InstructionInfo } from '@machinery/core';

import { Opcodes, SystemOpcodes } from '../opcodes';

export const lldt: InstructionInfo = {
  identifier: 'lldt',
  name: 'Load Local Descriptor Table Register',
  description:
    'The word operand (memory or register) to `LLDT` should contain a selector pointing to the Global Descriptor Table. The GDT entry should be a Local Descriptor Table Descriptor. If so, then the Local Descriptor Table Register is loaded from the entry. The descriptor cache entries for `DS`, `ES`, `SS`, and `CS` are not affected. The `LDT` field in the TSS is not changed.\n\nThe selector operand.is allowed to be zero. In that case, the Local Descriptor Table Register is marked invalid. All descriptor references (except by `LAR`, `VERR`, `VERW` or `LSL` instructions) will cause a `#GP` fault.\n\n`LLDT` appears in operating systems software; it does not appear in applications programs.',
  modifies: [],
  undefined: [],
  forms: [
    // 0x0F 0x00 /2 - LLDT ew
    {
      opcode: [
        Opcodes.SYSTEM,
        SystemOpcodes.LTR_STR_LLDT_SLDT_VERR_VERW,
        'ModRM_110_010_00',
        'DISP_i16',
      ],
      operands: ['rm'],
      operation: [
        // Raise #6 in real mode
        '#6',
      ],
      cycles: 19,
    },
    {
      opcode: [
        Opcodes.SYSTEM,
        SystemOpcodes.LTR_STR_LLDT_SLDT_VERR_VERW,
        'ModRM_rm_010_00',
      ],
      operands: ['rm'],
      operation: [
        // Raise #6 in real mode
        '#6',
      ],
      cycles: 19,
    },
    {
      opcode: [
        Opcodes.SYSTEM,
        SystemOpcodes.LTR_STR_LLDT_SLDT_VERR_VERW,
        'ModRM_rm_010_01',
        'DISP_i8',
      ],
      operands: ['rm'],
      operation: [
        // Raise #6 in real mode
        '#6',
      ],
      cycles: 19,
    },
    {
      opcode: [
        Opcodes.SYSTEM,
        SystemOpcodes.LTR_STR_LLDT_SLDT_VERR_VERW,
        'ModRM_rm_010_10',
        'DISP_i16',
      ],
      operands: ['rm'],
      operation: [
        // Raise #6 in real mode
        '#6',
      ],
      cycles: 19,
    },
    {
      opcode: [
        Opcodes.SYSTEM,
        SystemOpcodes.LTR_STR_LLDT_SLDT_VERR_VERW,
        'ModRM_rm16_010_11',
      ],
      operands: ['rm'],
      operation: [
        // Raise #6 in real mode
        '#6',
      ],
      cycles: 17,
    },
  ],
};
