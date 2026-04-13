/**
 * Common properties of a field within a memory region.
 */
export interface MemoryRegionBaseField {
  /** The identifier for the field. */
  identifier: string;
  /** The optional full name of the field. */
  name?: string;
  /** An optional description of the field. */
  description?: string;
  /** The bit position to start the field. */
  offset: number;
  /** The number of bits the field is wide. */
  size: number;
  /** A disambiguating field value. */
  equals?: number;
}

export interface MemoryRegionSubdividedField extends MemoryRegionBaseField {
  fields?: MemoryRegionBaseField[];
}

export type MemoryRegionArrayFieldCell = Omit<
  MemoryRegionSubdividedField,
  'offset' | 'size'
>;

export interface MemoryRegionArrayField extends MemoryRegionBaseField {
  count: number;
  cell: MemoryRegionArrayFieldCell | MemoryRegionArrayFieldCell[];
}

export type MemoryRegionField =
  | MemoryRegionSubdividedField
  | MemoryRegionArrayField;

export interface BaseMemoryRegion {
  identifier: string;
  name: string;
  size?: number;
  /** Whether or not this region (without fields) is interpreted as signed (2's complement). */
  signed?: boolean;
}

export interface BaseOffsettableMemoryRegion extends BaseMemoryRegion {
  offset?: number | string;
}

export interface BaseSubdividedMemoryRegion extends BaseOffsettableMemoryRegion {
  fields: BaseOffsettableMemoryRegion[];
}

export interface ReadOnlyMemoryRegion extends BaseOffsettableMemoryRegion {
  data: number[];
  fields?: MemoryRegionField[];
}

export interface ReadWriteMemoryRegion extends BaseOffsettableMemoryRegion {
  data?: number[];
  fields?: MemoryRegionField[];
}

export type MemoryRegion = ReadOnlyMemoryRegion | ReadWriteMemoryRegion;

export type MemoryType = 'rom' | 'ram' | 'programmable';

export interface BaseMemoryInfo {
  identifier: string;
  name: string;
  endian: 'little' | 'big';
  min?: number;
  max?: number;
  length?: number;
  size?: number;
  signed?: boolean;
}

export type ReadOnlyMemoryInfo = BaseMemoryInfo & {
  type: 'rom';
  regions?: ReadOnlyMemoryRegion[];
};

export type ReadWriteMemoryInfo = BaseMemoryInfo & {
  type: 'ram';
  regions?: ReadWriteMemoryRegion[];
};

export type ProgrammableMemoryInfo = BaseMemoryInfo & {
  type: 'programmable';
  /**
   * The default value read when there's no region mapped into a byte.
   */
  default?: number;
  regions?: ReadWriteMemoryRegion[];
};

export type MemoryInfo =
  | ReadOnlyMemoryInfo
  | ReadWriteMemoryInfo
  | ProgrammableMemoryInfo;
