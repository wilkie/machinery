import { parse } from '../parser/parse.js';
import { lowerFile } from './lower.js';
import {
  isBundleDeclaration,
  isEnumDeclaration,
  isUnionDeclaration,
  type BundleDeclaration,
  type EnumDeclaration,
  type File as AstFile,
  type UnionDeclaration,
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

  it('skips unsupported declaration kinds silently (first-slice coverage)', () => {
    // Routines, units, machines etc. are parseable but not yet
    // lowered; they're dropped from the AST's declaration list.
    const src = [
      'enum Foo',
      '  a',
      '',
      'routine fetchByte',
      '',
      'bundle Bar',
      '  x:b',
      '',
    ].join('\n');
    const file = lowerOrFail(src);
    expect(file.declarations.map((d) => d.kind)).toEqual([
      'EnumDeclaration',
      'BundleDeclaration',
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
