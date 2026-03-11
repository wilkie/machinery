import type { InstructionInfo } from '@machinery/core';
import { InstructionDataTypes, InstructionOperandTypes } from '@machinery/core';

import { Opcodes } from '../opcodes';

// TODO: #13 for operand 0xffff
export const xchg: InstructionInfo = {
  identifier: 'xchg',
  name: 'Exchange Memory / Register with Register',
  description:
    'The two operands are exchanged. The order of the operands is immaterial. `BUS` `LOCK` is asserted for the duration of the exchange, regardless of the presence or absence of the `LOCK` prefix or `IOPL`.',
  commutative: true,
  modifies: [],
  undefined: [],
  locals: [
    {
      identifier: 'effective_address',
      name: 'Effective Address',
      size: 32,
    },
    {
      identifier: 'tmp',
      name: 'Temporary Value',
      size: 16,
    },
  ],
  forms: [
    // 0x86 /r - XCHG eb, rb
    // 0x86 /r - XCHG rb, eb
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + %{DISP}',
        'tmp = RAM:u8[effective_address]',
        'RAM:u8[effective_address] = ${MOD_RM_REG8}',
        '${MOD_RM_REG8} = tmp',
      ],
      opcode: [Opcodes.XCHG_EB_RB, 'ModRM_110_reg8_00', 'DISP_i16'],
      operands: ['rm', 'reg'],
      operandSize: 8,
      cycles: 5,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET}',
        'tmp = RAM:u8[effective_address]',
        'RAM:u8[effective_address] = ${MOD_RM_REG8}',
        '${MOD_RM_REG8} = tmp',
      ],
      opcode: [Opcodes.XCHG_EB_RB, 'ModRM_rm_reg8_00'],
      operands: ['rm', 'reg'],
      operandSize: 8,
      cycles: 5,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'tmp = RAM:u8[effective_address]',
        'RAM:u8[effective_address] = ${MOD_RM_REG8}',
        '${MOD_RM_REG8} = tmp',
      ],
      opcode: [Opcodes.XCHG_EB_RB, 'ModRM_rm_reg8_01', 'DISP_i8'],
      operands: ['rm', 'reg'],
      operandSize: 8,
      cycles: 5,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'tmp = RAM:u8[effective_address]',
        'RAM:u8[effective_address] = ${MOD_RM_REG8}',
        '${MOD_RM_REG8} = tmp',
      ],
      opcode: [Opcodes.XCHG_EB_RB, 'ModRM_rm_reg8_10', 'DISP_i16'],
      operands: ['rm', 'reg'],
      operandSize: 8,
      cycles: 5,
    },
    {
      operation: [
        'tmp = ${MOD_RM_REG8}',
        '${MOD_RM_REG8} = ${MOD_RM_RM8}',
        '${MOD_RM_RM8} = tmp',
      ],
      opcode: [Opcodes.XCHG_EB_RB, 'ModRM_rm8_reg8_11'],
      operands: ['reg', 'rm'],
      operandSize: 8,
      cycles: 3,
    },
    // 0x87 /r - XCHG ew, rw
    // 0x87 /r - XCHG rw, ew
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + %{DISP}',
        'tmp = RAM:u16[effective_address]',
        'RAM:u16[effective_address] = ${MOD_RM_REG16}',
        '${MOD_RM_REG16} = tmp',
      ],
      opcode: [Opcodes.XCHG_EW_RW, 'ModRM_110_reg16_00', 'DISP_i16'],
      operands: ['rm', 'reg'],
      operandSize: 16,
      cycles: 5,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET}',
        'tmp = RAM:u16[effective_address]',
        'RAM:u16[effective_address] = ${MOD_RM_REG16}',
        '${MOD_RM_REG16} = tmp',
      ],
      opcode: [Opcodes.XCHG_EW_RW, 'ModRM_rm_reg16_00'],
      operands: ['rm', 'reg'],
      operandSize: 16,
      cycles: 5,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'tmp = RAM:u16[effective_address]',
        'RAM:u16[effective_address] = ${MOD_RM_REG16}',
        '${MOD_RM_REG16} = tmp',
      ],
      opcode: [Opcodes.XCHG_EW_RW, 'ModRM_rm_reg16_01', 'DISP_i8'],
      operands: ['rm', 'reg'],
      operandSize: 16,
      cycles: 5,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'tmp = RAM:u16[effective_address]',
        'RAM:u16[effective_address] = ${MOD_RM_REG16}',
        '${MOD_RM_REG16} = tmp',
      ],
      opcode: [Opcodes.XCHG_EW_RW, 'ModRM_rm_reg16_10', 'DISP_i16'],
      operands: ['rm', 'reg'],
      operandSize: 16,
      cycles: 5,
    },
    {
      operation: [
        'tmp = ${MOD_RM_REG16}',
        '${MOD_RM_REG16} = ${MOD_RM_RM16}',
        '${MOD_RM_RM16} = tmp',
      ],
      opcode: [Opcodes.XCHG_EW_RW, 'ModRM_rm16_reg16_11'],
      operands: ['reg', 'rm'],
      operandSize: 16,
      cycles: 3,
    },
    // 0x90+rw - XCHG AX, rw
    // 0x90+rw - XCHG rw, AX
    {
      operation: ['tmp = AX', 'AX = ${MOD_RM_RM16}', '${MOD_RM_RM16} = tmp'],
      opcode: [
        {
          identifier: 'XCHG',
          name: 'XCHG ModRM Field',
          type: InstructionDataTypes.Operand,
          size: 8,
          fields: [
            {
              identifier: 'opcode',
              offset: 3,
              size: 5,
              match: 0b10010,
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
      operands: ['AX', 'rm'],
      operandSize: 16,
      cycles: 3,
    },
  ],
};
