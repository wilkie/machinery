import Node from './Node';
import type ExpressionNode from './ExpressionNode';
import type OperandNode from './OperandNode';

class ComparisonNode extends Node {
  readonly operand: ComparisonNode | ExpressionNode | OperandNode;

  constructor(operand: ComparisonNode | ExpressionNode | OperandNode) {
    super();
    this.operand = operand;
  }
}

export default ComparisonNode;
