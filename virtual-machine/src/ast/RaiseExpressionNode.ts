import type ComparisonNode from './ComparisonNode';
import ExpressionNode from './ExpressionNode';
import OperandNode from './OperandNode';

class RaiseExpressionNode extends ExpressionNode {
  readonly condition?: ComparisonNode;

  constructor(value: OperandNode, condition?: ComparisonNode) {
    super(value);

    this.condition = condition;
  }
}

export default RaiseExpressionNode;
