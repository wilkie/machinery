import { readFile } from 'fs/promises';
import { resolve } from 'path';
import Machine from './Machine';

const testDir = resolve(__dirname, '../../test/z80');

// RAM starts at this byte offset in the flat WebAssembly memory
const RAM_OFFSET = 0x140;

class TestMachine extends Machine {
  halted = 0;

  constructor(program: Uint8Array) {
    super();
    // Load program at address 0x100 in Z80 address space
    this.mem8.set(program, 0x100 + RAM_OFFSET);
    this.PC = 0x100;
    this.SP = 0xfffe;
  }

  step() {
    const pc = this.PC;

    // RST 0x10: assert A == B (8-bit equality)
    if (pc === 0x10) {
      if (this.A !== this.B) {
        this.halted = -1;
      }
      // Simulate RET: pop return address from stack
      const sp = this.SP;
      this.PC =
        this.mem8[sp + RAM_OFFSET] | (this.mem8[sp + 1 + RAM_OFFSET] << 8);
      this.SP = sp + 2;
      return;
    }

    // RST 0x18: assert HL == DE (16-bit equality)
    if (pc === 0x18) {
      if (this.HL !== this.DE) {
        this.halted = -1;
      }
      // Simulate RET
      const sp = this.SP;
      this.PC =
        this.mem8[sp + RAM_OFFSET] | (this.mem8[sp + 1 + RAM_OFFSET] << 8);
      this.SP = sp + 2;
      return;
    }

    this.decode_default();

    // Check if HALT was executed (mem8[42] is the HALTED state flag)
    if (this.mem8[42]) {
      this.halted = 1;
    }
  }
}

function runProgram(program: Uint8Array): { halted: number; pc: number } {
  const m = new TestMachine(program);
  for (let i = 0; i < 10000; i++) {
    m.step();
    if (m.halted) {
      return { halted: m.halted, pc: m.PC };
    }
  }
  return { halted: m.halted, pc: m.PC };
}

const simple = [
  'ld',
  'add',
  'sub',
  'and',
  'or',
  'xor',
  'inc',
  'dec',
  'jp',
  'jr',
  'call',
  'push',
  'ex',
  'ix',
  'iy',
  'bit',
  'cond',
];

describe('z80 simple instructions', () => {
  test.each(simple)('%s', async (name) => {
    const data = await readFile(resolve(testDir, `simple/bin/${name}.bin`));
    const { halted } = runProgram(data);
    expect(halted).toBe(1);
  });
});
