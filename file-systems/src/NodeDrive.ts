import Drive, { DriveTypes, MediaTypes } from './Drive.js';
import NativeFile from './NativeFile';
import NativeDirectory from './NativeDirectory';
import NativeStream from './NativeStream';
import * as fs from 'node:fs';
import { promises } from 'node:fs';
import { File, Directory, Stream } from '@machinery/core';
import type { Stat, FindOptions, DestroyOptions } from '@machinery/core';

const DEFAULT_STAT_VALUE = 0;
const DEFAULT_STAT_MODE_DIRECTORY = 0x4000;
const DEFAULT_STAT_MODE_FILE = 0x8000;
const DEFAULT_DIRECTORY_SIZE = 1024;
const DEFAULT_BLOCK_SIZE = 1024;

export class NodeStream extends NativeStream {
  _handle: promises.FileHandle;

  constructor(
    file: File,
    size: number,
    slice: number = 0,
    contentType: string = '',
    handle: promises.FileHandle,
  ) {
    super(file, size, slice, contentType);
    this._handle = handle;
  }

  clone(file: File, size: number, slice: number = 0, contentType: string = '') {
    return new NodeStream(file, size, slice, contentType, this._handle);
  }

  async readUnbuffered(
    start: number | bigint,
    length: number | bigint,
  ): Promise<Uint8Array<ArrayBuffer>> {
    start = Number(start);
    length = Math.min(Number(length), this.size - start);

    // Allocate the buffer
    const buffer = new Uint8Array(length);

    const ret = await this._handle.read({
      buffer: buffer,
      offset: 0,
      length: length,
      position: start,
    });

    const result = new Uint8Array(ret.bytesRead);
    result.set(
      new Uint8Array(
        ret.buffer.buffer as ArrayBuffer,
        ret.buffer.byteOffset,
        ret.bytesRead,
      ),
    );
    return result;
  }

  async writeUnbuffered(
    start: number | bigint,
    bytes: Uint8Array,
  ): Promise<number> {
    start = Number(start);
    const ret = await this._handle.write(bytes, 0, bytes.byteLength, start);
    return ret.bytesWritten;
  }

  async close(): Promise<boolean> {
    await this.flush();
    await this._handle.close();
    return true;
  }
}

export class NodeFile extends NativeFile {
  _nodeHandle: string;

  constructor(name: string, handle: string) {
    super(name, undefined, false, false);
    this._nodeHandle = handle;
  }

  async exists(): Promise<boolean> {
    return (await promises.access(this._nodeHandle)) === undefined;
  }

  async open(): Promise<Stream | undefined> {
    // You cannot open a directory
    if (this.isDirectory) {
      return undefined;
    }

    // Get a file handle
    const handle: promises.FileHandle = await promises.open(
      this._nodeHandle,
      'r+',
    );

    // Get file size when opening
    const size = await this.size();
    return new NodeStream(this, size, 0, '', handle);
  }

  async size(): Promise<number> {
    return (await this.stat()).st_size;
  }

  async stat(): Promise<Stat> {
    const stat: fs.Stats = await promises.stat(this._nodeHandle);

    return {
      st_dev: stat.dev,
      st_ino: stat.ino,
      st_mode: stat.mode,
      st_nlink: stat.nlink,
      st_uid: stat.uid,
      st_gid: stat.gid,
      st_rdev: stat.rdev,
      st_size: stat.size,
      st_atime: stat.atimeMs,
      st_mtime: stat.mtimeMs,
      st_ctime: stat.ctimeMs,
      st_blksize: stat.blksize,
      st_blocks: stat.blocks,
    };
  }
}

export class NodeDirectory extends NativeDirectory {
  _nodeHandle: string;

  constructor(
    name: string,
    handle: string,
    parent: Directory | undefined = undefined,
  ) {
    super(name, undefined, parent, false, false);
    this._nodeHandle = handle;
  }

  async createFile(name: string): Promise<File | undefined> {
    try {
      const newHandle = await promises.open(
        this._nodeHandle + '/' + name,
        'w+',
      );
      await newHandle.close();
      const newFile = new NodeFile(name, this._nodeHandle + '/' + name);
      return newFile;
    } catch {
      // Failed in some way
    }
    return undefined;
  }

  async createDirectory(name: string): Promise<Directory | undefined> {
    try {
      const result = await promises.mkdir(this._nodeHandle + '/' + name);
      if (result === undefined) {
        const newDir = new NodeDirectory(
          name,
          this._nodeHandle + '/' + name,
          this,
        );
        return newDir;
      }
    } catch {
      // Failed in some way
    }
    return undefined;
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

    // Commit to destroying the file
    try {
      await promises.unlink(this._nodeHandle + '/' + file.name);
    } catch {
      // Silently fail
    }

    return true;
  }

  async list(): Promise<File[]> {
    const localDir = await promises.opendir(this._nodeHandle);
    const ret: File[] = [];
    for await (const dirent of localDir) {
      ret.push(new NodeFile(dirent.name, this._nodeHandle + '/' + dirent.name));
    }
    return ret;
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

export class NodeDrive extends Drive {
  constructor(name: string, root: Directory) {
    super(name, DriveTypes.LocalStorage, MediaTypes.Fixed, root);
  }

  static from(path: string, name?: string): NodeDrive {
    const root = new NodeDirectory('/', path);
    return new NodeDrive(name || 'drive', root);
  }
}

export default NodeDrive;
