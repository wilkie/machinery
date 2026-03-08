import type ComparisonNode from './ComparisonNode';
import Node from './Node';

class NextIfNode extends Node {
  readonly condition: ComparisonNode;

  constructor(condition: ComparisonNode) {
    super();
    this.condition = condition;
  }
}

export default NextIfNode;
