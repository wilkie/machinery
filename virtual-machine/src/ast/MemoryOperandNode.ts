import OperandNode from './OperandNode';

import type { MemoryReference } from '../types';

class MemoryOperandNode extends OperandNode {
  readonly reference: MemoryReference;
  readonly from: OperandNode;

  constructor(
    reference: MemoryReference,
    from: OperandNode,
    coercion?: string,
  ) {
    super('', coercion);

    this.reference = reference;
    this.from = from;
  }
}

export default MemoryOperandNode;
