import type { InstructionInfo } from '@rawrs/architecture/Target';

import { Opcodes } from '../opcodes';

// TODO: segment moves
// TODO: protected mode (cannot mov to CS... on real mode, it seems possible even if undocumented)
// TODO: #13 for operand 0xffff
// TODO: #UD for FS, GS perhaps, if emulating a real 286
// TODO: #UD for abnormal segment selectors (reg/rm: 110 and 111)
export const stos: InstructionInfo = {
  identifier: 'stos',
  name: 'Store String Data',
  description:
    'STOS transfers the contents the `AL` or `AX` register to the memory byte or word at `ES:DI`. The operand must be addressable from the `ES` register, no segment override is possible.\n\nAfter the transfer is made, `DI` is automatically advanced. If the direction flag is 0 (`CLD` was executed), `DI` increments; if the direction flag is 1 (`STD` was executed), `DI` decrements. `DI` increments or decrements by 1 if a byte was moved; by 2 if a word was moved.\n\n`STOS` can be preceded by the `REP` prefix for a block fill of `CX` bytes or words. Refer to the `REP` instruction for details of this operation.',
  modifies: [],
  undefined: [],
  forms: [
    // 0xAA STOS mb
    // 0xAA STOSB
    {
      operation: [
        'loop',
        [
          'next if REP != 0 && CX == 0',
          'RAM:u8[ES_BASE + DI] = AL',
          'DI = DI + (DF == 1 ? -1 : 1)',
          'CX = REP != 0 ? CX - 1 : CX',
        ],
        'repeat if REP != 0',
      ],
      opcode: [Opcodes.STOS_MB],
      operandSize: 8,
      cycles: 3,
    },
    // 0xAB STOS mw
    // 0xAB STOSW
    {
      operation: [
        'loop',
        [
          'next if REP != 0 && CX == 0',
          'RAM:u16[ES_BASE + DI] = AX',
          'DI = DI + (DF == 1 ? -2 : 2)',
          'CX = REP != 0 ? CX - 1 : CX',
        ],
        'repeat if REP != 0',
      ],
      opcode: [Opcodes.STOS_MW],
      operandSize: 16,
      cycles: 3,
    },
  ],
};
