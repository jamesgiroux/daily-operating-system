# Patterns

Composed, opinionated, named after the job they do. **This is where drift happens** — when two surfaces re-implement nearly-identical UI with small variations, that's a missing pattern. Promote it.

A pattern knows about a domain concept (a claim, a trust state, a briefing, a meeting). A primitive doesn't.

## Index

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

_Wave 2 will add: TrustBand, ClaimRow, AboutThisIntelligencePanel, DossierSourceCoveragePanel, ReceiptCallout, StaleReportBanner, ConsistencyFindingBanner._

_Wave 3 will add: FormRow, SurfaceMasthead, GlanceRow._

_Wave 4 will add: MeetingHero, FolioActions, AgendaThreadList, PredictionsVsRealityGrid, SignalGrid, EscalationQuote, FindingsTriad, ChampionHealthBlock, CommitmentRow, SuggestedActionRow, RoleTransitionRow._

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
