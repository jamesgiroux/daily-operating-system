export type SubjectRef =
  | { account: string }
  | { project: string }
  | { person: string }
  | { meeting: string }
  | { user: string }
  | "global"
  | { multi: SubjectRef[] }
  | "unknown";

export type ReceiptTarget =
  | {
      kind: "claim";
      claimId: string;
      subject: SubjectRef;
      fieldPath?: string | null;
    }
  | {
      kind: "proposal";
      proposalId: string;
      subject: SubjectRef;
      fieldPath?: string | null;
    }
  | {
      kind: "workItem";
      actionId: string;
      backingClaimId?: string | null;
      subject?: SubjectRef | null;
    };

export type SurfaceContext =
  | "actions_work"
  | "entity_detail"
  | "daily_briefing"
  | "meeting_detail"
  | "mcp";

export type Freshness = "current" | "aging" | "stale" | "unknown";

export type RedactionLevel = "none" | "partial" | "full";

export type TrustBand =
  | "likely_current"
  | "use_with_caution"
  | "needs_verification"
  | "unscored";

export type ClaimState = "active" | "dormant" | "tombstoned" | "withdrawn";

export type SurfacingState = "active" | "dormant";

export type ClaimVerificationState = "active" | "contested" | "needs_user_decision";

export type FeedbackAction =
  | "confirm_current"
  | "mark_outdated"
  | "mark_false"
  | "wrong_subject"
  | "wrong_source"
  | "cannot_verify"
  | "needs_nuance"
  | "surface_inappropriate"
  | "not_relevant_here"
  | "merge_intent";

export type ClaimSensitivity = "public" | "internal" | "confidential" | "user_only";

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
  | {
      kind: "confidential_hidden";
      label: string;
    }
  | {
      kind: "user_only_hidden";
      label: string;
    };

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

export interface ReceiptTrust {
  band: TrustBand;
  sourceAsof?: string | null;
  freshness: Freshness;
  caveat?: string | null;
  rationale?: string | null;
}

export interface ReceiptLifecycle {
  claimState: ClaimState;
  surfacingState: SurfacingState;
  verificationState: ClaimVerificationState;
  updatedAt?: string | null;
}

export interface ProvenanceSource {
  label: string;
  sourceType?: string | null;
  asOf?: string | null;
  href?: string | null;
  redacted: boolean;
}

export interface ReceiptProvenance {
  sources: ProvenanceSource[];
  fieldPath?: string | null;
  evidenceSummary?: string | null;
  redaction: RedactionLevel;
}

export interface ReceiptAction {
  action: FeedbackAction;
  label: string;
  disabledReason?: string | null;
}

export interface ClaimReceipt {
  target: ReceiptTarget;
  surfaceContext: SurfaceContext;
  renderedText?: RenderableClaimText | null;
  trust: ReceiptTrust;
  lifecycle: ReceiptLifecycle;
  provenance: ReceiptProvenance;
  actions: ReceiptAction[];
}
