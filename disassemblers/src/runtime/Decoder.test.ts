import { Decoder, disassemble } from './Decoder';
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
  });

  describe('segment override prefix', () => {
    it('MOV AX, SS:[BP+0] -> 36 8B 46 00', () => {
      const instr = decode([0x36, 0x8b, 0x46, 0x00]);
      expect(instr!.mnemonic).toBe('mov');
      expect(instr!.segmentOverride).toBe('ss');
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
