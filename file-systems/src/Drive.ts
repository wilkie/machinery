import NativeDirectory from './NativeDirectory.js';
import { Directory } from '@machinery/core';

export type DriveEntry = {
  name: string;
  type: DriveType;
  mediaType: MediaType;
};

export type DriveEntrySerialization = {
  name: string;
  handle: FileSystemDirectoryHandle;
};

export const DriveTypes = {
  /**
   * The browser's StorageAPI.
   */
  BrowserStorage: 'browser',

  /**
   * The local filesystem, whenever available.
   */
  LocalStorage: 'local',

  /**
   * A Container filesystem, like a CD image.
   */
  ContainedStorage: 'contained',

  /**
   * This storage system is not persisted.
   */
  TemporaryStorage: 'temporary',
} as const;

export type DriveType = (typeof DriveTypes)[keyof typeof DriveTypes];

export const MediaTypes = {
  /**
   * A virtual disk the simulates a hard drive.
   */
  Fixed: 'fixed',

  /**
   * A Floppy disk (removable)
   */
  Floppy: 'floppy',

  /**
   * A compact-disc (CD)
   */
  Optical: 'optical',
} as const;

export type MediaType = (typeof MediaTypes)[keyof typeof MediaTypes];

export type DriveInfo = {
  name: string;
  type: DriveType;
};

const openDriveDB = async (
  namespace: string,
): Promise<IDBDatabase | undefined> => {
  return await dbOpen(namespace, (newDB) => {
    newDB.createObjectStore('drives', { keyPath: 'name' });
  });
};

const dbOpen = (
  namespace: string,
  createCallback: (database: IDBDatabase) => void,
): Promise<IDBDatabase | undefined> => {
  return new Promise((resolve) => {
    const DB_VERSION = 2;
    const DBOpenRequest = window.indexedDB.open(
      `_drives_${namespace}`,
      DB_VERSION,
    );

    DBOpenRequest.onerror = () => {
      resolve(undefined);
    };

    DBOpenRequest.onsuccess = () => {
      resolve(DBOpenRequest.result);
    };

    DBOpenRequest.onupgradeneeded = (event) => {
      // TypeScript does not currently understand this event having a target
      // @ts-expect-error: Property 'result' does not exist on type 'EventTarget'.
      createCallback(event.target.result);
    };
  });
};

const dbTransaction = <T>(
  db: IDBDatabase,
  objects: string[],
  callback: (transaction: IDBTransaction) => Promise<T>,
  mode: IDBTransactionMode = 'readwrite',
): Promise<T | boolean> => {
  return new Promise((resolve) => {
    const DBTransaction = db.transaction(objects, mode);

    DBTransaction.onerror = () => {
      resolve(false);
    };

    let ret: T | undefined = undefined;
    DBTransaction.oncomplete = () => {
      if (ret === undefined) {
        resolve(false);
      } else {
        resolve(ret as T);
      }
    };

    callback(DBTransaction).then((value: T) => {
      ret = value;
    });
    DBTransaction.commit();
    if (ret === undefined) {
      return false;
    }
    return ret as T;
  });
};

const dbGetAll = <T>(objectStore: IDBObjectStore): Promise<T[] | undefined> => {
  return new Promise((resolve) => {
    const request: IDBRequest = objectStore.getAll();

    request.onerror = () => {
      resolve(undefined);
    };

    request.onsuccess = () => {
      resolve(request.result as T[]);
    };
  });
};

/**
 * A storage system with a root directory.
 */
export class Drive {
  _name: string;
  _type: DriveType;
  _mediaType: MediaType;
  _root: NativeDirectory;

  constructor(
    name: string,
    type: DriveType,
    mediaType: MediaType,
    root: FileSystemDirectoryHandle | NativeDirectory,
  ) {
    this._name = name;
    this._type = type;
    this._mediaType = mediaType;
    this._root =
      root instanceof NativeDirectory
        ? root
        : new NativeDirectory(
            name,
            root,
            undefined,
            type === DriveTypes.LocalStorage,
          );
  }

  /**
   * Uses a picker to create a drive from a local directory, if available.
   *
   * Will return `undefined` if the picker is not available or the action is
   * cancelled.
   */
  static async pick(
    namespace: string = '',
    remember: boolean = false,
  ): Promise<Drive | undefined> {
    if (!Drive.localStorageAvailable) {
      return undefined;
    }

    const dir = await NativeDirectory.fromPicker();
    if (dir === undefined) {
      return undefined;
    }

    const ret = new Drive(
      dir.name,
      DriveTypes.LocalStorage,
      MediaTypes.Fixed,
      dir,
    );

    // Store this drive for later
    if (remember) {
      const db = await openDriveDB(namespace);
      if (db === undefined) {
        return ret;
      }

      await dbTransaction<boolean>(
        db,
        ['drives'],
        async (transaction) => {
          const objectStore = transaction.objectStore('drives');
          objectStore.add({
            name: dir.name,
            handle: dir.handle,
          });
          return true;
        },
        'readwrite',
      );
      db.close();
    }

    return ret;
  }

