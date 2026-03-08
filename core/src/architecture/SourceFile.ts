import File from '../File.js';
import TypedFile from '../TypedFile.js';

import ExtendedDataType from './ExtendedDataType';

/**
 * A typed representation of a source code file.
 */
export class SourceFile extends TypedFile {
  constructor(
    target: string,
    subtype: string,
    file: File,
    origin: TypedFile[] = [],
  ) {
    super(ExtendedDataType.SourceData, subtype, target, file, origin);
  }

  static type(): string {
    return ExtendedDataType.SourceData;
  }
}

export default SourceFile;
