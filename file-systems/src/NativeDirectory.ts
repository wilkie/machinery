import NativeFile from './NativeFile.js';
import { File, Directory, Stream, type Stat } from '@machinery/core';

const DEFAULT_STAT_VALUE = 0;
const DEFAULT_STAT_MODE_DIRECTORY = 0x4000;
const DEFAULT_DIRECTORY_SIZE = 1024;
const DEFAULT_BLOCK_SIZE = 1024;

export type FindOptions = {
  caseInsensitive?: boolean;
};

export type LocateOptions = FindOptions & {
  parent?: boolean;
};

export type DestroyOptions = FindOptions & {
  recursive?: boolean;
};

/**
 * A list of prohibited extensions when using direct filesystem access.
 *
 * We will 'mask' them if we try to write them and unmask them when we
 * read them back. It's not ideal, but it is what we have to do at the
 * moment.
 */
export const PROHIBITED_EXTENSIONS = ['.cfg', '.dll', '.grp', '.ini'];

export class NativeDirectory extends Directory {
  _parent: Directory | undefined;
  _files: File[] | undefined;

  /**
   * Creates a directory from a given lower-level handle or a set of native File instances.
   */
  constructor(
    name: string,
    handle:
      | FileSystemDirectoryHandle
      | FileList
      | File[]
      | undefined = undefined,
    parent: Directory | undefined,
    local: boolean = true,
    readonly: boolean = false,
  ) {
    super(name, handle, parent, local, readonly);
    this._parent = parent;
    this._files = undefined;
  }

  async exists(): Promise<boolean> {
    return false;
  }

  async size(): Promise<number> {
    return 0;
  }

