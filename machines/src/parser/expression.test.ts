import type { CstNode, CstElement, IToken } from 'chevrotain';
import { parseExpression } from './parse.js';

/**
 * Parse an expression and fail the test with a readable error if lexer
 * or parser errors were produced. Returns the root CST node.
 */
function parseOrFail(source: string): CstNode {
  const { cst, lexErrors, parseErrors } = parseExpression(source);
  expect(lexErrors).toEqual([]);
  expect(parseErrors).toEqual([]);
  expect(cst).toBeDefined();
  return cst!;
}

/**
 * Parse an expression that is EXPECTED to produce parse errors. Used
 * for negative cast tests where a loose colon should not be consumed
 * as a cast — leaving the `: u8` unconsumed triggers Chevrotain's
 * NotAllInputParsedException by design.
 */
function expectParseErrors(source: string): void {
  const { lexErrors, parseErrors } = parseExpression(source);
  expect(lexErrors).toEqual([]);
  expect(parseErrors.length).toBeGreaterThan(0);
}

/**
 * Walk an expression CST and collapse it to a Lisp-style S-expression
 * string. Binary operators with only one operand collapse to the inner
 * expression (`(+ a)` → `a`) so that precedence shows up structurally
 * in the output.
 *
 * This is intentionally minimal: just enough to make precedence and
 * associativity tests readable. If a more full-featured serialization
 * becomes useful, expand this alongside the grammar.
 */
function toSexp(node: CstElement): string {
  if (!isCstNode(node)) {
    // A plain token: return its image.
    return node.image;
  }

  switch (node.name) {
    case 'expression':
      return toSexp(firstChild(node, 'logicalOrExpr'));

    case 'logicalOrExpr':
      return binaryChain(node, 'logicalAndExpr', ['OrOr']);
    case 'logicalAndExpr':
      return binaryChain(node, 'bitOrExpr', ['AndAnd']);
    case 'bitOrExpr':
      return binaryChain(node, 'bitXorExpr', ['Pipe']);
    case 'bitXorExpr':
      return binaryChain(node, 'bitAndExpr', ['Caret']);
    case 'bitAndExpr':
      return binaryChain(node, 'equalityExpr', ['Amp']);
    case 'equalityExpr':
      return binaryChain(node, 'relationalExpr', ['EqualEqual', 'NotEqual']);
    case 'relationalExpr':
      return binaryChain(node, 'shiftExpr', [
        'LessEqual',
        'GreaterEqual',
        'Less',
        'Greater',
      ]);
    case 'shiftExpr':
      return binaryChain(node, 'additiveExpr', ['ShiftLeft', 'ShiftRight']);
    case 'additiveExpr':
      return binaryChain(node, 'multiplicativeExpr', ['Plus', 'Minus']);
    case 'multiplicativeExpr':
      return binaryChain(node, 'unaryExpr', ['Star', 'Slash']);

    case 'unaryExpr': {
      // Unary form: operator followed by a recursive unaryExpr.
      // Non-unary form: falls through to postfixExpr.
      for (const op of ['Bang', 'Tilde', 'Minus'] as const) {
        const opTok = firstTokenOrUndefined(node, op);
        if (opTok) {
          return `(${opTok.image} ${toSexp(firstChild(node, 'unaryExpr'))})`;
        }
      }
      return toSexp(firstChild(node, 'postfixExpr'));
    }

    case 'postfixExpr': {
      let out = toSexp(firstChild(node, 'primaryExpr'));
      const ops = orderedChildren(node, [
        'memberOp',
        'callOp',
        'indexOp',
        'castOp',
      ]);
      for (const op of ops) {
        if (op.name === 'memberOp') {
          const name = firstToken(op, 'Identifier').image;
          out = `(. ${out} ${name})`;
        } else if (op.name === 'callOp') {
          const args = asCstNodes(op.children['expression']).map(toSexp);
          out = args.length > 0
            ? `(call ${out} ${args.join(' ')})`
            : `(call ${out})`;
        } else if (op.name === 'indexOp') {
          const args = asCstNodes(op.children['expression']).map(toSexp);
          out = args.length === 1
            ? `(index ${out} ${args[0]})`
            : `(slice ${out} ${args[0]} ${args[1]})`;
        } else {
          // castOp
          const typeNode = firstChild(op, 'typeRef');
          const typeName = firstToken(typeNode, 'Identifier').image;
          out = `(cast ${out} ${typeName})`;
        }
      }
      return out;
    }

    case 'primaryExpr': {
      const dec = firstTokenOrUndefined(node, 'DecimalLiteral');
      if (dec) return dec.image;
      const hex = firstTokenOrUndefined(node, 'HexLiteral');
      if (hex) return hex.image;
      const str = firstTokenOrUndefined(node, 'StringLiteral');
      if (str) return str.image;
      const ident = firstTokenOrUndefined(node, 'Identifier');
      if (ident) return ident.image;
      const paren = firstChildOrUndefined(node, 'parenExpr');
      if (paren) return toSexp(paren);
      const rec = firstChildOrUndefined(node, 'recordLiteral');
      if (rec) return toSexp(rec);
      return '?';
    }

    case 'parenExpr':
      return toSexp(firstChild(node, 'expression'));

    case 'recordLiteral': {
      const fields = asCstNodes(node.children['recordField']).map(toSexp);
      return fields.length === 0 ? '{}' : `{${fields.join(' ')}}`;
    }

    case 'recordField': {
      const name = firstToken(node, 'Identifier').image;
      const value = toSexp(firstChild(node, 'expression'));
      return `${name}=${value}`;
    }

    default:
      return `<${node.name}>`;
  }
}

