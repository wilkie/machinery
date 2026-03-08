import type ExpressionNode from './ExpressionNode';
import Node from './Node';
import type OperandNode from './OperandNode';

class ArrayAccessNode extends Node {
  readonly index: ExpressionNode;
  readonly next?: OperandNode | ArrayAccessNode;

  constructor(index: ExpressionNode, next?: OperandNode | ArrayAccessNode) {
    super();
    this.index = index;
    this.next = next;
  }
}

export default ArrayAccessNode;
