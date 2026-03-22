import type { Target } from '@machinery/core';

import { memory } from './memory';
import { registers } from './registers';
import { state } from './state';
import { instructions } from './instructions';
import { operands } from './operands';
import { macros } from './macros';
import { Opcodes } from './opcodes';
import { interrupts } from './interrupts';

export { Opcodes };

export * as MZExecutable from './MZExecutable';

const Target286: Target = {
  identifier: 'i80286',
  class: 'x86',
  name: 'Intel 80286',
  description: '16-bit x86',
  endianness: 'little',
  alignmentFill: 0x90,
  memory,
  registers,
  fetch: {
    // We keep track of the instruction pointer in IP
    register: ['CS', 'IP'],

    // The internal effective address of the instruction pointer
    effectiveRegister: 'FETCH_IP',

    // We always advance the IP the size of the instruction during fetch/decode
    advancePointer: true,

    // And the instructions are fetched from the normal RAM
    memory: 'RAM',

    // 286 enforces a 10-byte maximum instruction length (including prefixes)
    maxInstructionLength: 10,
  },
  modes: [
    {
      identifier: 'real',
      name: 'Real Mode',
      decode: {
        locals: [
          {
            identifier: 'ip',
            name: 'Effective Instruction Pointer',
            size: 32,
          },
        ],
        initialization: {
          operation: ['ip = FETCH_IP'],
        },
        unknown: {
          operation: ['#UD if 1 == 1'],
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
          double: {
            locals: [
              {
                identifier: 'd',
                name: 'Decoded Double-Word',
                size: 32,
              },
            ],
            operation: ['d = RAM:u32[ip]', 'ip = ip + 4'],
          },
        },
      },
    },
    {
      identifier: 'protected',
      name: 'Protected Mode',
      decode: {
        locals: [
          {
            identifier: 'ip',
            name: 'Effective Instruction Pointer',
            size: 32,
          },
        ],
        initialization: {
          operation: ['ip = FETCH_IP'],
        },
        unknown: {
          operation: ['#UD if 1 == 1'],
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
          double: {
            locals: [
              {
                identifier: 'd',
                name: 'Decoded Double-Word',
                size: 32,
              },
            ],
            operation: ['d = RAM:u32[ip]', 'ip = ip + 4'],
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
  interrupts,
};

export default Target286;