/**
 * Walk a binary-chain rule (e.g., additiveExpr) and produce a
 * left-associative S-expression. If the rule has only one operand (no
 * operator siblings), the result is the bare operand — precedence
 * collapses show up this way.
 */
function binaryChain(
  node: CstNode,
  operandRule: string,
  opTokens: string[],
): string {
  const operands = asCstNodes(node.children[operandRule]);
  if (operands.length === 0) return '?';
  let out = toSexp(operands[0]!);
  // Collect the operators in source order, interleaved with operands.
  const ops: IToken[] = [];
  for (const tokName of opTokens) {
    const toks = asTokens(node.children[tokName]);
    ops.push(...toks);
  }
  ops.sort((a, b) => (a.startOffset ?? 0) - (b.startOffset ?? 0));
  for (let i = 0; i < ops.length; i++) {
    const op = ops[i]!;
    const rhs = operands[i + 1];
    if (!rhs) break;
    out = `(${op.image} ${out} ${toSexp(rhs)})`;
  }
  return out;
}

function orderedChildren(node: CstNode, ruleNames: string[]): CstNode[] {
  const all: CstNode[] = [];
  for (const name of ruleNames) {
    all.push(...asCstNodes(node.children[name]));
  }
  all.sort((a, b) => {
    const aOff = firstOffset(a);
    const bOff = firstOffset(b);
    return aOff - bOff;
  });
  return all;
}

function firstOffset(node: CstNode): number {
  for (const children of Object.values(node.children)) {
    for (const c of children) {
      if (isCstNode(c)) {
        return firstOffset(c);
      }
      return (c as IToken).startOffset ?? 0;
    }
  }
  return 0;
}

function firstChild(node: CstNode, name: string): CstNode {
  const nodes = asCstNodes(node.children[name]);
  if (nodes.length === 0) {
    throw new Error(`expected child ${name} on ${node.name}`);
  }
  return nodes[0]!;
}

function firstChildOrUndefined(
  node: CstNode,
  name: string,
): CstNode | undefined {
  return asCstNodes(node.children[name])[0];
}

function firstToken(node: CstNode, name: string): IToken {
  const toks = asTokens(node.children[name]);
  if (toks.length === 0) {
    throw new Error(`expected token ${name} on ${node.name}`);
  }
  return toks[0]!;
}

function firstTokenOrUndefined(
  node: CstNode,
  name: string,
): IToken | undefined {
  return asTokens(node.children[name])[0];
}

function isCstNode(x: CstElement): x is CstNode {
  return (x as CstNode).children !== undefined;
}

function asCstNodes(children: CstElement[] | undefined): CstNode[] {
  if (!children) return [];
  return children.filter(isCstNode);
}

