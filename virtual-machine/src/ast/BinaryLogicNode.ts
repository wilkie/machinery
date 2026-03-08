import ComparisonNode from './ComparisonNode';

class BinaryLogicNode extends ComparisonNode {
  readonly operator: string;
  readonly argument: ComparisonNode;

  constructor(
    operand: ComparisonNode,
    operator: string,
    argument: ComparisonNode,
  ) {
    super(operand);
    this.operator = operator;
    this.argument = argument;
  }
}

export default BinaryLogicNode;
