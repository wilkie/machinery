import ComparisonNode from './ComparisonNode';

class UnaryLogicNode extends ComparisonNode {
  readonly operator: string;

  constructor(operand: ComparisonNode, operator: string) {
    super(operand);
    this.operator = operator;
  }
}

export default UnaryLogicNode;
