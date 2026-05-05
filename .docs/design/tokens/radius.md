# Radius tokens

**Tier:** tokens
**Status:** canonical
**Owner:** James
**Last updated:** 2026-05-05
**`data-ds-name`:** `RadiusTokens`
**`data-ds-spec`:** `tokens/radius.md`
**Design system version introduced:** 0.4.0

## Job

Radius tokens define the restrained editorial rounding used across DailyOS. They keep controls, chips, cards, navigation, and overlays from drifting into one-off corner values.

## Tokens

- `--radius-editorial-sm` - compact controls and small inline affordances.
- `--radius-editorial-md` - nav items, small panels, and grouped controls.
- `--radius-editorial-lg` - larger repeated items and feature rows.
- `--radius-editorial-xl` - top-level floating chrome and larger overlays.

## Usage

Prefer the semantic editorial radius tokens over raw `px` values. New components should pick the smallest radius that still matches the interaction surface.

## Source

- **Runtime CSS:** `src/styles/design-tokens.css`
- **Reference CSS export:** `.docs/design/reference/_shared/styles/design-tokens.css`
- **Legacy grouping:** these tokens were previously documented inside `spacing.md`.
