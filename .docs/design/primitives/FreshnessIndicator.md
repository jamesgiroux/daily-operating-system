# FreshnessIndicator

**Tier:** primitive
**Status:** proposed
**Owner:** James
**Last updated:** 2026-05-02
**`data-ds-name`:** `FreshnessIndicator`
**`data-ds-spec`:** `primitives/FreshnessIndicator.md`
**Variants:** `format="relative" | "absolute" | "both"`; staleness coloring per age threshold
**Design system version introduced:** 0.1.0

## Job

Render raw recency — the `source_asof` timestamp of a piece of intelligence — as a relative age ("3h ago", "2d", "stale 5d") or absolute time. Distinct from trust judgment and completeness; this is **just the time**.

## When to use it

- Next to a claim, intelligence summary, or rendered fact where the user benefits from knowing how recent the underlying source is
- On chapter-level "as of" labels in briefing and entity surfaces
- Inline with a `TrustBandBadge` (often) — they render together but mean different things

## When NOT to use it

- For trust judgment — use `TrustBandBadge`
- For completeness — use `IntelligenceQualityBadge`
- For source attribution / which-source — use `ProvenanceTag`
- For "system was last refreshed at X" page-level chrome — that's FolioBar's `data-folio-status` slot

## States / variants

- `format="relative"` — "3h ago", "2d", "stale 5d" (default for inline)
- `format="absolute"` — "Apr 22, 10:30am" (when precision matters)
- `format="both"` — "3h ago · Apr 22 10:30" (for inspection contexts)

Staleness threshold (by entity / claim type, configurable):
- fresh — under threshold, normal text color
- aging — past threshold but under stale, muted
- stale — past stale threshold, color shifts to amber/saffron

## Tokens consumed

- `--font-mono` (timestamp typography)
- `--color-text-tertiary` (default)
- `--color-text-quaternary` (aging)
- `--color-spice-saffron` (stale; consider `--color-trust-use-with-caution` once defined)
- `--space-xs`

## API sketch

```tsx
<FreshnessIndicator at="2026-05-02T08:00:00Z" />
<FreshnessIndicator at={asof} format="both" />
<FreshnessIndicator at={asof} stalenessThreshold={48} /> {/* hours */}
```

CSS class form:

```html
<span class="freshness" data-staleness="fresh|aging|stale" data-ds-name="FreshnessIndicator">
  3h ago
</span>
```

## Source

- **Spec:** new for Wave 1
- **Substrate contract:** v1.4.0 `source_asof` per `.docs/plans/v1.4.0-waves.md` W3-G
- **Code:** to be implemented in `src/components/ui/FreshnessIndicator.tsx`
- **Closest existing component:** `src/components/editorial/ChapterFreshness.tsx` — chapter-level freshness strip; consider whether this composes FreshnessIndicator or stays a separate pattern

## Surfaces that consume it

DailyBriefing (per-meeting recency), AccountDetail (claim-level + chapter-level), MeetingDetail (transcript / capture recency), ProjectDetail, PersonDetail. Composed inside `TrustBand` pattern (Wave 2).

## Naming notes

`FreshnessChip` from the Wave 2 audit candidate list (Audit 04) is **consolidated into this primitive** — same concept, single canonical name. `FreshnessChip` is dropped from Wave 2 spec scope. The chip-shaped visual treatment is the default render of `FreshnessIndicator` (compact format).

## History

- 2026-05-02 — Proposed primitive per design system D5 refinement.
