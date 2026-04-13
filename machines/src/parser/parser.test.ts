import { readFileSync } from 'node:fs';
import {
  parse,
  getDeclarations,
  getEnums,
  getBundles,
  getUnions,
} from './parse.js';

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

  describe('enum body parsing', () => {
    it('captures variant names in an enum declaration', () => {
      const { cst, parseErrors } = parse('enum Foo\n  a\n  b\n  c\n');
      expect(parseErrors).toEqual([]);
      expect(cst).toBeDefined();
      expect(getEnums(cst!)).toEqual([
        { name: 'Foo', variants: ['a', 'b', 'c'] },
      ]);
    });

    it('handles a single-variant enum', () => {
      const { cst, parseErrors } = parse('enum Solo\n  only\n');
      expect(parseErrors).toEqual([]);
      expect(getEnums(cst!)).toEqual([{ name: 'Solo', variants: ['only'] }]);
    });

    it('handles an enum with no body', () => {
      const { cst, parseErrors } = parse('enum Foo\n');
      expect(parseErrors).toEqual([]);
      expect(getEnums(cst!)).toEqual([{ name: 'Foo', variants: [] }]);
    });

    it('handles an enum declared at EOF with no trailing newline', () => {
      const { cst, parseErrors } = parse('enum Foo\n  a\n  b');
      expect(parseErrors).toEqual([]);
      expect(getEnums(cst!)).toEqual([
        { name: 'Foo', variants: ['a', 'b'] },
      ]);
    });

    it('tolerates blank lines between variants', () => {
      const src = ['enum Foo', '  a', '', '  b', '', '  c', ''].join('\n');
      const { cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
      expect(getEnums(cst!)[0]!.variants).toEqual(['a', 'b', 'c']);
    });

    it('tolerates comments interleaved with variants', () => {
      const src = [
        'enum Foo',
        '  ; first',
        '  a',
        '  ; second',
        '  b',
        '  ; third',
        '  c',
        '',
      ].join('\n');
      const { cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
      expect(getEnums(cst!)[0]!.variants).toEqual(['a', 'b', 'c']);
    });

    it('captures multiple enums in the same file in source order', () => {
      const src = [
        'enum Width',
        '  u8',
        '  u16',
        '',
        'enum AluOp',
        '  add',
        '  or',
        '  xor',
        '',
      ].join('\n');
      const { cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
      expect(getEnums(cst!)).toEqual([
        { name: 'Width', variants: ['u8', 'u16'] },
        { name: 'AluOp', variants: ['add', 'or', 'xor'] },
      ]);
    });

    it('captures Foo\'s variants from minimal.machine', () => {
      const { cst, parseErrors } = parse(readSample('minimal.machine'));
      expect(parseErrors).toEqual([]);
      expect(getEnums(cst!)).toEqual([
        { name: 'Foo', variants: ['a', 'b', 'c'] },
      ]);
    });

    it('parses every enum shape from enums.machine', () => {
      const { cst, parseErrors } = parse(readSample('enums.machine'));
      expect(parseErrors).toEqual([]);
      expect(getEnums(cst!)).toEqual([
        { name: 'Width', variants: ['u8', 'u16'] },
        {
          name: 'AluOp',
          variants: ['add', 'or', 'adc', 'sbb', 'and', 'sub', 'xor', 'passA'],
        },
        { name: 'Commit', variants: ['onIssue', 'onRetire', 'never'] },
        { name: 'Solo', variants: ['only'] },
      ]);
    });

    it('does not treat register or routine decls as enums', () => {
      const src = [
        'register AX:u16',
        '  field AL:u8 @ 0',
        '',
        'routine foo',
        '  bar',
        '',
      ].join('\n');
      const { cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
      expect(getEnums(cst!)).toEqual([]);
    });
  });

  describe('bundle body parsing', () => {
    it('captures field names and types in a simple bundle', () => {
      const src = 'bundle Flags\n  cf:b\n  zf:b\n  sf:b\n';
      const { cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
      expect(getBundles(cst!)).toEqual([
        {
          name: 'Flags',
          fields: [
            { name: 'cf', type: 'b' },
            { name: 'zf', type: 'b' },
            { name: 'sf', type: 'b' },
          ],
        },
      ]);
    });

    it('captures bundle fields with mixed widths and user types', () => {
      const src = [
        'bundle BusRequest',
        '  valid:b',
        '  address:u20',
        '  op:BusOp',
        '  size:BusSize',
        '  data:u16',
        '',
      ].join('\n');
      const { cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
      expect(getBundles(cst!)).toEqual([
        {
          name: 'BusRequest',
          fields: [
            { name: 'valid', type: 'b' },
            { name: 'address', type: 'u20' },
            { name: 'op', type: 'BusOp' },
            { name: 'size', type: 'BusSize' },
            { name: 'data', type: 'u16' },
          ],
        },
      ]);
    });

    it('tolerates whitespace after the field colon', () => {
      // `op: BusOp` with a space after the colon — the lexer strips
      // horizontal whitespace so it should tokenize the same as `op:BusOp`.
      const src = 'bundle Foo\n  op: BusOp\n  size:   BusSize\n';
      const { cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
      expect(getBundles(cst!)[0]!.fields).toEqual([
        { name: 'op', type: 'BusOp' },
        { name: 'size', type: 'BusSize' },
      ]);
    });

    it('handles a bundle with no body', () => {
      const { cst, parseErrors } = parse('bundle Empty\n');
      expect(parseErrors).toEqual([]);
      expect(getBundles(cst!)).toEqual([{ name: 'Empty', fields: [] }]);
    });

    it('handles a bundle at EOF with no trailing newline', () => {
      const { cst, parseErrors } = parse('bundle Foo\n  a:b\n  c:u8');
      expect(parseErrors).toEqual([]);
      expect(getBundles(cst!)[0]!.fields).toEqual([
        { name: 'a', type: 'b' },
        { name: 'c', type: 'u8' },
      ]);
    });

    it('tolerates comments interleaved with fields', () => {
      const src = [
        'bundle Foo',
        '  ; the first field',
        '  a:b',
        '  ; the second field',
        '  b:u8',
        '',
      ].join('\n');
      const { cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
      expect(getBundles(cst!)[0]!.fields).toEqual([
        { name: 'a', type: 'b' },
        { name: 'b', type: 'u8' },
      ]);
    });

    it('captures multiple bundles in the same file in source order', () => {
      const src = [
        'bundle A',
        '  x:b',
        '',
        'bundle B',
        '  y:u8',
        '',
      ].join('\n');
      const { cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
      expect(getBundles(cst!)).toEqual([
        { name: 'A', fields: [{ name: 'x', type: 'b' }] },
        { name: 'B', fields: [{ name: 'y', type: 'u8' }] },
      ]);
    });

    it('does not treat enum or register decls as bundles', () => {
      const src = ['enum Foo', '  a', '', 'register AX:u16', ''].join('\n');
      const { cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
      expect(getBundles(cst!)).toEqual([]);
    });
  });

  describe('union body parsing', () => {
    it('captures arms and their payload types', () => {
      const src = [
        'union MicroOp',
        '  alu:AluMicro',
        '  istreamRead:IStreamRead',
        '  busRead:BusRead',
        '  retire:Retire',
        '',
      ].join('\n');
      const { cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
      expect(getUnions(cst!)).toEqual([
        {
          name: 'MicroOp',
          arms: [
            { name: 'alu', type: 'AluMicro' },
            { name: 'istreamRead', type: 'IStreamRead' },
            { name: 'busRead', type: 'BusRead' },
            { name: 'retire', type: 'Retire' },
          ],
        },
      ]);
    });

    it('handles a union with no body', () => {
      const { cst, parseErrors } = parse('union Empty\n');
      expect(parseErrors).toEqual([]);
      expect(getUnions(cst!)).toEqual([{ name: 'Empty', arms: [] }]);
    });

    it('tolerates comments interleaved with arms', () => {
      const src = [
        'union Op',
        '  ; the ALU path',
        '  alu:AluMicro',
        '',
        '  ; the bus path',
        '  bus:BusRead',
        '',
      ].join('\n');
      const { cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
      expect(getUnions(cst!)[0]!.arms).toEqual([
        { name: 'alu', type: 'AluMicro' },
        { name: 'bus', type: 'BusRead' },
      ]);
    });

    it('does not treat bundle decls as unions', () => {
      const { cst, parseErrors } = parse('bundle Foo\n  a:b\n');
      expect(parseErrors).toEqual([]);
      expect(getUnions(cst!)).toEqual([]);
    });
  });

  describe('bundles and unions together', () => {
    it('parses bundles.machine and captures every declaration', () => {
      const { cst, parseErrors } = parse(readSample('bundles.machine'));
      expect(parseErrors).toEqual([]);

      expect(getBundles(cst!)).toEqual([
        {
          name: 'Flags',
          fields: [
            { name: 'cf', type: 'b' },
            { name: 'pf', type: 'b' },
            { name: 'af', type: 'b' },
            { name: 'zf', type: 'b' },
            { name: 'sf', type: 'b' },
            { name: 'of', type: 'b' },
          ],
        },
        {
          name: 'BusRequest',
          fields: [
            { name: 'valid', type: 'b' },
            { name: 'address', type: 'u20' },
            { name: 'op', type: 'BusOp' },
            { name: 'size', type: 'BusSize' },
            { name: 'data', type: 'u16' },
          ],
        },
        {
          name: 'BusResponse',
          fields: [
            { name: 'grant', type: 'b' },
            { name: 'done', type: 'b' },
            { name: 'data', type: 'u16' },
          ],
        },
      ]);

      expect(getUnions(cst!)).toEqual([
        {
          name: 'MicroOp',
          arms: [
            { name: 'alu', type: 'AluMicro' },
            { name: 'istreamRead', type: 'IStreamRead' },
            { name: 'busRead', type: 'BusRead' },
            { name: 'busWrite', type: 'BusWrite' },
            { name: 'branch', type: 'Branch' },
            { name: 'retire', type: 'Retire' },
          ],
        },
      ]);

      // The getDeclarations skeleton still sees all four top-level decls
      // in source order.
      expect(
        getDeclarations(cst!).map((d) => `${d.kind}:${d.name}`),
      ).toEqual([
        'bundleDecl:Flags',
        'bundleDecl:BusRequest',
        'bundleDecl:BusResponse',
        'unionDecl:MicroOp',
      ]);
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
