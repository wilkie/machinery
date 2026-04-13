/**
 * Auxiliary data encoded into the instruction stream.
 */
export const InstructionDataTypes = {
  /** The default type; an encoding of which operation is to be done. */
  Opcode: 0,
  /** Some kind of auxiliary operand data. */
  Operand: 1,
  /** An amount to nudge an address. */
  Displacement: 2,
  /** A constant numerical value. */
  Immediate: 4,
  /** When a displacement can be written out as a pseudo-immediate value */
  DisplacementImmediate: 2 | 4,
} as const;

export type InstructionDataType =
  (typeof InstructionDataTypes)[keyof typeof InstructionDataTypes];

/**
 * The type data sources/destinations an instruction operand could have.
 */
export const InstructionOperandTypes = {
  /** Describes an operand that is equivalent to the source operand. */
  Source: 0,
  /** The operand represents a general register. */
  Register: 1,
  /** The operand represents a system register. */
  SystemRegister: 2,
  /** The operand represents a memory location. */
  Memory: 3,
  /** The operand represents an interrupt vector. */
  Interrupt: 4,
  /** The operand represents a relative jump. */
  RelativeJump: 5,
  /** The operand represents an absolute jump. */
  AbsoluteJump: 6,
} as const;

export type InstructionOperandType =
  (typeof InstructionOperandTypes)[keyof typeof InstructionOperandTypes];

export interface OpcodeMatcherFieldBase {
  /** The identifier used to refer to this field in code. */
  identifier: string;
  /** The name for this opcode field. */
  name?: string;
  /** The bit offset into the stream that represents this field. */
  offset: number;
  /** The width of this field by number of bits. */
  size: number;
  /** Whether or not this field is interpreted as signed (2's complement). */
  signed?: boolean;
  /** Requires this field to match exactly the given number. */
  match?: number;
  /**
   * The value to use when this field is referenced as a local variable.
   * If not set, defaults to the match value. Useful when the match value
   * is consumed for form identification but a different value is needed
   * for code generation (e.g., direct displacement uses rm=6 for matching
   * but needs rm=0 to select the DS segment base).
   *
   * This is also used when encoding this operand and it does not have an
   * exact match. This is useful for when fields don't actually care what
   * value they are, but assemblers should note that there is a preferred
   * pattern to actually use.
   * */
  value?: number;
  /**
   * The encoding expected to map to the subfield value when assembled.
   *
   * When any value is null, the specified encoding is not allowed.
   */
  encoding?: (string | null)[];
}

export interface OpcodeMatcherFieldNonRegister extends OpcodeMatcherFieldBase {
  /** The type of information this subfield conveys, if any. */
  type?: Exclude<
    InstructionOperandType,
    (typeof InstructionOperandTypes)['Register']
  >;
}

export interface OpcodeMatcherFieldRegister extends OpcodeMatcherFieldBase {
  /** Explicitly the 'register' opcode field type */
  type: (typeof InstructionOperandTypes)['Register'];
  /**
   * The register names that this opcode field encodes
   *
   * When any value is null, the specified encoding is not allowed.
   */
  encoding?: (string | null)[];
}

/**
 * Describes a field and/or pattern to match against a bit stream.
 */
export type OpcodeMatcherField =
  | OpcodeMatcherFieldNonRegister
  | OpcodeMatcherFieldRegister;

/**
 * A description of how to match a bytestream in order to identify a particular
 * operation in the machine code.
 */
export interface OpcodeMatcher {
  /** The identifier used to refer to this opcode field. */
  identifier: string;
  /** The name for this opcode field. */
  name?: string;
  /** Describes the data we are reading. */
  type: InstructionDataType;
  /** Describes the number of bits to match or read. */
  size: number;
  /** Whether or not this field is interpreted as signed (2's complement). */
  signed?: boolean;
  /** An optional set of subfield matches to further differentiate this overall pattern. */
  fields?: OpcodeMatcherField[];
  /**
   * Opcode byte aliases. When this matcher is inserted into the decoder trie,
   * the same reference is also inserted at each alias byte position.
   * All forms sharing this opcode byte must declare the same aliases.
   */
  aliases?: number[];
}