  /**
   * Returns the information for all existing drives.
   */
  static async list(namespace: string = ''): Promise<DriveInfo[]> {
    // Open the root directory of the browser filesystem
    const ret = [];
    if (Drive.browserStorageAvailable) {
      const base = await window.navigator.storage.getDirectory();
      const basedir = await base.getDirectoryHandle(`_drives_${namespace}`, {
        create: true,
      });
      for await (const value of basedir.values()) {
        ret.push({
          name: value.name,
          type: DriveTypes.BrowserStorage,
        });
      }
    }

    // List existing mapped local drives
    if (Drive.localStorageAvailable) {
      const db = await openDriveDB(namespace);
      if (db === undefined) {
        return ret;
      }

      await dbTransaction<DriveEntry[]>(
        db,
        ['drives'],
        async (transaction) => {
          const objectStore = transaction.objectStore('drives');
          const values = await dbGetAll<DriveEntrySerialization>(objectStore);
          if (values === undefined) {
            return [];
          }

          for (const value of values) {
            ret.push({
              name: value.name,
              type: DriveTypes.LocalStorage,
              mediaType: MediaTypes.Fixed,
            });
          }

          return [];
        },
        'readonly',
      );
    }

    return ret;
  }

  /**
   * Returns information about a particular drive with the given name, if
   * available.
   *
   * Returns undefined if the drive does not exist.
   */
  static async stat(
    name: string,
    namespace: string = '',
  ): Promise<DriveInfo | undefined> {
    if (Drive.browserStorageAvailable) {
      const base = await window.navigator.storage.getDirectory();
      const basedir = await base.getDirectoryHandle(`_drives_${namespace}`, {
        create: true,
      });
      try {
        await basedir.getDirectoryHandle(name);
      } catch (e) {
        const error: Error = e as Error;
        if (error.name === 'NotFoundError') {
          return undefined;
        }
      }
      return {
        name: name,
        type: DriveTypes.BrowserStorage,
      };
    }
  }

  /**
   * Returns an active instance of the existing Drive with the given name.
   *
   * Will return undefined if the drive with that name does not exist.
   */
  static async open(
    name: string,
    namespace: string = '',
  ): Promise<Drive | undefined> {
    if (Drive.browserStorageAvailable) {
      const base = await window.navigator.storage.getDirectory();
      const basedir = await base.getDirectoryHandle(`_drives_${namespace}`, {
        create: true,
      });
      try {
        const handle = await basedir.getDirectoryHandle(name);
        return new Drive(
          name,
          DriveTypes.BrowserStorage,
          MediaTypes.Fixed,
          handle,
        );
      } catch {
        // Let it fall through to the next check
      }
    }

    // See if we have any existing known local storage drive opened
    if (Drive.localStorageAvailable) {
      const db = await openDriveDB(namespace);
      if (db === undefined) {
        return undefined;
      }

      const handle = await dbTransaction<Drive | boolean>(
        db,
        ['drives'],
        async (transaction) => {
          const objectStore = transaction.objectStore('drives');
          const values = await dbGetAll<DriveEntrySerialization>(objectStore);
          if (values === undefined) {
            return false;
          }
          for (const value of values) {
            if (value.name === name) {
              return new Drive(
                name,
                DriveTypes.LocalStorage,
                MediaTypes.Fixed,
                value.handle,
              );
            }
          }
          return false;
        },
        'readonly',
      );

      if (handle !== false) {
        return handle as Drive;
      }
    }

    return undefined;
  }

  /**
   * Returns true if a drive with the given name already exists.
   */
  static async exists(name: string, namespace: string = ''): Promise<boolean> {
    return !!(await Drive.stat(name, namespace));
  }

  /**
   * Deletes the drive with the given name, if possible.
   *
   * Returns `true` if the drive was successfully removed.
   */
  static async destroy(name: string, namespace: string = ''): Promise<boolean> {
    if (!(await Drive.exists(name, namespace))) {
      return false;
    }

    if (Drive.browserStorageAvailable) {
      const base = await window.navigator.storage.getDirectory();
      const basedir = await base.getDirectoryHandle(`_drives_${namespace}`, {
        create: true,
      });
      await basedir.removeEntry(name, { recursive: true });
    }
    return true;
  }

  /**
   * Determines if the Storage API is available.
   */
  static get browserStorageAvailable(): boolean {
    // Browser Storage API is available if storage is active
    return !!window.navigator.storage;
  }

  /**
   * Determines if the local File System API is available.
   */
  static get localStorageAvailable(): boolean {
    // Local filesystem access is available if the directory picker is active
    // @ts-expect-error: showDirectoryPicker is not available in all environments
    return !!window.showDirectoryPicker;
  }

  static get canPersist(): boolean {
    return Drive.browserStorageAvailable;
  }

  /**
   * Creates a persisted drive with the given name.
   */
  static async create(
    name: string,
    namespace: string = '',
  ): Promise<Drive | undefined> {
    if (Drive.browserStorageAvailable) {
      const base = await window.navigator.storage.getDirectory();
      const basedir = await base.getDirectoryHandle(`_drives_${namespace}`, {
        create: true,
      });
      const handle = await basedir.getDirectoryHandle(name, { create: true });
      return new Drive(
        name,
        DriveTypes.BrowserStorage,
        MediaTypes.Fixed,
        handle,
      );
    }

    // No means to create a Drive
    return undefined;
  }

  /**
   * Gets the name of the Drive.
   */
  get name(): string {
    return this._name;
  }

  /**
   * Gets the type of backend for this Drive.
   */
  get type(): DriveType {
    return this._type;
  }

  /**
   * Gets the media type this drive simulates.
   */
  get mediaType(): MediaType {
    return this._mediaType;
  }

  /**
   * Gets the root directory for the Drive.
   */
  get root(): Directory {
    return this._root;
  }
}

export default Drive;
