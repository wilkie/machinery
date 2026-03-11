#!/usr/bin/env node

import { parseArgs } from 'node:util';
import { writeFileSync, mkdirSync, existsSync, readdirSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

import { GrammarGenerator } from './generator/GrammarGenerator';
import { EncoderGenerator } from './generator/EncoderGenerator';
import { IntelSyntax } from './generator/syntax/IntelSyntax';
import type { Syntax } from './generator/syntax/Syntax';
import { targets } from './targets';

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
  console.error();
  console.error(
    `Available targets: ${Object.keys(targets).join(', ') || '(none — run generate first)'}`,
  );
  return process.exit(1);
}

function generateTargetsRegistry(srcDir: string) {
  const targetsPath = resolve(srcDir, 'targets.ts');
  const lines: string[] = [];
  lines.push(
    `// Generated target registry — updated by \`machinery-asm generate\``,
  );
  lines.push(`// Do not edit manually`);
  lines.push(``);

  // Scan for target/syntax directories with an index.ts
  const entries: { key: string; importPath: string }[] = [];

  for (const targetDir of readdirSync(srcDir, { withFileTypes: true })) {
    if (!targetDir.isDirectory()) continue;
    // Skip non-target directories
    if (['generator', 'preprocessor', 'runtime'].includes(targetDir.name))
      continue;

    const targetPath = resolve(srcDir, targetDir.name);
    for (const syntaxDir of readdirSync(targetPath, { withFileTypes: true })) {
      if (!syntaxDir.isDirectory()) continue;

      const indexPath = resolve(targetPath, syntaxDir.name, 'index.ts');
      if (!existsSync(indexPath)) continue;

      const key = `${targetDir.name}/${syntaxDir.name}`;
      const importPath = `./${targetDir.name}/${syntaxDir.name}`;
      entries.push({ key, importPath });
    }
  }

  lines.push(``);
  lines.push(
    `export const targets: Record<string, () => Promise<{ assemble: (inputPath: string) => { binary: Uint8Array; origin: number } }>> = {`,
  );
  for (const { key, importPath } of entries) {
    lines.push(`  '${key}': () => import('${importPath}'),`);
  }
  lines.push(`};`);
  lines.push(``);

  writeFileSync(targetsPath, lines.join('\n'));
  console.log(`Updated target registry: ${targetsPath}`);
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

  // Regenerate the target registry
  const cliDir =
    typeof __dirname !== 'undefined'
      ? __dirname
      : dirname(fileURLToPath(import.meta.url));
  generateTargetsRegistry(resolve(cliDir));
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
    console.error(
      `Error: --target, --output, and input file are required for assemble`,
    );
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
      assemble(args.slice(1));
      break;
    default:
      console.error(`Unknown command: ${command}`);
      return usage();
  }
}

main();
