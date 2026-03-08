import NativeStream from './NativeStream.js';
import NativeFile from './NativeFile.js';
import NativeDirectory from './NativeDirectory.js';
import Drive, { DriveType, MediaType } from './Drive.js';
import { File, Directory, Stream, type Stat } from '@machinery/core';

export const FloppyDiskTypes = {
  Unknown: 'unknown',
}

export type FloppyDiskType = typeof FloppyDiskTypes[keyof typeof FloppyDiskTypes];

export const FileSystemTypes = {
  Raw: 'raw',
  FAT12: 'fat12',
  FAT16: 'fat16',
  FAT32: 'fat32',
}

export type FileSystemType = typeof FileSystemTypes[keyof typeof FileSystemTypes];

export interface DirectoryEntry {
  filename: string;
  size: number;
  flags: number;
  isDirectory: boolean;
  isHidden: boolean;
  isSystem: boolean;
  isReadOnly: boolean;
  isArchive: boolean;
  createdAt: number;
  accessedAt: number;
  modifiedAt: number;
  index: number;
}

export interface DiskInfo {
  sectorSize: number;
  sectorsPerCluster: number;
  reservedSectors: number;
  fatCount: number;
  maxRootDirectoryEntries: number;
  sectorsPerFAT: number;
  sectorsPerTrack: number;
  headCount: number;
  sectorCount: number;
  volumeId: number;
  volumeIdentifier: string;
  fileSystemType: string;
}

const UNKNOWN_INDEX = -1;
const FIRST_BYTE = 0;

const BYTES_PER_DIRECTORY_ENTRY = 32;

const DEFAULT_SECTOR_SIZE = 512;
const DEFAULT_STAT_VALUE = 0;
const DEFAULT_STAT_MODE_DIRECTORY = 0x4000;
const DEFAULT_STAT_MODE_FILE = 0x8000;
const DEFAULT_DIRECTORY_SIZE = 1024;
const DEFAULT_BLOCK_SIZE = 1024;

export class FloppyDiskStream extends NativeStream {
  async readUnbuffered(
    start: number,
    length: number,
  ): Promise<Uint8Array<ArrayBuffer>> {
    if (!(this.file instanceof FloppyDiskFile)) {
      return await super.readUnbuffered(start, length);
    }

    const file: FloppyDiskFile = this.file;
    const disk: FloppyDisk = file.disk;
    const entry: DirectoryEntry = file.directoryEntry;
    const stream = await disk.file.open();
    let index = await disk.lookup(entry.index, start);

    // Read that block and potentially the next blocks
    let data: Uint8Array<ArrayBuffer> = (await disk.read(
      index,
      stream,
    )) as Uint8Array<ArrayBuffer>;
    let { byteLength: total } = data;
    if (total > length) {
      data = data.slice(FIRST_BYTE, length);
      total = length;
    }

    while (index !== UNKNOWN_INDEX && total < length) {
      index = await disk.next(index);
      if (index === UNKNOWN_INDEX) {
        break;
      }

      let nextData = await disk.read(index, stream);
      total = total + nextData.byteLength;
      if (total > length) {
        nextData = nextData.slice(FIRST_BYTE, length - data.byteLength);
        total = length;
      }

      const newData = new Uint8Array(total);
      newData.set(data, FIRST_BYTE);
      newData.set(nextData, data.byteLength);
      data = newData;
    }

    return data;
  }

  async writeUnbuffered(/*start: number, bytes: Uint8Array*/): Promise<number> {
    // TODO: overlay writes (for now do not allow writes... wrap in an overlay)
    const NOTHING = 0;
    return NOTHING;
  }
}

export class FloppyDiskDirectory extends NativeDirectory {
  _disk: FloppyDisk;
  _entries: DirectoryEntry[];
  _files: File[] | undefined = undefined;

  constructor(
    name: string,
    disk: FloppyDisk,
    entries: DirectoryEntry[],
    parent: Directory | undefined = undefined,
  ) {
    // TODO: allow writes to an overlay
    super(name, undefined, parent, false, true);
    this._disk = disk;
    this._entries = entries;
  }

  get disk(): FloppyDisk {
    return this._disk;
  }

