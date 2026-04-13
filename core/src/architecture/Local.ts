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
