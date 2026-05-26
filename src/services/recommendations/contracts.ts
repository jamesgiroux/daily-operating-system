import type {
  ClaimState,
  ClaimVerificationState,
  RenderSurface,
  SubjectRef,
  SurfacingState,
  TrustBand,
} from "../claim-receipt/contracts";

export const RECOMMENDATION_METADATA_SCHEMA_VERSION = 1;

export type ClaimId = string;
export type IsoDateTimeString = string;

export type JsonValue =
  | null
  | boolean
  | number
  | string
  | JsonValue[]
  | { [key: string]: JsonValue };

export interface EvidenceRef {
  source: string;
  chunk?: string | null;
}

export interface RecommendationDraft {
  subject: SubjectRef;
  recommendedAction: RecommendedAction;
  evidence: EvidenceRef[];
  provenanceJson: string;
  sourceRef?: string | null;
  sourceAsof?: IsoDateTimeString | null;
  observedAt: IsoDateTimeString;
  text: string;
  salience: SalienceScore;
}

export interface RecommendationClaim {
  claimId: ClaimId;
  subject: SubjectRef;
  recommendedAction: RecommendedAction;
  evidence: EvidenceRef[];
  provenance: JsonValue;
  trust: TrustBand;
  salience: SalienceScore;
  feedbackState: FeedbackState;
  conversionState: ConversionState;
  claimState: ClaimState;
  surfacingState: SurfacingState;
  verificationState: ClaimVerificationState;
  createdAt: IsoDateTimeString;
  updatedAt: IsoDateTimeString;
}

export type RecommendedAction =
  | {
      kind: "scheduleMeeting";
      entityId: string;
      whenWindow: string;
      rationale: string;
    }
  | {
      kind: "sendMessage";
      entityId: string;
      channel: string;
      suggestedTopic: string;
    }
  | {
      kind: "reviewClaim";
      claimId: ClaimId;
      reason: string;
    }
  | {
      kind: "updateRecord";
      entityId: string;
      fieldPath: string;
      suggestedValue: string;
    }
  | {
      kind: "investigateChange";
      entityId: string;
      changeSummary: string;
    }
  | {
      kind: "custom";
      actionKind: string;
      payload: JsonValue;
    };

export type FeedbackState =
  | "pending"
  | { decided: RecommendationFeedbackDecision };

export type RecommendationFeedbackDecision =
  | { kind: "accept"; at: IsoDateTimeString }
  | { kind: "dismiss"; at: IsoDateTimeString; reason: DismissReason }
  | { kind: "notUseful"; at: IsoDateTimeString }
  | { kind: "tooNoisy"; at: IsoDateTimeString }
  | { kind: "convert"; at: IsoDateTimeString; into: ConversionTarget };

export type BoundedNote = string;

export type DismissReason =
  | "notRelevant"
  | "alreadyKnew"
  | "wrongSubject"
  | { other: BoundedNote };

export interface RecommendationFeedbackContext {
  surface: RenderSurface;
  invocationId: string;
}

export type ConversionTarget =
  | { action: string }
  | { claimCorrection: ClaimId }
  | { reviewQueue: string };

export type ConversionState =
  | { kind: "notConverted" }
  | { kind: "convertedToAction"; actionId: string }
  | { kind: "convertedToClaimCorrection"; claimId: ClaimId }
  | { kind: "convertedToReviewQueue"; queueItemId: string };

export interface SalienceScore {
  total: number;
  factors: SalienceFactor[];
}

export interface SalienceFactor {
  kind: SalienceFactorKind;
  value?: number | null;
  weight: number;
  rationale: FactorRationale;
}

export type SalienceFactorKind =
  | "importance"
  | "novelty"
  | "urgency"
  | "timing"
  | "userFit"
  | "freshness"
  | "trust"
  | "corroboration"
  | "contradiction"
  | "openLoopRelevance";

export type FactorRationale =
  | { kind: "importance"; trustBand: TrustBand; sourceAuthority: number }
  | { kind: "novelty"; vectorDistance: number; neighborCount: number }
  | {
      kind: "urgency";
      deadline?: IsoDateTimeString | null;
      decayFactor: number;
    }
  | {
      kind: "timing";
      signalAgeSecs: number;
      calendarProximitySecs?: number | null;
    }
  | { kind: "userFit"; feedbackHistoryScore: number }
  | { kind: "freshness"; decayFactor: number }
  | { kind: "trust"; trustBand: TrustBand }
  | { kind: "corroboration"; corroborationCount: number }
  | { kind: "contradiction"; contradictionCount: number }
  | {
      kind: "openLoopRelevance";
      openLoopCount: number;
      hasAction: boolean;
    };

export interface WhyThisNow {
  primaryFactor: SalienceFactorKind;
  text: string;
  triggers: TriggerRef[];
}

export type SurfacingDecision =
  | { kind: "render"; tier: SurfacingTier; whyThisNow: WhyThisNow }
  | { kind: "defer"; until: IsoDateTimeString; reason: DeferReason }
  | { kind: "suppress"; reason: SuppressReason };

export type SurfacingTier = "critical" | "notable" | "background" | "quiet";

export type DeferReason =
  | "cooldownActive"
  | "budgetExhausted"
  | "awaitingCorroboration"
  | "pendingTrigger";

export type SuppressReason =
  | "belowThreshold"
  | "userMutedSubject"
  | "dismissedRecently"
  | "contradictedWithStrongerEvidence";

export interface TriggerRef {
  triggerKind: TriggerKind;
  at: IsoDateTimeString;
  source: string;
}

export type TriggerKind =
  | "signalArrival"
  | "entityChange"
  | "scheduledScan"
  | "feedbackEcho";

export type EngagementSignal =
  | { kind: "rendered"; surface: RenderSurface; at: IsoDateTimeString }
  | { kind: "clicked"; surface: RenderSurface; at: IsoDateTimeString }
  | { kind: "dismissed"; surface: RenderSurface; at: IsoDateTimeString }
  | {
      kind: "ignored";
      surface: RenderSurface;
      renderAt: IsoDateTimeString;
      ignoredAt: IsoDateTimeString;
    };

export interface RecommendationMetadataEnvelope {
  recommendation: RecommendationMetadataPayload;
}

export interface RecommendationMetadataPayload {
  schemaVersion: typeof RECOMMENDATION_METADATA_SCHEMA_VERSION;
  recommendedAction: RecommendedAction;
  evidence: EvidenceRef[];
  salience: SalienceScore;
  feedbackState: FeedbackState;
  conversionState: ConversionState;
}
