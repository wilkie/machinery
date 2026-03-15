import type { InstructionInfo } from '@machinery/core';
import { InstructionDataTypes } from '@machinery/core';

import { Opcodes } from '../opcodes';

// Real-mode:
// #UD if indirect inter-segment jump operand is a register.

const CODE = `
JUMP FAR:
  If indirect then check access of EA doubleword #GP(0) or #55(0) if limit violation
  Destination selector is not null else #GP(0)
  Destination selector index is within its descriptor table limits else #GP (selector)
  Examine AR byte of destination selector for legal values:
    JUMP CONFORMING CODE SEGMENT:
      Descriptor DPL must be <= CPL else #GP (selector)
      Segment must be PRESENT else #NP (selector)
      IP must be in code segment limit else #GP(0)
      Load CS:IP from destination pointer
      Load CS-cache with new segment descriptor
    JUMP NONCONFORMING CODE SEGMENT:
      RPL of destination selector must be <= CPL else #GP (selector)
      Descriptor DPL must = CPL else #GP (selector)
      Segment must be PRESENT else #NP (selector)
      IP must be in code segment limit else #GP(0)
      Load CS:IP from destination pointer
      Load CS-cache with new segment descriptor
      Set RPL field of CS register to CPL
    JUMP TO CALL GATE:
      Descriptor DPL must be >= CPL else #GP (gate selector)
      Descriptor DPL must be >= gate selector RPL else #GP (gate selector)
      Gate must be PRESENT else #NP (gate selector)
      Examine selector to code segment given in call gate descriptor:
      Selector must not be null else #GP(0)
      Selector must be within its descriptor table limits else #GP (CS selector)
      Descriptor AR byte must indicate code segment else #GP (CS selector)
      If non-conforming, code segment descriptor DPL must = CPL else #GP (CS selector)
      If conforming, then code segment descriptor DPL must be <= CPL else #GP (CS selector)
      Code Segment must be PRESENT else #NP (CS selector)
      IP must be in code segment limit else #GP(0)
      Load CS:IP from call gate
      Load CS-cache with new code segment
      Set RPL of CS to CPL
    JUMP TASK GATE:
      Gate descriptor DPL must be >= CPL else #GP (gate selector)
      Gate descriptor DPL must be >= gate selector RPL else #GP (gate selector)
      Task Gate must be PRESENT else #NP (gate selector)
      Examine selector to TSS, given in Task Gate descriptor:
      Must specify global in the local/global bit else #GP (TSS selector)
      Index must be within GDT limits else #GP (TSS selector)
      Descriptor AR byte must specify available TSS (bottom bits 00001) else #GP (TSS selector)
      Task State Segment must be PRESENT else #NP (TSS selector)
      SWITCH_TASKS without nesting to TSS
      IP must be in code segment limit else #GP(0)
    JUMP TASK STATE SEGMENT:
      TSS DPL must be >= CPL else #GP (TSS selector)
      TSS DPL must be >= TSS selector RPL else #GP (TSS selector)
      Descriptor AR byte must specify available TSS (bottom bits 00001) else #GP (TSS selector)
      Task State Segment must be PRESENT else #NP (TSS selector)
      SWITCH_TASKS with nesting to TS.
      IP must be in code segment limit else #GP(0)
    Else GP (selector)
`;

