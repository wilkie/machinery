import Node from './Node';
import type ComparisonNode from './ComparisonNode';
import type StatementNode from './StatementNode';

class IfBlockNode extends Node {
  readonly body?: StatementNode;
  readonly elseBody?: StatementNode;
  readonly condition: ComparisonNode;

  constructor(condition: ComparisonNode, body?: StatementNode, elseBody?: StatementNode) {
    super();
    this.body = body;
    this.elseBody = elseBody;
    this.condition = condition;
  }
}

export default IfBlockNode;
