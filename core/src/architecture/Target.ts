import type { InstructionInfo } from './Instruction';
import type { InterruptsInfo } from './Interrupt';
import type { MacrosInfo } from './Macro';
import type { MemoryInfo } from './Memory';
import type { OpcodeMatcher } from './Opcode';
import type { RegisterInfo } from './Register';
import type { FetchInfo, ModeInfo, DecodeInfo } from './Processor';

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
