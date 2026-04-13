import type { OperationMaybeModes } from './Operation';

export interface InterruptInfo {
  /** A short identifier that uniquely identifies this interrupt vector */
  identifier: string;
  /** A more descriptive name for the interrupt vector */
  name: string;
  /** The interrupt vector index */
  index: number;
}

/**
 * Describes the built-in interrupt vectors and general interrupt handler for
 * the processor target.
 */
export interface InterruptsInfo {
  /**
   * Describes a handler routine that would run when an interrupt occurs.
   */
  handler: OperationMaybeModes;
  vectors: InterruptInfo[];
}
