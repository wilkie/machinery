#!/usr/bin/env node

import { parseArgs } from 'node:util';
import { writeFileSync } from 'node:fs';

import { targets } from './targets';

function usage(): never {
  console.error(`Usage: machinery-asm [options] <input>`);
  console.error();
  console.error(`Options:`);
  console.error(`  -t, --target <name>    Target processor (e.g. i286)`);
  console.error(`  -s, --syntax <name>    Assembly syntax (default: intel)`);
  console.error(`  -o, --output <file>    Output binary file`);
  console.error(`  <input>                Input assembly file`);
  console.error();
  console.error(
    `Available targets: ${Object.keys(targets).join(', ') || '(none — run generate first)'}`,
  );
  return process.exit(1);
}

async function main() {
  const { values, positionals } = parseArgs({
    args: process.argv.slice(2),
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
    console.error(`Error: --target, --output, and input file are required`);
    return usage();
  }

  const key = `${targetName}/${syntaxName}`;
  const targetLoader = targets[key];
  if (!targetLoader) {
    console.error(
      `Unknown target "${key}". Available: ${Object.keys(targets).join(', ') || '(none — run generate first)'}`,
    );
    return process.exit(1) as never;
  }

  const { assemble: assembleFile } = await targetLoader();
  const result = assembleFile(inputFile);
  writeFileSync(outputFile, result.binary);
  console.log(
    `Assembled ${inputFile} -> ${outputFile} (${result.binary.length} bytes, origin: 0x${result.origin.toString(16)})`,
  );
}

main();
