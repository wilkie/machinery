import { defineConfig } from 'tsup';

export default defineConfig([
  {
    entry: [
      'src/index.ts',
      'src/z80/intel/index.ts',
      'src/i286/intel/index.ts',
    ],
    format: ['esm', 'cjs'],
    dts: true,
    sourcemap: true,
    clean: true,
  },
  {
    entry: ['src/cli.ts', 'src/generate.ts'],
    format: ['esm'],
    sourcemap: true,
  },
]);
