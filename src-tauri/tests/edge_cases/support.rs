#![allow(dead_code)]

use chrono::{TimeZone, Utc};
use dailyos_lib::abilities::feedback::ClaimVerificationState;
use dailyos_lib::abilities::provenance::SubjectRef;
use dailyos_lib::abilities::trust::{
    compile_trust, CorroboratorWeight, CrossEntityCoherenceInput, FreshnessContext,
    LinearIssueStateContext, SourceLifecycleState, SurfaceClass, TargetFootprint, TrustBand,
    TrustConfig, TrustContext, TrustFactorInputs, TrustScore, UserFeedbackSignal,
};
use dailyos_lib::db::ActionDb;
use rusqlite::Connection;
use serde_json::Value;

use crate::harness::bundle_helpers::load_bundle_fixture;

pub fn bundle_output(bundle: u32) -> Value {
    load_bundle_fixture(bundle).expected.output
}

pub fn bundle_state(bundle: u32) -> Value {
    load_bundle_fixture(bundle)
        .expected
        .state
        .expect("bundle expected_state.json is present")["post_action_state"]
        .clone()
}

pub fn json_contains_token(value: &Value, token: &str) -> bool {
    serde_json::to_string(value)
        .expect("json serializes")
        .to_ascii_lowercase()
        .contains(&token.to_ascii_lowercase())
}

pub fn claim_text(state: &Value, claim_id: &str) -> String {
    state["intelligence_claims"]
        .as_array()
        .expect("intelligence_claims array")
        .iter()
        .find(|claim| claim["claim_id"] == claim_id)
        .unwrap_or_else(|| panic!("missing claim {claim_id}"))["text"]
        .as_str()
        .expect("claim text")
        .to_string()
}

pub fn matrix_row<'a>(output: &'a Value, field: &str) -> &'a Value {
    output["normalized_field_level_diff_matrix"]
        .as_array()
        .expect("normalized_field_level_diff_matrix array")
        .iter()
        .find(|row| row["field"] == field)
        .unwrap_or_else(|| panic!("missing matrix row {field}"))
}

pub fn trust_factor_value(computation: &dailyos_lib::abilities::trust::TrustComputation, name: &str) -> f64 {
    computation
        .evidence
        .factor_breakdown
        .iter()
        .find(|factor| factor.name == name)
        .unwrap_or_else(|| panic!("missing trust factor {name}"))
        .raw_value
}

pub fn compile_test_trust(inputs: TrustFactorInputs) -> dailyos_lib::abilities::trust::TrustComputation {
    compile_trust(
        &test_claim("Account health is at risk"),
        TrustContext {
            now: Utc.with_ymd_and_hms(2026, 5, 15, 12, 0, 0).unwrap(),
            renewal_context: None,
            config: TrustConfig::default(),
            factor_inputs: inputs,
            cross_entity: CrossEntityCoherenceInput {
                claim_text: "Account health is at risk".to_string(),
                target_footprint: TargetFootprint {
                    subject: SubjectRef::Account("account-example".to_string()),
                    names: vec!["Example Account".to_string()],
                    domains: vec!["example.com".to_string()],
                    related_subjects: Vec::new(),
                    allowed_aliases: Vec::new(),
                },
                portfolio_footprints: Vec::new(),
                cross_entity_context_expected: false,
            },
            target_surface: Some(SurfaceClass::Internal),
        },
    )
    .expect("trust compiles")
}

pub fn baseline_trust_inputs() -> TrustFactorInputs {
    TrustFactorInputs {
        source_reliability: 1.0,
        source_reliability_corroborators: Vec::new(),
        freshness: FreshnessContext {
            timestamp_known: true,
            age_days: 0.0,
        },
        corroboration_strength: 1.0,
        contradiction_count: 0,
        user_feedback: UserFeedbackSignal::None,
        subject_fit_confidence: 1.0,
        internal_consistency: 1.0,
        source_lifecycle: SourceLifecycleState::Active,
        linear_issue_state: LinearIssueStateContext::default(),
        read_state_indeterminate: false,
    }
}

pub fn score_of(inputs: TrustFactorInputs) -> f64 {
    compile_test_trust(inputs).score.value()
}

pub fn band_of(inputs: TrustFactorInputs) -> TrustBand {
    compile_test_trust(inputs).band
}

pub fn corroborator(weight: f64, confirms: bool) -> CorroboratorWeight {
    CorroboratorWeight {
        evidence_weight: weight,
        confirms,
    }
}

pub fn test_claim(text: &str) -> abilities_runtime::types::IntelligenceClaim {
    abilities_runtime::types::IntelligenceClaim {
        id: "dos282-edge-claim".to_string(),
        claim_version: 1,
        subject_ref: serde_json::json!({
            "kind": "account",
            "id": "account-example"
        })
        .to_string(),
        claim_type: "risk".to_string(),
        field_path: Some("risks.summary".to_string()),
        topic_key: None,
        text: text.to_string(),
        dedup_key: "dos282-edge-dedup".to_string(),
        item_hash: Some("dos282-edge-hash".to_string()),
        actor: "agent:test".to_string(),
        data_source: "test-fixture".to_string(),
        source_ref: None,
        source_asof: Some("2026-05-15T11:00:00Z".to_string()),
        observed_at: "2026-05-15T11:00:00Z".to_string(),
        created_at: "2026-05-15T11:00:00Z".to_string(),
        provenance_json: "{}".to_string(),
        metadata_json: None,
        claim_state: abilities_runtime::types::ClaimState::Active,
        surfacing_state: abilities_runtime::types::SurfacingState::Active,
        demotion_reason: None,
        reactivated_at: None,
        retraction_reason: None,
        expires_at: None,
        superseded_by: None,
        trust_score: Some(TrustScore::MAX),
        trust_computed_at: None,
        trust_version: None,
        thread_id: None,
        temporal_scope: abilities_runtime::types::TemporalScope::State,
        sensitivity: abilities_runtime::types::ClaimSensitivity::Internal,
        verification_state: ClaimVerificationState::Active,
        verification_reason: None,
        needs_user_decision_at: None,
    }
}

pub fn owner_resolution_db() -> Connection {
    let conn = Connection::open_in_memory().expect("open in-memory db");
    conn.execute_batch(
        "
        CREATE TABLE people (
            id TEXT PRIMARY KEY,
            email TEXT,
            name TEXT NOT NULL,
            archived INTEGER NOT NULL DEFAULT 0,
            updated_at TEXT
        );
        CREATE TABLE account_stakeholders (
            account_id TEXT NOT NULL,
            person_id TEXT NOT NULL,
            role TEXT
        );
        CREATE TABLE actions (
            id TEXT,
            title TEXT,
            priority INTEGER,
            status TEXT,
            created_at TEXT,
            updated_at TEXT,
            action_kind TEXT,
            commitment_id TEXT,
            owner_raw TEXT,
            owner_entity_id TEXT,
            owner_confidence REAL,
            owner_source TEXT
        );
        ",
    )
    .expect("create owner resolution schema");
    conn
}

pub fn action_db(conn: &Connection) -> &ActionDb {
    ActionDb::from_conn(conn)
}
