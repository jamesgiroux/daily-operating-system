# 0129 — Briefing view-model contract

**Date:** 2026-05-06
**Status:** proposed (lands with DOS-413, becomes accepted on W0 merge)
**Linear ticket:** DOS-413
**Plan:** `.docs/plans/daily-briefing-redesign-W0/DOS-413-plan.md`
**Supersedes:** none

## Context

The Daily Briefing redesign track ships a new top-level surface that composes Lead, Schedule, Predictions, Moving, and Watch sections. The current `DailyBriefing.tsx` is 1032 lines and shapes data inline (email ranking, calendar selection, today/past/future grouping, useState filtering). The redesign goal is "no logic in the display layer." For that to be enforceable, the briefing data needs a single canonical TypeScript shape that the view consumes pass-through.

## Decision

Define `BriefingViewModel` and surrounding types in `src/types/briefing.ts`, exposed via a single `get_briefing_view_model` Tauri command returning `BriefingLoadState` (the existing `useDashboardData` envelope pattern, applied to briefing data).

The contract is read-only. Mutations happen via separate named services (`actions::*`, `claims::*`, `predictions::*`).

## Key choices

### One Tauri command, atomic semantics

The contract's `BriefingLoadState` is `loading | error | empty | success`. One command produces it. Failure of any internal section ⇒ top-level `error`. No `partial_data` state — a half-assembled briefing is more confusing than the absent one. If partial-success ever becomes useful, add `degraded?: BriefingSectionId[]` to `success` then.

### Trust as first-class via TrustMixin

Every fact-bearing view-model type extends a strict named `TrustMixin` interface:

```ts
interface TrustMixin {
  trustBand: TrustBandWire;          // required
  trustFieldPath?: string;
  trustSourceDate?: string | null;
  renderedProvenance?: RenderedProvenanceSummary | null;
}
```

Stricter than the existing `TrustAnnotated<T>` (which marks `trustBand` optional) by design — the redesign requires all fact-bearing fields carry a band. W2 services emit `unscored` initially; DOS-320/411 fill real values.

Trust rollout policy: **W2 services may ship with `unscored` defaults.** The non-`unscored` happy-path test gates **W4 merge**, not W2. (Reconciles a contradiction surfaced in L0 round 2.)

### LifecycleMixin separated from TrustMixin

Claim-correction state (`correctionState: "none" | "corrected" | "contested"`) lives in `LifecycleMixin`, applied only to types whose underlying claim has a correction surface — signal feed items, suggested-action rows. Open actions, parked items, aging rows don't carry it.

### Headline split, not concatenated

`LeadViewModel.headline = { lead: string; punchLine?: string }`. The view wraps `punchLine` in an emphasis span if present. Never concatenates.

### Service-rendered text everywhere

Every section carries pre-rendered editorial copy as named fields: section headings (`heading`), margin labels (`label`), pre-pluralized counts (`countLabel`), summaries (`summary`), predictions collapsed copy (`collapsedLabel` + `expandHint`), signal "what" segments. The view never composes strings.

### `whatSegments` typed text

Signal feed items store text as `whatSegments: { text: string; emphasized?: boolean }[]` so inline italic spans are recoverable from the contract. Avoids regex-parsing rendered strings.

### Semantic enums, not paint names

`ReadinessSemantic = "healthy" | "needs_attention" | "in_progress" | "blocked" | "neutral"` (was `ReadinessColor` paint enum). Pattern owns paint mapping. Enables future palette swaps without contract changes.

### Service owns route construction

`DayStripNeighbor.href`, `BriefingActionView.href` carry route strings produced by the service. Clients are passive on routing. Cost: route changes require service updates. Benefit: no client-side route composition (matches the existing dashboard pattern).

### Eager predictions payload

`PredictionsViewModel.predictions[]` is always populated, even in collapsed default state. Trade-off: payload size grows with prediction count. Rationale: prediction lists are small (≤10), expand intent is friction-free, lazy fetch on expand would add a loading state and round-trip the user wouldn't anticipate.

## Token additions

`tokens/color.md` and `src/styles/design-tokens.css` gain `--color-signal-{kind}` aliases for the 8 SignalDot kinds. Some share paint with semantic entity tokens (e.g. `--color-signal-meeting` shares `--color-garden-larkspur` with `--color-person`). This is deliberate: signals appear only in the Moving feed, never on entity surfaces, so cross-surface collision doesn't render.

## Typography contract appendix

Each service-rendered string field paired with typography register, length budget, tone register:

