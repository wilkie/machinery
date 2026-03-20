import type { InstructionInfo } from '@machinery/core';
import { InstructionDataTypes, InstructionOperandTypes } from '@machinery/core';

import { Opcodes } from '../opcodes';

// TODO: segment moves
// TODO: protected mode (cannot mov to CS... on real mode, it seems possible even if undocumented)
// TODO: #13 for operand 0xffff
// TODO: #UD for FS, GS perhaps, if emulating a real 286
// TODO: #UD for abnormal segment selectors (reg/rm: 110 and 111)
export const mov: InstructionInfo = {
  identifier: 'mov',
  name: 'Move Data',
  description:
    "The second operand is copied to the first operand. If the destination operand is a segment register (`DS`, `ES`, or `SS`), then the associated segment register cache is also loaded. The data for the cache is obtained from the descriptor table entry for the selector given.\n\nA null selector (values 0000-0003) can be loaded into `DS` and `ES` registers without causing a protection exception. Any use of a segment register with a null selector to address memory will cause `#GP(0)` exception. No memory reference will occur.\n\nAny move into `SS` will inhibit all interrupts until after the execution of the next instruction.\n\nFollowing is a listing of the protected-mode checks and actions taken in the loading of a segment register:\n\n```\nIf SS is loaded:\n  If selector is null then #GP(0)\n  Selector index must be within its descriptor table limits else #GP (selector)\n  Selector's RPL must equal CPL else #GP (selector)\n  AR byte must indicate a writable data segment else #GP (selector)\n  DPL in the AR byte must equal CPL else #GP (selector)\n  Segment must be marked PRESENT else #SS (selector)\n  Load SS with selector\n  Load SS cache with descriptor\nIf ES or DS is loaded with non-null selector\n  Selector index must be within its descriptor table limits else #GP (selector)\n  AR byte must indicate data or readable code segment else #GP (selector)\n  If data or non-conforming code, then both the RPL and the\n    CPL must be less than or equal to OPL in AR byte else #GP (selector)\n  Segment must be marked PRESENT else #NP (selector)\nLoad segment register with selector\nLoad segment register cache with descriptor\nIf ES or DS is loaded with a null selector:\n  Load segment register with selector\n  Clear descriptor valid bit\n```",
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
    {
      identifier: 'seg_index',
      name: 'Segment Register Index',
      size: 16,
    },
  ],
  forms: [
    // 0x88 /r - MOV eb, rb
    {
      modes: {
        real: {
          operation: [
            'effective_address = ${MOD_RM_SEGMENT} + %{DISP:u16}',
            'RAM:u8[effective_address] = ${MOD_RM_REG8}',
          ],
        },
        protected: {
          operation: [
            'offset = %{DISP:u16}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'RAM:u8[effective_address] = ${MOD_RM_REG8}',
          ],
        },
      },
      opcode: [Opcodes.MOV_EB_RB, 'ModRM_110_reg8_00', 'DISP_i16'],
      operands: ['rm', 'reg'],
      operandSize: 8,
      cycles: 3,
    },
    {
      modes: {
        real: {
          operation: [
            'effective_address = ${MOD_RM_SEGMENT} + (${MOD_RM_OFFSET}):u16',
            'RAM:u8[effective_address] = ${MOD_RM_REG8}',
          ],
        },
        protected: {
          operation: [
            'offset = (${MOD_RM_OFFSET}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'RAM:u8[effective_address] = ${MOD_RM_REG8}',
          ],
        },
      },
      opcode: [Opcodes.MOV_EB_RB, 'ModRM_rm_reg8_00'],
      operands: ['rm', 'reg'],
      operandSize: 8,
      cycles: 3,
    },
    {
      modes: {
        real: {
          operation: [
            'effective_address = ${MOD_RM_SEGMENT} + (${MOD_RM_OFFSET} + %{DISP}):u16',
            'RAM:u8[effective_address] = ${MOD_RM_REG8}',
          ],
        },
        protected: {
          operation: [
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'RAM:u8[effective_address] = ${MOD_RM_REG8}',
          ],
        },
      },
      opcode: [Opcodes.MOV_EB_RB, 'ModRM_rm_reg8_01', 'DISP_i8'],
      operands: ['rm', 'reg'],
      operandSize: 8,
      cycles: 3,
    },
    {
      modes: {
        real: {
          operation: [
            'effective_address = ${MOD_RM_SEGMENT} + (${MOD_RM_OFFSET} + %{DISP}):u16',
            'RAM:u8[effective_address] = ${MOD_RM_REG8}',
          ],
        },
        protected: {
          operation: [
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'RAM:u8[effective_address] = ${MOD_RM_REG8}',
          ],
        },
      },
      opcode: [Opcodes.MOV_EB_RB, 'ModRM_rm_reg8_10', 'DISP_i16'],
      operands: ['rm', 'reg'],
      operandSize: 8,
      cycles: 3,
    },
    {
      operation: ['${MOD_RM_RM8} = ${MOD_RM_REG8}'],
      opcode: [Opcodes.MOV_EB_RB, 'ModRM_rm8_reg8_11'],
      operands: ['rm', 'reg'],
      operandSize: 8,
      cycles: 2,
    },
    // 0x89 /r - MOV ew, rw
    {
      modes: {
        real: {
          operation: [
            'offset = %{DISP:u16}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'RAM:u16[effective_address] = ${MOD_RM_REG16}',
          ],
        },
        protected: {
          operation: [
            'offset = %{DISP:u16}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'RAM:u16[effective_address] = ${MOD_RM_REG16}',
          ],
        },
      },
      opcode: [Opcodes.MOV_EW_RW, 'ModRM_110_reg16_00', 'DISP_i16'],
      operands: ['rm', 'reg'],
      operandSize: 16,
      cycles: 3,
    },
    {
      modes: {
        real: {
          operation: [
            'offset = (${MOD_RM_OFFSET}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'RAM:u16[effective_address] = ${MOD_RM_REG16}',
          ],
        },
        protected: {
          operation: [
            'offset = (${MOD_RM_OFFSET}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'RAM:u16[effective_address] = ${MOD_RM_REG16}',
          ],
        },
      },
      opcode: [Opcodes.MOV_EW_RW, 'ModRM_rm_reg16_00'],
      operands: ['rm', 'reg'],
      operandSize: 16,
      cycles: 3,
    },
    {
      modes: {
        real: {
          operation: [
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'RAM:u16[effective_address] = ${MOD_RM_REG16}',
          ],
        },
        protected: {
          operation: [
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'RAM:u16[effective_address] = ${MOD_RM_REG16}',
          ],
        },
      },
      opcode: [Opcodes.MOV_EW_RW, 'ModRM_rm_reg16_01', 'DISP_i8'],
      operands: ['rm', 'reg'],
      operandSize: 16,
      cycles: 3,
    },
    {
      modes: {
        real: {
          operation: [
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'RAM:u16[effective_address] = ${MOD_RM_REG16}',
          ],
        },
        protected: {
          operation: [
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'RAM:u16[effective_address] = ${MOD_RM_REG16}',
          ],
        },
      },
      opcode: [Opcodes.MOV_EW_RW, 'ModRM_rm_reg16_10', 'DISP_i16'],
      operands: ['rm', 'reg'],
      operandSize: 16,
      cycles: 3,
    },
    {
      operation: ['${MOD_RM_RM16} = ${MOD_RM_REG16}'],
      opcode: [Opcodes.MOV_EW_RW, 'ModRM_rm16_reg16_11'],
      operands: ['rm', 'reg'],
      operandSize: 16,
      cycles: 2,
    },
    // 0x8a /r - MOV rb, eb
    {
      modes: {
        real: {
          operation: [
            'effective_address = ${MOD_RM_SEGMENT} + %{DISP:u16}',
            '${MOD_RM_REG8} = RAM:u8[effective_address]',
          ],
        },
        protected: {
          operation: [
            'offset = %{DISP:u16}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            '${MOD_RM_REG8} = RAM:u8[effective_address]',
          ],
        },
      },
      opcode: [Opcodes.MOV_RB_EB, 'ModRM_110_reg8_00', 'DISP_i16'],
      operands: ['reg', 'rm'],
      operandSize: 8,
      cycles: 5,
    },
    {
      modes: {
        real: {
          operation: [
            'effective_address = ${MOD_RM_SEGMENT} + (${MOD_RM_OFFSET}):u16',
            '${MOD_RM_REG8} = RAM:u8[effective_address]',
          ],
        },
        protected: {
          operation: [
            'offset = (${MOD_RM_OFFSET}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            '${MOD_RM_REG8} = RAM:u8[effective_address]',
          ],
        },
      },
      opcode: [Opcodes.MOV_RB_EB, 'ModRM_rm_reg8_00'],
      operands: ['reg', 'rm'],
      operandSize: 8,
      cycles: 5,
    },
    {
      modes: {
        real: {
          operation: [
            'effective_address = ${MOD_RM_SEGMENT} + (${MOD_RM_OFFSET} + %{DISP}):u16',
            '${MOD_RM_REG8} = RAM:u8[effective_address]',
          ],
        },
        protected: {
          operation: [
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            '${MOD_RM_REG8} = RAM:u8[effective_address]',
          ],
        },
      },
      opcode: [Opcodes.MOV_RB_EB, 'ModRM_rm_reg8_01', 'DISP_i8'],
      operands: ['reg', 'rm'],
      operandSize: 8,
      cycles: 5,
    },
    {
      modes: {
        real: {
          operation: [
            'effective_address = ${MOD_RM_SEGMENT} + (${MOD_RM_OFFSET} + %{DISP}):u16',
            '${MOD_RM_REG8} = RAM:u8[effective_address]',
          ],
        },
        protected: {
          operation: [
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            '${MOD_RM_REG8} = RAM:u8[effective_address]',
          ],
        },
      },
      opcode: [Opcodes.MOV_RB_EB, 'ModRM_rm_reg8_10', 'DISP_i16'],
      operands: ['reg', 'rm'],
      operandSize: 8,
      cycles: 5,
    },
    {
      operation: ['${MOD_RM_REG8} = ${MOD_RM_RM8}'],
      opcode: [Opcodes.MOV_RB_EB, 'ModRM_rm8_reg8_11'],
      operands: ['reg', 'rm'],
      operandSize: 8,
      cycles: 2,
    },
    // 0x8b /r - MOV rw, ew
    {
      modes: {
        real: {
          operation: [
            'offset = %{DISP:u16}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            '${MOD_RM_REG16} = RAM:u16[effective_address]',
          ],
        },
        protected: {
          operation: [
            'offset = %{DISP:u16}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            '${MOD_RM_REG16} = RAM:u16[effective_address]',
          ],
        },
      },
      opcode: [Opcodes.MOV_RW_EW, 'ModRM_110_reg16_00', 'DISP_i16'],
      operands: ['reg', 'rm'],
      operandSize: 16,
      cycles: 5,
    },
    {
      modes: {
        real: {
          operation: [
            'offset = (${MOD_RM_OFFSET}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            '${MOD_RM_REG16} = RAM:u16[effective_address]',
          ],
        },
        protected: {
          operation: [
            'offset = (${MOD_RM_OFFSET}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            '${MOD_RM_REG16} = RAM:u16[effective_address]',
          ],
        },
      },
      opcode: [Opcodes.MOV_RW_EW, 'ModRM_rm_reg16_00'],
      operands: ['reg', 'rm'],
      operandSize: 16,
      cycles: 5,
    },
    {
      modes: {
        real: {
          operation: [
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            '${MOD_RM_REG16} = RAM:u16[effective_address]',
          ],
        },
        protected: {
          operation: [
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            '${MOD_RM_REG16} = RAM:u16[effective_address]',
          ],
        },
      },
      opcode: [Opcodes.MOV_RW_EW, 'ModRM_rm_reg16_01', 'DISP_i8'],
      operands: ['reg', 'rm'],
      operandSize: 16,
      cycles: 5,
    },
    {
      modes: {
        real: {
          operation: [
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            '${MOD_RM_REG16} = RAM:u16[effective_address]',
          ],
        },
        protected: {
          operation: [
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            '${MOD_RM_REG16} = RAM:u16[effective_address]',
          ],
        },
      },
      opcode: [Opcodes.MOV_RW_EW, 'ModRM_rm_reg16_10', 'DISP_i16'],
      operands: ['reg', 'rm'],
      operandSize: 16,
      cycles: 5,
    },
    {
      operation: ['${MOD_RM_REG16} = ${MOD_RM_RM16}'],
      opcode: [Opcodes.MOV_RW_EW, 'ModRM_rm16_reg16_11'],
      operands: ['reg', 'rm'],
      operandSize: 16,
      cycles: 2,
    },
    // 0x8c /0 - MOV ew, ES
    // 0x8c /1 - MOV ew, CS
    // 0x8c /2 - MOV ew, SS
    // 0x8c /3 - MOV ew, DS
    {
      modes: {
        real: {
          operation: [
            'offset = %{DISP:u16}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'seg_index = %{seg}',
            '#UD if seg_index > 3',
            'seg_index = %{seg}',
            '#UD if seg_index > 3',
            'RAM:u16[effective_address] = ${MOD_RM_REGS16}',
          ],
        },
        protected: {
          operation: [
            'offset = %{DISP:u16}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'seg_index = %{seg}',
            '#UD if seg_index > 3',
            'seg_index = %{seg}',
            '#UD if seg_index > 3',
            'RAM:u16[effective_address] = ${MOD_RM_REGS16}',
          ],
        },
      },
      opcode: [Opcodes.MOV_EW_SR, 'ModRM_110_seg_00', 'DISP_i16'],
      operands: ['rm', 'seg'],
      operandSize: 16,
      cycles: 3,
    },
    {
      modes: {
        real: {
          operation: [
            'offset = (${MOD_RM_OFFSET}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'seg_index = %{seg}',
            '#UD if seg_index > 3',
            'RAM:u16[effective_address] = ${MOD_RM_REGS16}',
          ],
        },
        protected: {
          operation: [
            'offset = (${MOD_RM_OFFSET}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'seg_index = %{seg}',
            '#UD if seg_index > 3',
            'RAM:u16[effective_address] = ${MOD_RM_REGS16}',
          ],
        },
      },
      opcode: [Opcodes.MOV_EW_SR, 'ModRM_rm_seg_00'],
      operands: ['rm', 'seg'],
      operandSize: 16,
      cycles: 3,
    },
    {
      modes: {
        real: {
          operation: [
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'seg_index = %{seg}',
            '#UD if seg_index > 3',
            'RAM:u16[effective_address] = ${MOD_RM_REGS16}',
          ],
        },
        protected: {
          operation: [
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'seg_index = %{seg}',
            '#UD if seg_index > 3',
            'RAM:u16[effective_address] = ${MOD_RM_REGS16}',
          ],
        },
      },
      opcode: [Opcodes.MOV_EW_SR, 'ModRM_rm_seg_01', 'DISP_i8'],
      operands: ['rm', 'seg'],
      operandSize: 16,
      cycles: 3,
    },
    {
      modes: {
        real: {
          operation: [
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'seg_index = %{seg}',
            '#UD if seg_index > 3',
            'RAM:u16[effective_address] = ${MOD_RM_REGS16}',
          ],
        },
        protected: {
          operation: [
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'seg_index = %{seg}',
            '#UD if seg_index > 3',
            'RAM:u16[effective_address] = ${MOD_RM_REGS16}',
          ],
        },
      },
      opcode: [Opcodes.MOV_EW_SR, 'ModRM_rm_seg_10', 'DISP_i16'],
      operands: ['rm', 'seg'],
      operandSize: 16,
      cycles: 3,
    },
    {
      operation: [
        'seg_index = %{seg}',
        '#UD if seg_index > 3',
        '${MOD_RM_RM16} = ${MOD_RM_REGS16}',
      ],
      opcode: [Opcodes.MOV_EW_SR, 'ModRM_rm_seg_11'],
      operands: ['rm', 'seg'],
      operandSize: 16,
      cycles: 2,
    },
    // 0x8e /0 - MOV ES, ew
    // 0x8e /1 - MOV CS, ew (undocumented, but valid in !PE real mode)
    // 0x8e /2 - MOV SS, ew
    // 0x8e /3 - MOV DS, ew
    {
      modes: {
        real: {
          operation: [
            'offset = %{DISP:u16}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            'seg_index = %{seg}',
            '#UD if seg_index == 1 || seg_index > 3',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            '${MOD_RM_REGS16} = RAM:u16[effective_address]',
          ],
        },
        protected: {
          operation: [
            '#UD if %{seg} == 1',
            'offset = %{DISP:u16}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            'seg_index = %{seg}',
            '#UD if seg_index == 1 || seg_index > 3',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            '${MOD_RM_REGS16} = RAM:u16[effective_address]',
          ],
        },
      },
      opcode: [Opcodes.MOV_SR_EW, 'ModRM_110_seg_00', 'DISP_i16'],
      operands: ['seg', 'rm'],
      operandSize: 16,
      cycles: 3,
    },
    {
      modes: {
        real: {
          operation: [
            'offset = (${MOD_RM_OFFSET}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            'seg_index = %{seg}',
            '#UD if seg_index == 1 || seg_index > 3',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            '${MOD_RM_REGS16} = RAM:u16[effective_address]',
          ],
        },
        protected: {
          operation: [
            '#UD if %{seg} == 1',
            'offset = (${MOD_RM_OFFSET}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            'seg_index = %{seg}',
            '#UD if seg_index == 1 || seg_index > 3',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            '${MOD_RM_REGS16} = RAM:u16[effective_address]',
          ],
        },
      },
      opcode: [Opcodes.MOV_SR_EW, 'ModRM_rm_seg_00'],
      operands: ['seg', 'rm'],
      operandSize: 16,
      cycles: 3,
    },
    {
      modes: {
        real: {
          operation: [
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            'seg_index = %{seg}',
            '#UD if seg_index == 1 || seg_index > 3',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            '${MOD_RM_REGS16} = RAM:u16[effective_address]',
          ],
        },
        protected: {
          operation: [
            '#UD if %{seg} == 1',
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            'seg_index = %{seg}',
            '#UD if seg_index == 1 || seg_index > 3',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            '${MOD_RM_REGS16} = RAM:u16[effective_address]',
          ],
        },
      },
      opcode: [Opcodes.MOV_SR_EW, 'ModRM_rm_seg_01', 'DISP_i8'],
      operands: ['seg', 'rm'],
      operandSize: 16,
      cycles: 3,
    },
    {
      modes: {
        real: {
          operation: [
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            'seg_index = %{seg}',
            '#UD if seg_index == 1 || seg_index > 3',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            '${MOD_RM_REGS16} = RAM:u16[effective_address]',
          ],
        },
        protected: {
          operation: [
            '#UD if %{seg} == 1',
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            'seg_index = %{seg}',
            '#UD if seg_index == 1 || seg_index > 3',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            '${MOD_RM_REGS16} = RAM:u16[effective_address]',
          ],
        },
      },
      opcode: [Opcodes.MOV_SR_EW, 'ModRM_rm_seg_10', 'DISP_i16'],
      operands: ['seg', 'rm'],
      operandSize: 16,
      cycles: 3,
    },
    {
      modes: {
        real: {
          operation: [
            'seg_index = %{seg}',
            '#UD if seg_index == 1 || seg_index > 3',
            '${MOD_RM_REGS16} = ${MOD_RM_RM16}',
          ],
        },
        protected: {
          operation: [
            'seg_index = %{seg}',
            '#UD if seg_index == 1 || seg_index > 3',
            '${MOD_RM_REGS16} = ${MOD_RM_RM16}',
          ],
        },
      },
      opcode: [Opcodes.MOV_SR_EW, 'ModRM_rm_seg_11'],
      operands: ['seg', 'rm'],
      operandSize: 16,
      cycles: 2,
    },
    // 0xA0 dw - MOV AL, xb
    {
      operation: [
        'AL = RAM:u8[(DATA_SEG_BASE == 0xffff ? DS_BASE : DATA_SEG_BASE) + %{mem}]',
      ],
      opcode: [Opcodes.MOV_AL_XB, 'IMM_MEM_u16'],
      operands: ['AL', 'mem'],
      operandSize: 8,
      cycles: 5,
    },
    // 0xA1 dw - MOV AX, xw
    {
      operation: [
        'AX = RAM:u16[(DATA_SEG_BASE == 0xffff ? DS_BASE : DATA_SEG_BASE) + %{mem}]',
      ],
      opcode: [Opcodes.MOV_AX_XW, 'IMM_MEM_u16'],
      operands: ['AX', 'mem'],
      operandSize: 16,
      cycles: 5,
    },
    // 0xA2 dw - MOV xb, AL
    {
      operation: [
        'RAM:u8[(DATA_SEG_BASE == 0xffff ? DS_BASE : DATA_SEG_BASE) + %{mem}] = AL',
      ],
      opcode: [Opcodes.MOV_XB_AL, 'IMM_MEM_u16'],
      operands: ['mem', 'AL'],
      operandSize: 8,
      cycles: 3,
    },
    // 0xA3 dw - MOV xw, AX
    {
      operation: [
        'RAM:u16[(DATA_SEG_BASE == 0xffff ? DS_BASE : DATA_SEG_BASE) + %{mem}] = AX',
      ],
      opcode: [Opcodes.MOV_XW_AX, 'IMM_MEM_u16'],
      operands: ['mem', 'AX'],
      operandSize: 16,
      cycles: 3,
    },
    // 0xB0+rb db - MOV rb, db
    {
      operation: ['${MOD_RM_RM8} = %{imm}'],
      opcode: [
        {
          identifier: 'MOV',
          name: 'MOV ModRM Field',
          type: InstructionDataTypes.Operand,
          size: 8,
          fields: [
            {
              identifier: 'opcode',
              offset: 3,
              size: 5,
              match: 0b10110,
            },
            {
              identifier: 'rm',
              offset: 0,
              size: 3,
              type: InstructionOperandTypes.Register,
              encoding: ['AL', 'CL', 'DL', 'BL', 'AH', 'CH', 'DH', 'BH'],
            },
          ],
        },
        'IMM_u8',
      ],
      operands: ['rm', 'imm'],
      operandSize: 8,
      cycles: 2,
    },
    // 0xB8+rb dw - MOV rw, dw
    {
      operation: ['${MOD_RM_RM16} = %{imm}'],
      opcode: [
        {
          identifier: 'MOV',
          name: 'MOV ModRM Field',
          type: InstructionDataTypes.Operand,
          size: 8,
          fields: [
            {
              identifier: 'opcode',
              offset: 3,
              size: 5,
              match: 0b10111,
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
        'IMM_u16',
      ],
      operands: ['rm', 'imm'],
      operandSize: 16,
      cycles: 2,
    },
    // 0xC6 /0 db - MOV eb, db
    {
      modes: {
        real: {
          operation: [
            'effective_address = ${MOD_RM_SEGMENT} + %{DISP:u16}',
            'RAM:u8[effective_address] = %{imm}',
          ],
        },
        protected: {
          operation: [
            'offset = %{DISP:u16}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'RAM:u8[effective_address] = %{imm}',
          ],
        },
      },
      opcode: [Opcodes.MOV_EB_DB, 'ModRM_110_000_00', 'DISP_i16', 'IMM_u8'],
      operands: ['rm', 'imm'],
      operandSize: 8,
      cycles: 3,
    },
    {
      modes: {
        real: {
          operation: [
            'effective_address = ${MOD_RM_SEGMENT} + (${MOD_RM_OFFSET}):u16',
            'RAM:u8[effective_address] = %{imm}',
          ],
        },
        protected: {
          operation: [
            'offset = (${MOD_RM_OFFSET}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'RAM:u8[effective_address] = %{imm}',
          ],
        },
      },
      opcode: [Opcodes.MOV_EB_DB, 'ModRM_rm_000_00', 'IMM_u8'],
      operands: ['rm', 'imm'],
      operandSize: 8,
      cycles: 3,
    },
    {
      modes: {
        real: {
          operation: [
            'effective_address = ${MOD_RM_SEGMENT} + (${MOD_RM_OFFSET} + %{DISP}):u16',
            'RAM:u8[effective_address] = %{imm}',
          ],
        },
        protected: {
          operation: [
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'RAM:u8[effective_address] = %{imm}',
          ],
        },
      },
      opcode: [Opcodes.MOV_EB_DB, 'ModRM_rm_000_01', 'DISP_i8', 'IMM_u8'],
      operands: ['rm', 'imm'],
      operandSize: 8,
      cycles: 3,
    },
    {
      modes: {
        real: {
          operation: [
            'effective_address = ${MOD_RM_SEGMENT} + (${MOD_RM_OFFSET} + %{DISP}):u16',
            'RAM:u8[effective_address] = %{imm}',
          ],
        },
        protected: {
          operation: [
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED8}',
            'RAM:u8[effective_address] = %{imm}',
          ],
        },
      },
      opcode: [Opcodes.MOV_EB_DB, 'ModRM_rm_000_10', 'DISP_i16', 'IMM_u8'],
      operands: ['rm', 'imm'],
      operandSize: 8,
      cycles: 3,
    },
    {
      operation: ['${MOD_RM_RM8} = %{imm}'],
      opcode: [Opcodes.MOV_EB_DB, 'ModRM_rm8_000_11', 'IMM_u8'],
      operands: ['rm', 'imm'],
      operandSize: 8,
      cycles: 2,
    },
    // 0xC7 /0 db - MOV ew, dw
    {
      modes: {
        real: {
          operation: [
            'offset = %{DISP:u16}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'RAM:u16[effective_address] = %{imm}',
          ],
        },
        protected: {
          operation: [
            'offset = %{DISP:u16}',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'RAM:u16[effective_address] = %{imm}',
          ],
        },
      },
      opcode: [Opcodes.MOV_EW_DW, 'ModRM_110_000_00', 'DISP_i16', 'IMM_u16'],
      operands: ['rm', 'imm'],
      operandSize: 16,
      cycles: 3,
    },
    {
      modes: {
        real: {
          operation: [
            'offset = (${MOD_RM_OFFSET}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'RAM:u16[effective_address] = %{imm}',
          ],
        },
        protected: {
          operation: [
            'offset = (${MOD_RM_OFFSET}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'RAM:u16[effective_address] = %{imm}',
          ],
        },
      },
      opcode: [Opcodes.MOV_EW_DW, 'ModRM_rm_000_00', 'IMM_u16'],
      operands: ['rm', 'imm'],
      operandSize: 16,
      cycles: 3,
    },
    {
      modes: {
        real: {
          operation: [
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'RAM:u16[effective_address] = %{imm}',
          ],
        },
        protected: {
          operation: [
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'RAM:u16[effective_address] = %{imm}',
          ],
        },
      },
      opcode: [Opcodes.MOV_EW_DW, 'ModRM_rm_000_01', 'DISP_i8', 'IMM_u16'],
      operands: ['rm', 'imm'],
      operandSize: 16,
      cycles: 3,
    },
    {
      modes: {
        real: {
          operation: [
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_REAL}',
            'RAM:u16[effective_address] = %{imm}',
          ],
        },
        protected: {
          operation: [
            'offset = (${MOD_RM_OFFSET} + %{DISP}):u16',
            'effective_address = ${MOD_RM_SEGMENT} + offset',
            '${SEGMENT_LIMIT_CHECK_PROTECTED16}',
            'RAM:u16[effective_address] = %{imm}',
          ],
        },
      },
      opcode: [Opcodes.MOV_EW_DW, 'ModRM_rm_000_10', 'DISP_i16', 'IMM_u16'],
      operands: ['rm', 'imm'],
      operandSize: 16,
      cycles: 3,
    },
    {
      operation: ['${MOD_RM_RM16} = %{imm}'],
      opcode: [Opcodes.MOV_EW_DW, 'ModRM_rm16_000_11', 'IMM_u16'],
      operands: ['rm', 'imm'],
      operandSize: 16,
      cycles: 2,
    },
  ],
};
