//! Readable claim-file projection and correction apply service.
//!
//! Files under `_dailyos_claims` are projections of canonical claims. They are
//! never source evidence, and applying edits routes through the existing
//! receipt-level feedback path.

use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs;
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};

use abilities_runtime::abilities::feedback::FeedbackAction;
use abilities_runtime::abilities::provenance::subject::SubjectRef as ReceiptSubjectRef;
use abilities_runtime::sensitivity::RenderActor;
use abilities_runtime::types::{ClaimSensitivity, ClaimSubjectRef, IntelligenceClaim};
use chrono::{DateTime, Duration, Utc};
use rusqlite::{params, OptionalExtension};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::db::claim_invalidation::SubjectRef as InvalidationSubjectRef;
use crate::db::ActionDb;
use crate::services::claim_receipt::contracts::{ReceiptTarget, SurfaceContext};
use crate::services::claim_receipt::feedback::{
    submit_claim_feedback_for_claim_file_apply, ClaimFeedbackRequest, ClaimFeedbackResponse,
    ClaimFileFeedbackApplyCommit, IdempotencyCache,
};
use crate::services::claims::repair_claim_feedback_recorded_signal;
use crate::services::entity_intelligence::auth::{EnvelopeSet, EnvelopeView};
use crate::services::workspace_ingestion::registry::CLAIM_FILE_PROJECTION_ROOT;
use crate::state::AppState;

pub const CLAIM_FILE_PROJECTION_VERSION: u32 = 1;
pub const CLAIM_FILE_LEGACY_SIDECAR_SCHEMA_VERSION: u32 = 1;
pub const CLAIM_FILE_SIDECAR_SCHEMA_VERSION: u32 = 2;
pub const CLAIM_FILE_MARKDOWN_NAME: &str = "claims.md";
pub const CLAIM_FILE_SIDECAR_NAME: &str = "claims.corrections.json";
const CLAIM_FILE_CORRECTION_APPLY_LEASE_SECS: i64 = 300;

