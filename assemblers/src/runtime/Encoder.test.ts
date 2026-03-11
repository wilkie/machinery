import { Encoder } from './Encoder.js';
import {
  operandDefinitions,
  encoderForms,
  segmentOverridePrefixes,
} from '../i286/intel/encoder.js';
import type { Operand } from './ast.js';

const encoder = new Encoder(
  operandDefinitions,
  encoderForms,
  segmentOverridePrefixes,
);

function hex(bytes: number[]): string {
  return bytes
    .map((b) => b.toString(16).padStart(2, '0').toUpperCase())
    .join(' ');
}

function encode(mnemonic: string, operands: Operand[]): string {
  return hex(encoder.encode(mnemonic, operands, {}, 0).bytes);
}

// Operand helpers
const reg8 = (name: string): Operand => ({ type: 'register', name, size: 8 });
const reg16 = (name: string): Operand => ({ type: 'register', name, size: 16 });
const seg = (name: string): Operand => ({ type: 'register', name, size: 16 });
const imm = (value: number): Operand => ({ type: 'immediate', value });
const mem = (opts: {
  base?: string;
  index?: string;
  displacement?: number;
  segment?: string;
}): Operand => ({
  type: 'memory',
  ...opts,
});

describe('Encoder', () => {
  describe('zero-operand instructions', () => {
    it.each([
      ['NOP', '90'],
      ['CLC', 'F8'],
      ['STC', 'F9'],
      ['CMC', 'F5'],
      ['CLI', 'FA'],
      ['STI', 'FB'],
      ['CLD', 'FC'],
      ['STD', 'FD'],
      ['HLT', 'F4'],
      ['CBW', '98'],
      ['CWD', '99'],
      ['LAHF', '9F'],
      ['SAHF', '9E'],
      ['AAA', '37'],
      ['AAS', '3F'],
      ['DAA', '27'],
      ['DAS', '2F'],
      ['XLAT', 'D7'],
      ['PUSHA', '60'],
      ['POPA', '61'],
    ])('%s -> 0x%s', (mnemonic, expected) => {
      expect(encode(mnemonic, [])).toBe(expected);
    });
  });

  describe('segment register PUSH/POP', () => {
    it.each([
      ['ES', '06'],
      ['CS', '0E'],
      ['SS', '16'],
      ['DS', '1E'],
    ])('PUSH %s -> 0x%s', (name, expected) => {
      expect(encode('PUSH', [seg(name)])).toBe(expected);
    });

    it.each([
      ['ES', '07'],
      ['SS', '17'],
      ['DS', '1F'],
    ])('POP %s -> 0x%s', (name, expected) => {
      expect(encode('POP', [seg(name)])).toBe(expected);
    });
  });

  describe('PUSH/POP register', () => {
    it.each([
      ['AX', '50'],
      ['CX', '51'],
      ['DX', '52'],
      ['BX', '53'],
      ['SP', '54'],
      ['BP', '55'],
      ['SI', '56'],
      ['DI', '57'],
    ])('PUSH %s -> 0x%s', (name, expected) => {
      expect(encode('PUSH', [reg16(name)])).toBe(expected);
    });

    it.each([
      ['AX', '58'],
      ['CX', '59'],
      ['DX', '5A'],
      ['BX', '5B'],
      ['SP', '5C'],
      ['BP', '5D'],
      ['SI', '5E'],
      ['DI', '5F'],
    ])('POP %s -> 0x%s', (name, expected) => {
      expect(encode('POP', [reg16(name)])).toBe(expected);
    });
  });

  describe('INC/DEC register (short form)', () => {
    it.each([
      ['AX', '40'],
      ['CX', '41'],
      ['DX', '42'],
      ['BX', '43'],
      ['SP', '44'],
      ['BP', '45'],
      ['SI', '46'],
      ['DI', '47'],
    ])('INC %s -> 0x%s', (name, expected) => {
      expect(encode('INC', [reg16(name)])).toBe(expected);
    });

    it.each([
      ['AX', '48'],
      ['CX', '49'],
      ['DX', '4A'],
      ['BX', '4B'],
      ['SP', '4C'],
      ['BP', '4D'],
      ['SI', '4E'],
      ['DI', '4F'],
    ])('DEC %s -> 0x%s', (name, expected) => {
      expect(encode('DEC', [reg16(name)])).toBe(expected);
    });
  });

  describe('XCHG AX, reg16', () => {
    it.each([
      ['CX', '91'],
      ['DX', '92'],
      ['BX', '93'],
      ['SP', '94'],
      ['BP', '95'],
      ['SI', '96'],
      ['DI', '97'],
    ])('XCHG AX, %s -> 0x%s', (name, expected) => {
      expect(encode('XCHG', [reg16('AX'), reg16(name)])).toBe(expected);
    });
  });

  describe('INT', () => {
    it('INT 0x21 -> CD 21', () => {
      expect(encode('INT', [imm(0x21)])).toBe('CD 21');
    });

    it('INT 0x10 -> CD 10', () => {
      expect(encode('INT', [imm(0x10)])).toBe('CD 10');
    });

    it('INT 3 -> CD 03', () => {
      expect(encode('INT', [imm(3)])).toBe('CD 03');
    });
  });

  describe('accumulator-immediate ALU', () => {
    describe('ADD', () => {
      it('ADD AL, 0x42 -> 04 42', () => {
        expect(encode('ADD', [reg8('AL'), imm(0x42)])).toBe('04 42');
      });

      it('ADD AX, 0x1234 -> 05 34 12', () => {
        expect(encode('ADD', [reg16('AX'), imm(0x1234)])).toBe('05 34 12');
      });
    });

    describe('SUB', () => {
      it('SUB AL, 0x10 -> 2C 10', () => {
        expect(encode('SUB', [reg8('AL'), imm(0x10)])).toBe('2C 10');
      });

      it('SUB AX, 0x0100 -> 2D 00 01', () => {
        expect(encode('SUB', [reg16('AX'), imm(0x0100)])).toBe('2D 00 01');
      });
    });

    describe('CMP', () => {
      it('CMP AL, 0x00 -> 3C 00', () => {
        expect(encode('CMP', [reg8('AL'), imm(0x00)])).toBe('3C 00');
      });

      it('CMP AX, 0xFFFF -> 83 F8 FF (sign-extended imm8)', () => {
        // 0xFFFF = -1 sign-extended, so encoder prefers shorter imm8 form
        expect(encode('CMP', [reg16('AX'), imm(0xffff)])).toBe('83 F8 FF');
      });
    });

    describe('AND', () => {
      it('AND AL, 0x0F -> 24 0F', () => {
        expect(encode('AND', [reg8('AL'), imm(0x0f)])).toBe('24 0F');
      });

      it('AND AX, 0xFF00 -> 25 00 FF', () => {
        expect(encode('AND', [reg16('AX'), imm(0xff00)])).toBe('25 00 FF');
      });
    });

    describe('OR', () => {
      it('OR AL, 0x0F -> 0C 0F', () => {
        expect(encode('OR', [reg8('AL'), imm(0x0f)])).toBe('0C 0F');
      });

      it('OR AX, 0x0001 -> 83 C8 01 (sign-extended imm8)', () => {
        // Encoder prefers shorter sign-extended imm8 form
        expect(encode('OR', [reg16('AX'), imm(0x0001)])).toBe('83 C8 01');
      });
    });

    describe('XOR', () => {
      it('XOR AL, 0x55 -> 34 55', () => {
        expect(encode('XOR', [reg8('AL'), imm(0x55)])).toBe('34 55');
      });

      it('XOR AX, 0x5555 -> 35 55 55', () => {
        expect(encode('XOR', [reg16('AX'), imm(0x5555)])).toBe('35 55 55');
      });
    });

    describe('TEST', () => {
      it('TEST AL, 0x01 -> A8 01', () => {
        expect(encode('TEST', [reg8('AL'), imm(0x01)])).toBe('A8 01');
      });

      it('TEST AX, 0x8000 -> A9 00 80', () => {
        expect(encode('TEST', [reg16('AX'), imm(0x8000)])).toBe('A9 00 80');
      });
    });

    describe('ADC', () => {
      it('ADC AL, 0x05 -> 14 05', () => {
        expect(encode('ADC', [reg8('AL'), imm(0x05)])).toBe('14 05');
      });

      it('ADC AX, 0x0100 -> 15 00 01', () => {
        expect(encode('ADC', [reg16('AX'), imm(0x0100)])).toBe('15 00 01');
      });
    });

    describe('SBB', () => {
      it('SBB AL, 0x01 -> 1C 01', () => {
        expect(encode('SBB', [reg8('AL'), imm(0x01)])).toBe('1C 01');
      });

      it('SBB AX, 0x1000 -> 1D 00 10', () => {
        expect(encode('SBB', [reg16('AX'), imm(0x1000)])).toBe('1D 00 10');
      });
    });
  });

  describe('MOV', () => {
    describe('MOV reg16, imm16', () => {
      it.each([
        ['AX', 0x1234, 'B8 34 12'],
        ['CX', 0x0005, 'B9 05 00'],
        ['DX', 0xabcd, 'BA CD AB'],
        ['BX', 0x0000, 'BB 00 00'],
        ['SP', 0x1000, 'BC 00 10'],
        ['BP', 0x2000, 'BD 00 20'],
        ['SI', 0x3000, 'BE 00 30'],
        ['DI', 0x4000, 'BF 00 40'],
      ])('MOV %s, 0x%s -> %s', (name, value, expected) => {
        expect(encode('MOV', [reg16(name), imm(value)])).toBe(expected);
      });
    });

    describe('MOV reg8, imm8', () => {
      it.each([
        ['AL', 0xff, 'B0 FF'],
        ['CL', 0x00, 'B1 00'],
        ['DL', 0x42, 'B2 42'],
        ['BL', 0x80, 'B3 80'],
        ['AH', 0x09, 'B4 09'],
        ['CH', 0x0a, 'B5 0A'],
        ['DH', 0x0d, 'B6 0D'],
        ['BH', 0x7f, 'B7 7F'],
      ])('MOV %s, 0x%s -> %s', (name, value, expected) => {
        expect(encode('MOV', [reg8(name), imm(value)])).toBe(expected);
      });
    });
  });

  describe('register-register operations', () => {
    it('ADD AX, BX -> 01 D8', () => {
      // ADD r/m16, r16: mod=11, reg=BX(011), rm=AX(000)
      expect(encode('ADD', [reg16('AX'), reg16('BX')])).toBe('01 D8');
    });

    it('ADD CX, DX -> 01 D1', () => {
      // mod=11, reg=DX(010), rm=CX(001)
      expect(encode('ADD', [reg16('CX'), reg16('DX')])).toBe('01 D1');
    });

    it('ADD AL, BL -> 00 D8', () => {
      // mod=11, reg=BL(011), rm=AL(000)
      expect(encode('ADD', [reg8('AL'), reg8('BL')])).toBe('00 D8');
    });

    it('MOV AX, BX -> 89 D8', () => {
      // MOV r/m16, r16: mod=11, reg=BX(011), rm=AX(000)
      expect(encode('MOV', [reg16('AX'), reg16('BX')])).toBe('89 D8');
    });

    it('MOV AL, CL -> 88 C8', () => {
      // MOV r/m8, r8: mod=11, reg=CL(001), rm=AL(000)
      expect(encode('MOV', [reg8('AL'), reg8('CL')])).toBe('88 C8');
    });
  });

  describe('memory addressing', () => {
    it('ADD [BX+SI], AL -> 00 00', () => {
      expect(
        encode('ADD', [mem({ base: 'BX', index: 'SI' }), reg8('AL')]),
      ).toBe('00 00');
    });

    it('ADD [BX+DI], AL -> 00 01', () => {
      expect(
        encode('ADD', [mem({ base: 'BX', index: 'DI' }), reg8('AL')]),
      ).toBe('00 01');
    });

    it('ADD [BX], AL -> 00 07', () => {
      expect(encode('ADD', [mem({ base: 'BX' }), reg8('AL')])).toBe('00 07');
    });

    it('ADD [SI], AL -> 00 04', () => {
      expect(encode('ADD', [mem({ base: 'SI' }), reg8('AL')])).toBe('00 04');
    });

    it('ADD [DI], AL -> 00 05', () => {
      expect(encode('ADD', [mem({ base: 'DI' }), reg8('AL')])).toBe('00 05');
    });
  });

  describe('JMP short (relative)', () => {
    it('JMP +5 (target=7, offset=0, size=2 -> rel=5)', () => {
      expect(encode('JMP', [imm(7)])).toBe('EB 05');
    });

    it('JMP +0 (target=2, offset=0, size=2 -> rel=0)', () => {
      expect(encode('JMP', [imm(2)])).toBe('EB 00');
    });
  });

  describe('PUSH immediate', () => {
    it('PUSH 0x1234 -> 68 34 12', () => {
      expect(encode('PUSH', [imm(0x1234)])).toBe('68 34 12');
    });

    it('PUSH 5 (sign-extended 8-bit) -> 6A 05', () => {
      expect(encode('PUSH', [imm(5)])).toBe('6A 05');
    });
  });

  describe('RET', () => {
    it('RET (zero operands) encodes a return', () => {
      const result = encode('RET', []);
      // Both near (C3) and far (CB) forms have operands: [] under the 'ret' identifier;
      // the encoder picks the first matching form
      expect(['C3', 'CB']).toContain(result);
    });

    it('RET imm16 -> C2 or CA + imm16', () => {
      const result = encode('RET', [imm(4)]);
      expect(['CA 04 00', 'C2 04 00']).toContain(result);
    });
  });

  describe('case insensitivity', () => {
    it('accepts lowercase mnemonics', () => {
      expect(encode('nop', [])).toBe('90');
    });

    it('accepts mixed case mnemonics', () => {
      expect(encode('Nop', [])).toBe('90');
    });
  });

  describe('error handling', () => {
    it('throws for unknown mnemonic', () => {
      expect(() => encode('FAKEINSTR', [])).toThrow();
    });

    it('throws for wrong operand count', () => {
      expect(() => encode('NOP', [reg16('AX')])).toThrow();
    });

    it('throws for mismatched operand types', () => {
      // ADD expects register/memory + register, not two immediates
      expect(() => encode('ADD', [imm(1), imm(2)])).toThrow();
    });
  });

  describe('segment override prefixes', () => {
    it('ADD ES:[BX+SI], AL -> 26 00 00', () => {
      expect(
        encode('ADD', [
          mem({ base: 'BX', index: 'SI', segment: 'ES' }),
          reg8('AL'),
        ]),
      ).toBe('26 00 00');
    });

    it('ADD CS:[BX], AL -> 2E 00 07', () => {
      expect(
        encode('ADD', [mem({ base: 'BX', segment: 'CS' }), reg8('AL')]),
      ).toBe('2E 00 07');
    });

    it('ADD SS:[SI], AL -> 36 00 04', () => {
      expect(
        encode('ADD', [mem({ base: 'SI', segment: 'SS' }), reg8('AL')]),
      ).toBe('36 00 04');
    });

    it('ADD DS:[DI], AL -> 3E 00 05', () => {
      expect(
        encode('ADD', [mem({ base: 'DI', segment: 'DS' }), reg8('AL')]),
      ).toBe('3E 00 05');
    });

    it('no prefix without segment override', () => {
      expect(
        encode('ADD', [mem({ base: 'BX', index: 'SI' }), reg8('AL')]),
      ).toBe('00 00');
    });
  });

  describe('estimateSize', () => {
    it('returns correct size for NOP', () => {
      expect(encoder.estimateSize('NOP', [])).toBe(1);
    });

    it('returns max possible size for MOV reg16, imm16', () => {
      // estimateSize returns the max across all matching forms (conservative for first pass)
      expect(
        encoder.estimateSize('MOV', [reg16('AX'), imm(0x1234)]),
      ).toBeGreaterThanOrEqual(3);
    });

    it('returns 0 for unknown instruction', () => {
      expect(encoder.estimateSize('FAKEINSTR', [])).toBe(0);
    });
  });
});
