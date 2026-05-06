# DOS-413 — BriefingViewModel contract — L0 plan (rev 3.1, APPROVED)

**Wave:** W0 (single-ticket; blocks all of W1-W6)
**Status:** L0 closed. Round 3 came back 2 APPROVE-WITH-REVISIONS + 2 REVISE; all required fixes mechanical, applied inline per architect's authorization (no round 4 needed). Plan advances to L1 self-validation.
**Round-3 verdicts:** see commit history; consolidated mechanical fixes applied in rev 3.1 commit.

## Round-3 mechanical fixes applied (rev 3.1)

- **Mirror token sync** — `--color-signal-*` aliases added to `.docs/design/reference/_shared/styles/design-tokens.css` (had been missed; reference HTML now resolves SignalDot vars correctly)
- **Mutation IDs** — `PredictionItem.id`, `WatchSuggestedActionRow.actionId`, `WatchAgingRow.actionId` added (required for `predictions::ack`, `actions::snooze/dismiss/restore/archive` calls)
- **State payload completeness** — `BriefingLoadState.error.detailMessage?` and `BriefingLoadState.empty.checklistItems?` added to cover the new state stub renders
- **ADR path typo fixed** — three references to `decisions/0129-briefing-view-model-contract.md` (was wrongly `.docs/adr/decisions/0129-...md-...md`)
- **ADR-0109 → 0129** in `PredictionsSection.md`
- **SignalDot camelCase classes** — `gong-call` → `gongCall`, `zendesk-ticket` → `zendeskTicket`, `slack-thread` → `slackThread`, `linear-issue` → `linearIssue` (CSS Module convention)
- **`data-folio-mark="pulsing"` dropped** from loading stub (markPulsing-dropped contradiction)
- **threadAction rendering rule** clarified — event-stop button, not nested anchor (MovingRow.md + SignalDot.md)
- **3 state pattern stubs** landed at proposed status: `BriefingLoadingState.md`, `BriefingErrorState.md`, `BriefingEmptyState.md`
- **MovingRow click-target** resolved (whole row via `role="link"` + tabindex, not wrapping `<a>`)
- **WatchRow per-variant anatomy** added (4 variants, ASCII anatomy each)
- **ProvenanceStat truncation + label-kind discriminator** resolved
- **Typography appendix expanded** in ADR 0129 (now exhaustive: ~50 fields covering crumbs, Lead.focusBlock, all section labels, ScheduleMeeting fields, IntelligenceQualityView, BriefingActionView, MeetingTimeViewModel, PillView, ProvenanceStat, ReadinessPair, all WatchRow variants, all DayChart fields, DayStrip current, all BriefingLoadState fields)
- **ADR future-tax note** — fold TrustMixin into TrustAnnotated<T> post-W6
- **ADR auth-error clarification** — `empty + googleAuth` vs `error + dependency_failed`
- **`// service-capped at ≤10`** comment on `PredictionsViewModel.predictions`
- **Mutation-verification clause** added to merge gate

## Tracked as W2/audit follow-ups (not in W0)

- Typography budget enforcement test (`pnpm test src/types/briefing.typography-budgets.test.ts`) → W2
- Signal-token paint-sharing constraint audit → design-system audit
- Open-action LifecycleMixin question → W2 substrate

## Round-2 findings addressed (21 items, all in scope per no-deferrals rule)

### Type contract changes

