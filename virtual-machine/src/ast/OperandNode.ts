import Node from './Node';
import type ArrayAccessNode from './ArrayAccessNode';

class OperandNode extends Node {
  readonly value: string | number;
  readonly next?: OperandNode | ArrayAccessNode;
  readonly coercion?: string;

  constructor(
    value: string | number,
    coercion?: string,
    next?: OperandNode | ArrayAccessNode,
  ) {
    super();
    this.value = value;
    this.coercion = coercion;
    this.next = next;
  }
}

export default OperandNode;
