// Daily Briefing view-model contract.
// See .docs/decisions/0129-briefing-view-model-contract.md
// and .docs/plans/daily-briefing-redesign-W0/DOS-413-plan.md.

import type {
  TrustBandWire,
  RenderedProvenanceSummary,
  LinkedEntity,
  DataFreshness,
  GoogleAuthStatus,
} from "./index";

// Re-export wire types consumers commonly need so they don't have to
// reach into ./index for transitively-shared shapes.
export type { TrustBandWire };
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
 */
export interface LifecycleMixin {
  correctionState?: "none" | "corrected" | "contested";
}

// ─── Top-level envelope ───────────────────────────────────────────────────

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
  label: string;
  status?: "todo" | "done";
}

export type BriefingLoadState =
  | { status: "loading" }
  | {
      status: "error";
      message: string;
      detailMessage?: string;
      code?: BriefingErrorCode;
      service?: BriefingSectionId;
    }
  | {
      status: "empty";
      message: string;
      googleAuth?: GoogleAuthStatus;
      checklistItems?: BriefingEmptyChecklistItem[];
    }
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
  isoDate: string;
  displayDate: string;
}

// ─── Folio bar ────────────────────────────────────────────────────────────

export interface BriefingFolioViewModel {
  label: string;
  crumbs: string[];
  dateLabel: string;
  readiness: ReadinessPair[];
  actions: FolioActionKind[];
  status?: string;
}

export interface ReadinessPair {
  label: string;
  semantic: ReadinessSemantic;
}

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
    label: string;
    isoDate: string;
    ariaLabel: string;
  };
  next: DayStripNeighbor;
}

export interface DayStripNeighbor {
  label: string;
  isoDate: string;
  preview: string;
  href: string;
}

// ─── Lead ─────────────────────────────────────────────────────────────────

export interface LeadViewModel {
  headline: { lead: string; punchLine?: string };
  focusCapacity: string;
  focusBlock?: string;
}

// ─── Schedule ────────────────────────────────────────────────────────────

export interface ScheduleViewModel {
  label: string;
  heading: string;
  countLabel: string;
  meetingMix: ScheduleMeetingMix;
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
  href?: string;
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
  startsAtIso: string;
  endsAtIso: string;
  startLabel: string;
  durationLabel: string;
}

export type MeetingStateTag =
  | "now"
  | "upcoming"
  | "ended"
  | "cancelled"
  | "building"
  | "no_briefing_yet";

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
  label: string;
  countLabel: string;
  collapsedLabel: string;
  expandHint: string;
  count: number;
  /** Service-capped at ≤10 items. */
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
  label: string;
  heading: string;
  countLabel: string;
  summary: string;
  /** ≤3, service-capped. */
  entities: MovingEntityViewModel[];
}

export interface MovingEntityViewModel {
  kind: MovingEntityKind;
  entity: LinkedEntity;
  href: string;
  statePill: PillView;
  lede: string;
  /** 3-5, service-ordered. */
  signals: MovingSignalViewModel[];
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
  label: string;
  value: string;
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

export interface WatchSuggestedActionRow extends WatchRowBase, LifecycleMixin {
  kind: "suggestedAction";
  /** Stable ID; required for actions::snooze, actions::dismiss, actions::add_to_meeting. */
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
  /** Stable ID; required for actions::restore and actions::archive. */
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
