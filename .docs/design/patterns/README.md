# Patterns

Composed, opinionated, named after the job they do. **This is where drift happens** — when two surfaces re-implement nearly-identical UI with small variations, that's a missing pattern. Promote it.

A pattern knows about a domain concept (a claim, a trust state, a briefing, a meeting). A primitive doesn't.

## Status vocabulary

- **proposed** — WIP, prototype, roadmap, reference-only, or source-only work that is not yet used by routed app UI.
- **integrated** — real app code exists and is used in the product, including shared components, page-local classes, or extracted components.
- **production** — integrated and included in a tagged release. Move a pattern here only after the release tag exists.

## Index

### Wave 1 (v1.4.3 substrate, 0.1.0)

| Name | Status | Job (one line) | Consumers |
|---|---|---|---|
| [`FolioBar`](./FolioBar.md) | integrated | Top frosted bar with crumbs + actions | every editorial surface |
| [`FloatingNavIsland`](./FloatingNavIsland.md) | integrated | Dual-pill app + chapter nav (the canonical local-nav pattern) | every stable surface |
| [`AtmosphereLayer`](./AtmosphereLayer.md) | integrated | Page-tinted radial gradient background | every editorial surface |
| [`MarginGrid`](./MarginGrid.md) | integrated | Two-column section layout (margin label + content) | Daily Briefing, AccountDetail |
| [`ChapterHeading`](./ChapterHeading.md) | integrated | Section opener (heavy rule + serif title + epigraph) | AccountDetail, ProjectDetail, etc. |
| [`Lead`](./Lead.md) | proposed | Single-sentence editorial headline | Daily Briefing |
| [`DayStrip`](./DayStrip.md) | proposed | Previous/current/next briefing-day navigation under FolioBar | Daily Briefing redesign reference candidate |
| [`DayChart`](./DayChart.md) | proposed | Visual day shape (hour ticks + bars + NOW line) | Daily Briefing redesign prototype only |
| [`MeetingSpineItem`](./MeetingSpineItem.md) | proposed | Magazine-style meeting list entry | Daily Briefing redesign reference candidate; source component exists but is not routed |
| [`InferredActionSelector`](./InferredActionSelector.md) | proposed | Suggested action with a dropdown of alternatives | Daily Briefing redesign reference candidate |

### Wave 2 (v1.4.4 trust UI, 0.2.0)

| Name | Status | Job (one line) | Consumers |
|---|---|---|---|
| [`TrustBand`](./TrustBand.md) | proposed | Canonical trust signal cluster (TrustBandBadge + ProvenanceTag + FreshnessIndicator) | source-only trust stack |
| [`ClaimRow`](./ClaimRow.md) | proposed | Single-claim render API (the unit consumed everywhere) | source-only trust stack |
| [`ReceiptCallout`](./ReceiptCallout.md) | proposed | v1.4.4 inspection layer; expandable receipt with resolver bands + corrections | source-only trust stack |
| [`AboutThisIntelligencePanel`](./AboutThisIntelligencePanel.md) | integrated | Chapter-level coverage / freshness / source explanation | shipped as `AboutIntelligence`/local panels |
| [`DossierSourceCoveragePanel`](./DossierSourceCoveragePanel.md) | integrated | Dossier-level source coverage explanation | shipped as `AboutThisDossier` |
| [`StaleReportBanner`](./StaleReportBanner.md) | integrated | Banner above generated reports when underlying intel has gone stale | inline in report shells |
| [`ConsistencyFindingBanner`](./ConsistencyFindingBanner.md) | integrated | Inline banner when trust compiler flags consistency findings | inline in MeetingDetail |

### Wave 3 (Settings substrate, 0.3.0)

| Name | Status | Job (one line) | Consumers |
|---|---|---|---|
| [`SurfaceMasthead`](./SurfaceMasthead.md) | integrated | Generic surface-top block (eyebrow + title + lede + accessory + glance slot) | Settings |
| [`FormRow`](./FormRow.md) | integrated | Universal label/help \| ctrl \| aux row | Settings (canonical), future settings-like surfaces |
| [`GlanceRow`](./GlanceRow.md) | proposed | Horizontal row of GlanceCell instances inside SurfaceMasthead's glance slot | source-only prototype |
| [`VitalsStrip`](./VitalsStrip.md) | integrated | Inline entity vital metrics with dot separators, highlights, and optional source attribution | AccountDetail, account/person/project editorial |
| [`EntityListShell`](./EntityListShell.md) | integrated | Shared entity-list header/search/tabs/end-state shell | Accounts, People, Projects |
| [`EntityRow`](./EntityRow.md) | integrated | Shared entity list row with accent/avatar, title, subtitle, and meta | Accounts, People, Projects |
| [`AccountViewSwitcher`](./AccountViewSwitcher.md) | integrated | Fixed bottom pill switcher for AccountDetail views | AccountDetail |
| [`SettingsSections`](./SettingsSections.md) | integrated | Real settings section stack and controls from `features/settings-ui` | Settings |
| [`YouCard`](./YouCard.md) | integrated | User identity/role/workspace settings pattern | Settings |
| [`ActivityLogSection`](./ActivityLogSection.md) | integrated | Filtered audit log viewer | Settings |
| [`DiagnosticsSection`](./DiagnosticsSection.md) | integrated | Developer/system diagnostic cards and controls | Settings |
| [`OnboardingFlow`](./OnboardingFlow.md) | integrated | Shipped onboarding chapter sequence and chrome | Onboarding |

