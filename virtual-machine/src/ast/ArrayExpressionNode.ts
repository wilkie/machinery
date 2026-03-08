import ExpressionNode from './ExpressionNode';
import OperandNode from './OperandNode';

class ArrayExpressionNode extends ExpressionNode {
  readonly index: ExpressionNode;

  constructor(array: OperandNode, index: ExpressionNode) {
    super(array);
    this.index = index;
  }
}

export default ArrayExpressionNode;
