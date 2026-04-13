import { readFileSync } from 'node:fs';
import { lex } from './lex.js';
import {
  Unit,
  Identifier,
  StringLiteral,
} from './tokens.js';

const readSample = (name: string): string =>
  readFileSync(new URL(`./__samples__/${name}`, import.meta.url), 'utf-8');

const names = (tokens: { tokenType: { name: string } }[]): string[] =>
  tokens.map((t) => t.tokenType.name);

describe('lexer — basic tokenization', () => {
  it('recognizes top-level construct keywords', () => {
    const { tokens, errors } = lex('unit machine register enum bundle union');
    expect(errors).toEqual([]);
    expect(names(tokens)).toEqual([
      'Unit',
      'Machine',
      'Register',
      'Enum',
      'Bundle',
      'Union',
    ]);
  });

  it('recognizes microcode-related keywords', () => {
    const { tokens, errors } = lex('microword operand routine call fetch');
    expect(errors).toEqual([]);
    expect(names(tokens)).toEqual([
      'Microword',
      'Operand',
      'Routine',
      'Call',
      'Fetch',
    ]);
  });

  it('recognizes control-flow keywords', () => {
    const { tokens, errors } = lex('mux when if elif else assert');
    expect(errors).toEqual([]);
    expect(names(tokens)).toEqual([
      'Mux',
      'When',
      'If',
      'Elif',
      'Else',
      'Assert',
    ]);
  });

  it('distinguishes keyword prefixes from identifiers', () => {
    // `unitX` is an identifier, not the `unit` keyword followed by X.
    const { tokens, errors } = lex('unit unitX unit_thing');
    expect(errors).toEqual([]);
    expect(tokens[0].tokenType).toBe(Unit);
    expect(tokens[1].tokenType).toBe(Identifier);
    expect(tokens[1].image).toBe('unitX');
    expect(tokens[2].tokenType).toBe(Identifier);
    expect(tokens[2].image).toBe('unit_thing');
  });

  it('recognizes hex and decimal literals', () => {
    const { tokens, errors } = lex('0x00 0xFF 0xb0 42 0');
    expect(errors).toEqual([]);
    expect(names(tokens)).toEqual([
      'HexLiteral',
      'HexLiteral',
      'HexLiteral',
      'DecimalLiteral',
      'DecimalLiteral',
    ]);
    expect(tokens[0].image).toBe('0x00');
    expect(tokens[1].image).toBe('0xFF');
  });

  it('recognizes string literals including escaped quotes', () => {
    const { tokens, errors } = lex('"hello world" "with \\"quotes\\" in it"');
    expect(errors).toEqual([]);
    expect(tokens[0].tokenType).toBe(StringLiteral);
    expect(tokens[1].tokenType).toBe(StringLiteral);
  });

  it('lexes multi-char operators as single tokens', () => {
    const { tokens, errors } = lex('-> <- := == != <= >= && || << >> ..');
    expect(errors).toEqual([]);
    expect(names(tokens)).toEqual([
      'Arrow',
      'BackArrow',
      'ColonEqual',
      'EqualEqual',
      'NotEqual',
      'LessEqual',
      'GreaterEqual',
      'AndAnd',
      'OrOr',
      'ShiftLeft',
      'ShiftRight',
      'DotDot',
    ]);
  });

  it('does not confuse `->` with `-` followed by `>`', () => {
    const { tokens, errors } = lex('a -> b   a - > b');
    expect(errors).toEqual([]);
    const filtered = names(tokens);
    // First three: Identifier Arrow Identifier
    expect(filtered.slice(0, 3)).toEqual(['Identifier', 'Arrow', 'Identifier']);
    // Next four: Identifier Minus Greater Identifier
    expect(filtered.slice(3)).toEqual([
      'Identifier',
      'Minus',
      'Greater',
      'Identifier',
    ]);
  });

  it('skips line comments and horizontal whitespace', () => {
    const { tokens, errors } = lex('; leading comment\n  unit foo  ; trailing\n');
    expect(errors).toEqual([]);
    // The only real tokens are: Unit, Identifier, plus Newlines.
    const nonNewline = names(tokens).filter((n) => n !== 'Newline' && n !== 'Indent' && n !== 'Outdent');
    expect(nonNewline).toEqual(['Unit', 'Identifier']);
  });
});

