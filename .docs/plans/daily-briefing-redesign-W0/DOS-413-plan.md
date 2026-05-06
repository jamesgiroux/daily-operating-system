# DOS-413 — BriefingViewModel contract — L0 plan (rev 2)

**Wave:** W0 (single-ticket; blocks all of W1-W6)
**Status:** Round 2 — addresses round-1 unanimous REJECT (architect-reviewer + codex challenge + codex independent consult)
**Round-1 verdict files:** see commit history of this file
**Reviewers required for round 2:** same three independents + design-consultation (added per round-1 feedback that this contract determines what the rendered surface can look like)

## Round-1 findings addressed

- **Top-level envelope, not per-section state.** Adopt the existing `useDashboardData` pattern (`{ status: loading | error | empty | success }`) with the success case carrying the full `BriefingViewModel` plus `DataFreshness`. Drop `cachedStale`. Drop `select()` helper.
- **One Tauri command, not five.** `get_briefing_view_model` returns the full result; W2 services are internal producers composed inside that one command. Matches `dashboard.rs::DashboardResult`.
- **Real types only.** Reuse audit rebuilt against `src/types/index.ts` and the existing `MeetingSpineItem` component shape.
- **Trust as first-class via `TrustAnnotated<T>`.** Every fact-bearing field carries `trustBand` directly; W2 services emit `unscored` until W4 fills real values.
- **Every nested type defined here, not deferred.** ScheduleMeeting, DayChartViewModel, MovingEntityViewModel, WatchRowViewModel (discriminated), PredictionItem, plus `BriefingFolioViewModel`, `DayStripViewModel`, `BriefingDateViewModel` — all inline.
- **Service-rendered editorial strings on the contract.** Headlines split as `{ lead, punchLine? }`; section summaries, count labels, predictions collapsed copy all named fields.
- **Mutation surface acknowledged.** Named write-side services for InferredActionSelector and claim corrections.
- **Sealed error code enum.**
- **Date model split into `isoDate` + `displayDate`.**
- **Reference HTML to be updated** alongside this plan to show D2.1 Moving signal feed shape and D4.d Predictions section, so contract maps to an accurate render.

## 1. Ticket reference and acceptance summary