  get diskEntries(): DirectoryEntry[] {
    return this._entries;
  }

  async list(): Promise<File[]> {
    this._files ??= await Promise.all(
      this.diskEntries.map(async (info) => {
        if (info.isDirectory) {
          // Craft a temporary 'file' to read it
          const file = new FloppyDiskFile(info.filename, this.disk, {
            ...info,
            size: DEFAULT_SECTOR_SIZE,
          });
          const directoryData = await file.read();

          // Parse sub-directory entries
          const subentries = FloppyDisk.parseDirectory(directoryData);
          return new FloppyDiskDirectory(
            info.filename,
            this.disk,
            subentries,
            this,
          );
        }

        return new FloppyDiskFile(info.filename, this.disk, info);
      }),
    );

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

export class FloppyDiskFile extends NativeFile {
  _disk: FloppyDisk;
  _directoryEntry: DirectoryEntry;

  constructor(name: string, disk: FloppyDisk, info: DirectoryEntry) {
    super(name, undefined, false, info.isReadOnly);
    this._disk = disk;
    this._directoryEntry = info;
  }

  get disk(): FloppyDisk {
    return this._disk;
  }

  get directoryEntry(): DirectoryEntry {
    return this._directoryEntry;
  }

  async exists(): Promise<boolean> {
    return true;
  }

  async open(): Promise<Stream | undefined> {
    // You cannot open a directory
    if (this.isDirectory) {
      return undefined;
    }

    const { size } = this.directoryEntry;
    return new FloppyDiskStream(this, size);
  }

  async size(): Promise<number> {
    return this.directoryEntry.size;
  }

  async stat(): Promise<Stat> {
    const { directoryEntry } = this;
    const { size: fileSize } = directoryEntry;
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

export class FileAllocationTable {
  _file: File;
  _type: FileSystemType;
  _info: DiskInfo;
  _data: Uint8Array;

  constructor(
    file: File,
    type: FileSystemType,
    info: DiskInfo,
    data: Uint8Array,
  ) {
    this._file = file;
    this._type = type;
    this._info = info;
    this._data = data;
  }

  get file(): File {
    return this._file;
  }

  get info(): DiskInfo {
    return this._info;
  }

  async first(index: number): Promise<number> {
    // For FAT systems, the first index is the first data index
    return index;
  }

  async next(index: number): Promise<number> {
    // Just read from the provided table data
    if (this._type === FileSystemTypes.FAT12) {
      // Look up the entry we need in the data
      const BITS_PER_ENTRY = 12;
      const BITS_PER_BYTE = 8;
      const HALF_BITS_PER_BYTE = 4;
      const HALF_BITS_MASK = 0xf;
      const NONE = 0;
      const bitOffset = BITS_PER_ENTRY * index;
      const byteOffset = Math.floor(bitOffset / BITS_PER_BYTE);
      const ONE = 1;
      const nextByteOffset = byteOffset + ONE;
      const ret =
        bitOffset % BITS_PER_BYTE > NONE
          ? ((this._data[byteOffset] >> HALF_BITS_PER_BYTE) & HALF_BITS_MASK) |
            (this._data[nextByteOffset] << HALF_BITS_PER_BYTE)
          : this._data[byteOffset] |
            ((this._data[nextByteOffset] & HALF_BITS_MASK) << BITS_PER_BYTE);

      // Bail out when the next index is 'bad' or too high
      const BAD_VALUE = 0xff7; /* 0xff8 is the end */
      if (ret >= BAD_VALUE) {
        return UNKNOWN_INDEX;
      }

      return ret;
    }

    throw new Error('We need to implement reading the table dynamically.');
    return UNKNOWN_INDEX;
  }
}

export class FloppyDisk {
  _file: File;
  _table: FileAllocationTable;
  _info: DiskInfo;

  /**
   * Creates a floppy disk instance around the given image file.
   */
  constructor(file: File, table: FileAllocationTable, info: DiskInfo) {
    this._file = file;
    this._table = table;
    this._info = info;
  }

  get file(): File {
    return this._file;
  }

  get info() {
    return this._info;
  }

  /**
   * Looks up data by its node index.
   */
  async first(index: number): Promise<number> {
    return await this._table.first(index);
  }

  async next(index: number): Promise<number> {
    return await this._table.next(index);
  }

  async lookup(index: number, offset: number): Promise<number> {
    index = await this._table.first(index);
    let position = 0;
    const { info } = this;
    const { sectorSize: blockSize } = info;

    // Keep travelling through the table
    while (index !== UNKNOWN_INDEX && position + blockSize <= offset) {
      position += blockSize;
      index = await this._table.next(index);
    }

    // We ran out of data before we got to the offset
    if (position + blockSize <= offset) {
      return UNKNOWN_INDEX;
    }

    return index;
  }

  /**
   * Reads the sector/block at the given index.
   */
  async read(
    index: number,
    stream: Stream | undefined = undefined,
  ): Promise<Uint8Array> {
    // TODO: use dimensions from disk info
    const { info } = this;
    const { sectorSize: blockSize } = info;
    stream ??= await this.file.open();
    if (stream === undefined) {
      return new Uint8Array([]);
    }

    const RESERVED_ENTRIES = 2;
    const ONE = 1;
    const BLOCK_SIZE_MINUS_ONE = blockSize - ONE;
    const rootDirectorySectors = Math.floor(
      (this.info.maxRootDirectoryEntries * BYTES_PER_DIRECTORY_ENTRY +
        BLOCK_SIZE_MINUS_ONE) /
        blockSize,
    );
    const firstDataSector =
      this.info.sectorsPerFAT * this.info.fatCount +
      this.info.reservedSectors +
      rootDirectorySectors;
    const clusterIndex = index - RESERVED_ENTRIES + firstDataSector;
    await stream.seek(clusterIndex * blockSize);
    return await stream.read(blockSize);
  }

  /**
   * Opens a file chooser to pick a floppy disk image and returns a Drive to
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
              description: 'All Disk Image Formats',
              accept: {
                'application/octet-stream+floppy-image': ['.img'],
              },
            },
            {
              description: 'Standard Disk Image',
              accept: {
                'application/octet-stream+img': ['.img'],
              },
            },
          ],
        });

      if (files === undefined || files.length === 0) {
        return undefined;
      }

      // Open image
      const [fileHandle] = files;
      const imageFile = new NativeFile(fileHandle.name, fileHandle, true);
      return await FloppyDisk.load(imageFile);
    }
  }

  /**
   * Loads a floppy disk from an image file.
   */
  static async load(file: File): Promise<Drive | undefined> {
    const stream = await file.open();
    if (stream === undefined) {
      return undefined;
    }

    // Try to read data to detect a FAT system
    const bootSector = await stream.read(DEFAULT_SECTOR_SIZE);
    if (FloppyDisk.isFAT12(bootSector)) {
      await stream.seek(FIRST_BYTE);
      return await FloppyDisk.loadFAT12(file, stream);
    }

    return undefined;
  }

  static isFAT12(bootSectorData: Uint8Array): boolean {
    // Get a text decoder ready
    const decoder = new TextDecoder();

    // We can look at the type tag for a quick reference, but we can also infer
    // this if we do not trust the string to be there. (TODO: infer by sector sizes)
    const MINIMUM_SECTOR_DATA_SIZE_START = 54;
    const MINIMUM_SECTOR_DATA_SIZE_END = 59;
    const typeTag = decoder.decode(
      bootSectorData.slice(
        MINIMUM_SECTOR_DATA_SIZE_START,
        MINIMUM_SECTOR_DATA_SIZE_END,
      ),
    );
    return typeTag === 'FAT12';
  }

  static async loadFAT12(file: File, stream: Stream): Promise<Drive> {
    const BOOT_SECTOR_DATA_SIZE = 64;
    const bootSector = await stream.read(BOOT_SECTOR_DATA_SIZE);
    const view = new DataView(bootSector.buffer);
    const decoder = new TextDecoder();

    const info = {
      sectorSize: view.getUint16(11, true),
      sectorsPerCluster: bootSector[13],
      reservedSectors: view.getUint16(14, true),
      fatCount: bootSector[16],
      maxRootDirectoryEntries: view.getUint16(17, true),
      sectorsPerFAT: view.getUint16(22, true),
      sectorsPerTrack: view.getUint16(24, true),
      headCount: view.getUint16(26, true),
      sectorCount: view.getUint32(32, true),
      volumeId: view.getUint32(39, true),
      volumeIdentifier: decoder
        .decode(bootSector.slice(43, 43 + 11))
        .trimRight(),
      fileSystemType: decoder.decode(bootSector.slice(54, 54 + 8)).trimRight(),
    };

    // Skip the reserved sectors. The next thing to follow is the FAT itself.
    await stream.seek(info.sectorSize * info.reservedSectors);

    // Read the whole allocation table
    const allocationTable = new FileAllocationTable(
      file,
      FileSystemTypes.FAT12,
      info,
      await stream.read(info.sectorSize * info.sectorsPerFAT),
    );
    await stream.seek(
      info.sectorSize *
        (info.sectorsPerFAT * info.fatCount + info.reservedSectors),
    );

    // Read the root directory
    const maxRootDirectorySize =
      info.maxRootDirectoryEntries * BYTES_PER_DIRECTORY_ENTRY;
    const rootDirectoryData = await stream.read(maxRootDirectorySize);

    const entries = FloppyDisk.parseDirectory(rootDirectoryData);

    const disk = new FloppyDisk(file, allocationTable, info);
    const root = new FloppyDiskDirectory(
      info.volumeIdentifier.length === 0 ? file.name : info.volumeIdentifier,
      disk,
      entries,
    );

    return new Drive(
      root.name,
      DriveType.ContainedStorage,
      MediaType.Floppy,
      root,
    );
  }

  static parseDirectory(directoryData: Uint8Array) {
    const decoder = new TextDecoder();
    const entries = [];
    const DIRECTORY_ENTRY_END = 0x0;
    const DIRECTORY_ENTRY_UNUSED = 0xe5;
    for (
      let i = 0;
      i <= directoryData.byteLength - BYTES_PER_DIRECTORY_ENTRY;
      i += BYTES_PER_DIRECTORY_ENTRY
    ) {
      const dirEntryData = directoryData.slice(i);
      const dirEntryView = new DataView(dirEntryData.buffer);
      if (dirEntryData[0] === DIRECTORY_ENTRY_UNUSED) {
        // Unused entry... skip over it
        continue;
      }
      if (dirEntryData[0] === DIRECTORY_ENTRY_END) {
        // Terminator
        break;
      }
      const FILENAME_LENGTH = 8;
      const FILENAME_EXT_END = 11;
      let filename =
        decoder.decode(dirEntryData.slice(0, FILENAME_LENGTH)).trimRight() +
        '.' +
        decoder
          .decode(dirEntryData.slice(FILENAME_LENGTH, FILENAME_EXT_END))
          .trimRight();
      if (filename.endsWith('.')) {
        filename = filename.substring(0, filename.length - 1);
      }
      // TODO Determine created at time:
      // byte 13 is hundreths of a second
      // byte 14-15 is time (hhhhhmmmmmmsssss) (multiply seconds by 2)
      // byte 16-17 is date (yyyyyyymmmmddddd)
      // byte 18-19 is accessed date (yyyyyyymmmmddddd)
      // byte 22-23 is modified time (hhhhmmmmmmsssss) (multiply seconds by 2)
      // byte 24-25 is modified date (yyyyyyymmmmddddd)

      // For FAT32, the index has its high 16-bits from offset 20

      const index = dirEntryView.getUint16(26, true);
      const flags = dirEntryData[11];
      const dirEntry = {
        filename,
        flags,
        createdAt: 0,
        accessedAt: 0,
        modifiedAt: 0,
        index,
        size: dirEntryView.getUint32(28, true),
        isDirectory: !!(flags & 0x10),
        isHidden: !!(flags & 0x02),
        isSystem: !!(flags & 0x04),
        isReadOnly: !!(flags & 0x01),
        isArchive: !!(flags & 0x20),
      };

      entries.push(dirEntry);
    }

    return entries;
  }
}

export default FloppyDisk;
