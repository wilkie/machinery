/**
 * Shared SingleStepTests runner for the i286 simulator.
 *
 * Each singlestep-XX.test.ts file imports this module and calls
 * `runSingleStepTests(prefix)` with its hex-digit prefix so that Jest
 * naturally spreads the opcode ranges across separate worker processes.
 */

import { existsSync, readdirSync } from 'fs';
import { resolve } from 'path';
import Machine from './Machine';
import { loadMooFile, type MooTest, type Registers } from '../../test/i286/moo';

// Resolve test data directory
const testDir = resolve(__dirname, '../../test/i286');
const singleStepPath =
  process.env.SINGLE_STEP_TESTS_PATH ||
  resolve(testDir, 'SingleStepTests/80286');
const realModeDir = resolve(singleStepPath, 'v1_real_mode');

export const HAS_TESTS = existsSync(realModeDir);

// Register name -> Machine property mapping
const REG_SETTERS: Record<keyof Registers, string> = {
  ax: 'AX',
  bx: 'BX',
  cx: 'CX',
  dx: 'DX',
  cs: 'CS',
  ss: 'SS',
  ds: 'DS',
  es: 'ES',
  sp: 'SP',
  bp: 'BP',
  si: 'SI',
  di: 'DI',
  ip: 'IP',
  flags: 'FLAGS',
};

// Flags mask: on 80286, bits 15-12 are always 0 in real mode (IOPL/NT),
// and bit 1 is always 1 (reserved). We mask to the flags the 286 actually has.
const FLAGS_MASK_286 = 0x0fd7; // CF, PF, AF, ZF, SF, TF, IF, DF, OF (no IOPL/NT in real mode)

// Internal register offsets for lazy flag evaluation state.
const FLAG_OP_OFFSET = 46; // mem16 index
const FLAG_OP_RESOLVED = 0x8000;
const CARRY_OFFSET = 252; // mem8 index — mirrors CF through RESOLVE_FLAGS

function setupMachine(test: MooTest): Machine {
  const m = new Machine();
  m.mode = 0; // real mode

  // Mark flags as already resolved so HLT's RESOLVE_FLAGS doesn't clobber
  // the initial flag state.
  m.mem16[FLAG_OP_OFFSET] = FLAG_OP_RESOLVED;
  if (test.initial.regs.flags !== undefined) {
    m.mem8[CARRY_OFFSET] = test.initial.regs.flags & 1;
  }

  // Set RAM first (before registers, since segment setters read memory)
  for (const [addr, value] of test.initial.ram) {
    m.mem8[addr + Machine.RAM_OFFSET] = value;
  }

  // Set registers — segments first, then GP regs, then IP last
  const segRegs: (keyof Registers)[] = ['cs', 'ss', 'ds', 'es'];
  const gpRegs: (keyof Registers)[] = [
    'ax',
    'bx',
    'cx',
    'dx',
    'sp',
    'bp',
    'si',
    'di',
  ];

  for (const reg of segRegs) {
    if (test.initial.regs[reg] !== undefined) {
      m[REG_SETTERS[reg]] = test.initial.regs[reg];
    }
  }
  for (const reg of gpRegs) {
    if (test.initial.regs[reg] !== undefined) {
      m[REG_SETTERS[reg]] = test.initial.regs[reg];
    }
  }
  if (test.initial.regs.flags !== undefined) {
    m.FLAGS = test.initial.regs.flags;
  }
  if (test.initial.regs.ip !== undefined) {
    m.IP = test.initial.regs.ip;
  }

  return m;
}

