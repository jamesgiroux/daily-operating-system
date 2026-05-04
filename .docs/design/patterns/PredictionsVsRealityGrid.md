# PredictionsVsRealityGrid

**Tier:** pattern
**Status:** proposed
**Owner:** James
**Last updated:** 2026-05-03
**`data-ds-name`:** `PredictionsVsRealityGrid`
**`data-ds-spec`:** `patterns/PredictionsVsRealityGrid.md`
**Variants:** `default`; `density="comfortable" | "compact"`
**Design system version introduced:** 0.4.0

## Job

Compare the meeting's actual outcome against the pre-meeting briefing so the user can see which predicted risks materialized and which planned wins landed.

## When to use it

- In MeetingDetail's "Predictions vs. Reality" chapter.
- When findings are explicitly derived from a pre-meeting prediction or briefing item.
- When the user needs a side-by-side risks / wins readout, not a chronological agenda recap.

## When NOT to use it

- For agenda item tracking — use `AgendaThreadList`.
- For post-meeting evidence-backed findings that are not tied to a prediction — use `FindingsTriad`.
- For a single champion relationship signal — use `ChampionHealthBlock`.

## States / variants

- **default** — two columns: "Risks (from the briefing)" and "Wins (from the briefing)".
- **compact** — preserves column semantics but tightens vertical gaps for narrow or dense surfaces.
- **empty risks** — keep the risks heading and render "No briefing risks confirmed" in secondary text.
- **empty wins** — keep the wins heading and render "No briefing wins confirmed" in secondary text.
- **mobile** — stack risks above wins; do not interleave findings.

## Composition

- `SectionLabel` primitive for the mono column headings, tinted terracotta for risks and sage for wins.
- `Dot` primitive for each finding marker: `tone="risk"` or `tone="win"`.
- `Text` primitives for finding title and impact paragraph.
- Two equal pattern columns inside the parent chapter; the grid owns column layout, not chapter spacing.

## Tokens consumed

- `--color-spice-terracotta` — risks heading and risk dot tone.
- `--color-garden-rosemary` — wins heading and win dot tone.
- `--color-text-primary` — finding title.
- `--color-text-secondary` — impact paragraph.
- `--font-mono` — column labels.
- `--font-sans` — finding title and impact copy.
- `--space-md`, `--space-lg` — row and column spacing.

## API sketch

```tsx
<PredictionsVsRealityGrid
  risks={[
    {
      title: "Procurement-led 12% pricing compression — CONFIRMED",
      impact: "Asked exactly as predicted. Marco named Acme as the comparator."
    }
  ]}
  wins={[
    {
      title: "92% Helpline adoption landed well",
      impact: "Marco asked for a case-study quote — first time this quarter."
    }
  ]}
/>
```

## Source

- **Code:** to be implemented in `src/components/meeting/PredictionsVsRealityGrid.tsx`
- **Mockup origin:** `.docs/_archive/mockups/claude-design-project/mockups/meeting/current/after.html:116-162`
- **Reference render:** Meeting "Predictions vs. Reality" chapter.

## Surfaces that consume it

- [MeetingDetail](../surfaces/MeetingDetail.md) — canonical consumer.

## Naming notes

`PredictionsVsRealityGrid` names the comparison contract directly. Avoid `FindingsGrid`; that blurs this pattern with `FindingsTriad`.

## History

- 2026-05-03 — Proposed for Wave 4
