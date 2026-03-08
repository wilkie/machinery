import {
  MemoryDrive,
  MemoryFile,
  MemoryDirectory,
  MemoryStream,
} from './MemoryDrive.js';
import { DriveType, MediaType } from './Drive.js';

describe('MemoryDrive', () => {
  it('creates a drive with the given name', () => {
    const drive = new MemoryDrive('test');
    expect(drive.name).toBe('test');
  });

  it('has TemporaryStorage type and Fixed media type', () => {
    const drive = new MemoryDrive('test');
    expect(drive.type).toBe(DriveType.TemporaryStorage);
    expect(drive.mediaType).toBe(MediaType.Fixed);
  });

  it('has a root directory', () => {
    const drive = new MemoryDrive('test');
    expect(drive.root).toBeInstanceOf(MemoryDirectory);
    expect(drive.root.isDirectory).toBe(true);
  });
});

describe('MemoryDirectory', () => {
  it('reports as a directory', () => {
    const dir = new MemoryDirectory('root');
    expect(dir.isDirectory).toBe(true);
    expect(dir.isData).toBe(false);
  });

  it('starts with an empty listing', async () => {
    const dir = new MemoryDirectory('root');
    const files = await dir.list();
    expect(files).toEqual([]);
  });

  it('can create files', async () => {
    const dir = new MemoryDirectory('root');
    const file = await dir.createFile('hello.txt');
    expect(file).toBeDefined();
    expect(file!.name).toBe('hello.txt');

    const files = await dir.list();
    expect(files).toHaveLength(1);
    expect(files[0].name).toBe('hello.txt');
  });

  it('can create files with initial data', async () => {
    const dir = new MemoryDirectory('root');
    const data = new Uint8Array([1, 2, 3, 4]);
    const file = await dir.createFile('data.bin', data);
    expect(file).toBeInstanceOf(MemoryFile);
    expect(await file!.size()).toBe(4);
  });

  it('can create subdirectories', async () => {
    const dir = new MemoryDirectory('root');
    const sub = await dir.createDirectory('subdir');
    expect(sub).toBeInstanceOf(MemoryDirectory);
    expect(sub!.isDirectory).toBe(true);
    expect(sub!.name).toBe('subdir');

    const files = await dir.list();
    expect(files).toHaveLength(1);
    expect(files[0].isDirectory).toBe(true);
  });

  it('lists multiple files and directories', async () => {
    const dir = new MemoryDirectory('root');
    await dir.createFile('a.txt');
    await dir.createDirectory('b');
    await dir.createFile('c.txt');

    const files = await dir.list();
    expect(files).toHaveLength(3);
    expect(files.map((f) => f.name)).toEqual(['a.txt', 'b', 'c.txt']);
  });

  it('reports existence', async () => {
    const dir = new MemoryDirectory('root');
    expect(await dir.exists()).toBe(true);
  });

  it('returns stat with directory mode', async () => {
    const dir = new MemoryDirectory('root');
    const stat = await dir.stat();
    expect(stat).toBeDefined();
    expect(stat!.st_mode).toBe(0x4000);
    expect(stat!.st_size).toBe(1024);
  });

  describe('find', () => {
    it('finds a file by name', async () => {
      const dir = new MemoryDirectory('root');
      await dir.createFile('target.txt');
      await dir.createFile('other.txt');

      const found = await dir.find('target.txt');
      expect(found).toBeDefined();
      expect(found!.name).toBe('target.txt');
    });

    it('returns undefined for missing files', async () => {
      const dir = new MemoryDirectory('root');
      await dir.createFile('exists.txt');

      const found = await dir.find('missing.txt');
      expect(found).toBeUndefined();
    });

    it('supports case-insensitive search', async () => {
      const dir = new MemoryDirectory('root');
      await dir.createFile('Hello.TXT');

      const found = await dir.find('hello.txt', { caseInsensitive: true });
      expect(found).toBeDefined();
      expect(found!.name).toBe('Hello.TXT');
    });
  });

  describe('locate', () => {
    it('locates files in nested directories', async () => {
      const root = new MemoryDirectory('root');
      const sub = (await root.createDirectory('sub')) as MemoryDirectory;
      await sub.createFile('deep.txt');

      const found = await root.locate('sub/deep.txt');
      expect(found).toBeDefined();
      expect(found!.name).toBe('deep.txt');
    });

    it('returns itself for empty path', async () => {
      const dir = new MemoryDirectory('root');
      const found = await dir.locate('');
      expect(found).toBe(dir);
    });

    it("returns itself for '.' path", async () => {
      const dir = new MemoryDirectory('root');
      const found = await dir.locate('.');
      expect(found).toBe(dir);
    });

    it('strips leading slash', async () => {
      const dir = new MemoryDirectory('root');
      await dir.createFile('file.txt');
      const found = await dir.locate('/file.txt');
      expect(found).toBeDefined();
      expect(found!.name).toBe('file.txt');
    });

    it('returns undefined for non-existent path', async () => {
      const dir = new MemoryDirectory('root');
      const found = await dir.locate('missing/file.txt');
      expect(found).toBeUndefined();
    });

    it('locates parent directory with parent option', async () => {
      const root = new MemoryDirectory('root');
      const sub = (await root.createDirectory('sub')) as MemoryDirectory;
      await sub.createFile('file.txt');

      const found = await root.locate('sub/file.txt', { parent: true });
      expect(found).toBeDefined();
      expect(found!.name).toBe('sub');
    });
  });

  describe('destroy', () => {
    it('removes a file by name', async () => {
      const dir = new MemoryDirectory('root');
      await dir.createFile('doomed.txt');
      await dir.createFile('safe.txt');

      const result = await dir.destroy('doomed.txt');
      expect(result).toBe(true);

      const files = await dir.list();
      expect(files).toHaveLength(1);
      expect(files[0].name).toBe('safe.txt');
    });

    it('returns false for non-existent file', async () => {
      const dir = new MemoryDirectory('root');
      const result = await dir.destroy('nope.txt');
      expect(result).toBe(false);
    });
  });
});

