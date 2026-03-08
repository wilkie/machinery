import { defineConfig } from 'tsup';

export default defineConfig({
  entry: ['src/index.ts', 'src/NodeDrive.ts', 'src/ZippedDrive.ts'],
  format: ['esm', 'cjs'],
  dts: true,
  sourcemap: true,
  clean: true,
  external: ['node:fs', '@zip.js/zip.js'],
});
