// Generated automatically by nearley, version 2.20.1
// http://github.com/Hardmath123/nearley
// Bypasses TS6133. Allow declared but unused functions.
// @ts-ignore
function id(d: any[]): any { return d[0]; }

import ArrayAccessNode from './ast/ArrayAccessNode';
import AssignmentNode from './ast/AssignmentNode';
import BinaryExpressionNode from './ast/BinaryExpressionNode';
import BinaryLogicNode from './ast/BinaryLogicNode';
import CallExpressionNode from './ast/CallExpressionNode';
import ChoiceExpressionNode from './ast/ChoiceExpressionNode';
import CommentNode from './ast/CommentNode';
import ComparisonNode from './ast/ComparisonNode';
import ComparisonEvaluationNode from './ast/ComparisonEvaluationNode';
import EmptyNode from './ast/EmptyNode';
import ExpressionNode from './ast/ExpressionNode';
import RaiseExpressionNode from './ast/RaiseExpressionNode';
import OperandNode from './ast/OperandNode';
import StatementNode from './ast/StatementNode';
import TernaryExpressionNode from './ast/TernaryExpressionNode';
import UnaryExpressionNode from './ast/UnaryExpressionNode';
import UnaryLogicNode from './ast/UnaryLogicNode';
import IfBlockNode from './ast/IfBlockNode';
import LoopBlockNode from './ast/LoopBlockNode';
import LoopIfNode from './ast/LoopIfNode';
import RepeatIfNode from './ast/RepeatIfNode';
import NextIfNode from './ast/NextIfNode';
import type { Token } from './Token';

const if_ = { test: (x: Token) => x.type === 'if' };
const dot = { test: (x: Token) => x.type === 'dot' };
const comment = { test: (x: Token) => x.type === 'comment' };
const not_terminator = { test: (x: Token) => x.type !== 'terminator' };
const end = { test: (x: Token) => x.type === 'end' };
const next = { test: (x: Token) => x.type === 'next' };
const loop = { test: (x: Token) => x.type === 'loop' };
const repeat = { test: (x: Token) => x.type === 'repeat' };
const system = { test: (x: Token) => x.type === 'system' };
const identifier = { test: (x: Token) => x.type === 'identifier' };
const number = { test: (x: Token) => x.type === 'number' };
const comparison = { test: (x: Token) => x.type === 'comparison' };
const logical_operator = { test: (x: Token) => x.type === 'logical_operator' };
const op_or = { test: (x: Token) => x.type === 'op_or' };
const op_xor = { test: (x: Token) => x.type === 'op_xor' };
const op_and = { test: (x: Token) => x.type === 'op_and' };
const op_shift = { test: (x: Token) => x.type === 'op_shift' };
const op_add = { test: (x: Token) => x.type === 'op_add' };
const op_mul = { test: (x: Token) => x.type === 'op_mul' };
const op_rotate = { test: (x: Token) => x.type === 'op_rotate' };
const unary_operator = { test: (x: Token) => x.type === 'unary_operator' };
const unary_logic_operator = { test: (x: Token) => x.type === 'unary_logic_operator' };
const assignment = { test: (x: Token) => x.type === 'assignment' };
const left_paren = { test: (x: Token) => x.type === 'left_paren' };
const right_paren = { test: (x: Token) => x.type === 'right_paren' };
const raise = { test: (x: Token) => x.type === 'raise' };
const ternary_if = { test: (x: Token) => x.type === 'ternary_if' };
const ternary_else = { test: (x: Token) => x.type === 'ternary_else' };
const terminator = { test: (x: Token) => x.type === 'terminator' };
const macro_start = { test: (x: Token) => x.type === 'macro_start' };
const macro_end = { test: (x: Token) => x.type === 'macro_end' };
const local = { test: (x: Token) => x.type === 'local' };
const list_start = { test: (x: Token) => x.type === 'list_start' };
const list_end = { test: (x: Token) => x.type === 'list_end' };
const list_delimiter = { test: (x: Token) => x.type === 'list_delimiter' };