describe('MemoryFile', () => {
  it('starts with empty data by default', async () => {
    const file = new MemoryFile('empty.txt');
    expect(await file.size()).toBe(0);
    expect(await file.exists()).toBe(true);
  });

  it('stores initial data', async () => {
    const data = new Uint8Array([10, 20, 30]);
    const file = new MemoryFile('data.bin', data);
    expect(await file.size()).toBe(3);
  });

  it('is not a directory', () => {
    const file = new MemoryFile('file.txt');
    expect(file.isDirectory).toBe(false);
    expect(file.isData).toBe(true);
  });

  it('returns stat with file mode', async () => {
    const data = new Uint8Array([1, 2, 3, 4, 5]);
    const file = new MemoryFile('file.txt', data);
    const stat = await file.stat();
    expect(stat).toBeDefined();
    expect(stat!.st_mode).toBe(0x8000);
    expect(stat!.st_size).toBe(5);
  });

  it('can be opened as a stream', async () => {
    const data = new Uint8Array([1, 2, 3]);
    const file = new MemoryFile('file.bin', data);
    const stream = await file.open();
    expect(stream).toBeInstanceOf(MemoryStream);
    expect(stream!.size).toBe(3);
  });

  describe('truncate', () => {
    it('truncates to a smaller size', async () => {
      const data = new Uint8Array([1, 2, 3, 4, 5]);
      const file = new MemoryFile('file.bin', data);
      const result = await file.truncate(3);
      expect(result).toBe(true);
      expect(await file.size()).toBe(3);
      expect(file._data).toEqual(new Uint8Array([1, 2, 3]));
    });

    it('extends with zeroes when truncating to larger size', async () => {
      const data = new Uint8Array([1, 2]);
      const file = new MemoryFile('file.bin', data);
      await file.truncate(5);
      expect(await file.size()).toBe(5);
      expect(file._data).toEqual(new Uint8Array([1, 2, 0, 0, 0]));
    });
  });
});

