import type { InstructionInfo } from '@rawrs/architecture/Target';

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
      opcode: [Opcodes.LEA, 'ModRM_110_reg_00', 'DISP_i16'],
      operandSize: 16,
      cycles: 3,
    },
    {
      operation: ['${MOD_RM_REG16} = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET}'],
      opcode: [Opcodes.LEA, 'ModRM_rm_reg_00'],
      operandSize: 16,
      cycles: 3,
    },
    {
      operation: [
        '${MOD_RM_REG16} = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
      ],
      opcode: [Opcodes.LEA, 'ModRM_rm_reg_01', 'DISP_i8'],
      operandSize: 16,
      cycles: 3,
    },
    {
      operation: [
        '${MOD_RM_REG16} = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
      ],
      opcode: [Opcodes.LEA, 'ModRM_rm_reg_10', 'DISP_i16'],
      operandSize: 16,
      cycles: 3,
    },
    {
      operation: ['${UD_EXCEPTION}'],
      opcode: [Opcodes.LEA, 'ModRM_rm_reg_11'],
      operandSize: 16,
      cycles: 1,
    },
  ],
};
