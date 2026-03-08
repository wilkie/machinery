import { InstructionDataTypes, InstructionOperandTypes } from '@machinery/core';
import type { OpcodeMatcher } from '@machinery/core';

export const operands: OpcodeMatcher[] = [
  // ModRM block special case where mod == 0b00 and rm == 0b110
  {
    identifier: 'ModRM_110_reg_00',
    name: 'ModRM',
    type: InstructionDataTypes.Operand,
    size: 8,
    fields: [
      {
        identifier: 'rm',
        offset: 0,
        size: 3,
        match: 0b110,
      },
      {
        identifier: 'reg',
        offset: 3,
        size: 3,
        type: InstructionOperandTypes.Register,
      },
      {
        identifier: 'mod',
        offset: 6,
        size: 2,
        match: 0b00,
      },
    ],
  },
  // ModRM block where mod == 0b00
  {
    identifier: 'ModRM_rm_reg_00',
    name: 'ModRM',
    type: InstructionDataTypes.Operand,
    size: 8,
    fields: [
      {
        identifier: 'rm',
        offset: 0,
        size: 3,
      },
      {
        identifier: 'reg',
        offset: 3,
        size: 3,
        type: InstructionOperandTypes.Register,
      },
      {
        identifier: 'mod',
        offset: 6,
        size: 2,
        match: 0b00,
      },
    ],
  },
  // ModRM block where mod == 0b01
  {
    identifier: 'ModRM_rm_reg_01',
    name: 'ModRM',
    type: InstructionDataTypes.Operand,
    size: 8,
    fields: [
      {
        identifier: 'rm',
        offset: 0,
        size: 3,
      },
      {
        identifier: 'reg',
        offset: 3,
        size: 3,
        type: InstructionOperandTypes.Register,
      },
      {
        identifier: 'mod',
        offset: 6,
        size: 2,
        match: 0b01,
      },
    ],
  },
  // ModRM block where mod == 0b10
  {
    identifier: 'ModRM_rm_reg_10',
    name: 'ModRM',
    type: InstructionDataTypes.Operand,
    size: 8,
    fields: [
      {
        identifier: 'rm',
        offset: 0,
        size: 3,
      },
      {
        identifier: 'reg',
        offset: 3,
        size: 3,
        type: InstructionOperandTypes.Register,
      },
      {
        identifier: 'mod',
        offset: 6,
        size: 2,
        match: 0b10,
      },
    ],
  },
  // ModRM block where mod == 0b11
  {
    identifier: 'ModRM_rm_reg_11',
    name: 'ModRM',
    type: InstructionDataTypes.Operand,
    size: 8,
    fields: [
      {
        identifier: 'rm',
        offset: 0,
        size: 3,
        type: InstructionOperandTypes.Register,
      },
      {
        identifier: 'reg',
        offset: 3,
        size: 3,
        type: InstructionOperandTypes.Register,
      },
      {
        identifier: 'mod',
        offset: 6,
        size: 2,
        match: 0b11,
      },
    ],
  },
  // ModRM block special case where mod == 0b00 and rm == 0b110 and reg == 0b0xx
  {
    identifier: 'ModRM_110_seg_00',
    name: 'ModRM',
    type: InstructionDataTypes.Operand,
    size: 8,
    fields: [
      {
        identifier: 'rm',
        offset: 0,
        size: 3,
        match: 0b110,
      },
      {
        identifier: 'reg',
        offset: 3,
        size: 3,
        type: InstructionOperandTypes.Register,
      },
      {
        identifier: 'mod',
        offset: 6,
        size: 2,
        match: 0b00,
      },
    ],
  },
  // ModRM block where mod == 0b00
  {
    identifier: 'ModRM_rm_seg_00',
    name: 'ModRM',
    type: InstructionDataTypes.Operand,
    size: 8,
    fields: [
      {
        identifier: 'rm',
        offset: 0,
        size: 3,
      },
      {
        identifier: 'reg',
        offset: 3,
        size: 3,
        type: InstructionOperandTypes.Register,
      },
      {
        identifier: 'mod',
        offset: 6,
        size: 2,
        match: 0b00,
      },
    ],
  },
  // ModRM block where mod == 0b01
  {
    identifier: 'ModRM_rm_seg_01',
    name: 'ModRM',
    type: InstructionDataTypes.Operand,
    size: 8,
    fields: [
      {
        identifier: 'rm',
        offset: 0,
        size: 3,
      },
      {
        identifier: 'reg',
        offset: 3,
        size: 3,
        type: InstructionOperandTypes.Register,
      },
      {
        identifier: 'mod',
        offset: 6,
        size: 2,
        match: 0b01,
      },
    ],
  },
  // ModRM block where mod == 0b10
  {
    identifier: 'ModRM_rm_seg_10',
    name: 'ModRM',
    type: InstructionDataTypes.Operand,
    size: 8,
    fields: [
      {
        identifier: 'rm',
        offset: 0,
        size: 3,
      },
      {
        identifier: 'reg',
        offset: 3,
        size: 3,
        type: InstructionOperandTypes.Register,
      },
      {
        identifier: 'mod',
        offset: 6,
        size: 2,
        match: 0b10,
      },
    ],
  },
  // ModRM block where mod == 0b11
  {
    identifier: 'ModRM_rm_seg_11',
    name: 'ModRM',
    type: InstructionDataTypes.Operand,
    size: 8,
    fields: [
      {
        identifier: 'rm',
        offset: 0,
        size: 3,
        type: InstructionOperandTypes.Register,
      },
      {
        identifier: 'reg',
        offset: 3,
        size: 3,
        type: InstructionOperandTypes.Register,
      },
      {
        identifier: 'mod',
        offset: 6,
        size: 2,
        match: 0b11,
      },
    ],
  },
  // ModRM block special case where mod == 0b00 and rm == 0b110 and reg == 0b000
  {
    identifier: 'ModRM_110_000_00',
    name: 'ModRM',
    type: InstructionDataTypes.Operand,
    size: 8,
    fields: [
      {
        identifier: 'rm',
        offset: 0,
        size: 3,
        match: 0b110,
      },
      {
        identifier: 'reg',
        offset: 3,
        size: 3,
        match: 0b000,
      },
      {
        identifier: 'mod',
        offset: 6,
        size: 2,
        match: 0b00,
      },
    ],
  },
  // ModRM block where mod == 0b00 and reg == 0b000
  {
    identifier: 'ModRM_rm_000_00',
    name: 'ModRM',
    type: InstructionDataTypes.Operand,
    size: 8,
    fields: [
      {
        identifier: 'rm',
        offset: 0,
        size: 3,
      },
      {
        identifier: 'reg',
        offset: 3,
        size: 3,
        match: 0b000,
      },
      {
        identifier: 'mod',
        offset: 6,
        size: 2,
        match: 0b00,
      },
    ],
  },
  // ModRM block where mod == 0b01 and reg == 0b000
  {
    identifier: 'ModRM_rm_000_01',
    name: 'ModRM',
    type: InstructionDataTypes.Operand,
    size: 8,
    fields: [
      {
        identifier: 'rm',
        offset: 0,
        size: 3,
      },
      {
        identifier: 'reg',
        offset: 3,
        size: 3,
        match: 0b000,
      },
      {
        identifier: 'mod',
        offset: 6,
        size: 2,
        match: 0b01,
      },
    ],
  },
  // ModRM block where mod == 0b10 and reg == 0b000
  {
    identifier: 'ModRM_rm_000_10',
    name: 'ModRM',
    type: InstructionDataTypes.Operand,
    size: 8,
    fields: [
      {
        identifier: 'rm',
        offset: 0,
        size: 3,
      },
      {
        identifier: 'reg',
        offset: 3,
        size: 3,
        match: 0b000,
      },
      {
        identifier: 'mod',
        offset: 6,
        size: 2,
        match: 0b10,
      },
    ],
  },
  // ModRM block where mod == 0b11 and reg == 0b000
  {
    identifier: 'ModRM_rm_000_11',
    name: 'ModRM',
    type: InstructionDataTypes.Operand,
    size: 8,
    fields: [
      {
        identifier: 'rm',
        offset: 0,
        size: 3,
        type: InstructionOperandTypes.Register,
      },
      {
        identifier: 'reg',
        offset: 3,
        size: 3,
        match: 0b000,
      },
      {
        identifier: 'mod',
        offset: 6,
        size: 2,
        match: 0b11,
      },
    ],
  },
  // ModRM block special case where mod == 0b00 and rm == 0b110 and reg == 0b001
  {
    identifier: 'ModRM_110_001_00',
    name: 'ModRM',
    type: InstructionDataTypes.Operand,
    size: 8,
    fields: [
      {
        identifier: 'rm',
        offset: 0,
        size: 3,
        match: 0b110,
      },
      {
        identifier: 'reg',
        offset: 3,
        size: 3,
        match: 0b001,
      },
      {
        identifier: 'mod',
        offset: 6,
        size: 2,
        match: 0b00,
      },
    ],
  },
  // ModRM block where mod == 0b00 and reg == 0b001
  {
    identifier: 'ModRM_rm_001_00',
    name: 'ModRM',
    type: InstructionDataTypes.Operand,
    size: 8,
    fields: [
      {
        identifier: 'rm',
        offset: 0,
        size: 3,
      },
      {
        identifier: 'reg',
        offset: 3,
        size: 3,
        match: 0b001,
      },
      {
        identifier: 'mod',
        offset: 6,
        size: 2,
        match: 0b00,
      },
    ],
  },
  // ModRM block where mod == 0b01 and reg == 0b001
  {
    identifier: 'ModRM_rm_001_01',
    name: 'ModRM',
    type: InstructionDataTypes.Operand,
    size: 8,
    fields: [
      {
        identifier: 'rm',
        offset: 0,
        size: 3,
      },
      {
        identifier: 'reg',
        offset: 3,
        size: 3,
        match: 0b001,
      },
      {
        identifier: 'mod',
        offset: 6,
        size: 2,
        match: 0b01,
      },
    ],
  },
  // ModRM block where mod == 0b10 and reg == 0b001
  {
    identifier: 'ModRM_rm_001_10',
    name: 'ModRM',
    type: InstructionDataTypes.Operand,
    size: 8,
    fields: [
      {
        identifier: 'rm',
        offset: 0,
        size: 3,
      },
      {
        identifier: 'reg',
        offset: 3,
        size: 3,
        match: 0b001,
      },
      {
        identifier: 'mod',
        offset: 6,
        size: 2,
        match: 0b10,
      },
    ],
  },
  // ModRM block where mod == 0b11 and reg == 0b001
  {
    identifier: 'ModRM_rm_001_11',
    name: 'ModRM',
    type: InstructionDataTypes.Operand,
    size: 8,
    fields: [
      {
        identifier: 'rm',
        offset: 0,
        size: 3,
        type: InstructionOperandTypes.Register,
      },
      {
        identifier: 'reg',
        offset: 3,
        size: 3,
        match: 0b001,
      },
      {
        identifier: 'mod',
        offset: 6,
        size: 2,
        match: 0b11,
      },
    ],
  },
  // ModRM block special case where mod == 0b00 and rm == 0b110 and reg == 0b010
  {
    identifier: 'ModRM_110_010_00',
    name: 'ModRM',
    type: InstructionDataTypes.Operand,
    size: 8,
    fields: [
      {
        identifier: 'rm',
        offset: 0,
        size: 3,
        match: 0b110,
      },
      {
        identifier: 'reg',
        offset: 3,
        size: 3,
        match: 0b010,
      },
      {
        identifier: 'mod',
        offset: 6,
        size: 2,
        match: 0b00,
      },
    ],
  },
  // ModRM block where mod == 0b00 and reg == 0b010
  {
    identifier: 'ModRM_rm_010_00',
    name: 'ModRM',
    type: InstructionDataTypes.Operand,
    size: 8,
    fields: [
      {
        identifier: 'rm',
        offset: 0,
        size: 3,
      },
      {
        identifier: 'reg',
        offset: 3,
        size: 3,
        match: 0b010,
      },
      {
        identifier: 'mod',
        offset: 6,
        size: 2,
        match: 0b00,
      },
    ],
  },
  // ModRM block where mod == 0b01 and reg == 0b010
  {
    identifier: 'ModRM_rm_010_01',
    name: 'ModRM',
    type: InstructionDataTypes.Operand,
    size: 8,
    fields: [
      {
        identifier: 'rm',
        offset: 0,
        size: 3,
      },
      {
        identifier: 'reg',
        offset: 3,
        size: 3,
        match: 0b010,
      },
      {
        identifier: 'mod',
        offset: 6,
        size: 2,
        match: 0b01,
      },
    ],
  },
  // ModRM block where mod == 0b10 and reg == 0b010
  {
    identifier: 'ModRM_rm_010_10',
    name: 'ModRM',
    type: InstructionDataTypes.Operand,
    size: 8,
    fields: [
      {
        identifier: 'rm',
        offset: 0,
        size: 3,
      },
      {
        identifier: 'reg',
        offset: 3,
        size: 3,
        match: 0b010,
      },
      {
        identifier: 'mod',
        offset: 6,
        size: 2,
        match: 0b10,
      },
    ],
  },
  // ModRM block where mod == 0b11 and reg == 0b010
  {
    identifier: 'ModRM_rm_010_11',
    name: 'ModRM',
    type: InstructionDataTypes.Operand,
    size: 8,
    fields: [
      {
        identifier: 'rm',
        offset: 0,
        size: 3,
        type: InstructionOperandTypes.Register,
      },
      {
        identifier: 'reg',
        offset: 3,
        size: 3,
        match: 0b010,
      },
      {
        identifier: 'mod',
        offset: 6,
        size: 2,
        match: 0b11,
      },
    ],
  },
  // ModRM block special case where mod == 0b00 and rm == 0b110 and reg == 0b011
  {
    identifier: 'ModRM_110_011_00',
    name: 'ModRM',
    type: InstructionDataTypes.Operand,
    size: 8,
    fields: [
      {
        identifier: 'rm',
        offset: 0,
        size: 3,
        match: 0b110,
      },
      {
        identifier: 'reg',
        offset: 3,
        size: 3,
        match: 0b011,
      },
      {
        identifier: 'mod',
        offset: 6,
        size: 2,
        match: 0b00,
      },
    ],
  },
  // ModRM block where mod == 0b00 and reg == 0b011
  {
    identifier: 'ModRM_rm_011_00',
    name: 'ModRM',
    type: InstructionDataTypes.Operand,
    size: 8,
    fields: [
      {
        identifier: 'rm',
        offset: 0,
        size: 3,
      },
      {
        identifier: 'reg',
        offset: 3,
        size: 3,
        match: 0b011,
      },
      {
        identifier: 'mod',
        offset: 6,
        size: 2,
        match: 0b00,
      },
    ],
  },
  // ModRM block where mod == 0b01 and reg == 0b011
  {
    identifier: 'ModRM_rm_011_01',
    name: 'ModRM',
    type: InstructionDataTypes.Operand,
    size: 8,
    fields: [
      {
        identifier: 'rm',
        offset: 0,
        size: 3,
      },
      {
        identifier: 'reg',
        offset: 3,
        size: 3,
        match: 0b011,
      },
      {
        identifier: 'mod',
        offset: 6,
        size: 2,
        match: 0b01,
      },
    ],
  },
  // ModRM block where mod == 0b10 and reg == 0b011
  {
    identifier: 'ModRM_rm_011_10',
    name: 'ModRM',
    type: InstructionDataTypes.Operand,
    size: 8,
    fields: [
      {
        identifier: 'rm',
        offset: 0,
        size: 3,
      },
      {
        identifier: 'reg',
        offset: 3,
        size: 3,
        match: 0b011,
      },
      {
        identifier: 'mod',
        offset: 6,
        size: 2,
        match: 0b10,
      },
    ],
  },
  // ModRM block where mod == 0b11 and reg == 0b011
  {
    identifier: 'ModRM_rm_011_11',
    name: 'ModRM',
    type: InstructionDataTypes.Operand,
    size: 8,
    fields: [
      {
        identifier: 'rm',
        offset: 0,
        size: 3,
        type: InstructionOperandTypes.Register,
      },
      {
        identifier: 'reg',
        offset: 3,
        size: 3,
        match: 0b011,
      },
      {
        identifier: 'mod',
        offset: 6,
        size: 2,
        match: 0b11,
      },
    ],
  },
  // ModRM block special case where mod == 0b00 and rm == 0b110 and reg == 0b100
  {
    identifier: 'ModRM_110_100_00',
    name: 'ModRM',
    type: InstructionDataTypes.Operand,
    size: 8,
    fields: [
      {
        identifier: 'rm',
        offset: 0,
        size: 3,
        match: 0b110,
      },
      {
        identifier: 'reg',
        offset: 3,
        size: 3,
        match: 0b100,
      },
      {
        identifier: 'mod',
        offset: 6,
        size: 2,
        match: 0b00,
      },
    ],
  },
  // ModRM block where mod == 0b00 and reg == 0b100
  {
    identifier: 'ModRM_rm_100_00',
    name: 'ModRM',
    type: InstructionDataTypes.Operand,
    size: 8,
    fields: [
      {
        identifier: 'rm',
        offset: 0,
        size: 3,
      },
      {
        identifier: 'reg',
        offset: 3,
        size: 3,
        match: 0b100,
      },
      {
        identifier: 'mod',
        offset: 6,
        size: 2,
        match: 0b00,
      },
    ],
  },
  // ModRM block where mod == 0b01 and reg == 0b100
  {
    identifier: 'ModRM_rm_100_01',
    name: 'ModRM',
    type: InstructionDataTypes.Operand,
    size: 8,
    fields: [
      {
        identifier: 'rm',
        offset: 0,
        size: 3,
      },
      {
        identifier: 'reg',
        offset: 3,
        size: 3,
        match: 0b100,
      },
      {
        identifier: 'mod',
        offset: 6,
        size: 2,
        match: 0b01,
      },
    ],
  },
  // ModRM block where mod == 0b10 and reg == 0b100
  {
    identifier: 'ModRM_rm_100_10',
    name: 'ModRM',
    type: InstructionDataTypes.Operand,
    size: 8,
    fields: [
      {
        identifier: 'rm',
        offset: 0,
        size: 3,
      },
      {
        identifier: 'reg',
        offset: 3,
        size: 3,
        match: 0b100,
      },
      {
        identifier: 'mod',
        offset: 6,
        size: 2,
        match: 0b10,
      },
    ],
  },
  // ModRM block where mod == 0b11 and reg == 0b100
  {
    identifier: 'ModRM_rm_100_11',
    name: 'ModRM',
    type: InstructionDataTypes.Operand,
    size: 8,
    fields: [
      {
        identifier: 'rm',
        offset: 0,
        size: 3,
        type: InstructionOperandTypes.Register,
      },
      {
        identifier: 'reg',
        offset: 3,
        size: 3,
        match: 0b100,
      },
      {
        identifier: 'mod',
        offset: 6,
        size: 2,
        match: 0b11,
      },
    ],
  },
  // ModRM block special case where mod == 0b00 and rm == 0b110 and reg == 0b101
  {
    identifier: 'ModRM_110_101_00',
    name: 'ModRM',
    type: InstructionDataTypes.Operand,
    size: 8,
    fields: [
      {
        identifier: 'rm',
        offset: 0,
        size: 3,
        match: 0b110,
      },
      {
        identifier: 'reg',
        offset: 3,
        size: 3,
        match: 0b101,
      },
      {
        identifier: 'mod',
        offset: 6,
        size: 2,
        match: 0b00,
      },
    ],
  },
  // ModRM block where mod == 0b00 and reg == 0b101
  {
    identifier: 'ModRM_rm_101_00',
    name: 'ModRM',
    type: InstructionDataTypes.Operand,
    size: 8,
    fields: [
      {
        identifier: 'rm',
        offset: 0,
        size: 3,
      },
      {
        identifier: 'reg',
        offset: 3,
        size: 3,
        match: 0b101,
      },
      {
        identifier: 'mod',
        offset: 6,
        size: 2,
        match: 0b00,
      },
    ],
  },
  // ModRM block where mod == 0b01 and reg == 0b101
  {
    identifier: 'ModRM_rm_101_01',
    name: 'ModRM',
    type: InstructionDataTypes.Operand,
    size: 8,
    fields: [
      {
        identifier: 'rm',
        offset: 0,
        size: 3,
      },
      {
        identifier: 'reg',
        offset: 3,
        size: 3,
        match: 0b101,
      },
      {
        identifier: 'mod',
        offset: 6,
        size: 2,
        match: 0b01,
      },
    ],
  },
  // ModRM block where mod == 0b10 and reg == 0b101
  {
    identifier: 'ModRM_rm_101_10',
    name: 'ModRM',
    type: InstructionDataTypes.Operand,
    size: 8,
    fields: [
      {
        identifier: 'rm',
        offset: 0,
        size: 3,
      },
      {
        identifier: 'reg',
        offset: 3,
        size: 3,
        match: 0b101,
      },
      {
        identifier: 'mod',
        offset: 6,
        size: 2,
        match: 0b10,
      },
    ],
  },
  // ModRM block where mod == 0b11 and reg == 0b101
  {
    identifier: 'ModRM_rm_101_11',
    name: 'ModRM',
    type: InstructionDataTypes.Operand,
    size: 8,
    fields: [
      {
        identifier: 'rm',
        offset: 0,
        size: 3,
        type: InstructionOperandTypes.Register,
      },
      {
        identifier: 'reg',
        offset: 3,
        size: 3,
        match: 0b101,
      },
      {
        identifier: 'mod',
        offset: 6,
        size: 2,
        match: 0b11,
      },
    ],
  },
  // ModRM block special case where mod == 0b00 and rm == 0b110 and reg == 0b110
  {
    identifier: 'ModRM_110_110_00',
    name: 'ModRM',
    type: InstructionDataTypes.Operand,
    size: 8,
    fields: [
      {
        identifier: 'rm',
        offset: 0,
        size: 3,
        match: 0b110,
      },
      {
        identifier: 'reg',
        offset: 3,
        size: 3,
        match: 0b110,
      },
      {
        identifier: 'mod',
        offset: 6,
        size: 2,
        match: 0b00,
      },
    ],
  },
  // ModRM block where mod == 0b00 and reg == 0b110
  {
    identifier: 'ModRM_rm_110_00',
    name: 'ModRM',
    type: InstructionDataTypes.Operand,
    size: 8,
    fields: [
      {
        identifier: 'rm',
        offset: 0,
        size: 3,
      },
      {
        identifier: 'reg',
        offset: 3,
        size: 3,
        match: 0b110,
      },
      {
        identifier: 'mod',
        offset: 6,
        size: 2,
        match: 0b00,
      },
    ],
  },
  // ModRM block where mod == 0b01 and reg == 0b110
  {
    identifier: 'ModRM_rm_110_01',
    name: 'ModRM',
    type: InstructionDataTypes.Operand,
    size: 8,
    fields: [
      {
        identifier: 'rm',
        offset: 0,
        size: 3,
      },
      {
        identifier: 'reg',
        offset: 3,
        size: 3,
        match: 0b110,
      },
      {
        identifier: 'mod',
        offset: 6,
        size: 2,
        match: 0b01,
      },
    ],
  },
  // ModRM block where mod == 0b10 and reg == 0b110
  {
    identifier: 'ModRM_rm_110_10',
    name: 'ModRM',
    type: InstructionDataTypes.Operand,
    size: 8,
    fields: [
      {
        identifier: 'rm',
        offset: 0,
        size: 3,
      },
      {
        identifier: 'reg',
        offset: 3,
        size: 3,
        match: 0b110,
      },
      {
        identifier: 'mod',
        offset: 6,
        size: 2,
        match: 0b10,
      },
    ],
  },
  // ModRM block where mod == 0b11 and reg == 0b110
  {
    identifier: 'ModRM_rm_110_11',
    name: 'ModRM',
    type: InstructionDataTypes.Operand,
    size: 8,
    fields: [
      {
        identifier: 'rm',
        offset: 0,
        size: 3,
        type: InstructionOperandTypes.Register,
      },
      {
        identifier: 'reg',
        offset: 3,
        size: 3,
        match: 0b110,
      },
      {
        identifier: 'mod',
        offset: 6,
        size: 2,
        match: 0b11,
      },
    ],
  },
  // ModRM block special case where mod == 0b00 and rm == 0b110 and reg == 0b111
  {
    identifier: 'ModRM_110_111_00',
    name: 'ModRM',
    type: InstructionDataTypes.Operand,
    size: 8,
    fields: [
      {
        identifier: 'rm',
        offset: 0,
        size: 3,
        match: 0b110,
      },
      {
        identifier: 'reg',
        offset: 3,
        size: 3,
        match: 0b111,
      },
      {
        identifier: 'mod',
        offset: 6,
        size: 2,
        match: 0b00,
      },
    ],
  },
  // ModRM block where mod == 0b00 and reg == 0b111
  {
    identifier: 'ModRM_rm_111_00',
    name: 'ModRM',
    type: InstructionDataTypes.Operand,
    size: 8,
    fields: [
      {
        identifier: 'rm',
        offset: 0,
        size: 3,
      },
      {
        identifier: 'reg',
        offset: 3,
        size: 3,
        match: 0b111,
      },
      {
        identifier: 'mod',
        offset: 6,
        size: 2,
        match: 0b00,
      },
    ],
  },
  // ModRM block where mod == 0b01 and reg == 0b111
  {
    identifier: 'ModRM_rm_111_01',
    name: 'ModRM',
    type: InstructionDataTypes.Operand,
    size: 8,
    fields: [
      {
        identifier: 'rm',
        offset: 0,
        size: 3,
      },
      {
        identifier: 'reg',
        offset: 3,
        size: 3,
        match: 0b111,
      },
      {
        identifier: 'mod',
        offset: 6,
        size: 2,
        match: 0b01,
      },
    ],
  },
  // ModRM block where mod == 0b10 and reg == 0b111
  {
    identifier: 'ModRM_rm_111_10',
    name: 'ModRM',
    type: InstructionDataTypes.Operand,
    size: 8,
    fields: [
      {
        identifier: 'rm',
        offset: 0,
        size: 3,
      },
      {
        identifier: 'reg',
        offset: 3,
        size: 3,
        match: 0b111,
      },
      {
        identifier: 'mod',
        offset: 6,
        size: 2,
        match: 0b10,
      },
    ],
  },
  // ModRM block where mod == 0b11 and reg == 0b111
  {
    identifier: 'ModRM_rm_111_11',
    name: 'ModRM',
    type: InstructionDataTypes.Operand,
    size: 8,
    fields: [
      {
        identifier: 'rm',
        offset: 0,
        size: 3,
        type: InstructionOperandTypes.Register,
      },
      {
        identifier: 'reg',
        offset: 3,
        size: 3,
        match: 0b111,
      },
      {
        identifier: 'mod',
        offset: 6,
        size: 2,
        match: 0b11,
      },
    ],
  },
  {
    identifier: 'DISP_i8',
    name: '8-bit Displacement Value',
    type: InstructionDataTypes.Displacement,
    size: 8,
    signed: true,
    fields: [
      {
        identifier: 'DISP',
        offset: 0,
        size: 8,
        signed: true,
      },
    ],
  },
  {
    identifier: 'DISP_i16',
    name: '16-bit Displacement Value',
    type: InstructionDataTypes.Displacement,
    size: 16,
    signed: true,
    fields: [
      {
        identifier: 'DISP',
        offset: 0,
        size: 16,
        signed: true,
      },
    ],
  },
  {
    identifier: 'IMM_u8',
    name: '8-bit Unsigned Immediate Value',
    type: InstructionDataTypes.Immediate,
    size: 8,
    fields: [
      {
        identifier: 'IMM',
        offset: 0,
        size: 8,
      },
    ],
  },
  {
    identifier: 'IMM_i8',
    name: '8-bit Signed Immediate Value',
    type: InstructionDataTypes.Immediate,
    size: 8,
    signed: true,
    fields: [
      {
        identifier: 'IMM',
        offset: 0,
        size: 8,
        signed: true,
      },
    ],
  },
  {
    identifier: 'IMM_u16',
    name: '16-bit Unsigned Immediate Value',
    type: InstructionDataTypes.Immediate,
    size: 16,
    fields: [
      {
        identifier: 'IMM',
        offset: 0,
        size: 16,
      },
    ],
  },
  {
    identifier: 'IMM_i16',
    name: '16-bit Signed Immediate Value',
    type: InstructionDataTypes.Immediate,
    size: 16,
    signed: true,
    fields: [
      {
        identifier: 'IMM',
        offset: 0,
        size: 16,
        signed: true,
      },
    ],
  },
  {
    identifier: 'IMM_u32',
    name: '32-bit Unsigned Immediate Value',
    fields: [
      {
        identifier: 'IMM',
        offset: 0,
        size: 32,
      },
    ],
    type: InstructionDataTypes.Immediate,
    size: 32,
  },
];
