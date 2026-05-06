# Primitives

The smallest reusable units. Button, Input, Card shell, Pill, Chip, Avatar. Generic, unopinionated, no domain knowledge.

A primitive is *not* a primitive if it knows about claims, trust, briefings, or any DailyOS concept. That's a pattern.

## Status vocabulary

- **proposed** — WIP, prototype, roadmap, or source-only work that is not yet integrated into routed app UI.
- **integrated** — real app code exists and is used in the product, including shared components, page-local classes, or extracted modules.
- **production** — integrated and included in a tagged release.

## Index

### Wave 1 (v1.4.3 substrate, 0.1.0)

| Name | Status | Job (one line) | Source |
|---|---|---|---|
| [`Pill`](./Pill.md) | integrated | Visual primitive for inline status / label / category badges | `src/components/ui/Pill.tsx` |
| [`HealthBadge`](./HealthBadge.md) | integrated | Shared health score dot/score/trend visual | `src/components/shared/HealthBadge.tsx` |
| [`StatusDot`](./StatusDot.md) | integrated | Connector/system status dot with optional label | `src/components/shared/StatusDot.tsx` |
| [`Avatar`](./Avatar.md) | integrated | Person photo/initial fallback avatar | `src/components/ui/Avatar.tsx` |
| [`TrustBandBadge`](./TrustBandBadge.md) | proposed | v1.4.0 surface trust band (`likely_current` / `use_with_caution` / `needs_verification`) | `src/components/ui/TrustBandBadge.tsx` |
| [`IntelligenceQualityBadge`](./IntelligenceQualityBadge.md) | integrated | Intelligence completeness (`sparse` / `developing` / `ready` / `fresh`) | `src/components/entity/` |
| [`FreshnessIndicator`](./FreshnessIndicator.md) | integrated | Raw recency timestamp + relative age | `src/components/ui/FreshnessIndicator.tsx` |
| [`ProvenanceTag`](./ProvenanceTag.md) | integrated | Source attribution label, suppresses synthesized | `src/components/ui/` |
| [`EntityChip`](./EntityChip.md) | integrated | Entity reference with entity-type color | `src/components/ui/EntityChip.tsx` |
| [`TypeBadge`](./TypeBadge.md) | integrated | Account-type categorical (Customer / Internal / Partner) | `_shared/.type-badge` + AccountHero |

### Wave 2 (v1.4.4 trust UI, 0.2.0)

| Name | Status | Job (one line) | Source |
|---|---|---|---|
| [`SourceCoverageLine`](./SourceCoverageLine.md) | proposed | Compact line summarizing source coverage (e.g., "Glean · 4 sources · 2 stale") | `src/components/ui/SourceCoverageLine.tsx` |
| [`ConfidenceScoreChip`](./ConfidenceScoreChip.md) | proposed | Numerical confidence score chip with threshold-based tone | `src/components/ui/ConfidenceScoreChip.tsx` |
| [`VerificationStatusFlag`](./VerificationStatusFlag.md) | proposed | Consistency state per v1.4.0 (`ok` / `corrected` / `flagged`) | `src/components/ui/VerificationStatusFlag.tsx` |
| [`DataGapNotice`](./DataGapNotice.md) | proposed | Inline warning that intelligence is missing critical inputs | `src/components/ui/DataGapNotice.tsx` |
| [`AsOfTimestamp`](./AsOfTimestamp.md) | proposed | Static "as of" timestamp label (companion to FreshnessIndicator) | `src/components/ui/AsOfTimestamp.tsx` |

### Wave 3 (Settings substrate, 0.3.0)

| Name | Status | Job (one line) | Source |
|---|---|---|---|
| [`InlineInput`](./InlineInput.md) | proposed | Click-to-edit text input with pencil affordance | target `src/components/ui/InlineInput.tsx` |
| [`EditableText`](./EditableText.md) | integrated | Click-to-edit display text used by meeting, entity, and report surfaces | `src/components/ui/EditableText.tsx` |
| [`FolioRefreshButton`](./FolioRefreshButton.md) | integrated | Editorial mono refresh/run button used in folio and hero action areas | `src/components/ui/folio-refresh-button.tsx` |
| [`Switch`](./Switch.md) | integrated | Aria-checked toggle button | `src/components/ui/Switch.tsx` |
| [`Segmented`](./Segmented.md) | integrated | Tinted button group with `aria-pressed` state | `src/components/ui/Segmented.tsx` |
| [`RemovableChip`](./RemovableChip.md) | integrated | Chip with × removal affordance (distinct from `Pill`) | `src/components/ui/RemovableChip.tsx` |
| [`GlanceCell`](./GlanceCell.md) | proposed | Single key/value stat cell (composed in `GlanceRow`) | `src/components/ui/GlanceCell.tsx` |

### Wave 4 (Meeting Detail substrate, 0.4.0)

| Name | Status | Job (one line) | Source |
|---|---|---|---|
| [`MeetingStatusPill`](./MeetingStatusPill.md) | integrated | Meeting temporal state (`upcoming` / `in-progress` / `past` / `cancelled`) | `src/components/meeting/MeetingStatusPill.tsx` |

### Daily Briefing redesign (0.6.0)

| Name | Status | Job (one line) | Source |
|---|---|---|---|
| [`SignalDot`](./SignalDot.md) | proposed | Tinted-dot signal-feed bullet (8 kinds: meeting, action, email, lifecycle, gongCall, zendeskTicket, slackThread, linearIssue) | target `src/components/dashboard/SignalDot.tsx` (W1) |
| [`ProvenanceStat`](./ProvenanceStat.md) | proposed | Labeled metric with optional trend tint (up/down/flat) | target `src/components/dashboard/ProvenanceStat.tsx` (W1) |

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
