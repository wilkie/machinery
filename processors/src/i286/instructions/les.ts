import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

// TODO: selector load logic
// TODO: protected mode
export const les: InstructionInfo = {
  identifier: 'les',
  name: 'Load Doubleword Pointer into ES',
  description:
    'The four-byte pointer at the memory location indicated by the second operand is loaded into a segment register and a word register. The first word of the pointer (the offset) is loaded into the register indicated by the first operand. The last word of the pointer (the selector) is loaded into the segment register (`DS` or `ES`) given by the instruction opcode.\n\nWhen the segment register is loaded, its associated cache is also loaded. The data for the cache is obtained from the descriptor table entry for the selector given.\n\nA null selector (values 0000-0003) can be loaded into `DS` or `ES` without a protection exception. Any memory reference using such a segment register value will cause a `#GP(0)` exception but will not result in a memory reference. The saved segment register value will be null.\n\nFollowing is a list of checks and actions taken when loading the `DS` or `ES` registers:\n\n```\nIf selector is non-null then:\n  Selector index must be within its descriptor table limits else #GP (selector)\n  Examine descriptor AR byte:\n\n    Data segment or readable non-conforming code segment\n      Descriptor DPL >= CPL else #GP (selector)\n      Descriptor DPL >= selector RPL else #GP (selector)\n    Readable conforming code segment\n      No DPL, RPL, or CPL checks\n    Else #GP (selector)\n  Segment must be present else #NP (selector)\n  Load registers from operand\n  Load segment register descriptor cache\nIf selector is null then:\n  Load registers from operand\n  Mark segment register cache as invalid\n```',
  modifies: [],
  undefined: [],
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
    // 0xC4 /r - LES rw, ed
    {
      modes: {
        real: {
          operation: [
            'offset = %{DISP:u16}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            '${MOD_RM_REG16} = RAM:u16[effective_address]',
            'ES = RAM:u16[effective_address + 2]',
          ],
        },
        protected: {
          operation: [
            'offset = %{DISP:u16}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            '${MOD_RM_REG16} = RAM:u16[effective_address]',
            'ES = RAM:u16[effective_address + 2]',
          ],
        },
      },
      opcode: [Opcodes.LES, 'ModRM_110_reg16_00', 'DISP_i16'],
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
            '${MOD_RM_REG16} = RAM:u16[effective_address]',
            'ES = RAM:u16[effective_address + 2]',
          ],
        },
        protected: {
          operation: [
            'offset = ${MOD_RM_OFFSET}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            '${MOD_RM_REG16} = RAM:u16[effective_address]',
            'ES = RAM:u16[effective_address + 2]',
          ],
        },
      },
      opcode: [Opcodes.LES, 'ModRM_rm_reg16_00'],
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
            '${MOD_RM_REG16} = RAM:u16[effective_address]',
            'ES = RAM:u16[effective_address + 2]',
          ],
        },
        protected: {
          operation: [
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            '${MOD_RM_REG16} = RAM:u16[effective_address]',
            'ES = RAM:u16[effective_address + 2]',
          ],
        },
      },
      opcode: [Opcodes.LES, 'ModRM_rm_reg16_01', 'DISP_i8'],
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
            '${MOD_RM_REG16} = RAM:u16[effective_address]',
            'ES = RAM:u16[effective_address + 2]',
          ],
        },
        protected: {
          operation: [
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            '${MOD_RM_REG16} = RAM:u16[effective_address]',
            'ES = RAM:u16[effective_address + 2]',
          ],
        },
      },
      opcode: [Opcodes.LES, 'ModRM_rm_reg16_10', 'DISP_i16'],
      operands: ['reg', 'rm'],
      operandSize: 16,
      cycles: 7,
    },
    {
      operation: ['${UD_EXCEPTION}'],
      opcode: [Opcodes.LES, 'ModRM_rm16_reg16_11'],
      operands: ['reg', 'rm'],
      operandSize: 16,
      cycles: 1,
    },
  ],
};
