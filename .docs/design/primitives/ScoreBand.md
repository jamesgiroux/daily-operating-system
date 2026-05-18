## ScoreBand

**Tier:** primitive
**Status:** proposed
**Owner:** James
**Last updated:** 2026-05-18
**`data-ds-name`:** `ScoreBand`
**`data-ds-spec`:** `primitives/ScoreBand.md`
**Variants:** `value="on-track" | "watching" | "action-needed" | "no-read"`
**Design system version introduced:** 0.1.0 (v1.4.3 W2 Wave 1, the 11th primitive folded in per DOS-325)

## Job

The visual primitive for rendering a single entity-intelligence score as an interpretive band label. Per DOS-325 voice rule (issue body §"What good looks like"): "renders a plain-language band label. No raw number in the headline."

ScoreBand is the *visual* primitive. It has no knowledge of claims, trust factors, or substrate — the caller decides which band to render based on substrate-derived score logic that lives ABOVE the primitive layer (per primitives README discipline).

## When to use it

- Per-claim score rendering on entity-detail surfaces (Account / Project / Person Detail in v1.4.4 W2).
- Anywhere DOS-325 voice rule applies — interpretive band, evidence-drawer-companion, no raw number in headline.
- When the source of truth is a single claim's score (NOT an aggregated rollup — that's HealthBadge).

## When NOT to use it

- For aggregated entity-health rollups (multiple claims rolled into one indicator) — use `HealthBadge`. (HealthBadge today exposes color-band tokens — `green | yellow | red | insufficient-data` — and gets its DOS-325 voice-rule label-discipline pass in v1.4.4 via DOS-693; until then, HealthBadge and ScoreBand do NOT co-render on the same surface in W2.)
- For trust band states (`likely_current | use_with_caution | needs_verification`) — use `TrustBandBadge`.
- For intelligence completeness (`sparse | developing | ready | fresh`) — use `IntelligenceQualityBadge`.
- For freshness (raw recency + relative age) — use `FreshnessIndicator`.

## States / variants

The four label values are DailyOS magazine voice for the four band-rendering cardinal directions, per DOS-325 voice rule + ADR-0083 product vocabulary:

- `value="on-track"` → label `On Track` (background sage-15, text sage) — score in a confident range.
- `value="watching"` → label `Watching` (background turmeric-15, text turmeric) — score in an emphasis range; warrants attention but not alarm.
- `value="action-needed"` → label `Action Needed` (background terracotta-15, text terracotta) — score in an urgency range; user attention required.
- `value="no-read"` → label `No Read` (background charcoal-4, text tertiary) — insufficient data to band.

Optional `label` prop overrides the canonical vocabulary for callers that need locale-specific or context-specific text. Default usage SHOULD pass `value` only and let the primitive render the canonical label.

## Tokens consumed

- `--color-garden-sage-15`, `--color-garden-sage` (on-track)
- `--color-spice-turmeric-15`, `--color-spice-turmeric` (watching)
- `--color-spice-terracotta-15`, `--color-spice-terracotta` (action-needed)
- `--color-desk-charcoal-4`, `--color-text-tertiary` (no-read)
- `--font-mono`

## API sketch

```tsx
<ScoreBand value="on-track" />
<ScoreBand value="watching" />
<ScoreBand value="action-needed" />
<ScoreBand value="no-read" />

// Optional locale-specific label override
<ScoreBand value="on-track" label="Steady" />
```

CSS class form for the WP block (PR-D4):

```html
<span class="dailyos-score-band" data-value="on-track" data-ds-name="ScoreBand" data-ds-spec="primitives/ScoreBand.md">
  On Track
</span>
```

## Source

- **DOS-325 issue body** — score-band rendering acceptance criteria + voice rule. Three primitives in scope (ScoreBand, TrendStrip, EvidenceDrawer); v1.4.3 W2 takes ScoreBand only (per V1.3 §6.4 + lineage tickets DOS-688 / DOS-689 / DOS-690).
- **Code:** ships in `src/components/ui/ScoreBand.tsx` + `ScoreBand.module.css` (v1.4.3 W2 PR-D1).
- **WP block:** ships in `wp/dailyos/blocks/score-band/` (v1.4.3 W2 PR-D4).

## Surfaces that consume it

Wave 1 consumers (v1.4.4 W2 Entity Surfaces — see DOS-690): Account Detail, Project Detail, Person Detail. ScoreBand renders per-claim scores in the trust-band-bearing claim rows of each entity-detail surface.

## Co-render with HealthBadge (v1.4.4)

A v1.4.4 entity-detail surface may render BOTH a ScoreBand (per-claim) and a HealthBadge (entity-rollup) on the same page. The co-render is safe in W2 because HealthBadge today does NOT use label vocabulary (only color bands). When HealthBadge gets the DOS-325 voice-rule label-discipline pass (DOS-693), the co-render scenario gets re-validated at L4 — that ticket owns the decision on whether HealthBadge picks the same four labels or distinct ones.

## Naming notes

`ScoreBand` is the primitive. `TrendStrip` (DOS-325 sibling, deferred to v1.4.4 DOS-688) renders directional movement of a score over time; `EvidenceDrawer` (DOS-325 sibling, deferred to v1.4.4 DOS-689) renders the source-math drawer that opens from a ScoreBand instance.

## History

- 2026-05-18 — Initial author for v1.4.3 W2 PR-D1 per DOS-682 §6.4 + DOS-325 fold. `proposed` until the WP block lands in PR-D4 (then promote to `integrated` per the WP-block-consumption-counts rule).
