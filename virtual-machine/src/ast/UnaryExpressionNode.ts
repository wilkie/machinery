import ExpressionNode from './ExpressionNode';

class UnaryExpressionNode extends ExpressionNode {
  readonly operator: string;

  constructor(operand: ExpressionNode, operator: string) {
    super(operand);
    this.operator = operator;
  }
}

export default UnaryExpressionNode;
