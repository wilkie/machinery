import type { DecodedInstruction, DecodedOperand } from './types';

/**
 * A syntax formatter renders decoded instructions into human-readable text.
 */
export interface Syntax {
  /** Format a decoded instruction into its string representation. */
  formatInstruction(instr: DecodedInstruction): string;
  /** Format a single operand. */
  formatOperand(operand: DecodedOperand, instrAddress: number): string;
}
