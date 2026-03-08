import TypedFile from './TypedFile';
import File from './File';
import { DataTypes } from './DataTypes';

/**
 * A generic, typed representation of some binary data.
 */
export default abstract class BinaryFile extends TypedFile {
  constructor(
    target: string,
    subtype: string,
    file: File,
    origin: TypedFile[] = [],
  ) {
    super(DataTypes.BinaryData, subtype, target, file, origin);
  }

  static type(): string {
    return DataTypes.BinaryData;
  }
}
