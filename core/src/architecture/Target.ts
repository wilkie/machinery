/**
 * Operations are strings that describe code. An operation might be itself
 * a set of strings to be expanded out. This can make it more obvious how
 * to specify a code body of a block.
 */
export type Operation = (string | Operation)[];

/**
 * Describes the data representation for a register of a machine target.
 */
export enum RegisterType {
  /** Holds an integer value. */
  Integer = 0,
  /** Holds a standard IEEE-754 floating point value. */
  FloatingPoint = 1,
}

/**
 * Describes a space within a register.
 */
export interface RegisterField {
  /** The identifier used to refer to this register field. */
  identifier: string;
  /** The name of the register field. */
  name?: string;
  /** A description of this register field. */
  description?: string;
  /** The bit position to start the field. */
  offset: number;
  /** The number of bits the field is wide. */
  size: number;
  /** Whether or not this register field can be referred to on its own. */
  global?: boolean;
}

/**
 * Describes a specific register for a machine target.
 */
export interface RegisterInfo {
  /** The identifier used to refer to this register. */
  identifier: string;
  /** The name of the register. */
  name?: string;
  /** A description of this register. */
  description?: string;
  /** Any alternative names for the register. */
  aliases?: string[];
  /** The size of the register as a number of bits. */
  size: number;
  /** The type of data representation expected of this register. */
  type: RegisterType;
  /** Optionally, how the register is further broken down into named subfields. */
  fields?: RegisterField[];
  /** Optionally, what the initial value of the register should be. Defaults to 0 for integer types. */
  initialValue?: number;
  /** Some optional code that runs before this register is accessed */
  get?: {
    /**
     * The local variables and state that are used as named by identifier in the
     * operation.
     */
    locals?: LocalInfo[];
    /**
     * The code to run just prior to when the register is accessed.
     */
    operation?: Operation;
    /**
     * We might specify different operations depending on mode.
     */
    modes?: {
      [mode: string]: {
        /**
         * The local variables and state that are used as named by identifier in the
         * operation.
         */
        locals?: LocalInfo[];
        /**
         * The code to run just after the register is modified.
         */
        operation: Operation;
      };
    };
  };
  /** Some optional code that runs after this register is written */
  set?: {
    /**
     * The local variables and state that are used as named by identifier in the
     * operation.
     */
    locals?: LocalInfo[];
    /**
     * The code to run just after the register is modified.
     */
    operation?: Operation;
    /**
     * We might specify different operations depending on mode.
     */
    modes?: {
      [mode: string]: {
        /**
         * The local variables and state that are used as named by identifier in the
         * operation.
         */
        locals?: LocalInfo[];
        /**
         * The code to run just after the register is modified.
         */
        operation: Operation;
      };
    };
  };
}

/**
 * Auxiliary data encoded into the instruction stream.
 */
export enum InstructionDataType {
  /** The default type; an encoding of which operation is to be done. */
  Opcode = 0,
  /** Some kind of auxiliary operand data. */
  Operand = 1,
  /** An amount to nudge an address. */
  Displacement = 2,
  /** A constant numerical value. */
  Immediate = 3,
}

/**
 * The type data sources/destinations an instruction operand could have.
 */
export enum InstructionOperandType {
  /** Describes an operand that is equivalent to the source operand. */
  Source = 0,
  /** The operand represents a general register. */
  Register = 1,
  /** The operand represents a system register. */
  SystemRegister = 2,
  /** The operand represents a memory location. */
  Memory = 3,
  /** The operand represents an interrupt vector. */
  Interrupt = 4,
  /** The operand represents a relative jump. */
  RelativeJump = 5,
  /** The operand represents an absolute jump. */
  AbsoluteJump = 6,
}

/**
 * Describes a field and/or pattern to match against a bit stream.
 */
export interface OpcodeMatcherField {
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
  /** The type of information this subfield conveys, if any. */
  type?: InstructionOperandType;
  /** Requires this field to match exactly the given number. */
  match?: number;
}

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
}

export interface MacrosInfo {
  [key: string]: string | number | string[];
}

export interface LocalInfo {
  /**
   * The identifier used to refer to the variable within operations.
   */
  identifier: string;
  /**
   * A constant value this local actually represents.
   */
  value?: number;
  /**
   * The long form name of the local, if any.
   */
  name?: string;
  /**
   * A description of what this local is for, if any.
   */
  description?: string;
  /**
   * The size of the local variable in bits.
   */
  size: number;
  /**
   * Whether or not the local variable is considered signed.
   */
  signed?: boolean;
}

/**
 * Describes a variant of an instruction. Useful within the context of an
 * InstructionInfo description.
 */
