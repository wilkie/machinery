import Directory from './Directory.js';
import TypedFile from './TypedFile.js';

import type { RJSFSchema } from '@rjsf/utils';

/**
 * The possible states a process can be in.
 */
export const ProcessStates = {
  Queued: 0,
  Running: 1,
  Completed: 2,
  Failed: 3,
  TimedOut: 4,
};

export type ProcessState = (typeof ProcessStates)[keyof typeof ProcessStates];

/**
 * The common process options that you can pass along when constructing
 * any Process.
 */
export type ProcessorOptions = {
  /**
   * A callback when an output is generated.
   */
  onOutput?: (id: number, file: TypedFile) => void;
  /**
   * A callback when a new process is placed on the queue.
   */
  onQueue?: (
    id: number,
    filelist: TypedFile[],
    info: ProcessInfo,
    aggregate: boolean,
  ) => void;
  /**
   * A callback for when a process changes its state.
   */
  onUpdate?: (id: number, state: ProcessState) => void;
  /**
   * A callback for standard output for the process.
   */
  onMessage?: (id: number, data: string) => void;
  /**
   * A callback for standard error output for the process.
   */
  onError?: (id: number, message: string) => void;
  /**
   * The working directory for the processes to run within. Can help processes
   * find related files.
   */
  workingDirectory?: Directory;
  /**
   * A set of configuration options to pass along to processes that match the given 'action'.
   */
  configuration?: {
    [key: string]: ProcessOptions;
  };
};

/**
 * Describes an input or output to a Process.
 */
export type ProcessPipeInfo = {
  type: string;
  subtype?: string;
};

/**
 * Describes a Process.
 */
export type ProcessInfo = {
  /** A description of the action this process is making. */
  action: string;
  /** A descriptive name for the process. */
  name: string;
  /** A description of the inputs this process is providing. */
  input: ProcessPipeInfo[];
  /** A description of the outputs this process may create. */
  output: ProcessPipeInfo[];
  /** A JSON-Schema description of options that this process can take. */
  schema?: RJSFSchema;
  /** A helpful description for apply rules to the UI elements representing the options. */
  uiSchema?: {
    /* eslint-disable-next-line @typescript-eslint/no-explicit-any -- Just allow any blob for the properties block */
    [key: string]: any;
  };
};

/**
 * General type for incoming process options.
 *
 * A process can describe these via the 'schema' property of their ProcessInfo.
 */
export type ProcessOptions = {
  /* eslint-disable-next-line @typescript-eslint/no-explicit-any -- Just allow any blob for the properties block */
  [key: string]: any;
};

/**
 * A type to pass a definition of a Process.
 */
export type ProcessDefinition<
  OPTIONS extends ProcessorOptions = ProcessorOptions,
> = (new (options: OPTIONS) => Process<TypedFile, TypedFile, OPTIONS>) & {
  subtype: string;

  canCreate: <_OPTIONS>(file: TypedFile) => boolean;
  canTransform: <_OPTIONS>(file: TypedFile) => boolean;

  processInfo: ProcessInfo;
  aggregate: boolean;
};

/**
 * A process is a transform that takes some input, of type IN, and produces,
 * asynchronously, some output of the OUT type.
 */
export abstract class Process<
  IN extends TypedFile,
  OUT extends TypedFile,
  OPTIONS extends ProcessorOptions = ProcessorOptions,