| Field | Register | Length budget | Tone |
|---|---|---|---|
| `Lead.headline.lead` | serif display, 38–58px depending on width | ≤20 words | declarative; states the day |
| `Lead.headline.punchLine` | serif display, same scale, turmeric emphasis | ≤12 words | sharp; the one thing to nail |
| `Lead.focusCapacity` | mono, 12–13px | ≤80 chars | observational |
| `Schedule.heading` | serif h2, 32px | ≤4 words | naming |
| `Schedule.summary` | serif italic 16px, 300 weight | ≤2 sentences, ≤140 chars | editorial |
| `Schedule.countLabel` | mono uppercase 11px | ≤16 chars | terse count |
| `ScheduleMeeting.context` | serif 14–15px | ≤2 sentences, ≤200 chars | editorial; what to know going in |
| `Predictions.label` | mono uppercase 11px | "Predictions" | terse |
| `Predictions.countLabel` | mono uppercase 11px | "N today" | terse count |
| `Predictions.collapsedLabel` | sans 14px | "N predictions today" | observational |
| `Predictions.expandHint` | mono uppercase 11px | "expand" / "collapse" | affordance label |
| `PredictionItem.text` | serif 17px | ≤2 sentences, ≤200 chars | hypothesis |
| `Moving.heading` | serif h2, 32px | ≤3 words | naming |
| `Moving.summary` | serif italic 16px, 300 weight | ≤1 sentence, ≤120 chars | editorial framing |
| `Moving.countLabel` | mono uppercase 11px | "N entities" | terse count |
| `MovingEntityViewModel.lede` | serif italic 15px, 300 weight | ≤2 sentences, ≤180 chars | editorial; why this entity is moving |
| `MovingSignalViewModel.whatSegments` | sans 13px | ≤14 words per signal | terse with optional italic emphasis |
| `Watch.heading` | serif h2, 32px | "Watch" | naming |
| `Watch.summary` | serif italic 16px, 300 weight | ≤1 sentence | editorial framing |
| `WatchRow.what` | serif italic 16px, 300 weight | ≤1 sentence, ≤140 chars | observational |
| `BriefingFolioViewModel.label` | mono uppercase 11px | "Daily Briefing" | identity |
| `BriefingFolioViewModel.dateLabel` | mono uppercase 11px | "THURSDAY, APRIL 23, 2026" | mono cap date |
| `BriefingDateViewModel.displayDate` | serif | "Thursday, April 23, 2026" | display |
| `DayStripNeighbor.preview` | serif italic 14px, 300 weight | ≤80 chars | one-line glance |
| `BriefingFolioViewModel.crumbs[]` | mono uppercase 11px, ` › ` separator | ≤4 segments, ≤24 chars per segment | breadcrumb |
| `BriefingFolioViewModel.status` | mono italic 11px | ≤40 chars | terse status |
| `Lead.focusBlock` | serif 16px | ≤2 sentences, ≤180 chars | optional secondary callout |
| `Schedule.label` / `Moving.label` / `Watch.label` / `Predictions.label` | mono uppercase 11px | ≤16 chars | margin grid identity |
| `ScheduleMeeting.title` | serif 18px | ≤60 chars | meeting title |
| `ScheduleMeeting.eyebrow.entityName` | mono uppercase 11px | ≤24 chars | eyebrow entity |
| `ScheduleMeeting.eyebrow.relationship` | mono uppercase 11px | ≤16 chars | eyebrow relationship |
| `ScheduleMeeting.attendeeSummary` | sans 13px | ≤80 chars | "Jen Park, Dan Mitchell, +2" |
| `IntelligenceQualityView.label` | mono uppercase 10px | ≤20 chars | "Briefing fresh", "Notes captured" |
| `BriefingActionView.label` (link / create) | sans 13px | ≤32 chars | affordance label |
| `MeetingTimeViewModel.startLabel` | mono 14px | ≤10 chars | "10:00 AM" |
| `MeetingTimeViewModel.durationLabel` | mono 12px | ≤16 chars | "45m", "30m · ended" |
| `PillView.label` | mono uppercase 10px | ≤20 chars | "Renewing ↑", "At Risk ↓" |
| `ProvenanceStat.label` | mono 11px | ≤14 chars | "Health", "Last touch" |
| `ProvenanceStat.value` | mono 11px, weight 600 | ≤16 chars | "71 +3", "82%" |
| `ReadinessPair.label` | mono uppercase 11px | ≤24 chars | "3 briefings ready" |
| `WatchRow.who` | mono uppercase 11px | ≤24 chars | entity name |
| `WatchSuggestedActionRow.selector.triggerLabel` | sans 13px | ≤32 chars | "Snooze to Q3" |
| `WatchSuggestedActionRow.selector.options[].label` | sans 13px | ≤48 chars | option text |
| `WatchOpenActionRow.checkButtonLabel` | (a11y only) | ≤40 chars | "Mark complete" |
| `WatchParkedRow.parkedLabel` | mono 11px, tertiary | ≤24 chars | "Parked", "Snoozed until Q3" |
| `WatchAgingRow.ageLabel` | mono 11px | ≤24 chars | "Aging — 12 days" |
| `WatchAgingOption.label` | sans 12px | ≤16 chars | "Restore" / "Archive" |
| `DayChartViewModel.hourTicks[].label` | mono 10px | ≤8 chars | "9", "12 PM" |
| `DayChartViewModel.legend[].label` | mono 10px | ≤16 chars | "Customer" |
| `DayChartBarViewModel.title` | mono 11px | ≤24 chars | "Acme renewal" |
| `DayChartBarViewModel.timeLabel` | mono 10px | ≤16 chars | "10:00 · 45M" |
| `DayChartBarViewModel.tooltip` | (hover) | ≤80 chars | full sentence |
| `DayChartViewModel.nowLine.label` | mono uppercase 10px | "NOW" | indicator |
| `DayStripViewModel.current.label` | sans 14px | ≤16 chars | "Today" |
| `DayStripViewModel.current.ariaLabel` | (a11y only) | ≤80 chars | "Today, Thursday April 23" |
| `BriefingLoadState.error.message` | serif 28px | ≤16 words | error headline |
| `BriefingLoadState.error.detailMessage` | serif italic 17px, 300 | ≤2 sentences, ≤180 chars | error detail |
| `BriefingLoadState.empty.message` | serif italic 19px, 300 | ≤2 sentences, ≤180 chars | empty lede |
| `BriefingLoadState.empty.checklistItems[].label` | sans 14px | ≤80 chars | one-line checklist item |

