import ExpressionNode from './ExpressionNode';
import type ComparisonNode from './ComparisonNode';

class TernaryExpressionNode extends ExpressionNode {
  readonly condition: ComparisonNode;
  readonly whenFalse: ExpressionNode;

  constructor(
    condition: ComparisonNode,
    whenTrue: ExpressionNode,
    whenFalse: ExpressionNode,
  ) {
    super(whenTrue);
    this.condition = condition;
    this.whenFalse = whenFalse;
  }
}

export default TernaryExpressionNode;
