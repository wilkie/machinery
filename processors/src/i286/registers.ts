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
          operation: [
            'ES_BASE = ES << 4',
            'ES_LIMIT_MIN = 0',
            'ES_LIMIT_MAX = 0xffff',
          ],
        },
        protected: {
          locals: [
            {
              identifier: 'tmp',
              name: 'Selector value',
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
              identifier: 'desc_a',
              name: 'Descriptor A bit',
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
              identifier: 'desc_valid',
              name: 'Descriptor valid flag',
              size: 8,
            },
            {
              identifier: 'inverted',
              name: 'Expand-down flag',
              size: 1,
            },
          ],
          operation: [
            'tmp = ES',
            'index = tmp >> 3',
            ';; null selector: allow for ES, mark segment invalid so any access faults',
            'if index == 0',
            [
              'ES_BASE = 0',
              'ES_LIMIT_MIN = 0xffff',
              'ES_LIMIT_MAX = 0',
              'ES_ACCESS = 0',
            ],
            'end if',
            'if index != 0',
            [
              'ERROR_CODE = ES',
              ';; descriptor table bounds check',
              '${DESCRIPTOR_BOUNDS_CHECK}',
              ';; load descriptor fields from GDT or LDT',
              '${LOAD_DESCRIPTOR_FIELDS}',
              ';; must be a segment descriptor (S=1)',
              '#GP if desc_s != 1',
              ';; must be data or readable code (not execute-only)',
              '#GP if (desc_type & 0b100) == 0b100 && (desc_type & 0b001) == 0',
              ';; DPL check: if data or non-conforming code, CPL <= DPL and RPL <= DPL',
              'if (desc_type & 0b110) != 0b110',
              ['#GP if CS.RPL > desc_dpl', '#GP if ES.RPL > desc_dpl'],
              'end if',
              ';; present check',
              '#NP if desc_p != 1',
              ';; load segment cache',
              'inverted = ((desc_type & 0b110) == 0b010) ? 1 : 0',
              'ES_BASE = desc_base',
              'ES_LIMIT_MIN = inverted == 1 ? desc_limit + 1 : 0',
              'ES_LIMIT_MAX = inverted == 1 ? 0xffff : desc_limit',
              'ES_ACCESS = desc_type',
              ';; set accessed bit',
              '${SET_DESCRIPTOR_ACCESSED}',
            ],
            'end if',
          ],
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
            'CS_LIMIT_MIN = 0',
            'CS_LIMIT_MAX = 0xffff',
          ],
        },
        protected: {
          locals: [
            {
              identifier: 'tmp',
              name: 'Selector value',
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
          ],
          operation: [
            'tmp = CS',
            'index = tmp >> 3',
            ';; null selector not allowed for CS',
            '#GP if index == 0',
            'ERROR_CODE = CS',
            ';; descriptor table bounds check',
            '${DESCRIPTOR_BOUNDS_CHECK}',
            ';; load descriptor fields from GDT or LDT',
            '${LOAD_DESCRIPTOR_FIELDS}',
            ';; must be a segment descriptor (S=1)',
            '#GP if desc_s != 1',
            ';; must be a code segment (executable)',
            '#GP if (desc_type & 0b100) != 0b100',
            ';; DPL check: for non-conforming code, DPL must equal CPL',
            'if (desc_type & 0b010) != 0b010',
            ['#GP if CS.RPL != desc_dpl'],
            'end if',
            ';; present check',
            '#NP if desc_p != 1',
            ';; load segment cache',
            'IP = FETCH_IP - CS_BASE',
            'CS_BASE = desc_base',
            'FETCH_IP = CS_BASE + IP',
            'CS_LIMIT_MIN = 0',
            'CS_LIMIT_MAX = desc_limit',
            'CS_ACCESS = desc_type',
            ';; set accessed bit',
            '${SET_DESCRIPTOR_ACCESSED}',
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
            'SS_LIMIT_MIN = 0',
            'SS_LIMIT_MAX = 0xffff',
            'SS_BASE1 = SS_BASE',
            'SS_BASE2 = SS_BASE',
            'SS_LIMIT_MIN1 = SS_LIMIT_MIN',
            'SS_LIMIT_MIN2 = SS_LIMIT_MIN',
            'SS_LIMIT_MAX1 = SS_LIMIT_MAX',
            'SS_LIMIT_MAX2 = SS_LIMIT_MAX',
          ],
        },
        protected: {
          locals: [
            {
              identifier: 'tmp',
              name: 'Selector value',
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
              identifier: 'inverted',
              name: 'Expand-down flag',
              size: 1,
            },
          ],
          operation: [
            'tmp = SS',
            'index = tmp >> 3',
            ';; null selector not allowed for SS',
            '#GP if index == 0',
            'ERROR_CODE = SS',
            ';; descriptor table bounds check',
            '${DESCRIPTOR_BOUNDS_CHECK}',
            ';; load descriptor fields from GDT or LDT',
            '${LOAD_DESCRIPTOR_FIELDS}',
            ';; must be a segment descriptor (S=1)',
            '#GP if desc_s != 1',
            ';; must be a writable data segment (not code, writable bit set)',
            '#GP if (desc_type & 0b100) == 0b100',
            '#GP if (desc_type & 0b001) != 0b001',
            ';; DPL must equal CPL, RPL must equal CPL',
            '#GP if desc_dpl != CS.RPL',
            '#GP if SS.RPL != CS.RPL',
            ';; present check',
            '#SS if desc_p != 1',
            ';; load segment cache',
            'inverted = ((desc_type & 0b010) == 0b010) ? 1 : 0',
            'SS_BASE = desc_base',
            'SS_LIMIT_MIN = inverted == 1 ? desc_limit + 1 : 0',
            'SS_LIMIT_MAX = inverted == 1 ? 0xffff : desc_limit',
            'SS_ACCESS = desc_type',
            'SS_BASE1 = SS_BASE',
            'SS_BASE2 = SS_BASE',
            'SS_LIMIT_MIN1 = SS_LIMIT_MIN',
            'SS_LIMIT_MIN2 = SS_LIMIT_MIN',
            'SS_LIMIT_MAX1 = SS_LIMIT_MAX',
            'SS_LIMIT_MAX2 = SS_LIMIT_MAX',
            'SS_ACCESS1 = SS_ACCESS',
            'SS_ACCESS2 = SS_ACCESS',
            ';; set accessed bit',
            '${SET_DESCRIPTOR_ACCESSED}',
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
            'DS_LIMIT_MIN = 0',
            'DS_LIMIT_MAX = 0xffff',
            'DS_BASE1 = DS_BASE',
            'DS_BASE2 = DS_BASE',
            'DS_BASE3 = DS_BASE',
            'DS_BASE4 = DS_BASE',
            'DS_LIMIT_MIN1 = DS_LIMIT_MIN',
            'DS_LIMIT_MIN2 = DS_LIMIT_MIN',
            'DS_LIMIT_MIN3 = DS_LIMIT_MIN',
            'DS_LIMIT_MIN4 = DS_LIMIT_MIN',
            'DS_LIMIT_MAX1 = DS_LIMIT_MAX',
            'DS_LIMIT_MAX2 = DS_LIMIT_MAX',
            'DS_LIMIT_MAX3 = DS_LIMIT_MAX',
            'DS_LIMIT_MAX4 = DS_LIMIT_MAX',
          ],
        },
        protected: {
          locals: [
            {
              identifier: 'tmp',
              name: 'Selector value',
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
              identifier: 'inverted',
              name: 'Expand-down flag',
              size: 1,
            },
          ],
          operation: [
            'tmp = DS',
            'index = tmp >> 3',
            ';; null selector: allow for DS, mark segment invalid so any access faults',
            'if index == 0',
            [
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
            'if index != 0',
            [
              'ERROR_CODE = DS',
              ';; descriptor table bounds check',
              '${DESCRIPTOR_BOUNDS_CHECK}',
              ';; load descriptor fields from GDT or LDT',
              '${LOAD_DESCRIPTOR_FIELDS}',
              ';; must be a segment descriptor (S=1)',
              '#GP if desc_s != 1',
              ';; must be data or readable code (not execute-only)',
              '#GP if (desc_type & 0b100) == 0b100 && (desc_type & 0b001) == 0',
              ';; DPL check: if data or non-conforming code, CPL <= DPL and RPL <= DPL',
              'if (desc_type & 0b110) != 0b110',
              ['#GP if CS.RPL > desc_dpl', '#GP if DS.RPL > desc_dpl'],
              'end if',
              ';; present check',
              '#NP if desc_p != 1',
              ';; load segment cache',
              'inverted = ((desc_type & 0b110) == 0b010) ? 1 : 0',
              'DS_BASE = desc_base',
              'DS_LIMIT_MIN = inverted == 1 ? desc_limit + 1 : 0',
              'DS_LIMIT_MAX = inverted == 1 ? 0xffff : desc_limit',
              'DS_ACCESS = desc_type',
              'DS_BASE1 = DS_BASE',
              'DS_BASE2 = DS_BASE',
              'DS_BASE3 = DS_BASE',
              'DS_BASE4 = DS_BASE',
              'DS_LIMIT_MIN1 = DS_LIMIT_MIN',
              'DS_LIMIT_MIN2 = DS_LIMIT_MIN',
              'DS_LIMIT_MIN3 = DS_LIMIT_MIN',
              'DS_LIMIT_MIN4 = DS_LIMIT_MIN',
              'DS_LIMIT_MAX1 = DS_LIMIT_MAX',
              'DS_LIMIT_MAX2 = DS_LIMIT_MAX',
              'DS_LIMIT_MAX3 = DS_LIMIT_MAX',
              'DS_LIMIT_MAX4 = DS_LIMIT_MAX',
              'DS_ACCESS1 = DS_ACCESS',
              'DS_ACCESS2 = DS_ACCESS',
              'DS_ACCESS3 = DS_ACCESS',
              'DS_ACCESS4 = DS_ACCESS',
              ';; set accessed bit',
              '${SET_DESCRIPTOR_ACCESSED}',
            ],
            'end if',
          ],
        },
      },
    },
  },
  // 386-extended segment selectors which are still 'available' in 286 real modes
  // We mark their use as to trigger the Invalid Opcode exception
  {
    identifier: 'FS',
    name: 'F Segment Selector',
    size: 16,
    type: RegisterTypes.Segment,
    get: {
      modes: {
        real: {
          operation: ['#UD'],
        },
        protected: {
          operation: ['#UD'],
        },
      },
    },
    set: {
      modes: {
        real: {
          operation: ['#UD'],
        },
        protected: {
          operation: ['#UD'],
        },
      },
    },
  },
  {
    identifier: 'GS',
    name: 'G Segment Selector',
    size: 16,
    type: RegisterTypes.Segment,
    get: {
      modes: {
        real: {
          operation: ['#UD'],
        },
        protected: {
          operation: ['#UD'],
        },
      },
    },
    set: {
      modes: {
        real: {
          operation: ['#UD'],
        },
        protected: {
          operation: ['#UD'],
        },
      },
    },
  },
  // These are padding to make segment selector register calculation more efficient
  // We obvious mark their use as to trigger the Invalid Opcode (#UD) exception
  {
    identifier: 'HS',
    name: 'Extra Register 1',
    size: 16,
    type: RegisterTypes.Integer,
    get: {
      modes: {
        real: {
          operation: ['#UD'],
        },
        protected: {
          operation: ['#UD'],
        },
      },
    },
    set: {
      modes: {
        real: {
          operation: ['#UD'],
        },
        protected: {
          operation: ['#UD'],
        },
      },
    },
  },
  {
    identifier: 'IS',
    name: 'Extra Register 2',
    size: 16,
    type: RegisterTypes.Integer,
    get: {
      modes: {
        real: {
          operation: ['#UD'],
        },
        protected: {
          operation: ['#UD'],
        },
      },
    },
    set: {
      modes: {
        real: {
          operation: ['#UD'],
        },
        protected: {
          operation: ['#UD'],
        },
      },
    },
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
    size: 64,
    type: RegisterTypes.Integer,
    fields: [
      {
        identifier: 'limit',
        name: 'TSS Limit',
        offset: 0,
        size: 16,
      },
      {
        identifier: 'selector',
        name: 'TSS Selector',
        offset: 16,
        size: 16,
      },
      {
        identifier: 'base',
        name: 'TSS Base Offset',
        offset: 32,
        size: 24,
      },
    ],
  },
  // Data structure addresses
  {
    identifier: 'IDTR',
    name: 'IDT Register',
    size: 64,
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
        offset: 32,
        size: 24,
      },
    ],
  },
  {
    identifier: 'GDTR',
    name: 'GDT Register',
    size: 64,
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
        offset: 32,
        size: 24,
      },
    ],
  },
  {
    identifier: 'LDTR',
    name: 'LDT Register',
    size: 64,
    type: RegisterTypes.Integer,
    fields: [
      {
        identifier: 'limit',
        name: 'LDT Limit Register',
        offset: 0,
        size: 16,
      },
      {
        identifier: 'selector',
        name: 'LDT Selector',
        offset: 16,
        size: 16,
      },
      {
        identifier: 'base',
        name: 'LDT Base Offset Register',
        offset: 32,
        size: 24,
      },
    ],
  },
];
