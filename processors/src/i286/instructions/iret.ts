import type { InstructionInfo } from '@rawrs/architecture/Target';

import { Opcodes } from '../opcodes';

const CODE_LISTING = `INTERRUPT RETURN:
  If Nested Task Flag=1 then
    RETURN FROM NESTED TASK:
      Examine Back Link Selector in TSS addressed by the current Task Register:
        Must specify global in the local/global bit else #TS (new TSS selector)
        Index must be within GDT limits else #TS (new TSS selector)
        AR byte must specify TSS else #TS (new TSS selector)
        New TSS must be busy else #TS (new TSS selector)
        Task State Segment must be PRESENT else #NP (new TSS selector)
      SWITCH_TASKS without nesting to TSS specified by back link selector
        Mark the task just abandoned as NOT BUSY
        IP must be in code segment limit else #GP(0) 
  If Nested Task Flag=0 then
    INTERRUPT RETURN ON STACK:
      Second word on stack must be within stack limits else #SS(0)
      Return CS selector RPL must be >= CPL else #GP (Return selector)
      If return selector RPL = CPL then
        INTERRUPT RETURN TO SAME LEVEL:
        Top 6 bytes on stack must be within limits else #SS(0)
        Return CS selector (at SP+2) must be non-null else #GP(0)
        Selector index must be within its descriptor table limits else #GP(Return selector)
        AR byte must indicate code segment else #GP(Return selector)
        If non-conforming then code segment DPL must = CPL else #GP (Return selector)
        If conforming then code segment DPL must be <= CPL else #GP (Return selector)
        Segment must be PRESENT else #NP (Return selector)
        IP must be in code segment limit else #GP(0)
        Load CS:IP from stack
        Load CS-cache with new code segment descriptor
        Load flags with third word on stack
        Increment SP by 6
      Else
        INTERRUPT RETURN TO OUTER PRIVILEGE LEVEL:
        Top 10 bytes on stack must be within limits else #SS(0)
        Examine return CS selector (at SP+2) and associated descriptor:
          Selector must be non-null else #GP(0)
          Selector index must be within its descriptor table limits else #GP (Return selector)
          AR byte must indicate code segment else #GP (Return selector)
          If non-conforming then code segment DPL must = CS selector RPL else #GP (Return selector)
          If conforming then code segment DPL must be > CPL else #GP (Return selector)
          Segment must be PRESENT else #NP (Return selector)
        Examine return SS selector (at SP+8) and associated descriptor:
          Selector must be non-null else #GP(0)
          Selector index must be within its descriptor table limits else #GP (SS selector)
          Selector RPL must equal the RPL of the return CS selector else #GP (SS selector)
          AR byte must indicate a writable data segment else #GP (SS selector)
          Stack segment DPL must equal the RPL of the return CS selector else #GP (SS selector)
          SS must be PRESENT else #SS (SS selector) 
        IP must be in code segment limit else #GP(O)
        Load CS:IP from stack
        Load flags with values at (SP+4)
        Load SS:SP from stack
        Set CPL to the RPL of the return CS selector
        Load the CS-cache with the CS descriptor
        Load the SS-cache with the SS descriptor
        For each of ES and DS:
          If the current register setting is not valid for the outer level, then zero the register and
            clear the valid flag
          To be valid, the register setting must satisfy the following properties:
            Selector index must be within descriptor table limits
            AR byte must indicate data or readable code segment
            If segment is data or non-conforming code, then:
              DPL must be >= CPL, or
              DPL must be >= RPL.`;

const description =
  'In real address mode, `IRET` pops `IP`, `CS`, and `FLAGS` from the stack in that order, and resumes the interrupted routine.\n\nIn protected mode, the action of `IRET` depends on the setting of the Nested Task Flag (`NT`) bit in the flag register. When popping the new flag image from the stack, note that the `IOPL` bits in the flag register are changed only when `CPL=0`.\n\nIf `NT=0`, `IRET` returns from an interrupt procedure without a task switch. The code returned to must be equally or less privileged than the interrupt routine as indicated by the `RPL` bits of the `CS` selector popped from the stack. If the destination code is of less privilege, `IRET` then also pops `SP` and `SS` from the stack.\n\nIf `NT=1`, `IRET` reverses the operation of a `CALL` or `INT` that caused a task switch. The task executing `IRET` has its updated state saved in its Task State Segment. This means that if the task is reentered, the code that follows `IRET` will be executed.\n\nThe exact checks and actions performed by `IRET` in protected mode are following\n\n```\n' +
  CODE_LISTING +
  '\n```';

// TODO: #13 if stack wraps around from 0xffff to 0
export const iret: InstructionInfo = {
  identifier: 'iret',
  name: 'Interrupt Return',
  description,
  modifies: ['OF', 'SF', 'ZF', 'AF', 'PF', 'CF'],
  undefined: [],
  locals: [
    {
      identifier: 'stack_address',
      name: 'Effective Stack Address',
      size: 32,
    },
  ],
  forms: [
    // 0xCF - IRET
    {
      operation: [
        'stack_address = SS_BASE + SP',
        'IP = RAM:u16[stack_address]',
        'CS = RAM:u16[stack_address + 2]',
        'FLAGS = RAM:u16[stack_address + 4] | 0b10',
        'SP = SP + 6',
      ],
      opcode: [Opcodes.IRET],
      operandSize: 16,
      cycles: 17, // protected-mode: 31, 55 if lesser privilege, 169 if switching tasks (NT = 1)
    },
  ],
};
