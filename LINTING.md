# Linting

Structured linting for DailyOS frontend. ESLint (TS/TSX) + Stylelint (CSS modules). See Linear DOS-533.

## Why this exists

`tsc --noEmit` catches type errors. Nothing else catches floating promises, stale `useEffect` deps, inline styles, hardcoded hex literals, or circular imports. With AI authoring most code, those classes ship without intervention. Linting is the cheap intervention.

This is not a style enforcement system. There is no Prettier and there will not be one. Lint rules either catch bugs or enforce contracts. If a rule does neither, drop it.

## Running

```bash
pnpm lint           # full repo (eslint + stylelint)
pnpm lint:eslint    # ts/tsx only
pnpm lint:stylelint # css modules only
pnpm lint:fix       # auto-fix what's safely fixable
```

Pre-commit (`.githooks/pre-commit` via lint-staged) lints only changed files — fast.
CI runs full repo lint as a separate fast job (Linux, < 60s).

## Rule tiers

### Tier 1 — bug-catching (error-level)

| Rule | What it catches |
|---|---|
| `@typescript-eslint/no-floating-promises` | Dropped `await` on Tauri `invoke()` — silent failures |
| `@typescript-eslint/no-misused-promises` | `<button onClick={asyncFn}>` swallowing rejections |
| `react-hooks/rules-of-hooks` | Hook ordering bugs |
| `@typescript-eslint/consistent-type-imports` | `import type` discipline |
| `react/forbid-dom-props` (style) | Inline styles — DailyOS cardinal rule |
| `react/forbid-component-props` (style) | Same, for component props |
| `import/no-duplicates` | Duplicate imports after rebase |
| `unused-imports/no-unused-imports` | Dead imports after refactor |
| `no-empty` | `catch {}` swallowing errors |
| `prefer-const` | Reflexive `let` |

### Tier 1 — bug-catching (warn-level)

| Rule | Why warn, not error |
|---|---|
| `react-hooks/exhaustive-deps` | Noisy on first run; promote to error after class sweep |
| `@typescript-eslint/no-unsafe-*` (5 rules) | Forces type guards on `unknown` from `invoke`. Warn during migration; promote per-folder as type discipline catches up |
| `@typescript-eslint/no-explicit-any` | Visible in IDE; AI reaches for `any` reflexively |
| `@typescript-eslint/no-unused-vars` | `_` prefix allowed; respects intentional unused params |
| `import/no-cycle` | Slow rule (max-depth 6); warn keeps it visible without blocking |
| `no-console` | `console.warn` and `console.error` allowed |

### Tier 2 — DailyOS contracts (CSS layer, all error)

Stylelint, applied to `src/**/*.{css,module.css}`:

- `declaration-property-value-allowed-list` — `color`, `background`, `border-color`, `font-family` must use `var(--*)` tokens or CSS keywords.
- `color-no-hex` — belt-and-suspenders ban on hex literals across all properties.
- Source of tokens (`src/styles/design-tokens.css`) is allowlisted.

### Path-scoped overrides

- **Test files** (`*.test.{ts,tsx}`, `*.spec.{ts,tsx}`, `__tests__/**`) — `no-unsafe-*`, `no-explicit-any`, `no-console` all off. Tests routinely poke at unknown JSON shapes and mock factories.
- **`src/services/**`, `src/hooks/**`** — `no-misused-promises` enforced strictly. Floating promises in the data layer lose data; in UI they cause papercuts.

## Escape hatches

### Runtime-computed values (the only legitimate inline-style case)

If a value is genuinely runtime-derived (e.g. computed from a numeric prop), set a CSS custom property via the `style` prop and bind a className that consumes it:

```tsx
// CSS module:
// .progressBar {
//   width: calc(100% * var(--progress, 0));
// }

<div
  className={styles.progressBar}
  style={{ '--progress': progress / 100 } as CSSProperties}
/>
```

Yes, this still trips `react/forbid-dom-props`. Add a one-line disable with a rationale comment:

```tsx
// eslint-disable-next-line react/forbid-dom-props -- runtime --progress passthrough
style={{ '--progress': progress / 100 } as CSSProperties}
```

The rationale comment is mandatory. "Why is this disabled?" is a reviewer question that should be answerable in 5 seconds.

### Per-file disables

Only acceptable for migration. Every disable should have a rationale comment AND a Linear ticket driving it back to compliance. If a disable has been there for 30 days without a ticket, the rule is wrong or the work is undone.

### `as any`

Don't. Reach for type guards (`isFoo(x)` predicates) or `as unknown as Foo` if you genuinely need to bypass. `as any` warns; if it errors a CI run, the fix is "type it properly," not "disable the rule."

## Disciplines

1. **Lints catch bugs OR enforce contracts. Nothing in between.** Stylistic lints become bikesheds.
2. **`error` blocks CI. `warn` is IDE-visible.** Never let warnings accumulate; track them down or promote them.
3. **Allowlists are migration tools, not permanent.** Every allowlisted file gets a Linear ticket. If the list grows, the rule is wrong.
4. **Path-scoped rules.** Different folders, different risk profiles.
5. **Custom rules > grep when AST awareness matters.** Replace `scripts/check_*.sh` shell scripts with ESLint custom rules where the rule needs to walk syntax (e.g. JSXText vocabulary checks).
6. **AI-blind rules win.** A rule that catches a class of AI-author mistake is high-value. A rule catching nothing real is noise — drop it.

## Adding a new rule

1. Justify it: bug class or contract violation. If it's just a preference, no.
2. Land it as `warn` first. Watch the count for a week.
3. If the count is small and tractable, fix the violations and promote to `error`. If the count is huge, file a sweep ticket.
4. Document the rule in this file with a one-line "what it catches."

## Tool notes

- ESLint uses **flat config** (`eslint.config.js`, ESM). Legacy `.eslintrc*` files are not supported here.
- Type-aware rules require `parserOptions.project` — they're slow but they catch the highest-value bugs (no-floating-promises, no-unsafe-*).
- Stylelint deliberately does NOT extend `stylelint-config-standard`. The standard config bundles selector-class-pattern, lowercase-hex, etc. — pure noise for our shape.
- ESLint cache lives at `node_modules/.cache/eslint/`. Stylelint cache at `node_modules/.cache/stylelint/`. Both gitignored via `node_modules/`.

## See also

- `.docs/design/tokens/color.md` — token vocabulary
- `src/styles/design-tokens.css` — token definitions (allowlisted in stylelint)
- `.docs/design/VIOLATIONS.md` — historical drift log
- `CLAUDE.md` cardinal rules — the contract these rules enforce
