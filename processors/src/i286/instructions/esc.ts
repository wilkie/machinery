import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

// On the 286 without a coprocessor (EM=0, MP=0), ESC instructions are
// effectively NOPs: they consume the ModRM byte and any displacement
// but perform no operation.  When EM=1, they raise #NM (not implemented here).
export const esc: InstructionInfo = {
  identifier: 'esc',
  name: 'Escape to Coprocessor',
  description:
    'Provides a mechanism for the processor to pass instructions to a coprocessor. On a 286 without a math coprocessor, ESC instructions are treated as NOPs.',
  modifies: [],
  undefined: [],
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
    // 0xD8 - ESC with disp16 (mod=00, rm=110)
    {
      opcode: [Opcodes.ESC, 'ModRM_110_reg16_00', 'DISP_i16'],
      operands: [],
      modes: {
        real: {
          operation: [
            'offset = %{DISP:u16}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
          ],
        },
        protected: {
          operation: [
            'offset = %{DISP:u16}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
          ],
        },
      },
      cycles: 2,
    },
    // 0xD8 - ESC with [rm] (mod=00)
    {
      opcode: [Opcodes.ESC, 'ModRM_rm_reg16_00'],
      operands: [],
      modes: {
        real: {
          operation: [
            'offset = (${MOD_RM_OFFSET}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
          ],
        },
        protected: {
          operation: [
            'offset = (${MOD_RM_OFFSET}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
          ],
        },
      },
      cycles: 2,
    },
    // 0xD8 - ESC with [rm+disp8] (mod=01)
    {
      opcode: [Opcodes.ESC, 'ModRM_rm_reg16_01', 'DISP_i8'],
      operands: [],
      modes: {
        real: {
          operation: [
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
          ],
        },
        protected: {
          operation: [
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
          ],
        },
      },
      cycles: 2,
    },
    // 0xD8 - ESC with [rm+disp16] (mod=10)
    {
      opcode: [Opcodes.ESC, 'ModRM_rm_reg16_10', 'DISP_i16'],
      operands: [],
      modes: {
        real: {
          operation: [
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
          ],
        },
        protected: {
          operation: [
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
          ],
        },
      },
      cycles: 2,
    },
    // 0xD8 - ESC with reg (mod=11) — no memory access
    {
      opcode: [Opcodes.ESC, 'ModRM_rm16_reg16_11'],
      operands: [],
      operation: [],
      cycles: 2,
    },
  ],
};
