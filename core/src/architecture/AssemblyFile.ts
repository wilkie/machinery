import File from '../File.js';
import TypedFile from '../TypedFile.js';

import ExtendedDataType from './ExtendedDataType';

/**
 * A typed representation of an assembly file.
 */
export class AssemblyFile extends TypedFile {
  constructor(
    target: string,
    subtype: string,
    file: File,
    origin: TypedFile[] = [],
  ) {
    super(ExtendedDataType.AssemblyData, subtype, target, file, origin);
  }

  static type(): string {
    return ExtendedDataType.AssemblyData;
  }
}

export default AssemblyFile;
