# SurfaceMasthead

**Tier:** pattern
**Status:** proposed
**Owner:** James
**Last updated:** 2026-05-03
**`data-ds-name`:** `SurfaceMasthead`
**`data-ds-spec`:** `patterns/SurfaceMasthead.md`
**Variants:** `density="compact" | "default" | "rich"`; `accessory` slot; `glance` slot for `GlanceRow`
**Design system version introduced:** 0.3.0

## Job

The canonical top-of-surface block — eyebrow + title + lede with optional accessory (status pill, action) and optional glance row underneath. Generalizes from Settings' `s-masthead`, MeetingDetail's hero, and similar surface-opening compositions. Replaces ad-hoc per-surface hero treatments.

This pattern subsumes `MeetingHero` per design system D5 (synthesis Wave 4 reconciliation): MeetingHero is a `SurfaceMasthead` composed with `MeetingStatusPill` accessory + meeting metadata, not a separate pattern.

## When to use it

- Top of any non-briefing surface that needs a structured opening (Settings, MeetingDetail, future Reports surfaces)
- When you need eyebrow + title + lede pairing with optional accessory
- When the surface has surface-level "at-a-glance" stats that should sit near the hero (use `glance` slot for GlanceRow)

## When NOT to use it

- DailyBriefing's opening — that's the `Lead` pattern (one big sentence, different shape)
- Entity hero pages (AccountDetail, PersonDetail, ProjectDetail) — those have their own hero patterns (`AccountHero`, etc.) with entity-identity treatment
- Modal headers — use modal chrome instead

## Composition

```
[Eyebrow — mono uppercase 11px, color text-tertiary, optional metadata like "Last edited 4 minutes ago"]
[Title — serif 36-52px, weight 400, color text-primary]
[Lede — serif 17-21px, weight 300, color text-secondary, optional]
[Accessory slot — top-right, optional (e.g., MeetingStatusPill, action button)]
[Glance slot — below masthead, optional (renders GlanceRow)]
```

Two-column layout when accessory is present (title block | accessory). Single column otherwise.

## Variants

- **compact** — title 28px, no lede, accessory inline-right
- **default** — title 36px, lede if provided, accessory in own column
- **rich** — title 52px, full lede, glance row below, accessory in own column

## Composed-by examples

**MeetingHero** = SurfaceMasthead with:
- eyebrow: "Meeting Recap · Tuesday Apr 17 · 11:00–11:56 AM PT"
- title: "Meridian Harbor — Q2 Business Review"
- lede: one-paragraph synthesis
- accessory: `<MeetingStatusPill state="wrapped" duration="56 min" />`

**Settings masthead** = SurfaceMasthead with:
- eyebrow: "Settings · Last edited 4 minutes ago"
- title: "Settings"
- lede: "A quiet morning, all systems steady. One connector — Gravatar — is refreshing its avatar cache; nothing to act on."
- glance: `<GlanceRow cells={[connectors, database, ai, anomalies]} />`

## Tokens consumed

- `--font-mono` (eyebrow), `--font-serif` (title + lede)
- `--color-text-tertiary` (eyebrow), `--color-text-primary` (title), `--color-text-secondary` (lede)
- `--space-md`, `--space-lg`, `--space-xl` (vertical rhythm)

## API sketch

```tsx
<SurfaceMasthead
  eyebrow="Meeting Recap · Tuesday Apr 17 · 11:00–11:56 AM PT"
  title="Meridian Harbor — Q2 Business Review"
  lede="Marco raised pricing in the first 4 minutes — he wants a 12% reduction tied to a 24-month renewal. Aoife (procurement) backed him..."
  accessory={<MeetingStatusPill state="wrapped" />}
  density="rich"
/>
```

## Source

- **Spec:** new for Wave 3
- **Mockup substrate:** Settings `.s-masthead` (`mockups/surfaces/settings/app.jsx` lines 51-75); MeetingDetail `.cur-hero-title` + surrounding (`mockups/meeting/current/after.html` lines 39-64)
- **Code:** to be implemented in `src/components/layout/SurfaceMasthead.tsx`

## Surfaces that consume it

Settings (canonical Wave 3 use), MeetingDetail (Wave 4 — replaces standalone MeetingHero), future Reports surfaces, future "deliverable" surfaces (case studies, briefings-as-deliverable).

**Not consumed by**: DailyBriefing (uses `Lead`), AccountDetail / ProjectDetail / PersonDetail (use entity-specific heroes already canonical in `src/components/entity/EntityHeroBase.tsx`).

## Naming notes

`SurfaceMasthead` — "masthead" is the editorial term for a publication's nameplate; SurfaceMasthead is the top-of-surface equivalent. Don't rename to `PageHeader` (too generic) or `Hero` (overlaps with entity heroes).

`MeetingHero` is intentionally **not** a separate pattern — it's a documented composition example (above). This is the synthesis D-series reconciliation: don't fork patterns when composition will do.

## History

- 2026-05-03 — Proposed pattern for Wave 3, also subsumes MeetingHero (Wave 4) per synthesis reconciliation.
