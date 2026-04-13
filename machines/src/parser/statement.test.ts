import type { CstNode, CstElement, IToken } from 'chevrotain';
import { parse, parseStatement } from './parse.js';

/**
 * Parse a single statement and fail the test with a readable error if
 * the lexer or parser produces any errors. Returns the root CST node.
 */
function parseStmtOrFail(source: string): CstNode {
  const { cst, lexErrors, parseErrors } = parseStatement(source);
  expect(lexErrors).toEqual([]);
  expect(parseErrors).toEqual([]);
  expect(cst).toBeDefined();
  return cst!;
}

/**
 * Walk a statement CST and collapse it to a Lisp-style S-expression
 * string. Leans on the expression-grammar conventions from
 * expression.test.ts but only covers the node kinds that appear at
 * statement level.
 */
function stmtSexp(node: CstElement): string {
  if (!isCstNode(node)) return (node as IToken).image;

  switch (node.name) {
    case 'statement': {
      // Exactly one child: the specific statement kind.
      // `muxStmt` is no longer a direct child — it's now reachable
      // via `exprOrAssignStmt → expression → primary → muxStmt`.
      for (const key of [
        'callStmt',
        'fetchStmt',
        'wireDeclStmt',
        'ifStmt',
        'assertStmt',
        'anonAssignStmt',
        'exprOrAssignStmt',
      ]) {
        const child = asCstNodes(node.children[key])[0];
        if (child) return stmtSexp(child);
      }
      return '<statement?>';
    }

    case 'anonAssignStmt': {
      const expr = exprSexp(firstChild(node, 'expression'));
      return `(= * ${expr})`;
    }

    case 'callStmt': {
      const expr = exprSexp(firstChild(node, 'expression'));
      const target = asCstNodes(node.children['callReturnTarget'])[0];
      if (!target) return `(call ${expr})`;
      return `(call ${expr} -> ${targetSexp(target)})`;
    }

    case 'fetchStmt': {
      const name = firstToken(node, 'Identifier').image;
      return `(fetch ${name})`;
    }

    case 'wireDeclStmt': {
      const name = firstToken(node, 'Identifier').image;
      const typeNode = firstChildOrUndefined(node, 'typeRef');
      const typed = typeNode
        ? `${name}:${firstToken(typeNode, 'Identifier').image}`
        : name;
      const value = exprSexp(firstChild(node, 'expression'));
      return `(wire ${typed} ${value})`;
    }

    case 'ifStmt': {
      const cond = exprSexp(firstChild(node, 'expression'));
      const thenBlock = blockSexp(firstChild(node, 'statementBlock'));
      const elifs = asCstNodes(node.children['elifBranch']).map((elif) => {
        const c = exprSexp(firstChild(elif, 'expression'));
        const b = blockSexp(firstChild(elif, 'statementBlock'));
        return `(elif ${c} ${b})`;
      });
      const elseNode = firstChildOrUndefined(node, 'elseBranch');
      const elseStr = elseNode
        ? ` (else ${blockSexp(firstChild(elseNode, 'statementBlock'))})`
        : '';
      const elifStr = elifs.length > 0 ? ' ' + elifs.join(' ') : '';
      return `(if ${cond} ${thenBlock}${elifStr}${elseStr})`;
    }

    case 'assertStmt': {
      const expr = exprSexp(firstChild(node, 'expression'));
      return `(assert ${expr})`;
    }

    case 'muxStmt': {
      const cond = exprSexp(firstChild(node, 'expression'));
      const arms = asCstNodes(node.children['muxArm']).map(armSexp);
      return `(mux ${cond} ${arms.join(' ')})`;
    }

    case 'exprOrAssignStmt': {
      const exprs = asCstNodes(node.children['expression']);
      const lhs = exprSexp(exprs[0]!);
      if (exprs.length === 1) return lhs;
      // Assignment form — find which operator was consumed.
      const eq = asTokens(node.children['Equal'])[0];
      const op = eq ? '=' : '<-';
      const rhs = exprSexp(exprs[1]!);
      return `(${op} ${lhs} ${rhs})`;
    }

    default:
      return `<${node.name}>`;
  }
}

