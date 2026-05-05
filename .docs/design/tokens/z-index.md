# Z-index tokens

**Tier:** tokens
**Status:** canonical
**Owner:** James
**Last updated:** 2026-05-05
**`data-ds-name`:** `ZIndexTokens`
**`data-ds-spec`:** `tokens/z-index.md`
**Design system version introduced:** 0.4.0

## Job

Z-index tokens define the shared layer stack. They prevent magic numbers and keep atmosphere, content, app chrome, and lock overlays in predictable order.

## Tokens

- `--z-atmosphere` - background AtmosphereLayer.
- `--z-page-content` - standard page content above atmosphere.
- `--z-app-shell` - FolioBar, FloatingNavIsland, and top-level persistent chrome.
- `--z-lock` - app lock overlay above every other layer.

## Usage

Use these tokens anywhere a component needs explicit stacking. New z-index values require a documented layer-system reason.

## Source

- **Runtime CSS:** `src/styles/design-tokens.css`
- **Reference CSS export:** `.docs/design/reference/_shared/styles/design-tokens.css`
- **Legacy grouping:** these tokens were previously documented inside `motion.md`.
