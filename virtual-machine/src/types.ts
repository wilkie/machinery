import type {
  InstructionForm,
  InstructionInfo,
  OpcodeMatcher,
  MemoryInfo,
  MemoryRegion,
  MemoryType,
  LocalInfo,
} from '@machinery/core';

import type { StatementNode, ExpressionNode } from './ast';

export interface ExpressionType {
  size: number;
  signed: boolean;
}

export interface LocalsInfo {
  [key: string]: LocalInfo;
}

export interface LocalMapping {
  identifier: string;
  size?: number;
  signed?: boolean;
}

export interface LocalMap {
  [key: string]: LocalMapping;
}

export interface MemoryMapping {
  start: number;
  size: number;
  length: number;
  data: Uint8Array | number[];
  type?: MemoryType;
  info: MemoryInfo | MemoryRegion;
}

export interface MemoryMap {
  [key: string]: MemoryMapping;
}

export interface RegisterMapping {
  size: number;
  index: number;
  identifier: string;
  initialValue?: number;
  offset?: number;
  length?: number;
}

export interface RegisterMap {
  start: number;
  size: number;
  registers: {
    [key: string]: RegisterMapping;
  };
}

export interface InputMap {
  [key: string]: number;
}

export interface DecoderPartial {
  matcher: string | OpcodeMatcher;
  mask: number;
  and: number;
  map: Partial<DecoderMap>;
}

export interface DecoderWildcard {
  matcher: string | OpcodeMatcher;
  map: DecoderMap;
}

export interface DecoderMap {
  instruction?: InstructionInfo;
  form?: InstructionForm;
  variant?: number;
  inputs?: InputMap;
  index?: number;
  exact?: DecoderMap[];
  partial?: DecoderPartial[];
  wildcard?: DecoderWildcard;
}

export interface ParsedOperation {
  localMap: LocalMap;
  statement: StatementNode;
}

export interface BaseReference {
  identifier: string;
}

export interface RegisterReference extends BaseReference {
  type: 'register';
  mapping: RegisterMapping;
  size: number;
  signed?: boolean;
}

export interface MemoryAccessReference extends BaseReference {
  type: 'memory';
  index: ExpressionNode;
  size: number;
  signed?: boolean;
  references?: MemoryAccessReference | MemoryFieldReference;
}

export interface MemoryFieldReference extends BaseReference {
  type: 'memory';
  offset: number;
  size?: number;
  signed?: boolean;
  references?: MemoryAccessReference | MemoryFieldReference;
}

export interface MemoryReference extends BaseReference {
  type: 'memory';
  mapping: MemoryMapping;
  address: ExpressionNode;
  offset?: number;
  size: number;
  signed?: boolean;
  references?: MemoryAccessReference | MemoryFieldReference;
}

export interface LocalReference extends BaseReference {
  type: 'local';
  mapping: LocalMapping;
}

export interface SystemReference extends BaseReference {
  type: 'system';
  tags: string[];
}

/**
 * An identifier reference.
 *
 * It may be a chain via 'references' for dotted identifiers. For instance,
 * `RAM.IVT[%{vector}].segment` would be something like:
 *
 * ```
 * {
 *   type: 'memory',
 *   identifier: 'RAM',
 *   references: {
 *     type: 'region',
 *     identfier: 'IVT',
 *     offset: 0,
 *     references: {
 *       type: 'element',
 *       identfier: 'IVD',
 *       index: <ExpressionNode>,
 *       references: {
 *         type: 'field',
 *         identfier: 'segment',
 *         offset: 0,
 *         size: 16,
 *         endian: 'little',
 *         references: undefined,
 *       },
 *     },
 *   },
 * },
 * ```
 *
 * There are many types of objects that can be referenced by a `Reference`.
 * A `memory` is an overall memory device. A `region` is then a named slice
 * of memory. Within that slice might be a `field`, but a `field` can also
 * be a slice of a `register`.
 *
 * For array references, such an access would result in an `element` type
 * which would have an `index` property. That property could be a number if
 * the array is indexed by a constant value, or an `ExpressionNode` for more
 * complicated situations.
 *
 * Ultimately, when the reference terminates (when `references` is
 * `undefined`,) there will be properties which will help determine how to
 * process the value. The `size` will indicate the width of the field. The
 * `offset` helps identify where in the parent object that bitstream starts.
 * And the `endian` property will indicate the expected byte order.
 */
export type Reference =
  | RegisterReference
  | MemoryReference
  | MemoryAccessReference
  | LocalReference
  | SystemReference;

export interface GeneratorContext {
  /** The current mode we are generating for */
  mode: string;
  /**
   * Contains all knowledge about defined locals.
   */
  locals: LocalsInfo;
  /**
   * Contains the mapping between local and an allocated variable.
   */
  localMap: LocalMap;
}

/**
 * Contains the generated code for a StatementNode and information about how it
 * affects state.
 */
export interface RegisterChoiceInfo {
  indexExpr: string;
  registerIndex: number;
}

export interface GeneratedStatement {
  /**
   * Contains a list of accessed registers.
   */
  accesses: string[];
  /**
   * Contains a list of modified registers.
   */
  modifies: string[];
  /**
   * Registers from choice expressions that need conditional get/set guards.
   * Maps register name to the index expression and the register's expected
   * index value. When present, get/set side effects for these registers are
   * only emitted inside a conditional check (indexExpr === registerIndex).
   */
  choiceIndexes?: { [name: string]: RegisterChoiceInfo };
  /**
   * Contains the code that was generated.
   */
  code: string[];
  /**
   * Contains parsing context.
   */
  context: GeneratorContext;
}

/**
 * Contains the parsed code for a target.
 */
export interface IntermediateRepresentation {
  /**
   * Each instruction form expanded into an AST form
   *
   * For each instruction, there is an AST root node in an array conforming to
   * the array of instruction forms described in the target.
   */
  instructions: {
    [key: string]: {
      [mode: string]: {
        operation: ParsedOperation;
        finalize?: ParsedOperation;
      };
    }[];
  };
  /** Parsed and resolved register get/set operations */
  registers: {
    [key: string]: {
      get?: {
        [mode: string]: ParsedOperation;
      };
      set?: {
        [mode: string]: ParsedOperation;
      };
    };
  };
  /** Parsed and resolved interrupt handlers */
  interrupts: {
    handler?: {
      [mode: string]: ParsedOperation;
    };
    vectors: {
      [key: string]: {
        vector: number;
      };
    };
  };
  /** The system state that is embedded in the machine linear memory */
  system: {
    mode?: MemoryReference;
  };
}
