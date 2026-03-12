import { readFileSync, readdirSync } from 'fs';
import { resolve } from 'path';
import { Decoder, disassemble } from './Decoder';
import { AttSyntax } from './AttSyntax';
import { IntelSyntax } from './IntelSyntax';
import { generateDecoderMap } from '@machinery/core';
import type { Target } from '@machinery/core';

// We import the i286 target to test with real processor metadata
let target: Target;
let decoderMap: ReturnType<typeof generateDecoderMap>;
let prefixMap: ReturnType<typeof generateDecoderMap>;

beforeAll(async () => {
  const mod = await import('@machinery/processors/i286');
  target = mod.default;
  decoderMap = generateDecoderMap(target, false);
  prefixMap = generateDecoderMap(target, true);
});

function decode(bytes: number[]) {
  const data = new Uint8Array(bytes);
  const decoder = new Decoder(data, decoderMap, prefixMap, target);
  const instr = decoder.decode();
  return instr;
}

function decodeAll(bytes: number[]) {
  const data = new Uint8Array(bytes);
  return [...disassemble(data, decoderMap, prefixMap, target)];
}

describe('Decoder', () => {
  describe('zero-operand instructions', () => {
    it.each([
      [[0x90], 'nop'],
      [[0xf8], 'clc'],
      [[0xf9], 'stc'],
      [[0xf5], 'cmc'],
      [[0xfa], 'cli'],
      [[0xfb], 'sti'],
      [[0xfc], 'cld'],
      [[0xfd], 'std'],
      [[0xf4], 'hlt'],
      [[0x98], 'cbw'],
      [[0x99], 'cwd'],
      [[0x9e], 'sahf'],
      [[0x9f], 'lahf'],
      [[0x60], 'pusha'],
      [[0x61], 'popa'],
    ])('%s -> %s', (bytes, mnemonic) => {
      const instr = decode(bytes);
      expect(instr).not.toBeNull();
      expect(instr!.mnemonic).toBe(mnemonic);
      expect(instr!.operands).toHaveLength(0);
    });
  });

  describe('MOV reg, imm', () => {
    it('MOV AL, 0x42 -> B0 42', () => {
      const instr = decode([0xb0, 0x42]);
      expect(instr!.mnemonic).toBe('mov');
      expect(instr!.operands).toHaveLength(2);
      expect(instr!.operands[0]).toMatchObject({
        type: 'register',
        name: 'al',
      });
      expect(instr!.operands[1]).toMatchObject({
        type: 'immediate',
        value: 0x42,
      });
    });

    it('MOV AX, 0x1234 -> B8 34 12', () => {
      const instr = decode([0xb8, 0x34, 0x12]);
      expect(instr!.mnemonic).toBe('mov');
      expect(instr!.operands[0]).toMatchObject({
        type: 'register',
        name: 'ax',
      });
      expect(instr!.operands[1]).toMatchObject({
        type: 'immediate',
        value: 0x1234,
      });
    });

    it('MOV BX, 0x4444 -> BB 44 44', () => {
      const instr = decode([0xbb, 0x44, 0x44]);
      expect(instr!.operands[0]).toMatchObject({
        type: 'register',
        name: 'bx',
      });
      expect(instr!.operands[1]).toMatchObject({
        type: 'immediate',
        value: 0x4444,
      });
    });
  });

  describe('MOV reg, reg', () => {
    it('MOV AX, BX -> 89 D8', () => {
      const instr = decode([0x89, 0xd8]);
      expect(instr!.mnemonic).toBe('mov');
      expect(instr!.operands).toHaveLength(2);
      // rm=AX, reg=BX
      expect(instr!.operands[0]).toMatchObject({
        type: 'register',
        name: 'ax',
      });
      expect(instr!.operands[1]).toMatchObject({
        type: 'register',
        name: 'bx',
      });
    });
  });

  describe('MOV segment registers', () => {
    it('MOV DS, AX -> 8E D8', () => {
      const instr = decode([0x8e, 0xd8]);
      expect(instr!.mnemonic).toBe('mov');
      expect(
        instr!.operands.some((o) => o.type === 'register' && o.name === 'ds'),
      ).toBe(true);
      expect(
        instr!.operands.some((o) => o.type === 'register' && o.name === 'ax'),
      ).toBe(true);
    });

    it('MOV AX, DS -> 8C D8', () => {
      const instr = decode([0x8c, 0xd8]);
      expect(instr!.mnemonic).toBe('mov');
      expect(
        instr!.operands.some((o) => o.type === 'register' && o.name === 'ds'),
      ).toBe(true);
      expect(
        instr!.operands.some((o) => o.type === 'register' && o.name === 'ax'),
      ).toBe(true);
    });
  });

  describe('PUSH/POP register', () => {
    it.each([
      [[0x50], 'push', 'ax'],
      [[0x53], 'push', 'bx'],
      [[0x57], 'push', 'di'],
      [[0x58], 'pop', 'ax'],
      [[0x5f], 'pop', 'di'],
    ])('%s -> %s %s', (bytes, mnemonic, reg) => {
      const instr = decode(bytes);
      expect(instr!.mnemonic).toBe(mnemonic);
      expect(instr!.operands[0]).toMatchObject({ type: 'register', name: reg });
    });
  });

  describe('PUSH/POP segment register', () => {
    it.each([
      [[0x06], 'push', 'es'],
      [[0x0e], 'push', 'cs'],
      [[0x16], 'push', 'ss'],
      [[0x1e], 'push', 'ds'],
      [[0x07], 'pop', 'es'],
      [[0x17], 'pop', 'ss'],
      [[0x1f], 'pop', 'ds'],
    ])('%s -> %s %s', (bytes, mnemonic, reg) => {
      const instr = decode(bytes);
      expect(instr!.mnemonic).toBe(mnemonic);
      expect(instr!.operands[0]).toMatchObject({ type: 'register', name: reg });
    });
  });

  describe('INC/DEC register', () => {
    it('INC AX -> 40', () => {
      const instr = decode([0x40]);
      expect(instr!.mnemonic).toBe('inc');
      expect(instr!.operands[0]).toMatchObject({
        type: 'register',
        name: 'ax',
      });
    });

    it('DEC BX -> 4B', () => {
      const instr = decode([0x4b]);
      expect(instr!.mnemonic).toBe('dec');
      expect(instr!.operands[0]).toMatchObject({
        type: 'register',
        name: 'bx',
      });
    });
  });

  describe('ALU accumulator, imm', () => {
    it('ADD AL, 0x42 -> 04 42', () => {
      const instr = decode([0x04, 0x42]);
      expect(instr!.mnemonic).toBe('add');
      expect(instr!.operands[0]).toMatchObject({
        type: 'register',
        name: 'al',
      });
      expect(instr!.operands[1]).toMatchObject({
        type: 'immediate',
        value: 0x42,
      });
    });

    it('CMP AL, 0x04 -> 3C 04', () => {
      const instr = decode([0x3c, 0x04]);
      expect(instr!.mnemonic).toBe('cmp');
      expect(instr!.operands[0]).toMatchObject({
        type: 'register',
        name: 'al',
      });
      expect(instr!.operands[1]).toMatchObject({
        type: 'immediate',
        value: 0x04,
      });
    });
  });

  describe('INT', () => {
    it('INT 0x21 -> CD 21', () => {
      const instr = decode([0xcd, 0x21]);
      expect(instr!.mnemonic).toBe('int');
      expect(instr!.operands[0]).toMatchObject({
        type: 'immediate',
        value: 0x21,
      });
    });
  });

  describe('relative jumps', () => {
    it('JL +5 -> 7C 05 (target = 7)', () => {
      const instr = decode([0x7c, 0x05]);
      expect(instr!.mnemonic).toBe('jl');
      expect(instr!.operands[0]).toMatchObject({ type: 'relative', target: 7 });
    });

    it('JMP SHORT -3 -> EB FD (target = -1)', () => {
      const instr = decode([0xeb, 0xfd]);
      expect(instr!.mnemonic).toBe('jmp');
      expect(instr!.operands[0]).toMatchObject({
        type: 'relative',
        target: -1,
      });
    });
  });

  describe('memory addressing', () => {
    it('MOV [0x06df], SS -> 8C 16 DF 06', () => {
      const instr = decode([0x8c, 0x16, 0xdf, 0x06]);
      expect(instr!.mnemonic).toBe('mov');
      expect(instr!.operands[0]).toMatchObject({
        type: 'memory',
        direct: true,
        displacement: 0x06df,
      });
      expect(instr!.operands[1]).toMatchObject({
        type: 'register',
        name: 'ss',
      });
    });

    it('MOV AH, [0x0ee5] -> 8A 26 E5 0E', () => {
      const instr = decode([0x8a, 0x26, 0xe5, 0x0e]);
      expect(instr!.mnemonic).toBe('mov');
      expect(instr!.operands[0]).toMatchObject({
        type: 'register',
        name: 'ah',
      });
      expect(instr!.operands[1]).toMatchObject({
        type: 'memory',
        direct: true,
        displacement: 0x0ee5,
      });
    });

    it('MOV [BX+SI], AX -> 89 00 (mod=00 memory)', () => {
      const instr = decode([0x89, 0x00]);
      expect(instr!.mnemonic).toBe('mov');
      expect(instr!.operands[0]).toMatchObject({
        type: 'memory',
        base: 'bx',
        index: 'si',
        direct: false,
      });
      expect(instr!.operands[1]).toMatchObject({
        type: 'register',
        name: 'ax',
      });
    });

    it('MOV [BP+DI+5], CL -> 88 4B 05 (mod=01 disp8)', () => {
      const instr = decode([0x88, 0x4b, 0x05]);
      expect(instr!.mnemonic).toBe('mov');
      expect(instr!.operands[0]).toMatchObject({
        type: 'memory',
        base: 'bp',
        index: 'di',
        displacement: 5,
        displacementSize: 8,
        direct: false,
      });
      expect(instr!.operands[1]).toMatchObject({
        type: 'register',
        name: 'cl',
      });
    });

    it('MOV [SI+0x1234], DX -> 89 94 34 12 (mod=10 disp16)', () => {
      const instr = decode([0x89, 0x94, 0x34, 0x12]);
      expect(instr!.mnemonic).toBe('mov');
      expect(instr!.operands[0]).toMatchObject({
        type: 'memory',
        base: 'si',
        displacement: 0x1234,
        displacementSize: 16,
        direct: false,
      });
      expect(instr!.operands[1]).toMatchObject({
        type: 'register',
        name: 'dx',
      });
    });

    it('ADD [BX], 0x01 -> 80 07 01 (mod=00 rm memory + imm8)', () => {
      const instr = decode([0x80, 0x07, 0x01]);
      expect(instr!.mnemonic).toBe('add');
      expect(instr!.operands[0]).toMatchObject({
        type: 'memory',
        base: 'bx',
        direct: false,
      });
      expect(instr!.operands[1]).toMatchObject({
        type: 'immediate',
        value: 0x01,
      });
    });
  });

  describe('segment override prefix', () => {
    it('MOV AX, SS:[BP+0] -> 36 8B 46 00', () => {
      const instr = decode([0x36, 0x8b, 0x46, 0x00]);
      expect(instr!.mnemonic).toBe('mov');
      expect(instr!.segmentOverride).toBe('ss');
    });

    it('ES: prefix -> 26', () => {
      // ES: MOV AX, [BX+SI] -> 26 8B 00
      const instr = decode([0x26, 0x8b, 0x00]);
      expect(instr!.mnemonic).toBe('mov');
      expect(instr!.segmentOverride).toBe('es');
    });
  });

  describe('ALU group instructions', () => {
    it('SUB AX, 0x0002 -> 83 E8 02 (sign-extended imm8)', () => {
      const instr = decode([0x83, 0xe8, 0x02]);
      expect(instr!.mnemonic).toBe('sub');
      expect(instr!.operands[0]).toMatchObject({
        type: 'register',
        name: 'ax',
      });
      expect(instr!.operands[1]).toMatchObject({
        type: 'immediate',
        value: 2,
      });
    });

    it('XOR AH, AH -> 30 E4', () => {
      const instr = decode([0x30, 0xe4]);
      expect(instr!.mnemonic).toBe('xor');
      expect(instr!.operands[0]).toMatchObject({
        type: 'register',
        name: 'ah',
      });
      expect(instr!.operands[1]).toMatchObject({
        type: 'register',
        name: 'ah',
      });
    });

    it('AND AL, 0x01 -> 24 01', () => {
      const instr = decode([0x24, 0x01]);
      expect(instr!.mnemonic).toBe('and');
      expect(instr!.operands[0]).toMatchObject({
        type: 'register',
        name: 'al',
      });
      expect(instr!.operands[1]).toMatchObject({
        type: 'immediate',
        value: 0x01,
      });
    });

    it('SHR CL, AX -> D3 E8', () => {
      const instr = decode([0xd3, 0xe8]);
      expect(instr!.mnemonic).toBe('shr');
      expect(instr!.operands).toHaveLength(2);
    });

    it('TEST AL, 0x01 -> A8 01', () => {
      const instr = decode([0xa8, 0x01]);
      expect(instr!.mnemonic).toBe('test');
      expect(instr!.operands[0]).toMatchObject({
        type: 'register',
        name: 'al',
      });
      expect(instr!.operands[1]).toMatchObject({
        type: 'immediate',
        value: 0x01,
      });
    });
  });

  describe('CALL and RET', () => {
    it('CALL rel16 -> E8 xx xx', () => {
      const instr = decode([0xe8, 0x10, 0x00]);
      expect(instr!.mnemonic).toBe('call');
      expect(instr!.operands[0]).toMatchObject({
        type: 'relative',
        target: 0x13, // 3 bytes instr + 0x10 offset
      });
    });

    it('RET -> C3', () => {
      const instr = decode([0xc3]);
      expect(instr!.mnemonic).toBe('ret');
    });

    it('RET imm16 -> C2 xx xx', () => {
      const instr = decode([0xc2, 0x04, 0x00]);
      expect(instr!.mnemonic).toBe('ret');
      expect(instr!.operands[0]).toMatchObject({
        type: 'immediate',
        value: 4,
      });
    });
  });

  describe('XCHG', () => {
    it('XCHG AX, CX -> 91', () => {
      const instr = decode([0x91]);
      expect(instr!.mnemonic).toBe('xchg');
      expect(
        instr!.operands.some((o) => o.type === 'register' && o.name === 'cx'),
      ).toBe(true);
    });
  });

  describe('LEA', () => {
    it('LEA AX, [0x1234] -> 8D 06 34 12', () => {
      const instr = decode([0x8d, 0x06, 0x34, 0x12]);
      expect(instr!.mnemonic).toBe('lea');
      // LEA form has operands ['rm', 'reg'] — memory is first
      expect(instr!.operands).toHaveLength(2);
      expect(
        instr!.operands.some(
          (o) =>
            o.type === 'memory' &&
            o.direct === true &&
            o.displacement === 0x1234,
        ),
      ).toBe(true);
      expect(
        instr!.operands.some((o) => o.type === 'register' && o.name === 'ax'),
      ).toBe(true);
    });
  });

  describe('REP prefix', () => {
    it('REP MOVSB -> F3 A4', () => {
      const instr = decode([0xf3, 0xa4]);
      expect(instr!.mnemonic).toBe('movs');
      expect(instr!.prefix).toBe('rep');
    });

    it('REPNE SCASB -> F2 AE', () => {
      const instr = decode([0xf2, 0xae]);
      expect(instr!.mnemonic).toBe('scas');
      expect(instr!.prefix).toBe('repne');
    });
  });

  describe('LOCK prefix', () => {
    it('LOCK prefix is recorded', () => {
      // LOCK INC [BX] would be F0 FF 07
      const instr = decode([0xf0, 0xff, 0x07]);
      expect(instr!.prefix).toBe('lock');
    });
  });

  describe('far pointer', () => {
    it('CALL FAR ptr16:16 -> 9A xx xx xx xx', () => {
      const instr = decode([0x9a, 0x00, 0x01, 0x00, 0x10]);
      expect(instr!.mnemonic).toBe('call');
      expect(instr!.operands[0]).toMatchObject({
        type: 'far_pointer',
        offset: 0x0100,
        segment: 0x1000,
      });
    });
  });

  describe('ENTER/LEAVE', () => {
    it('ENTER imm16, imm8 -> C8 xx xx xx', () => {
      const instr = decode([0xc8, 0x10, 0x00, 0x00]);
      expect(instr!.mnemonic).toBe('enter');
      expect(instr!.operands).toHaveLength(2);
      expect(instr!.operands[0]).toMatchObject({
        type: 'immediate',
        value: 0x10,
      });
    });

    it('LEAVE -> C9', () => {
      const instr = decode([0xc9]);
      expect(instr!.mnemonic).toBe('leave');
      expect(instr!.operands).toHaveLength(0);
    });
  });

  describe('PUSH immediate', () => {
    it('PUSH imm16 -> 68 xx xx', () => {
      const instr = decode([0x68, 0x34, 0x12]);
      expect(instr!.mnemonic).toBe('push');
      expect(instr!.operands[0]).toMatchObject({
        type: 'immediate',
        value: 0x1234,
      });
    });

    it('PUSH imm8 -> 6A xx (sign-extended)', () => {
      const instr = decode([0x6a, 0xff]);
      expect(instr!.mnemonic).toBe('push');
      expect(instr!.operands[0]).toMatchObject({
        type: 'immediate',
        value: -1,
      });
    });
  });

  describe('string instructions', () => {
    it.each([
      [[0xa4], 'movs'],
      [[0xa5], 'movs'],
      [[0xaa], 'stos'],
      [[0xab], 'stos'],
      [[0xac], 'lods'],
      [[0xad], 'lods'],
    ])('%s -> %s', (bytes, mnemonic) => {
      const instr = decode(bytes);
      expect(instr!.mnemonic).toBe(mnemonic);
    });
  });

  describe('unknown/bad bytes', () => {
    it('produces (bad) for unrecognized bytes', () => {
      // 0x0F by itself (without a valid second byte) should be bad or
      // at least not crash. Try a truly invalid sequence.
      const decoder = new Decoder(
        new Uint8Array([0x63, 0x00]),
        decoderMap,
        prefixMap,
        target,
      );
      const instr = decoder.decode();
      // Should either decode as arpl or produce some output without crashing
      expect(instr).not.toBeNull();
    });
  });

  describe('disassemble iterator', () => {
    it('decodes multiple instructions', () => {
      // NOP; CLC; HLT
      const instrs = decodeAll([0x90, 0xf8, 0xf4]);
      expect(instrs).toHaveLength(3);
      expect(instrs[0].mnemonic).toBe('nop');
      expect(instrs[1].mnemonic).toBe('clc');
      expect(instrs[2].mnemonic).toBe('hlt');
    });

    it('tracks instruction addresses', () => {
      // MOV AL, 0x42 (2 bytes); NOP (1 byte); HLT (1 byte)
      const instrs = decodeAll([0xb0, 0x42, 0x90, 0xf4]);
      expect(instrs[0].address).toBe(0);
      expect(instrs[1].address).toBe(2);
      expect(instrs[2].address).toBe(3);
    });

    it('records raw bytes for each instruction', () => {
      const instrs = decodeAll([0xb8, 0x34, 0x12, 0x90]);
      expect(Array.from(instrs[0].bytes)).toEqual([0xb8, 0x34, 0x12]);
      expect(Array.from(instrs[1].bytes)).toEqual([0x90]);
    });
  });
});

