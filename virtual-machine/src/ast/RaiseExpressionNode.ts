import type ComparisonNode from './ComparisonNode';
import ExpressionNode from './ExpressionNode';
import OperandNode from './OperandNode';

class RaiseExpressionNode extends ExpressionNode {
  readonly condition?: ComparisonNode;
  /** True when this is a fault (IP should point to the faulting instruction).
   *  False for software interrupts / traps (IP should point past the instruction). */
  readonly fault: boolean;

  constructor(value: OperandNode, condition?: ComparisonNode, fault = true) {
    super(value);

    this.condition = condition;
    this.fault = fault;
  }
}

export default RaiseExpressionNode;
