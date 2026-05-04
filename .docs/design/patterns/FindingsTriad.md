# FindingsTriad

**Tier:** pattern
**Status:** proposed
**Owner:** James
**Last updated:** 2026-05-03
**`data-ds-name`:** `FindingsTriad`
**`data-ds-spec`:** `patterns/FindingsTriad.md`
**Variants:** `default`; `column="wins" | "risks" | "decisions"`
**Design system version introduced:** 0.4.0

## Job

Group the meeting's most important wins, risks, and decisions into three scannable columns with enough evidence for the user to trust each finding.

## When to use it

- In MeetingDetail's "Findings" chapter.
- When the content has exactly three semantic buckets: wins, risks, and decisions.
- When findings may include evidence quotes and speaker attribution.

## When NOT to use it

- For prediction-backed comparison — use `PredictionsVsRealityGrid`.
- For a single relationship-health finding — use `ChampionHealthBlock`.
- For confirmed commitments — use `CommitmentRow`.

## States / variants

- **default** — three columns in reading order: Wins, Risks, Decisions.
- **with evidence** — finding includes quote block and attribution.
- **without evidence** — decisions may render title + impact only when no quote exists.
- **empty column** — keep the column heading and render a quiet "No findings captured" message.
- **mobile** — stack Wins, Risks, Decisions in that order.

## Composition

- `SectionLabel` primitive for each column heading, tinted by column where applicable.
- `Dot` primitive for win, risk, and decision markers.
- `Text` primitives for finding title and impact paragraph.
- Evidence quote primitive for quoted transcript support.
- Attribution primitive for speaker name and timestamp below the evidence quote.

## Tokens consumed

- `--color-garden-rosemary` — wins heading and win dot.
- `--color-spice-terracotta` — risks heading and risk dot.
- `--color-text-tertiary` — decision dot or neutral marker.
- `--color-text-primary` — finding titles and evidence text.
- `--color-text-secondary` — impact and attribution text.
- `--font-mono` — column labels and attribution.
- `--font-sans`, `--font-serif` — finding copy and evidence quote.
- `--space-md`, `--space-lg` — card rhythm and column gaps.

## API sketch

```tsx
<FindingsTriad
  wins={[
    {
      title: "Marco asked for a case-study quote",
      impact: "First time this quarter. Public reference candidate.",
      evidence: {
        quote: "The team's been telling me it's the best ticketing setup we've had in three years — happy to say that on the record.",
        attribution: "Marco · 11:31"
      }
    }
  ]}
  risks={[]}
  decisions={[]}
/>
```

## Source

- **Code:** to be implemented in `src/components/meeting/FindingsTriad.tsx`
- **Mockup origin:** `.docs/_archive/mockups/claude-design-project/mockups/meeting/current/after.html:195-241`
- **Reference render:** Meeting "Findings" chapter.

## Surfaces that consume it

- [MeetingDetail](../surfaces/MeetingDetail.md) — canonical consumer.

## Naming notes

`FindingsTriad` reflects the fixed three-column information shape. Do not rename to `FindingsGrid`; the three buckets are part of the pattern contract.

## History

- 2026-05-03 — Proposed for Wave 4
