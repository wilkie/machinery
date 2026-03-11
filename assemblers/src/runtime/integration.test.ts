import { Assembler } from './Assembler.js';
import {
  operandDefinitions,
  encoderForms,
  segmentOverridePrefixes,
  prefixBytes,
  endianness,
  alignmentFill,
} from '../i286/intel/encoder.js';
import type { ASTNode } from './ast.js';

function hex(bytes: Uint8Array): string {
  return Array.from(bytes)
    .map((b) => b.toString(16).padStart(2, '0').toUpperCase())
    .join(' ');
}

function createAssembler(): Assembler {
  return new Assembler(
    { parse: (input: string) => JSON.parse(input) },
    operandDefinitions,
    encoderForms,
    segmentOverridePrefixes,
    prefixBytes,
    endianness,
    alignmentFill,
  );
}

describe('Integration: full program assembly', () => {
  const assembler = createAssembler();

  it('assembles a minimal COM program (MOV AX, 4C00h; INT 21h)', () => {
    const result = assembler.assembleAST([
      { type: 'directive', name: 'ORG', args: [0x100] },
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
    expect(result.origin).toBe(0x100);
    expect(hex(result.binary)).toBe('B8 00 4C CD 21');
  });

  it('assembles JMP SHORT forward over data', () => {
    // JMP SHORT skip ; +2 for JMP itself, +3 for DB
    // DB 0xAA, 0xBB, 0xCC
    // skip: NOP
    const ast: ASTNode[] = [
      {
        type: 'instruction',
        mnemonic: 'JMP',
        operands: [{ type: 'immediate', value: 'skip', short: true }],
      },
      { type: 'directive', name: 'DB', args: [0xaa, 0xbb, 0xcc] },
      { type: 'label', name: 'skip' },
      { type: 'instruction', mnemonic: 'NOP', operands: [] },
    ];
    const result = assembler.assembleAST(ast);
    // JMP SHORT +3 -> EB 03, then AA BB CC, then 90
    expect(hex(result.binary)).toBe('EB 03 AA BB CC 90');
  });

  it('assembles backward jump (infinite loop)', () => {
    const ast: ASTNode[] = [
      { type: 'label', name: 'loop' },
      {
        type: 'instruction',
        mnemonic: 'NOP',
        operands: [],
      },
      {
        type: 'instruction',
        mnemonic: 'JMP',
        operands: [{ type: 'immediate', value: 'loop', short: true }],
      },
    ];
    const result = assembler.assembleAST(ast);
    // NOP -> 90, JMP -3 (back to 0 from offset 3) -> EB FD
    expect(hex(result.binary)).toBe('90 EB FD');
  });

  it('assembles CALL with label and RET', () => {
    const ast: ASTNode[] = [
      {
        type: 'instruction',
        mnemonic: 'CALL',
        operands: [{ type: 'immediate', value: 'func' }],
      },
      { type: 'instruction', mnemonic: 'HLT', operands: [] },
      { type: 'label', name: 'func' },
      { type: 'instruction', mnemonic: 'RET', operands: [] },
    ];
    const result = assembler.assembleAST(ast);
    // CALL func (offset 0, size 3, target 4) -> E8 01 00
    // HLT -> F4
    // RET -> C3
    expect(hex(result.binary)).toBe('E8 01 00 F4 C3');
  });

  it('resolves EQU used in an instruction', () => {
    const ast: ASTNode[] = [
      { type: 'directive', name: 'EQU', args: ['SEVEN', 7] },
      {
        type: 'instruction',
        mnemonic: 'MOV',
        operands: [
          { type: 'register', name: 'AL', size: 8 },
          { type: 'immediate', value: 'SEVEN' },
        ],
      },
    ];
    const result = assembler.assembleAST(ast);
    expect(hex(result.binary)).toBe('B0 07');
  });

  it('assembles TIMES directive', () => {
    const ast: ASTNode[] = [
      // TIMES args format: [count, 'DB'|'DW'|'DD', ...values]
      { type: 'directive', name: 'TIMES', args: [4, 'DB', 0xcc] },
    ];
    const result = assembler.assembleAST(ast);
    expect(hex(result.binary)).toBe('CC CC CC CC');
  });

  it('assembles memory operations with displacement', () => {
    const ast: ASTNode[] = [
      {
        type: 'instruction',
        mnemonic: 'MOV',
        operands: [
          { type: 'register', name: 'AX', size: 16 },
          { type: 'memory', base: 'BX', displacement: 4 },
        ],
      },
    ];
    const result = assembler.assembleAST(ast);
    // MOV AX, [BX+4] -> 8B 47 04
    expect(hex(result.binary)).toBe('8B 47 04');
  });

  it('assembles memory operations with segment override', () => {
    const ast: ASTNode[] = [
      {
        type: 'instruction',
        mnemonic: 'MOV',
        operands: [
          { type: 'register', name: 'AL', size: 8 },
          { type: 'memory', base: 'BX', segment: 'ES' },
        ],
      },
    ];
    const result = assembler.assembleAST(ast);
    // ES: MOV AL, [BX] -> 26 8A 07
    expect(hex(result.binary)).toBe('26 8A 07');
  });

  it('assembles LEA', () => {
    const ast: ASTNode[] = [
      {
        type: 'instruction',
        mnemonic: 'LEA',
        operands: [
          { type: 'register', name: 'SI', size: 16 },
          { type: 'memory', base: 'BX', index: 'DI', displacement: 0x10 },
        ],
      },
    ];
    const result = assembler.assembleAST(ast);
    // LEA SI, [BX+DI+0x10] -> 8D 71 10
    expect(hex(result.binary)).toBe('8D 71 10');
  });

  it('assembles MOV to/from segment register', () => {
    const ast: ASTNode[] = [
      {
        type: 'instruction',
        mnemonic: 'MOV',
        operands: [
          { type: 'register', name: 'DS', size: 16 },
          { type: 'register', name: 'AX', size: 16 },
        ],
      },
      {
        type: 'instruction',
        mnemonic: 'MOV',
        operands: [
          { type: 'register', name: 'AX', size: 16 },
          { type: 'register', name: 'ES', size: 16 },
        ],
      },
    ];
    const result = assembler.assembleAST(ast);
    // MOV DS, AX -> 8E D8
    // MOV AX, ES -> 8C C0
    expect(hex(result.binary)).toBe('8E D8 8C C0');
  });

  it('assembles string instructions', () => {
    const ast: ASTNode[] = [
      { type: 'instruction', mnemonic: 'MOVSB', operands: [] },
      { type: 'instruction', mnemonic: 'MOVSW', operands: [] },
      { type: 'instruction', mnemonic: 'STOSB', operands: [] },
      { type: 'instruction', mnemonic: 'STOSW', operands: [] },
      { type: 'instruction', mnemonic: 'LODSB', operands: [] },
      { type: 'instruction', mnemonic: 'LODSW', operands: [] },
    ];
    const result = assembler.assembleAST(ast);
    expect(hex(result.binary)).toBe('A4 A5 AA AB AC AD');
  });

  it('assembles conditional jumps', () => {
    const ast: ASTNode[] = [
      {
        type: 'instruction',
        mnemonic: 'CMP',
        operands: [
          { type: 'register', name: 'AX', size: 16 },
          { type: 'immediate', value: 0 },
        ],
      },
      {
        type: 'instruction',
        mnemonic: 'JZ',
        operands: [{ type: 'immediate', value: 'done' }],
      },
      { type: 'instruction', mnemonic: 'NOP', operands: [] },
      { type: 'label', name: 'done' },
      { type: 'instruction', mnemonic: 'HLT', operands: [] },
    ];
    const result = assembler.assembleAST(ast);
    // CMP AX, 0 -> 3D 00 00 (or 83 F8 00)
    // JZ done (skip 1 byte NOP) -> 74 01
    // NOP -> 90
    // HLT -> F4
    const bytes = Array.from(result.binary);
    // Find JZ opcode (0x74) and check the displacement
    const jzIdx = bytes.indexOf(0x74);
    expect(jzIdx).toBeGreaterThan(0);
    expect(bytes[jzIdx + 1]).toBe(0x01); // skip 1 byte (NOP)
    // Last two bytes should be NOP, HLT
    expect(bytes[bytes.length - 2]).toBe(0x90);
    expect(bytes[bytes.length - 1]).toBe(0xf4);
  });

  it('assembles PUSH/POP sequence', () => {
    const ast: ASTNode[] = [
      {
        type: 'instruction',
        mnemonic: 'PUSH',
        operands: [{ type: 'register', name: 'AX', size: 16 }],
      },
      {
        type: 'instruction',
        mnemonic: 'PUSH',
        operands: [{ type: 'register', name: 'BX', size: 16 }],
      },
      {
        type: 'instruction',
        mnemonic: 'POP',
        operands: [{ type: 'register', name: 'BX', size: 16 }],
      },
      {
        type: 'instruction',
        mnemonic: 'POP',
        operands: [{ type: 'register', name: 'AX', size: 16 }],
      },
    ];
    const result = assembler.assembleAST(ast);
    expect(hex(result.binary)).toBe('50 53 5B 58');
  });

  it('assembles DB with mixed strings and bytes', () => {
    const ast: ASTNode[] = [
      { type: 'directive', name: 'DB', args: ['AB', 0x00] },
    ];
    const result = assembler.assembleAST(ast);
    // 'A' = 0x41, 'B' = 0x42, null terminator
    expect(hex(result.binary)).toBe('41 42 00');
  });

  it('assembles DW with label reference', () => {
    const ast: ASTNode[] = [
      { type: 'directive', name: 'ORG', args: [0x100] },
      { type: 'instruction', mnemonic: 'NOP', operands: [] },
      { type: 'label', name: 'data' },
      { type: 'directive', name: 'DW', args: ['data'] },
    ];
    const result = assembler.assembleAST(ast);
    // NOP -> 90
    // data label at 0x101
    // DW data -> 01 01 (little-endian 0x0101)
    expect(hex(result.binary)).toBe('90 01 01');
  });

  it('label math in immediates', () => {
    const ast: ASTNode[] = [
      { type: 'directive', name: 'ORG', args: [0x100] },
      { type: 'label', name: 'start' },
      {
        type: 'instruction',
        mnemonic: 'MOV',
        operands: [
          { type: 'register', name: 'AX', size: 16 },
          {
            type: 'immediate',
            value: {
              type: 'expression',
              op: '+',
              left: 'start',
              right: 4,
            },
          },
        ],
      },
    ];
    const result = assembler.assembleAST(ast);
    // start = 0x100, start+4 = 0x104
    // MOV AX, 0x0104 -> B8 04 01
    expect(hex(result.binary)).toBe('B8 04 01');
  });
});
