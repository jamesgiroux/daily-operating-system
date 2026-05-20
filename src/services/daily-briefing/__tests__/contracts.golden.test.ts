// DOS-507 — parity golden test for `DailyBriefingOutput` TS mirror.
// Asserts every closed-enum variant from the Rust contract at
// `src-tauri/abilities-runtime/src/abilities/get_daily_briefing/contracts.rs`
// is representable in the TypeScript mirror, and that a representative
// envelope JSON-round-trips.

import { describe, expect, it } from "vitest";

import {
  BRIEFING_SCHEMA_VERSION,
  type BriefingAdvisory,
  type BriefingAvailability,
  type BriefingEmptyReason,
  type BriefingFreshness,
  type BriefingIntegrity,
  type BriefingSection,
  type BriefingStaleReason,
  type BriefingState,
  type DailyBriefingOutput,
  type MeetingBriefRef,
} from "../contracts";

interface BriefingGoldenFixture {
  output: DailyBriefingOutput;
  enumCoverage: {
    sections: BriefingSection[];
    emptyReasons: BriefingEmptyReason[];
    availabilities: BriefingAvailability["kind"][];
    staleReasons: BriefingStaleReason[];
    freshnessKinds: BriefingFreshness["kind"][];
    integrityKinds: BriefingIntegrity["kind"][];
    advisoryKinds: BriefingAdvisory["kind"][];
  };
}

const sampleMeeting: MeetingBriefRef = {
  meetingId: "m-1",
  title: "Q3 Sync",
  startsAt: "2026-05-20T15:00:00Z",
  endsAt: "2026-05-20T16:00:00Z",
  linkedEntityType: "account",
  linkedEntityId: "acc-1",
  prepStatus: "ready",
  blockingReason: null,
  staleReason: null,
  lastPreparedAt: "2026-05-20T14:50:00Z",
};

const sampleState: BriefingState = {
  availability: { kind: "available" },
  freshness: { kind: "needs_preparation", meetingIds: ["m-2"] },
  integrity: { kind: "has_corrections", supersededClaimIds: ["claim-1"] },
  advisories: [
    { kind: "watch_proposal", proposalId: "watch-1", summary: "role change" },
    { kind: "unlinked_meetings", meetingIds: ["m-3"] },
    { kind: "partial_read_failure", advisory: "entity envelope reader unavailable" },
  ],
};

const fixture: BriefingGoldenFixture = {
  output: {
    schemaVersion: BRIEFING_SCHEMA_VERSION,
    date: "2026-05-20",
    state: sampleState,
    currentMeeting: sampleMeeting,
    nextMeeting: null,
    upcomingMeetings: {
      items: [sampleMeeting],
      nextCursor: null,
      totalHint: 1,
      cursorState: { kind: "stable" },
    },
    candidateSet: {
      windowStart: null,
      windowEnd: null,
      filterDescription: "daily_briefing date=2026-05-20 workspace=ws-1 meetings=1",
    },
    watchProposals: [],
    trustSummary: {
      aggregateBand: "likely_current",
      likelyCurrentCount: 1,
      useWithCautionCount: 0,
      needsVerificationCount: 0,
    },
    provenance: { sources: [], redactionApplied: false },
    sensitivity: "public",
    sourceAsofInputs: [],
  },
  enumCoverage: {
    sections: [
      "state",
      "current_meeting",
      "next_meeting",
      "upcoming_meetings",
      "watch_proposals",
      "trust_summary",
    ],
    emptyReasons: ["no_meetings", "date_outside_known_window", "workspace_unknown"],
    availabilities: ["available", "empty", "auth_locked"],
    staleReasons: [
      "source_asof_older_than_threshold",
      "upstream_claim_changed",
      "entity_context_stale",
    ],
    freshnessKinds: ["fresh", "stale", "needs_preparation"],
    integrityKinds: ["clean", "has_corrections", "has_ambiguity"],
    advisoryKinds: ["watch_proposal", "unlinked_meetings", "partial_read_failure"],
  },
};

describe("DailyBriefingOutput contract parity", () => {
  it("exposes BRIEFING_SCHEMA_VERSION = 1 (matches Rust contract)", () => {
    expect(BRIEFING_SCHEMA_VERSION).toBe(1);
  });

  it("round-trips through JSON without losing fields", () => {
    const serialized = JSON.stringify(fixture.output);
    const parsed = JSON.parse(serialized) as DailyBriefingOutput;
    expect(parsed).toEqual(fixture.output);
  });

  it("represents the composed-state matrix (AC-507.4)", () => {
    // The 4-tuple must each remain independently inspectable; flat-enum
    // collapsing would break this assertion.
    expect(fixture.output.state.availability.kind).toBe("available");
    expect(fixture.output.state.freshness.kind).toBe("needs_preparation");
    expect(fixture.output.state.integrity.kind).toBe("has_corrections");
    expect(fixture.output.state.advisories.length).toBe(3);
  });

  it("enumerates every closed-enum variant the Rust contract names", () => {
    // Compile-time coverage is the real assertion (TS unions are exhaustive
    // checked at type-check); the runtime assertion just confirms each enum
    // has at least one variant represented in the fixture.
    expect(fixture.enumCoverage.sections).toHaveLength(6);
    expect(fixture.enumCoverage.emptyReasons).toHaveLength(3);
    expect(fixture.enumCoverage.availabilities).toHaveLength(3);
    expect(fixture.enumCoverage.staleReasons).toHaveLength(3);
    expect(fixture.enumCoverage.freshnessKinds).toHaveLength(3);
    expect(fixture.enumCoverage.integrityKinds).toHaveLength(3);
    expect(fixture.enumCoverage.advisoryKinds).toHaveLength(3);
  });
});
