/**
 * Describes a rule that is used to identify a file format.
 */
export interface IdentifierRule {
  /** The byte position to look for */
  at: number;
  /** The byte sequence to look for */
  match: Uint8Array | number[];
}

export type Endianness = 'little' | 'big';

export type DataTypes =
  | 'u8'
  | 'u16'
  | 'u32'
  | 'u64'
  | 'i8'
  | 'i16'
  | 'i32'
  | 'i64';

/**
 * Describes one field within some data block.
 */
export interface DataFieldDefinition {
  /** A name to refer to this field. */
  name: string;
  /** The data type for this piece of data */
  type: DataTypes;
  /** If this is an array of data, the number of entries */
  count?: number;
  /**
   * Override the endianness just for this field. Otherwise it is the block
   * endianness or the file endianness depending on which is defined.
   */
  endianness?: Endianness;
}

/**
 * Describes the structure of a block of data.
 */
export interface DataBlockDefinition {
  /** An ordered set of fields for this data block */
  fields: DataFieldDefinition[];
  /** What endian style is used in this data block. Defaults to the file endianness. */
  endianness?: Endianness;
}

/**
 * A property that is defined by parsing a data block of a particular type.
 */
export interface DataBlockSubBlockProperty {
  /** The location to read the data block as either a specific offset or an expression */
  at: number | string;
  /** The data block definition from the 'types' section to use to define this block */
  type: string;
  /** If this is an array, this computes the length of that array. */
  countExpression?: string;
}

/**
 * Describes a field that is defined dynamically using parsed data blocks.
 *
 * If it is just a string, this is a property that is defined by some expression
 * code that refers to parsed data block fields.
 */
export interface DataBlockExpressionProperty {
  expression?: string;
}

export type DataBlockProperty = DataBlockSubBlockProperty | string;

/**
 * Holds all of the dynamically defined properties for file format data.
 */
export interface DataBlockProperties {
  [key: string]: DataBlockProperty;
}

/**
 * Describes realized data that can be parsed from the file.
 */
export interface DataBlock {
  /** The data block base type which defines the data in the file structure itself. */
  type: string;
  /** Where the data starts */
  offset: number;
  /** Other properties that are defined via the data entries. */
  properties?: DataBlockProperties;
}

/**
 * Holds all of the defined data block types that can be used to define the
 * base types for data blocks.
 */
export interface DataBlockDefinitions {
  [key: string]: DataBlockDefinition;
}

/**
 * Holds all of the data blocks that are populated when reading the file.
 */
export interface DataBlocks {
  [key: string]: DataBlock;
}

/**
 * Describes an executable file format.
 */
export interface FileFormat {
  /** A name for the file format. */
  name: string;
  /**
   * A known extension (or extensions) for this format.
   *
   * If more than one are specified, they are specified in order of preference.
   */
  extension?: string | string[];
  /** Rules used to identify this file from a datastream. */
  identify?: IdentifierRule[];
  /** What endian style is used in this file, by default. */
  endianness: Endianness;
  /** Definitions of data blocks that can be found in this data. */
  types?: DataBlockDefinitions;
  /** Rules to parse data from the file. */
  data?: DataBlocks;
}