function asTokens(children: CstElement[] | undefined): IToken[] {
  if (!children) return [];
  return children.filter((c): c is IToken => !isCstNode(c));
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe('expression grammar — primary', () => {
  it('parses decimal literals', () => {
    expect(toSexp(parseOrFail('42'))).toBe('42');
  });

  it('parses hex literals', () => {
    expect(toSexp(parseOrFail('0xFF'))).toBe('0xFF');
  });

  it('parses string literals', () => {
    expect(toSexp(parseOrFail('"hello"'))).toBe('"hello"');
  });

  it('parses identifiers', () => {
    expect(toSexp(parseOrFail('foo'))).toBe('foo');
  });

  it('parses parenthesized expressions transparently', () => {
    expect(toSexp(parseOrFail('(42)'))).toBe('42');
    expect(toSexp(parseOrFail('((a))'))).toBe('a');
  });

  it('parses empty record literals', () => {
    expect(toSexp(parseOrFail('{}'))).toBe('{}');
  });

  it('parses single-field record literals', () => {
    expect(toSexp(parseOrFail('{a: 1}'))).toBe('{a=1}');
  });

  it('parses multi-field record literals', () => {
    expect(toSexp(parseOrFail('{a: 1, b: 2, c: 3}'))).toBe('{a=1 b=2 c=3}');
  });
});

describe('expression grammar — postfix', () => {
  it('parses member access', () => {
    expect(toSexp(parseOrFail('a.b'))).toBe('(. a b)');
  });

  it('parses chained member access', () => {
    expect(toSexp(parseOrFail('a.b.c'))).toBe('(. (. a b) c)');
  });

  it('parses zero-argument function calls', () => {
    expect(toSexp(parseOrFail('foo()'))).toBe('(call foo)');
  });

  it('parses single-argument function calls', () => {
    expect(toSexp(parseOrFail('foo(x)'))).toBe('(call foo x)');
  });

  it('parses multi-argument function calls', () => {
    expect(toSexp(parseOrFail('foo(a, b, c)'))).toBe('(call foo a b c)');
  });

  it('parses single-bit indexing', () => {
    expect(toSexp(parseOrFail('raw[7]'))).toBe('(index raw 7)');
  });

  it('parses bit-slice indexing', () => {
    expect(toSexp(parseOrFail('disp[15:0]'))).toBe('(slice disp 15 0)');
  });

  it('parses indexing with an expression subscript', () => {
    expect(toSexp(parseOrFail('raw[W + 1]'))).toBe('(index raw (+ W 1))');
  });

  it('chains member, call, and index operators left-to-right', () => {
    expect(toSexp(parseOrFail('a.b[0]'))).toBe('(index (. a b) 0)');
    expect(toSexp(parseOrFail('foo(x).bar'))).toBe('(. (call foo x) bar)');
    expect(toSexp(parseOrFail('a.b.c(1)'))).toBe('(call (. (. a b) c) 1)');
  });
});

describe('expression grammar — unary', () => {
  it('parses logical not', () => {
    expect(toSexp(parseOrFail('!a'))).toBe('(! a)');
  });

  it('parses bitwise not', () => {
    expect(toSexp(parseOrFail('~a'))).toBe('(~ a)');
  });

  it('parses unary minus', () => {
    expect(toSexp(parseOrFail('-a'))).toBe('(- a)');
  });

  it('stacks multiple unary operators', () => {
    expect(toSexp(parseOrFail('!!a'))).toBe('(! (! a))');
    expect(toSexp(parseOrFail('~-a'))).toBe('(~ (- a))');
  });

  it('binds unary tighter than binary', () => {
    expect(toSexp(parseOrFail('!a && b'))).toBe('(&& (! a) b)');
    expect(toSexp(parseOrFail('-a + b'))).toBe('(+ (- a) b)');
  });
});

describe('expression grammar — binary precedence', () => {
  it('additive is left-associative', () => {
    expect(toSexp(parseOrFail('a + b + c'))).toBe('(+ (+ a b) c)');
  });

  it('multiplicative is left-associative', () => {
    expect(toSexp(parseOrFail('a * b / c'))).toBe('(/ (* a b) c)');
  });

  it('multiplicative binds tighter than additive', () => {
    expect(toSexp(parseOrFail('a + b * c'))).toBe('(+ a (* b c))');
    expect(toSexp(parseOrFail('a * b + c'))).toBe('(+ (* a b) c)');
  });

  it('additive binds tighter than shift', () => {
    expect(toSexp(parseOrFail('a + b << c'))).toBe('(<< (+ a b) c)');
  });

  it('shift binds tighter than relational', () => {
    expect(toSexp(parseOrFail('a << b < c'))).toBe('(< (<< a b) c)');
  });

  it('relational binds tighter than equality', () => {
    expect(toSexp(parseOrFail('a < b == c'))).toBe('(== (< a b) c)');
  });

  it('equality binds tighter than bitwise and', () => {
    expect(toSexp(parseOrFail('a == b & c'))).toBe('(& (== a b) c)');
  });

  it('bitwise and binds tighter than bitwise xor', () => {
    expect(toSexp(parseOrFail('a & b ^ c'))).toBe('(^ (& a b) c)');
  });

  it('bitwise xor binds tighter than bitwise or', () => {
    expect(toSexp(parseOrFail('a ^ b | c'))).toBe('(| (^ a b) c)');
  });

  it('bitwise or binds tighter than logical and', () => {
    expect(toSexp(parseOrFail('a | b && c'))).toBe('(&& (| a b) c)');
  });

  it('logical and binds tighter than logical or', () => {
    expect(toSexp(parseOrFail('a && b || c'))).toBe('(|| (&& a b) c)');
  });

  it('full precedence cascade', () => {
    expect(toSexp(parseOrFail('a || b && c | d ^ e & f == g < h << i + j * k'))).toBe(
      '(|| a (&& b (| c (^ d (& e (== f (< g (<< h (+ i (* j k))))))))))',
    );
  });

  it('parentheses override precedence', () => {
    expect(toSexp(parseOrFail('(a + b) * c'))).toBe('(* (+ a b) c)');
    expect(toSexp(parseOrFail('a * (b + c)'))).toBe('(* a (+ b c))');
  });
});

describe('expression grammar — realistic cases from ALU_NEW.machine', () => {
  it('parses a flag computation', () => {
    // From aluFlags<W>: `af = raw[4] ^ a[4] ^ b[4]`
    expect(toSexp(parseOrFail('raw[4] ^ a[4] ^ b[4]'))).toBe(
      '(^ (^ (index raw 4) (index a 4)) (index b 4))',
    );
  });

  it('parses a parity reduction', () => {
    // From parity8: `~(v[0] ^ v[1] ^ v[2] ^ v[3] ^ v[4] ^ v[5] ^ v[6] ^ v[7])`
    const src = '~(v[0] ^ v[1] ^ v[2] ^ v[3] ^ v[4] ^ v[5] ^ v[6] ^ v[7])';
    // Just verify it parses; the full S-expression is tedious.
    const cst = parseOrFail(src);
    expect(cst).toBeDefined();
  });

  it('parses an overflow-flag expression', () => {
    // From aluFlags<W> add case: `(~(a[msb] ^ b[msb])) & (a[msb] ^ r[msb])`
    const src = '(~(a[msb] ^ b[msb])) & (a[msb] ^ r[msb])';
    expect(toSexp(parseOrFail(src))).toBe(
      '(& (~ (^ (index a msb) (index b msb))) (^ (index a msb) (index r msb)))',
    );
  });

  it('parses a register-bit comparison', () => {
    // From aluFlags<W>: `zf = r == 0`
    expect(toSexp(parseOrFail('r == 0'))).toBe('(== r 0)');
  });

  it('parses a chained member-access condition', () => {
    // Common readiness expression: `busResponse.done && !hold`
    expect(toSexp(parseOrFail('busResponse.done && !hold'))).toBe(
      '(&& (. busResponse done) (! hold))',
    );
  });

  it('parses an ALU record literal with multiple fields', () => {
    // From addRmReg8's micro block, approximately:
    const src =
      '{ op: add, width: u8, srcA: mem, srcB: reg, dest: tmp, commit: onRetire }';
    expect(toSexp(parseOrFail(src))).toBe(
      '{op=add width=u8 srcA=mem srcB=reg dest=tmp commit=onRetire}',
    );
  });

  it('parses an empty record literal for fieldless microwords', () => {
    // The `Retire {}` case.
    expect(toSexp(parseOrFail('{}'))).toBe('{}');
  });

  it('parses a function-call expression with a complex argument', () => {
    // Like `parity8(r:u8)` — except cast isn't supported yet, so use
    // `parity8(r)`.
    expect(toSexp(parseOrFail('parity8(r)'))).toBe('(call parity8 r)');
  });
});

describe('expression grammar — cast', () => {
  it('parses a tight cast on an identifier', () => {
    expect(toSexp(parseOrFail('a:u8'))).toBe('(cast a u8)');
  });

  it('parses a tight cast on a numeric literal', () => {
    expect(toSexp(parseOrFail('0:u8'))).toBe('(cast 0 u8)');
  });

  it('parses a chained cast', () => {
    expect(toSexp(parseOrFail('a:u8:u16'))).toBe(
      '(cast (cast a u8) u16)',
    );
  });

  it('binds cast tighter than binary operators', () => {
    // `a:u17 + b:u17` — cast each operand, then add.
    expect(toSexp(parseOrFail('a:u17 + b:u17'))).toBe(
      '(+ (cast a u17) (cast b u17))',
    );
  });

  it('binds cast tighter than equality', () => {
    // From aluFlags: `raw:u8 == 0`.
    expect(toSexp(parseOrFail('raw:u8 == 0'))).toBe(
      '(== (cast raw u8) 0)',
    );
  });

  it('parses cast on a parenthesized expression', () => {
    // From prefetcher: `(busResponse.data >> 8):u8`.
    expect(toSexp(parseOrFail('(busResponse.data >> 8):u8'))).toBe(
      '(cast (>> (. busResponse data) 8) u8)',
    );
  });

  it('parses cast after a function call', () => {
    expect(toSexp(parseOrFail('foo(x):u8'))).toBe(
      '(cast (call foo x) u8)',
    );
  });

  it('parses cast after a member access', () => {
    expect(toSexp(parseOrFail('foo.bar:u8'))).toBe(
      '(cast (. foo bar) u8)',
    );
  });

  it('parses cast after an index operation', () => {
    expect(toSexp(parseOrFail('raw[0]:b'))).toBe(
      '(cast (index raw 0) b)',
    );
  });

  it('parses cast inside a record literal with a loose field colon', () => {
    // `{ a: b:u8 }` — the outer `: ` is loose (record field), the
    // inner `:u8` is tight (cast).
    expect(toSexp(parseOrFail('{ a: b:u8 }'))).toBe('{a=(cast b u8)}');
  });

  it('parses cast inside a record literal with a tight field colon', () => {
    // `{a:b:u8}` — the outer `:` is tight (still a record field via
    // categories), the inner `:u8` is tight (cast). The record field
    // rule consumes the first colon via category, the cast rule consumes
    // the second colon specifically as TightColon.
    expect(toSexp(parseOrFail('{a:b:u8}'))).toBe('{a=(cast b u8)}');
  });

  it('does not treat a loose colon as cast', () => {
    // `a : u8` — the `:` has spaces on both sides, so it's loose Colon,
    // not TightColon. The cast rule (which requires TightColon) refuses
    // to match, so `a` parses and `: u8` is left over — Chevrotain
    // reports NotAllInputParsedException for the trailing tokens.
    expectParseErrors('a : u8');
  });

  it('does not treat a half-tight colon as cast', () => {
    // Space on either side breaks the tight-colon rule, same as fully
    // loose.
    expectParseErrors('a :u8');
    expectParseErrors('a: u8');
  });

  it('keeps bit slices working with tight colons', () => {
    // `raw[7:0]` — the inner colon is tight. Bit slices use
    // `CONSUME(Colon)` which accepts TightColon via category.
    expect(toSexp(parseOrFail('raw[7:0]'))).toBe('(slice raw 7 0)');
  });

  it('keeps bit slices working with loose colons', () => {
    expect(toSexp(parseOrFail('raw[7 : 0]'))).toBe('(slice raw 7 0)');
  });
});

describe('expression grammar — integration with declaration bodies', () => {
  it('microword readyClause accepts a complex expression', async () => {
    const { parse } = await import('./parse.js');
    const src = [
      'microword Foo',
      '  fields',
      '    dest:Local',
      '  ready: prefetch.valid && !hold && cycleCount > 0',
      '',
    ].join('\n');
    const { cst, parseErrors } = parse(src);
    expect(parseErrors).toEqual([]);
    expect(cst).toBeDefined();
  });

  it('microword readyClause accepts a bare literal', async () => {
    const { parse } = await import('./parse.js');
    const { cst, parseErrors } = parse('microword Foo\n  ready: 1\n');
    expect(parseErrors).toEqual([]);
    expect(cst).toBeDefined();
  });

  it('microword readyClause accepts a member-access expression', async () => {
    const { parse } = await import('./parse.js');
    const { cst, parseErrors } = parse(
      'microword Foo\n  ready: busResponse.done\n',
    );
    expect(parseErrors).toEqual([]);
    expect(cst).toBeDefined();
  });
});
