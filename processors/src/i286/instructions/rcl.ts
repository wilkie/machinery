import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const rcl: InstructionInfo = {
  identifier: 'rcl',
  name: 'Rotate Carry Left',
  description: '',
  modifies: ['CF'],
  undefined: ['OF'],
  macros: {
    ALU8_OP: [
      '${RESOLVE_FLAGS}',
      'tmp_a = CARRY == 1 ? (tmp_a | 0x100) : tmp_a',
      'tmp = tmp_a <~[9] tmp_b',
      // CF = carry bit (bit 8) of 9-bit result; OF = MSB XOR CF from last rotation
      'CARRY = (tmp >> 8) & 0x1',
      'CF = CARRY',
      // OF = MSB(result) XOR new CF = bit 7 XOR bit 8 of 9-bit result
      'OF = ((tmp >> 7) ^ (tmp >> 8)) & 0x1',
      'flag_op = ${FLAG_OP_RESOLVED}',
    ],
    ALU16_OP: [
      '${RESOLVE_FLAGS}',
      'tmp_a = CARRY == 1 ? (tmp_a | 0x10000) : tmp_a',
      'tmp = tmp_a <~[17] tmp_b',
      'CARRY = (tmp >> 16) & 0x1',
      'CF = CARRY',
      'OF = ((tmp >> 15) ^ (tmp >> 16)) & 0x1',
      'flag_op = ${FLAG_OP_RESOLVED}',
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
      name: 'Temporary Shift Result',
      size: 32,
    },
    {
      identifier: 'tmp_a',
      name: 'Temporary Shift Value',
      size: 32,
    },
    {
      identifier: 'tmp_b',
      name: 'Temporary Shift Amount',
      size: 8,
    },
  ],
  forms: [
    // 0xD0 /2 - RCL eb, 1
    {
      modes: {
        real: {
          operation: [
            'effective_address = ${MOD_RM_SEGMENT} + %{DISP:u16}',
            'tmp_a = RAM:u8[effective_address]',
            'tmp_b = 1',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = tmp',
          ],
        },
        protected: {
          operation: [
            'offset = %{DISP:u16}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'tmp_a = RAM:u8[effective_address]',
            'tmp_b = 1',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = tmp',
          ],
        },
      },
      modifies: ['OF', 'CF'],
      undefined: [],
      opcode: [
        Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EB,
        'ModRM_110_010_00',
        'DISP_i16',
      ],
      operands: ['rm'],
      operandSize: 8,
      cycles: 7,
    },
    {
      modes: {
        real: {
          operation: [
            'effective_address = ${MOD_RM_SEGMENT} + (${MOD_RM_OFFSET}):u16',
            'tmp_a = RAM:u8[effective_address]',
            'tmp_b = 1',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = tmp',
          ],
        },
        protected: {
          operation: [
            'offset = (${MOD_RM_OFFSET}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'tmp_a = RAM:u8[effective_address]',
            'tmp_b = 1',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = tmp',
          ],
        },
      },
      modifies: ['OF', 'CF'],
      undefined: [],
      opcode: [Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EB, 'ModRM_rm_010_00'],
      operands: ['rm'],
      operandSize: 8,
      cycles: 7,
    },
    {
      modes: {
        real: {
          operation: [
            'effective_address = ${MOD_RM_SEGMENT} + (${MOD_RM_OFFSET} + %{DISP}):u16',
            'tmp_a = RAM:u8[effective_address]',
            'tmp_b = 1',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = tmp',
          ],
        },
        protected: {
          operation: [
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'tmp_a = RAM:u8[effective_address]',
            'tmp_b = 1',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = tmp',
          ],
        },
      },
      modifies: ['OF', 'CF'],
      undefined: [],
      opcode: [
        Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EB,
        'ModRM_rm_010_01',
        'DISP_i8',
      ],
      operands: ['rm'],
      operandSize: 8,
      cycles: 7,
    },
    {
      modes: {
        real: {
          operation: [
            'effective_address = ${MOD_RM_SEGMENT} + (${MOD_RM_OFFSET} + %{DISP}):u16',
            'tmp_a = RAM:u8[effective_address]',
            'tmp_b = 1',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = tmp',
          ],
        },
        protected: {
          operation: [
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'tmp_a = RAM:u8[effective_address]',
            'tmp_b = 1',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = tmp',
          ],
        },
      },
      modifies: ['OF', 'CF'],
      undefined: [],
      opcode: [
        Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EB,
        'ModRM_rm_010_10',
        'DISP_i16',
      ],
      operands: ['rm'],
      operandSize: 8,
      cycles: 7,
    },
    {
      operation: [
        'tmp_a = ${MOD_RM_RM8}',
        'tmp_b = 1',
        '${ALU8_OP}',
        '${MOD_RM_RM8} = tmp',
      ],
      modifies: ['OF', 'CF'],
      undefined: [],
      opcode: [Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EB, 'ModRM_rm8_010_11'],
      operands: ['rm'],
      operandSize: 8,
      cycles: 2,
    },
    // 0xD2 /2 - RCL eb, CL
    {
      modes: {
        real: {
          operation: [
            'next if (CL & 0x1f) == 0',
            'tmp_b = (CL & 0x1f) % 9',
            'effective_address = ${MOD_RM_SEGMENT} + %{DISP:u16}',
            'tmp_a = RAM:u8[effective_address]',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = tmp',
          ],
        },
        protected: {
          operation: [
            'next if (CL & 0x1f) == 0',
            'tmp_b = (CL & 0x1f) % 9',
            'offset = %{DISP:u16}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'tmp_a = RAM:u8[effective_address]',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = tmp',
          ],
        },
      },
      opcode: [
        Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EB_CL,
        'ModRM_110_010_00',
        'DISP_i16',
      ],
      operands: ['rm', 'CL'],
      operandSize: 8,
      cycles: 8,
    },
    {
      modes: {
        real: {
          operation: [
            'next if (CL & 0x1f) == 0',
            'tmp_b = (CL & 0x1f) % 9',
            'effective_address = ${MOD_RM_SEGMENT} + (${MOD_RM_OFFSET}):u16',
            'tmp_a = RAM:u8[effective_address]',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = tmp',
          ],
        },
        protected: {
          operation: [
            'next if (CL & 0x1f) == 0',
            'tmp_b = (CL & 0x1f) % 9',
            'offset = (${MOD_RM_OFFSET}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'tmp_a = RAM:u8[effective_address]',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = tmp',
          ],
        },
      },
      opcode: [Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EB_CL, 'ModRM_rm_010_00'],
      operands: ['rm', 'CL'],
      operandSize: 8,
      cycles: 8,
    },
    {
      modes: {
        real: {
          operation: [
            'next if (CL & 0x1f) == 0',
            'tmp_b = (CL & 0x1f) % 9',
            'effective_address = ${MOD_RM_SEGMENT} + (${MOD_RM_OFFSET} + %{DISP}):u16',
            'tmp_a = RAM:u8[effective_address]',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = tmp',
          ],
        },
        protected: {
          operation: [
            'next if (CL & 0x1f) == 0',
            'tmp_b = (CL & 0x1f) % 9',
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'tmp_a = RAM:u8[effective_address]',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = tmp',
          ],
        },
      },
      opcode: [
        Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EB_CL,
        'ModRM_rm_010_01',
        'DISP_i8',
      ],
      operands: ['rm', 'CL'],
      operandSize: 8,
      cycles: 8,
    },
    {
      modes: {
        real: {
          operation: [
            'next if (CL & 0x1f) == 0',
            'tmp_b = (CL & 0x1f) % 9',
            'effective_address = ${MOD_RM_SEGMENT} + (${MOD_RM_OFFSET} + %{DISP}):u16',
            'tmp_a = RAM:u8[effective_address]',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = tmp',
          ],
        },
        protected: {
          operation: [
            'next if (CL & 0x1f) == 0',
            'tmp_b = (CL & 0x1f) % 9',
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'tmp_a = RAM:u8[effective_address]',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = tmp',
          ],
        },
      },
      opcode: [
        Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EB_CL,
        'ModRM_rm_010_10',
        'DISP_i16',
      ],
      operands: ['rm', 'CL'],
      operandSize: 8,
      cycles: 8,
    },
    {
      operation: [
        'next if (CL & 0x1f) == 0',
        'tmp_b = (CL & 0x1f) % 9',
        'tmp_a = ${MOD_RM_RM8}',
        '${ALU8_OP}',
        '${MOD_RM_RM8} = tmp',
      ],
      opcode: [Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EB_CL, 'ModRM_rm8_010_11'],
      operands: ['rm', 'CL'],
      operandSize: 8,
      cycles: 5,
    },
    // 0xC0 /2 - RCL eb, db
    {
      modes: {
        real: {
          operation: [
            'next if (%{imm} & 0x1f) == 0',
            'tmp_b = (%{imm} & 0x1f) % 9',
            'effective_address = ${MOD_RM_SEGMENT} + %{DISP:u16}',
            'tmp_a = RAM:u8[effective_address]',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = tmp',
          ],
        },
        protected: {
          operation: [
            'next if (%{imm} & 0x1f) == 0',
            'tmp_b = (%{imm} & 0x1f) % 9',
            'offset = %{DISP:u16}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'tmp_a = RAM:u8[effective_address]',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = tmp',
          ],
        },
      },
      opcode: [
        Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EB_DB,
        'ModRM_110_010_00',
        'DISP_i16',
        'IMM_u8',
      ],
      operands: ['rm', 'imm'],
      operandSize: 8,
      cycles: 8,
    },
    {
      modes: {
        real: {
          operation: [
            'next if (%{imm} & 0x1f) == 0',
            'tmp_b = (%{imm} & 0x1f) % 9',
            'effective_address = ${MOD_RM_SEGMENT} + (${MOD_RM_OFFSET}):u16',
            'tmp_a = RAM:u8[effective_address]',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = tmp',
          ],
        },
        protected: {
          operation: [
            'next if (%{imm} & 0x1f) == 0',
            'tmp_b = (%{imm} & 0x1f) % 9',
            'offset = (${MOD_RM_OFFSET}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'tmp_a = RAM:u8[effective_address]',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = tmp',
          ],
        },
      },
      opcode: [
        Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EB_DB,
        'ModRM_rm_010_00',
        'IMM_u8',
      ],
      operands: ['rm', 'imm'],
      operandSize: 8,
      cycles: 8,
    },
    {
      modes: {
        real: {
          operation: [
            'next if (%{imm} & 0x1f) == 0',
            'tmp_b = (%{imm} & 0x1f) % 9',
            'effective_address = ${MOD_RM_SEGMENT} + (${MOD_RM_OFFSET} + %{DISP}):u16',
            'tmp_a = RAM:u8[effective_address]',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = tmp',
          ],
        },
        protected: {
          operation: [
            'next if (%{imm} & 0x1f) == 0',
            'tmp_b = (%{imm} & 0x1f) % 9',
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'tmp_a = RAM:u8[effective_address]',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = tmp',
          ],
        },
      },
      opcode: [
        Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EB_DB,
        'ModRM_rm_010_01',
        'DISP_i8',
        'IMM_u8',
      ],
      operands: ['rm', 'imm'],
      operandSize: 8,
      cycles: 8,
    },
    {
      modes: {
        real: {
          operation: [
            'next if (%{imm} & 0x1f) == 0',
            'tmp_b = (%{imm} & 0x1f) % 9',
            'effective_address = ${MOD_RM_SEGMENT} + (${MOD_RM_OFFSET} + %{DISP}):u16',
            'tmp_a = RAM:u8[effective_address]',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = tmp',
          ],
        },
        protected: {
          operation: [
            'next if (%{imm} & 0x1f) == 0',
            'tmp_b = (%{imm} & 0x1f) % 9',
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'tmp_a = RAM:u8[effective_address]',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = tmp',
          ],
        },
      },
      opcode: [
        Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EB_DB,
        'ModRM_rm_010_10',
        'DISP_i16',
        'IMM_u8',
      ],
      operands: ['rm', 'imm'],
      operandSize: 8,
      cycles: 8,
    },
    {
      operation: [
        'next if (%{imm} & 0x1f) == 0',
        'tmp_b = (%{imm} & 0x1f) % 9',
        'tmp_a = ${MOD_RM_RM8}',
        '${ALU8_OP}',
        '${MOD_RM_RM8} = tmp',
      ],
      opcode: [
        Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EB_DB,
        'ModRM_rm8_010_11',
        'IMM_u8',
      ],
      operands: ['rm', 'imm'],
      operandSize: 8,
      cycles: 5,
    },
    // 0xD1 /2 - RCL ew, 1
    {
      modes: {
        real: {
          operation: [
            'offset = %{DISP:u16}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'tmp_a = RAM:u16[effective_address]',
            'tmp_b = 1',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = tmp',
          ],
        },
        protected: {
          operation: [
            'offset = %{DISP:u16}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'tmp_a = RAM:u16[effective_address]',
            'tmp_b = 1',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = tmp',
          ],
        },
      },
      modifies: ['OF', 'CF', 'ZF', 'PF', 'SF'],
      opcode: [
        Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EW,
        'ModRM_110_010_00',
        'DISP_i16',
      ],
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
            'tmp_a = RAM:u16[effective_address]',
            'tmp_b = 1',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = tmp',
          ],
        },
        protected: {
          operation: [
            'offset = (${MOD_RM_OFFSET}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'tmp_a = RAM:u16[effective_address]',
            'tmp_b = 1',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = tmp',
          ],
        },
      },
      modifies: ['OF', 'CF', 'ZF', 'PF', 'SF'],
      opcode: [Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EW, 'ModRM_rm_010_00'],
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
            'tmp_a = RAM:u16[effective_address]',
            'tmp_b = 1',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = tmp',
          ],
        },
        protected: {
          operation: [
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'tmp_a = RAM:u16[effective_address]',
            'tmp_b = 1',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = tmp',
          ],
        },
      },
      modifies: ['OF', 'CF', 'ZF', 'PF', 'SF'],
      opcode: [
        Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EW,
        'ModRM_rm_010_01',
        'DISP_i8',
      ],
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
            'tmp_a = RAM:u16[effective_address]',
            'tmp_b = 1',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = tmp',
          ],
        },
        protected: {
          operation: [
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'tmp_a = RAM:u16[effective_address]',
            'tmp_b = 1',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = tmp',
          ],
        },
      },
      modifies: ['OF', 'CF', 'ZF', 'PF', 'SF'],
      opcode: [
        Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EW,
        'ModRM_rm_010_10',
        'DISP_i16',
      ],
      operands: ['rm'],
      operandSize: 16,
      cycles: 7,
    },
    {
      operation: [
        'tmp_a = ${MOD_RM_RM16}',
        'tmp_b = 1',
        '${ALU16_OP}',
        '${MOD_RM_RM16} = tmp',
      ],
      modifies: ['OF', 'CF', 'ZF', 'PF', 'SF'],
      opcode: [Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EW, 'ModRM_rm16_010_11'],
      operands: ['rm'],
      operandSize: 16,
      cycles: 2,
    },
    // 0xD3 /2 - RCL ew, CL
    {
      modes: {
        real: {
          operation: [
            'offset = %{DISP:u16}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'next if (CL & 0x1f) == 0',
            'tmp_b = (CL & 0x1f) % 17',
            'tmp_a = RAM:u16[effective_address]',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = tmp',
          ],
        },
        protected: {
          operation: [
            'offset = %{DISP:u16}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'next if (CL & 0x1f) == 0',
            'tmp_b = (CL & 0x1f) % 17',
            'tmp_a = RAM:u16[effective_address]',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = tmp',
          ],
        },
      },
      opcode: [
        Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EW_CL,
        'ModRM_110_010_00',
        'DISP_i16',
      ],
      operands: ['rm', 'CL'],
      operandSize: 16,
      cycles: 8,
    },
    {
      modes: {
        real: {
          operation: [
            'offset = (${MOD_RM_OFFSET}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'next if (CL & 0x1f) == 0',
            'tmp_b = (CL & 0x1f) % 17',
            'tmp_a = RAM:u16[effective_address]',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = tmp',
          ],
        },
        protected: {
          operation: [
            'offset = (${MOD_RM_OFFSET}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'next if (CL & 0x1f) == 0',
            'tmp_b = (CL & 0x1f) % 17',
            'tmp_a = RAM:u16[effective_address]',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = tmp',
          ],
        },
      },
      opcode: [Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EW_CL, 'ModRM_rm_010_00'],
      operands: ['rm', 'CL'],
      operandSize: 16,
      cycles: 8,
    },
    {
      modes: {
        real: {
          operation: [
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'next if (CL & 0x1f) == 0',
            'tmp_b = (CL & 0x1f) % 17',
            'tmp_a = RAM:u16[effective_address]',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = tmp',
          ],
        },
        protected: {
          operation: [
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'next if (CL & 0x1f) == 0',
            'tmp_b = (CL & 0x1f) % 17',
            'tmp_a = RAM:u16[effective_address]',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = tmp',
          ],
        },
      },
      opcode: [
        Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EW_CL,
        'ModRM_rm_010_01',
        'DISP_i8',
      ],
      operands: ['rm', 'CL'],
      operandSize: 16,
      cycles: 8,
    },
    {
      modes: {
        real: {
          operation: [
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'next if (CL & 0x1f) == 0',
            'tmp_b = (CL & 0x1f) % 17',
            'tmp_a = RAM:u16[effective_address]',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = tmp',
          ],
        },
        protected: {
          operation: [
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'next if (CL & 0x1f) == 0',
            'tmp_b = (CL & 0x1f) % 17',
            'tmp_a = RAM:u16[effective_address]',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = tmp',
          ],
        },
      },
      opcode: [
        Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EW_CL,
        'ModRM_rm_010_10',
        'DISP_i16',
      ],
      operands: ['rm', 'CL'],
      operandSize: 16,
      cycles: 8,
    },
    {
      operation: [
        'next if (CL & 0x1f) == 0',
        'tmp_b = (CL & 0x1f) % 17',
        'tmp_a = ${MOD_RM_RM16}',
        '${ALU16_OP}',
        '${MOD_RM_RM16} = tmp',
      ],
      opcode: [Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EW_CL, 'ModRM_rm16_010_11'],
      operands: ['rm', 'CL'],
      operandSize: 16,
      cycles: 5,
    },
    // 0xC1 /2 - RCL ew, db
    {
      modes: {
        real: {
          operation: [
            'offset = %{DISP:u16}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'next if (%{imm} & 0x1f) == 0',
            'tmp_b = (%{imm} & 0x1f) % 17',
            'tmp_a = RAM:u16[effective_address]',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = tmp',
          ],
        },
        protected: {
          operation: [
            'offset = %{DISP:u16}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'next if (%{imm} & 0x1f) == 0',
            'tmp_b = (%{imm} & 0x1f) % 17',
            'tmp_a = RAM:u16[effective_address]',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = tmp',
          ],
        },
      },
      opcode: [
        Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EW_DB,
        'ModRM_110_010_00',
        'DISP_i16',
        'IMM_u8',
      ],
      operands: ['rm', 'imm'],
      operandSize: 16,
      cycles: 8,
    },
    {
      modes: {
        real: {
          operation: [
            'offset = (${MOD_RM_OFFSET}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'next if (%{imm} & 0x1f) == 0',
            'tmp_b = (%{imm} & 0x1f) % 17',
            'tmp_a = RAM:u16[effective_address]',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = tmp',
          ],
        },
        protected: {
          operation: [
            'offset = (${MOD_RM_OFFSET}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'next if (%{imm} & 0x1f) == 0',
            'tmp_b = (%{imm} & 0x1f) % 17',
            'tmp_a = RAM:u16[effective_address]',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = tmp',
          ],
        },
      },
      opcode: [
        Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EW_DB,
        'ModRM_rm_010_00',
        'IMM_u8',
      ],
      operands: ['rm', 'imm'],
      operandSize: 16,
      cycles: 8,
    },
    {
      modes: {
        real: {
          operation: [
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'next if (%{imm} & 0x1f) == 0',
            'tmp_b = (%{imm} & 0x1f) % 17',
            'tmp_a = RAM:u16[effective_address]',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = tmp',
          ],
        },
        protected: {
          operation: [
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'next if (%{imm} & 0x1f) == 0',
            'tmp_b = (%{imm} & 0x1f) % 17',
            'tmp_a = RAM:u16[effective_address]',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = tmp',
          ],
        },
      },
      opcode: [
        Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EW_DB,
        'ModRM_rm_010_01',
        'DISP_i8',
        'IMM_u8',
      ],
      operands: ['rm', 'imm'],
      operandSize: 16,
      cycles: 8,
    },
    {
      modes: {
        real: {
          operation: [
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'next if (%{imm} & 0x1f) == 0',
            'tmp_b = (%{imm} & 0x1f) % 17',
            'tmp_a = RAM:u16[effective_address]',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = tmp',
          ],
        },
        protected: {
          operation: [
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'next if (%{imm} & 0x1f) == 0',
            'tmp_b = (%{imm} & 0x1f) % 17',
            'tmp_a = RAM:u16[effective_address]',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = tmp',
          ],
        },
      },
      opcode: [
        Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EW_DB,
        'ModRM_rm_010_10',
        'DISP_i16',
        'IMM_u8',
      ],
      operands: ['rm', 'imm'],
      operandSize: 16,
      cycles: 8,
    },
    {
      operation: [
        'next if (%{imm} & 0x1f) == 0',
        'tmp_b = (%{imm} & 0x1f) % 17',
        'tmp_a = ${MOD_RM_RM16}',
        '${ALU16_OP}',
        '${MOD_RM_RM16} = tmp',
      ],
      opcode: [
        Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EW_DB,
        'ModRM_rm16_010_11',
        'IMM_u8',
      ],
      operands: ['rm', 'imm'],
      operandSize: 16,
      cycles: 5,
    },
  ],
};
