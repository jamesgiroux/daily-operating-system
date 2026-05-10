// Stylelint config for DailyOS CSS modules. Token-only enforcement.
// See LINTING.md and DOS-533.
//
// Intentionally NOT extending stylelint-config-standard — its defaults
// include noisy stylistic rules (selector-class-pattern, lowercase-hex)
// that are pure overhead. Only enable rules that catch contract violations.
import { readFileSync } from 'node:fs';

// Baseline allowlist — files that already use hex literals at the time the
// rule landed. Owned by DOS-523 (hex sweep). Drive to empty; do not grow.
const baseline = JSON.parse(
  readFileSync(new URL('./.stylelint-baseline.json', import.meta.url), 'utf-8'),
);

/** @type {import('stylelint').Config} */
export default {
  ignoreFiles: [
    'node_modules/**',
    'dist/**',
    'src-tauri/target/**',
    '.docs/design/reference/**',
    '.docs/_archive/**',
    '.claude/**',
    '.codex/**',
    // Token source — hex literals belong here.
    'src/styles/design-tokens.css',
  ],
  rules: {
    // Force tokens for color-bearing properties. Allowlist accepts var(--*),
    // CSS keywords (transparent, currentColor, inherit, none), and 0
    // for shorthands. Plain hex / rgb literals fail.
    'declaration-property-value-allowed-list': [
      {
        '/^color$/': [
          '/^var\\(--/',
          'transparent',
          'currentColor',
          'inherit',
          'unset',
          'initial',
        ],
        '/^background(-color)?$/': [
          '/^var\\(--/',
          'transparent',
          'currentColor',
          'inherit',
          'unset',
          'initial',
          'none',
          '/^linear-gradient\\(/',
          '/^radial-gradient\\(/',
        ],
        '/^border(-(top|right|bottom|left))?-color$/': [
          '/^var\\(--/',
          'transparent',
          'currentColor',
          'inherit',
          'unset',
          'initial',
        ],
        '/^font-family$/': [
          '/^var\\(--/',
          'inherit',
          'unset',
          'initial',
        ],
      },
      {
        message: (selector) =>
          `Use a design token (var(--*)) for ${selector}. See src/styles/design-tokens.css.`,
        severity: 'error',
      },
    ],

    // Belt-and-suspenders: explicitly ban hex literals in any property.
    // Catches box-shadow, outline-color, etc. that aren't covered above.
    'color-no-hex': [
      true,
      {
        message:
          'Hex literals are banned. Use design tokens from src/styles/design-tokens.css.',
        severity: 'error',
      },
    ],
  },
  overrides: [
    {
      files: baseline,
      rules: {
        'declaration-property-value-allowed-list': null,
        'color-no-hex': null,
      },
    },
  ],
};
