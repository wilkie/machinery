import Tokenizer from './Tokenizer';
import Parser, { ParseError } from './Parser';
import {
  StatementNode,
  AssignmentNode,
  ExpressionNode,
  BinaryExpressionNode,
  UnaryExpressionNode,
  TernaryExpressionNode,
  CallExpressionNode,
  ChoiceExpressionNode,
  OperandNode,
  ArrayAccessNode,
  ComparisonNode,
  ComparisonEvaluationNode,
  BinaryLogicNode,
  UnaryLogicNode,
  IfBlockNode,
  LoopBlockNode,
  LoopIfNode,
  RepeatIfNode,
  NextIfNode,
  RaiseExpressionNode,
  CommentNode,
  EmptyNode,
} from './ast';
import type Node from './ast/Node';

/** Walk the statement linked list into an array of inner nodes */
function statements(root: StatementNode): Node[] {
  const nodes: Node[] = [];
  let current: StatementNode | undefined = root;
  while (current) {
    nodes.push(current.node);
    current = current.next;
  }
  return nodes;
}

// ─── Tokenizer ──────────────────────────────────────────────────────────────

describe('Tokenizer', () => {
  describe('basic tokenization', () => {
    it('tokenizes an identifier', () => {
      const t = new Tokenizer();
      const tokens = t.tokenize('AX ;');
      expect(tokens).toEqual([
        { type: 'identifier', value: 'AX', coercion: undefined },
        { type: 'terminator', value: ';', coercion: undefined },
      ]);
    });

    it('tokenizes a decimal number', () => {
      const t = new Tokenizer();
      const tokens = t.tokenize('42 ;');
      expect(tokens[0]).toMatchObject({ type: 'number', value: 42 });
    });

    it('tokenizes a hex number', () => {
      const t = new Tokenizer();
      const tokens = t.tokenize('0xFF ;');
      expect(tokens[0]).toMatchObject({ type: 'number', value: 255 });
    });

    it('tokenizes a binary number', () => {
      const t = new Tokenizer();
      const tokens = t.tokenize('0b1010 ;');
      expect(tokens[0]).toMatchObject({ type: 'number', value: 10 });
    });

    it('tokenizes a negative number', () => {
      const t = new Tokenizer();
      const tokens = t.tokenize('-3 ;');
      expect(tokens[0]).toMatchObject({ type: 'number', value: -3 });
    });

    it('tokenizes a system identifier', () => {
      const t = new Tokenizer();
      const tokens = t.tokenize('@halt ;');
      expect(tokens[0]).toMatchObject({ type: 'system', value: '@halt' });
    });

    it('filters whitespace', () => {
      const t = new Tokenizer();
      const tokens = t.tokenize('  AX   ;  ');
      expect(tokens.length).toBe(2);
    });
  });

  describe('operators', () => {
    it('tokenizes logical operators', () => {
      const t = new Tokenizer();
      expect(t.tokenize('&& ;')[0].type).toBe('logical_operator');
      expect(t.tokenize('|| ;')[0].type).toBe('logical_operator');
    });

    it('tokenizes bitwise operators', () => {
      const t = new Tokenizer();
      expect(t.tokenize('| ;')[0].type).toBe('op_or');
      expect(t.tokenize('^ ;')[0].type).toBe('op_xor');
      expect(t.tokenize('& ;')[0].type).toBe('op_and');
    });

    it('tokenizes shift operators', () => {
      const t = new Tokenizer();
      expect(t.tokenize('>> ;')[0].type).toBe('op_shift');
      expect(t.tokenize('<< ;')[0].type).toBe('op_shift');
      expect(t.tokenize('>>> ;')[0].type).toBe('op_shift');
    });

    it('tokenizes arithmetic operators', () => {
      const t = new Tokenizer();
      expect(t.tokenize('+ ;')[0].type).toBe('op_add');
      expect(t.tokenize('- ;')[0].type).toBe('op_add');
      expect(t.tokenize('* ;')[0].type).toBe('op_mul');
      expect(t.tokenize('/ ;')[0].type).toBe('op_mul');
      expect(t.tokenize('% ;')[0].type).toBe('op_mul');
      expect(t.tokenize('// ;')[0].type).toBe('op_mul');
    });

    it('tokenizes comparison operators', () => {
      const t = new Tokenizer();
      for (const op of ['==', '!=', '>', '<', '>=', '<=']) {
        expect(t.tokenize(`${op} ;`)[0].type).toBe('comparison');
      }
    });

    it('tokenizes unary operators', () => {
      const t = new Tokenizer();
      expect(t.tokenize('~ ;')[0].type).toBe('unary_operator');
      expect(t.tokenize('! ;')[0].type).toBe('unary_logic_operator');
    });
  });

  describe('comments', () => {
    it('tokenizes a comment', () => {
      const t = new Tokenizer();
      const tokens = t.tokenize(';; hello world ; ');
      // The comment regex captures between ;; and \s;\s, stripping the delimiter
      expect(tokens[0]).toMatchObject({ type: 'comment', value: ' hello world' });
    });
  });

  describe('macro expansion', () => {
    it('expands a simple string macro', () => {
      const t = new Tokenizer({ MY_REG: 'AX' });
      const tokens = t.tokenize('${MY_REG} ;');
      expect(tokens[0]).toMatchObject({ type: 'identifier', value: 'AX' });
    });

    it('expands nested macros', () => {
      const t = new Tokenizer({ INNER: 'AX', OUTER: '${INNER}' });
      const tokens = t.tokenize('${OUTER} ;');
      expect(tokens[0]).toMatchObject({ type: 'identifier', value: 'AX' });
    });

    it('expands a multi-line (array) macro with terminators', () => {
      const t = new Tokenizer({ BODY: ['AX = 1', 'BX = 2'] });
      const tokens = t.tokenize('${BODY} ;');
      // Should have: AX = 1 ; BX = 2 ; (plus trailing ;)
      const identifiers = tokens
        .filter((tok) => tok.type === 'identifier')
        .map((tok) => tok.value);
      expect(identifiers).toEqual(['AX', 'BX']);
      // Terminators inserted between operations
      const terminators = tokens.filter((tok) => tok.type === 'terminator');
      expect(terminators.length).toBeGreaterThanOrEqual(2);
    });

    it('throws on undefined macro', () => {
      const t = new Tokenizer();
      expect(() => t.tokenize('${NONEXISTENT} ;')).toThrow(
        'Macro NONEXISTENT not found',
      );
    });

    it('throws on coercion applied to multi-line macro', () => {
      const t = new Tokenizer({ MULTI: ['a', 'b'] });
      expect(() => t.tokenize('${MULTI:u16} ;')).toThrow('multi-line');
    });
  });

  describe('macro caching', () => {
    it('caches macro expansions', () => {
      const t = new Tokenizer({ FOO: 'AX' });
      t.tokenize('${FOO} ;');
      t.tokenize('${FOO} ;');
      expect(t.macroCache.size).toBe(1);
    });

    it('caches instruction-level macros too', () => {
      const t = new Tokenizer();
      const macros = { LOCAL_MACRO: 'BX' };
      t.tokenize('${LOCAL_MACRO} ;', macros);
      t.tokenize('${LOCAL_MACRO} ;', macros);
      expect(t.macroCache.size).toBe(1);
    });
  });

  describe('type coercions', () => {
    it('parses coercion on identifier', () => {
      const t = new Tokenizer();
      const tokens = t.tokenize('AX:u16 ;');
      expect(tokens[0]).toMatchObject({
        type: 'identifier',
        value: 'AX',
        coercion: 'u16',
      });
    });

    it('parses coercion on number', () => {
      const t = new Tokenizer();
      const tokens = t.tokenize('42:i32 ;');
      expect(tokens[0]).toMatchObject({
        type: 'number',
        value: 42,
        coercion: 'i32',
      });
    });

    it('parses coercion on system identifier', () => {
      const t = new Tokenizer();
      const tokens = t.tokenize('@sys:u8 ;');
      expect(tokens[0]).toMatchObject({
        type: 'system',
        value: '@sys',
        coercion: 'u8',
      });
    });

    it('applies coercion to string macro expansion', () => {
      const t = new Tokenizer({ FOO: 'AX' });
      const tokens = t.tokenize('${FOO:u32} ;');
      expect(tokens[0]).toMatchObject({
        type: 'identifier',
        value: 'AX',
        coercion: 'u32',
      });
    });
  });

  describe('locals', () => {
    it('tokenizes a local reference', () => {
      const t = new Tokenizer();
      const tokens = t.tokenize('%{imm} ;');
      expect(tokens[0]).toMatchObject({ type: 'local', value: 'imm' });
    });
  });

  describe('brackets and delimiters', () => {
    it('tokenizes list start and end', () => {
      const t = new Tokenizer();
      const tokens = t.tokenize('[ ] ;');
      expect(tokens[0].type).toBe('list_start');
      expect(tokens[1].type).toBe('list_end');
    });

    it('tokenizes list end with coercion', () => {
      const t = new Tokenizer();
      const tokens = t.tokenize(']:u16 ;');
      expect(tokens[0].type).toBe('list_end');
      expect(tokens[0].coercion).toBe('u16');
    });
  });
});

