import type { Stat } from './Stat';
import File from './File.js';
import Stream from './Stream.js';
import type { DataType } from './DataTypes';
import { DataTypes } from './DataTypes';

/**
 * A file abstraction that retains the provenance of the file data and tags to
 * indicate what system produced it and what its intended purpose is.
 */
export default abstract class TypedFile extends File {
  _file: File;
  _type: string;
  _subtype: string;
  _target: string;
  _origin: TypedFile[];

  constructor(
    type: DataType | string,
    subtype: string,
    target: string,
    file: File,
    origin: TypedFile[] = [],
  ) {
    super(file?.name, undefined, false, false);

    // Pull out inner file
    if (file instanceof TypedFile) {
      file = file.file;
    }

    this._file = file;
    this._type = type as string;
    this._subtype = subtype;
    this._target = target;
    this._origin = origin;
  }

  static type(): string {
    return DataTypes.Unknown;
  }

  get isLocal(): boolean {
    return this._file.isLocal;
  }

  get isData(): boolean {
    return this._file.isData;
  }

  get isDirectory(): boolean {
    return this._file.isDirectory;
  }

  get isReadOnly(): boolean {
    return this._file.isReadOnly;
  }

  get handle():
    | FileSystemHandle
    | FileList
    | globalThis.File
    | File[]
    | undefined {
    return this._file.handle;
  }

  get name(): string {
    return this._file.name;
  }

  get type(): string {
    return this._type;
  }

  get subtype(): string {
    return this._subtype;
  }

  get isOrigin(): boolean {
    return this._origin.length === 0;
  }

  get origin(): TypedFile[] {
    return [...this._origin];
  }

  get file(): File {
    return this._file;
  }

  get target(): string {
    return this._target;
  }

  async exists(): Promise<boolean> {
    return await this._file.exists();
  }

  async open(): Promise<Stream | undefined> {
    return await this._file.open();
  }

  async size(): Promise<number> {
    return await this._file.size();
  }

  async stat(): Promise<Stat | undefined> {
    return await this._file.stat();
  }

  async getNativeFile(): Promise<globalThis.File | undefined> {
    return await this._file.getNativeFile();
  }

  async truncate(size: number): Promise<boolean> {
    return await this._file.truncate(size);
  }

  async read(): Promise<Uint8Array> {
    return await this._file.read();
  }
}
