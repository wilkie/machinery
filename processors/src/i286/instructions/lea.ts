import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const lea: InstructionInfo = {
  identifier: 'lea',
  name: 'Load Effective Address',
  description:
    'The effective address (offset part) of the second operand is placed in the first (register) operand.',
  modifies: [],
  undefined: [],
  forms: [
    // 0x8D /r - LEA rw, m
    {
      operation: ['${MOD_RM_REG16} = ${MOD_RM_SEGMENT} + %{DISP}'],
      opcode: [Opcodes.LEA, 'ModRM_110_reg16_00', 'DISP_IMM_i16'],
      operands: ['reg', 'rm'],
      operandSize: 16,
      cycles: 3,
    },
    {
      operation: ['${MOD_RM_REG16} = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET}'],
      opcode: [Opcodes.LEA, 'ModRM_rm_reg16_00'],
      operands: ['reg', 'rm'],
      operandSize: 16,
      cycles: 3,
    },
    {
      operation: [
        '${MOD_RM_REG16} = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
      ],
      opcode: [Opcodes.LEA, 'ModRM_rm_reg16_01', 'DISP_IMM_i8'],
      operands: ['reg', 'rm'],
      operandSize: 16,
      cycles: 3,
    },
    {
      operation: [
        '${MOD_RM_REG16} = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
      ],
      opcode: [Opcodes.LEA, 'ModRM_rm_reg16_10', 'DISP_IMM_i16'],
      operands: ['reg', 'rm'],
      operandSize: 16,
      cycles: 3,
    },
    {
      operation: ['${UD_EXCEPTION}'],
      opcode: [Opcodes.LEA, 'ModRM_rm16_reg16_11'],
      operands: ['reg', 'rm'],
      operandSize: 16,
      cycles: 1,
    },
  ],
};
