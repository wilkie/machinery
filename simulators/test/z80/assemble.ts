#!/usr/bin/env npx tsx
/**
 * Assembles all .asm test files into flat binaries using @machinery/assemblers.
 *
 * Usage: npx tsx test/z80/assemble.ts
 */

import { readdirSync, writeFileSync, existsSync, mkdirSync } from 'fs';
import { resolve, basename } from 'path';
import { assemble } from '@machinery/assemblers/z80/intel';

const testRoot = resolve(import.meta.dirname);

const dirs = [resolve(testRoot, 'simple')];

let assembled = 0;
let failed = 0;

for (const dir of dirs) {
  if (!existsSync(dir)) continue;

  const binDir = resolve(dir, 'bin');
  mkdirSync(binDir, { recursive: true });

  const asmFiles = readdirSync(dir).filter((f) => f.endsWith('.asm'));
  for (const file of asmFiles) {
    const inputPath = resolve(dir, file);
    const outputPath = resolve(binDir, basename(file, '.asm') + '.bin');

    try {
      const result = assemble(inputPath);
      writeFileSync(outputPath, result.binary);
      assembled++;
    } catch (err: unknown) {
      failed++;
      const message = err instanceof Error ? err.message : String(err);
      console.error(`FAIL: ${file}: ${message}`);
    }
  }
}

console.log(
  `Assembled ${assembled} files${failed > 0 ? `, ${failed} failed` : ''}`,
);
if (failed > 0) process.exit(1);
