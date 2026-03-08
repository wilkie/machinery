import { ExtendedDataTypes } from './ExtendedDataType';
import { ExecutableFile } from './ExecutableFile';
import { ObjectFile } from './ObjectFile';
import { AggregateToolchainProcess } from './Process';
import type { ProcessInfo } from '../Processor';
import type { ToolchainProcessorOptions } from './Process';

/**
 * This process takes a set of object data and produces an executable.
 */
export abstract class Linker extends AggregateToolchainProcess<
  ObjectFile,
  ExecutableFile
> {
  abstract run(
    id: number,
    input: ObjectFile[],
    options: ToolchainProcessorOptions,
  ): Promise<ExecutableFile | undefined>;

  static processInfo: ProcessInfo = {
    action: 'link',
    name: 'Generic Linker',
    input: [
      {
        type: ExtendedDataTypes.ObjectData,
      },
    ],
    output: [
      {
        type: ExtendedDataTypes.ExecutableData,
      },
    ],
  };
}

export default Linker;
