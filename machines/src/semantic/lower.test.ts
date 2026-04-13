import { parse } from '../parser/parse.js';
import { lowerFile } from './lower.js';
import {
  isBundleDeclaration,
  isEnumDeclaration,
  isUnionDeclaration,
  isRegisterDeclaration,
  isUnitDeclaration,
  isMachineDeclaration,
  isMicrowordDeclaration,
  isOperandDeclaration,
  isRoutineDeclaration,
  type BundleDeclaration,
  type EnumDeclaration,
  type File as AstFile,
  type MachineDeclaration,
  type MicrowordDeclaration,
  type OperandDeclaration,
  type RegisterDeclaration,
  type RoutineDeclaration,
  type UnionDeclaration,
  type UnitDeclaration,
} from './ast.js';

/**
 * Parse and lower a source string, asserting both phases are clean.
 * Returns the lowered AST root for assertion access.
 */
function lowerOrFail(source: string): AstFile {
  const { cst, lexErrors, parseErrors } = parse(source);
  expect(lexErrors).toEqual([]);
  expect(parseErrors).toEqual([]);
  expect(cst).toBeDefined();
  return lowerFile(cst!);
}

describe('ast lowering — file-level shape', () => {
  it('lowers an empty source to a File node with no declarations', () => {
    const file = lowerOrFail('');
    expect(file.kind).toBe('File');
    expect(file.declarations).toEqual([]);
  });

  it('lowers a single-declaration source to one child', () => {
    const file = lowerOrFail('enum Foo\n  a\n  b\n');
    expect(file.declarations).toHaveLength(1);
    expect(file.declarations[0]!.kind).toBe('EnumDeclaration');
  });

  it('preserves source order across multiple declarations', () => {
    const src = [
      'enum Foo',
      '  a',
      '',
      'bundle Bar',
      '  x:b',
      '',
      'union Baz',
      '  a:Foo',
      '',
    ].join('\n');
    const file = lowerOrFail(src);
    expect(file.declarations.map((d) => d.kind)).toEqual([
      'EnumDeclaration',
      'BundleDeclaration',
      'UnionDeclaration',
    ]);
    expect(file.declarations.map((d) => (d as { name: string }).name)).toEqual([
      'Foo',
      'Bar',
      'Baz',
    ]);
  });

  it('lowers every declaration kind in source order', () => {
    const src = [
      'enum Foo',
      '  a',
      '',
      'bundle Bar',
      '  x:b',
      '',
      'union Baz',
      '  a:Foo',
      '',
      'register AX:u16',
      '',
      'unit alu',
      '',
      'machine cpu',
      '',
      'microword Mw',
      '  fields',
      '    x:u8',
      '',
      'operand Imm',
      '  size: 1',
      '',
      'routine fetchByte',
      '',
    ].join('\n');
    const file = lowerOrFail(src);
    expect(file.declarations.map((d) => d.kind)).toEqual([
      'EnumDeclaration',
      'BundleDeclaration',
      'UnionDeclaration',
      'RegisterDeclaration',
      'UnitDeclaration',
      'MachineDeclaration',
      'MicrowordDeclaration',
      'OperandDeclaration',
      'RoutineDeclaration',
    ]);
  });
});