### Wave 4 (Meeting Detail substrate, 0.4.0)

| Name | Status | Job (one line) | Consumers |
|---|---|---|---|
| [`SuggestedActionRow`](./SuggestedActionRow.md) | integrated | AI-suggested action item with Accept/Dismiss | ActionsPage |
| [`FolioActions`](./FolioActions.md) | proposed | Sub-row of action buttons below FolioBar (actions, not nav per D2) | MeetingDetail (canonical), Reports |
| [`PostMeetingIntelligence`](./PostMeetingIntelligence.md) | integrated | Actual post-meeting recap system composing threads, predictions, signals, findings, commitments, and role changes | MeetingDetail |
| [`TalkBalanceBar`](./TalkBalanceBar.md) | integrated | Customer/internal talk ratio bar | MeetingDetail |
| [`ActionRow`](./ActionRow.md) | integrated | Shared action row variants for list, compact, and meeting outcomes | ActionsPage, MeetingDetail, TheWork |
| [`IntelligenceFeedback`](./IntelligenceFeedback.md) | integrated | Legacy inline helpful/not-helpful feedback control | MeetingDetail, reports, entity chapters |
| [`FinisMarker`](./FinisMarker.md) | integrated | Editorial end marker with optional freshness timestamp | editorial surfaces |
| [`AgendaThreadList`](./AgendaThreadList.md) | integrated | Predicted agenda items checked off post-meeting (✓ / ○ / +) | `PostMeetingIntelligence` local class family |
| [`PredictionsVsRealityGrid`](./PredictionsVsRealityGrid.md) | integrated | Two-column risks/wins comparison vs briefing predictions | `PostMeetingIntelligence` local class family |
| [`SignalGrid`](./SignalGrid.md) | integrated | 2x2 stats grid (Question density, Decision maker, Forward-looking, Monologue risk) | `PostMeetingIntelligence` local class family |
| [`EscalationQuote`](./EscalationQuote.md) | integrated | Highlighted attributed quote where the room turned | `PostMeetingIntelligence` local class family |
| [`FindingsTriad`](./FindingsTriad.md) | integrated | Wins / Risks / Decisions evidence groups | `PostMeetingIntelligence` local class family |
| [`ChampionHealthBlock`](./ChampionHealthBlock.md) | integrated | Champion relationship state + evidence + risk paragraph | `PostMeetingIntelligence` local class family |
| [`CommitmentRow`](./CommitmentRow.md) | integrated | Captured commitment with YOURS / THEIRS tag | `PostMeetingIntelligence` local class family |
| [`RoleTransitionRow`](./RoleTransitionRow.md) | integrated | Person role transition (before-status → after-status pill chain) | `PostMeetingIntelligence` local class family |

### Daily Briefing redesign (0.6.0)

| Name | Status | Job (one line) | Consumers |
|---|---|---|---|
| [`MovingRow`](./MovingRow.md) | proposed | Three-column entity-movement row (identity / lede + signals / stats); 5 kinds | Daily Briefing (Moving section) |
| [`WatchRow`](./WatchRow.md) | proposed | Adaptive triage row, 4 kinds (suggestedAction / openAction / parked / aging) | Daily Briefing (Watch section) |
| [`PredictionsSection`](./PredictionsSection.md) | proposed | Collapsed-by-default predictions list within MarginGrid | Daily Briefing |
| [`BriefingLoadingState`](./BriefingLoadingState.md) | proposed | Centered editorial holding state with optional pulsing dot | Daily Briefing (and future editorial surfaces) |
| [`BriefingErrorState`](./BriefingErrorState.md) | proposed | Centered editorial error frame with retry / diagnostics affordances | Daily Briefing (and future editorial surfaces) |
| [`BriefingEmptyState`](./BriefingEmptyState.md) | proposed | Left-aligned cold-start frame with eyebrow / headline / lede / checklist / CTA | Daily Briefing (and future editorial surfaces) |

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
