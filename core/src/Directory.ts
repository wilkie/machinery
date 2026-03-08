import File from './File.js';

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

export default abstract class Directory extends File {
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
    super(name, handle, local, readonly);
    this._parent = parent;
    this._files = undefined;
  }

  abstract get parent(): Directory | undefined;

  abstract get isData(): boolean;

  abstract get isDirectory(): boolean;

  abstract createFile(name: string): Promise<File | undefined>;

  abstract createDirectory(name: string): Promise<Directory | undefined>;

  abstract locate(
    path: string,
    options: LocateOptions,
  ): Promise<File | undefined>;

  abstract find(name: string, options: FindOptions): Promise<File | undefined>;

  /**
   * Returns all file instances within this directory.
   *
   * This includes those that represent a directory as well.
   *
   * To filter out directories, do something like:
   *
   * (await myDirectory.list()).filter(x => !(x instanceof Directory))
   */
  abstract list(): Promise<File[]>;

  /**
   * Removes the file with the given name from the directory or the directory,
   * if it is empty.
   */
  abstract destroy(name: string, options: DestroyOptions): Promise<boolean>;

  /**
   * Returns only file instances that represent directories.
   */
  abstract getDirectories(): Promise<Directory[]>;
}
