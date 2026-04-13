import { readFileSync } from 'node:fs';
import { parse, getDeclarations } from './parse.js';

const readSample = (name: string): string =>
  readFileSync(new URL(`../lexer/__samples__/${name}`, import.meta.url), 'utf-8');

const kinds = (file: ReturnType<typeof parse>['cst']): string[] => {
  if (!file) return [];
  return getDeclarations(file).map((d) => d.kind);
};

const names = (file: ReturnType<typeof parse>['cst']): string[] => {
  if (!file) return [];
  return getDeclarations(file).map((d) => d.name);
};

describe('parser — top-level declaration skeleton', () => {
  describe('happy-path single declarations', () => {
    it('parses a single enum declaration with a body', () => {
      const { cst, lexErrors, parseErrors } = parse('enum Foo\n  a\n  b\n');
      expect(lexErrors).toEqual([]);
      expect(parseErrors).toEqual([]);
      expect(kinds(cst)).toEqual(['enumDecl']);
      expect(names(cst)).toEqual(['Foo']);
    });

    it('parses a single register declaration with a type annotation', () => {
      const { cst, lexErrors, parseErrors } = parse('register AX:u16\n');
      expect(lexErrors).toEqual([]);
      expect(parseErrors).toEqual([]);
      expect(kinds(cst)).toEqual(['registerDecl']);
      expect(names(cst)).toEqual(['AX']);
    });

    it('parses a register declaration with an indented body', () => {
      const src = 'register AX:u16\n  field AL:u8 @ 0\n  field AH:u8 @ 8\n';
      const { cst, lexErrors, parseErrors } = parse(src);
      expect(lexErrors).toEqual([]);
      expect(parseErrors).toEqual([]);
      expect(kinds(cst)).toEqual(['registerDecl']);
      expect(names(cst)).toEqual(['AX']);
    });

    it('parses each top-level keyword as its own declaration kind', () => {
      const src =
        'bundle Foo\n' +
        'union Bar\n' +
        'microword Baz\n' +
        'operand Qux\n' +
        'machine Mach\n';
      const { cst, lexErrors, parseErrors } = parse(src);
      expect(lexErrors).toEqual([]);
      expect(parseErrors).toEqual([]);
      expect(kinds(cst)).toEqual([
        'bundleDecl',
        'unionDecl',
        'microwordDecl',
        'operandDecl',
        'machineDecl',
      ]);
      expect(names(cst)).toEqual(['Foo', 'Bar', 'Baz', 'Qux', 'Mach']);
    });

    it('parses a declaration with no trailing newline', () => {
      // End-of-file without a final newline is the one case where
      // headerTerminator's OPTION bails out entirely.
      const { cst, lexErrors, parseErrors } = parse('register SI:u16');
      expect(lexErrors).toEqual([]);
      expect(parseErrors).toEqual([]);
      expect(kinds(cst)).toEqual(['registerDecl']);
      expect(names(cst)).toEqual(['SI']);
    });
  });

  describe('unit and routine signatures', () => {
    it('parses a unit with no type parameters', () => {
      const { cst, lexErrors, parseErrors } = parse('unit alu\n');
      expect(lexErrors).toEqual([]);
      expect(parseErrors).toEqual([]);
      expect(kinds(cst)).toEqual(['unitDecl']);
      expect(names(cst)).toEqual(['alu']);
    });

    it('parses a unit with a type parameter list', () => {
      const { cst, lexErrors, parseErrors } = parse('unit alu<W:Width>\n');
      expect(lexErrors).toEqual([]);
      expect(parseErrors).toEqual([]);
      expect(kinds(cst)).toEqual(['unitDecl']);
      expect(names(cst)).toEqual(['alu']);
    });

    it('parses a unit with multiple type parameters', () => {
      const { cst, lexErrors, parseErrors } = parse(
        'unit foo<W:Width, T:AluOp>\n',
      );
      expect(lexErrors).toEqual([]);
      expect(parseErrors).toEqual([]);
      expect(kinds(cst)).toEqual(['unitDecl']);
    });

    it('parses a routine with no parameters and no return type', () => {
      const { cst, lexErrors, parseErrors } = parse('routine addRmReg8\n');
      expect(lexErrors).toEqual([]);
      expect(parseErrors).toEqual([]);
      expect(kinds(cst)).toEqual(['routineDecl']);
      expect(names(cst)).toEqual(['addRmReg8']);
    });

    it('parses a routine with parameters and a return type', () => {
      const src = 'routine eaCalc(mod:u2, rm:u3) -> (ea:u16)\n';
      const { cst, lexErrors, parseErrors } = parse(src);
      expect(lexErrors).toEqual([]);
      expect(parseErrors).toEqual([]);
      expect(kinds(cst)).toEqual(['routineDecl']);
      expect(names(cst)).toEqual(['eaCalc']);
    });

    it('parses a routine with an empty return type', () => {
      const { cst, lexErrors, parseErrors } = parse('routine fetchModRm -> ()\n');
      expect(lexErrors).toEqual([]);
      expect(parseErrors).toEqual([]);
      expect(kinds(cst)).toEqual(['routineDecl']);
      expect(names(cst)).toEqual(['fetchModRm']);
    });

    it('parses a routine with only a return type (no param list)', () => {
      const { cst, lexErrors, parseErrors } = parse(
        'routine fetchDispWord -> (disp:u16)\n',
      );
      expect(lexErrors).toEqual([]);
      expect(parseErrors).toEqual([]);
      expect(kinds(cst)).toEqual(['routineDecl']);
    });

    it('parses a routine with only a parameter list (no return type)', () => {
      const { cst, lexErrors, parseErrors } = parse('routine memWrite(addr:u16)\n');
      expect(lexErrors).toEqual([]);
      expect(parseErrors).toEqual([]);
      expect(kinds(cst)).toEqual(['routineDecl']);
    });
  });

  describe('block skipping', () => {
    it('skips over an indented body containing arbitrary tokens', () => {
      const src = [
        'routine foo',
        '  fetch ModRM',
        '  call eaCalc(mod, rm) -> ea',
        '  AluMicro { op: add, width: u8, srcA: mem, srcB: ModRM.reg,',
        '             dest: tmp, writeFlags: 1, commit: onRetire }',
        '  Retire {}',
        '',
      ].join('\n');
      const { cst, lexErrors, parseErrors } = parse(src);
      expect(lexErrors).toEqual([]);
      expect(parseErrors).toEqual([]);
      expect(kinds(cst)).toEqual(['routineDecl']);
      expect(names(cst)).toEqual(['foo']);
    });

    it('skips over deeply nested blocks', () => {
      const src = [
        'unit foo',
        '  mux a',
        '    when 0',
        '      mux b',
        '        when 0',
        '          1',
        '        when 1',
        '          2',
        '    when 1',
        '      3',
        '',
      ].join('\n');
      const { cst, lexErrors, parseErrors } = parse(src);
      expect(lexErrors).toEqual([]);
      expect(parseErrors).toEqual([]);
      expect(kinds(cst)).toEqual(['unitDecl']);
      expect(names(cst)).toEqual(['foo']);
    });

    it('handles blank lines and comment-only lines inside a body', () => {
      const src = [
        'enum Foo',
        '  a',
        '',
        '  ; a comment on its own line',
        '  b',
        '',
      ].join('\n');
      const { cst, lexErrors, parseErrors } = parse(src);
      expect(lexErrors).toEqual([]);
      expect(parseErrors).toEqual([]);
      expect(kinds(cst)).toEqual(['enumDecl']);
    });

    it('handles multiple top-level declarations separated by blank lines', () => {
      const src = [
        'enum Foo',
        '  a',
        '  b',
        '',
        '',
        'register AX:u16',
        '  field AL:u8 @ 0',
        '',
        'machine cpu',
        '  registers',
        '    state:u8 = 0',
        '',
      ].join('\n');
      const { cst, lexErrors, parseErrors } = parse(src);
      expect(lexErrors).toEqual([]);
      expect(parseErrors).toEqual([]);
      expect(kinds(cst)).toEqual(['enumDecl', 'registerDecl', 'machineDecl']);
      expect(names(cst)).toEqual(['Foo', 'AX', 'cpu']);
    });
  });

  describe('sample files', () => {
    it('parses minimal.machine: one enum, one register', () => {
      const { cst, lexErrors, parseErrors } = parse(readSample('minimal.machine'));
      expect(lexErrors).toEqual([]);
      expect(parseErrors).toEqual([]);
      expect(kinds(cst)).toEqual(['enumDecl', 'registerDecl']);
      expect(names(cst)).toEqual(['Foo', 'AX']);
    });

    it('parses nested.machine: one unit with deeply nested body', () => {
      const { cst, lexErrors, parseErrors } = parse(readSample('nested.machine'));
      expect(lexErrors).toEqual([]);
      expect(parseErrors).toEqual([]);
      expect(kinds(cst)).toEqual(['unitDecl']);
      expect(names(cst)).toEqual(['select']);
    });

    it('parses brackets.machine: one routine with record literals', () => {
      const { cst, lexErrors, parseErrors } = parse(readSample('brackets.machine'));
      expect(lexErrors).toEqual([]);
      expect(parseErrors).toEqual([]);
      expect(kinds(cst)).toEqual(['routineDecl']);
      expect(names(cst)).toEqual(['foo']);
    });

    it('parses operators.machine without confusing operator tokens for grammar', () => {
      const { cst, lexErrors, parseErrors } = parse(readSample('operators.machine'));
      expect(lexErrors).toEqual([]);
      expect(parseErrors).toEqual([]);
      expect(kinds(cst)).toEqual(['unitDecl']);
      expect(names(cst)).toEqual(['alu']);
    });
  });

  describe('error reporting', () => {
    it('reports an error when a top-level token is not a declaration keyword', () => {
      const { parseErrors } = parse('garbage here\n');
      expect(parseErrors.length).toBeGreaterThan(0);
    });

    it('reports an error when a register declaration is missing its type', () => {
      // `register AX` with no colon/type.
      const { parseErrors } = parse('register AX\n');
      expect(parseErrors.length).toBeGreaterThan(0);
    });

    it('reports an error when a routine parameter is missing its type', () => {
      // `routine foo(x)` with no `:type` — the param rule requires Colon typeRef.
      const { parseErrors } = parse('routine foo(x)\n');
      expect(parseErrors.length).toBeGreaterThan(0);
    });

    it('recovers from a malformed declaration and parses subsequent ones', () => {
      const src = ['garbage', '', 'register AX:u16', ''].join('\n');
      const { cst, parseErrors } = parse(src);
      // The parser should report an error for the first line but still
      // recognize the register declaration afterward.
      expect(parseErrors.length).toBeGreaterThan(0);
      expect(kinds(cst)).toContain('registerDecl');
    });
  });
});
