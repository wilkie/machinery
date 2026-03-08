import type ComparisonNode from './ComparisonNode';
import Node from './Node';

class LoopIfNode extends Node {
  readonly condition: ComparisonNode;

  constructor(condition: ComparisonNode) {
    super();
    this.condition = condition;
  }
}

export default LoopIfNode;
