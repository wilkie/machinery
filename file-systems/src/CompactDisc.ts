import NativeStream from './NativeStream.js';
import NativeFile from './NativeFile.js';
import NativeDirectory from './NativeDirectory.js';
import Drive, { DriveType, MediaType } from './Drive.js';
import { File, Stream, type Stat } from '@machinery/core';

export enum CompactDiscTrackMode {
  Unknown,
  LeadIn,
  LeadOut,
  Audio,
  Mode1,
  Mode2,
}

export type CompactDiscTrackInfo = {
  number: number;
  mode: CompactDiscTrackMode;
  sectorSize: number;
  indexPoints: {
    [key: string]: number;
  };
  flags: string[];
  imageIndex: number;
};

export type CompactDiscImageInfo = {
  filename: string;
  type: string;
  trackIndices: number[];
};

export type CompactDiscInfo = {
  name: string;
  type: string;
  images: CompactDiscImageInfo[];
  tracks: CompactDiscTrackInfo[];
};

export type CompactDiscVolumeDescriptor = {
  type: CompactDiscVolumeType;
  identifier: string;
  systemIdentifier: string;
  volumeIdentifier: string;
  size: number;
  sequence: number;
  blockSize: number;
  pathTableSize: number;
  pathTableOffset: number;
  optionalPathTableOffset: number;
  directoryEntryData: Uint8Array;
  volumeSetIdentifier: string;
  publishedIdentifier: string;
  dataPreparerIdentifier: string;
  applicationIdentifier: string;
  copyrightIdentifier: string;
  abstractIdentifier: string;
  bibliographicIdentifier: string;
  createdAt: number;
  modifiedAt: number;
  expiresAt: number;
  effectiveAt: number;
  data: Uint8Array;
};

export type CompactDiscDirectoryEntry = {
  recordSize: number;
  extendedAttributeRecordSize: number;
  entriesOffset: number;
  entriesSize: number;
  flags: number;
  fileUnitSize: number;
  interleaveGapSize: number;
  volumeSequence: number;

  // Optionally available on certain entries
  filename?: string;

  // Added by us by interpreting flags
  isDirectory: boolean;
  isHidden: boolean;

  // Added by us later
  offset?: number;
};

export enum CompactDiscVolumeType {
  BootRecord = 0,
  Primary = 1,
  Supplementary = 2,
  Partition = 3,
  Terminator = 255,
}

export enum CompactDiscFileFlags {
  Hidden = 0x1,
  Directory = 0x2,
  Associated = 0x04,
  Record = 0x8,
  HasOwner = 0x10,
  HasExtendedAttributes = 0x20,
  NotFinalDirectory = 0x80,
}

// The number of 'empty' sectors to expect in some images
const EMPTY_BOOT_SECTORS = 16;

const MINIMUM_SECTOR_SIZE = 2048;
const AUDIO_TRACK_SECTOR_SIZE = 2352;
const MODE2_SECTOR_SIZE = 2336;

const DEFAULT_BIN_DATA_OFFSET = 16;
const MAXIMUM_DATA_OFFSET = 16;

const DEFAULT_STAT_VALUE = 0;
const DEFAULT_STAT_MODE_DIRECTORY = 0x4000;
const DEFAULT_STAT_MODE_FILE = 0x8000;
const DEFAULT_DIRECTORY_SIZE = 1024;
const DEFAULT_BLOCK_SIZE = 1024;

export class CompactDiscStream extends NativeStream {
  async readUnbuffered(
    start: number,
    length: number,
  ): Promise<Uint8Array<ArrayBuffer>> {
    const file: CompactDiscFile = this.file as CompactDiscFile;
    const iso: CompactDisc = file.iso;
    const stream = await iso.file.open();
    if (!stream) {
      return new Uint8Array([]);
    }
    const entry: CompactDiscDirectoryEntry = file.isoDirectoryEntry;
    await stream.seek(iso.calculateOffset(entry.entriesOffset) + start);
    return await stream.read(length);
  }

  async writeUnbuffered(/*start: number, bytes: Uint8Array*/): Promise<number> {
    // Do not allow writes to readonly stream
    return 0;
  }
}

const COMPACT_DISC_AUDIO_SAMPLE_RATE = 44100;
const COMPACT_DISC_AUDIO_CHANNELS = 2;
const COMPACT_DISC_AUDIO_BYTES_PER_SAMPLE = 2;

