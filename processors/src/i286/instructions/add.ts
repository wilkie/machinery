import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const add: InstructionInfo = {
  identifier: 'add',
  name: 'Integer Addition',
  description:
    '`ADD` and `ADC` perform an integer addition on the two operands. The `ADC` instruction also adds in the initial state of the carry flag. The result of the addition goes to the first operand. `ADC` is usually executed as part of a multi-byte or multi-word addition operation. When a byte immediate value is added to a word operand, the immediate value is first sign-extended.',
  modifies: ['OF', 'SF', 'ZF', 'AF', 'PF', 'CF'],
  undefined: [],
  macros: {
    ALU8_OP: [
      'alu_result = a + b',
      'flag_op = ${FLAG_OP_ALU} | ${FLAG_OP_8BIT}',
    ],
    ALU16_OP: [
      'alu_result = a + b',
      'flag_op = ${FLAG_OP_ALU} | ${FLAG_OP_16BIT}',
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
    // 0x00 /r - ADD eb, rb
    {
      modes: {
        real: {
          operation: [
            'effective_address = ${MOD_RM_SEGMENT} + %{DISP}',
            'a = RAM:u8[effective_address]',
            'b = ${MOD_RM_REG8}',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = alu_result',
          ],
        },
        protected: {
          operation: [
            'offset = %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'a = RAM:u8[effective_address]',
            'b = ${MOD_RM_REG8}',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = alu_result',
          ],
        },
      },
      opcode: [Opcodes.ADD_EB_RB, 'ModRM_110_reg8_00', 'DISP_i16'],
      operands: ['rm', 'reg'],
      operandSize: 8,
      cycles: 7,
    },
    {
      modes: {
        real: {
          operation: [
            'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET}',
            'a = RAM:u8[effective_address]',
            'b = ${MOD_RM_REG8}',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = alu_result',
          ],
        },
        protected: {
          operation: [
            'offset = ${MOD_RM_OFFSET}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'a = RAM:u8[effective_address]',
            'b = ${MOD_RM_REG8}',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = alu_result',
          ],
        },
      },
      opcode: [Opcodes.ADD_EB_RB, 'ModRM_rm_reg8_00'],
      operands: ['rm', 'reg'],
      operandSize: 8,
      cycles: 7,
    },
    {
      modes: {
        real: {
          operation: [
            'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
            'a = RAM:u8[effective_address]',
            'b = ${MOD_RM_REG8}',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = alu_result',
          ],
        },
        protected: {
          operation: [
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'a = RAM:u8[effective_address]',
            'b = ${MOD_RM_REG8}',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = alu_result',
          ],
        },
      },
      opcode: [Opcodes.ADD_EB_RB, 'ModRM_rm_reg8_01', 'DISP_i8'],
      operands: ['rm', 'reg'],
      operandSize: 8,
      cycles: 7,
    },
    {
      modes: {
        real: {
          operation: [
            'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
            'a = RAM:u8[effective_address]',
            'b = ${MOD_RM_REG8}',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = alu_result',
          ],
        },
        protected: {
          operation: [
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'a = RAM:u8[effective_address]',
            'b = ${MOD_RM_REG8}',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = alu_result',
          ],
        },
      },
      opcode: [Opcodes.ADD_EB_RB, 'ModRM_rm_reg8_10', 'DISP_i16'],
      operands: ['rm', 'reg'],
      operandSize: 8,
      cycles: 7,
    },
    {
      operation: [
        'a = ${MOD_RM_RM8}',
        'b = ${MOD_RM_REG8}',
        '${ALU8_OP}',
        '${MOD_RM_RM8} = alu_result',
      ],
      opcode: [Opcodes.ADD_EB_RB, 'ModRM_rm8_reg8_11'],
      operands: ['rm', 'reg'],
      operandSize: 8,
      cycles: 2,
    },
    // 0x01 /r - ADD ew, rw
    {
      modes: {
        real: {
          operation: [
            'offset = %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'a = RAM:u16[effective_address]',
            'b = ${MOD_RM_REG16}',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = alu_result',
          ],
        },
        protected: {
          operation: [
            'offset = %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'a = RAM:u16[effective_address]',
            'b = ${MOD_RM_REG16}',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = alu_result',
          ],
        },
      },
      opcode: [Opcodes.ADD_EW_RW, 'ModRM_110_reg16_00', 'DISP_i16'],
      operands: ['rm', 'reg'],
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
            'a = RAM:u16[effective_address]',
            'b = ${MOD_RM_REG16}',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = alu_result',
          ],
        },
        protected: {
          operation: [
            'offset = ${MOD_RM_OFFSET}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'a = RAM:u16[effective_address]',
            'b = ${MOD_RM_REG16}',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = alu_result',
          ],
        },
      },
      opcode: [Opcodes.ADD_EW_RW, 'ModRM_rm_reg16_00'],
      operands: ['rm', 'reg'],
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
            'a = RAM:u16[effective_address]',
            'b = ${MOD_RM_REG16}',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = alu_result',
          ],
        },
        protected: {
          operation: [
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'a = RAM:u16[effective_address]',
            'b = ${MOD_RM_REG16}',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = alu_result',
          ],
        },
      },
      opcode: [Opcodes.ADD_EW_RW, 'ModRM_rm_reg16_01', 'DISP_i8'],
      operands: ['rm', 'reg'],
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
            'a = RAM:u16[effective_address]',
            'b = ${MOD_RM_REG16}',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = alu_result',
          ],
        },
        protected: {
          operation: [
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'a = RAM:u16[effective_address]',
            'b = ${MOD_RM_REG16}',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = alu_result',
          ],
        },
      },
      opcode: [Opcodes.ADD_EW_RW, 'ModRM_rm_reg16_10', 'DISP_i16'],
      operands: ['rm', 'reg'],
      operandSize: 16,
      cycles: 7,
    },
    {
      operation: [
        'a = ${MOD_RM_RM16}',
        'b = ${MOD_RM_REG16}',
        '${ALU16_OP}',
        '${MOD_RM_RM16} = alu_result',
      ],
      opcode: [Opcodes.ADD_EW_RW, 'ModRM_rm16_reg16_11'],
      operands: ['rm', 'reg'],
      operandSize: 16,
      cycles: 2,
    },
    // 0x02 /r - ADD rb, eb
    {
      modes: {
        real: {
          operation: [
            'effective_address = ${MOD_RM_SEGMENT} + %{DISP}',
            'a = ${MOD_RM_REG8}',
            'b = RAM:u8[effective_address]',
            '${ALU8_OP}',
            '${MOD_RM_REG8} = alu_result',
          ],
        },
        protected: {
          operation: [
            'offset = %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'a = ${MOD_RM_REG8}',
            'b = RAM:u8[effective_address]',
            '${ALU8_OP}',
            '${MOD_RM_REG8} = alu_result',
          ],
        },
      },
      opcode: [Opcodes.ADD_RB_EB, 'ModRM_110_reg8_00', 'DISP_i16'],
      operands: ['reg', 'rm'],
      operandSize: 8,
      cycles: 7,
    },
    {
      modes: {
        real: {
          operation: [
            'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET}',
            'a = ${MOD_RM_REG8}',
            'b = RAM:u8[effective_address]',
            '${ALU8_OP}',
            '${MOD_RM_REG8} = alu_result',
          ],
        },
        protected: {
          operation: [
            'offset = ${MOD_RM_OFFSET}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'a = ${MOD_RM_REG8}',
            'b = RAM:u8[effective_address]',
            '${ALU8_OP}',
            '${MOD_RM_REG8} = alu_result',
          ],
        },
      },
      opcode: [Opcodes.ADD_RB_EB, 'ModRM_rm_reg8_00'],
      operands: ['reg', 'rm'],
      operandSize: 8,
      cycles: 7,
    },
    {
      modes: {
        real: {
          operation: [
            'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
            'a = ${MOD_RM_REG8}',
            'b = RAM:u8[effective_address]',
            '${ALU8_OP}',
            '${MOD_RM_REG8} = alu_result',
          ],
        },
        protected: {
          operation: [
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'a = ${MOD_RM_REG8}',
            'b = RAM:u8[effective_address]',
            '${ALU8_OP}',
            '${MOD_RM_REG8} = alu_result',
          ],
        },
      },
      opcode: [Opcodes.ADD_RB_EB, 'ModRM_rm_reg8_01', 'DISP_i8'],
      operands: ['reg', 'rm'],
      operandSize: 8,
      cycles: 7,
    },
    {
      modes: {
        real: {
          operation: [
            'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
            'a = ${MOD_RM_REG8}',
            'b = RAM:u8[effective_address]',
            '${ALU8_OP}',
            '${MOD_RM_REG8} = alu_result',
          ],
        },
        protected: {
          operation: [
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'a = ${MOD_RM_REG8}',
            'b = RAM:u8[effective_address]',
            '${ALU8_OP}',
            '${MOD_RM_REG8} = alu_result',
          ],
        },
      },
      opcode: [Opcodes.ADD_RB_EB, 'ModRM_rm_reg8_10', 'DISP_i16'],
      operands: ['reg', 'rm'],
      operandSize: 8,
      cycles: 7,
    },
    {
      operation: [
        'a = ${MOD_RM_REG8}',
        'b = ${MOD_RM_RM8}',
        '${ALU8_OP}',
        '${MOD_RM_REG8} = alu_result',
      ],
      opcode: [Opcodes.ADD_RB_EB, 'ModRM_rm8_reg8_11'],
      operands: ['reg', 'rm'],
      operandSize: 8,
      cycles: 2,
    },
    // 0x03 /r - ADD rw, ew
    {
      modes: {
        real: {
          operation: [
            'offset = %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'a = ${MOD_RM_REG16}',
            'b = RAM:u16[effective_address]',
            '${ALU16_OP}',
            '${MOD_RM_REG16} = alu_result',
          ],
        },
        protected: {
          operation: [
            'offset = %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'a = ${MOD_RM_REG16}',
            'b = RAM:u16[effective_address]',
            '${ALU16_OP}',
            '${MOD_RM_REG16} = alu_result',
          ],
        },
      },
      opcode: [Opcodes.ADD_RW_EW, 'ModRM_110_reg16_00', 'DISP_i16'],
      operands: ['reg', 'rm'],
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
            'a = ${MOD_RM_REG16}',
            'b = RAM:u16[effective_address]',
            '${ALU16_OP}',
            '${MOD_RM_REG16} = alu_result',
          ],
        },
        protected: {
          operation: [
            'offset = ${MOD_RM_OFFSET}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'a = ${MOD_RM_REG16}',
            'b = RAM:u16[effective_address]',
            '${ALU16_OP}',
            '${MOD_RM_REG16} = alu_result',
          ],
        },
      },
      opcode: [Opcodes.ADD_RW_EW, 'ModRM_rm_reg16_00'],
      operands: ['reg', 'rm'],
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
            'a = ${MOD_RM_REG16}',
            'b = RAM:u16[effective_address]',
            '${ALU16_OP}',
            '${MOD_RM_REG16} = alu_result',
          ],
        },
        protected: {
          operation: [
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'a = ${MOD_RM_REG16}',
            'b = RAM:u16[effective_address]',
            '${ALU16_OP}',
            '${MOD_RM_REG16} = alu_result',
          ],
        },
      },
      opcode: [Opcodes.ADD_RW_EW, 'ModRM_rm_reg16_01', 'DISP_i8'],
      operands: ['reg', 'rm'],
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
            'a = ${MOD_RM_REG16}',
            'b = RAM:u16[effective_address]',
            '${ALU16_OP}',
            '${MOD_RM_REG16} = alu_result',
          ],
        },
        protected: {
          operation: [
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'a = ${MOD_RM_REG16}',
            'b = RAM:u16[effective_address]',
            '${ALU16_OP}',
            '${MOD_RM_REG16} = alu_result',
          ],
        },
      },
      opcode: [Opcodes.ADD_RW_EW, 'ModRM_rm_reg16_10', 'DISP_i16'],
      operands: ['reg', 'rm'],
      operandSize: 16,
      cycles: 7,
    },
    {
      operation: [
        'a = ${MOD_RM_REG16}',
        'b = ${MOD_RM_RM16}',
        '${ALU16_OP}',
        '${MOD_RM_REG16} = alu_result',
      ],
      opcode: [Opcodes.ADD_RW_EW, 'ModRM_rm16_reg16_11'],
      operands: ['reg', 'rm'],
      operandSize: 16,
      cycles: 2,
    },
    // 0x04 db - ADD AL, db
    {
      operation: ['a = AL', 'b = %{imm}', '${ALU8_OP}', 'AL = alu_result'],
      opcode: [Opcodes.ADD_AL_DB, 'IMM_u8'],
      operands: ['AL', 'imm'],
      operandSize: 8,
      encodingPriority: 1,
      cycles: 3,
    },
    // 0x05 dw - ADD AX, dw
    {
      operation: ['a = AX', 'b = %{imm}', '${ALU16_OP}', 'AX = alu_result'],
      opcode: [Opcodes.ADD_AX_DW, 'IMM_u16'],
      operands: ['AX', 'imm'],
      operandSize: 16,
      encodingPriority: 1,
      cycles: 3,
    },
    // 0x80 /0 db - ADD eb, db
    {
      modes: {
        real: {
          operation: [
            'effective_address = ${MOD_RM_SEGMENT} + %{DISP}',
            'a = RAM:u8[effective_address]',
            'b = %{imm}',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = alu_result',
          ],
        },
        protected: {
          operation: [
            'offset = %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'a = RAM:u8[effective_address]',
            'b = %{imm}',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = alu_result',
          ],
        },
      },
      opcode: [Opcodes.ALU_EB_DB, 'ModRM_110_000_00', 'DISP_i16', 'IMM_u8'],
      operands: ['rm', 'imm'],
      operandSize: 8,
      cycles: 7,
    },
    {
      modes: {
        real: {
          operation: [
            'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET}',
            'a = RAM:u8[effective_address]',
            'b = %{imm}',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = alu_result',
          ],
        },
        protected: {
          operation: [
            'offset = ${MOD_RM_OFFSET}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'a = RAM:u8[effective_address]',
            'b = %{imm}',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = alu_result',
          ],
        },
      },
      opcode: [Opcodes.ALU_EB_DB, 'ModRM_rm_000_00', 'IMM_u8'],
      operands: ['rm', 'imm'],
      operandSize: 8,
      cycles: 7,
    },
    {
      modes: {
        real: {
          operation: [
            'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
            'a = RAM:u8[effective_address]',
            'b = %{imm}',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = alu_result',
          ],
        },
        protected: {
          operation: [
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'a = RAM:u8[effective_address]',
            'b = %{imm}',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = alu_result',
          ],
        },
      },
      opcode: [Opcodes.ALU_EB_DB, 'ModRM_rm_000_01', 'DISP_i8', 'IMM_u8'],
      operands: ['rm', 'imm'],
      operandSize: 8,
      cycles: 7,
    },
    {
      modes: {
        real: {
          operation: [
            'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
            'a = RAM:u8[effective_address]',
            'b = %{imm}',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = alu_result',
          ],
        },
        protected: {
          operation: [
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'a = RAM:u8[effective_address]',
            'b = %{imm}',
            '${ALU8_OP}',
            'RAM:u8[effective_address] = alu_result',
          ],
        }
      },
      opcode: [Opcodes.ALU_EB_DB, 'ModRM_rm_000_10', 'DISP_i16', 'IMM_u8'],
      operands: ['rm', 'imm'],
      operandSize: 8,
      cycles: 7,
    },
    {
      operation: [
        'a = ${MOD_RM_RM8}',
        'b = %{imm}',
        '${ALU8_OP}',
        '${MOD_RM_RM8} = alu_result',
      ],
      opcode: [Opcodes.ALU_EB_DB, 'ModRM_rm8_000_11', 'IMM_u8'],
      operands: ['rm', 'imm'],
      operandSize: 8,
      cycles: 3,
    },
    // 0x81 /0 dw - ADD ew, dw
    {
      modes: {
        real: {
          operation: [
            'offset = %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'a = RAM:u16[effective_address]',
            'b = %{imm}',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = alu_result',
          ],
        },
        protected: {
          operation: [
            'offset = %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'a = RAM:u16[effective_address]',
            'b = %{imm}',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = alu_result',
          ],
        },
      },
      opcode: [Opcodes.ALU_EW_DW, 'ModRM_110_000_00', 'DISP_i16', 'IMM_u16'],
      operands: ['rm', 'imm'],
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
            'a = RAM:u16[effective_address]',
            'b = %{imm}',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = alu_result',
          ],
        },
        protected: {
          operation: [
            'offset = ${MOD_RM_OFFSET}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'a = RAM:u16[effective_address]',
            'b = %{imm}',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = alu_result',
          ],
        },
      },
      opcode: [Opcodes.ALU_EW_DW, 'ModRM_rm_000_00', 'IMM_u16'],
      operands: ['rm', 'imm'],
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
            'a = RAM:u16[effective_address]',
            'b = %{imm}',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = alu_result',
          ],
        },
        protected: {
          operation: [
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'a = RAM:u16[effective_address]',
            'b = %{imm}',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = alu_result',
          ],
        },
      },
      opcode: [Opcodes.ALU_EW_DW, 'ModRM_rm_000_01', 'DISP_i8', 'IMM_u16'],
      operands: ['rm', 'imm'],
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
            'a = RAM:u16[effective_address]',
            'b = %{imm}',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = alu_result',
          ],
        },
        protected: {
          operation: [
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'a = RAM:u16[effective_address]',
            'b = %{imm}',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = alu_result',
          ],
        },
      },
      opcode: [Opcodes.ALU_EW_DW, 'ModRM_rm_000_10', 'DISP_i16', 'IMM_u16'],
      operands: ['rm', 'imm'],
      operandSize: 16,
      cycles: 7,
    },
    {
      operation: [
        'a = ${MOD_RM_RM16}',
        'b = %{imm}',
        '${ALU16_OP}',
        '${MOD_RM_RM16} = alu_result',
      ],
      opcode: [Opcodes.ALU_EW_DW, 'ModRM_rm16_000_11', 'IMM_u16'],
      operands: ['rm', 'imm'],
      operandSize: 16,
      cycles: 3,
    },
    // 0x83 /0 db - ADD ew, db
    {
      modes: {
        real: {
          operation: [
            'offset = %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'a = RAM:u16[effective_address]',
            'b = %{imm}',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = alu_result',
          ],
        },
        protected: {
          operation: [
            'offset = %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'a = RAM:u16[effective_address]',
            'b = %{imm}',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = alu_result',
          ],
        },
      },
      opcode: [Opcodes.ALU_EW_DB, 'ModRM_110_000_00', 'DISP_i16', 'IMM_i8'],
      operands: ['rm', 'imm'],
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
            'a = RAM:u16[effective_address]',
            'b = %{imm}',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = alu_result',
          ],
        },
        protected: {
          operation: [
            'offset = ${MOD_RM_OFFSET}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'a = RAM:u16[effective_address]',
            'b = %{imm}',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = alu_result',
          ],
        },
      },
      opcode: [Opcodes.ALU_EW_DB, 'ModRM_rm_000_00', 'IMM_i8'],
      operands: ['rm', 'imm'],
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
            'a = RAM:u16[effective_address]',
            'b = %{imm}',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = alu_result',
          ],
        },
        protected: {
          operation: [
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'a = RAM:u16[effective_address]',
            'b = %{imm}',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = alu_result',
          ],
        },
      },
      opcode: [Opcodes.ALU_EW_DB, 'ModRM_rm_000_01', 'DISP_i8', 'IMM_i8'],
      operands: ['rm', 'imm'],
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
            'a = RAM:u16[effective_address]',
            'b = %{imm}',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = alu_result',
          ],
        },
        protected: {
          operation: [
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'a = RAM:u16[effective_address]',
            'b = %{imm}',
            '${ALU16_OP}',
            'RAM:u16[effective_address] = alu_result',
          ],
        },
      },
      opcode: [Opcodes.ALU_EW_DB, 'ModRM_rm_000_10', 'DISP_i16', 'IMM_i8'],
      operands: ['rm', 'imm'],
      operandSize: 16,
      cycles: 7,
    },
    {
      operation: [
        'a = ${MOD_RM_RM16}',
        'b = %{imm}',
        '${ALU16_OP}',
        '${MOD_RM_RM16} = alu_result',
      ],
      opcode: [Opcodes.ALU_EW_DB, 'ModRM_rm16_000_11', 'IMM_i8'],
      operands: ['rm', 'imm'],
      operandSize: 16,
      cycles: 3,
    },
  ],
};
