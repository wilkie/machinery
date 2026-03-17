import type { InstructionInfo } from '@machinery/core';
import { InstructionDataTypes, InstructionOperandTypes } from '@machinery/core';

import { Opcodes, SystemOpcodes } from '../opcodes';

// TODO: #13 for offset of 0xffff
export const pop: InstructionInfo = {
  identifier: 'pop',
  name: 'Pop a Word from the Stack',
  description:
    "The word on the top of the stack, addressed by `SS:SP`, replaces the previous contents of the memory, register, or segment register operand. The stack pointer `SP` is incremented by 2 to point to the new top of stack.\n\nIf the destination operand is another segment register (`DS`, `ES`, `SS`, `FS`, or `GS`), the value popped must be a selector. In protected mode, loading the selector initiates automatic loading of the descriptor information associated with that selector into the hidden part of the segment register; loading also initiates validation of both the selector and the descriptor information.\n\nA null value (0000-0003) may be loaded into the `DS` or `ES` register without causing a protection exception. Attempts to reference memory using a segment register with a null value will cause `#GP(0)` exception. No memory reference will occur. The saved value of the segment register will be null.\n\nA `POP SS` instruction will inhibit all interrupts, including `NMI`, until after the execution of the next instruction. This permits a `POP SP` instruction to be performed first.\n\nFollowing is a listing of the protected-mode checks and actions taken in the loading of a segment register:\n\n```\nIf SS is loaded:\n  If selector is null then #GP(0)\n  Selector index must be within its descriptor table limits else #GP (selector)\n  Selector's RPL must equal CPL else #GP (selector)\n  AR byte must indicate a writable data segment else #GP (selector)\n  DPL in the AR byte must equal CPL else #GP (selector)\n  Segment must be marked PRESENT else #SS (selector)\n  Load SS register with selector\n  Load SS cache with descriptor\nIf ES or DS is loaded with non-null selector:\n  AR byte must indicate data or readable code segment else #GP (selector)\n  If data or non-conforming code, then both the RPL and the\n    CPL must be less than or equal to DPL in AR byte else #GP (selector)\n  Segment must be marked PRESENT else #NP (selector)\n  Load segment register with selector\n  Load segment register cache with descriptor\nIf ES or DS is loaded with a null selector:\n  Load segment register with selector\n  Clear valid bit in cache\n```",
  modifies: [],
  undefined: [],
  macros: {
    OP_REAL: [
      'offset = SP',
      'effective_address = SS_BASE + offset',
      '#GP if offset == 0xffff',
      'value = RAM:u16[effective_address]',
      'SP = SP + 2',
    ],
    OP_PROTECTED: [
      'offset = SP',
      'effective_address = SS_BASE + offset',
      '#GP if (offset + 1) < SS_LIMIT_MIN',
      '#GP if (offset + 1) > SS_LIMIT_MAX',
      'value = RAM:u16[effective_address]',
      'SP = SP + 2',
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
      identifier: 'value',
      name: 'Resulting Value',
      size: 16,
    },
  ],
  forms: [
    // 0x07 - POP ES
    {
      opcode: [Opcodes.POP_ES],
      operands: ['ES'],
      modes: {
        real: {
          operation: ['${OP_REAL}', 'ES = value'],
        },
        protected: {
          operation: ['${OP_PROTECTED}', 'ES = value'],
        },
      },
      cycles: 5, // protected-mode: 20
    },
    // 0x17 - POP SS
    {
      opcode: [Opcodes.POP_SS],
      operands: ['SS'],
      modes: {
        real: {
          operation: [
            '${OP_REAL}',
            'SS = value',
            // TODO: Inhibit interrupts...including NMI... for the next instruction somehow
            // ----  Basically, do not allow them when the current instruction is immediately after a pop ss?
          ],
        },
        protected: {
          operation: ['${OP_PROTECTED}', 'SS = value'],
        },
      },
      cycles: 5, // protected-mode: 20
    },
    // 0x1F - POP DS
    {
      opcode: [Opcodes.POP_DS],
      operands: ['DS'],
      modes: {
        real: {
          operation: ['${OP_REAL}', 'DS = value'],
        },
        protected: {
          operation: ['${OP_PROTECTED}', 'DS = value'],
        },
      },
      cycles: 5, // protected-mode: 20
    },
    // 0x0F 0xA1 - POP FS
    {
      opcode: [Opcodes.SYSTEM, SystemOpcodes.POP_FS],
      operands: ['FS'],
      modes: {
        real: {
          operation: ['${OP_REAL}', 'FS = value'],
        },
        protected: {
          operation: ['${OP_PROTECTED}', 'FS = value'],
        },
      },
      cycles: 5,
    },
    // 0x0F 0xA9 - POP GS
    {
      opcode: [Opcodes.SYSTEM, SystemOpcodes.POP_GS],
      operands: ['GS'],
      modes: {
        real: {
          operation: ['${OP_REAL}', 'GS = value'],
        },
        protected: {
          operation: ['${OP_PROTECTED}', 'GS = value'],
        },
      },
      cycles: 5,
    },
    // 0x58+rw - POP rw
    {
      modes: {
        real: {
          operation: ['${OP_REAL}', '${MOD_RM_RM16} = value'],
        },
        protected: {
          operation: ['${OP_PROTECTED}', '${MOD_RM_RM16} = value'],
        },
      },
      opcode: [
        {
          identifier: 'OpcodeRM',
          name: 'POP Opcode Field',
          type: InstructionDataTypes.Operand,
          size: 8,
          fields: [
            {
              identifier: 'opcode',
              offset: 3,
              size: 5,
              match: 0b01011,
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
      operands: ['rm'],
      operandSize: 16,
      cycles: 5,
    },
    // 0x8F /0 - POP mw
    {
      modes: {
        real: {
          operation: [
            '${OP_REAL}',
            'offset = %{DISP:u16}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'RAM:u16[effective_address] = value',
          ],
        },
        protected: {
          operation: [
            '${OP_PROTECTED}',
            'offset = %{DISP:u16}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'RAM:u16[effective_address] = value',
          ],
        },
      },
      opcode: [Opcodes.POP_MW, 'ModRM_110_000_00', 'DISP_i16'],
      operands: ['rm'],
      operandSize: 16,
      cycles: 5,
    },
    {
      modes: {
        real: {
          operation: [
            '${OP_REAL}',
            'offset = (${MOD_RM_OFFSET}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'RAM:u16[effective_address] = value',
          ],
        },
        protected: {
          operation: [
            '${OP_PROTECTED}',
            'offset = (${MOD_RM_OFFSET}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'RAM:u16[effective_address] = value',
          ],
        },
      },
      opcode: [Opcodes.POP_MW, 'ModRM_rm_000_00'],
      operands: ['rm'],
      operandSize: 16,
      cycles: 5,
    },
    {
      modes: {
        real: {
          operation: [
            '${OP_REAL}',
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'RAM:u16[effective_address] = value',
          ],
        },
        protected: {
          operation: [
            '${OP_PROTECTED}',
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'RAM:u16[effective_address] = value',
          ],
        },
      },
      opcode: [Opcodes.POP_MW, 'ModRM_rm_000_01', 'DISP_i8'],
      operands: ['rm'],
      operandSize: 16,
      cycles: 5,
    },
    {
      modes: {
        real: {
          operation: [
            '${OP_REAL}',
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'RAM:u16[effective_address] = value',
          ],
        },
        protected: {
          operation: [
            '${OP_PROTECTED}',
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'RAM:u16[effective_address] = value',
          ],
        },
      },
      opcode: [Opcodes.POP_MW, 'ModRM_rm_000_10', 'DISP_i16'],
      operands: ['rm'],
      operandSize: 16,
      cycles: 5,
    },
    {
      modes: {
        real: {
          operation: ['${OP_REAL}', '${MOD_RM_RM16} = value'],
        },
        protected: {
          operation: ['${OP_PROTECTED}', '${MOD_RM_RM16} = value'],
        },
      },
      opcode: [Opcodes.POP_MW, 'ModRM_rm16_000_11'],
      operands: ['rm'],
      operandSize: 16,
      cycles: 3,
    },
  ],
};