> {
  _options: OPTIONS;

  constructor(options: OPTIONS) {
    this._options = options;
  }

  get options(): OPTIONS {
    return this._options;
  }

  static subtype: string = '';

  /**
   * Whether or not the process expects to be run when all input files are
   * produced. As opposed to being able to transform single files in parallel.
   */
  static get aggregate(): boolean {
    return false;
  }

  /**
   * Specific implementations of processes provide information about the
   * transformation here. The Processor uses this to determine how to queue
   * processes in order to create new data.
   */
  static _processInfo: ProcessInfo = {
    action: 'unknown',
    name: 'Unknown',
    input: [],
    output: [],
  };

  static get processInfo(): ProcessInfo {
    return this._processInfo;
  }

  static set processInfo(value: ProcessInfo) {
    this._processInfo = value;
  }

  static collectProcessInfo<OPTIONS extends ProcessorOptions>(): ProcessInfo {
    // Find the processInfo and subtype
    // We have to super-coerce the prototype type to the ProcessDefinition
    let current: ProcessDefinition<OPTIONS> = this
      .prototype as unknown as ProcessDefinition<OPTIONS>;
    let subtype: string | undefined = this.subtype;
    let processInfo: ProcessInfo | undefined = this.processInfo;
    while (!processInfo && !subtype && current) {
      processInfo ??= current.processInfo;
      subtype ??= current.subtype;
      current = current.prototype;
    }

    // Squish subtype into generic ProcessInfo
    const ret: ProcessInfo = {
      name: processInfo.name,
      action: processInfo.action,
      input: [
        ...processInfo.input.map((input) => ({ ...input, subtype: subtype })),
      ],
      output: [
        ...processInfo.output.map((output) => ({
          ...output,
          subtype: subtype,
        })),
      ],
    };

    // Return it
    return ret;
  }

  /**
   * Whether or not this process takes the given type of file as input.
   */
  static canTransform<OPTIONS extends ProcessorOptions>(
    file: TypedFile,
  ): boolean {
    return this.collectProcessInfo<OPTIONS>().input.some(
      (info) => info.type === file.type && info.subtype === file.subtype,
    );
  }

  /**
   * Whether or not this process creates the given type of file.
   */
  static canCreate<OPTIONS extends ProcessorOptions>(file: TypedFile): boolean {
    return this.collectProcessInfo<OPTIONS>().output.some(
      (info) => info.type === file.type && info.subtype === file.subtype,
    );
  }

  /**
   * Performs the transformation.
   */
  abstract run(
    id: number,
    input: IN[],
    options: ProcessOptions,
  ): Promise<OUT | undefined>;
}

/**
 * An aggregate process is a process that takes a set of inputs, each of type IN,
 * and produces, asynchronously, some singular output of the OUT type.
 */
export abstract class AggregateProcess<
  IN extends TypedFile,
  OUT extends TypedFile,
  OPTIONS extends ProcessorOptions = ProcessorOptions,
> extends Process<IN, OUT, OPTIONS> {
  static get aggregate(): boolean {
    return true;
  }
}

/**
 * Describes a potential machine target.
 */
export type Target = {
  name: string;
};

/**
 * Describes a build system for a particular target.
 */
export type TargetBuildSystem<
  OPTIONS extends ProcessorOptions = ProcessorOptions,
> = {
  name: string;
  target: string;
  subtype: string;
  processes: ProcessDefinition<OPTIONS>[];
};

const PROCESS_TIMEOUT_SECONDS = 5000;
const MAX_TRANSFORM_PASSES = 10;

/**
 * This orchestrates a transformation.
 */
export class Processor<OPTIONS extends ProcessorOptions = ProcessorOptions> {
  _instances: Map<
    ProcessDefinition<OPTIONS>,
    Process<TypedFile, TypedFile, OPTIONS>
  >;
  _queue: Promise<TypedFile | undefined>[] = [];
  _targetInfo: TargetBuildSystem<OPTIONS>;
  _options: OPTIONS;
  _queueCount: number = 0;

  /**
   * Creates a processor for the given target information.
   */
  constructor(targetInfo: TargetBuildSystem<OPTIONS>, options: OPTIONS) {
    this._targetInfo = targetInfo;
    this._options = options || {};
    this._instances = new Map<
      ProcessDefinition<OPTIONS>,
      Process<TypedFile, TypedFile, OPTIONS>
    >();
  }

  get options(): OPTIONS {
    return { ...this._options };
  }

  /**
   * Queues a single process to run on the given file list.
   *
   * Returns true if the process is queued to run.
   */
  queue(process: ProcessDefinition<OPTIONS>, files: TypedFile[]): boolean {
    // Determine if an instance already exists
    const instance = this._instances.has(process)
      ? this._instances.get(process)
      : this._instances.set(process, new process(this.options)).get(process);

    if (!instance) {
      return false;
    }

    const id = this._queueCount;
    this._queueCount++;

    if (this._options.onQueue) {
      this._options.onQueue(id, files, process.processInfo, process.aggregate);
    }

    this._queue.push(
      new Promise((resolve, reject) => {
        // Create timeout timeout
        const timer = setTimeout(() => {
          // Update process state to suggest it timed out
          if (this._options.onUpdate) {
            this._options.onUpdate(id, ProcessStates.TimedOut);
          }
          resolve(undefined);
        }, PROCESS_TIMEOUT_SECONDS);

        if (this._options.onUpdate) {
          this._options.onUpdate(id, ProcessStates.Running);
        }

        // TODO: pass id and hook into onMessage/onError for stdout, stderr
        instance
          .run(
            id,
            files,
            (this._options.configuration || {})[
              process.processInfo.action || ''
            ] || {},
          )
          .then((result) => {
            clearTimeout(timer);
            if (this._options.onUpdate) {
              if (!result) {
                this._options.onUpdate(id, ProcessStates.Failed);
              } else {
                this._options.onUpdate(id, ProcessStates.Completed);
              }
            }

            if (this._options.onOutput && result) {
              this._options.onOutput(id, result);
            }
            resolve(result);
          })
          .catch((error) => reject(error));
      }),
    );
    return true;
  }

