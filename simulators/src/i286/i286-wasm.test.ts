import { readFile } from 'fs/promises';
import { resolve } from 'path';
import WasmMachine from './WasmMachine';

const testDir = resolve(__dirname, '../../test/i286');
const wasmPath = resolve(__dirname, 'machine.wasm');

class TestWasmMachine extends WasmMachine {
  halted = 0;

  interrupt_real(vector: number): number {
    if (vector === 0x23) {
      // Assert AX === BX
      if (this.AX !== this.BX) {
        this.halted = -1;
      }
      return 1;
    } else if (vector === 0x22) {
      // Assert AL === AH
      if (this.AL !== this.AH) {
        this.halted = -1;
      }
      return 1;
    } else if (vector === 0x21) {
      const AH = this.AH;
      const AL = this.AL;
      const DX = this.DX;
      if (AH === 0x25) {
        // Set interrupt vector
        const base = AL * 4 + WasmMachine.RAM_OFFSET;
        this.mem16[base >> 1] = DX;
        this.mem16[(base >> 1) + 1] = this.DS;
      } else if (AH === 0x4c || AH === 0) {
        this.halted = 1;
      }
      return 1;
    }
    return 0; // Not handled — do IVT vectoring
  }
}

let wasmBinary: Buffer;

beforeAll(async () => {
  wasmBinary = await readFile(wasmPath);
});

async function runProgram(
  program: Uint8Array,
): Promise<{ halted: number; ip: number }> {
  const m = new TestWasmMachine();
  await m.init(wasmBinary);
  m.mem8.set(program, 0x100 + WasmMachine.RAM_OFFSET);
  m.CS = 0x0;
  m.IP = 0x100;
  for (let i = 0; i < 100000; i++) {
    m.decode_real();
    if (m.halted) {
      return { halted: m.halted, ip: m.IP };
    }
  }
  return { halted: m.halted, ip: m.IP };
}

const simple = [
  'adc',
  'add',
  'and',
  'call',
  'cbw',
  'clc',
  'cmc',
  'cwd',
  'dec',
  'inc',
  'jmp',
  'not',
  'or',
  'pop',
  'push',
  'ret',
  'sbb',
  'stc',
  'sub',
  'xor',
];

const complex = [
  'aaa',
  'aad',
  'aam',
  'aas',
  'adc',
  'add',
  'and',
  'bound',
  'call',
  'cbw',
  'cwd',
  'cmp',
  'cmps',
  'cwd',
  'daa',
  'das',
  'dec',
  'div',
  'enter',
  'idiv',
  'imul',
  'inc',
  'ja',
  'jae',
  'jb',
  'jbe',
  'jcxz',
  'je',
  'jg',
  'jge',
  'jl',
  'jle',
  'jmp',
  'jne',
  'jno',
  'jnp',
  'jns',
  'jo',
  'jp',
  'js',
  'lahf',
  'lea',
  'leave',
  'lods',
  'mov',
  'movs',
  'mul',
  'neg',
  'not',
  'or',
  'pop',
  'popa',
  'popf',
  'push',
  'pusha',
  'pushf',
  'rcl',
  'rcr',
  'ret',
  'rol',
  'ror',
  'sahf',
  'sal',
  'sar',
  'sbb',
  'scas',
  'shr',
  'stos',
  'sub',
  'test',
  'xchg',
  'xor',
];

describe('i286 wasm simple instructions', () => {
  test.each(simple)('%s', async (name) => {
    const data = await readFile(resolve(testDir, `simple/bin/${name}.com`));
    const { halted } = await runProgram(data);
    expect(halted).toBe(1);
  });
});

describe('i286 wasm complex instructions', () => {
  test.each(complex)('%s', async (name) => {
    const data = await readFile(resolve(testDir, `complex/bin/${name}.com`));
    const { halted } = await runProgram(data);
    expect(halted).toBe(1);
  });
});