describe('MemoryStream', () => {
  it('reads data from the file', async () => {
    const data = new Uint8Array([10, 20, 30, 40, 50]);
    const file = new MemoryFile('file.bin', data);
    const stream = (await file.open()) as MemoryStream;

    const result = await stream.read(3);
    expect(result).toEqual(new Uint8Array([10, 20, 30]));
  });

  it('advances position on read', async () => {
    const data = new Uint8Array([1, 2, 3, 4, 5]);
    const file = new MemoryFile('file.bin', data);
    const stream = (await file.open()) as MemoryStream;

    await stream.read(2);
    expect(stream.position).toBe(2);

    const rest = await stream.read(3);
    expect(rest).toEqual(new Uint8Array([3, 4, 5]));
    expect(stream.position).toBe(5);
  });

  it('writes data to the file', async () => {
    const file = new MemoryFile('file.bin');
    const stream = (await file.open()) as MemoryStream;

    const written = await stream.write(new Uint8Array([100, 200]));
    expect(written).toBe(2);
    expect(stream.position).toBe(2);

    // Read back
    await stream.seek(0);
    const result = await stream.read(2);
    expect(result).toEqual(new Uint8Array([100, 200]));
  });

  it('overwrites existing data', async () => {
    const data = new Uint8Array([1, 2, 3, 4, 5]);
    const file = new MemoryFile('file.bin', data);
    const stream = (await file.open()) as MemoryStream;

    await stream.seek(1);
    await stream.write(new Uint8Array([99, 98]));

    await stream.seek(0);
    const result = await stream.read(5);
    expect(result).toEqual(new Uint8Array([1, 99, 98, 4, 5]));
  });

  it('appends data beyond current file size', async () => {
    const data = new Uint8Array([1, 2]);
    const file = new MemoryFile('file.bin', data);
    const stream = (await file.open()) as MemoryStream;

    await stream.seek(2);
    await stream.write(new Uint8Array([3, 4, 5]));

    expect(await file.size()).toBe(5);
    await stream.seek(0);
    const result = await stream.read(5);
    expect(result).toEqual(new Uint8Array([1, 2, 3, 4, 5]));
  });

  describe('seek', () => {
    it('seeks to an absolute position', async () => {
      const data = new Uint8Array([1, 2, 3, 4, 5]);
      const file = new MemoryFile('file.bin', data);
      const stream = (await file.open()) as MemoryStream;

      const pos = await stream.seek(3);
      expect(pos).toBe(3);
      expect(stream.position).toBe(3);
    });

    it('clamps to zero for negative positions', async () => {
      const data = new Uint8Array([1, 2, 3]);
      const file = new MemoryFile('file.bin', data);
      const stream = (await file.open()) as MemoryStream;

      const pos = await stream.seek(-5);
      expect(pos).toBe(0);
      expect(stream.position).toBe(0);
    });
  });

  describe('skip', () => {
    it('advances position by the given amount', async () => {
      const data = new Uint8Array([1, 2, 3, 4, 5]);
      const file = new MemoryFile('file.bin', data);
      const stream = (await file.open()) as MemoryStream;

      await stream.seek(1);
      const pos = await stream.skip(2);
      expect(pos).toBe(3);
      expect(stream.position).toBe(3);
    });
  });

  describe('rewind', () => {
    it('moves position back by the given amount', async () => {
      const data = new Uint8Array([1, 2, 3, 4, 5]);
      const file = new MemoryFile('file.bin', data);
      const stream = (await file.open()) as MemoryStream;

      await stream.seek(4);
      const pos = await stream.rewind(2);
      expect(pos).toBe(2);
      expect(stream.position).toBe(2);
    });

    it('clamps to zero when rewinding past start', async () => {
      const data = new Uint8Array([1, 2, 3]);
      const file = new MemoryFile('file.bin', data);
      const stream = (await file.open()) as MemoryStream;

      await stream.seek(1);
      const pos = await stream.rewind(5);
      expect(pos).toBe(0);
    });
  });

  it('can close without error', async () => {
    const file = new MemoryFile('file.bin');
    const stream = (await file.open()) as MemoryStream;
    const result = await stream.close();
    expect(result).toBe(true);
  });
});
