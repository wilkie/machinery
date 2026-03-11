import type { Target } from '@machinery/core';

import { memory } from './memory';
import { registers } from './registers';
import { state } from './state';
import { instructions } from './instructions';
import { operands } from './operands';
import { Opcodes } from './opcodes';
import { macros } from './macros';

export { Opcodes };

const TargetZ80: Target = {
  identifier: 'z80',
  class: 'z80',
  name: 'Zilog Z80',
  description: '8-bit microprocessor',
  endianness: 'little',
  alignmentFill: 0x00,
  memory,
  registers,
  fetch: {
    register: 'PC',
    memory: 'RAM',
    advancePointer: true,
  },
  modes: [
    {
      identifier: 'default',
      name: 'Default',
      decode: {
        locals: [
          {
            identifier: 'ip',
            name: 'Instruction Pointer',
            size: 16,
          },
        ],
        initialization: {
          operation: ['ip = PC'],
        },
        read: {
          byte: {
            locals: [
              {
                identifier: 'b',
                name: 'Decoded Byte',
                size: 8,
              },
            ],
            operation: ['b = RAM:u8[ip]', 'ip = ip + 1'],
          },
          word: {
            locals: [
              {
                identifier: 'w',
                name: 'Decoded Word',
                size: 16,
              },
            ],
            operation: ['w = RAM:u16[ip]', 'ip = ip + 2'],
          },
        },
      },
    },
    {
      identifier: 'halted',
      name: 'CPU Halted',
    },
  ],
  state,
  instructions,
  operands,
  macros,
};

export default TargetZ80;
