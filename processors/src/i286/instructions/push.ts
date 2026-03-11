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
    OP: [
      'tmp = SP - 2',
      'stack_address = SS_BASE + tmp',
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
      operation: ['value = ES', '${OP}'],
      cycles: 3,
    },
    // 0x0E - PUSH CS
    {
      opcode: [Opcodes.PUSH_CS],
      operands: ['CS'],
      operation: ['value = CS', '${OP}'],
      cycles: 3,
    },
    // 0x16 - PUSH SS
    {
      opcode: [Opcodes.PUSH_SS],
      operands: ['SS'],
      operation: ['value = SS', '${OP}'],
      cycles: 3,
    },
    // 0x1E - PUSH DS
    {
      opcode: [Opcodes.PUSH_DS],
      operands: ['DS'],
      operation: ['value = DS', '${OP}'],
      cycles: 3,
    },
    // 0x0F 0xA0 - PUSH FS
    {
      opcode: [Opcodes.SYSTEM, SystemOpcodes.PUSH_FS],
      operands: ['FS'],
      operation: ['value = FS', '${OP}'],
      cycles: 3,
    },
    // 0x0F 0xA8 - PUSH GS
    {
      opcode: [Opcodes.SYSTEM, SystemOpcodes.PUSH_GS],
      operands: ['GS'],
      operation: ['value = GS', '${OP}'],
      cycles: 3,
    },
    // 0x50+rw - PUSH rw
    {
      operation: ['value = ${MOD_RM_RM16}', '${OP}'],
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
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + %{DISP}',
        'value = RAM:u16[effective_address]',
        '${OP}',
      ],
      opcode: [Opcodes.CALL_JMP_INC_DEC_PUSH, 'ModRM_110_110_00', 'DISP_i16'],
      operands: ['rm'],
      operandSize: 16,
      cycles: 5,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET}',
        'value = RAM:u16[effective_address]',
        '${OP}',
      ],
      opcode: [Opcodes.CALL_JMP_INC_DEC_PUSH, 'ModRM_rm_110_00'],
      operands: ['rm'],
      operandSize: 16,
      cycles: 5,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'value = RAM:u16[effective_address]',
        '${OP}',
      ],
      opcode: [Opcodes.CALL_JMP_INC_DEC_PUSH, 'ModRM_rm_110_01', 'DISP_i8'],
      operands: ['rm'],
      operandSize: 16,
      cycles: 5,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'value = RAM:u16[effective_address]',
        '${OP}',
      ],
      opcode: [Opcodes.CALL_JMP_INC_DEC_PUSH, 'ModRM_rm_110_10', 'DISP_i16'],
      operands: ['rm'],
      operandSize: 16,
      cycles: 5,
    },
    {
      operation: ['value = ${MOD_RM_RM16}', '${OP}'],
      opcode: [Opcodes.CALL_JMP_INC_DEC_PUSH, 'ModRM_rm16_110_11'],
      operands: ['rm'],
      operandSize: 16,
      cycles: 3,
    },
    // 0x68 dw - PUSH dw
    {
      operation: ['value = %{imm}', '${OP}'],
      opcode: [Opcodes.PUSH_DW, 'IMM_u16'],
      operands: ['imm'],
      operandSize: 16,
      cycles: 3,
    },
    // 0x6A db - PUSH db
    {
      operation: ['value = %{imm}', '${OP}'],
      opcode: [Opcodes.PUSH_DB, 'IMM_i8'],
      operands: ['imm'],
      operandSize: 16,
      cycles: 3,
    },
  ],
};