Services that produce these fields must respect the budgets. The W2 service ticket plans must reference this table.

**Enforcement (deferred to W2):** a fixture-driven length-budget assertion test (`pnpm test src/types/briefing.typography-budgets.test.ts`) validates per-field budgets against canonical fixtures. Tracked as a W2 follow-up so this ADR doesn't grow the test surface; W2 service tickets gate on it.

## Auth-error vs empty-with-googleAuth disambiguation

Two adjacent states distinguish "user has not connected" from "creds went stale during operation":

- **`empty` + `googleAuth` (not authenticated)** → user has never connected Google, or has revoked. Renders `EditorialEmptyState` pattern with the connect-Google CTA.
- **`error` + `code: dependency_failed` + `service: lead | schedule | predictions | moving | watch`** → creds expired mid-session, or a downstream signal source (Glean, Gong) is unreachable. Renders `EditorialErrorState` pattern with "Try again" + "Diagnostics" affordances.

Services emit `empty` for "missing prerequisite" and `error` for "transient failure of an operation that should have worked." A forced reauth flow (creds detected stale) returns `error` with `code: dependency_failed` and `service: schedule` (or whichever section first detected the auth failure).

## Future-tax follow-up

Once W6 cutover removes the legacy `DailyBriefing.tsx`, evaluate whether `TrustAnnotated&lt;T&gt;` itself should tighten `trustBand` to required and absorb `TrustMixin`. Until then, the two coexist:
- `TrustMixin` (this contract): `trustBand` required, used by all fact-bearing fields in `BriefingViewModel`.
- `TrustAnnotated&lt;T&gt;` (existing in `src/types/index.ts:169`): `trustBand` optional, used by legacy types.

If they converge, the type alias becomes `TrustMixin = TrustAnnotated&lt;{}&gt;` with the optionality tightened in one place.

## Consequences

**Positive**

- The view becomes pure pass-through. Audit (DOS-438) catches any `.filter`/`.sort`/`.reduce` on view-model arrays as a regression.
- Trust band becomes load-bearing on every fact-bearing field at W0, so DOS-320/411 land as value fills, not type refactors.
- W1 component agents work against frozen contract enums — no mid-wave shape drift.

**Negative**

- The contract is large (~300 lines). Maintenance cost on schema changes.
- Service-rendered editorial copy concentrates writing responsibility in W2 services; design-system typography review needs to land alongside service work.
- Token aliases share paint with semantic entity tokens — design judgment call to revisit if cross-surface collision shows up.

**Neutral**

- The redesign track ships its own contract; the existing `DashboardData` shape stays for the legacy `DailyBriefing.tsx` until W6 cutover.

## L0 review history

- Round 1: REJECT (3 reviewers unanimous) — fictional types, deferred sub-models, missing top-level slots, per-section state vs envelope.
- Round 2: 3 of 4 APPROVE-WITH-REVISIONS, 1 REVISE — trust mixin inconsistency, partial-data semantics, missing fields (heading + countLabel + expandHint + whatSegments + urgency), token taxonomy collision, missing pattern stubs, inline CSS deferral.
- Round 3: 2 of 4 APPROVE-WITH-REVISIONS, 2 REVISE — all required fixes mechanical (no new design decisions). Per architect's explicit guidance, fixes applied inline as part of round-3 verdict (no round 4 needed). Closed mechanical fixes: ADR path typo, typography appendix completeness, mutation IDs (PredictionItem.id + WatchSuggestedActionRow/AgingRow.actionId), error.detailMessage + empty.checklistItems, mirror token sync, SignalDot camelCase class names, data-folio-mark loading-stub contradiction, threadAction nested-link rule (resolved as event-stop button), 3 state pattern stubs landed, ProvenanceStat truncation + WatchRow per-variant anatomy resolved.

## References

- Plan: `.docs/plans/daily-briefing-redesign-W0/DOS-413-plan.md`
- Reference: `.docs/design/reference/surfaces/briefing-redesign.html` + 3 state stubs
- Decisions doc: `.docs/plans/v1.4.0-daily-briefing-redesign-decisions.md`
- Wave plan: `.docs/plans/daily-briefing-redesign-waves.md`
- Existing pattern this conforms to: `src/hooks/useDashboardData.ts`, `src-tauri/src/services/dashboard.rs`
