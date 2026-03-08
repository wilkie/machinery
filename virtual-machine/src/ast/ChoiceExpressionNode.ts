import ExpressionNode from './ExpressionNode';
import OperandNode from './OperandNode';

class ChoiceExpressionNode extends ExpressionNode {
  readonly choices: OperandNode[];
  readonly coercion?: string;

  constructor(
    choices: OperandNode[],
    index: ExpressionNode,
    coercion?: string,
  ) {
    super(index);
    this.choices = choices;
    this.coercion = coercion;
  }

  get index(): ExpressionNode {
    return this.operand as ExpressionNode;
  }
}

export default ChoiceExpressionNode;
