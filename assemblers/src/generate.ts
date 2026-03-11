#!/usr/bin/env node

import { parseArgs } from 'node:util';
import { writeFileSync, mkdirSync } from 'node:fs';
import { resolve } from 'node:path';

import { GrammarGenerator } from './generator/GrammarGenerator';
import { EncoderGenerator } from './generator/EncoderGenerator';
import { IntelSyntax } from './generator/syntax/IntelSyntax';
import type { Syntax } from './generator/syntax/Syntax';

const syntaxes: Record<string, () => Syntax> = {
  intel: () => new IntelSyntax(),
};

function usage(): never {
  console.error(`Usage: machinery-generate [options]`);
  console.error();
  console.error(`Options:`);
  console.error(`  -t, --target <name>    Target processor (e.g. i286)`);
  console.error(`  -s, --syntax <name>    Assembly syntax (default: intel)`);
  console.error(`  -o, --output <dir>     Output directory`);
  return process.exit(1);
}

async function main() {
  const { values } = parseArgs({
    args: process.argv.slice(2),
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
    console.error(`Error: --target and --output are required`);
    return usage();
  }

  const syntaxFactory = syntaxes[syntaxName];
  if (!syntaxFactory) {
    console.error(
      `Unknown syntax: ${syntaxName}. Valid options: ${Object.keys(syntaxes).join(', ')}`,
    );
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

main();