function armSexp(arm: CstNode): string {
  const whenArm = asCstNodes(arm.children['whenArm'])[0];
  if (whenArm) {
    const pattern = exprSexp(firstChild(whenArm, 'expression'));
    const body = bodySexp(firstChild(whenArm, 'muxArmBody'));
    return `(when ${pattern} ${body})`;
  }
  const elseArm = asCstNodes(arm.children['elseArm'])[0];
  if (elseArm) {
    const body = bodySexp(firstChild(elseArm, 'muxArmBody'));
    return `(else ${body})`;
  }
  return '<muxArm?>';
}

function bodySexp(muxArmBody: CstNode): string {
  // Inline form: a single `statement` child.
  const inlineStmt = asCstNodes(muxArmBody.children['statement'])[0];
  if (inlineStmt) return stmtSexp(inlineStmt);
  // Block form: a single `statementBlock` child with nested statements.
  const block = asCstNodes(muxArmBody.children['statementBlock'])[0];
  if (block) return blockSexp(block);
  return '<body?>';
}

function blockSexp(block: CstNode): string {
  const stmts = asCstNodes(block.children['statement']).map(stmtSexp);
  return stmts.length === 1 ? stmts[0]! : `(block ${stmts.join(' ')})`;
}

function targetSexp(target: CstNode): string {
  const ids = asTokens(target.children['Identifier']).map((t) => t.image);
  return ids.length === 1 ? ids[0]! : `(${ids.join(' ')})`;
}

// ---- Minimal expression serializer (statements embed expressions) --------
// Reuses the structure from expression.test.ts but only the pieces that
// statements currently invoke. Kept local so this file is self-contained.

function exprSexp(node: CstElement): string {
  if (!isCstNode(node)) return (node as IToken).image;
  switch (node.name) {
    case 'expression':
      return exprSexp(firstChild(node, 'ternaryExpr'));
    case 'ternaryExpr':
    case 'logicalOrExpr':
    case 'logicalAndExpr':
    case 'bitOrExpr':
    case 'bitXorExpr':
    case 'bitAndExpr':
    case 'equalityExpr':
    case 'relationalExpr':
    case 'shiftExpr':
    case 'additiveExpr':
    case 'multiplicativeExpr':
      return collapsePassThrough(node);
    case 'unaryExpr': {
      // Unary form carries a leading Bang / Tilde / Minus token and
      // recurses into another unaryExpr. The non-unary form just
      // wraps a postfixExpr. Distinguish by checking for the
      // prefix token.
      for (const op of ['Bang', 'Tilde', 'Minus'] as const) {
        const opTok = firstTokenOrUndefined(node, op);
        if (opTok) {
          return `(${opTok.image} ${exprSexp(firstChild(node, 'unaryExpr'))})`;
        }
      }
      return exprSexp(firstChild(node, 'postfixExpr'));
    }
    case 'postfixExpr':
      return postfixSexp(node);
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
      if (paren) return exprSexp(paren);
      const rec = firstChildOrUndefined(node, 'recordLiteral');
      if (rec) return exprSexp(rec);
      const mux = firstChildOrUndefined(node, 'muxStmt');
      if (mux) return stmtSexp(mux);
      return '?';
    }
    case 'parenExpr': {
      const inner = firstChildOrUndefined(node, 'expression');
      return inner ? exprSexp(inner) : '()';
    }
    case 'recordLiteral': {
      const fields = asCstNodes(node.children['recordField']).map(exprSexp);
      return fields.length === 0 ? '{}' : `{${fields.join(' ')}}`;
    }
    case 'recordField': {
      const name = firstToken(node, 'Identifier').image;
      const value = exprSexp(firstChild(node, 'expression'));
      return `${name}=${value}`;
    }
    default:
      return `<${node.name}>`;
  }
}