#[derive(Debug, thiserror::Error)]
pub enum ClaimFileError {
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("path rejected: {0}")]
    PathRejected(String),
    #[error("projection failed: {0}")]
    ProjectionFailed(String),
    #[error("apply failed: {0}")]
    ApplyFailed(String),
    #[error("db: {0}")]
    Db(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ClaimFileProjectionResult {
    pub run_id: String,
    pub markdown_rel_path: String,
    pub sidecar_rel_path: String,
    pub claim_count: usize,
    pub markdown_checksum: String,
    pub sidecar_checksum: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct ClaimFileProjectionEvalSnapshot {
    pub markdown_checksum: String,
    pub sidecar_checksum: String,
    pub claim_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ClaimFileProjectionChangeStatus {
    pub entity_subject_compact: String,
    pub current_claim_watermark: String,
    pub markdown_rel_path: String,
    pub sidecar_rel_path: String,
    pub latest_run_id: Option<String>,
    pub latest_run_status: Option<String>,
    pub latest_claim_watermark: Option<String>,
    pub latest_attempted_at: Option<String>,
    pub needs_repair: bool,
    pub repair_reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ClaimFileApplyResult {
    pub applied_count: usize,
    pub skipped_count: usize,
    pub failures: Vec<ClaimFileApplyFailure>,
    pub responses: Vec<ClaimFeedbackResponse>,
    pub rerender: Option<ClaimFileProjectionResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ClaimFileApplyFailure {
    pub claim_id: Option<String>,
    pub error_class: String,
    pub error_detail_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ClaimFileSidecar {
    pub schema_version: u32,
    pub projection_version: u32,
    pub entity_subject_ref: serde_json::Value,
    pub entity_subject_compact: String,
    pub markdown_rel_path: String,
    pub sidecar_rel_path: String,
    pub claims: Vec<ClaimFileClaim>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ClaimFileClaim {
    pub semantic_identity: ClaimSemanticIdentityV1,
    pub runtime_claim_id: String,
    pub runtime_claim_version: u64,
    pub claim_text: String,
    pub trust_band: String,
    pub sensitivity: String,
    pub lifecycle: ClaimFileLifecycle,
    pub provenance_summary: ClaimFileProvenanceSummary,
    pub feedback_rows: Vec<ClaimFileFeedbackRow>,
    pub contradiction_edges: Vec<ClaimFileContradictionEdge>,
    pub superseded_by_semantic_identity: Option<ClaimSemanticIdentityV1>,
    pub replay_status: ClaimFileReplayStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ClaimSemanticIdentityV1 {
    pub identity_version: u32,
    pub identity_kind: String,
    pub item_hash: String,
    pub subject_ref_compact: String,
    pub subject_ref: serde_json::Value,
    pub claim_type: String,
    pub field_path: Option<String>,
    pub dedup_key_components_hash: String,
    pub source_ref: Option<String>,
    pub data_source: String,
    pub actor: String,
    pub observed_at: String,
    pub source_asof: Option<String>,
    pub source_content_hash: Option<String>,
    pub runtime_claim_id: String,
    pub runtime_claim_version: u64,
    pub claim_state: String,
    pub superseded_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ClaimFileProvenanceSummary {
    pub data_source: String,
    pub source_ref: Option<String>,
    pub source_asof: Option<String>,
    pub observed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ClaimFileFeedbackRow {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub feedback_id: String,
    pub action: String,
    pub actor: String,
    pub actor_id: Option<String>,
    pub payload_json: Option<serde_json::Value>,
    pub submitted_at: String,
    pub applied_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ClaimFileLifecycle {
    pub claim_state: String,
    pub surfacing_state: String,
    pub demotion_reason: Option<String>,
    pub retraction_reason: Option<String>,
    pub superseded_by: Option<String>,
    pub verification_state: String,
    pub verification_reason: Option<String>,
    pub needs_user_decision_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ClaimFileContradictionEdge {
    pub edge_id: String,
    pub branch_kind: String,
    pub role: String,
    pub primary_runtime_claim_id: String,
    pub contradicting_runtime_claim_id: String,
    pub primary_semantic_identity: Option<ClaimSemanticIdentityV1>,
    pub contradicting_semantic_identity: Option<ClaimSemanticIdentityV1>,
    pub detected_at: String,
    pub reconciliation_kind: Option<String>,
    pub reconciliation_note: Option<String>,
    pub reconciled_at: Option<String>,
    pub winner_runtime_claim_id: Option<String>,
    pub winner_semantic_identity: Option<ClaimSemanticIdentityV1>,
    pub merged_runtime_claim_id: Option<String>,
    pub merged_semantic_identity: Option<ClaimSemanticIdentityV1>,
    pub replay_status: ClaimFileReplayStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ClaimFileReplayStatus {
    pub status: String,
    pub reason: Option<String>,
    pub last_attempted_at: Option<String>,
}

#[derive(Debug, Clone)]
struct RenderBundle {
    run_id: String,
    subject_ref_json: String,
    entity_subject_compact: String,
    markdown_rel_path: PathBuf,
    sidecar_rel_path: PathBuf,
    entity_claim_invalidation_version: i64,
    claim_watermark: String,
    markdown: String,
    sidecar: ClaimFileSidecar,
    sidecar_json: String,
    markdown_checksum: String,
    sidecar_checksum: String,
}

#[derive(Debug, Clone)]
struct ParsedCorrection {
    claim_id: String,
    projected_claim_version: u64,
    projected_identity_hash: String,
    action: FeedbackAction,
    metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CorrectionApplyState {
    Claimed,
    AlreadyApplied { feedback_id: Option<String> },
    InProgress,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CorrectionFailureMarkState {
    MarkedFailed,
    AlreadyApplied,
}

struct CorrectionApplyRecord<'a> {
    apply_key: &'a str,
    sidecar_checksum: &'a str,
    claim_id: &'a str,
    projected_claim_version: u64,
    projected_identity_hash: &'a str,
    feedback_action: &'a str,
    payload_hash: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CommittedProjectionRun {
    pub run_id: String,
}

#[derive(Debug, Clone)]
struct FileEnvelope {
    claim_ids: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProjectionRunSummary {
    run_id: String,
    status: String,
    claim_watermark: String,
    markdown_rel_path: String,
    sidecar_rel_path: String,
    attempted_at: String,
}

impl EnvelopeView for FileEnvelope {
    fn ability(&self) -> &str {
        "claim_file_projection"
    }

    fn claim_ids(&self) -> BTreeSet<String> {
        self.claim_ids.clone()
    }

    fn proposal_ids(&self) -> BTreeSet<String> {
        BTreeSet::new()
    }
}

pub async fn render_entity_claim_file(
    state: &AppState,
    workspace_root: PathBuf,
    subject_ref_json: String,
) -> Result<ClaimFileProjectionResult, ClaimFileError> {
    render_entity_claim_file_for_status(state, workspace_root, subject_ref_json, None).await
}

pub async fn detect_entity_claim_file_changes(
    state: &AppState,
    subject_ref_json: String,
) -> Result<ClaimFileProjectionChangeStatus, ClaimFileError> {
    let status = state
        .db_read(move |db| {
            let bundle = build_render_bundle_db(db, &subject_ref_json)?;
            projection_change_status_for_bundle(db, &bundle)
        })
        .await
        .map_err(|error| ClaimFileError::Db(error.to_string()))?;
    Ok(status)
}

pub async fn repair_claim_file_projection(
    state: &AppState,
    workspace_root: PathBuf,
    subject_ref_json: String,
) -> Result<ClaimFileProjectionResult, ClaimFileError> {
    let status = detect_entity_claim_file_changes(state, subject_ref_json.clone()).await?;
    let repaired_from_run_id = status
        .latest_run_id
        .filter(|_| status.latest_run_status.as_deref() == Some("failed"));
    render_entity_claim_file_for_status(
        state,
        workspace_root,
        subject_ref_json,
        repaired_from_run_id,
    )
    .await
}

async fn render_entity_claim_file_for_status(
    state: &AppState,
    workspace_root: PathBuf,
    subject_ref_json: String,
    repaired_from_run_id: Option<String>,
) -> Result<ClaimFileProjectionResult, ClaimFileError> {
    let bundle = build_render_bundle(state, subject_ref_json).await?;
    ensure_projection_path_binding_state(state, &bundle).await?;
    let write_result = write_projection_files(
        &workspace_root,
        &bundle.markdown_rel_path,
        &bundle.sidecar_rel_path,
        &bundle.markdown,
        &bundle.sidecar_json,
    );
    let status = if write_result.is_err() {
        "failed"
    } else if repaired_from_run_id.is_some() {
        "repaired"
    } else {
        "committed"
    };
    record_projection_run(
        state,
        &bundle,
        status,
        repaired_from_run_id,
        write_result.as_ref().err(),
    )
    .await?;
    write_result?;

    Ok(ClaimFileProjectionResult {
        run_id: bundle.run_id,
        markdown_rel_path: path_to_slash_string(&bundle.markdown_rel_path),
        sidecar_rel_path: path_to_slash_string(&bundle.sidecar_rel_path),
        claim_count: bundle.sidecar.claims.len(),
        markdown_checksum: bundle.markdown_checksum,
        sidecar_checksum: bundle.sidecar_checksum,
    })
}

pub async fn apply_claim_file_corrections(
    state: &AppState,
    workspace_root: PathBuf,
    markdown_rel_path: PathBuf,
    actor_principal_id: String,
) -> Result<ClaimFileApplyResult, ClaimFileError> {
    let markdown_rel_path = validate_projection_relative_path(&markdown_rel_path)?;
    let sidecar_rel_path = sidecar_path_for_markdown_rel(&markdown_rel_path)?;

    let markdown = read_stable_to_string(&workspace_root, &markdown_rel_path)?;
    let sidecar_json = read_stable_to_string(&workspace_root, &sidecar_rel_path)?;
    let sidecar_checksum = sha256_hex(sidecar_json.as_bytes());
    let sidecar: ClaimFileSidecar = serde_json::from_str(&sidecar_json)?;
    verify_sidecar_contract(&sidecar)?;
    verify_sidecar_paths(&sidecar, &markdown_rel_path, &sidecar_rel_path)?;
    validate_committed_sidecar_projection(state, &sidecar, &sidecar_checksum).await?;

    let corrections = match parse_markdown_corrections(&markdown, &sidecar, &sidecar_checksum) {
        Ok(corrections) => corrections,
        Err(error) => return Ok(parse_failure_apply_result(sidecar.claims.len(), error)),
    };
    if corrections.is_empty() {
        return Ok(ClaimFileApplyResult {
            applied_count: 0,
            skipped_count: sidecar.claims.len(),
            failures: Vec::new(),
            responses: Vec::new(),
            rerender: None,
        });
    }
    let claim_ids = sidecar
        .claims
        .iter()
        .map(|claim| claim.runtime_claim_id.clone())
        .collect::<BTreeSet<_>>();
    let envelope = FileEnvelope { claim_ids };
    let set = EnvelopeSet::new(&envelope);
    let actor = RenderActor::user(actor_principal_id.clone(), Some(actor_principal_id.clone()));
    let cache = IdempotencyCache::new();
    let mut responses = Vec::new();
    let mut failures = Vec::new();

    for correction in corrections {
        let payload_hash = correction_payload_hash(&correction);
        let apply_key =
            correction_apply_idempotency_key(&sidecar_checksum, &correction, &payload_hash);
        match claim_correction_apply(
            state,
            &apply_key,
            &sidecar_checksum,
            &correction,
            &payload_hash,
        )
        .await?
        {
            CorrectionApplyState::AlreadyApplied { feedback_id } => {
                if let Some(feedback_id) = feedback_id {
                    repair_claim_file_feedback_signal(state, feedback_id).await?;
                }
                continue;
            }
            CorrectionApplyState::InProgress => {
                failures.push(ClaimFileApplyFailure {
                    claim_id: Some(correction.claim_id),
                    error_class: "correction_apply_in_progress".to_string(),
                    error_detail_hash: redacted_hash(&apply_key),
                });
                continue;
            }
            CorrectionApplyState::Claimed => {}
        }
        let current_failures =
            current_projection_failures(state, &sidecar, std::slice::from_ref(&correction)).await?;
        if let Some(failure) = current_failures.into_iter().next() {
            match mark_correction_apply_failed(state, &apply_key, "stale_projection").await? {
                CorrectionFailureMarkState::MarkedFailed => failures.push(failure),
                CorrectionFailureMarkState::AlreadyApplied => continue,
            }
            continue;
        }
        let target = receipt_target_for_claim(&sidecar, &correction.claim_id)?;
        let request = ClaimFeedbackRequest {
            target,
            action: correction.action,
            surface: SurfaceContext::EntityDetail,
            metadata: correction.metadata,
            idempotency_key: None,
        };
        match submit_claim_feedback_for_claim_file_apply(
            state,
            &set,
            &actor,
            &cache,
            request,
            ClaimFileFeedbackApplyCommit {
                expected_claim_version: correction.projected_claim_version,
                correction_apply_key: apply_key.clone(),
            },
        )
        .await
        {
            Ok(response) => {
                responses.push(response);
            }
            Err(error) => {
                match mark_correction_apply_failed(state, &apply_key, &error.to_string()).await? {
                    CorrectionFailureMarkState::MarkedFailed => {
                        failures.push(ClaimFileApplyFailure {
                            claim_id: Some(correction.claim_id),
                            error_class: "feedback_rejected".to_string(),
                            error_detail_hash: redacted_hash(&error.to_string()),
                        });
                    }
                    CorrectionFailureMarkState::AlreadyApplied => continue,
                }
            }
        }
    }

    let rerender = if failures.is_empty() {
        match render_entity_claim_file(
            state,
            workspace_root,
            serde_json::to_string(&sidecar.entity_subject_ref)?,
        )
        .await
        {
            Ok(result) => Some(result),
            Err(error) => {
                failures.push(ClaimFileApplyFailure {
                    claim_id: None,
                    error_class: "projection_rerender_failed".to_string(),
                    error_detail_hash: redacted_hash(&error.to_string()),
                });
                None
            }
        }
    } else {
        None
    };

    Ok(ClaimFileApplyResult {
        applied_count: responses.len(),
        skipped_count: sidecar
            .claims
            .len()
            .saturating_sub(responses.len() + failures.len()),
        failures,
        responses,
        rerender,
    })
}

async fn build_render_bundle(
    state: &AppState,
    subject_ref_json: String,
) -> Result<RenderBundle, ClaimFileError> {
    state
        .db_read(move |db| build_render_bundle_db(db, &subject_ref_json))
        .await
        .map_err(|error| ClaimFileError::Db(error.to_string()))
}

fn build_render_bundle_db(db: &ActionDb, subject_ref_json: &str) -> Result<RenderBundle, String> {
    let subject_value: serde_json::Value =
        serde_json::from_str(subject_ref_json).map_err(|error| error.to_string())?;
    let subject = abilities_runtime::types::subject_ref_from_json(&subject_value)
        .map_err(|error| format!("subject_ref: {error}"))?;
    let entity_subject_compact = canonical_subject_compact(&subject)?;
    let entity_kind = supported_entity_kind_slug(&subject).ok_or_else(|| {
        "claim file projection supports account/project/person subjects".to_string()
    })?;
    let entity_id = subject_id_for_path(&subject)
        .ok_or_else(|| "claim file projection subject requires an id".to_string())?;
    let entity_slug = projection_entity_slug(entity_id, &entity_subject_compact);
    let markdown_rel_path = PathBuf::from(CLAIM_FILE_PROJECTION_ROOT)
        .join(entity_kind)
        .join(&entity_slug)
        .join(CLAIM_FILE_MARKDOWN_NAME);
    let sidecar_rel_path = PathBuf::from(CLAIM_FILE_PROJECTION_ROOT)
        .join(entity_kind)
        .join(entity_slug)
        .join(CLAIM_FILE_SIDECAR_NAME);

    let mut claims = load_claims_for_projection(db, subject_ref_json, entity_kind, entity_id)?;
    claims.sort_by(|a, b| {
        a.claim_type
            .cmp(&b.claim_type)
            .then_with(|| a.field_path.cmp(&b.field_path))
            .then_with(|| a.id.cmp(&b.id))
    });
    let entity_claim_invalidation_version = current_entity_claim_version(db, &subject)?;
    let claim_watermark = claim_watermark(&claims, entity_claim_invalidation_version);
    let claim_file_claims = claims
        .iter()
        .map(|claim| claim_file_claim(db, claim))
        .collect::<Result<Vec<_>, _>>()?;
    let sidecar = ClaimFileSidecar {
        schema_version: CLAIM_FILE_SIDECAR_SCHEMA_VERSION,
        projection_version: CLAIM_FILE_PROJECTION_VERSION,
        entity_subject_ref: subject_value,
        entity_subject_compact: entity_subject_compact.clone(),
        markdown_rel_path: path_to_slash_string(&markdown_rel_path),
        sidecar_rel_path: path_to_slash_string(&sidecar_rel_path),
        claims: claim_file_claims,
    };
    let sidecar_json = serde_json::to_string_pretty(&sidecar).map_err(|error| error.to_string())?;
    let sidecar_checksum = sha256_hex(sidecar_json.as_bytes());
    let markdown = render_markdown(&sidecar, &sidecar_checksum);
    let markdown_checksum = sha256_hex(markdown.as_bytes());

    Ok(RenderBundle {
        run_id: uuid::Uuid::new_v4().to_string(),
        subject_ref_json: subject_ref_json.to_string(),
        entity_subject_compact,
        markdown_rel_path,
        sidecar_rel_path,
        entity_claim_invalidation_version,
        claim_watermark,
        markdown,
        sidecar,
        sidecar_json,
        markdown_checksum,
        sidecar_checksum,
    })
}

pub(crate) fn render_entity_claim_file_eval_snapshot_db(
    db: &ActionDb,
    subject_ref_json: &str,
) -> Result<ClaimFileProjectionEvalSnapshot, String> {
    let bundle = build_render_bundle_db(db, subject_ref_json)?;
    Ok(ClaimFileProjectionEvalSnapshot {
        markdown_checksum: bundle.markdown_checksum,
        sidecar_checksum: bundle.sidecar_checksum,
        claim_count: bundle.sidecar.claims.len(),
    })
}

fn load_claims_for_projection(
    db: &ActionDb,
    subject_ref_json: &str,
    entity_kind: &str,
    entity_id: &str,
) -> Result<Vec<IntelligenceClaim>, String> {
    let mut claims = crate::services::claims::load_claims_active(db, subject_ref_json, None)
        .map_err(|error| error.to_string())?;
    let mut existing_ids = claims
        .iter()
        .map(|claim| claim.id.clone())
        .collect::<BTreeSet<_>>();
    for claim_id in correction_bearing_terminal_claim_ids(db, entity_kind, entity_id)? {
        if !existing_ids.insert(claim_id.clone()) {
            continue;
        }
        if let Some(claim) = crate::services::claims::load_claim_by_id(db.conn_ref(), &claim_id)
            .map_err(|error| error.to_string())?
        {
            claims.push(claim);
        }
    }
    Ok(claims)
}

fn correction_bearing_terminal_claim_ids(
    db: &ActionDb,
    entity_kind: &str,
    entity_id: &str,
) -> Result<Vec<String>, String> {
    let mut stmt = db
        .conn_ref()
        .prepare(
            "SELECT ic.id
             FROM intelligence_claims ic
             WHERE json_valid(ic.subject_ref) = 1
               AND lower(json_extract(ic.subject_ref, '$.kind')) = lower(?1)
               AND json_extract(ic.subject_ref, '$.id') = ?2
               AND NOT (ic.claim_state = 'active' AND ic.surfacing_state = 'active')
               AND EXISTS (
                   SELECT 1
                   FROM claim_feedback cf
                   WHERE cf.claim_id = ic.id
               )
             ORDER BY ic.created_at ASC, ic.id ASC",
        )
        .map_err(|error| error.to_string())?;
    let rows = stmt
        .query_map(params![entity_kind, entity_id], |row| {
            row.get::<_, String>(0)
        })
        .map_err(|error| error.to_string())?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|error| error.to_string())?);
    }
    Ok(out)
}

fn claim_file_claim(db: &ActionDb, claim: &IntelligenceClaim) -> Result<ClaimFileClaim, String> {
    let semantic_identity = semantic_identity_for_loaded_claim(claim)?;
    Ok(ClaimFileClaim {
        runtime_claim_id: claim.id.clone(),
        runtime_claim_version: claim.claim_version,
        claim_text: claim.text.clone(),
        trust_band: trust_band_label(claim.trust_score).to_string(),
        sensitivity: sensitivity_label(&claim.sensitivity),
        lifecycle: lifecycle_for_claim(claim),
        provenance_summary: ClaimFileProvenanceSummary {
            data_source: claim.data_source.clone(),
            source_ref: claim.source_ref.clone(),
            source_asof: claim.source_asof.clone(),
            observed_at: claim.observed_at.clone(),
        },
        feedback_rows: load_feedback_rows(db, &claim.id)?,
        contradiction_edges: load_contradiction_edges(db, &claim.id)?,
        superseded_by_semantic_identity: claim
            .superseded_by
            .as_deref()
            .map(|claim_id| semantic_identity_for_claim_id(db, claim_id))
            .transpose()?
            .flatten(),
        replay_status: projected_replay_status(),
        semantic_identity,
    })
}

pub(crate) fn semantic_identity_for_loaded_claim(
    claim: &IntelligenceClaim,
) -> Result<ClaimSemanticIdentityV1, String> {
    let subject_value: serde_json::Value =
        serde_json::from_str(&claim.subject_ref).map_err(|error| error.to_string())?;
    let subject = abilities_runtime::types::subject_ref_from_json(&subject_value)
        .map_err(|error| format!("claim subject_ref: {error}"))?;
    let subject_ref_compact = canonical_subject_compact(&subject)?;
    Ok(semantic_identity_for_claim(
        claim,
        subject_value,
        subject_ref_compact,
    ))
}

fn lifecycle_for_claim(claim: &IntelligenceClaim) -> ClaimFileLifecycle {
    ClaimFileLifecycle {
        claim_state: format!("{:?}", claim.claim_state).to_ascii_lowercase(),
        surfacing_state: format!("{:?}", claim.surfacing_state).to_ascii_lowercase(),
        demotion_reason: claim.demotion_reason.clone(),
        retraction_reason: claim.retraction_reason.clone(),
        superseded_by: claim.superseded_by.clone(),
        verification_state: format!("{:?}", claim.verification_state).to_ascii_lowercase(),
        verification_reason: claim.verification_reason.clone(),
        needs_user_decision_at: claim.needs_user_decision_at.clone(),
    }
}

fn projected_replay_status() -> ClaimFileReplayStatus {
    ClaimFileReplayStatus {
        status: "projected".to_string(),
        reason: None,
        last_attempted_at: None,
    }
}

fn semantic_identity_for_claim(
    claim: &IntelligenceClaim,
    subject_ref: serde_json::Value,
    subject_ref_compact: String,
) -> ClaimSemanticIdentityV1 {
    let item_hash = claim
        .item_hash
        .clone()
        .unwrap_or_else(|| sha256_hex(claim.text.as_bytes()));
    let is_user_note = claim.claim_type == "user_note";
    let identity_kind = if is_user_note {
        "user_note_v1"
    } else {
        "claim_dedup_v1"
    };
    let field_path_component = claim.field_path.clone().unwrap_or_default();
    let components = format!(
        "{}\u{1f}{}\u{1f}{}\u{1f}{}",
        item_hash, subject_ref_compact, claim.claim_type, field_path_component
    );
    let dedup_key_components_hash = if is_user_note {
        crate::services::claims::compute_user_note_dedup_key(
            &subject_ref_compact,
            &claim.actor,
            &claim.observed_at,
        )
    } else {
        sha256_hex(components.as_bytes())
    };
    ClaimSemanticIdentityV1 {
        identity_version: 1,
        identity_kind: identity_kind.to_string(),
        item_hash,
        subject_ref_compact,
        subject_ref,
        claim_type: claim.claim_type.clone(),
        field_path: claim.field_path.clone(),
        dedup_key_components_hash,
        source_ref: claim.source_ref.clone(),
        data_source: claim.data_source.clone(),
        actor: claim.actor.clone(),
        observed_at: claim.observed_at.clone(),
        source_asof: claim.source_asof.clone(),
        source_content_hash: Some(source_content_hash_for_claim(claim)),
        runtime_claim_id: claim.id.clone(),
        runtime_claim_version: claim.claim_version,
        claim_state: format!("{:?}", claim.claim_state).to_ascii_lowercase(),
        superseded_by: claim.superseded_by.clone(),
    }
}

fn load_feedback_rows(db: &ActionDb, claim_id: &str) -> Result<Vec<ClaimFileFeedbackRow>, String> {
    let mut stmt = db
        .conn_ref()
        .prepare(
            "SELECT id, feedback_type, actor, actor_id, payload_json, submitted_at, applied_at
             FROM claim_feedback
             WHERE claim_id = ?1
             ORDER BY submitted_at ASC, id ASC",
        )
        .map_err(|error| error.to_string())?;
    let rows = stmt
        .query_map(params![claim_id], |row| {
            let payload_raw: Option<String> = row.get(4)?;
            Ok(ClaimFileFeedbackRow {
                feedback_id: row.get(0)?,
                action: row.get(1)?,
                actor: row.get(2)?,
                actor_id: row.get(3)?,
                payload_json: payload_raw
                    .as_deref()
                    .and_then(|raw| serde_json::from_str(raw).ok()),
                submitted_at: row.get(5)?,
                applied_at: row.get(6)?,
            })
        })
        .map_err(|error| error.to_string())?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|error| error.to_string())?);
    }
    Ok(out)
}

fn load_contradiction_edges(
    db: &ActionDb,
    claim_id: &str,
) -> Result<Vec<ClaimFileContradictionEdge>, String> {
    let mut stmt = db
        .conn_ref()
        .prepare(
            "SELECT id, primary_claim_id, contradicting_claim_id, branch_kind, detected_at,
                    reconciliation_kind, reconciliation_note, reconciled_at, winner_claim_id,
                    merged_claim_id
             FROM claim_contradictions
             WHERE primary_claim_id = ?1 OR contradicting_claim_id = ?1
             ORDER BY detected_at ASC, id ASC",
        )
        .map_err(|error| error.to_string())?;
    let rows = stmt
        .query_map(params![claim_id], |row| {
            Ok(ContradictionEdgeRow {
                edge_id: row.get(0)?,
                primary_claim_id: row.get(1)?,
                contradicting_claim_id: row.get(2)?,
                branch_kind: row.get(3)?,
                detected_at: row.get(4)?,
                reconciliation_kind: row.get(5)?,
                reconciliation_note: row.get(6)?,
                reconciled_at: row.get(7)?,
                winner_claim_id: row.get(8)?,
                merged_claim_id: row.get(9)?,
            })
        })
        .map_err(|error| error.to_string())?;
    let mut out = Vec::new();
    for row in rows {
        let row = row.map_err(|error| error.to_string())?;
        let primary_semantic_identity = semantic_identity_for_claim_id(db, &row.primary_claim_id)?;
        let contradicting_semantic_identity =
            semantic_identity_for_claim_id(db, &row.contradicting_claim_id)?;
        let winner_semantic_identity = row
            .winner_claim_id
            .as_deref()
            .map(|claim_id| semantic_identity_for_claim_id(db, claim_id))
            .transpose()?
            .flatten();
        let merged_semantic_identity = row
            .merged_claim_id
            .as_deref()
            .map(|claim_id| semantic_identity_for_claim_id(db, claim_id))
            .transpose()?
            .flatten();
        out.push(ClaimFileContradictionEdge {
            edge_id: row.edge_id,
            branch_kind: row.branch_kind,
            role: if row.primary_claim_id == claim_id {
                "primary".to_string()
            } else {
                "contradicting".to_string()
            },
            primary_runtime_claim_id: row.primary_claim_id,
            contradicting_runtime_claim_id: row.contradicting_claim_id,
            primary_semantic_identity,
            contradicting_semantic_identity,
            detected_at: row.detected_at,
            reconciliation_kind: row.reconciliation_kind,
            reconciliation_note: row.reconciliation_note,
            reconciled_at: row.reconciled_at,
            winner_runtime_claim_id: row.winner_claim_id,
            winner_semantic_identity,
            merged_runtime_claim_id: row.merged_claim_id,
            merged_semantic_identity,
            replay_status: projected_replay_status(),
        });
    }
    Ok(out)
}

#[derive(Debug)]
struct ContradictionEdgeRow {
    edge_id: String,
    primary_claim_id: String,
    contradicting_claim_id: String,
    branch_kind: String,
    detected_at: String,
    reconciliation_kind: Option<String>,
    reconciliation_note: Option<String>,
    reconciled_at: Option<String>,
    winner_claim_id: Option<String>,
    merged_claim_id: Option<String>,
}

fn semantic_identity_for_claim_id(
    db: &ActionDb,
    claim_id: &str,
) -> Result<Option<ClaimSemanticIdentityV1>, String> {
    crate::services::claims::load_claim_by_id(db.conn_ref(), claim_id)
        .map_err(|error| error.to_string())?
        .as_ref()
        .map(semantic_identity_for_loaded_claim)
        .transpose()
}

fn render_markdown(sidecar: &ClaimFileSidecar, sidecar_checksum: &str) -> String {
    let mut out = String::new();
    out.push_str("# DailyOS Claims\n\n");
    out.push_str("These files mirror canonical claim state. Edit only the dailyos-action and dailyos-payload lines inside a claim block.\n\n");
    out.push_str(&format!(
        "sidecar: `{}`\nsidecar_checksum: `{}`\n\n",
        sidecar.sidecar_rel_path, sidecar_checksum
    ));
    for claim in &sidecar.claims {
        out.push_str(&format!(
            "## {}\n\n",
            markdown_inline_text(&claim.claim_text)
        ));
        out.push_str(&format!("- Trust: `{}`\n", claim.trust_band));
        out.push_str(&format!("- Sensitivity: `{}`\n", claim.sensitivity));
        if claim.lifecycle.claim_state != "active" || claim.lifecycle.surfacing_state != "active" {
            out.push_str(&format!(
                "- Lifecycle: `{}` / `{}`\n",
                claim.lifecycle.claim_state, claim.lifecycle.surfacing_state
            ));
        }
        out.push_str(&format!(
            "- Source: `{}` `{}`\n",
            markdown_inline_text(&claim.provenance_summary.data_source),
            claim
                .provenance_summary
                .source_ref
                .as_deref()
                .map(markdown_inline_text)
                .unwrap_or_else(|| "source_ref_unavailable".to_string())
        ));
        if let Some(source_asof) = claim.provenance_summary.source_asof.as_deref() {
            out.push_str(&format!(
                "- Source as-of: `{}`\n",
                markdown_inline_text(source_asof)
            ));
        }
        out.push('\n');
        out.push_str("<!-- dailyos-claim-start -->\n");
        out.push_str(&format!("dailyos-claim-id: {}\n", claim.runtime_claim_id));
        out.push_str(&format!(
            "dailyos-claim-version: {}\n",
            claim.runtime_claim_version
        ));
        out.push_str(&format!(
            "dailyos-identity-hash: {}\n",
            claim.semantic_identity.dedup_key_components_hash
        ));
        out.push_str(&format!("dailyos-sidecar-checksum: {}\n", sidecar_checksum));
        out.push_str("dailyos-action: none\n");
        out.push_str("dailyos-payload: {}\n");
        out.push_str("<!-- dailyos-claim-end -->\n\n");
    }
    out
}

async fn record_projection_run(
    state: &AppState,
    bundle: &RenderBundle,
    status: &str,
    repaired_from_run_id: Option<String>,
    write_error: Option<&std::io::Error>,
) -> Result<(), ClaimFileError> {
    let run = bundle.clone();
    let error_class = write_error.map(|error| error.kind().to_string());
    let error_detail_hash = write_error.map(|error| redacted_hash(&error.to_string()));
    let status = status.to_string();
    state
        .db_write(move |db| {
            insert_projection_run(
                db,
                &run,
                &status,
                repaired_from_run_id.as_deref(),
                error_class.as_deref(),
                error_detail_hash.as_deref(),
            )
        })
        .await
        .map_err(|error| ClaimFileError::Db(error.to_string()))?;
    Ok(())
}

async fn ensure_projection_path_binding_state(
    state: &AppState,
    bundle: &RenderBundle,
) -> Result<(), ClaimFileError> {
    let bundle = bundle.clone();
    state
        .db_write(move |db| {
            let now = Utc::now().to_rfc3339();
            ensure_projection_path_binding(db, &bundle, &now)
        })
        .await
        .map_err(|error| ClaimFileError::Db(error.to_string()))
}

fn insert_projection_run(
    db: &ActionDb,
    run: &RenderBundle,
    status: &str,
    repaired_from_run_id: Option<&str>,
    error_class: Option<&str>,
    error_detail_hash: Option<&str>,
) -> Result<(), String> {
    let now = Utc::now().to_rfc3339();
    ensure_projection_path_binding(db, run, &now)?;
    db.conn_ref()
        .execute(
            "INSERT INTO claim_file_projection_runs (
                id, entity_subject_ref_json, entity_subject_compact, projection_root,
                markdown_rel_path, sidecar_rel_path, projection_version,
                sidecar_schema_version, entity_claim_invalidation_version,
                claim_watermark, markdown_checksum, sidecar_checksum, status,
                error_class, error_detail_hash, attempted_at, succeeded_at,
                repaired_from_run_id, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?16, ?16)",
            params![
                &run.run_id,
                &run.subject_ref_json,
                &run.entity_subject_compact,
                CLAIM_FILE_PROJECTION_ROOT,
                path_to_slash_string(&run.markdown_rel_path),
                path_to_slash_string(&run.sidecar_rel_path),
                CLAIM_FILE_PROJECTION_VERSION,
                CLAIM_FILE_SIDECAR_SCHEMA_VERSION,
                run.entity_claim_invalidation_version,
                &run.claim_watermark,
                &run.markdown_checksum,
                &run.sidecar_checksum,
                status,
                error_class,
                error_detail_hash,
                &now,
                if matches!(status, "committed" | "repaired") {
                    Some(now.as_str())
                } else {
                    None
                },
                repaired_from_run_id,
            ],
        )
        .map_err(|error| error.to_string())?;
    if matches!(status, "committed" | "repaired") {
        for claim in &run.sidecar.claims {
            let semantic_identity_json = serde_json::to_string(&claim.semantic_identity)
                .map_err(|error| error.to_string())?;
            db.conn_ref()
                .execute(
                    "INSERT INTO claim_file_projection_run_claims (
                        run_id, claim_id, claim_version, semantic_identity_json,
                        trust_band, sensitivity
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        &run.run_id,
                        &claim.runtime_claim_id,
                        claim.runtime_claim_version,
                        semantic_identity_json,
                        &claim.trust_band,
                        &claim.sensitivity,
                    ],
                )
                .map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

async fn validate_committed_sidecar_projection(
    state: &AppState,
    sidecar: &ClaimFileSidecar,
    sidecar_checksum: &str,
) -> Result<CommittedProjectionRun, ClaimFileError> {
    let sidecar = sidecar.clone();
    let sidecar_checksum = sidecar_checksum.to_string();
    state
        .db_read(move |db| {
            validate_committed_sidecar_projection_db(db, &sidecar, &sidecar_checksum)
        })
        .await
        .map_err(|error| ClaimFileError::Db(error.to_string()))
}

fn validate_committed_sidecar_projection_db(
    db: &ActionDb,
    sidecar: &ClaimFileSidecar,
    sidecar_checksum: &str,
) -> Result<CommittedProjectionRun, String> {
    validate_sidecar_projection_db(
        db,
        sidecar,
        sidecar_checksum,
        CLAIM_FILE_SIDECAR_SCHEMA_VERSION,
    )
}

pub(crate) fn validate_replay_sidecar_projection_db(
    db: &ActionDb,
    sidecar: &ClaimFileSidecar,
    sidecar_checksum: &str,
) -> Result<CommittedProjectionRun, String> {
    validate_sidecar_projection_db(db, sidecar, sidecar_checksum, sidecar.schema_version)
}

fn validate_sidecar_projection_db(
    db: &ActionDb,
    sidecar: &ClaimFileSidecar,
    sidecar_checksum: &str,
    expected_sidecar_schema_version: u32,
) -> Result<CommittedProjectionRun, String> {
    let binding: Option<(String, String)> = db
        .conn_ref()
        .query_row(
            "SELECT sidecar_rel_path, entity_subject_compact
               FROM claim_file_projection_path_bindings
              WHERE markdown_rel_path = ?1",
            [&sidecar.markdown_rel_path],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    let Some((bound_sidecar_path, bound_subject)) = binding else {
        return Err("projection path binding missing".to_string());
    };
    if bound_sidecar_path != sidecar.sidecar_rel_path
        || bound_subject != sidecar.entity_subject_compact
    {
        return Err("projection path binding mismatch".to_string());
    }

    let run_id: Option<String> = db
        .conn_ref()
        .query_row(
            "SELECT id
               FROM claim_file_projection_runs
              WHERE entity_subject_compact = ?1
                AND markdown_rel_path = ?2
                AND sidecar_rel_path = ?3
                AND sidecar_checksum = ?4
                AND projection_root = ?5
                AND projection_version = ?6
                AND sidecar_schema_version = ?7
                AND status IN ('committed', 'repaired')
              ORDER BY attempted_at DESC, id DESC
              LIMIT 1",
            params![
                &sidecar.entity_subject_compact,
                &sidecar.markdown_rel_path,
                &sidecar.sidecar_rel_path,
                sidecar_checksum,
                CLAIM_FILE_PROJECTION_ROOT,
                CLAIM_FILE_PROJECTION_VERSION,
                expected_sidecar_schema_version,
            ],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    let Some(run_id) = run_id else {
        return Err("committed projection run not found for sidecar".to_string());
    };

    let run_claim_count: i64 = db
        .conn_ref()
        .query_row(
            "SELECT count(*)
               FROM claim_file_projection_run_claims
              WHERE run_id = ?1",
            [&run_id],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    if run_claim_count != sidecar.claims.len() as i64 {
        return Err("projection claim membership count mismatch".to_string());
    }

    for claim in &sidecar.claims {
        if claim.semantic_identity.runtime_claim_id != claim.runtime_claim_id
            || claim.semantic_identity.runtime_claim_version != claim.runtime_claim_version
        {
            return Err("sidecar claim identity/version mismatch".to_string());
        }
        let row: Option<(u64, String, String, String)> = db
            .conn_ref()
            .query_row(
                "SELECT claim_version, semantic_identity_json, trust_band, sensitivity
                   FROM claim_file_projection_run_claims
                  WHERE run_id = ?1
                    AND claim_id = ?2",
                params![&run_id, &claim.runtime_claim_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .optional()
            .map_err(|error| error.to_string())?;
        let Some((claim_version, semantic_identity_json, trust_band, sensitivity)) = row else {
            return Err("projection claim membership missing".to_string());
        };
        let semantic_identity: ClaimSemanticIdentityV1 =
            serde_json::from_str(&semantic_identity_json).map_err(|error| error.to_string())?;
        if claim_version != claim.runtime_claim_version
            || semantic_identity != claim.semantic_identity
            || trust_band != claim.trust_band
            || sensitivity != claim.sensitivity
        {
            return Err("projection claim membership mismatch".to_string());
        }
    }

    Ok(CommittedProjectionRun { run_id })
}

fn ensure_projection_path_binding(
    db: &ActionDb,
    run: &RenderBundle,
    now: &str,
) -> Result<(), String> {
    let markdown_rel_path = path_to_slash_string(&run.markdown_rel_path);
    let sidecar_rel_path = path_to_slash_string(&run.sidecar_rel_path);
    let inserted = db
        .conn_ref()
        .execute(
            "INSERT OR IGNORE INTO claim_file_projection_path_bindings (
                markdown_rel_path, sidecar_rel_path, entity_subject_compact,
                projection_root, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
            params![
                &markdown_rel_path,
                &sidecar_rel_path,
                &run.entity_subject_compact,
                CLAIM_FILE_PROJECTION_ROOT,
                now,
            ],
        )
        .map_err(|error| error.to_string())?;
    if inserted == 1 {
        return Ok(());
    }

    let mut stmt = db
        .conn_ref()
        .prepare(
            "SELECT markdown_rel_path, sidecar_rel_path, entity_subject_compact
               FROM claim_file_projection_path_bindings
              WHERE markdown_rel_path = ?1
                 OR sidecar_rel_path = ?2
                 OR entity_subject_compact = ?3",
        )
        .map_err(|error| error.to_string())?;
    let rows = stmt
        .query_map(
            params![
                &markdown_rel_path,
                &sidecar_rel_path,
                &run.entity_subject_compact,
            ],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .map_err(|error| error.to_string())?;
    let mut saw_matching_binding = false;
    for row in rows {
        let (existing_markdown, existing_sidecar, existing_subject) =
            row.map_err(|error| error.to_string())?;
        if existing_markdown != markdown_rel_path
            || existing_sidecar != sidecar_rel_path
            || existing_subject != run.entity_subject_compact
        {
            return Err("projection path collision".to_string());
        }
        saw_matching_binding = true;
    }
    if !saw_matching_binding {
        return Err("projection path binding missing after insert".to_string());
    }
    db.conn_ref()
        .execute(
            "UPDATE claim_file_projection_path_bindings
                SET updated_at = ?1
              WHERE markdown_rel_path = ?2",
            params![now, &markdown_rel_path],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn projection_change_status_for_bundle(
    db: &ActionDb,
    bundle: &RenderBundle,
) -> Result<ClaimFileProjectionChangeStatus, String> {
    let latest = latest_projection_run(db, &bundle.entity_subject_compact)?;
    let mut repair_reasons = Vec::new();
    if let Some(latest) = &latest {
        if latest.status == "failed" {
            repair_reasons.push("previous_projection_failed".to_string());
        }
        if latest.claim_watermark != bundle.claim_watermark {
            repair_reasons.push("claim_watermark_changed".to_string());
        }
    } else {
        repair_reasons.push("missing_projection".to_string());
    }

    Ok(ClaimFileProjectionChangeStatus {
        entity_subject_compact: bundle.entity_subject_compact.clone(),
        current_claim_watermark: bundle.claim_watermark.clone(),
        markdown_rel_path: path_to_slash_string(&bundle.markdown_rel_path),
        sidecar_rel_path: path_to_slash_string(&bundle.sidecar_rel_path),
        latest_run_id: latest.as_ref().map(|run| run.run_id.clone()),
        latest_run_status: latest.as_ref().map(|run| run.status.clone()),
        latest_claim_watermark: latest.as_ref().map(|run| run.claim_watermark.clone()),
        latest_attempted_at: latest.as_ref().map(|run| run.attempted_at.clone()),
        needs_repair: !repair_reasons.is_empty(),
        repair_reasons,
    })
}

fn latest_projection_run(
    db: &ActionDb,
    entity_subject_compact: &str,
) -> Result<Option<ProjectionRunSummary>, String> {
    db.conn_ref()
        .query_row(
            "SELECT id, status, claim_watermark, markdown_rel_path, sidecar_rel_path, attempted_at
               FROM claim_file_projection_runs
              WHERE entity_subject_compact = ?1
              ORDER BY attempted_at DESC, created_at DESC, id DESC
              LIMIT 1",
            [entity_subject_compact],
            |row| {
                Ok(ProjectionRunSummary {
                    run_id: row.get(0)?,
                    status: row.get(1)?,
                    claim_watermark: row.get(2)?,
                    markdown_rel_path: row.get(3)?,
                    sidecar_rel_path: row.get(4)?,
                    attempted_at: row.get(5)?,
                })
            },
        )
        .optional()
        .map_err(|error| error.to_string())
}

fn parse_markdown_corrections(
    markdown: &str,
    sidecar: &ClaimFileSidecar,
    sidecar_checksum: &str,
) -> Result<Vec<ParsedCorrection>, ClaimFileError> {
    let mut corrections = Vec::new();
    let mut current: Option<ParsedBlock> = None;
    for line in markdown.lines() {
        let trimmed = line.trim();
        if trimmed == "<!-- dailyos-claim-start -->" {
            if current.is_some() {
                return Err(ClaimFileError::BadRequest(
                    "nested claim block is ambiguous".to_string(),
                ));
            }
            current = Some(ParsedBlock::default());
            continue;
        }
        if trimmed == "<!-- dailyos-claim-end -->" {
            if let Some(block) = current.take() {
                if let Some(correction) = correction_from_block(block, sidecar, sidecar_checksum)? {
                    corrections.push(correction);
                }
            } else {
                return Err(ClaimFileError::BadRequest(
                    "claim block end without start".to_string(),
                ));
            }
            continue;
        }
        let Some(block) = current.as_mut() else {
            continue;
        };
        if block.directives_closed {
            continue;
        }
        if trimmed.is_empty() {
            block.directives_closed = true;
            continue;
        }
        if let Some(value) = trimmed.strip_prefix("dailyos-claim-id:") {
            block.claim_id = Some(value.trim().to_string());
        } else if let Some(value) = trimmed.strip_prefix("dailyos-claim-version:") {
            block.version_header = Some(value.trim().to_string());
        } else if let Some(value) = trimmed.strip_prefix("dailyos-identity-hash:") {
            block.identity_hash = Some(value.trim().to_string());
        } else if let Some(value) = trimmed.strip_prefix("dailyos-sidecar-checksum:") {
            block.sidecar_checksum = Some(value.trim().to_string());
        } else if let Some(value) = trimmed.strip_prefix("dailyos-action:") {
            block.action = Some(value.trim().to_string());
        } else if let Some(value) = trimmed.strip_prefix("dailyos-payload:") {
            block.payload = Some(value.trim().to_string());
        } else if trimmed.starts_with("dailyos-") {
            continue;
        } else {
            block.directives_closed = true;
        }
    }
    if current.is_some() {
        return Err(ClaimFileError::BadRequest(
            "unterminated claim block".to_string(),
        ));
    }
    Ok(corrections)
}

fn parse_failure_apply_result(skipped_count: usize, error: ClaimFileError) -> ClaimFileApplyResult {
    let error_class = parse_failure_error_class(&error).to_string();
    let error_detail_hash = redacted_hash(&error.to_string());
    ClaimFileApplyResult {
        applied_count: 0,
        skipped_count,
        failures: vec![ClaimFileApplyFailure {
            claim_id: None,
            error_class,
            error_detail_hash,
        }],
        responses: Vec::new(),
        rerender: None,
    }
}

fn parse_failure_error_class(error: &ClaimFileError) -> &'static str {
    let message = error.to_string();
    if message.contains("sidecar_mismatch") {
        "sidecar_mismatch"
    } else if message.contains("unsupported file feedback action") {
        "unsupported_action"
    } else if message.contains("unterminated claim block") {
        "parse_unterminated_block"
    } else if message.contains("nested claim block") {
        "parse_nested_block"
    } else if message.contains("not present in sidecar") {
        "unknown_claim"
    } else {
        "parse_failed"
    }
}

#[derive(Debug, Default)]
struct ParsedBlock {
    claim_id: Option<String>,
    version_header: Option<String>,
    identity_hash: Option<String>,
    sidecar_checksum: Option<String>,
    action: Option<String>,
    payload: Option<String>,
    directives_closed: bool,
}

fn correction_from_block(
    block: ParsedBlock,
    sidecar: &ClaimFileSidecar,
    sidecar_checksum: &str,
) -> Result<Option<ParsedCorrection>, ClaimFileError> {
    let claim_id = block.claim_id.ok_or_else(|| {
        ClaimFileError::BadRequest("claim block missing dailyos-claim-id".to_string())
    })?;
    let claim = sidecar
        .claims
        .iter()
        .find(|claim| claim.runtime_claim_id == claim_id)
        .ok_or_else(|| {
            ClaimFileError::BadRequest("claim block not present in sidecar".to_string())
        })?;
    let block_checksum = block.sidecar_checksum.ok_or_else(|| {
        ClaimFileError::BadRequest("claim block missing dailyos-sidecar-checksum".to_string())
    })?;
    if block_checksum != sidecar_checksum {
        return Err(ClaimFileError::BadRequest("sidecar_mismatch".to_string()));
    }
    let projected_claim_version = block
        .version_header
        .ok_or_else(|| {
            ClaimFileError::BadRequest("claim block missing dailyos-claim-version".to_string())
        })?
        .parse::<u64>()
        .map_err(|_| ClaimFileError::BadRequest("invalid dailyos-claim-version".to_string()))?;
    if projected_claim_version != claim.runtime_claim_version {
        return Err(ClaimFileError::BadRequest(
            "stale_projection_claim_version".to_string(),
        ));
    }
    let projected_identity_hash = block.identity_hash.ok_or_else(|| {
        ClaimFileError::BadRequest("claim block missing dailyos-identity-hash".to_string())
    })?;
    if projected_identity_hash != claim.semantic_identity.dedup_key_components_hash {
        return Err(ClaimFileError::BadRequest(
            "stale_projection_identity_hash".to_string(),
        ));
    }
    let action_raw = block.action.unwrap_or_else(|| "none".to_string());
    if action_raw == "none" {
        return Ok(None);
    }
    let action = feedback_action_from_slug(&action_raw)?;
    let metadata = metadata_for_action(action, block.payload.as_deref(), claim)?;
    Ok(Some(ParsedCorrection {
        claim_id,
        projected_claim_version,
        projected_identity_hash,
        action,
        metadata,
    }))
}

async fn current_projection_failures(
    state: &AppState,
    sidecar: &ClaimFileSidecar,
    corrections: &[ParsedCorrection],
) -> Result<Vec<ClaimFileApplyFailure>, ClaimFileError> {
    let sidecar = sidecar.clone();
    let corrections = corrections.to_vec();
    state
        .db_read(move |db| current_projection_failures_db(db, &sidecar, &corrections))
        .await
        .map_err(|error| ClaimFileError::Db(error.to_string()))
}

fn current_projection_failures_db(
    db: &ActionDb,
    sidecar: &ClaimFileSidecar,
    corrections: &[ParsedCorrection],
) -> Result<Vec<ClaimFileApplyFailure>, String> {
    let mut failures = Vec::new();
    for correction in corrections {
        let Some(projected_claim) = sidecar
            .claims
            .iter()
            .find(|claim| claim.runtime_claim_id == correction.claim_id)
        else {
            failures.push(stale_projection_failure(
                &correction.claim_id,
                "sidecar_claim_missing",
            ));
            continue;
        };
        let Some(current_claim) =
            crate::services::claims::load_claim_by_id(db.conn_ref(), &correction.claim_id)
                .map_err(|error| error.to_string())?
        else {
            failures.push(stale_projection_failure(
                &correction.claim_id,
                "claim_missing",
            ));
            continue;
        };
        let current_identity = semantic_identity_for_claim(
            &current_claim,
            sidecar.entity_subject_ref.clone(),
            sidecar.entity_subject_compact.clone(),
        );
        if current_claim.claim_version != correction.projected_claim_version
            || current_claim.claim_version != projected_claim.runtime_claim_version
            || current_identity.dedup_key_components_hash != correction.projected_identity_hash
            || current_identity.dedup_key_components_hash
                != projected_claim.semantic_identity.dedup_key_components_hash
        {
            failures.push(stale_projection_failure(
                &correction.claim_id,
                "current_claim_changed",
            ));
        }
    }
    Ok(failures)
}

fn stale_projection_failure(claim_id: &str, reason: &str) -> ClaimFileApplyFailure {
    ClaimFileApplyFailure {
        claim_id: Some(claim_id.to_string()),
        error_class: "stale_projection".to_string(),
        error_detail_hash: redacted_hash(reason),
    }
}

fn correction_payload_hash(correction: &ParsedCorrection) -> String {
    match correction.metadata.as_ref() {
        Some(metadata) => sha256_hex(metadata.to_string().as_bytes()),
        None => sha256_hex(b"null"),
    }
}

fn correction_apply_idempotency_key(
    sidecar_checksum: &str,
    correction: &ParsedCorrection,
    payload_hash: &str,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"claim_file_correction_apply_v1");
    hasher.update([0x1f]);
    hasher.update(sidecar_checksum.as_bytes());
    hasher.update([0x1f]);
    hasher.update(correction.claim_id.as_bytes());
    hasher.update([0x1f]);
    hasher.update(correction.projected_claim_version.to_string().as_bytes());
    hasher.update([0x1f]);
    hasher.update(correction.projected_identity_hash.as_bytes());
    hasher.update([0x1f]);
    hasher.update(correction.action.as_str().as_bytes());
    hasher.update([0x1f]);
    hasher.update(payload_hash.as_bytes());
    format!("{:x}", hasher.finalize())
}

async fn claim_correction_apply(
    state: &AppState,
    apply_key: &str,
    sidecar_checksum: &str,
    correction: &ParsedCorrection,
    payload_hash: &str,
) -> Result<CorrectionApplyState, ClaimFileError> {
    let apply_key = apply_key.to_string();
    let sidecar_checksum = sidecar_checksum.to_string();
    let claim_id = correction.claim_id.clone();
    let projected_claim_version = correction.projected_claim_version;
    let projected_identity_hash = correction.projected_identity_hash.clone();
    let feedback_action = correction.action.as_str().to_string();
    let payload_hash = payload_hash.to_string();
    state
        .db_write(move |db| {
            let record = CorrectionApplyRecord {
                apply_key: &apply_key,
                sidecar_checksum: &sidecar_checksum,
                claim_id: &claim_id,
                projected_claim_version,
                projected_identity_hash: &projected_identity_hash,
                feedback_action: &feedback_action,
                payload_hash: &payload_hash,
            };
            claim_correction_apply_db(db, &record)
        })
        .await
        .map_err(|error| ClaimFileError::Db(error.to_string()))
}

fn claim_correction_apply_db(
    db: &ActionDb,
    record: &CorrectionApplyRecord<'_>,
) -> Result<CorrectionApplyState, String> {
    let now = Utc::now();
    let now_text = now.to_rfc3339();
    let inserted = db
        .conn_ref()
        .execute(
            "INSERT OR IGNORE INTO claim_file_correction_apply_events (
                idempotency_key, sidecar_checksum, claim_id,
                projected_claim_version, projected_identity_hash, feedback_action,
                payload_hash, status, claimed_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'claimed', ?8, ?8)",
            params![
                record.apply_key,
                record.sidecar_checksum,
                record.claim_id,
                record.projected_claim_version,
                record.projected_identity_hash,
                record.feedback_action,
                record.payload_hash,
                &now_text,
            ],
        )
        .map_err(|error| error.to_string())?;
    if inserted == 1 {
        return Ok(CorrectionApplyState::Claimed);
    }

    let (status, claimed_at, feedback_id): (String, String, Option<String>) = db
        .conn_ref()
        .query_row(
            "SELECT status, claimed_at, feedback_id
              FROM claim_file_correction_apply_events
              WHERE idempotency_key = ?1",
            [record.apply_key],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "correction apply event missing after insert conflict".to_string())?;
    match status.as_str() {
        "applied" => Ok(CorrectionApplyState::AlreadyApplied { feedback_id }),
        "claimed" => {
            if correction_apply_claim_expired(&claimed_at, now)? {
                db.conn_ref()
                    .execute(
                        "UPDATE claim_file_correction_apply_events
                            SET claimed_at = ?1,
                                updated_at = ?1
                          WHERE idempotency_key = ?2
                            AND status = 'claimed'",
                        params![&now_text, record.apply_key],
                    )
                    .map_err(|error| error.to_string())?;
                Ok(CorrectionApplyState::Claimed)
            } else {
                Ok(CorrectionApplyState::InProgress)
            }
        }
        "failed" => {
            db.conn_ref()
                .execute(
                    "UPDATE claim_file_correction_apply_events
                        SET status = 'claimed',
                            error_detail_hash = NULL,
                            failed_at = NULL,
                            claimed_at = ?1,
                            updated_at = ?1
                      WHERE idempotency_key = ?2",
                    params![&now_text, record.apply_key],
                )
                .map_err(|error| error.to_string())?;
            Ok(CorrectionApplyState::Claimed)
        }
        other => Err(format!("unknown correction apply status `{other}`")),
    }
}

async fn mark_correction_apply_failed(
    state: &AppState,
    apply_key: &str,
    error_detail: &str,
) -> Result<CorrectionFailureMarkState, ClaimFileError> {
    let apply_key = apply_key.to_string();
    let error_detail_hash = redacted_hash(error_detail);
    state
        .db_write(move |db| mark_correction_apply_failed_db(db, &apply_key, &error_detail_hash))
        .await
        .map_err(|error| ClaimFileError::Db(error.to_string()))
}

async fn repair_claim_file_feedback_signal(
    state: &AppState,
    feedback_id: String,
) -> Result<(), ClaimFileError> {
    state
        .db_write(move |db| {
            let clock = crate::services::context::SystemClock;
            let rng = crate::services::context::SystemRng;
            let external = crate::services::context::ExternalClients::default();
            let ctx = crate::services::context::ServiceContext::new_live(&clock, &rng, &external)
                .with_actor("user");
            repair_claim_feedback_recorded_signal(&ctx, db, &feedback_id)
                .map_err(|error| error.to_string())
        })
        .await
        .map_err(|error| ClaimFileError::Db(error.to_string()))
}

fn mark_correction_apply_failed_db(
    db: &ActionDb,
    apply_key: &str,
    error_detail_hash: &str,
) -> Result<CorrectionFailureMarkState, String> {
    let now = Utc::now().to_rfc3339();
    let updated = db
        .conn_ref()
        .execute(
            "UPDATE claim_file_correction_apply_events
                SET status = 'failed',
                    error_detail_hash = ?1,
                    failed_at = ?2,
                    updated_at = ?2
              WHERE idempotency_key = ?3
                AND status = 'claimed'",
            params![error_detail_hash, &now, apply_key],
        )
        .map_err(|error| error.to_string())?;
    if updated == 1 {
        return Ok(CorrectionFailureMarkState::MarkedFailed);
    }

    let status: Option<String> = db
        .conn_ref()
        .query_row(
            "SELECT status
               FROM claim_file_correction_apply_events
              WHERE idempotency_key = ?1",
            [apply_key],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    match status.as_deref() {
        Some("applied") => Ok(CorrectionFailureMarkState::AlreadyApplied),
        Some("failed") => Ok(CorrectionFailureMarkState::MarkedFailed),
        Some("claimed") => Err("correction apply event was not marked failed".to_string()),
        Some(other) => Err(format!("unknown correction apply status `{other}`")),
        None => Err("correction apply event missing while marking failed".to_string()),
    }
}

fn correction_apply_claim_expired(claimed_at: &str, now: DateTime<Utc>) -> Result<bool, String> {
    let claimed_at = DateTime::parse_from_rfc3339(claimed_at)
        .map_err(|error| format!("invalid correction apply claimed_at: {error}"))?
        .with_timezone(&Utc);
    Ok(now.signed_duration_since(claimed_at)
        > Duration::seconds(CLAIM_FILE_CORRECTION_APPLY_LEASE_SECS))
}

fn metadata_for_action(
    action: FeedbackAction,
    payload: Option<&str>,
    claim: &ClaimFileClaim,
) -> Result<Option<serde_json::Value>, ClaimFileError> {
    let payload_value = match payload.map(str::trim).filter(|value| !value.is_empty()) {
        Some(raw) => Some(serde_json::from_str::<serde_json::Value>(raw)?),
        None => None,
    };
    let source_content_hash = claim.semantic_identity.source_content_hash.as_deref();
    if matches!(action, FeedbackAction::WrongSource) && source_content_hash.is_none() {
        return Err(ClaimFileError::BadRequest(
            "wrong_source missing source_content_hash".to_string(),
        ));
    }

    Ok(Some(with_file_projection_metadata(
        payload_value,
        source_content_hash,
    )))
}

fn with_file_projection_metadata(
    payload_value: Option<serde_json::Value>,
    source_content_hash: Option<&str>,
) -> serde_json::Value {
    let mut obj = payload_value
        .and_then(|value| value.as_object().cloned())
        .unwrap_or_default();
    obj.insert(
        "surface".to_string(),
        serde_json::Value::String("file_projection".to_string()),
    );
    obj.insert(
        "entry_point".to_string(),
        serde_json::Value::String("claim_file_projection".to_string()),
    );
    obj.insert(
        "claim_file_projection_version".to_string(),
        serde_json::json!(CLAIM_FILE_PROJECTION_VERSION),
    );
    if let Some(hash) = source_content_hash {
        obj.insert(
            "source_content_hash".to_string(),
            serde_json::Value::String(hash.to_string()),
        );
    } else {
        obj.remove("source_content_hash");
    }
    serde_json::Value::Object(obj)
}

fn feedback_action_from_slug(value: &str) -> Result<FeedbackAction, ClaimFileError> {
    match value.trim() {
        "confirm_current" => Ok(FeedbackAction::ConfirmCurrent),
        "mark_outdated" => Ok(FeedbackAction::MarkOutdated),
        "mark_false" => Ok(FeedbackAction::MarkFalse),
        "wrong_subject" => Ok(FeedbackAction::WrongSubject),
        "wrong_source" => Ok(FeedbackAction::WrongSource),
        "cannot_verify" => Ok(FeedbackAction::CannotVerify),
        "needs_nuance" => Ok(FeedbackAction::NeedsNuance),
        other => Err(ClaimFileError::BadRequest(format!(
            "unsupported file feedback action `{other}`"
        ))),
    }
}

fn receipt_target_for_claim(
    sidecar: &ClaimFileSidecar,
    claim_id: &str,
) -> Result<ReceiptTarget, ClaimFileError> {
    let claim = sidecar
        .claims
        .iter()
        .find(|claim| claim.runtime_claim_id == claim_id)
        .ok_or_else(|| ClaimFileError::BadRequest("claim not present in sidecar".to_string()))?;
    let subject = receipt_subject_ref_from_json(&claim.semantic_identity.subject_ref)?;
    Ok(ReceiptTarget::Claim {
        claim_id: claim.runtime_claim_id.clone(),
        subject,
        field_path: claim.semantic_identity.field_path.clone(),
    })
}

fn receipt_subject_ref_from_json(
    value: &serde_json::Value,
) -> Result<ReceiptSubjectRef, ClaimFileError> {
    let subject = abilities_runtime::types::subject_ref_from_json(value)
        .map_err(ClaimFileError::BadRequest)?;
    Ok(match subject {
        ClaimSubjectRef::Account { id } => ReceiptSubjectRef::Account(id),
        ClaimSubjectRef::Project { id } => ReceiptSubjectRef::Project(id),
        ClaimSubjectRef::Person { id } => ReceiptSubjectRef::Person(id),
        ClaimSubjectRef::Action { id } => ReceiptSubjectRef::Action(id),
        ClaimSubjectRef::Meeting { id } => ReceiptSubjectRef::Meeting(id),
        ClaimSubjectRef::Global => ReceiptSubjectRef::Global,
        ClaimSubjectRef::Multi(subjects) => ReceiptSubjectRef::Multi(
            subjects
                .iter()
                .map(|subject| {
                    let value = subject_ref_json_for_runtime(subject);
                    receipt_subject_ref_from_json(&value)
                })
                .collect::<Result<Vec<_>, _>>()?,
        ),
        ClaimSubjectRef::Email { .. } => {
            return Err(ClaimFileError::BadRequest(
                "email subject receipt feedback is not supported for claim files".to_string(),
            ));
        }
    })
}

fn validate_projection_relative_path(path: &Path) -> Result<PathBuf, ClaimFileError> {
    if path.is_absolute() {
        return Err(ClaimFileError::PathRejected("absolute path".to_string()));
    }
    let mut components = path.components();
    match components.next() {
        Some(Component::Normal(root)) if root == CLAIM_FILE_PROJECTION_ROOT => {}
        _ => {
            return Err(ClaimFileError::PathRejected(
                "path must be under _dailyos_claims".to_string(),
            ))
        }
    }
    for component in path.components() {
        let Component::Normal(name) = component else {
            return Err(ClaimFileError::PathRejected(
                "non-normal path component".to_string(),
            ));
        };
        let text = name.to_string_lossy();
        if text.is_empty() || text == "." || text == ".." || text.contains('\0') {
            return Err(ClaimFileError::PathRejected(
                "unsafe path component".to_string(),
            ));
        }
    }
    Ok(path.to_path_buf())
}

fn projection_abs_path(workspace_root: &Path, rel_path: &Path) -> Result<PathBuf, ClaimFileError> {
    let rel_path = validate_projection_relative_path(rel_path)?;
    let canonical_root = workspace_root
        .canonicalize()
        .map_err(|error| ClaimFileError::PathRejected(error.to_string()))?;
    let candidate = canonical_root.join(&rel_path);
    validate_existing_projection_ancestors(&canonical_root, &candidate)?;
    Ok(candidate)
}

fn sidecar_path_for_markdown_rel(path: &Path) -> Result<PathBuf, ClaimFileError> {
    if path.file_name().and_then(|name| name.to_str()) != Some(CLAIM_FILE_MARKDOWN_NAME) {
        return Err(ClaimFileError::PathRejected(
            "apply path must point to claims.md".to_string(),
        ));
    }
    let mut sidecar = path.to_path_buf();
    sidecar.set_file_name(CLAIM_FILE_SIDECAR_NAME);
    Ok(sidecar)
}

fn verify_sidecar_paths(
    sidecar: &ClaimFileSidecar,
    markdown_rel_path: &Path,
    sidecar_rel_path: &Path,
) -> Result<(), ClaimFileError> {
    if sidecar.markdown_rel_path != path_to_slash_string(markdown_rel_path) {
        return Err(ClaimFileError::BadRequest(
            "sidecar markdown path mismatch".to_string(),
        ));
    }
    if sidecar.sidecar_rel_path != path_to_slash_string(sidecar_rel_path) {
        return Err(ClaimFileError::BadRequest(
            "sidecar path mismatch".to_string(),
        ));
    }
    Ok(())
}

fn verify_sidecar_contract(sidecar: &ClaimFileSidecar) -> Result<(), ClaimFileError> {
    if !matches!(
        sidecar.schema_version,
        CLAIM_FILE_LEGACY_SIDECAR_SCHEMA_VERSION | CLAIM_FILE_SIDECAR_SCHEMA_VERSION
    ) {
        return Err(ClaimFileError::BadRequest(
            "unsupported sidecar schema_version".to_string(),
        ));
    }
    if sidecar.projection_version != CLAIM_FILE_PROJECTION_VERSION {
        return Err(ClaimFileError::BadRequest(
            "unsupported claim file projection_version".to_string(),
        ));
    }
    Ok(())
}

#[cfg(unix)]
fn write_projection_files(
    workspace_root: &Path,
    markdown_rel_path: &Path,
    sidecar_rel_path: &Path,
    markdown: &str,
    sidecar_json: &str,
) -> std::io::Result<()> {
    let markdown_rel_path = validate_projection_relative_path_for_io(markdown_rel_path)?;
    let sidecar_rel_path = validate_projection_relative_path_for_io(sidecar_rel_path)?;
    let root = open_workspace_root_dir(workspace_root)?;
    let (sidecar_parent, sidecar_name) =
        open_projection_parent_dir_at(&root, &sidecar_rel_path, true)?;
    let (markdown_parent, markdown_name) =
        open_projection_parent_dir_at(&root, &markdown_rel_path, true)?;
    atomic_write_projection_file_at(&sidecar_parent, &sidecar_name, sidecar_json.as_bytes())?;
    atomic_write_projection_file_at(&markdown_parent, &markdown_name, markdown.as_bytes())?;
    Ok(())
}

#[cfg(not(unix))]
fn atomic_write_projection_file(path: &Path, content: &str) -> std::io::Result<()> {
    let parent = path.parent().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "projection path has no parent",
        )
    })?;
    let file_name = path.file_name().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "projection path has no file name",
        )
    })?;
    let tmp_name = format!(
        ".{}.{}.tmp",
        file_name.to_string_lossy(),
        uuid::Uuid::new_v4()
    );
    let tmp_path = parent.join(tmp_name);
    let write_result = write_projection_temp_file(&tmp_path, content.as_bytes())
        .and_then(|()| fs::rename(&tmp_path, path));
    if write_result.is_err() {
        match fs::remove_file(&tmp_path) {
            Ok(()) | Err(_) => {}
        }
    }
    write_result
}

#[cfg(not(unix))]
fn write_projection_temp_file(path: &Path, content: &[u8]) -> std::io::Result<()> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
    }
    let mut file = options.open(path)?;
    file.write_all(content)?;
    file.sync_all()?;
    Ok(())
}

#[cfg(not(unix))]
fn create_projection_parent_dirs(canonical_root: &Path, file_path: &Path) -> std::io::Result<()> {
    let parent = file_path.parent().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "projection path has no parent",
        )
    })?;
    let relative_parent = parent.strip_prefix(canonical_root).map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "projection parent escapes workspace",
        )
    })?;
    let mut current = canonical_root.to_path_buf();
    for component in relative_parent.components() {
        let Component::Normal(name) = component else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "non-normal projection parent component",
            ));
        };
        current.push(name);
        match fs::symlink_metadata(&current) {
            Ok(metadata) => validate_projection_dir_metadata(canonical_root, &current, &metadata)?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                fs::create_dir(&current)?;
                let metadata = fs::symlink_metadata(&current)?;
                validate_projection_dir_metadata(canonical_root, &current, &metadata)?;
            }
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

#[cfg(not(unix))]
fn write_projection_files(
    workspace_root: &Path,
    markdown_rel_path: &Path,
    sidecar_rel_path: &Path,
    markdown: &str,
    sidecar_json: &str,
) -> std::io::Result<()> {
    let canonical_root = workspace_root.canonicalize()?;
    let markdown_path = projection_abs_path_io(workspace_root, markdown_rel_path)?;
    let sidecar_path = projection_abs_path_io(workspace_root, sidecar_rel_path)?;
    create_projection_parent_dirs(&canonical_root, &sidecar_path)?;
    create_projection_parent_dirs(&canonical_root, &markdown_path)?;
    atomic_write_projection_file(&sidecar_path, sidecar_json)?;
    atomic_write_projection_file(&markdown_path, markdown)?;
    Ok(())
}

#[cfg(unix)]
fn open_workspace_root_dir(workspace_root: &Path) -> std::io::Result<fs::File> {
    use std::os::unix::fs::MetadataExt;

    let requested_metadata = fs::symlink_metadata(workspace_root)?;
    if requested_metadata.file_type().is_symlink() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "workspace root must not be a symlink",
        ));
    }
    if !requested_metadata.is_dir() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "workspace root must be a directory",
        ));
    }
    let canonical_root = workspace_root.canonicalize()?;
    let canonical_metadata = fs::symlink_metadata(&canonical_root)?;
    let file = open_dir_path(&canonical_root)?;
    let opened_metadata = file.metadata()?;
    let requested_matches_opened = requested_metadata.dev() == opened_metadata.dev()
        && requested_metadata.ino() == opened_metadata.ino();
    let canonical_matches_opened = canonical_metadata.dev() == opened_metadata.dev()
        && canonical_metadata.ino() == opened_metadata.ino();
    if !requested_matches_opened || !canonical_matches_opened {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "workspace root changed while opening",
        ));
    }
    Ok(file)
}

