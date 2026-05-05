# Primitives

The smallest reusable units. Button, Input, Card shell, Pill, Chip, Avatar. Generic, unopinionated, no domain knowledge.

A primitive is *not* a primitive if it knows about claims, trust, briefings, or any DailyOS concept. That's a pattern.

## Index

### Wave 1 (v1.4.3 substrate, 0.1.0)

| Name | Status | Job (one line) | Source |
|---|---|---|---|
| [`Pill`](./Pill.md) | canonical/shipped | Visual primitive for inline status / label / category badges | `src/components/ui/Pill.tsx` |
| [`TrustBandBadge`](./TrustBandBadge.md) | canonical/shipped | v1.4.0 surface trust band (`likely_current` / `use_with_caution` / `needs_verification`) | `src/components/ui/TrustBandBadge.tsx` |
| [`IntelligenceQualityBadge`](./IntelligenceQualityBadge.md) | canonical/shipped | Intelligence completeness (`sparse` / `developing` / `ready` / `fresh`) | `src/components/entity/` |
| [`FreshnessIndicator`](./FreshnessIndicator.md) | canonical/shipped | Raw recency timestamp + relative age | `src/components/ui/FreshnessIndicator.tsx` |
| [`ProvenanceTag`](./ProvenanceTag.md) | canonical/shipped | Source attribution label, suppresses synthesized | `src/components/ui/` |
| [`EntityChip`](./EntityChip.md) | canonical/shipped | Entity reference with entity-type color | `src/components/ui/EntityChip.tsx` |
| [`TypeBadge`](./TypeBadge.md) | canonical/shipped | Account-type categorical (Customer / Internal / Partner) | `_shared/.type-badge` + AccountHero |

### Wave 2 (v1.4.4 trust UI, 0.2.0)

| Name | Status | Job (one line) | Source |
|---|---|---|---|
| [`SourceCoverageLine`](./SourceCoverageLine.md) | canonical/shipped | Compact line summarizing source coverage (e.g., "Glean · 4 sources · 2 stale") | `src/components/ui/SourceCoverageLine.tsx` |
| [`ConfidenceScoreChip`](./ConfidenceScoreChip.md) | canonical/shipped | Numerical confidence score chip with threshold-based tone | `src/components/ui/ConfidenceScoreChip.tsx` |
| [`VerificationStatusFlag`](./VerificationStatusFlag.md) | canonical/shipped | Consistency state per v1.4.0 (`ok` / `corrected` / `flagged`) | `src/components/ui/VerificationStatusFlag.tsx` |
| [`DataGapNotice`](./DataGapNotice.md) | canonical/shipped | Inline warning that intelligence is missing critical inputs | `src/components/ui/DataGapNotice.tsx` |
| [`AsOfTimestamp`](./AsOfTimestamp.md) | canonical/shipped | Static "as of" timestamp label (companion to FreshnessIndicator) | `src/components/ui/AsOfTimestamp.tsx` |

### Wave 3 (Settings substrate, 0.3.0)

| Name | Status | Job (one line) | Source |
|---|---|---|---|
| [`InlineInput`](./InlineInput.md) | roadmap/planned | Click-to-edit text input with pencil affordance | target `src/components/ui/InlineInput.tsx` |
| [`Switch`](./Switch.md) | canonical/shipped | Aria-checked toggle button | `src/components/ui/Switch.tsx` |
| [`Segmented`](./Segmented.md) | canonical/shipped | Tinted button group with `aria-pressed` state | `src/components/ui/Segmented.tsx` |
| [`RemovableChip`](./RemovableChip.md) | canonical/shipped | Chip with × removal affordance (distinct from `Pill`) | `src/components/ui/RemovableChip.tsx` |
| [`GlanceCell`](./GlanceCell.md) | canonical/shipped | Single key/value stat cell (composed in `GlanceRow`) | `src/components/ui/GlanceCell.tsx` |

### Wave 4 (Meeting Detail substrate, 0.4.0)

| Name | Status | Job (one line) | Source |
|---|---|---|---|
| [`MeetingStatusPill`](./MeetingStatusPill.md) | canonical/shipped | Meeting status (wrapped / processing / failed) for MeetingHero accessory | `src/components/meeting/MeetingStatusPill.tsx` |

## Conventions

- **Names are short and generic.** `Button`, not `BaseButton` or `PrimaryActionButton`.
- **Variants are documented.** Every visible variation (size, intent, density) is in the spec.
- **Tokens only.** Primitives consume tokens. They never hardcode values.
- **Composition aware.** A primitive should compose cleanly inside any pattern. Avoid layout opinions; let patterns set spacing.
- **One file per primitive.** `Button.md`, not `Buttons.md`. Granularity makes it greppable.

## Adding a primitive

1. Confirm it's actually a primitive (no domain knowledge, used or usable in 2+ patterns).
2. Copy `../_TEMPLATE-entry.md` here.
3. Fill in the spec. Empty sections mean it's not ready.
4. Add to the index above.
5. If a reference render exists, link it.

## Adding a primitive that already exists in `src/`

Promotion is a markdown PR that documents what's already there — no code change required to *promote*. Code changes to consolidate variants come after.
