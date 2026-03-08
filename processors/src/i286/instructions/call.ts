import type { InstructionInfo } from '@machinery/core';
import { InstructionDataType } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const call: InstructionInfo = {
  identifier: 'call',
  name: 'Call',
  description: '',
  modifies: [],
  undefined: [],
  locals: [
    {
      identifier: 'effective_address',
      name: 'Effective Address',
      size: 32,
    },
    {
      identifier: 'stack_address',
      name: 'Effective Stack Address',
      size: 32,
    },
    {
      identifier: 'tmp',
      name: 'Temporary register',
      size: 16,
    },
  ],
  forms: [
    // 0xe8 cw - CALL cw
    {
      operation: [
        'tmp = SP - 2',
        'stack_address = SS_BASE + tmp',
        'RAM:u16[stack_address] = IP',
        'SP = tmp',
        'IP = IP + %{IMM}',
      ],
      opcode: [Opcodes.CALL_CW, 'IMM_i16'],
      operandSize: 8,
      cycles: 7,
    },
    // 0xFF /2 - CALL ew
    {
      operation: [
        'tmp = SP - 2',
        'stack_address = SS_BASE + tmp',
        'RAM:u16[stack_address] = IP',
        'SP = tmp',
        'effective_address = ${MOD_RM_SEGMENT} + %{DISP}',
        'IP = RAM:u16[effective_address]',
      ],
      opcode: [Opcodes.CALL_JMP_INC_DEC_PUSH, 'ModRM_110_010_00', 'DISP_i16'],
      operandSize: 16,
      cycles: 11,
    },
    {
      operation: [
        'tmp = SP - 2',
        'stack_address = SS_BASE + tmp',
        'RAM:u16[stack_address] = IP',
        'SP = tmp',
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET}',
        'IP = RAM:u16[effective_address]',
      ],
      opcode: [Opcodes.CALL_JMP_INC_DEC_PUSH, 'ModRM_rm_010_00'],
      operandSize: 16,
      cycles: 11,
    },
    {
      operation: [
        'tmp = SP - 2',
        'stack_address = SS_BASE + tmp',
        'RAM:u16[stack_address] = IP',
        'SP = tmp',
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'IP = RAM:u16[effective_address]',
      ],
      opcode: [Opcodes.CALL_JMP_INC_DEC_PUSH, 'ModRM_rm_010_01', 'DISP_i8'],
      operandSize: 16,
      cycles: 11,
    },
    {
      operation: [
        'tmp = SP - 2',
        'stack_address = SS_BASE + tmp',
        'RAM:u16[stack_address] = IP',
        'SP = tmp',
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'IP = RAM:u16[effective_address]',
      ],
      opcode: [Opcodes.CALL_JMP_INC_DEC_PUSH, 'ModRM_rm_010_10', 'DISP_i16'],
      operandSize: 16,
      cycles: 11,
    },
    {
      operation: [
        'tmp = SP - 2',
        'stack_address = SS_BASE + tmp',
        'RAM:u16[stack_address] = IP',
        'SP = tmp',
        'IP = ${MOD_RM_RM16}',
      ],
      opcode: [Opcodes.CALL_JMP_INC_DEC_PUSH, 'ModRM_rm_010_11'],
      operandSize: 16,
      cycles: 7,
    },
    // 0x9a cd - CALL cd
    // TODO: deal with selector access rights bits (AR)
    // TODO: use a setter for the segment selector so we can get the base and raise exceptions
    {
      operation: [
        'tmp = SP - 4',
        'stack_address = SS_BASE + tmp',
        'RAM:u16[stack_address] = IP',
        'RAM:u16[stack_address + 2] = CS',
        'SP = tmp',
        'CS = NEW_CS',
        'IP = NEW_IP',
      ],
      opcode: [
        Opcodes.CALL_CD,
        {
          identifier: 'NEW_IP',
          name: 'New IP Value',
          type: InstructionDataType.Immediate,
          size: 16,
        },
        {
          identifier: 'NEW_CS',
          name: 'New CS Value',
          type: InstructionDataType.Immediate,
          size: 16,
        },
      ],
      operandSize: 8,
      cycles: 7,
    },
    // 0xFF /3 - CALL ed
    // TODO: deal with selector access rights bits (AR)
    // TODO: use a setter for the segment selector so we can get the base and raise exceptions
    {
      operation: [
        'tmp = SP - 4',
        'stack_address = SS_BASE + tmp',
        'RAM:u16[stack_address] = IP',
        'RAM:u16[stack_address + 2] = CS',
        'SP = tmp',
        'effective_address = ${MOD_RM_SEGMENT} + %{DISP}',
        'IP = RAM:u16[effective_address]',
        'CS = RAM:u16[effective_address + 2]',
      ],
      opcode: [Opcodes.CALL_JMP_INC_DEC_PUSH, 'ModRM_110_011_00', 'DISP_i16'],
      operandSize: 16,
      cycles: 11,
    },
    {
      operation: [
        'tmp = SP - 4',
        'stack_address = SS_BASE + tmp',
        'RAM:u16[stack_address] = IP',
        'RAM:u16[stack_address + 2] = CS',
        'SP = tmp',
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET}',
        'IP = RAM:u16[effective_address]',
        'CS = RAM:u16[effective_address + 2]',
      ],
      opcode: [Opcodes.CALL_JMP_INC_DEC_PUSH, 'ModRM_rm_011_00'],
      operandSize: 16,
      cycles: 11,
    },
    {
      operation: [
        'tmp = SP - 4',
        'stack_address = SS_BASE + tmp',
        'RAM:u16[stack_address] = IP',
        'RAM:u16[stack_address + 2] = CS',
        'SP = tmp',
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'IP = RAM:u16[effective_address]',
        'CS = RAM:u16[effective_address + 2]',
      ],
      opcode: [Opcodes.CALL_JMP_INC_DEC_PUSH, 'ModRM_rm_011_01', 'DISP_i8'],
      operandSize: 16,
      cycles: 11,
    },
    {
      operation: [
        'tmp = SP - 4',
        'stack_address = SS_BASE + tmp',
        'RAM:u16[stack_address] = IP',
        'RAM:u16[stack_address + 2] = CS',
        'SP = tmp',
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'IP = RAM:u16[effective_address]',
        'CS = RAM:u16[effective_address + 2]',
      ],
      opcode: [Opcodes.CALL_JMP_INC_DEC_PUSH, 'ModRM_rm_011_10', 'DISP_i16'],
      operandSize: 16,
      cycles: 11,
    },
    {
      operation: [
        'tmp = SP - 4',
        'stack_address = SS_BASE + tmp',
        'RAM:u16[stack_address] = IP',
        'RAM:u16[stack_address + 2] = CS',
        'SP = tmp',
        'effective_address = ${MOD_RM_RM16}',
        'IP = RAM:u16[effective_address]',
        'CS = RAM:u16[effective_address + 2]',
      ],
      opcode: [Opcodes.CALL_JMP_INC_DEC_PUSH, 'ModRM_rm_011_11'],
      operandSize: 16,
      cycles: 7,
    },
  ],
};
