#!/usr/bin/env node

/**
 * Scans for target/syntax directories and regenerates the targets.ts registry.
 * Run as part of the build process.
 */

import { writeFileSync, existsSync, readdirSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const srcDir =
  typeof __dirname !== 'undefined'
    ? __dirname
    : dirname(fileURLToPath(import.meta.url));

const targetsPath = resolve(srcDir, 'targets.ts');
const lines: string[] = [];
lines.push(`// Generated target registry — updated by \`generate-targets\``);
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
