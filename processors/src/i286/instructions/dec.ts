import type { InstructionInfo } from '@machinery/core';
import { InstructionDataTypes, InstructionOperandTypes } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const dec: InstructionInfo = {
  identifier: 'dec',
  name: 'Decrement by 1',
  description:
    '1 is subtracted from the operand. Note that the carry flag is not changed by this instruction. If you want the carry flag set, use the `SUB` instruction with a second operand of 1.',
  modifies: ['OF', 'SF', 'ZF', 'AF', 'PF'],
  undefined: [],
  macros: {
    ALU8_OP: [
      '${RESOLVE_CF}',
      'b = 1',
      'alu_result = a - 1',
      'flag_op = ${FLAG_OP_ALU} | ${FLAG_OP_NOCF} | ${FLAG_OP_SUB} | ${FLAG_OP_8BIT}',
    ],
    ALU16_OP: [
      '${RESOLVE_CF}',
      'b = 1',
      'alu_result = a - 1',
      'flag_op = ${FLAG_OP_ALU} | ${FLAG_OP_NOCF} | ${FLAG_OP_SUB} | ${FLAG_OP_16BIT}',
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
    // 0xFE /1 - DEC eb
    {
      modes: {
        real: {
          operation: [
            'effective_address = ${MOD_RM_SEGMENT} + %{DISP:u16}',
            'a = RAM:u8[effective_address]',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = alu_result',
          ],
        },
        protected: {
          operation: [
            'offset = %{DISP:u16}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'a = RAM:u8[effective_address]',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = alu_result',
          ],
        },
      },
      opcode: [Opcodes.INC_DEC, 'ModRM_110_001_00', 'DISP_i16'],
      operands: ['rm'],
      operandSize: 8,
      cycles: 7,
    },
    {
      modes: {
        real: {
          operation: [
            'effective_address = ${MOD_RM_SEGMENT} + (${MOD_RM_OFFSET}):u16',
            'a = RAM:u8[effective_address]',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = alu_result',
          ],
        },
        protected: {
          operation: [
            'offset = (${MOD_RM_OFFSET}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'a = RAM:u8[effective_address]',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = alu_result',
          ],
        },
      },
      opcode: [Opcodes.INC_DEC, 'ModRM_rm_001_00'],
      operands: ['rm'],
      operandSize: 8,
      cycles: 7,
    },
    {
      modes: {
        real: {
          operation: [
            'effective_address = ${MOD_RM_SEGMENT} + (${MOD_RM_OFFSET} + %{DISP}):u16',
            'a = RAM:u8[effective_address]',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = alu_result',
          ],
        },
        protected: {
          operation: [
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'a = RAM:u8[effective_address]',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = alu_result',
          ],
        },
      },
      opcode: [Opcodes.INC_DEC, 'ModRM_rm_001_01', 'DISP_i8'],
      operands: ['rm'],
      operandSize: 8,
      cycles: 7,
    },
    {
      modes: {
        real: {
          operation: [
            'effective_address = ${MOD_RM_SEGMENT} + (${MOD_RM_OFFSET} + %{DISP}):u16',
            'a = RAM:u8[effective_address]',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = alu_result',
          ],
        },
        protected: {
          operation: [
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'a = RAM:u8[effective_address]',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = alu_result',
          ],
        },
      },
      opcode: [Opcodes.INC_DEC, 'ModRM_rm_001_10', 'DISP_i16'],
      operands: ['rm'],
      operandSize: 8,
      cycles: 7,
    },
    {
      operation: [
        'a = ${MOD_RM_RM8}',
        '${ALU8_OP}',
        '${MOD_RM_RM8} = alu_result',
      ],
      opcode: [Opcodes.INC_DEC, 'ModRM_rm8_001_11'],
      operands: ['rm'],
      operandSize: 8,
      cycles: 2,
    },
    // 0xFF /1 - DEC ew
    {
      modes: {
        real: {
          operation: [
            'offset = %{DISP:u16}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'a = RAM:u16[effective_address]',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = alu_result',
          ],
        },
        protected: {
          operation: [
            'offset = %{DISP:u16}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'a = RAM:u16[effective_address]',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = alu_result',
          ],
        },
      },
      opcode: [Opcodes.CALL_JMP_INC_DEC_PUSH, 'ModRM_110_001_00', 'DISP_i16'],
      operands: ['rm'],
      operandSize: 16,
      cycles: 7,
    },
    {
      modes: {
        real: {
          operation: [
            'offset = (${MOD_RM_OFFSET}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'a = RAM:u16[effective_address]',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = alu_result',
          ],
        },
        protected: {
          operation: [
            'offset = (${MOD_RM_OFFSET}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'a = RAM:u16[effective_address]',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = alu_result',
          ],
        },
      },
      opcode: [Opcodes.CALL_JMP_INC_DEC_PUSH, 'ModRM_rm_001_00'],
      operands: ['rm'],
      operandSize: 16,
      cycles: 7,
    },
    {
      modes: {
        real: {
          operation: [
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'a = RAM:u16[effective_address]',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = alu_result',
          ],
        },
        protected: {
          operation: [
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'a = RAM:u16[effective_address]',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = alu_result',
          ],
        },
      },
      opcode: [Opcodes.CALL_JMP_INC_DEC_PUSH, 'ModRM_rm_001_01', 'DISP_i8'],
      operands: ['rm'],
      operandSize: 16,
      cycles: 7,
    },
    {
      modes: {
        real: {
          operation: [
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'a = RAM:u16[effective_address]',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = alu_result',
          ],
        },
        protected: {
          operation: [
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'a = RAM:u16[effective_address]',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = alu_result',
          ],
        },
      },
      opcode: [Opcodes.CALL_JMP_INC_DEC_PUSH, 'ModRM_rm_001_10', 'DISP_i16'],
      operands: ['rm'],
      operandSize: 16,
      cycles: 7,
    },
    {
      operation: [
        'a = ${MOD_RM_RM16}',
        '${ALU16_OP}',
        '${MOD_RM_RM16} = alu_result',
      ],
      opcode: [Opcodes.CALL_JMP_INC_DEC_PUSH, 'ModRM_rm16_001_11'],
      operands: ['rm'],
      operandSize: 16,
      cycles: 2,
    },
    // 0x48+rw - DEC rw
    {
      operation: [
        'a = ${MOD_RM_RM16}',
        '${ALU16_OP}',
        '${MOD_RM_RM16} = alu_result',
      ],
      opcode: [
        {
          identifier: 'OpcodeRM',
          name: 'DEC Opcode Field',
          type: InstructionDataTypes.Operand,
          size: 8,
          fields: [
            {
              identifier: 'opcode',
              offset: 3,
              size: 5,
              match: 0b01001,
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
      cycles: 2,
    },
  ],
};