describe('ast lowering — enum declarations', () => {
  it('lowers a single-variant enum', () => {
    const file = lowerOrFail('enum Solo\n  only\n');
    const decl = file.declarations[0] as EnumDeclaration;
    expect(decl.kind).toBe('EnumDeclaration');
    expect(decl.name).toBe('Solo');
    expect(decl.variants.map((v) => v.name)).toEqual(['only']);
  });

  it('lowers a multi-variant enum', () => {
    const file = lowerOrFail('enum Op\n  add\n  sub\n  and\n  or\n');
    const decl = file.declarations[0] as EnumDeclaration;
    expect(decl.variants.map((v) => v.name)).toEqual([
      'add',
      'sub',
      'and',
      'or',
    ]);
  });

  it('lowers an empty enum to variants: []', () => {
    const file = lowerOrFail('enum Empty\n');
    const decl = file.declarations[0] as EnumDeclaration;
    expect(decl.variants).toEqual([]);
  });

  it('handles `fetch` as an enum variant name via the contextual-keyword path', () => {
    const file = lowerOrFail('enum State\n  fetch\n  execute\n');
    const decl = file.declarations[0] as EnumDeclaration;
    expect(decl.variants.map((v) => v.name)).toEqual(['fetch', 'execute']);
  });

  it('tolerates comments interleaved with variants', () => {
    const src = [
      'enum Foo',
      '  ; first',
      '  a',
      '  ; second',
      '  b',
      '',
    ].join('\n');
    const file = lowerOrFail(src);
    const decl = file.declarations[0] as EnumDeclaration;
    expect(decl.variants.map((v) => v.name)).toEqual(['a', 'b']);
  });

  it('attaches source locations to the enum node and each variant', () => {
    const file = lowerOrFail('enum Foo\n  a\n  b\n');
    const decl = file.declarations[0] as EnumDeclaration;
    expect(decl.loc.startLine).toBe(1);
    expect(decl.loc.startColumn).toBe(1);
    expect(decl.variants[0]!.loc.startLine).toBe(2);
    expect(decl.variants[1]!.loc.startLine).toBe(3);
  });
});

describe('ast lowering — bundle declarations', () => {
  it('lowers a simple bundle with bit fields', () => {
    const src = 'bundle Flags\n  cf:b\n  zf:b\n  sf:b\n';
    const file = lowerOrFail(src);
    const decl = file.declarations[0] as BundleDeclaration;
    expect(decl.kind).toBe('BundleDeclaration');
    expect(decl.name).toBe('Flags');
    expect(decl.fields.map((f) => f.name)).toEqual(['cf', 'zf', 'sf']);
    expect(decl.fields.map((f) => f.type.name)).toEqual(['b', 'b', 'b']);
  });

  it('lowers a bundle with mixed-width fields and user types', () => {
    const src = [
      'bundle BusRequest',
      '  valid:b',
      '  address:u20',
      '  op:BusOp',
      '  size:BusSize',
      '  data:u16',
      '',
    ].join('\n');
    const file = lowerOrFail(src);
    const decl = file.declarations[0] as BundleDeclaration;
    expect(decl.fields.map((f) => `${f.name}:${f.type.name}`)).toEqual([
      'valid:b',
      'address:u20',
      'op:BusOp',
      'size:BusSize',
      'data:u16',
    ]);
  });

  it('lowers an empty bundle to fields: []', () => {
    const file = lowerOrFail('bundle Empty\n');
    const decl = file.declarations[0] as BundleDeclaration;
    expect(decl.fields).toEqual([]);
  });

  it('every field has a location pointing at its source line', () => {
    const src = 'bundle Foo\n  a:b\n  b:u8\n  c:u16\n';
    const file = lowerOrFail(src);
    const decl = file.declarations[0] as BundleDeclaration;
    expect(decl.fields.map((f) => f.loc.startLine)).toEqual([2, 3, 4]);
  });
});

describe('ast lowering — union declarations', () => {
  it('lowers a union of microword variants', () => {
    const src = [
      'union MicroOp',
      '  alu:AluMicro',
      '  istreamRead:IStreamRead',
      '  busRead:BusRead',
      '  retire:Retire',
      '',
    ].join('\n');
    const file = lowerOrFail(src);
    const decl = file.declarations[0] as UnionDeclaration;
    expect(decl.kind).toBe('UnionDeclaration');
    expect(decl.name).toBe('MicroOp');
    expect(decl.arms.map((a) => `${a.name}:${a.type.name}`)).toEqual([
      'alu:AluMicro',
      'istreamRead:IStreamRead',
      'busRead:BusRead',
      'retire:Retire',
    ]);
  });

  it('lowers an empty union', () => {
    const file = lowerOrFail('union Empty\n');
    const decl = file.declarations[0] as UnionDeclaration;
    expect(decl.arms).toEqual([]);
  });
});

