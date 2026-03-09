import type { InstructionInfo } from '@machinery/core';
import { InstructionDataTypes, InstructionOperandTypes } from '@machinery/core';

import { Opcodes } from '../opcodes';

// TODO: exceptions
export const inc: InstructionInfo = {
  identifier: 'inc',
  name: 'Increment by 1',
  description:
    '1 is added to the operand. Note that the carry flag is not changed by this instruction. If you want the carry flag set, use the `ADD` instruction with a second operand of 1. ',
  modifies: ['OF', 'SF', 'ZF', 'AF', 'PF'],
  undefined: [],
  macros: {
    ALU8_OP: [
      '${RESOLVE_CF}',
      'b = 1',
      'alu_result = a + 1',
      'flag_op = ${FLAG_OP_ALU} | ${FLAG_OP_NOCF} | ${FLAG_OP_8BIT}',
    ],
    ALU16_OP: [
      '${RESOLVE_CF}',
      'b = 1',
      'alu_result = a + 1',
      'flag_op = ${FLAG_OP_ALU} | ${FLAG_OP_NOCF} | ${FLAG_OP_16BIT}',
    ],
  },
  locals: [
    {
      identifier: 'effective_address',
      name: 'Effective Address',
      size: 32,
    },
  ],
  forms: [
    // 0xFE /0 - INC eb
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + %{DISP}',
        'a = RAM:u8[effective_address]',
        '${ALU8_OP}',
        'RAM:u8[effective_address] = alu_result',
      ],
      opcode: [Opcodes.INC_DEC, 'ModRM_110_000_00', 'DISP_i16'],
      operands: ['rm'],
      operandSize: 8,
      cycles: 7,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET}',
        'a = RAM:u8[effective_address]',
        '${ALU8_OP}',
        'RAM:u8[effective_address] = alu_result',
      ],
      opcode: [Opcodes.INC_DEC, 'ModRM_rm_000_00'],
      operands: ['rm'],
      operandSize: 8,
      cycles: 7,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'a = RAM:u8[effective_address]',
        '${ALU8_OP}',
        'RAM:u8[effective_address] = alu_result',
      ],
      opcode: [Opcodes.INC_DEC, 'ModRM_rm_000_01', 'DISP_i8'],
      operands: ['rm'],
      operandSize: 8,
      cycles: 7,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'a = RAM:u8[effective_address]',
        '${ALU8_OP}',
        'RAM:u8[effective_address] = alu_result',
      ],
      opcode: [Opcodes.INC_DEC, 'ModRM_rm_000_10', 'DISP_i16'],
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
      opcode: [Opcodes.INC_DEC, 'ModRM_rm16_000_11'],
      operands: ['rm'],
      operandSize: 8,
      cycles: 2,
    },
    // 0xFF /0 - INC ew
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + %{DISP}',
        'a = RAM:u16[effective_address]',
        '${ALU16_OP}',
        'RAM:u16[effective_address] = alu_result',
      ],
      opcode: [Opcodes.CALL_JMP_INC_DEC_PUSH, 'ModRM_110_000_00', 'DISP_i16'],
      operands: ['rm'],
      operandSize: 16,
      cycles: 7,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET}',
        'a = RAM:u16[effective_address]',
        '${ALU16_OP}',
        'RAM:u16[effective_address] = alu_result',
      ],
      opcode: [Opcodes.CALL_JMP_INC_DEC_PUSH, 'ModRM_rm_000_00'],
      operands: ['rm'],
      operandSize: 16,
      cycles: 7,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'a = RAM:u16[effective_address]',
        '${ALU16_OP}',
        'RAM:u16[effective_address] = alu_result',
      ],
      opcode: [Opcodes.CALL_JMP_INC_DEC_PUSH, 'ModRM_rm_000_01', 'DISP_i8'],
      operands: ['rm'],
      operandSize: 16,
      cycles: 7,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'a = RAM:u16[effective_address]',
        '${ALU16_OP}',
        'RAM:u16[effective_address] = alu_result',
      ],
      opcode: [Opcodes.CALL_JMP_INC_DEC_PUSH, 'ModRM_rm_000_10', 'DISP_i16'],
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
      opcode: [Opcodes.CALL_JMP_INC_DEC_PUSH, 'ModRM_rm16_000_11'],
      operands: ['rm'],
      operandSize: 8,
      cycles: 2,
    },
    // 0x40+rw - INC rw
    {
      operation: [
        'a = ${MOD_RM_RM16}',
        '${ALU16_OP}',
        '${MOD_RM_RM16} = alu_result',
      ],
      opcode: [
        {
          identifier: 'OpcodeRM',
          name: 'INC Opcode Field',
          type: InstructionDataTypes.Operand,
          size: 8,
          fields: [
            {
              identifier: 'opcode',
              offset: 3,
              size: 5,
              match: 0b01000,
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