export const jmp: InstructionInfo = {
  identifier: 'jmp',
  name: 'Jump',
  description:
    "The JMP instruction transfers program control to a different instruction stream without recording any return information.\n\nFor inter-segment jumps, the destination can be a code segment, a call gate, a task gate, or a Task State Segment. The latter two destinations cause a complete task switch to take place.\n\nControl transfers within a segment use the `JMP cw` or `JMP cb` forms. The operand is a relative offset added modulo `0x10000` to the offset of the instruction that follows the `JMP`. The result is the new value of `IP`; the value of `CS` is unchanged. The byte operand is sign-extended before it is added; it can therefore be used to address labels within 128 bytes in either direction from the next instruction.\n\nIndirect jumps within a segment use the `JMP ew` form. The contents of the register or memory operand is an absolute offset, which becomes the new value of `IP`. Again, `CS` is unchanged.\n\nInter-segment jumps in real address mode simply set `IP` to the offset part of the long pointer and set `CS` to the selector part of the pointer.\n\nIn protected mode, inter-segment jumps cause the CPU to consult the descriptor addressed by the selector part of the long pointer. The `AR` byte of the descriptor determines the type of the destination. Following are the possible destinations:\n\n1. **Code segment**: The addressability and visibility of the destination are verified, and `CS` and `IP` are loaded with the destination pointer values.\n2. **Call gate**: The offset part of the destination pointer is ignored. After checking for validity, the processor jumps to the location stored in the call gate descriptor.\n3. **Task gate**: The current task's state is saved in its Task State Segment (TSS), and the TSS named in the task gate is used to load a new context. The outgoing task is marked not busy, the new TSS is marked busy, and execution resumes at the point at which the new task was last suspended.\n4. **TSS**: The current task is suspended and the new task is initiated as in 3 above except that there is no intervening gate.\n\nFollowing is the list of checks and actions taken for long jumps in protected mode:\n\n```" +
    CODE +
    '```',
  modifies: [],
  undefined: [],
  macros: {
    // Protected-mode far JMP logic — handles direct code segment jumps
    // and jump through call gates (no stack operations, no privilege transitions).
    // Expects locals: gate_sel, gate_off, tmp, index,
    //   desc_type, desc_s, desc_a, desc_dpl, desc_p, desc_limit, desc_base,
    //   gate_target_sel, gate_target_off, gate_word_count.
    JMP_FAR_PROTECTED: [
      ';; Look up the descriptor for the selector',
      'tmp = gate_sel',
      'index = tmp >> 3',
      'ERROR_CODE = gate_sel',
      '#GP if index == 0',
      '${DESCRIPTOR_BOUNDS_CHECK}',
      '${LOAD_DESCRIPTOR_FIELDS}',
      '${LOAD_DESCRIPTOR_A}',

      ';; === DIRECT CODE SEGMENT JUMP (S=1) ===',
      'if desc_s == 1',
      [
        ';; Must be executable code segment',
        '#GP if (desc_type & 0b100) != 0b100',
        ';; Non-conforming: RPL must be <= CPL, DPL must equal CPL',
        'if (desc_type & 0b010) != 0b010',
        [
          '#GP if (gate_sel & 0x0003) > CS.RPL',
          '#GP if desc_dpl != CS.RPL',
        ],
        'end if',
        ';; Conforming: DPL must be <= CPL',
        'if (desc_type & 0b010) == 0b010',
        ['#GP if desc_dpl > CS.RPL'],
        'end if',
        ';; Must be present',
        '#NP if desc_p != 1',
        ';; Load CS:IP via setter (set RPL to CPL)',
        'CS = (gate_sel & 0xFFFC) | CS.RPL',
        'IP = gate_off',
      ],
      'end if',

      ';; === SYSTEM DESCRIPTOR — CALL GATE (S=0) ===',
      'if desc_s == 0',
      [
        ';; Must be a call gate (A=0, type=010)',
        '#GP if desc_a != 0',
        '#GP if desc_type != 0b010',
        ';; Gate DPL must be >= CPL',
        '#GP if desc_dpl < CS.RPL',
        ';; Gate DPL must be >= selector RPL',
        '#GP if desc_dpl < (gate_sel & 0x0003)',
        ';; Gate must be present',
        '#NP if desc_p != 1',

        ';; Read call gate descriptor fields',
        '${LOAD_CALL_GATE_FIELDS}',

        ';; Look up target code segment',
        'tmp = gate_target_sel',
        'index = tmp >> 3',
        'ERROR_CODE = gate_target_sel',
        '#GP if index == 0',
        '${DESCRIPTOR_BOUNDS_CHECK}',
        '${LOAD_DESCRIPTOR_FIELDS}',
        ';; Must be code segment (S=1, executable)',
        '#GP if desc_s != 1',
        '#GP if (desc_type & 0b100) != 0b100',
        ';; Non-conforming: DPL must equal CPL (JMP cannot change privilege)',
        'if (desc_type & 0b010) != 0b010',
        ['#GP if desc_dpl != CS.RPL'],
        'end if',
        ';; Conforming: DPL must be <= CPL',
        'if (desc_type & 0b010) == 0b010',
        ['#GP if desc_dpl > CS.RPL'],
        'end if',
        ';; Must be present',
        '#NP if desc_p != 1',

        ';; Load CS:IP from gate (set RPL to CPL)',
        'CS = (gate_target_sel & 0xFFFC) | CS.RPL',
        'IP = gate_target_off',
      ],
      'end if',
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
    // Protected-mode call gate support
    {
      identifier: 'gate_sel',
      name: 'Far jump selector',
      size: 16,
    },
    {
      identifier: 'gate_off',
      name: 'Far jump offset',
      size: 16,
    },
    {
      identifier: 'tmp',
      name: 'Temporary register',
      size: 16,
    },
    {
      identifier: 'index',
      name: 'Descriptor table entry index',
      size: 16,
    },
    {
      identifier: 'desc_type',
      name: 'Descriptor type field',
      size: 8,
    },
    {
      identifier: 'desc_s',
      name: 'Descriptor S bit',
      size: 8,
    },
    {
      identifier: 'desc_a',
      name: 'Descriptor A bit',
      size: 8,
    },
    {
      identifier: 'desc_dpl',
      name: 'Descriptor privilege level',
      size: 8,
    },
    {
      identifier: 'desc_p',
      name: 'Descriptor present bit',
      size: 8,
    },
    {
      identifier: 'desc_limit',
      name: 'Descriptor limit',
      size: 16,
    },
    {
      identifier: 'desc_base',
      name: 'Descriptor base',
      size: 32,
    },
    {
      identifier: 'gate_target_sel',
      name: 'Call gate target CS selector',
      size: 16,
    },
    {
      identifier: 'gate_target_off',
      name: 'Call gate target IP offset',
      size: 16,
    },
    {
      identifier: 'gate_word_count',
      name: 'Call gate parameter word count',
      size: 8,
    },
  ],
  forms: [
    // 0xEB cb - JMP NEAR cb
    {
      operation: ['IP = IP + %{imm}'],
      opcode: [Opcodes.JMP_CB, 'IMM_i8'],
      operands: ['rel'],
      operandSize: 8,
      distance: 'short',
      addressing: 'relative',
      cycles: 7,
    },
    // 0xE9 cw - JMP NEAR cw
    {
      operation: ['IP = IP + %{imm}'],
      opcode: [Opcodes.JMP_CW, 'IMM_i16'],
      operands: ['rel'],
      operandSize: 8,
      distance: 'near',
      addressing: 'relative',
      cycles: 7,
    },
    // 0xEA cd - JMP FAR cd
    {
      modes: {
        real: {
          operation: ['CS = %{NEW_CS}', 'IP = %{NEW_IP}'],
        },
        protected: {
          operation: [
            'gate_sel = %{NEW_CS}',
            'gate_off = %{NEW_IP}',
            '${JMP_FAR_PROTECTED}',
          ],
        },
      },
      opcode: [
        Opcodes.JMPF_CD,
        {
          identifier: 'NEW_IP',
          name: 'New IP Value',
          type: InstructionDataTypes.Immediate,
          size: 16,
        },
        {
          identifier: 'NEW_CS',
          name: 'New CS Value',
          type: InstructionDataTypes.Immediate,
          size: 16,
        },
      ],
      operands: ['ptr'],
      operandSize: 8,
      distance: 'far',
      addressing: 'absolute',
      cycles: 11,
    },
    // 0xFF /4 - JMP NEAR ew (absolute offset)
    {
      modes: {
        real: {
          operation: [
            'offset = %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'IP = RAM:u16[effective_address]',
          ],
        },
        protected: {
          operation: [
            'offset = %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'IP = RAM:u16[effective_address]',
          ],
        },
      },
      opcode: [Opcodes.CALL_JMP_INC_DEC_PUSH, 'ModRM_110_100_00', 'DISP_i16'],
      operands: ['rm'],
      operandSize: 16,
      distance: 'near',
      addressing: 'absolute',
      cycles: 11,
    },
    {
      modes: {
        real: {
          operation: [
            'offset = ${MOD_RM_OFFSET}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'IP = RAM:u16[effective_address]',
          ],
        },
        protected: {
          operation: [
            'offset = ${MOD_RM_OFFSET}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'IP = RAM:u16[effective_address]',
          ],
        },
      },
      opcode: [Opcodes.CALL_JMP_INC_DEC_PUSH, 'ModRM_rm_100_00'],
      operands: ['rm'],
      operandSize: 16,
      distance: 'near',
      addressing: 'absolute',
      cycles: 11,
    },
    {
      modes: {
        real: {
          operation: [
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'IP = RAM:u16[effective_address]',
          ],
        },
        protected: {
          operation: [
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'IP = RAM:u16[effective_address]',
          ],
        },
      },
      opcode: [Opcodes.CALL_JMP_INC_DEC_PUSH, 'ModRM_rm_100_01', 'DISP_i8'],
      operands: ['rm'],
      operandSize: 16,
      distance: 'near',
      addressing: 'absolute',
      cycles: 11,
    },
    {
      modes: {
        real: {
          operation: [
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'IP = RAM:u16[effective_address]',
          ],
        },
        protected: {
          operation: [
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'IP = RAM:u16[effective_address]',
          ],
        },
      },
      opcode: [Opcodes.CALL_JMP_INC_DEC_PUSH, 'ModRM_rm_100_10', 'DISP_i16'],
      operands: ['rm'],
      operandSize: 16,
      distance: 'near',
      addressing: 'absolute',
      cycles: 11,
    },
    {
      operation: ['IP = ${MOD_RM_RM16}'],
      opcode: [Opcodes.CALL_JMP_INC_DEC_PUSH, 'ModRM_rm16_100_11'],
      operands: ['rm'],
      operandSize: 16,
      distance: 'near',
      addressing: 'absolute',
      cycles: 7,
    },
    // 0xFF /5 - JMP FAR ed (far indirect through memory)
    {
      modes: {
        real: {
          operation: [
            'offset = %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'IP = RAM:u16[effective_address]',
            'CS = RAM:u16[effective_address + 2]',
          ],
        },
        protected: {
          operation: [
            'offset = %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'gate_off = RAM:u16[effective_address]',
            'gate_sel = RAM:u16[effective_address + 2]',
            '${JMP_FAR_PROTECTED}',
          ],
        },
      },
      opcode: [Opcodes.CALL_JMP_INC_DEC_PUSH, 'ModRM_110_101_00', 'DISP_i16'],
      operands: ['rm'],
      operandSize: 16,
      distance: 'far',
      addressing: 'absolute',
      cycles: 15,
    },
    {
      modes: {
        real: {
          operation: [
            'offset = ${MOD_RM_OFFSET}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'IP = RAM:u16[effective_address]',
            'CS = RAM:u16[effective_address + 2]',
          ],
        },
        protected: {
          operation: [
            'offset = ${MOD_RM_OFFSET}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'gate_off = RAM:u16[effective_address]',
            'gate_sel = RAM:u16[effective_address + 2]',
            '${JMP_FAR_PROTECTED}',
          ],
        },
      },
      opcode: [Opcodes.CALL_JMP_INC_DEC_PUSH, 'ModRM_rm_101_00'],
      operands: ['rm'],
      operandSize: 16,
      distance: 'far',
      addressing: 'absolute',
      cycles: 11,
    },
    {
      modes: {
        real: {
          operation: [
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'IP = RAM:u16[effective_address]',
            'CS = RAM:u16[effective_address + 2]',
          ],
        },
        protected: {
          operation: [
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'gate_off = RAM:u16[effective_address]',
            'gate_sel = RAM:u16[effective_address + 2]',
            '${JMP_FAR_PROTECTED}',
          ],
        },
      },
      opcode: [Opcodes.CALL_JMP_INC_DEC_PUSH, 'ModRM_rm_101_01', 'DISP_i8'],
      operands: ['rm'],
      operandSize: 16,
      distance: 'far',
      addressing: 'absolute',
      cycles: 11,
    },
    {
      modes: {
        real: {
          operation: [
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'IP = RAM:u16[effective_address]',
            'CS = RAM:u16[effective_address + 2]',
          ],
        },
        protected: {
          operation: [
            'offset = ${MOD_RM_OFFSET} + %{DISP}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'gate_off = RAM:u16[effective_address]',
            'gate_sel = RAM:u16[effective_address + 2]',
            '${JMP_FAR_PROTECTED}',
          ],
        },
      },
      opcode: [Opcodes.CALL_JMP_INC_DEC_PUSH, 'ModRM_rm_101_10', 'DISP_i16'],
      operands: ['rm'],
      operandSize: 16,
      distance: 'far',
      addressing: 'absolute',
      cycles: 11,
    },
    {
      operation: [
        // Raise error... cannot use a register
        '#6',
      ],
      opcode: [Opcodes.CALL_JMP_INC_DEC_PUSH, 'ModRM_rm16_101_11'],
      operands: ['rm'],
      operandSize: 16,
      distance: 'far',
      addressing: 'absolute',
      cycles: 7,
    },
  ],
};
