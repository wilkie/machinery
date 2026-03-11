import type { Target } from '@machinery/core';

import { registers } from './registers';
import { instructions } from './instructions';
import { operands } from './operands';
import { Opcodes } from './opcodes';

export { Opcodes };

const TargetZ80: Target = {
  identifier: 'z80',
  class: 'z80',
  name: 'Zilog Z80',
  description: '8-bit microprocessor',
  endianness: 'little',
  alignmentFill: 0x00,
  registers,
  fetch: {
    register: 'PC',
    memory: 'RAM',
    advancePointer: true,
  },
  instructions,
  operands,
};

export default TargetZ80;
