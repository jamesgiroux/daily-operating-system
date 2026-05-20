// DOS-459 — parity golden test for `EntityIntelligenceEnvelope` TS mirror.
// Mirrors the pattern from `src/services/claim-receipt/__tests__/contracts.golden.test.ts`.
// Asserts every closed enum variant from the Rust contract is representable in the
// TypeScript mirror and that a representative envelope round-trips through JSON.

import { describe, expect, it } from "vitest";

import {
  ENVELOPE_SCHEMA_VERSION,
  type ClaimSensitivity,
  type ClaimState,
  type ClaimVerificationState,
  type ContextDepth,
  type CursorState,
  type EmptyReason,
  type EntityIntelligenceEnvelope,
  type EntityKind,
  type EnvelopeSection,
  type ExclusionReason,
  type Freshness,
  type InclusionReason,
  type RenderSurface,
  type SectionState,
  type SubjectRef,
  type SurfacingState,
  type TouchpointKind,
  type TrustBand,
} from "../contracts";

interface ContractGoldenFixture {
  envelope: EntityIntelligenceEnvelope;
  enumCoverage: {
    entityKinds: EntityKind[];
    contextDepths: ContextDepth[];
    envelopeSections: EnvelopeSection[];
    cursorStates: CursorState[];
    emptyReasons: EmptyReason[];
    sectionStates: SectionState[];
    subjectRefs: SubjectRef[];
    trustBands: TrustBand[];
    freshness: Freshness[];
    claimStates: ClaimState[];
    surfacingStates: SurfacingState[];
    verificationStates: ClaimVerificationState[];
    claimSensitivities: ClaimSensitivity[];
    renderSurfaces: RenderSurface[];
    touchpointKinds: TouchpointKind[];
    inclusionReasons: InclusionReason[];
    exclusionReasons: ExclusionReason[];
  };
}

const goldenFixture = {
  envelope: {
    schemaVersion: ENVELOPE_SCHEMA_VERSION,
    subject: {
      kind: "account",
      id: "account-1",
      subjectRef: { account: "account-1" },
      displayLabel: "account:account-1",
    },
    sections: {
      facts: { kind: "present", item_count: 1 },
      health: { kind: "empty", reason: "not_processed_yet" },
      metadata_proposals: { kind: "empty", reason: "no_evidence_backed_proposal" },
      open_loops: { kind: "empty", reason: "stale" },
      touchpoints: { kind: "empty", reason: "no_relevant_touchpoints" },
      threads: { kind: "empty", reason: "not_processed_yet" },
      record: { kind: "present", item_count: 1 },
    },
    facts: {
      items: [
        {
          claimId: "claim-1",
          subjectRef: { account: "account-1" },
          fieldPath: "health.summary",
          claimType: "account_health",
          renderedText: {
            text: "Account health is stable.",
            policy: {
              kind: "render",
              sensitivity: "internal",
              surface: "tauri_entity_detail",
              claimId: "claim-1",
              affordance: null,
            },
          },
          trustBand: "likely_current",
          freshness: "current",
          sourceAsof: "2026-05-19T12:00:00Z",
          sensitivity: "internal",
          lifecycleState: "active",
          surfacingState: "active",
          verificationState: "active",
          provenance: { sourceIds: ["claim_source:claim-1"] },
        },
      ],
      nextCursor: null,
      totalHint: 1,
      cursorState: { kind: "stable" },
    },
    healthStory: null,
    metadataProposals: {
      items: [],
      nextCursor: null,
      totalHint: 0,
      cursorState: { kind: "stable" },
    },
    openLoops: {
      items: [],
      nextCursor: null,
      totalHint: 0,
      cursorState: { kind: "stable" },
    },
    touchpoints: {
      items: [
        {
          upcoming: {
            items: [],
            nextCursor: null,
            totalHint: 0,
            cursorState: { kind: "stable" },
          },
          recent: {
            items: [],
            nextCursor: null,
            totalHint: 0,
            cursorState: { kind: "stable" },
          },
          candidateSet: {
            windowStart: null,
            windowEnd: null,
            filterDescription: "no candidate set yet",
          },
          emptyReason: "no_relevant_touchpoints",
          subjectScope: {
            primary: { account: "account-1" },
            alsoIncludes: [],
          },
        },
      ],
      nextCursor: null,
      totalHint: 1,
      cursorState: { kind: "stable" },
    },
    threads: {
      items: [],
      nextCursor: null,
      totalHint: 0,
      cursorState: { kind: "stable" },
    },
    recordEntries: {
      items: [
        {
          claimId: "claim-1",
          subjectRef: { account: "account-1" },
          claimType: "account_health",
          recordedAt: "2026-05-19T12:00:00Z",
          renderedText: {
            text: "Account health is stable.",
            policy: {
              kind: "render",
              sensitivity: "internal",
              surface: "tauri_entity_detail",
              affordance: null,
            },
          },
          trustBand: "likely_current",
          sensitivity: "internal",
          provenance: { sourceIds: ["claim_source:claim-1"] },
        },
      ],
      nextCursor: null,
      totalHint: 1,
      cursorState: { kind: "stable" },
    },
    trust: {
      aggregateBand: "likely_current",
      sectionCaveats: {},
    },
    provenance: {
      sources: [
        {
          id: "claim_source:claim-1",
          label: "google",
          sourceType: "google",
          asOf: "2026-05-19T12:00:00Z",
          redacted: false,
        },
      ],
      redactionApplied: false,
    },
    sensitivity: "internal",
  },
  enumCoverage: {
    entityKinds: ["account", "project", "person"],
    contextDepths: ["shallow", "standard", "deep"],
    envelopeSections: [
      "facts",
      "health",
      "metadata_proposals",
      "open_loops",
      "touchpoints",
      "threads",
      "record",
    ],
    cursorStates: [
      { kind: "stable" },
      { kind: "data_shifted", advisory: "rows shifted" },
      { kind: "invalidated", reason: "schema changed", restart_required: true },
    ],
    emptyReasons: [
      "not_connected",
      "not_processed_yet",
      "filtered_out_by_subject",
      "no_relevant_touchpoints",
      "stale",
      "no_evidence_backed_proposal",
      "unsupported_for_subject",
      "not_requested",
      { partial_failure: { advisory: "section error" } },
    ],
    sectionStates: [
      { kind: "present", item_count: 1 },
      { kind: "empty", reason: "stale" },
    ],
    subjectRefs: [
      { account: "account-1" },
      { project: "project-1" },
      { person: "person-1" },
      { meeting: "meeting-1" },
      { user: "user-1" },
      "global",
      { multi: [{ account: "a-1" }, { meeting: "m-1" }] },
      "unknown",
    ],
    trustBands: ["likely_current", "use_with_caution", "needs_verification", "unscored"],
    freshness: ["current", "aging", "stale", "unknown"],
    claimStates: ["active", "dormant", "tombstoned", "withdrawn"],
    surfacingStates: ["active", "dormant"],
    verificationStates: ["active", "contested", "needs_user_decision"],
    claimSensitivities: ["public", "internal", "confidential", "user_only"],
    renderSurfaces: [
      "tauri_entity_detail",
      "tauri_briefing_prep",
      "tauri_meeting_detail",
      "tauri_email_summary",
      "action",
      "tauri_provenance",
      "tauri_report",
      "tauri_chat",
      "mcp_tool",
      "mcp_tool_detail",
      "p2_publication",
      "log_structured",
      "push_notification",
    ],
    touchpointKinds: ["meeting", "email_thread", "document", "salesforce", "linear"],
    inclusionReasons: ["subject_match", "entity_link", "attendee_match", "domain_match"],
    exclusionReasons: ["subject_mismatch", "outside_window", "low_confidence", "suppressed"],
  },
} satisfies ContractGoldenFixture;

