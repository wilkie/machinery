import type { InstructionInfo } from '@machinery/core';
import { InstructionDataTypes, InstructionOperandTypes } from '@machinery/core';

import { Opcodes, SystemOpcodes } from '../opcodes';

// TODO: determine what the machine should do when SP <= 1 (286 documentation says 'shut down')
export const push: InstructionInfo = {
  identifier: 'push',
  name: 'Push a Word onto the Stack',
  description:
    'The stack pointer `SP` is decremented by 2, and the operand is placed on the new top of stack, which is pointed to by `SS:SP`.\n\nThe 80286 `PUSH SP` instruction pushes the value of `SP` as it existed before the instruction. This differs from the 8086, which pushes the new (decremented by 2) value.',
  modifies: [],
  undefined: [],
  macros: {
    OP_REAL: [
      'tmp = SP - 2',
      'offset = tmp',
      'stack_address = SS_BASE + offset',
      '#GP if offset == 0xffff',
      'RAM:u16[stack_address] = value',
      'SP = tmp',
    ],
    OP_PROTECTED: [
      'tmp = SP - 2',
      'offset = tmp',
      'stack_address = SS_BASE + offset',
      '#GP if (offset + 1) < SS_LIMIT_MIN',
      '#GP if (offset + 1) > SS_LIMIT_MAX',
      'RAM:u16[stack_address] = value',
      'SP = tmp',
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
      identifier: 'stack_address',
      name: 'Effective Stack Address',
      size: 32,
    },
    {
      identifier: 'value',
      name: 'Value to be pushed',
      size: 16,
    },
    {
      identifier: 'tmp',
      name: 'Temporary stack pointer',
      size: 16,
    },
  ],
  forms: [
    // 0x06 - PUSH ES
    {
      opcode: [Opcodes.PUSH_ES],
      operands: ['ES'],
      modes: {
        real: {
          operation: ['value = ES', '${OP_REAL}'],
        },
        protected: {
          operation: ['value = ES', '${OP_PROTECTED}'],
        },
      },
      cycles: 3,
    },
    // 0x0E - PUSH CS
    {
      opcode: [Opcodes.PUSH_CS],
      operands: ['CS'],
      modes: {
        real: {
          operation: ['value = CS', '${OP_REAL}'],
        },
        protected: {
          operation: ['value = CS', '${OP_PROTECTED}'],
        },
      },
      cycles: 3,
    },
    // 0x16 - PUSH SS
    {
      opcode: [Opcodes.PUSH_SS],
      operands: ['SS'],
      modes: {
        real: {
          operation: ['value = SS', '${OP_REAL}'],
        },
        protected: {
          operation: ['value = SS', '${OP_PROTECTED}'],
        },
      },
      cycles: 3,
    },
    // 0x1E - PUSH DS
    {
      opcode: [Opcodes.PUSH_DS],
      operands: ['DS'],
      modes: {
        real: {
          operation: ['value = DS', '${OP_REAL}'],
        },
        protected: {
          operation: ['value = DS', '${OP_PROTECTED}'],
        },
      },
      cycles: 3,
    },
    // 0x0F 0xA0 - PUSH FS
    {
      opcode: [Opcodes.SYSTEM, SystemOpcodes.PUSH_FS],
      operands: ['FS'],
      modes: {
        real: {
          operation: ['value = FS', '${OP_REAL}'],
        },
        protected: {
          operation: ['value = FS', '${OP_PROTECTED}'],
        },
      },
      cycles: 3,
    },
    // 0x0F 0xA8 - PUSH GS
    {
      opcode: [Opcodes.SYSTEM, SystemOpcodes.PUSH_GS],
      operands: ['GS'],
      modes: {
        real: {
          operation: ['value = GS', '${OP_REAL}'],
        },
        protected: {
          operation: ['value = GS', '${OP_PROTECTED}'],
        },
      },
      cycles: 3,
    },
    // 0x50+rw - PUSH rw
    {
      modes: {
        real: {
          operation: ['value = ${MOD_RM_RM16}', '${OP_REAL}'],
        },
        protected: {
          operation: ['value = ${MOD_RM_RM16}', '${OP_PROTECTED}'],
        },
      },
      opcode: [
        {
          identifier: 'OpcodeRM',
          name: 'PUSH Opcode Field',
          type: InstructionDataTypes.Operand,
          size: 8,
          fields: [
            {
              identifier: 'opcode',
              offset: 3,
              size: 5,
              match: 0b01010,
            },
            {
              identifier: 'rm',
              offset: 0,
              size: 3,
              type: InstructionOperandTypes.Register,
              encoding: ['AX', 'CX', 'DX', 'BX', 'SP', 'BP', 'SI', 'DI'],
            },
          ],
        },
      ],
      operands: ['rm'],
      operandSize: 16,
      cycles: 3,
    },
    // 0xFF /6 - PUSH mw
    {
      modes: {
        real: {
          operation: [
            'offset = %{DISP:u16}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'value = RAM:u16[effective_address]',
            '${OP_REAL}',
          ],
        },
        protected: {
          operation: [
            'offset = %{DISP:u16}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'value = RAM:u16[effective_address]',
            '${OP_PROTECTED}',
          ],
        },
      },
      opcode: [Opcodes.CALL_JMP_INC_DEC_PUSH, 'ModRM_110_110_00', 'DISP_i16'],
      operands: ['rm'],
      operandSize: 16,
      cycles: 5,
    },
    {
      modes: {
        real: {
          operation: [
            'offset = ${MOD_RM_OFFSET}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'value = RAM:u16[effective_address]',
            '${OP_REAL}',
          ],
        },
        protected: {
          operation: [
            'offset = ${MOD_RM_OFFSET}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'value = RAM:u16[effective_address]',
            '${OP_PROTECTED}',
          ],
        },
      },
      opcode: [Opcodes.CALL_JMP_INC_DEC_PUSH, 'ModRM_rm_110_00'],
      operands: ['rm'],
      operandSize: 16,
      cycles: 5,
    },
    {
      modes: {
        real: {
          operation: [
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'value = RAM:u16[effective_address]',
            '${OP_REAL}',
          ],
        },
        protected: {
          operation: [
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'value = RAM:u16[effective_address]',
            '${OP_PROTECTED}',
          ],
        },
      },
      opcode: [Opcodes.CALL_JMP_INC_DEC_PUSH, 'ModRM_rm_110_01', 'DISP_i8'],
      operands: ['rm'],
      operandSize: 16,
      cycles: 5,
    },
    {
      modes: {
        real: {
          operation: [
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'value = RAM:u16[effective_address]',
            '${OP_REAL}',
          ],
        },
        protected: {
          operation: [
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'value = RAM:u16[effective_address]',
            '${OP_PROTECTED}',
          ],
        },
      },
      opcode: [Opcodes.CALL_JMP_INC_DEC_PUSH, 'ModRM_rm_110_10', 'DISP_i16'],
      operands: ['rm'],
      operandSize: 16,
      cycles: 5,
    },
    {
      modes: {
        real: {
          operation: ['value = ${MOD_RM_RM16}', '${OP_REAL}'],
        },
        protected: {
          operation: ['value = ${MOD_RM_RM16}', '${OP_PROTECTED}'],
        },
      },
      opcode: [Opcodes.CALL_JMP_INC_DEC_PUSH, 'ModRM_rm16_110_11'],
      operands: ['rm'],
      operandSize: 16,
      cycles: 3,
    },
    // 0x68 dw - PUSH dw
    {
      modes: {
        real: {
          operation: ['value = %{imm}', '${OP_REAL}'],
        },
        protected: {
          operation: ['value = %{imm}', '${OP_PROTECTED}'],
        },
      },
      opcode: [Opcodes.PUSH_DW, 'IMM_u16'],
      operands: ['imm'],
      operandSize: 16,
      cycles: 3,
    },
    // 0x6A db - PUSH db
    {
      modes: {
        real: {
          operation: ['value = %{imm}', '${OP_REAL}'],
        },
        protected: {
          operation: ['value = %{imm}', '${OP_PROTECTED}'],
        },
      },
      opcode: [Opcodes.PUSH_DB, 'IMM_i8'],
      operands: ['imm'],
      operandSize: 16,
      cycles: 3,
    },
  ],
};
