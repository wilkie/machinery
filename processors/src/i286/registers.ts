import { RegisterTypes } from '@machinery/core';
import type { RegisterInfo } from '@machinery/core';

export const registers: RegisterInfo[] = [
  // General registers (in an optimized order matching instruction encoding)
  {
    identifier: 'AX',
    name: 'General A',
    size: 16,
    type: RegisterTypes.Integer,
    fields: [
      {
        global: true,
        identifier: 'AL',
        name: 'AX Low Byte',
        offset: 0,
        size: 8,
      },
      {
        global: true,
        identifier: 'AH',
        name: 'AX High Byte',
        offset: 8,
        size: 8,
      },
    ],
  },
  {
    identifier: 'CX',
    name: 'General C',
    size: 16,
    type: RegisterTypes.Integer,
    fields: [
      {
        global: true,
        identifier: 'CL',
        name: 'CX Low Byte',
        offset: 0,
        size: 8,
      },
      {
        global: true,
        identifier: 'CH',
        name: 'CX High Byte',
        offset: 8,
        size: 8,
      },
    ],
  },
  {
    identifier: 'DX',
    name: 'General D',
    size: 16,
    type: RegisterTypes.Integer,
    fields: [
      {
        global: true,
        identifier: 'DL',
        name: 'DX Low Byte',
        offset: 0,
        size: 8,
      },
      {
        global: true,
        identifier: 'DH',
        name: 'DX High Byte',
        offset: 8,
        size: 8,
      },
    ],
  },
  {
    identifier: 'BX',
    name: 'General B',
    size: 16,
    type: RegisterTypes.Integer,
    fields: [
      {
        global: true,
        identifier: 'BL',
        name: 'BX Low Byte',
        offset: 0,
        size: 8,
      },
      {
        global: true,
        identifier: 'BH',
        name: 'BX High Byte',
        offset: 8,
        size: 8,
      },
    ],
  },
  {
    identifier: 'SP',
    name: 'Stack Pointer',
    size: 16,
    type: RegisterTypes.Integer,
  },
  {
    identifier: 'BP',
    name: 'Base Pointer',
    size: 16,
    type: RegisterTypes.Integer,
  },
  {
    identifier: 'SI',
    name: 'Source Index',
    size: 16,
    type: RegisterTypes.Integer,
  },
  {
    identifier: 'DI',
    name: 'Data Index',
    size: 16,
    type: RegisterTypes.Integer,
  },
  {
    identifier: 'IP',
    name: 'Instruction Pointer',
    size: 16,
    type: RegisterTypes.Integer,
    get: {
      operation: ['IP = FETCH_IP - CS_BASE'],
    },
    set: {
      operation: ['FETCH_IP = CS_BASE + IP'],
    },
  },
  {
    identifier: 'FLAGS',
    name: 'CPU Flags',
    size: 32,
    type: RegisterTypes.Integer,
    fields: [
      {
        global: true,
        identifier: 'CF',
        name: 'Carry Flag',
        offset: 0,
        size: 1,
      },
      {
        global: true,
        identifier: 'PF',
        name: 'Parity Flag',
        offset: 2,
        size: 1,
      },
      {
        global: true,
        identifier: 'AF',
        name: 'Auxiliary Carry Flag',
        offset: 4,
        size: 1,
      },
      {
        global: true,
        identifier: 'ZF',
        name: 'Zero Flag',
        offset: 6,
        size: 1,
      },
      {
        global: true,
        identifier: 'SF',
        name: 'Sign Flag',
        offset: 7,
        size: 1,
      },
      {
        global: true,
        identifier: 'TF',
        name: 'Trap Flag',
        offset: 8,
        size: 1,
      },
      {
        global: true,
        identifier: 'IF',
        name: 'Interrupt Enable Flag',
        offset: 9,
        size: 1,
      },
      {
        global: true,
        identifier: 'DF',
        name: 'Direction Flag',
        offset: 10,
        size: 1,
      },
      {
        global: true,
        identifier: 'OF',
        name: 'Overflow Flag',
        offset: 11,
        size: 1,
      },
      {
        global: true,
        identifier: 'IOPL',
        name: 'I/O Privilege Level',
        offset: 12,
        size: 2,
      },
      {
        global: true,
        identifier: 'NT',
        name: 'Nested Task Flag',
        offset: 14,
        size: 1,
      },
    ],
  },
  // Segment registers
  {
    identifier: 'ES',
    name: 'E Segment Selector',
    size: 16,
    type: RegisterTypes.Segment,
    fields: [
      {
        identifier: 'RPL',
        name: 'Requestor Protection Level',
        offset: 0,
        size: 2,
      },
    ],
    set: {
      modes: {
        real: {
          operation: ['ES_BASE = ES << 4'],
        },
      },
    },
  },
  {
    identifier: 'CS',
    name: 'Code Segment Selector',
    size: 16,
    type: RegisterTypes.Segment,
    fields: [
      {
        identifier: 'RPL',
        name: 'Requestor Protection Level',
        offset: 0,
        size: 2,
      },
    ],
    set: {
      modes: {
        real: {
          operation: [
            'IP = FETCH_IP - CS_BASE',
            'CS_BASE = CS << 4',
            'FETCH_IP = CS_BASE + IP',
          ],
        },
      },
    },
  },
  {
    identifier: 'SS',
    name: 'Stack Segment Selector',
    size: 16,
    type: RegisterTypes.Segment,
    fields: [
      {
        identifier: 'RPL',
        name: 'Requestor Protection Level',
        offset: 0,
        size: 2,
      },
    ],
    set: {
      modes: {
        real: {
          operation: [
            'SS_BASE = SS << 4',
            'SS_BASE1 = SS << 4',
            'SS_BASE2 = SS << 4',
          ],
        },
      },
    },
  },
  {
    identifier: 'DS',
    name: 'Data Segment Selector',
    size: 16,
    type: RegisterTypes.Segment,
    fields: [
      {
        identifier: 'RPL',
        name: 'Requestor Protection Level',
        offset: 0,
        size: 2,
      },
    ],
    set: {
      modes: {
        real: {
          operation: [
            'DS_BASE = DS << 4',
            'DS_BASE1 = DS << 4',
            'DS_BASE2 = DS << 4',
            'DS_BASE3 = DS << 4',
            'DS_BASE4 = DS << 4',
          ],
        },
      },
    },
  },
  // 386-extended segment selectors which are still 'available' in 286 real modes
  {
    identifier: 'FS',
    name: 'F Segment Selector',
    size: 16,
    type: RegisterTypes.Segment,
    fields: [
      {
        identifier: 'RPL',
        name: 'Requestor Protection Level',
        offset: 0,
        size: 2,
      },
    ],
    set: {
      modes: {
        real: {
          operation: ['FS_BASE = FS << 4'],
        },
      },
    },
  },
  {
    identifier: 'GS',
    name: 'G Segment Selector',
    size: 16,
    type: RegisterTypes.Segment,
    fields: [
      {
        identifier: 'RPL',
        name: 'Requestor Protection Level',
        offset: 0,
        size: 2,
      },
    ],
    set: {
      modes: {
        real: {
          operation: ['GS_BASE = GS << 4'],
        },
      },
    },
  },
  // These are padding to make segment selector register calculation more efficient
  {
    identifier: 'HS',
    name: 'Extra Register 1',
    size: 16,
    type: RegisterTypes.Integer,
  },
  {
    identifier: 'IS',
    name: 'Extra Register 2',
    size: 16,
    type: RegisterTypes.Integer,
  },
  // System registers
  // Machine Status Word (sometimes referred to as CR0)
  {
    identifier: 'MSW',
    name: 'Machine Status Words',
    size: 16,
    type: RegisterTypes.Integer,
    fields: [
      {
        identifier: 'PE',
        name: 'Protected-mode Enable',
        offset: 0,
        size: 1,
      },
      {
        identifier: 'MP',
        offset: 1,
        size: 1,
      },
      {
        identifier: 'EM',
        offset: 2,
        size: 1,
      },
      {
        identifier: 'TS',
        offset: 3,
        size: 1,
      },
    ],
  },
  {
    identifier: 'TR',
    name: 'Task Register',
    size: 16,
    type: RegisterTypes.Integer,
  },
  // Data structure addresses
  {
    identifier: 'IDTR',
    name: 'IDT Register',
    size: 40,
    type: RegisterTypes.Integer,
    fields: [
      {
        identifier: 'limit',
        name: 'IDT Limit Register',
        offset: 0,
        size: 16,
      },
      {
        identifier: 'base',
        name: 'IDT Base Offset Register',
        offset: 16,
        size: 24,
      },
    ],
  },
  {
    identifier: 'GDTR',
    name: 'GDT Register',
    size: 40,
    type: RegisterTypes.Integer,
    fields: [
      {
        identifier: 'limit',
        name: 'GDT Limit Register',
        offset: 0,
        size: 16,
      },
      {
        identifier: 'base',
        name: 'GDT Base Offset Register',
        offset: 16,
        size: 24,
      },
    ],
  },
  {
    identifier: 'LDTR',
    name: 'LDT Register',
    size: 40,
    type: RegisterTypes.Integer,
    fields: [
      {
        identifier: 'limit',
        name: 'LDT Limit Register',
        offset: 0,
        size: 16,
      },
      {
        identifier: 'base',
        name: 'LDT Base Offset Register',
        offset: 16,
        size: 24,
      },
    ],
  },
];
