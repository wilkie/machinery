import type { InstructionInfo } from '@machinery/core';

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
    {
      identifier: 'offset',
      name: 'Effective Offset',
      size: 32,
    },
    {
      identifier: 'ret_cs',
      name: 'Return CS selector',
      size: 16,
    },
    {
      identifier: 'ret_flags',
      name: 'Return FLAGS value',
      size: 16,
    },
    {
      identifier: 'ret_sp',
      name: 'Return SP (outer privilege)',
      size: 16,
    },
    {
      identifier: 'ret_ss',
      name: 'Return SS (outer privilege)',
      size: 16,
    },
    {
      identifier: 'ret_rpl',
      name: 'Return CS RPL (new CPL)',
      size: 8,
    },
    {
      identifier: 'old_cpl',
      name: 'CPL before return',
      size: 8,
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
      identifier: 'seg_valid',
      name: 'Segment validity flag',
      size: 1,
    },
    {
      identifier: 'task_switched',
      name: 'Task switch performed flag',
      size: 1,
    },
    {
      identifier: 'back_link',
      name: 'Back link TSS selector',
      size: 16,
    },
    {
      identifier: 'old_tss_index',
      name: 'Old TSS GDT index',
      size: 16,
    },
  ],
  forms: [
    // 0xCF - IRET
    {
      modes: {
        real: {
          operation: [
            'offset = SP',
            'stack_address = SS_BASE + offset',
            '#GP if offset == 0xffff',
            '#GP if (offset + 2) == 0xffff',
            '#GP if (offset + 4) == 0xffff',
            'IP = RAM:u16[stack_address]',
            'CS = RAM:u16[stack_address + 2]',
            'FLAGS = RAM:u16[stack_address + 4] | 0b10',
            'SP = SP + 6',
          ],
        },
        protected: {
          operation: [
            ';; RETURN FROM NESTED TASK (NT=1)',
            'task_switched = 0',
            'if NT == 1',
            [
              '${RESOLVE_FLAGS}',
              ';; save old TSS GDT index',
              'old_tss_index = TR.selector >> 3',
              ';; read back link selector from current TSS',
              'back_link = RAM.TSS.back_link',
              'ERROR_CODE = back_link',
              ';; must specify GDT (TI bit = 0)',
              '#TS if (back_link & 0x0004) != 0',
              ';; index must be within GDT limits',
              'index = back_link >> 3',
              '#TS if index == 0',
              '#TS if (index * 8) > GDTR.limit',
              ';; must be a system descriptor (S=0)',
              '#TS if RAM.GDT.gates[index].SD.S != 0',
              ';; must be a TSS (A=1)',
              '#TS if RAM.GDT.gates[index].SD.A != 1',
              ';; must be a busy TSS (type=001)',
              '#TS if RAM.GDT.gates[index].SD.type != 0b001',
              ';; must be present',
              '#NP if RAM.GDT.gates[index].SD.P != 1',
              ';; save current task state to current TSS',
              'RAM.TSS.IP = IP',
              'RAM.TSS.FLAGS = FLAGS',
              'RAM.TSS.AX = AX',
              'RAM.TSS.CX = CX',
              'RAM.TSS.DX = DX',
              'RAM.TSS.BX = BX',
              'RAM.TSS.SP = SP',
              'RAM.TSS.BP = BP',
              'RAM.TSS.SI = SI',
              'RAM.TSS.DI = DI',
              'RAM.TSS.ES = ES',
              'RAM.TSS.CS = CS',
              'RAM.TSS.SS = SS',
              'RAM.TSS.DS = DS',
              ';; mark old TSS as not busy (type = 000)',
              'RAM.GDT.gates[old_tss_index].SD.type = 0b000',
              ';; switch TR to new TSS',
              'TR.selector = back_link',
              'TR.base = RAM.GDT.gates[index].SD.base',
              'TR.limit = RAM.GDT.gates[index].SD.limit',
              ';; load general registers from new TSS',
              'AX = RAM.TSS.AX',
              'CX = RAM.TSS.CX',
              'DX = RAM.TSS.DX',
              'BX = RAM.TSS.BX',
              'BP = RAM.TSS.BP',
              'SI = RAM.TSS.SI',
              'DI = RAM.TSS.DI',
              ';; load segment registers (triggers validation via set operations)',
              'ES = RAM.TSS.ES',
              'CS = RAM.TSS.CS',
              'SS = RAM.TSS.SS',
              'DS = RAM.TSS.DS',
              'SP = RAM.TSS.SP',
              ';; load FLAGS (full replacement including IOPL)',
              'FLAGS = RAM.TSS.FLAGS | 0b10',
              ';; load IP (uses new CS_BASE from CS set)',
              'IP = RAM.TSS.IP',
              ';; IP must be in code segment limit',
              'ERROR_CODE = 0',
              '#GP if IP > CS_LIMIT_MAX',
              'task_switched = 1',
            ],
            'end if',

            ';; INTERRUPT RETURN ON STACK (NT=0)',
            'if task_switched == 0',
            [
              'offset = SP',
              'stack_address = SS_BASE + offset',
              ';; at least 6 bytes must be readable on the stack',
              '#SS if (offset + 5) < SS_LIMIT_MIN',
              '#SS if (offset + 5) > SS_LIMIT_MAX',
              ';; read return CS to determine privilege transition',
              'ret_cs = RAM:u16[stack_address + 2]',
              'ret_rpl = ret_cs & 0x0003',
              'old_cpl = CS.RPL',
              ';; return CS RPL must be >= CPL',
              'ERROR_CODE = ret_cs',
              '#GP if ret_rpl < old_cpl',
              ';; validate return CS selector (may be in GDT or LDT)',
              'tmp = ret_cs',
              'index = tmp >> 3',
              '#GP if index == 0',
              '${DESCRIPTOR_BOUNDS_CHECK}',
              '${LOAD_DESCRIPTOR_FIELDS}',
              '#GP if desc_s != 1',
              '#GP if (desc_type & 0b100) != 0b100',
              ';; DPL checks for non-conforming code',
              'if (desc_type & 0b010) != 0b010',
              [
                ';; non-conforming: DPL must equal return CS RPL',
                '#GP if desc_dpl != ret_rpl',
              ],
              'end if',
              ';; DPL checks for conforming code',
              'if (desc_type & 0b010) == 0b010 && ret_rpl == old_cpl',
              [
                ';; same-level conforming: DPL must be <= CPL',
                '#GP if desc_dpl > old_cpl',
              ],
              'end if',
              'if (desc_type & 0b010) == 0b010 && ret_rpl > old_cpl',
              [
                ';; outer-level conforming: DPL must be > CPL',
                '#GP if desc_dpl <= old_cpl',
              ],
              'end if',
              ';; segment must be present',
              '#NP if desc_p != 1',

              ';; RETURN TO SAME PRIVILEGE LEVEL',
              'if ret_rpl == old_cpl',
              [
                'ret_flags = RAM:u16[stack_address + 4]',
                ';; load CS:IP from stack',
                'IP = RAM:u16[stack_address]',
                'CS = ret_cs',
                ';; load FLAGS; IOPL only changes if old CPL == 0',
                'if old_cpl == 0',
                ['FLAGS = ret_flags | 0b10'],
                'end if',
                'if old_cpl != 0',
                ['FLAGS = (ret_flags & 0xcfff) | (FLAGS & 0x3000) | 0b10'],
                'end if',
                'SP = SP + 6',
              ],
              'end if',

              ';; RETURN TO OUTER PRIVILEGE LEVEL',
              'if ret_rpl > old_cpl',
              [
                ';; 10 bytes must be readable on the stack',
                '#SS if (offset + 9) > SS_LIMIT_MAX',
                ';; read all return values from stack',
                'ret_flags = RAM:u16[stack_address + 4]',
                'ret_sp = RAM:u16[stack_address + 6]',
                'ret_ss = RAM:u16[stack_address + 8]',
                ';; load CS:IP from stack (CS set validates the selector)',
                'IP = RAM:u16[stack_address]',
                'CS = ret_cs',
                ';; load FLAGS; IOPL only changes if old CPL == 0',
                'if old_cpl == 0',
                ['FLAGS = ret_flags | 0b10'],
                'end if',
                'if old_cpl != 0',
                ['FLAGS = (ret_flags & 0xcfff) | (FLAGS & 0x3000) | 0b10'],
                'end if',
                ';; load SS:SP from stack (SS set validates against new CPL)',
                'SS = ret_ss',
                'SP = ret_sp',
                ';; invalidate ES if not valid for outer privilege level',
                'index = ES >> 3',
                'tmp = ES',
                'seg_valid = 1',
                'if index == 0',
                ['seg_valid = 0'],
                'end if',
                ';; bounds check (GDT or LDT based on TI bit)',
                'if seg_valid == 1 && (tmp & 0x0004) == 0 && (index * 8) > GDTR.limit',
                ['seg_valid = 0'],
                'end if',
                'if seg_valid == 1 && (tmp & 0x0004) != 0 && (index * 8) > LDTR.limit',
                ['seg_valid = 0'],
                'end if',
                ';; S bit check (GDT or LDT)',
                'if seg_valid == 1 && (tmp & 0x0004) == 0 && RAM.GDT.gates[index].SD.S != 1',
                ['seg_valid = 0'],
                'end if',
                'if seg_valid == 1 && (tmp & 0x0004) != 0 && RAM.LDT.gates[index].SD.S != 1',
                ['seg_valid = 0'],
                'end if',
                ';; read type field (GDT or LDT)',
                'desc_type = 0',
                'if seg_valid == 1 && (tmp & 0x0004) == 0',
                ['desc_type = RAM.GDT.gates[index].SD.type'],
                'end if',
                'if seg_valid == 1 && (tmp & 0x0004) != 0',
                ['desc_type = RAM.LDT.gates[index].SD.type'],
                'end if',
                ';; non-readable code segment is invalid',
                'if seg_valid == 1 && (desc_type & 0b100) == 0b100 && (desc_type & 0b001) == 0',
                ['seg_valid = 0'],
                'end if',
                ';; data or non-conforming code: DPL must be >= CPL or >= RPL',
                'if seg_valid == 1 && (desc_type & 0b110) != 0b110 && (tmp & 0x0004) == 0 && RAM.GDT.gates[index].SD.DPL < ret_rpl && RAM.GDT.gates[index].SD.DPL < (ES & 0x0003)',
                ['seg_valid = 0'],
                'end if',
                'if seg_valid == 1 && (desc_type & 0b110) != 0b110 && (tmp & 0x0004) != 0 && RAM.LDT.gates[index].SD.DPL < ret_rpl && RAM.LDT.gates[index].SD.DPL < (ES & 0x0003)',
                ['seg_valid = 0'],
                'end if',
                'if seg_valid == 0 && index != 0',
                [
                  'ES = 0',
                  'ES_BASE = 0',
                  'ES_LIMIT_MIN = 0xffff',
                  'ES_LIMIT_MAX = 0',
                  'ES_ACCESS = 0',
                ],
                'end if',
                ';; invalidate DS if not valid for outer privilege level',
                'index = DS >> 3',
                'tmp = DS',
                'seg_valid = 1',
                'if index == 0',
                ['seg_valid = 0'],
                'end if',
                ';; bounds check (GDT or LDT based on TI bit)',
                'if seg_valid == 1 && (tmp & 0x0004) == 0 && (index * 8) > GDTR.limit',
                ['seg_valid = 0'],
                'end if',
                'if seg_valid == 1 && (tmp & 0x0004) != 0 && (index * 8) > LDTR.limit',
                ['seg_valid = 0'],
                'end if',
                ';; S bit check (GDT or LDT)',
                'if seg_valid == 1 && (tmp & 0x0004) == 0 && RAM.GDT.gates[index].SD.S != 1',
                ['seg_valid = 0'],
                'end if',
                'if seg_valid == 1 && (tmp & 0x0004) != 0 && RAM.LDT.gates[index].SD.S != 1',
                ['seg_valid = 0'],
                'end if',
                ';; read type field (GDT or LDT)',
                'desc_type = 0',
                'if seg_valid == 1 && (tmp & 0x0004) == 0',
                ['desc_type = RAM.GDT.gates[index].SD.type'],
                'end if',
                'if seg_valid == 1 && (tmp & 0x0004) != 0',
                ['desc_type = RAM.LDT.gates[index].SD.type'],
                'end if',
                ';; non-readable code segment is invalid',
                'if seg_valid == 1 && (desc_type & 0b100) == 0b100 && (desc_type & 0b001) == 0',
                ['seg_valid = 0'],
                'end if',
                ';; data or non-conforming code: DPL must be >= CPL or >= RPL',
                'if seg_valid == 1 && (desc_type & 0b110) != 0b110 && (tmp & 0x0004) == 0 && RAM.GDT.gates[index].SD.DPL < ret_rpl && RAM.GDT.gates[index].SD.DPL < (DS & 0x0003)',
                ['seg_valid = 0'],
                'end if',
                'if seg_valid == 1 && (desc_type & 0b110) != 0b110 && (tmp & 0x0004) != 0 && RAM.LDT.gates[index].SD.DPL < ret_rpl && RAM.LDT.gates[index].SD.DPL < (DS & 0x0003)',
                ['seg_valid = 0'],
                'end if',
                'if seg_valid == 0 && index != 0',
                [
                  'DS = 0',
                  'DS_BASE = 0',
                  'DS_LIMIT_MIN = 0xffff',
                  'DS_LIMIT_MAX = 0',
                  'DS_ACCESS = 0',
                  'DS_BASE1 = 0',
                  'DS_BASE2 = 0',
                  'DS_BASE3 = 0',
                  'DS_BASE4 = 0',
                  'DS_LIMIT_MIN1 = 0xffff',
                  'DS_LIMIT_MIN2 = 0xffff',
                  'DS_LIMIT_MIN3 = 0xffff',
                  'DS_LIMIT_MIN4 = 0xffff',
                  'DS_LIMIT_MAX1 = 0',
                  'DS_LIMIT_MAX2 = 0',
                  'DS_LIMIT_MAX3 = 0',
                  'DS_LIMIT_MAX4 = 0',
                  'DS_ACCESS1 = 0',
                  'DS_ACCESS2 = 0',
                  'DS_ACCESS3 = 0',
                  'DS_ACCESS4 = 0',
                ],
                'end if',
              ],
              'end if',
            ],
            'end if',
          ],
        },
      },
      opcode: [Opcodes.IRET],
      operands: [],
      operandSize: 16,
      cycles: 17, // protected-mode: 31, 55 if lesser privilege, 169 if switching tasks (NT = 1)
    },
  ],
};