export class CompactDiscAudio {
  _stream: Stream;
  _audio: AudioContext;
  _audioLoadingPromise: Promise<void>;
  _gain: GainNode;
  _buffer_position: number;

  constructor(stream: Stream) {
    this._stream = stream;
    this._audio = new window.AudioContext();
    this._audioLoadingPromise = this._audio.suspend();
    this._gain = this._audio.createGain();
    this._gain.gain.value = 0.0001;
    this._gain.connect(this._audio.destination);
    this._buffer_position = 0;
  }

  get duration(): number {
    /* total bytes / samples per second / 2 bytes per sample / 2 channels */
    return (
      this._stream.size /
      COMPACT_DISC_AUDIO_SAMPLE_RATE /
      COMPACT_DISC_AUDIO_BYTES_PER_SAMPLE /
      COMPACT_DISC_AUDIO_CHANNELS
    );
  }

  get time(): number {
    return this._audio.getOutputTimestamp().contextTime || 0;
  }

  get position(): number {
    // Multiply the time by the bytes per second
    // (sample rate * 2 bytes per sample * 2 channels)
    return (
      this.time *
      COMPACT_DISC_AUDIO_SAMPLE_RATE *
      COMPACT_DISC_AUDIO_BYTES_PER_SAMPLE *
      COMPACT_DISC_AUDIO_CHANNELS
    );
  }

  async play(): Promise<void> {
    await this._audioLoadingPromise;

    const loadSamples = async () => {
      const source = this._audio.createBufferSource();
      const BUFFER_SECONDS = 5;
      const buffer = this._audio.createBuffer(
        COMPACT_DISC_AUDIO_CHANNELS,
        BUFFER_SECONDS * COMPACT_DISC_AUDIO_SAMPLE_RATE,
        COMPACT_DISC_AUDIO_SAMPLE_RATE,
      );
      const leftData = buffer.getChannelData(0);
      const rightData = buffer.getChannelData(1);
      const data = await this._stream.read(
        BUFFER_SECONDS *
          COMPACT_DISC_AUDIO_SAMPLE_RATE *
          COMPACT_DISC_AUDIO_BYTES_PER_SAMPLE *
          COMPACT_DISC_AUDIO_CHANNELS,
      );
      if (!data || data.byteLength === 0) {
        return;
      }
      const view = new DataView(data.buffer);
      for (
        let i = 0;
        i < data.byteLength;
        i += COMPACT_DISC_AUDIO_CHANNELS * COMPACT_DISC_AUDIO_BYTES_PER_SAMPLE
      ) {
        leftData[i >> COMPACT_DISC_AUDIO_BYTES_PER_SAMPLE] = view.getInt16(
          i,
          true,
        );
        rightData[i >> COMPACT_DISC_AUDIO_BYTES_PER_SAMPLE] = view.getInt16(
          i + COMPACT_DISC_AUDIO_BYTES_PER_SAMPLE,
          true,
        );
      }
      source.buffer = buffer;
      source.connect(this._gain);
      source.start(this._buffer_position);
      this._buffer_position += 5.0;

      // Queue the next buffer when this one ends
      source.addEventListener('ended', () => {
        loadSamples();
      });
    };

    if (this._buffer_position === 0) {
      // Decode the initial samples
      await loadSamples();
      await loadSamples();
    }

    await this._audio.resume();
  }

  async pause(): Promise<void> {
    await this._audioLoadingPromise;
    await this._audio.suspend();
  }
}

export class CompactDiscFile extends NativeFile {
  _iso: CompactDisc;
  _entry: CompactDiscDirectoryEntry;

  constructor(
    name: string,
    iso: CompactDisc,
    entry: CompactDiscDirectoryEntry,
  ) {
    super(name, undefined, false, true);
    this._iso = iso;
    this._entry = entry;
  }

  get isoDirectoryEntry(): CompactDiscDirectoryEntry {
    return this._entry;
  }

  get iso(): CompactDisc {
    return this._iso;
  }

  async exists(): Promise<boolean> {
    return true;
  }

  async open(): Promise<Stream | undefined> {
    // You cannot open a directory
    if (this.isDirectory) {
      return undefined;
    }

    const size = this.isoDirectoryEntry.entriesSize;
    return new CompactDiscStream(this, size);
  }

