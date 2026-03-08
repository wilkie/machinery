import TypedFile from './TypedFile';
import File from './File';
import { DataTypes } from './DataTypes';

/**
 * A generic, typed representation of some text data.
 */
export default abstract class TextFile extends TypedFile {
  constructor(
    target: string,
    subtype: string,
    file: File,
    origin: TypedFile[] = [],
  ) {
    super(DataTypes.TextData, subtype, target, file, origin);
  }

  static type(): string {
    return DataTypes.TextData;
  }
}
