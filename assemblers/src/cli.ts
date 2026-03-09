#!/usr/bin/env node

import { parseArgs } from 'node:util';
import { writeFileSync, readFileSync, mkdirSync } from 'node:fs';
import { resolve } from 'node:path';

import { GrammarGenerator } from './generator/GrammarGenerator';
import { EncoderGenerator } from './generator/EncoderGenerator';
import { IntelSyntax } from './generator/syntax/IntelSyntax';
import type { Syntax } from './generator/syntax/Syntax';

const syntaxes: Record<string, () => Syntax> = {
  intel: () => new IntelSyntax(),
};

function usage(): never {
  console.error(`Usage: machinery-asm <command> [options]`);
  console.error();
  console.error(`Commands:`);
  console.error(`  generate  Generate grammar and encoder for a target`);
  console.error(`  assemble  Assemble a source file`);
  console.error();
  console.error(`Generate options:`);
  console.error(`  -t, --target <name>    Target processor (e.g. i286)`);
  console.error(`  -s, --syntax <name>    Assembly syntax (default: intel)`);
  console.error(`  -o, --output <dir>     Output directory`);
  console.error();
  console.error(`Assemble options:`);
  console.error(`  -t, --target <name>    Target processor (e.g. i286)`);
  console.error(`  -s, --syntax <name>    Assembly syntax (default: intel)`);
  console.error(`  -o, --output <file>    Output binary file`);
  console.error(`  <input>                Input assembly file`);
  return process.exit(1);
}

async function generate(args: string[]) {
  const { values } = parseArgs({
    args,
    options: {
      target: { type: 'string', short: 't' },
      syntax: { type: 'string', short: 's', default: 'intel' },
      output: { type: 'string', short: 'o' },
    },
    strict: true,
  });

  const targetName = values.target;
  const syntaxName = values.syntax!;
  const outputDir = values.output;

  if (!targetName || !outputDir) {
    console.error(`Error: --target and --output are required for generate`);
    return usage();
  }

  const syntaxFactory = syntaxes[syntaxName];
  if (!syntaxFactory) {
    console.error(`Unknown syntax: ${syntaxName}. Valid options: ${Object.keys(syntaxes).join(', ')}`);
    return process.exit(1) as never;
  }

  let target;
  try {
    const mod = await import(`@machinery/processors/${targetName}`);
    target = mod.default;
  } catch {
    console.error(
      `Failed to load target "${targetName}". Make sure @machinery/processors/${targetName} exists.`,
    );
    return process.exit(1) as never;
  }

  const syntax = syntaxFactory();
  const outPath = resolve(outputDir);
  mkdirSync(outPath, { recursive: true });

  // Generate grammar
  const grammarGen = new GrammarGenerator(target, syntax);
  const grammarContent = grammarGen.generate();
  const grammarPath = resolve(outPath, 'grammar.ne');
  writeFileSync(grammarPath, grammarContent);
  console.log(`Generated grammar: ${grammarPath}`);

  // Generate encoder
  const encoderGen = new EncoderGenerator(target);
  const encoderContent = encoderGen.generate();
  const encoderPath = resolve(outPath, 'encoder.ts');
  writeFileSync(encoderPath, encoderContent);
  console.log(`Generated encoder: ${encoderPath}`);
}

async function assemble(args: string[]) {
  const { values, positionals } = parseArgs({
    args,
    options: {
      target: { type: 'string', short: 't' },
      syntax: { type: 'string', short: 's', default: 'intel' },
      output: { type: 'string', short: 'o' },
    },
    strict: true,
    allowPositionals: true,
  });

  const targetName = values.target;
  const syntaxName = values.syntax!;
  const outputFile = values.output;
  const inputFile = positionals[0];

  if (!targetName || !outputFile || !inputFile) {
    console.error(`Error: --target, --output, and input file are required for assemble`);
    return usage();
  }

  // Dynamically import the target's grammar and encoder
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  let grammar: any, encoderModule: any;
  try {
    grammar = await import(`./${targetName}/${syntaxName}/grammar`);
    encoderModule = await import(`./${targetName}/${syntaxName}/encoder`);
  } catch {
    console.error(
      `Failed to load grammar/encoder for ${targetName}/${syntaxName}. Run 'machinery-asm generate' first.`,
    );
    return process.exit(1) as never;
  }

  // Set up parser
  const nearley = await import('nearley');

  const source = readFileSync(inputFile, 'utf-8');

  // Import and set up assembler
  const { Assembler } = await import('./runtime/Assembler');
  const assembler = new Assembler(
    {
      parse(input: string) {
        const p = new nearley.default.Parser(
          nearley.default.Grammar.fromCompiled(grammar.default || grammar),
        );
        p.feed(input);
        if (p.results.length === 0) {
          throw new Error('No parse results');
        }
        return p.results[0];
      },
    },
    encoderModule.operandDefinitions,
    encoderModule.encoderForms,
    encoderModule.segmentOverridePrefixes,
  );

  const result = assembler.assemble(source);
  writeFileSync(outputFile, result.binary);
  console.log(
    `Assembled ${inputFile} -> ${outputFile} (${result.binary.length} bytes, origin: 0x${result.origin.toString(16)})`,
  );
}

async function main() {
  const args = process.argv.slice(2);
  const command = args[0];

  if (!command) {
    return usage();
  }

  switch (command) {
    case 'generate':
      await generate(args.slice(1));
      break;
    case 'assemble':
      await assemble(args.slice(1));
      break;
    default:
      console.error(`Unknown command: ${command}`);
      return usage();
  }
}

main();