describe('ast lowering — register declarations', () => {
  it('lowers a bare register', () => {
    const file = lowerOrFail('register SI:u16\n');
    const decl = file.declarations[0] as RegisterDeclaration;
    expect(decl.kind).toBe('RegisterDeclaration');
    expect(decl.name).toBe('SI');
    expect(decl.type.name).toBe('u16');
    expect(decl.fields).toEqual([]);
  });

  it('lowers a register with bit fields', () => {
    const src = 'register AX:u16\n  field AL:u8 @ 0\n  field AH:u8 @ 8\n';
    const file = lowerOrFail(src);
    const decl = file.declarations[0] as RegisterDeclaration;
    expect(decl.fields.map((f) => ({
      name: f.name,
      type: f.type.name,
      offset: f.offset,
    }))).toEqual([
      { name: 'AL', type: 'u8', offset: 0 },
      { name: 'AH', type: 'u8', offset: 8 },
    ]);
  });

  it('lowers a register with non-contiguous offsets', () => {
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
    const file = lowerOrFail(src);
    const decl = file.declarations[0] as RegisterDeclaration;
    expect(decl.fields.map((f) => f.offset)).toEqual([0, 2, 4, 6, 7, 11]);
  });

  it('lowers a field with a hex offset', () => {
    const src = 'register R:u32\n  field lo:u8 @ 0x0\n  field hi:u8 @ 0x10\n';
    const file = lowerOrFail(src);
    const decl = file.declarations[0] as RegisterDeclaration;
    expect(decl.fields.map((f) => f.offset)).toEqual([0, 16]);
  });

  it('lowers a field with no offset clause to offset: undefined', () => {
    const src = 'register Foo:u8\n  field x:b\n';
    const file = lowerOrFail(src);
    const decl = file.declarations[0] as RegisterDeclaration;
    expect(decl.fields[0]!.offset).toBeUndefined();
  });
});

describe('ast lowering — unit declarations', () => {
  it('lowers a unit with no type parameters', () => {
    const file = lowerOrFail('unit alu\n');
    const decl = file.declarations[0] as UnitDeclaration;
    expect(decl.kind).toBe('UnitDeclaration');
    expect(decl.name).toBe('alu');
    expect(decl.typeParams).toEqual([]);
  });

  it('lowers a unit with a single type parameter', () => {
    const file = lowerOrFail('unit alu<W:Width>\n');
    const decl = file.declarations[0] as UnitDeclaration;
    expect(decl.typeParams.map((p) => ({
      name: p.name,
      type: p.type.name,
    }))).toEqual([{ name: 'W', type: 'Width' }]);
  });

  it('lowers a unit with multiple type parameters', () => {
    const file = lowerOrFail('unit foo<W:Width, T:AluOp>\n');
    const decl = file.declarations[0] as UnitDeclaration;
    expect(decl.typeParams.map((p) => p.name)).toEqual(['W', 'T']);
    expect(decl.typeParams.map((p) => p.type.name)).toEqual(['Width', 'AluOp']);
  });
});

describe('ast lowering — machine declarations', () => {
  it('lowers a bare machine', () => {
    const file = lowerOrFail('machine cpu\n');
    const decl = file.declarations[0] as MachineDeclaration;
    expect(decl.kind).toBe('MachineDeclaration');
    expect(decl.name).toBe('cpu');
    expect(decl.id).toBeUndefined();
  });

  it('lowers a machine with an id tag', () => {
    const src = ['machine executionUnit', '  id i286', ''].join('\n');
    const file = lowerOrFail(src);
    const decl = file.declarations[0] as MachineDeclaration;
    expect(decl.name).toBe('executionUnit');
    expect(decl.id).toBe('i286');
  });

  it('lowers a machine with a body containing other sections (id still found)', () => {
    const src = [
      'machine cpu',
      '  id mychip',
      '  registers',
      '    state:u8 = 0',
      '',
      '  wires in',
      '    valid:b',
      '',
    ].join('\n');
    const file = lowerOrFail(src);
    const decl = file.declarations[0] as MachineDeclaration;
    expect(decl.id).toBe('mychip');
  });
});