export interface InstructionForm {
  /**
   * The byte code or codes that represents this instruction.
   *
   * If the value is over 255, then this value signifies a command to
   * interpret the byte value instead of considering it needing to be
   * an exact match. With this, one can describe opcode forms that are
   * masked by particular values to depict a different form of the
   * instruction.
   */
  opcode: (number | string | OpcodeMatcher)[];
  /** Any alternative short-form names for the instruction when in this form. */
  aliases?: string[];
  /** Overrides for flags modified, if any */
  modifies?: string[];
  /** Overrides for any flags left undefined, if any */
  undefined?: string[];
  /**
   * The local variables and state that are used as named by identifier in the
   * operation.
   */
  locals?: LocalInfo[];
  /**
   * A set of pseudo-code that describes the operation being performed.
   */
  operation?: Operation;
  /**
   * A set of pseudo-code that describes what to do after the operation has finished.
   */
  finalize?: string[];
  /**
   * If known, the number of cycles this instruction takes to complete.
   */
  cycles?: number;
  /**
   * The operand size in bits, if needed.
   */
  operandSize?: number;
  /**
   * An optional specific name for this form.
   */
  name?: string;
  /**
   * An extended description detailing this form in particular.
   */
  description?: string;
  /**
   * We might specify different values depending on mode.
   */
  modes?: {
    [mode: string]: Omit<InstructionForm, 'opcode'>;
  };
}

/**
 * Describes a single type of instruction.
 */
export interface InstructionInfo {
  /** A short-form name for the instruction e.g. ADD */
  identifier: string;
  /** Any alternative short-form names for the instruction. */
  aliases?: string[];
  /** A full name for the instruction, possibily can be localized */
  name: string;
  /** A short description of the instruction. */
  description: string;
  /** Flags modified, if any */
  modifies: string[];
  /** Whether or not this instruction is actually a prefix to another one. */
  prefix?: boolean;
  /** Flags left undefined, if any */
  undefined?: string[];
  /** Any macros defined just for this instruction */
  macros?: MacrosInfo;
  /** Any locals defined for all instruction forms */
  locals?: LocalInfo[];
  /** A set of different forms the instruction may take depending on arguments. */
  forms: InstructionForm[];
}

/**
 * Describes general information about the fetch mechanism.
 */
export interface FetchInfo {
  /**
   * Which register is the instruction pointer.
   *
   * If this is a list, this means there is a segment or page register that is
   * involved in computing the effective address. The actual effective address
   * should be provided in `effectiveRegister`.
   *
   * The order these should be specified is in increasingly specific order. So,
   * if there is a segment register, this is specified first. So the next
   * register indicates the offset within the segment.
   *
   * Ex. In x86, the instruction pointer is indicated by the combination of the
   * code segment selector and the instruction pointer. This is then:
   * `['CS', 'IP']`.
   */
  register: string | string[];
  /**
   * The precomputed register that actually indicates the address. This is likely
   * an interal register (one provided in the machine 'state' rather than the
   * official registers).
   *
   * If none provided, this is just assumed to be the register given in `register`.
   */
  effectiveRegister?: string;
  /**
   * Which memory to fetch instructions from (and for which the instruction
   * pointer references). Refers to a memory identifier.
   */
  memory: string;
  /** Whether or not the instruction pointer advances during the fetch phase. */
  advancePointer?: boolean;
}

/**
 * Describes the decoder.
 */
export interface DecodeInfo {
  /**
   * The local variables and state that are used as named by identifier in any
   * decode operation.
   */
  locals?: LocalInfo[];
  /**
   * The operation that occurs when a new instruction is decoded.
   */
  initialization?: {
    /**
     * The local variables and state that are used as named by identifier in the
     * decode initialization routine.
     */
    locals?: LocalInfo[];
    /**
     * The code to run just prior to the decoding of an instruction.
     */
    operation: Operation;
  };
  /**
   * Describes read operations for the decoding phase.
   *
   * The decoder will occasionally need to fetch data from the instruction
   * memory. This directs the decoder to how to do that.
   */
  read?: {
    /**
     * Describes how to fetch a byte.
     */
    byte?: {
      /**
       * The local variables and state that are used as named by identifier in any
       * byte read.
       *
       * The local described might be repeated for subsequent byte reads which means
       * it will be assigned a new name. This just helps reference the read byte in
       * the read operation.
       */
      locals?: LocalInfo[];
      /**
       * The code to run to read a byte from instruction memory.
       */
      operation: Operation;
    };
    /**
     * Describes how to fetch a word (two bytes).
     */
    word?: {
      /**
       * The local variables and state that are used as named by identifier in any
       * two byte read.
       *
       * The local described might be repeated for subsequent reads which means it
       * will be assigned a new name. This just helps reference the read data in
       * the read operation.
       */
      locals?: LocalInfo[];
      /**
       * The code to run to read two bytes from instruction memory.
       */
      operation: Operation;
    };
    /**
     * Describes how to fetch a double word (four bytes).
     */
    double?: {
      /**
       * The local variables and state that are used as named by identifier in any
       * four byte read.
       *
       * The local described might be repeated for subsequent reads which means it
       * will be assigned a new name. This just helps reference the read data in
       * the read operation.
       */
      locals?: LocalInfo[];
      /**
       * The code to run to read four bytes from instruction memory.
       */
      operation: Operation;
    };
    /**
     * Describes how to fetch a quad word (eight bytes).
     */
    quad?: {
      /**
       * The local variables and state that are used as named by identifier in any
       * eight byte read.
       *
       * The local described might be repeated for subsequent reads which means it
       * will be assigned a new name. This just helps reference the read data in
       * the read operation.
       */
      locals?: LocalInfo[];
      /**
       * The code to run to read eight bytes from instruction memory.
       */
      operation: Operation;
    };
  };
}

