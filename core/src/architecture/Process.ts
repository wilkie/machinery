import type { ProcessorOptions } from '../Processor';
import type TypedFile from '../TypedFile';
import { AggregateProcess, Process } from '../Processor';

/**
 * A common structure for output relating to a line in a text document.
 */
export interface LineMessage {
  message: string;
  context?: string;
  filename: string;
  line: number;
  column?: number;
}

/**
 * The common process options that you can pass along when constructing
 * any Process.
 */
export interface ToolchainProcessorOptions extends ProcessorOptions {
  /**
   * A callback triggered to provide information when a line of code causes an error.
   */
  onLineError?: (id: number, info: LineMessage) => void;
  /**
   * A callback triggered to provide information when a line of code causes a warning.
   */
  onLineWarning?: (id: number, info: LineMessage) => void;
}

/**
 * A common process for transforms that deal with the programming compilation pipeline
 * in some way.
 *
 * This includes a lot of common nodes in the compilation pipeline like compilers
 * and assemblers, but also disassemblers.
 */
export abstract class ToolchainProcess<
  IN extends TypedFile,
  OUT extends TypedFile,
> extends Process<IN, OUT, ToolchainProcessorOptions> {}

/**
 * A common process for transforms that deal with the programming compilation pipeline
 * and aggregate a set of data into one new form.
 *
 * This is generally the abstraction for a Linker.
 */
export abstract class AggregateToolchainProcess<
  IN extends TypedFile,
  OUT extends TypedFile,
> extends AggregateProcess<IN, OUT, ToolchainProcessorOptions> {}
