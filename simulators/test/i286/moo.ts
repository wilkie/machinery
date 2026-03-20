/**
 * MOO (Machine Opcode Operation) binary format parser.
 * Parses .MOO and .MOO.gz files from the SingleStepTests project.
 *
 * Format spec: https://github.com/dbalsom/moo/blob/main/doc/moo_format_v1.md
 */

import { gunzipSync } from 'zlib';
import { readFileSync } from 'fs';

export interface MooTest {
  idx: number;
  name: string;
  bytes: number[];
  initial: CpuState;
  final: CpuState;
  exception?: { code: number; flags: number };
}

export interface CpuState {
  regs: Partial<Registers>;
  ram: [number, number][]; // [address, byte] pairs
}

export interface Registers {
  ax: number;
  bx: number;
  cx: number;
  dx: number;
  cs: number;
  ss: number;
  ds: number;
  es: number;
  sp: number;
  bp: number;
  si: number;
  di: number;
  ip: number;
  flags: number;
}

const REG_NAMES: (keyof Registers)[] = [
  'ax',
  'bx',
  'cx',
  'dx',
  'cs',
  'ss',
  'ds',
  'es',
  'sp',
  'bp',
  'si',
  'di',
  'ip',
  'flags',
];

class Reader {
  private buf: Buffer;
  private pos: number;

  constructor(buf: Buffer) {
    this.buf = buf;
    this.pos = 0;
  }

  get remaining() {
    return this.buf.length - this.pos;
  }

  get offset() {
    return this.pos;
  }

  readU8(): number {
    const v = this.buf.readUInt8(this.pos);
    this.pos += 1;
    return v;
  }

  readU16(): number {
    const v = this.buf.readUInt16LE(this.pos);
    this.pos += 2;
    return v;
  }

  readU32(): number {
    const v = this.buf.readUInt32LE(this.pos);
    this.pos += 4;
    return v;
  }

  readBytes(n: number): Buffer {
    const slice = this.buf.subarray(this.pos, this.pos + n);
    this.pos += n;
    return slice;
  }

  readTag(): string {
    return this.readBytes(4).toString('ascii');
  }

  readChunkHeader(): { tag: string; length: number } {
    const tag = this.readTag();
    const length = this.readU32();
    return { tag, length };
  }

  skip(n: number) {
    this.pos += n;
  }

  subReader(length: number): Reader {
    const sub = new Reader(this.buf.subarray(this.pos, this.pos + length));
    this.pos += length;
    return sub;
  }
}

function decodeRegs(reader: Reader): Partial<Registers> {
  const mask = reader.readU16();
  const regs: Partial<Registers> = {};
  for (let i = 0; i < REG_NAMES.length; i++) {
    if (mask & (1 << i)) {
      regs[REG_NAMES[i]] = reader.readU16();
    }
  }
  return regs;
}

function decodeRam(reader: Reader): [number, number][] {
  const count = reader.readU32();
  const ram: [number, number][] = [];
  for (let i = 0; i < count; i++) {
    const addr = reader.readU32();
    const value = reader.readU8();
    ram.push([addr, value]);
  }
  return ram;
}

function decodeCpuState(reader: Reader, length: number): CpuState {
  const state: CpuState = { regs: {}, ram: [] };
  const end = reader.offset + length;
  while (reader.offset < end) {
    const { tag, length: chunkLen } = reader.readChunkHeader();
    switch (tag) {
      case 'REGS':
        state.regs = decodeRegs(reader.subReader(chunkLen));
        break;
      case 'RAM ':
        state.ram = decodeRam(reader);
        break;
      case 'QUEU':
        reader.skip(chunkLen); // Skip instruction queue
        break;
      case 'RG32':
        reader.skip(chunkLen); // Skip 32-bit regs (not needed for 286)
        break;
      default:
        reader.skip(chunkLen);
        break;
    }
  }
  return state;
}

function decodeTest(reader: Reader): MooTest {
  // First 4 bytes are raw u32 test index
  const idx = reader.readU32();
  const test: MooTest = {
    idx,
    name: '',
    bytes: [],
    initial: { regs: {}, ram: [] },
    final: { regs: {}, ram: [] },
  };

  while (reader.remaining > 0) {
    const { tag, length: chunkLen } = reader.readChunkHeader();
    switch (tag) {
      case 'NAME': {
        const nameLen = reader.readU32();
        test.name = reader.readBytes(nameLen).toString('utf8');
        break;
      }
      case 'BYTS': {
        const bytesLen = reader.readU32();
        test.bytes = [...reader.readBytes(bytesLen)];
        break;
      }
      case 'GMET':
        reader.skip(chunkLen); // Skip generation metadata
        break;
      case 'INIT':
        test.initial = decodeCpuState(reader.subReader(chunkLen), chunkLen);
        break;
      case 'FINA':
        test.final = decodeCpuState(reader.subReader(chunkLen), chunkLen);
        break;
      case 'EXCP': {
        const code = reader.readU8();
        const flags = reader.readU32();
        test.exception = { code, flags };
        break;
      }
      case 'CYCL':
        reader.skip(chunkLen); // Skip cycle data
        break;
      case 'HASH':
        reader.skip(chunkLen); // Skip hash
        break;
      default:
        reader.skip(chunkLen);
        break;
    }
  }
  return test;
}

export interface MooFile {
  version: { major: number; minor: number };
  testCount: number;
  cpuId: string;
  tests: MooTest[];
  opcode?: string;
  mnemonic?: string;
  regMask?: number;
}

export function parseMoo(data: Buffer): MooFile {
  const reader = new Reader(data);

  // Read file header
  const { tag: headerTag, length: headerLen } = reader.readChunkHeader();
  if (headerTag !== 'MOO ') {
    throw new Error(
      `Invalid MOO file: expected 'MOO ' header, got '${headerTag}'`,
    );
  }

  const headerReader = reader.subReader(headerLen);
  const major = headerReader.readU8();
  const minor = headerReader.readU8();
  headerReader.skip(2); // reserved
  const testCount = headerReader.readU32();
  const cpuId = headerReader.readBytes(4).toString('ascii').trim();

  const file: MooFile = {
    version: { major, minor },
    testCount,
    cpuId,
    tests: [],
  };

  // Read top-level chunks
  while (reader.remaining > 0) {
    const { tag, length } = reader.readChunkHeader();
    switch (tag) {
      case 'META': {
        const metaReader = reader.subReader(length);
        // Parse META subchunks
        while (metaReader.remaining > 0) {
          const { tag: metaTag, length: metaLen } =
            metaReader.readChunkHeader();
          switch (metaTag) {
            case 'OPCD': {
              const opcLen = metaReader.readU8();
              file.opcode = [...metaReader.readBytes(opcLen)]
                .map((b) => b.toString(16).padStart(2, '0'))
                .join(' ');
              break;
            }
            case 'MNEM': {
              const mnLen = metaReader.readU16();
              file.mnemonic = metaReader.readBytes(mnLen).toString('utf8');
              break;
            }
            default:
              metaReader.skip(metaLen);
              break;
          }
        }
        break;
      }
      case 'RMSK': {
        file.regMask = reader.readU16();
        if (length > 2) reader.skip(length - 2);
        break;
      }
      case 'TEST':
        file.tests.push(decodeTest(reader.subReader(length)));
        break;
      default:
        reader.skip(length);
        break;
    }
  }

  return file;
}

export function loadMooFile(path: string): MooFile {
  let data = readFileSync(path);
  if (path.endsWith('.gz')) {
    data = gunzipSync(data);
  }
  return parseMoo(data);
}
