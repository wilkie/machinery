import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

const description =
  '`RET` transfers control to a return address located on the stack. The address is usually placed on the stack by a `CALL` instruction; in that case, the return is made to the instruction that follows the `CALL`.\n\nThere is an optional numeric parameter to `RET`. It gives the number of stack bytes to be released after the return address is popped. These bytes are typically used as input parameters to the procedure called.\n\nFor the intra-segment return, the address on the stack is a 2-byte quantity popped into `IP`. The `CS` register is unchanged.\n\nFor the inter-segment return, the address on the stack is a 4-byte-long pointer. The offset is popped first, followed by the selector. In real address mode, `CS` and `IP` are directly loaded.\n\nIn protected mode, an inter-segment return causes the processor to consult the descriptor addressed by the return selector. The `AR` byte of the descriptor must indicate a code segment of equal or less privilege (of greater or equal numeric value) than the current privilege level. Returns to a lesser privilege level cause the stack to be reloaded from the value saved beyond the parameter block.\n\nThe `DS` and `ES` segment registers may be set to zero by the inter-segment `RET` instruction. If these registers refer to segments which cannot be used by the new privilege level, they are set to zero to prevent unauthorized access.\n\nThe following list of checks and actions describes the protected-mode inter-segment return in detail.\n\n```\nInter-segment RET:\n  Second word on stack must be within stack limits else #SS(0)\n  ' +
  'Return selector RPL must be >= CPL else #GP (return selector)\n  If return selector RPL = CPL then\n    RETURN TO SAME LEVEL:\n    Return selector must be non-null else #GP(0)\n    Selector index must be within its descriptor table limits else #GP (selector)\n    Descriptor AR byte must indicate code segment else #GP (selector)\n    If non-conforming then code segment DPL must equal CPL else #GP (selector)\n    If conforming then code segment DPL must be <= CPL else #GP (selector)\n    Code segment must be PRESENT else #NP (selector)\n    Top word on stack must be within stack limits else #SS(0)\n    IP must be in code segment limit else #GP(0)\n    Load CS:IP from stack\n    Load CS-cache with descriptor\n    Increment SP by 4 plus the immediate offset if it exists\n  Else\n    RETURN TO OUTER PRIVILEGE LEVEL:\n    Top (8+ immediate) bytes on stack must be within stack limits else #SS(0)\n    Examine return CS selector (at SP+2) and associated descriptor:\n      Selector must be non-null else #GP(0)\n      Selector index must be within its descriptor table limits else #GP (selector)\n      Descriptor AR byte must indicate code segment else #GP (selector)\n      If non-conforming then code segment DPL must equal return selector RPL else #GP (selector)\n      If conforming then code segment DPL must be <= return selector RPL else #GP (selector)\n      Segment must be PRESENT else #NP (selector)\n    Examine return SS selector (at SP+6+imm) and associated descriptor:\n      Selector must be non-null else #GP(0)\n      Selector index must be within its descriptor table limits else #GP (selector)\n      Selector RPL must equal the RPL of the return CS selector else #GP (selector)\n      Descriptor AR byte must indicate a writable data segment else #GP (selector)\n      Descriptor DPL must equal the RPL of the return CS selector else #GP (selector)\n      Segment must be PRESENT else #SS (selector)\n    IP must be in code segment limit else #GP(0)\n    Set CPL to the RPL of the return CS selector\n    Load CS:IP from stack\n    Set CS RPL to CPL\n    Increment SP by 4 plus the immediate offset if it exists\n    Load SS:SP from stack\n    Load the CS-cache with the return CS descriptor\n    Load the SS-cache with the return SS descriptor\n    For each of ES and DS:\n      If the current register setting is not valid for the outer level, set the\n        register to null (selector = AR = 0)\n      To be valid, the register setting must satisfy the following properties:\n        Selector index must be within descriptor table limits\n        Descriptor AR byte must indicate data or readable code segment\n        If segment is data or non-conforming code, then:\n          DPL must be >= CPL, or\n          DPL must be >= RPL\n```';

