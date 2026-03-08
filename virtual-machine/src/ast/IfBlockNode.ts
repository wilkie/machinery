import Node from './Node';
import type ComparisonNode from './ComparisonNode';
import type StatementNode from './StatementNode';

class IfBlockNode extends Node {
  readonly body?: StatementNode;
  readonly condition: ComparisonNode;

  constructor(condition: ComparisonNode, body?: StatementNode) {
    super();
    this.body = body;
    this.condition = condition;
  }
}

export default IfBlockNode;
