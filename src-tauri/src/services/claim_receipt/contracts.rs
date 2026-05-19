//! Route-neutral DTO for surface-agnostic claim receipt rendering.
//!
//! Receipt is a render-side projection of existing claim/proposal state — it carries
//! no new substrate writes. Surfaces consume it through `render::render_receipt_for`.

use abilities_runtime::abilities::feedback::FeedbackAction;
use abilities_runtime::abilities::provenance::subject::SubjectRef;
use abilities_runtime::abilities::trust::types::TrustBand;
use abilities_runtime::sensitivity::{ClaimVerificationState, RenderableClaimText};
use abilities_runtime::types::{ClaimState, SurfacingState};
use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ReceiptTarget {
    #[serde(rename_all = "camelCase")]
    Claim {
        claim_id: String,
        subject: SubjectRef,
        field_path: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    Proposal {
        proposal_id: String,
        subject: SubjectRef,
        field_path: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    WorkItem {
        action_id: String,
        backing_claim_id: Option<String>,
        subject: Option<SubjectRef>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceContext {
    ActionsWork,
    EntityDetail,
    DailyBriefing,
    MeetingDetail,
    Mcp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Freshness {
    Current,
    Aging,
    Stale,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RedactionLevel {
    None,
    Partial,
    Full,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReceiptTrust {
    pub band: TrustBand,
    pub source_asof: Option<DateTime<Utc>>,
    pub freshness: Freshness,
    pub caveat: Option<String>,
    pub rationale: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReceiptLifecycle {
    pub claim_state: ClaimState,
    pub surfacing_state: SurfacingState,
    pub verification_state: ClaimVerificationState,
    pub updated_at: Option<DateTime<Utc>>,
}

/// Generic label; never the raw source id. Raw source identity is gated by render policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProvenanceSource {
    pub label: String,
    pub source_type: Option<String>,
    pub as_of: Option<DateTime<Utc>>,
    pub href: Option<String>,
    pub redacted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReceiptProvenance {
    pub sources: Vec<ProvenanceSource>,
    pub field_path: Option<String>,
    pub evidence_summary: Option<String>,
    pub redaction: RedactionLevel,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReceiptAction {
    pub action: FeedbackAction,
    pub label: String,
    pub disabled_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ClaimReceipt {
    pub target: ReceiptTarget,
    pub surface_context: SurfaceContext,
    pub rendered_text: Option<RenderableClaimText>,
    pub trust: ReceiptTrust,
    pub lifecycle: ReceiptLifecycle,
    pub provenance: ReceiptProvenance,
    pub actions: Vec<ReceiptAction>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn receipt_target_serde_tagged_camel_case() {
        let target = ReceiptTarget::Claim {
            claim_id: "c1".into(),
            subject: SubjectRef::Global,
            field_path: Some("status".into()),
        };
        let json = serde_json::to_value(&target).unwrap();
        assert_eq!(json["kind"], "claim");
        assert!(json.get("claimId").is_some());
        assert!(json.get("fieldPath").is_some());
        let round: ReceiptTarget = serde_json::from_value(json).unwrap();
        assert_eq!(round, target);
    }

    #[test]
    fn surface_context_serde_snake_case() {
        let json = serde_json::to_string(&SurfaceContext::ActionsWork).unwrap();
        assert_eq!(json, "\"actions_work\"");
        let json = serde_json::to_string(&SurfaceContext::DailyBriefing).unwrap();
        assert_eq!(json, "\"daily_briefing\"");
    }

    #[test]
    fn provenance_redacted_flag_present() {
        let src = ProvenanceSource {
            label: "internal note".into(),
            source_type: None,
            as_of: None,
            href: None,
            redacted: true,
        };
        let json = serde_json::to_value(&src).unwrap();
        assert_eq!(json["redacted"], true);
    }

    #[test]
    fn receipt_action_carries_feedback_action() {
        let act = ReceiptAction {
            action: FeedbackAction::ConfirmCurrent,
            label: "Looks right".into(),
            disabled_reason: None,
        };
        let json = serde_json::to_value(&act).unwrap();
        assert_eq!(json["action"], "confirm_current");
        assert_eq!(json["label"], "Looks right");
    }
}
