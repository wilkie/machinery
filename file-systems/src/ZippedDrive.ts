import NativeStream from './NativeStream.js';
import NativeDirectory from './NativeDirectory.js';
import NativeFile from './NativeFile.js';
import Drive, { DriveTypes, MediaTypes } from './Drive.js';
import { File, Directory, Stream, type Stat } from '@machinery/core';

import { BlobReader, ZipReader } from '@zip.js/zip.js';
import type { Entry } from '@zip.js/zip.js';

const DEFAULT_STAT_VALUE = 0;
const DEFAULT_STAT_MODE_DIRECTORY = 0x4000;
const DEFAULT_STAT_MODE_FILE = 0x8000;
const DEFAULT_DIRECTORY_SIZE = 1024;
const DEFAULT_BLOCK_SIZE = 1024;

export class ZippedStream extends NativeStream {
  _stream: TransformStream = new TransformStream();
  _response: Response | undefined;
  _writerPromise: Promise<Blob> | undefined;
  _readerPromise: Promise<ArrayBuffer> | undefined;

  async readUnbuffered(
    start: number,
    length: number,
  ): Promise<Uint8Array<ArrayBuffer>> {
    // Start reading, if not already
    if (!(this.file instanceof ZippedFile)) {
      return new Uint8Array([]);
    }
    const zippedFile: ZippedFile = this.file as ZippedFile;
    const zippedEntry: EntryExtended = zippedFile._entry;
    if (zippedEntry.directory) {
      return new Uint8Array([]);
    }
    const writable = this._stream.writable;
    const getData = zippedEntry.getData.bind(zippedEntry);
    this._response ||= new Response(this._stream.readable);
    this._writerPromise ||= getData(writable);
    this._readerPromise ||= this._response.arrayBuffer();

    // Wait until we have produced enough data
    const data = await this._readerPromise;

    // Return the appropriate slice of that data
    length ||= this.size - start;
    return new Uint8Array(data.slice(start, start + length));
  }

  async writeUnbuffered(/*start: number, bytes: Uint8Array*/): Promise<number> {
    // TODO: overlay writes (for now do not allow writes... wrap in an overlay)
    return 0;
  }
}

type EntryExtended = {
  fullname?: string;
} & Entry;

export class ZippedDirectory extends NativeDirectory {
  _zipFile: File;
  _zipReader: ZipReader<Blob>;
  _allEntries: Entry[];
  _entries: EntryExtended[];
  _files: File[] | undefined;

  constructor(
    name: string,
    entries: EntryExtended[],
    zipFile: File,
    zipReader: ZipReader<Blob>,
    allEntries: EntryExtended[],
    parent: Directory | undefined = undefined,
  ) {
    super(name, undefined, parent, false, true);
    this._zipFile = zipFile;
    this._zipReader = zipReader;
    this._allEntries = allEntries;
    this._entries = entries;
    this._files = undefined;
  }

  async list(): Promise<File[]> {
    this._files ||= this._entries.map((entry) => {
      let fullname = entry.fullname || entry.filename;
      if (!fullname.endsWith('/')) {
        fullname = fullname + '/';
      }
      const name = entry.filename.endsWith('/')
        ? entry.filename.substring(0, entry.filename.length - 1)
        : entry.filename;

      if (entry.directory) {
        const subentries = this._allEntries
          .filter((subentry) => subentry.filename.startsWith(fullname))
          .map((subentry) => ({
            ...subentry,
            fullname: subentry.filename,
            filename: subentry.filename.substring(fullname.length),
          }))
          .filter(
            (subentry) =>
              ((subentry.filename.endsWith('/') &&
                subentry.filename.match(/[/]/g)?.length === 1) ||
                !subentry.filename.includes('/')) &&
              subentry.filename.length > 0,
          );
        return new ZippedDirectory(
          name,
          subentries,
          this._zipFile,
          this._zipReader,
          this._allEntries,
          this,
        );
      }

      return new ZippedFile(name, entry, this._zipFile, this._zipReader);
    });

    return this._files;
  }

  async exists(): Promise<boolean> {
    return true;
  }

  async stat(): Promise<Stat> {
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

export class ZippedFile extends NativeFile {
  _zipFile: File;
  _zipReader: ZipReader<Blob>;
  _entry: EntryExtended;

  constructor(
    name: string,
    entry: EntryExtended,
    zipFile: File,
    zipReader: ZipReader<Blob>,
  ) {
    super(name, undefined, false, true);
    this._zipFile = zipFile;
    this._zipReader = zipReader;
    this._entry = entry;
  }

  async exists(): Promise<boolean> {
    return true;
  }

  async open(): Promise<Stream | undefined> {
    // You cannot open a directory
    if (this.isDirectory) {
      return undefined;
    }

    const size = this._entry.uncompressedSize;
    return new ZippedStream(this, size);
  }

  async size(): Promise<number> {
    return this._entry.uncompressedSize;
  }

  async stat(): Promise<Stat> {
    const fileSize = this._entry.uncompressedSize;
    // TODO: make blocksize the cluster size (sector size)
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
}

export class ZippedDrive extends Drive {
  _entries: Entry[];

  constructor(name: string, entries: Entry[], root: Directory) {
    super(name, DriveTypes.ContainedStorage, MediaTypes.Fixed, root);
    this._entries = entries;
  }

  /**
   * Opens a file chooser to pick a zip file and returns a Drive to
   * represent it.
   */
  static async pick(): Promise<Drive | undefined> {
    if (Drive.localStorageAvailable) {
      const files: FileSystemFileHandle[] | undefined =
        // @ts-expect-error: showOpenFilePicker is not available in all environments
        await window.showOpenFilePicker({
          startIn: 'desktop',
          types: [
            {
              description: 'All Compressed Formats',
              accept: {
                'application/octet-stream+zip': ['.zip'],
              },
            },
            {
              description: 'Zip File',
              accept: {
                'application/octet-stream+zip': ['.zip'],
              },
            },
          ],
        });

      if (files === undefined || files.length === 0) {
        return undefined;
      }

      // Open image
      const fileHandle = files[0];
      const zipFile = new NativeFile(fileHandle.name, fileHandle, true);
      return await ZippedDrive.load(zipFile);
    }
  }

  static async load(zipFile: File): Promise<Drive | undefined> {
    const stream = await zipFile.open();
    if (!stream) {
      return undefined;
    }

    const zipFileReader = new BlobReader(stream.blob);
    const zipReader = new ZipReader<Blob>(zipFileReader);

    // Pulling out all entries
    const allEntries = await zipReader.getEntries();
    const entries = allEntries.filter(
      (entry) =>
        (entry.filename.endsWith('/') &&
          entry.filename.match(/[/]/g)?.length === 1) ||
        !entry.filename.includes('/'),
    );
    const root = new ZippedDirectory(
      zipFile.name,
      entries,
      zipFile,
      zipReader,
      allEntries,
    );
    return new ZippedDrive(zipFile.name, allEntries, root);
  }
}

export default ZippedDrive;
