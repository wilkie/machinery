#!/usr/bin/env node

import { parseArgs } from 'node:util';
import { readFileSync } from 'node:fs';
import { resolve, basename } from 'node:path';

import { generateDecoderMap } from '@machinery/core';
import type { Target } from '@machinery/core';

import { Decoder } from './runtime/Decoder';
import { IntelSyntax } from './runtime/IntelSyntax';
import { AttSyntax } from './runtime/AttSyntax';
import type { Syntax } from './runtime/Syntax';

const syntaxes: Record<string, () => Syntax> = {
  intel: () => new IntelSyntax(),
  att: () => new AttSyntax(),
};

const targets: Record<string, () => Promise<Target>> = {
  i286: async () => {
    const mod = await import('@machinery/processors/i286');
    return mod.default;
  },
};

function usage(): never {
  console.error(`Usage: machinery-disasm [options] <input>`);
  console.error();
  console.error(`Options:`);
  console.error(`  -t, --target <name>    Target processor (e.g. i286)`);
  console.error(`  -s, --syntax <name>    Output syntax (default: intel)`);
  console.error(`  -o, --origin <addr>    Origin address (default: 0)`);
  console.error();
  console.error(
    `Available targets: ${Object.keys(targets).join(', ') || '(none)'}`,
  );
  console.error(`Available syntaxes: ${Object.keys(syntaxes).join(', ')}`);
  return process.exit(1);
}

async function main() {
  const { values, positionals } = parseArgs({
    options: {
      target: { type: 'string', short: 't' },
      syntax: { type: 'string', short: 's', default: 'intel' },
      origin: { type: 'string', short: 'o', default: '0' },
    },
    strict: true,
    allowPositionals: true,
  });

  const targetName = values.target;
  const syntaxName = values.syntax!;
  const origin = parseInt(values.origin!, 0);
  const inputFile = positionals[0];

  if (!targetName || !inputFile) {
    console.error(`Error: --target and input file are required`);
    return usage();
  }

  const targetLoader = targets[targetName];
  if (!targetLoader) {
    console.error(
      `Unknown target: ${targetName}. Available: ${Object.keys(targets).join(', ')}`,
    );
    return process.exit(1) as never;
  }

  const syntaxFactory = syntaxes[syntaxName];
  if (!syntaxFactory) {
    console.error(
      `Unknown syntax: ${syntaxName}. Available: ${Object.keys(syntaxes).join(', ')}`,
    );
    return process.exit(1) as never;
  }

  const target = await targetLoader();
  const syntax = syntaxFactory();

  // Build decoder maps from processor metadata
  const decoderMap = generateDecoderMap(target, false);
  const prefixMap = generateDecoderMap(target, true);

  // Read input binary
  const absInput = resolve(inputFile);
  const data = new Uint8Array(readFileSync(absInput));

  // Print header (mimicking objdump style)
  console.log();
  console.log(`${basename(absInput)}:     file format binary`);
  console.log();
  console.log();
  console.log(`Disassembly of section .data:`);
  console.log();
  console.log(`${origin.toString(16).padStart(8, '0')} <.data>:`);

  // Decode and print
  const decoder = new Decoder(data, decoderMap, prefixMap, target);
  while (decoder.hasMore()) {
    const instr = decoder.decode();
    if (!instr) break;

    const addr = (instr.address + origin).toString(16).padStart(4, ' ');

    const hexBytes = Array.from(instr.bytes)
      .map((b) => b.toString(16).padStart(2, '0'))
      .join(' ');

    const asmText = syntax.formatInstruction(instr);

    // Format: address: hex_bytes \t mnemonic operands
    console.log(`${addr}:\t${hexBytes.padEnd(24)}\t${asmText}`);
  }
}

main();
