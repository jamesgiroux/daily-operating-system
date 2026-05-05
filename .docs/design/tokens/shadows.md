# Shadow tokens

**Tier:** tokens
**Status:** canonical
**Owner:** James
**Last updated:** 2026-05-05
**`data-ds-name`:** `ShadowTokens`
**`data-ds-spec`:** `tokens/shadows.md`
**Design system version introduced:** 0.4.0

## Job

Shadow tokens define DailyOS elevation. Shadows are quiet and functional: they separate overlays, dropdowns, and floating chrome without becoming decorative.

## Tokens

- `--shadow-sm` - subtle hover or small-surface lift.
- `--shadow-md` - low panel elevation with a faint boundary.
- `--shadow-lg`, `--shadow-xl`, `--shadow-2xl` - progressively stronger overlay elevation.
- `--shadow-modal` - modal/dialog elevation.
- `--shadow-dropdown` - menus, pickers, and transient lists.

## Usage

Use the semantic token that matches the component role. Avoid raw `box-shadow` values where one of these tokens applies.

## Source

- **Runtime CSS:** `src/styles/design-tokens.css`
- **Reference CSS export:** `.docs/design/reference/_shared/styles/design-tokens.css`
- **Legacy grouping:** these tokens were previously documented inside `motion.md`.
