# Layout tokens

**Tier:** tokens
**Status:** canonical
**Owner:** James
**Last updated:** 2026-05-05
**`data-ds-name`:** `LayoutTokens`
**`data-ds-spec`:** `tokens/layout.md`
**Design system version introduced:** 0.4.0

## Job

DailyOS layout tokens define the app shell's fixed dimensions and page-width contracts. They prevent each surface from inventing its own chrome clearance, content width, or page padding.

## Tokens

- `--folio-height` - fixed FolioBar height.
- `--folio-padding-top`, `--folio-padding-bottom`, `--folio-padding-left`, `--folio-padding-right` - FolioBar internal geometry.
- `--page-padding-horizontal`, `--page-padding-bottom`, `--page-margin-top` - default editorial page spacing.
- `--page-max-width` - maximum surface width.
- `--page-content-width-standard` - standard reading/content column.
- `--page-content-width-reading` - long-form reading column.
- `--nav-island-right` - FloatingNavIsland right inset.

## Usage

Use these tokens in shell, page, masthead, and editorial layout CSS. Do not hardcode page-width or fixed-chrome clearance values in surface modules unless the surface has a documented exception.

## Source

- **Runtime CSS:** `src/styles/design-tokens.css`
- **Reference CSS export:** `.docs/design/reference/_shared/styles/design-tokens.css`
- **Legacy grouping:** these tokens were previously documented inside `spacing.md`.
