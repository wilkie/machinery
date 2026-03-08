import NativeStream from './NativeStream.js';
import NativeFile from './NativeFile.js';
import NativeDirectory from './NativeDirectory.js';
import Drive, { DriveType, MediaType } from './Drive.js';
import { File, Directory, Stream } from '@machinery/core';
import type { Stat, FindOptions, DestroyOptions } from '@machinery/core';

const DEFAULT_STAT_VALUE = 0;
const DEFAULT_STAT_MODE_DIRECTORY = 0x4000;
const DEFAULT_STAT_MODE_FILE = 0x8000;
const DEFAULT_DIRECTORY_SIZE = 1024;
const DEFAULT_BLOCK_SIZE = 1024;

export class MemoryStream extends NativeStream {
  async readUnbuffered(
    start: number,
    length: number,
  ): Promise<Uint8Array<ArrayBuffer>> {
    if (!(this.file instanceof MemoryFile)) {
      return await super.readUnbuffered(start, length);
    }

    const file: MemoryFile = this.file;
    length ||= file._data.byteLength - start;
    return file._data.slice(start, start + length);
  }

  async writeUnbuffered(
    start: number,
    bytes: Uint8Array<ArrayBuffer>,
  ): Promise<number> {
    if (!(this.file instanceof MemoryFile)) {
      return await super.writeUnbuffered(start, bytes);
    }

    const file: MemoryFile = this.file;
    if (start + bytes.byteLength > file._data.byteLength) {
      // Appending
      // TODO: make this a bit more efficient!
      const newData = new Uint8Array(start + bytes.byteLength);
      newData.set(file._data.slice(0, start), 0);
      file._data = newData;
    }
    file._data.set(bytes, start);
    return bytes.byteLength;
  }
}

export class MemoryDirectory extends NativeDirectory {
  _files: File[] = [];

  constructor(name: string, parent: Directory | undefined = undefined) {
    super(name, undefined, parent, false, false);
    this._files = [];
  }

  async createFile(
    name: string,
    data?: Uint8Array<ArrayBuffer>,
  ): Promise<File | undefined> {
    const newFile = new MemoryFile(name, data);
    this._files.push(newFile);
    return newFile;
  }

  async createDirectory(name: string): Promise<Directory | undefined> {
    const newDir = new MemoryDirectory(name, this);
    this._files.push(newDir);
    return newDir;
  }

  async destroy(name: string, options: DestroyOptions = {}): Promise<boolean> {
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

    const files = this._files;
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

    return true;
  }

  async list(): Promise<File[]> {
    return this._files;
  }

  async exists(): Promise<boolean> {
    return true;
  }

  async stat(): Promise<Stat | undefined> {
    const fileSize = await this.size();
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

  async size(): Promise<number> {
    // Some generic size for the directory
    return DEFAULT_DIRECTORY_SIZE;
  }
}

export class MemoryFile extends NativeFile {
  _data: Uint8Array<ArrayBuffer>;

  constructor(name: string, data?: Uint8Array<ArrayBuffer>) {
    super(name, undefined, false, false);

    this._data = data || new Uint8Array([]);
  }

  async exists(): Promise<boolean> {
    return true;
  }

  async open(): Promise<Stream | undefined> {
    // You cannot open a directory
    if (this.isDirectory) {
      return undefined;
    }

    // You cannot open a file that has been deleted
    if (!(await this.exists())) {
      return undefined;
    }

    return new MemoryStream(this, this._data.byteLength);
  }

  async size(): Promise<number> {
    return this._data.byteLength;
  }

  async stat(): Promise<Stat | undefined> {
    const fileSize = this.isDirectory
      ? DEFAULT_DIRECTORY_SIZE
      : this._data.byteLength;
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

  async truncate(size: number): Promise<boolean> {
    const newData = new Uint8Array(size);
    newData.set(this._data.slice(0, size), 0);
    this._data = newData;
    return true;
  }
}

export class MemoryDrive extends Drive {
  constructor(name: string) {
    const root = new MemoryDirectory(name);
    super(name, DriveType.TemporaryStorage, MediaType.Fixed, root);
  }
}

export default MemoryDrive;
