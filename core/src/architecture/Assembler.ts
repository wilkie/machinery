import { ExtendedDataTypes } from './ExtendedDataType';
import { AssemblyFile } from './AssemblyFile';
import { ObjectFile } from './ObjectFile';
import { ToolchainProcess } from './Process';
import type { ProcessInfo } from '../Processor';
import type { ToolchainProcessorOptions } from './Process';

/**
 * This process takes assembly source data and produces an assembled object file.
 */
export abstract class Assembler extends ToolchainProcess<
  AssemblyFile,
  ObjectFile
> {
  abstract run(
    id: number,
    input: AssemblyFile[],
    options: ToolchainProcessorOptions,
  ): Promise<ObjectFile | undefined>;

  static processInfo: ProcessInfo = {
    action: 'assemble',
    name: 'Generic Assembler',
    input: [
      {
        type: ExtendedDataTypes.AssemblyData,
      },
    ],
    output: [
      {
        type: ExtendedDataTypes.ObjectData,
      },
    ],
  };
}

export default Assembler;
