import type { InstructionInfo } from '@rawrs/architecture/Target';

import { Opcodes } from '../opcodes';

// TODO: #13 for offset 0xffff
export const not: InstructionInfo = {
  identifier: 'not',
  name: "One's Complement Negation",
  description:
    'The operand is inverted; that is, every 1 becomes a 0 and vice versa.',
  modifies: [],
  undefined: [],
  locals: [
    {
      identifier: 'effective_address',
      name: 'Effective Address',
      size: 32,
    },
    {
      identifier: 'value',
      name: 'Resulting Value',
      size: 16,
    },
  ],
  forms: [
    // 0xf6 /2 - NOT eb
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + %{DISP}',
        'value = ~RAM:u8[effective_address]',
        'RAM:u8[effective_address] = value',
      ],
      opcode: [Opcodes.ALU_LOGIC_EB, 'ModRM_110_010_00', 'DISP_i16'],
      operandSize: 8,
      cycles: 7,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET}',
        'value = ~RAM:u8[effective_address]',
        'RAM:u8[effective_address] = value',
      ],
      opcode: [Opcodes.ALU_LOGIC_EB, 'ModRM_rm_010_00'],
      operandSize: 8,
      cycles: 7,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'value = ~RAM:u8[effective_address]',
        'RAM:u8[effective_address] = value',
      ],
      opcode: [Opcodes.ALU_LOGIC_EB, 'ModRM_rm_010_01', 'DISP_i8'],
      operandSize: 8,
      cycles: 7,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'value = ~RAM:u8[effective_address]',
        'RAM:u8[effective_address] = value',
      ],
      opcode: [Opcodes.ALU_LOGIC_EB, 'ModRM_rm_010_10', 'DISP_i16'],
      operandSize: 8,
      cycles: 7,
    },
    {
      operation: ['${MOD_RM_RM8} = ~${MOD_RM_RM8}'],
      opcode: [Opcodes.ALU_LOGIC_EB, 'ModRM_rm_010_11'],
      operandSize: 8,
      cycles: 2,
    },
    // 0xf7 /3 - NOT ew
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + %{DISP}',
        'value = ~RAM:u16[effective_address]',
        'RAM:u16[effective_address] = value',
      ],
      opcode: [Opcodes.ALU_LOGIC_EW, 'ModRM_110_010_00', 'DISP_i16'],
      operandSize: 16,
      cycles: 7,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET}',
        'value = ~RAM:u16[effective_address]',
        'RAM:u16[effective_address] = value',
      ],
      opcode: [Opcodes.ALU_LOGIC_EW, 'ModRM_rm_010_00'],
      operandSize: 16,
      cycles: 7,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'value = ~RAM:u16[effective_address]',
        'RAM:u16[effective_address] = value',
      ],
      opcode: [Opcodes.ALU_LOGIC_EW, 'ModRM_rm_010_01', 'DISP_i8'],
      operandSize: 16,
      cycles: 7,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'value = ~RAM:u16[effective_address]',
        'RAM:u16[effective_address] = value',
      ],
      opcode: [Opcodes.ALU_LOGIC_EW, 'ModRM_rm_010_10', 'DISP_i16'],
      operandSize: 16,
      cycles: 7,
    },
    {
      operation: ['${MOD_RM_RM16} = ~${MOD_RM_RM16}'],
      opcode: [Opcodes.ALU_LOGIC_EW, 'ModRM_rm_010_11'],
      operandSize: 16,
      cycles: 2,
    },
  ],
};
