type Operation = (string | Operation)[];

/**
 * Defines the loading operation for an executable of a particular target.
 */
export interface LoadRules {
  /**
   * A boolean expression that verifies that this executable can run on the
   * particular target this ruleset is defined under.
   */
  identify?: string;
  /**
   * A set of expressions that when evaluated will load the executable into
   * memory and initialize that program state where it can then run.
   */
  operation: Operation;
}

/**
 * Holds the loader rules for particular targets.
 */
export interface LoadRulesTable {
  [target: string]: LoadRules;
}

/**
 * Describes an executable file format.
 *
 * This defines the generic data found within the executable and also a set
 * of rules to load the executable in various architectures.
 */
export interface Executable {
  /** The name of the FileFormat description that defines the executable file */
  format: string;
  /** Rules to load the executable */
  load: LoadRulesTable;
}
