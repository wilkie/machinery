import {
  InstructionDataTypes,
  InstructionOperandTypes,
  type InstructionInfo,
} from '@machinery/core';

import { Opcodes } from '../opcodes';

export const cp: InstructionInfo = {
  identifier: 'cp',
  name: 'Compare',
  description:
    'Subtracts the operand from A but discards the result, only setting flags. A is not modified.',
  modifies: ['SF', 'ZF', 'HF', 'PF', 'NF', 'CF'],
  macros: {
    ALU_OP: ['alu_result = a - b', 'flag_op = ${FLAG_OP_ALU_SUB8}'],
  },
  forms: [
    {
      opcode: [Opcodes.CP_B],
      operands: ['B'],
      operandSize: 8,
      operation: ['a = A', 'b = B', '${ALU_OP}'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.CP_C],
      operands: ['C'],
      operandSize: 8,
      operation: ['a = A', 'b = C', '${ALU_OP}'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.CP_D],
      operands: ['D'],
      operandSize: 8,
      operation: ['a = A', 'b = D', '${ALU_OP}'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.CP_E],
      operands: ['E'],
      operandSize: 8,
      operation: ['a = A', 'b = E', '${ALU_OP}'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.CP_H],
      operands: ['H'],
      operandSize: 8,
      operation: ['a = A', 'b = H', '${ALU_OP}'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.CP_L],
      operands: ['L'],
      operandSize: 8,
      operation: ['a = A', 'b = L', '${ALU_OP}'],
      cycles: 4,
    },
    {
      opcode: [
        {
          identifier: 'Opcode_CP_xHLx',
          name: 'CP (HL) Opcode Field',
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
              match: 0b10111,
            },
          ],
        },
      ],
      operands: ['rm'],
      operandSize: 8,
      operation: ['a = A', 'b = RAM:u8[HL]', '${ALU_OP}'],
      cycles: 7,
    },
    {
      opcode: [Opcodes.CP_A],
      operands: ['A'],
      operandSize: 8,
      operation: ['a = A', 'b = A', '${ALU_OP}'],
      cycles: 4,
    },
    {
      opcode: [Opcodes.CP_N, 'IMM_u8'],
      operands: ['imm'],
      operandSize: 8,
      operation: ['a = A', 'b = %{imm}', '${ALU_OP}'],
      cycles: 7,
    },

    // CP (IX+d) / CP (IY+d)
    {
      opcode: ['DD_IX', Opcodes.CP_xHLx, 'DISP_i8'],
      operands: ['rm'],
      operandSize: 8,
      operation: ['a = A', 'b = RAM:u8[IX + %{DISP}]', '${ALU_OP}'],
      cycles: 19,
    },
    {
      opcode: ['FD_IY', Opcodes.CP_xHLx, 'DISP_i8'],
      operands: ['rm'],
      operandSize: 8,
      operation: ['a = A', 'b = RAM:u8[IY + %{DISP}]', '${ALU_OP}'],
      cycles: 19,
    },
  ],
};
