import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const ror: InstructionInfo = {
  identifier: 'ror',
  name: 'Rotate Right',
  description: '',
  modifies: ['CF'],
  undefined: ['OF'],
  macros: {
    ALU8_OP: [
      'tmp = tmp_a ~>[8] tmp_b',
      'CARRY = (tmp_b % 8) == 0 ? CARRY : (tmp_a >> (tmp_b - 1)) & 0x1',
      'CF = CARRY',
      'OF = (((tmp >> 6) - 1) & 0x3) < 2 ? 1 : 0',
      'flag_op = flag_op | ${FLAG_OP_NOCF} | ${FLAG_OP_NOOF}',
    ],
    ALU16_OP: [
      'tmp = tmp_a ~>[16] tmp_b',
      'CARRY = (tmp_b % 16) == 0 ? CARRY : (tmp_a >> (tmp_b - 1)) & 0x1',
      'CF = CARRY',
      'OF = (((tmp >> 14) - 1) & 0x3) < 2 ? 1 : 0',
      'flag_op = flag_op | ${FLAG_OP_NOCF} | ${FLAG_OP_NOOF}',
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
    // 0xD0 /1 - ROR eb, 1
    {
      modes: {
        real: {
          operation: [
            'effective_address = ${MOD_RM_SEGMENT} + %{DISP}',
            'tmp_a = RAM:u8[effective_address]',
            'tmp_b = 1',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = tmp',
          ],
        },
        protected: {
          operation: [
            'offset = %{DISP}',
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
        'ModRM_110_001_00',
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
            'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET}',
            'tmp_a = RAM:u8[effective_address]',
            'tmp_b = 1',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = tmp',
          ],
        },
        protected: {
          operation: [
            'offset = ${MOD_RM_OFFSET}',
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
      opcode: [Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EB, 'ModRM_rm_001_00'],
      operands: ['rm'],
      operandSize: 8,
      cycles: 7,
    },
    {
      modes: {
        real: {
          operation: [
            'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
            'tmp_a = RAM:u8[effective_address]',
            'tmp_b = 1',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = tmp',
          ],
        },
        protected: {
          operation: [
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
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
        'ModRM_rm_001_01',
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
            'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
            'tmp_a = RAM:u8[effective_address]',
            'tmp_b = 1',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = tmp',
          ],
        },
        protected: {
          operation: [
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
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
        'ModRM_rm_001_10',
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
      opcode: [Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EB, 'ModRM_rm8_001_11'],
      operands: ['rm'],
      operandSize: 8,
      cycles: 2,
    },
    // 0xD2 /1 - ROR eb, CL
    {
      modes: {
        real: {
          operation: [
            'next if (CL & 0x1f) == 0',
            'effective_address = ${MOD_RM_SEGMENT} + %{DISP}',
            'tmp_a = RAM:u8[effective_address]',
            'tmp_b = CL & 0x1f',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = tmp',
          ],
        },
        protected: {
          operation: [
            'next if (CL & 0x1f) == 0',
            'offset = %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'tmp_a = RAM:u8[effective_address]',
            'tmp_b = CL & 0x1f',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = tmp',
          ],
        },
      },
      opcode: [
        Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EB_CL,
        'ModRM_110_001_00',
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
            'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET}',
            'tmp_a = RAM:u8[effective_address]',
            'tmp_b = CL & 0x1f',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = tmp',
          ],
        },
        protected: {
          operation: [
            'next if (CL & 0x1f) == 0',
            'offset = ${MOD_RM_OFFSET}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'tmp_a = RAM:u8[effective_address]',
            'tmp_b = CL & 0x1f',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = tmp',
          ],
        },
      },
      opcode: [Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EB_CL, 'ModRM_rm_001_00'],
      operands: ['rm', 'CL'],
      operandSize: 8,
      cycles: 8,
    },
    {
      modes: {
        real: {
          operation: [
            'next if (CL & 0x1f) == 0',
            'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
            'tmp_a = RAM:u8[effective_address]',
            'tmp_b = CL & 0x1f',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = tmp',
          ],
        },
        protected: {
          operation: [
            'next if (CL & 0x1f) == 0',
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'tmp_a = RAM:u8[effective_address]',
            'tmp_b = CL & 0x1f',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = tmp',
          ],
        },
      },
      opcode: [
        Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EB_CL,
        'ModRM_rm_001_01',
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
            'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
            'tmp_a = RAM:u8[effective_address]',
            'tmp_b = CL & 0x1f',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = tmp',
          ],
        },
        protected: {
          operation: [
            'next if (CL & 0x1f) == 0',
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'tmp_a = RAM:u8[effective_address]',
            'tmp_b = CL & 0x1f',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = tmp',
          ],
        },
      },
      opcode: [
        Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EB_CL,
        'ModRM_rm_001_10',
        'DISP_i16',
      ],
      operands: ['rm', 'CL'],
      operandSize: 8,
      cycles: 8,
    },
    {
      operation: [
        'next if (CL & 0x1f) == 0',
        'tmp_a = ${MOD_RM_RM8}',
        'tmp_b = CL & 0x1f',
        '${ALU8_OP}',
        '${MOD_RM_RM8} = tmp',
      ],
      opcode: [Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EB_CL, 'ModRM_rm8_001_11'],
      operands: ['rm', 'CL'],
      operandSize: 8,
      cycles: 5,
    },
    // 0xC0 /1 - ROR eb, db
    {
      modes: {
        real: {
          operation: [
            'next if (%{imm} & 0x1f) == 0',
            'effective_address = ${MOD_RM_SEGMENT} + %{DISP}',
            'tmp_a = RAM:u8[effective_address]',
            'tmp_b = %{imm} & 0x1f',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = tmp',
          ],
        },
        protected: {
          operation: [
            'next if (%{imm} & 0x1f) == 0',
            'offset = %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'tmp_a = RAM:u8[effective_address]',
            'tmp_b = %{imm} & 0x1f',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = tmp',
          ],
        },
      },
      opcode: [
        Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EB_DB,
        'ModRM_110_001_00',
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
            'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET}',
            'tmp_a = RAM:u8[effective_address]',
            'tmp_b = %{imm} & 0x1f',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = tmp',
          ],
        },
        protected: {
          operation: [
            'next if (%{imm} & 0x1f) == 0',
            'offset = ${MOD_RM_OFFSET}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'tmp_a = RAM:u8[effective_address]',
            'tmp_b = %{imm} & 0x1f',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = tmp',
          ],
        },
      },
      opcode: [
        Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EB_DB,
        'ModRM_rm_001_00',
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
            'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
            'tmp_a = RAM:u8[effective_address]',
            'tmp_b = %{imm} & 0x1f',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = tmp',
          ],
        },
        protected: {
          operation: [
            'next if (%{imm} & 0x1f) == 0',
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'tmp_a = RAM:u8[effective_address]',
            'tmp_b = %{imm} & 0x1f',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = tmp',
          ],
        },
      },
      opcode: [
        Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EB_DB,
        'ModRM_rm_001_01',
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
            'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
            'tmp_a = RAM:u8[effective_address]',
            'tmp_b = %{imm} & 0x1f',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = tmp',
          ],
        },
        protected: {
          operation: [
            'next if (%{imm} & 0x1f) == 0',
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'tmp_a = RAM:u8[effective_address]',
            'tmp_b = %{imm} & 0x1f',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = tmp',
          ],
        },
      },
      opcode: [
        Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EB_DB,
        'ModRM_rm_001_10',
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
        'tmp_a = ${MOD_RM_RM8}',
        'tmp_b = %{imm} & 0x1f',
        '${ALU8_OP}',
        '${MOD_RM_RM8} = tmp',
      ],
      opcode: [
        Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EB_DB,
        'ModRM_rm8_001_11',
        'IMM_u8',
      ],
      operands: ['rm', 'imm'],
      operandSize: 8,
      cycles: 5,
    },
    // 0xD1 /1 - ROR ew, 1
    {
      modes: {
        real: {
          operation: [
            'offset = %{DISP}',
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
            'offset = %{DISP}',
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
        'ModRM_110_001_00',
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
            'offset = ${MOD_RM_OFFSET}',
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
            'offset = ${MOD_RM_OFFSET}',
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
      opcode: [Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EW, 'ModRM_rm_001_00'],
      operands: ['rm'],
      operandSize: 16,
      cycles: 7,
    },
    {
      modes: {
        real: {
          operation: [
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
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
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
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
        'ModRM_rm_001_01',
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
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
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
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
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
        'ModRM_rm_001_10',
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
      opcode: [Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EW, 'ModRM_rm16_001_11'],
      operands: ['rm'],
      operandSize: 16,
      cycles: 2,
    },
    // 0xD3 /1 - ROR ew, CL
    {
      modes: {
        real: {
          operation: [
            'next if (CL & 0x1f) == 0',
            'offset = %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'tmp_a = RAM:u16[effective_address]',
            'tmp_b = CL & 0x1f',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = tmp',
          ],
        },
        protected: {
          operation: [
            'next if (CL & 0x1f) == 0',
            'offset = %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'tmp_a = RAM:u16[effective_address]',
            'tmp_b = CL & 0x1f',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = tmp',
          ],
        },
      },
      opcode: [
        Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EW_CL,
        'ModRM_110_001_00',
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
            'next if (CL & 0x1f) == 0',
            'offset = ${MOD_RM_OFFSET}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'tmp_a = RAM:u16[effective_address]',
            'tmp_b = CL & 0x1f',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = tmp',
          ],
        },
        protected: {
          operation: [
            'next if (CL & 0x1f) == 0',
            'offset = ${MOD_RM_OFFSET}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'tmp_a = RAM:u16[effective_address]',
            'tmp_b = CL & 0x1f',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = tmp',
          ],
        },
      },
      opcode: [Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EW_CL, 'ModRM_rm_001_00'],
      operands: ['rm', 'CL'],
      operandSize: 16,
      cycles: 8,
    },
    {
      modes: {
        real: {
          operation: [
            'next if (CL & 0x1f) == 0',
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'tmp_a = RAM:u16[effective_address]',
            'tmp_b = CL & 0x1f',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = tmp',
          ],
        },
        protected: {
          operation: [
            'next if (CL & 0x1f) == 0',
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'tmp_a = RAM:u16[effective_address]',
            'tmp_b = CL & 0x1f',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = tmp',
          ],
        },
      },
      opcode: [
        Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EW_CL,
        'ModRM_rm_001_01',
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
            'next if (CL & 0x1f) == 0',
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'tmp_a = RAM:u16[effective_address]',
            'tmp_b = CL & 0x1f',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = tmp',
          ],
        },
        protected: {
          operation: [
            'next if (CL & 0x1f) == 0',
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'tmp_a = RAM:u16[effective_address]',
            'tmp_b = CL & 0x1f',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = tmp',
          ],
        },
      },
      opcode: [
        Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EW_CL,
        'ModRM_rm_001_10',
        'DISP_i16',
      ],
      operands: ['rm', 'CL'],
      operandSize: 16,
      cycles: 8,
    },
    {
      operation: [
        'next if (CL & 0x1f) == 0',
        'tmp_a = ${MOD_RM_RM16}',
        'tmp_b = CL & 0x1f',
        '${ALU16_OP}',
        '${MOD_RM_RM16} = tmp',
      ],
      opcode: [Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EW_CL, 'ModRM_rm16_001_11'],
      operands: ['rm', 'CL'],
      operandSize: 16,
      cycles: 5,
    },
    // 0xC1 /1 - ROR ew, db
    {
      modes: {
        real: {
          operation: [
            'next if (%{imm} & 0x1f) == 0',
            'offset = %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'tmp_a = RAM:u16[effective_address]',
            'tmp_b = %{imm} & 0x1f',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = tmp',
          ],
        },
        protected: {
          operation: [
            'next if (%{imm} & 0x1f) == 0',
            'offset = %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'tmp_a = RAM:u16[effective_address]',
            'tmp_b = %{imm} & 0x1f',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = tmp',
          ],
        },
      },
      opcode: [
        Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EW_DB,
        'ModRM_110_001_00',
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
            'next if (%{imm} & 0x1f) == 0',
            'offset = ${MOD_RM_OFFSET}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'tmp_a = RAM:u16[effective_address]',
            'tmp_b = %{imm} & 0x1f',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = tmp',
          ],
        },
        protected: {
          operation: [
            'next if (%{imm} & 0x1f) == 0',
            'offset = ${MOD_RM_OFFSET}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'tmp_a = RAM:u16[effective_address]',
            'tmp_b = %{imm} & 0x1f',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = tmp',
          ],
        },
      },
      opcode: [
        Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EW_DB,
        'ModRM_rm_001_00',
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
            'next if (%{imm} & 0x1f) == 0',
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'tmp_a = RAM:u16[effective_address]',
            'tmp_b = %{imm} & 0x1f',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = tmp',
          ],
        },
        protected: {
          operation: [
            'next if (%{imm} & 0x1f) == 0',
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'tmp_a = RAM:u16[effective_address]',
            'tmp_b = %{imm} & 0x1f',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = tmp',
          ],
        },
      },
      opcode: [
        Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EW_DB,
        'ModRM_rm_001_01',
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
            'next if (%{imm} & 0x1f) == 0',
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'tmp_a = RAM:u16[effective_address]',
            'tmp_b = %{imm} & 0x1f',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = tmp',
          ],
        },
        protected: {
          operation: [
            'next if (%{imm} & 0x1f) == 0',
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'tmp_a = RAM:u16[effective_address]',
            'tmp_b = %{imm} & 0x1f',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = tmp',
          ],
        },
      },
      opcode: [
        Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EW_DB,
        'ModRM_rm_001_10',
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
        'tmp_a = ${MOD_RM_RM16}',
        'tmp_b = %{imm} & 0x1f',
        '${ALU16_OP}',
        '${MOD_RM_RM16} = tmp',
      ],
      opcode: [
        Opcodes.ROL_ROR_RCL_RCR_SAL_SAR_SHR_EW_DB,
        'ModRM_rm16_001_11',
        'IMM_u8',
      ],
      operands: ['rm', 'imm'],
      operandSize: 16,
      cycles: 5,
    },
  ],
};
