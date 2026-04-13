import type { OperationMaybeModes } from './Operation';

/**
 * Describes the data representation for a register of a machine target.
 */
export const RegisterTypes = {
  /** Holds an integer value. */
  Integer: 0,
  /** Holds a standard IEEE-754 floating point value. */
  FloatingPoint: 1,
  /** Holds a segment selector value. */
  Segment: 2,
} as const;

export type RegisterType = (typeof RegisterTypes)[keyof typeof RegisterTypes];

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
  /** Optionally, how this field is further broken down into named subfields. */
  fields?: RegisterField[];
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
  get?: OperationMaybeModes;
  /** Some optional code that runs after this register is written */
  set?: OperationMaybeModes;
}
