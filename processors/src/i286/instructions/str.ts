import type { InstructionInfo } from '@machinery/core';

import { Opcodes, SystemOpcodes } from '../opcodes';

// TODO: protected mode
export const str: InstructionInfo = {
  identifier: 'str',
  name: 'Store Task Register',
  description:
    'The contents of the Task Register are copied to the 2-byte register or memory location indicated by the effective address operand.',
  modifies: [],
  undefined: [],
  macros: {
    OP: [
      '${RESOLVE_FLAGS}',
      // Raise #6 in real mode
      '#6',
      // "memw(2, effective_address, TR)", // (protected mode)
    ],
    OP_REG: [
      '${RESOLVE_FLAGS}',
      // Raise #6 in real mode
      '#6',
      // "${MOD_RM_RM16} = TR", // (protected mode)
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
  ],
  forms: [
    // 0x0F 0x00 /1 - STR ew
    {
      modes: {
        real: {
          operation: [
            'offset = %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            '${OP}',
          ],
        },
        protected: {
          operation: [
            'offset = %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            '${OP}',
          ],
        },
      },
      opcode: [
        Opcodes.SYSTEM,
        SystemOpcodes.LTR_STR_LLDT_SLDT_VERR_VERW,
        'ModRM_110_001_00',
        'DISP_i16',
      ],
      operands: ['rm'],
      operandSize: 16,
      cycles: 3,
    },
    {
      modes: {
        real: {
          operation: [
            'offset = ${MOD_RM_OFFSET}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            '${OP}',
          ],
        },
        protected: {
          operation: [
            'offset = ${MOD_RM_OFFSET}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            '${OP}',
          ],
        },
      },
      opcode: [
        Opcodes.SYSTEM,
        SystemOpcodes.LTR_STR_LLDT_SLDT_VERR_VERW,
        'ModRM_rm_001_00',
      ],
      operands: ['rm'],
      operandSize: 16,
      cycles: 3,
    },
    {
      modes: {
        real: {
          operation: [
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            '${OP}',
          ],
        },
        protected: {
          operation: [
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            '${OP}',
          ],
        },
      },
      opcode: [
        Opcodes.SYSTEM,
        SystemOpcodes.LTR_STR_LLDT_SLDT_VERR_VERW,
        'ModRM_rm_001_01',
        'DISP_i8',
      ],
      operands: ['rm'],
      operandSize: 16,
      cycles: 3,
    },
    {
      modes: {
        real: {
          operation: [
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            '${OP}',
          ],
        },
        protected: {
          operation: [
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            '${OP}',
          ],
        },
      },
      opcode: [
        Opcodes.SYSTEM,
        SystemOpcodes.LTR_STR_LLDT_SLDT_VERR_VERW,
        'ModRM_rm_001_10',
        'DISP_i16',
      ],
      operands: ['rm'],
      operandSize: 16,
      cycles: 3,
    },
    {
      operation: ['${OP_REG}'],
      opcode: [
        Opcodes.SYSTEM,
        SystemOpcodes.LTR_STR_LLDT_SLDT_VERR_VERW,
        'ModRM_rm16_001_11',
      ],
      operands: ['rm'],
      operandSize: 16,
      cycles: 2,
    },
  ],
};
