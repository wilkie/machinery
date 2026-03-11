import {
  InstructionDataTypes,
  InstructionOperandTypes,
  type InstructionInfo,
} from '@machinery/core';

import { Opcodes } from '../opcodes';

export const ex_af: InstructionInfo = {
  identifier: 'ex',
  name: 'Exchange',
  description: 'Exchanges the contents of two register pairs.',
  modifies: [],
  forms: [
    // EX AF, AF'
    {
      opcode: [Opcodes.EX_AF_AF],
      operands: ['AF', "AF'"],
      operation: ['tmp = AF', "AF = AF'", "AF' = tmp"],
      cycles: 4,
    },
    // EX DE, HL
    {
      opcode: [Opcodes.EX_DE_HL],
      operands: ['DE', 'HL'],
      operation: ['tmp = DE', 'DE = HL', 'HL = tmp'],
      cycles: 4,
    },
    // EX (SP), HL
    {
      opcode: [
        {
          identifier: 'Opcode_EX_xSPx_HL',
          name: 'EX (SP), HL Opcode Field',
          type: InstructionDataTypes.Operand,
          size: 8,
          fields: [
            {
              identifier: 'opcode_low',
              offset: 0,
              size: 3,
              match: 0b011,
            },
            {
              identifier: 'rm',
              offset: 3,
              size: 3,
              match: 0b100,
              type: InstructionOperandTypes.Memory,
              encoding: ['SP'],
            },
            {
              identifier: 'opcode_high',
              offset: 6,
              size: 2,
              match: 0b11,
            },
          ],
        },
      ],
      operands: ['rm', 'HL'],
      operation: ['tmp = RAM:u16[SP]', 'RAM:u16[SP] = HL', 'HL = tmp'],
      cycles: 19,
    },
  ],
};

export const exx: InstructionInfo = {
  identifier: 'exx',
  name: 'Exchange All',
  description:
    "Exchanges BC, DE, and HL with their shadow registers BC', DE', and HL'.",
  modifies: [],
  forms: [
    {
      opcode: [Opcodes.EXX],
      operands: [],
      operation: [
        'tmp = BC',
        "BC = BC'",
        "BC' = tmp",
        'tmp = DE',
        "DE = DE'",
        "DE' = tmp",
        'tmp = HL',
        "HL = HL'",
        "HL' = tmp",
      ],
      cycles: 4,
    },
  ],
};