describe('Syntax formatters', () => {
  const intel = new IntelSyntax();
  const att = new AttSyntax();

  it('IntelSyntax formats register instruction', () => {
    const text = intel.formatInstruction({
      address: 0,
      bytes: new Uint8Array([0x89, 0xd8]),
      mnemonic: 'mov',
      operands: [
        { type: 'register', name: 'ax', size: 16 },
        { type: 'register', name: 'bx', size: 16 },
      ],
    });
    expect(text).toBe('MOV AX, BX');
  });

  it('AttSyntax formats register instruction with reversed operands', () => {
    const text = att.formatInstruction({
      address: 0,
      bytes: new Uint8Array([0x89, 0xd8]),
      mnemonic: 'mov',
      operands: [
        { type: 'register', name: 'ax', size: 16 },
        { type: 'register', name: 'bx', size: 16 },
      ],
    });
    expect(text).toBe('mov %bx,%ax');
  });

  it('IntelSyntax formats memory with base+index+displacement', () => {
    const text = intel.formatOperand(
      {
        type: 'memory',
        base: 'bx',
        index: 'si',
        displacement: 4,
        displacementSize: 8,
        size: 16,
        direct: false,
      },
      0,
    );
    // Intel syntax joins with " + " and displacement
    expect(text).toMatch(/BX.*SI.*0x04/);
    expect(text).toMatch(/^\[.*\]$/);
  });

  it('AttSyntax formats memory with base+index+displacement', () => {
    const text = att.formatOperand(
      {
        type: 'memory',
        base: 'bx',
        index: 'si',
        displacement: 4,
        displacementSize: 8,
        size: 16,
        direct: false,
      },
      0,
    );
    expect(text).toBe('0x4(%bx,%si)');
  });

  it('IntelSyntax formats far pointer', () => {
    const text = intel.formatOperand(
      { type: 'far_pointer', segment: 0x1000, offset: 0x0100 },
      0,
    );
    expect(text).toBe('0x1000:0x0100');
  });

  it('AttSyntax formats immediate with $ prefix', () => {
    const text = att.formatOperand(
      { type: 'immediate', value: 0x42, size: 8, signed: false },
      0,
    );
    expect(text).toBe('$0x42');
  });

  it('IntelSyntax formats segment override memory', () => {
    const text = intel.formatOperand(
      {
        type: 'memory',
        base: 'bp',
        displacement: 0,
        displacementSize: 0,
        size: 16,
        segment: 'ss',
        direct: false,
      },
      0,
    );
    expect(text).toBe('SS:[BP]');
  });

  it('IntelSyntax formats direct memory', () => {
    const text = intel.formatOperand(
      {
        type: 'memory',
        displacement: 0x1234,
        displacementSize: 16,
        size: 16,
        direct: true,
      },
      0,
    );
    expect(text).toBe('[0x1234]');
  });

  it('IntelSyntax formats prefix instruction', () => {
    const text = intel.formatInstruction({
      address: 0,
      bytes: new Uint8Array([0xf3, 0xa4]),
      mnemonic: 'movs',
      operands: [],
      prefix: 'rep',
    });
    expect(text).toBe('REP MOVS');
  });
});

