# Tokens

Tokens are the lowest-level design decisions. Color values, type ramps, spacing scales, motion curves. Every primitive and pattern consumes tokens — primitives and patterns should *never* hardcode raw values.

## Files

- [`color.md`](./color.md) — palette (paper, desk, spice, garden), semantic colors (text, rule, surface), entity aliases, trust band tokens (proposed), tints, overlays
- [`typography.md`](./typography.md) — font families (serif, sans, mono, mark), type pairings, conventions
- [`spacing.md`](./spacing.md) — base 4px scale, layout tokens, border radii
- [`motion.md`](./motion.md) — transitions, backdrop/glass, shadows, z-index, keyframes

## Conventions

- **Token names are semantic, not visual.** `--color-trust-likely-current`, not `--color-green-500`. (Some legacy tokens are named for paint, e.g. `--color-spice-turmeric`; these are valid where the meaning is the paint color.)
- **Reference values** (the actual hex/px/ms) live in `.docs/design/reference/_shared/tokens.css` (after DS-XCUT-02 lands; currently in `src/styles/design-tokens.css`).
- **Adding a new token** requires answering: is this load-bearing across 3+ entries, or is it a one-off that should live in the consumer? If the latter, don't add a token.
- **Renaming a token** is a coordinated change — markdown spec, runtime CSS, every primitive/pattern that consumes it, and `src/` references. Don't do it casually.

## Source

- **Markdown specs:** this directory (canonical)
- **Runtime CSS (current):** `src/styles/design-tokens.css`
- **Mockup substrate:** `.docs/mockups/claude-design-project/mockups/surfaces/_shared/tokens.css` (will move to `.docs/design/reference/_shared/tokens.css` per DS-XCUT-02)
