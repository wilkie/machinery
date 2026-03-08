import NativeStream from './NativeStream.js';
import type { Stat } from '@machinery/core';
import { File, Stream } from '@machinery/core';

const DEFAULT_STAT_VALUE = 0;
const DEFAULT_STAT_MODE_DIRECTORY = 0x4000;
const DEFAULT_STAT_MODE_FILE = 0x8000;
const DEFAULT_DIRECTORY_SIZE = 1024;
const DEFAULT_BLOCK_SIZE = 1024;

export class NativeFile extends File {
  _name: string;
  _handle: FileSystemHandle | FileList | globalThis.File | File[] | undefined;
  _local: boolean;
  _readonly: boolean;

  /**
   * Constructs a local File from an existing lower-level API class.
   */
  constructor(
    name: string,
    handle:
      | FileSystemHandle
      | FileList
      | globalThis.File
      | File[]
      | undefined = undefined,
    local: boolean = true,
    readonly: boolean = false,
  ) {
    super(name, handle, local, readonly);
    this._name = name;
    this._handle = handle;
    this._local = local;
    this._readonly = readonly;
  }

  /**
   * The native handle for the file or directory.
   *
   * The type of this handle depends on the underlying API or system used to
   * retrieve it. You cannot rely on any one particular standard being used.
   *
   * This is best avoided in favor of the other abstracted methods.
   */
  get handle():
    | FileSystemHandle
    | FileList
    | globalThis.File
    | File[]
    | undefined {
    return this._handle;
  }

  /**
   * The name of this instance as was used to locate this file.
   */
  get name(): string {
    return this._name;
  }

  /**
   * Whether or not this instance contains readable byte data.
   */
  get isData(): boolean {
    return true;
  }

  get isLocal(): boolean {
    return this._local;
  }

  get isReadOnly(): boolean {
    return this._readonly;
  }

  /**
   * Whether or not this instance represents a directory (a set of files).
   */
  get isDirectory(): boolean {
    return false;
  }

  /**
   * Determines if the file is still persisted.
   *
   * It may be deleted by another application on deleted on the system by some
   * other means than our interface. In that case, this will return false.
   *
   * It will return true if the file/directory still exists where it was
   * originally located.
   */
  async exists(): Promise<boolean> {
    const nativeFile = this.isDirectory
      ? undefined
      : await this.getNativeFile();
    if (this.isData && nativeFile === undefined) {
      // The file has disappeared
      return false;
    }

    return !!nativeFile;
  }

  /**
   * Provides a stream for accessing the data for a file for reading or writing.
   */
  async open(): Promise<Stream | undefined> {
    // You cannot open a directory
    if (this.isDirectory) {
      return undefined;
    }

    // You cannot open a file that has been deleted
    if (!(await this.exists())) {
      return undefined;
    }

    // Get the current filesize (in case it has changed)
    const nativeFile = await this.getNativeFile();
    if (nativeFile === undefined) {
      // File disappeared
      return undefined;
    }
    const size = nativeFile.size;
    return new NativeStream(this, size);
  }

  async size(): Promise<number> {
    const nativeFile = this.isDirectory
      ? undefined
      : await this.getNativeFile();
    if (nativeFile === undefined) {
      // File disappeared
      return 0;
    }
    return this.isDirectory ? DEFAULT_DIRECTORY_SIZE : nativeFile.size;
  }

  /**
   * Returns a POSIX-style stat block describing the current file.
   */
  async stat(): Promise<Stat | undefined> {
    const nativeFile = this.isDirectory
      ? undefined
      : await this.getNativeFile();
    if (!this.isDirectory && nativeFile === undefined) {
      // The file has disappeared
      return undefined;
    }

    const fileSize = this.isDirectory
      ? DEFAULT_DIRECTORY_SIZE
      : nativeFile?.size || 0;
    return {
      st_dev: DEFAULT_STAT_VALUE,
      st_ino: DEFAULT_STAT_VALUE,
      st_mode: this.isDirectory
        ? DEFAULT_STAT_MODE_DIRECTORY
        : DEFAULT_STAT_MODE_FILE,
      st_nlink: DEFAULT_STAT_VALUE,
      st_uid: DEFAULT_STAT_VALUE,
      st_gid: DEFAULT_STAT_VALUE,
      st_rdev: DEFAULT_STAT_VALUE,
      st_size: fileSize,
      st_atime: DEFAULT_STAT_VALUE,
      st_mtime: DEFAULT_STAT_VALUE,
      st_ctime: DEFAULT_STAT_VALUE,
      st_blksize: DEFAULT_BLOCK_SIZE,
      st_blocks: fileSize / DEFAULT_BLOCK_SIZE,
    };
  }

  /**
   * Gets the native `File` instance.
   *
   * A `File`, in this case, does not refer to a `File` within our system, but
   * rather the low-level `File` class found underneath the other APIs. This
   * does its best to present that singular API instance regardless of the other
   * APIs potentially in use.
   *
   * A native `File` class is a type of `Blob` instance.
   */
  async getNativeFile(): Promise<globalThis.File | undefined> {
    if (
      typeof FileSystemFileHandle !== 'undefined' &&
      this.handle instanceof FileSystemFileHandle
    ) {
      try {
        return await this.handle.getFile();
      } catch (e) {
        const error: Error = e as Error;
        if (error.name === 'NotFoundError') {
          return undefined;
        }
        throw error;
      }
    } else if (
      typeof globalThis.File !== 'undefined' &&
      this.handle instanceof globalThis.File
    ) {
      return this.handle as globalThis.File;
    }

    return undefined;
  }

  /**
   * Truncates the existing file down to the given size.
   */
  async truncate(size: number): Promise<boolean> {
    if (this.isReadOnly) {
      // Cannot truncate a readonly file
      return false;
    }

    if (
      typeof FileSystemFileHandle !== 'undefined' &&
      this.handle instanceof FileSystemFileHandle
    ) {
      const stream = await this.handle.createWritable();
      await stream.write({
        type: 'truncate',
        size,
      });
      await stream.close();
    }
    return true;
  }

  /**
   * Reads the entire file as one big array.
   *
   * It is generally better to `open` the file as a `Stream` and accessing data
   * a chunk at a time instead of just allocating a large array.
   */
  async read(): Promise<Uint8Array> {
    const stream = await this.open();
    if (stream === undefined) {
      return new Uint8Array([]);
    }
    await stream.seek(0);
    return await stream.read(await this.size());
  }
}

export default NativeFile;
