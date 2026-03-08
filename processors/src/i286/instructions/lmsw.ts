import type { InstructionInfo } from '@machinery/core';

import { Opcodes, SystemOpcodes } from '../opcodes';

// TODO: #13 for operand 0xffff
export const lmsw: InstructionInfo = {
  identifier: 'lmsw',
  name: 'Load Machine Status Word',
  description:
    'The Machine Status Word (`LMSW` or referred to as `CR0` in modern systems) is loaded from the source operand. This instruction may be used to switch to protected mode. If so, then it must be followed by an intra-segment jump to flush the instruction queue. `LMSW` will not switch back to Real Address Mode.\n\n`LMSW` appears only in operating systems software. It does not appear in applications programs.',
  modifies: [],
  undefined: [],
  locals: [
    {
      identifier: 'effective_address',
      name: 'Effective Address',
      size: 32,
    },
  ],
  forms: [
    // 0x0F 0x01 /6 - LMSW ew
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + %{DISP}',
        'MSW = RAM:u16[effective_address]',
      ],
      opcode: [
        Opcodes.SYSTEM,
        SystemOpcodes.SGDT_SIDT_LMSW_SMSW_LGDT_LIDT,
        'ModRM_110_110_00',
        'DISP_i16',
      ],
      operandSize: 16,
      cycles: 6,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET}',
        'MSW = RAM:u16[effective_address]',
      ],
      opcode: [
        Opcodes.SYSTEM,
        SystemOpcodes.SGDT_SIDT_LMSW_SMSW_LGDT_LIDT,
        'ModRM_rm_110_00',
      ],
      operandSize: 16,
      cycles: 6,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'MSW = RAM:u16[effective_address]',
      ],
      opcode: [
        Opcodes.SYSTEM,
        SystemOpcodes.SGDT_SIDT_LMSW_SMSW_LGDT_LIDT,
        'ModRM_rm_110_01',
        'DISP_i8',
      ],
      operandSize: 16,
      cycles: 6,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'MSW = RAM:u16[effective_address]',
      ],
      opcode: [
        Opcodes.SYSTEM,
        SystemOpcodes.SGDT_SIDT_LMSW_SMSW_LGDT_LIDT,
        'ModRM_rm_110_10',
        'DISP_i16',
      ],
      operandSize: 16,
      cycles: 6,
    },
    {
      operation: ['MSW = ${MOD_RM_RM16}'],
      opcode: [
        Opcodes.SYSTEM,
        SystemOpcodes.SGDT_SIDT_LMSW_SMSW_LGDT_LIDT,
        'ModRM_rm_110_11',
      ],
      operandSize: 16,
      cycles: 3,
    },
  ],
};
