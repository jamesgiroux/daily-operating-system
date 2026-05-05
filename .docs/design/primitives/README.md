# Primitives

The smallest reusable units. Button, Input, Card shell, Pill, Chip, Avatar. Generic, unopinionated, no domain knowledge.

A primitive is *not* a primitive if it knows about claims, trust, briefings, or any DailyOS concept. That's a pattern.

## Status vocabulary

- **canonical/shipped** — shared source component consumed by shipped routed UI.
- **shipped-local/extraction-needed** — real shipped UI, but still source-local, inline-styled, or tied to one domain implementation.
- **implemented/unintegrated** — source exists, but no shipped routed surface consumes it yet.
- **roadmap/planned** — no shipped source component under this name.

## Index

### Wave 1 (v1.4.3 substrate, 0.1.0)

| Name | Status | Job (one line) | Source |
|---|---|---|---|
| [`Pill`](./Pill.md) | canonical/shipped | Visual primitive for inline status / label / category badges | `src/components/ui/Pill.tsx` |
| [`HealthBadge`](./HealthBadge.md) | canonical/shipped | Shared health score dot/score/trend visual | `src/components/shared/HealthBadge.tsx` |
| [`StatusDot`](./StatusDot.md) | canonical/shipped | Connector/system status dot with optional label | `src/components/shared/StatusDot.tsx` |
| [`Avatar`](./Avatar.md) | canonical/shipped | Person photo/initial fallback avatar | `src/components/ui/Avatar.tsx` |
| [`TrustBandBadge`](./TrustBandBadge.md) | implemented/unintegrated | v1.4.0 surface trust band (`likely_current` / `use_with_caution` / `needs_verification`) | `src/components/ui/TrustBandBadge.tsx` |
| [`IntelligenceQualityBadge`](./IntelligenceQualityBadge.md) | canonical/shipped | Intelligence completeness (`sparse` / `developing` / `ready` / `fresh`) | `src/components/entity/` |
| [`FreshnessIndicator`](./FreshnessIndicator.md) | canonical/shipped | Raw recency timestamp + relative age | `src/components/ui/FreshnessIndicator.tsx` |
| [`ProvenanceTag`](./ProvenanceTag.md) | canonical/shipped | Source attribution label, suppresses synthesized | `src/components/ui/` |
| [`EntityChip`](./EntityChip.md) | canonical/shipped | Entity reference with entity-type color | `src/components/ui/EntityChip.tsx` |
| [`TypeBadge`](./TypeBadge.md) | shipped-local/extraction-needed | Account-type categorical (Customer / Internal / Partner) | `_shared/.type-badge` + AccountHero |

### Wave 2 (v1.4.4 trust UI, 0.2.0)

| Name | Status | Job (one line) | Source |
|---|---|---|---|
| [`SourceCoverageLine`](./SourceCoverageLine.md) | implemented/unintegrated | Compact line summarizing source coverage (e.g., "Glean · 4 sources · 2 stale") | `src/components/ui/SourceCoverageLine.tsx` |
| [`ConfidenceScoreChip`](./ConfidenceScoreChip.md) | implemented/unintegrated | Numerical confidence score chip with threshold-based tone | `src/components/ui/ConfidenceScoreChip.tsx` |
| [`VerificationStatusFlag`](./VerificationStatusFlag.md) | implemented/unintegrated | Consistency state per v1.4.0 (`ok` / `corrected` / `flagged`) | `src/components/ui/VerificationStatusFlag.tsx` |
| [`DataGapNotice`](./DataGapNotice.md) | implemented/unintegrated | Inline warning that intelligence is missing critical inputs | `src/components/ui/DataGapNotice.tsx` |
| [`AsOfTimestamp`](./AsOfTimestamp.md) | implemented/unintegrated | Static "as of" timestamp label (companion to FreshnessIndicator) | `src/components/ui/AsOfTimestamp.tsx` |

### Wave 3 (Settings substrate, 0.3.0)

| Name | Status | Job (one line) | Source |
|---|---|---|---|
| [`InlineInput`](./InlineInput.md) | roadmap/planned | Click-to-edit text input with pencil affordance | target `src/components/ui/InlineInput.tsx` |
| [`EditableText`](./EditableText.md) | canonical/shipped | Click-to-edit display text used by meeting, entity, and report surfaces | `src/components/ui/EditableText.tsx` |
| [`FolioRefreshButton`](./FolioRefreshButton.md) | canonical/shipped | Editorial mono refresh/run button used in folio and hero action areas | `src/components/ui/folio-refresh-button.tsx` |
| [`Switch`](./Switch.md) | canonical/shipped | Aria-checked toggle button | `src/components/ui/Switch.tsx` |
| [`Segmented`](./Segmented.md) | canonical/shipped | Tinted button group with `aria-pressed` state | `src/components/ui/Segmented.tsx` |
| [`RemovableChip`](./RemovableChip.md) | canonical/shipped | Chip with × removal affordance (distinct from `Pill`) | `src/components/ui/RemovableChip.tsx` |
| [`GlanceCell`](./GlanceCell.md) | implemented/unintegrated | Single key/value stat cell (composed in `GlanceRow`) | `src/components/ui/GlanceCell.tsx` |

### Wave 4 (Meeting Detail substrate, 0.4.0)

| Name | Status | Job (one line) | Source |
|---|---|---|---|
| [`MeetingStatusPill`](./MeetingStatusPill.md) | canonical/shipped | Meeting temporal state (`upcoming` / `in-progress` / `past` / `cancelled`) | `src/components/meeting/MeetingStatusPill.tsx` |

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
