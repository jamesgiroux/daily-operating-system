# Spacing tokens

**Tier:** tokens
**Status:** canonical
**Owner:** James
**Last updated:** 2026-05-02
**Design system version introduced:** 0.1.0

## Job

DailyOS spacing scale, base 4px grid. Set by ADR-0073.

## Scale

- `--space-xs`  `4px`
- `--space-sm`  `8px`
- `--space-md`  `16px`
- `--space-lg`  `24px`
- `--space-xl`  `32px`
- `--space-2xl` `48px`
- `--space-3xl` `56px`
- `--space-4xl` `72px`
- `--space-5xl` `80px`

## Layout tokens (fixed dimensions for app shell)

- `--folio-height` `40px` — FolioBar fixed height
- `--folio-padding-top` `10px`
- `--folio-padding-bottom` `10px`
- `--folio-padding-left` `80px`
- `--folio-padding-right` `48px`
- `--page-padding-horizontal` `120px` — left/right page padding
- `--page-padding-bottom` `160px`
- `--page-margin-top` `40px` — clearance for fixed FolioBar
- `--page-max-width` `1180px`
- `--page-content-width-standard` `900px`
- `--page-content-width-reading` `760px`
- `--nav-island-right` `28px` — distance from right edge

## Border radius

- `--radius-editorial-sm` `4px`  — search button, small elements
- `--radius-editorial-md` `10px` — nav island items
- `--radius-editorial-lg` `12px` — featured action boxes
- `--radius-editorial-xl` `16px` — nav island container

## When to use which

- **xs / sm** — gaps within tight clusters: pill internal padding, icon + label gaps, related metadata pairs
- **md** — standard card padding, related-element gaps, list-row gaps
- **lg** — section internal padding, between distinct content blocks
- **xl** — between major sub-sections within a chapter
- **2xl-5xl** — between chapters / major sections, hero padding, before-finis spacing

## Conventions

- Stick to the scale; arbitrary px values are a smell.
- Use `gap` (flex/grid) over margin where layout permits.
- Vertical rhythm: chapters separated by `--space-2xl` to `--space-3xl`; finis separated by `--space-4xl`.
- Page max width: most editorial surfaces use `--page-max-width` (1180px); long-form prose uses `--page-content-width-reading` (760px).

## Source

- **Code:** `src/styles/design-tokens.css`
- **Mockup substrate:** `.docs/mockups/claude-design-project/mockups/surfaces/_shared/tokens.css`

## History

- 2026-05-02 — Promoted to canonical.
- ADR-0073 — original spacing scale definition.
