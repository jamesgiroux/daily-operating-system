import { describe, expect, it } from "vitest";

import {
  RECOMMENDATION_METADATA_SCHEMA_VERSION,
  type ConversionState,
  type ConversionTarget,
  type DeferReason,
  type DismissReason,
  type EngagementSignal,
  type FactorRationale,
  type FeedbackState,
  type RecommendationFeedbackContext,
  type RecommendationMetadataEnvelope,
  type RecommendedAction,
  type SalienceFactorKind,
  type SurfacingDecision,
  type SurfacingTier,
  type SuppressReason,
  type TriggerKind,
} from "../contracts";

interface ContractGoldenFixture {
  metadata: RecommendationMetadataEnvelope;
  decidedFeedback: FeedbackState;
  conversionTarget: ConversionTarget;
  conversionState: ConversionState;
  surfacing: SurfacingDecision;
  engagement: EngagementSignal;
  feedbackContext: RecommendationFeedbackContext;
  enumCoverage: {
    recommendedActions: RecommendedAction[];
    dismissReasons: DismissReason[];
    conversionTargets: ConversionTarget[];
    conversionStates: ConversionState[];
    salienceFactorKinds: SalienceFactorKind[];
    factorRationales: FactorRationale[];
    surfacingTiers: SurfacingTier[];
    deferReasons: DeferReason[];
    suppressReasons: SuppressReason[];
    triggerKinds: TriggerKind[];
    engagementSignals: EngagementSignal[];
  };
}

const at = "2026-05-26T12:00:00Z";

const goldenFixture = {
  metadata: {
    recommendation: {
      schemaVersion: RECOMMENDATION_METADATA_SCHEMA_VERSION,
      recommendedAction: {
        kind: "scheduleMeeting",
        entityId: "acct-example",
        whenWindow: "next_week",
        rationale: "recent support trend needs follow-up",
      },
      evidence: [
        {
          source: "claim:claim-source-1",
          chunk: "chunk-1",
        },
      ],
      salience: {
        total: 0.73,
        factors: [
          {
            kind: "urgency",
            value: 0.8,
            weight: 0.17,
            rationale: {
              kind: "urgency",
              deadline: "2026-05-30T12:00:00Z",
              decayFactor: 0.92,
            },
          },
        ],
      },
      feedbackState: "pending",
      conversionState: {
        kind: "notConverted",
      },
    },
  },
  decidedFeedback: {
    decided: {
      kind: "dismiss",
      at,
      reason: {
        other: "already handled elsewhere",
      },
    },
  },
  conversionTarget: {
    claimCorrection: "claim-correction-1",
  },
  conversionState: {
    kind: "convertedToReviewQueue",
    queueItemId: "queue-item-1",
  },
  surfacing: {
    kind: "render",
    tier: "notable",
    whyThisNow: {
      primaryFactor: "urgency",
      text: "Salience driven by urgency.",
      triggers: [
        {
          triggerKind: "signalArrival",
          at,
          source: "signal:workspace-file-changed",
        },
      ],
    },
  },
  engagement: {
    kind: "ignored",
    surface: "tauri_entity_detail",
    renderAt: at,
    ignoredAt: at,
  },
  feedbackContext: {
    surface: "tauri_entity_detail",
    invocationId: "invocation-1",
  },
  enumCoverage: {
    recommendedActions: [
      {
        kind: "scheduleMeeting",
        entityId: "acct-example",
        whenWindow: "next_week",
        rationale: "recent support trend needs follow-up",
      },
      {
        kind: "sendMessage",
        entityId: "person-example",
        channel: "email",
        suggestedTopic: "follow up on open loop",
      },
      {
        kind: "reviewClaim",
        claimId: "claim-1",
        reason: "source changed",
      },
      {
        kind: "updateRecord",
        entityId: "acct-example",
        fieldPath: "account.health",
        suggestedValue: "stable",
      },
      {
        kind: "investigateChange",
        entityId: "acct-example",
        changeSummary: "usage changed",
      },
      {
        kind: "custom",
        actionKind: "partnerWorkflow",
        payload: { workflowId: "workflow-1" },
      },
    ],
    dismissReasons: ["notRelevant", "alreadyKnew", "wrongSubject", { other: "note" }],
    conversionTargets: [
      { action: "action-1" },
      { claimCorrection: "claim-2" },
      { reviewQueue: "queue-1" },
    ],
    conversionStates: [
      { kind: "notConverted" },
      { kind: "convertedToAction", actionId: "action-1" },
      { kind: "convertedToClaimCorrection", claimId: "claim-2" },
      { kind: "convertedToReviewQueue", queueItemId: "queue-1" },
    ],
    salienceFactorKinds: [
      "importance",
      "novelty",
      "urgency",
      "timing",
      "userFit",
      "freshness",
      "trust",
      "corroboration",
      "contradiction",
      "openLoopRelevance",
    ],
    factorRationales: [
      { kind: "importance", trustBand: "likely_current", sourceAuthority: 0.9 },
      { kind: "novelty", vectorDistance: 0.3, neighborCount: 2 },
      { kind: "urgency", deadline: null, decayFactor: 0.6 },
      { kind: "timing", signalAgeSecs: 60, calendarProximitySecs: null },
      { kind: "userFit", feedbackHistoryScore: 0.5 },
      { kind: "freshness", decayFactor: 0.8 },
      { kind: "trust", trustBand: "use_with_caution" },
      { kind: "corroboration", corroborationCount: 2 },
      { kind: "contradiction", contradictionCount: 1 },
      { kind: "openLoopRelevance", openLoopCount: 3, hasAction: true },
    ],
    surfacingTiers: ["critical", "notable", "background", "quiet"],
    deferReasons: [
      "cooldownActive",
      "budgetExhausted",
      "awaitingCorroboration",
      "pendingTrigger",
    ],
    suppressReasons: [
      "belowThreshold",
      "userMutedSubject",
      "dismissedRecently",
      "contradictedWithStrongerEvidence",
    ],
    triggerKinds: ["signalArrival", "entityChange", "scheduledScan", "feedbackEcho"],
    engagementSignals: [
      { kind: "rendered", surface: "action", at },
      { kind: "clicked", surface: "action", at },
      { kind: "dismissed", surface: "action", at },
      { kind: "ignored", surface: "action", renderAt: at, ignoredAt: at },
    ],
  },
} satisfies ContractGoldenFixture;

