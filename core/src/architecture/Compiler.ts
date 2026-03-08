import { ExtendedDataTypes } from './ExtendedDataType';
import { SourceFile } from './SourceFile';
import { AssemblyFile } from './AssemblyFile';
import { ObjectFile } from './ObjectFile';
import type { ProcessInfo } from '../Processor';
import { ToolchainProcess } from './Process';
import type { ToolchainProcessorOptions } from './Process';

export abstract class Compiler extends ToolchainProcess<
  SourceFile,
  ObjectFile | AssemblyFile
> {
  abstract run(
    id: number,
    input: SourceFile[],
    options: ToolchainProcessorOptions,
  ): Promise<ObjectFile | AssemblyFile | undefined>;

  static processInfo: ProcessInfo = {
    action: 'compile',
    name: 'Generic Compiler',
    input: [
      {
        type: ExtendedDataTypes.SourceData,
      },
    ],
    output: [
      {
        type: ExtendedDataTypes.ExecutableData,
      },
      {
        type: ExtendedDataTypes.AssemblyData,
      },
    ],
  };
}

export default Compiler;
