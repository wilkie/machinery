import type { InstructionInfo } from '@machinery/core';

import { Opcodes, SystemOpcodes } from '../opcodes';

export const verr: InstructionInfo = {
  identifier: 'verr',
  name: 'Verify Segment for Reading',
  description:
    '`VERR` expects the 2-byte register or memory operand to contain the value of a selector. The instruction determine whether the segment denoted by the selector is reachable from the current privilege level; the instructions also determine whether it is readable. See `VERW` for the respective instruction that checks for writable data selectors. If the segment is determined to be accessible, the zero flag is set to 1; if the segment is not accessible, it is set to 0. To set `ZF`, the following conditions must be met:\n\n1. The selector must denote a descriptor within the bounds of the table (`GDT` or `LDT`); that is, the selector must be "defined."\n2. The selector must denote the descriptor of a code or data segment.\n3. If the instruction is `VERR`, the segment must be readable. If the instruction is `VERW`, the segment must be a writable data segment.\n4. If the code segment is readable and conforming, the descriptor privilege level (`DPL`) can be any value for `VERR`. Otherwise, the `DPL` must be greater than or equal to (have less or the same privilege as) both the current privilege level and the selector\'s `RPL`.\n\nThe validation performed is the same as if the segment were loaded into `DS` or `ES` and the indicated access (read or write) were performed. The zero flag receives the result of the validation. The selector\'s value cannot result in a protection exception. This enables the software to anticipate possible segment access problems.',
  modifies: ['ZF'],
  undefined: [],
  forms: [
    // 0x0F 0x00 /4 - VERR ew
    {
      opcode: [
        Opcodes.SYSTEM,
        SystemOpcodes.LTR_STR_LLDT_SLDT_VERR_VERW,
        'ModRM_110_100_00',
        'DISP_i16',
      ],
      operands: ['rm'],
      operation: [
        // Raise #6 in real mode
        '#6',
      ],
      cycles: 14,
    },
    {
      opcode: [
        Opcodes.SYSTEM,
        SystemOpcodes.LTR_STR_LLDT_SLDT_VERR_VERW,
        'ModRM_rm_100_00',
      ],
      operands: ['rm'],
      operation: [
        // Raise #6 in real mode
        '#6',
      ],
      cycles: 14,
    },
    {
      opcode: [
        Opcodes.SYSTEM,
        SystemOpcodes.LTR_STR_LLDT_SLDT_VERR_VERW,
        'ModRM_rm_100_01',
        'DISP_i8',
      ],
      operands: ['rm'],
      operation: [
        // Raise #6 in real mode
        '#6',
      ],
      cycles: 14,
    },
    {
      opcode: [
        Opcodes.SYSTEM,
        SystemOpcodes.LTR_STR_LLDT_SLDT_VERR_VERW,
        'ModRM_rm_100_10',
        'DISP_i16',
      ],
      operands: ['rm'],
      operation: [
        // Raise #6 in real mode
        '#6',
      ],
      cycles: 14,
    },
    {
      opcode: [
        Opcodes.SYSTEM,
        SystemOpcodes.LTR_STR_LLDT_SLDT_VERR_VERW,
        'ModRM_rm16_100_11',
      ],
      operands: ['rm'],
      operation: [
        // Raise #6 in real mode
        '#6',
      ],
      cycles: 14,
    },
  ],
};