function postfixSexp(node: CstNode): string {
  let out = exprSexp(firstChild(node, 'primaryExpr'));
  const ops = orderedChildren(node, [
    'memberOp',
    'callOp',
    'indexOp',
    'castOp',
    'constructOp',
  ]);
  for (const op of ops) {
    if (op.name === 'memberOp') {
      const nameTok =
        firstTokenOrUndefined(op, 'Identifier') ??
        firstTokenOrUndefined(op, 'Fetch');
      const name = nameTok?.image ?? '?';
      out = `(. ${out} ${name})`;
    } else if (op.name === 'callOp') {
      const args = asCstNodes(op.children['expression']).map(exprSexp);
      const typeArgs = asCstNodes(op.children['typeRef']).map((t) =>
        firstToken(t, 'Identifier').image,
      );
      const head = typeArgs.length > 0 ? `call<${typeArgs.join(',')}>` : 'call';
      out = args.length > 0
        ? `(${head} ${out} ${args.join(' ')})`
        : `(${head} ${out})`;
    } else if (op.name === 'indexOp') {
      const args = asCstNodes(op.children['expression']).map(exprSexp);
      out = args.length === 1
        ? `(index ${out} ${args[0]})`
        : `(slice ${out} ${args[0]} ${args[1]})`;
    } else if (op.name === 'castOp') {
      const typeNode = firstChild(op, 'typeRef');
      const typeName = firstToken(typeNode, 'Identifier').image;
      out = `(cast ${out} ${typeName})`;
    } else {
      // constructOp
      const fields = asCstNodes(op.children['recordField']).map(exprSexp);
      out = fields.length > 0
        ? `(cstr ${out} ${fields.join(' ')})`
        : `(cstr ${out})`;
    }
  }
  return out;
}

function collapsePassThrough(node: CstNode): string {
  // For binary-chain rules that aren't being exercised (no operator
  // tokens), walk the single operand child down to the next level.
  const childKeys = Object.keys(node.children).filter((k) => {
    const first = node.children[k]?.[0];
    return first && isCstNode(first);
  });
  if (childKeys.length !== 1) {
    // There are operator tokens present; this path isn't used by the
    // statement tests here so fall through.
    return `<${node.name}>`;
  }
  const firstKey = childKeys[0]!;
  const children = asCstNodes(node.children[firstKey]);
  if (children.length === 1) {
    return exprSexp(children[0]!);
  }
  // Multiple operands → binary chain with operators. Fall back to the
  // `expression.test.ts` handling by delegating to a local chain helper.
  return binaryChain(node, firstKey);
}

function binaryChain(node: CstNode, operandRule: string): string {
  const operands = asCstNodes(node.children[operandRule]);
  if (operands.length === 0) return '?';
  let out = exprSexp(operands[0]!);
  // Collect operator tokens in source order.
  const ops: IToken[] = [];
  for (const key of Object.keys(node.children)) {
    for (const c of node.children[key] ?? []) {
      if (!isCstNode(c)) ops.push(c as IToken);
    }
  }
  ops.sort((a, b) => (a.startOffset ?? 0) - (b.startOffset ?? 0));
  for (let i = 0; i < ops.length; i++) {
    const op = ops[i]!;
    const rhs = operands[i + 1];
    if (!rhs) break;
    out = `(${op.image} ${out} ${exprSexp(rhs)})`;
  }
  return out;
}

// ---- CST helpers (duplicated from expression.test.ts for isolation) ------

function orderedChildren(node: CstNode, ruleNames: string[]): CstNode[] {
  const all: CstNode[] = [];
  for (const name of ruleNames) {
    all.push(...asCstNodes(node.children[name]));
  }
  all.sort((a, b) => firstOffset(a) - firstOffset(b));
  return all;
}

