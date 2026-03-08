import ExpressionNode from './ExpressionNode';
import type OperandNode from './OperandNode';

class BinaryExpressionNode extends ExpressionNode {
  readonly argument: ExpressionNode | OperandNode;
  readonly operator: string;

  constructor(
    operand: ExpressionNode,
    operator: string,
    argument: ExpressionNode,
  ) {
    super(operand);
    this.operator = operator;
    this.argument = argument;
  }
}

export default BinaryExpressionNode;
