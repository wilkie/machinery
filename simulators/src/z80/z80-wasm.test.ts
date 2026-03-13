import { readFile } from 'fs/promises';
import { resolve } from 'path';
import WasmMachine from './WasmMachine';

const testDir = resolve(__dirname, '../../test/z80');
const wasmPath = resolve(__dirname, 'machine.wasm');

class TestWasmMachine extends WasmMachine {
  halted = 0;

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
        this.mem8[sp + WasmMachine.RAM_OFFSET] |
        (this.mem8[sp + 1 + WasmMachine.RAM_OFFSET] << 8);
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
        this.mem8[sp + WasmMachine.RAM_OFFSET] |
        (this.mem8[sp + 1 + WasmMachine.RAM_OFFSET] << 8);
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

let wasmBinary: Buffer;

beforeAll(async () => {
  wasmBinary = await readFile(wasmPath);
});

async function runProgram(
  program: Uint8Array,
): Promise<{ halted: number; pc: number }> {
  const m = new TestWasmMachine();
  await m.init(wasmBinary);
  // Load program at address 0x100 in Z80 address space
  m.mem8.set(program, 0x100 + WasmMachine.RAM_OFFSET);
  m.PC = 0x100;
  m.SP = 0xfffe;
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
  'adc',
  'sub',
  'sbc',
  'and',
  'or',
  'xor',
  'inc',
  'dec',
  'jp',
  'jr',
  'cp',
  'call',
  'push',
  'pop',
  'ex',
  'djnz',
  'rotate',
  'misc',
  'block',
  'ix',
  'iy',
  'bit',
  'cond',
];

describe('z80 wasm simple instructions', () => {
  test.each(simple)('%s', async (name) => {
    const data = await readFile(resolve(testDir, `simple/bin/${name}.bin`));
    const { halted } = await runProgram(data);
    expect(halted).toBe(1);
  });
});