/**
 * Common properties of a field within a memory region.
 */
export interface MemoryRegionBaseField {
  /** The identifier for the field. */
  identifier: string;
  /** The optional full name of the field. */
  name?: string;
  /** An optional description of the field. */
  description?: string;
  /** The bit position to start the field. */
  offset: number;
  /** The number of bits the field is wide. */
  size: number;
}

export interface MemoryRegionSubdividedField extends MemoryRegionBaseField {
  fields?: MemoryRegionBaseField[];
}

export interface MemoryRegionArrayField extends MemoryRegionBaseField {
  count: number;
  cell: Omit<MemoryRegionSubdividedField, 'offset' | 'size'>;
}

export type MemoryRegionField =
  | MemoryRegionSubdividedField
  | MemoryRegionArrayField;

export interface BaseMemoryRegion {
  identifier: string;
  name: string;
  size?: number;
}

export interface BaseOffsettableMemoryRegion extends BaseMemoryRegion {
  offset?: number | string;
}

export interface BaseSubdividedMemoryRegion extends BaseOffsettableMemoryRegion {
  fields: BaseOffsettableMemoryRegion[];
}

export interface ReadOnlyMemoryRegion extends BaseOffsettableMemoryRegion {
  data: number[];
  fields?: MemoryRegionField[];
}

export interface ReadWriteMemoryRegion extends BaseOffsettableMemoryRegion {
  data?: number[];
  fields?: MemoryRegionField[];
}

export type MemoryRegion = ReadOnlyMemoryRegion | ReadWriteMemoryRegion;

export type MemoryType = 'rom' | 'ram' | 'programmable';

export interface BaseMemoryInfo {
  identifier: string;
  name: string;
  endian: 'little' | 'big';
  min?: number;
  max?: number;
  length?: number;
}

export type ReadOnlyMemoryInfo = BaseMemoryInfo & {
  type: 'rom';
  regions?: ReadOnlyMemoryRegion[];
};

export type ReadWriteMemoryInfo = BaseMemoryInfo & {
  type: 'ram';
  regions?: ReadWriteMemoryRegion[];
};

export type ProgrammableMemoryInfo = BaseMemoryInfo & {
  type: 'programmable';
  regions?: ReadWriteMemoryRegion[];
};

export type MemoryInfo =
  | ReadOnlyMemoryInfo
  | ReadWriteMemoryInfo
  | ProgrammableMemoryInfo;

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
  handler: {
    /**
     * The local variables and state that are used as named by identifier in the
     * operation.
     */
    locals?: LocalInfo[];
    /**
     * The code to run to handle a generic interrupt.
     */
    operation: Operation;
  };
  vectors: InterruptInfo[];
}

/**
 * A mode in which a core can be currently operating.
 */
export interface ModeInfo {
  /** A simple unique key identifying this mode. */
  identifier: string;
  /** A descriptive name for this mode. */
  name: string;
  /** A description of the decode phase for this mode. */
  decode?: DecodeInfo;
  /** The operation optionally performed when the mode is transitioned into. */
  transition?: {
    /**
     * The local variables and state that are used as named by identifier in the
     * operation.
     */
    locals?: LocalInfo[];
    /**
     * The code to run to initialize the mode.
     */
    operation: Operation;
  };
}

/**
 * A general description of a machine target.
 */
export interface Target {
  /** A unique identifier for this machine. */
  identifier: string;
  /** A unique family classification this machine belongs to. */
  class: string;
  /** The identifying name for the machine. */
  name: string;
  /** A descriptive name for the machine. */
  description: string;
  /** A description of all the registers for this machine target. */
  registers: RegisterInfo[];
  /** A description of the memory map. */
  memory?: MemoryInfo[];
  /** A description of the fetch phase. */
  fetch?: FetchInfo;
  /** A description of the decode phase. */
  decode?: DecodeInfo;
  /** An optional description of all internal registers for this machine target. */
  state?: RegisterInfo[];
  /** A description of all of the instructions that machine target can execute. */
  instructions: InstructionInfo[];
  /** Common operand matchers */
  operands: OpcodeMatcher[];
  /** Any macros defined for the machine */
  macros?: MacrosInfo;
  /** A description of the interrupts for this target */
  interrupts?: InterruptsInfo;
  /** A description of modes the code can be in */
  modes?: ModeInfo[];
}
