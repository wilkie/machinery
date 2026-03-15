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
  macros: {
    // Protected-mode far RET logic — handles same-privilege and outer-privilege returns.
    // Expects locals: offset, stack_address, ret_cs, ret_rpl, old_cpl,
    //   ret_sp, ret_ss, ret_imm, tmp, index, desc_type, desc_s, desc_dpl, desc_p,
    //   desc_limit, desc_base, seg_valid.
    // ret_imm must be set before invocation (0 for RETF, %{imm} for RETF dw).
    RETF_PROTECTED: [
      'offset = SP',
      'stack_address = SS_BASE + offset',
      ';; At least 4 bytes (IP + CS) must be readable',
      '#SS if (offset + 3) < SS_LIMIT_MIN',
      '#SS if (offset + 3) > SS_LIMIT_MAX',

      ';; Read return CS to determine privilege transition',
      'ret_cs = RAM:u16[stack_address + 2]',
      'ret_rpl = ret_cs & 0x0003',
      'old_cpl = CS.RPL',

      ';; Return CS RPL must be >= CPL',
      'ERROR_CODE = ret_cs',
      '#GP if ret_rpl < old_cpl',

      ';; Validate return CS selector',
      'tmp = ret_cs',
      'index = tmp >> 3',
      '#GP if index == 0',
      '${DESCRIPTOR_BOUNDS_CHECK}',
      '${LOAD_DESCRIPTOR_FIELDS}',
      '#GP if desc_s != 1',
      '#GP if (desc_type & 0b100) != 0b100',
      ';; Non-conforming: DPL must equal return RPL',
      'if (desc_type & 0b010) != 0b010',
      ['#GP if desc_dpl != ret_rpl'],
      'end if',
      ';; Conforming: DPL must be <= return RPL',
      'if (desc_type & 0b010) == 0b010',
      ['#GP if desc_dpl > ret_rpl'],
      'end if',
      ';; Must be present',
      '#NP if desc_p != 1',

      ';; === RETURN TO SAME PRIVILEGE LEVEL ===',
      'if ret_rpl == old_cpl',
      [
        'IP = RAM:u16[stack_address]',
        'CS = ret_cs',
        'SP = SP + 4 + ret_imm',
      ],
      'end if',

      ';; === RETURN TO OUTER PRIVILEGE LEVEL ===',
      'if ret_rpl > old_cpl',
      [
        ';; Check stack has room for full outer return frame',
        '#SS if (offset + 7 + ret_imm) > SS_LIMIT_MAX',

        ';; Read return SS:SP from beyond the parameter block',
        'ret_sp = RAM:u16[stack_address + 4 + ret_imm]',
        'ret_ss = RAM:u16[stack_address + 6 + ret_imm]',

        ';; Load CS:IP from stack (CS setter validates and loads cache)',
        'IP = RAM:u16[stack_address]',
        'CS = ret_cs',

        ';; Load SS:SP (SS setter validates against new CS.RPL)',
        'SS = ret_ss',
        'SP = ret_sp',

        ';; Invalidate ES if not valid for outer privilege level',
        'index = ES >> 3',
        'seg_valid = 1',
        'if index == 0',
        ['seg_valid = 0'],
        'end if',
        'if seg_valid == 1 && (index * 8) > GDTR.limit',
        ['seg_valid = 0'],
        'end if',
        'if seg_valid == 1 && RAM.GDT.gates[index].SD.S != 1',
        ['seg_valid = 0'],
        'end if',
        'desc_type = 0',
        'if seg_valid == 1',
        ['desc_type = RAM.GDT.gates[index].SD.type'],
        'end if',
        ';; Non-readable code segment is invalid',
        'if seg_valid == 1 && (desc_type & 0b100) == 0b100 && (desc_type & 0b001) == 0',
        ['seg_valid = 0'],
        'end if',
        ';; Data or non-conforming code: DPL must be >= CPL or >= RPL',
        'if seg_valid == 1 && (desc_type & 0b110) != 0b110 && RAM.GDT.gates[index].SD.DPL < ret_rpl && RAM.GDT.gates[index].SD.DPL < (ES & 0x0003)',
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

        ';; Invalidate DS if not valid for outer privilege level',
        'index = DS >> 3',
        'seg_valid = 1',
        'if index == 0',
        ['seg_valid = 0'],
        'end if',
        'if seg_valid == 1 && (index * 8) > GDTR.limit',
        ['seg_valid = 0'],
        'end if',
        'if seg_valid == 1 && RAM.GDT.gates[index].SD.S != 1',
        ['seg_valid = 0'],
        'end if',
        'desc_type = 0',
        'if seg_valid == 1',
        ['desc_type = RAM.GDT.gates[index].SD.type'],
        'end if',
        'if seg_valid == 1 && (desc_type & 0b100) == 0b100 && (desc_type & 0b001) == 0',
        ['seg_valid = 0'],
        'end if',
        'if seg_valid == 1 && (desc_type & 0b110) != 0b110 && RAM.GDT.gates[index].SD.DPL < ret_rpl && RAM.GDT.gates[index].SD.DPL < (DS & 0x0003)',
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
  },
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
    // Protected-mode privilege transition support
    {
      identifier: 'ret_cs',
      name: 'Return CS selector',
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
      identifier: 'ret_imm',
      name: 'Immediate parameter block size',
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
          operation: ['ret_imm = 0', '${RETF_PROTECTED}'],
        },
      },
      opcode: [Opcodes.RETF],
      operands: [],
      operandSize: 16,
      distance: 'far',
      cycles: 15,
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
          operation: ['ret_imm = %{imm}', '${RETF_PROTECTED}'],
        },
      },
      opcode: [Opcodes.RETF_DW, 'IMM_u16'],
      operands: ['imm'],
      operandSize: 16,
      distance: 'far',
      cycles: 15,
    },
  ],
};
