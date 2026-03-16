@preprocessor typescript

@{%
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
%}

statement -> assignment %terminator %terminator:* statement:? {% (data) => new StatementNode(data[0], data[3] || undefined) %}
           | expression %terminator %terminator:* statement:? {% (data) => new StatementNode(data[0], data[3] || undefined) %}
           | %next %if_ comparison %terminator:* statement:? {% (data) => new StatementNode(new NextIfNode(data[2]), data[4] || undefined) %}
           | %loop %terminator %terminator:* statement:? %repeat %if_ comparison %terminator:* statement:? {% (data) => new StatementNode(new LoopBlockNode('', new RepeatIfNode(data[6]), data[3] || undefined), data[8] || undefined) %}
           | %loop %if_ comparison %terminator %terminator:* statement:? %repeat %terminator:* statement:? {% (data) => new StatementNode(new LoopBlockNode('', new LoopIfNode(data[2]), data[5] || undefined), data[8] || undefined) %}
           | %if_ comparison %terminator:* statement:? %end %if_ %terminator:* statement:? {% (data) => new StatementNode(new IfBlockNode(data[1], data[3]), data[7] || undefined) %}
           | %raise operand %if_ comparison %terminator:* statement:? {% (data) => new StatementNode(new RaiseExpressionNode(data[1], data[3]), data[5] || undefined) %}
           | %comment %terminator:* statement:? {% (data) => new StatementNode(new CommentNode(data[0].value), data[2] || undefined) %}

# An assignment has a left-hand identifier (or system identifier) and then some expression that
# we will write to it.
assignment -> named %assignment expression
            {% (data) => new AssignmentNode(data[0], data[2]) %}
            | %macro_start %identifier %macro_end %assignment expression
            {% (data) => new AssignmentNode(new OperandNode(data[1].value, data[1].coercion), data[4]) %}
            | %macro_start %list_start operand (%list_delimiter operand):* %list_end %list_start expression %list_end %macro_end %assignment expression
            {% (data) => new AssignmentNode(new ChoiceExpressionNode([data[2], ...((data[3] || []).map((set: NearleyToken) => set[1]))], data[6], data[7].coercion), data[10]) %}

# Expression precedence levels (lowest to highest):
#   ternary < or < xor < and < shift < add < mul < rotate < unary < atom
# Each level left-recurses into itself for left-associativity,
# and the right operand is the next-higher precedence level.

expression -> comparison %ternary_if expression %ternary_else expression
            {% (data) => new TernaryExpressionNode(data[0], data[2], data[4]) %}
            | expr_or
            {% (data) => data[0] %}

expr_or -> expr_or %op_or expr_xor
            {% (data) => new BinaryExpressionNode(data[0], data[1].value.toString(), data[2]) %}
            | expr_xor
            {% (data) => data[0] %}

expr_xor -> expr_xor %op_xor expr_and
            {% (data) => new BinaryExpressionNode(data[0], data[1].value.toString(), data[2]) %}
            | expr_and
            {% (data) => data[0] %}

expr_and -> expr_and %op_and expr_shift
            {% (data) => new BinaryExpressionNode(data[0], data[1].value.toString(), data[2]) %}
            | expr_shift
            {% (data) => data[0] %}

expr_shift -> expr_shift %op_shift expr_add
            {% (data) => new BinaryExpressionNode(data[0], data[1].value.toString(), data[2]) %}
            | expr_add
            {% (data) => data[0] %}

expr_add -> expr_add %op_add expr_mul
            {% (data) => new BinaryExpressionNode(data[0], data[1].value.toString(), data[2]) %}
            | expr_mul
            {% (data) => data[0] %}

expr_mul -> expr_mul %op_mul expr_rotate
            {% (data) => new BinaryExpressionNode(data[0], data[1].value.toString(), data[2]) %}
            | expr_rotate
            {% (data) => data[0] %}

expr_rotate -> expr_rotate %op_rotate expr_unary
            {% (data) => new BinaryExpressionNode(data[0], data[1].value.toString(), data[2]) %}
            | expr_unary
            {% (data) => data[0] %}

expr_unary -> %unary_operator expr_unary
            {% (data) => new UnaryExpressionNode(data[1], data[0].value.toString()) %}
            | expr_atom
            {% (data) => data[0] %}

expr_atom -> %left_paren expression %right_paren
            {% (data) => new ExpressionNode(data[1], data[2].coercion) %}
            | named %left_paren expression (%list_delimiter expression):* %right_paren
            {% (data) => new CallExpressionNode(data[0].value.toString(), [data[2], ...((data[3] || []).map((set: NearleyToken) => set[1]))]) %}
            | %raise operand
            {% (data) => new RaiseExpressionNode(data[1]) %}
            | %macro_start %identifier %macro_end
            {% (data) => new OperandNode(data[1].value, data[1].coercion) %}
            | %macro_start %list_start operand (%list_delimiter operand):* %list_end %list_start expression %list_end %macro_end
            {% (data) => new ChoiceExpressionNode([data[2], ...((data[3] || []).map((set: NearleyToken) => set[1]))], data[6], data[7].coercion) %}
            | operand
            {% (data) => new ExpressionNode(data[0]) %}

# Some expression that evaluates to a boolean value
# comparison is left-recursive for logical operators (&&, ||)
# comparison_atom handles single comparisons, parenthesized, and negated comparisons
comparison -> comparison %logical_operator comparison_atom
            {% (data) => new BinaryLogicNode(data[0], data[1].value.toString(), data[2]) %}
            | comparison_atom
            {% (data) => data[0] %}

comparison_atom -> %left_paren comparison %right_paren
            {% (data) => new ComparisonNode(data[1]) %}
            | %unary_logic_operator comparison_atom
            {% (data) => new UnaryLogicNode(data[1], data[0].value.toString()) %}
            | expr_or %comparison expr_or
            {% (data) => new ComparisonEvaluationNode(data[0], data[1].value.toString(), data[2]) %}

# A named identifier and possibly dotted
named -> %identifier (%dot named):?
       {% (data) => new OperandNode(data[0].value, data[0].coercion, data[1]?.[1]) %}
       | %identifier %list_start expression %list_end (%dot named):?
       {% (data) => new OperandNode(data[0].value, data[0].coercion, new ArrayAccessNode(data[2], data[4]?.[1])) %}
       | %system (%dot named):?
       {% (data) => new OperandNode(data[0].value, data[0].coercion, data[1]?.[1]) %}

# An operand that can be used as inputs to expressions and calls
operand -> named
         {% (data) => data[0] %}
         | %local
         {% (data) => new OperandNode(data[0].value, data[0].coercion) %}
         | %number
         {% (data) => new OperandNode(data[0].value, data[0].coercion) %}