// ─── Parser / Grammar ───────────────────────────────────────────────────────

describe('Parser', () => {
  const parse = (code: string, macros?: Record<string, unknown>) =>
    new Parser(macros).parse(code + ' ;', macros);

  describe('simple operands', () => {
    it('parses an identifier expression', () => {
      const ast = parse('AX');
      expect(ast.node).toBeInstanceOf(ExpressionNode);
      const expr = ast.node as ExpressionNode;
      expect(expr.operand).toBeInstanceOf(OperandNode);
      expect((expr.operand as OperandNode).value).toBe('AX');
    });

    it('parses a numeric expression', () => {
      const ast = parse('42');
      const expr = ast.node as ExpressionNode;
      expect((expr.operand as OperandNode).value).toBe(42);
    });

    it('parses a system identifier', () => {
      const ast = parse('@halt');
      const expr = ast.node as ExpressionNode;
      expect((expr.operand as OperandNode).value).toBe('@halt');
    });
  });

  describe('dotted access', () => {
    it('parses two-level dotted identifier', () => {
      const ast = parse('RAM.data');
      const expr = ast.node as ExpressionNode;
      const op = expr.operand as OperandNode;
      expect(op.value).toBe('RAM');
      expect(op.next).toBeInstanceOf(OperandNode);
      expect((op.next as OperandNode).value).toBe('data');
    });

    it('parses three-level dotted identifier', () => {
      const ast = parse('RAM.IVT.segment');
      const op = (ast.node as ExpressionNode).operand as OperandNode;
      expect(op.value).toBe('RAM');
      const lvl2 = op.next as OperandNode;
      expect(lvl2.value).toBe('IVT');
      const lvl3 = lvl2.next as OperandNode;
      expect(lvl3.value).toBe('segment');
    });
  });

  describe('array access', () => {
    it('parses array index on identifier', () => {
      const ast = parse('foo[0]');
      const op = (ast.node as ExpressionNode).operand as OperandNode;
      expect(op.value).toBe('foo');
      expect(op.next).toBeInstanceOf(ArrayAccessNode);
      const arr = op.next as ArrayAccessNode;
      expect((arr.index as ExpressionNode).operand).toBeInstanceOf(OperandNode);
    });

    it('parses array index followed by dotted access', () => {
      const ast = parse('foo[0].bar');
      const op = (ast.node as ExpressionNode).operand as OperandNode;
      const arr = op.next as ArrayAccessNode;
      expect(arr.next).toBeInstanceOf(OperandNode);
      expect((arr.next as OperandNode).value).toBe('bar');
    });
  });

  describe('assignments', () => {
    it('parses a simple assignment', () => {
      const ast = parse('AX = 1');
      expect(ast.node).toBeInstanceOf(AssignmentNode);
      const assign = ast.node as AssignmentNode;
      expect((assign.destination as OperandNode).value).toBe('AX');
    });

    it('parses assignment to dotted identifier', () => {
      const ast = parse('RAM.data = AX');
      const assign = ast.node as AssignmentNode;
      const dest = assign.destination as OperandNode;
      expect(dest.value).toBe('RAM');
      expect((dest.next as OperandNode).value).toBe('data');
    });

    it('parses assignment to array index', () => {
      const ast = parse('foo[0] = 1');
      const assign = ast.node as AssignmentNode;
      const dest = assign.destination as OperandNode;
      expect(dest.next).toBeInstanceOf(ArrayAccessNode);
    });
  });

  describe('arithmetic expressions', () => {
    it('parses addition', () => {
      const ast = parse('AX + BX');
      // BinaryExpressionNode extends ExpressionNode, so ast.node IS the binary node
      const bin = ast.node as BinaryExpressionNode;
      expect(bin).toBeInstanceOf(BinaryExpressionNode);
      expect(bin.operator).toBe('+');
    });

    it('respects mul > add precedence', () => {
      // AX + BX * CX => +(AX, *(BX, CX))
      const ast = parse('AX + BX * CX');
      const bin = ast.node as BinaryExpressionNode;
      expect(bin.operator).toBe('+');
      expect(bin.argument).toBeInstanceOf(BinaryExpressionNode);
      expect((bin.argument as BinaryExpressionNode).operator).toBe('*');
    });

    it('parentheses override precedence', () => {
      // (AX + BX) * CX => *((AX + BX), CX)
      const ast = parse('(AX + BX) * CX');
      const bin = ast.node as BinaryExpressionNode;
      expect(bin.operator).toBe('*');
      // Left operand is an ExpressionNode wrapping the parenthesized addition
      expect(bin.operand).toBeInstanceOf(ExpressionNode);
    });

    it('is left-associative', () => {
      // A - B - C => (A - B) - C
      const ast = parse('A - B - C');
      const outer = ast.node as BinaryExpressionNode;
      expect(outer.operator).toBe('-');
      // Left child is also subtraction (left-assoc)
      expect(outer.operand).toBeInstanceOf(BinaryExpressionNode);
      expect((outer.operand as BinaryExpressionNode).operator).toBe('-');
    });
  });

  describe('all binary operators', () => {
    const ops = [
      ['|', 'op_or'],
      ['^', 'op_xor'],
      ['&', 'op_and'],
      ['>>', 'op_shift'],
      ['<<', 'op_shift'],
      ['+', 'op_add'],
      ['-', 'op_add'],
      ['*', 'op_mul'],
      ['/', 'op_mul'],
      ['%', 'op_mul'],
    ];

    it.each(ops)('parses binary operator %s', (op) => {
      const ast = parse(`A ${op} B`);
      // BinaryExpressionNode IS ast.node (extends ExpressionNode)
      const bin = ast.node as BinaryExpressionNode;
      expect(bin).toBeInstanceOf(BinaryExpressionNode);
      expect(bin.operator).toBe(op);
    });
  });

  describe('full precedence chain', () => {
    it('nests operators by precedence (or < xor < and < shift < add < mul)', () => {
      // A | B ^ C & D << E + F * G
      // Should be: or(A, xor(B, and(C, shift(D, add(E, mul(F, G))))))
      const ast = parse('A | B ^ C & D << E + F * G');
      const or_ = ast.node as BinaryExpressionNode;
      expect(or_.operator).toBe('|');
      const xor_ = or_.argument as BinaryExpressionNode;
      expect(xor_.operator).toBe('^');
      const and_ = xor_.argument as BinaryExpressionNode;
      expect(and_.operator).toBe('&');
      const shift_ = and_.argument as BinaryExpressionNode;
      expect(shift_.operator).toBe('<<');
      const add_ = shift_.argument as BinaryExpressionNode;
      expect(add_.operator).toBe('+');
      const mul_ = add_.argument as BinaryExpressionNode;
      expect(mul_.operator).toBe('*');
    });
  });

  describe('unary expressions', () => {
    it('parses bitwise NOT', () => {
      const ast = parse('~AX');
      // UnaryExpressionNode extends ExpressionNode, so ast.node IS the unary node
      expect(ast.node).toBeInstanceOf(UnaryExpressionNode);
      expect((ast.node as UnaryExpressionNode).operator).toBe('~');
    });

    it('parses nested unary', () => {
      const ast = parse('~~AX');
      const outer = ast.node as UnaryExpressionNode;
      expect(outer.operator).toBe('~');
      expect(outer.operand).toBeInstanceOf(UnaryExpressionNode);
    });
  });

  describe('ternary expressions', () => {
    it('parses a ternary expression', () => {
      const ast = parse('AX == 1 ? BX : CX');
      expect(ast.node).toBeInstanceOf(TernaryExpressionNode);
      const tern = ast.node as TernaryExpressionNode;
      expect(tern.condition).toBeInstanceOf(ComparisonEvaluationNode);
      // whenTrue is stored in .operand (via ExpressionNode)
      expect(tern.operand).toBeInstanceOf(ExpressionNode);
      expect(tern.whenFalse).toBeInstanceOf(ExpressionNode);
    });
  });

  describe('comparisons', () => {
    const compOps = ['==', '!=', '>', '<', '>=', '<='];

    it.each(compOps)('parses comparison operator %s in an if block', (op) => {
      const ast = parse(`if AX ${op} 1 ; end if`);
      const ifNode = ast.node as IfBlockNode;
      const cond = ifNode.condition as ComparisonEvaluationNode;
      expect(cond.operator).toBe(op);
    });
  });

  describe('logical operators', () => {
    it('parses && in comparison', () => {
      const ast = parse('if AX == 1 && BX == 2 ; end if');
      const ifNode = ast.node as IfBlockNode;
      expect(ifNode.condition).toBeInstanceOf(BinaryLogicNode);
      expect((ifNode.condition as BinaryLogicNode).operator).toBe('&&');
    });

    it('parses || in comparison', () => {
      const ast = parse('if AX == 1 || BX == 2 ; end if');
      const ifNode = ast.node as IfBlockNode;
      expect((ifNode.condition as BinaryLogicNode).operator).toBe('||');
    });

    it('chains logical operators left-associatively', () => {
      // A == 1 && B == 2 && C == 3 => &&(&&(A==1, B==2), C==3)
      const ast = parse('if A == 1 && B == 2 && C == 3 ; end if');
      const ifNode = ast.node as IfBlockNode;
      const outer = ifNode.condition as BinaryLogicNode;
      expect(outer.operator).toBe('&&');
      expect(outer.operand).toBeInstanceOf(BinaryLogicNode);
    });
  });

  describe('negated comparisons', () => {
    it('parses ! operator on comparison', () => {
      const ast = parse('if !(AX == 1) ; end if');
      const ifNode = ast.node as IfBlockNode;
      expect(ifNode.condition).toBeInstanceOf(UnaryLogicNode);
      const neg = ifNode.condition as UnaryLogicNode;
      expect(neg.operator).toBe('!');
      expect(neg.operand).toBeInstanceOf(ComparisonNode);
    });
  });

  describe('parenthesized comparisons', () => {
    it('parses parenthesized comparison', () => {
      const ast = parse('if (AX == 1) ; end if');
      const ifNode = ast.node as IfBlockNode;
      expect(ifNode.condition).toBeInstanceOf(ComparisonNode);
      // The inner operand should be the actual ComparisonEvaluationNode
      expect((ifNode.condition as ComparisonNode).operand).toBeInstanceOf(
        ComparisonEvaluationNode,
      );
    });
  });

  describe('if blocks', () => {
    it('parses empty if block', () => {
      const ast = parse('if AX == 1 ; end if');
      expect(ast.node).toBeInstanceOf(IfBlockNode);
      const ifNode = ast.node as IfBlockNode;
      // Body may be undefined or an EmptyNode (grammar ambiguity: the ;
      // between comparison and 'end' can be consumed by %terminator:* or statement)
      if (ifNode.body) {
        expect(ifNode.body.node).toBeInstanceOf(EmptyNode);
      }
    });

    it('parses if block with body', () => {
      const ast = parse('if AX == 1 ; BX = 2 ; end if');
      const ifNode = ast.node as IfBlockNode;
      expect(ifNode.body).toBeDefined();
      expect(ifNode.body!.node).toBeInstanceOf(AssignmentNode);
    });

    it('parses if block with multiple statements', () => {
      const ast = parse('if AX == 1 ; BX = 2 ; CX = 3 ; end if');
      const ifNode = ast.node as IfBlockNode;
      const bodyStmts = statements(ifNode.body!);
      expect(bodyStmts.length).toBe(2);
      expect(bodyStmts[0]).toBeInstanceOf(AssignmentNode);
      expect(bodyStmts[1]).toBeInstanceOf(AssignmentNode);
    });

    it('parses nested if blocks', () => {
      const ast = parse(
        'if AX == 1 ; if BX == 2 ; CX = 3 ; end if ; end if',
      );
      const outer = ast.node as IfBlockNode;
      const inner = outer.body!.node as IfBlockNode;
      expect(inner).toBeInstanceOf(IfBlockNode);
      expect(inner.body!.node).toBeInstanceOf(AssignmentNode);
    });
  });

  describe('loop blocks', () => {
    it('parses loop/repeat-if', () => {
      const ast = parse('loop ; AX = AX + 1 ; repeat if AX < 10');
      expect(ast.node).toBeInstanceOf(LoopBlockNode);
      const loop = ast.node as LoopBlockNode;
      expect(loop.condition).toBeInstanceOf(RepeatIfNode);
    });

    it('parses loop-if/repeat', () => {
      const ast = parse('loop if AX < 10 ; AX = AX + 1 ; repeat');
      expect(ast.node).toBeInstanceOf(LoopBlockNode);
      const loop = ast.node as LoopBlockNode;
      expect(loop.condition).toBeInstanceOf(LoopIfNode);
    });
  });

  describe('next if', () => {
    it('parses next if statement', () => {
      const ast = parse('next if AX == 0');
      expect(ast.node).toBeInstanceOf(NextIfNode);
      const nextIf = ast.node as NextIfNode;
      expect(nextIf.condition).toBeInstanceOf(ComparisonEvaluationNode);
    });
  });

  describe('raise expressions', () => {
    it('parses unconditional raise in expression', () => {
      // raise as expression inside assignment
      const ast = parse('#GP');
      const expr = ast.node as ExpressionNode;
      expect(expr).toBeInstanceOf(RaiseExpressionNode);
    });

    it('parses conditional raise statement', () => {
      const ast = parse('#GP if AX == 0');
      expect(ast.node).toBeInstanceOf(RaiseExpressionNode);
      const raise = ast.node as RaiseExpressionNode;
      expect(raise.condition).toBeInstanceOf(ComparisonEvaluationNode);
    });

    it('parses raise with named operand', () => {
      const ast = parse('#SS if offset > limit');
      const raise = ast.node as RaiseExpressionNode;
      expect((raise.operand as OperandNode).value).toBe('SS');
    });
  });

  describe('comments', () => {
    it('parses a comment', () => {
      const ast = parse(';; some note ;');
      expect(ast.node).toBeInstanceOf(CommentNode);
      // Comment regex captures between ;; and \s;\s, stripping delimiter chars
      expect((ast.node as CommentNode).message).toBe(' some note');
    });

    it('comment followed by statement', () => {
      const ast = parse(';; note ; AX = 1');
      const nodes = statements(ast);
      expect(nodes[0]).toBeInstanceOf(CommentNode);
      expect(nodes[1]).toBeInstanceOf(AssignmentNode);
    });
  });

  describe('empty statements', () => {
    it('parses empty statement', () => {
      const ast = new Parser().parse(';');
      expect(ast.node).toBeInstanceOf(EmptyNode);
    });
  });

  describe('multi-statement chaining', () => {
    it('chains two statements', () => {
      const ast = parse('AX = 1 ; BX = 2');
      const nodes = statements(ast);
      expect(nodes.length).toBe(2);
      expect(nodes[0]).toBeInstanceOf(AssignmentNode);
      expect(nodes[1]).toBeInstanceOf(AssignmentNode);
    });

    it('chains three statements', () => {
      const ast = parse('AX = 1 ; BX = 2 ; CX = 3');
      expect(statements(ast).length).toBe(3);
    });

    it('chains statement after if block', () => {
      const ast = parse('if AX == 1 ; BX = 2 ; end if ; CX = 3');
      const nodes = statements(ast);
      expect(nodes.length).toBe(2);
      expect(nodes[0]).toBeInstanceOf(IfBlockNode);
      expect(nodes[1]).toBeInstanceOf(AssignmentNode);
    });
  });

  describe('call expressions', () => {
    it('parses single-arg call', () => {
      const ast = parse('foo(AX)');
      // CallExpressionNode extends ExpressionNode, so ast.node IS the call node
      expect(ast.node).toBeInstanceOf(CallExpressionNode);
      const call = ast.node as CallExpressionNode;
      expect(call.name).toBe('foo');
      expect(call.args.length).toBe(1);
    });

    it('parses multi-arg call', () => {
      const ast = parse('foo(AX, BX, CX)');
      expect(ast.node).toBeInstanceOf(CallExpressionNode);
      const call = ast.node as CallExpressionNode;
      expect(call.args.length).toBe(3);
    });
  });

  describe('choice expressions', () => {
    it('parses choice expression in expression position', () => {
      const ast = parse('${[A,B,C][idx]}');
      // ChoiceExpressionNode is returned directly as ast.node
      expect(ast.node).toBeInstanceOf(ChoiceExpressionNode);
      const choice = ast.node as ChoiceExpressionNode;
      expect(choice.choices.length).toBe(3);
    });

    it('parses choice expression in assignment destination', () => {
      const ast = parse('${[A,B][idx]} = 1');
      expect(ast.node).toBeInstanceOf(AssignmentNode);
      const assign = ast.node as AssignmentNode;
      expect(assign.destination).toBeInstanceOf(ChoiceExpressionNode);
    });
  });

  describe('macro expansion in parser', () => {
    it('expands string macro during parsing', () => {
      const ast = new Parser({ REG: 'AX' }).parse('${REG} = 1 ;');
      const assign = ast.node as AssignmentNode;
      expect((assign.destination as OperandNode).value).toBe('AX');
    });

    it('expands multi-line macro into multiple statements', () => {
      const ast = new Parser({ BODY: ['AX = 1', 'BX = 2'] }).parse(
        '${BODY} ;',
      );
      const nodes = statements(ast);
      // Array macros append '; ' per element, plus the trailing ; from input,
      // producing an extra EmptyNode at the end
      expect(nodes.length).toBe(3);
      expect(nodes[0]).toBeInstanceOf(AssignmentNode);
      expect(nodes[1]).toBeInstanceOf(AssignmentNode);
      expect(nodes[2]).toBeInstanceOf(EmptyNode);
    });
  });

  describe('coercions', () => {
    it('preserves coercion on operand', () => {
      const ast = parse('AX:u16');
      const expr = ast.node as ExpressionNode;
      expect((expr.operand as OperandNode).coercion).toBe('u16');
    });
  });

  describe('complex real-world patterns', () => {
    it('parses flag resolution expression (ternary with complex comparison)', () => {
      const ast = parse('ZF = flag_op < 8 ? (alu_result == 0 ? 1 : 0) : ZF');
      expect(ast.node).toBeInstanceOf(AssignmentNode);
      const assign = ast.node as AssignmentNode;
      expect(assign.expression).toBeInstanceOf(TernaryExpressionNode);
    });

    it('parses memory access with dotted array index', () => {
      const ast = parse('RAM.GDT.gates[index].SD.P');
      const op = (ast.node as ExpressionNode).operand as OperandNode;
      expect(op.value).toBe('RAM');
      // Walk through: RAM -> GDT -> gates[index] -> SD -> P
      const gdt = op.next as OperandNode;
      expect(gdt.value).toBe('GDT');
      const gates = gdt.next as OperandNode;
      expect(gates.value).toBe('gates');
      expect(gates.next).toBeInstanceOf(ArrayAccessNode);
      const arr = gates.next as ArrayAccessNode;
      const sd = arr.next as OperandNode;
      expect(sd.value).toBe('SD');
      const p = sd.next as OperandNode;
      expect(p.value).toBe('P');
    });

    it('parses conditional raise with complex comparison', () => {
      const ast = parse('#GP if (desc_type & 0b100) != 0b100');
      expect(ast.node).toBeInstanceOf(RaiseExpressionNode);
      const raise = ast.node as RaiseExpressionNode;
      expect(raise.condition).toBeInstanceOf(ComparisonEvaluationNode);
    });

    it('parses if with logical operators', () => {
      const ast = parse(
        'if vector == 8 || vector == 10 || vector == 11 ; end if',
      );
      const ifNode = ast.node as IfBlockNode;
      // Should be left-associative: ||(||(v==8, v==10), v==11)
      const outer = ifNode.condition as BinaryLogicNode;
      expect(outer.operator).toBe('||');
      expect(outer.operand).toBeInstanceOf(BinaryLogicNode);
    });

    it('parses deeply nested if blocks (depth 5)', () => {
      const code = [
        'if A == 1',
        'if B == 2',
        'if C == 3',
        'if D == 4',
        'if E == 5',
        'X = 1',
        'end if',
        'end if',
        'end if',
        'end if',
        'end if',
      ].join(' ; ');
      const ast = parse(code);
      let node = ast.node as IfBlockNode;
      for (let i = 0; i < 4; i++) {
        expect(node).toBeInstanceOf(IfBlockNode);
        node = node.body!.node as IfBlockNode;
      }
      expect(node).toBeInstanceOf(IfBlockNode);
      expect(node.body!.node).toBeInstanceOf(AssignmentNode);
    });
  });

  describe('error handling', () => {
    it('throws ParseError on invalid syntax', () => {
      expect(() => parse('= = =')).toThrow(ParseError);
    });

    it('throws ParseError on unmatched parenthesis', () => {
      expect(() => parse('(AX + BX')).toThrow(ParseError);
    });

    it('throws ParseError on incomplete if block', () => {
      // Missing 'end if' — parser reaches end of input with no valid parse
      expect(() => parse('if AX == 1 ; BX = 2')).toThrow(ParseError);
    });

    it('ParseError includes the source code', () => {
      try {
        parse('= = =');
        fail('expected to throw');
      } catch (e) {
        expect(e).toBeInstanceOf(ParseError);
        const err = e as ParseError;
        expect(err.code).toContain('= = =');
        expect(err.tokens.length).toBeGreaterThan(0);
      }
    });

    it('ParseError message includes context for unexpected end', () => {
      try {
        parse('if AX == 1 ; BX = 2');
        fail('expected to throw');
      } catch (e) {
        expect(e).toBeInstanceOf(ParseError);
        expect((e as ParseError).message).toContain('unexpected end of input');
      }
    });

    it('warns on ambiguous parse', () => {
      const spy = jest.spyOn(console, 'warn').mockImplementation();
      // This may or may not trigger an ambiguity warning depending on the grammar.
      // Just ensure it doesn't crash.
      try {
        parse('AX');
      } finally {
        spy.mockRestore();
      }
    });
  });
});
