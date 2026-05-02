# Tokens

Tokens are the lowest-level design decisions. Color values, type ramps, spacing scales, motion curves. Every primitive and pattern consumes tokens — primitives and patterns should *never* hardcode raw values.

## Files

- `color.md` — palette, semantic color roles, trust-band colors, surface backgrounds, text contrast pairs
- `typography.md` — font families, type scale, line-height pairs, weight roles, headline vs body conventions
- `spacing.md` — base unit, scale, layout grid, component padding conventions
- `motion.md` — durations, easing curves, transition roles (enter, exit, hover, state-change)

## Conventions

- **Token names are semantic, not visual.** `--color-trust-likely-current`, not `--color-green-500`.
- **Reference values** (the actual hex/px/ms) live in `reference/_shared/tokens.css`. Markdown specs name and explain; CSS is the runtime artifact.
- **Adding a new token** requires answering: is this load-bearing across 3+ entries, or is it a one-off that should live in the consumer? If the latter, don't add a token.
- **Renaming a token** is a coordinated change — markdown spec, `tokens.css`, every primitive/pattern that consumes it, and `src/` references. Don't do it casually.

## Source

- **Markdown specs:** this directory
- **Runtime CSS:** `../reference/_shared/tokens.css` (when populated)
- **App code:** to be inventoried by Audit 01
