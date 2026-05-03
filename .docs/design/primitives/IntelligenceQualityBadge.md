# IntelligenceQualityBadge

**Tier:** primitive
**Status:** canonical
**Owner:** James
**Last updated:** 2026-05-02
**`data-ds-name`:** `IntelligenceQualityBadge`
**`data-ds-spec`:** `primitives/IntelligenceQualityBadge.md`
**Variants:** `quality="sparse" | "developing" | "ready" | "fresh"`
**Design system version introduced:** 0.1.0

## Job

Render the **completeness** of intelligence on an entity (account, project, person, meeting prep). Distinct from trust judgment (`TrustBandBadge`) and from raw recency (`FreshnessIndicator`) — this signals "how built-out is the dossier" not "should you trust it" or "how recent is it."

## When to use it

- On entity surfaces (account / project / person hero, briefing meeting items) to signal how complete the AI-derived intelligence is
- When the user benefits from knowing whether to expect a thin or rich set of signals
- D-spine briefing's `prep-state` chip consumes this primitive (variants align)

## When NOT to use it

- For trust judgment — use `TrustBandBadge`
- For raw recency / age — use `FreshnessIndicator`
- For source attribution — use `ProvenanceTag`

## States / variants

- `quality="sparse"` — minimal data captured; expect thin output
- `quality="developing"` — partial data; some signal but more in flight
- `quality="ready"` — sufficient data for normal-density output
- `quality="fresh"` — recently enriched; full-density output

D-spine adds two related labels that map to the same primitive:
- `building` ↔ `developing` (in-progress generation)
- `captured` ↔ `ready` (raw input has been captured but not yet enriched)
- `new` is an additional signal — first-touch; treat as `sparse` or new variant TBD

Quality dot color (per `_shared/primitives.css`):
- sparse → `--color-text-tertiary` (grey)
- developing → `--color-spice-saffron`
- ready → `--color-garden-sage`
- fresh → `--color-garden-sage` brighter

## Tokens consumed

- `--color-garden-sage`, `--color-spice-saffron`, `--color-text-tertiary` (per variant dot)
- `--font-mono` (label, uppercase, small-caps treatment)
- `--space-xs`, `--space-sm`

## API sketch

```tsx
<IntelligenceQualityBadge quality="ready" />
<IntelligenceQualityBadge quality="sparse" showLabel={false} />
<IntelligenceQualityBadge quality="developing" enrichedAt="2026-05-02T10:30:00Z" />
```

`showLabel` toggle for icon-only mode. `enrichedAt` optional metadata feeding the trust math but not necessarily rendered (use `FreshnessIndicator` for that).

## Source

- **Code:** `src/components/entity/IntelligenceQualityBadge.tsx`
- **CSS:** `src/components/entity/IntelligenceQualityBadge.module.css`
- **Mockup substrate (compatible):** `_shared/primitives.css` `.hero-eyebrow .quality-dot`
- **D-spine `prep-state` consumer:** `.docs/mockups/claude-design-project/mockups/briefing/variations/D-spine.html` lines 683-702

## Surfaces that consume it

DailyBriefing (per-meeting prep state), AccountDetail (hero quality dot), ProjectDetail, PersonDetail, MeetingDetail.

## Naming notes

Existing primitive in `src/`. Confirmed in synthesis D5 — keep this name; do not rename to `QualityBadge`. D-spine's `prep-state` chip is the intended consumer (drop the local CSS, render `IntelligenceQualityBadge` instead).

## History

- 2026-05-02 — Documented as canonical (existing src/ primitive).
- Audit 04 — surfaced as the existing primitive most components touch for completeness signaling.
- Audit 03 — D-spine `prep-state` consolidation candidate.
