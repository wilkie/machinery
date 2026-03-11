import { Assembler } from './Assembler.js';
import { operandDefinitions, encoderForms } from '../i286/intel/encoder.js';
import type { Program } from './ast.js';

function hex(bytes: Uint8Array): string {
  return Array.from(bytes)
    .map((b) => b.toString(16).padStart(2, '0').toUpperCase())
    .join(' ');
}

// Simple mock parser that takes a pre-built AST instead of parsing text
function createAssembler(): Assembler {
  return new Assembler(
    { parse: (input: string) => JSON.parse(input) as Program },
    operandDefinitions,
    encoderForms,
  );
}

describe('Assembler', () => {
  const assembler = createAssembler();

  describe('assembleAST', () => {
    it('assembles a single NOP', () => {
      const result = assembler.assembleAST([
        { type: 'instruction', mnemonic: 'NOP', operands: [] },
      ]);
      expect(hex(result.binary)).toBe('90');
    });

    it('assembles multiple instructions', () => {
      const result = assembler.assembleAST([
        { type: 'instruction', mnemonic: 'NOP', operands: [] },
        { type: 'instruction', mnemonic: 'HLT', operands: [] },
      ]);
      expect(hex(result.binary)).toBe('90 F4');
    });

    it('assembles MOV + INT sequence', () => {
      const result = assembler.assembleAST([
        {
          type: 'instruction',
          mnemonic: 'MOV',
          operands: [
            { type: 'register', name: 'AX', size: 16 },
            { type: 'immediate', value: 0x4c00 },
          ],
        },
        {
          type: 'instruction',
          mnemonic: 'INT',
          operands: [{ type: 'immediate', value: 0x21 }],
        },
      ]);
      // MOV AX, 0x4C00 -> B8 00 4C
      // INT 0x21       -> CD 21
      expect(hex(result.binary)).toBe('B8 00 4C CD 21');
    });
  });

  describe('labels', () => {
    it('collects label addresses', () => {
      const result = assembler.assembleAST([
        { type: 'label', name: 'start' },
        { type: 'instruction', mnemonic: 'NOP', operands: [] },
        { type: 'label', name: 'end' },
        { type: 'instruction', mnemonic: 'HLT', operands: [] },
      ]);
      expect(result.labels['start']).toBe(0);
      expect(result.labels['end']).toBe(1);
      expect(hex(result.binary)).toBe('90 F4');
    });

    it('resolves label references in immediates', () => {
      const result = assembler.assembleAST([
        { type: 'label', name: 'data' },
        {
          type: 'instruction',
          mnemonic: 'MOV',
          operands: [
            { type: 'register', name: 'AX', size: 16 },
            { type: 'immediate', value: 'data' },
          ],
        },
      ]);
      // 'data' is at offset 0, so MOV AX, 0x0000 -> B8 00 00
      expect(hex(result.binary)).toBe('B8 00 00');
    });
  });

  describe('directives', () => {
    describe('ORG', () => {
      it('sets the origin address', () => {
        const result = assembler.assembleAST([
          { type: 'directive', name: 'ORG', args: [0x100] },
          { type: 'label', name: 'start' },
          { type: 'instruction', mnemonic: 'NOP', operands: [] },
        ]);
        expect(result.origin).toBe(0x100);
        expect(result.labels['start']).toBe(0x100);
      });
    });

    describe('DB', () => {
      it('emits raw bytes', () => {
        const result = assembler.assembleAST([
          { type: 'directive', name: 'DB', args: [0x55, 0xaa, 0xff] },
        ]);
        expect(hex(result.binary)).toBe('55 AA FF');
      });

      it('emits string characters as bytes', () => {
        const result = assembler.assembleAST([
          { type: 'directive', name: 'DB', args: ['Hi'] },
        ]);
        // 'H' = 0x48, 'i' = 0x69
        expect(hex(result.binary)).toBe('48 69');
      });
    });

    describe('DW', () => {
      it('emits little-endian 16-bit words', () => {
        const result = assembler.assembleAST([
          { type: 'directive', name: 'DW', args: [0x1234, 0x5678] },
        ]);
        expect(hex(result.binary)).toBe('34 12 78 56');
      });
    });

    describe('EQU', () => {
      it('defines constants resolved in operands', () => {
        const result = assembler.assembleAST([
          { type: 'directive', name: 'EQU', args: ['SCREEN', 0xb800] },
          {
            type: 'instruction',
            mnemonic: 'MOV',
            operands: [
              { type: 'register', name: 'AX', size: 16 },
              { type: 'immediate', value: 'SCREEN' },
            ],
          },
        ]);
        // MOV AX, 0xB800 -> B8 00 B8
        expect(hex(result.binary)).toBe('B8 00 B8');
      });
    });
  });

  describe('options', () => {
    it('respects origin from options parameter', () => {
      const result = assembler.assembleAST(
        [
          { type: 'label', name: 'entry' },
          { type: 'instruction', mnemonic: 'NOP', operands: [] },
        ],
        { origin: 0x7c00 },
      );
      expect(result.origin).toBe(0x7c00);
      expect(result.labels['entry']).toBe(0x7c00);
    });
  });

  describe('mixed instructions and data', () => {
    it('assembles a simple program with data', () => {
      const result = assembler.assembleAST([
        // MOV AX, 0x4C00
        {
          type: 'instruction',
          mnemonic: 'MOV',
          operands: [
            { type: 'register', name: 'AX', size: 16 },
            { type: 'immediate', value: 0x4c00 },
          ],
        },
        // INT 0x21
        {
          type: 'instruction',
          mnemonic: 'INT',
          operands: [{ type: 'immediate', value: 0x21 }],
        },
        // DB 'Hello', 0
        { type: 'directive', name: 'DB', args: ['Hello', 0] },
      ]);

      expect(hex(result.binary)).toBe('B8 00 4C CD 21 48 65 6C 6C 6F 00');
    });

    it('tracks label offsets correctly with data directives', () => {
      const result = assembler.assembleAST([
        { type: 'directive', name: 'DB', args: [0x00, 0x00, 0x00] },
        { type: 'label', name: 'code' },
        { type: 'instruction', mnemonic: 'NOP', operands: [] },
      ]);
      expect(result.labels['code']).toBe(3);
    });
  });
});