1. **TrustMixin interface, applied via `extends`.** Replaces inconsistent mix of `TrustAnnotated<T>` wrap + inlined fields. Trust band stays **required** for fact-bearing types. (architect N1, codex challenge new-1)
2. **Drop `partial_data` from `BriefingErrorCode`.** Atomic semantics: any service failure → top-level error. (architect N6, codex challenge new-3)
3. **Add `error.service` field** to `BriefingLoadState.error` so the failing producer is identifiable independent of the category code. (codex challenge new-4)
4. **Move `correctionState` off `WatchRowBase`** to specific claim-bearing variants (signal-bearing rows only). (architect N2)
5. **Split `intelligenceBadge` into `intelligenceQuality` + `trustBand` separate fields.** Each maps to its own canonical primitive. (design 2)
6. **Drop `markPulsing` from `BriefingFolioViewModel`.** FolioBar pattern derives animation locally from `freshness`. (design 10)
7. **Rename `ReadinessColor` to semantic enum.** `"healthy" | "needs_attention" | "in_progress" | "blocked" | "neutral"`. Pattern owns the semantic→paint mapping. (design 8)
8. **`Schedule.heading` field added.** Section h2 text ("Today's schedule"). (codex independent gap 1)
9. **`Moving.heading` field added.** Section h2 text ("What's moving"). (codex independent gap 4)
10. **`Predictions` section gains `label`, `countLabel`, `expandHint`.** Margin label, count, and "expand" hint as separate service-rendered fields. (codex independent gaps 2-3, codex challenge new-7)
11. **`MovingSignalViewModel.what` → `whatSegments` typed.** Array of `{ text: string; emphasized?: boolean }` to represent inline italic spans. (codex independent gap 5)
12. **`MovingSignalViewModel.urgency` field added.** `"normal" | "overdue"` so red overdue rows have a semantic source. (codex independent gap 6)
13. **Time field naming symmetry.** `startsAtIso` / `endsAtIso` to match `isoDate` convention. (architect N8)
14. **Fix `DataFreshness` audit row.** Real shape is an object union, not flat string union. (codex challenge new-8)

### Trust rollout reconciliation

15. **Pick one trust rollout policy.** **Decision: W2 services emit `unscored` initially; non-`unscored` happy-path test gates W4 merge, not W2.** Removes the contradiction between rev-2 "until W4" and rev-2 "W2 tests assert non-unscored." Test contract named in section 7. (architect N3, codex challenge new-2)

### Mutation enforcement

16. **W2 ticket acceptance includes mutation verification.** Each W2 service ticket's L0 plan must list the mutation commands it depends on and either confirm existence or include the create-mutation work. Captured in this plan; will be re-stated in W2 ticket bodies. (codex challenge new-5)

### W0 scope expansion (no deferrals)

17. **Add `--color-signal-*` token aliases.** New entries in `.docs/design/tokens/color.md` and `src/styles/design-tokens.css` for the 8 SignalDot kinds. (design 1)
18. **Land 3 reference HTML state stubs.** `briefing-redesign-loading.html`, `briefing-redesign-error.html`, `briefing-redesign-empty.html` (the empty-with-googleAuth case). (design 6)
19. **Land `proposed`-status pattern specs.** `MovingRow.md`, `WatchRow.md`, `PredictionsSection.md` — variants matching contract enums 1:1. (design 4)
20. **Land `proposed`-status primitive specs.** `SignalDot.md`, `ProvenanceStat.md`. (design 3)
21. **Land canonical module CSS files.** `_shared/styles/SignalDot.module.css`, `_shared/styles/ProvenanceStat.module.css`, `_shared/styles/PredictionsSection.module.css`. Reference HTML drops inline `<style>` for these, links them via `<link>`. (design 9)
22. **Typography contract appendix in decisions/0129-briefing-view-model-contract.md.** Each service-rendered string field paired with typography register, length budget, tone register. (design 5)
23. **`correctionState` visual treatment** in `SignalDot.md` primitive spec. (design 7)
24. **ADR notes for eager predictions payload + service owns route construction.** (architect N7, N9)

## 1. Ticket reference and acceptance summary

