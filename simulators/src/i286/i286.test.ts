import { readFile } from 'fs/promises';
import { resolve } from 'path';
import Machine from './Machine';

const testDir = resolve(__dirname, '../../test/i286');

class TestMachine extends Machine {
  halted = 0;

  constructor(program: Uint8Array) {
    super();
    this.mem8.set(program, 0x100 + Machine.RAM_OFFSET);
    this.CS = 0x0;
    this.IP = 0x100;
  }

  private handleAssertions(vector: number): boolean {
    if (vector === 0x23) {
      if (this.AX !== this.BX) {
        console.log(`FAIL: AX=0x${this.AX.toString(16)} AH=0x${this.BX.toString(16)} IP=0x${(this.IP & 0xffff).toString(16)}`);
        this.halted = -1;
      }
      return true;
    } else if (vector === 0x22) {
      if (this.AL !== this.AH) {
        console.log(`FAIL: AL=0x${this.AL.toString(16)} AH=0x${this.AH.toString(16)} IP=0x${(this.IP & 0xffff).toString(16)}`);
        this.halted = -1;
      }
      return true;
    } else if (vector === 0x21) {
      const AH = this.AH;
      const AL = this.AL;
      const DX = this.DX;
      if (AH === 0x25) {
        const base = AL * 4 + Machine.RAM_OFFSET;
        this.mem16[base >> 1] = DX;
        this.mem16[(base >> 1) + 1] = this.DS;
      } else if (AH === 0x4c || AH === 0) {
        this.halted = 1;
      }
      return true;
    }
    return false;
  }

  interrupt_real(vector: number) {
    if (!this.handleAssertions(vector)) {
      super.interrupt_real(vector);
    }
  }

  interrupt_protected(vector: number) {
    if (!this.handleAssertions(vector)) {
      super.interrupt_protected(vector);
    } else {
      // Clear SOFTWARE_INT flag (mem8[268]) that was set by the INT instruction.
      // Without this, the next hardware exception would see stale SOFTWARE_INT=1
      // and incorrectly treat it as a software INT (clobbering the error code).
      this.mem8[268] = 0;
    }
  }
}

function runProgram(program: Uint8Array): { halted: number; ip: number } {
  const m = new TestMachine(program);
  for (let i = 0; i < 10000; i++) {
    if (m.mode === 0) {
      m.decode_real();
    } else {
      m.decode_protected();
    }
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

describe('i286 simple instructions', () => {
  test.each(simple)('%s', async (name) => {
    const data = await readFile(resolve(testDir, `simple/bin/${name}.com`));
    const { halted } = runProgram(data);
    expect(halted).toBe(1);
  });
});

describe('i286 complex instructions', () => {
  test.each(complex)('%s', async (name) => {
    const data = await readFile(resolve(testDir, `complex/bin/${name}.com`));
    const { halted } = runProgram(data);
    expect(halted).toBe(1);
  });
});

const protected_mode = [
  'gdt_idt',
  'mode_switch',
  'descriptor_ops',
  'interrupts',
  'segment_load',
  'task_switch',
  'privilege',
  'exceptions',
  'ldt',
  'strings',
  'expand_down',
  'conforming',
  'gates',
  'task_gate',
  'iopl',
  'stack_fault',
  'arpl',
];

describe('i286 protected mode', () => {
  test.each(protected_mode)('%s', async (name) => {
    const data = await readFile(resolve(testDir, `protected/bin/${name}.com`));
    const { halted } = runProgram(data);
    expect(halted).toBe(1);
  });
});