#[cfg(unix)]
fn open_dir_path(path: &Path) -> std::io::Result<fs::File> {
    use std::os::fd::FromRawFd;
    use std::os::unix::ffi::OsStrExt;

    let c_path = std::ffi::CString::new(path.as_os_str().as_bytes()).map_err(|_| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "path contains nul byte")
    })?;
    let fd = unsafe {
        libc::open(
            c_path.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
        )
    };
    if fd < 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(unsafe { fs::File::from_raw_fd(fd) })
}

#[cfg(unix)]
fn duplicate_dir_fd(fd: std::os::fd::RawFd) -> std::io::Result<fs::File> {
    use std::os::fd::FromRawFd;

    let duped = unsafe { libc::fcntl(fd, libc::F_DUPFD_CLOEXEC, 0) };
    if duped < 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(unsafe { fs::File::from_raw_fd(duped) })
}

#[cfg(unix)]
fn open_projection_parent_dir_at(
    root: &fs::File,
    rel_path: &Path,
    create: bool,
) -> std::io::Result<(fs::File, OsString)> {
    use std::os::fd::{AsRawFd, FromRawFd};

    let rel_path = validate_projection_relative_path_for_io(rel_path)?;
    let file_name = rel_path.file_name().map(OsString::from).ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "projection path has no file name",
        )
    })?;
    let parent = rel_path.parent().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "projection path has no parent",
        )
    })?;
    let mut dir = duplicate_dir_fd(root.as_raw_fd())?;
    for component in parent.components() {
        let Component::Normal(name) = component else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "non-normal projection parent component",
            ));
        };
        let c_name = cstring_from_os(name)?;
        let mut fd = unsafe {
            libc::openat(
                dir.as_raw_fd(),
                c_name.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            )
        };
        if fd < 0 {
            let open_error = std::io::Error::last_os_error();
            if create && open_error.kind() == std::io::ErrorKind::NotFound {
                let mkdir_result =
                    unsafe { libc::mkdirat(dir.as_raw_fd(), c_name.as_ptr(), 0o700) };
                if mkdir_result < 0 {
                    let mkdir_error = std::io::Error::last_os_error();
                    if mkdir_error.kind() != std::io::ErrorKind::AlreadyExists {
                        return Err(mkdir_error);
                    }
                }
                fd = unsafe {
                    libc::openat(
                        dir.as_raw_fd(),
                        c_name.as_ptr(),
                        libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                    )
                };
            }
        }
        if fd < 0 {
            return Err(map_no_follow_io_error(std::io::Error::last_os_error()));
        }
        let next = unsafe { fs::File::from_raw_fd(fd) };
        let metadata = next.metadata()?;
        if !metadata.is_dir() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "projection ancestor is not a directory",
            ));
        }
        dir = next;
    }
    Ok((dir, file_name))
}

