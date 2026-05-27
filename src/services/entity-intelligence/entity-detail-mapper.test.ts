import { describe, expect, it } from "vitest";

import type { EntityIntelligence } from "@/types";
import type {
  EntityFact,
  EntityIntelligenceEnvelope,
  Paginated,
} from "./contracts";
import type { EntityIntelligenceAbilityResponse } from "./invoke";
import { mergeEntityDetailIntelligence } from "./entity-detail-mapper";

function emptyPage<T>(): Paginated<T> {
  return {
    items: [],
    nextCursor: null,
    totalHint: 0,
    cursorState: { kind: "stable" },
  };
}

function fact(overrides: Partial<EntityFact>): EntityFact {
  const claimId = overrides.claimId ?? "claim-1";
  return {
    claimId,
    subjectRef: { account: "account-1" },
    fieldPath: null,
    claimType: "entity_summary",
    renderedText: {
      text: "Claim-backed summary.",
      policy: {
        kind: "render",
        sensitivity: "internal",
        surface: "tauri_entity_detail",
        claimId,
        affordance: null,
      },
    },
    trustBand: "likely_current",
    freshness: "current",
    sourceAsof: "2026-05-20T12:00:00Z",
    sensitivity: "internal",
    lifecycleState: "active",
    surfacingState: "active",
    verificationState: "active",
    provenance: { sourceIds: ["source-1"] },
    ...overrides,
  };
}

function responseWithFacts(facts: EntityFact[]): EntityIntelligenceAbilityResponse {
  const envelope: EntityIntelligenceEnvelope = {
    schemaVersion: 1,
    subject: {
      kind: "account",
      id: "account-1",
      subjectRef: { account: "account-1" },
      displayLabel: "account:account-1",
    },
    sections: {
      facts: { kind: "present", item_count: facts.length },
      open_loops: { kind: "present", item_count: 1 },
      record: { kind: "present", item_count: facts.length },
    },
    facts: {
      ...emptyPage<EntityFact>(),
      items: facts,
      totalHint: facts.length,
    },
    healthStory: null,
    metadataProposals: emptyPage(),
    openLoops: {
      ...emptyPage(),
      items: [
        {
          openLoop: {
            id: "loop-1",
            subject: { entity_type: "account", entity_id: "account-1" },
            loop_kind: "meeting_follow_up",
            description: "Confirm rollout date.",
            owner: "Customer Success",
            due_date: "2026-05-28",
            status: "open",
            source_asof: "2026-05-22T12:00:00Z",
            claim_type: "open_loop",
          },
          receiptTarget: {
            claimId: "claim-loop-1",
            subjectRef: { account: "account-1" },
            fieldPath: "open_loops",
          },
          trustBand: "use_with_caution",
          freshness: "current",
          provenance: { sourceIds: ["source-1"] },
        },
      ],
      totalHint: 1,
    },
    touchpoints: emptyPage(),
    threads: emptyPage(),
    recordEntries: emptyPage(),
    trust: {
      aggregateBand: "likely_current",
      sectionCaveats: {},
    },
    provenance: {
      sources: [
        {
          id: "source-1",
          label: "Meeting transcript",
          sourceType: "meeting",
          asOf: "2026-05-21T12:00:00Z",
          redacted: false,
        },
      ],
      redactionApplied: false,
    },
    sensitivity: "internal",
  };

  return {
    invocation_id: "inv-1",
    ability_name: "get_entity_intelligence",
    ability_version: "0.1.0",
    schema_version: 1,
    data: envelope,
    rendered_provenance: {
      value: { produced_at: "2026-05-19T12:00:00Z" },
    },
  };
}

function legacyIntelligence(): EntityIntelligence {
  return {
    version: 1,
    entityId: "account-1",
    entityType: "account",
    enrichedAt: "2026-05-01T12:00:00Z",
    sourceFileCount: 0,
    sourceManifest: [],
    executiveAssessment: "Legacy summary.",
    risks: [{ text: "Legacy risk.", urgency: "watch" }],
    recentWins: [{ text: "Legacy win." }],
    stakeholderInsights: [],
  };
}

