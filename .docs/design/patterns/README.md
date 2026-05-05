# Patterns

Composed, opinionated, named after the job they do. **This is where drift happens** — when two surfaces re-implement nearly-identical UI with small variations, that's a missing pattern. Promote it.

A pattern knows about a domain concept (a claim, a trust state, a briefing, a meeting). A primitive doesn't.

## Status vocabulary

- **canonical/shipped** — shared source component consumed by shipped routed UI.
- **shipped-local/extraction-needed** — real shipped UI, but still inline, surface-local, or not extracted to a named reusable component.
- **implemented/unintegrated** — source exists under this name, but no shipped routed surface consumes it yet.
- **roadmap/planned** — no shipped source component under this name.

## Index

### Wave 1 (v1.4.3 substrate, 0.1.0)

| Name | Status | Job (one line) | Consumers |
|---|---|---|---|
| [`FolioBar`](./FolioBar.md) | canonical/shipped | Top frosted bar with crumbs + actions | every editorial surface |
| [`FloatingNavIsland`](./FloatingNavIsland.md) | canonical/shipped | Dual-pill app + chapter nav (the canonical local-nav pattern) | every stable surface |
| [`AtmosphereLayer`](./AtmosphereLayer.md) | canonical/shipped | Page-tinted radial gradient background | every editorial surface |
| [`MarginGrid`](./MarginGrid.md) | canonical/shipped | Two-column section layout (margin label + content) | DailyBriefing, AccountDetail |
| [`ChapterHeading`](./ChapterHeading.md) | canonical/shipped | Section opener (heavy rule + serif title + epigraph) | AccountDetail, ProjectDetail, etc. |
| [`Lead`](./Lead.md) | roadmap/planned | Single-sentence editorial headline | DailyBriefing |
| [`DayChart`](./DayChart.md) | implemented/unintegrated | Visual day shape (hour ticks + bars + NOW line) | D-spine prototype only |
| [`MeetingSpineItem`](./MeetingSpineItem.md) | roadmap/planned | Magazine-style meeting list entry | DailyBriefing |
| [`MeetingCard`](./MeetingCard.md) | canonical/shipped | Shared schedule/timeline meeting row | DailyBriefing, WeekPage |
| [`BriefingMeetingCard`](./BriefingMeetingCard.md) | canonical/shipped | Expandable briefing schedule row with prep details | DailyBriefing |
| [`DailyBriefingAttentionSection`](./DailyBriefingAttentionSection.md) | shipped-local/extraction-needed | Priority/action/email/lifecycle section used in the briefing | DailyBriefing |
| [`EntityPortraitCard`](./EntityPortraitCard.md) | implemented/unintegrated | Color-banded entity portrait with thread | D-spine prototype only |
| [`ThreadMark`](./ThreadMark.md) | implemented/unintegrated | Universal "talk about this" hover affordance | D-spine prototype only |
| [`AskAnythingDock`](./AskAnythingDock.md) | implemented/unintegrated | Multi-line conversational dock | D-spine prototype only |

### Wave 2 (v1.4.4 trust UI, 0.2.0)

| Name | Status | Job (one line) | Consumers |
|---|---|---|---|
| [`TrustBand`](./TrustBand.md) | implemented/unintegrated | Canonical trust signal cluster (TrustBandBadge + ProvenanceTag + FreshnessIndicator) | source-only trust stack |
| [`ClaimRow`](./ClaimRow.md) | implemented/unintegrated | Single-claim render API (the unit consumed everywhere) | source-only trust stack |
| [`ReceiptCallout`](./ReceiptCallout.md) | implemented/unintegrated | v1.4.4 inspection layer; expandable receipt with resolver bands + corrections | source-only trust stack |
| [`AboutThisIntelligencePanel`](./AboutThisIntelligencePanel.md) | shipped-local/extraction-needed | Chapter-level coverage / freshness / source explanation | shipped as `AboutIntelligence`/local panels |
| [`DossierSourceCoveragePanel`](./DossierSourceCoveragePanel.md) | shipped-local/extraction-needed | Dossier-level source coverage explanation | shipped as `AboutThisDossier` |
| [`StaleReportBanner`](./StaleReportBanner.md) | shipped-local/extraction-needed | Banner above generated reports when underlying intel has gone stale | inline in report shells |
| [`ConsistencyFindingBanner`](./ConsistencyFindingBanner.md) | shipped-local/extraction-needed | Inline banner when trust compiler flags consistency findings | inline in MeetingDetail |

### Wave 3 (Settings substrate, 0.3.0)

