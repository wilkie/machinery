import type { InstructionInfo } from '@machinery/core';
import { InstructionDataTypes } from '@machinery/core';

import { Opcodes } from '../opcodes';

export const enter: InstructionInfo = {
  identifier: 'enter',
  name: 'Make Stack Frame for Procedure Parameters',
  description:
    '`ENTER` is used to create the stack frame required by most block-structured high-level languages. the first operand specifies how many bytes of dynamic storage are to be allocated on the stack for the routine being entered. The second operand gives the lexical nesting level of the routine within the highlevel-language source code. It determines how many stack frame pointers are copied into the new stack frame from the preceding frame. `BP` is used as the current stack frame pointer.\n\nIf the second operand is `0`, `ENTER` pushes `BP`, sets `BP` to `SP`, and subtracts the first operand from `SP`.\n\nFor example, a procedure with 12 bytes of local variables would have an `ENTER 12, 0` instruction at its entry point and a `LEAVE` instruction before every `RET`. The 12 local bytes would be addressed as negative offsets from `[BP]`.\n\nThe formal definition of the `ENTER` instruction for all cases is given by the following listing. `LEVEL` denotes the value of the second operand.\n\n```\nLEVEL := LEVEL MOD 32\nPush BP\nSet a temporary value FRAME_PTR := SP\nIf LEVEL > 0 then\n  Repeat (LEVEL - 1) times:\n    BP := BP - 2\n    Push the word pointed to by BP\n  End Repeat\n  Push FRAME_PTR\nEnd if\nBP := FRAME_PTR\nSP := SP - first operand\n```',
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
    {
      identifier: 'last_offset',
      name: 'Last Write Offset',
      size: 32,
    },
    {
      identifier: 'last_value',
      name: 'Last Value',
      size: 16,
    },
    {
      identifier: 'current_value',
      name: 'Current Value',
      size: 16,
    },
    {
      identifier: 'frame_ptr',
      name: 'Frame Pointer',
      size: 16,
    },
    {
      identifier: 'level',
      name: 'Level Counter',
      size: 8,
    },
  ],
  forms: [
    // 0xC8 dw 00 - ENTER dw, 0   (11 cycles)
    // 0xC8 dw 01 - ENTER dw, 1   (15 cycles)
    // 0xC8 dw db - ENTER dw, db  (12 + 4(db) cycles)
    {
      modes: {
        real: {
          operation: [
            'level = %{level} % 32',
            // Push BP at frame_ptr location
            'offset = (SP - 2):u16',
            '#GP if offset == 0xffff',
            // Emulate bus behavior when BP=SP
            'if BP == SP',
            [
              // Save the pre-push low byte for bus-pending emulation
              'last_value = RAM:u8[SS_BASE + offset]',
              'RAM:u16[SS_BASE + offset] = BP',
              // Initialize frame_ptr (SP after BP push)
              'frame_ptr = offset',
              // When the level is specified, copy display before writing BP
              // (the 286 reads the old frame before committing the BP push)
              'if level > 0',
              [
                // The BP push is the first pending write on the bus
                'last_offset = frame_ptr',
                // Copy display entries from old frame using offset as write pointer
                'loop if level > 1',
                [
                  'BP = BP - 2',
                  'offset = (offset - 2):u16',
                  '#GP if BP == 0xffff',
                  '#GP if offset == 0xffff',
                  'current_value = RAM:u8[SS_BASE + offset]',
                  'RAM:u16[SS_BASE + offset] = RAM:u16[SS_BASE + BP]',
                  'if BP == last_offset',
                  [
                    ';; emulate the memory bus not yet committing the prior write',
                    'RAM:u16[SS_BASE + offset] = last_value | (RAM:u16[SS_BASE + BP] & 0xff00)',
                  ],
                  'end if',
                  'last_value = current_value',
                  'last_offset = offset',
                  'level = level - 1',
                ],
                'repeat',
                // Push the frame pointer
                'offset = (offset - 2):u16',
                '#GP if offset == 0xffff',
                'RAM:u16[SS_BASE + offset] = frame_ptr',
              ],
              'end if',
            ],
            'else',
            [
              'RAM:u16[SS_BASE + offset] = BP',
              // Initialize frame_ptr (SP after BP push)
              'frame_ptr = offset',
              // When the level is specified, copy display before writing BP
              // (the 286 reads the old frame before committing the BP push)
              'if level > 0',
              [
                // The BP push is the first pending write on the bus
                'last_offset = frame_ptr',
                // Copy display entries from old frame using offset as write pointer
                'loop if level > 1',
                [
                  'BP = BP - 2',
                  'offset = (offset - 2):u16',
                  '#GP if BP == 0xffff',
                  '#GP if offset == 0xffff',
                  'RAM:u16[SS_BASE + offset] = RAM:u16[SS_BASE + BP]',
                  'level = level - 1',
                ],
                'repeat',
                // Push the frame pointer
                'offset = (offset - 2):u16',
                '#GP if offset == 0xffff',
                'RAM:u16[SS_BASE + offset] = frame_ptr',
              ],
              'end if',
            ],
            'end if',
            // Maintain frame_ptr into BP, set SP to end of frame
            'BP = frame_ptr',
            'SP = (offset - %{imm}):u16',
          ],
        },
        protected: {
          operation: [
            'level = %{level} % 32',
            // Push BP
            'SP = SP - 2',
            'offset = SP',
            'stack_address = SS_BASE + offset',
            '#GP if (offset + 1) < SS_LIMIT_MIN',
            '#GP if (offset + 1) > SS_LIMIT_MAX',
            'RAM:u16[stack_address] = BP',
            // Initialize frame_ptr
            'frame_ptr = SP',
            // When the level is specified
            'if level > 0',
            [
              // We wind the stack for larger levels
              'loop if level > 1',
              [
                'BP = BP - 2',
                'SP = SP - 2',
                'offset = BP',
                '#GP if (offset + 1) < SS_LIMIT_MIN',
                '#GP if (offset + 1) > SS_LIMIT_MAX',
                'offset = SP',
                '#GP if (offset + 1) < SS_LIMIT_MIN',
                '#GP if (offset + 1) > SS_LIMIT_MAX',
                'RAM:u16[SS_BASE + SP] = RAM:u16[SS_BASE + BP]',
                'level = level - 1',
              ],
              'repeat',
              // We always push the frame pointer
              'SP = SP - 2',
              'offset = SP',
              '#GP if (offset + 1) < SS_LIMIT_MIN',
              '#GP if (offset + 1) > SS_LIMIT_MAX',
              'RAM:u16[SS_BASE + SP] = frame_ptr',
            ],
            'end if',
            // Maintain frame_ptr into BP
            'BP = frame_ptr',
            // Allocate requested space on the stack
            'SP = SP - %{imm}',
          ],
        },
      },
      opcode: [
        Opcodes.ENTER,
        'IMM_u16',
        {
          identifier: 'level',
          name: 'LEVEL Immediate Value',
          type: InstructionDataTypes.Immediate,
          size: 8,
        },
      ],
      operands: ['imm', 'level'],
      operandSize: 16,
      cycles: 12,
    },
  ],
};
