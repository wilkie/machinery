import OperandNode from './OperandNode';

import type { LocalReference } from '../types';

class LocalOperandNode extends OperandNode {
  readonly reference: LocalReference;
  readonly from: OperandNode;

  constructor(reference: LocalReference, from: OperandNode, coercion?: string) {
    super(reference.mapping.identifier, coercion);

    this.reference = reference;
    this.from = from;
  }
}

export default LocalOperandNode;
