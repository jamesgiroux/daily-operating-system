//! Readable claim-file projection and correction apply service.
//!
//! Files under `_dailyos_claims` are projections of canonical claims. They are
//! never source evidence, and applying edits routes through the existing
//! receipt-level feedback path.

use std::collections::BTreeSet;
use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};

use abilities_runtime::abilities::feedback::FeedbackAction;
use abilities_runtime::abilities::provenance::subject::SubjectRef as ReceiptSubjectRef;
use abilities_runtime::sensitivity::RenderActor;
use abilities_runtime::types::{ClaimSensitivity, ClaimSubjectRef, IntelligenceClaim};
use chrono::Utc;
use rusqlite::params;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::db::claim_invalidation::SubjectRef as InvalidationSubjectRef;
use crate::db::ActionDb;
use crate::services::claim_receipt::contracts::{ReceiptTarget, SurfaceContext};
use crate::services::claim_receipt::feedback::{
    submit_claim_feedback, ClaimFeedbackRequest, ClaimFeedbackResponse, IdempotencyCache,
};
use crate::services::entity_intelligence::auth::{EnvelopeSet, EnvelopeView};
use crate::services::workspace_ingestion::registry::CLAIM_FILE_PROJECTION_ROOT;
use crate::state::AppState;

pub const CLAIM_FILE_PROJECTION_VERSION: u32 = 1;
pub const CLAIM_FILE_LEGACY_SIDECAR_SCHEMA_VERSION: u32 = 1;
pub const CLAIM_FILE_SIDECAR_SCHEMA_VERSION: u32 = 2;
pub const CLAIM_FILE_MARKDOWN_NAME: &str = "claims.md";
pub const CLAIM_FILE_SIDECAR_NAME: &str = "claims.corrections.json";

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
    #[serde(default)]
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
    action: FeedbackAction,
    metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone)]
struct FileEnvelope {
    claim_ids: BTreeSet<String>,
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
    let bundle = build_render_bundle(state, subject_ref_json).await?;
    let markdown_path = projection_abs_path(&workspace_root, &bundle.markdown_rel_path)?;
    let sidecar_path = projection_abs_path(&workspace_root, &bundle.sidecar_rel_path)?;

    let write_result = write_projection_files(
        &workspace_root,
        &markdown_path,
        &sidecar_path,
        &bundle.markdown,
        &bundle.sidecar_json,
    );
    record_projection_run(state, &bundle, write_result.as_ref().err()).await?;
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
    let markdown_path = projection_abs_path(&workspace_root, &markdown_rel_path)?;
    let sidecar_rel_path = sidecar_path_for_markdown_rel(&markdown_rel_path)?;
    let sidecar_path = projection_abs_path(&workspace_root, &sidecar_rel_path)?;

    let markdown = read_stable_to_string(&markdown_path)?;
    let sidecar_json = read_stable_to_string(&sidecar_path)?;
    let sidecar_checksum = sha256_hex(sidecar_json.as_bytes());
    let sidecar: ClaimFileSidecar = serde_json::from_str(&sidecar_json)?;
    verify_sidecar_contract(&sidecar)?;
    verify_sidecar_paths(&sidecar, &markdown_rel_path, &sidecar_rel_path)?;

