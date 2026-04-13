import type { OpcodeMatcher } from './Opcode';
import type { OperationMaybeModes, OperationNotViaMode } from './Operation';
import type { MacrosInfo } from './Macro';
import type { LocalInfo } from './Local';

export interface FinalizableOperationNotViaMode extends OperationNotViaMode {
  /**
   * A set of pseudo-code that describes what to do after the operation has finished.
   */
  finalize?: string[];
}

export interface FinalizableOperationViaMode<
  T = FinalizableOperationNotViaMode,
> {
  /**
   * We might specify different operations depending on mode.
   */
  modes?: {
    [mode: string]: T;
  };
}

export interface InstructionFormBase {
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
   * If known, the number of cycles this instruction takes to complete.
   */
  cycles?: number;
  /**
   * The operand size in bits, if needed.
   */
  operandSize?: number;
  /**
   * The assembly-syntax operands for this form, in Intel order (destination first).
   * References field identifiers from OpcodeMatcher fields ('rm', 'reg', 'seg')
   * and keywords: 'imm' (immediate), 'rel' (relative offset), 'ptr' (far pointer).
   */
  operands?: string[];
  /**
   * A list of allowed prefixes by name, which is especially important for
   * prefixes that have been disallowed.
   */
  allow?: string[];
  /**
   * An optional specific name for this form.
   */
  name?: string;
  /**
   * An extended description detailing this form in particular.
   */
  description?: string;
  /**
   * If this form is a segment override prefix, the name of the segment register
   * it overrides (e.g., 'ES', 'CS', 'SS', 'DS'). Used by assembler generators
   * to emit the correct prefix byte when a memory operand specifies a segment.
   */
  segmentOverride?: string;
  /**
   * The distance qualifier for branch/return instructions.
   * - 'short': smallest intra-segment offset (e.g., 8-bit on x86)
   * - 'near': full intra-segment offset or indirect
   * - 'far': inter-segment (e.g., CALL FAR, RETF)
   */
  distance?: 'short' | 'near' | 'far';
  /**
   * How the branch target is specified.
   * - 'relative': offset from current instruction pointer
   * - 'absolute': direct or indirect absolute address
   */
  addressing?: 'relative' | 'absolute';
  /**
   * Encoding priority for tiebreaking when multiple forms produce the same
   * byte count. Lower values are preferred. Defaults to 0 if not specified.
   */
  encodingPriority?: number;
}

export type InstructionFormFlat = InstructionFormBase &
  FinalizableOperationNotViaMode;
export type InstructionFormModes = InstructionFormBase &
  FinalizableOperationViaMode<
    Omit<InstructionFormBase, 'opcode'> & FinalizableOperationNotViaMode
  >;

/**
 * Describes a variant of an instruction. Useful within the context of an
 * InstructionInfo description.
 */
export type InstructionForm = InstructionFormFlat | InstructionFormModes;

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
  /**
   * When this is a prefix, whether or not instructions need to explicitly
   * mark that they are allowed with this prefix.
   */
  disallowed?: OperationMaybeModes;
  /** Whether the instruction's operands are commutative (order doesn't affect result). */
  commutative?: boolean;
  /** Flags left undefined, if any */
  undefined?: string[];
  /** Any macros defined just for this instruction */
  macros?: MacrosInfo;
  /** Any locals defined for all instruction forms */
  locals?: LocalInfo[];
  /** A set of different forms the instruction may take depending on arguments. */
  forms: InstructionForm[];
}
