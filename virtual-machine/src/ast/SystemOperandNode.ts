import OperandNode from './OperandNode';

import type { SystemReference } from '../types';

class SystemOperandNode extends OperandNode {
  readonly reference: SystemReference;
  readonly from: OperandNode;

  constructor(
    reference: SystemReference,
    from: OperandNode,
    coercion?: string,
  ) {
    super('', coercion);

    this.reference = reference;
    this.from = from;
  }
}

export default SystemOperandNode;