function firstOffset(node: CstNode): number {
  for (const children of Object.values(node.children)) {
    for (const c of children) {
      if (isCstNode(c)) return firstOffset(c);
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

function firstChildOrUndefined(node: CstNode, name: string): CstNode | undefined {
  return asCstNodes(node.children[name])[0];
}

function firstToken(node: CstNode, name: string): IToken {
  const toks = asTokens(node.children[name]);
  if (toks.length === 0) {
    throw new Error(`expected token ${name} on ${node.name}`);
  }
  return toks[0]!;
}

function firstTokenOrUndefined(node: CstNode, name: string): IToken | undefined {
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

describe('statement grammar — call directive', () => {
  it('parses a zero-argument call with no return', () => {
    expect(stmtSexp(parseStmtOrFail('call fetchModRm'))).toBe(
      '(call fetchModRm)',
    );
  });

  it('parses a call with arguments', () => {
    expect(stmtSexp(parseStmtOrFail('call memWrite8(ea, tmp)'))).toBe(
      '(call (call memWrite8 ea tmp))',
    );
  });

  it('parses a call with a single-identifier return target', () => {
    expect(stmtSexp(parseStmtOrFail('call memRead8(ea) -> mdr'))).toBe(
      '(call (call memRead8 ea) -> mdr)',
    );
  });

  it('parses a call with a tuple return target', () => {
    expect(
      stmtSexp(parseStmtOrFail('call doStuff(a, b) -> (x, y)')),
    ).toBe('(call (call doStuff a b) -> (x y))');
  });

  it('parses the full eaCalc call shape from ALU_NEW.machine', () => {
    expect(
      stmtSexp(parseStmtOrFail('call eaCalc(ModRM.mod, ModRM.rm) -> ea')),
    ).toBe('(call (call eaCalc (. ModRM mod) (. ModRM rm)) -> ea)');
  });
});

describe('statement grammar — fetch directive', () => {
  it('parses a fetch directive', () => {
    expect(stmtSexp(parseStmtOrFail('fetch ModRM'))).toBe('(fetch ModRM)');
  });

  it('parses different operand names', () => {
    expect(stmtSexp(parseStmtOrFail('fetch Disp8'))).toBe('(fetch Disp8)');
    expect(stmtSexp(parseStmtOrFail('fetch Disp16'))).toBe('(fetch Disp16)');
  });
});

describe('statement grammar — bare expression statements', () => {
  it('parses a bare identifier', () => {
    expect(stmtSexp(parseStmtOrFail('foo'))).toBe('foo');
  });

  it('parses an empty record literal (Retire {})', () => {
    expect(stmtSexp(parseStmtOrFail('Retire {}'))).toBe('(cstr Retire)');
  });

  it('parses a microword constructor with single field', () => {
    expect(
      stmtSexp(parseStmtOrFail('IStreamRead { dest: modrm }')),
    ).toBe('(cstr IStreamRead dest=modrm)');
  });

  it('parses a microword constructor with many fields', () => {
    const src =
      'AluMicro { op: add, width: u8, srcA: mem, srcB: ModRM.reg, dest: tmp, writeFlags: 1, commit: onRetire }';
    expect(stmtSexp(parseStmtOrFail(src))).toBe(
      '(cstr AluMicro op=add width=u8 srcA=mem srcB=(. ModRM reg) dest=tmp writeFlags=1 commit=onRetire)',
    );
  });

  it('parses a bare empty parens `()` statement', () => {
    expect(stmtSexp(parseStmtOrFail('()'))).toBe('()');
  });
});

describe('statement grammar — mux statement', () => {
  it('parses a mux with two block-form when arms', () => {
    const src = ['mux x', '  when 0', '    foo', '  when 1', '    bar'].join(
      '\n',
    );
    expect(stmtSexp(parseStmtOrFail(src))).toBe(
      '(mux x (when 0 foo) (when 1 bar))',
    );
  });

  it('parses a mux with inline when arms', () => {
    const src = ['mux mod', '  when 0: ()', '  when 3: ()'].join('\n');
    expect(stmtSexp(parseStmtOrFail(src))).toBe(
      '(mux mod (when 0 ()) (when 3 ()))',
    );
  });

  it('parses a mux with mixed inline and block arms', () => {
    const src = [
      'mux mod',
      '  when 0: ()',
      '  when 1',
      '    fetch Disp8',
      '  when 2',
      '    fetch Disp16',
      '  when 3: ()',
    ].join('\n');
    expect(stmtSexp(parseStmtOrFail(src))).toBe(
      '(mux mod (when 0 ()) (when 1 (fetch Disp8)) (when 2 (fetch Disp16)) (when 3 ()))',
    );
  });

  it('parses a mux with an else arm', () => {
    const src = [
      'mux op',
      '  when 0',
      '    foo',
      '  else',
      '    bar',
    ].join('\n');
    expect(stmtSexp(parseStmtOrFail(src))).toBe(
      '(mux op (when 0 foo) (else bar))',
    );
  });

  it('parses a mux with a multi-statement block arm', () => {
    const src = [
      'mux mod',
      '  when 1',
      '    fetch Disp8',
      '    AluMicro { op: add }',
    ].join('\n');
    expect(stmtSexp(parseStmtOrFail(src))).toBe(
      '(mux mod (when 1 (block (fetch Disp8) (cstr AluMicro op=add))))',
    );
  });

  it('parses a mux whose condition is a complex expression', () => {
    const src = ['mux AluOp.add | AluOp.adc', '  when 0', '    foo'].join(
      '\n',
    );
    expect(stmtSexp(parseStmtOrFail(src))).toBe(
      '(mux (| (. AluOp add) (. AluOp adc)) (when 0 foo))',
    );
  });
});

describe('statement grammar — assignment', () => {
  it('parses a combinational assignment with `=`', () => {
    expect(stmtSexp(parseStmtOrFail('cf = raw'))).toBe('(= cf raw)');
  });

  it('parses a register next-state assignment with `<-`', () => {
    expect(stmtSexp(parseStmtOrFail('state <- ExecuteState.fetch'))).toBe(
      '(<- state (. ExecuteState fetch))',
    );
  });

  it('parses an assignment with a member-access LHS', () => {
    expect(stmtSexp(parseStmtOrFail('busRequest.valid = 1'))).toBe(
      '(= (. busRequest valid) 1)',
    );
  });

  it('parses an assignment with a deep member-access LHS', () => {
    expect(stmtSexp(parseStmtOrFail('FLAGS.CF <- raw[W]'))).toBe(
      '(<- (. FLAGS CF) (index raw W))',
    );
  });

  it('parses a record-literal RHS on `=`', () => {
    const src = 'pfResponse = { grant: 0, done: 0, data: 0 }';
    expect(stmtSexp(parseStmtOrFail(src))).toBe(
      '(= pfResponse {grant=0 done=0 data=0})',
    );
  });

  it('parses an assignment with a call RHS', () => {
    expect(
      stmtSexp(parseStmtOrFail('FLAGS <- applyFlags(FLAGS, r.flags)')),
    ).toBe('(<- FLAGS (call applyFlags FLAGS (. r flags)))');
  });

  it('parses a complex comparison RHS', () => {
    expect(stmtSexp(parseStmtOrFail('out.empty = qCount == 0'))).toBe(
      '(= (. out empty) (== qCount 0))',
    );
  });

  it('parses a shift + cast assignment', () => {
    expect(
      stmtSexp(parseStmtOrFail('q1 <- (busResponse.data >> 8):u8')),
    ).toBe('(<- q1 (cast (>> (. busResponse data) 8) u8))');
  });

  it('still parses a bare expression statement without an assignment', () => {
    expect(stmtSexp(parseStmtOrFail('foo'))).toBe('foo');
  });

  it('still parses a microword constructor without an assignment', () => {
    expect(stmtSexp(parseStmtOrFail('Retire {}'))).toBe('(cstr Retire)');
  });
});

describe('statement grammar — wire declaration', () => {
  it('parses a plain wire declaration', () => {
    expect(stmtSexp(parseStmtOrFail('wire a = aluSelect(srcA)'))).toBe(
      '(wire a (call aluSelect srcA))',
    );
  });

  it('parses a typed wire declaration', () => {
    expect(stmtSexp(parseStmtOrFail('wire raw:u17 = a + b'))).toBe(
      '(wire raw:u17 (+ a b))',
    );
  });

  it('parses a wire with a cast RHS', () => {
    expect(stmtSexp(parseStmtOrFail('wire head:u8 = raw:u8'))).toBe(
      '(wire head:u8 (cast raw u8))',
    );
  });

  it('parses a wire with a turbofish call RHS', () => {
    expect(
      stmtSexp(parseStmtOrFail('wire r = alu::<width>(op, a, b)')),
    ).toBe('(wire r (call<width> alu op a b))');
  });
});

describe('statement grammar — if / elif / else', () => {
  it('parses a plain if with a single-statement body', () => {
    const src = ['if cond', '  foo'].join('\n');
    expect(stmtSexp(parseStmtOrFail(src))).toBe('(if cond foo)');
  });

  it('parses an if with a multi-statement body', () => {
    const src = ['if cond', '  foo', '  bar'].join('\n');
    expect(stmtSexp(parseStmtOrFail(src))).toBe(
      '(if cond (block foo bar))',
    );
  });

  it('parses an if / else', () => {
    const src = ['if cond', '  foo', 'else', '  bar'].join('\n');
    expect(stmtSexp(parseStmtOrFail(src))).toBe(
      '(if cond foo (else bar))',
    );
  });

  it('parses an if / elif / else', () => {
    const src = [
      'if a',
      '  foo',
      'elif b',
      '  bar',
      'elif c',
      '  baz',
      'else',
      '  qux',
    ].join('\n');
    expect(stmtSexp(parseStmtOrFail(src))).toBe(
      '(if a foo (elif b bar) (elif c baz) (else qux))',
    );
  });

  it('parses an if with an assignment body (writeFlags case)', () => {
    const src = ['if writeFlags', '  FLAGS <- applyFlags(FLAGS, r.flags)'].join(
      '\n',
    );
    expect(stmtSexp(parseStmtOrFail(src))).toBe(
      '(if writeFlags (<- FLAGS (call applyFlags FLAGS (. r flags))))',
    );
  });

  it('parses an if nested inside another if', () => {
    const src = [
      'if outer',
      '  if inner',
      '    foo',
      '  else',
      '    bar',
      'else',
      '  baz',
    ].join('\n');
    expect(stmtSexp(parseStmtOrFail(src))).toBe(
      '(if outer (if inner foo (else bar)) (else baz))',
    );
  });
});

describe('statement grammar — assert', () => {
  it('parses a simple assert', () => {
    expect(stmtSexp(parseStmtOrFail('assert cond'))).toBe('(assert cond)');
  });

  it('parses an assert with a complex expression', () => {
    expect(
      stmtSexp(parseStmtOrFail('assert !(control.pop && out.empty)')),
    ).toBe('(assert (! (&& (. control pop) (. out empty))))');
  });
});

describe('statement grammar — integration with microword effect blocks', () => {
  it('parses IStreamRead\'s effect block', () => {
    const src = [
      'microword IStreamRead',
      '  fields',
      '    dest:Local',
      '  ready: prefetch.valid',
      '  effect',
      '    dest <- prefetch.byte',
      '    prefetchControl.pop = 1',
      '',
    ].join('\n');
    const { cst, parseErrors } = parse(src);
    expect(parseErrors).toEqual([]);
    expect(cst).toBeDefined();
  });

  it('parses BusRead\'s effect block with a record-literal assignment', () => {
    const src = [
      'microword BusRead',
      '  fields',
      '    width:BusSize',
      '    addr:Local',
      '    dest:Local',
      '  ready: busResponse.done',
      '  effect',
      '    busRequest = { valid: 1, op: BusOp.read, size: width,',
      '                   address: addr, data: 0 }',
      '    dest <- busResponse.data',
      '',
    ].join('\n');
    const { cst, parseErrors } = parse(src);
    expect(parseErrors).toEqual([]);
    expect(cst).toBeDefined();
  });

  it('parses Branch\'s effect block with an if', () => {
    const src = [
      'microword Branch',
      '  fields',
      '    cond:BranchCond',
      '    target:MicroAddr',
      '  ready: 1',
      '  effect',
      '    if evalCond(cond)',
      '      microPc <- target',
      '',
    ].join('\n');
    const { cst, parseErrors } = parse(src);
    expect(parseErrors).toEqual([]);
    expect(cst).toBeDefined();
  });

  it('parses Retire\'s effect block with a bare call and assignment', () => {
    const src = [
      'microword Retire',
      '  fields {}',
      '  ready: 1',
      '  effect',
      '    commitStaged()',
      '    state <- ExecuteState.fetch',
      '',
    ].join('\n');
    const { cst, parseErrors } = parse(src);
    expect(parseErrors).toEqual([]);
    expect(cst).toBeDefined();
  });

  it('parses AluMicro\'s effect block with wires and an if', () => {
    // A minimally-simplified AluMicro effect (omitting the elided
    // "..." placeholder args the real source uses).
    const src = [
      'microword AluMicro',
      '  fields',
      '    op:AluOp',
      '    width:Width',
      '    srcA:AluSrc',
      '    srcB:AluSrc',
      '    dest:AluDest',
      '    commit:Commit',
      '    writeFlags:b',
      '  ready: 1',
      '  effect',
      '    wire a = aluSelect(srcA)',
      '    wire b = aluSelect(srcB)',
      '    wire r = alu::<width>(op, a, b, FLAGS.CF)',
      '    if writeFlags',
      '      FLAGS <- applyFlags(FLAGS, r.flags)',
      '',
    ].join('\n');
    const { cst, parseErrors } = parse(src);
    expect(parseErrors).toEqual([]);
    expect(cst).toBeDefined();
  });
});

describe('statement grammar — integration with routine micro blocks', () => {
  it('parses eaCalc with two nested mux statements', () => {
    const src = [
      'routine eaCalc(mod:u2, rm:u3) -> (ea:u16)',
      '  micro',
      '    mux rm',
      '      when 0',
      '        AluMicro { op: add, width: u16, srcA: BX, srcB: SI, dest: ea, commit: never }',
      '      when 7',
      '        AluMicro { op: passA, width: u16, srcA: BX, srcB: zero, dest: ea, commit: never }',
      '    mux mod',
      '      when 0: ()',
      '      when 1',
      '        fetch Disp8',
      '        AluMicro { op: add, width: u16, srcA: ea, srcB: disp, dest: ea, commit: never }',
      '      when 3: ()',
      '',
    ].join('\n');
    const { cst, parseErrors } = parse(src);
    expect(parseErrors).toEqual([]);
    expect(cst).toBeDefined();
  });

  it('parses addRmReg8 with call/fetch/construct/retire', () => {
    const src = [
      'routine addRmReg8',
      '  entry: 0x00',
      '  micro',
      '    fetch ModRM',
      '    call eaCalc(ModRM.mod, ModRM.rm) -> ea',
      '    call memRead8(ea) -> mdr',
      '    AluMicro { op: add, width: u8, srcA: mem, srcB: ModRM.reg, dest: tmp, writeFlags: 1, commit: onRetire }',
      '    call memWrite8(ea, tmp)',
      '    Retire {}',
      '',
    ].join('\n');
    const { cst, parseErrors } = parse(src);
    expect(parseErrors).toEqual([]);
    expect(cst).toBeDefined();
  });
});

describe('statement grammar — integration with operand fetch blocks', () => {
  it('parses ModRM fetch block with one IStreamRead', () => {
    const src = [
      'operand ModRM',
      '  size: 1',
      '  fetch',
      '    IStreamRead { dest: modrm }',
      '',
    ].join('\n');
    const { cst, parseErrors } = parse(src);
    expect(parseErrors).toEqual([]);
    expect(cst).toBeDefined();
  });

  it('parses Disp8 fetch with an IStreamRead and an AluMicro', () => {
    const src = [
      'operand Disp8',
      '  size: 1',
      '  fetch',
      '    IStreamRead { dest: tmp }',
      '    AluMicro { op: passA, width: u8, srcA: tmp, srcB: zero, dest: disp, writeFlags: 0, commit: never }',
      '',
    ].join('\n');
    const { cst, parseErrors } = parse(src);
    expect(parseErrors).toEqual([]);
    expect(cst).toBeDefined();
  });

  it('parses Disp16 fetch with two IStreamReads into bit slices', () => {
    const src = [
      'operand Disp16',
      '  size: 2',
      '  fetch',
      '    IStreamRead { dest: disp[7:0] }',
      '    IStreamRead { dest: disp[15:8] }',
      '',
    ].join('\n');
    const { cst, parseErrors } = parse(src);
    expect(parseErrors).toEqual([]);
    expect(cst).toBeDefined();
  });
});