  async size(): Promise<number> {
    return this.isoDirectoryEntry.entriesSize;
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
}

export class CompactDiscDirectory extends NativeDirectory {
  _iso: CompactDisc;
  _entry: CompactDiscDirectoryEntry;
  _entries: CompactDiscDirectoryEntry[];
  _files: CompactDiscFile[] | undefined;

  constructor(
    name: string,
    iso: CompactDisc,
    entry: CompactDiscDirectoryEntry,
    entries: CompactDiscDirectoryEntry[],
    parent: CompactDiscDirectory | undefined = undefined,
  ) {
    super(name, undefined, parent, false, true);
    this._iso = iso;
    this._entry = entry;
    this._entries = entries;
    this._files = undefined;
  }

  get isoDirectoryEntry(): CompactDiscDirectoryEntry {
    return this._entry;
  }

  get iso(): CompactDisc {
    return this._iso;
  }

  get isoDirectoryEntries(): CompactDiscDirectoryEntry[] {
    return this._entries;
  }

  async list(): Promise<File[]> {
    let stream = undefined;

    this._files ||= await Promise.all(
      this.isoDirectoryEntries
        .filter((info: CompactDiscDirectoryEntry) => !!info.filename)
        .map(async (info: CompactDiscDirectoryEntry) => {
          if (info.isDirectory) {
            // Parse directory entries
            stream ||= await this.iso.file.open();
            const entries = await this.iso.parseDirectory(stream, info);
            return new CompactDiscDirectory(
              info.filename as string,
              this.iso,
              info,
              entries,
            );
          }
          return new CompactDiscFile(info.filename as string, this.iso, info);
        }),
    );

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

export class CompactDisc {
  _files: File[];
  _primaryVolume: CompactDiscVolumeDescriptor;
  _info: CompactDiscInfo;
  _dataTracks: CompactDiscTrackInfo[];
  _sectorSize: number;
  _dataOffset: number;

  constructor(
    files: File[],
    primaryVolume: CompactDiscVolumeDescriptor,
    info: CompactDiscInfo,
  ) {
    this._files = files;
    this._primaryVolume = primaryVolume;
    this._info = info;
    this._dataTracks = CompactDisc.filterDataTracks(info);
    this._sectorSize = this._dataTracks[0].sectorSize;
    this._dataOffset = this._dataTracks[0].indexPoints[1];
  }

  get info(): CompactDiscInfo {
    return { ...this._info };
  }

  get tracks(): CompactDiscTrackInfo[] {
    return [...this.info.tracks];
  }

  get dataTracks(): CompactDiscTrackInfo[] {
    return [...this._dataTracks];
  }

  get audioTracks(): CompactDiscTrackInfo[] {
    return CompactDisc.filterAudioTracks(this._info);
  }

  get mixedMode(): boolean {
    return this.dataTracks.length > 0 && this.audioTracks.length > 0;
  }

  get dataTrack(): CompactDiscTrackInfo {
    return { ...this._dataTracks[0] };
  }

  get primaryVolumeDescriptor(): CompactDiscVolumeDescriptor {
    return this._primaryVolume;
  }

  get blockSize(): number {
    return this.primaryVolumeDescriptor.blockSize;
  }

  get sectorSize(): number {
    return this._sectorSize;
  }

  get dataOffset(): number {
    return this._dataOffset;
  }

  calculateOffset(offset: number): number {
    // Calculate padding via error correction
    const eccLength = this.sectorSize - MINIMUM_SECTOR_SIZE;
    const byteOffset = offset * this.blockSize;
    const sectors = byteOffset / MINIMUM_SECTOR_SIZE;
    const eccOffset = sectors * eccLength;
    return byteOffset + eccOffset + this.dataOffset;
  }

  static filterDataTracks(info: CompactDiscInfo): CompactDiscTrackInfo[] {
    return info.tracks.filter(
      (track) =>
        ![
          CompactDiscTrackMode.Audio,
          CompactDiscTrackMode.LeadIn,
          CompactDiscTrackMode.LeadOut,
        ].includes(track.mode),
    );
  }

  static filterAudioTracks(info: CompactDiscInfo): CompactDiscTrackInfo[] {
    return info.tracks.filter(
      (track) => track.mode === CompactDiscTrackMode.Audio,
    );
  }

  trackInfoFor(number: number): CompactDiscTrackInfo | undefined {
    return this.tracks.find((track) => track.number === number);
  }

  async open(
    trackNumber: number,
    index: number = 1,
  ): Promise<Stream | undefined> {
    const info = this.trackInfoFor(trackNumber);
    if (!info) {
      return undefined;
    }

    const file = this._files[info.imageIndex];
    const start = info.indexPoints['1'];
    const nextTrackInfo = this.trackInfoFor(trackNumber + 1);
    const end =
      info.indexPoints['-1'] ||
      nextTrackInfo?.indexPoints?.['1'] ||
      (await file.size()) ||
      0;
    const ret = new NativeStream(file, end - start, start);

    if (!ret) {
      return undefined;
    }

    // Skip to the index given
    if (index !== 1) {
      if (info.indexPoints[index.toString()] !== undefined) {
        await ret.skip(info.indexPoints[index.toString()] - start);
      }
    }

    return ret;
  }

  async openAudio(trackNumber: number): Promise<CompactDiscAudio | undefined> {
    if (this.trackInfoFor(trackNumber)?.mode !== CompactDiscTrackMode.Audio) {
      return undefined;
    }

    const stream: Stream | undefined = await this.open(trackNumber);
    return stream ? new CompactDiscAudio(stream) : undefined;
  }

  static async pick(): Promise<Drive | undefined> {
    if (Drive.localStorageAvailable) {
      const files: FileSystemFileHandle[] | undefined =
        // @ts-expect-error: showOpenFilePicker is not available in all environments
        await window.showOpenFilePicker({
          startIn: 'desktop',
          multiple: true,
          types: [
            {
              description: 'All Disk Image Formats',
              accept: {
                'application/octet-stream+nrg': [
                  '.iso',
                  '.bin',
                  '.cue',
                  '.nrg',
                ],
              },
            },
            {
              description: 'CompactDisc File',
              accept: {
                'application/x-iso9660-image': ['.iso'],
              },
            },
            {
              description: 'BIN File',
              accept: {
                'application/octet-stream+bin+cue': ['.bin', '.cue'],
              },
            },
            {
              description: 'Nero Disk File',
              accept: {
                'application/octet-stream+nrg': ['.nrg'],
              },
            },
          ],
        });

      if (files === undefined || files.length === 0) {
        return undefined;
      }

      // Look for bin/cue combination
      let info: CompactDiscInfo | undefined;
      if (files.length > 1) {
        // Is there a CUE file?
        const cueFileHandle: FileSystemFileHandle | undefined = files.filter(
          (file) => file.name.toLowerCase().endsWith('.cue'),
        )[0];

        if (cueFileHandle) {
          const cueFile = new NativeFile(
            cueFileHandle.name,
            cueFileHandle,
            true,
          );
          // Read CUE file for information about how to parse the associated BIN
          info = await CompactDisc.parseCue(cueFile);
        }
      }

      const fileHandle = files[0];
      const name = fileHandle.name.toLowerCase();
      const isBin = name.endsWith('.bin');
      const isoFile = new NativeFile(fileHandle.name, fileHandle, true);
      const iso = await CompactDisc.load(
        [isoFile],
        isBin ? AUDIO_TRACK_SECTOR_SIZE : MINIMUM_SECTOR_SIZE,
        isBin ? DEFAULT_BIN_DATA_OFFSET : 0,
        info,
      );
      if (!iso) {
        return undefined;
      }
      const dir = await iso.rootDirectory();
      if (!dir) {
        return undefined;
      }

      return new Drive(
        dir.name,
        DriveType.ContainedStorage,
        MediaType.Optical,
        dir,
      );
    }

    return undefined;
  }

  get file(): File {
    return this._files[this.dataTracks[0].imageIndex];
  }

  get files(): File[] {
    return [...this._files];
  }

  get isMixedMode(): boolean {
    return (
      this.dataTracks.length > 0 &&
      this.dataTracks.length !== this.tracks.length
    );
  }

  async rootDirectory(): Promise<CompactDiscDirectory | undefined> {
    const stream = await this.file.open();
    if (!stream) {
      return undefined;
    }

    // Look at the directory entry info
    const volumeInfo = this.primaryVolumeDescriptor;
    const directoryEntry = CompactDisc.parseDirectoryEntry(
      volumeInfo.directoryEntryData,
    );

    // Parse the root directory structure
    const entries = await this.parseDirectory(stream, directoryEntry);

    await stream.close();
    return new CompactDiscDirectory(
      volumeInfo.volumeIdentifier,
      this,
      directoryEntry,
      entries,
    );
  }

  static async parseNero(file: File): Promise<CompactDiscInfo | undefined> {
    type DAOInfo = {
      [key: string]: {
        sectorSize: number;
        mode: number;
        indexPoints: {
          [key: string]: number;
        };
      };
    };

    type TemporaryTracksInfo = {
      [key: string]: CompactDiscTrackInfo;
    };

    const name = file.name.replace(/\.[^.]+$/, '');

    const decodeBCD: (value: number) => number = (value: number) =>
      (value & 0xf) + ((value & 0xf0) >> 4) * 10;

    const ret: CompactDiscInfo = {
      name: name,
      type: 'nrg',
      tracks: [],
      images: [
        {
          filename: file.name,
          type: 'nrg',
          trackIndices: [],
        },
      ],
    };

    // Get a text decoder ready
    const decoder = new TextDecoder();

    // Open file
    const stream = await file.open();
    if (!stream) {
      return undefined;
    }

    // Seek to end of file
    const FOOTER_SIZE = 12;
    await stream.seek(stream.size - FOOTER_SIZE);
    const footerData = await stream.read(FOOTER_SIZE);
    let offset = 0;

    if (decoder.decode(footerData.slice(0, 4)) === 'NER5') {
      // Nero version 2
      offset = Number(new DataView(footerData.buffer).getBigUint64(4));
    } else if (decoder.decode(footerData.slice(4, 8)) === 'NERO') {
      // Nero version 1
      offset = new DataView(footerData.buffer).getUint32(8);
    }

    // Seek to chunk offset
    await stream.seek(offset);

    // Start reading chunks
    let lastChunk = '';
    let currentTrack = undefined;
    const DAO: DAOInfo = {};
    const tracks: TemporaryTracksInfo = {};

    while (lastChunk !== 'END!') {
      const chunkHeader = await stream.read(8);
      const chunkSize = new DataView(chunkHeader.buffer).getUint32(4);
      lastChunk = decoder.decode(chunkHeader.slice(0, 4));

      if (lastChunk === 'CUEX') {
        // Read chunk data
        const chunkData = await stream.read(chunkSize);

        for (let start = 0; start < chunkData.byteLength - 7; start += 8) {
          const cuePointData = chunkData.slice(start, start + 8);

          // Parse cue structure
          const cue = {
            mode: cuePointData[0],
            track: decodeBCD(cuePointData[1]),
            index: decodeBCD(cuePointData[2]),
            offset: new DataView(cuePointData.buffer).getInt32(4),
          };

          tracks[cue.track] ||= {
            mode: CompactDiscTrackMode.Unknown,
            number: cue.track,
            sectorSize: MINIMUM_SECTOR_SIZE,
            indexPoints: {},
            flags: [],
            imageIndex: 0,
          };
          currentTrack = tracks[cue.track];

          tracks[cue.track].indexPoints[cue.index] = cue.offset * 2048;

          // 0xaa when bcd-decoded is 0x6e (100 + 10)
          if (cue.track === 0x6e) {
            // Lead-out
            currentTrack.mode = CompactDiscTrackMode.LeadOut;
          } else if (cue.track === 0x00) {
            // Lead-out
            currentTrack.mode = CompactDiscTrackMode.LeadIn;
          } else if (cue.mode === 0x01 || cue.mode === 0x21) {
            // Audio
            currentTrack.mode = CompactDiscTrackMode.Audio;
          } else {
            // Data
            currentTrack.mode = CompactDiscTrackMode.Mode2;
          }
        }
      } else if (lastChunk === 'DAOX') {
        // Parse DAO (Disc-At-Once) table
        const chunkData = await stream.read(chunkSize);

        // Append to existing DAO information
        const firstTrackNumber = chunkData[20];

        // Read each track info for this Disc-At-Once session
        let trackNumber = firstTrackNumber;
        for (let start = 22; start < chunkData.byteLength - 41; start += 42) {
          const trackData = chunkData.slice(start);
          const trackView = new DataView(trackData.buffer);
          DAO[trackNumber.toString()] = {
            sectorSize: trackView.getUint16(12),
            // 0x0000: Data
            // 0x0300: Mode 2 Form 1 Data
            // 0x0500: Raw Data
            // 0x0600: Raw Mode 2 Form 1 Data
            // 0x0700: Audio
            // 0x0f00: Raw Data + sub-channel
            // 0x1000: Audio + sub-channel
            // 0x1100: Raw Mode 2 Form 1 Data + sub-channel
            mode: trackView.getUint16(14),
            indexPoints: {
              '0': Number(trackView.getBigInt64(18)),
              '1': Number(trackView.getBigInt64(26)),
              '-1': Number(trackView.getBigInt64(34)),
            },
          };
          ret.images[0].trackIndices.push(trackNumber);
          trackNumber++;
        }
      } else {
        // Skip chunk
        await stream.skip(chunkSize);
      }
    }

    // Compile an ordered list of tracks
    ret.tracks = Object.values(tracks).sort((track) => track?.number);

    // Append DAO information to tracks
    ret.tracks.forEach((track) => {
      const DAOInfo = DAO[track.number.toString()];
      track.sectorSize = DAOInfo?.sectorSize || track.sectorSize;
      track.indexPoints = DAOInfo?.indexPoints || track.indexPoints;
    });

    return ret;
  }

  static async parseCue(file: File): Promise<CompactDiscInfo | undefined> {
    type TemporaryTracksInfo = {
      [key: string]: CompactDiscTrackInfo;
    };

    // Read and parse the lines of the cue file
    const stream = await file.open();
    if (!stream) {
      return undefined;
    }

    const name = file.name.replace(/\.[^.]+$/, '');

    const ret: CompactDiscInfo = {
      name: name,
      type: 'bin+cue',
      tracks: [],
      images: [],
    };

    let currentImage = undefined;
    let currentTrack: CompactDiscTrackInfo | undefined = undefined;

    let line = await stream.readLine('utf-8');
    const tracks: TemporaryTracksInfo = {};
    while (line !== undefined) {
      // Parse line
      line = line.trim();
      const parts = line.split(' ');
      const [commandType, key, value] = parts;
      if (commandType.toUpperCase() === 'FILE') {
        // We are looking at a new file
        ret.images.push({
          filename: key,
          type: value.toUpperCase(),
          trackIndices: [],
        });
        currentImage = ret.images[ret.images.length - 1];
        currentTrack = undefined;
      } else if (commandType.toUpperCase() === 'TRACK' && currentImage) {
        // We are adding a track to the current file
        const modeParts = value.split('/').map((part) => part.toUpperCase());
        const modeMap: { [key: string]: CompactDiscTrackMode } = {
          AUDIO: CompactDiscTrackMode.Audio,
          MODE1: CompactDiscTrackMode.Mode1,
        };

        const trackKey = parseInt(key).toString();
        tracks[trackKey] = {
          number: parseInt(parts[1]),
          mode: modeMap[modeParts[0]],
          sectorSize:
            modeParts[0] === 'AUDIO'
              ? AUDIO_TRACK_SECTOR_SIZE
              : parseInt(modeParts[1]),
          flags: [],
          indexPoints: {},
          imageIndex: ret.images.length - 1,
        };

        currentTrack = tracks[trackKey];
        currentImage.trackIndices.push(parseInt(key));
      } else if (parts[0].toUpperCase() === 'FLAGS' && currentTrack) {
        currentTrack.flags.push(key);
      } else if (parts[0].toUpperCase() === 'INDEX' && currentTrack) {
        const number = parseInt(key).toString();
        const [minutes, seconds, fraction] = value
          .split(':')
          .map((value) => parseInt(value));

        // Get the number of centi-seconds (parts of 100 per second)
        const CENTISECONDS_PER_MINUTE = 6000;
        const CENTISECONDS_PER_SECOND = 100;
        const cseconds =
          minutes * CENTISECONDS_PER_MINUTE +
          seconds * CENTISECONDS_PER_SECOND +
          fraction;

        // Multiply the number of seconds by the number of bytes per centi-second
        // (44100hz sample rate across 2 channels at 2 bytes per sample)
        currentTrack.indexPoints[number] =
          cseconds *
          (COMPACT_DISC_AUDIO_SAMPLE_RATE / CENTISECONDS_PER_SECOND) *
          COMPACT_DISC_AUDIO_CHANNELS *
          COMPACT_DISC_AUDIO_BYTES_PER_SAMPLE;

        // Add 16 bytes to the initial data track
        if (currentTrack.mode === CompactDiscTrackMode.Mode1) {
          currentTrack.indexPoints[number] += 16;
        }
      }
      line = await stream.readLine('utf-8');
    }

    // Sort tracks
    ret.tracks = Object.values(tracks).sort(
      (track: CompactDiscTrackInfo) => track.number,
    );

    return ret;
  }

  static async load(
    files: File[],
    sectorSize: number = MINIMUM_SECTOR_SIZE,
    dataOffset: number = 0,
    info: CompactDiscInfo | undefined,
  ): Promise<CompactDisc | undefined> {
    if (files.length === 0) {
      return undefined;
    }

    // Get the file representing the first data track
    const file = files[0];

    // Determine the file type we are dealing with
    const type: string = file.name.includes('.')
      ? (file.name.toLowerCase().split('.').pop() as string)
      : 'binary';

    // Parse the info sheet, if necessary
    if (type === 'nrg') {
      info ||= await CompactDisc.parseNero(file);
    }

    // The number of sectors worth of data we will initially read
    const SECTORS_TO_READ = 10;

    // Get a default info sheet (with our provided guesses for sectorSize and
    // track offset)
    info ||= {
      name: file.name.replace(/\.[^.]+$/, ''),
      type: type,
      tracks: [
        {
          number: 1,
          mode: CompactDiscTrackMode.Unknown,
          sectorSize: sectorSize,
          indexPoints: {
            '1': dataOffset,
          },
          flags: [],
          imageIndex: 0,
        },
      ],
      images: [
        {
          filename: file.name,
          type: type,
          trackIndices: [],
        },
      ],
    };

    // First: Get the volume descriptors
    const stream = await file.open();
    if (!stream) {
      return undefined;
    }

    // Brute-force sector size selection if we get it wrong in our given guesses
    const dataTracks = CompactDisc.filterDataTracks(info);

    let primaryInfo = undefined;
    for (const track of dataTracks) {
      // If our track mode is unknown, we have to try a few options
      let data = new Uint8Array([]);
      if (track.mode === CompactDiscTrackMode.Unknown) {
        let sectorSizesToAttempt = [track.sectorSize];
        let attemptedAll = false;
        while (sectorSizesToAttempt.length > 0) {
          sectorSize = sectorSizesToAttempt.pop() as number;
          // Skip the header area and go to the designated volume descriptor block
          await stream.seek(sectorSize * EMPTY_BOOT_SECTORS);

          // Read the first few volume descriptors
          data = await stream.read(sectorSize * 1);

          // Try to determine the sector size and course-correct if we are wrong
          const decoder = new TextDecoder();
          let found = false;
          for (let i = dataOffset; i <= MAXIMUM_DATA_OFFSET; i++) {
            const decoded = decoder.decode(data.slice(1 + i, 1 + i + 5));
            if (decoded === 'CD001') {
              // found it
              found = true;
              dataOffset = i;
              break;
            }
          }

          if (!found) {
            if (sectorSizesToAttempt.length == 0) {
              if (!attemptedAll) {
                // Retry with some common sector sizes
                attemptedAll = true;
                dataOffset = 0;
                sectorSizesToAttempt = [
                  MINIMUM_SECTOR_SIZE,
                  AUDIO_TRACK_SECTOR_SIZE,
                  MODE2_SECTOR_SIZE,
                ].filter((size) => size != sectorSize);
              } else {
                // Hmm, cannot determine
                return undefined;
              }
            }
          } else {
            // Slide our streamed data over the appropriate offset
            data = data.slice(dataOffset);

            // Update our track info
            track.sectorSize = sectorSize;
            track.indexPoints[1] = dataOffset;
            break;
          }
        }
      } else {
        // Read the data from the provided values
        sectorSize = track.sectorSize;
        dataOffset = track.indexPoints['1'];

        // Skip the 16 'empty' sectors past the offset
        await stream.seek(dataOffset + EMPTY_BOOT_SECTORS * sectorSize);
        data = await stream.read(sectorSize * SECTORS_TO_READ);
      }

      for (let i = 0; i < data.byteLength; i += sectorSize) {
        const volumeInfo = CompactDisc.parseVolumeDescriptor(data.slice(i));
        if (
          volumeInfo.type === CompactDiscVolumeType.Primary &&
          volumeInfo.identifier === 'CD001'
        ) {
          // This is a primary volume
          // (Some CompactDisc files specify more than one primary for some reason...
          // do not overwrite it when it is already set)
          primaryInfo ||= volumeInfo;
        } else if (volumeInfo.type === CompactDiscVolumeType.Terminator) {
          // This contains a sentinel value to halt iteration
          break;
        }
      }
      break;
    }

    // Close file stream
    await stream?.close();

    if (!primaryInfo) {
      return undefined;
    }

    // Create the CompactDisc instance
    return new CompactDisc(files, primaryInfo, info);
  }

  static parseVolumeDescriptor(data: Uint8Array): CompactDiscVolumeDescriptor {
    const decoder = new TextDecoder();
    const view = new DataView(data.buffer);

    return {
      type: data[0] as CompactDiscVolumeType,
      identifier: decoder.decode(data.slice(1, 1 + 5)),
      systemIdentifier: decoder.decode(data.slice(8, 8 + 32)).trimRight(),
      volumeIdentifier: decoder.decode(data.slice(40, 40 + 32)).trimRight(),
      size: view.getUint16(120, true),
      sequence: view.getUint16(124, true),
      blockSize: view.getUint16(128, true),
      pathTableSize: view.getUint32(132, true),
      pathTableOffset: view.getUint32(140, true),
      optionalPathTableOffset: view.getUint32(144, true),
      directoryEntryData: data.slice(156, 156 + 34),
      volumeSetIdentifier: decoder
        .decode(data.slice(190, 190 + 128))
        .trimRight(),
      publishedIdentifier: decoder
        .decode(data.slice(318, 318 + 128))
        .trimRight(),
      dataPreparerIdentifier: decoder
        .decode(data.slice(446, 446 + 128))
        .trimRight(),
      applicationIdentifier: decoder
        .decode(data.slice(574, 574 + 128))
        .trimRight(),
      copyrightIdentifier: decoder
        .decode(data.slice(702, 702 + 37))
        .trimRight(),
      abstractIdentifier: decoder.decode(data.slice(739, 739 + 37)).trimRight(),
      bibliographicIdentifier: decoder
        .decode(data.slice(776, 776 + 37))
        .trimRight(),
      createdAt: 0,
      modifiedAt: 0,
      expiresAt: 0,
      effectiveAt: 0,
      data: data,
    };
  }

  static parseDirectoryEntry(data: Uint8Array): CompactDiscDirectoryEntry {
    const size = data[0];
    const view = new DataView(data.buffer);
    const PARTIAL_DIRECTORY_ENTRY_SIZE = 34;
    const filenameInfo =
      data.byteLength > PARTIAL_DIRECTORY_ENTRY_SIZE
        ? {
            filename: new TextDecoder()
              .decode(data.slice(33, 33 + view.getUint8(32)))
              .split(';')[0],
          }
        : {};

    const flags = view.getUint8(25);
    return {
      recordSize: size,
      extendedAttributeRecordSize: view.getUint8(1),
      entriesOffset: view.getUint32(2, true),
      entriesSize: view.getUint32(10, true),
      flags: flags,
      fileUnitSize: view.getUint8(26),
      interleaveGapSize: view.getUint8(27),
      volumeSequence: view.getUint16(28, true),
      isDirectory: !!(flags & CompactDiscFileFlags.Directory),
      isHidden: !!(flags & CompactDiscFileFlags.Hidden),
      ...filenameInfo,
    };
  }

  async parseDirectory(
    stream: Stream,
    directoryEntry: CompactDiscDirectoryEntry,
  ): Promise<CompactDiscDirectoryEntry[]> {
    const size = directoryEntry.entriesSize;
    await stream.seek(this.calculateOffset(directoryEntry.entriesOffset));

    // We will fill this with directory entry data
    const ret = [];

    // Read in a few sectors of data
    const data = await stream.read(size);
    let ptr = 0;
    while (ptr < data.byteLength) {
      // Seek toward a filled in directory entry (one where the record size is
      // greater than zero)
      while (ptr < data.byteLength && data[ptr] === 0) {
        ptr++;
      }

      if (ptr >= data.byteLength) {
        break;
      }

      // Read this directory entry
      const entry = CompactDisc.parseDirectoryEntry(
        data.slice(ptr, ptr + data[ptr]),
      );
      entry.offset = entry.entriesOffset * this.blockSize;
      ptr += entry.recordSize;
      ret.push(entry);
    }

    return ret;
  }
}

export default CompactDisc;
