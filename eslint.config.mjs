import tseslint from 'typescript-eslint';
import prettier from 'eslint-config-prettier';

export default tseslint.config(
  {
    ignores: [
      '**/dist/',
      '**/node_modules/',
      '**/.turbo/',
      '**/coverage/',
      '**/grammar.ts',
      '**/Machine.ts',
      'assemblers/src/i286/',
      '**/*.inc',
      '**/*.asm',
      '**/*.dsm',
      '**/*.com',
      '**/*.hex',
    ],
  },
  ...tseslint.configs.recommended,
  prettier,
  {
    rules: {
      // Allow underscore-prefixed unused vars (common pattern in this codebase)
      '@typescript-eslint/no-unused-vars': [
        'error',
        {
          argsIgnorePattern: '^_',
          varsIgnorePattern: '^_',
        },
      ],
      // Prefer const objects with 'as const' over enums
      'no-restricted-syntax': [
        'error',
        {
          selector: 'TSEnumDeclaration',
          message:
            'Use `const X = { ... } as const` with a derived type instead of enum.',
        },
      ],
    },
  },
);
