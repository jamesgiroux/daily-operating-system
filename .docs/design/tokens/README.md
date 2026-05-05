# Tokens

Tokens are the lowest-level design decisions. Color values, type ramps, spacing scales, motion curves. Every primitive and pattern consumes tokens — primitives and patterns should *never* hardcode raw values.

## Files

- [`color.md`](./color.md) — palette (paper, desk, spice, garden), semantic colors (text, rule, surface), entity aliases, shipped trust band tokens, tints, overlays
- [`typography.md`](./typography.md) — font families (serif, sans, mono, mark), type pairings, conventions
- [`spacing.md`](./spacing.md) — base 4px scale, layout tokens, border radii
- [`motion.md`](./motion.md) — transitions, backdrop/glass, shadows, z-index, keyframes
- [`layout.md`](./layout.md) — app shell dimensions, page widths, fixed chrome clearance
- [`radius.md`](./radius.md) — editorial radius scale
- [`shadows.md`](./shadows.md) — elevation and overlay shadows
- [`glass.md`](./glass.md) — frosted chrome/backdrop tokens
- [`z-index.md`](./z-index.md) — shared layer stack

## Conventions

- **Token names are semantic, not visual.** `--color-trust-likely-current`, not `--color-green-500`. (Some legacy tokens are named for paint, e.g. `--color-spice-turmeric`; these are valid where the meaning is the paint color.)
- **Reference values** (the actual hex/px/ms) live in `src/styles/design-tokens.css`; `.docs/design/reference/_shared/styles/design-tokens.css` is the mirrored reference export. `.docs/design/reference/_shared/tokens.css` is only a compatibility entrypoint that delegates to the mirrored file.
- **Adding a new token** requires answering: is this load-bearing across 3+ entries, or is it a one-off that should live in the consumer? If the latter, don't add a token.
- **Renaming a token** is a coordinated change — markdown spec, runtime CSS, every primitive/pattern that consumes it, and `src/` references. Don't do it casually.

## Source

- **Markdown specs:** this directory (canonical)
- **Runtime CSS (current):** `src/styles/design-tokens.css`
- **Reference CSS export:** `.docs/design/reference/_shared/styles/design-tokens.css`
