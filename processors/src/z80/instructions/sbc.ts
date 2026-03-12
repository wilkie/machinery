import {
  InstructionDataTypes,
  InstructionOperandTypes,
  type InstructionInfo,
} from '@machinery/core';

import { Opcodes, EDOpcodes } from '../opcodes';

export const sbc: InstructionInfo = {
  identifier: 'sbc',
  name: 'Subtract with Carry',
  description:
    'Subtracts the operand and carry flag from the accumulator (8-bit) or from HL (16-bit).',
  modifies: ['SF', 'ZF', 'HF', 'PF', 'NF', 'CF'],
  macros: {
    ALU8_OP: [
      'b = b + CF',
      'alu_result = a - b',
      'flag_op = ${FLAG_OP_ALU_SUB8} | ${FLAG_OP_CARRY}',
    ],
    ALU16_OP: [
      'b = b + CF',
      'alu_result = a - b',
      'flag_op = ${FLAG_OP_ALU_SUB16} | ${FLAG_OP_CARRY}',
    ],
  },
  forms: [
    // SBC A, r
    {
      opcode: [Opcodes.SBC_A_B],
      operands: ['A', 'B'],
      operandSize: 8,
      operation: ['${RESOLVE_CF}', 'a = A', 'b = B', '${ALU8_OP}', 'A = alu_result'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.SBC_A_C],
      operands: ['A', 'C'],
      operandSize: 8,
      operation: ['${RESOLVE_CF}', 'a = A', 'b = C', '${ALU8_OP}', 'A = alu_result'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.SBC_A_D],
      operands: ['A', 'D'],
      operandSize: 8,
      operation: ['${RESOLVE_CF}', 'a = A', 'b = D', '${ALU8_OP}', 'A = alu_result'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.SBC_A_E],
      operands: ['A', 'E'],
      operandSize: 8,
      operation: ['${RESOLVE_CF}', 'a = A', 'b = E', '${ALU8_OP}', 'A = alu_result'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.SBC_A_H],
      operands: ['A', 'H'],
      operandSize: 8,
      operation: ['${RESOLVE_CF}', 'a = A', 'b = H', '${ALU8_OP}', 'A = alu_result'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.SBC_A_L],
      operands: ['A', 'L'],
      operandSize: 8,
      operation: ['${RESOLVE_CF}', 'a = A', 'b = L', '${ALU8_OP}', 'A = alu_result'],
      cycles: 4,
    },
    {
      opcode: [
        {
          identifier: 'Opcode_SBC_A_xHLx',
          name: 'SBC A, (HL) Opcode Field',
          type: InstructionDataTypes.Operand,
          size: 8,
          fields: [
            {
              identifier: 'rm',
              offset: 0,
              size: 3,
              match: 0b110,
              type: InstructionOperandTypes.Memory,
              encoding: ['HL'],
            },
            {
              identifier: 'opcode',
              offset: 3,
              size: 5,
              match: 0b10011,
            },
          ],
        },
      ],
      operands: ['A', 'rm'],
      operandSize: 8,
      operation: ['${RESOLVE_CF}', 'a = A', 'b = RAM:u8[HL]', '${ALU8_OP}', 'A = alu_result'],
      cycles: 7,
    },
    {
      opcode: [Opcodes.SBC_A_A],
      operands: ['A', 'A'],
      operandSize: 8,
      operation: ['${RESOLVE_CF}', 'a = A', 'b = A', '${ALU8_OP}', 'A = alu_result'],
      cycles: 4,
    },

    // SBC A, n
    {
      opcode: [Opcodes.SBC_A_N, 'IMM_u8'],
      operands: ['A', 'imm'],
      operandSize: 8,
      operation: ['${RESOLVE_CF}', 'a = A', 'b = %{imm}', '${ALU8_OP}', 'A = alu_result'],
      cycles: 7,
    },

    // SBC A, (IX+d) / SBC A, (IY+d)
    {
      opcode: ['DD_IX', Opcodes.SBC_A_xHLx, 'DISP_i8'],
      operands: ['A', 'rm'],
      operandSize: 8,
      operation: ['${RESOLVE_CF}', 'a = A', 'b = RAM:u8[IX + %{DISP}]', '${ALU8_OP}', 'A = alu_result'],
      cycles: 19,
    },
    {
      opcode: ['FD_IY', Opcodes.SBC_A_xHLx, 'DISP_i8'],
      operands: ['A', 'rm'],
      operandSize: 8,
      operation: ['${RESOLVE_CF}', 'a = A', 'b = RAM:u8[IY + %{DISP}]', '${ALU8_OP}', 'A = alu_result'],
      cycles: 19,
    },

    // SBC HL, rp (ED prefix)
    {
      opcode: [Opcodes.ED_PREFIX, EDOpcodes.SBC_HL_BC],
      operands: ['HL', 'BC'],
      operandSize: 16,
      operation: ['${RESOLVE_CF}', 'a = HL', 'b = BC', '${ALU16_OP}', 'HL = alu_result'],
      cycles: 15,
    },
    {
      opcode: [Opcodes.ED_PREFIX, EDOpcodes.SBC_HL_DE],
      operands: ['HL', 'DE'],
      operandSize: 16,
      operation: ['${RESOLVE_CF}', 'a = HL', 'b = DE', '${ALU16_OP}', 'HL = alu_result'],
      cycles: 15,
    },
    {
      opcode: [Opcodes.ED_PREFIX, EDOpcodes.SBC_HL_HL],
      operands: ['HL', 'HL'],
      operandSize: 16,
      operation: ['${RESOLVE_CF}', 'a = HL', 'b = HL', '${ALU16_OP}', 'HL = alu_result'],
      cycles: 15,
    },
    {
      opcode: [Opcodes.ED_PREFIX, EDOpcodes.SBC_HL_SP],
      operands: ['HL', 'SP'],
      operandSize: 16,
      operation: ['${RESOLVE_CF}', 'a = HL', 'b = SP', '${ALU16_OP}', 'HL = alu_result'],
      cycles: 15,
    },
  ],
};
