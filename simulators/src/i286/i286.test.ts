import { readFile } from 'fs/promises';
import Machine from './Machine';

class TestMachine extends Machine {
  halted = 0;

  constructor(program: Uint8Array) {
    super();
    this.mem8.set(program, 0x100 + 0x1a0);
    this.CS = 0x0;
    this.IP = 0x100;
  }

  interrupt(vector: number) {
    if (vector === 0x23) {
      // Assert AX === BX
      if (this.AX !== this.BX) {
        this.halted = -1;
      }
    } else if (vector === 0x22) {
      // Assert AL === AH
      if (this.AL !== this.AH) {
        this.halted = -1;
      }
    } else if (vector === 0x21) {
      const AH = this.AH;
      const AL = this.AL;
      const DX = this.DX;
      if (AH === 0x25) {
        // Set interrupt vector
        const base = (AL * 4) + 0x1a0;
        this.mem16[base >> 1] = this.DS;
        this.mem16[(base >> 1) + 1] = DX;
      } else if (AH === 0x4c || AH === 0) {
        this.halted = 1;
      }
    } else {
      super.interrupt(vector);
    }
  }
}

function runProgram(program: Uint8Array): { halted: number; ip: number } {
  const m = new TestMachine(program);
  for (let i = 0; i < 10000; i++) {
    m.decode_real();
    if (m.halted) {
      return { halted: m.halted, ip: m.IP };
    }
  }
  return { halted: m.halted, ip: m.IP };
}

const simple = [
  'adc', 'add', 'and', 'call', 'cbw', 'clc', 'cmc', 'cwd',
  'dec', 'inc', 'jmp', 'not', 'or', 'pop', 'push', 'ret',
  'sbb', 'stc', 'sub', 'xor',
];

const complex = [
  'aaa', 'aad', 'aam', 'aas', 'adc', 'add', 'and', 'bound',
  'call', 'cbw', 'cwd', 'cmp', 'cmps', 'cwd', 'daa', 'das',
  'dec', 'div', 'enter', 'idiv', 'imul', 'inc',
  'ja', 'jae', 'jb', 'jbe', 'jcxz', 'je', 'jg', 'jge',
  'jl', 'jle', 'jmp', 'jne', 'jno', 'jnp', 'jns', 'jo', 'jp', 'js',
  'lahf', 'lea', 'leave', 'lods', 'mov', 'movs', 'mul', 'neg',
  'not', 'or', 'pop', 'popa', 'popf', 'push', 'pusha', 'pushf',
  'rcl', 'rcr', 'ret', 'rol', 'ror', 'sahf', 'sal', 'sar',
  'sbb', 'scas', 'shr', 'stos', 'sub', 'test', 'xchg', 'xor',
];

describe('i286 simple instructions', () => {
  test.each(simple)('%s', async (name) => {
    const data = await readFile(`test/i286/simple/bin/${name}.com`);
    const { halted } = runProgram(data);
    expect(halted).toBe(1);
  });
});

describe('i286 complex instructions', () => {
  test.each(complex)('%s', async (name) => {
    const data = await readFile(`test/i286/complex/bin/${name}.com`);
    const { halted } = runProgram(data);
    expect(halted).toBe(1);
  });
});
