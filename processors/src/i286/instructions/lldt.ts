import type { InstructionInfo } from '@machinery/core';

import { Opcodes, SystemOpcodes } from '../opcodes';

export const lldt: InstructionInfo = {
  identifier: 'lldt',
  name: 'Load Local Descriptor Table Register',
  description:
    'The word operand (memory or register) to `LLDT` should contain a selector pointing to the Global Descriptor Table. The GDT entry should be a Local Descriptor Table Descriptor. If so, then the Local Descriptor Table Register is loaded from the entry. The descriptor cache entries for `DS`, `ES`, `SS`, and `CS` are not affected. The `LDT` field in the TSS is not changed.\n\nThe selector operand is allowed to be zero. In that case, the Local Descriptor Table Register is marked invalid. All descriptor references (except by `LAR`, `VERR`, `VERW` or `LSL` instructions) will cause a `#GP` fault.\n\n`LLDT` appears in operating systems software; it does not appear in applications programs.',
  modifies: [],
  undefined: [],
  macros: {
    OP: [
      '${RESOLVE_FLAGS}',
      ';; CPL must be 0',
      '#GP if CS.RPL != 0',
      ';; extract GDT index from selector',
      'index = tmp >> 3',
      ';; null selector: mark LDTR as invalid',
      'LDTR.selector = tmp',
      'if index == 0',
      ['LDTR.base = 0', 'LDTR.limit = 0'],
      'end if',
      'if index != 0',
      [
        'ERROR_CODE = tmp',
        ';; selector must be within GDT limits',
        '#GP if (index * 8) > GDTR.limit',
        ';; must be a system descriptor (S=0)',
        '#GP if RAM.GDT.gates[index].SD.S != 0',
        ';; must be an LDT descriptor (A=0, type=0b001)',
        '#GP if RAM.GDT.gates[index].SD.A != 0',
        '#GP if RAM.GDT.gates[index].SD.type != 0b001',
        ';; must be present',
        '#NP if RAM.GDT.gates[index].SD.P != 1',
        ';; load LDTR and cache base/limit',
        'LDTR.base = RAM.GDT.gates[index].SD.base',
        'LDTR.limit = RAM.GDT.gates[index].SD.limit',
      ],
      'end if',
    ],
  },
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
      identifier: 'tmp',
      name: 'Temporary Value',
      size: 16,
    },
    {
      identifier: 'index',
      name: 'GDT entry index',
      size: 16,
    },
  ],
  forms: [
    // 0x0F 0x00 /2 - LLDT ew
    {
      modes: {
        real: {
          operation: ['#6'],
        },
        protected: {
          operation: [
            'offset = %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'tmp = RAM:u16[effective_address]',
            '${OP}',
          ],
        },
      },
      opcode: [
        Opcodes.SYSTEM,
        SystemOpcodes.LTR_STR_LLDT_SLDT_VERR_VERW,
        'ModRM_110_010_00',
        'DISP_i16',
      ],
      operands: ['rm'],
      operandSize: 16,
      cycles: 19,
    },
    {
      modes: {
        real: {
          operation: ['#6'],
        },
        protected: {
          operation: [
            'offset = ${MOD_RM_OFFSET}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'tmp = RAM:u16[effective_address]',
            '${OP}',
          ],
        },
      },
      opcode: [
        Opcodes.SYSTEM,
        SystemOpcodes.LTR_STR_LLDT_SLDT_VERR_VERW,
        'ModRM_rm_010_00',
      ],
      operands: ['rm'],
      operandSize: 16,
      cycles: 19,
    },
    {
      modes: {
        real: {
          operation: ['#6'],
        },
        protected: {
          operation: [
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'tmp = RAM:u16[effective_address]',
            '${OP}',
          ],
        },
      },
      opcode: [
        Opcodes.SYSTEM,
        SystemOpcodes.LTR_STR_LLDT_SLDT_VERR_VERW,
        'ModRM_rm_010_01',
        'DISP_i8',
      ],
      operands: ['rm'],
      operandSize: 16,
      cycles: 19,
    },
    {
      modes: {
        real: {
          operation: ['#6'],
        },
        protected: {
          operation: [
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'tmp = RAM:u16[effective_address]',
            '${OP}',
          ],
        },
      },
      opcode: [
        Opcodes.SYSTEM,
        SystemOpcodes.LTR_STR_LLDT_SLDT_VERR_VERW,
        'ModRM_rm_010_10',
        'DISP_i16',
      ],
      operands: ['rm'],
      operandSize: 16,
      cycles: 19,
    },
    {
      modes: {
        real: {
          operation: ['#6'],
        },
        protected: {
          operation: ['tmp = ${MOD_RM_RM16}', '${OP}'],
        },
      },
      opcode: [
        Opcodes.SYSTEM,
        SystemOpcodes.LTR_STR_LLDT_SLDT_VERR_VERW,
        'ModRM_rm16_010_11',
      ],
      operands: ['rm'],
      operandSize: 16,
      cycles: 17,
    },
  ],
};
