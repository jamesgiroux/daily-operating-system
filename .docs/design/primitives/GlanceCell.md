# GlanceCell

**Tier:** primitive
**Status:** proposed
**Owner:** James
**Last updated:** 2026-05-03
**`data-ds-name`:** `GlanceCell`
**`data-ds-spec`:** `primitives/GlanceCell.md`
**Variants:** `status="none" | "healthy" | "warn"`
**Design system version introduced:** 0.3.0

## Job

GlanceCell presents one compact key/value stat for a masthead glance row. It gives users a quick read on a surface-level metric and can add a small status dot when the value needs health or warning context.

## When to use it

- One key/value stat inside `GlanceRow`.
- Surface mastheads that need fast system, account, quota, or activity context.
- Values where a small dot can clarify healthy or warning status.
- Short labels and values that remain scannable as a row.

## When NOT to use it

- Full analytics cards, charts, or trends; use a metrics pattern.
- Interactive controls; use Button, `Switch`, `Segmented`, or `InlineInput`.
- Long explanatory values; use a detail row or section content.
- Standalone status labels; use `Pill` or a named status primitive.

## States / variants

- `status="none"` — value renders without a dot.
- `status="healthy"` — value includes a leading healthy dot for steady states.
- `status="warn"` — value includes a leading warning dot for thresholds or recovered anomalies.
- `default` — key uses muted small text; value uses stronger text and compact stat rhythm.
- `responsive` — cell keeps stable min width inside `GlanceRow` and wraps only at row breakpoints.

## Tokens consumed

- `.docs/design/tokens/color.md` — muted key text, value text, healthy dot, warning dot, cell divider or boundary.
- `.docs/design/tokens/typography.md` — key/value stat typography via `--font-sans`.
- `.docs/design/tokens/spacing.md` — cell padding, key/value gap, row gap.
- `--color-garden-sage-15`, `--color-spice-terracotta-15` — status-dot tone roles.
- `--color-text-secondary`, `--font-sans`, `--space-xs`, `--space-sm` — compact stat rhythm.

## API sketch

```tsx
<GlanceCell label="AI today" value="82%" status="warn" />
<GlanceCell label="Database" value="220.8 MB" status="healthy" />
```

## Source

- **Mockup origin:** `.docs/_archive/mockups/claude-design-project/mockups/surfaces/settings/app.jsx` lines 57-74 (masthead glance section)
- **Code:** to be implemented in `src/components/ui/GlanceCell.tsx`

## Surfaces that consume it

- Settings (canonical): `.docs/design/surfaces/Settings.md`
- Future SurfaceMasthead consumers through the `GlanceRow` pattern.

## Naming notes

`GlanceCell` is the primitive-tier name in the four-tier taxonomy. It composes inside `GlanceRow`; the row owns layout, while the cell owns key/value/status rendering.

## History

- 2026-05-03 — Proposed primitive for Wave 3 (Settings substrate).
