/**
 * A set of types for compilation-related data.
 */
export const ExtendedDataTypes = {
  AssemblyData: 'assembly',
  SourceData: 'source',
  ObjectData: 'object',
  ExecutableData: 'executable',
  LibraryData: 'library',
} as const;

export type ExtendedDataType =
  (typeof ExtendedDataTypes)[keyof typeof ExtendedDataTypes];

export default ExtendedDataTypes;
