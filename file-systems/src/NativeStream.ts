import { File, Stream } from '@machinery/core';

let fileReaderReadAsArrayBuffer: ((blob: Blob) => void) | undefined = undefined;
const hookFileReader: () => void = () => {
  if (!fileReaderReadAsArrayBuffer) {
    if (typeof FileReader === 'undefined') {
      return;
    }

    fileReaderReadAsArrayBuffer = FileReader.prototype.readAsArrayBuffer;
    FileReader.prototype.readAsArrayBuffer = function (blob) {
      const self: FileReader = this as FileReader;
      if (self !== null && blob instanceof WrappedBlob) {
        (async () => {
          Object.defineProperty(self, 'result', {
            value: await blob.bytes(),
            writable: false,
          });
          if (self.onload) {
            self.onload(
              new ProgressEvent('load', {
                // TypeScript does not currently understand creating a load event with a target
                // @ts-expect-error: Object literal may only specify known properties, and 'target' does not exist in type 'ProgressEventInit'.
                target: self,
              }) as ProgressEvent<FileReader>,
            );
          }
        })();
      } else if (fileReaderReadAsArrayBuffer) {
        fileReaderReadAsArrayBuffer(blob);
      }
    };
  }
};

export class WrappedBlob extends Blob {
  _stream: Stream;

  constructor(stream: Stream) {
    super();
    this._stream = stream;
  }

  get type(): string {
    return this._stream.contentType;
  }

  get size(): number {
    return this._stream.size;
  }

  async arrayBuffer(): Promise<ArrayBuffer> {
    const bytes = await this.bytes();
    return bytes.buffer as ArrayBuffer;
  }

  async text(): Promise<string> {
    const decoder = new TextDecoder();
    return decoder.decode(await this.bytes());
  }

  async bytes(): Promise<Uint8Array<ArrayBuffer>> {
    await this._stream.seek(0);
    const ret = await this._stream.read(this._stream.size);
    return ret;
  }

  slice(
    start: number = 0,
    end: number | undefined = undefined,
    contentType: string = '',
  ) {
    return this._stream.slice(start, end || this.size, contentType).blob;
  }

  stream(): ReadableStream {
    // TODO: implement
    return new ReadableStream();
  }
}

export class NativeStream {
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

    // Ensure blob libraries are available to consume us
    hookFileReader();
  }

  get file(): File {
    return this._file;
  }

  get contentType(): string {
    return this._type;
  }

  get position(): number {
    return this._pos;
  }

  get size(): number {
    return this._size;
  }

  clone(file: File, size: number, slice: number = 0, contentType: string = '') {
    const StreamConstructor: new (
      file: File,
      size: number,
      slice: number,
      contentType: string,
    ) => Stream = this.constructor as new (
      file: File,
      size: number,
      slice: number,
      contentType: string,
    ) => Stream;
    return new StreamConstructor(file, size, slice, contentType);
  }

  slice(
    start: number = 0,
    end: number | undefined = undefined,
    contentType: string = '',
  ): Stream {
    end ||= this.size;
    return this.clone(
      this._file,
      end - start,
      this._slice + start,
      contentType,
    );
  }

  get blob(): Blob {
    return new WrappedBlob(this) as Blob;
  }

  async flush(): Promise<boolean> {
    // Write out pending buffers
    return true;
  }

  async seek(position: number | bigint): Promise<number> {
    await this.flush();
    this._pos = Math.max(0, Number(position));
    return this._pos;
  }

  async skip(length: number | bigint): Promise<number> {
    return await this.seek(this.position + Number(length));
  }

  async rewind(length: number | bigint): Promise<number> {
    return await this.seek(this.position - Number(length));
  }

  async readLine(encoding: string = 'utf-8'): Promise<string | undefined> {
    const decoder = new TextDecoder(encoding, {
      fatal: true,
    });
    const MINIMUM_READ = 1000;
    const LINEFEED = 0xa;
    const CARRIAGE_RETURN = 0xd;
    const UNKNOWN_INDEX = -1;

    let data = await this.readUnbuffered(this._pos, MINIMUM_READ);
    const lineEnd = data.findIndex(
      (e) => e === LINEFEED || e === CARRIAGE_RETURN,
    );

    if (lineEnd === UNKNOWN_INDEX) {
      return undefined;
    }

    // Allow both CRLF and just LF / CR by itself
    const CRLF_LENGTH = 2;
    const returnLength =
      data[lineEnd] === CARRIAGE_RETURN && data[lineEnd + 1] === LINEFEED
        ? CRLF_LENGTH
        : 1;
    data = data.slice(0, lineEnd);

    let ret = undefined;
    try {
      ret = decoder.decode(data);
      this._pos += data.byteLength + returnLength;
    } catch {
      return undefined;
    }

    return ret;
  }

  async read(length: number | bigint): Promise<Uint8Array<ArrayBuffer>> {
    // TODO: buffer chunks
    const ret = await this.readUnbuffered(this._pos + this._slice, length);
    this._pos += ret.byteLength;
    return ret;
  }

  async readUnbuffered(
    start: number | bigint,
    length: number | bigint,
  ): Promise<Uint8Array<ArrayBuffer>> {
    start = Number(start);
    length = Math.min(Number(length), this.size - start);
    const nativeFile: globalThis.File | undefined =
      await this.file.getNativeFile();
    if (nativeFile === undefined) {
      return new Uint8Array([]);
    }
    const buffer = await nativeFile.slice(start, start + length).arrayBuffer();
    return new Uint8Array(buffer);
  }

  async write(bytes: Uint8Array<ArrayBuffer>): Promise<number> {
    // TODO: buffer chunks
    const ret = await this.writeUnbuffered(this._pos + this._slice, bytes);
    this._pos += ret;
    return ret;
  }

  async writeUnbuffered(
    start: number | bigint,
    bytes: Uint8Array<ArrayBuffer>,
  ): Promise<number> {
    // TODO: do not write past size if not appending
    start = Number(start);
    if (
      typeof FileSystemFileHandle !== 'undefined' &&
      this.file.handle instanceof FileSystemFileHandle
    ) {
      const stream = await this.file.handle.createWritable({
        keepExistingData: true,
      });
      await stream.seek(start);
      await stream.write({
        type: 'write',
        data: bytes,
      });
      await stream.close();
    }
    return bytes.byteLength;
  }

  async close(): Promise<boolean> {
    await this.flush();
    return true;
  }
}

export default NativeStream;
