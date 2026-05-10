// Flat config (ESM). See LINTING.md for the rule-tier rationale and DOS-533.
import { readFileSync } from 'node:fs';
import tseslint from 'typescript-eslint';
import react from 'eslint-plugin-react';
import reactHooks from 'eslint-plugin-react-hooks';
import importPlugin from 'eslint-plugin-import';
import unusedImports from 'eslint-plugin-unused-imports';

// Baseline allowlist. Each rule maps to the files that already violate it at
// the time the lint config landed. Existing offenders get the rule disabled
// per-file so new files are blocked while the sweep tickets pay down the
// backlog. Drive these lists to empty; do not grow them.
//
// Owners: react/forbid-dom-props + forbid-component-props → DOS-526 epic.
//         no-floating-promises + no-misused-promises → file a sweep ticket.
const baseline = JSON.parse(
  readFileSync(new URL('./.eslint-baseline.json', import.meta.url), 'utf-8'),
);

export default tseslint.config(
  {
    ignores: [
      'dist/**',
      'node_modules/**',
      'src-tauri/target/**',
      'src-tauri/binaries/**',
      '.docs/design/reference/**',
      '.docs/_archive/**',
      '.claude/**',
      '.codex/**',
      '*.config.js',
      '*.config.ts',
      'vite.config.*',
      'vitest.config.*',
      'eslint.config.js',
      'stylelint.config.js',
      // Duplicate hook file (.ts and .tsx coexist with the same basename).
      // Confuses TS project resolution. Deduplicating is its own ticket.
      'src/hooks/useActivePreset.tsx',
    ],
  },

  // Base — TS-aware linting only on real source.
  ...tseslint.configs.recommendedTypeChecked,

  {
    files: ['src/**/*.{ts,tsx}'],
    plugins: {
      react,
      'react-hooks': reactHooks,
      import: importPlugin,
      'unused-imports': unusedImports,
    },
    languageOptions: {
      parserOptions: {
        project: './tsconfig.json',
        tsconfigRootDir: import.meta.dirname,
      },
    },
    settings: {
      react: { version: '18' },
      'import/resolver': {
        typescript: { project: './tsconfig.json' },
        node: true,
      },
    },
    rules: {
      // ─── Tier 1: bug-catching (DOS-533) ────────────────────────────────
      '@typescript-eslint/no-floating-promises': 'error',
      '@typescript-eslint/no-misused-promises': [
        'error',
        { checksVoidReturn: { attributes: false } },
      ],
      'react-hooks/rules-of-hooks': 'error',
      'react-hooks/exhaustive-deps': 'warn',
      '@typescript-eslint/no-unsafe-assignment': 'warn',
      '@typescript-eslint/no-unsafe-call': 'warn',
      '@typescript-eslint/no-unsafe-member-access': 'warn',
      '@typescript-eslint/no-unsafe-argument': 'warn',
      '@typescript-eslint/no-unsafe-return': 'warn',
      '@typescript-eslint/no-explicit-any': 'warn',
      '@typescript-eslint/consistent-type-imports': [
        'error',
        { prefer: 'type-imports', fixStyle: 'inline-type-imports' },
      ],

      // Demoted to warn — these are code-quality nice-to-haves rather than
      // active bug classes. Visible in IDE without blocking CI. Promote per
      // rule once the codebase is clean.
      '@typescript-eslint/no-unnecessary-type-assertion': 'warn',
      '@typescript-eslint/restrict-template-expressions': 'warn',
      '@typescript-eslint/no-base-to-string': 'warn',
      '@typescript-eslint/require-await': 'warn',
      '@typescript-eslint/no-redundant-type-constituents': 'warn',
      '@typescript-eslint/await-thenable': 'warn',
      '@typescript-eslint/no-unused-expressions': 'warn',

      // Cardinal rule — no inline styles. Folds in DOS-526 / DOS-532.
      // Runtime-computed values: use a CSS custom property bound via className.
      'react/forbid-dom-props': [
        'error',
        {
          forbid: [
            {
              propName: 'style',
              message:
                'Use a CSS module class (cardinal rule). Runtime values: pass a CSS custom property via className.',
            },
          ],
        },
      ],
      'react/forbid-component-props': [
        'error',
        {
          forbid: [
            {
              propName: 'style',
              message: 'Pass className, not style.',
            },
          ],
        },
      ],

      // ─── Tier 3: hygiene ────────────────────────────────────────────────
      'import/no-cycle': ['warn', { maxDepth: 6 }],
      'import/no-duplicates': 'error',
      'unused-imports/no-unused-imports': 'error',
      '@typescript-eslint/no-unused-vars': [
        'warn',
        {
          argsIgnorePattern: '^_',
          varsIgnorePattern: '^_',
          caughtErrorsIgnorePattern: '^_',
        },
      ],
      'no-empty': ['error', { allowEmptyCatch: false }],
      'no-console': ['warn', { allow: ['warn', 'error'] }],
      'prefer-const': 'error',

      // Disable defaults that conflict with TS-aware rules above.
      'no-unused-vars': 'off',
    },
  },

  // Test files: relax the unsafe-* rules; tests routinely interact with
  // unknown JSON shapes and mock factories.
  {
    files: ['src/**/*.{test,spec}.{ts,tsx}', 'src/**/__tests__/**/*.{ts,tsx}'],
    rules: {
      '@typescript-eslint/no-unsafe-assignment': 'off',
      '@typescript-eslint/no-unsafe-call': 'off',
      '@typescript-eslint/no-unsafe-member-access': 'off',
      '@typescript-eslint/no-unsafe-argument': 'off',
      '@typescript-eslint/no-unsafe-return': 'off',
      '@typescript-eslint/no-explicit-any': 'off',
      'no-console': 'off',
    },
  },

  // Service layer: stricter promise discipline. Floating promises here are
  // bugs that lose data, not UI papercuts.
  {
    files: ['src/services/**/*.{ts,tsx}', 'src/hooks/**/*.{ts,tsx}'],
    rules: {
      '@typescript-eslint/no-misused-promises': 'error',
    },
  },

  // ─── Baseline allowlists ─────────────────────────────────────────────
  // Each block disables one rule for the files that already violate it.
  // These lists are paid down by sweep tickets, not appended to.
  {
    files: baseline['react/forbid-dom-props'],
    rules: { 'react/forbid-dom-props': 'off' },
  },
  {
    files: baseline['react/forbid-component-props'],
    rules: { 'react/forbid-component-props': 'off' },
  },
  {
    files: baseline['@typescript-eslint/no-floating-promises'],
    rules: { '@typescript-eslint/no-floating-promises': 'off' },
  },
  {
    files: baseline['@typescript-eslint/no-misused-promises'],
    rules: { '@typescript-eslint/no-misused-promises': 'off' },
  },
);
