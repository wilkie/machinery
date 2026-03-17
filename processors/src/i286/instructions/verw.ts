import type { InstructionInfo } from '@machinery/core';

import { Opcodes, SystemOpcodes } from '../opcodes';

export const verw: InstructionInfo = {
  identifier: 'verw',
  name: 'Verify Segment for Writing',
  description:
    '`VERW` expects the 2-byte register or memory operand to contain the value of a selector. The instruction determine whether the segment denoted by the selector is reachable from the current privilege level; the instructions also determine whether it is writable. See `VERR` for the respective instruction that checks for readable selectors. If the segment is determined to be accessible, the zero flag is set to 1; if the segment is not accessible, it is set to 0. To set `ZF`, the following conditions must be met:\n\n1. The selector must denote a descriptor within the bounds of the table (`GDT` or `LDT`); that is, the selector must be "defined."\n2. The selector must denote the descriptor of a code or data segment.\n3. If the instruction is `VERR`, the segment must be readable. If the instruction is `VERW`, the segment must be a writable data segment.\n4. If the code segment is readable and conforming, the descriptor privilege level (`DPL`) can be any value for `VERR`. Otherwise, the `DPL` must be greater than or equal to (have less or the same privilege as) both the current privilege level and the selector\'s `RPL`.\n\nThe validation performed is the same as if the segment were loaded into `DS` or `ES` and the indicated access (read or write) were performed. The zero flag receives the result of the validation. The selector\'s value cannot result in a protection exception. This enables the software to anticipate possible segment access problems.',
  modifies: ['ZF'],
  undefined: [],
  macros: {
    OP: [
      '${RESOLVE_FLAGS}',
      'ZF = 0',
      'rpl = tmp & 0x0003',
      '${RESOLVE_DESCRIPTOR}',
      'if desc_valid == 1 && desc_s == 1',
      [
        ';; must be a writable data segment: type bit 2 = 0 (data), type bit 0 = 1 (writable)',
        'if (desc_type & 0b100) == 0 && (desc_type & 0b001) == 0b001',
        [
          ';; DPL >= CPL and DPL >= RPL',
          'if desc_dpl >= CS.RPL && desc_dpl >= rpl',
          ['ZF = 1'],
          'end if',
        ],
        'end if',
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
      name: 'Descriptor Table Index',
      size: 16,
    },
    {
      identifier: 'rpl',
      name: 'Requestor Privilege Level',
      size: 8,
    },
    {
      identifier: 'desc_type',
      name: 'Descriptor Type',
      size: 8,
    },
    {
      identifier: 'desc_s',
      name: 'Descriptor S Bit',
      size: 8,
    },
    {
      identifier: 'desc_a',
      name: 'Descriptor A Bit',
      size: 8,
    },
    {
      identifier: 'desc_dpl',
      name: 'Descriptor DPL',
      size: 8,
    },
    {
      identifier: 'desc_p',
      name: 'Descriptor P Bit',
      size: 8,
    },
    {
      identifier: 'desc_limit',
      name: 'Descriptor Limit',
      size: 16,
    },
    {
      identifier: 'desc_base',
      name: 'Descriptor Base',
      size: 32,
    },
    {
      identifier: 'desc_valid',
      name: 'Descriptor Valid Flag',
      size: 8,
    },
  ],
  forms: [
    // 0x0F 0x00 /5 - VERW ew
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
        'ModRM_110_101_00',
        'DISP_i16',
      ],
      operands: ['rm'],
      operandSize: 16,
      cycles: 14,
    },
    {
      modes: {
        real: {
          operation: ['#6'],
        },
        protected: {
          operation: [
            'offset = (${MOD_RM_OFFSET}):u16',
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
        'ModRM_rm_101_00',
      ],
      operands: ['rm'],
      operandSize: 16,
      cycles: 14,
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
        'ModRM_rm_101_01',
        'DISP_i8',
      ],
      operands: ['rm'],
      operandSize: 16,
      cycles: 14,
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
        'ModRM_rm_101_10',
        'DISP_i16',
      ],
      operands: ['rm'],
      operandSize: 16,
      cycles: 14,
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
        'ModRM_rm16_101_11',
      ],
      operands: ['rm'],
      operandSize: 16,
      cycles: 14,
    },
  ],
};
