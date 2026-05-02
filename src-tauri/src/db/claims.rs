//! DOS-7 D1: row types for the claims commit substrate.
//!
//! D1 ships types only. The commit_claim algorithm (D2), 9-mechanism
//! backfill (D3), hard-delete role refactor (D4), and reconcile pass
//! (D5) consume these types but live in services/claims.rs.

use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClaimState {
    Active,
    Dormant,
    Tombstoned,
    Withdrawn,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SurfacingState {
    Active,
    Dormant,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TemporalScope {
    State,
    PointInTime,
    Trend,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClaimSensitivity {
    Public,
    Internal,
    Confidential,
    UserOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BranchKind {
    Contradiction,
    Clarification,
    Supersession,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReconciliationKind {
    UserPickedWinner,
    EvidenceConverged,
    MergedAsQualified,
    BothDormant,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackType {
    Confirm,
    Correct,
    Reject,
    WrongSubject,
    CannotVerify,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RepairJobState {
    Pending,
    InProgress,
    Completed,
    Failed,
    BudgetExhausted,
}

/// Mirror of the `intelligence_claims` row.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct IntelligenceClaim {
    pub id: String,
    pub subject_ref: String,
    pub claim_type: String,
    pub field_path: Option<String>,
    pub topic_key: Option<String>,
    pub text: String,
    pub dedup_key: String,
    pub item_hash: Option<String>,
    pub actor: String,
    pub data_source: String,
    pub source_ref: Option<String>,
    pub source_asof: Option<String>,
    pub observed_at: String,
    pub created_at: String,
    pub provenance_json: String,
    pub metadata_json: Option<String>,
    pub claim_state: ClaimState,
    pub surfacing_state: SurfacingState,
    pub demotion_reason: Option<String>,
    pub reactivated_at: Option<String>,
    pub retraction_reason: Option<String>,
    pub expires_at: Option<String>,
    pub superseded_by: Option<String>,
    pub trust_score: Option<f64>,
    pub trust_computed_at: Option<String>,
    pub trust_version: Option<i64>,
    pub thread_id: Option<String>,
    pub temporal_scope: TemporalScope,
    pub sensitivity: ClaimSensitivity,
}

/// Mirror of the `claim_corroborations` row.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ClaimCorroboration {
    pub id: String,
    pub claim_id: String,
    pub data_source: String,
    pub source_asof: Option<String>,
    pub source_mechanism: Option<String>,
    pub strength: f64,
    pub reinforcement_count: i64,
    pub last_reinforced_at: String,
    pub created_at: String,
}

/// Mirror of the `claim_contradictions` row.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ClaimContradiction {
    pub id: String,
    pub primary_claim_id: String,
    pub contradicting_claim_id: String,
    pub branch_kind: BranchKind,
    pub detected_at: String,
    pub reconciliation_kind: Option<ReconciliationKind>,
    pub reconciliation_note: Option<String>,
    pub reconciled_at: Option<String>,
    pub winner_claim_id: Option<String>,
    pub merged_claim_id: Option<String>,
}

/// Mirror of the `agent_trust_ledger` row.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct AgentTrustLedger {
    pub id: i64,
    pub agent_kind: String,
    pub agent_id: String,
    pub claim_type: Option<String>,
    pub correct_count: i64,
    pub incorrect_count: i64,
    pub total_count: i64,
    pub last_updated_at: String,
}

/// Mirror of the `claim_feedback` row.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct ClaimFeedbackRow {
    pub id: String,
    pub claim_id: String,
    pub feedback_type: FeedbackType,
    pub actor: String,
    pub actor_id: Option<String>,
    pub payload_json: Option<String>,
    pub submitted_at: String,
}

/// Mirror of the `claim_repair_job` row.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct ClaimRepairJob {
    pub id: String,
    pub claim_id: String,
    pub feedback_id: Option<String>,
    pub state: RepairJobState,
    pub attempts: i64,
    pub max_attempts: i64,
    pub last_attempt_at: Option<String>,
    pub completed_at: Option<String>,
    pub error_message: Option<String>,
    pub created_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip_enum<T>(value: T, expected_json: &str)
    where
        T: Serialize + for<'de> Deserialize<'de> + PartialEq + std::fmt::Debug,
    {
        let json = serde_json::to_string(&value).unwrap();
        assert_eq!(json, expected_json);
        let back: T = serde_json::from_str(&json).unwrap();
        assert_eq!(value, back);
    }

    #[test]
    fn claim_state_serde_roundtrip() {
        roundtrip_enum(ClaimState::Active, "\"active\"");
        roundtrip_enum(ClaimState::Dormant, "\"dormant\"");
        roundtrip_enum(ClaimState::Tombstoned, "\"tombstoned\"");
        roundtrip_enum(ClaimState::Withdrawn, "\"withdrawn\"");
    }

    #[test]
    fn surfacing_state_serde_roundtrip() {
        roundtrip_enum(SurfacingState::Active, "\"active\"");
        roundtrip_enum(SurfacingState::Dormant, "\"dormant\"");
    }

    #[test]
    fn temporal_scope_serde_roundtrip() {
        roundtrip_enum(TemporalScope::State, "\"state\"");
        roundtrip_enum(TemporalScope::PointInTime, "\"point_in_time\"");
        roundtrip_enum(TemporalScope::Trend, "\"trend\"");
    }

    #[test]
    fn claim_sensitivity_serde_roundtrip() {
        roundtrip_enum(ClaimSensitivity::Public, "\"public\"");
        roundtrip_enum(ClaimSensitivity::Internal, "\"internal\"");
        roundtrip_enum(ClaimSensitivity::Confidential, "\"confidential\"");
        roundtrip_enum(ClaimSensitivity::UserOnly, "\"user_only\"");
    }

    #[test]
    fn branch_kind_serde_roundtrip() {
        roundtrip_enum(BranchKind::Contradiction, "\"contradiction\"");
        roundtrip_enum(BranchKind::Clarification, "\"clarification\"");
        roundtrip_enum(BranchKind::Supersession, "\"supersession\"");
    }

    #[test]
    fn reconciliation_kind_serde_roundtrip() {
        roundtrip_enum(
            ReconciliationKind::UserPickedWinner,
            "\"user_picked_winner\"",
        );
        roundtrip_enum(
            ReconciliationKind::EvidenceConverged,
            "\"evidence_converged\"",
        );
        roundtrip_enum(
            ReconciliationKind::MergedAsQualified,
            "\"merged_as_qualified\"",
        );
        roundtrip_enum(ReconciliationKind::BothDormant, "\"both_dormant\"");
    }

    #[test]
    fn feedback_type_serde_roundtrip() {
        roundtrip_enum(FeedbackType::Confirm, "\"confirm\"");
        roundtrip_enum(FeedbackType::Correct, "\"correct\"");
        roundtrip_enum(FeedbackType::Reject, "\"reject\"");
        roundtrip_enum(FeedbackType::WrongSubject, "\"wrong_subject\"");
        roundtrip_enum(FeedbackType::CannotVerify, "\"cannot_verify\"");
    }

    #[test]
    fn repair_job_state_serde_roundtrip() {
        roundtrip_enum(RepairJobState::Pending, "\"pending\"");
        roundtrip_enum(RepairJobState::InProgress, "\"in_progress\"");
        roundtrip_enum(RepairJobState::Completed, "\"completed\"");
        roundtrip_enum(RepairJobState::Failed, "\"failed\"");
        roundtrip_enum(RepairJobState::BudgetExhausted, "\"budget_exhausted\"");
    }

    #[test]
    fn intelligence_claim_roundtrip_preserves_snake_case_and_options() {
        let claim = IntelligenceClaim {
            id: "claim-1".to_string(),
            subject_ref: r#"{"kind":"account","id":"acct-1"}"#.to_string(),
            claim_type: "risk".to_string(),
            field_path: Some("health.risk".to_string()),
            topic_key: None,
            text: "Renewal risk is elevated".to_string(),
            dedup_key: "dedup-1".to_string(),
            item_hash: Some("hash-1".to_string()),
            actor: "agent:prepare_meeting:1.0".to_string(),
            data_source: "glean".to_string(),
            source_ref: None,
            source_asof: Some("2026-05-02T00:00:00Z".to_string()),
            observed_at: "2026-05-02T01:00:00Z".to_string(),
            created_at: "2026-05-02T01:00:01Z".to_string(),
            provenance_json: r#"{"schema":1}"#.to_string(),
            metadata_json: None,
            claim_state: ClaimState::Active,
            surfacing_state: SurfacingState::Active,
            demotion_reason: None,
            reactivated_at: None,
            retraction_reason: None,
            expires_at: None,
            superseded_by: None,
            trust_score: Some(0.82),
            trust_computed_at: Some("2026-05-02T01:00:02Z".to_string()),
            trust_version: Some(1),
            thread_id: Some("thread-1".to_string()),
            temporal_scope: TemporalScope::State,
            sensitivity: ClaimSensitivity::Internal,
        };

        let json = serde_json::to_string(&claim).unwrap();
        assert!(json.contains("\"field_path\":\"health.risk\""));
        assert!(json.contains("\"topic_key\":null"));
        assert!(json.contains("\"claim_state\":\"active\""));
        assert!(json.contains("\"temporal_scope\":\"state\""));
        assert!(json.contains("\"sensitivity\":\"internal\""));

        let back: IntelligenceClaim = serde_json::from_str(&json).unwrap();
        assert_eq!(back, claim);
    }

    #[test]
    fn intelligence_claim_roundtrip_preserves_some_optional_fields() {
        let claim = IntelligenceClaim {
            id: "claim-2".to_string(),
            subject_ref: r#"{"kind":"person","id":"person-1"}"#.to_string(),
            claim_type: "role".to_string(),
            field_path: None,
            topic_key: Some("stakeholder_role".to_string()),
            text: "Economic buyer".to_string(),
            dedup_key: "dedup-2".to_string(),
            item_hash: None,
            actor: "user".to_string(),
            data_source: "user_input".to_string(),
            source_ref: Some(r#"{"source":"manual"}"#.to_string()),
            source_asof: None,
            observed_at: "2026-05-02T02:00:00Z".to_string(),
            created_at: "2026-05-02T02:00:01Z".to_string(),
            provenance_json: "{}".to_string(),
            metadata_json: Some(r#"{"source_mechanism":"manual"}"#.to_string()),
            claim_state: ClaimState::Tombstoned,
            surfacing_state: SurfacingState::Dormant,
            demotion_reason: Some("consolidation_prune".to_string()),
            reactivated_at: None,
            retraction_reason: Some("user_removal".to_string()),
            expires_at: Some("2026-06-02T00:00:00Z".to_string()),
            superseded_by: Some("claim-3".to_string()),
            trust_score: None,
            trust_computed_at: None,
            trust_version: None,
            thread_id: None,
            temporal_scope: TemporalScope::PointInTime,
            sensitivity: ClaimSensitivity::Confidential,
        };

        let json = serde_json::to_string(&claim).unwrap();
        assert!(json.contains("\"topic_key\":\"stakeholder_role\""));
        assert!(json.contains("\"field_path\":null"));
        assert!(json.contains("\"claim_state\":\"tombstoned\""));
        assert!(json.contains("\"surfacing_state\":\"dormant\""));
        assert!(json.contains("\"temporal_scope\":\"point_in_time\""));
        assert!(json.contains("\"sensitivity\":\"confidential\""));

        let back: IntelligenceClaim = serde_json::from_str(&json).unwrap();
        assert_eq!(back, claim);
    }
}