  async stat(): Promise<Stat | undefined> {
    const fileSize = DEFAULT_DIRECTORY_SIZE;

    return {
      st_dev: DEFAULT_STAT_VALUE,
      st_ino: DEFAULT_STAT_VALUE,
      st_mode: DEFAULT_STAT_MODE_DIRECTORY,
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

  get parent(): Directory | undefined {
    return this._parent;
  }

  /**
   * The name of this instance as was used to locate this file.
   */
  get name(): string {
    return this._name;
  }

  get isData(): boolean {
    return false;
  }

  get isLocal(): boolean {
    return this._local;
  }

  get isReadOnly(): boolean {
    return this._readonly;
  }

  get isDirectory(): boolean {
    return true;
  }

  async createFile(name: string): Promise<File | undefined> {
    if (this.isReadOnly) {
      // Cannot create a file if it is readonly
      return undefined;
    }

    if (
      typeof FileSystemDirectoryHandle !== 'undefined' &&
      this.handle instanceof FileSystemDirectoryHandle
    ) {
      const dir = this.handle as FileSystemDirectoryHandle;
      // Mask filenames to get around extension prohibition
      if (this.isLocal) {
        for (const ext of PROHIBITED_EXTENSIONS) {
          if (name.toLowerCase().endsWith(ext)) {
            name = name + '_masked';
            break;
          }
        }
      }
      const file = new NativeFile(
        name,
        await dir.getFileHandle(name, { create: true }),
        this.isLocal,
      );
      const files = await this.list();
      files.push(file);
      return file;
    }

    return undefined;
  }

  async createDirectory(name: string): Promise<Directory | undefined> {
    if (this.isReadOnly) {
      // Cannot create a file if it is readonly
      return undefined;
    }

    if (
      typeof FileSystemDirectoryHandle !== 'undefined' &&
      this.handle instanceof FileSystemDirectoryHandle
    ) {
      const dir = this.handle as FileSystemDirectoryHandle;
      const subdir = await dir.getDirectoryHandle(name, { create: true });
      const files = await this.list();
      const ret = new NativeDirectory(name, subdir, this, this.isLocal);
      files.push(ret);
      return ret;
    }

    return undefined;
  }

  async locate(
    path: string,
    options: LocateOptions = {},
  ): Promise<File | undefined> {
    if (path.startsWith('/')) {
      path = path.slice(1);
    }

    // Return ourself for an empty path or a "." path
    if (path === '' || path === '.') {
      return this;
    }

    const parts: string[] = path.split('/');
    if (options.parent) {
      // We are the parent of a single part
      if (parts.length === 1) {
        return this;
      }

      // Just remove the last part of the path and find that
      parts.splice(parts.length - 1, parts.length - 1);

      options = { ...options };
      options.parent = false;
    }

    const found = await this.find(parts[0], options as FindOptions);

    // We either found what we want or not
    if (parts.length === 1 || found === undefined) {
      return found;
    }

    // Look at what we found... we expect this to be a Directory
    if (found instanceof Directory) {
      return await found.locate(parts.slice(1).join('/'), options);
    }

    // We hit a file instead of a directory
    return undefined;
  }

  async find(
    name: string,
    options: FindOptions = {},
  ): Promise<File | undefined> {
    // Look at our files
    return (await this.list()).find((file) =>
      options.caseInsensitive
        ? file.name.toLowerCase() === name.toLowerCase()
        : file.name === name,
    );
  }

  /**
   * Returns all file instances within this directory.
   *
   * This includes those that represent a directory as well.
   *
   * To filter out directories, do something like:
   *
   * (await myDirectory.list()).filter(x => !(x instanceof Directory))
   */
  async list(): Promise<File[]> {
    this._files ||= await (async () => {
      if (
        typeof FileSystemDirectoryHandle !== 'undefined' &&
        this.handle instanceof FileSystemDirectoryHandle
      ) {
        const ret = [];
        for await (const value of this.handle.values()) {
          // Unmask filenames that are used to get around extension prohibition
          const MASKED_MARKER = '_masked';
          const masked = value.name.toLowerCase().endsWith(MASKED_MARKER);
          const name =
            this.isLocal && masked
              ? value.name.substring(
                  0,
                  value.name.length - MASKED_MARKER.length,
                )
              : value.name;
          ret.push(
            value.kind === 'directory'
              ? new NativeDirectory(
                  name,
                  value as FileSystemDirectoryHandle,
                  this,
                  this.isLocal,
                )
              : new NativeFile(name, value, this.isLocal),
          );
        }
        return ret;
      } else if (
        typeof FileList !== 'undefined' &&
        this.handle instanceof FileList
      ) {
        return Array.from(this.handle).map(
          (subhandle: globalThis.File) =>
            new NativeFile(subhandle.name, subhandle, this.isLocal),
        );
      } else if (Array.isArray(this.handle)) {
        return this.handle;
      }
    })();

    return this._files || [];
  }

  /**
   * Removes the file with the given name from the directory or the directory,
   * if it is empty.
   */
  async destroy(name: string, options: DestroyOptions = {}): Promise<boolean> {
    // Do not allow destruction of a readonly directory
    if (this.isReadOnly) {
      return false;
    }

    // Find the file
    const file = await this.find(name, options as FindOptions);

    if (!file) {
      // Cannot find the file
      return false;
    }

    // Do not allow destroying a readonly file
    if (file.isReadOnly) {
      return false;
    }

    // Navigate the APIs to remove the file
    if (
      typeof FileSystemDirectoryHandle !== 'undefined' &&
      this.handle instanceof FileSystemDirectoryHandle
    ) {
      const dir = this.handle as FileSystemDirectoryHandle;
      try {
        await dir.removeEntry(file.name);
      } catch (e) {
        const error: Error = e as Error;
        if (error.name === 'NotFoundError') {
          // File was not found
          return false;
        }
        throw error;
      } finally {
        // Remove the file from the listing
        const files = await this.list();
        const index = files.findIndex((file) =>
          options.caseInsensitive
            ? file.name.toLowerCase() === name.toLowerCase()
            : file.name === name,
        );
        if (index === 0) {
          files.shift();
        } else {
          files.splice(index, index);
        }
      }
      return true;
    }

    return false;
  }

  /**
   * Returns only file instances that represent directories.
   */
  async getDirectories(): Promise<Directory[]> {
    const files = await this.list();
    return files.filter((x) => x instanceof Directory);
  }

  /**
   * Constructs a Directory from a given set of File objects provided by a
   * FileList object.
   *
   * This returns a Directory made from the first common directory in the file
   * listing. That is, this only works if all the files have the same common
   * directory as a prefix to their webkitRelativePath field.
   *
   * If there is no such field, it just puts all of the given files as though
   * they were all in the same directory.
   */
  static fromFileList(files: FileList): Directory {
    type TemporaryFileList = { [key: string]: TemporaryFileList | File };

    const root: TemporaryFileList = {};
    const sortedFiles = Array.from(files).sort((a, b) =>
      a.webkitRelativePath.localeCompare(b.webkitRelativePath),
    );

    for (const file of sortedFiles) {
      // Parse the webkit relative path
      const parts = (file.webkitRelativePath || file.name).split('/');

      let current: TemporaryFileList = root;
      for (const part of parts.slice(0, parts.length - 1)) {
        current[part] ||= {};
        current = current[part] as TemporaryFileList;
      }
      current[parts.pop() as string] = new NativeFile(file.name, file, false);
    }

    // Turn collections of files into Directory objects
    const formDirectory: (
      name: string,
      filelist: TemporaryFileList,
    ) => Directory = (name, filelist) => {
      const subfiles = [];
      for (const [name, value] of Object.entries(filelist)) {
        if (!(value instanceof File)) {
          subfiles.push(formDirectory(name, value));
        } else {
          subfiles.push(value);
        }
      }

      return new NativeDirectory(name, subfiles, undefined, false);
    };

    // The first and only entry in root is the name of the directory
    // This function only supports a FileList that contains files within a
    // single directory, therefore.
    const name: string = Object.keys(root)[0];
    return formDirectory(name, root[name] as TemporaryFileList);
  }

  static async fromPicker(): Promise<Directory | undefined> {
    // @ts-expect-error: showDirectoryPicker is not available in all environments
    const handle = await window.showDirectoryPicker();
    if (!handle) {
      return undefined;
    }
    return new NativeDirectory(handle.name, handle, undefined, true);
  }

  async open(): Promise<Stream | undefined> {
    return undefined;
  }

  async truncate(_size: number): Promise<boolean> {
    return false;
  }

  async read(): Promise<Uint8Array> {
    return new Uint8Array([]);
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
}

export default NativeDirectory;
