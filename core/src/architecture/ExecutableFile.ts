import File from '../File.js';
import TypedFile from '../TypedFile.js';

import ExtendedDataType from './ExtendedDataType';

/**
 * A typed representation of an executable.
 */
export class ExecutableFile extends TypedFile {
  constructor(
    target: string,
    subtype: string,
    file: File,
    origin: TypedFile[] = [],
  ) {
    super(ExtendedDataType.ExecutableData, subtype, target, file, origin);
  }

  static type(): string {
    return ExtendedDataType.ExecutableData;
  }
}

export default ExecutableFile;
