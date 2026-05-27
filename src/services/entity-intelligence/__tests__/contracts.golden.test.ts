// Parity golden test for `EntityIntelligenceEnvelope` TS mirror.
// Mirrors the pattern from `src/services/claim-receipt/__tests__/contracts.golden.test.ts`.
// Asserts every closed enum variant from the Rust contract is representable in the
// TypeScript mirror and that a representative envelope round-trips through JSON.

import { describe, expect, it } from "vitest";

import {
  ENVELOPE_SCHEMA_VERSION,
  ENVELOPE_SCHEMA_VERSION_V1,
  ENVELOPE_SCHEMA_VERSION_V2,
  type ClaimSensitivity,
  type ClaimState,
  type ClaimVerificationState,
  type ContextDepth,
  type CursorState,
  type EmptyReasonV1,
  type EmptyReasonV2,
  type EntityIntelligenceEnvelopeV1,
  type EntityIntelligenceEnvelopeV2,
  type EntityIntelligenceInputV1,
  type EntityIntelligenceInputV2,
  type EntityKind,
  type EnvelopeSectionV1,
  type EnvelopeSectionV2,
  type ExclusionReason,
  type Freshness,
  type InclusionReason,
  type RenderSurface,
  type RelationshipInclusionReason,
  type SectionStateV1,
  type SectionStateV2,
  type SubjectRef,
  type SurfacingState,
  type TouchpointKind,
  type TrustBand,
} from "../contracts";

interface SharedEnumCoverage {
  entityKinds: EntityKind[];
  contextDepths: ContextDepth[];
  cursorStates: CursorState[];
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
}

interface ContractGoldenFixtureV1 {
  envelope: EntityIntelligenceEnvelopeV1;
  enumCoverage: {
    envelopeSections: EnvelopeSectionV1[];
    emptyReasons: EmptyReasonV1[];
    sectionStates: SectionStateV1[];
  } & SharedEnumCoverage;
}

interface SchemaV2EnumCoverage {
  envelopeSections: EnvelopeSectionV2[];
  emptyReasons: EmptyReasonV2[];
  sectionStates: SectionStateV2[];
  relationshipInclusionReasons: RelationshipInclusionReason[];
}

const schemaV1SectionRequest = {
  schemaVersion: ENVELOPE_SCHEMA_VERSION_V1,
  entityType: "account",
  entityId: "account-1",
  depth: "standard",
  sections: ["facts"],
} satisfies EntityIntelligenceInputV1;

const schemaV2RelationshipsSectionRequest = {
  schemaVersion: ENVELOPE_SCHEMA_VERSION_V2,
  entityType: "account",
  entityId: "account-1",
  depth: "standard",
  sections: ["relationships"],
} satisfies EntityIntelligenceInputV2;

if (false) {
  const schemaV1RelationshipsSectionRequest = {
    schemaVersion: ENVELOPE_SCHEMA_VERSION_V1,
    entityType: "account",
    entityId: "account-1",
    depth: "standard",
    sections: [
      // @ts-expect-error Schema v1 cannot request schema v2 relationships.
      "relationships",
    ],
  } satisfies EntityIntelligenceInputV1;

  void schemaV1RelationshipsSectionRequest;
}

const schemaV2EnumCoverage = {
  envelopeSections: [
    "facts",
    "health",
    "metadata_proposals",
    "open_loops",
    "relationships",
    "touchpoints",
    "threads",
    "record",
  ],
  emptyReasons: [
    "not_connected",
    "not_processed_yet",
    "filtered_out_by_subject",
    "no_relevant_touchpoints",
    "no_relevant_relationships",
    "stale",
    "no_evidence_backed_proposal",
    "unsupported_for_subject",
    "not_requested",
    { partial_failure: { advisory: "section error" } },
  ],
  sectionStates: [
    { kind: "present", item_count: 1 },
    { kind: "empty", reason: "no_relevant_relationships" },
  ],
  relationshipInclusionReasons: [
    "subject_match",
    "hierarchy",
    "explicit_link",
    "attendee_match",
    "co_attendance",
    "work_item",
    "content_link",
  ],
} satisfies SchemaV2EnumCoverage;