#[cfg(unix)]
fn atomic_write_projection_file_at(
    parent: &fs::File,
    file_name: &std::ffi::OsStr,
    content: &[u8],
) -> std::io::Result<()> {
    use std::os::fd::{AsRawFd, FromRawFd};

    let final_name = cstring_from_os(file_name)?;
    let tmp_name = OsString::from(format!(
        ".{}.{}.tmp",
        file_name.to_string_lossy(),
        uuid::Uuid::new_v4()
    ));
    let tmp_name = cstring_from_os(&tmp_name)?;
    let write_result = (|| {
        let fd = unsafe {
            libc::openat(
                parent.as_raw_fd(),
                tmp_name.as_ptr(),
                libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                0o600,
            )
        };
        if fd < 0 {
            return Err(map_no_follow_io_error(std::io::Error::last_os_error()));
        }
        let mut file = unsafe { fs::File::from_raw_fd(fd) };
        file.write_all(content)?;
        file.sync_all()?;
        drop(file);
        let renamed = unsafe {
            libc::renameat(
                parent.as_raw_fd(),
                tmp_name.as_ptr(),
                parent.as_raw_fd(),
                final_name.as_ptr(),
            )
        };
        if renamed < 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(())
    })();
    if write_result.is_err() {
        let _ = unsafe { libc::unlinkat(parent.as_raw_fd(), tmp_name.as_ptr(), 0) };
    }
    write_result
}