describe("mergeEntityDetailIntelligence", () => {
  it("fails closed instead of rendering legacy intelligence when no claim envelope is available", () => {
    const legacy = legacyIntelligence();

    expect(mergeEntityDetailIntelligence(legacy, null)).toBeNull();
  });

  it("fails closed instead of rendering legacy intelligence when the envelope has no claim-backed content", () => {
    const legacy = legacyIntelligence();
    const empty = responseWithFacts([]);
    empty.data.openLoops.items = [];

    expect(mergeEntityDetailIntelligence(legacy, empty)).toBeNull();
  });

  it("returns null when a claim envelope and legacy intelligence are both empty", () => {
    const empty = responseWithFacts([]);
    empty.data.openLoops.items = [];

    expect(mergeEntityDetailIntelligence(null, empty)).toBeNull();
  });

  it("ignores facts and open loops that do not belong to the envelope subject", () => {
    const foreign = responseWithFacts([
      fact({
        claimId: "claim-foreign-summary",
        subjectRef: { account: "account-2" },
        claimType: "entity_summary",
        renderedText: {
          text: "Foreign summary.",
          policy: {
            kind: "render",
            sensitivity: "internal",
            surface: "tauri_entity_detail",
            claimId: "claim-foreign-summary",
            affordance: null,
          },
        },
      }),
    ]);
    foreign.data.openLoops.items = [
      {
        ...foreign.data.openLoops.items[0],
        openLoop: {
          ...foreign.data.openLoops.items[0].openLoop,
          subject: { entity_type: "account", entity_id: "account-2" },
        },
        receiptTarget: {
          ...foreign.data.openLoops.items[0].receiptTarget,
          subjectRef: { account: "account-2" },
        },
      },
    ];

    const merged = mergeEntityDetailIntelligence(legacyIntelligence(), foreign);

    expect(merged).toBeNull();
  });

  it("does not expose redacted source labels through detail provenance", () => {
    const response = responseWithFacts([
      fact({
        claimId: "claim-risk",
        claimType: "entity_risk",
        renderedText: {
          text: "Claim-backed risk.",
          policy: {
            kind: "render",
            sensitivity: "internal",
            surface: "tauri_entity_detail",
            claimId: "claim-risk",
            affordance: null,
          },
        },
      }),
    ]);
    response.data.provenance.sources[0] = {
      id: "source-1",
      label: "Sensitive transcript",
      sourceType: "meeting",
      asOf: "2026-05-21T12:00:00Z",
      redacted: true,
    };

    const merged = mergeEntityDetailIntelligence(legacyIntelligence(), response);

    expect(merged?.risks[0].itemSource).toEqual(expect.objectContaining({
      source: "Redacted source",
      reference: undefined,
    }));
    expect(merged?.sourceManifest).toEqual([
      {
        filename: "Redacted source",
        modifiedAt: "2026-05-21T12:00:00Z",
        format: undefined,
      },
    ]);
  });

  it("does not retain stale legacy source manifests for claim-backed content without source rows", () => {
    const legacy = legacyIntelligence();
    legacy.sourceFileCount = 1;
    legacy.sourceManifest = [
      { filename: "legacy.md", modifiedAt: "2026-05-01T12:00:00Z", format: "markdown" },
    ];
    const response = responseWithFacts([
      fact({
        claimId: "claim-summary",
        claimType: "entity_summary",
        provenance: { sourceIds: [] },
      }),
    ]);
    response.data.provenance.sources = [];

    const merged = mergeEntityDetailIntelligence(legacy, response);

    expect(merged?.executiveAssessment).toBe("Claim-backed summary.");
    expect(merged?.sourceFileCount).toBe(0);
    expect(merged?.sourceManifest).toEqual([]);
  });

  it("does not render incomplete claim fact arrays when the claim facts page is partial", () => {
    const legacy = legacyIntelligence();
    legacy.risks = [
      { text: "Legacy risk one.", urgency: "high" },
      { text: "Legacy risk two.", urgency: "medium" },
    ];
    const partial = responseWithFacts([
      fact({
        claimId: "claim-risk",
        claimType: "entity_risk",
        renderedText: {
          text: "First claim page risk.",
          policy: {
            kind: "render",
            sensitivity: "internal",
            surface: "tauri_entity_detail",
            claimId: "claim-risk",
            affordance: null,
          },
        },
      }),
    ]);
    partial.data.facts.nextCursor = "facts:offset=50";
    partial.data.facts.totalHint = 51;

    const merged = mergeEntityDetailIntelligence(legacy, partial);

    expect(merged?.risks).toEqual([]);
  });

  it("clears a stale legacy pull quote when a claim-backed summary replaces the assessment", () => {
    const legacy = legacyIntelligence();
    legacy.pullQuote = "Legacy quote.";

    const merged = mergeEntityDetailIntelligence(
      legacy,
      responseWithFacts([
        fact({
          claimId: "claim-summary",
          claimType: "entity_summary",
        }),
      ]),
    );

    expect(merged?.executiveAssessment).toBe("Claim-backed summary.");
    expect(merged?.pullQuote).toBeUndefined();
  });

  it("overlays scored claims and provenance onto the legacy detail model", () => {
    const merged = mergeEntityDetailIntelligence(
      legacyIntelligence(),
      responseWithFacts([
        fact({
          claimId: "claim-summary",
          claimType: "entity_summary",
          renderedText: {
            text: "Claim-backed summary.",
            policy: {
              kind: "render",
              sensitivity: "internal",
              surface: "tauri_entity_detail",
              claimId: "claim-summary",
              affordance: null,
            },
          },
        }),
        fact({
          claimId: "claim-risk",
          claimType: "entity_risk",
          renderedText: {
            text: "Claim-backed risk.",
            policy: {
              kind: "render",
              sensitivity: "internal",
              surface: "tauri_entity_detail",
              claimId: "claim-risk",
              affordance: null,
            },
          },
          trustBand: "needs_verification",
          sourceAsof: "2026-05-21T12:00:00Z",
        }),
        fact({
          claimId: "claim-win",
          claimType: "entity_win",
          renderedText: {
            text: "Claim-backed win.",
            policy: {
              kind: "render",
              sensitivity: "internal",
              surface: "tauri_entity_detail",
              claimId: "claim-win",
              affordance: null,
            },
          },
        }),
      ]),
    );

    expect(merged?.executiveAssessment).toBe("Claim-backed summary.");
    expect(merged?.executiveAssessmentRenderPolicy?.claimId).toBe("claim-summary");
    expect(merged?.risks).toEqual([
      expect.objectContaining({
        text: "Claim-backed risk.",
        claimId: "claim-risk",
        urgency: "medium",
        itemSource: expect.objectContaining({
          source: "Meeting transcript",
          confidence: 0.35,
          sourcedAt: "2026-05-21T12:00:00Z",
        }),
      }),
    ]);
    expect(merged?.recentWins).toEqual([
      expect.objectContaining({
        text: "Claim-backed win.",
        claimId: "claim-win",
      }),
    ]);
    expect(merged?.openCommitments).toEqual([
      expect.objectContaining({
        commitmentId: "claim-loop-1",
        description: "Confirm rollout date.",
        dueDate: "2026-05-28",
      }),
    ]);
    expect(merged?.sourceFileCount).toBe(1);
    expect(merged?.sourceManifest).toEqual([
      { filename: "Meeting transcript", modifiedAt: "2026-05-21T12:00:00Z", format: "meeting" },
    ]);
    expect(merged?.enrichedAt).toBe("2026-05-22T12:00:00Z");
  });
});
