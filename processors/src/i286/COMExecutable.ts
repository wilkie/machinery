import type { Executable, FileFormat } from '@machinery/core';

/**
 * Defines the DOS COM executable file format.
 */
export const format: FileFormat = {
  name: 'com-executable',
  extension: 'COM',
  endianness: 'little',
};

/**
 * Defines a loader that can load an MZ executable into a generic virtual machine.
 */
export const loader: Executable = {
  format: 'com-executable',
  load: {
    x86: {
      operation: [
        ';; allocate the PSP to some segment',
        'load_segment = 0xf',
        ';; prepare PSP',
        'address = load_segment << 4',
        ';; place program at the next segment after 256 bytes',
        'load_segment = load_segment + 16',
        ';; INT 20h',
        'RAM:u16[address] = 0x20cc',
        ';; heap segment',
        'RAM:u16[address + 0x2] = ((load_segment << 4) + file.size + 2) >> 4',
        ';; environment segment',
        'RAM:u16[address + 0x2c] = 0',
        ';; command line length',
        'RAM:u8[address + 0x80] = 0',
        ';; copy data',
        'address = address + 0x100',
        'RAM:u8[address..(address + file.size)] = file[0..file.size]',
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
        'CS = load_segment',
        'IP = 0',
        'SS = load_segment',
        'SP = 0xfffe',
      ],
    },
  },
};
