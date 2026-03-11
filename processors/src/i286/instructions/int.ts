import type { InstructionInfo } from '@machinery/core';

import { Opcodes } from '../opcodes';

const CODE_LISTING = `INTERRUPT
  Interrupt vector must be within IDT table limits else #GP (vector number * 8+2+EXT)
  Descriptor AR byte must indicate interrupt gate, trap gate, or task gate else #GP (vector number X
    8+2+EXT)
  If INT instruction then gate descriptor DPL must be 2: CPL else #GP (vector number X 8+2+EXT)
  Gate must be PRESENT else #NP (vector number X 8+2+EXT)
  If TRAP GATE or INTERRUPT GATE:
  Examine CS selector and descriptor given in the gate descriptor:
    Selector must be non-null else #GP (EXT)
    Selector must be within its descriptor table limits else #GP (selector + EXT)
    Descriptor AR byte must indicate code segment else #GP (selector + EXT)
    Segment must be PRESENT else #NP (selector + EXT)
  If code segment is non-conforming and DPL < CPL then
    INTERRUPT TO INNER PRIVILEGE:
      Check selector and descriptor for new stack in current Task State Segment:
        Selector must be non-null else #TS(EXT)
        Selector index must be within its descriptor table limits else #TS (SS selector+EXT)
        Selector's RPL must equal DPL of code segment else #TS (SS selector + EXT)
        Stack segment DPL must equal DPL of code segment else #TS (SS selector+ EXT)
        Descriptor must indicate writable data segment else #TS (SS selector+ EXT)
        Segment must be PRESENT else #SS (SS selector + EXT)
      New stack must have room for 10 bytes else #SS(0)
      IP must be in CS limit else #GP(0)
      Load new SS and SP value from TSS
      Load new CS and IP value from gate
      Load CS descriptor
      Load SS descriptor
      Push long pointer to old stack onto new stack
      Push return address onto new stack
      Set CPL to new code segment DPL
      Set RPL of CS to CPL
      If INTERRUPT GATE then set the Interrupts Enabled Flag to 0 (disabled)
      Set the Trap Flag to 0
      Set the Nested Task Flag to 0
  If code segment is conforming or code segment DPL = CPL then
    INTERRUPT TO SAME PRIVILEGE LEVEL:
      Current stack limits must allow pushing 6 bytes else #SS(0)
      If interrupt was caused by fault with error code then
        Stack limits must allow push of two more bytes else #SS(0)
      IP must be in CS limit else #GP(0)
      Push flags onto stack
      Push current CS selector onto stack
      Push return offset onto stack
      Load CS:IP from gate
      Load CS descriptor
      Set the RPL field of CS to CPL
      Push error code (if any) onto stack
        If INTERRUPT GATE then set the Interrupts Enabled Flag to 0 (disabled)
      Set the Trap Flag to 0
      Set the Nested Task Flag to 0
  Else #GP (CS selector + EXT)
If TASK GATE:
  Examine selector to TSS, given in Task Gate descriptor:
    Must specify global in the local/global bit else #GP (TSS selector) 
    Index must be within GDT limits else #GP (TSS selector)
    AR byte must specify availab!e TSS (bottom bits 00001) else #GP (TSS selector)
    Task State Segment must be PRESENT else #NP (TSS selector)
  SWITCH_TASKS with nesting to TSS
  If interrupt was caused by fault with error code then
    Stack limits must allow push of two more bytes else #SS(0)
    Push error code onto stack
  IP must be in CS limit else #GP(0)`;

export const int: InstructionInfo = {
  identifier: 'int',
  name: 'Call to Interrupt Procedure',
  description:
    'The `INT` instruction generates via software a call to an interrupt procedure. The immediate operand, from 0 to 255, gives the index number into the Interrupt Descriptor Table of the interrupt routine to be called. In protected mode, the `IDT` consists of 8-byte descriptors; the descriptor for the interrupt invoked must indicate an interrupt gate, a trap gate, or a task gate. In real address mode, the lDT is an array of 4-byte long pointers at the fixed location 0x0000.\n\nThe `INTO` instruction is identical to the `INT` instruction except that the interrupt number is implicitly `4`, and the interrupt is made only if the overflow flag is on. The clock counts for the four forms of `INT db` are valid for `INTO`, with the number of clocks increased by `1` for the overflow flag test.\n\nThe first 32 interrupts are reserved by Intel for systems use. Some of these interrupts are exception handlers for internally-generated faults. Most of these exception handlers should not be invoked with the `INT` instruction.\n\nGenerally, interrupts behave like far `CALL`s except that the flags register is pushed onto the stack before the return address. Interrupt procedures return via the `IRET` instruction, which pops the flags from the stack.\n\nIn Real Address mode, `INT` pushes the flags, `CS` and the return `IP` onto the stack in that order, then resets the Trap Flag (`TF`), then jumps to the long pointer indexed by the interrupt number, in the interrupt vector table (`IVT`).\n\nIn Protected mode, `INT` also resets the Trap Flag. In Protected mode, the precise semantics of the `INT` instruction are given by the following:\n\n```\n' +
    CODE_LISTING +
    '\n```\n\n**Note**: `EXT` is 1 if an external event (i.e., a single step, an external interrupt, an `MF` exception, or an `MP` exception) caused the interrupt; 0 if not (i.e., an `INT` instruction or other exceptions).',
  modifies: [],
  undefined: [],
  forms: [
    // 0xCC - INT 3
    {
      opcode: [Opcodes.INT_3],
      operands: [],
      operation: ['${RESOLVE_FLAGS}', '#3'],
      cycles: 23, // protected-mode: 40 to same privilege, 78 raised privilege, 167 if via task gate
    },
    // 0xCD - INT db
    {
      opcode: [Opcodes.INT_DB, 'IMM_u8'],
      operands: ['imm'],
      operation: ['${RESOLVE_FLAGS}', '#%{imm}'],
      cycles: 23, // protected-mode: 40 to same privilege, 78 raised privilege, 167 if via task gate
    },
    // 0xCE - INTO
    {
      opcode: [Opcodes.INTO],
      operands: [],
      operation: ['${RESOLVE_FLAGS}', '#4 if OF > 0x0'],
      cycles: 24, // 3, if no jump occurs
    },
  ],
};
