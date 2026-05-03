# Patterns

Composed, opinionated, named after the job they do. **This is where drift happens** — when two surfaces re-implement nearly-identical UI with small variations, that's a missing pattern. Promote it.

A pattern knows about a domain concept (a claim, a trust state, a briefing, a meeting). A primitive doesn't.

## Index

### Wave 1 (v1.4.3 substrate, 0.1.0)

| Name | Status | Job (one line) | Consumers |
|---|---|---|---|
| [`FolioBar`](./FolioBar.md) | canonical | Top frosted bar with crumbs + actions | every editorial surface |
| [`FloatingNavIsland`](./FloatingNavIsland.md) | canonical | Dual-pill app + chapter nav (the canonical local-nav pattern) | every stable surface |
| [`AtmosphereLayer`](./AtmosphereLayer.md) | canonical | Page-tinted radial gradient background | every editorial surface |
| [`MarginGrid`](./MarginGrid.md) | canonical | Two-column section layout (margin label + content) | DailyBriefing, AccountDetail |
| [`ChapterHeading`](./ChapterHeading.md) | canonical | Section opener (heavy rule + serif title + epigraph) | AccountDetail, ProjectDetail, etc. |
| [`Lead`](./Lead.md) | proposed | Single-sentence editorial headline | DailyBriefing |
| [`DayChart`](./DayChart.md) | proposed | Visual day shape (hour ticks + bars + NOW line) | DailyBriefing |
| [`MeetingSpineItem`](./MeetingSpineItem.md) | proposed | Magazine-style meeting list entry | DailyBriefing |
| [`EntityPortraitCard`](./EntityPortraitCard.md) | proposed | Color-banded entity portrait with thread | DailyBriefing |
| [`ThreadMark`](./ThreadMark.md) | proposed | Universal "talk about this" hover affordance | DailyBriefing (cross-version) |
| [`AskAnythingDock`](./AskAnythingDock.md) | proposed | Multi-line conversational dock | DailyBriefing (cross-version) |

### Wave 2 (v1.4.4 trust UI, 0.2.0)

| Name | Status | Job (one line) | Consumers |
|---|---|---|---|
| [`TrustBand`](./TrustBand.md) | proposed | Canonical trust signal cluster (TrustBandBadge + ProvenanceTag + FreshnessIndicator) | every claim-rendering surface |
| [`ClaimRow`](./ClaimRow.md) | proposed | Single-claim render API (the unit consumed everywhere) | every claim-rendering surface |
| [`ReceiptCallout`](./ReceiptCallout.md) | proposed | v1.4.4 inspection layer; expandable receipt with resolver bands + corrections | every claim-rendering surface |
| [`AboutThisIntelligencePanel`](./AboutThisIntelligencePanel.md) | proposed | Chapter-level coverage / freshness / source explanation | DailyBriefing chapters, entity surfaces |
| [`DossierSourceCoveragePanel`](./DossierSourceCoveragePanel.md) | proposed | Dossier-level source coverage explanation | AccountDetail, ProjectDetail, PersonDetail |
| [`StaleReportBanner`](./StaleReportBanner.md) | proposed | Banner above generated reports when underlying intel has gone stale | report surfaces |
| [`ConsistencyFindingBanner`](./ConsistencyFindingBanner.md) | proposed | Inline banner when v1.4.0 trust compiler flags consistency findings | claim-rendering surfaces |

### Wave 3 (Settings substrate, 0.3.0)

| Name | Status | Job (one line) | Consumers |
|---|---|---|---|
| [`SurfaceMasthead`](./SurfaceMasthead.md) | proposed | Generic surface-top block (eyebrow + title + lede + accessory + glance slot) | Settings, MeetingDetail (subsumes MeetingHero) |
| [`FormRow`](./FormRow.md) | proposed | Universal label/help \| ctrl \| aux row | Settings (canonical), future settings-like surfaces |
| [`GlanceRow`](./GlanceRow.md) | proposed | Horizontal row of GlanceCell instances inside SurfaceMasthead's glance slot | Settings masthead |

### Wave 4 (Meeting Detail substrate, 0.4.0)

| Name | Status | Job (one line) | Consumers |
|---|---|---|---|
| [`SuggestedActionRow`](./SuggestedActionRow.md) | proposed | AI-suggested action item with Accept/Dismiss; meeting + work contexts | MeetingDetail, AccountDetail Work surface |
| [`FolioActions`](./FolioActions.md) | proposed | Sub-row of action buttons below FolioBar (actions, not nav per D2) | MeetingDetail (canonical), Reports |
| [`AgendaThreadList`](./AgendaThreadList.md) | proposed | Predicted agenda items checked off post-meeting (✓ / ○ / +) | MeetingDetail |
| [`PredictionsVsRealityGrid`](./PredictionsVsRealityGrid.md) | proposed | Two-column risks/wins comparison vs briefing predictions | MeetingDetail |
| [`SignalGrid`](./SignalGrid.md) | proposed | 2x2 stats grid (Question density, Decision maker, Forward-looking, Monologue risk) | MeetingDetail |
| [`EscalationQuote`](./EscalationQuote.md) | proposed | Highlighted attributed quote where the room turned | MeetingDetail |
| [`FindingsTriad`](./FindingsTriad.md) | proposed | Three-column Wins / Risks / Decisions cards with evidence quotes | MeetingDetail |
| [`ChampionHealthBlock`](./ChampionHealthBlock.md) | proposed | Champion relationship state + evidence + risk paragraph | MeetingDetail |
| [`CommitmentRow`](./CommitmentRow.md) | proposed | Captured commitment with YOURS / THEIRS tag | MeetingDetail |
| [`RoleTransitionRow`](./RoleTransitionRow.md) | proposed | Person role transition (before-status → after-status pill chain) | MeetingDetail |

## Conventions

- **Named after the job, not the surface.** `TrustBand`, not `BriefingTrustBand`. If a pattern is unique to one surface, it's probably surface-internal and doesn't need promotion yet.
- **PascalCase.** No suffixes like `Component`, `Container`, `Wrapper`.
- **Composes primitives.** A pattern that doesn't compose primitives is suspicious — it might be a primitive itself.
- **Has a clear API.** Pattern specs document the input/output/customization surface.
- **Variants are first-class.** A pattern with 4 variants is fine. A pattern with 4 forks across 4 surfaces is a bug.

## Adding a pattern

1. Confirm: does this appear (or will it appear) in 2+ surfaces? If only one surface uses it, it's surface-internal.
2. Copy `../_TEMPLATE-entry.md` here.
3. Fill out **Composition** (which primitives) and **Variants** carefully — these are the contract.
4. List every consuming surface in **Surfaces that consume it**.
5. If you're consolidating drift, note the previous variants and where they lived in **History**.

## Reviewing for drift

If you're auditing or reviewing a PR, ask:

1. Is this UI also rendered somewhere else with small differences? → likely a missing pattern.
2. Does this pattern's spec match every consumer's actual usage? → if not, the spec is stale or the consumers drifted.
3. Did a recent PR add a "private" component that should be a pattern? → promote it.
