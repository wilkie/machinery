/**
 * A decoded operand from an instruction.
 */
export type DecodedOperand =
  | RegisterOperand
  | MemoryOperand
  | ImmediateOperand
  | RelativeOperand
  | FarPointerOperand;

export interface RegisterOperand {
  type: 'register';
  name: string;
  size: number;
}

export interface MemoryOperand {
  type: 'memory';
  base?: string;
  index?: string;
  displacement: number;
  displacementSize: number;
  size: number;
  segment?: string;
  /** Whether this is a direct address (no base/index, just displacement). */
  direct: boolean;
}

export interface ImmediateOperand {
  type: 'immediate';
  value: number;
  size: number;
  signed: boolean;
}

export interface RelativeOperand {
  type: 'relative';
  offset: number;
  size: number;
  /** Absolute target address (instruction address + instruction size + offset). */
  target: number;
}

export interface FarPointerOperand {
  type: 'far_pointer';
  segment: number;
  offset: number;
}

/**
 * A fully decoded instruction.
 */
export interface DecodedInstruction {
  /** Address (offset) of the instruction within the binary. */
  address: number;
  /** The raw bytes of the instruction. */
  bytes: Uint8Array;
  /** The mnemonic (e.g., 'mov', 'add', 'jl'). */
  mnemonic: string;
  /** Decoded operands in instruction order. */
  operands: DecodedOperand[];
  /** Segment override prefix, if any. */
  segmentOverride?: string;
  /** Whether this instruction has a REP/REPZ/REPNZ prefix. */
  prefix?: string;
}