interface NearleyToken {
  value: any;
  [key: string]: any;
};

interface NearleyLexer {
  reset: (chunk: string, info: any) => void;
  next: () => NearleyToken | undefined;
  save: () => any;
  formatError: (token: never) => string;
  has: (tokenType: string) => boolean;
};

interface NearleyRule {
  name: string;
  symbols: NearleySymbol[];
  postprocess?: (d: any[], loc?: number, reject?: {}) => any;
};

type NearleySymbol = string | { literal: any } | { test: (token: any) => boolean };

interface Grammar {
  Lexer: NearleyLexer | undefined;
  ParserRules: NearleyRule[];
  ParserStart: string;
};

const grammar: Grammar = {
  Lexer: undefined,
  ParserRules: [
    {"name": "statement$ebnf$1", "symbols": []},
    {"name": "statement$ebnf$1", "symbols": ["statement$ebnf$1", terminator], "postprocess": (d) => d[0].concat([d[1]])},
    {"name": "statement$ebnf$2", "symbols": ["statement"], "postprocess": id},
    {"name": "statement$ebnf$2", "symbols": [], "postprocess": () => null},
    {"name": "statement", "symbols": ["assignment", terminator, "statement$ebnf$1", "statement$ebnf$2"], "postprocess": (data) => new StatementNode(data[0], data[3] || undefined)},
    {"name": "statement$ebnf$3", "symbols": []},
    {"name": "statement$ebnf$3", "symbols": ["statement$ebnf$3", terminator], "postprocess": (d) => d[0].concat([d[1]])},
    {"name": "statement$ebnf$4", "symbols": ["statement"], "postprocess": id},
    {"name": "statement$ebnf$4", "symbols": [], "postprocess": () => null},
    {"name": "statement", "symbols": ["expression", terminator, "statement$ebnf$3", "statement$ebnf$4"], "postprocess": (data) => new StatementNode(data[0], data[3] || undefined)},
    {"name": "statement$ebnf$5", "symbols": []},
    {"name": "statement$ebnf$5", "symbols": ["statement$ebnf$5", terminator], "postprocess": (d) => d[0].concat([d[1]])},
    {"name": "statement$ebnf$6", "symbols": ["statement"], "postprocess": id},
    {"name": "statement$ebnf$6", "symbols": [], "postprocess": () => null},
    {"name": "statement", "symbols": [next, if_, "comparison", "statement$ebnf$5", "statement$ebnf$6"], "postprocess": (data) => new StatementNode(new NextIfNode(data[2]), data[4] || undefined)},
    {"name": "statement$ebnf$7", "symbols": []},
    {"name": "statement$ebnf$7", "symbols": ["statement$ebnf$7", terminator], "postprocess": (d) => d[0].concat([d[1]])},
    {"name": "statement$ebnf$8", "symbols": ["statement"], "postprocess": id},
    {"name": "statement$ebnf$8", "symbols": [], "postprocess": () => null},
    {"name": "statement$ebnf$9", "symbols": []},
    {"name": "statement$ebnf$9", "symbols": ["statement$ebnf$9", terminator], "postprocess": (d) => d[0].concat([d[1]])},
    {"name": "statement$ebnf$10", "symbols": ["statement"], "postprocess": id},
    {"name": "statement$ebnf$10", "symbols": [], "postprocess": () => null},
    {"name": "statement", "symbols": [loop, terminator, "statement$ebnf$7", "statement$ebnf$8", repeat, if_, "comparison", "statement$ebnf$9", "statement$ebnf$10"], "postprocess": (data) => new StatementNode(new LoopBlockNode('', new RepeatIfNode(data[6]), data[3] || undefined), data[8] || undefined)},
    {"name": "statement$ebnf$11", "symbols": []},
    {"name": "statement$ebnf$11", "symbols": ["statement$ebnf$11", terminator], "postprocess": (d) => d[0].concat([d[1]])},
    {"name": "statement$ebnf$12", "symbols": ["statement"], "postprocess": id},
    {"name": "statement$ebnf$12", "symbols": [], "postprocess": () => null},
    {"name": "statement$ebnf$13", "symbols": []},
    {"name": "statement$ebnf$13", "symbols": ["statement$ebnf$13", terminator], "postprocess": (d) => d[0].concat([d[1]])},
    {"name": "statement$ebnf$14", "symbols": ["statement"], "postprocess": id},
    {"name": "statement$ebnf$14", "symbols": [], "postprocess": () => null},
    {"name": "statement", "symbols": [loop, if_, "comparison", terminator, "statement$ebnf$11", "statement$ebnf$12", repeat, "statement$ebnf$13", "statement$ebnf$14"], "postprocess": (data) => new StatementNode(new LoopBlockNode('', new LoopIfNode(data[2]), data[5] || undefined), data[8] || undefined)},
    {"name": "statement$ebnf$15", "symbols": []},
    {"name": "statement$ebnf$15", "symbols": ["statement$ebnf$15", terminator], "postprocess": (d) => d[0].concat([d[1]])},
    {"name": "statement$ebnf$16", "symbols": ["statement"], "postprocess": id},
    {"name": "statement$ebnf$16", "symbols": [], "postprocess": () => null},
    {"name": "statement$ebnf$17", "symbols": []},
    {"name": "statement$ebnf$17", "symbols": ["statement$ebnf$17", terminator], "postprocess": (d) => d[0].concat([d[1]])},
    {"name": "statement$ebnf$18", "symbols": ["statement"], "postprocess": id},
    {"name": "statement$ebnf$18", "symbols": [], "postprocess": () => null},
    {"name": "statement", "symbols": [if_, "comparison", "statement$ebnf$15", "statement$ebnf$16", end, if_, "statement$ebnf$17", "statement$ebnf$18"], "postprocess": (data) => new StatementNode(new IfBlockNode(data[1], data[3]), data[7] || undefined)},
    {"name": "statement$ebnf$19", "symbols": []},
    {"name": "statement$ebnf$19", "symbols": ["statement$ebnf$19", terminator], "postprocess": (d) => d[0].concat([d[1]])},
    {"name": "statement$ebnf$20", "symbols": ["statement"], "postprocess": id},
    {"name": "statement$ebnf$20", "symbols": [], "postprocess": () => null},
    {"name": "statement", "symbols": [raise, "operand", if_, "comparison", "statement$ebnf$19", "statement$ebnf$20"], "postprocess": (data) => new StatementNode(new RaiseExpressionNode(data[1], data[3]), data[5] || undefined)},
    {"name": "statement$ebnf$21", "symbols": []},
    {"name": "statement$ebnf$21", "symbols": ["statement$ebnf$21", terminator], "postprocess": (d) => d[0].concat([d[1]])},
    {"name": "statement$ebnf$22", "symbols": ["statement"], "postprocess": id},
    {"name": "statement$ebnf$22", "symbols": [], "postprocess": () => null},
    {"name": "statement", "symbols": [comment, "statement$ebnf$21", "statement$ebnf$22"], "postprocess": (data) => new StatementNode(new CommentNode(data[0].value), data[2] || undefined)},
    {"name": "assignment", "symbols": ["named", assignment, "expression"], "postprocess": (data) => new AssignmentNode(data[0], data[2])},
    {"name": "assignment", "symbols": [macro_start, identifier, macro_end, assignment, "expression"], "postprocess": (data) => new AssignmentNode(new OperandNode(data[1].value, data[1].coercion), data[4])},
    {"name": "assignment$ebnf$1", "symbols": []},
    {"name": "assignment$ebnf$1$subexpression$1", "symbols": [list_delimiter, "operand"]},
    {"name": "assignment$ebnf$1", "symbols": ["assignment$ebnf$1", "assignment$ebnf$1$subexpression$1"], "postprocess": (d) => d[0].concat([d[1]])},
    {"name": "assignment", "symbols": [macro_start, list_start, "operand", "assignment$ebnf$1", list_end, list_start, "expression", list_end, macro_end, assignment, "expression"], "postprocess": (data) => new AssignmentNode(new ChoiceExpressionNode([data[2], ...((data[3] || []).map((set: NearleyToken) => set[1]))], data[6], data[7].coercion), data[10])},
    {"name": "expression", "symbols": ["comparison", ternary_if, "expression", ternary_else, "expression"], "postprocess": (data) => new TernaryExpressionNode(data[0], data[2], data[4])},
    {"name": "expression", "symbols": ["expr_or"], "postprocess": (data) => data[0]},
    {"name": "expr_or", "symbols": ["expr_or", op_or, "expr_xor"], "postprocess": (data) => new BinaryExpressionNode(data[0], data[1].value.toString(), data[2])},
    {"name": "expr_or", "symbols": ["expr_xor"], "postprocess": (data) => data[0]},
    {"name": "expr_xor", "symbols": ["expr_xor", op_xor, "expr_and"], "postprocess": (data) => new BinaryExpressionNode(data[0], data[1].value.toString(), data[2])},
    {"name": "expr_xor", "symbols": ["expr_and"], "postprocess": (data) => data[0]},
    {"name": "expr_and", "symbols": ["expr_and", op_and, "expr_shift"], "postprocess": (data) => new BinaryExpressionNode(data[0], data[1].value.toString(), data[2])},
    {"name": "expr_and", "symbols": ["expr_shift"], "postprocess": (data) => data[0]},
    {"name": "expr_shift", "symbols": ["expr_shift", op_shift, "expr_add"], "postprocess": (data) => new BinaryExpressionNode(data[0], data[1].value.toString(), data[2])},
    {"name": "expr_shift", "symbols": ["expr_add"], "postprocess": (data) => data[0]},
    {"name": "expr_add", "symbols": ["expr_add", op_add, "expr_mul"], "postprocess": (data) => new BinaryExpressionNode(data[0], data[1].value.toString(), data[2])},
    {"name": "expr_add", "symbols": ["expr_mul"], "postprocess": (data) => data[0]},
    {"name": "expr_mul", "symbols": ["expr_mul", op_mul, "expr_rotate"], "postprocess": (data) => new BinaryExpressionNode(data[0], data[1].value.toString(), data[2])},
    {"name": "expr_mul", "symbols": ["expr_rotate"], "postprocess": (data) => data[0]},
    {"name": "expr_rotate", "symbols": ["expr_rotate", op_rotate, "expr_unary"], "postprocess": (data) => new BinaryExpressionNode(data[0], data[1].value.toString(), data[2])},
    {"name": "expr_rotate", "symbols": ["expr_unary"], "postprocess": (data) => data[0]},
    {"name": "expr_unary", "symbols": [unary_operator, "expr_unary"], "postprocess": (data) => new UnaryExpressionNode(data[1], data[0].value.toString())},
    {"name": "expr_unary", "symbols": ["expr_atom"], "postprocess": (data) => data[0]},
    {"name": "expr_atom", "symbols": [left_paren, "expression", right_paren], "postprocess": (data) => new ExpressionNode(data[1], data[2].coercion)},
    {"name": "expr_atom$ebnf$1", "symbols": []},
    {"name": "expr_atom$ebnf$1$subexpression$1", "symbols": [list_delimiter, "expression"]},
    {"name": "expr_atom$ebnf$1", "symbols": ["expr_atom$ebnf$1", "expr_atom$ebnf$1$subexpression$1"], "postprocess": (d) => d[0].concat([d[1]])},
    {"name": "expr_atom", "symbols": ["named", left_paren, "expression", "expr_atom$ebnf$1", right_paren], "postprocess": (data) => new CallExpressionNode(data[0].value.toString(), [data[2], ...((data[3] || []).map((set: NearleyToken) => set[1]))])},
    {"name": "expr_atom", "symbols": [raise, "operand"], "postprocess": (data) => new RaiseExpressionNode(data[1])},
    {"name": "expr_atom", "symbols": [macro_start, identifier, macro_end], "postprocess": (data) => new OperandNode(data[1].value, data[1].coercion)},
    {"name": "expr_atom$ebnf$2", "symbols": []},
    {"name": "expr_atom$ebnf$2$subexpression$1", "symbols": [list_delimiter, "operand"]},
    {"name": "expr_atom$ebnf$2", "symbols": ["expr_atom$ebnf$2", "expr_atom$ebnf$2$subexpression$1"], "postprocess": (d) => d[0].concat([d[1]])},
    {"name": "expr_atom", "symbols": [macro_start, list_start, "operand", "expr_atom$ebnf$2", list_end, list_start, "expression", list_end, macro_end], "postprocess": (data) => new ChoiceExpressionNode([data[2], ...((data[3] || []).map((set: NearleyToken) => set[1]))], data[6], data[7].coercion)},
    {"name": "expr_atom", "symbols": ["operand"], "postprocess": (data) => new ExpressionNode(data[0])},
    {"name": "comparison", "symbols": ["comparison", logical_operator, "comparison_atom"], "postprocess": (data) => new BinaryLogicNode(data[0], data[1].value.toString(), data[2])},
    {"name": "comparison", "symbols": ["comparison_atom"], "postprocess": (data) => data[0]},
    {"name": "comparison_atom", "symbols": [left_paren, "comparison", right_paren], "postprocess": (data) => new ComparisonNode(data[1])},
    {"name": "comparison_atom", "symbols": [unary_logic_operator, "comparison_atom"], "postprocess": (data) => new UnaryLogicNode(data[1], data[0].value.toString())},
    {"name": "comparison_atom", "symbols": ["expr_or", comparison, "expr_or"], "postprocess": (data) => new ComparisonEvaluationNode(data[0], data[1].value.toString(), data[2])},
    {"name": "named$ebnf$1$subexpression$1", "symbols": [dot, "named"]},
    {"name": "named$ebnf$1", "symbols": ["named$ebnf$1$subexpression$1"], "postprocess": id},
    {"name": "named$ebnf$1", "symbols": [], "postprocess": () => null},
    {"name": "named", "symbols": [identifier, "named$ebnf$1"], "postprocess": (data) => new OperandNode(data[0].value, data[0].coercion, data[1]?.[1])},
    {"name": "named$ebnf$2$subexpression$1", "symbols": [dot, "named"]},
    {"name": "named$ebnf$2", "symbols": ["named$ebnf$2$subexpression$1"], "postprocess": id},
    {"name": "named$ebnf$2", "symbols": [], "postprocess": () => null},
    {"name": "named", "symbols": [identifier, list_start, "expression", list_end, "named$ebnf$2"], "postprocess": (data) => new OperandNode(data[0].value, data[0].coercion, new ArrayAccessNode(data[2], data[4]?.[1]))},
    {"name": "named$ebnf$3$subexpression$1", "symbols": [dot, "named"]},
    {"name": "named$ebnf$3", "symbols": ["named$ebnf$3$subexpression$1"], "postprocess": id},
    {"name": "named$ebnf$3", "symbols": [], "postprocess": () => null},
    {"name": "named", "symbols": [system, "named$ebnf$3"], "postprocess": (data) => new OperandNode(data[0].value, data[0].coercion, data[1]?.[1])},
    {"name": "operand", "symbols": ["named"], "postprocess": (data) => data[0]},
    {"name": "operand", "symbols": [local], "postprocess": (data) => new OperandNode(data[0].value, data[0].coercion)},
    {"name": "operand", "symbols": [number], "postprocess": (data) => new OperandNode(data[0].value, data[0].coercion)}
  ],
  ParserStart: "statement",
};

export default grammar;