  /**
   * Transforms all the given files into new forms based on the current target's
   * processes. It will only do so if the origin file is newer than the prior
   * output or such output does not exist in the file list.
   */
  async transform(files: TypedFile[]): Promise<TypedFile[]> {
    // Get the aggregate process list
    const aggregates = this._targetInfo.processes.filter(
      (process) => process.aggregate,
    );
    return await this.transformPass([], files, [], aggregates);
  }

  /**
   * Internal method to encapsulate a particular transformation pass.
   */
  private async transformPass(
    results: TypedFile[],
    files: TypedFile[],
    processed: TypedFile[],
    aggregates: ProcessDefinition<OPTIONS>[],
    count: number = 0,
  ): Promise<TypedFile[]> {
    // Do not allow more than 10 passes
    if (count === MAX_TRANSFORM_PASSES) {
      console.log('ran out of time');
      return results;
    }

    // Retain the aggregate processes

    // Perform each 'pass' over the files to create a new list of unprocessed files
    let left: TypedFile[] = [];

    for (const file of files) {
      // Determine the process we want to use to transform and queue it
      let queued: boolean = false;
      for (const process of this._targetInfo.processes) {
        // Ignore aggregate processes for now
        if (!process.aggregate) {
          // This process transform one type into another...
          // is it a candidate for this file?
          if (process.canTransform<OPTIONS>(file)) {
            // Have we already transformed it? Look for an existing file that
            // has ours as an origin and this process can create it.
            const existing: TypedFile[] = files.filter(
              (subfile) =>
                process.canCreate(subfile) && subfile.origin.includes(file),
            );

            if (existing.length !== 0) {
              // TODO: Do not queue if the existing file is newer
              results = results.concat(existing);
              continue;
            }

            this.queue(process, [file]);
            processed.push(file);
            queued = true;
          }
        }
      }

      if (!queued) {
        left.push(file);
      }
    }

    // If there is nothing in the queue, run possible queued aggregates
    if (this._queue.length === 0) {
      // If there are no aggregates left, we have nothing to do
      if (aggregates.length === 0) {
        // Return the current set of resulting files
        return results;
      }

      // Try to see if there are any aggregates to run
      const aggregatesLeft = [];
      for (const process of aggregates) {
        // Filter out input files for the aggregate
        const inputs = files.filter((file) =>
          process.canTransform<OPTIONS>(file),
        );
        if (inputs.length > 0) {
          // Check for preexisting aggregate output
          const existing: TypedFile[] = files.filter(
            (subfile) =>
              process.canCreate(subfile) &&
              inputs.every((input) => subfile.origin.includes(input)),
          );

          if (existing.length !== 0) {
            // TODO: Do not queue if the existing file is newer
            results = results.concat(existing);
            continue;
          }
          this.queue(process, inputs);
        } else {
          aggregatesLeft.push(process);
        }
      }

      if (aggregatesLeft.length === aggregates.length) {
        // We failed to queue any aggregates either. Bail!
        return results;
      }

      // Truncate our aggregates so we don't run any twice over
      aggregates = aggregatesLeft;

      // We won't do any more processing of existing files after we perform our aggregates
      left = [];
    }

    // Wait until this pass completes
    const result = (await Promise.all(this._queue)).filter(
      (item) => item !== undefined,
    );

    // Clear queue.
    this._queue = [];

    // Run another pass with the results of this one
    return await this.transformPass(
      results.concat(result),
      left.concat(result),
      processed,
      aggregates,
      count + 1,
    );
  }
}

export default Processor;
