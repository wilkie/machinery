import type { FileFormat } from '@machinery/core';
import { Stream } from '@machinery/core';

export interface FileObject {
  [key: string]: string;
}

/**
 * Parses a datastream to identify and read FileFormat-described data.
 */
class FileReader {
  readonly format: FileFormat;

  /**
   * Creates a FileReader than can identify and read the given format.
   */
  constructor(format: FileFormat) {
    this.format = format;
  }

  /**
   * Identifies whether the given data stream is a possible match to the file.
   */
  async matches(data: Uint8Array | Stream): Promise<boolean> {
    // Go through the format rules for identification
    for (const rule of this.format.identify || []) {
      const bytes =
        data instanceof Stream
          ? await data.readUnbuffered(rule.at, rule.match.length)
          : data;

      if (bytes.some((x, i) => x !== rule.match[i])) {
        continue;
      }

      return true;
    }

    return false;
  }

  /**
   * Parses the data stream and produces a realized file structure object.
   */
  async load(_data: Uint8Array): Promise<FileObject> {
    return {
      foo: 'bar',
    };
  }
}

export default FileReader;
