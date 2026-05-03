# GlanceRow

**Tier:** pattern
**Status:** proposed
**Owner:** James
**Last updated:** 2026-05-03
**`data-ds-name`:** `GlanceRow`
**`data-ds-spec`:** `patterns/GlanceRow.md`
**Variants:** `default`, `compact`, `wrap`
**Design system version introduced:** `0.3.0`

## Job

A horizontal row of compact key/value stats for the `SurfaceMasthead` glance slot. It surfaces health and usage signals at the top of a surface without turning the masthead into a dashboard.

## When to use it

- A surface masthead needs quick operational status before the user enters section content.
- The stats are short key/value facts with optional status dots.
- The row has a small, stable set of cells, typically four.
- The information summarizes surface health rather than asking the user to complete a workflow.

## When NOT to use it

- The content needs charts, trend lines, tables, or drill-down controls; use a section-level analytics or status pattern.
- The stats are the primary body of the page; build a dedicated surface section instead.
- The masthead has only one action/status pill; use `SurfaceMasthead`'s accessory slot.

## Composition

`GlanceRow` composes `GlanceCell` instances and is hosted by `SurfaceMasthead` through its `glance` slot.

- **GlanceRow** - lays out cells horizontally, controls spacing, wrapping, and equal-width behavior.
- **GlanceCell** - renders one key/value stat and optional status dot.
- **SurfaceMasthead** - owns eyebrow, title, lede, and placement of the glance slot; `GlanceRow` does not repeat masthead typography.

Cells should be scannable in one pass. Keep keys terse and values concrete: "Connectors / 6 healthy", "Database / 220.8 MB", "AI today / 82%", "Anomalies 24h / 2 recovered".

## Variants

- **default** - four equal-width cells in one row for rich mastheads.
- **compact** - reduced gaps and cell padding for compact mastheads or constrained surfaces.
- **wrap** - responsive layout that wraps cells into two columns on narrow containers.

Cell status is supplied per `GlanceCell`: `healthy`, `warn`, `error`, or `neutral`. The row does not compute thresholds; consumers pass already-resolved status.

## Tokens consumed

- `--font-mono` - compact stat keys and machine-like values when appropriate.
- `--font-sans` - readable value text when values are phrases.
- `--color-text-primary` - glance values.
- `--color-text-secondary` - stat keys.
- `--color-text-tertiary` - muted secondary values.
- `--color-status-healthy`, `--color-status-warn`, `--color-status-error`, `--color-status-neutral` - optional status dots.
- `--color-border-subtle` - optional cell dividers or row boundary.
- `--space-xs`, `--space-sm`, `--space-md`, `--space-lg` - cell inner spacing and inter-cell gap.

## API sketch

```tsx
type GlanceCellStatus = 'healthy' | 'warn' | 'error' | 'neutral';

type GlanceRowProps = {
  cells: Array<{
    key: React.ReactNode;
    value: React.ReactNode;
    status?: GlanceCellStatus;
  }>;
  variant?: 'default' | 'compact' | 'wrap';
};

<SurfaceMasthead
  eyebrow="Settings · Last edited 4 minutes ago"
  title="Settings"
  lede="A quiet morning, all systems steady."
  glance={
    <GlanceRow
      cells={[
        { key: 'Connectors', value: '6 healthy', status: 'healthy' },
        { key: 'Database', value: '220.8 MB', status: 'healthy' },
        { key: 'AI today', value: '82%', status: 'warn' },
        { key: 'Anomalies 24h', value: '2 recovered', status: 'warn' },
      ]}
    />
  }
/>
```

## Source

- **Mockup origin:** `.docs/mockups/claude-design-project/mockups/surfaces/settings/app.jsx` lines 57-74 (`.s-glance` with four `.gcell` instances).
- **Host mockup:** `.docs/mockups/claude-design-project/mockups/surfaces/settings/app.jsx` lines 51-75 (`.s-masthead` hosting the glance row).
- **Host pattern:** `.docs/design/patterns/SurfaceMasthead.md` lines 20-23, 30-38, and 56-60 define the glance slot and Settings composition.
- **Consumer reference:** `.docs/design/surfaces/Settings.md` lines 24-29 and 58-70 name the Settings masthead `GlanceRow` and `GlanceCell` usage.
- **Code:** to be implemented in `src/components/layout/GlanceRow.tsx` and `src/components/layout/GlanceCell.tsx`.

## Surfaces that consume it

- `Settings` - primary Wave 3 consumer inside `SurfaceMasthead`, with Connectors, Database, AI today, and Anomalies 24h cells.
- Future non-briefing surfaces that use `SurfaceMasthead` and need a small at-a-glance health row.

## Naming notes

Canonical name is `GlanceRow`. Keep `GlanceCell` as the primitive/pattern building block for one stat; keep `GlanceRow` for the horizontal composition.

Do not rename to `StatsGrid`, `MetricRow`, or `MastheadStats`; those names imply heavier dashboard behavior than this pattern owns.

## History

- 2026-05-03 — Proposed pattern for Wave 3.
