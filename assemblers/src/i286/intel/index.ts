import { readFileSync } from 'fs';
import { dirname, resolve } from 'path';
import nearley from 'nearley';

import { Assembler } from '../../runtime/Assembler';
import type { AssembleResult } from '../../runtime/Assembler';
import { Preprocessor } from '../../preprocessor/Preprocessor';
import {
  operandDefinitions,
  encoderForms,
  segmentOverridePrefixes,
  prefixBytes,
  endianness,
  alignmentFill,
} from './encoder';
import grammar from './grammar';

export { operandDefinitions, encoderForms, segmentOverridePrefixes, prefixBytes, endianness, alignmentFill };
export type { EncoderField, EncoderMatcher, OpcodeEntry, EncoderForm } from './encoder';

/**
 * Assemble an i286 Intel-syntax assembly file to a flat binary.
 */
export function assemble(inputPath: string): AssembleResult {
  const absInput = resolve(inputPath);
  const inputDir = dirname(absInput);

  const rawSource = readFileSync(absInput, 'utf-8');
  const preprocessor = new Preprocessor({
    includePaths: [inputDir],
    readFile: (path: string) => readFileSync(path, 'utf-8'),
  });
  const source = preprocessor.process(rawSource, absInput);

  const assembler = new Assembler(
    {
      parse(input: string) {
        const p = new nearley.Parser(
          nearley.Grammar.fromCompiled(grammar),
        );
        p.feed(input);
        if (p.results.length === 0) {
          throw new Error('No parse results');
        }
        return p.results[0];
      },
    },
    operandDefinitions,
    encoderForms,
    segmentOverridePrefixes,
    prefixBytes,
    endianness,
    alignmentFill,
  );

  return assembler.assemble(source);
}