describe("entity intelligence envelope contract golden fixture", () => {
  it("parses representative envelope JSON as EntityIntelligenceEnvelope", () => {
    const parsedFixture = JSON.parse(JSON.stringify(goldenFixture)) as ContractGoldenFixture;
    expect(parsedFixture.envelope.schemaVersion).toBe(ENVELOPE_SCHEMA_VERSION);
    expect(parsedFixture.envelope.subject.kind).toBe("account");
    expect(parsedFixture.envelope.facts.items[0]?.claimId).toBe("claim-1");
    // AC-459.9 — every list-shape field carries `cursorState`.
    expect(parsedFixture.envelope.facts.cursorState).toEqual({ kind: "stable" });
    expect(parsedFixture.envelope.metadataProposals.cursorState).toEqual({ kind: "stable" });
    expect(parsedFixture.envelope.openLoops.cursorState).toEqual({ kind: "stable" });
    expect(parsedFixture.envelope.touchpoints.cursorState).toEqual({ kind: "stable" });
    expect(parsedFixture.envelope.threads.cursorState).toEqual({ kind: "stable" });
    expect(parsedFixture.envelope.recordEntries.cursorState).toEqual({ kind: "stable" });
  });

  it("sections map enumerates all 7 EnvelopeSection variants (AC-459.2)", () => {
    const { sections } = goldenFixture.envelope;
    const keys = Object.keys(sections).sort();
    expect(keys).toEqual([
      "facts",
      "health",
      "metadata_proposals",
      "open_loops",
      "record",
      "threads",
      "touchpoints",
    ]);
  });

  it("every empty section carries a typed reason (AC-459.2)", () => {
    const { sections } = goldenFixture.envelope;
    for (const state of Object.values(sections)) {
      if (state?.kind === "empty") {
        expect(state.reason).toBeDefined();
      }
    }
  });

  it("per-fact provenance is ProvenanceRef, not inlined ProvenanceSource[] (architecture F9)", () => {
    const fact = goldenFixture.envelope.facts.items[0];
    expect(fact?.provenance.sourceIds).toEqual(["claim_source:claim-1"]);
    // Top-level envelope provenance carries the actual source descriptors.
    expect(goldenFixture.envelope.provenance.sources[0]?.id).toBe("claim_source:claim-1");
  });

  it("covers every closed enum variant mirrored from Rust", () => {
    const { enumCoverage } = goldenFixture;
    expect(enumCoverage.entityKinds).toHaveLength(3);
    expect(enumCoverage.contextDepths).toHaveLength(3);
    expect(enumCoverage.envelopeSections).toHaveLength(7);
    expect(enumCoverage.cursorStates).toHaveLength(3);
    expect(enumCoverage.emptyReasons).toHaveLength(9);
    expect(enumCoverage.subjectRefs).toHaveLength(8);
    expect(enumCoverage.trustBands).toHaveLength(4);
    expect(enumCoverage.freshness).toHaveLength(4);
    expect(enumCoverage.claimStates).toHaveLength(4);
    expect(enumCoverage.surfacingStates).toHaveLength(2);
    expect(enumCoverage.verificationStates).toHaveLength(3);
    expect(enumCoverage.claimSensitivities).toHaveLength(4);
    expect(enumCoverage.renderSurfaces).toHaveLength(13);
    expect(enumCoverage.touchpointKinds).toHaveLength(5);
    expect(enumCoverage.inclusionReasons).toHaveLength(4);
    expect(enumCoverage.exclusionReasons).toHaveLength(4);
  });
});