describe('lexer — indent post-processor', () => {
  it('emits INDENT when a line is deeper than its predecessor', () => {
    const { tokens, errors } = lex('a\n  b\n');
    expect(errors).toEqual([]);
    expect(names(tokens)).toEqual([
      'Identifier', // a
      'Newline',
      'Indent',
      'Identifier', // b
      'Newline',
      'Outdent',
    ]);
  });

  it('emits OUTDENT when a line returns to a prior level', () => {
    const { tokens, errors } = lex('a\n  b\n    c\n  d\n');
    expect(errors).toEqual([]);
    expect(names(tokens)).toEqual([
      'Identifier', // a
      'Newline',
      'Indent',
      'Identifier', // b
      'Newline',
      'Indent',
      'Identifier', // c
      'Newline',
      'Outdent',
      'Identifier', // d
      'Newline',
      'Outdent',
    ]);
  });

  it('drains all remaining indents at end of file', () => {
    // No trailing newline; file ends mid-block. EOF drain must close both.
    const { tokens, errors } = lex('a\n  b\n    c');
    expect(errors).toEqual([]);
    const n = names(tokens);
    expect(n.filter((x) => x === 'Indent').length).toBe(2);
    expect(n.filter((x) => x === 'Outdent').length).toBe(2);
  });

  it('treats blank lines as indent-neutral', () => {
    const { tokens, errors } = lex('a\n\n  b\n\n  c\n');
    expect(errors).toEqual([]);
    const n = names(tokens);
    // Exactly one INDENT (into the b/c block) and one OUTDENT (at EOF).
    expect(n.filter((x) => x === 'Indent').length).toBe(1);
    expect(n.filter((x) => x === 'Outdent').length).toBe(1);
  });

  it('treats comment-only lines as indent-neutral', () => {
    const { tokens, errors } = lex('a\n  ; a comment\n  b\n');
    expect(errors).toEqual([]);
    const n = names(tokens);
    expect(n.filter((x) => x === 'Indent').length).toBe(1);
    expect(n.filter((x) => x === 'Outdent').length).toBe(1);
  });

  it('suppresses newlines inside braces', () => {
    const src = 'Foo { a: 1,\n      b: 2,\n      c: 3 }\n';
    const { tokens, errors } = lex(src);
    expect(errors).toEqual([]);
    const n = names(tokens);
    // No Newline tokens should appear between LBrace and the matching RBrace.
    const lBraceIdx = n.indexOf('LBrace');
    const rBraceIdx = n.indexOf('RBrace');
    expect(lBraceIdx).toBeGreaterThanOrEqual(0);
    expect(rBraceIdx).toBeGreaterThan(lBraceIdx);
    expect(n.slice(lBraceIdx, rBraceIdx)).not.toContain('Newline');
  });

  it('suppresses newlines inside parentheses', () => {
    const src = 'call foo(a,\n         b,\n         c)\n';
    const { tokens, errors } = lex(src);
    expect(errors).toEqual([]);
    const n = names(tokens);
    const lParenIdx = n.indexOf('LParen');
    const rParenIdx = n.indexOf('RParen');
    expect(lParenIdx).toBeGreaterThanOrEqual(0);
    expect(rParenIdx).toBeGreaterThan(lParenIdx);
    expect(n.slice(lParenIdx, rParenIdx)).not.toContain('Newline');
  });

  it('suppresses newlines inside brackets', () => {
    const src = 'a[0,\n  1,\n  2]\n';
    const { tokens, errors } = lex(src);
    expect(errors).toEqual([]);
    const n = names(tokens);
    const lIdx = n.indexOf('LBracket');
    const rIdx = n.indexOf('RBracket');
    expect(n.slice(lIdx, rIdx)).not.toContain('Newline');
  });

  it('resumes indent tracking after a closing bracket', () => {
    // After `}` on line 2, the next line's indent should matter again.
    const src = 'a\n  Foo { x: 1,\n        y: 2 }\n  b\n';
    const { tokens, errors } = lex(src);
    expect(errors).toEqual([]);
    // Should see one INDENT (into the a-block) and one OUTDENT (EOF).
    const n = names(tokens);
    expect(n.filter((x) => x === 'Indent').length).toBe(1);
    expect(n.filter((x) => x === 'Outdent').length).toBe(1);
  });

  it('handles an empty source', () => {
    const { tokens, errors } = lex('');
    expect(errors).toEqual([]);
    expect(tokens).toEqual([]);
  });

  it('handles a source with only comments', () => {
    const { tokens, errors } = lex('; just a comment\n; another\n');
    expect(errors).toEqual([]);
    expect(names(tokens)).toEqual(['Newline', 'Newline']);
  });
});

describe('lexer — sample files', () => {
  it('lexes minimal.machine without errors', () => {
    const { tokens, errors } = lex(readSample('minimal.machine'));
    expect(errors).toEqual([]);
    const n = names(tokens);
    expect(n).toContain('Enum');
    expect(n).toContain('Register');
    // Two top-level blocks, each one level deep → two INDENT/OUTDENT pairs.
    expect(n.filter((x) => x === 'Indent').length).toBe(2);
    expect(n.filter((x) => x === 'Outdent').length).toBe(2);
  });

  it('lexes nested.machine and produces the expected nesting depth', () => {
    const { tokens, errors } = lex(readSample('nested.machine'));
    expect(errors).toEqual([]);
    const n = names(tokens);
    // unit > mux x > when 0 > mux y > when 0/1 is five levels deep.
    // The deepest sub-tree produces 5 INDENTs and the outer structure drains
    // all of them by EOF.
    const indentCount = n.filter((x) => x === 'Indent').length;
    const outdentCount = n.filter((x) => x === 'Outdent').length;
    expect(indentCount).toBeGreaterThanOrEqual(4);
    expect(indentCount).toBe(outdentCount);
  });

  it('lexes brackets.machine with newlines suppressed inside braces', () => {
    const { tokens, errors } = lex(readSample('brackets.machine'));
    expect(errors).toEqual([]);
    const n = names(tokens);
    // The record literal `AluMicro { ... }` spans three physical lines, so
    // without suppression there would be newlines between LBrace and RBrace.
    // Find the first LBrace and verify the window is newline-free.
    const lBraceIdx = n.indexOf('LBrace');
    const rBraceIdx = n.indexOf('RBrace');
    expect(lBraceIdx).toBeGreaterThanOrEqual(0);
    expect(rBraceIdx).toBeGreaterThan(lBraceIdx);
    expect(n.slice(lBraceIdx, rBraceIdx)).not.toContain('Newline');
  });

  it('lexes operators.machine with every multi-char operator intact', () => {
    const { tokens, errors } = lex(readSample('operators.machine'));
    expect(errors).toEqual([]);
    const n = names(tokens);
    expect(n).toContain('ColonEqual');
    expect(n).toContain('EqualEqual');
    expect(n).toContain('NotEqual');
    expect(n).toContain('Arrow');
    expect(n).toContain('BackArrow');
    expect(n).toContain('DotDot');
  });
});