function compareFinalState(m: Machine, test: MooTest): string[] {
  const errors: string[] = [];

  for (const [reg, expected] of Object.entries(test.final.regs) as [
    keyof Registers,
    number,
  ][]) {
    const prop = REG_SETTERS[reg];
    const actual = m[prop] as number;

    if (reg === 'flags') {
      const mask = FLAGS_MASK_286;
      const maskedActual = actual & mask;
      const maskedExpected = expected & mask;
      if (maskedActual !== maskedExpected) {
        const diff = maskedActual ^ maskedExpected;
        const flagNames: string[] = [];
        if (diff & 0x001) flagNames.push('CF');
        if (diff & 0x004) flagNames.push('PF');
        if (diff & 0x010) flagNames.push('AF');
        if (diff & 0x040) flagNames.push('ZF');
        if (diff & 0x080) flagNames.push('SF');
        if (diff & 0x100) flagNames.push('TF');
        if (diff & 0x200) flagNames.push('IF');
        if (diff & 0x400) flagNames.push('DF');
        if (diff & 0x800) flagNames.push('OF');
        errors.push(
          `FLAGS: expected 0x${maskedExpected.toString(16).padStart(4, '0')}, ` +
            `got 0x${maskedActual.toString(16).padStart(4, '0')} ` +
            `(diff: ${flagNames.join(', ')})`,
        );
      }
    } else {
      const mask = 0xffff;
      if ((actual & mask) !== (expected & mask)) {
        errors.push(
          `${prop}: expected 0x${(expected & mask).toString(16).padStart(4, '0')}, ` +
            `got 0x${(actual & mask).toString(16).padStart(4, '0')}`,
        );
      }
    }
  }

  for (const [addr, expected] of test.final.ram) {
    const actual = m.mem8[addr + Machine.RAM_OFFSET];
    if (actual !== expected) {
      errors.push(
        `RAM[0x${addr.toString(16)}]: expected 0x${expected.toString(16).padStart(2, '0')}, ` +
          `got 0x${actual.toString(16).padStart(2, '0')}`,
      );
    }
  }

  return errors;
}

function runTest(test: MooTest): string[] {
  const m = setupMachine(test);

  const maxIter = 1000;
  for (let i = 0; i < maxIter; i++) {
    if (m.mode === 2) break;
    if (m.mode === 0) {
      m.decode_real();
    } else {
      m.decode_protected();
    }
  }

  if (m.mode !== 2) {
    return [
      `Machine did not halt after ${maxIter} iterations (mode=${m.mode})`,
    ];
  }

  return compareFinalState(m, test);
}

// Discover test files filtered by hex prefix (e.g. "0" matches 00-0F)
function getTestFiles(prefix: string): string[] {
  if (!HAS_TESTS) return [];
  try {
    return readdirSync(realModeDir)
      .filter(
        (f) =>
          (f.endsWith('.MOO.gz') || f.endsWith('.MOO')) &&
          f.toUpperCase().startsWith(prefix.toUpperCase()),
      )
      .sort();
  } catch {
    return [];
  }
}

/**
 * Register SingleStepTests for all opcodes starting with the given hex prefix.
 * Call this from each singlestep-XX.test.ts file.
 */
export function runSingleStepTests(prefix: string): void {
  if (!HAS_TESTS) {
    describe(`SingleStepTests ${prefix}x`, () => {
      it.skip('skipped — test data not found', () => {});
    });
    return;
  }

  const testFiles = getTestFiles(prefix);

  describe(`SingleStepTests ${prefix}x`, () => {
    for (const file of testFiles) {
      const opcode = file.replace(/\.MOO(\.gz)?$/, '');

      describe(`opcode ${opcode}`, () => {
        let tests: ReturnType<typeof loadMooFile> | null = null;

        beforeAll(() => {
          tests = loadMooFile(resolve(realModeDir, file));
        });

        it('passes all test cases', () => {
          if (!tests || tests.tests.length === 0) return;

          const sampleFailures: string[] = [];
          let passed = 0;
          let failed = 0;

          for (const test of tests.tests) {
            const errors = runTest(test);
            if (errors.length > 0) {
              failed++;
              if (sampleFailures.length < 5) {
                sampleFailures.push(
                  `Test #${test.idx} "${test.name}" [${test.bytes.map((b) => b.toString(16).padStart(2, '0')).join(' ')}]:\n` +
                    errors.map((e) => `  ${e}`).join('\n'),
                );
                sampleFailures.push(JSON.stringify(test, null, 2));
              }
            } else {
              passed++;
            }
          }

          if (failed > 0) {
            const total = tests!.tests.length;
            throw new Error(
              `${failed}/${total} failed (${passed} passed):\n\n` +
                sampleFailures.join('\n\n') +
                (failed > 5 ? `\n\n... and ${failed - 5} more` : ''),
            );
          }
        });
      });
    }
  });
}