[DOS-413](https://linear.app/a8c/issue/DOS-413) — Define BriefingViewModel contract.

Acceptance criteria from ticket (round-3 interpretation):

- [ ] `BriefingViewModel` type exported from `src/types/briefing.ts`
- [ ] One variant per section (Lead, Schedule, Predictions, Moving, Watch)
- [ ] All states modeled — at the top-level envelope, not per-section. The ticket text says "loading / error / empty / cached-stale / present" but `cachedStale` is dropped (no infra exists); `freshness: DataFreshness` rides on `success`. The intent (every load condition representable) is satisfied.
- [ ] No business logic in the type — pure shape
- [ ] ADR landed at `.docs/decisions/0129-briefing-view-model-contract.md`

Round-3 expansion (recorded here, not in the original ticket): W0 also lands token aliases, 5 design-system stub specs, 3 canonical module CSS files, 3 reference HTML state stubs, and a typography contract appendix in the ADR.

## 2. What I'm building

### Type definitions

- `src/types/briefing.ts` — TS contract (full inline definitions in section 5)
- `src-tauri/src/services/briefing_view_model.rs` (skeleton + serde-derived types only)
- `src-tauri/src/commands/briefing.rs` — `get_briefing_view_model` Tauri command stub returning `loading` until W2 lands

### Token aliases

- `.docs/design/tokens/color.md` — extend with a `Signal kind` section (8 aliases)
- `src/styles/design-tokens.css` — corresponding CSS variables

### Design-system stubs (proposed status)

- `.docs/design/primitives/SignalDot.md`
- `.docs/design/primitives/ProvenanceStat.md`
- `.docs/design/patterns/MovingRow.md`
- `.docs/design/patterns/WatchRow.md`
- `.docs/design/patterns/PredictionsSection.md`

### Canonical module CSS

- `.docs/design/reference/_shared/styles/SignalDot.module.css`
- `.docs/design/reference/_shared/styles/ProvenanceStat.module.css`
- `.docs/design/reference/_shared/styles/PredictionsSection.module.css`

### Reference HTML

- Update `briefing-redesign.html` — drop provisional inline CSS for the three new modules; link the canonical files via `<link rel="stylesheet">`
- Add 3 state stubs:
  - `.docs/design/reference/surfaces/briefing-redesign-loading.html`
  - `.docs/design/reference/surfaces/briefing-redesign-error.html`
  - `.docs/design/reference/surfaces/briefing-redesign-empty.html`

### ADR

- `.docs/decisions/0129-briefing-view-model-contract.md` — design note + typography contract appendix + ADR-noted decisions (eager predictions, service-owned routes, trust rollout policy)

## 3. What I'm NOT building

- W2 service implementations (DOS-414..419)
- W5 view component (DOS-429)
- W1 component implementations (DOS-420..422, 426)
- Full pattern/primitive specs beyond `proposed`-status stubs (W1 tickets refine)

## 4. Reuse audit (against actual codebase, verified by grep)

| Need | Real type / file | Verified path |
|---|---|---|
| Top-level load envelope (existing pattern) | `DashboardLoadState` shape | `src/hooks/useDashboardData.ts:9` |
| Top-level Rust result | `DashboardResult` enum | `src-tauri/src/services/dashboard.rs:137` |
| Freshness | `DataFreshness` (object union: `{ freshness: "fresh"; generatedAt }` \| `{ freshness: "stale"; dataDate; generatedAt }` \| `{ freshness: "unknown" }`) | `src/types/index.ts:539` |
| Google auth status | `GoogleAuthStatus` | `src/types/index.ts:969` |
| Trust band | `TrustBandWire = "likely_current" \| "use_with_caution" \| "needs_verification" \| "unscored"` | `src/types/index.ts:120` |
| Provenance | `RenderedProvenanceSummary`, `RenderedFieldAttribution` | `src/types/index.ts:146, 134` |
| Sensitivity-aware text | `RenderableClaimText`, `RenderPolicy`, `ClaimSensitivity` | `src/types/index.ts:115, 107, 71` |
| Linked entity | `LinkedEntity` | `src/types/index.ts:235` |
| Account health | `IntelligenceAccountHealth`, `AccountHealth` | `src/types/index.ts:2183, 1488` |
| Meeting spine | `MeetingSpineItemProps`, `MeetingSpineState`, `MeetingSpineType`, `MeetingSpinePrepState` | `src/components/dashboard/MeetingSpineItem.tsx:14, 7, 8, 12` |
| Predictions | `PredictionResult`, `PredictionScorecard`, `PredictionCategory` | `src/types/index.ts:53, 59, 52` |
| Pill tone | `PillTone` | `src/components/ui/Pill.tsx:5` |
| TrustBandBadge primitive | spec + tsx exist | `.docs/design/primitives/TrustBandBadge.md`, `src/components/ui/TrustBandBadge.tsx` |
| Trust band tokens | `--color-trust-{likely-current,use-with-caution,needs-verification}` already exist | `.docs/design/tokens/color.md:88-100`, `src/styles/design-tokens.css` |

`TrustAnnotated<T>` exists at `src/types/index.ts:169` but defines optional fields. The contract uses a stricter named `TrustMixin` interface (defined in section 5) rather than the generic, because rev-2 reviewers correctly flagged that the contract requires non-optional trust bands on fact-bearing fields.

## 5. Service / view-model contract surface

```ts
// src/types/briefing.ts

import type {
  TrustBandWire,
  RenderedProvenanceSummary,
  RenderableClaimText,
  RenderPolicy,
  ClaimSensitivity,
  LinkedEntity,
  IntelligenceAccountHealth,
  AccountHealth,
  DataFreshness,
  GoogleAuthStatus,
  PredictionResult,
} from "./index";
import type { PillTone } from "@/components/ui/Pill";
import type {
  MeetingSpineState,
  MeetingSpineType,
} from "@/components/dashboard/MeetingSpineItem";

// ─── Trust mixin ──────────────────────────────────────────────────────────

/**
 * Trust mixin applied via `extends` to every fact-bearing view-model type.
 * Trust band is REQUIRED on these types (`unscored` is the W2 default until
 * DOS-320 / DOS-411 fill values). Stricter than the generic `TrustAnnotated<T>`
 * by design.
 */
export interface TrustMixin {
  trustBand: TrustBandWire;
  trustFieldPath?: string;
  trustSourceDate?: string | null;
  renderedProvenance?: RenderedProvenanceSummary | null;
}

/**
 * Lifecycle mixin — claim-correction state. Applied to types whose underlying
 * claim has a correction history (signal feed items, suggested-action rows).
 * Not on every TrustMixin-bearing type because not every claim is correctable
 * via DOS-411 (e.g. an action's existence is not a "correction" surface).
 */
export interface LifecycleMixin {
  correctionState?: "none" | "corrected" | "contested";
}

// ─── Top-level envelope ───────────────────────────────────────────────────

/** Sealed enum of error categories. `partial_data` removed: atomic semantics. */
export type BriefingErrorCode =
  | "service_unavailable"
  | "dependency_failed"
  | "rate_limited"
  | "internal";

export type BriefingSectionId =
  | "lead"
  | "schedule"
  | "predictions"
  | "moving"
  | "watch";

export interface BriefingEmptyChecklistItem {
  label: string;                     // service-rendered
  status?: "todo" | "done";          // optional checkmark state
}

export type BriefingLoadState =
  | { status: "loading" }
  | {
      status: "error";
      message: string;                        // primary headline
      detailMessage?: string;                 // secondary detail sentence
      code?: BriefingErrorCode;
      service?: BriefingSectionId;            // failing producer, when known
    }
  | {
      status: "empty";
      message: string;
      googleAuth?: GoogleAuthStatus;
      checklistItems?: BriefingEmptyChecklistItem[];  // shown in empty state body
    }
  | {
      status: "success";
      model: BriefingViewModel;
      freshness: DataFreshness;             // object union per index.ts:539
      googleAuth?: GoogleAuthStatus;
    };

// ─── Top-level model ──────────────────────────────────────────────────────

export interface BriefingViewModel {
  date: BriefingDateViewModel;
  folio: BriefingFolioViewModel;
  dayStrip: DayStripViewModel;
  lead: LeadViewModel;
  schedule: ScheduleViewModel;
  predictions: PredictionsViewModel;
  moving: MovingViewModel;
  watch: WatchViewModel;
}

// ─── Date ─────────────────────────────────────────────────────────────────

export interface BriefingDateViewModel {
  isoDate: string;       // "2026-04-23"
  displayDate: string;   // "Thursday, April 23, 2026" — service-rendered
}

// ─── Folio bar ────────────────────────────────────────────────────────────

export interface BriefingFolioViewModel {
  label: string;                    // "Daily Briefing"
  crumbs: string[];                 // ["Reference", "Surfaces", "Daily Briefing"]
  dateLabel: string;                // "THURSDAY, APRIL 23, 2026" — mono caps
  readiness: ReadinessPair[];
  actions: FolioActionKind[];
  status?: string;                  // optional italic mono status text
  // markPulsing dropped — FolioBar pattern derives animation from freshness
}

export interface ReadinessPair {
  label: string;                    // "3 briefings ready"
  semantic: ReadinessSemantic;
}

/** Semantic states. Pattern owns paint mapping. */
export type ReadinessSemantic =
  | "healthy"
  | "needs_attention"
  | "in_progress"
  | "blocked"
  | "neutral";

export type FolioActionKind =
  | "refresh"
  | "regenerate"
  | "archive"
  | "discover"
  | "new";

// ─── DayStrip nav ────────────────────────────────────────────────────────

export interface DayStripViewModel {
  prev: DayStripNeighbor;
  current: {
    label: string;                  // "Today"
    isoDate: string;
    ariaLabel: string;
  };
  next: DayStripNeighbor;
}

export interface DayStripNeighbor {
  label: string;                    // "Yesterday" / "Tomorrow"
  isoDate: string;
  preview: string;                  // service-rendered
  href: string;                     // service owns route construction (ADR note)
}

// ─── Lead ─────────────────────────────────────────────────────────────────

export interface LeadViewModel {
  /** Headline split — view never composes the emphasis span itself. */
  headline: { lead: string; punchLine?: string };
  focusCapacity: string;
  focusBlock?: string;
}

// ─── Schedule ────────────────────────────────────────────────────────────

export interface ScheduleViewModel {
  /** Margin grid label (e.g. "Today"). */
  label: string;
  /** Section heading h2 (e.g. "Today's schedule"). */
  heading: string;
  /** Pre-pluralized count label (e.g. "6 meetings"). */
  countLabel: string;
  meetingMix: ScheduleMeetingMix;
  /** Editorial summary sentence — service-rendered. */
  summary: string;
  dayChart: DayChartViewModel;
  meetings: ScheduleMeeting[];
}

export interface ScheduleMeetingMix {
  customer: number;
  partner: number;
  internal: number;
  personal: number;
  oneOnOne: number;
  cancelled: number;
}

export interface ScheduleMeeting extends TrustMixin {
  id: string;
  href?: string;                    // briefing detail link
  accentType: MeetingSpineType;
  state: MeetingSpineState;
  time: MeetingTimeViewModel;
  stateTags: MeetingStateTag[];
  title: string;
  eyebrow: { entityName: string; relationship?: string };
  context: string;
  attendeeSummary: string;
  intelligenceQuality: IntelligenceQualityView;
  briefingAction: BriefingActionView;
}

export interface MeetingTimeViewModel {
  startsAtIso: string;              // ISO-8601, raw
  endsAtIso: string;                // ISO-8601, raw
  startLabel: string;               // "10:00" or "10:00 AM"
  durationLabel: string;            // "45m", "30m · ended", "Cancelled"
}

export type MeetingStateTag =
  | "now"
  | "upcoming"
  | "ended"
  | "cancelled"
  | "building"
  | "no_briefing_yet";

/** Completeness vocabulary — owned by IntelligenceQualityBadge primitive. */
export interface IntelligenceQualityView {
  level: "fresh" | "ready" | "developing" | "sparse" | "captured" | "no_briefing";
  label: string;
}

export type BriefingActionView =
  | { kind: "link"; label: string; href: string }
  | { kind: "create"; label: string }
  | { kind: "none" };

// ─── DayChart ────────────────────────────────────────────────────────────

export interface DayChartViewModel {
  rangeStartHour: number;
  rangeEndHour: number;
  hourTicks: DayChartHourTick[];
  legend: DayChartLegendItem[];
  bars: DayChartBarViewModel[];
  nowLine: { label: string; leftPct: number; isoTime: string } | null;
}

export interface DayChartHourTick {
  label: string;
  muted: boolean;
}

export interface DayChartLegendItem {
  kind: DayChartBarKind;
  label: string;
}

export type DayChartBarKind =
  | "customer"
  | "internal"
  | "partner"
  | "personal"
  | "oneOnOne"
  | "project"
  | "cancelled";

export interface DayChartBarViewModel {
  kind: DayChartBarKind;
  state?: "past" | "now" | "upcoming" | "cancelled";
  layout: { leftPct: number; widthPct: number };
  title: string;
  timeLabel: string;
  tooltip: string;
}

// ─── Predictions ─────────────────────────────────────────────────────────

export interface PredictionsViewModel {
  /** Margin grid label ("Predictions"). */
  label: string;
  /** Pre-pluralized count label ("3 today"). */
  countLabel: string;
  /** Default-state count line ("3 predictions today"). */
  collapsedLabel: string;
  /** Default-state expand affordance hint ("expand"). */
  expandHint: string;
  /** Numeric count — for type-narrowing and analytics. */
  count: number;
  /** Eager-loaded — see decisions/0129-briefing-view-model-contract.md rationale.
   *  Service-capped at ≤10 items. */
  predictions: PredictionItem[];
}

export interface PredictionItem extends TrustMixin {
  /** Stable ID; required for predictions::ack(prediction_id). */
  id: string;
  text: string;
  confidence: { value: number; label: string };
  abilitySource: { id: string; label: string };
  basisLink: { label: string; href: string };
}

// ─── Moving ──────────────────────────────────────────────────────────────

export interface MovingViewModel {
  label: string;                    // "Moving"
  heading: string;                  // "What's moving"
  countLabel: string;               // "3 entities"
  summary: string;
  entities: MovingEntityViewModel[];  // ≤3, service-capped
}

export interface MovingEntityViewModel {
  kind: MovingEntityKind;
  entity: LinkedEntity;
  href: string;
  statePill: PillView;
  lede: string;
  signals: MovingSignalViewModel[];   // 3-5, service-ordered
  provenanceStats: ProvenanceStat[];
}

export type MovingEntityKind =
  | "customer"
  | "person"
  | "project"
  | "internal"
  | "lifecycle";

export interface PillView {
  label: string;
  tone: PillTone;
}

export interface MovingSignalViewModel extends TrustMixin, LifecycleMixin {
  kind: SignalDotKind;
  when: string;
  /** Typed text segments — emphasized=true renders as italic. */
  whatSegments: WhatSegment[];
  urgency: "normal" | "overdue";
  threadAction?: { label: string; href: string };
}

export interface WhatSegment {
  text: string;
  emphasized?: boolean;
}

export type SignalDotKind =
  | "meeting"
  | "action"
  | "email"
  | "lifecycle"
  | "gong-call"
  | "zendesk-ticket"
  | "slack-thread"
  | "linear-issue";

export interface ProvenanceStat extends TrustMixin {
  label: string;                    // "Health"
  value: string;                    // "71 +3"
  trend?: "up" | "down" | "flat";
}

// ─── Watch ────────────────────────────────────────────────────────────────

export interface WatchViewModel {
  label: string;
  heading: string;
  countLabel: string;
  summary: string;
  rows: WatchRowViewModel[];
}

export type WatchRowViewModel =
  | WatchSuggestedActionRow
  | WatchOpenActionRow
  | WatchParkedRow
  | WatchAgingRow;

interface WatchRowBase extends TrustMixin {
  who: string;
  what: string;
}

/** Claim-bearing variants extend LifecycleMixin; non-claim variants don't. */
export interface WatchSuggestedActionRow extends WatchRowBase, LifecycleMixin {
  kind: "suggestedAction";
  /** Stable ID; required for actions::snooze(action_id), actions::dismiss(action_id), actions::add_to_meeting(action_id, meeting_id). */
  actionId: string;
  selector: InferredActionSelectorViewModel;
}

export interface WatchOpenActionRow extends WatchRowBase {
  kind: "openAction";
  actionId: string;
  checkButtonLabel: string;
}

export interface WatchParkedRow extends WatchRowBase {
  kind: "parked";
  parkedLabel: string;
}

export interface WatchAgingRow extends WatchRowBase {
  kind: "aging";
  /** Stable ID; required for actions::restore(action_id) and actions::archive(action_id). */
  actionId: string;
  ageLabel: string;
  since: string;
  options: WatchAgingOption[];
}

export interface WatchAgingOption {
  id: "restore" | "archive";
  label: string;
}

export interface InferredActionSelectorViewModel {
  triggerLabel: string;
  options: InferredActionOption[];
  selectedOptionId: string;
}

export interface InferredActionOption {
  id: string;
  label: string;
  confidence?: { value: number; label: string };
  divider?: boolean;
}
```

### Mutation surface (read-only contract; mutations called separately)

The contract is read-only. The following named mutation services pair with the read shape:

- `actions::snooze(action_id, until)` — Watch suggested-action snooze
- `actions::dismiss(action_id)` — Watch dismiss
- `actions::add_to_meeting(action_id, meeting_id)` — Watch suggested-action selector option
- `actions::mark_complete(action_id)` — WatchOpenActionRow check
- `actions::restore(action_id)` / `actions::archive(action_id)` — WatchAgingRow options
- `claims::correct(claim_id, correction)` — Moving SignalDot correction (DOS-411)
- `claims::contest(claim_id, reason)` — Moving SignalDot contest
- `predictions::ack(prediction_id)` — Predictions item ack

**Enforcement:** each W2 service ticket's L0 plan must verify the mutations its read-side affordances depend on already exist OR include the create-mutation work in scope. (Currently, `dismiss_suggested_action` exists at `src-tauri/src/commands/actions_calendar.rs:138`; others to be confirmed by W2 ticket plans.)

### IPC topology decision

**One Tauri command: `get_briefing_view_model`**, returning `BriefingLoadStateWire` (Rust-side mirror of `BriefingLoadState`). Rationale:

- Matches existing `get_dashboard_data` pattern. No new IPC infra.
- Atomic loading: success means all sections coherent.
- W2 services are internal producers composed in `briefing_view_model.rs::compose()`.
- Failure modes: any service error ⇒ top-level `BriefingLoadState.error` with the failing service in `service`, the category in `code`. No partial-success — `partial_data` removed from the enum.
- Empty handling: e.g. no Google auth ⇒ top-level `empty` with `googleAuth` for the view's connect-Google CTA.

### Trust rollout policy

W2 services emit `unscored` initially. The non-`unscored` happy-path test (section 7) gates **W4 merge**, not W2. This removes the rev-2 contradiction.

### Token additions

New entries in `tokens/color.md` and `src/styles/design-tokens.css`:

```
--color-signal-meeting        → larkspur paint, semantic note: shared paint with --color-person but signal context
--color-signal-action         → saffron paint
--color-signal-email          → sage paint, semantic note: shared paint with --color-trust-likely-current but signal context
--color-signal-lifecycle      → turmeric paint
--color-signal-gong-call      → terracotta paint
--color-signal-zendesk-ticket → text-tertiary (grey)
--color-signal-slack-thread   → eucalyptus paint
--color-signal-linear-issue   → olive paint, semantic note: shared paint with --color-project but signal context
```

Each alias documented in `tokens/color.md` with the explicit "signal kind, not entity kind" semantic separation. Cross-surface collision risk acknowledged (signals appear in the Moving feed; entity tokens appear on entity surfaces).

## 6. Display-layer purity

Contract IS the enforcement mechanism:

- All editorial copy is service-rendered (headlines, summaries, count labels, expand hints, predictions copy).
- All array ordering is service-determined.
- All counts are pre-pluralized labels.
- All time/duration labels are pre-formatted.
- All semantic-→-paint mappings live in patterns, not in service.
- `whatSegments` typing forbids the view from parsing emphasis from raw strings.

The W6 view-purity audit (DOS-438) reads off these conventions.

## 7. Test plan

- `pnpm tsc --noEmit` — zero TS errors.
- `pnpm test src/types/briefing.test.ts` — exhaustiveness assertions:
  - Switch over `BriefingLoadState.status` covers all 4 cases (compile-time `assertNever`).
  - Switch over `WatchRowViewModel.kind` covers all 4 cases.
  - Switch over `SignalDotKind` covers all 8 cases.
  - Switch over `BriefingErrorCode` covers all 4 cases (post `partial_data` removal).
- `cargo test briefing_view_model::serde_roundtrip` — Rust ↔ TS canonical fixture roundtrip.
- `pnpm test src/types/briefing.fixture.test.ts` — happy-path fixture asserts every `TrustMixin`-bearing field has a `trustBand` value (any value, including `unscored`). **Renamed test gate** for W4: `pnpm test src/types/briefing.w4-trust.test.ts` asserts the same fixture has non-`unscored` bands. **W4 merge gate** consumes this test; W2 merge does not.

## 8. Risk + rollback

**Risks:**

- **Token collision visual regression.** Three `--color-signal-*` aliases share paint with semantic tokens. Mitigation: documented as deliberate; signals appear only in the Moving feed; cross-surface usage tested via reference render.
- **Pattern stub drift.** Stubs are `proposed` status. W1 component tickets must reconcile any divergence. Mitigation: contract enums are 1:1 with stub variant lists; `audit-reference.py` checks reference HTML uses real classes.
- **Mutation existence not yet verified.** Some mutations may not exist. Mitigation: W2 ticket plans verify or scope-in.

**Rollback:** types-only contract. Token additions and stub specs are additive (no existing surface depends on them). Revert the commits.

## 9. Wave dependencies

- **Consumes:** nothing.
- **Blocks:** every redesign ticket DOS-414..438.
- **Cross-track:** `TrustBandWire`, `RenderedProvenanceSummary`, `--color-trust-*` tokens already exist; DOS-320 / DOS-411 fill values. No hard W4 stall — W2 ships `unscored` defaults.

## 10. Merge gate artifacts

- `pnpm tsc --noEmit` clean
- `cargo clippy -- -D warnings` clean for the Rust types
- `cargo test briefing_view_model::serde_roundtrip` passes
- `pnpm test src/types/briefing.test.ts` exhaustiveness suite passes
- `pnpm test src/types/briefing.fixture.test.ts` passes (happy-path has `trustBand` assigned, value-irrelevant)
- `BriefingViewModel` exported and importable from `@/types/briefing`
- decisions/0129-briefing-view-model-contract.md landed with typography contract appendix
- Token aliases land in `tokens/color.md` + `design-tokens.css`
- 5 stub specs land at `proposed` status (SignalDot, ProvenanceStat, MovingRow, WatchRow, PredictionsSection)
- 3 canonical module CSS files land
- Reference HTML drops inline CSS for those modules; links them
- 3 state stubs land (loading, error, empty)
- 3 state pattern stubs land at `proposed` status (BriefingLoadingState, BriefingErrorState, BriefingEmptyState)
- W2 ticket bodies (DOS-414..419) updated with the mutation-verification clause: "L0 plan must list mutation commands consumed and confirm existence or include create-mutation work"

## L0 reviewer dispatch (round 3 — final per pacing rule)

Same four reviewers:

1. `/codex` adversarial challenge — verify rev-3 introduces no new contradictions; confirm trust-mixin consistency, partial-data removal, mutation enforcement.
2. `architect-reviewer` — confirm round-2 5 mechanical edits + new W0 scope additions land cleanly.
3. `/codex` independent consult — section-coverage check against updated reference HTML + new state stubs; verify all contract fields trace to rendered surface.
4. `design-consultation` — token alias review, stub-spec quality, typography contract appendix.

**Pass rule: unanimous approval.** Round 3 is the LAST permitted round per the L0-L6 pacing rule. Round-3 non-unanimous = L6 escalation conversation with the user.