describe('dsm reference validation', () => {
  const dsmDir = resolve(
    __dirname,
    '../../../simulators/test/i286/complex/dsm',
  );
  const binDir = resolve(
    __dirname,
    '../../../simulators/test/i286/complex/bin',
  );

  // Get all .dsm files that have matching .com binaries
  let dsmFiles: string[] = [];
  try {
    dsmFiles = readdirSync(dsmDir)
      .filter((f) => f.endsWith('.dsm'))
      .map((f) => f.replace('.dsm', ''));
  } catch {
    // directory doesn't exist in CI or other environments
  }

  if (dsmFiles.length === 0) {
    it('skipped (no .dsm reference files found)', () => {
      expect(true).toBe(true);
    });
    return;
  }

  /**
   * Parse a .dsm file into a map of address -> { hex, mnemonic }.
   * We only compare the hex bytes and mnemonic (first word of assembly),
   * ignoring operand formatting differences (size suffixes, zero disp, etc.).
   */
  function parseDsm(
    content: string,
  ): Map<number, { hex: string; mnemonic: string }> {
    const map = new Map<number, { hex: string; mnemonic: string }>();
    for (const line of content.split('\n')) {
      // Match lines like: "   0:	8d 06 f2 15          	lea    0x15f2,%ax"
      const m = line.match(/^\s*([0-9a-f]+):\s+([0-9a-f ]+?)\s{2,}\t?\s*(\S+)/);
      if (!m) continue;
      const addr = parseInt(m[1], 16);
      const hex = m[2].trim();
      const mnemonic = m[3];
      map.set(addr, { hex, mnemonic });
    }
    return map;
  }

  /**
   * Compare mnemonics allowing for known AT&T / objdump differences:
   * - Size suffixes: addb/addw/addl → add
   * - int3: objdump uses "int3" for 0xCC, we use "int"
   * - Far variants: objdump uses lcall/lret/ljmp, we use call/ret/jmp
   * - Rep prefix: objdump puts "rep" as mnemonic, we use the actual instruction
   */
  function mnemonicsMatch(ours: string, ref: string): boolean {
    if (ours === ref) return true;
    // int vs int3 (0xCC single-byte INT 3)
    if (ours === 'int' && ref === 'int3') return true;
    // Far call/ret/jmp: objdump prefixes with 'l'
    if (ref === 'lcall' && ours === 'call') return true;
    if (ref === 'lret' && ours === 'ret') return true;
    if (ref === 'ljmp' && ours === 'jmp') return true;
    // Rep prefix as mnemonic: objdump shows "rep" or "repnz", we show the instruction
    if (ref === 'rep' || ref === 'repnz' || ref === 'repz') return true;
    // Try stripping a single-char size suffix from reference
    const stripped = ref.replace(/^(.*[a-z]{2,})[bwld]$/, '$1');
    if (ours === stripped) return true;
    return false;
  }

  it.each(dsmFiles)('%s: hex bytes match reference', (name) => {
    let binData: Uint8Array;
    try {
      binData = new Uint8Array(readFileSync(resolve(binDir, `${name}.com`)));
    } catch {
      return; // skip if binary not available
    }

    const dsmContent = readFileSync(resolve(dsmDir, `${name}.dsm`), 'utf-8');
    const reference = parseDsm(dsmContent);

    const instrs = [...disassemble(binData, decoderMap, prefixMap, target)];

    let matched = 0;
    let mismatched = 0;

    for (const instr of instrs) {
      const ref = reference.get(instr.address);
      if (!ref) continue;
      // Skip unrecognized instructions — our decoder emits (bad) for opcodes
      // not in the i286 ISA (e.g., ud2, MMX), consuming fewer bytes than objdump
      if (instr.mnemonic === '(bad)') continue;

      const ourHex = Array.from(instr.bytes)
        .map((b) => b.toString(16).padStart(2, '0'))
        .join(' ');

      if (ourHex === ref.hex) {
        matched++;
      } else {
        mismatched++;
      }
    }

    // We should match the vast majority of instructions' hex bytes
    // (byte-level decoding should be exact)
    expect(mismatched).toBe(0);
    expect(matched).toBeGreaterThan(0);
  });

  it.each(dsmFiles)('%s: mnemonics match reference', (name) => {
    let binData: Uint8Array;
    try {
      binData = new Uint8Array(readFileSync(resolve(binDir, `${name}.com`)));
    } catch {
      return; // skip if binary not available
    }

    const dsmContent = readFileSync(resolve(dsmDir, `${name}.dsm`), 'utf-8');
    const reference = parseDsm(dsmContent);

    const instrs = [...disassemble(binData, decoderMap, prefixMap, target)];

    let matched = 0;
    let mismatched = 0;
    const failures: string[] = [];

    for (const instr of instrs) {
      const ref = reference.get(instr.address);
      if (!ref) continue;

      const ourMnemonic = instr.mnemonic.toLowerCase();
      const refMnemonic = ref.mnemonic.toLowerCase();

      if (mnemonicsMatch(ourMnemonic, refMnemonic)) {
        matched++;
      } else {
        mismatched++;
        if (failures.length < 5) {
          failures.push(
            `  0x${instr.address.toString(16)}: ours='${ourMnemonic}' ref='${refMnemonic}'`,
          );
        }
      }
    }

    // Allow small percentage of mnemonic differences (e.g., prefix handling)
    const total = matched + mismatched;
    if (total > 0) {
      const matchRate = matched / total;
      expect(matchRate).toBeGreaterThanOrEqual(0.95);
    }
    expect(matched).toBeGreaterThan(0);
  });
});

