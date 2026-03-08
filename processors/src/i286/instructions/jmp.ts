import type { InstructionInfo } from '@machinery/core';
import { InstructionDataType } from '@machinery/core';

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
  macros: {},
  locals: [
    {
      identifier: 'effective_address',
      name: 'Effective Address',
      size: 32,
    },
  ],
  forms: [
    // 0xEB cb - JMP NEAR cb
    {
      operation: ['IP = IP + %{IMM}'],
      opcode: [Opcodes.JMP_CB, 'IMM_i8'],
      operandSize: 8,
      cycles: 7,
    },
    // 0xE9 cw - JMP NEAR cw
    {
      operation: ['IP = IP + %{IMM}'],
      opcode: [Opcodes.JMP_CW, 'IMM_i16'],
      operandSize: 8,
      cycles: 7,
    },
    // 0xEA cd - JMP FAR cd
    {
      operation: ['CS = %{NEW_CS}', 'IP = %{NEW_IP}'],
      opcode: [
        Opcodes.JMPF_CD,
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
      cycles: 11, // 23 in protected mode, 38 via same-privilege call-gate, 175 via tss, 180 via task gate
    },
    // 0xFF /4 - JMP NEAR ew (absolute offset)
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + %{DISP}',
        'IP = RAM:u16[effective_address]',
      ],
      opcode: [Opcodes.CALL_JMP_INC_DEC_PUSH, 'ModRM_110_100_00', 'DISP_i16'],
      operandSize: 16,
      cycles: 11,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET}',
        'IP = RAM:u16[effective_address]',
      ],
      opcode: [Opcodes.CALL_JMP_INC_DEC_PUSH, 'ModRM_rm_100_00'],
      operandSize: 16,
      cycles: 11,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'IP = RAM:u16[effective_address]',
      ],
      opcode: [Opcodes.CALL_JMP_INC_DEC_PUSH, 'ModRM_rm_100_01', 'DISP_i8'],
      operandSize: 16,
      cycles: 11,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'IP = RAM:u16[effective_address]',
      ],
      opcode: [Opcodes.CALL_JMP_INC_DEC_PUSH, 'ModRM_rm_100_10', 'DISP_i16'],
      operandSize: 16,
      cycles: 11,
    },
    {
      operation: ['IP = ${MOD_RM_RM16}'],
      opcode: [Opcodes.CALL_JMP_INC_DEC_PUSH, 'ModRM_rm_100_11'],
      operandSize: 16,
      cycles: 7,
    },
    // 0xFF /5 - JMP FAR ed
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + %{DISP}',
        'IP = RAM:u16[effective_address]',
        'CS = RAM:u16[effective_address + 2]',
      ],
      opcode: [Opcodes.CALL_JMP_INC_DEC_PUSH, 'ModRM_110_101_00', 'DISP_i16'],
      operandSize: 16,
      cycles: 15, // 26 in protected-mode, 41 via same-privilege call gate, 178 via tss, 183 via task gate
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET}',
        'IP = RAM:u16[effective_address]',
        'CS = RAM:u16[effective_address + 2]',
      ],
      opcode: [Opcodes.CALL_JMP_INC_DEC_PUSH, 'ModRM_rm_101_00'],
      operandSize: 16,
      cycles: 11,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'IP = RAM:u16[effective_address]',
        'CS = RAM:u16[effective_address + 2]',
      ],
      opcode: [Opcodes.CALL_JMP_INC_DEC_PUSH, 'ModRM_rm_101_01', 'DISP_i8'],
      operandSize: 16,
      cycles: 11,
    },
    {
      operation: [
        'effective_address = ${MOD_RM_SEGMENT} + ${MOD_RM_OFFSET} + %{DISP}',
        'IP = RAM:u16[effective_address]',
        'CS = RAM:u16[effective_address + 2]',
      ],
      opcode: [Opcodes.CALL_JMP_INC_DEC_PUSH, 'ModRM_rm_101_10', 'DISP_i16'],
      operandSize: 16,
      cycles: 11,
    },
    {
      operation: [
        // Raise error... cannot use a register
        '#6',
      ],
      opcode: [Opcodes.CALL_JMP_INC_DEC_PUSH, 'ModRM_rm_101_11'],
      operandSize: 16,
      cycles: 7,
    },
  ],
};