describe('ast lowering — microword declarations', () => {
  it('lowers a microword with no body', () => {
    const file = lowerOrFail('microword Empty\n');
    const decl = file.declarations[0] as MicrowordDeclaration;
    expect(decl.kind).toBe('MicrowordDeclaration');
    expect(decl.name).toBe('Empty');
    expect(decl.fields).toEqual([]);
  });

  it('lowers a microword with block-form fields', () => {
    const src = [
      'microword AluMicro',
      '  fields',
      '    op:AluOp',
      '    width:Width',
      '    dest:AluDest',
      '',
    ].join('\n');
    const file = lowerOrFail(src);
    const decl = file.declarations[0] as MicrowordDeclaration;
    expect(decl.fields.map((f) => `${f.name}:${f.type.name}`)).toEqual([
      'op:AluOp',
      'width:Width',
      'dest:AluDest',
    ]);
  });

  it('lowers a microword with inline empty fields (Retire case)', () => {
    const src = 'microword Retire\n  fields {}\n';
    const file = lowerOrFail(src);
    const decl = file.declarations[0] as MicrowordDeclaration;
    expect(decl.fields).toEqual([]);
  });

  it('lowers a microword with inline fields', () => {
    const src = 'microword Foo\n  fields { a:u8, b:u16 }\n';
    const file = lowerOrFail(src);
    const decl = file.declarations[0] as MicrowordDeclaration;
    expect(decl.fields.map((f) => f.name)).toEqual(['a', 'b']);
  });

  it('ignores ready/terminal/effect sections (deferred)', () => {
    // These sections are parsed but not yet lowered; the AST for
    // now only surfaces the `fields` section.
    const src = [
      'microword IStreamRead',
      '  fields',
      '    dest:Local',
      '  ready: prefetch.valid',
      '  terminal: 0',
      '  effect',
      '    dest <- prefetch.byte',
      '',
    ].join('\n');
    const file = lowerOrFail(src);
    const decl = file.declarations[0] as MicrowordDeclaration;
    expect(decl.name).toBe('IStreamRead');
    expect(decl.fields.map((f) => f.name)).toEqual(['dest']);
  });
});

describe('ast lowering — operand declarations', () => {
  it('lowers a bare operand', () => {
    const file = lowerOrFail('operand Empty\n');
    const decl = file.declarations[0] as OperandDeclaration;
    expect(decl.kind).toBe('OperandDeclaration');
    expect(decl.name).toBe('Empty');
    expect(decl.size).toBeUndefined();
    expect(decl.fields).toEqual([]);
  });

  it('lowers an operand with size and bit-offset fields', () => {
    const src = [
      'operand ModRM',
      '  size: 1',
      '  fields',
      '    mod:u2 @ 6',
      '    reg:u3 @ 3',
      '    rm:u3 @ 0',
      '',
    ].join('\n');
    const file = lowerOrFail(src);
    const decl = file.declarations[0] as OperandDeclaration;
    expect(decl.size).toBe(1);
    expect(decl.fields.map((f) => ({
      name: f.name,
      type: f.type.name,
      offset: f.offset,
    }))).toEqual([
      { name: 'mod', type: 'u2', offset: 6 },
      { name: 'reg', type: 'u3', offset: 3 },
      { name: 'rm', type: 'u3', offset: 0 },
    ]);
  });

  it('lowers size with a hex literal', () => {
    const file = lowerOrFail('operand Foo\n  size: 0x02\n');
    const decl = file.declarations[0] as OperandDeclaration;
    expect(decl.size).toBe(2);
  });
});