describe("recommendations contract golden", () => {
  it("keeps the TypeScript mirror aligned with the Rust serde shape", () => {
    const parsed = JSON.parse(JSON.stringify(goldenFixture)) as ContractGoldenFixture;

    expect(parsed).toEqual(goldenFixture);
    expect(parsed.metadata.recommendation.schemaVersion).toBe(1);
    expect(parsed.metadata.recommendation.recommendedAction.kind).toBe("scheduleMeeting");
    expect(parsed.metadata.recommendation.feedbackState).toBe("pending");
    expect(parsed.decidedFeedback).toEqual({
      decided: {
        kind: "dismiss",
        at,
        reason: { other: "already handled elsewhere" },
      },
    });
    expect(parsed.conversionTarget).toEqual({ claimCorrection: "claim-correction-1" });
    expect(parsed.conversionState).toEqual({
      kind: "convertedToReviewQueue",
      queueItemId: "queue-item-1",
    });
    expect(parsed.engagement.kind).toBe("ignored");
  });

  it("covers every closed enum and externally tagged tuple shape", () => {
    const coverage = goldenFixture.enumCoverage;

    expect(coverage.recommendedActions.map((action) => action.kind)).toEqual([
      "scheduleMeeting",
      "sendMessage",
      "reviewClaim",
      "updateRecord",
      "investigateChange",
      "custom",
    ]);
    expect(coverage.dismissReasons).toEqual([
      "notRelevant",
      "alreadyKnew",
      "wrongSubject",
      { other: "note" },
    ]);
    expect(coverage.conversionTargets).toEqual([
      { action: "action-1" },
      { claimCorrection: "claim-2" },
      { reviewQueue: "queue-1" },
    ]);
    expect(coverage.conversionStates.map((state) => state.kind)).toEqual([
      "notConverted",
      "convertedToAction",
      "convertedToClaimCorrection",
      "convertedToReviewQueue",
    ]);
    expect(coverage.salienceFactorKinds).toHaveLength(10);
    expect(coverage.factorRationales.map((rationale) => rationale.kind)).toEqual([
      "importance",
      "novelty",
      "urgency",
      "timing",
      "userFit",
      "freshness",
      "trust",
      "corroboration",
      "contradiction",
      "openLoopRelevance",
    ]);
    expect(coverage.surfacingTiers).toHaveLength(4);
    expect(coverage.deferReasons).toHaveLength(4);
    expect(coverage.suppressReasons).toHaveLength(4);
    expect(coverage.triggerKinds).toHaveLength(4);
    expect(coverage.engagementSignals.map((signal) => signal.kind)).toEqual([
      "rendered",
      "clicked",
      "dismissed",
      "ignored",
    ]);
  });
});
