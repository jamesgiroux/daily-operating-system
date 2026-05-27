// TypeScript mirror of `EntityIntelligenceEnvelope` DTO.
//
// Parity contract for the Rust producer at
// `src-tauri/abilities-runtime/src/abilities/get_entity_intelligence/contracts.rs`.
// Renames Rust snake_case fields to camelCase per the producer's serde annotations.
// Per L0 packet §5.1 and locked decisions in §13 (server-signed cursor pagination,
// per-fact `ProvenanceRef`, typed empty reasons, 1-outer-N-inner block model).

export type EntityKind = "account" | "project" | "person" | "meeting";

export type ContextDepth = "shallow" | "standard" | "deep";

export type EnvelopeSectionV1 =
  | "facts"
  | "health"
  | "metadata_proposals"
  | "open_loops"
  | "touchpoints"
  | "threads"
  | "record";

export type EnvelopeSectionV2 = EnvelopeSectionV1 | "relationships";

export type EnvelopeSection = EnvelopeSectionV2;

export const ENVELOPE_SCHEMA_VERSION_V1 = 1;
export const ENVELOPE_SCHEMA_VERSION_V2 = 2;
export const ENVELOPE_SCHEMA_VERSION = ENVELOPE_SCHEMA_VERSION_V2;

export type EnvelopeSchemaVersion =
  | typeof ENVELOPE_SCHEMA_VERSION_V1
  | typeof ENVELOPE_SCHEMA_VERSION_V2;

export type EnvelopeSectionForSchema<Version extends EnvelopeSchemaVersion> =
  Version extends typeof ENVELOPE_SCHEMA_VERSION_V1
    ? EnvelopeSectionV1
    : EnvelopeSectionV2;

interface EntityIntelligenceInputBase {
  entityType: EntityKind;
  entityId: string;
  depth: ContextDepth;
}

export type EntityIntelligenceInputV1 = EntityIntelligenceInputBase & {
  schemaVersion: typeof ENVELOPE_SCHEMA_VERSION_V1;
  sections?: EnvelopeSectionV1[];
};

export type EntityIntelligenceInputV2 = EntityIntelligenceInputBase & {
  schemaVersion: typeof ENVELOPE_SCHEMA_VERSION_V2;
  sections?: EnvelopeSectionV2[];
};

export type EntityIntelligenceInput =
  | EntityIntelligenceInputV1
  | EntityIntelligenceInputV2;

// ---- subject ---------------------------------------------------------------

export type SubjectRef =
  | { account: string }
  | { project: string }
  | { person: string }
  | { meeting: string }
  | { user: string }
  | "global"
  | { multi: SubjectRef[] }
  | "unknown";

export interface NormalizedSubject {
  kind: EntityKind;
  id: string;
  subjectRef: SubjectRef;
  displayLabel: string;
}

// ---- cursor + pagination ---------------------------------------------------

export type Cursor = string;

export type CursorState =
  | { kind: "stable" }
  | { kind: "data_shifted"; advisory: string }
  | { kind: "invalidated"; reason: string; restart_required: boolean };

export interface Paginated<T> {
  items: T[];
  nextCursor: Cursor | null;
  totalHint: number | null;
  cursorState: CursorState;
}

// ---- empty + section state ------------------------------------------------

export type EmptyReasonV1 =
  | "not_connected"
  | "not_processed_yet"
  | "filtered_out_by_subject"
  | "no_relevant_touchpoints"
  | "stale"
  | "no_evidence_backed_proposal"
  | "unsupported_for_subject"
  | "not_requested"
  | { partial_failure: { advisory: string } };

export type EmptyReasonV2 = EmptyReasonV1 | "no_relevant_relationships";

export type EmptyReason = EmptyReasonV2;

export type EmptyReasonForSchema<Version extends EnvelopeSchemaVersion> =
  Version extends typeof ENVELOPE_SCHEMA_VERSION_V1 ? EmptyReasonV1 : EmptyReasonV2;

export type SectionStateV1 =
  | { kind: "present"; item_count: number }
  | { kind: "empty"; reason: EmptyReasonV1 };

export type SectionStateV2 =
  | { kind: "present"; item_count: number }
  | { kind: "empty"; reason: EmptyReasonV2 };

export type SectionState = SectionStateV2;

export type SectionStateForSchema<Version extends EnvelopeSchemaVersion> =
  Version extends typeof ENVELOPE_SCHEMA_VERSION_V1 ? SectionStateV1 : SectionStateV2;

// ---- per-fact provenance reference (ADR-0130 §2 amendment) ----------------

export interface ProvenanceRef {
  sourceIds: string[];
}