    let corrections = parse_markdown_corrections(&markdown, &sidecar, &sidecar_checksum)?;
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
        let target = receipt_target_for_claim(&sidecar, &correction.claim_id)?;
        let request = ClaimFeedbackRequest {
            target,
            action: correction.action,
            surface: SurfaceContext::EntityDetail,
            metadata: correction.metadata,
            idempotency_key: None,
        };
        match submit_claim_feedback(state, &set, &actor, &cache, request).await {
            Ok(response) => responses.push(response),
            Err(error) => failures.push(ClaimFileApplyFailure {
                claim_id: Some(correction.claim_id),
                error_class: "feedback_rejected".to_string(),
                error_detail_hash: redacted_hash(&error.to_string()),
            }),
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
    let entity_slug = slug_segment(entity_id);
    let markdown_rel_path = PathBuf::from(CLAIM_FILE_PROJECTION_ROOT)
        .join(entity_kind)
        .join(entity_slug)
        .join(CLAIM_FILE_MARKDOWN_NAME);
    let sidecar_rel_path = PathBuf::from(CLAIM_FILE_PROJECTION_ROOT)
        .join(entity_kind)
        .join(slug_segment(entity_id))
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
    let identity_kind = if claim.claim_type == "user_note" {
        "user_note_v1"
    } else {
        "claim_dedup_v1"
    };
    let field_path_component = claim.field_path.clone().unwrap_or_default();
    let components = format!(
        "{}\u{1f}{}\u{1f}{}\u{1f}{}",
        item_hash, subject_ref_compact, claim.claim_type, field_path_component
    );
    ClaimSemanticIdentityV1 {
        identity_version: 1,
        identity_kind: identity_kind.to_string(),
        item_hash,
        subject_ref_compact,
        subject_ref,
        claim_type: claim.claim_type.clone(),
        field_path: claim.field_path.clone(),
        dedup_key_components_hash: sha256_hex(components.as_bytes()),
        source_ref: claim.source_ref.clone(),
        data_source: claim.data_source.clone(),
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
    write_error: Option<&std::io::Error>,
) -> Result<(), ClaimFileError> {
    let run = bundle.clone();
    let status = if write_error.is_some() {
        "failed".to_string()
    } else {
        "committed".to_string()
    };
    let error_class = write_error.map(|error| error.kind().to_string());
    let error_detail_hash = write_error.map(|error| redacted_hash(&error.to_string()));
    state
        .db_write(move |db| {
            insert_projection_run(
                db,
                &run,
                &status,
                error_class.as_deref(),
                error_detail_hash.as_deref(),
            )
        })
        .await
        .map_err(|error| ClaimFileError::Db(error.to_string()))?;
    Ok(())
}

fn insert_projection_run(
    db: &ActionDb,
    run: &RenderBundle,
    status: &str,
    error_class: Option<&str>,
    error_detail_hash: Option<&str>,
) -> Result<(), String> {
    let now = Utc::now().to_rfc3339();
    db.conn_ref()
        .execute(
            "INSERT INTO claim_file_projection_runs (
                id, entity_subject_ref_json, entity_subject_compact, projection_root,
                markdown_rel_path, sidecar_rel_path, projection_version,
                sidecar_schema_version, entity_claim_invalidation_version,
                claim_watermark, markdown_checksum, sidecar_checksum, status,
                error_class, error_detail_hash, attempted_at, succeeded_at, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?16, ?16)",
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
                if status == "committed" { Some(now.as_str()) } else { None },
            ],
        )
        .map_err(|error| error.to_string())?;
    if status == "committed" {
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

#[derive(Debug, Default)]
struct ParsedBlock {
    claim_id: Option<String>,
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
    let action_raw = block.action.unwrap_or_else(|| "none".to_string());
    if action_raw == "none" {
        return Ok(None);
    }
    let action = feedback_action_from_slug(&action_raw)?;
    let metadata = metadata_for_action(action, block.payload.as_deref(), claim)?;
    Ok(Some(ParsedCorrection {
        claim_id,
        action,
        metadata,
    }))
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
    if matches!(action, FeedbackAction::WrongSource) {
        let mut obj = payload_value
            .and_then(|value| value.as_object().cloned())
            .unwrap_or_default();
        if !obj.contains_key("source_content_hash") {
            let hash = claim
                .semantic_identity
                .source_content_hash
                .as_deref()
                .ok_or_else(|| {
                    ClaimFileError::BadRequest(
                        "wrong_source missing source_content_hash".to_string(),
                    )
                })?;
            obj.insert(
                "source_content_hash".to_string(),
                serde_json::Value::String(hash.to_string()),
            );
        }
        return Ok(Some(serde_json::Value::Object(obj)));
    }
    Ok(payload_value)
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

fn write_projection_files(
    workspace_root: &Path,
    markdown_path: &Path,
    sidecar_path: &Path,
    markdown: &str,
    sidecar_json: &str,
) -> std::io::Result<()> {
    let canonical_root = workspace_root.canonicalize()?;
    create_projection_parent_dirs(&canonical_root, sidecar_path)?;
    create_projection_parent_dirs(&canonical_root, markdown_path)?;
    crate::util::atomic_write_str(sidecar_path, sidecar_json)?;
    crate::util::atomic_write_str(markdown_path, markdown)?;
    Ok(())
}

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

fn read_stable_to_string(path: &Path) -> Result<String, ClaimFileError> {
    let (mut file, before) = open_projection_file_no_follow(path)?;
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
fn open_projection_file_no_follow(path: &Path) -> Result<(fs::File, fs::Metadata), ClaimFileError> {
    use std::os::unix::fs::OpenOptionsExt;

    let file = fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)
        .map_err(|error| {
            if error.raw_os_error() == Some(libc::ELOOP) {
                ClaimFileError::PathRejected("projection file is a symlink".to_string())
            } else {
                ClaimFileError::Io(error)
            }
        })?;
    let metadata = file.metadata()?;
    validate_projection_file_metadata(&metadata)?;
    Ok((file, metadata))
}

#[cfg(not(unix))]
fn open_projection_file_no_follow(path: &Path) -> Result<(fs::File, fs::Metadata), ClaimFileError> {
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
        commit_claim, record_claim_feedback, ClaimFeedbackInput, ClaimProposal, CommittedClaim,
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
        ServiceContext::test_live(clock, rng, external)
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
dailyos-sidecar-checksum: checksum-1
dailyos-action: wrong_source
dailyos-payload: {}
<!-- dailyos-claim-end -->
";

        let corrections =
            parse_markdown_corrections(markdown, &sidecar, "checksum-1").expect("parse");

        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].action, FeedbackAction::WrongSource);
        assert_eq!(
            corrections[0]
                .metadata
                .as_ref()
                .and_then(|metadata| metadata.get("source_content_hash")),
            Some(&serde_json::json!("abcdef0123456789"))
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
                 dailyos-sidecar-checksum: checksum-1\n\
                 dailyos-action: {slug}\n\
                 dailyos-payload: {payload}\n\
                 <!-- dailyos-claim-end -->\n"
            );

            let corrections =
                parse_markdown_corrections(&markdown, &sidecar, "checksum-1").expect("parse");

            assert_eq!(corrections.len(), 1, "{slug} should map one correction");
            assert_eq!(corrections[0].action, expected, "{slug} action maps");
        }
    }

    #[test]
    fn parser_rejects_ambiguous_unterminated_claim_block() {
        let sidecar = fixture_sidecar();
        let markdown = "\
<!-- dailyos-claim-start -->
dailyos-claim-id: claim-1
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
dailyos-sidecar-checksum: stale
dailyos-action: mark_false
dailyos-payload: {}
<!-- dailyos-claim-end -->
";

        let err = parse_markdown_corrections(markdown, &sidecar, "current").unwrap_err();

        assert!(err.to_string().contains("sidecar_mismatch"));
    }

    #[test]
    fn parser_ignores_directive_like_text_after_claim_block_header() {
        let sidecar = fixture_sidecar();
        let markdown = "\
<!-- dailyos-claim-start -->
dailyos-claim-id: claim-1
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

        let err =
            read_stable_to_string(&projection_dir.join(CLAIM_FILE_MARKDOWN_NAME)).unwrap_err();

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

        let err =
            read_stable_to_string(&projection_dir.join(CLAIM_FILE_MARKDOWN_NAME)).unwrap_err();

        assert!(err.to_string().contains("hard links"));
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
        let claim = fixture_intelligence_claim("user_note", None, Some("note-hash-1"));
        let identity = semantic_identity_for_claim(
            &claim,
            serde_json::json!({"kind": "account", "id": "acct-1"}),
            r#"{"id":"acct-1","kind":"account"}"#.to_string(),
        );

        assert_eq!(identity.identity_kind, "user_note_v1");
        assert_eq!(identity.item_hash, "note-hash-1");
    }
}
