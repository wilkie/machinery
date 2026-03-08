import type ExpressionNode from './ExpressionNode';
import type ChoiceExpressionNode from './ChoiceExpressionNode';
import Node from './Node';
import type OperandNode from './OperandNode';

class AssignmentNode extends Node {
  readonly destination: OperandNode | ChoiceExpressionNode;
  readonly expression: ExpressionNode;

  constructor(
    destination: OperandNode | ChoiceExpressionNode,
    expression: ExpressionNode,
  ) {
    super();
    this.destination = destination;
    this.expression = expression;
  }
}

export default AssignmentNode;
