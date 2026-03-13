import type { MemoryInfo } from '@machinery/core';

const BASE_2 = 2;

const parityArray: number[] = Array.from({ length: 256 }, (_, i) => {
  const binary = i.toString(BASE_2); // Convert the number to binary
  const onesCount = binary.split('1').length - 1; // Count the number of 1s
  return onesCount % BASE_2 === 0 ? 1 : 0; // Return 1 if the count is even, otherwise 0
});

export const memory: MemoryInfo[] = [
  {
    // System ROM
    identifier: 'ROM',
    name: 'Read-only Memory',
    type: 'rom',
    endian: 'little',
    regions: [
      {
        // The parity data map to cache PF checks
        identifier: 'PARITY',
        name: 'Parity (PF) Map',
        data: parityArray,
      },
    ],
  },
  // IO port map
  {
    identifier: 'IO',
    name: 'I/O Device Interface',
    type: 'programmable',
    length: 0x10000,
    endian: 'little',
  },
  // Rest is RAM
  {
    identifier: 'RAM',
    name: 'Random-access Memory',
    type: 'ram',
    min: 16384,
    max: 1073741824,
    endian: 'little',
    regions: [
      // Real-mode interrupt table (0000:0000)
      {
        identifier: 'IVT',
        name: 'Interrupt Vector Table',
        offset: 0,
        fields: [
          {
            identifier: 'vectors',
            name: 'Interrupt Vectors',
            count: 256,
            offset: 0,
            size: 32,
            cell: {
              identifier: 'IVD',
              name: 'Interrupt Vector Descriptor',
              fields: [
                {
                  identifier: 'segment',
                  name: 'Interrupt Handler Target CS',
                  offset: 0,
                  size: 16,
                },
                {
                  identifier: 'offset',
                  name: 'Interrupt Handler Target IP',
                  offset: 16,
                  size: 16,
                },
              ],
            },
          },
        ],
      },
      // Protected-mode Interrupt Descriptor Table (IDT)
      {
        identifier: 'IDT',
        name: 'Interrupt Descriptor Table',
        offset: 'IDTR.base',
        fields: [
          {
            identifier: 'gates',
            name: 'Interrupt Gates',
            count: 256,
            offset: 0,
            size: 64,
            cell: {
              identifier: 'IGD',
              name: 'Interrupt Gate Descriptor',
              fields: [
                {
                  identifier: 'offset',
                  name: 'Interrupt Code Offset',
                  offset: 0,
                  size: 16,
                },
                {
                  identifier: 'segment',
                  name: 'Interrupt Code Segment Selector',
                  offset: 16,
                  size: 16,
                },
                {
                  identifier: 'T',
                  name: 'Trap Bit',
                  offset: 40,
                  size: 1,
                },
                {
                  identifier: 'DPL',
                  name: 'Destination Protection Level',
                  offset: 45,
                  size: 2,
                },
                {
                  identifier: 'P',
                  name: 'Present Bit',
                  offset: 47,
                  size: 1,
                },
              ],
            },
          },
        ],
      },
      // Protected-mode Global Descriptor Table (GDT)
      {
        identifier: 'GDT',
        name: 'Global Descriptor Table',
        offset: 'GDTR.base',
        fields: [
          {
            identifier: 'gates',
            name: 'Gates',
            count: 256,
            offset: 0,
            size: 64,
            cell: [
              {
                identifier: 'SD',
                name: 'Segment Descriptor',
                fields: [
                  {
                    identifier: 'limit',
                    name: 'Segment Limit',
                    offset: 0,
                    size: 16,
                  },
                  {
                    identifier: 'base',
                    name: 'Segment Base',
                    offset: 16,
                    size: 24,
                  },
                  {
                    identifier: 'A',
                    name: 'Accessed',
                    offset: 40,
                    size: 1,
                  },
                  {
                    identifier: 'type',
                    name: 'Gate type',
                    offset: 41,
                    size: 3,
                  },
                  {
                    // Marks this field as a segment descriptor and not a call gate */
                    identifier: 'S',
                    name: 'Segment Descriptor',
                    offset: 44,
                    size: 1,
                    equals: 1,
                  },
                  {
                    identifier: 'DPL',
                    name: 'Destination Protection Level',
                    offset: 45,
                    size: 2,
                  },
                  {
                    identifier: 'P',
                    name: 'Present Bit',
                    offset: 47,
                    size: 1,
                  },
                ],
              },
            ],
          },
        ],
      },
    ],
  },
];
