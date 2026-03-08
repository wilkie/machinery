import ExpressionNode from './ExpressionNode';
import OperandNode from './OperandNode';

class CallExpressionNode extends ExpressionNode {
  readonly args: ExpressionNode[];

  constructor(name: string, args: ExpressionNode[]) {
    super(new OperandNode(name));
    this.args = args;
  }

  get name(): string {
    return (this.operand as OperandNode).value.toString();
  }
}

export default CallExpressionNode;