describe('ast lowering — routine declarations', () => {
  it('lowers a bare routine with no body', () => {
    const file = lowerOrFail('routine foo\n');
    const decl = file.declarations[0] as RoutineDeclaration;
    expect(decl.kind).toBe('RoutineDeclaration');
    expect(decl.name).toBe('foo');
    expect(decl.params).toEqual([]);
    expect(decl.returnType).toBeUndefined();
    expect(decl.entry).toBeUndefined();
    expect(decl.allow).toEqual([]);
    expect(decl.modifies).toEqual([]);
    expect(decl.references).toEqual([]);
  });

  it('lowers a routine with parameters and return type', () => {
    const file = lowerOrFail('routine eaCalc(mod:u2, rm:u3) -> (ea:u16)\n');
    const decl = file.declarations[0] as RoutineDeclaration;
    expect(decl.name).toBe('eaCalc');
    expect(decl.params.map((p) => `${p.name}:${p.type.name}`)).toEqual([
      'mod:u2',
      'rm:u3',
    ]);
    expect(decl.returnType).toBeDefined();
    expect(decl.returnType!.map((r) => `${r.name}:${r.type.name}`)).toEqual([
      'ea:u16',
    ]);
  });

  it('lowers a routine with an empty return type', () => {
    const file = lowerOrFail('routine foo -> ()\n');
    const decl = file.declarations[0] as RoutineDeclaration;
    expect(decl.returnType).toEqual([]);
  });

  it('lowers an entry section with a decimal literal', () => {
    const file = lowerOrFail('routine foo\n  entry: 42\n');
    const decl = file.declarations[0] as RoutineDeclaration;
    expect(decl.entry).toBe(42);
  });

  it('lowers an entry section with a hex literal', () => {
    const file = lowerOrFail('routine foo\n  entry: 0xB0\n');
    const decl = file.declarations[0] as RoutineDeclaration;
    expect(decl.entry).toBe(0xb0);
  });

  it('returns undefined for a complex entry expression', () => {
    const file = lowerOrFail('routine foo\n  entry: 0x80 + 1\n');
    const decl = file.declarations[0] as RoutineDeclaration;
    expect(decl.entry).toBeUndefined();
  });

  it('lowers allow and modifies lists', () => {
    const src = [
      'routine foo',
      '  allow: [lock]',
      '  modifies: [OF, SF, ZF, AF, PF, CF]',
      '',
    ].join('\n');
    const file = lowerOrFail(src);
    const decl = file.declarations[0] as RoutineDeclaration;
    expect(decl.allow).toEqual(['lock']);
    expect(decl.modifies).toEqual(['OF', 'SF', 'ZF', 'AF', 'PF', 'CF']);
  });

  it('lowers a references block with quoted strings', () => {
    const src = [
      'routine foo',
      '  references',
      '    - "Intel 80286 Programmer\'s Reference Manual, ADD"',
      '    - "a second reference"',
      '',
    ].join('\n');
    const file = lowerOrFail(src);
    const decl = file.declarations[0] as RoutineDeclaration;
    expect(decl.references).toEqual([
      "Intel 80286 Programmer's Reference Manual, ADD",
      'a second reference',
    ]);
  });

  it('lowers a fully-loaded routine with every metadata section', () => {
    const src = [
      'routine addRmReg8',
      '  description',
      '    Integer addition.',
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
    const file = lowerOrFail(src);
    const decl = file.declarations[0] as RoutineDeclaration;
    expect(decl).toMatchObject({
      kind: 'RoutineDeclaration',
      name: 'addRmReg8',
      entry: 0,
      allow: ['lock'],
      modifies: ['OF', 'SF', 'ZF', 'AF', 'PF', 'CF'],
      references: ["Intel 80286 Programmer's Reference Manual, ADD"],
    });
  });
});

describe('ast lowering — EXEMPLAR.machine integration', () => {
  it('lowers the real EXEMPLAR.machine file without errors', async () => {
    // The acid test for AST lowering: read the full design file,
    // parse and lower it, and verify that every declaration kind
    // shows up in the AST.
    const { readFileSync } = await import('node:fs');
    const aluPath = new URL(
      '../../EXEMPLAR.machine',
      import.meta.url,
    );
    const src = readFileSync(aluPath, 'utf-8');
    const file = lowerOrFail(src);

    // The file has many declarations of various kinds; verify at
    // least one of each kind we've lowered shows up.
    const kinds = new Set(file.declarations.map((d) => d.kind));
    expect(kinds).toContain('EnumDeclaration');
    expect(kinds).toContain('BundleDeclaration');
    expect(kinds).toContain('UnionDeclaration');
    expect(kinds).toContain('RegisterDeclaration');
    expect(kinds).toContain('UnitDeclaration');
    expect(kinds).toContain('MachineDeclaration');
    expect(kinds).toContain('MicrowordDeclaration');
    expect(kinds).toContain('OperandDeclaration');
    expect(kinds).toContain('RoutineDeclaration');

    // Spot-check: `addRmReg8` should round-trip with entry: 0x00
    // and the full metadata.
    const addRmReg8 = file.declarations
      .filter(isRoutineDeclaration)
      .find((r) => r.name === 'addRmReg8');
    expect(addRmReg8).toBeDefined();
    expect(addRmReg8!.entry).toBe(0);
    expect(addRmReg8!.allow).toEqual(['lock']);

    // Spot-check: `executionUnit` machine should have id 'i286'.
    const executionUnit = file.declarations
      .filter(isMachineDeclaration)
      .find((m) => m.name === 'executionUnit');
    expect(executionUnit).toBeDefined();
    expect(executionUnit!.id).toBe('i286');

    // Spot-check: `AluMicro` microword should lower with its
    // seven field declarations.
    const aluMicro = file.declarations
      .filter(isMicrowordDeclaration)
      .find((m) => m.name === 'AluMicro');
    expect(aluMicro).toBeDefined();
    expect(aluMicro!.fields.map((f) => f.name)).toEqual([
      'op',
      'width',
      'srcA',
      'srcB',
      'dest',
      'commit',
      'writeFlags',
    ]);

    // Spot-check: `ModRM` operand should lower with size=1 and
    // three bit-offset fields.
    const modrm = file.declarations
      .filter(isOperandDeclaration)
      .find((o) => o.name === 'ModRM');
    expect(modrm).toBeDefined();
    expect(modrm!.size).toBe(1);
    expect(modrm!.fields.map((f) => ({
      name: f.name,
      offset: f.offset,
    }))).toEqual([
      { name: 'mod', offset: 6 },
      { name: 'reg', offset: 3 },
      { name: 'rm', offset: 0 },
    ]);

    // Spot-check: `AX` register should lower with its AL/AH fields.
    const ax = file.declarations
      .filter(isRegisterDeclaration)
      .find((r) => r.name === 'AX');
    expect(ax).toBeDefined();
    expect(ax!.fields.map((f) => ({
      name: f.name,
      offset: f.offset,
    }))).toEqual([
      { name: 'AL', offset: 0 },
      { name: 'AH', offset: 8 },
    ]);

    // Spot-check: `aluFlags` unit should lower with its `<W:Width>`
    // type parameter.
    const aluFlags = file.declarations
      .filter(isUnitDeclaration)
      .find((u) => u.name === 'aluFlags');
    expect(aluFlags).toBeDefined();
    expect(aluFlags!.typeParams.map((p) => `${p.name}:${p.type.name}`)).toEqual(
      ['W:Width'],
    );
  });
});

describe('ast lowering — discriminator predicates', () => {
  it('isEnumDeclaration narrows correctly', () => {
    const src = [
      'enum E\n  a',
      'bundle B\n  x:b',
      'union U\n  a:E',
      '',
    ].join('\n');
    const file = lowerOrFail(src);
    const enums = file.declarations.filter(isEnumDeclaration);
    expect(enums).toHaveLength(1);
    expect(enums[0]!.name).toBe('E');
  });

  it('isBundleDeclaration narrows correctly', () => {
    const src = 'enum E\n  a\nbundle B\n  x:b\n';
    const file = lowerOrFail(src);
    const bundles = file.declarations.filter(isBundleDeclaration);
    expect(bundles).toHaveLength(1);
    expect(bundles[0]!.name).toBe('B');
  });

  it('isUnionDeclaration narrows correctly', () => {
    const src = 'union U\n  a:X\n  b:Y\n';
    const file = lowerOrFail(src);
    const unions = file.declarations.filter(isUnionDeclaration);
    expect(unions).toHaveLength(1);
    expect(unions[0]!.arms.map((a) => a.name)).toEqual(['a', 'b']);
  });
});