[DOS-413](https://linear.app/a8c/issue/DOS-413) — Define BriefingViewModel contract.

Acceptance criteria from ticket:

- [ ] `BriefingViewModel` type exported from `src/types/briefing.ts`
- [ ] One variant per section (Lead, Schedule, Predictions, Moving, Watch)
- [ ] All five states (loading / error / empty / cached-stale / present) modeled
- [ ] No business logic in the type — pure shape
- [ ] ADR or design note documenting the contract

**Round-2 amendment to the ticket interpretation:** "five states modeled" applies to the top-level envelope (`BriefingLoadState`), not per-section. Per-section state is fictional given current infra. The ticket's intent — every load condition representable — is satisfied.

## 2. What I'm building

Files added:

- `src/types/briefing.ts` — exports `BriefingLoadState`, `BriefingViewModel`, and every nested view-model type listed in section 5.
- `src-tauri/src/services/briefing_view_model.rs` (skeleton + types only — implementation in W2; this ticket lands the Rust types so frontend can deserialize). Mirrors the TS shape via serde.
- `src-tauri/src/commands/briefing.rs` — `get_briefing_view_model` Tauri command stub returning `BriefingLoadStateWire`. Stub returns `loading` until W2 lands.
- `.docs/adr/ADR-0109-briefing-view-model-contract.md` — design note (sequence number reserved, will be confirmed before merge).

Files modified:

- `.docs/design/surfaces/DailyBriefingRedesign.md` — references the contract by name.
- `.docs/design/reference/surfaces/briefing-redesign.html` — adds the missing **Predictions section** (D4.d) and updates the **Moving rows** to the locked Shape A signal feed (D2.1 — 3-column with SignalDot typedot list). This brings the reference into sync with the locked decisions before downstream tickets build against it.

## 3. What I'm NOT building

- Service implementations (W2 — DOS-414..419)
- Frontend hook to consume the contract (W5 — DOS-429)
- View component (W5 — DOS-429)
- Component tickets (W1 — DOS-420..422, 426)

Files explicitly off-limits:
- `src/components/` (any component code)
- `src-tauri/src/services/{moving,watch,briefing_schedule,predictions}.rs` (those are W2 ticket allowlist)
- The view-side hook

## 4. Reuse audit (against actual codebase)

Verified against `src/types/index.ts` and `src-tauri/src/services/dashboard.rs`:

| Need | Existing type | Location |
|---|---|---|
| Top-level load envelope | mirror of `DashboardLoadState` shape | `src/hooks/useDashboardData.ts:9` |
| Top-level Rust result | mirror of `DashboardResult` enum | `src-tauri/src/services/dashboard.rs:137` |
| Freshness | `DataFreshness = "fresh" \| "stale" \| "unknown"` | `src/types/index.ts:539` |
| Google auth status (for empty-state) | `GoogleAuthStatus` | `src/types/index.ts` |
| Trust band | `TrustBandWire = "likely_current" \| "use_with_caution" \| "needs_verification" \| "unscored"` | `src/types/index.ts:120` |
| Trust mixin | `TrustAnnotated<T>` (already exists) — adds `trustBand`, `trustFieldPath`, `trustSourceDate`, `renderedProvenance` | `src/types/index.ts:169` |
| Provenance summary | `RenderedProvenanceSummary` | `src/types/index.ts:146` |
| Field attribution | `RenderedFieldAttribution` | `src/types/index.ts:134` |
| Sensitivity-aware text | `RenderableClaimText` | `src/types/index.ts:115` |
| Render policy | `RenderPolicy`, `RenderPolicyKind`, `ClaimSensitivity` | `src/types/index.ts:107, 87, 71` |
| Linked entity | `LinkedEntity` | `src/types/index.ts:235` |
| Entity health | `IntelligenceAccountHealth`, `AccountHealth`, `HealthSnapshot`, `HealthTrendTag` | `src/types/index.ts:2183, 1488, 2983, 2199` |
| Meeting spine props | `MeetingSpineItemProps`, `MeetingSpineState`, `MeetingSpineType`, `MeetingSpinePrepState` | `src/components/dashboard/MeetingSpineItem.tsx:14, 7, 8, 12` |
| Predictions | `PredictionResult`, `PredictionScorecard`, `PredictionCategory` | `src/types/index.ts:53, 59, 52` |
| Pill tone | `PillTone` | `src/components/ui/Pill.tsx` |

The four types I previously named (`EntityRef`, `HealthBand`, `HealthScore`, `ClaimRef`) **do not exist** and are dropped from this plan.

## 5. Service / view-model contract surface

Full inline definition. No deferrals. No "TODO in ADR" placeholders.

```ts
// src/types/briefing.ts

import type {
  TrustBandWire,
  TrustAnnotated,
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
  MeetingSpinePrepState,
} from "@/components/dashboard/MeetingSpineItem";

// ─── Top-level envelope ───────────────────────────────────────────────────

/** Briefing error codes — sealed enum so each W2 service uses the same vocabulary. */
export type BriefingErrorCode =
  | "service_unavailable"   // a downstream service was unreachable
  | "dependency_failed"     // upstream dependency (Glean, Calendar) failed
  | "rate_limited"          // request was throttled
  | "partial_data"          // some sections succeeded, some didn't (use with care)
  | "internal";             // unexpected error

export type BriefingLoadState =
  | { status: "loading" }
  | { status: "error"; message: string; code?: BriefingErrorCode }
  | { status: "empty"; message: string; googleAuth?: GoogleAuthStatus }
  | {
      status: "success";
      model: BriefingViewModel;
      freshness: DataFreshness;
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

// ─── Folio bar (page header) ─────────────────────────────────────────────

export interface BriefingFolioViewModel {
  label: string;                    // "Daily Briefing"
  crumbs: string[];                 // ["Reference", "Surfaces", "Daily Briefing"]
  dateLabel: string;                // "THURSDAY, APRIL 23, 2026" — mono caps, service-rendered
  readiness: ReadinessPair[];
  actions: FolioActionKind[];
  markPulsing: boolean;
  status?: string;                  // optional italic mono status text
}

export interface ReadinessPair {
  label: string;                    // "3 briefings ready"
  color: ReadinessColor;
}

export type ReadinessColor =
  | "sage"
  | "terracotta"
  | "turmeric"
  | "larkspur"
  | "olive";

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
    ariaLabel: string;              // "Today, Thursday April 23"
  };
  next: DayStripNeighbor;
}

export interface DayStripNeighbor {
  label: string;                    // "Yesterday" / "Tomorrow"
  isoDate: string;
  preview: string;                  // "Wed · Acme call captured · 2 actions logged"
  href: string;                     // "/briefing?date=2026-04-22"
}

// ─── Lead (hero) ─────────────────────────────────────────────────────────

export interface LeadViewModel {
  /** Headline split so the view never composes the emphasis span itself. */
  headline: { lead: string; punchLine?: string };
  focusCapacity: string;            // "3h available · 2 deep work blocks · light afternoon after 2:00"
  focusBlock?: string;              // optional secondary callout text
}

// ─── Schedule ────────────────────────────────────────────────────────────

export interface ScheduleViewModel {
  label: string;                    // "Today" / "Schedule"
  countLabel: string;               // "6 meetings" — service handles pluralization
  meetingMix: ScheduleMeetingMix;
  summary: string;                  // service-rendered editorial sentence
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

export interface ScheduleMeeting {
  id: string;
  href?: string;                    // briefing detail link
  accentType: MeetingSpineType;     // reuses existing component type
  state: MeetingSpineState;         // reuses existing component type
  time: MeetingTimeViewModel;
  stateTags: MeetingStateTag[];     // multi-tag possible; ordered for render
  title: string;
  eyebrow: { entityName: string; relationship?: string };
  context: string;                  // service-rendered paragraph
  attendeeSummary: string;          // "Jen Park, Dan Mitchell, +2"
  intelligenceBadge: TrustAnnotated<IntelligenceBadgeView>;
  briefingAction: BriefingActionView;
}

export interface MeetingTimeViewModel {
  startsAt: string;                 // ISO-8601
  endsAt: string;                   // ISO-8601
  startLabel: string;               // "10:00" or "10:00 AM"
  durationLabel: string;            // "45m" or "30m · ended" or "Cancelled"
}

export type MeetingStateTag =
  | "now"
  | "upcoming"
  | "ended"
  | "cancelled"
  | "building"
  | "no_briefing_yet";

export interface IntelligenceBadgeView {
  level: "fresh" | "ready" | "developing" | "sparse" | "captured" | "no_briefing";
  label: string;                    // "Briefing fresh", "Notes captured", "No briefing yet"
}

export type BriefingActionView =
  | { kind: "link"; label: string; href: string }   // "Read full briefing →"
  | { kind: "create"; label: string }                // "Create briefing"
  | { kind: "none" };

// ─── DayChart ────────────────────────────────────────────────────────────

export interface DayChartViewModel {
  rangeStartHour: number;           // 7
  rangeEndHour: number;             // 17
  hourTicks: DayChartHourTick[];
  legend: DayChartLegendItem[];
  bars: DayChartBarViewModel[];
  nowLine: { label: string; leftPct: number; isoTime: string } | null;
}

export interface DayChartHourTick {
  label: string;                    // "9", "12 PM"
  muted: boolean;                   // typically true for noon
}

export interface DayChartLegendItem {
  kind: DayChartBarKind;
  label: string;                    // "Customer", "1:1", etc.
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
  layout: { leftPct: number; widthPct: number };  // service-computed
  title: string;                    // "Acme renewal"
  timeLabel: string;                // "10:00 · 45M" or "Cancelled"
  tooltip: string;                  // "Acme renewal · 10:00 · 45m"
}

// ─── Predictions ─────────────────────────────────────────────────────────

export interface PredictionsViewModel {
  count: number;
  collapsedLabel: string;           // "3 predictions today · expand"
  predictions: PredictionItem[];    // populated; UI decides expand/collapse render
}

export interface PredictionItem {
  text: string;                     // service-rendered prediction sentence
  confidence: { value: number; label: string };  // value 0..1, label "87%"
  abilitySource: { id: string; label: string };
  basisLink: { label: string; href: string };
  /** Trust mixin — ability outputs always carry a trust band. */
  trustBand: TrustBandWire;
  trustFieldPath?: string;
  trustSourceDate?: string | null;
  renderedProvenance?: RenderedProvenanceSummary | null;
}

// ─── Moving (Shape A) ────────────────────────────────────────────────────

export interface MovingViewModel {
  label: string;                    // "Moving"
  countLabel: string;               // "3 entities"
  summary: string;                  // service-rendered editorial sentence
  entities: MovingEntityViewModel[];  // ≤3, service-capped
}

export interface MovingEntityViewModel {
  kind: MovingEntityKind;
  entity: LinkedEntity;             // existing LinkedEntity type
  href: string;                     // entity detail page
  statePill: PillView;              // "Renewing ↑"
  lede: string;                     // 1-2 sentences, service-truncated
  signals: MovingSignalViewModel[]; // 3-5, service-ordered by relevance
  provenanceStats: ProvenanceStat[];
}

export type MovingEntityKind =
  | "customer"
  | "person"
  | "project"
  | "internal"
  | "lifecycle";

export interface PillView {
  label: string;                    // "Renewing ↑"
  tone: PillTone;                   // reuse existing PillTone enum
}

export interface MovingSignalViewModel {
  kind: SignalDotKind;
  when: string;                     // "10:00", "3h ago", "Overnight"
  what: string;                     // signal text — service-rendered
  threadAction?: { label: string; href: string };  // "talk" button affordance
  /** Trust + lifecycle: corrected SignalDot variant uses this. */
  trustBand: TrustBandWire;
  trustFieldPath?: string;
  trustSourceDate?: string | null;
  /** When the underlying claim has been corrected (DOS-411 output). */
  correctionState?: "none" | "corrected" | "contested";
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

export interface ProvenanceStat {
  label: string;                    // "Health"
  value: string;                    // "71 +3"
  trend?: "up" | "down" | "flat";
  /** Per-stat trust — Health, Stage, Confidence each have their own band. */
  trustBand: TrustBandWire;
  trustFieldPath?: string;
  trustSourceDate?: string | null;
}

// ─── Watch (discriminated union over kind) ───────────────────────────────

export interface WatchViewModel {
  label: string;                    // "Watch"
  countLabel: string;               // "5 quiet"
  summary: string;                  // service-rendered editorial sentence
  rows: WatchRowViewModel[];
}

export type WatchRowViewModel =
  | WatchSuggestedActionRow
  | WatchOpenActionRow
  | WatchParkedRow
  | WatchAgingRow;

interface WatchRowBase {
  who: string;                      // entity / person name
  what: string;                     // 1-line description, service-rendered
  trustBand: TrustBandWire;
  trustFieldPath?: string;
  trustSourceDate?: string | null;
  correctionState?: "none" | "corrected" | "contested";
}

export interface WatchSuggestedActionRow extends WatchRowBase {
  kind: "suggestedAction";
  selector: InferredActionSelectorViewModel;
}

export interface WatchOpenActionRow extends WatchRowBase {
  kind: "openAction";
  actionId: string;
  checkButtonLabel: string;         // accessibility label for the check
}

export interface WatchParkedRow extends WatchRowBase {
  kind: "parked";
  parkedLabel: string;              // "Parked", "Snoozed until Q3"
}

export interface WatchAgingRow extends WatchRowBase {
  kind: "aging";
  ageLabel: string;                 // "Aging — 12 days"
  since: string;                    // ISO-8601 — when the row first qualified
  options: WatchAgingOption[];
}

export interface WatchAgingOption {
  id: "restore" | "archive";
  label: string;
}

export interface InferredActionSelectorViewModel {
  triggerLabel: string;             // "Snooze to Q3"
  options: InferredActionOption[];  // includes a divider entry before dismiss
  selectedOptionId: string;
}

export interface InferredActionOption {
  id: string;
  label: string;                    // "Snooze until Q3 review"
  confidence?: { value: number; label: string };  // optional "87% · picked"
  divider?: boolean;                // true marks the dismiss separator entry
}
```

### Mutation surface (read-only contract; mutations called separately)

The contract above is read-only. The following named mutation services exist or are planned and pair with the read shape:

- `actions::snooze(action_id, until)` — Watch suggested-action snooze
- `actions::dismiss(action_id)` — Watch dismiss
- `actions::add_to_meeting(action_id, meeting_id)` — Watch suggested-action selector option
- `actions::mark_complete(action_id)` — WatchOpenActionRow check button
- `actions::restore(action_id)` / `actions::archive(action_id)` — WatchAgingRow options
- `claims::correct(claim_id, correction)` — Moving SignalDot correction (DOS-411)
- `claims::contest(claim_id, reason)` — Moving SignalDot contest
- `predictions::ack(prediction_id)` — Predictions item dismissal

Implementations of any not-yet-existing mutations are out of scope for DOS-413 but the names are reserved here so W2/W4 services know where to wire write-side.

### IPC topology decision

**One Tauri command: `get_briefing_view_model`**, returning `BriefingLoadStateWire` (Rust-side mirror of `BriefingLoadState`). Rationale:

- Matches existing `get_dashboard_data` pattern. No new IPC infra.
- Atomic loading semantics: success means all sections coherent.
- W2 services are internal producers composed in `briefing_view_model.rs::compose()`.
- Failure modes: any service error ⇒ top-level `BriefingLoadState.error` with the failing service in `code`.
- Empty-state handling: e.g. no Google auth ⇒ top-level `empty` with `googleAuth` for the view to show the connect-Google CTA.

## 6. Display-layer purity

This contract IS the enforcement mechanism. The view consumes pre-rendered strings and pre-sorted arrays. Specifically:

- All editorial copy (headlines, summaries, count labels, predictions collapsed copy) is service-rendered — view never composes strings.
- All array ordering is service-determined — view never sorts/filters.
- All counts are pre-pluralized labels — view never reaches for `count > 1 ? "s" : ""`.
- All time/duration labels are pre-formatted — view never reaches for `formatDate`.
- Headline emphasis split as `{ lead, punchLine }` so the view's only job is to wrap `punchLine` in the emphasis span if present.

The W6 view-purity audit (DOS-438) reads off these conventions: any `.filter`/`.sort`/`.reduce` in the view layer touching `BriefingViewModel` data is a violation.

## 7. Test plan

Concrete commands and expected pass criteria:

- `pnpm tsc --noEmit` — zero type errors after the new types ship.
- `pnpm test src/types/briefing.test.ts` — exhaustiveness assertions:
  - Switch over `BriefingLoadState.status` covers all 4 cases (compile-time `never` exhaustion check).
  - Switch over `WatchRowViewModel.kind` covers all 4 cases.
  - Switch over `SignalDotKind` covers all 8 cases.
- `cargo test briefing_view_model::serde_roundtrip` — Rust-side fixture that serializes a fully-populated `BriefingViewModel` and deserializes it back; asserts JSON shape equals what TS expects (canonical fixture file shared between Rust and TS).
- `pnpm tsc --noEmit` against a `mockBriefingViewModel` constant in test code that exercises every state of every section, every nested type variant, and the `unscored` trust-band default. If the mock compiles, the contract is internally consistent.

## 8. Risk + rollback

**Risks:**

- **Reference HTML drift remains a risk.** The plan updates the reference to show D2.1 + D4.d, but if locked decisions evolve again, the contract needs revision. Mitigation: tie the contract version to the decisions doc; any decisions-doc edit triggers a contract review.
- **Trust-band default `unscored` lets services ship without provenance.** If W2 ships with `unscored` everywhere and W4's DOS-320/DOS-411 slip, the user sees no trust signal. Mitigation: W2 service tests assert non-`unscored` trust band on a documented happy-path fixture; this prevents regressions.
- **Mutation surface listed but not enforced.** The contract names mutations but doesn't gate W2 services on having them implemented. Mitigation: each W2 service ticket's L0 plan must list the mutations it depends on and verify their existence.
- **Round-2 review still finds gaps.** Mitigation: 2nd revision cycle is the last permitted before L6 escalation. If round 2 isn't unanimous, this becomes an L6 conversation.

**Rollback:** types-only ticket. Rollback = revert the commit. Zero runtime effect on shipped code (the Tauri command stub returns `loading` until W2 services exist).

## 9. Wave dependencies

- **Consumes:** nothing.
- **Blocks:** every other redesign ticket (DOS-414..438) via the `BriefingViewModel` shape.
- **Cross-track dependency on v1.4.0:** `TrustBandWire`, `TrustAnnotated<T>`, `RenderedProvenanceSummary` already exist in `src/types/index.ts`. DOS-320 (W6 trust-band data shape) and DOS-411 (claim-lifecycle correction state) populate these fields with real values; W2 services emit `unscored` and `correctionState: "none"` until those land. No hard W4 stall.

## 10. Merge gate artifacts

Required for L2 + L3 sign-off:

- `pnpm tsc --noEmit` clean (zero TS errors)
- `cargo clippy -- -D warnings` clean for the Rust types
- `cargo test briefing_view_model::serde_roundtrip` passes
- Switch-exhaustiveness tests pass (compile-time `assertNever`)
- `BriefingViewModel` exported and importable from `@/types/briefing`
- ADR landed at `.docs/adr/ADR-0109-briefing-view-model-contract.md` (sequence number reserved)
- Reference HTML updated to D2.1 + D4.d shape (Predictions section visible, Moving rows show signal feed)
- This plan saved with round-2 status

## L0 reviewer dispatch (round 2)

Same three reviewers + the design-consultation seat (added per round-1 architect-reviewer concern that this contract determines what the rendered surface can look like):

1. **`/codex` adversarial challenge** — focus on whether round-2 still over-engineers; whether mutation-surface naming is enough or needs to be enforced; whether the `correctionState` slot is the right place for DOS-411 wiring.

2. **`architect-reviewer` subagent** — confirms round-1 concerns 1-13 are addressed and no new architectural issues surface.

3. **`/codex` independent consult** — section-coverage check now against the updated reference HTML (with D2.1 + D4.d landed). Confirms every rendered field has a contract slot.

4. **`design-consultation` subagent** — design-system layering check: does the contract correctly separate visual concerns from data concerns? Does it under-constrain typography choices that the design system already specifies?

Pass rule: unanimous approval. **Revision cycle counter: 2 of 2.** Round-2 non-unanimous = L6 escalation.
