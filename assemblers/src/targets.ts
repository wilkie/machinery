// Generated target registry — updated by `generate-targets`
// Do not edit manually

export const targets: Record<
  string,
  () => Promise<{
    assemble: (inputPath: string) => { binary: Uint8Array; origin: number };
  }>
> = {
  'i286/intel': () => import('./i286/intel'),
  'z80/intel': () => import('./z80/intel'),
};