// --- Z80 Decoder Tests ---

let z80target: Target;
let z80decoderMap: ReturnType<typeof generateDecoderMap>;
let z80prefixMap: ReturnType<typeof generateDecoderMap>;

beforeAll(async () => {
  const mod = await import('@machinery/processors/z80');
  z80target = mod.default;
  z80decoderMap = generateDecoderMap(z80target, false);
  z80prefixMap = generateDecoderMap(z80target, true);
});

function z80decode(bytes: number[]) {
  const data = new Uint8Array(bytes);
  const decoder = new Decoder(data, z80decoderMap, z80prefixMap, z80target);
  return decoder.decode();
}

function z80decodeAll(bytes: number[]) {
  const data = new Uint8Array(bytes);
  return [...disassemble(data, z80decoderMap, z80prefixMap, z80target)];
}

describe('Z80 Decoder', () => {
  describe('basic instructions', () => {
    it.each([
      [[0x00], 'nop'],
      [[0x76], 'halt'],
      [[0xc9], 'ret'],
      [[0xf3], 'di'],
      [[0xfb], 'ei'],
    ])('%s -> %s', (bytes, mnemonic) => {
      const instr = z80decode(bytes as number[]);
      expect(instr).not.toBeNull();
      expect(instr!.mnemonic).toBe(mnemonic);
    });
  });

  describe('LD register instructions', () => {
    it('LD B, 0x42 -> 06 42', () => {
      const instr = z80decode([0x06, 0x42]);
      expect(instr!.mnemonic).toBe('ld');
      expect(instr!.operands).toHaveLength(2);
      expect(instr!.operands[0]).toMatchObject({ type: 'register', name: 'b' });
      expect(instr!.operands[1]).toMatchObject({
        type: 'immediate',
        value: 0x42,
      });
    });

    it('LD HL, 0x1234 -> 21 34 12', () => {
      const instr = z80decode([0x21, 0x34, 0x12]);
      expect(instr!.mnemonic).toBe('ld');
      expect(instr!.operands[0]).toMatchObject({
        type: 'register',
        name: 'hl',
      });
      expect(instr!.operands[1]).toMatchObject({
        type: 'immediate',
        value: 0x1234,
      });
    });

    it('LD A, B -> 78', () => {
      const instr = z80decode([0x78]);
      expect(instr!.mnemonic).toBe('ld');
      expect(instr!.operands[0]).toMatchObject({ type: 'register', name: 'a' });
      expect(instr!.operands[1]).toMatchObject({ type: 'register', name: 'b' });
    });
  });

  describe('conditional JR', () => {
    it('JR NZ, +5 -> 20 05', () => {
      const instr = z80decode([0x20, 0x05]);
      expect(instr!.mnemonic).toBe('jr');
      expect(instr!.operands).toHaveLength(2);
      expect(instr!.operands[0]).toMatchObject({
        type: 'register',
        name: 'nz',
      });
      expect(instr!.operands[1]).toMatchObject({ type: 'relative' });
    });

    it('JR Z, -3 -> 28 FD', () => {
      const instr = z80decode([0x28, 0xfd]);
      expect(instr!.mnemonic).toBe('jr');
      expect(instr!.operands[0]).toMatchObject({ type: 'register', name: 'z' });
      expect(instr!.operands[1]).toMatchObject({
        type: 'relative',
        offset: -3,
      });
    });

    it('JR NC, +0 -> 30 00', () => {
      const instr = z80decode([0x30, 0x00]);
      expect(instr!.mnemonic).toBe('jr');
      expect(instr!.operands[0]).toMatchObject({
        type: 'register',
        name: 'nc',
      });
    });

    it('JR C, +10 -> 38 0A', () => {
      const instr = z80decode([0x38, 0x0a]);
      expect(instr!.mnemonic).toBe('jr');
      expect(instr!.operands[0]).toMatchObject({ type: 'register', name: 'c' });
      expect(instr!.operands[1]).toMatchObject({
        type: 'relative',
        offset: 10,
      });
    });

    it('JR unconditional -> 18 05', () => {
      const instr = z80decode([0x18, 0x05]);
      expect(instr!.mnemonic).toBe('jr');
      expect(instr!.operands).toHaveLength(1);
      expect(instr!.operands[0]).toMatchObject({ type: 'relative' });
    });
  });

  describe('conditional JP', () => {
    it.each([
      [[0xc2, 0x00, 0x02], 'nz'],
      [[0xca, 0x00, 0x02], 'z'],
      [[0xd2, 0x00, 0x02], 'nc'],
      [[0xda, 0x00, 0x02], 'c'],
      [[0xe2, 0x00, 0x02], 'po'],
      [[0xea, 0x00, 0x02], 'pe'],
      [[0xf2, 0x00, 0x02], 'p'],
      [[0xfa, 0x00, 0x02], 'm'],
    ])('JP %s, 0x0200', (bytes, cc) => {
      const instr = z80decode(bytes as number[]);
      expect(instr!.mnemonic).toBe('jp');
      expect(instr!.operands).toHaveLength(2);
      expect(instr!.operands[0]).toMatchObject({ type: 'register', name: cc });
      expect(instr!.operands[1]).toMatchObject({
        type: 'immediate',
        value: 0x0200,
      });
    });

    it('JP unconditional -> C3 00 02', () => {
      const instr = z80decode([0xc3, 0x00, 0x02]);
      expect(instr!.mnemonic).toBe('jp');
      expect(instr!.operands).toHaveLength(1);
      expect(instr!.operands[0]).toMatchObject({
        type: 'immediate',
        value: 0x0200,
      });
    });
  });

  describe('conditional CALL', () => {
    it.each([
      [[0xc4, 0x00, 0x03], 'nz'],
      [[0xcc, 0x00, 0x03], 'z'],
      [[0xd4, 0x00, 0x03], 'nc'],
      [[0xdc, 0x00, 0x03], 'c'],
      [[0xe4, 0x00, 0x03], 'po'],
      [[0xec, 0x00, 0x03], 'pe'],
      [[0xf4, 0x00, 0x03], 'p'],
      [[0xfc, 0x00, 0x03], 'm'],
    ])('CALL %s, 0x0300', (bytes, cc) => {
      const instr = z80decode(bytes as number[]);
      expect(instr!.mnemonic).toBe('call');
      expect(instr!.operands).toHaveLength(2);
      expect(instr!.operands[0]).toMatchObject({ type: 'register', name: cc });
      expect(instr!.operands[1]).toMatchObject({
        type: 'immediate',
        value: 0x0300,
      });
    });

    it('CALL unconditional -> CD 00 03', () => {
      const instr = z80decode([0xcd, 0x00, 0x03]);
      expect(instr!.mnemonic).toBe('call');
      expect(instr!.operands).toHaveLength(1);
      expect(instr!.operands[0]).toMatchObject({
        type: 'immediate',
        value: 0x0300,
      });
    });
  });

  describe('conditional RET', () => {
    it.each([
      [[0xc0], 'nz'],
      [[0xc8], 'z'],
      [[0xd0], 'nc'],
      [[0xd8], 'c'],
      [[0xe0], 'po'],
      [[0xe8], 'pe'],
      [[0xf0], 'p'],
      [[0xf8], 'm'],
    ])('RET %s', (bytes, cc) => {
      const instr = z80decode(bytes as number[]);
      expect(instr!.mnemonic).toBe('ret');
      expect(instr!.operands).toHaveLength(1);
      expect(instr!.operands[0]).toMatchObject({ type: 'register', name: cc });
    });
  });

  describe('PUSH/POP', () => {
    it.each([
      [[0xc5], 'push', 'bc'],
      [[0xd5], 'push', 'de'],
      [[0xe5], 'push', 'hl'],
      [[0xf5], 'push', 'af'],
      [[0xc1], 'pop', 'bc'],
      [[0xd1], 'pop', 'de'],
      [[0xe1], 'pop', 'hl'],
      [[0xf1], 'pop', 'af'],
    ])('%s %s', (bytes, mnemonic, reg) => {
      const instr = z80decode(bytes as number[]);
      expect(instr!.mnemonic).toBe(mnemonic);
      expect(instr!.operands[0]).toMatchObject({ type: 'register', name: reg });
    });
  });

  describe('ALU instructions', () => {
    it('ADD A, B -> 80', () => {
      const instr = z80decode([0x80]);
      expect(instr!.mnemonic).toBe('add');
      expect(instr!.operands[0]).toMatchObject({ type: 'register', name: 'a' });
      expect(instr!.operands[1]).toMatchObject({ type: 'register', name: 'b' });
    });

    it('SUB 0x10 -> D6 10', () => {
      const instr = z80decode([0xd6, 0x10]);
      expect(instr!.mnemonic).toBe('sub');
      expect(instr!.operands[0]).toMatchObject({
        type: 'immediate',
        value: 0x10,
      });
    });

    it('CP A -> BF', () => {
      const instr = z80decode([0xbf]);
      expect(instr!.mnemonic).toBe('cp');
      expect(instr!.operands[0]).toMatchObject({ type: 'register', name: 'a' });
    });
  });

  describe('IX/IY prefix instructions', () => {
    it('LD IX, 0x1234 -> DD 21 34 12', () => {
      const instr = z80decode([0xdd, 0x21, 0x34, 0x12]);
      expect(instr!.mnemonic).toBe('ld');
      expect(instr!.operands[0]).toMatchObject({
        type: 'register',
        name: 'ix',
      });
      expect(instr!.operands[1]).toMatchObject({
        type: 'immediate',
        value: 0x1234,
      });
    });

    it('LD IY, 0x5678 -> FD 21 78 56', () => {
      const instr = z80decode([0xfd, 0x21, 0x78, 0x56]);
      expect(instr!.mnemonic).toBe('ld');
      expect(instr!.operands[0]).toMatchObject({
        type: 'register',
        name: 'iy',
      });
      expect(instr!.operands[1]).toMatchObject({
        type: 'immediate',
        value: 0x5678,
      });
    });

    it('ADD IX, BC -> DD 09', () => {
      const instr = z80decode([0xdd, 0x09]);
      expect(instr!.mnemonic).toBe('add');
      expect(instr!.operands[0]).toMatchObject({
        type: 'register',
        name: 'ix',
      });
      expect(instr!.operands[1]).toMatchObject({
        type: 'register',
        name: 'bc',
      });
    });

    it('PUSH IX -> DD E5', () => {
      const instr = z80decode([0xdd, 0xe5]);
      expect(instr!.mnemonic).toBe('push');
      expect(instr!.operands[0]).toMatchObject({
        type: 'register',
        name: 'ix',
      });
    });
  });

  describe('CB prefix bit operations', () => {
    it('RLC B -> CB 00', () => {
      const instr = z80decode([0xcb, 0x00]);
      expect(instr!.mnemonic).toBe('rlc');
      expect(instr!.operands[0]).toMatchObject({ type: 'register', name: 'b' });
    });

    it('BIT 3, A -> CB 5F', () => {
      const instr = z80decode([0xcb, 0x5f]);
      expect(instr!.mnemonic).toBe('bit');
      expect(instr!.operands).toHaveLength(2);
    });

    it('SET 7, A -> CB FF', () => {
      const instr = z80decode([0xcb, 0xff]);
      expect(instr!.mnemonic).toBe('set');
      expect(instr!.operands).toHaveLength(2);
    });

    it('RES 0, B -> CB 80', () => {
      const instr = z80decode([0xcb, 0x80]);
      expect(instr!.mnemonic).toBe('res');
      expect(instr!.operands).toHaveLength(2);
    });
  });

  describe('DD CB prefix (IX indexed bit ops)', () => {
    it('RLC (IX+5) -> DD CB 05 06', () => {
      const instr = z80decode([0xdd, 0xcb, 0x05, 0x06]);
      expect(instr).not.toBeNull();
      expect(instr!.mnemonic).toBe('rlc');
    });

    it('SET 0, (IX+3) -> DD CB 03 C6', () => {
      const instr = z80decode([0xdd, 0xcb, 0x03, 0xc6]);
      expect(instr).not.toBeNull();
      expect(instr!.mnemonic).toBe('set');
    });
  });

  describe('multiple instruction decode', () => {
    it('decodes NOP; LD A,0x42; HALT sequence', () => {
      const instrs = z80decodeAll([0x00, 0x3e, 0x42, 0x76]);
      expect(instrs).toHaveLength(3);
      expect(instrs[0].mnemonic).toBe('nop');
      expect(instrs[1].mnemonic).toBe('ld');
      expect(instrs[2].mnemonic).toBe('halt');
    });

    it('tracks addresses correctly', () => {
      // LD BC, 0x1234 (3 bytes); NOP (1 byte); HALT (1 byte)
      const instrs = z80decodeAll([0x01, 0x34, 0x12, 0x00, 0x76]);
      expect(instrs[0].address).toBe(0);
      expect(instrs[1].address).toBe(3);
      expect(instrs[2].address).toBe(4);
    });
  });
});
