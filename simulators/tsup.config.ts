import { defineConfig } from 'tsup';
import { copyFileSync, mkdirSync } from 'fs';

export default defineConfig({
  entry: ['src/index.ts', 'src/i286/index.ts', 'src/z80/index.ts'],
  format: ['esm', 'cjs'],
  dts: true,
  sourcemap: true,
  clean: true,
  onSuccess: async () => {
    // Copy .wasm files to dist
    mkdirSync('dist/i286', { recursive: true });
    mkdirSync('dist/z80', { recursive: true });
    copyFileSync('src/i286/machine.wasm', 'dist/i286/machine.wasm');
    copyFileSync('src/z80/machine.wasm', 'dist/z80/machine.wasm');
  },
});
