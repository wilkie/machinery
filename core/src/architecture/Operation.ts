import type { LocalInfo } from './Local';

/**
 * Operations are strings that describe code. An operation might be itself
 * a set of strings to be expanded out. This can make it more obvious how
 * to specify a code body of a block.
 */
export type Operation = (string | Operation)[];

export interface OperationNotViaMode {
  /**
   * The local variables and state that are used as named by identifier in the
   * operation.
   */
  locals?: LocalInfo[];
  /**
   * The code to run just prior to when the register is accessed.
   */
  operation: Operation;
}

export interface OperationViaMode {
  /**
   * We might specify different operations depending on mode.
   */
  modes?: {
    [mode: string]: OperationNotViaMode;
  };
}

export type OperationMaybeModes = OperationNotViaMode | OperationViaMode;
