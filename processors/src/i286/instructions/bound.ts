import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const bound: InstructionInfo = {
  identifier: 'bound',
  name: 'Check Array Index Against Bounds',
  description:
    '`BOUND` is used to ensure that a signed array index is within the limits defined by a two-word block of memory. The first operand (a register) must be greater than or equal to the first word in memory, and less than or equal to the second word in memory. If the register is not within the bounds, an INTERRUPT 5 occurs.\n\nThe two-word block might typically be found just before the array itself and therefore would be accessible at a constant offset of -4 from the array, simplifying the addressing.',
  modifies: [''],
  undefined: [],
  macros: {
    OP_REAL: [
      // Each word read must not cross the segment boundary (offset 0xFFFF)
      '#GP if offset == 0xFFFF',
      '#GP if (offset + 2):u16 == 0xFFFF',
      'lower_bound = RAM:i16[effective_address]',
      // Upper bound address wraps at 16-bit segment boundary
      'effective_address = effective_address - offset + (offset + 2):u16',
      'upper_bound = RAM:i16[effective_address]',
      'tmp = ${MOD_RM_REG16:i16}',
      '#5 if tmp < lower_bound || tmp > upper_bound',
    ],
    OP_PROTECTED: [
      '#GP if (offset + 1) < ${MOD_RM_SEGMENT_LIMIT_MIN}',
      '#GP if (offset + 1) > ${MOD_RM_SEGMENT_LIMIT_MAX}',
      '#GP if (offset + 3) < ${MOD_RM_SEGMENT_LIMIT_MIN}',
      '#GP if (offset + 3) > ${MOD_RM_SEGMENT_LIMIT_MAX}',
      'lower_bound = RAM:i16[effective_address]',
      'upper_bound = RAM:i16[effective_address + 2]',
      'tmp = ${MOD_RM_REG16:i16}',
      '#5 if tmp < lower_bound || tmp > upper_bound',
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
      signed: true,
    },
    {
      identifier: 'lower_bound',
      name: 'Lower Bound Value for Test',
      size: 16,
      signed: true,
    },
    {
      identifier: 'upper_bound',
      name: 'Upper Bound Value for Test',
      size: 16,
      signed: true,
    },
  ],
  forms: [
    // 0x62 /r - BOUND rw, md
    {
      modes: {
        real: {
          operation: [
            'offset = %{DISP:u16}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${OP_REAL}',
          ],
        },
        protected: {
          operation: [
            'offset = %{DISP:u16}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${OP_PROTECTED}',
          ],
        },
      },
      opcode: [Opcodes.BOUND, 'ModRM_110_reg16_00', 'DISP_i16'],
      operands: ['reg', 'rm'],
      operandSize: 16,
      cycles: 13,
    },
    {
      modes: {
        real: {
          operation: [
            'offset = (${MOD_RM_OFFSET}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${OP_REAL}',
          ],
        },
        protected: {
          operation: [
            'offset = (${MOD_RM_OFFSET}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${OP_PROTECTED}',
          ],
        },
      },
      opcode: [Opcodes.BOUND, 'ModRM_rm_reg16_00'],
      operands: ['reg', 'rm'],
      operandSize: 16,
      cycles: 13,
    },
    {
      modes: {
        real: {
          operation: [
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${OP_REAL}',
          ],
        },
        protected: {
          operation: [
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${OP_PROTECTED}',
          ],
        },
      },
      opcode: [Opcodes.BOUND, 'ModRM_rm_reg16_01', 'DISP_i8'],
      operands: ['reg', 'rm'],
      operandSize: 16,
      cycles: 13,
    },
    {
      modes: {
        real: {
          operation: [
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${OP_REAL}',
          ],
        },
        protected: {
          operation: [
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${OP_PROTECTED}',
          ],
        },
      },
      opcode: [Opcodes.BOUND, 'ModRM_rm_reg16_10', 'DISP_i16'],
      operands: ['reg', 'rm'],
      operandSize: 16,
      cycles: 13,
    },
    {
      operation: ['${UD_EXCEPTION}'],
      opcode: [Opcodes.BOUND, 'ModRM_rm16_reg16_11'],
      operands: ['reg', 'rm'],
      operandSize: 16,
      cycles: 1,
    },
  ],
};
