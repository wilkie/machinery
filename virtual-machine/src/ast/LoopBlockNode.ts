import Node from './Node';
import LoopIfNode from './LoopIfNode';
import RepeatIfNode from './RepeatIfNode';
import StatementNode from './StatementNode';

class LoopBlockNode extends Node {
  readonly name: string;
  readonly body?: StatementNode;
  readonly condition: LoopIfNode | RepeatIfNode;

  constructor(
    name: string,
    condition: LoopIfNode | RepeatIfNode,
    body?: StatementNode,
  ) {
    super();
    this.name = name;
    this.body = body;
    this.condition = condition;
  }
}

export default LoopBlockNode;