#[cfg(unix)]
fn cstring_from_os(value: &std::ffi::OsStr) -> std::io::Result<std::ffi::CString> {
    use std::os::unix::ffi::OsStrExt;

    std::ffi::CString::new(value.as_bytes()).map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "projection path contains nul byte",
        )
    })
}

#[cfg(unix)]
fn map_no_follow_io_error(error: std::io::Error) -> std::io::Error {
    if error.raw_os_error() == Some(libc::ELOOP) {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "projection path contains symlink",
        )
    } else {
        error
    }
}

fn validate_projection_relative_path_for_io(path: &Path) -> std::io::Result<PathBuf> {
    validate_projection_relative_path(path)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidInput, error.to_string()))
}

#[cfg(not(unix))]
fn projection_abs_path_io(workspace_root: &Path, rel_path: &Path) -> std::io::Result<PathBuf> {
    projection_abs_path(workspace_root, rel_path)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidInput, error.to_string()))
}

fn validate_existing_projection_ancestors(
    canonical_root: &Path,
    file_path: &Path,
) -> Result<(), ClaimFileError> {
    let Some(parent) = file_path.parent() else {
        return Err(ClaimFileError::PathRejected(
            "projection path has no parent".to_string(),
        ));
    };
    let relative_parent = parent.strip_prefix(canonical_root).map_err(|_| {
        ClaimFileError::PathRejected("projection parent escapes workspace".to_string())
    })?;
    let mut current = canonical_root.to_path_buf();
    for component in relative_parent.components() {
        let Component::Normal(name) = component else {
            return Err(ClaimFileError::PathRejected(
                "non-normal projection parent component".to_string(),
            ));
        };
        current.push(name);
        match fs::symlink_metadata(&current) {
            Ok(metadata) => validate_projection_dir_metadata(canonical_root, &current, &metadata)
                .map_err(|error| ClaimFileError::PathRejected(error.to_string()))?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(ClaimFileError::PathRejected(error.to_string())),
        }
    }
    Ok(())
}

fn validate_projection_dir_metadata(
    canonical_root: &Path,
    path: &Path,
    metadata: &fs::Metadata,
) -> std::io::Result<()> {
    if metadata.file_type().is_symlink() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "projection path contains symlink",
        ));
    }
    if !metadata.is_dir() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "projection ancestor is not a directory",
        ));
    }
    let canonical_path = path.canonicalize()?;
    if !canonical_path.starts_with(canonical_root) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "projection parent escapes workspace",
        ));
    }
    Ok(())
}

fn read_stable_to_string(workspace_root: &Path, rel_path: &Path) -> Result<String, ClaimFileError> {
    let rel_path = validate_projection_relative_path(rel_path)?;
    let (mut file, before) = open_projection_file_no_follow(workspace_root, &rel_path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    let after = file.metadata()?;
    validate_projection_file_metadata(&after)?;
    if !same_open_file_metadata(&before, &after)
        || before.len() != after.len()
        || before.modified().ok() != after.modified().ok()
    {
        return Err(ClaimFileError::PathRejected(
            "file changed while reading; retry apply".to_string(),
        ));
    }
    Ok(content)
}

#[cfg(unix)]
fn open_projection_file_no_follow(
    workspace_root: &Path,
    rel_path: &Path,
) -> Result<(fs::File, fs::Metadata), ClaimFileError> {
    use std::os::fd::{AsRawFd, FromRawFd};

    let root = open_workspace_root_dir(workspace_root).map_err(ClaimFileError::Io)?;
    let (parent, file_name) =
        open_projection_parent_dir_at(&root, rel_path, false).map_err(|error| {
            if error.raw_os_error() == Some(libc::ELOOP)
                || error.kind() == std::io::ErrorKind::InvalidInput
            {
                ClaimFileError::PathRejected(error.to_string())
            } else {
                ClaimFileError::Io(error)
            }
        })?;
    let file_name = cstring_from_os(&file_name).map_err(ClaimFileError::Io)?;
    let fd = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            file_name.as_ptr(),
            libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    if fd < 0 {
        let error = std::io::Error::last_os_error();
        return Err(if error.raw_os_error() == Some(libc::ELOOP) {
            ClaimFileError::PathRejected("projection file is a symlink".to_string())
        } else {
            ClaimFileError::Io(error)
        });
    }
    let file = unsafe { fs::File::from_raw_fd(fd) };
    let metadata = file.metadata()?;
    validate_projection_file_metadata(&metadata)?;
    Ok((file, metadata))
}

#[cfg(not(unix))]
fn open_projection_file_no_follow(
    workspace_root: &Path,
    rel_path: &Path,
) -> Result<(fs::File, fs::Metadata), ClaimFileError> {
    let path = projection_abs_path(workspace_root, rel_path)?;
    let before = fs::symlink_metadata(path)?;
    validate_projection_file_metadata(&before)?;
    let file = fs::File::open(path)?;
    let metadata = file.metadata()?;
    validate_projection_file_metadata(&metadata)?;
    Ok((file, metadata))
}

fn validate_projection_file_metadata(metadata: &fs::Metadata) -> Result<(), ClaimFileError> {
    if metadata.file_type().is_symlink() {
        return Err(ClaimFileError::PathRejected(
            "projection file is a symlink".to_string(),
        ));
    }
    if !metadata.is_file() {
        return Err(ClaimFileError::PathRejected(
            "projection path is not a file".to_string(),
        ));
    }
    if projection_file_has_multiple_links(metadata) {
        return Err(ClaimFileError::PathRejected(
            "projection file has multiple hard links".to_string(),
        ));
    }
    Ok(())
}

#[cfg(unix)]
fn same_open_file_metadata(before: &fs::Metadata, after: &fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;

    before.dev() == after.dev() && before.ino() == after.ino()
}

#[cfg(not(unix))]
fn same_open_file_metadata(_before: &fs::Metadata, _after: &fs::Metadata) -> bool {
    true
}

#[cfg(unix)]
fn projection_file_has_multiple_links(metadata: &fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;

    metadata.nlink() > 1
}

#[cfg(not(unix))]
fn projection_file_has_multiple_links(_metadata: &fs::Metadata) -> bool {
    false
}

fn current_entity_claim_version(db: &ActionDb, subject: &ClaimSubjectRef) -> Result<i64, String> {
    let Some(kind) = subject_kind_slug(subject) else {
        return Ok(0);
    };
    let Some(id) = subject_id_for_path(subject) else {
        return Ok(0);
    };
    db.current_subject_claim_version(kind, id)
        .map_err(|error| error.to_string())
}

fn claim_watermark(claims: &[IntelligenceClaim], entity_claim_version: i64) -> String {
    let mut entries = claims
        .iter()
        .map(|claim| format!("{}:{}", claim.id, claim.claim_version))
        .collect::<Vec<_>>();
    entries.sort();
    entries.push(format!("entity_claim_version:{entity_claim_version}"));
    sha256_hex(entries.join("\n").as_bytes())
}

fn canonical_subject_compact(subject: &ClaimSubjectRef) -> Result<String, String> {
    let subject = invalidation_subject_from_claim_subject(subject)?;
    crate::services::claims::canonical_subject_ref(&subject).map_err(|error| error.to_string())
}

fn invalidation_subject_from_claim_subject(
    subject: &ClaimSubjectRef,
) -> Result<InvalidationSubjectRef, String> {
    Ok(match subject {
        ClaimSubjectRef::Account { id } => InvalidationSubjectRef::Account { id: id.clone() },
        ClaimSubjectRef::Project { id } => InvalidationSubjectRef::Project { id: id.clone() },
        ClaimSubjectRef::Person { id } => InvalidationSubjectRef::Person { id: id.clone() },
        ClaimSubjectRef::Action { id } => InvalidationSubjectRef::Action { id: id.clone() },
        ClaimSubjectRef::Meeting { id } => InvalidationSubjectRef::Meeting { id: id.clone() },
        ClaimSubjectRef::Email { id } => InvalidationSubjectRef::Email { id: id.clone() },
        ClaimSubjectRef::Global => InvalidationSubjectRef::Global,
        ClaimSubjectRef::Multi(subjects) => InvalidationSubjectRef::Multi(
            subjects
                .iter()
                .map(invalidation_subject_from_claim_subject)
                .collect::<Result<Vec<_>, _>>()?,
        ),
    })
}

fn subject_kind_slug(subject: &ClaimSubjectRef) -> Option<&'static str> {
    match subject {
        ClaimSubjectRef::Account { .. } => Some("account"),
        ClaimSubjectRef::Project { .. } => Some("project"),
        ClaimSubjectRef::Person { .. } => Some("person"),
        ClaimSubjectRef::Action { .. } => Some("action"),
        ClaimSubjectRef::Meeting { .. } => Some("meeting"),
        ClaimSubjectRef::Email { .. } => Some("email"),
        ClaimSubjectRef::Global | ClaimSubjectRef::Multi(_) => None,
    }
}

fn supported_entity_kind_slug(subject: &ClaimSubjectRef) -> Option<&'static str> {
    match subject {
        ClaimSubjectRef::Account { .. } => Some("account"),
        ClaimSubjectRef::Project { .. } => Some("project"),
        ClaimSubjectRef::Person { .. } => Some("person"),
        _ => None,
    }
}

fn subject_id_for_path(subject: &ClaimSubjectRef) -> Option<&str> {
    match subject {
        ClaimSubjectRef::Account { id }
        | ClaimSubjectRef::Project { id }
        | ClaimSubjectRef::Person { id }
        | ClaimSubjectRef::Action { id }
        | ClaimSubjectRef::Meeting { id }
        | ClaimSubjectRef::Email { id } => Some(id.as_str()),
        ClaimSubjectRef::Global | ClaimSubjectRef::Multi(_) => None,
    }
}

fn subject_ref_json_for_runtime(subject: &ClaimSubjectRef) -> serde_json::Value {
    match subject {
        ClaimSubjectRef::Account { id } => serde_json::json!({"kind": "account", "id": id}),
        ClaimSubjectRef::Project { id } => serde_json::json!({"kind": "project", "id": id}),
        ClaimSubjectRef::Person { id } => serde_json::json!({"kind": "person", "id": id}),
        ClaimSubjectRef::Action { id } => serde_json::json!({"kind": "action", "id": id}),
        ClaimSubjectRef::Meeting { id } => serde_json::json!({"kind": "meeting", "id": id}),
        ClaimSubjectRef::Email { id } => serde_json::json!({"kind": "email", "id": id}),
        ClaimSubjectRef::Global => serde_json::json!({"kind": "global"}),
        ClaimSubjectRef::Multi(subjects) => serde_json::json!({
            "kind": "multi",
            "subjects": subjects.iter().map(subject_ref_json_for_runtime).collect::<Vec<_>>()
        }),
    }
}

fn slug_segment(raw: &str) -> String {
    let mut out = String::new();
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if ch == '-' || ch == '_' || ch == ' ' || ch == '.' {
            out.push('-');
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "entity".to_string()
    } else {
        trimmed
    }
}

fn projection_entity_slug(raw_id: &str, entity_subject_compact: &str) -> String {
    let display_slug = slug_segment(raw_id);
    let identity_hash = sha256_hex(entity_subject_compact.as_bytes());
    format!("{}-{}", display_slug, &identity_hash[..12])
}

fn trust_band_label(score: Option<f64>) -> &'static str {
    match score {
        Some(score) if score >= 0.75 => "likely_current",
        Some(score) if score >= 0.45 => "use_with_caution",
        _ => "needs_verification",
    }
}

fn sensitivity_label(sensitivity: &ClaimSensitivity) -> String {
    serde_json::to_value(sensitivity)
        .ok()
        .and_then(|value| value.as_str().map(ToString::to_string))
        .unwrap_or_else(|| format!("{sensitivity:?}").to_ascii_lowercase())
}

pub(crate) fn source_content_hash_for_claim(claim: &IntelligenceClaim) -> String {
    let mut hasher = Sha256::new();
    hasher.update(claim.data_source.as_bytes());
    hasher.update([0x1f]);
    if let Some(source_ref) = claim.source_ref.as_deref() {
        hasher.update(source_ref.as_bytes());
    }
    hasher.update([0x1f]);
    if let Some(item_hash) = claim.item_hash.as_deref() {
        hasher.update(item_hash.as_bytes());
    }
    format!("{:x}", hasher.finalize())
}

fn markdown_inline_text(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut previous_was_space = false;
    for ch in value.chars() {
        let next = if ch.is_control() { ' ' } else { ch };
        if next.is_whitespace() {
            if !previous_was_space {
                out.push(' ');
                previous_was_space = true;
            }
        } else {
            out.push(next);
            previous_was_space = false;
        }
    }
    out.trim().to_string()
}

