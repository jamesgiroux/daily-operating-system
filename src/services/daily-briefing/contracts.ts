// DOS-507 — TypeScript mirror of `DailyBriefingOutput` DTO.
//
// Parity contract for the Rust producer at
// `src-tauri/abilities-runtime/src/abilities/get_daily_briefing/contracts.rs`.
// Renames Rust snake_case fields to camelCase per the producer's serde annotations.
//
// Per L0 packet §5.10 and locked decisions in §13 Q9 (Read-only; no auto-enqueue)
// and cycle-1 correctness F3 (`BriefingState` is a composed struct, NOT a flat
// enum — real briefings have multi-dimensional state).

import type {
  CandidateSetRef,
  ClaimSensitivity,
  Cursor,
  EnvelopeProvenance,
  Paginated,
  TrustBand,
} from "../entity-intelligence/contracts";

export const BRIEFING_SCHEMA_VERSION = 1;

export type BriefingSection =
  | "state"
  | "current_meeting"
  | "next_meeting"
  | "upcoming_meetings"
  | "watch_proposals"
  | "trust_summary";

export interface DailyBriefingInput {
  schemaVersion: number;
  /** ISO `YYYY-MM-DD` (chrono::NaiveDate, schemars overrides JSON Schema to string). */
  date: string;
  workspaceId: string;
  /** Opaque server-signed cursor for `upcomingMeetings` re-invocation (AC-507.10). */
  upcomingMeetingsCursor?: Cursor;
  sections?: BriefingSection[];
}

// ---- meeting brief ref ----------------------------------------------------

export interface MeetingBriefRef {
  meetingId: string;
  title: string | null;
  startsAt: string | null;
  endsAt: string | null;
  linkedEntityType: string | null;
  linkedEntityId: string | null;
  /**
   * Lower-snake_case PrepStatus discriminant: `ready` | `prep_needed` |
   * `queued` | `running` | `limited` | `stale` | `failed` |
   * `blocked_no_entity` | `user_suppressed` | `user_dismissed`.
   */
  prepStatus: string;
  blockingReason: string | null;
  staleReason: string | null;
  lastPreparedAt: string | null;
}

// ---- briefing state — composed (cycle-1 correctness F3) -------------------

export type BriefingEmptyReason =
  | "no_meetings"
  | "date_outside_known_window"
  | "workspace_unknown";

export type BriefingAvailability =
  | { kind: "available" }
  | { kind: "empty"; reason: BriefingEmptyReason }
  | { kind: "auth_locked" };

export type BriefingStaleReason =
  | "source_asof_older_than_threshold"
  | "upstream_claim_changed"
  | "entity_context_stale";

export type BriefingFreshness =
  | { kind: "fresh" }
  | { kind: "stale"; reason: BriefingStaleReason }
  | { kind: "needs_preparation"; meetingIds: string[] };

export interface AmbiguityPair {
  claimIdA: string;
  claimIdB: string;
  reason: string;
}

export type BriefingIntegrity =
  | { kind: "clean" }
  | { kind: "has_corrections"; supersededClaimIds: string[] }
  | { kind: "has_ambiguity"; ambiguousPairs: AmbiguityPair[] };

export type BriefingAdvisory =
  | { kind: "watch_proposal"; proposalId: string; summary: string }
  | { kind: "unlinked_meetings"; meetingIds: string[] }
  | { kind: "partial_read_failure"; advisory: string };

/**
 * AC-507.4 — `BriefingState` is a composed struct, NOT a flat enum. The 4-tuple
 * (availability, freshness, integrity, advisories) carries the briefing's
 * multi-dimensional posture without forcing a precedence that doesn't exist
 * in the domain. Consumers render each axis independently.
 */
export interface BriefingState {
  availability: BriefingAvailability;
  freshness: BriefingFreshness;
  integrity: BriefingIntegrity;
  advisories: BriefingAdvisory[];
}

// ---- watch proposals + trust summary --------------------------------------

export interface WatchProposal {
  proposalId: string;
  subjectKind: string;
  subjectId: string;
  headline: string;
  trustBand: TrustBand;
  sensitivity: ClaimSensitivity;
}

export interface BriefingTrustSummary {
  aggregateBand: TrustBand;
  likelyCurrentCount: number;
  useWithCautionCount: number;
  needsVerificationCount: number;
}

export interface SourceAsofRef {
  source: string;
  asOf: string;
}

// ---- output envelope -------------------------------------------------------

export interface DailyBriefingOutput {
  schemaVersion: number;
  date: string;
  state: BriefingState;
  currentMeeting: MeetingBriefRef | null;
  nextMeeting: MeetingBriefRef | null;
  /** AC-507.10 — paginated; cursor is opaque server-signed. */
  upcomingMeetings: Paginated<MeetingBriefRef>;
  candidateSet: CandidateSetRef;
  watchProposals: WatchProposal[];
  trustSummary: BriefingTrustSummary;
  provenance: EnvelopeProvenance;
  sensitivity: ClaimSensitivity;
  sourceAsofInputs: SourceAsofRef[];
}