// TODO: #13 if stack wraps around from 0xffff to 0
export const ret: InstructionInfo = {
  identifier: 'ret',
  name: 'Return from Procedure',
  description,
  modifies: [],
  undefined: [],
  locals: [
    {
      identifier: 'stack_address',
      name: 'Effective Stack Address',
      size: 32,
    },
    {
      identifier: 'offset',
      name: 'Effective Offset',
      size: 32,
    },
  ],
  forms: [
    // 0xC3 - RET near
    {
      modes: {
        real: {
          operation: [
            'offset = SP',
            'stack_address = SS_BASE + offset',
            '#GP if offset == 0xffff',
            'IP = RAM:u16[stack_address]',
            'SP = SP + 2',
          ],
        },
        protected: {
          operation: [
            'offset = SP',
            'stack_address = SS_BASE + offset',
            '#GP if (offset + 1) < SS_LIMIT_MIN',
            '#GP if (offset + 1) > SS_LIMIT_MAX',
            'IP = RAM:u16[stack_address]',
            'SP = SP + 2',
          ],
        },
      },
      opcode: [Opcodes.RET],
      operands: [],
      operandSize: 16,
      distance: 'near',
      cycles: 11,
    },
    // 0xCB - RET far
    // 0xCB - RET to lesser privilege
    {
      modes: {
        real: {
          operation: [
            'offset = SP',
            'stack_address = SS_BASE + offset',
            '#GP if offset == 0xffff',
            '#GP if (offset + 2) == 0xffff',
            'IP = RAM:u16[stack_address]',
            'CS = RAM:u16[stack_address + 2]',
            'SP = SP + 4',
          ],
        },
        protected: {
          operation: [
            'offset = SP',
            'stack_address = SS_BASE + offset',
            '#GP if (offset + 3) < SS_LIMIT_MIN',
            '#GP if (offset + 3) > SS_LIMIT_MAX',
            'IP = RAM:u16[stack_address]',
            'CS = RAM:u16[stack_address + 2]',
            'SP = SP + 4',
          ],
        },
      },
      opcode: [Opcodes.RETF],
      operands: [],
      operandSize: 16,
      distance: 'far',
      cycles: 15, // protected-mode: 25, 55 if switching stacks
    },
    // 0xC2 dw - RET near dw
    {
      modes: {
        real: {
          operation: [
            'offset = SP',
            'stack_address = SS_BASE + offset',
            '#GP if offset == 0xffff',
            'IP = RAM:u16[stack_address]',
            'SP = SP + 2 + %{imm}',
          ],
        },
        protected: {
          operation: [
            'offset = SP',
            'stack_address = SS_BASE + offset',
            '#GP if (offset + 1) < SS_LIMIT_MIN',
            '#GP if (offset + 1) > SS_LIMIT_MAX',
            'IP = RAM:u16[stack_address]',
            'SP = SP + 2 + %{imm}',
          ],
        },
      },
      opcode: [Opcodes.RET_DW, 'IMM_u16'],
      operands: ['imm'],
      operandSize: 16,
      distance: 'near',
      cycles: 11,
    },
    // 0xCA dw - RET far dw
    // 0xCA dw - RET to lesser privilege dw
    {
      modes: {
        real: {
          operation: [
            'offset = SP',
            'stack_address = SS_BASE + offset',
            '#GP if offset == 0xffff',
            '#GP if (offset + 2) == 0xffff',
            'IP = RAM:u16[stack_address]',
            'CS = RAM:u16[stack_address + 2]',
            'SP = SP + 4 + %{imm}',
          ],
        },
        protected: {
          operation: [
            'offset = SP',
            'stack_address = SS_BASE + offset',
            '#GP if (offset + 3) < SS_LIMIT_MIN',
            '#GP if (offset + 3) > SS_LIMIT_MAX',
            'IP = RAM:u16[stack_address]',
            'CS = RAM:u16[stack_address + 2]',
            'SP = SP + 4 + %{imm}',
          ],
        },
      },
      opcode: [Opcodes.RETF_DW, 'IMM_u16'],
      operands: ['imm'],
      operandSize: 16,
      distance: 'far',
      cycles: 15, // protected-mode: 25, 55 if switching stacks
    },
  ],
};
