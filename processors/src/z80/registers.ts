import { RegisterTypes } from '@machinery/core';
import type { RegisterInfo } from '@machinery/core';

export const registers: RegisterInfo[] = [
  // Main register pairs (order matches encoding: B=0, C=1, D=2, E=3, H=4, L=5, (HL)=6, A=7)
  // But the register pairs are: BC=0, DE=1, HL=2, SP=3 (for qq) or AF=3 (for pp)
  {
    identifier: 'AF',
    name: 'Accumulator and Flags',
    size: 16,
    type: RegisterTypes.Integer,
    fields: [
      {
        global: true,
        identifier: 'F',
        name: 'Flags',
        offset: 0,
        size: 8,
      },
      {
        global: true,
        identifier: 'A',
        name: 'Accumulator',
        offset: 8,
        size: 8,
      },
    ],
  },
  {
    identifier: 'BC',
    name: 'Register Pair BC',
    size: 16,
    type: RegisterTypes.Integer,
    fields: [
      {
        global: true,
        identifier: 'C',
        name: 'Register C',
        offset: 0,
        size: 8,
      },
      {
        global: true,
        identifier: 'B',
        name: 'Register B',
        offset: 8,
        size: 8,
      },
    ],
  },
  {
    identifier: 'DE',
    name: 'Register Pair DE',
    size: 16,
    type: RegisterTypes.Integer,
    fields: [
      {
        global: true,
        identifier: 'E',
        name: 'Register E',
        offset: 0,
        size: 8,
      },
      {
        global: true,
        identifier: 'D',
        name: 'Register D',
        offset: 8,
        size: 8,
      },
    ],
  },
  {
    identifier: 'HL',
    name: 'Register Pair HL',
    size: 16,
    type: RegisterTypes.Integer,
    fields: [
      {
        global: true,
        identifier: 'L',
        name: 'Register L',
        offset: 0,
        size: 8,
      },
      {
        global: true,
        identifier: 'H',
        name: 'Register H',
        offset: 8,
        size: 8,
      },
    ],
  },
  // Stack pointer
  {
    identifier: 'SP',
    name: 'Stack Pointer',
    size: 16,
    type: RegisterTypes.Integer,
  },
  // Program counter
  {
    identifier: 'PC',
    name: 'Program Counter',
    size: 16,
    type: RegisterTypes.Integer,
  },
  // Index registers
  {
    identifier: 'IX',
    name: 'Index Register X',
    size: 16,
    type: RegisterTypes.Integer,
    fields: [
      {
        global: true,
        identifier: 'IXL',
        name: 'IX Low Byte',
        offset: 0,
        size: 8,
      },
      {
        global: true,
        identifier: 'IXH',
        name: 'IX High Byte',
        offset: 8,
        size: 8,
      },
    ],
  },
  {
    identifier: 'IY',
    name: 'Index Register Y',
    size: 16,
    type: RegisterTypes.Integer,
    fields: [
      {
        global: true,
        identifier: 'IYL',
        name: 'IY Low Byte',
        offset: 0,
        size: 8,
      },
      {
        global: true,
        identifier: 'IYH',
        name: 'IY High Byte',
        offset: 8,
        size: 8,
      },
    ],
  },
  // Interrupt and refresh registers
  {
    identifier: 'I',
    name: 'Interrupt Vector',
    size: 8,
    type: RegisterTypes.Integer,
  },
  {
    identifier: 'R',
    name: 'Memory Refresh',
    size: 8,
    type: RegisterTypes.Integer,
  },
  // Alternate register set (shadow registers)
  {
    identifier: "AF'",
    name: 'Alternate Accumulator and Flags',
    size: 16,
    type: RegisterTypes.Integer,
    fields: [
      {
        identifier: "F'",
        name: 'Alternate Flags',
        offset: 0,
        size: 8,
      },
      {
        identifier: "A'",
        name: 'Alternate Accumulator',
        offset: 8,
        size: 8,
      },
    ],
  },
  {
    identifier: "BC'",
    name: 'Alternate Register Pair BC',
    size: 16,
    type: RegisterTypes.Integer,
    fields: [
      {
        identifier: "C'",
        name: 'Alternate Register C',
        offset: 0,
        size: 8,
      },
      {
        identifier: "B'",
        name: 'Alternate Register B',
        offset: 8,
        size: 8,
      },
    ],
  },
  {
    identifier: "DE'",
    name: 'Alternate Register Pair DE',
    size: 16,
    type: RegisterTypes.Integer,
    fields: [
      {
        identifier: "E'",
        name: 'Alternate Register E',
        offset: 0,
        size: 8,
      },
      {
        identifier: "D'",
        name: 'Alternate Register D',
        offset: 8,
        size: 8,
      },
    ],
  },
  {
    identifier: "HL'",
    name: 'Alternate Register Pair HL',
    size: 16,
    type: RegisterTypes.Integer,
    fields: [
      {
        identifier: "L'",
        name: 'Alternate Register L',
        offset: 0,
        size: 8,
      },
      {
        identifier: "H'",
        name: 'Alternate Register H',
        offset: 8,
        size: 8,
      },
    ],
  },
  // Flags register (expanded view — fields of F)
  // The F register fields are: S Z - H - P/V N C
  // Bit 7: S (Sign), Bit 6: Z (Zero), Bit 5: Y/F5 (undocumented),
  // Bit 4: H (Half-carry), Bit 3: X/F3 (undocumented),
  // Bit 2: P/V (Parity/Overflow), Bit 1: N (Subtract), Bit 0: C (Carry)
];
