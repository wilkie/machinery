import type { BusRequestInfo } from './Bus';
import type { InstructionInfo } from './Instruction';
import type { InterruptsInfo } from './Interrupt';
import type { LocalInfo } from './Local';
import type { MacrosInfo } from './Macro';
import type { MemoryInfo } from './Memory';
import type { OpcodeMatcher } from './Opcode';
import type { Operation, OperationNotViaMode } from './Operation';
import type { RegisterInfo } from './Register';

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
  /** Maximum instruction length in bytes (including prefixes). Exceeding this raises #GP. */
  maxInstructionLength?: number;
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
  unknown?: OperationNotViaMode;
  /**
   * Instruction length limit in bytes. When the number of consumed bytes
   * exceeds this value, the `operation` is executed (typically raising #GP).
   */
  limit?: {
    bytes: number;
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
    dword?: {
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
 * A mode in which a core can be currently operating.
 *
 * A mode can have its own decoding behavior and generally has its own decoding
 * function in generated simulators.
 */
export interface ModeInfo {
  /** A simple unique key identifying this mode. */
  identifier: string;
  /** A descriptive name for this mode. */
  name: string;
  /** A description of the decode phase for this mode. */
  decode?: DecodeInfo;
  /** The operation optionally performed when the mode is transitioned into. */
  transition?: OperationNotViaMode;
}

/**
 * Describes a processor core and its instruction set and its internal state.
 */
export interface ProcessorInfo {
  /** A unique identifier for this machine. */
  identifier: string;
  /** A unique family classification this machine belongs to. */
  family: string;
  /** The identifying name for the machine. */
  name: string;
  /** A descriptive name for the machine. */
  description: string;
  /** Typical buses attached to the core */
  buses?: BusRequestInfo[];
  /** Internal memory */
  memory?: MemoryInfo[];
  /** A description of all the registers for this machine target. */
  registers: RegisterInfo[];
  /** An optional description of all internal registers for this machine target. */
  state?: RegisterInfo[];
  /** A description of all of the instructions that machine target can execute. */
  instructions: InstructionInfo[];
  /** A description of modes the code can be in */
  modes?: ModeInfo[];
  /** A description of the fetch phase. */
  fetch?: FetchInfo;
  /** A description of the decode phase. */
  decode?: DecodeInfo;
  /** Common operand matchers */
  operands: OpcodeMatcher[];
  /** A description of the interrupts for this target */
  interrupts?: InterruptsInfo;
  /** Any macros defined for the machine */
  macros?: MacrosInfo;
  /**
   * The byte order for multi-byte values in instruction encoding.
   * - 'little': least significant byte first (x86, ARM in LE mode)
   * - 'big': most significant byte first (68k, MIPS in BE mode)
   * - 'bi': switchable at runtime (ARM, MIPS) — assembler defaults to target convention
   */
  endianness?: 'little' | 'big' | 'bi';
  /**
   * The byte value used for alignment padding. Defaults to 0x00.
   * x86 uses 0x90 (NOP) so alignment padding is executable as no-ops.
   */
  alignmentFill?: number;
}
