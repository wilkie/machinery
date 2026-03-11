import type { MemoryInfo } from '@machinery/core';

const BASE_2 = 2;

const parityArray: number[] = Array.from({ length: 256 }, (_, i) => {
  const binary = i.toString(BASE_2);
  const onesCount = binary.split('1').length - 1;
  return onesCount % BASE_2 === 0 ? 1 : 0;
});

export const memory: MemoryInfo[] = [
  {
    identifier: 'ROM',
    name: 'Read-only Memory',
    type: 'rom',
    endian: 'little',
    regions: [
      {
        identifier: 'PARITY',
        name: 'Parity (PF) Map',
        data: parityArray,
      },
    ],
  },
  {
    identifier: 'IO',
    name: 'I/O Port Space',
    type: 'programmable',
    length: 0x100,
    endian: 'little',
  },
  {
    identifier: 'RAM',
    name: 'Random-access Memory',
    type: 'ram',
    min: 16384,
    max: 65536,
    endian: 'little',
  },
];
