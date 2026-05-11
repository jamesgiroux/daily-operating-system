# MeterCluster

**Tier:** pattern
**Status:** proposed
**Owner:** James
**Last updated:** 2026-05-10
**`data-ds-name`:** `MeterCluster`
**`data-ds-spec`:** `patterns/MeterCluster.md`
**Variants:** `data-meters="2 | 3 | 4"` (column count); per-meter `tone="sage | saffron | terracotta | larkspur"`; per-meter `data-direction="up | down"` for trend coloring
**Design system version introduced:** 0.6.0

## Job

Render a horizontal cluster of single-metric meters as a mid-page narrative health readout. Each meter is one independent dimension (Momentum, Confidence in date, Open risks, Partner engagement, etc.) with a tone-coded value, fill bar, and trend text.

Mid-page narrative — not a masthead vital, not a dashboard. Distinct from `VitalsStrip` (which is masthead-bound by spec) and `DimensionBar` (entity-relationship-specific, fixed 6-row scoring). MeterCluster is for "here's how the project is doing across N independent axes" mid-chapter.

## When to use it

- On Project Detail or Account Detail when narrative health needs an at-a-glance multi-axis read in the body of the page
- In report surfaces when momentum / confidence / risk / engagement deserve numeric or tonal expression alongside prose
- When the page already has masthead vitals and needs a separate mid-page health readout that doesn't read as duplicate chrome

## When NOT to use it

- For masthead-level entity vitals — use `VitalsStrip` (its spec covers the under-hero use)
- For entity-relationship dimension scoring (6 fixed dimensions with evidence drill-in) — use `DimensionBar`
- For a single high-stakes status callout — use `Callout` with `tone="success | caution | warning"` and a narrative body
- For sparse non-meter data — use `GlanceRow` of `GlanceCell` primitives

## States / variants

- **column count** — `data-meters="2 | 3 | 4"` selects grid layout. Default 4. Reflowing to single-column on narrow viewports is a surface-local concern (DailyOS is macOS-app, not web responsive)
- **value tone** — semantic intent on the headline value: `sage` (healthy / strong), `saffron` (caution / wavering), `terracotta` (urgency / risk), `larkspur` (engagement / forward-looking)
- **fill tone** — same vocabulary as value tone, controls the bar fill color
- **trend direction** — `data-direction="up | down"` colors the trend text (rosemary up, terracotta down). When trend is unchanged, omit `data-direction`. Per design decision 2026-05-10, arrow glyphs (↑↓) are NOT included — text suffices ("from steady · 2 wks", "+1 this week")

## Composition

Owns its `.meter`, `.meter-bar`, `.meter-fill`, `.meter-trend` rendering directly. Does not compose other primitives. Sits inside a `ChapterHeading` + `FreshnessLine` chapter shell.

## Tokens consumed

- `--color-garden-rosemary`, `--color-garden-sage` (sage tone)
- `--color-spice-saffron` (saffron tone)
- `--color-spice-terracotta` (terracotta tone)
- `--color-garden-larkspur` (larkspur tone)
- `--color-desk-charcoal-4` (bar track background)
- `--color-rule-heavy` (cluster top/bottom rules)
- `--color-text-primary`, `--color-text-tertiary`
- `--font-serif` (value), `--font-mono` (axis label, trend)
- `--space-lg`, `--space-2xl`

## API sketch

```html
<section class="meter-cluster"
         data-ds-name="MeterCluster"
         data-ds-spec="patterns/MeterCluster.md"
         data-meters="4">
  <div class="meter">
    <div class="meter-axis-label">Momentum</div>
    <div class="meter-value" data-tone="sage">Strong</div>
    <div class="meter-bar"><div class="meter-fill" data-tone="sage" style="width: 78%;"></div></div>
    <div class="meter-trend" data-direction="up">from steady · 2 wks</div>
  </div>
  <div class="meter">
    <div class="meter-axis-label">Confidence in date</div>
    <div class="meter-value" data-tone="saffron">Wavering</div>
    <div class="meter-bar"><div class="meter-fill" data-tone="saffron" style="width: 52%;"></div></div>
    <div class="meter-trend" data-direction="down">from confident · since Mon</div>
  </div>
  <!-- 2 more meters -->
</section>
```

React form:

```tsx
<MeterCluster
  meters={[
    { axisLabel: 'Momentum',           value: 'Strong',   tone: 'sage',       fillPct: 78, trend: 'from steady · 2 wks',     direction: 'up' },
    { axisLabel: 'Confidence in date', value: 'Wavering', tone: 'saffron',    fillPct: 52, trend: 'from confident · since Mon', direction: 'down' },
    { axisLabel: 'Open risks',         value: '3 active', tone: 'terracotta', fillPct: 38, trend: '+1 this week',              direction: 'down' },
    { axisLabel: 'Partner engagement', value: 'Heated',   tone: 'larkspur',   fillPct: 84, trend: '↑ 11 touches/wk',           direction: 'up' },
  ]}
/>
```

## Source

- **Spec:** new for v1.4.2 project-detail d-spine
- **Reference CSS:** `.docs/design/reference/_shared/styles/MeterCluster.module.css`
- **Code:** to be shipped at `src/components/entity/MeterCluster.tsx`
- **Mockup origin:** `.docs/design/figma/mockups/project-detail/variations/D-composite.html` (`.meter-cluster`, `.meter-bar`, `.meter-value`, `.meter-trend`)

## Surfaces that consume it

- ProjectDetail (canonical consumer, v1.4.2 — health readout above the phase plan)
- AccountDetail (potential consumer for account-level momentum/risk readouts)
- Reports when a multi-axis health readout anchors a narrative chapter

## Naming notes

`MeterCluster` — "meter" (single-metric readout with bar + value + trend) + "cluster" (horizontal grouping of N independent meters). Not `HealthMeters` because health is a specific semantic and meters can carry non-health metrics (engagement, momentum, velocity). Not `SignalCluster` because `SignalGrid` already exists for a different use (2x2 dense signal categorization). Not `VitalsStrip` because that pattern's spec is masthead-bound.

## History

- 2026-05-10 — Proposed for v1.4.2 project-detail d-spine. Trend arrows dropped per chrome-overlap audit (`.docs/design/_audits/project-d-spine-chrome-overlap.md`); text suffices. `VitalsStrip` was considered as a parent but its spec is masthead-only; `DimensionBar` was considered but it's relationship-scoring-specific.
