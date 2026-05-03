# Primitives

The smallest reusable units. Button, Input, Card shell, Pill, Chip, Avatar. Generic, unopinionated, no domain knowledge.

A primitive is *not* a primitive if it knows about claims, trust, briefings, or any DailyOS concept. That's a pattern.

## Index

### Wave 1 (v1.4.3 substrate, 0.1.0)

| Name | Status | Job (one line) | Source |
|---|---|---|---|
| [`Pill`](./Pill.md) | canonical | Visual primitive for inline status / label / category badges | `_shared/.pill` |
| [`TrustBandBadge`](./TrustBandBadge.md) | proposed | v1.4.0 surface trust band (`likely_current` / `use_with_caution` / `needs_verification`) | new (DOS-320 contract) |
| [`IntelligenceQualityBadge`](./IntelligenceQualityBadge.md) | canonical | Intelligence completeness (`sparse` / `developing` / `ready` / `fresh`) | `src/components/entity/` |
| [`FreshnessIndicator`](./FreshnessIndicator.md) | proposed | Raw recency timestamp + relative age | new (`source_asof` contract) |
| [`ProvenanceTag`](./ProvenanceTag.md) | canonical | Source attribution label, suppresses synthesized | `src/components/ui/` |
| [`EntityChip`](./EntityChip.md) | canonical | Entity reference with entity-type color | `src/components/ui/{meeting,email}-entity-chip.tsx` |
| [`TypeBadge`](./TypeBadge.md) | canonical | Account-type categorical (Customer / Internal / Partner) | `_shared/.type-badge` + AccountHero |

### Wave 2 (v1.4.4 trust UI, 0.2.0)

| Name | Status | Job (one line) | Source |
|---|---|---|---|
| [`SourceCoverageLine`](./SourceCoverageLine.md) | proposed | Compact line summarizing source coverage (e.g., "Glean · 4 sources · 2 stale") | new |
| [`ConfidenceScoreChip`](./ConfidenceScoreChip.md) | proposed | Numerical confidence score chip with threshold-based tone | new |
| [`VerificationStatusFlag`](./VerificationStatusFlag.md) | proposed | Consistency state per v1.4.0 (`ok` / `corrected` / `flagged`) | new |
| [`DataGapNotice`](./DataGapNotice.md) | proposed | Inline warning that intelligence is missing critical inputs | new |
| [`AsOfTimestamp`](./AsOfTimestamp.md) | proposed | Static "as of" timestamp label (companion to FreshnessIndicator) | new |

### Wave 3 (Settings substrate, 0.3.0)

| Name | Status | Job (one line) | Source |
|---|---|---|---|
| [`InlineInput`](./InlineInput.md) | proposed | Click-to-edit text input with pencil affordance | mockup `settings/parts.jsx` |
| [`Switch`](./Switch.md) | proposed | Aria-checked toggle button | mockup `settings/parts.jsx` |
| [`Segmented`](./Segmented.md) | proposed | Tinted button group with `aria-pressed` state | mockup `settings/parts.jsx` |
| [`RemovableChip`](./RemovableChip.md) | proposed | Chip with × removal affordance (distinct from `Pill`) | mockup `settings/parts.jsx` |
| [`GlanceCell`](./GlanceCell.md) | proposed | Single key/value stat cell (composed in `GlanceRow`) | mockup `settings/app.jsx` masthead |

### Wave 4 (Meeting Detail substrate, 0.4.0)

| Name | Status | Job (one line) | Source |
|---|---|---|---|
| [`MeetingStatusPill`](./MeetingStatusPill.md) | proposed | Meeting status (wrapped / processing / failed) for MeetingHero accessory | mockup `meeting/current/after.html` |

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
