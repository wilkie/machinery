import type { Executable, FileFormat } from '@machinery/core';

export const MZ_MAGIC_M = 0x4d;
export const MZ_MAGIC_Z = 0x5a;

/**
 * Defines the MZ executable file format.
 */
export const format: FileFormat = {
  name: 'mz-executable',
  identify: [{ at: 0, match: [MZ_MAGIC_M, MZ_MAGIC_Z] /* 'MZ' */ }],
  endianness: 'little',
  types: {
    MZHeader: {
      fields: [
        { name: 'e_magic', type: 'u16' },
        { name: 'e_cblp', type: 'u16' },
        { name: 'e_cp', type: 'u16' },
        { name: 'e_crlc', type: 'u16' },
        { name: 'e_cparhdr', type: 'u16' },
        { name: 'e_minalloc', type: 'u16' },
        { name: 'e_maxalloc', type: 'u16' },
        { name: 'e_ss', type: 'u16' },
        { name: 'e_sp', type: 'u16' },
        { name: 'e_csum', type: 'u16' },
        { name: 'e_ip', type: 'u16' },
        { name: 'e_cs', type: 'u16' },
        { name: 'e_lfarlc', type: 'u16' },
        { name: 'e_ovno', type: 'u16' },
        { name: 'e_res', type: 'u16', count: 4 },
        { name: 'e_oemid', type: 'u16' },
        { name: 'e_oeminfo', type: 'u16' },
        { name: 'e_res2', type: 'u16', count: 10 },
        { name: 'e_lfanew', type: 'u32' },
      ],
    },
    Relocation: {
      fields: [
        { name: 'offset', type: 'u16' },
        { name: 'segment', type: 'u16' },
      ],
    },
  },
  data: {
    header: {
      type: 'MZHeader',
      offset: 0,
      properties: {
        relocations: {
          at: 'header.e_lfarlc',
          type: 'Relocation',
          countExpression: 'header.e_crlc',
        },
        imageStart: 'header.e_cparhdr * 16',
        imageSize: 'file.size - header.imageStart',
      },
    },
  },
};

/**
 * Defines a loader that can load an MZ executable into a generic virtual machine.
 */
export const loader: Executable = {
  format: 'mz-executable',
  load: {
    x86: {
      operation: [
        ';; allocate a segment for the PSP',
        'load_segment = 0x1',
        ';; prepare PSP',
        'address = load_segment << 4',
        ';; place program at the next segment after the PSP (256 bytes)',
        'load_segment = load_segment + 16',
        ';; INT 20h',
        'RAM:u16[address] = 0x20cc',
        ';; heap segment',
        'RAM:u16[address + 0x2] = ((load_segment << 4) + header.imageSize + 2) >> 4',
        ';; environment segment',
        'RAM:u16[address + 0x2c] = 0',
        ';; command line length',
        'RAM:u8[address + 0x80] = 0',
        ';; copy data',
        'address = address + 0x100',
        'RAM:u8[address..(address + header.imageSize)] = file[header.imageStart..file.size]',
        ';; perform relocations',
        'i = 0',
        'loop if i < header.relocations.length',
        [
          'address = header.relocations[i].segment << 4 + header.relocations[i].offset',
          'RAM:u16[address] = RAM:u16[address] + load_segment',
          'i = i + 1',
        ],
        'repeat',
        ';; set register values',
        'DS = load_segment - 16',
        'CS = header.e_cs + load_segment',
        'IP = header.e_ip',
        'SS = header.e_ss + load_segment',
        'SP = header.e_sp',
      ],
    },
  },
};
