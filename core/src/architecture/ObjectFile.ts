import File from '../File.js';
import TypedFile from '../TypedFile.js';

import ExtendedDataType from './ExtendedDataType';

/**
 * A typed representation of an object file.
 */
export class ObjectFile extends TypedFile {
  constructor(
    target: string,
    subtype: string,
    file: File,
    origin: TypedFile[] = [],
  ) {
    super(ExtendedDataType.ObjectData, subtype, target, file, origin);
  }

  static type(): string {
    return ExtendedDataType.ObjectData;
  }
}

export default ObjectFile;