fn path_to_slash_string(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn redacted_hash(value: &str) -> String {
    sha256_hex(value.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_utils::test_db;
    use crate::services::claims::{
        commit_claim, record_claim_feedback, record_claim_feedback_for_claim_file_apply,
        ClaimError, ClaimFeedbackInput, ClaimFileFeedbackApplyInput, ClaimProposal, CommittedClaim,
        TombstoneSpec,
    };
    use crate::services::context::{ExternalClients, FixedClock, SeedableRng, ServiceContext};
    use abilities_runtime::sensitivity::ClaimVerificationState;
    use abilities_runtime::types::{ClaimState, SurfacingState, TemporalScope};

    const TEST_TS: &str = "2026-06-02T12:00:00Z";

    fn fixture_sidecar() -> ClaimFileSidecar {
        let subject = serde_json::json!({"kind": "account", "id": "acct-1"});
        ClaimFileSidecar {
            schema_version: CLAIM_FILE_SIDECAR_SCHEMA_VERSION,
            projection_version: CLAIM_FILE_PROJECTION_VERSION,
            entity_subject_ref: subject.clone(),
            entity_subject_compact: r#"{"id":"acct-1","kind":"account"}"#.to_string(),
            markdown_rel_path: "_dailyos_claims/account/acct-1/claims.md".to_string(),
            sidecar_rel_path: "_dailyos_claims/account/acct-1/claims.corrections.json".to_string(),
            claims: vec![ClaimFileClaim {
                semantic_identity: ClaimSemanticIdentityV1 {
                    identity_version: 1,
                    identity_kind: "claim_dedup_v1".to_string(),
                    item_hash: "item-hash-1".to_string(),
                    subject_ref_compact: r#"{"id":"acct-1","kind":"account"}"#.to_string(),
                    subject_ref: subject,
                    claim_type: "risk".to_string(),
                    field_path: Some("health.risk".to_string()),
                    dedup_key_components_hash: "identity-hash-1".to_string(),
                    source_ref: Some("fixture://source-1".to_string()),
                    data_source: "unit_test".to_string(),
                    actor: "agent:test".to_string(),
                    observed_at: "2026-06-02T12:00:00Z".to_string(),
                    source_asof: Some("2026-06-02T12:00:00Z".to_string()),
                    source_content_hash: Some("abcdef0123456789".to_string()),
                    runtime_claim_id: "claim-1".to_string(),
                    runtime_claim_version: 3,
                    claim_state: "active".to_string(),
                    superseded_by: None,
                },
                runtime_claim_id: "claim-1".to_string(),
                runtime_claim_version: 3,
                claim_text: "Renewal risk is elevated".to_string(),
                trust_band: "likely_current".to_string(),
                sensitivity: "internal".to_string(),
                lifecycle: ClaimFileLifecycle {
                    claim_state: "active".to_string(),
                    surfacing_state: "active".to_string(),
                    demotion_reason: None,
                    retraction_reason: None,
                    superseded_by: None,
                    verification_state: "active".to_string(),
                    verification_reason: None,
                    needs_user_decision_at: None,
                },
                provenance_summary: ClaimFileProvenanceSummary {
                    data_source: "unit_test".to_string(),
                    source_ref: Some("fixture://source-1".to_string()),
                    source_asof: Some("2026-06-02T12:00:00Z".to_string()),
                    observed_at: "2026-06-02T12:00:00Z".to_string(),
                },
                feedback_rows: Vec::new(),
                contradiction_edges: Vec::new(),
                superseded_by_semantic_identity: None,
                replay_status: projected_replay_status(),
            }],
        }
    }

    fn fixture_intelligence_claim(
        claim_type: &str,
        field_path: Option<&str>,
        item_hash: Option<&str>,
    ) -> IntelligenceClaim {
        IntelligenceClaim {
            id: "claim-identity-1".to_string(),
            claim_version: 7,
            subject_ref: serde_json::json!({"kind": "account", "id": "acct-1"}).to_string(),
            claim_type: claim_type.to_string(),
            field_path: field_path.map(ToString::to_string),
            topic_key: None,
            text: "Identity fixture claim".to_string(),
            dedup_key: "dedup-fixture".to_string(),
            item_hash: item_hash.map(ToString::to_string),
            actor: "agent:test".to_string(),
            data_source: "unit_test".to_string(),
            source_ref: Some("fixture://source-identity".to_string()),
            source_asof: Some("2026-06-02T12:00:00Z".to_string()),
            observed_at: "2026-06-02T12:00:00Z".to_string(),
            created_at: "2026-06-02T12:00:00Z".to_string(),
            provenance_json: "{}".to_string(),
            metadata_json: None,
            claim_state: ClaimState::Active,
            surfacing_state: SurfacingState::Active,
            demotion_reason: None,
            reactivated_at: None,
            retraction_reason: None,
            expires_at: None,
            superseded_by: None,
            trust_score: Some(0.8),
            trust_computed_at: None,
            trust_version: None,
            thread_id: None,
            temporal_scope: TemporalScope::State,
            sensitivity: ClaimSensitivity::Internal,
            verification_state: ClaimVerificationState::Active,
            verification_reason: None,
            needs_user_decision_at: None,
        }
    }

    fn test_context_parts() -> (FixedClock, SeedableRng, ExternalClients) {
        let now = chrono::DateTime::parse_from_rfc3339(TEST_TS)
            .expect("test timestamp")
            .with_timezone(&Utc);
        (
            FixedClock::new(now),
            SeedableRng::new(7),
            ExternalClients::default(),
        )
    }

    fn test_live_context<'a>(
        clock: &'a FixedClock,
        rng: &'a SeedableRng,
        external: &'a ExternalClients,
    ) -> ServiceContext<'a> {
        ServiceContext::test_live(clock, rng, external).with_actor("user:test")
    }

    fn fixture_claim_proposal(text: &str) -> ClaimProposal {
        ClaimProposal {
            id: None,
            expected_claim_version: None,
            subject_ref: r#"{"kind":"account","id":"acct-1"}"#.to_string(),
            claim_type: "risk".to_string(),
            field_path: Some("health.risk".to_string()),
            topic_key: None,
            text: text.to_string(),
            actor: "agent:test".to_string(),
            data_source: "unit_test".to_string(),
            source_ref: Some("fixture://source".to_string()),
            source_asof: Some(TEST_TS.to_string()),
            observed_at: TEST_TS.to_string(),
            provenance_json: "{}".to_string(),
            metadata_json: None,
            thread_id: None,
            temporal_scope: Some(TemporalScope::State),
            sensitivity: Some(ClaimSensitivity::Internal),
            supersedes: None,
            tombstone: None,
        }
    }

    fn inserted_claim_id(result: CommittedClaim) -> String {
        match result {
            CommittedClaim::Inserted { claim } | CommittedClaim::Tombstoned { claim } => claim.id,
            other => panic!("expected inserted/tombstoned claim, got {other:?}"),
        }
    }

    fn claim_feedback_recorded_signal_id_for_test(feedback_id: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(format!("claim-feedback:{feedback_id}:recorded").as_bytes());
        format!("sig-once-{:x}", hasher.finalize())
    }

    #[test]
    fn sidecar_serialization_round_trips_contract_fields() {
        let mut sidecar = fixture_sidecar();
        sidecar.claims[0].feedback_rows.push(ClaimFileFeedbackRow {
            feedback_id: "feedback-1".to_string(),
            action: "cannot_verify".to_string(),
            actor: "user".to_string(),
            actor_id: Some("user-fixture".to_string()),
            payload_json: None,
            submitted_at: TEST_TS.to_string(),
            applied_at: None,
        });
        let json = serde_json::to_string_pretty(&sidecar).expect("serialize sidecar");
        let decoded: ClaimFileSidecar = serde_json::from_str(&json).expect("decode sidecar");

        assert_eq!(decoded.schema_version, CLAIM_FILE_SIDECAR_SCHEMA_VERSION);
        assert_eq!(CLAIM_FILE_SIDECAR_SCHEMA_VERSION, 2);
        assert_eq!(decoded.projection_version, CLAIM_FILE_PROJECTION_VERSION);
        assert_eq!(
            decoded.claims[0].semantic_identity.identity_kind,
            "claim_dedup_v1"
        );
        assert_eq!(decoded.claims[0].runtime_claim_id, "claim-1");
        assert_eq!(decoded.claims[0].lifecycle.claim_state, "active");
        assert_eq!(decoded.claims[0].replay_status.status, "projected");
        assert_eq!(decoded.claims[0].feedback_rows[0].feedback_id, "feedback-1");
        assert!(json.contains("\"feedbackId\""));
        assert!(decoded.claims[0].contradiction_edges.is_empty());
    }

    #[test]
    fn legacy_v1_sidecar_deserializes_without_feedback_id() {
        let mut sidecar = fixture_sidecar();
        sidecar.schema_version = CLAIM_FILE_LEGACY_SIDECAR_SCHEMA_VERSION;
        sidecar.claims[0].feedback_rows.push(ClaimFileFeedbackRow {
            feedback_id: String::new(),
            action: "cannot_verify".to_string(),
            actor: "user".to_string(),
            actor_id: Some("user-fixture".to_string()),
            payload_json: None,
            submitted_at: TEST_TS.to_string(),
            applied_at: None,
        });
        let mut json = serde_json::to_value(&sidecar).expect("serialize sidecar");
        let feedback = json
            .pointer_mut("/claims/0/feedbackRows/0")
            .and_then(serde_json::Value::as_object_mut)
            .expect("feedback object");
        feedback.remove("feedbackId");
        let decoded: ClaimFileSidecar =
            serde_json::from_value(json).expect("decode legacy sidecar");

        assert_eq!(
            decoded.schema_version,
            CLAIM_FILE_LEGACY_SIDECAR_SCHEMA_VERSION
        );
        assert_eq!(decoded.claims[0].feedback_rows[0].feedback_id, "");
        verify_sidecar_contract(&decoded).expect("v1 sidecar remains readable");
    }

    #[test]
    fn sidecar_contract_rejects_unsupported_versions() {
        let mut sidecar = fixture_sidecar();
        sidecar.schema_version = CLAIM_FILE_SIDECAR_SCHEMA_VERSION + 1;
        let err = verify_sidecar_contract(&sidecar).unwrap_err();
        assert!(err.to_string().contains("sidecar schema_version"));

        let mut sidecar = fixture_sidecar();
        sidecar.projection_version = CLAIM_FILE_PROJECTION_VERSION + 1;
        let err = verify_sidecar_contract(&sidecar).unwrap_err();
        assert!(err.to_string().contains("projection_version"));
    }

    #[test]
    fn parser_ignores_arbitrary_prose_without_structured_claim_blocks() {
        let sidecar = fixture_sidecar();
        let corrections = parse_markdown_corrections(
            "This prose mentions claim-1 and says it is false, but it is not a bounded edit block.",
            &sidecar,
            "checksum",
        )
        .expect("parse arbitrary prose");

        assert!(corrections.is_empty());
    }

    #[test]
    fn parser_maps_structured_wrong_source_to_feedback_metadata() {
        let sidecar = fixture_sidecar();
        let markdown = "\
<!-- dailyos-claim-start -->
dailyos-claim-id: claim-1
dailyos-claim-version: 3
dailyos-identity-hash: identity-hash-1
dailyos-sidecar-checksum: checksum-1
dailyos-action: wrong_source
dailyos-payload: {}
<!-- dailyos-claim-end -->
";

        let corrections =
            parse_markdown_corrections(markdown, &sidecar, "checksum-1").expect("parse");

        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].action, FeedbackAction::WrongSource);
        assert_eq!(corrections[0].projected_claim_version, 3);
        assert_eq!(corrections[0].projected_identity_hash, "identity-hash-1");
        assert_eq!(
            corrections[0]
                .metadata
                .as_ref()
                .and_then(|metadata| metadata.get("source_content_hash")),
            Some(&serde_json::json!("abcdef0123456789"))
        );
        assert_eq!(
            corrections[0]
                .metadata
                .as_ref()
                .and_then(|metadata| metadata.get("surface")),
            Some(&serde_json::json!("file_projection"))
        );
        assert_eq!(
            corrections[0]
                .metadata
                .as_ref()
                .and_then(|metadata| metadata.get("entry_point")),
            Some(&serde_json::json!("claim_file_projection"))
        );
    }

    #[test]
    fn parser_overwrites_forged_source_hash_for_file_projection_feedback() {
        let sidecar = fixture_sidecar();
        for action in ["confirm_current", "mark_false"] {
            let markdown = format!(
                "\
<!-- dailyos-claim-start -->
dailyos-claim-id: claim-1
dailyos-claim-version: 3
dailyos-identity-hash: identity-hash-1
dailyos-sidecar-checksum: checksum-1
dailyos-action: {action}
dailyos-payload: {{\"source_content_hash\":\"forged\",\"surface\":\"spoofed\",\"entry_point\":\"spoofed\"}}
<!-- dailyos-claim-end -->
"
            );

            let corrections =
                parse_markdown_corrections(&markdown, &sidecar, "checksum-1").expect("parse");

            assert_eq!(corrections.len(), 1);
            let metadata = corrections[0].metadata.as_ref().expect("metadata");
            assert_eq!(
                metadata.get("source_content_hash"),
                Some(&serde_json::json!("abcdef0123456789")),
                "{action} must use the sidecar's canonical source hash"
            );
            assert_eq!(
                metadata.get("surface"),
                Some(&serde_json::json!("file_projection")),
                "{action} must not preserve forged surface metadata"
            );
            assert_eq!(
                metadata.get("entry_point"),
                Some(&serde_json::json!("claim_file_projection")),
                "{action} must not preserve forged entry point metadata"
            );
        }
    }

    #[test]
    fn parser_drops_payload_source_hash_when_sidecar_has_no_canonical_hash() {
        let mut sidecar = fixture_sidecar();
        sidecar.claims[0].semantic_identity.source_content_hash = None;
        let markdown = "\
<!-- dailyos-claim-start -->
dailyos-claim-id: claim-1
dailyos-claim-version: 3
dailyos-identity-hash: identity-hash-1
dailyos-sidecar-checksum: checksum-1
dailyos-action: confirm_current
dailyos-payload: {\"source_content_hash\":\"forged\"}
<!-- dailyos-claim-end -->
";

        let corrections =
            parse_markdown_corrections(markdown, &sidecar, "checksum-1").expect("parse");

        let metadata = corrections[0].metadata.as_ref().expect("metadata");
        assert!(
            metadata.get("source_content_hash").is_none(),
            "file projection must not let payload author source hash authority"
        );
    }

    #[test]
    fn parser_maps_supported_structured_actions() {
        let sidecar = fixture_sidecar();
        for (slug, expected, payload) in [
            ("confirm_current", FeedbackAction::ConfirmCurrent, "{}"),
            ("mark_outdated", FeedbackAction::MarkOutdated, "{}"),
            ("mark_false", FeedbackAction::MarkFalse, "{}"),
            ("cannot_verify", FeedbackAction::CannotVerify, "{}"),
            (
                "wrong_subject",
                FeedbackAction::WrongSubject,
                "{\"corrected_subject_ref\":{\"kind\":\"project\",\"id\":\"proj-2\"}}",
            ),
            (
                "needs_nuance",
                FeedbackAction::NeedsNuance,
                "{\"corrected_text\":\"Needs account-specific nuance.\"}",
            ),
        ] {
            let markdown = format!(
                "<!-- dailyos-claim-start -->\n\
                 dailyos-claim-id: claim-1\n\
                 dailyos-claim-version: 3\n\
                 dailyos-identity-hash: identity-hash-1\n\
                 dailyos-sidecar-checksum: checksum-1\n\
                 dailyos-action: {slug}\n\
                 dailyos-payload: {payload}\n\
                 <!-- dailyos-claim-end -->\n"
            );

            let corrections =
                parse_markdown_corrections(&markdown, &sidecar, "checksum-1").expect("parse");

            assert_eq!(corrections.len(), 1, "{slug} should map one correction");
            assert_eq!(corrections[0].action, expected, "{slug} action maps");
            assert_eq!(
                corrections[0]
                    .metadata
                    .as_ref()
                    .and_then(|metadata| metadata.get("surface")),
                Some(&serde_json::json!("file_projection")),
                "{slug} should carry file projection surface metadata"
            );
        }
    }

    #[test]
    fn parser_rejects_stale_claim_version_and_identity_hash() {
        let sidecar = fixture_sidecar();
        let stale_version = "\
<!-- dailyos-claim-start -->
dailyos-claim-id: claim-1
dailyos-claim-version: 2
dailyos-identity-hash: identity-hash-1
dailyos-sidecar-checksum: checksum-1
dailyos-action: mark_false
dailyos-payload: {}
<!-- dailyos-claim-end -->
";
        let err = parse_markdown_corrections(stale_version, &sidecar, "checksum-1").unwrap_err();
        assert!(err.to_string().contains("stale_projection_claim_version"));

        let stale_identity = "\
<!-- dailyos-claim-start -->
dailyos-claim-id: claim-1
dailyos-claim-version: 3
dailyos-identity-hash: stale-identity
dailyos-sidecar-checksum: checksum-1
dailyos-action: mark_false
dailyos-payload: {}
<!-- dailyos-claim-end -->
";
        let err = parse_markdown_corrections(stale_identity, &sidecar, "checksum-1").unwrap_err();
        assert!(err.to_string().contains("stale_projection_identity_hash"));
    }

    #[test]
    fn parser_rejects_ambiguous_unterminated_claim_block() {
        let sidecar = fixture_sidecar();
        let markdown = "\
<!-- dailyos-claim-start -->
dailyos-claim-id: claim-1
dailyos-claim-version: 3
dailyos-identity-hash: identity-hash-1
dailyos-sidecar-checksum: checksum-1
dailyos-action: mark_false
dailyos-payload: {}
";

        let err = parse_markdown_corrections(markdown, &sidecar, "checksum-1").unwrap_err();

        assert!(err.to_string().contains("unterminated claim block"));
    }

    #[test]
    fn parser_rejects_sidecar_checksum_mismatch() {
        let sidecar = fixture_sidecar();
        let markdown = "\
<!-- dailyos-claim-start -->
dailyos-claim-id: claim-1
dailyos-claim-version: 3
dailyos-identity-hash: identity-hash-1
dailyos-sidecar-checksum: stale
dailyos-action: mark_false
dailyos-payload: {}
<!-- dailyos-claim-end -->
";

        let err = parse_markdown_corrections(markdown, &sidecar, "current").unwrap_err();

        assert!(err.to_string().contains("sidecar_mismatch"));
    }

    #[test]
    fn apply_parse_failure_reports_sidecar_mismatch_artifact_without_responses() {
        let sidecar = fixture_sidecar();
        let markdown = "\
<!-- dailyos-claim-start -->
dailyos-claim-id: claim-1
dailyos-claim-version: 3
dailyos-identity-hash: identity-hash-1
dailyos-sidecar-checksum: stale
dailyos-action: mark_false
dailyos-payload: {}
<!-- dailyos-claim-end -->
";
        let error = parse_markdown_corrections(markdown, &sidecar, "current")
            .expect_err("sidecar mismatch should fail parsing");
        let result = parse_failure_apply_result(sidecar.claims.len(), error);

        assert_eq!(result.applied_count, 0);
        assert_eq!(result.skipped_count, sidecar.claims.len());
        assert!(result.responses.is_empty());
        assert!(result.rerender.is_none());
        assert_eq!(result.failures.len(), 1);
        assert_eq!(result.failures[0].claim_id, None);
        assert_eq!(result.failures[0].error_class, "sidecar_mismatch");
    }

    #[test]
    fn apply_parse_failure_reports_unsupported_action_artifact_without_responses() {
        let sidecar = fixture_sidecar();
        let markdown = "\
<!-- dailyos-claim-start -->
dailyos-claim-id: claim-1
dailyos-claim-version: 3
dailyos-identity-hash: identity-hash-1
dailyos-sidecar-checksum: checksum-1
dailyos-action: rewrite_claim
dailyos-payload: {}
<!-- dailyos-claim-end -->
";
        let error = parse_markdown_corrections(markdown, &sidecar, "checksum-1")
            .expect_err("unsupported action should fail parsing");
        let result = parse_failure_apply_result(sidecar.claims.len(), error);

        assert_eq!(result.applied_count, 0);
        assert_eq!(result.skipped_count, sidecar.claims.len());
        assert!(result.responses.is_empty());
        assert!(result.rerender.is_none());
        assert_eq!(result.failures.len(), 1);
        assert_eq!(result.failures[0].claim_id, None);
        assert_eq!(result.failures[0].error_class, "unsupported_action");
    }

    #[test]
    fn parser_ignores_directive_like_text_after_claim_block_header() {
        let sidecar = fixture_sidecar();
        let markdown = "\
<!-- dailyos-claim-start -->
dailyos-claim-id: claim-1
dailyos-claim-version: 3
dailyos-identity-hash: identity-hash-1
dailyos-sidecar-checksum: checksum-1
dailyos-action: none
dailyos-payload: {}

## Source claim text
dailyos-action: mark_false
dailyos-payload: {}
<!-- dailyos-claim-end -->
";

        let corrections =
            parse_markdown_corrections(markdown, &sidecar, "checksum-1").expect("parse");

        assert!(
            corrections.is_empty(),
            "rendered claim prose must not be parsed as correction directives"
        );
    }

    #[test]
    fn render_markdown_keeps_claim_text_outside_machine_block() {
        let mut sidecar = fixture_sidecar();
        sidecar.claims[0].claim_text = "Claim text\n\
dailyos-action: mark_false\n\
<!-- dailyos-claim-end -->"
            .to_string();

        let markdown = render_markdown(&sidecar, "checksum-1");
        let corrections =
            parse_markdown_corrections(&markdown, &sidecar, "checksum-1").expect("parse");

        assert!(markdown.contains("## Claim text dailyos-action: mark_false"));
        assert!(corrections.is_empty());
    }

    #[test]
    fn projection_path_validator_rejects_non_managed_roots_and_traversal() {
        assert!(validate_projection_relative_path(Path::new(
            "_dailyos_claims/account/acct-1/claims.md"
        ))
        .is_ok());
        assert!(validate_projection_relative_path(Path::new("Accounts/acct-1/claims.md")).is_err());
        assert!(validate_projection_relative_path(Path::new(
            "_dailyos_claims/../Accounts/acct-1.md"
        ))
        .is_err());
    }

    #[test]
    fn projection_entity_slug_disambiguates_colliding_display_slugs() {
        assert_eq!(slug_segment("acct.example"), slug_segment("acct-example"));

        let dotted =
            projection_entity_slug("acct.example", r#"{"id":"acct.example","kind":"account"}"#);
        let dashed =
            projection_entity_slug("acct-example", r#"{"id":"acct-example","kind":"account"}"#);

        assert_ne!(dotted, dashed);
        assert!(dotted.starts_with("acct-example-"));
        assert!(dashed.starts_with("acct-example-"));
    }

    #[cfg(unix)]
    #[test]
    fn projection_abs_path_rejects_managed_root_symlink() {
        let workspace = tempfile::tempdir().expect("workspace");
        let outside = tempfile::tempdir().expect("outside");
        std::os::unix::fs::symlink(
            outside.path(),
            workspace.path().join(CLAIM_FILE_PROJECTION_ROOT),
        )
        .expect("symlink projection root");

        let err = projection_abs_path(
            workspace.path(),
            Path::new("_dailyos_claims/account/acct-1/claims.md"),
        )
        .unwrap_err();

        assert!(err.to_string().contains("symlink"));
        assert!(
            !outside.path().join("account").exists(),
            "projection path resolution must not create directories through the symlink"
        );
    }

    #[cfg(unix)]
    #[test]
    fn read_stable_to_string_rejects_projection_file_symlink() {
        let workspace = tempfile::tempdir().expect("workspace");
        let outside = tempfile::NamedTempFile::new().expect("outside file");
        let projection_dir = workspace
            .path()
            .join(CLAIM_FILE_PROJECTION_ROOT)
            .join("account")
            .join("acct-1");
        std::fs::create_dir_all(&projection_dir).expect("projection dir");
        std::os::unix::fs::symlink(
            outside.path(),
            projection_dir.join(CLAIM_FILE_MARKDOWN_NAME),
        )
        .expect("projection file symlink");

        let err = read_stable_to_string(
            workspace.path(),
            Path::new("_dailyos_claims/account/acct-1/claims.md"),
        )
        .unwrap_err();

        assert!(err.to_string().contains("symlink"));
    }

    #[cfg(unix)]
    #[test]
    fn read_stable_to_string_rejects_projection_file_hardlink() {
        let workspace = tempfile::tempdir().expect("workspace");
        let outside = tempfile::NamedTempFile::new().expect("outside file");
        std::fs::write(outside.path(), "outside").expect("write outside file");
        let projection_dir = workspace
            .path()
            .join(CLAIM_FILE_PROJECTION_ROOT)
            .join("account")
            .join("acct-1");
        std::fs::create_dir_all(&projection_dir).expect("projection dir");
        std::fs::hard_link(
            outside.path(),
            projection_dir.join(CLAIM_FILE_MARKDOWN_NAME),
        )
        .expect("projection file hardlink");

        let err = read_stable_to_string(
            workspace.path(),
            Path::new("_dailyos_claims/account/acct-1/claims.md"),
        )
        .unwrap_err();

        assert!(err.to_string().contains("hard links"));
    }

    #[cfg(unix)]
    #[test]
    fn write_projection_files_rejects_parent_symlink() {
        let workspace = tempfile::tempdir().expect("workspace");
        let outside = tempfile::tempdir().expect("outside");
        let account_dir = workspace
            .path()
            .join(CLAIM_FILE_PROJECTION_ROOT)
            .join("account");
        std::fs::create_dir_all(&account_dir).expect("projection account dir");
        std::os::unix::fs::symlink(outside.path(), account_dir.join("acct-1"))
            .expect("parent symlink");

        let err = write_projection_files(
            workspace.path(),
            Path::new("_dailyos_claims/account/acct-1/claims.md"),
            Path::new("_dailyos_claims/account/acct-1/claims.corrections.json"),
            "markdown",
            "{}",
        )
        .unwrap_err();

        assert!(
            err.to_string().contains("symlink")
                || err.to_string().contains("not a directory")
                || err.to_string().contains("Not a directory")
        );
        assert!(
            !outside.path().join(CLAIM_FILE_MARKDOWN_NAME).exists(),
            "writer must not follow parent symlink outside the workspace"
        );
    }

    #[cfg(unix)]
    #[test]
    fn write_projection_files_rejects_workspace_root_symlink() {
        let outside = tempfile::tempdir().expect("outside");
        let link_parent = tempfile::tempdir().expect("link parent");
        let workspace_link = link_parent.path().join("workspace-link");
        std::os::unix::fs::symlink(outside.path(), &workspace_link).expect("workspace symlink");

        let err = write_projection_files(
            &workspace_link,
            Path::new("_dailyos_claims/account/acct-1/claims.md"),
            Path::new("_dailyos_claims/account/acct-1/claims.corrections.json"),
            "markdown",
            "{}",
        )
        .unwrap_err();

        assert!(err.to_string().contains("symlink"));
        assert!(
            !outside.path().join(CLAIM_FILE_PROJECTION_ROOT).exists(),
            "writer must not anchor projection IO through a symlinked workspace root"
        );
    }

    #[test]
    fn projection_loader_preserves_feedback_bearing_terminal_claims() {
        let db = test_db();
        let (clock, rng, external) = test_context_parts();
        let ctx = test_live_context(&clock, &rng, &external);

        let active_claim_id = inserted_claim_id(
            commit_claim(&ctx, &db, fixture_claim_proposal("Active projection claim"))
                .expect("commit active claim"),
        );
        let withdrawn_with_feedback_id = inserted_claim_id(
            commit_claim(
                &ctx,
                &db,
                fixture_claim_proposal("Terminal projection claim with feedback"),
            )
            .expect("commit feedback-bearing claim"),
        );
        let feedback_outcome = record_claim_feedback(
            &ctx,
            &db,
            ClaimFeedbackInput {
                claim_id: withdrawn_with_feedback_id.clone(),
                action: FeedbackAction::MarkFalse,
                actor: "user".to_string(),
                actor_id: Some("user".to_string()),
                payload_json: None,
            },
        )
        .expect("record feedback");

        let mut tombstone = fixture_claim_proposal("Terminal projection claim without feedback");
        tombstone.tombstone = Some(TombstoneSpec {
            retraction_reason: "user_marked_false".to_string(),
            expires_at: None,
        });
        let withdrawn_without_feedback_id =
            inserted_claim_id(commit_claim(&ctx, &db, tombstone).expect("commit tombstone claim"));

        let claims = load_claims_for_projection(
            &db,
            r#"{"kind":"account","id":"acct-1"}"#,
            "account",
            "acct-1",
        )
        .expect("load projection claims");
        let ids = claims
            .iter()
            .map(|claim| claim.id.as_str())
            .collect::<BTreeSet<_>>();

        assert!(ids.contains(active_claim_id.as_str()));
        assert!(ids.contains(withdrawn_with_feedback_id.as_str()));
        assert!(!ids.contains(withdrawn_without_feedback_id.as_str()));

        let terminal_claim = claims
            .iter()
            .find(|claim| claim.id == withdrawn_with_feedback_id)
            .expect("terminal claim included");
        let sidecar_claim = claim_file_claim(&db, terminal_claim).expect("sidecar claim");

        assert_eq!(sidecar_claim.lifecycle.claim_state, "withdrawn");
        assert_eq!(sidecar_claim.lifecycle.surfacing_state, "dormant");
        assert_eq!(sidecar_claim.feedback_rows.len(), 1);
        assert_eq!(
            sidecar_claim.feedback_rows[0].feedback_id,
            feedback_outcome.feedback_id
        );
        let json = serde_json::to_string(&sidecar_claim).expect("serialize sidecar claim");
        assert!(json.contains("\"feedbackId\""));
        assert!(json.contains(&feedback_outcome.feedback_id));
    }

    #[test]
    fn current_projection_failures_rejects_claim_version_drift_after_render() {
        let db = test_db();
        let (clock, rng, external) = test_context_parts();
        let ctx = test_live_context(&clock, &rng, &external);
        let claim_id = inserted_claim_id(
            commit_claim(&ctx, &db, fixture_claim_proposal("Version drift fixture"))
                .expect("commit projection claim"),
        );
        let bundle = build_render_bundle_db(&db, r#"{"kind":"account","id":"acct-1"}"#)
            .expect("build render bundle");
        let edited_markdown = bundle
            .markdown
            .replace("dailyos-action: none", "dailyos-action: mark_false");
        let corrections =
            parse_markdown_corrections(&edited_markdown, &bundle.sidecar, &bundle.sidecar_checksum)
                .expect("parse edited projection");
        assert_eq!(corrections.len(), 1);

        record_claim_feedback(
            &ctx,
            &db,
            ClaimFeedbackInput {
                claim_id: claim_id.clone(),
                action: FeedbackAction::MarkOutdated,
                actor: "user".to_string(),
                actor_id: Some("user".to_string()),
                payload_json: None,
            },
        )
        .expect("record feedback after render");

        let failures = current_projection_failures_db(&db, &bundle.sidecar, &corrections)
            .expect("check current projection");

        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].claim_id.as_deref(), Some(claim_id.as_str()));
        assert_eq!(failures[0].error_class, "stale_projection");
    }

    #[test]
    fn correction_apply_ledger_replays_applied_reclaims_failed_and_leased_events() {
        let db = test_db();
        let (clock, rng, external) = test_context_parts();
        let ctx = test_live_context(&clock, &rng, &external);
        let claim_id = inserted_claim_id(
            commit_claim(&ctx, &db, fixture_claim_proposal("Apply ledger fixture"))
                .expect("commit projection claim"),
        );
        let correction = ParsedCorrection {
            claim_id: claim_id.clone(),
            projected_claim_version: 1,
            projected_identity_hash: "identity-hash".to_string(),
            action: FeedbackAction::ConfirmCurrent,
            metadata: None,
        };
        let payload_hash = correction_payload_hash(&correction);
        let apply_key =
            correction_apply_idempotency_key("sidecar-checksum", &correction, &payload_hash);

        assert_eq!(
            claim_correction_apply_db(
                &db,
                &CorrectionApplyRecord {
                    apply_key: &apply_key,
                    sidecar_checksum: "sidecar-checksum",
                    claim_id: &claim_id,
                    projected_claim_version: correction.projected_claim_version,
                    projected_identity_hash: &correction.projected_identity_hash,
                    feedback_action: correction.action.as_str(),
                    payload_hash: &payload_hash,
                },
            )
            .expect("claim apply"),
            CorrectionApplyState::Claimed
        );
        let feedback = record_claim_feedback_for_claim_file_apply(
            &ctx,
            &db,
            ClaimFeedbackInput {
                claim_id: claim_id.clone(),
                action: FeedbackAction::ConfirmCurrent,
                actor: "user".to_string(),
                actor_id: Some("user".to_string()),
                payload_json: None,
            },
            ClaimFileFeedbackApplyInput {
                expected_claim_version: correction.projected_claim_version,
                correction_apply_key: apply_key.clone(),
            },
        )
        .expect("record feedback id");
        let stored_feedback_id: Option<String> = db
            .conn_ref()
            .query_row(
                "SELECT feedback_id
                   FROM claim_file_correction_apply_events
                  WHERE idempotency_key = ?1
                    AND status = 'applied'",
                [&apply_key],
                |row| row.get(0),
            )
            .optional()
            .expect("read applied feedback id");
        assert_eq!(
            stored_feedback_id.as_deref(),
            Some(feedback.feedback_id.as_str())
        );
        assert_eq!(
            claim_correction_apply_db(
                &db,
                &CorrectionApplyRecord {
                    apply_key: &apply_key,
                    sidecar_checksum: "sidecar-checksum",
                    claim_id: &claim_id,
                    projected_claim_version: correction.projected_claim_version,
                    projected_identity_hash: &correction.projected_identity_hash,
                    feedback_action: correction.action.as_str(),
                    payload_hash: &payload_hash,
                },
            )
            .expect("replay applied"),
            CorrectionApplyState::AlreadyApplied {
                feedback_id: Some(feedback.feedback_id.clone())
            }
        );

        let failed_key = format!("{apply_key}-failed");
        assert_eq!(
            claim_correction_apply_db(
                &db,
                &CorrectionApplyRecord {
                    apply_key: &failed_key,
                    sidecar_checksum: "sidecar-checksum",
                    claim_id: &claim_id,
                    projected_claim_version: correction.projected_claim_version,
                    projected_identity_hash: &correction.projected_identity_hash,
                    feedback_action: correction.action.as_str(),
                    payload_hash: &payload_hash,
                },
            )
            .expect("claim failed apply"),
            CorrectionApplyState::Claimed
        );
        mark_correction_apply_failed_db(&db, &failed_key, &redacted_hash("rerender failed"))
            .expect("mark failed");
        assert_eq!(
            claim_correction_apply_db(
                &db,
                &CorrectionApplyRecord {
                    apply_key: &failed_key,
                    sidecar_checksum: "sidecar-checksum",
                    claim_id: &claim_id,
                    projected_claim_version: correction.projected_claim_version,
                    projected_identity_hash: &correction.projected_identity_hash,
                    feedback_action: correction.action.as_str(),
                    payload_hash: &payload_hash,
                },
            )
            .expect("reclaim failed"),
            CorrectionApplyState::Claimed
        );

        let leased_key = format!("{apply_key}-leased");
        assert_eq!(
            claim_correction_apply_db(
                &db,
                &CorrectionApplyRecord {
                    apply_key: &leased_key,
                    sidecar_checksum: "sidecar-checksum",
                    claim_id: &claim_id,
                    projected_claim_version: correction.projected_claim_version,
                    projected_identity_hash: &correction.projected_identity_hash,
                    feedback_action: correction.action.as_str(),
                    payload_hash: &payload_hash,
                },
            )
            .expect("claim leased apply"),
            CorrectionApplyState::Claimed
        );
        assert_eq!(
            claim_correction_apply_db(
                &db,
                &CorrectionApplyRecord {
                    apply_key: &leased_key,
                    sidecar_checksum: "sidecar-checksum",
                    claim_id: &claim_id,
                    projected_claim_version: correction.projected_claim_version,
                    projected_identity_hash: &correction.projected_identity_hash,
                    feedback_action: correction.action.as_str(),
                    payload_hash: &payload_hash,
                },
            )
            .expect("fresh claim is in progress"),
            CorrectionApplyState::InProgress
        );
        let expired_at = (Utc::now()
            - Duration::seconds(CLAIM_FILE_CORRECTION_APPLY_LEASE_SECS + 1))
        .to_rfc3339();
        db.conn_ref()
            .execute(
                "UPDATE claim_file_correction_apply_events
                    SET claimed_at = ?1,
                        updated_at = ?1
                  WHERE idempotency_key = ?2",
                params![&expired_at, &leased_key],
            )
            .expect("age leased claim");
        assert_eq!(
            claim_correction_apply_db(
                &db,
                &CorrectionApplyRecord {
                    apply_key: &leased_key,
                    sidecar_checksum: "sidecar-checksum",
                    claim_id: &claim_id,
                    projected_claim_version: correction.projected_claim_version,
                    projected_identity_hash: &correction.projected_identity_hash,
                    feedback_action: correction.action.as_str(),
                    payload_hash: &payload_hash,
                },
            )
            .expect("expired claim is reclaimed"),
            CorrectionApplyState::Claimed
        );
    }

    #[tokio::test]
    async fn apply_claim_file_corrections_repairs_signal_for_already_applied_feedback() {
        let db_dir = tempfile::tempdir().expect("db dir");
        let db_path = db_dir.path().join("dailyos.db");
        let db_service =
            crate::db_service::DbService::open_at_with_fixture_provider_for_tests(db_path)
                .await
                .expect("open test db service");
        let state = AppState::test_with_db_service(db_service);
        let workspace = tempfile::tempdir().expect("workspace");
        let _claim_id = state
            .db_write(|db| {
                let (clock, rng, external) = test_context_parts();
                let ctx = test_live_context(&clock, &rng, &external);
                commit_claim(
                    &ctx,
                    db,
                    fixture_claim_proposal("Public apply signal repair fixture"),
                )
                .map(inserted_claim_id)
                .map_err(|error| error.to_string())
            })
            .await
            .expect("seed projection claim");

        let projection = render_entity_claim_file(
            &state,
            workspace.path().to_path_buf(),
            r#"{"kind":"account","id":"acct-1"}"#.to_string(),
        )
        .await
        .expect("render projection");
        let markdown_rel_path = PathBuf::from(&projection.markdown_rel_path);
        let sidecar_rel_path =
            sidecar_path_for_markdown_rel(&markdown_rel_path).expect("sidecar path");
        let markdown_path = workspace.path().join(&markdown_rel_path);
        let sidecar_path = workspace.path().join(&sidecar_rel_path);
        let original_sidecar_json =
            std::fs::read_to_string(&sidecar_path).expect("read original sidecar");
        let edited_markdown = std::fs::read_to_string(&markdown_path)
            .expect("read markdown")
            .replace("dailyos-action: none", "dailyos-action: cannot_verify");
        assert!(edited_markdown.contains("dailyos-action: cannot_verify"));
        std::fs::write(&markdown_path, &edited_markdown).expect("write edited markdown");

        let sidecar: ClaimFileSidecar =
            serde_json::from_str(&original_sidecar_json).expect("decode original sidecar");
        let corrections =
            parse_markdown_corrections(&edited_markdown, &sidecar, &projection.sidecar_checksum)
                .expect("parse edited correction");
        assert_eq!(corrections.len(), 1);
        let correction = corrections[0].clone();
        let payload_hash = correction_payload_hash(&correction);
        let apply_key = correction_apply_idempotency_key(
            &projection.sidecar_checksum,
            &correction,
            &payload_hash,
        );
        let feedback_id = {
            let sidecar_checksum = projection.sidecar_checksum.clone();
            let apply_key = apply_key.clone();
            let payload_hash = payload_hash.clone();
            state
                .db_write(move |db| {
                    let apply_state = claim_correction_apply_db(
                        db,
                        &CorrectionApplyRecord {
                            apply_key: &apply_key,
                            sidecar_checksum: &sidecar_checksum,
                            claim_id: &correction.claim_id,
                            projected_claim_version: correction.projected_claim_version,
                            projected_identity_hash: &correction.projected_identity_hash,
                            feedback_action: correction.action.as_str(),
                            payload_hash: &payload_hash,
                        },
                    )?;
                    if apply_state != CorrectionApplyState::Claimed {
                        return Err(format!("expected claimed apply state, got {apply_state:?}"));
                    }
                    let (clock, rng, external) = test_context_parts();
                    let ctx = test_live_context(&clock, &rng, &external);
                    record_claim_feedback_for_claim_file_apply(
                        &ctx,
                        db,
                        ClaimFeedbackInput {
                            claim_id: correction.claim_id.clone(),
                            action: correction.action,
                            actor: "user".to_string(),
                            actor_id: Some("user-fixture".to_string()),
                            payload_json: correction
                                .metadata
                                .as_ref()
                                .map(|value| value.to_string()),
                        },
                        ClaimFileFeedbackApplyInput {
                            expected_claim_version: correction.projected_claim_version,
                            correction_apply_key: apply_key,
                        },
                    )
                    .map(|outcome| outcome.feedback_id)
                    .map_err(|error| error.to_string())
                })
                .await
                .expect("seed applied correction")
        };
        let recorded_signal_id = claim_feedback_recorded_signal_id_for_test(&feedback_id);
        let signal_count = {
            let recorded_signal_id = recorded_signal_id.clone();
            state
                .db_write(move |db| {
                    db.conn_ref()
                        .query_row(
                            "SELECT count(*) FROM signal_events WHERE id = ?1",
                            params![&recorded_signal_id],
                            |row| row.get::<_, i64>(0),
                        )
                        .map_err(|error| error.to_string())
                })
                .await
                .expect("count recorded signal")
        };
        assert_eq!(signal_count, 1);
        {
            let recorded_signal_id = recorded_signal_id.clone();
            state
                .db_write(move |db| {
                    db.conn_ref()
                        .execute(
                            "DELETE FROM signal_events WHERE id = ?1",
                            params![&recorded_signal_id],
                        )
                        .map(|_| ())
                        .map_err(|error| error.to_string())
                })
                .await
                .expect("delete recorded signal");
        }

        std::fs::write(&markdown_path, &edited_markdown).expect("restore edited markdown");
        std::fs::write(&sidecar_path, &original_sidecar_json).expect("restore original sidecar");
        let second = apply_claim_file_corrections(
            &state,
            workspace.path().to_path_buf(),
            markdown_rel_path,
            "user-fixture".to_string(),
        )
        .await
        .expect("second apply");

        assert_eq!(second.applied_count, 0);
        assert!(second.failures.is_empty());
        let (feedback_count, signal_count, repaired_payload) = {
            let feedback_id = feedback_id.clone();
            let recorded_signal_id = recorded_signal_id.clone();
            state
                .db_write(move |db| {
                    let feedback_count = db
                        .conn_ref()
                        .query_row(
                            "SELECT count(*) FROM claim_feedback WHERE id = ?1",
                            params![&feedback_id],
                            |row| row.get::<_, i64>(0),
                        )
                        .map_err(|error| error.to_string())?;
                    let signal_count = db
                        .conn_ref()
                        .query_row(
                            "SELECT count(*) FROM signal_events WHERE id = ?1",
                            params![&recorded_signal_id],
                            |row| row.get::<_, i64>(0),
                        )
                        .map_err(|error| error.to_string())?;
                    let repaired_payload = db
                        .conn_ref()
                        .query_row(
                            "SELECT value FROM signal_events WHERE id = ?1",
                            params![&recorded_signal_id],
                            |row| row.get::<_, String>(0),
                        )
                        .map_err(|error| error.to_string())?;
                    Ok((feedback_count, signal_count, repaired_payload))
                })
                .await
                .expect("read repair proof")
        };
        assert_eq!(feedback_count, 1);
        assert_eq!(signal_count, 1);
        assert!(
            repaired_payload.contains("\"recovered\":true"),
            "AlreadyApplied branch must repair the missing recorded-feedback signal"
        );
    }

    #[test]
    fn claim_file_feedback_apply_rejects_stale_expected_version_inside_writer_transaction() {
        let db = test_db();
        let (clock, rng, external) = test_context_parts();
        let ctx = test_live_context(&clock, &rng, &external);
        let claim_id = inserted_claim_id(
            commit_claim(
                &ctx,
                &db,
                fixture_claim_proposal("Atomic stale feedback fixture"),
            )
            .expect("commit projection claim"),
        );
        let correction = ParsedCorrection {
            claim_id: claim_id.clone(),
            projected_claim_version: 1,
            projected_identity_hash: "identity-hash".to_string(),
            action: FeedbackAction::ConfirmCurrent,
            metadata: None,
        };
        let payload_hash = correction_payload_hash(&correction);
        let apply_key =
            correction_apply_idempotency_key("sidecar-checksum", &correction, &payload_hash);
        assert_eq!(
            claim_correction_apply_db(
                &db,
                &CorrectionApplyRecord {
                    apply_key: &apply_key,
                    sidecar_checksum: "sidecar-checksum",
                    claim_id: &claim_id,
                    projected_claim_version: correction.projected_claim_version,
                    projected_identity_hash: &correction.projected_identity_hash,
                    feedback_action: correction.action.as_str(),
                    payload_hash: &payload_hash,
                },
            )
            .expect("claim apply"),
            CorrectionApplyState::Claimed
        );
        record_claim_feedback(
            &ctx,
            &db,
            ClaimFeedbackInput {
                claim_id: claim_id.clone(),
                action: FeedbackAction::MarkOutdated,
                actor: "user".to_string(),
                actor_id: Some("user".to_string()),
                payload_json: None,
            },
        )
        .expect("bump claim version after projection");
        let before_count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT count(*)
                   FROM claim_feedback
                  WHERE claim_id = ?1",
                [&claim_id],
                |row| row.get(0),
            )
            .expect("feedback count before stale apply");

        let err = record_claim_feedback_for_claim_file_apply(
            &ctx,
            &db,
            ClaimFeedbackInput {
                claim_id: claim_id.clone(),
                action: FeedbackAction::ConfirmCurrent,
                actor: "user".to_string(),
                actor_id: Some("user".to_string()),
                payload_json: None,
            },
            ClaimFileFeedbackApplyInput {
                expected_claim_version: 1,
                correction_apply_key: apply_key.clone(),
            },
        )
        .expect_err("stale projected version must reject inside the writer transaction");
        assert!(matches!(
            err,
            ClaimError::StaleVersion {
                claim_id: ref stale_claim_id,
                expected: 1,
                current: 2,
            } if stale_claim_id == &claim_id
        ));
        let after_count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT count(*)
                   FROM claim_feedback
                  WHERE claim_id = ?1",
                [&claim_id],
                |row| row.get(0),
            )
            .expect("feedback count after stale apply");
        assert_eq!(after_count, before_count);
        let status: String = db
            .conn_ref()
            .query_row(
                "SELECT status
                   FROM claim_file_correction_apply_events
                  WHERE idempotency_key = ?1",
                [&apply_key],
                |row| row.get(0),
            )
            .expect("read apply status");
        assert_eq!(status, "claimed");
    }

    #[test]
    fn resumed_expired_worker_does_not_overwrite_newer_applied_correction() {
        let db = test_db();
        let (clock, rng, external) = test_context_parts();
        let ctx = test_live_context(&clock, &rng, &external);
        let claim_id = inserted_claim_id(
            commit_claim(
                &ctx,
                &db,
                fixture_claim_proposal("Expired worker replay fixture"),
            )
            .expect("commit projection claim"),
        );
        let correction = ParsedCorrection {
            claim_id: claim_id.clone(),
            projected_claim_version: 1,
            projected_identity_hash: "identity-hash".to_string(),
            action: FeedbackAction::ConfirmCurrent,
            metadata: None,
        };
        let payload_hash = correction_payload_hash(&correction);
        let apply_key =
            correction_apply_idempotency_key("sidecar-checksum", &correction, &payload_hash);
        let record = CorrectionApplyRecord {
            apply_key: &apply_key,
            sidecar_checksum: "sidecar-checksum",
            claim_id: &claim_id,
            projected_claim_version: correction.projected_claim_version,
            projected_identity_hash: &correction.projected_identity_hash,
            feedback_action: correction.action.as_str(),
            payload_hash: &payload_hash,
        };

        assert_eq!(
            claim_correction_apply_db(&db, &record).expect("worker A claims correction"),
            CorrectionApplyState::Claimed
        );
        let expired_at = (Utc::now()
            - Duration::seconds(CLAIM_FILE_CORRECTION_APPLY_LEASE_SECS + 1))
        .to_rfc3339();
        db.conn_ref()
            .execute(
                "UPDATE claim_file_correction_apply_events
                    SET claimed_at = ?1,
                        updated_at = ?1
                  WHERE idempotency_key = ?2",
                params![&expired_at, &apply_key],
            )
            .expect("expire worker A lease");
        assert_eq!(
            claim_correction_apply_db(&db, &record).expect("worker B reclaims expired correction"),
            CorrectionApplyState::Claimed
        );
        let applied = record_claim_feedback_for_claim_file_apply(
            &ctx,
            &db,
            ClaimFeedbackInput {
                claim_id: claim_id.clone(),
                action: FeedbackAction::ConfirmCurrent,
                actor: "user".to_string(),
                actor_id: Some("user".to_string()),
                payload_json: None,
            },
            ClaimFileFeedbackApplyInput {
                expected_claim_version: 1,
                correction_apply_key: apply_key.clone(),
            },
        )
        .expect("worker B atomically applies no-op feedback");
        let worker_a_err = record_claim_feedback_for_claim_file_apply(
            &ctx,
            &db,
            ClaimFeedbackInput {
                claim_id: claim_id.clone(),
                action: FeedbackAction::ConfirmCurrent,
                actor: "user".to_string(),
                actor_id: Some("user".to_string()),
                payload_json: None,
            },
            ClaimFileFeedbackApplyInput {
                expected_claim_version: 1,
                correction_apply_key: apply_key.clone(),
            },
        )
        .expect_err("worker A cannot apply after worker B finalized the ledger");
        assert!(
            worker_a_err.to_string().contains("was not claimable"),
            "unexpected worker A error: {worker_a_err}"
        );

        assert_eq!(
            mark_correction_apply_failed_db(&db, &apply_key, &redacted_hash("worker A failed"))
                .expect("worker A failure cleanup"),
            CorrectionFailureMarkState::AlreadyApplied
        );
        let (status, feedback_id): (String, Option<String>) = db
            .conn_ref()
            .query_row(
                "SELECT status, feedback_id
                   FROM claim_file_correction_apply_events
                  WHERE idempotency_key = ?1",
                [&apply_key],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("read final apply row");
        assert_eq!(status, "applied");
        assert_eq!(feedback_id.as_deref(), Some(applied.feedback_id.as_str()));
        assert_eq!(
            claim_correction_apply_db(&db, &record).expect("future retry reads applied"),
            CorrectionApplyState::AlreadyApplied {
                feedback_id: Some(applied.feedback_id.clone())
            }
        );
        let feedback_count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT count(*)
                   FROM claim_feedback
                  WHERE claim_id = ?1",
                [&claim_id],
                |row| row.get(0),
            )
            .expect("count feedback rows");
        assert_eq!(feedback_count, 1);
    }

    #[test]
    fn projection_subject_kind_is_limited_to_account_project_person() {
        assert_eq!(
            supported_entity_kind_slug(&ClaimSubjectRef::Account {
                id: "acct-1".to_string()
            }),
            Some("account")
        );
        assert_eq!(
            supported_entity_kind_slug(&ClaimSubjectRef::Project {
                id: "proj-1".to_string()
            }),
            Some("project")
        );
        assert_eq!(
            supported_entity_kind_slug(&ClaimSubjectRef::Person {
                id: "person-1".to_string()
            }),
            Some("person")
        );
        assert_eq!(
            supported_entity_kind_slug(&ClaimSubjectRef::Action {
                id: "action-1".to_string()
            }),
            None
        );
        assert_eq!(supported_entity_kind_slug(&ClaimSubjectRef::Global), None);
    }

    #[test]
    fn semantic_identity_hashes_compute_dedup_key_components() {
        let claim = fixture_intelligence_claim("risk", Some("health.risk"), Some("item-hash-1"));
        let identity = semantic_identity_for_claim(
            &claim,
            serde_json::json!({"kind": "account", "id": "acct-1"}),
            r#"{"id":"acct-1","kind":"account"}"#.to_string(),
        );
        let expected_subject = r#"{"id":"acct-1","kind":"account"}"#;
        let expected_components =
            format!("item-hash-1\u{1f}{expected_subject}\u{1f}risk\u{1f}health.risk");

        assert_eq!(identity.identity_kind, "claim_dedup_v1");
        assert_eq!(
            identity.dedup_key_components_hash,
            sha256_hex(expected_components.as_bytes())
        );
        assert_eq!(
            crate::services::claims::compute_dedup_key(
                "item-hash-1",
                &identity.subject_ref_compact,
                "risk",
                Some("health.risk")
            ),
            "item-hash-1:{\"id\":\"acct-1\",\"kind\":\"account\"}:risk:health.risk"
        );
        assert_eq!(identity.runtime_claim_version, 7);
    }

    #[test]
    fn user_note_identity_uses_user_note_kind() {
        let mut claim = fixture_intelligence_claim("user_note", None, Some("note-hash-1"));
        claim.actor = "user:fixture".to_string();
        let identity = semantic_identity_for_claim(
            &claim,
            serde_json::json!({"kind": "account", "id": "acct-1"}),
            r#"{"id":"acct-1","kind":"account"}"#.to_string(),
        );

        assert_eq!(identity.identity_kind, "user_note_v1");
        assert_eq!(identity.actor, "user:fixture");
        assert_eq!(identity.item_hash, "note-hash-1");
        assert_eq!(
            identity.dedup_key_components_hash,
            crate::services::claims::compute_user_note_dedup_key(
                &identity.subject_ref_compact,
                "user:fixture",
                TEST_TS
            )
        );
    }

    #[test]
    fn projection_change_status_detects_failed_run_and_repaired_success() {
        let db = test_db();
        let (clock, rng, external) = test_context_parts();
        let ctx = test_live_context(&clock, &rng, &external);
        commit_claim(
            &ctx,
            &db,
            fixture_claim_proposal("Claim file repair fixture"),
        )
        .expect("commit projection claim");
        let subject_ref_json = r#"{"kind":"account","id":"acct-1"}"#;
        let bundle = build_render_bundle_db(&db, subject_ref_json).expect("build render bundle");

        insert_projection_run(
            &db,
            &bundle,
            "failed",
            None,
            Some("PermissionDenied"),
            Some("redacted-error-hash"),
        )
        .expect("insert failed projection run");
        let failed_status =
            projection_change_status_for_bundle(&db, &bundle).expect("read failed status");

        assert!(failed_status.needs_repair);
        assert_eq!(failed_status.latest_run_status.as_deref(), Some("failed"));
        assert!(failed_status
            .repair_reasons
            .contains(&"previous_projection_failed".to_string()));

        let repaired_bundle = RenderBundle {
            run_id: "zz-repaired-run".to_string(),
            ..bundle.clone()
        };
        insert_projection_run(
            &db,
            &repaired_bundle,
            "repaired",
            Some(&bundle.run_id),
            None,
            None,
        )
        .expect("insert repaired projection run");
        let repaired_status = projection_change_status_for_bundle(&db, &repaired_bundle)
            .expect("read repaired status");
        let repaired_from_run_id: Option<String> = db
            .conn_ref()
            .query_row(
                "SELECT repaired_from_run_id
                   FROM claim_file_projection_runs
                  WHERE id = ?1",
                [&repaired_bundle.run_id],
                |row| row.get(0),
            )
            .expect("read repaired link");
        let repaired_membership_count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT count(*)
                   FROM claim_file_projection_run_claims
                  WHERE run_id = ?1",
                [&repaired_bundle.run_id],
                |row| row.get(0),
            )
            .expect("read repaired membership");

        assert!(!repaired_status.needs_repair);
        assert_eq!(
            repaired_status.latest_run_status.as_deref(),
            Some("repaired")
        );
        assert_eq!(
            repaired_from_run_id.as_deref(),
            Some(bundle.run_id.as_str())
        );
        assert!(repaired_membership_count > 0);
    }

    #[test]
    fn projection_path_binding_rejects_same_path_for_different_subject() {
        let db = test_db();
        let (clock, rng, external) = test_context_parts();
        let ctx = test_live_context(&clock, &rng, &external);
        commit_claim(&ctx, &db, fixture_claim_proposal("Path binding fixture"))
            .expect("commit projection claim");
        let bundle = build_render_bundle_db(&db, r#"{"kind":"account","id":"acct-1"}"#)
            .expect("build render bundle");

        ensure_projection_path_binding(&db, &bundle, TEST_TS).expect("bind projection path");

        let mut collision = bundle.clone();
        collision.entity_subject_compact = r#"{"id":"acct-2","kind":"account"}"#.to_string();
        let err = ensure_projection_path_binding(&db, &collision, TEST_TS)
            .expect_err("same path must not bind a different subject");

        assert!(err.contains("projection path collision"));
    }

    #[test]
    fn committed_sidecar_validation_requires_recorded_run_and_claim_membership() {
        let db = test_db();
        let (clock, rng, external) = test_context_parts();
        let ctx = test_live_context(&clock, &rng, &external);
        commit_claim(
            &ctx,
            &db,
            fixture_claim_proposal("Committed sidecar fixture"),
        )
        .expect("commit projection claim");
        let bundle = build_render_bundle_db(&db, r#"{"kind":"account","id":"acct-1"}"#)
            .expect("build render bundle");

        ensure_projection_path_binding(&db, &bundle, TEST_TS).expect("bind projection path");
        let err = validate_committed_sidecar_projection_db(
            &db,
            &bundle.sidecar,
            &bundle.sidecar_checksum,
        )
        .expect_err("sidecar without a committed run must not apply");
        assert!(err.contains("committed projection run not found"));

        insert_projection_run(&db, &bundle, "committed", None, None, None)
            .expect("record committed projection");
        let committed = validate_committed_sidecar_projection_db(
            &db,
            &bundle.sidecar,
            &bundle.sidecar_checksum,
        )
        .expect("validate committed sidecar");
        assert_eq!(committed.run_id, bundle.run_id);

        let err = validate_committed_sidecar_projection_db(
            &db,
            &bundle.sidecar,
            "wrong-sidecar-checksum",
        )
        .expect_err("checksum mismatch must not match a committed run");
        assert!(err.contains("committed projection run not found"));

        let mut tampered_membership = bundle.sidecar.clone();
        tampered_membership.claims[0]
            .semantic_identity
            .dedup_key_components_hash
            .push_str("-tampered");
        let err = validate_committed_sidecar_projection_db(
            &db,
            &tampered_membership,
            &bundle.sidecar_checksum,
        )
        .expect_err("sidecar claim identity tampering must not apply");
        assert!(err.contains("projection claim membership mismatch"));

        let mut tampered_paths = bundle.sidecar.clone();
        tampered_paths.markdown_rel_path = "_dailyos_claims/account/acct-1/claims.md".to_string();
        tampered_paths.sidecar_rel_path =
            "_dailyos_claims/account/acct-1/claims.corrections.json".to_string();
        let tampered_checksum = sha256_hex(
            serde_json::to_string_pretty(&tampered_paths)
                .unwrap()
                .as_bytes(),
        );
        let err =
            validate_committed_sidecar_projection_db(&db, &tampered_paths, &tampered_checksum)
                .expect_err("recomputed sidecar path tampering must not match a committed run");
        assert!(
            err.contains("projection path binding missing")
                || err.contains("committed projection run not found")
        );
    }
}
