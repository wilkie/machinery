import type { InstructionInfo } from '@machinery/core';

import { Opcodes, SystemOpcodes } from '../opcodes';

export const ltr: InstructionInfo = {
  identifier: 'ltr',
  name: 'Load Task Register',
  description:
    'The Task Register is loaded from the source register or memory location given by the operand. The loaded `TSS` is marked busy. A task switch operation does not occur.\n\n`LTR` appears only in operating systems software. It is not used in applications programs.',
  modifies: [],
  undefined: [],
  macros: {
    OP: [
      '${RESOLVE_FLAGS}',
      ';; CPL must be 0',
      '#GP if CS.RPL != 0',
      'ERROR_CODE = tmp',
      ';; extract GDT index from selector',
      'index = tmp >> 3',
      ';; selector must not be null',
      '#GP if index == 0',
      ';; selector must be within GDT limits',
      '#GP if (index * 8) > GDTR.limit',
      ';; must be a system descriptor (S=0)',
      '#GP if RAM.GDT.gates[index].SD.S != 0',
      ';; must be an available TSS (A=1, type=0b000)',
      '#GP if RAM.GDT.gates[index].SD.A != 1',
      '#GP if RAM.GDT.gates[index].SD.type != 0b000',
      ';; must be present',
      '#NP if RAM.GDT.gates[index].SD.P != 1',
      ';; load task register and cache base/limit',
      'TR.selector = tmp',
      'TR.base = RAM.GDT.gates[index].SD.base',
      'TR.limit = RAM.GDT.gates[index].SD.limit',
      ';; mark TSS as busy (type = 0b001)',
      'RAM.GDT.gates[index].SD.type = 0b001',
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
    // 0x0F 0x00 /3 - LTR ew
    {
      modes: {
        real: {
          operation: ['#6'],
        },
        protected: {
          operation: [
            'offset = %{DISP:u16}',
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
        'ModRM_110_011_00',
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
        'ModRM_rm_011_00',
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
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
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
        'ModRM_rm_011_01',
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
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
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
        'ModRM_rm_011_10',
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
        'ModRM_rm16_011_11',
      ],
      operands: ['rm'],
      operandSize: 16,
      cycles: 17,
    },
  ],
};
