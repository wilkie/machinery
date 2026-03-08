/**
 * The general data type for the given data object.
 */
export const DataTypes = {
  Unknown: 'unknown',
  TextData: 'text',
  BinaryData: 'binary',
};

export type DataType = (typeof DataTypes)[keyof typeof DataTypes];
