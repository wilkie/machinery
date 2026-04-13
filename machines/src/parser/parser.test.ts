import { readFileSync } from 'node:fs';
import {
  parse,
  getDeclarations,
  getEnums,
  getBundles,
  getUnions,
  getRegisters,
  getMicrowords,
  getOperands,
  getRoutines,
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
      // Wrap the statement-y content in a `micro` section. The micro
      // section's body is still consumed opaquely via the `block`
      // skipper until statement grammar lands, which is what this
      // test exercises.
      const src = [
        'routine foo',
        '  micro',
        '    fetch ModRM',
        '    call eaCalc(mod, rm) -> ea',
        '    AluMicro { op: add, width: u8, srcA: mem, srcB: ModRM.reg,',
        '               dest: tmp, writeFlags: 1, commit: onRetire }',
        '    Retire {}',
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
      expect(kinds(cst)).toEqual(['routineDecl', 'unitDecl', 'machineDecl']);
      expect(names(cst)).toEqual(['foo', 'alu', 'counter']);
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
        '  entry: 0x00',
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

  describe('register body parsing', () => {
    it('captures fields with decimal bit offsets', () => {
      const src = 'register AX:u16\n  field AL:u8 @ 0\n  field AH:u8 @ 8\n';
      const { cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
      expect(getRegisters(cst!)).toEqual([
        {
          name: 'AX',
          type: 'u16',
          fields: [
            { name: 'AL', type: 'u8', offset: 0 },
            { name: 'AH', type: 'u8', offset: 8 },
          ],
        },
      ]);
    });

    it('captures fields with hex bit offsets', () => {
      const src = 'register R:u32\n  field lo:u8 @ 0x0\n  field hi:u8 @ 0x10\n';
      const { cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
      expect(getRegisters(cst!)[0]!.fields).toEqual([
        { name: 'lo', type: 'u8', offset: 0 },
        { name: 'hi', type: 'u8', offset: 16 },
      ]);
    });

    it('captures fields with non-contiguous offsets', () => {
      const src = [
        'register FLAGS:u16',
        '  field CF:b @ 0',
        '  field PF:b @ 2',
        '  field AF:b @ 4',
        '  field ZF:b @ 6',
        '  field SF:b @ 7',
        '  field OF:b @ 11',
        '',
      ].join('\n');
      const { cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
      expect(getRegisters(cst!)[0]!.fields.map((f) => f.offset)).toEqual([
        0, 2, 4, 6, 7, 11,
      ]);
    });

    it('handles a register with no body (bare declaration)', () => {
      const { cst, parseErrors } = parse('register SI:u16\n');
      expect(parseErrors).toEqual([]);
      expect(getRegisters(cst!)).toEqual([
        { name: 'SI', type: 'u16', fields: [] },
      ]);
    });

    it('handles a register at EOF with no trailing newline', () => {
      const { cst, parseErrors } = parse('register AX:u16\n  field AL:u8 @ 0');
      expect(parseErrors).toEqual([]);
      expect(getRegisters(cst!)).toEqual([
        {
          name: 'AX',
          type: 'u16',
          fields: [{ name: 'AL', type: 'u8', offset: 0 }],
        },
      ]);
    });

    it('handles a field with no offset clause', () => {
      // Shape the grammar tolerates but real files don't use yet.
      const src = 'register Foo:u8\n  field x:b\n';
      const { cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
      expect(getRegisters(cst!)[0]!.fields).toEqual([
        { name: 'x', type: 'b', offset: undefined },
      ]);
    });

    it('tolerates comments interleaved with fields', () => {
      const src = [
        'register AX:u16',
        '  ; low byte',
        '  field AL:u8 @ 0',
        '  ; high byte',
        '  field AH:u8 @ 8',
        '',
      ].join('\n');
      const { cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
      expect(getRegisters(cst!)[0]!.fields.map((f) => f.name)).toEqual([
        'AL',
        'AH',
      ]);
    });

    it('captures multiple registers in the same file in source order', () => {
      const src = [
        'register AX:u16',
        '  field AL:u8 @ 0',
        '',
        'register SI:u16',
        '',
        'register FLAGS:u16',
        '  field CF:b @ 0',
        '',
      ].join('\n');
      const { cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
      expect(getRegisters(cst!).map((r) => r.name)).toEqual([
        'AX',
        'SI',
        'FLAGS',
      ]);
      expect(getRegisters(cst!)[1]!.fields).toEqual([]);
    });

    it('parses every shape from registers.machine', () => {
      const { cst, parseErrors } = parse(readSample('registers.machine'));
      expect(parseErrors).toEqual([]);
      const regs = getRegisters(cst!);
      expect(regs.map((r) => r.name)).toEqual([
        'AX',
        'BX',
        'CX',
        'DX',
        'SI',
        'DI',
        'BP',
        'SP',
        'CS',
        'DS',
        'ES',
        'SS',
        'IP',
        'FLAGS',
      ]);

      // Spot-check a few shapes: general-purpose register, bare segment
      // register, and FLAGS' non-contiguous bit layout.
      expect(regs.find((r) => r.name === 'AX')).toEqual({
        name: 'AX',
        type: 'u16',
        fields: [
          { name: 'AL', type: 'u8', offset: 0 },
          { name: 'AH', type: 'u8', offset: 8 },
        ],
      });
      expect(regs.find((r) => r.name === 'CS')).toEqual({
        name: 'CS',
        type: 'seg16',
        fields: [],
      });
      expect(regs.find((r) => r.name === 'FLAGS')!.fields).toEqual([
        { name: 'CF', type: 'b', offset: 0 },
        { name: 'PF', type: 'b', offset: 2 },
        { name: 'AF', type: 'b', offset: 4 },
        { name: 'ZF', type: 'b', offset: 6 },
        { name: 'SF', type: 'b', offset: 7 },
        { name: 'OF', type: 'b', offset: 11 },
      ]);
    });

    it('does not treat enum or bundle decls as registers', () => {
      const src = [
        'enum Foo',
        '  a',
        '',
        'bundle Bar',
        '  x:b',
        '',
      ].join('\n');
      const { cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
      expect(getRegisters(cst!)).toEqual([]);
    });
  });

  describe('microword body parsing', () => {
    it('captures fields from a microword with every section present', () => {
      const src = [
        'microword AluMicro',
        '  description',
        '    The one-cycle ALU-shaped microword.',
        '',
        '  fields',
        '    op:AluOp',
        '    width:Width',
        '    dest:AluDest',
        '',
        '  ready: 1',
        '',
        '  effect',
        '    wire a = aluSelect(srcA)',
        '',
      ].join('\n');
      const { cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
      expect(getMicrowords(cst!)).toEqual([
        {
          name: 'AluMicro',
          fields: [
            { name: 'op', type: 'AluOp' },
            { name: 'width', type: 'Width' },
            { name: 'dest', type: 'AluDest' },
          ],
        },
      ]);
    });

    it('handles an inline empty fields section (fields {})', () => {
      const src = [
        'microword Retire',
        '  fields {}',
        '  ready: 1',
        '',
      ].join('\n');
      const { cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
      expect(getMicrowords(cst!)).toEqual([
        { name: 'Retire', fields: [] },
      ]);
    });

    it('handles an inline non-empty fields section', () => {
      const src = [
        'microword Mini',
        '  fields { dest:Local }',
        '  ready: 1',
        '',
      ].join('\n');
      const { cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
      expect(getMicrowords(cst!)).toEqual([
        { name: 'Mini', fields: [{ name: 'dest', type: 'Local' }] },
      ]);
    });

    it('handles an inline fields section with multiple entries', () => {
      const src = [
        'microword Foo',
        '  fields { a:u8, b:u16, c:Local }',
        '',
      ].join('\n');
      const { cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
      expect(getMicrowords(cst!)[0]!.fields).toEqual([
        { name: 'a', type: 'u8' },
        { name: 'b', type: 'u16' },
        { name: 'c', type: 'Local' },
      ]);
    });

    it('handles a microword with no body', () => {
      const { cst, parseErrors } = parse('microword Empty\n');
      expect(parseErrors).toEqual([]);
      expect(getMicrowords(cst!)).toEqual([
        { name: 'Empty', fields: [] },
      ]);
    });

    it('handles a microword with only a fields section', () => {
      const src = 'microword Foo\n  fields\n    x:b\n    y:u8\n';
      const { cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
      expect(getMicrowords(cst!)[0]!.fields).toEqual([
        { name: 'x', type: 'b' },
        { name: 'y', type: 'u8' },
      ]);
    });

    it('handles ready with a complex expression opaquely', () => {
      // The readyClause rule skips the expression, so any token
      // sequence on the line should be accepted.
      const src = [
        'microword Foo',
        '  fields',
        '    dest:Local',
        '  ready: prefetch.valid && !hold && cycleCount > 0',
        '',
      ].join('\n');
      const { cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
      expect(getMicrowords(cst!)[0]!.fields).toEqual([
        { name: 'dest', type: 'Local' },
      ]);
    });

    it('handles an effect block full of arbitrary tokens', () => {
      const src = [
        'microword Foo',
        '  fields',
        '    dest:Local',
        '  effect',
        '    dest <- prefetch.byte',
        '    prefetchControl.pop = 1',
        '    if dest == 0',
        '      FLAGS.ZF <- 1',
        '',
      ].join('\n');
      const { cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
      expect(getMicrowords(cst!)[0]!.fields).toEqual([
        { name: 'dest', type: 'Local' },
      ]);
    });

    it('accepts sections in an unusual order', () => {
      // Real files always use description → fields → ready → effect,
      // but the grammar doesn't enforce order.
      const src = [
        'microword Foo',
        '  effect',
        '    a <- b',
        '  ready: 1',
        '  fields',
        '    x:b',
        '',
      ].join('\n');
      const { cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
      expect(getMicrowords(cst!)[0]!.fields).toEqual([
        { name: 'x', type: 'b' },
      ]);
    });

    it('captures multiple microwords in the same file in source order', () => {
      const src = [
        'microword A',
        '  fields',
        '    x:b',
        '',
        'microword B',
        '  fields',
        '    y:u8',
        '  ready: 1',
        '',
      ].join('\n');
      const { cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
      expect(getMicrowords(cst!)).toEqual([
        { name: 'A', fields: [{ name: 'x', type: 'b' }] },
        { name: 'B', fields: [{ name: 'y', type: 'u8' }] },
      ]);
    });

    it('parses every microword shape from microwords.machine', () => {
      const { cst, parseErrors } = parse(readSample('microwords.machine'));
      expect(parseErrors).toEqual([]);
      const mws = getMicrowords(cst!);
      expect(mws.map((m) => m.name)).toEqual([
        'AluMicro',
        'IStreamRead',
        'BusRead',
        'BusWrite',
        'Branch',
        'Retire',
      ]);

      // Spot-check each shape:
      expect(mws.find((m) => m.name === 'AluMicro')!.fields).toEqual([
        { name: 'op', type: 'AluOp' },
        { name: 'width', type: 'Width' },
        { name: 'srcA', type: 'AluSrc' },
        { name: 'srcB', type: 'AluSrc' },
        { name: 'dest', type: 'AluDest' },
        { name: 'commit', type: 'Commit' },
        { name: 'writeFlags', type: 'b' },
      ]);
      expect(mws.find((m) => m.name === 'IStreamRead')!.fields).toEqual([
        { name: 'dest', type: 'Local' },
      ]);
      expect(mws.find((m) => m.name === 'BusRead')!.fields).toEqual([
        { name: 'width', type: 'BusSize' },
        { name: 'addr', type: 'Local' },
        { name: 'dest', type: 'Local' },
      ]);
      expect(mws.find((m) => m.name === 'Branch')!.fields).toEqual([
        { name: 'cond', type: 'BranchCond' },
        { name: 'target', type: 'MicroAddr' },
      ]);
      expect(mws.find((m) => m.name === 'Retire')!.fields).toEqual([]);
    });

    it('does not treat bundle or register decls as microwords', () => {
      const src = [
        'bundle Foo',
        '  a:b',
        '',
        'register AX:u16',
        '  field AL:u8 @ 0',
        '',
      ].join('\n');
      const { cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
      expect(getMicrowords(cst!)).toEqual([]);
    });

    it('parses a microword with a terminal clause', () => {
      const src = [
        'microword Retire',
        '  fields {}',
        '  ready: 1',
        '  terminal: 1',
        '',
      ].join('\n');
      const { cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
      expect(getMicrowords(cst!)).toEqual([{ name: 'Retire', fields: [] }]);
    });

    it('parses ready and terminal clauses in either order', () => {
      const src = [
        'microword Foo',
        '  fields',
        '    x:u8',
        '  terminal: 0',
        '  ready: 1',
        '',
      ].join('\n');
      const { cst: _cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
    });

    it('parses a terminal clause with a complex expression', () => {
      // Rare but legal — the clause takes any expression, so a
      // field-dependent terminal flag is parseable.
      const src = [
        'microword Foo',
        '  fields',
        '    done:b',
        '  ready: 1',
        '  terminal: done',
        '',
      ].join('\n');
      const { cst: _cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
    });

    it('still accepts `terminal` as a regular identifier elsewhere', () => {
      // Soft-keyword regression: bundle fields named `terminal`
      // should still work.
      const src = 'bundle Foo\n  terminal:b\n  value:u8\n';
      const { cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
      expect(getBundles(cst!)[0]!.fields).toEqual([
        { name: 'terminal', type: 'b' },
        { name: 'value', type: 'u8' },
      ]);
    });
  });

  describe('operand body parsing', () => {
    it('captures name, size, and bit-offset fields for a structured operand', () => {
      const src = [
        'operand ModRM',
        '  size: 1',
        '  fields',
        '    mod:u2 @ 6',
        '    reg:u3 @ 3',
        '    rm:u3 @ 0',
        '',
      ].join('\n');
      const { cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
      expect(getOperands(cst!)).toEqual([
        {
          name: 'ModRM',
          size: 1,
          fields: [
            { name: 'mod', type: 'u2', offset: 6 },
            { name: 'reg', type: 'u3', offset: 3 },
            { name: 'rm', type: 'u3', offset: 0 },
          ],
        },
      ]);
    });

    it('captures size with a hex literal', () => {
      const src = 'operand Foo\n  size: 0x04\n';
      const { cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
      expect(getOperands(cst!)).toEqual([
        { name: 'Foo', size: 4, fields: [] },
      ]);
    });

    it('handles operands with no fields section', () => {
      const src = [
        'operand Disp16',
        '  size: 2',
        '  fetch',
        '    IStreamRead { dest: disp }',
        '',
      ].join('\n');
      const { cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
      expect(getOperands(cst!)).toEqual([
        { name: 'Disp16', size: 2, fields: [] },
      ]);
    });

    it('handles operands with no body', () => {
      const { cst, parseErrors } = parse('operand Empty\n');
      expect(parseErrors).toEqual([]);
      expect(getOperands(cst!)).toEqual([
        { name: 'Empty', size: undefined, fields: [] },
      ]);
    });

    it('handles an inline description in an operand body', () => {
      const src = [
        'operand Disp16',
        '  description: "16-bit little-endian displacement."',
        '  size: 2',
        '',
      ].join('\n');
      const { cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
      expect(getOperands(cst!)).toEqual([
        { name: 'Disp16', size: 2, fields: [] },
      ]);
    });

    it('handles a block description in an operand body', () => {
      const src = [
        'operand Foo',
        '  description',
        '    A multi-line description that goes on',
        '    for several words and punctuation.',
        '  size: 1',
        '',
      ].join('\n');
      const { cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
      expect(getOperands(cst!)).toEqual([
        { name: 'Foo', size: 1, fields: [] },
      ]);
    });

    it('handles fields without bit offsets (falls through as undefined)', () => {
      const src = [
        'operand Foo',
        '  size: 2',
        '  fields',
        '    a:u8',
        '    b:u8',
        '',
      ].join('\n');
      const { cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
      expect(getOperands(cst!)[0]!.fields).toEqual([
        { name: 'a', type: 'u8', offset: undefined },
        { name: 'b', type: 'u8', offset: undefined },
      ]);
    });

    it('handles a fetch block full of record literals across multiple lines', () => {
      const src = [
        'operand Foo',
        '  size: 1',
        '  fetch',
        '    IStreamRead { dest: tmp }',
        '    AluMicro { op: passA, width: u8, srcA: tmp, srcB: zero,',
        '               dest: disp, writeFlags: 0, commit: never }',
        '',
      ].join('\n');
      const { cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
      expect(getOperands(cst!)).toEqual([
        { name: 'Foo', size: 1, fields: [] },
      ]);
    });

    it('parses every operand shape from operands.machine', () => {
      const { cst, parseErrors } = parse(readSample('operands.machine'));
      expect(parseErrors).toEqual([]);
      const ops = getOperands(cst!);
      expect(ops.map((o) => o.name)).toEqual(['ModRM', 'Disp8', 'Disp16']);

      expect(ops.find((o) => o.name === 'ModRM')).toEqual({
        name: 'ModRM',
        size: 1,
        fields: [
          { name: 'mod', type: 'u2', offset: 6 },
          { name: 'reg', type: 'u3', offset: 3 },
          { name: 'rm', type: 'u3', offset: 0 },
        ],
      });
      expect(ops.find((o) => o.name === 'Disp8')).toEqual({
        name: 'Disp8',
        size: 1,
        fields: [],
      });
      expect(ops.find((o) => o.name === 'Disp16')).toEqual({
        name: 'Disp16',
        size: 2,
        fields: [],
      });
    });

    it('does not treat microword or bundle decls as operands', () => {
      const src = [
        'bundle Foo',
        '  a:b',
        '',
        'microword Bar',
        '  fields',
        '    x:u8',
        '',
      ].join('\n');
      const { cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
      expect(getOperands(cst!)).toEqual([]);
    });
  });

  describe('size soft-keyword', () => {
    it('still accepts `size` as a bundle field name', () => {
      // Critical regression test: BusRequest bundle uses `size:BusSize`
      // as a field name. The Size soft keyword must not break this.
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
      expect(getBundles(cst!)[0]!.fields).toEqual([
        { name: 'valid', type: 'b' },
        { name: 'address', type: 'u20' },
        { name: 'op', type: 'BusOp' },
        { name: 'size', type: 'BusSize' },
        { name: 'data', type: 'u16' },
      ]);
    });

    it('still accepts `size` as a microword field name', () => {
      const src = 'microword Foo\n  fields\n    size:u8\n';
      const { cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
      expect(getMicrowords(cst!)[0]!.fields).toEqual([
        { name: 'size', type: 'u8' },
      ]);
    });

    it('still accepts `size` as a union arm name', () => {
      const src = 'union Foo\n  size:SizeInfo\n';
      const { cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
      expect(getUnions(cst!)[0]!.arms).toEqual([
        { name: 'size', type: 'SizeInfo' },
      ]);
    });
  });

  describe('routine body parsing', () => {
    it('parses a routine with an inline description', () => {
      const { cst, parseErrors } = parse(
        'routine foo\n  description: "a single-line description"\n',
      );
      expect(parseErrors).toEqual([]);
      expect(getRoutines(cst!)).toEqual([
        {
          name: 'foo',
          entry: undefined,
          allow: [],
          modifies: [],
          references: [],
        },
      ]);
    });

    it('parses a routine with a block description', () => {
      const src = [
        'routine foo',
        '  description',
        '    A multi-line description',
        '    that spans several lines.',
        '',
      ].join('\n');
      const { cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
      expect(getRoutines(cst!)[0]!.name).toBe('foo');
    });

    it('captures entry as a decimal value', () => {
      const { cst, parseErrors } = parse('routine foo\n  entry: 42\n');
      expect(parseErrors).toEqual([]);
      expect(getRoutines(cst!)[0]!.entry).toBe(42);
    });

    it('captures entry as a hex value', () => {
      const { cst, parseErrors } = parse('routine foo\n  entry: 0xB0\n');
      expect(parseErrors).toEqual([]);
      expect(getRoutines(cst!)[0]!.entry).toBe(0xb0);
    });

    it('returns undefined for a complex entry expression', () => {
      // An expression like `0x80 + 1` is not a bare literal, so the
      // walker reports it as undefined until the extractor grows.
      const { cst, parseErrors } = parse('routine foo\n  entry: 0x80 + 1\n');
      expect(parseErrors).toEqual([]);
      expect(getRoutines(cst!)[0]!.entry).toBeUndefined();
    });

    it('parses an allow list with one entry', () => {
      const { cst, parseErrors } = parse(
        'routine foo\n  allow: [lock]\n',
      );
      expect(parseErrors).toEqual([]);
      expect(getRoutines(cst!)[0]!.allow).toEqual(['lock']);
    });

    it('parses an empty allow list', () => {
      const { cst, parseErrors } = parse('routine foo\n  allow: []\n');
      expect(parseErrors).toEqual([]);
      expect(getRoutines(cst!)[0]!.allow).toEqual([]);
    });

    it('parses a modifies list with multiple entries', () => {
      const { cst, parseErrors } = parse(
        'routine foo\n  modifies: [OF, SF, ZF, AF, PF, CF]\n',
      );
      expect(parseErrors).toEqual([]);
      expect(getRoutines(cst!)[0]!.modifies).toEqual([
        'OF',
        'SF',
        'ZF',
        'AF',
        'PF',
        'CF',
      ]);
    });

    it('parses a references block with one entry', () => {
      const src = [
        'routine foo',
        '  references',
        '    - "Intel 80286 Programmer\'s Reference Manual, ADD"',
        '',
      ].join('\n');
      const { cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
      expect(getRoutines(cst!)[0]!.references).toEqual([
        "Intel 80286 Programmer's Reference Manual, ADD",
      ]);
    });

    it('parses a references block with multiple entries', () => {
      const src = [
        'routine foo',
        '  references',
        '    - "manual A"',
        '    - "manual B"',
        '    - "manual C"',
        '',
      ].join('\n');
      const { cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
      expect(getRoutines(cst!)[0]!.references).toEqual([
        'manual A',
        'manual B',
        'manual C',
      ]);
    });

    it('skips over a micro block opaquely', () => {
      // The micro block contains call directives and record literals
      // that we don't have statement grammar for yet, but the opaque
      // block skipper should still consume them without errors.
      const src = [
        'routine addRmReg8',
        '  entry: 0x00',
        '  micro',
        '    fetch ModRM',
        '    call eaCalc(ModRM.mod, ModRM.rm) -> ea',
        '    call memRead8(ea) -> mdr',
        '    AluMicro { op: add, width: u8, srcA: mem, srcB: ModRM.reg,',
        '               dest: tmp, writeFlags: 1, commit: onRetire }',
        '    call memWrite8(ea, tmp)',
        '    Retire {}',
        '',
      ].join('\n');
      const { cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
      expect(getRoutines(cst!)[0]!.entry).toBe(0);
    });

    it('parses a routine with every section present', () => {
      const src = [
        'routine addRmReg8',
        '  description',
        '    Integer addition of an 8-bit register into an 8-bit r/m operand.',
        '',
        '  references',
        '    - "Intel 80286 Programmer\'s Reference Manual, ADD"',
        '',
        '  entry: 0x00',
        '  allow: [lock]',
        '  modifies: [OF, SF, ZF, AF, PF, CF]',
        '',
        '  micro',
        '    Retire {}',
        '',
      ].join('\n');
      const { cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
      const routines = getRoutines(cst!);
      expect(routines).toEqual([
        {
          name: 'addRmReg8',
          entry: 0,
          allow: ['lock'],
          modifies: ['OF', 'SF', 'ZF', 'AF', 'PF', 'CF'],
          references: ["Intel 80286 Programmer's Reference Manual, ADD"],
        },
      ]);
    });

    it('accepts sections in an unusual order', () => {
      const src = [
        'routine foo',
        '  modifies: [CF]',
        '  entry: 0x01',
        '  allow: [lock]',
        '',
      ].join('\n');
      const { cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
      expect(getRoutines(cst!)[0]).toEqual({
        name: 'foo',
        entry: 1,
        allow: ['lock'],
        modifies: ['CF'],
        references: [],
      });
    });

    it('captures multiple routines in source order', () => {
      const src = [
        'routine foo',
        '  entry: 0x10',
        '',
        'routine bar',
        '  entry: 0x20',
        '',
        'routine baz',
        '  entry: 0x30',
        '',
      ].join('\n');
      const { cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
      expect(getRoutines(cst!).map((r) => ({ name: r.name, entry: r.entry }))).toEqual([
        { name: 'foo', entry: 0x10 },
        { name: 'bar', entry: 0x20 },
        { name: 'baz', entry: 0x30 },
      ]);
    });

    it('preserves routine parameter and return-type parsing', () => {
      // Regression: the earlier routine-signature tests should still
      // pass after the body grammar change.
      const src = 'routine eaCalc(mod:u2, rm:u3) -> (ea:u16)\n';
      const { cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
      expect(getRoutines(cst!)).toEqual([
        {
          name: 'eaCalc',
          entry: undefined,
          allow: [],
          modifies: [],
          references: [],
        },
      ]);
    });

    it('parses every routine shape from routines.machine', () => {
      const { cst, parseErrors } = parse(readSample('routines.machine'));
      expect(parseErrors).toEqual([]);
      const routines = getRoutines(cst!);
      expect(routines.map((r) => r.name)).toEqual([
        'eaCalc',
        'memRead8',
        'memWrite8',
        'addRmReg8',
        'nop',
      ]);
      expect(routines.find((r) => r.name === 'addRmReg8')).toEqual({
        name: 'addRmReg8',
        entry: 0,
        allow: ['lock'],
        modifies: ['OF', 'SF', 'ZF', 'AF', 'PF', 'CF'],
        references: ["Intel 80286 Programmer's Reference Manual, ADD"],
      });
      expect(routines.find((r) => r.name === 'nop')).toEqual({
        name: 'nop',
        entry: 0x90,
        allow: [],
        modifies: [],
        references: [],
      });
    });

    it('does not treat microword or operand decls as routines', () => {
      const src = [
        'microword Foo',
        '  ready: 1',
        '',
        'operand Bar',
        '  size: 1',
        '',
      ].join('\n');
      const { cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
      expect(getRoutines(cst!)).toEqual([]);
    });
  });

  describe('unit body parsing', () => {
    it('parses an anonymous single-output unit with a bare `= expr`', () => {
      const src = [
        'unit parity8',
        '  wires in',
        '    v:u8',
        '  wires out',
        '    *:b',
        '',
        '  = ~(v[0] ^ v[1])',
        '',
      ].join('\n');
      const { cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
      expect(kinds(cst)).toEqual(['unitDecl']);
      expect(names(cst)).toEqual(['parity8']);
    });

    it('parses a unit with both wires in and wires out sections', () => {
      const src = [
        'unit aluSelect',
        '  wires in',
        '    sel:AluSrc',
        '    reg:u16',
        '  wires out',
        '    *:u16',
        '',
        '  mux sel',
        '    when AluSrc.zero: 0',
        '    when AluSrc.reg:  reg',
        '',
      ].join('\n');
      const { cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
      expect(kinds(cst)).toEqual(['unitDecl']);
    });

    it('parses a unit with type parameters and parameterized types', () => {
      const src = [
        'unit aluFlags<W:Width>',
        '  wires in',
        '    a:u{W}',
        '    b:u{W}',
        '    raw:u{W+1}',
        '  wires out',
        '    *:Flags',
        '',
        '  wire r:u{W} = raw:u{W}',
        '',
      ].join('\n');
      const { cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
      expect(kinds(cst)).toEqual(['unitDecl']);
    });

    it('parses a unit with mux and multi-statement arms', () => {
      const src = [
        'unit flagCompute',
        '  wires in',
        '    op:AluOp',
        '    raw:u8',
        '  wires out',
        '    *:Flags',
        '',
        '  mux op',
        '    when AluOp.add',
        '      cf = raw[7]',
        '      af = raw[3]',
        '    else',
        '      cf = 0',
        '      af = 0',
        '',
      ].join('\n');
      const { cst: _cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
    });

    it('parses a unit with an empty body', () => {
      const { cst, parseErrors } = parse('unit empty\n');
      expect(parseErrors).toEqual([]);
      expect(kinds(cst)).toEqual(['unitDecl']);
    });

    it('does not treat routine or machine decls as units', () => {
      const src = [
        'routine foo',
        '  entry: 0x00',
        '',
        'machine bar',
        '  wires in',
        '    x:b',
        '',
      ].join('\n');
      const { cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
      expect(kinds(cst)).toEqual(['routineDecl', 'machineDecl']);
    });
  });

  describe('machine body parsing', () => {
    it('parses a machine with id, description, registers, wires, default', () => {
      const src = [
        'machine cpu',
        '  id i286',
        '  description',
        '    A tiny CPU.',
        '',
        '  registers',
        '    state:ExecuteState = ExecuteState.fetch',
        '    microPc:u8 = 0',
        '',
        '  wires in',
        '    prefetch:PrefetchOut',
        '',
        '  wires out',
        '    prefetchControl:PrefetchControl',
        '',
        '  default',
        '    prefetchControl = { pop: 0 }',
        '',
      ].join('\n');
      const { cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
      expect(kinds(cst)).toEqual(['machineDecl']);
      expect(names(cst)).toEqual(['cpu']);
    });

    it('parses a machine with a register that has nested field decls', () => {
      const src = [
        'machine cpu',
        '  registers',
        '    modrm:u8 = 0',
        '      field mod:u2 @ 6',
        '      field reg:u3 @ 3',
        '      field rm:u3 @ 0',
        '    state:u8 = 0',
        '',
      ].join('\n');
      const { cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
      expect(kinds(cst)).toEqual(['machineDecl']);
    });

    it('parses a machine with assert and if/else statement bodies', () => {
      const src = [
        'machine cpu',
        '  wires in',
        '    valid:b',
        '    pop:b',
        '  wires out',
        '    ack:b',
        '',
        '  assert !(pop && !valid)',
        '',
        '  if valid',
        '    ack = 1',
        '  else',
        '    ack = 0',
        '',
      ].join('\n');
      const { cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
      expect(kinds(cst)).toEqual(['machineDecl']);
    });

    it('parses a machine with a top-level mux', () => {
      const src = [
        'machine cpu',
        '  registers',
        '    state:u8 = 0',
        '',
        '  mux state',
        '    when 0',
        '      state <- 1',
        '    when 1',
        '      state <- 2',
        '    else',
        '      state <- 0',
        '',
      ].join('\n');
      const { cst: _cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
    });

    it('parses a machine with an empty body', () => {
      const { cst, parseErrors } = parse('machine cpu\n');
      expect(parseErrors).toEqual([]);
      expect(kinds(cst)).toEqual(['machineDecl']);
    });

    it('parses the full units-and-machines sample file', () => {
      const { cst, lexErrors, parseErrors } = parse(
        readSample('units-and-machines.machine'),
      );
      expect(lexErrors).toEqual([]);
      expect(parseErrors).toEqual([]);
      expect(kinds(cst)).toEqual([
        'unitDecl',
        'unitDecl',
        'unitDecl',
        'machineDecl',
      ]);
      expect(names(cst)).toEqual([
        'parity8',
        'aluSelect',
        'aluFlags',
        'executionUnit',
      ]);
    });

    it('parses a rom declaration listing one routine', () => {
      const src = [
        'machine cpu',
        '  rom microcode:MicroOp[]',
        '    addRmReg8',
        '',
      ].join('\n');
      const { cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
      expect(kinds(cst)).toEqual(['machineDecl']);
    });

    it('parses a rom declaration listing multiple routines', () => {
      const src = [
        'machine cpu',
        '  rom microcode:MicroOp[]',
        '    addRmReg8',
        '    addRmReg16',
        '    subRmReg8',
        '    subRmReg16',
        '    movRmReg8',
        '',
      ].join('\n');
      const { cst: _cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
    });

    it('parses multiple rom declarations in one machine', () => {
      const src = [
        'machine cpu',
        '  rom microcode:MicroOp[]',
        '    addRmReg8',
        '    subRmReg8',
        '',
        '  rom altRom:MicroOp[]',
        '    otherRoutine',
        '',
      ].join('\n');
      const { cst: _cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
    });

    it('parses a rom declaration with an empty listing', () => {
      const src = [
        'machine cpu',
        '  rom microcode:MicroOp[]',
        '',
      ].join('\n');
      const { cst: _cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
    });

    it('parses a nested-form rom with one decode surface', () => {
      const src = [
        'machine cpu',
        '  rom microops',
        '    microcode:MicroOp[]',
        '      addRmReg8',
        '',
      ].join('\n');
      const { cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
      expect(kinds(cst)).toEqual(['machineDecl']);
    });

    it('parses a nested-form rom with multiple decode surfaces', () => {
      // The motivating case: x86-style execute unit with a primary
      // opcode space and a 0x0F-prefixed escape opcode space.
      const src = [
        'machine executionUnit',
        '  rom microops',
        '    microcode:MicroOp[]',
        '      addRmReg8',
        '      subRmReg8',
        '      movRmReg8',
        '    systemcode:MicroOp[]',
        '      lgdt',
        '      sgdt',
        '      lldt',
        '      sldt',
        '      lmsw',
        '',
      ].join('\n');
      const { cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
      expect(kinds(cst)).toEqual(['machineDecl']);
      expect(names(cst)).toEqual(['executionUnit']);
    });

    it('parses a nested rom with an empty body', () => {
      const src = [
        'machine cpu',
        '  rom microops',
        '',
      ].join('\n');
      const { cst: _cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
    });

    it('parses a nested rom with an empty surface listing', () => {
      const src = [
        'machine cpu',
        '  rom microops',
        '    microcode:MicroOp[]',
        '',
      ].join('\n');
      const { cst: _cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
    });

    it('parses nested rom with member-access decode references', () => {
      // Verify that `microops.microcode.decode(prefetch.byte)` parses
      // as an expression inside a machine body statement. This is the
      // call shape the execute unit uses to dispatch via a nested
      // rom's decode surfaces.
      const src = [
        'machine executionUnit',
        '  registers',
        '    microPC:u16 = 0',
        '    state:u8 = 0',
        '',
        '  rom microops',
        '    microcode:MicroOp[]',
        '      addRmReg8',
        '    systemcode:MicroOp[]',
        '      lgdt',
        '',
        '  if state == 0',
        '    microPC <- microops.microcode.decode(0)',
        '  else',
        '    microPC <- microops.systemcode.decode(0)',
        '',
      ].join('\n');
      const { cst: _cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
    });

    it('parses a mix of shorthand and nested roms in one machine', () => {
      // Edge case: a machine can declare a shorthand rom alongside
      // a nested one. Both forms are first-class and coexist.
      const src = [
        'machine cpu',
        '  rom simpleRom:MicroOp[]',
        '    onlyRoutine',
        '',
        '  rom complexRom',
        '    primary:MicroOp[]',
        '      addRmReg8',
        '    secondary:MicroOp[]',
        '      lgdt',
        '',
      ].join('\n');
      const { cst: _cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
    });

    it('parses a rom declaration interleaved with other machine sections', () => {
      const src = [
        'machine executionUnit',
        '  registers',
        '    state:u8 = 0',
        '',
        '  rom microcode:MicroOp[]',
        '    addRmReg8',
        '',
        '  wires in',
        '    prefetch:b',
        '',
        '  wires out',
        '    valid:b',
        '',
        '  default',
        '    valid = 0',
        '',
      ].join('\n');
      const { cst: _cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
    });

    it('parses array typeRefs in field positions', () => {
      // Verify array type works elsewhere typeRef is consumed.
      const src = [
        'bundle Buffer',
        '  storage:MicroOp[]',
        '  count:u8',
        '',
      ].join('\n');
      const { cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
      expect(kinds(cst)).toEqual(['bundleDecl']);
    });

    it('parses an array of a parameterized type', () => {
      const src = [
        'bundle Foo',
        '  data:u{W}[]',
        '',
      ].join('\n');
      const { cst: _cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
    });

    it('still accepts `rom` as a regular identifier elsewhere', () => {
      // The Rom token is a soft keyword categorized as Identifier, so
      // existing uses of `rom` as a field/local name should still work.
      const src = 'bundle Foo\n  rom:u8\n  size:u16\n';
      const { cst, parseErrors } = parse(src);
      expect(parseErrors).toEqual([]);
      expect(getBundles(cst!)[0]!.fields).toEqual([
        { name: 'rom', type: 'u8' },
        { name: 'size', type: 'u16' },
      ]);
    });

    it('parses a machine with instance declarations mirroring executionUnit', () => {
      // Exercises the `instance` syntax in a realistic shape: a
      // stateful machine that instantiates several combinational
      // unit instances for its operand muxes and ALU stages, with
      // both inline and block-form port bindings.
      const src = [
        'machine executionUnit',
        '  registers',
        '    state:u8 = 0',
        '    ea:u16 = 0',
        '    tmp:u16 = 0',
        '    mdr:u16 = 0',
        '    imm:u16 = 0',
        '    ip:u16 = 0',
        '',
        '  wires in',
        '    prefetch:PrefetchOut',
        '',
        '  wires out',
        '    prefetchControl:PrefetchControl',
        '',
        '  default',
        '    prefetchControl = { pop: 0 }',
        '',
        '  instance regRead : registerSelect',
        '    sel = 0',
        '    width = Width.u16',
        '',
        '  instance aluSrcA : aluSelect',
        '    reg = regRead.out',
        '    imm = imm',
        '    tmp = tmp',
        '    mdr = mdr',
        '    ea  = ea',
        '    ip  = ip',
        '',
        '  instance aluSrcB : aluSelect',
        '    reg = regRead.out',
        '    imm = imm',
        '    tmp = tmp',
        '    mdr = mdr',
        '    ea  = ea',
        '    ip  = ip',
        '',
        '  instance mainAlu : alu::<u16>',
        '    cin = 0',
        '',
        '  rom microcode:MicroOp[]',
        '    addRmReg8',
        '',
      ].join('\n');
      const { cst, lexErrors, parseErrors } = parse(src);
      expect(lexErrors).toEqual([]);
      expect(parseErrors).toEqual([]);
      expect(kinds(cst)).toEqual(['machineDecl']);
      expect(names(cst)).toEqual(['executionUnit']);
    });

    it('parses the real core/ALU_NEW.machine file end to end', () => {
      // The acid test: can the parser consume the full hand-written
      // i286 design doc from core/ALU_NEW.machine without any lex or
      // parse errors? Every declaration kind, every section marker,
      // every expression form, and the full statement grammar get
      // exercised by this file.
      const aluPath = new URL(
        '../../../core/ALU_NEW.machine',
        import.meta.url,
      );
      const src = readFileSync(aluPath, 'utf-8');
      const { cst, lexErrors, parseErrors } = parse(src);
      expect(lexErrors).toEqual([]);
      expect(parseErrors).toEqual([]);
      expect(cst).toBeDefined();
      // Spot-check that the expected top-level declarations are all
      // present in source order. If the count or names drift because
      // the file was edited, this test will flag it.
      const decls = getDeclarations(cst!);
      expect(decls.length).toBeGreaterThan(40);
      expect(decls.map((d) => d.name)).toContain('addRmReg8');
      expect(decls.map((d) => d.name)).toContain('executionUnit');
      expect(decls.map((d) => d.name)).toContain('aluFlags');
      expect(decls.map((d) => d.name)).toContain('prefetcher');
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
