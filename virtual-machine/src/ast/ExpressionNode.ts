import Node from './Node';
import type OperandNode from './OperandNode';

class ExpressionNode extends Node {
  readonly operand: ExpressionNode | OperandNode;
  readonly coercion?: string;

  constructor(operand: ExpressionNode | OperandNode, coercion?: string) {
    super();
    this.operand = operand;
    this.coercion = coercion;
  }
}

export default ExpressionNode;
