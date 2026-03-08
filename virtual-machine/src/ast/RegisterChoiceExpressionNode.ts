import ChoiceExpressionNode from './ChoiceExpressionNode';
import ExpressionNode from './ExpressionNode';
import RegisterOperandNode from './RegisterOperandNode';

class RegisterChoiceExpressionNode extends ChoiceExpressionNode {
  readonly choices: RegisterOperandNode[];

  constructor(
    choices: RegisterOperandNode[],
    index: ExpressionNode,
    coercion?: string,
  ) {
    super(choices, index, coercion);

    this.choices = choices;
  }
}

export default RegisterChoiceExpressionNode;