const schemaV1GoldenFixture = {
  envelope: {
    schemaVersion: ENVELOPE_SCHEMA_VERSION_V1,
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
          label: "fixture_source",
          sourceType: "fixture_source",
          asOf: "2026-05-19T12:00:00Z",
          redacted: false,
        },
      ],
      redactionApplied: false,
    },
    sensitivity: "internal",
  },
  enumCoverage: {
    entityKinds: ["account", "project", "person", "meeting"],
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
} satisfies ContractGoldenFixtureV1;

const displayLabelPolicy = {
  kind: "render",
  sensitivity: "internal",
  surface: "tauri_entity_detail",
  claimId: null,
  affordance: null,
} as const;

const schemaV2Envelope = {
  ...schemaV1GoldenFixture.envelope,
  schemaVersion: ENVELOPE_SCHEMA_VERSION_V2,
  sections: {
    ...schemaV1GoldenFixture.envelope.sections,
    relationships: { kind: "present", item_count: 1 },
  },
  relationships: {
    items: [
      {
        edges: {
          items: [
            {
              edgeId: "relationship-edge-1",
              edgeType: "account_participant",
              subjectRef: { account: "account-1" },
              relatedSubjectRef: { person: "person-1" },
              relatedDisplayLabel: {
                text: "Example Person",
                policy: displayLabelPolicy,
              },
              observedAt: "2026-05-20T12:00:00Z",
              sourceAsof: "2026-05-20T12:00:00Z",
              confidence: 0.92,
              sensitivity: "internal",
              inclusionReason: "attendee_match",
              traversalDepth: 1,
              trustBand: "likely_current",
              freshness: "current",
              provenance: { sourceIds: ["relationship_source:relationship-edge-1"] },
              caveats: [],
            },
          ],
          nextCursor: null,
          totalHint: 1,
          cursorState: { kind: "stable" },
        },
        participants: {
          items: [
            {
              subjectRef: { person: "person-1" },
              displayLabel: {
                text: "Example Person",
                policy: displayLabelPolicy,
              },
              role: {
                text: "Executive sponsor",
                policy: displayLabelPolicy,
              },
              relationship: {
                text: "stakeholder",
                policy: displayLabelPolicy,
              },
              sensitivity: "internal",
              normalizedTouchpointCount: 3,
              recentTouchpointIds: ["meeting-1", "meeting-2"],
              lastSeenAt: "2026-05-20T12:00:00Z",
              trustBand: "likely_current",
              freshness: "current",
              provenance: { sourceIds: ["relationship_source:person-1"] },
              caveats: [],
            },
          ],
          nextCursor: null,
          totalHint: 1,
          cursorState: { kind: "stable" },
        },
        candidateSet: {
          windowStart: null,
          windowEnd: "2026-05-20T12:00:00Z",
          filterDescription: "account relationship neighborhood",
        },
        emptyReason: null,
        subjectScope: {
          primary: { account: "account-1" },
          alsoIncludes: [{ person: "person-1" }],
        },
        truncation: {
          edgesTruncated: false,
          participantsTruncated: false,
          perEdgeCap: 50,
        },
        caveats: [],
      },
    ],
    nextCursor: null,
    totalHint: 1,
    cursorState: { kind: "stable" },
  },
} satisfies EntityIntelligenceEnvelopeV2;

