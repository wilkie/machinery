import type File from './File';

export default abstract class Stream {
  _file: File;
  _pos: number;
  _slice: number;
  _size: number;
  _type: string;

  constructor(
    file: File,
    size: number,
    slice: number = 0,
    contentType: string = '',
  ) {
    this._file = file;
    this._pos = 0;
    this._slice = slice;
    this._size = size;
    this._type = contentType;
  }

  abstract get file(): File;

  abstract get contentType(): string;

  abstract get position(): number;

  abstract get size(): number;

  abstract clone(
    file: File,
    size: number,
    slice: number,
    contentType: string,
  ): Stream;

  abstract slice(
    start: number,
    end: number | undefined,
    contentType: string,
  ): Stream;

  abstract get blob(): Blob;

  abstract flush(): Promise<boolean>;

  abstract seek(position: number | bigint): Promise<number>;

  abstract skip(length: number | bigint): Promise<number>;

  abstract rewind(length: number | bigint): Promise<number>;

  abstract readLine(encoding: string): Promise<string | undefined>;

  abstract read(length: number | bigint): Promise<Uint8Array<ArrayBuffer>>;

  abstract readUnbuffered(
    start: number | bigint,
    length: number | bigint,
  ): Promise<Uint8Array<ArrayBuffer>>;

  abstract write(bytes: Uint8Array<ArrayBuffer>): Promise<number>;

  abstract writeUnbuffered(
    start: number | bigint,
    bytes: Uint8Array<ArrayBuffer>,
  ): Promise<number>;

  abstract close(): Promise<boolean>;
}
