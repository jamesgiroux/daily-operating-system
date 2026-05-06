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

Services that produce these fields must respect the budgets. The W2 service ticket plans must reference this table.

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
- Round 3: pending. Last permitted round per pacing rule before L6 escalation.

## References

- Plan: `.docs/plans/daily-briefing-redesign-W0/DOS-413-plan.md`
- Reference: `.docs/design/reference/surfaces/briefing-redesign.html` + 3 state stubs
- Decisions doc: `.docs/plans/v1.4.0-daily-briefing-redesign-decisions.md`
- Wave plan: `.docs/plans/daily-briefing-redesign-waves.md`
- Existing pattern this conforms to: `src/hooks/useDashboardData.ts`, `src-tauri/src/services/dashboard.rs`