describe("entity intelligence envelope contract golden fixture", () => {
  it("versions section request types by schema", () => {
    expect(schemaV1SectionRequest.sections).toEqual(["facts"]);
    expect(schemaV2RelationshipsSectionRequest.sections).toEqual(["relationships"]);
  });

  it("parses representative schema v1 envelope JSON without relationships", () => {
    const parsedFixture = JSON.parse(
      JSON.stringify(schemaV1GoldenFixture),
    ) as ContractGoldenFixtureV1;
    expect(parsedFixture.envelope.schemaVersion).toBe(ENVELOPE_SCHEMA_VERSION_V1);
    expect(parsedFixture.envelope.subject.kind).toBe("account");
    expect(parsedFixture.envelope.facts.items[0]?.claimId).toBe("claim-1");
    expect("relationships" in parsedFixture.envelope).toBe(false);
    // AC-459.9 — every list-shape field carries `cursorState`.
    expect(parsedFixture.envelope.facts.cursorState).toEqual({ kind: "stable" });
    expect(parsedFixture.envelope.metadataProposals.cursorState).toEqual({ kind: "stable" });
    expect(parsedFixture.envelope.openLoops.cursorState).toEqual({ kind: "stable" });
    expect(parsedFixture.envelope.touchpoints.cursorState).toEqual({ kind: "stable" });
    expect(parsedFixture.envelope.threads.cursorState).toEqual({ kind: "stable" });
    expect(parsedFixture.envelope.recordEntries.cursorState).toEqual({ kind: "stable" });
  });

  it("parses representative schema v2 envelope JSON with relationships", () => {
    const parsedEnvelope = JSON.parse(
      JSON.stringify(schemaV2Envelope),
    ) as EntityIntelligenceEnvelopeV2;
    expect(parsedEnvelope.schemaVersion).toBe(ENVELOPE_SCHEMA_VERSION);
    expect(parsedEnvelope.schemaVersion).toBe(ENVELOPE_SCHEMA_VERSION_V2);
    expect(parsedEnvelope.relationships?.cursorState).toEqual({ kind: "stable" });
    expect(parsedEnvelope.relationships?.items[0]?.edges.cursorState).toEqual({ kind: "stable" });
    expect(parsedEnvelope.relationships?.items[0]?.participants.cursorState).toEqual({
      kind: "stable",
    });
    expect(parsedEnvelope.relationships?.items[0]?.edges.items[0]?.relatedSubjectRef).toEqual({
      person: "person-1",
    });
  });

  it("schema v1 sections map enumerates all 7 EnvelopeSection variants (AC-459.2)", () => {
    const { sections } = schemaV1GoldenFixture.envelope;
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

  it("schema v2 sections map enumerates all 8 EnvelopeSection variants", () => {
    const keys = Object.keys(schemaV2Envelope.sections).sort();
    expect(keys).toEqual([
      "facts",
      "health",
      "metadata_proposals",
      "open_loops",
      "record",
      "relationships",
      "threads",
      "touchpoints",
    ]);
  });

  it("every empty section carries a typed reason (AC-459.2)", () => {
    const { sections } = schemaV1GoldenFixture.envelope;
    for (const state of Object.values(sections)) {
      if (state?.kind === "empty") {
        expect(state.reason).toBeDefined();
      }
    }
  });

  it("per-fact provenance is ProvenanceRef, not inlined ProvenanceSource[] (architecture F9)", () => {
    const fact = schemaV1GoldenFixture.envelope.facts.items[0];
    expect(fact?.provenance.sourceIds).toEqual(["claim_source:claim-1"]);
    // Top-level envelope provenance carries the actual source descriptors.
    expect(schemaV1GoldenFixture.envelope.provenance.sources[0]?.id).toBe(
      "claim_source:claim-1",
    );
  });

  it("keeps schema v1 enum coverage schema v1 only", () => {
    const { enumCoverage } = schemaV1GoldenFixture;
    expect(enumCoverage.entityKinds).toHaveLength(4);
    expect(enumCoverage.contextDepths).toHaveLength(3);
    expect(enumCoverage.envelopeSections).toHaveLength(7);
    expect(enumCoverage.envelopeSections).not.toContain("relationships");
    expect(enumCoverage.cursorStates).toHaveLength(3);
    expect(enumCoverage.emptyReasons).toHaveLength(9);
    expect(enumCoverage.emptyReasons).not.toContain("no_relevant_relationships");
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

  it("covers schema v2 relationship-only enum variants", () => {
    expect(schemaV2EnumCoverage.envelopeSections).toHaveLength(8);
    expect(schemaV2EnumCoverage.envelopeSections).toContain("relationships");
    expect(schemaV2EnumCoverage.emptyReasons).toHaveLength(10);
    expect(schemaV2EnumCoverage.emptyReasons).toContain("no_relevant_relationships");
    expect(schemaV2EnumCoverage.sectionStates).toContainEqual({
      kind: "empty",
      reason: "no_relevant_relationships",
    });
    expect(schemaV2EnumCoverage.relationshipInclusionReasons).toHaveLength(7);
  });
});
