import ComparisonNode from './ComparisonNode';
import type ExpressionNode from './ExpressionNode';

class ComparisonEvaluationNode extends ComparisonNode {
  readonly operator: string;
  readonly argument: ComparisonNode | ExpressionNode;

  constructor(
    operand: ComparisonNode | ExpressionNode,
    operator: string,
    argument: ComparisonNode | ExpressionNode,
  ) {
    super(operand);
    this.operator = operator;
    this.argument = argument;
  }
}

export default ComparisonEvaluationNode;