// ---- envelope-level provenance + trust + sensitivity ----------------------

export interface EnvelopeProvenanceSource {
  id: string;
  label: string;
  sourceType: string | null;
  asOf: string | null;
  redacted: boolean;
}

export interface EnvelopeProvenance {
  sources: EnvelopeProvenanceSource[];
  redactionApplied: boolean;
}

export type TrustBand =
  | "likely_current"
  | "use_with_caution"
  | "needs_verification"
  | "unscored";

export interface EnvelopeTrustSummary<Section extends EnvelopeSection = EnvelopeSection> {
  aggregateBand: TrustBand;
  sectionCaveats: Partial<Record<Section, string>>;
}

export type ClaimSensitivity =
  | "public"
  | "internal"
  | "confidential"
  | "user_only";

// ---- claim-backed primitives ----------------------------------------------

export type Freshness = "current" | "aging" | "stale" | "unknown";

export type ClaimState = "active" | "dormant" | "tombstoned" | "withdrawn";

export type SurfacingState = "active" | "dormant";

export type ClaimVerificationState =
  | "active"
  | "contested"
  | "needs_user_decision";

export type RenderSurface =
  | "tauri_entity_detail"
  | "tauri_briefing_prep"
  | "tauri_meeting_detail"
  | "tauri_email_summary"
  | "action"
  | "tauri_provenance"
  | "tauri_report"
  | "tauri_chat"
  | "mcp_tool"
  | "mcp_tool_detail"
  | "p2_publication"
  | "log_structured"
  | "push_notification";

export type RenderPolicyKind = "render" | "redacted" | "drop";

export type RedactionAffordance =
  | {
      kind: "confidential_click_to_reveal";
      claim_id: string;
      label: string;
      audit_required: boolean;
    }
  | { kind: "confidential_hidden"; label: string }
  | { kind: "user_only_hidden"; label: string };

export interface RenderPolicy {
  kind: RenderPolicyKind;
  sensitivity: ClaimSensitivity;
  surface: RenderSurface;
  claimId?: string | null;
  affordance?: RedactionAffordance | null;
}

export interface RenderableClaimText {
  text: string;
  policy: RenderPolicy;
}

export interface EntityFact {
  claimId: string;
  subjectRef: SubjectRef;
  fieldPath: string | null;
  claimType: string;
  renderedText: RenderableClaimText;
  trustBand: TrustBand;
  freshness: Freshness;
  sourceAsof: string | null;
  sensitivity: ClaimSensitivity;
  lifecycleState: ClaimState;
  surfacingState: SurfacingState;
  verificationState: ClaimVerificationState;
  provenance: ProvenanceRef;
}

export interface HealthStoryRow {
  label: string;
  body: string;
  evidenceClaimIds: string[];
  provenance: ProvenanceRef;
}

export interface HealthStory {
  headline: string | null;
  rows: HealthStoryRow[];
}

export interface MetadataProposal {
  proposalId: string;
  subjectRef: SubjectRef;
  fieldPath: string;
  currentValue: string | null;
  proposedValue: string;
  trustBand: TrustBand;
  sensitivity: ClaimSensitivity;
  provenance: ProvenanceRef;
}

// ---- open loop with receipt ----------------------------------------------

export interface ReceiptTargetRef {
  claimId: string;
  subjectRef: SubjectRef;
  fieldPath: string | null;
}

export interface OpenLoopSubject {
  entity_type: string;
  entity_id: string;
}

export interface OpenLoop {
  id: string;
  subject: OpenLoopSubject;
  loop_kind: string;
  description: string;
  owner?: string | null;
  due_date?: string | null;
  status?: string | null;
  source_asof?: string | null;
  claim_type: string;
}

export interface OpenLoopWithReceipt {
  openLoop: OpenLoop;
  receiptTarget: ReceiptTargetRef;
  trustBand: TrustBand;
  freshness: Freshness;
  provenance: ProvenanceRef;
}

// ---- touchpoint -----------------------------------------------------------

export type TouchpointKind =
  | "meeting"
  | "email_thread"
  | "document"
  | "salesforce"
  | "linear";

export type InclusionReason =
  | "subject_match"
  | "entity_link"
  | "attendee_match"
  | "domain_match";

export type ExclusionReason =
  | "subject_mismatch"
  | "outside_window"
  | "low_confidence"
  | "suppressed";

export interface Touchpoint {
  meetingId: string | null;
  kind: TouchpointKind;
  when: string;
  subjectRef: SubjectRef;
  inclusionReason: InclusionReason;
  exclusionReason: ExclusionReason | null;
  trustBand: TrustBand;
  freshness: Freshness;
  provenance: ProvenanceRef;
}

