import {
  ArrayAccessNode,
  BinaryExpressionNode,
  ComparisonNode,
  ExpressionNode,
  OperandNode,
  TernaryExpressionNode,
  UnaryExpressionNode,
  UnaryLogicNode,
} from './ast';

export interface Evaluatable {
  [key: string]: Evaluatable | number | number[] | string;
}

/**
 * Performs the given ExpressionNode and yields the result.
 */
class Evaluator {
  readonly locals: Evaluatable;

  constructor(locals: Evaluatable) {
    this.locals = locals;
  }

  evaluateTruth(node: ComparisonNode | OperandNode): boolean {
    if (node instanceof OperandNode) {
      return false;
    } else if (node instanceof UnaryLogicNode) {
      if (node.operator === '!') {
        return !this.evaluateTruth(node.operand);
      }
    } else {
      return true;
    }

    console.log(node);
    throw new Error('unknown logic node');
    return false;
  }

  evaluate(node: ExpressionNode | OperandNode): number {
    if (node instanceof OperandNode) {
      if (typeof node.value === 'number') {
        return node.value;
      } else if (typeof node.value === 'boolean') {
        return node.value;
      } else if (typeof node.value === 'string') {
        // TODO: lookup operand
        let operand: OperandNode | ArrayAccessNode | undefined = node;
        let current: Evaluatable | number | undefined = this.locals;
        while (current) {
          if (operand instanceof OperandNode) {
            current = current[operand.value] as Evaluatable;
            operand = operand.next;
          } else if (operand instanceof ArrayAccessNode) {
            const index = this.evaluate(operand.index);
            current = current[index] as Evaluatable;
            operand = operand.next;
          } else {
            break;
          }
        }
        return typeof current === 'number' ? current : 0;
      }
    } else if (node instanceof UnaryExpressionNode) {
      if (node.operator === '~') {
        return ~this.evaluate(node.operand);
      } else {
        throw new Error('unknown unary operator');
      }
    } else if (node instanceof BinaryExpressionNode) {
      if (node.operator === '+') {
        return this.evaluate(node.operand) + this.evaluate(node.argument);
      } else if (node.operator === '-') {
        return this.evaluate(node.operand) - this.evaluate(node.argument);
      } else if (node.operator === '/') {
        return this.evaluate(node.operand) / this.evaluate(node.argument);
      } else if (node.operator === '//') {
        return Math.floor(
          this.evaluate(node.operand) / this.evaluate(node.argument),
        );
      } else if (node.operator === '*') {
        return this.evaluate(node.operand) * this.evaluate(node.argument);
      } else if (node.operator === '%') {
        return this.evaluate(node.operand) & this.evaluate(node.argument);
      } else if (node.operator === '&') {
        return this.evaluate(node.operand) & this.evaluate(node.argument);
      } else if (node.operator === '|') {
        return this.evaluate(node.operand) | this.evaluate(node.argument);
      } else {
        throw new Error('unknown binary operator');
      }
    } else if (node instanceof TernaryExpressionNode) {
      const condition = this.evaluate(node.condition);
      if (condition) {
        return this.evaluate(node.operand);
      }
      return this.evaluate(node.whenFalse);
    } else if (node.operand instanceof OperandNode) {
      return this.evaluate(node.operand);
    }

    console.log(node);
    throw new Error('unknown node');
  }
}

export default Evaluator;
