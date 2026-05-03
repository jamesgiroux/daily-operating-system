# TrustBandBadge

**Tier:** primitive
**Status:** proposed
**Owner:** James
**Last updated:** 2026-05-02
**`data-ds-name`:** `TrustBandBadge`
**`data-ds-spec`:** `primitives/TrustBandBadge.md`
**Variants:** `band="likely_current" | "use_with_caution" | "needs_verification"`
**Design system version introduced:** 0.1.0

## Job

Render the v1.4.0 surface trust band on a piece of intelligence — the user-facing judgment ("can I trust this?"). Wired to the v1.4.0 substrate render contract (DOS-320: render surfaces filter by trust band).

## When to use it

- On any surface that renders a claim, derived intelligence, or AI-produced content where trust banding is meaningful (briefing items, account detail facts, meeting prep, project state, person stakeholder data)
- When the user benefits from a quick visual signal of how much to lean on the rendered information
- Inline next to the content; not as a global page-level signal

## When NOT to use it

- For raw recency ("3h ago") — use `FreshnessIndicator`
- For intelligence completeness ("sparse / developing / ready / fresh") — use `IntelligenceQualityBadge`
- For source attribution — use `ProvenanceTag`
- For receipt-level inspection (resolver bands like `Resolved / ResolvedWithFlag`) — use `ResolverConfidenceBadge` (Wave 2)

## States / variants

- `band="likely_current"` — green family (sage). The user can lean on this with confidence.
- `band="use_with_caution"` — amber family (saffron). Some signal but not conclusive; verify before betting.
- `band="needs_verification"` — red family (terracotta). Don't act on this without confirming.

Optional `compact` variant for inline-with-text rendering (smaller, dot-only or single-letter).

## Tokens consumed

- `--color-trust-likely-current` (proposed; likely sage-15 / rosemary text)
- `--color-trust-use-with-caution` (proposed; likely saffron-15 / turmeric-darkened text)
- `--color-trust-needs-verification` (proposed; likely terracotta-15 / chili text)
- `--font-mono` (label, uppercase, letter-spacing)
- `--space-xs`, `--space-sm`

## API sketch

```tsx
<TrustBandBadge band="likely_current" />
<TrustBandBadge band="use_with_caution" compact />
<TrustBandBadge band="needs_verification">Verify before acting</TrustBandBadge>
```

Composes `Pill` underneath with mapped tone (`sage` / `turmeric` / `terracotta`) but exposes the v1.4.0 vocabulary at the API surface.

## Source

- **Spec:** new for Wave 1
- **Substrate contract:** v1.4.0 render trust band per `.docs/plans/v1.4.0-waves.md:631` (DOS-320)
- **Code:** to be implemented in `src/components/ui/TrustBandBadge.tsx` (Wave 1 follow-on)

## Surfaces that consume it

DailyBriefing (meeting prep state), AccountDetail (claim-level surfacing), MeetingDetail (intelligence sections), ProjectDetail, PersonDetail. Foundational for v1.4.4 receipts (composed inside `TrustBand` pattern).

## Naming notes

Distinct from `IntelligenceQualityBadge` (completeness vocabulary) and `FreshnessIndicator` (raw recency). The three render together often but mean different things — see `00-synthesis.md` D5 for the rationale.

## History

- 2026-05-02 — Proposed primitive per design system D5. Maps to v1.4.0 substrate render contract.