| Name | Status | Job (one line) | Consumers |
|---|---|---|---|
| [`SurfaceMasthead`](./SurfaceMasthead.md) | canonical/shipped | Generic surface-top block (eyebrow + title + lede + accessory + glance slot) | Settings |
| [`FormRow`](./FormRow.md) | canonical/shipped | Universal label/help \| ctrl \| aux row | Settings (canonical), future settings-like surfaces |
| [`GlanceRow`](./GlanceRow.md) | implemented/unintegrated | Horizontal row of GlanceCell instances inside SurfaceMasthead's glance slot | source-only prototype |
| [`VitalsStrip`](./VitalsStrip.md) | canonical/shipped | Inline entity vital metrics with dot separators, highlights, and optional source attribution | AccountDetail, account/person/project editorial |
| [`EntityListShell`](./EntityListShell.md) | canonical/shipped | Shared entity-list header/search/tabs/end-state shell | Accounts, People, Projects |
| [`EntityRow`](./EntityRow.md) | canonical/shipped | Shared entity list row with accent/avatar, title, subtitle, and meta | Accounts, People, Projects |
| [`AccountViewSwitcher`](./AccountViewSwitcher.md) | canonical/shipped | Fixed bottom pill switcher for AccountDetail views | AccountDetail |
| [`SettingsSections`](./SettingsSections.md) | shipped-local/extraction-needed | Real settings section stack and controls from `features/settings-ui` | Settings |
| [`YouCard`](./YouCard.md) | canonical/shipped | User identity/role/workspace settings pattern | Settings |
| [`ActivityLogSection`](./ActivityLogSection.md) | canonical/shipped | Filtered audit log viewer | Settings |
| [`DiagnosticsSection`](./DiagnosticsSection.md) | canonical/shipped | Developer/system diagnostic cards and controls | Settings |
| [`OnboardingFlow`](./OnboardingFlow.md) | canonical/shipped | Shipped onboarding chapter sequence and chrome | Onboarding |

### Wave 4 (Meeting Detail substrate, 0.4.0)

| Name | Status | Job (one line) | Consumers |
|---|---|---|---|
| [`SuggestedActionRow`](./SuggestedActionRow.md) | canonical/shipped | AI-suggested action item with Accept/Dismiss | ActionsPage |
| [`FolioActions`](./FolioActions.md) | roadmap/planned | Sub-row of action buttons below FolioBar (actions, not nav per D2) | MeetingDetail (canonical), Reports |
| [`PostMeetingIntelligence`](./PostMeetingIntelligence.md) | canonical/shipped | Actual post-meeting recap system composing threads, predictions, signals, findings, commitments, and role changes | MeetingDetail |
| [`TalkBalanceBar`](./TalkBalanceBar.md) | canonical/shipped | Customer/internal talk ratio bar | MeetingDetail |
| [`ActionRow`](./ActionRow.md) | canonical/shipped | Shared action row variants for list, compact, and meeting outcomes | ActionsPage, MeetingDetail, TheWork |
| [`IntelligenceFeedback`](./IntelligenceFeedback.md) | canonical/shipped | Legacy inline helpful/not-helpful feedback control | MeetingDetail, reports, entity chapters |
| [`FinisMarker`](./FinisMarker.md) | canonical/shipped | Editorial end marker with optional freshness timestamp | editorial surfaces |
| [`AgendaThreadList`](./AgendaThreadList.md) | shipped-local/extraction-needed | Predicted agenda items checked off post-meeting (✓ / ○ / +) | `PostMeetingIntelligence` local class family |
| [`PredictionsVsRealityGrid`](./PredictionsVsRealityGrid.md) | shipped-local/extraction-needed | Two-column risks/wins comparison vs briefing predictions | `PostMeetingIntelligence` local class family |
| [`SignalGrid`](./SignalGrid.md) | shipped-local/extraction-needed | 2x2 stats grid (Question density, Decision maker, Forward-looking, Monologue risk) | `PostMeetingIntelligence` local class family |
| [`EscalationQuote`](./EscalationQuote.md) | shipped-local/extraction-needed | Highlighted attributed quote where the room turned | `PostMeetingIntelligence` local class family |
| [`FindingsTriad`](./FindingsTriad.md) | shipped-local/extraction-needed | Wins / Risks / Decisions evidence groups | `PostMeetingIntelligence` local class family |
| [`ChampionHealthBlock`](./ChampionHealthBlock.md) | shipped-local/extraction-needed | Champion relationship state + evidence + risk paragraph | `PostMeetingIntelligence` local class family |
| [`CommitmentRow`](./CommitmentRow.md) | shipped-local/extraction-needed | Captured commitment with YOURS / THEIRS tag | `PostMeetingIntelligence` local class family |
| [`RoleTransitionRow`](./RoleTransitionRow.md) | shipped-local/extraction-needed | Person role transition (before-status → after-status pill chain) | `PostMeetingIntelligence` local class family |

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
