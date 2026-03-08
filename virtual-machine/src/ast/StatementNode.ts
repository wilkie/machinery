import Node from './Node';

class StatementNode extends Node {
  readonly node: Node;
  readonly next?: StatementNode;

  constructor(node: Node, next?: StatementNode) {
    super();
    this.node = node;
    this.next = next;
  }
}

export default StatementNode;
