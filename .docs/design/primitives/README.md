# Primitives

The smallest reusable units. Button, Input, Card shell, Pill, Chip, Avatar. Generic, unopinionated, no domain knowledge.

A primitive is *not* a primitive if it knows about claims, trust, briefings, or any DailyOS concept. That's a pattern.

## Index

| Name | Status | Job (one line) | Source |
|---|---|---|---|
| [`Pill`](./Pill.md) | canonical | Visual primitive for inline status / label / category badges | `_shared/.pill` |
| [`TrustBandBadge`](./TrustBandBadge.md) | proposed | v1.4.0 surface trust band (`likely_current` / `use_with_caution` / `needs_verification`) | new (DOS-320 contract) |
| [`IntelligenceQualityBadge`](./IntelligenceQualityBadge.md) | canonical | Intelligence completeness (`sparse` / `developing` / `ready` / `fresh`) | `src/components/entity/` |
| [`FreshnessIndicator`](./FreshnessIndicator.md) | proposed | Raw recency timestamp + relative age | new (`source_asof` contract) |
| [`ProvenanceTag`](./ProvenanceTag.md) | canonical | Source attribution label, suppresses synthesized | `src/components/ui/` |
| [`EntityChip`](./EntityChip.md) | canonical | Entity reference with entity-type color | `src/components/ui/{meeting,email}-entity-chip.tsx` |
| [`TypeBadge`](./TypeBadge.md) | canonical | Account-type categorical (Customer / Internal / Partner) | `_shared/.type-badge` + AccountHero |

_Wave 2 will add: FreshnessChip (consolidate with FreshnessIndicator), SourceCoverageLine, ConfidenceScoreChip, VerificationStatusFlag, DataGapNotice, AsOfTimestamp._

_Wave 3 will add: InlineInput, Switch, Segmented, RemovableChip, GlanceCell._

_Wave 4 will add: MeetingStatusPill._

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