export interface CandidateSetRef {
  windowStart: string | null;
  windowEnd: string | null;
  filterDescription: string;
}

export interface SubjectScope {
  primary: SubjectRef;
  alsoIncludes: SubjectRef[];
}

export interface TouchpointBundle<Reason extends EmptyReason = EmptyReason> {
  upcoming: Paginated<Touchpoint>;
  recent: Paginated<Touchpoint>;
  candidateSet: CandidateSetRef;
  emptyReason: Reason | null;
  subjectScope: SubjectScope;
}

// ---- relationships + participation ---------------------------------------

export type RelationshipInclusionReason =
  | "subject_match"
  | "hierarchy"
  | "explicit_link"
  | "attendee_match"
  | "co_attendance"
  | "work_item"
  | "content_link";

export interface RelationshipEdge {
  edgeId: string;
  edgeType: string;
  subjectRef: SubjectRef;
  relatedSubjectRef: SubjectRef;
  relatedDisplayLabel: RenderableClaimText | null;
  observedAt: string | null;
  sourceAsof: string | null;
  confidence: number;
  sensitivity: ClaimSensitivity;
  inclusionReason: RelationshipInclusionReason;
  traversalDepth: number;
  trustBand: TrustBand;
  freshness: Freshness;
  provenance: ProvenanceRef;
  caveats: string[];
}

export interface RelationshipParticipant {
  subjectRef: SubjectRef;
  displayLabel: RenderableClaimText | null;
  role: RenderableClaimText | null;
  relationship: RenderableClaimText | null;
  sensitivity: ClaimSensitivity;
  normalizedTouchpointCount: number;
  recentTouchpointIds: string[];
  lastSeenAt: string | null;
  trustBand: TrustBand;
  freshness: Freshness;
  provenance: ProvenanceRef;
  caveats: string[];
}

export interface RelationshipTruncation {
  edgesTruncated: boolean;
  participantsTruncated: boolean;
  perEdgeCap: number;
}

export interface RelationshipsBundle {
  edges: Paginated<RelationshipEdge>;
  participants: Paginated<RelationshipParticipant>;
  candidateSet: CandidateSetRef;
  emptyReason: EmptyReasonV2 | null;
  subjectScope: SubjectScope;
  truncation: RelationshipTruncation;
  caveats: string[];
}

// ---- threads + record ----------------------------------------------------

export interface ThreadSummary {
  threadId: string;
  title: string | null;
  lastActivityAt: string | null;
  messageCount: number;
  provenance: ProvenanceRef;
}

export interface RecordEntry {
  claimId: string;
  subjectRef: SubjectRef;
  claimType: string;
  recordedAt: string;
  renderedText: RenderableClaimText;
  trustBand: TrustBand;
  sensitivity: ClaimSensitivity;
  provenance: ProvenanceRef;
}

// ---- envelope --------------------------------------------------------------

interface EntityIntelligenceEnvelopeBase<Version extends EnvelopeSchemaVersion> {
  subject: NormalizedSubject;
  sections: Partial<
    Record<EnvelopeSectionForSchema<Version>, SectionStateForSchema<Version>>
  >;
  facts: Paginated<EntityFact>;
  healthStory: HealthStory | null;
  metadataProposals: Paginated<MetadataProposal>;
  openLoops: Paginated<OpenLoopWithReceipt>;
  touchpoints: Paginated<TouchpointBundle<EmptyReasonForSchema<Version>>>;
  threads: Paginated<ThreadSummary>;
  recordEntries: Paginated<RecordEntry>;
  trust: EnvelopeTrustSummary<EnvelopeSectionForSchema<Version>>;
  provenance: EnvelopeProvenance;
  sensitivity: ClaimSensitivity;
}

export interface EntityIntelligenceEnvelopeV1
  extends EntityIntelligenceEnvelopeBase<typeof ENVELOPE_SCHEMA_VERSION_V1> {
  schemaVersion: typeof ENVELOPE_SCHEMA_VERSION_V1;
  relationships?: never;
}

export interface EntityIntelligenceEnvelopeV2
  extends EntityIntelligenceEnvelopeBase<typeof ENVELOPE_SCHEMA_VERSION_V2> {
  schemaVersion: typeof ENVELOPE_SCHEMA_VERSION_V2;
  relationships?: Paginated<RelationshipsBundle>;
}

export type EntityIntelligenceEnvelope =
  | EntityIntelligenceEnvelopeV1
  | EntityIntelligenceEnvelopeV2;
