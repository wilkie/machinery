import OperandNode from './OperandNode';

import type { RegisterReference } from '../types';

class RegisterOperandNode extends OperandNode {
  readonly reference: RegisterReference;
  readonly from: OperandNode;

  constructor(
    reference: RegisterReference,
    from: OperandNode,
    coercion?: string,
  ) {
    super('', coercion);

    this.reference = reference;
    this.from = from;
  }
}

export default RegisterOperandNode;
