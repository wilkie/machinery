import type { Stat } from './Stat';
import type Stream from './Stream.js';

export default abstract class File {
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
  abstract get handle():
    | FileSystemHandle
    | FileList
    | globalThis.File
    | File[]
    | undefined;

  /**
   * The name of this instance as was used to locate this file.
   */
  abstract get name(): string;

  /**
   * Whether or not this instance contains readable byte data.
   */
  abstract get isData(): boolean;

  abstract get isLocal(): boolean;

  abstract get isReadOnly(): boolean;

  /**
   * Whether or not this instance represents a directory (a set of files).
   */
  abstract get isDirectory(): boolean;

  /**
   * Determines if the file is still persisted.
   *
   * It may be deleted by another application on deleted on the system by some
   * other means than our interface. In that case, this will return false.
   *
   * It will return true if the file/directory still exists where it was
   * originally located.
   */
  abstract exists(): Promise<boolean>;

  /**
   * Provides a stream for accessing the data for a file for reading or writing.
   */
  abstract open(): Promise<Stream | undefined>;

  abstract size(): Promise<number>;

  /**
   * Returns a POSIX-style stat block describing the current file.
   */
  abstract stat(): Promise<Stat | undefined>;

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
  abstract getNativeFile(): Promise<globalThis.File | undefined>;

  /**
   * Truncates the existing file down to the given size.
   */
  abstract truncate(size: number): Promise<boolean>;

  /**
   * Reads the entire file as one big array.
   *
   * It is generally better to `open` the file as a `Stream` and accessing data
   * a chunk at a time instead of just allocating a large array.
   */
  abstract read(): Promise<Uint8Array>;
}
