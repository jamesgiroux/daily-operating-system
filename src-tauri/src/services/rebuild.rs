//! First-class rebuild replay helpers.
//!
//! This module owns rebuild replay planning and reporting. The L1a slice is
//! deliberately scoped to current-encrypted/Replica proof: it replays W3 claim
//! file correction sidecars through the claim service, and it records which
//! storage/cutover claims remain outside this proof slice.

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::{params, OptionalExtension};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::abilities::feedback::FeedbackAction;
use crate::db::ActionDb;
use crate::services::claim_files::{
    semantic_identity_for_loaded_claim, source_content_hash_for_claim,
    validate_replay_sidecar_projection_db, ClaimFileClaim, ClaimFileContradictionEdge,
    ClaimFileFeedbackRow, ClaimFileLifecycle, ClaimFileProvenanceSummary, ClaimFileReplayStatus,
    ClaimFileSidecar, ClaimSemanticIdentityV1, CLAIM_FILE_LEGACY_SIDECAR_SCHEMA_VERSION,
    CLAIM_FILE_SIDECAR_SCHEMA_VERSION,
};
use crate::services::claims::{
    claim_feedback_replay_content_hash, load_claim_by_id, record_claim_feedback_replay,
    recorded_claim_feedback_replay_outcome, repair_claim_feedback_recorded_signal, ClaimError,
    ClaimFeedbackInput, ClaimFeedbackReplayInput,
};
use crate::services::context::ServiceContext;

const LEGACY_V282_UNSET_FEEDBACK_CONTENT_HASH: &str = "legacy-v282-unset";

#[derive(Debug, thiserror::Error)]
pub enum RebuildError {
    #[error("invalid sidecar: {0}")]
    InvalidSidecar(String),
    #[error("claim replay failed: {0}")]
    ClaimReplay(String),
    #[error("db: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReplayEventStatus {
    Applied,
    AlreadyApplied,
    OrphanMissing,
    OrphanAmbiguous,
    Failed,
}

impl ReplayEventStatus {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Applied => "applied",
            Self::AlreadyApplied => "already_applied",
            Self::OrphanMissing => "orphan_missing",
            Self::OrphanAmbiguous => "orphan_ambiguous",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ReplayEventOutcome {
    pub sidecar_event_id: String,
    pub source_runtime_claim_id: String,
    pub resolved_claim_id: Option<String>,
    pub action: String,
    pub status: ReplayEventStatus,
    pub reason_code: Option<String>,
    pub applied_feedback_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CorrectionReplayReport {
    pub run_id: String,
    pub sidecar_schema_version: u32,
    pub applied_count: usize,
    pub already_applied_count: usize,
    pub orphaned_count: usize,
    pub failed_count: usize,
    pub events: Vec<ReplayEventOutcome>,
    pub plain_sqlite_recovery_proven: bool,
    pub live_cutover_proven: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Resolution {
    Resolved(String),
    Missing,
    Ambiguous { candidate_count: usize },
}

#[derive(Debug, Clone, Copy)]
struct ReplayScope<'a> {
    run_id: &'a str,
    sidecar_checksum: &'a str,
    sidecar_schema_version: u32,
}

#[derive(Debug, Clone, Copy)]
struct SidecarClaimRef<'a> {
    runtime_claim_id: &'a str,
    identity: &'a ClaimSemanticIdentityV1,
}

#[derive(Debug, Clone, Copy)]
struct SidecarFeedbackRef<'a> {
    row: &'a ClaimFileFeedbackRow,
    event_id: &'a str,
    action: FeedbackAction,
}

#[derive(Debug, Clone)]
struct SidecarFeedbackReplayIdentity {
    action: FeedbackAction,
    content_hash: String,
}

struct ClaimReplayJournalInput<'a> {
    scope: ReplayScope<'a>,
    source_claim: SidecarClaimRef<'a>,
    resolved_claim_id: Option<&'a str>,
    sidecar_event_id: &'a str,
    action: &'a str,
    reason_code: Option<&'a str>,
    feedback_content_hash: &'a str,
}

#[derive(Debug, Clone)]
struct ExistingReplayEvent {
    sidecar_schema_version: u32,
    sidecar_checksum: String,
    source_runtime_claim_id: String,
    resolved_claim_id: Option<String>,
    action: String,
    status: String,
    reason_code: Option<String>,
    semantic_identity_hash: String,
    feedback_content_hash: String,
    applied_feedback_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ReplayEventClaim {
    Claimed,
    ExistingTerminal {
        status: ReplayEventStatus,
        reason_code: Option<String>,
        applied_feedback_id: Option<String>,
    },
}

pub fn replay_claim_file_sidecar_corrections(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    run_id: &str,
    sidecar: &ClaimFileSidecar,
    sidecar_checksum: &str,
) -> Result<CorrectionReplayReport, RebuildError> {
    if run_id.trim().is_empty() {
        return Err(RebuildError::InvalidSidecar(
            "run_id is required for rebuild replay".to_string(),
        ));
    }
    let verified_sidecar_checksum = verified_sidecar_checksum(sidecar, sidecar_checksum)?;
    validate_replay_sidecar(sidecar)?;
    let committed_projection =
        validate_replay_sidecar_projection_db(db, sidecar, &verified_sidecar_checksum)
            .map_err(RebuildError::InvalidSidecar)?;

    let scope = ReplayScope {
        run_id,
        sidecar_checksum: &verified_sidecar_checksum,
        sidecar_schema_version: sidecar.schema_version,
    };
    log::debug!(
        "authorized claim-file sidecar replay projection_run_id={} replay_run_id={}",
        committed_projection.run_id,
        run_id
    );
    let sidecar_feedback_events = sidecar_feedback_events(sidecar)?;
    let mut events = Vec::new();
    for claim in &sidecar.claims {
        let resolution = resolve_claim_by_semantic_identity(db, &claim.semantic_identity)?;
        let source_claim = SidecarClaimRef {
            runtime_claim_id: &claim.runtime_claim_id,
            identity: &claim.semantic_identity,
        };
        for (feedback_index, feedback) in claim.feedback_rows.iter().enumerate() {
            let action = feedback_action_from_slug(&feedback.action)?;
            let sidecar_event_id =
                sidecar_feedback_event_id(sidecar, claim, feedback, feedback_index)?;
            let feedback_ref = SidecarFeedbackRef {
                row: feedback,
                event_id: &sidecar_event_id,
                action,
            };
            let event = match &resolution {
                Resolution::Resolved(claim_id) => replay_feedback_event(
                    ctx,
                    db,
                    scope,
                    source_claim,
                    claim_id,
                    feedback_ref,
                    &sidecar_feedback_events,
                )?,
                Resolution::Missing => record_orphan_replay_event(
                    db,
                    scope,
                    source_claim,
                    feedback_ref,
                    ReplayEventStatus::OrphanMissing,
                    "semantic_identity_missing",
                )?,
                Resolution::Ambiguous { candidate_count } => record_orphan_replay_event(
                    db,
                    scope,
                    source_claim,
                    feedback_ref,
                    ReplayEventStatus::OrphanAmbiguous,
                    &format!("semantic_identity_ambiguous_{candidate_count}"),
                )?,
            };
            events.push(event);
        }
    }

    let applied_count = events
        .iter()
        .filter(|event| event.status == ReplayEventStatus::Applied)
        .count();
    let already_applied_count = events
        .iter()
        .filter(|event| event.status == ReplayEventStatus::AlreadyApplied)
        .count();
    let orphaned_count = events
        .iter()
        .filter(|event| {
            matches!(
                event.status,
                ReplayEventStatus::OrphanMissing | ReplayEventStatus::OrphanAmbiguous
            )
        })
        .count();
    let failed_count = events
        .iter()
        .filter(|event| event.status == ReplayEventStatus::Failed)
        .count();

    Ok(CorrectionReplayReport {
        run_id: run_id.to_string(),
        sidecar_schema_version: sidecar.schema_version,
        applied_count,
        already_applied_count,
        orphaned_count,
        failed_count,
        events,
        plain_sqlite_recovery_proven: false,
        live_cutover_proven: false,
    })
}

fn verified_sidecar_checksum(
    sidecar: &ClaimFileSidecar,
    supplied_checksum: &str,
) -> Result<String, RebuildError> {
    let supplied_checksum = supplied_checksum.trim();
    if supplied_checksum.is_empty() {
        return Err(RebuildError::InvalidSidecar(
            "sidecar checksum does not match supplied sidecar payload".to_string(),
        ));
    }
    for candidate in sidecar_payload_checksum_candidates(sidecar)? {
        if supplied_checksum == candidate {
            return Ok(supplied_checksum.to_string());
        }
    }
    Err(RebuildError::InvalidSidecar(
        "sidecar checksum does not match supplied sidecar payload".to_string(),
    ))
}

fn sidecar_payload_checksum_candidates(
    sidecar: &ClaimFileSidecar,
) -> Result<Vec<String>, RebuildError> {
    let mut candidates = vec![sidecar_payload_checksum(sidecar)?];
    if sidecar.schema_version == CLAIM_FILE_LEGACY_SIDECAR_SCHEMA_VERSION {
        let legacy_checksum = legacy_v1_sidecar_payload_checksum(sidecar)?;
        if !candidates.contains(&legacy_checksum) {
            candidates.push(legacy_checksum);
        }
    }
    Ok(candidates)
}

fn sidecar_payload_checksum(sidecar: &ClaimFileSidecar) -> Result<String, RebuildError> {
    json_payload_checksum(sidecar)
}

fn legacy_v1_sidecar_payload_checksum(sidecar: &ClaimFileSidecar) -> Result<String, RebuildError> {
    json_payload_checksum(&LegacyV1SidecarChecksum::from(sidecar))
}

fn json_payload_checksum<T: Serialize>(payload: &T) -> Result<String, RebuildError> {
    let sidecar_json = serde_json::to_string_pretty(payload)?;
    let mut hasher = Sha256::new();
    hasher.update(sidecar_json.as_bytes());
    Ok(format!("{:x}", hasher.finalize()))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LegacyV1SidecarChecksum<'a> {
    schema_version: u32,
    projection_version: u32,
    entity_subject_ref: &'a serde_json::Value,
    entity_subject_compact: &'a str,
    markdown_rel_path: &'a str,
    sidecar_rel_path: &'a str,
    claims: Vec<LegacyV1ClaimChecksum<'a>>,
}

impl<'a> From<&'a ClaimFileSidecar> for LegacyV1SidecarChecksum<'a> {
    fn from(sidecar: &'a ClaimFileSidecar) -> Self {
        Self {
            schema_version: sidecar.schema_version,
            projection_version: sidecar.projection_version,
            entity_subject_ref: &sidecar.entity_subject_ref,
            entity_subject_compact: &sidecar.entity_subject_compact,
            markdown_rel_path: &sidecar.markdown_rel_path,
            sidecar_rel_path: &sidecar.sidecar_rel_path,
            claims: sidecar
                .claims
                .iter()
                .map(LegacyV1ClaimChecksum::from)
                .collect(),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LegacyV1ClaimChecksum<'a> {
    semantic_identity: &'a ClaimSemanticIdentityV1,
    runtime_claim_id: &'a str,
    runtime_claim_version: u64,
    claim_text: &'a str,
    trust_band: &'a str,
    sensitivity: &'a str,
    lifecycle: &'a ClaimFileLifecycle,
    provenance_summary: &'a ClaimFileProvenanceSummary,
    feedback_rows: Vec<LegacyV1FeedbackRowChecksum<'a>>,
    contradiction_edges: Vec<LegacyV1ContradictionEdgeChecksum<'a>>,
    replay_status: &'a ClaimFileReplayStatus,
}

impl<'a> From<&'a ClaimFileClaim> for LegacyV1ClaimChecksum<'a> {
    fn from(claim: &'a ClaimFileClaim) -> Self {
        Self {
            semantic_identity: &claim.semantic_identity,
            runtime_claim_id: &claim.runtime_claim_id,
            runtime_claim_version: claim.runtime_claim_version,
            claim_text: &claim.claim_text,
            trust_band: &claim.trust_band,
            sensitivity: &claim.sensitivity,
            lifecycle: &claim.lifecycle,
            provenance_summary: &claim.provenance_summary,
            feedback_rows: claim
                .feedback_rows
                .iter()
                .map(LegacyV1FeedbackRowChecksum::from)
                .collect(),
            contradiction_edges: claim
                .contradiction_edges
                .iter()
                .map(LegacyV1ContradictionEdgeChecksum::from)
                .collect(),
            replay_status: &claim.replay_status,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LegacyV1FeedbackRowChecksum<'a> {
    action: &'a str,
    actor: &'a str,
    actor_id: &'a Option<String>,
    payload_json: &'a Option<serde_json::Value>,
    submitted_at: &'a str,
    applied_at: &'a Option<String>,
}

impl<'a> From<&'a ClaimFileFeedbackRow> for LegacyV1FeedbackRowChecksum<'a> {
    fn from(feedback: &'a ClaimFileFeedbackRow) -> Self {
        Self {
            action: &feedback.action,
            actor: &feedback.actor,
            actor_id: &feedback.actor_id,
            payload_json: &feedback.payload_json,
            submitted_at: &feedback.submitted_at,
            applied_at: &feedback.applied_at,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LegacyV1ContradictionEdgeChecksum<'a> {
    edge_id: &'a str,
    branch_kind: &'a str,
    role: &'a str,
    primary_runtime_claim_id: &'a str,
    contradicting_runtime_claim_id: &'a str,
    primary_semantic_identity: &'a Option<ClaimSemanticIdentityV1>,
    contradicting_semantic_identity: &'a Option<ClaimSemanticIdentityV1>,
    detected_at: &'a str,
    reconciliation_kind: &'a Option<String>,
    reconciliation_note: &'a Option<String>,
    reconciled_at: &'a Option<String>,
    winner_runtime_claim_id: &'a Option<String>,
    merged_runtime_claim_id: &'a Option<String>,
    replay_status: &'a ClaimFileReplayStatus,
}

impl<'a> From<&'a ClaimFileContradictionEdge> for LegacyV1ContradictionEdgeChecksum<'a> {
    fn from(edge: &'a ClaimFileContradictionEdge) -> Self {
        Self {
            edge_id: &edge.edge_id,
            branch_kind: &edge.branch_kind,
            role: &edge.role,
            primary_runtime_claim_id: &edge.primary_runtime_claim_id,
            contradicting_runtime_claim_id: &edge.contradicting_runtime_claim_id,
            primary_semantic_identity: &edge.primary_semantic_identity,
            contradicting_semantic_identity: &edge.contradicting_semantic_identity,
            detected_at: &edge.detected_at,
            reconciliation_kind: &edge.reconciliation_kind,
            reconciliation_note: &edge.reconciliation_note,
            reconciled_at: &edge.reconciled_at,
            winner_runtime_claim_id: &edge.winner_runtime_claim_id,
            merged_runtime_claim_id: &edge.merged_runtime_claim_id,
            replay_status: &edge.replay_status,
        }
    }
}

fn validate_replay_sidecar(sidecar: &ClaimFileSidecar) -> Result<(), RebuildError> {
    if !matches!(
        sidecar.schema_version,
        CLAIM_FILE_LEGACY_SIDECAR_SCHEMA_VERSION | CLAIM_FILE_SIDECAR_SCHEMA_VERSION
    ) {
        return Err(RebuildError::InvalidSidecar(
            "unsupported sidecar schema_version".to_string(),
        ));
    }
    let mut feedback_ids = HashSet::new();
    for claim in &sidecar.claims {
        if claim
            .semantic_identity
            .source_content_hash
            .as_deref()
            .is_none_or(str::is_empty)
        {
            return Err(RebuildError::InvalidSidecar(
                "sidecar semantic identity missing source_content_hash".to_string(),
            ));
        }
        for (feedback_index, feedback) in claim.feedback_rows.iter().enumerate() {
            feedback_action_from_slug(&feedback.action)?;
            canonical_feedback_submitted_at(&feedback.submitted_at)?;
            if sidecar.schema_version == CLAIM_FILE_LEGACY_SIDECAR_SCHEMA_VERSION
                && !feedback.feedback_id.trim().is_empty()
            {
                return Err(RebuildError::InvalidSidecar(
                    "legacy v1 sidecar feedback row cannot supply feedback_id".to_string(),
                ));
            }
            if feedback
                .actor_id
                .as_deref()
                .is_none_or(|actor_id| actor_id.trim().is_empty())
            {
                return Err(RebuildError::InvalidSidecar(
                    "sidecar feedback row missing stable actor_id".to_string(),
                ));
            }
            if sidecar.schema_version == CLAIM_FILE_SIDECAR_SCHEMA_VERSION
                && feedback.feedback_id.trim().is_empty()
            {
                return Err(RebuildError::InvalidSidecar(
                    "sidecar feedback row missing stable feedback_id".to_string(),
                ));
            }
            let event_id = sidecar_feedback_event_id(sidecar, claim, feedback, feedback_index)?;
            if !feedback_ids.insert(event_id) {
                return Err(RebuildError::InvalidSidecar(
                    "sidecar feedback row reused stable feedback_id".to_string(),
                ));
            }
        }
    }
    Ok(())
}

fn sidecar_feedback_events(
    sidecar: &ClaimFileSidecar,
) -> Result<HashMap<String, SidecarFeedbackReplayIdentity>, RebuildError> {
    let mut event_ids = HashMap::new();
    for claim in &sidecar.claims {
        for (feedback_index, feedback) in claim.feedback_rows.iter().enumerate() {
            let action = feedback_action_from_slug(&feedback.action)?;
            let payload_json = feedback_payload_json(feedback)?;
            let submitted_at = canonical_feedback_submitted_at(&feedback.submitted_at)?;
            let content_hash =
                feedback_content_hash(feedback, action, payload_json.as_deref(), &submitted_at)?;
            let event_id = sidecar_feedback_event_id(sidecar, claim, feedback, feedback_index)?;
            if event_ids
                .insert(
                    event_id,
                    SidecarFeedbackReplayIdentity {
                        action,
                        content_hash,
                    },
                )
                .is_some()
            {
                return Err(RebuildError::InvalidSidecar(
                    "sidecar feedback row reused stable feedback_id".to_string(),
                ));
            }
        }
    }
    Ok(event_ids)
}

fn sidecar_feedback_event_id(
    sidecar: &ClaimFileSidecar,
    claim: &ClaimFileClaim,
    feedback: &ClaimFileFeedbackRow,
    feedback_index: usize,
) -> Result<String, RebuildError> {
    if sidecar.schema_version == CLAIM_FILE_LEGACY_SIDECAR_SCHEMA_VERSION {
        if !feedback.feedback_id.trim().is_empty() {
            return Err(RebuildError::InvalidSidecar(
                "legacy v1 sidecar feedback row cannot supply feedback_id".to_string(),
            ));
        }
        return legacy_v1_sidecar_feedback_event_id(claim, feedback, feedback_index);
    }
    let explicit = feedback.feedback_id.trim();
    if !explicit.is_empty() {
        return Ok(explicit.to_string());
    }
    Err(RebuildError::InvalidSidecar(
        "sidecar feedback row missing stable feedback_id".to_string(),
    ))
}

fn legacy_v1_sidecar_feedback_event_id(
    claim: &ClaimFileClaim,
    feedback: &ClaimFileFeedbackRow,
    feedback_index: usize,
) -> Result<String, RebuildError> {
    let action = feedback_action_from_slug(&feedback.action)?;
    let payload_json = feedback_payload_json(feedback)?;
    let submitted_at = canonical_feedback_submitted_at(&feedback.submitted_at)?;
    let content_hash =
        feedback_content_hash(feedback, action, payload_json.as_deref(), &submitted_at)?;
    let mut hasher = Sha256::new();
    hasher.update(b"dailyos-claim-file-feedback-v1");
    hasher.update([0x1f]);
    hasher.update(claim.runtime_claim_id.as_bytes());
    hasher.update([0x1f]);
    hasher.update(claim.runtime_claim_version.to_string().as_bytes());
    hasher.update([0x1f]);
    hasher.update(claim.semantic_identity.dedup_key_components_hash.as_bytes());
    hasher.update([0x1f]);
    hasher.update(feedback_index.to_string().as_bytes());
    hasher.update([0x1f]);
    hasher.update(submitted_at.as_bytes());
    hasher.update([0x1f]);
    hasher.update(content_hash.as_bytes());
    Ok(format!("legacy-v1:{:x}", hasher.finalize()))
}

fn feedback_payload_json(feedback: &ClaimFileFeedbackRow) -> Result<Option<String>, RebuildError> {
    feedback
        .payload_json
        .as_ref()
        .map(serde_json::to_string)
        .transpose()
        .map_err(RebuildError::Json)
}

fn feedback_content_hash(
    feedback: &ClaimFileFeedbackRow,
    action: FeedbackAction,
    payload_json: Option<&str>,
    submitted_at: &str,
) -> Result<String, RebuildError> {
    claim_feedback_replay_content_hash(
        action,
        &feedback.actor,
        feedback.actor_id.as_deref(),
        payload_json,
        submitted_at,
    )
    .map_err(|error| RebuildError::InvalidSidecar(error.to_string()))
}

fn canonical_feedback_submitted_at(submitted_at: &str) -> Result<String, RebuildError> {
    if submitted_at.trim().is_empty() {
        return Err(RebuildError::InvalidSidecar(
            "sidecar feedback row missing submitted_at".to_string(),
        ));
    }
    let parsed = DateTime::parse_from_rfc3339(submitted_at.trim()).map_err(|error| {
        RebuildError::InvalidSidecar(format!(
            "sidecar feedback row submitted_at must be RFC3339: {error}"
        ))
    })?;
    Ok(parsed
        .with_timezone(&Utc)
        .to_rfc3339_opts(SecondsFormat::AutoSi, true))
}

fn replay_feedback_event(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    scope: ReplayScope<'_>,
    source_claim: SidecarClaimRef<'_>,
    resolved_claim_id: &str,
    feedback: SidecarFeedbackRef<'_>,
    sidecar_feedback_events: &HashMap<String, SidecarFeedbackReplayIdentity>,
) -> Result<ReplayEventOutcome, RebuildError> {
    let payload_json = feedback_payload_json(feedback.row)?;
    let submitted_at = canonical_feedback_submitted_at(&feedback.row.submitted_at)?;
    let content_hash = feedback_content_hash(
        feedback.row,
        feedback.action,
        payload_json.as_deref(),
        &submitted_at,
    )?;
    let preserve_legacy_unset_checksum = read_existing_replay_event(db, feedback.event_id)?
        .as_ref()
        .is_some_and(|existing| {
            existing.status == "claimed"
                && existing.sidecar_checksum == LEGACY_V283_UNSET_SIDECAR_CHECKSUM
        });
    if let ReplayEventClaim::ExistingTerminal {
        status,
        reason_code,
        applied_feedback_id,
    } = claim_replay_event(
        db,
        ClaimReplayJournalInput {
            scope,
            source_claim,
            resolved_claim_id: Some(resolved_claim_id),
            sidecar_event_id: feedback.event_id,
            action: &feedback.row.action,
            reason_code: None,
            feedback_content_hash: &content_hash,
        },
    )? {
        return Ok(ReplayEventOutcome {
            sidecar_event_id: feedback.event_id.to_string(),
            source_runtime_claim_id: source_claim.runtime_claim_id.to_string(),
            resolved_claim_id: Some(resolved_claim_id.to_string()),
            action: feedback.row.action.clone(),
            status,
            reason_code,
            applied_feedback_id,
        });
    }

    let replay_input = ClaimFeedbackReplayInput {
        feedback: ClaimFeedbackInput {
            claim_id: resolved_claim_id.to_string(),
            action: feedback.action,
            actor: feedback.row.actor.clone(),
            actor_id: feedback.row.actor_id.clone(),
            payload_json: payload_json.clone(),
        },
        replay_event_id: feedback.event_id.to_string(),
        submitted_at: submitted_at.clone(),
    };
    let replay_feedback_already_recorded =
        match recorded_claim_feedback_replay_outcome(db, &replay_input) {
            Ok(outcome) => outcome.is_some(),
            Err(error) if claim_replay_error_is_terminal(&error) => {
                if let Some(existing_feedback) = existing_replay_feedback_for_event(
                    db,
                    feedback.event_id,
                    resolved_claim_id,
                    feedback.action,
                    &content_hash,
                )? {
                    repair_claim_feedback_recorded_signal(ctx, db, &existing_feedback.feedback_id)
                        .map_err(|error| RebuildError::ClaimReplay(error.to_string()))?;
                    let marked = mark_replay_event_already_applied_with_feedback(
                        db,
                        feedback.event_id,
                        &existing_feedback,
                        preserve_legacy_unset_checksum,
                    )?;
                    return replay_event_outcome_from_existing(
                        feedback.event_id,
                        source_claim,
                        Some(resolved_claim_id),
                        &feedback.row.action,
                        marked,
                    );
                }
                let reason_code = "claim_feedback_replay_failed";
                let reason_detail_hash = reason_detail_hash(&error.to_string());
                let marked = mark_replay_event_terminal(
                    db,
                    feedback.event_id,
                    ReplayEventStatus::Failed.as_str(),
                    Some(reason_code),
                    Some(&reason_detail_hash),
                    None,
                    Some(&content_hash),
                )?;
                return replay_event_outcome_from_existing(
                    feedback.event_id,
                    source_claim,
                    Some(resolved_claim_id),
                    &feedback.row.action,
                    marked,
                );
            }
            Err(error) => return Err(RebuildError::ClaimReplay(error.to_string())),
        };

    if !replay_feedback_already_recorded
        && replay_feedback_is_stale(
            db,
            resolved_claim_id,
            &submitted_at,
            sidecar_feedback_events,
        )?
    {
        let reason_code = "stale_replay_watermark";
        let marked = mark_replay_event_terminal(
            db,
            feedback.event_id,
            ReplayEventStatus::Failed.as_str(),
            Some(reason_code),
            None,
            None,
            Some(&content_hash),
        )?;
        return replay_event_outcome_from_existing(
            feedback.event_id,
            source_claim,
            Some(resolved_claim_id),
            &feedback.row.action,
            marked,
        );
    }

    let outcome = match record_claim_feedback_replay(ctx, db, replay_input) {
        Ok(outcome) => outcome,
        Err(error) => {
            if !claim_replay_error_is_terminal(&error) {
                return Err(RebuildError::ClaimReplay(error.to_string()));
            }
            let reason_code = "claim_feedback_replay_failed";
            let reason_detail_hash = reason_detail_hash(&error.to_string());
            let marked = mark_replay_event_terminal(
                db,
                feedback.event_id,
                ReplayEventStatus::Failed.as_str(),
                Some(reason_code),
                Some(&reason_detail_hash),
                None,
                Some(&content_hash),
            )?;
            return replay_event_outcome_from_existing(
                feedback.event_id,
                source_claim,
                Some(resolved_claim_id),
                &feedback.row.action,
                marked,
            );
        }
    };

    let status = if outcome.applied_at_pending {
        ReplayEventStatus::Applied
    } else {
        ReplayEventStatus::AlreadyApplied
    };
    let marked = mark_replay_event_terminal(
        db,
        feedback.event_id,
        status.as_str(),
        None,
        None,
        Some(&outcome.feedback_id),
        Some(&content_hash),
    )?;
    replay_event_outcome_from_existing(
        feedback.event_id,
        source_claim,
        Some(resolved_claim_id),
        &feedback.row.action,
        marked,
    )
}

fn record_orphan_replay_event(
    db: &ActionDb,
    scope: ReplayScope<'_>,
    source_claim: SidecarClaimRef<'_>,
    feedback: SidecarFeedbackRef<'_>,
    status: ReplayEventStatus,
    reason_code: &str,
) -> Result<ReplayEventOutcome, RebuildError> {
    let payload_json = feedback_payload_json(feedback.row)?;
    let submitted_at = canonical_feedback_submitted_at(&feedback.row.submitted_at)?;
    let content_hash = feedback_content_hash(
        feedback.row,
        feedback.action,
        payload_json.as_deref(),
        &submitted_at,
    )?;
    if let ReplayEventClaim::ExistingTerminal {
        status,
        reason_code,
        applied_feedback_id,
    } = claim_replay_event(
        db,
        ClaimReplayJournalInput {
            scope,
            source_claim,
            resolved_claim_id: None,
            sidecar_event_id: feedback.event_id,
            action: &feedback.row.action,
            reason_code: Some(reason_code),
            feedback_content_hash: &content_hash,
        },
    )? {
        return Ok(ReplayEventOutcome {
            sidecar_event_id: feedback.event_id.to_string(),
            source_runtime_claim_id: source_claim.runtime_claim_id.to_string(),
            resolved_claim_id: None,
            action: feedback.row.action.clone(),
            status,
            reason_code,
            applied_feedback_id,
        });
    }
    mark_replay_event_terminal(
        db,
        feedback.event_id,
        status.as_str(),
        Some(reason_code),
        None,
        None,
        Some(&content_hash),
    )?;
    Ok(ReplayEventOutcome {
        sidecar_event_id: feedback.event_id.to_string(),
        source_runtime_claim_id: source_claim.runtime_claim_id.to_string(),
        resolved_claim_id: None,
        action: feedback.row.action.clone(),
        status,
        reason_code: Some(reason_code.to_string()),
        applied_feedback_id: None,
    })
}

fn replay_feedback_is_stale(
    db: &ActionDb,
    resolved_claim_id: &str,
    submitted_at: &str,
    sidecar_feedback_events: &HashMap<String, SidecarFeedbackReplayIdentity>,
) -> Result<bool, RebuildError> {
    let replay_submitted_at = DateTime::parse_from_rfc3339(submitted_at)
        .map_err(|error| {
            RebuildError::InvalidSidecar(format!(
                "sidecar feedback row submitted_at must be RFC3339: {error}"
            ))
        })?
        .with_timezone(&Utc);

    let matching_sidecar_replay_event_ids =
        matching_sidecar_replay_event_ids(db, resolved_claim_id, sidecar_feedback_events)?;

    let mut stmt = db.conn_ref().prepare(
        "SELECT submitted_at, replay_event_id
           FROM claim_feedback
          WHERE claim_id = ?1",
    )?;
    let feedback_rows = stmt.query_map(params![resolved_claim_id], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
    })?;
    for stored in feedback_rows {
        let (stored, replay_event_id) = stored?;
        if replay_event_id
            .as_deref()
            .is_some_and(|event_id| matching_sidecar_replay_event_ids.contains(event_id))
        {
            continue;
        }
        if stored_timestamp_after(&stored, replay_submitted_at, "claim_feedback.submitted_at")? {
            return Ok(true);
        }
    }

    let mut stmt = db.conn_ref().prepare(
        "SELECT created_at, correction_event_log_id
           FROM version_events
          WHERE claim_id = ?1
            AND previous_version IS NOT NULL
            AND event_kind IN (
                'claim.updated',
                'claim.corrected',
                'claim.superseded',
                'claim.tombstoned',
                'claim.conflict_detected'
            )",
    )?;
    let version_rows = stmt.query_map(params![resolved_claim_id], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
    })?;
    for stored in version_rows {
        let (stored, correction_event_log_id) = stored?;
        if correction_event_log_id
            .as_deref()
            .is_some_and(|event_id| matching_sidecar_replay_event_ids.contains(event_id))
        {
            continue;
        }
        if stored_timestamp_after(&stored, replay_submitted_at, "version_events.created_at")? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn matching_sidecar_replay_event_ids(
    db: &ActionDb,
    resolved_claim_id: &str,
    sidecar_feedback_events: &HashMap<String, SidecarFeedbackReplayIdentity>,
) -> Result<HashSet<String>, RebuildError> {
    let mut matching_event_ids = HashSet::new();
    let mut stmt = db.conn_ref().prepare(
        "SELECT replay_event_id, feedback_type, actor, actor_id, payload_json, submitted_at
           FROM claim_feedback
          WHERE claim_id = ?1
            AND replay_event_id IS NOT NULL",
    )?;
    let rows = stmt.query_map(params![resolved_claim_id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, Option<String>>(3)?,
            row.get::<_, Option<String>>(4)?,
            row.get::<_, String>(5)?,
        ))
    })?;
    for row in rows {
        let (event_id, feedback_type, actor, actor_id, payload_json, submitted_at) = row?;
        let Some(expected) = sidecar_feedback_events.get(&event_id) else {
            continue;
        };
        if feedback_type != expected.action.as_str() {
            continue;
        }
        let content_hash = claim_feedback_replay_content_hash(
            expected.action,
            &actor,
            actor_id.as_deref(),
            payload_json.as_deref(),
            &submitted_at,
        )
        .map_err(|error| RebuildError::ClaimReplay(error.to_string()))?;
        if content_hash == expected.content_hash {
            matching_event_ids.insert(event_id);
        }
    }
    Ok(matching_event_ids)
}

fn stored_timestamp_after(
    stored: &str,
    replay_submitted_at: DateTime<Utc>,
    field: &str,
) -> Result<bool, RebuildError> {
    let stored_at = DateTime::parse_from_rfc3339(stored)
        .map_err(|error| RebuildError::ClaimReplay(format!("{field} is not RFC3339: {error}")))?
        .with_timezone(&Utc);
    Ok(stored_at > replay_submitted_at)
}

fn claim_replay_error_is_terminal(error: &ClaimError) -> bool {
    matches!(
        error,
        ClaimError::UnknownClaimId(_)
            | ClaimError::ClaimNotFound(_)
            | ClaimError::UnsupportedClaimType { .. }
            | ClaimError::InvalidFeedback(_)
            | ClaimError::InvalidActor(_)
            | ClaimError::StaleVersion { .. }
            | ClaimError::InflatedVersion { .. }
            | ClaimError::MissingExpectedClaimVersion { .. }
            | ClaimError::ActorClassNotAllowed { .. }
            | ClaimError::ActorNotPermittedForClaimType { .. }
            | ClaimError::TombstonedPreGate
    )
}

fn replay_event_outcome_from_existing(
    sidecar_event_id: &str,
    source_claim: SidecarClaimRef<'_>,
    resolved_claim_id: Option<&str>,
    action: &str,
    existing: ExistingReplayEvent,
) -> Result<ReplayEventOutcome, RebuildError> {
    let status = match existing.status.as_str() {
        "applied" => ReplayEventStatus::Applied,
        "already_applied" => ReplayEventStatus::AlreadyApplied,
        "orphan_missing" => ReplayEventStatus::OrphanMissing,
        "orphan_ambiguous" => ReplayEventStatus::OrphanAmbiguous,
        "failed" => ReplayEventStatus::Failed,
        other => {
            return Err(RebuildError::InvalidSidecar(format!(
                "unknown replay event status `{other}`"
            )))
        }
    };
    Ok(ReplayEventOutcome {
        sidecar_event_id: sidecar_event_id.to_string(),
        source_runtime_claim_id: source_claim.runtime_claim_id.to_string(),
        resolved_claim_id: existing
            .resolved_claim_id
            .or_else(|| resolved_claim_id.map(str::to_string)),
        action: action.to_string(),
        status,
        reason_code: existing.reason_code,
        applied_feedback_id: existing.applied_feedback_id,
    })
}

fn resolve_claim_by_semantic_identity(
    db: &ActionDb,
    identity: &ClaimSemanticIdentityV1,
) -> Result<Resolution, RebuildError> {
    let subject_kind = identity
        .subject_ref
        .get("kind")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            RebuildError::InvalidSidecar("semantic identity missing subject kind".to_string())
        })?;
    let subject_id = identity
        .subject_ref
        .get("id")
        .and_then(serde_json::Value::as_str);

    let mut stmt = db.conn_ref().prepare(
        "SELECT id
         FROM intelligence_claims
         WHERE claim_type = ?1
           AND COALESCE(field_path, '') = COALESCE(?2, '')
           AND json_valid(subject_ref) = 1
           AND lower(json_extract(subject_ref, '$.kind')) = lower(?3)
           AND (?4 IS NULL OR json_extract(subject_ref, '$.id') = ?4)
         ORDER BY created_at ASC, id ASC",
    )?;
    let rows = stmt.query_map(
        params![
            &identity.claim_type,
            identity.field_path.as_deref(),
            subject_kind,
            subject_id,
        ],
        |row| row.get::<_, String>(0),
    )?;
    let mut candidates = Vec::new();
    for row in rows {
        let claim_id = row?;
        if let Some(claim) = load_claim_by_id(db.conn_ref(), &claim_id)
            .map_err(|error| RebuildError::ClaimReplay(error.to_string()))?
        {
            let candidate_identity =
                semantic_identity_for_loaded_claim(&claim).map_err(RebuildError::ClaimReplay)?;
            if candidate_identity.dedup_key_components_hash == identity.dedup_key_components_hash {
                candidates.push((claim_id, claim));
            }
        }
    }
    if candidates.is_empty() {
        return Ok(Resolution::Missing);
    }
    if candidates.len() == 1 {
        let (claim_id, claim) = candidates.remove(0);
        return if claim_matches_identity_provenance(&claim, identity) {
            Ok(Resolution::Resolved(claim_id))
        } else {
            Ok(Resolution::Missing)
        };
    }

    let narrowed = candidates
        .iter()
        .filter(|(_, claim)| claim_matches_identity_provenance(claim, identity))
        .collect::<Vec<_>>();
    if narrowed.len() == 1 {
        return Ok(Resolution::Resolved(narrowed[0].0.clone()));
    }
    if narrowed.is_empty() {
        return Ok(Resolution::Missing);
    }

    Ok(Resolution::Ambiguous {
        candidate_count: narrowed.len(),
    })
}

fn read_existing_replay_event(
    db: &ActionDb,
    sidecar_event_id: &str,
) -> Result<Option<ExistingReplayEvent>, RebuildError> {
    db.conn_ref()
        .query_row(
            "SELECT sidecar_schema_version, sidecar_checksum, source_runtime_claim_id, resolved_claim_id,
                    action, status, reason_code, semantic_identity_hash,
                    feedback_content_hash, applied_feedback_id
             FROM rebuild_correction_replay_events
             WHERE sidecar_event_id = ?1",
            params![sidecar_event_id],
            |row| {
                let schema_version = row.get::<_, i64>(0)?;
                Ok(ExistingReplayEvent {
                    sidecar_schema_version: schema_version as u32,
                    sidecar_checksum: row.get(1)?,
                    source_runtime_claim_id: row.get(2)?,
                    resolved_claim_id: row.get(3)?,
                    action: row.get(4)?,
                    status: row.get(5)?,
                    reason_code: row.get(6)?,
                    semantic_identity_hash: row.get(7)?,
                    feedback_content_hash: row.get(8)?,
                    applied_feedback_id: row.get(9)?,
                })
            },
        )
        .optional()
        .map_err(RebuildError::Db)
}

fn claim_replay_event(
    db: &ActionDb,
    input: ClaimReplayJournalInput<'_>,
) -> Result<ReplayEventClaim, RebuildError> {
    if let Some(existing) = read_existing_replay_event(db, input.sidecar_event_id)? {
        validate_existing_replay_event(&existing, &input)?;
        if existing_is_recoverable_orphan(&existing) && input.resolved_claim_id.is_some() {
            reclaim_orphan_replay_event(db, &existing, &input)?;
            return Ok(ReplayEventClaim::Claimed);
        }
        increment_claimed_attempt_count(db, input.sidecar_event_id)?;
        return replay_event_claim_from_existing(existing);
    }

    let now = Utc::now().to_rfc3339();
    let inserted = db.conn_ref().execute(
        "INSERT INTO rebuild_correction_replay_events (
            sidecar_event_id, run_id, sidecar_schema_version, source_runtime_claim_id,
            sidecar_checksum, resolved_claim_id, action, status, reason_code, semantic_identity_hash,
            feedback_content_hash, attempt_count, claimed_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'claimed', ?8, ?9, ?10, 1, ?11, ?11)
         ON CONFLICT(sidecar_event_id) DO NOTHING",
        params![
            input.sidecar_event_id,
            input.scope.run_id,
            input.scope.sidecar_schema_version as i64,
            input.source_claim.runtime_claim_id,
            input.scope.sidecar_checksum,
            input.resolved_claim_id,
            input.action,
            input.reason_code,
            &input.source_claim.identity.dedup_key_components_hash,
            input.feedback_content_hash,
            &now,
        ],
    )?;
    let existing = read_existing_replay_event(db, input.sidecar_event_id)?.ok_or_else(|| {
        RebuildError::InvalidSidecar("replay event claim was not persisted".to_string())
    })?;
    validate_existing_replay_event(&existing, &input)?;
    if existing_is_recoverable_orphan(&existing) && input.resolved_claim_id.is_some() {
        reclaim_orphan_replay_event(db, &existing, &input)?;
        return Ok(ReplayEventClaim::Claimed);
    }
    if inserted == 0 {
        increment_claimed_attempt_count(db, input.sidecar_event_id)?;
    }
    replay_event_claim_from_existing(existing)
}

fn increment_claimed_attempt_count(
    db: &ActionDb,
    sidecar_event_id: &str,
) -> Result<(), RebuildError> {
    db.conn_ref().execute(
        "UPDATE rebuild_correction_replay_events
         SET attempt_count = attempt_count + 1,
             updated_at = ?1
         WHERE sidecar_event_id = ?2
           AND status = 'claimed'",
        params![Utc::now().to_rfc3339(), sidecar_event_id],
    )?;
    Ok(())
}

fn validate_existing_replay_event(
    existing: &ExistingReplayEvent,
    input: &ClaimReplayJournalInput<'_>,
) -> Result<(), RebuildError> {
    if existing.sidecar_schema_version != input.scope.sidecar_schema_version
        || existing.source_runtime_claim_id != input.source_claim.runtime_claim_id
        || existing.action != input.action
        || existing.semantic_identity_hash != input.source_claim.identity.dedup_key_components_hash
    {
        return Err(RebuildError::InvalidSidecar(
            "sidecar replay event already recorded for a different target or feedback content"
                .to_string(),
        ));
    }
    if existing.sidecar_checksum != input.scope.sidecar_checksum
        && existing.sidecar_checksum != LEGACY_V283_UNSET_SIDECAR_CHECKSUM
    {
        return Err(RebuildError::InvalidSidecar(
            "sidecar replay event already recorded for a different sidecar checksum or feedback content"
                .to_string(),
        ));
    }
    if existing.resolved_claim_id.as_deref() != input.resolved_claim_id
        && !(existing_is_recoverable_orphan(existing) && input.resolved_claim_id.is_some())
    {
        return Err(RebuildError::InvalidSidecar(
            "sidecar replay event already recorded for a different target or feedback content"
                .to_string(),
        ));
    }
    let content_hash_matches = existing.feedback_content_hash == input.feedback_content_hash
        || (existing.feedback_content_hash == LEGACY_V282_UNSET_FEEDBACK_CONTENT_HASH
            && (existing.status == "claimed" || existing_is_recoverable_orphan(existing)));
    if !content_hash_matches {
        return Err(RebuildError::InvalidSidecar(
            "sidecar replay event already recorded for a different target or feedback content"
                .to_string(),
        ));
    }
    Ok(())
}

fn existing_is_recoverable_orphan(existing: &ExistingReplayEvent) -> bool {
    matches!(
        existing.status.as_str(),
        "orphan_missing" | "orphan_ambiguous"
    ) && existing.resolved_claim_id.is_none()
}

const LEGACY_V283_UNSET_SIDECAR_CHECKSUM: &str = "legacy-v283-unset";

fn reclaim_orphan_replay_event(
    db: &ActionDb,
    existing: &ExistingReplayEvent,
    input: &ClaimReplayJournalInput<'_>,
) -> Result<(), RebuildError> {
    db.conn_ref().execute(
        "UPDATE rebuild_correction_replay_events
         SET status = 'claimed',
             resolved_claim_id = ?1,
             reason_code = NULL,
             reason_detail_hash = NULL,
             applied_feedback_id = NULL,
             attempt_count = attempt_count + 1,
             updated_at = ?2
         WHERE sidecar_event_id = ?3
           AND status IN ('orphan_missing', 'orphan_ambiguous')
           AND resolved_claim_id IS NULL",
        params![
            input.resolved_claim_id,
            Utc::now().to_rfc3339(),
            input.sidecar_event_id,
        ],
    )?;
    log::debug!(
        "reclaimed orphaned replay event sidecar_event_id={} prior_status={}",
        input.sidecar_event_id,
        existing.status
    );
    Ok(())
}

fn replay_event_claim_from_existing(
    existing: ExistingReplayEvent,
) -> Result<ReplayEventClaim, RebuildError> {
    Ok(match existing.status.as_str() {
        "claimed" => ReplayEventClaim::Claimed,
        "applied" | "already_applied" => ReplayEventClaim::ExistingTerminal {
            status: ReplayEventStatus::AlreadyApplied,
            reason_code: existing.reason_code,
            applied_feedback_id: existing.applied_feedback_id,
        },
        "orphan_missing" => ReplayEventClaim::ExistingTerminal {
            status: ReplayEventStatus::OrphanMissing,
            reason_code: existing.reason_code,
            applied_feedback_id: existing.applied_feedback_id,
        },
        "orphan_ambiguous" => ReplayEventClaim::ExistingTerminal {
            status: ReplayEventStatus::OrphanAmbiguous,
            reason_code: existing.reason_code,
            applied_feedback_id: existing.applied_feedback_id,
        },
        "failed" => ReplayEventClaim::ExistingTerminal {
            status: ReplayEventStatus::Failed,
            reason_code: existing.reason_code,
            applied_feedback_id: existing.applied_feedback_id,
        },
        other => {
            return Err(RebuildError::InvalidSidecar(format!(
                "unknown replay event status `{other}`"
            )))
        }
    })
}

struct ExistingReplayFeedback {
    feedback_id: String,
    content_hash: String,
}

fn existing_replay_feedback_for_event(
    db: &ActionDb,
    replay_event_id: &str,
    resolved_claim_id: &str,
    action: FeedbackAction,
    expected_content_hash: &str,
) -> Result<Option<ExistingReplayFeedback>, RebuildError> {
    let row: Option<(
        String,
        String,
        String,
        String,
        Option<String>,
        Option<String>,
        String,
    )> = db
        .conn_ref()
        .query_row(
            "SELECT id, claim_id, feedback_type, actor, actor_id, payload_json, submitted_at
               FROM claim_feedback
              WHERE replay_event_id = ?1",
            params![replay_event_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                ))
            },
        )
        .optional()?;
    let Some((feedback_id, claim_id, feedback_type, actor, actor_id, payload_json, submitted_at)) =
        row
    else {
        return Ok(None);
    };
    if claim_id != resolved_claim_id || feedback_type != action.as_str() {
        return Ok(None);
    }
    let content_hash = claim_feedback_replay_content_hash(
        action,
        &actor,
        actor_id.as_deref(),
        payload_json.as_deref(),
        &submitted_at,
    )
    .map_err(|error| RebuildError::ClaimReplay(error.to_string()))?;
    if content_hash != expected_content_hash {
        return Ok(None);
    }
    Ok(Some(ExistingReplayFeedback {
        feedback_id,
        content_hash,
    }))
}

fn mark_replay_event_already_applied_with_feedback(
    db: &ActionDb,
    sidecar_event_id: &str,
    feedback: &ExistingReplayFeedback,
    preserve_legacy_unset_checksum: bool,
) -> Result<ExistingReplayEvent, RebuildError> {
    let now = Utc::now().to_rfc3339();
    let sidecar_checksum =
        preserve_legacy_unset_checksum.then_some(LEGACY_V283_UNSET_SIDECAR_CHECKSUM);
    db.conn_ref().execute(
        "UPDATE rebuild_correction_replay_events
         SET status = 'already_applied',
             reason_code = NULL,
             reason_detail_hash = NULL,
             applied_feedback_id = ?2,
             feedback_content_hash = ?3,
             sidecar_checksum = COALESCE(?5, sidecar_checksum),
             applied_at = ?4,
             updated_at = ?4
         WHERE sidecar_event_id = ?1
           AND status = 'claimed'",
        params![
            sidecar_event_id,
            &feedback.feedback_id,
            &feedback.content_hash,
            &now,
            sidecar_checksum,
        ],
    )?;
    read_existing_replay_event(db, sidecar_event_id)?.ok_or_else(|| {
        RebuildError::InvalidSidecar(format!(
            "replay event disappeared before already-applied recovery: {sidecar_event_id}"
        ))
    })
}

fn mark_replay_event_terminal(
    db: &ActionDb,
    sidecar_event_id: &str,
    status: &str,
    reason_code: Option<&str>,
    reason_detail_hash: Option<&str>,
    applied_feedback_id: Option<&str>,
    feedback_content_hash: Option<&str>,
) -> Result<ExistingReplayEvent, RebuildError> {
    let now = Utc::now().to_rfc3339();
    db.conn_ref().execute(
        "UPDATE rebuild_correction_replay_events
         SET status = ?1,
             reason_code = COALESCE(?2, reason_code),
             reason_detail_hash = COALESCE(?3, reason_detail_hash),
             applied_feedback_id = COALESCE(?4, applied_feedback_id),
             feedback_content_hash = COALESCE(?5, feedback_content_hash),
             applied_at = CASE WHEN ?1 IN ('applied', 'already_applied') THEN ?6 ELSE applied_at END,
             updated_at = ?6
         WHERE sidecar_event_id = ?7
           AND NOT (
             status IN ('applied', 'already_applied')
             AND ?1 = 'failed'
           )",
        params![
            status,
            reason_code,
            reason_detail_hash,
            applied_feedback_id,
            feedback_content_hash,
            &now,
            sidecar_event_id
        ],
    )?;
    read_existing_replay_event(db, sidecar_event_id)?.ok_or_else(|| {
        RebuildError::InvalidSidecar(format!(
            "replay event disappeared before terminal mark: {sidecar_event_id}"
        ))
    })
}

fn claim_matches_identity_provenance(
    claim: &abilities_runtime::types::IntelligenceClaim,
    identity: &ClaimSemanticIdentityV1,
) -> bool {
    claim.source_ref == identity.source_ref
        && claim.data_source == identity.data_source
        && claim.observed_at == identity.observed_at
        && claim.source_asof == identity.source_asof
        && identity
            .source_content_hash
            .as_deref()
            .is_some_and(|hash| !hash.is_empty() && source_content_hash_for_claim(claim) == hash)
}

fn reason_detail_hash(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn feedback_action_from_slug(value: &str) -> Result<FeedbackAction, RebuildError> {
    match value.trim() {
        "confirm_current" => Ok(FeedbackAction::ConfirmCurrent),
        "mark_outdated" => Ok(FeedbackAction::MarkOutdated),
        "mark_false" => Ok(FeedbackAction::MarkFalse),
        "wrong_subject" => Ok(FeedbackAction::WrongSubject),
        "wrong_source" => Ok(FeedbackAction::WrongSource),
        "cannot_verify" => Ok(FeedbackAction::CannotVerify),
        "needs_nuance" => Ok(FeedbackAction::NeedsNuance),
        "surface_inappropriate" => Ok(FeedbackAction::SurfaceInappropriate),
        "not_relevant_here" => Ok(FeedbackAction::NotRelevantHere),
        "merge_intent" => Ok(FeedbackAction::MergeIntent),
        other => Err(RebuildError::InvalidSidecar(format!(
            "unsupported feedback action `{other}`"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_utils::test_db;
    use crate::services::claim_files::{
        ClaimFileClaim, ClaimFileContradictionEdge, ClaimFileLifecycle, ClaimFileProvenanceSummary,
        ClaimFileReplayStatus, CLAIM_FILE_PROJECTION_VERSION, CLAIM_FILE_SIDECAR_SCHEMA_VERSION,
    };
    use crate::services::claims::{
        commit_claim, record_claim_feedback, record_claim_feedback_replay, ClaimFeedbackInput,
        ClaimFeedbackReplayInput, ClaimProposal, CommittedClaim, TombstoneSpec,
    };
    use crate::services::context::{ExternalClients, FixedClock, SeedableRng, ServiceContext};
    use abilities_runtime::types::{ClaimSensitivity, TemporalScope};

    const TS: &str = "2026-06-05T12:00:00Z";
    const REPLAYED_FEEDBACK_TS: &str = "2026-05-01T08:30:00Z";
    const SUBJECT: &str = r#"{"kind":"account","id":"acct-1"}"#;

    fn ctx_parts() -> (FixedClock, SeedableRng, ExternalClients) {
        let now = chrono::DateTime::parse_from_rfc3339(TS)
            .expect("timestamp")
            .with_timezone(&Utc);
        (
            FixedClock::new(now),
            SeedableRng::new(9),
            ExternalClients::default(),
        )
    }

    fn proposal(text: &str) -> ClaimProposal {
        ClaimProposal {
            id: None,
            expected_claim_version: None,
            subject_ref: SUBJECT.to_string(),
            claim_type: "risk".to_string(),
            field_path: Some("health.risk".to_string()),
            topic_key: None,
            text: text.to_string(),
            actor: "agent:test".to_string(),
            data_source: "unit_test".to_string(),
            source_ref: Some("fixture://source-1".to_string()),
            source_asof: Some(TS.to_string()),
            observed_at: TS.to_string(),
            provenance_json: "{}".to_string(),
            metadata_json: None,
            thread_id: None,
            temporal_scope: Some(TemporalScope::State),
            sensitivity: Some(ClaimSensitivity::Internal),
            supersedes: None,
            tombstone: None,
        }
    }

    fn inserted_claim_id(outcome: CommittedClaim) -> String {
        match outcome {
            CommittedClaim::Inserted { claim }
            | CommittedClaim::Reinforced { claim, .. }
            | CommittedClaim::Tombstoned { claim } => claim.id,
            CommittedClaim::Forked { new_claim_id, .. } => new_claim_id,
        }
    }

    fn seed_account(db: &ActionDb) {
        db.conn_ref()
            .execute(
                "INSERT INTO accounts (id, name, updated_at) VALUES (?1, ?2, ?3)",
                params!["acct-1", "Account 1", TS],
            )
            .expect("seed account");
    }

    fn replay_status() -> ClaimFileReplayStatus {
        ClaimFileReplayStatus {
            status: "projected".to_string(),
            reason: None,
            last_attempted_at: None,
        }
    }

    fn sidecar_for_claim(
        db: &ActionDb,
        fresh_claim_id: &str,
        source_runtime_claim_id: &str,
        feedback_id: &str,
    ) -> ClaimFileSidecar {
        let fresh_claim = load_claim_by_id(db.conn_ref(), fresh_claim_id)
            .expect("load claim")
            .expect("claim exists");
        let mut identity =
            semantic_identity_for_loaded_claim(&fresh_claim).expect("semantic identity");
        identity.runtime_claim_id = source_runtime_claim_id.to_string();
        identity.runtime_claim_version = 4;
        let subject = serde_json::json!({"kind":"account","id":"acct-1"});
        ClaimFileSidecar {
            schema_version: CLAIM_FILE_SIDECAR_SCHEMA_VERSION,
            projection_version: CLAIM_FILE_PROJECTION_VERSION,
            entity_subject_ref: subject,
            entity_subject_compact: r#"{"id":"acct-1","kind":"account"}"#.to_string(),
            markdown_rel_path: "_dailyos_claims/account/acct-1/claims.md".to_string(),
            sidecar_rel_path: "_dailyos_claims/account/acct-1/claims.corrections.json".to_string(),
            claims: vec![ClaimFileClaim {
                semantic_identity: identity,
                runtime_claim_id: source_runtime_claim_id.to_string(),
                runtime_claim_version: 4,
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
                    source_asof: Some(TS.to_string()),
                    observed_at: TS.to_string(),
                },
                feedback_rows: vec![ClaimFileFeedbackRow {
                    feedback_id: feedback_id.to_string(),
                    action: "cannot_verify".to_string(),
                    actor: "user".to_string(),
                    actor_id: Some("user-fixture".to_string()),
                    payload_json: None,
                    submitted_at: TS.to_string(),
                    applied_at: None,
                }],
                contradiction_edges: Vec::<ClaimFileContradictionEdge>::new(),
                superseded_by_semantic_identity: None,
                replay_status: replay_status(),
            }],
        }
    }

    fn sidecar_checksum(sidecar: &ClaimFileSidecar) -> String {
        let sidecar_json = serde_json::to_string_pretty(sidecar).expect("serialize sidecar");
        let mut hasher = Sha256::new();
        hasher.update(sidecar_json.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    fn authorize_sidecar(db: &ActionDb, sidecar: &ClaimFileSidecar) -> String {
        let checksum = sidecar_checksum(sidecar);
        authorize_sidecar_with_checksum(db, sidecar, &checksum);
        checksum
    }

    fn authorize_sidecar_with_checksum(db: &ActionDb, sidecar: &ClaimFileSidecar, checksum: &str) {
        let run_id = format!("projection-run-{}", &checksum[..12]);
        db.conn_ref()
            .execute(
                "INSERT OR IGNORE INTO claim_file_projection_path_bindings (
                    markdown_rel_path, sidecar_rel_path, entity_subject_compact,
                    projection_root, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, '_dailyos_claims', ?4, ?4)",
                params![
                    &sidecar.markdown_rel_path,
                    &sidecar.sidecar_rel_path,
                    &sidecar.entity_subject_compact,
                    TS,
                ],
            )
            .expect("insert path binding");
        db.conn_ref()
            .execute(
                "INSERT OR IGNORE INTO claim_file_projection_runs (
                    id, entity_subject_ref_json, entity_subject_compact, projection_root,
                    markdown_rel_path, sidecar_rel_path, projection_version,
                    sidecar_schema_version, entity_claim_invalidation_version, claim_watermark,
                    markdown_checksum, sidecar_checksum, status, attempted_at, succeeded_at,
                    created_at, updated_at
                 ) VALUES (
                    ?1, ?2, ?3, '_dailyos_claims', ?4, ?5, ?6, ?7, 0, 'watermark',
                    'markdown-checksum', ?8, 'committed', ?9, ?9, ?9, ?9
                 )",
                params![
                    &run_id,
                    serde_json::to_string(&sidecar.entity_subject_ref).expect("serialize subject"),
                    &sidecar.entity_subject_compact,
                    &sidecar.markdown_rel_path,
                    &sidecar.sidecar_rel_path,
                    sidecar.projection_version as i64,
                    sidecar.schema_version as i64,
                    &checksum,
                    TS,
                ],
            )
            .expect("insert projection run");
        for claim in &sidecar.claims {
            db.conn_ref()
                .execute(
                    "INSERT OR IGNORE INTO claim_file_projection_run_claims (
                        run_id, claim_id, claim_version, semantic_identity_json,
                        trust_band, sensitivity
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        &run_id,
                        &claim.runtime_claim_id,
                        claim.runtime_claim_version as i64,
                        serde_json::to_string(&claim.semantic_identity)
                            .expect("serialize semantic identity"),
                        &claim.trust_band,
                        &claim.sensitivity,
                    ],
                )
                .expect("insert projection run claim");
        }
    }

    fn replay_authorized(
        ctx: &ServiceContext<'_>,
        db: &ActionDb,
        run_id: &str,
        sidecar: &ClaimFileSidecar,
    ) -> Result<CorrectionReplayReport, RebuildError> {
        let checksum = authorize_sidecar(db, sidecar);
        replay_claim_file_sidecar_corrections(ctx, db, run_id, sidecar, &checksum)
    }

    fn feedback_count_for_replay(db: &ActionDb, replay_event_id: &str) -> i64 {
        db.conn_ref()
            .query_row(
                "SELECT count(*) FROM claim_feedback WHERE replay_event_id = ?1",
                params![replay_event_id],
                |row| row.get(0),
            )
            .expect("count replay feedback")
    }

    fn feedback_id_for_replay(db: &ActionDb, replay_event_id: &str) -> String {
        db.conn_ref()
            .query_row(
                "SELECT id FROM claim_feedback WHERE replay_event_id = ?1",
                params![replay_event_id],
                |row| row.get(0),
            )
            .expect("read replay feedback id")
    }

    fn claim_feedback_recorded_signal_id(feedback_id: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(format!("claim-feedback:{feedback_id}:recorded").as_bytes());
        format!("sig-once-{:x}", hasher.finalize())
    }

    fn signal_count_for_id(db: &ActionDb, signal_id: &str) -> i64 {
        db.conn_ref()
            .query_row(
                "SELECT count(*) FROM signal_events WHERE id = ?1",
                params![signal_id],
                |row| row.get(0),
            )
            .expect("count signal by id")
    }

    fn replay_journal_count(db: &ActionDb) -> i64 {
        db.conn_ref()
            .query_row(
                "SELECT count(*) FROM rebuild_correction_replay_events",
                [],
                |row| row.get(0),
            )
            .expect("count replay journal")
    }

    fn feedback_submitted_at_for_replay(db: &ActionDb, replay_event_id: &str) -> String {
        db.conn_ref()
            .query_row(
                "SELECT submitted_at FROM claim_feedback WHERE replay_event_id = ?1",
                params![replay_event_id],
                |row| row.get(0),
            )
            .expect("read replay feedback submitted_at")
    }

    fn replay_journal_status_and_reason(
        db: &ActionDb,
        replay_event_id: &str,
    ) -> (String, Option<String>, Option<String>) {
        db.conn_ref()
            .query_row(
                "SELECT status, reason_code, reason_detail_hash
                 FROM rebuild_correction_replay_events
                 WHERE sidecar_event_id = ?1",
                params![replay_event_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("read replay journal status")
    }

    fn replay_journal_resolved_claim_id(db: &ActionDb, replay_event_id: &str) -> Option<String> {
        db.conn_ref()
            .query_row(
                "SELECT resolved_claim_id
                 FROM rebuild_correction_replay_events
                 WHERE sidecar_event_id = ?1",
                params![replay_event_id],
                |row| row.get(0),
            )
            .expect("read replay journal target")
    }

    fn insert_claimed_legacy_replay_journal(
        db: &ActionDb,
        sidecar: &ClaimFileSidecar,
        resolved_claim_id: &str,
    ) {
        let sidecar_claim = &sidecar.claims[0];
        let feedback = &sidecar_claim.feedback_rows[0];
        db.conn_ref()
            .execute(
                "INSERT INTO rebuild_correction_replay_events (
                    sidecar_event_id, run_id, sidecar_schema_version, source_runtime_claim_id,
                    resolved_claim_id, action, status, reason_code, reason_detail_hash,
                    semantic_identity_hash, feedback_content_hash, applied_feedback_id,
                    attempt_count, claimed_at, applied_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'claimed', NULL, NULL, ?7, ?8, NULL, 1, ?9, NULL, ?9)",
                params![
                    &feedback.feedback_id,
                    "run-before-crash",
                    sidecar.schema_version as i64,
                    &sidecar_claim.runtime_claim_id,
                    resolved_claim_id,
                    &feedback.action,
                    &sidecar_claim.semantic_identity.dedup_key_components_hash,
                    LEGACY_V282_UNSET_FEEDBACK_CONTENT_HASH,
                    TS,
                ],
            )
            .expect("insert claimed legacy journal row");
    }

    fn claim_verification_state_and_version(db: &ActionDb, claim_id: &str) -> (String, i64) {
        db.conn_ref()
            .query_row(
                "SELECT verification_state, claim_version
                 FROM intelligence_claims
                 WHERE id = ?1",
                params![claim_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("read claim verification state")
    }

    fn targeted_repair_job_count(db: &ActionDb, claim_id: &str) -> i64 {
        db.conn_ref()
            .query_row(
                "SELECT count(*)
                 FROM invalidation_jobs
                 WHERE job_kind = ?1
                   AND json_extract(payload_json, '$.claim_id') = ?2",
                params![crate::db::invalidation_jobs::KIND_TARGETED_REPAIR, claim_id],
                |row| row.get(0),
            )
            .expect("count targeted repair jobs")
    }

    #[test]
    fn dos832_replay_sidecar_resolves_fresh_claim_by_semantic_identity_not_runtime_uuid() {
        let db = test_db();
        let (clock, rng, external) = ctx_parts();
        let ctx = ServiceContext::test_live(&clock, &rng, &external);
        seed_account(&db);
        let fresh_claim_id = inserted_claim_id(
            commit_claim(&ctx, &db, proposal("Renewal risk is elevated"))
                .expect("commit fresh claim"),
        );
        let mut sidecar = sidecar_for_claim(
            &db,
            &fresh_claim_id,
            "old-runtime-claim-id",
            "old-feedback-1",
        );
        sidecar.claims[0].feedback_rows[0].submitted_at = REPLAYED_FEEDBACK_TS.to_string();

        let report = replay_authorized(&ctx, &db, "run-1", &sidecar).expect("replay sidecar");

        assert_eq!(report.applied_count, 1);
        assert!(!report.plain_sqlite_recovery_proven);
        assert!(!report.live_cutover_proven);
        assert_eq!(
            report.events[0].source_runtime_claim_id,
            "old-runtime-claim-id"
        );
        assert_eq!(
            report.events[0].resolved_claim_id.as_deref(),
            Some(fresh_claim_id.as_str())
        );
        assert_eq!(feedback_count_for_replay(&db, "old-feedback-1"), 1);
        assert_eq!(
            feedback_submitted_at_for_replay(&db, "old-feedback-1"),
            REPLAYED_FEEDBACK_TS
        );
        let (verification_state, claim_version) =
            claim_verification_state_and_version(&db, &fresh_claim_id);
        assert_eq!(verification_state, "contested");
        assert_eq!(claim_version, 2);
        assert_eq!(targeted_repair_job_count(&db, &fresh_claim_id), 1);
    }

    #[test]
    fn dos832_replay_rejects_uncommitted_sidecar_before_mutation() {
        let db = test_db();
        let (clock, rng, external) = ctx_parts();
        let ctx = ServiceContext::test_live(&clock, &rng, &external);
        seed_account(&db);
        let fresh_claim_id = inserted_claim_id(
            commit_claim(&ctx, &db, proposal("Renewal risk is elevated"))
                .expect("commit fresh claim"),
        );
        let sidecar = sidecar_for_claim(
            &db,
            &fresh_claim_id,
            "old-runtime-claim-id",
            "forged-feedback",
        );
        let checksum = sidecar_checksum(&sidecar);

        let err = replay_claim_file_sidecar_corrections(&ctx, &db, "run-1", &sidecar, &checksum)
            .expect_err("uncommitted sidecar must not replay");

        assert!(
            matches!(&err, RebuildError::InvalidSidecar(message) if message.contains("projection path binding missing") || message.contains("committed projection run not found")),
            "unexpected error: {err:?}"
        );
        assert_eq!(feedback_count_for_replay(&db, "forged-feedback"), 0);
        assert_eq!(replay_journal_count(&db), 0);
    }

    #[test]
    fn dos832_replay_rejects_tampered_sidecar_with_borrowed_committed_checksum() {
        let db = test_db();
        let (clock, rng, external) = ctx_parts();
        let ctx = ServiceContext::test_live(&clock, &rng, &external);
        seed_account(&db);
        let fresh_claim_id = inserted_claim_id(
            commit_claim(&ctx, &db, proposal("Renewal risk is elevated"))
                .expect("commit fresh claim"),
        );
        let sidecar = sidecar_for_claim(
            &db,
            &fresh_claim_id,
            "old-runtime-claim-id",
            "committed-feedback",
        );
        let committed_checksum = authorize_sidecar(&db, &sidecar);

        let mut tampered = sidecar;
        tampered.claims[0].feedback_rows[0].feedback_id = "borrowed-checksum-feedback".to_string();
        tampered.claims[0].feedback_rows[0].action = "mark_false".to_string();
        tampered.claims[0].feedback_rows[0].submitted_at = "2026-05-02T09:15:00Z".to_string();
        let err = replay_claim_file_sidecar_corrections(
            &ctx,
            &db,
            "run-1",
            &tampered,
            &committed_checksum,
        )
        .expect_err("tampered sidecar must not borrow a committed checksum");

        assert!(
            matches!(err, RebuildError::InvalidSidecar(message) if message.contains("sidecar checksum"))
        );
        assert_eq!(
            feedback_count_for_replay(&db, "borrowed-checksum-feedback"),
            0
        );
        assert_eq!(replay_journal_count(&db), 0);
    }

    #[test]
    fn dos832_replay_claims_event_id_before_mutation_and_survives_restart() {
        let db = test_db();
        let (clock, rng, external) = ctx_parts();
        let ctx = ServiceContext::test_live(&clock, &rng, &external);
        seed_account(&db);
        let fresh_claim_id = inserted_claim_id(
            commit_claim(&ctx, &db, proposal("Renewal risk is elevated"))
                .expect("commit fresh claim"),
        );
        let sidecar = sidecar_for_claim(
            &db,
            &fresh_claim_id,
            "old-runtime-claim-id",
            "old-feedback-2",
        );

        replay_authorized(&ctx, &db, "run-1", &sidecar).expect("first replay");
        let second = replay_authorized(&ctx, &db, "run-1", &sidecar).expect("second replay");

        assert_eq!(second.applied_count, 0);
        assert_eq!(second.already_applied_count, 1);
        assert_eq!(feedback_count_for_replay(&db, "old-feedback-2"), 1);
        let (status, reason_code, _) = replay_journal_status_and_reason(&db, "old-feedback-2");
        assert_eq!(status, "applied");
        assert_eq!(reason_code, None);
    }

    #[test]
    fn dos832_replay_derives_event_id_for_legacy_v1_sidecar_feedback() {
        let db = test_db();
        let (clock, rng, external) = ctx_parts();
        let ctx = ServiceContext::test_live(&clock, &rng, &external);
        seed_account(&db);
        let fresh_claim_id = inserted_claim_id(
            commit_claim(&ctx, &db, proposal("Renewal risk is elevated"))
                .expect("commit fresh claim"),
        );
        let mut sidecar = sidecar_for_claim(&db, &fresh_claim_id, "old-runtime-claim-id", "");
        sidecar.schema_version = CLAIM_FILE_LEGACY_SIDECAR_SCHEMA_VERSION;
        sidecar.claims[0]
            .contradiction_edges
            .push(ClaimFileContradictionEdge {
                edge_id: "legacy-edge-1".to_string(),
                branch_kind: "contradiction".to_string(),
                role: "primary".to_string(),
                primary_runtime_claim_id: "old-runtime-claim-id".to_string(),
                contradicting_runtime_claim_id: "other-runtime-claim-id".to_string(),
                primary_semantic_identity: None,
                contradicting_semantic_identity: None,
                detected_at: TS.to_string(),
                reconciliation_kind: None,
                reconciliation_note: None,
                reconciled_at: None,
                winner_runtime_claim_id: None,
                winner_semantic_identity: None,
                merged_runtime_claim_id: None,
                merged_semantic_identity: None,
                replay_status: replay_status(),
            });

        let legacy_json = serde_json::to_string_pretty(&LegacyV1SidecarChecksum::from(&sidecar))
            .expect("serialize legacy sidecar");
        assert!(
            !legacy_json.contains("\"feedbackId\""),
            "legacy v1 sidecar checksum must not include defaulted v2 feedbackId"
        );
        assert!(
            !legacy_json.contains("\"supersededBySemanticIdentity\""),
            "legacy v1 sidecar checksum must not include post-v1 claim fields"
        );
        assert!(
            !legacy_json.contains("\"winnerSemanticIdentity\"")
                && !legacy_json.contains("\"mergedSemanticIdentity\""),
            "legacy v1 sidecar checksum must not include post-v1 contradiction fields"
        );
        let mut hasher = Sha256::new();
        hasher.update(legacy_json.as_bytes());
        let legacy_checksum = format!("{:x}", hasher.finalize());
        let decoded: ClaimFileSidecar =
            serde_json::from_str(&legacy_json).expect("decode legacy sidecar");
        authorize_sidecar_with_checksum(&db, &decoded, &legacy_checksum);

        let report =
            replay_claim_file_sidecar_corrections(&ctx, &db, "run-1", &decoded, &legacy_checksum)
                .expect("replay legacy sidecar");

        assert_eq!(report.applied_count, 1);
        assert!(report.events[0].sidecar_event_id.starts_with("legacy-v1:"));
        assert_eq!(
            feedback_count_for_replay(&db, &report.events[0].sidecar_event_id),
            1
        );
    }

    #[test]
    fn dos832_replay_rejects_legacy_v1_sidecar_with_injected_feedback_id() {
        let db = test_db();
        let (clock, rng, external) = ctx_parts();
        let ctx = ServiceContext::test_live(&clock, &rng, &external);
        seed_account(&db);
        let fresh_claim_id = inserted_claim_id(
            commit_claim(&ctx, &db, proposal("Renewal risk is elevated"))
                .expect("commit fresh claim"),
        );
        let mut sidecar = sidecar_for_claim(&db, &fresh_claim_id, "old-runtime-claim-id", "");
        sidecar.schema_version = CLAIM_FILE_LEGACY_SIDECAR_SCHEMA_VERSION;
        let legacy_checksum =
            legacy_v1_sidecar_payload_checksum(&sidecar).expect("legacy checksum");
        authorize_sidecar_with_checksum(&db, &sidecar, &legacy_checksum);
        sidecar.claims[0].feedback_rows[0].feedback_id = "chosen-replay-id".to_string();

        let err =
            replay_claim_file_sidecar_corrections(&ctx, &db, "run-1", &sidecar, &legacy_checksum)
                .expect_err("injected v1 feedback_id must reject");

        assert!(
            matches!(err, RebuildError::InvalidSidecar(message) if message.contains("legacy v1"))
        );
        assert_eq!(feedback_count_for_replay(&db, "chosen-replay-id"), 0);
        assert_eq!(replay_journal_count(&db), 0);
    }

    #[test]
    fn dos832_replay_rejects_v2_sidecar_without_source_content_hash() {
        let db = test_db();
        let (clock, rng, external) = ctx_parts();
        let ctx = ServiceContext::test_live(&clock, &rng, &external);
        seed_account(&db);
        let fresh_claim_id = inserted_claim_id(
            commit_claim(&ctx, &db, proposal("Renewal risk is elevated"))
                .expect("commit fresh claim"),
        );
        let mut sidecar = sidecar_for_claim(
            &db,
            &fresh_claim_id,
            "old-runtime-claim-id",
            "missing-source-hash-feedback",
        );
        sidecar.claims[0].semantic_identity.source_content_hash = None;

        let err = replay_authorized(&ctx, &db, "run-1", &sidecar)
            .expect_err("v2 sidecar without source hash must reject");

        assert!(
            matches!(err, RebuildError::InvalidSidecar(message) if message.contains("source_content_hash"))
        );
        assert_eq!(
            feedback_count_for_replay(&db, "missing-source-hash-feedback"),
            0
        );
        assert_eq!(replay_journal_count(&db), 0);
    }

    #[test]
    fn dos832_replay_rejects_same_event_id_with_changed_feedback_content() {
        let db = test_db();
        let (clock, rng, external) = ctx_parts();
        let ctx = ServiceContext::test_live(&clock, &rng, &external);
        seed_account(&db);
        let fresh_claim_id = inserted_claim_id(
            commit_claim(&ctx, &db, proposal("Renewal risk is elevated"))
                .expect("commit fresh claim"),
        );
        let mut first_sidecar = sidecar_for_claim(
            &db,
            &fresh_claim_id,
            "old-runtime-claim-id",
            "payload-event",
        );
        first_sidecar.claims[0].feedback_rows[0].action = "needs_nuance".to_string();
        first_sidecar.claims[0].feedback_rows[0].payload_json =
            Some(serde_json::json!({ "corrected_text": "Risk is limited to the pilot group" }));
        replay_authorized(&ctx, &db, "run-1", &first_sidecar).expect("first replay");

        let mut second_sidecar = first_sidecar;
        second_sidecar.claims[0].feedback_rows[0].payload_json =
            Some(serde_json::json!({ "corrected_text": "Risk is limited to the renewal window" }));
        let err = replay_authorized(&ctx, &db, "run-2", &second_sidecar)
            .expect_err("same replay id must not hide changed payload");

        assert!(
            matches!(err, RebuildError::InvalidSidecar(message) if message.contains("feedback content"))
        );
        assert_eq!(feedback_count_for_replay(&db, "payload-event"), 1);
    }

    #[test]
    fn dos832_replay_rejects_same_event_id_with_changed_submitted_at() {
        let db = test_db();
        let (clock, rng, external) = ctx_parts();
        let ctx = ServiceContext::test_live(&clock, &rng, &external);
        seed_account(&db);
        let fresh_claim_id = inserted_claim_id(
            commit_claim(&ctx, &db, proposal("Renewal risk is elevated"))
                .expect("commit fresh claim"),
        );
        let mut first_sidecar = sidecar_for_claim(
            &db,
            &fresh_claim_id,
            "old-runtime-claim-id",
            "submitted-at-event",
        );
        first_sidecar.claims[0].feedback_rows[0].submitted_at = REPLAYED_FEEDBACK_TS.to_string();
        replay_authorized(&ctx, &db, "run-1", &first_sidecar).expect("first replay");

        let mut second_sidecar = first_sidecar;
        second_sidecar.claims[0].feedback_rows[0].submitted_at = "2026-05-02T08:30:00Z".to_string();
        let err = replay_authorized(&ctx, &db, "run-2", &second_sidecar)
            .expect_err("same replay id must not hide changed submitted_at");

        assert!(
            matches!(err, RebuildError::InvalidSidecar(message) if message.contains("feedback content"))
        );
        assert_eq!(
            feedback_submitted_at_for_replay(&db, "submitted-at-event"),
            REPLAYED_FEEDBACK_TS
        );
    }

    #[test]
    fn dos832_replay_rejects_same_event_id_from_different_sidecar_checksum() {
        let db = test_db();
        let (clock, rng, external) = ctx_parts();
        let ctx = ServiceContext::test_live(&clock, &rng, &external);
        seed_account(&db);
        let fresh_claim_id = inserted_claim_id(
            commit_claim(&ctx, &db, proposal("Renewal risk is elevated"))
                .expect("commit fresh claim"),
        );
        let first_sidecar = sidecar_for_claim(
            &db,
            &fresh_claim_id,
            "old-runtime-claim-id",
            "checksum-event",
        );
        replay_authorized(&ctx, &db, "run-1", &first_sidecar).expect("first replay");

        let mut second_sidecar = first_sidecar;
        second_sidecar.claims[0]
            .claim_text
            .push_str(" (projection-only checksum drift)");
        let err = replay_authorized(&ctx, &db, "run-2", &second_sidecar)
            .expect_err("same replay id must stay bound to original sidecar checksum");

        assert!(
            matches!(err, RebuildError::InvalidSidecar(message) if message.contains("sidecar checksum"))
        );
        assert_eq!(feedback_count_for_replay(&db, "checksum-event"), 1);
    }

    #[test]
    fn dos832_replay_rejects_sidecar_event_older_than_existing_feedback() {
        let db = test_db();
        let (clock, rng, external) = ctx_parts();
        let ctx = ServiceContext::test_live(&clock, &rng, &external);
        seed_account(&db);
        let fresh_claim_id = inserted_claim_id(
            commit_claim(&ctx, &db, proposal("Renewal risk is elevated"))
                .expect("commit fresh claim"),
        );
        let mut sidecar = sidecar_for_claim(
            &db,
            &fresh_claim_id,
            "old-runtime-claim-id",
            "stale-sidecar-feedback",
        );
        sidecar.claims[0].feedback_rows[0].submitted_at = REPLAYED_FEEDBACK_TS.to_string();
        let checksum = authorize_sidecar(&db, &sidecar);

        record_claim_feedback(
            &ctx,
            &db,
            ClaimFeedbackInput {
                claim_id: fresh_claim_id.clone(),
                action: FeedbackAction::NeedsNuance,
                actor: "user".to_string(),
                actor_id: Some("user-fixture".to_string()),
                payload_json: Some(
                    serde_json::json!({ "corrected_text": "newer correction" }).to_string(),
                ),
            },
        )
        .expect("record newer feedback");

        let report = replay_claim_file_sidecar_corrections(&ctx, &db, "run-1", &sidecar, &checksum)
            .expect("stale replay is terminalized");

        assert_eq!(report.failed_count, 1);
        assert_eq!(feedback_count_for_replay(&db, "stale-sidecar-feedback"), 0);
        let (status, reason_code, _) =
            replay_journal_status_and_reason(&db, "stale-sidecar-feedback");
        assert_eq!(status, "failed");
        assert_eq!(reason_code.as_deref(), Some("stale_replay_watermark"));
    }

    #[test]
    fn dos832_replay_rejects_offset_sidecar_older_than_existing_feedback() {
        let db = test_db();
        let (clock, rng, external) = ctx_parts();
        let ctx = ServiceContext::test_live(&clock, &rng, &external);
        seed_account(&db);
        let fresh_claim_id = inserted_claim_id(
            commit_claim(&ctx, &db, proposal("Renewal risk is elevated"))
                .expect("commit fresh claim"),
        );
        let mut sidecar = sidecar_for_claim(
            &db,
            &fresh_claim_id,
            "old-runtime-claim-id",
            "offset-stale-feedback",
        );
        sidecar.claims[0].feedback_rows[0].submitted_at = "2026-05-01T09:30:00+02:00".to_string();
        let checksum = authorize_sidecar(&db, &sidecar);

        record_claim_feedback_replay(
            &ctx,
            &db,
            ClaimFeedbackReplayInput {
                feedback: ClaimFeedbackInput {
                    claim_id: fresh_claim_id.clone(),
                    action: FeedbackAction::ConfirmCurrent,
                    actor: "user".to_string(),
                    actor_id: Some("user-fixture".to_string()),
                    payload_json: None,
                },
                replay_event_id: "existing-newer-feedback".to_string(),
                submitted_at: "2026-05-01T08:30:00+00:00".to_string(),
            },
        )
        .expect("record newer feedback with offset-compatible timestamp");

        let report = replay_claim_file_sidecar_corrections(&ctx, &db, "run-1", &sidecar, &checksum)
            .expect("offset stale replay is terminalized");

        assert_eq!(report.failed_count, 1);
        assert_eq!(feedback_count_for_replay(&db, "offset-stale-feedback"), 0);
        let (status, reason_code, _) =
            replay_journal_status_and_reason(&db, "offset-stale-feedback");
        assert_eq!(status, "failed");
        assert_eq!(reason_code.as_deref(), Some("stale_replay_watermark"));
    }

    #[test]
    fn dos832_replay_rejects_fractional_second_newer_feedback_watermark() {
        let db = test_db();
        let (clock, rng, external) = ctx_parts();
        let ctx = ServiceContext::test_live(&clock, &rng, &external);
        seed_account(&db);
        let fresh_claim_id = inserted_claim_id(
            commit_claim(&ctx, &db, proposal("Renewal risk is elevated"))
                .expect("commit fresh claim"),
        );
        let mut sidecar = sidecar_for_claim(
            &db,
            &fresh_claim_id,
            "old-runtime-claim-id",
            "fractional-stale-feedback",
        );
        sidecar.claims[0].feedback_rows[0].submitted_at = "2026-05-01T08:30:00Z".to_string();
        let checksum = authorize_sidecar(&db, &sidecar);

        record_claim_feedback_replay(
            &ctx,
            &db,
            ClaimFeedbackReplayInput {
                feedback: ClaimFeedbackInput {
                    claim_id: fresh_claim_id.clone(),
                    action: FeedbackAction::ConfirmCurrent,
                    actor: "user".to_string(),
                    actor_id: Some("user-fixture".to_string()),
                    payload_json: None,
                },
                replay_event_id: "existing-fractional-feedback".to_string(),
                submitted_at: "2026-05-01T08:30:00.500+00:00".to_string(),
            },
        )
        .expect("record newer fractional feedback");

        let report = replay_claim_file_sidecar_corrections(&ctx, &db, "run-1", &sidecar, &checksum)
            .expect("fractional stale replay is terminalized");

        assert_eq!(report.failed_count, 1);
        assert_eq!(
            feedback_count_for_replay(&db, "fractional-stale-feedback"),
            0
        );
        let (status, reason_code, _) =
            replay_journal_status_and_reason(&db, "fractional-stale-feedback");
        assert_eq!(status, "failed");
        assert_eq!(reason_code.as_deref(), Some("stale_replay_watermark"));
    }

    #[test]
    fn dos832_replay_rejects_fractional_second_newer_version_event_watermark() {
        let db = test_db();
        let (clock, rng, external) = ctx_parts();
        let ctx = ServiceContext::test_live(&clock, &rng, &external);
        seed_account(&db);
        let fresh_claim_id = inserted_claim_id(
            commit_claim(&ctx, &db, proposal("Renewal risk is elevated"))
                .expect("commit fresh claim"),
        );
        let mut sidecar = sidecar_for_claim(
            &db,
            &fresh_claim_id,
            "old-runtime-claim-id",
            "fractional-stale-version-event",
        );
        sidecar.claims[0].feedback_rows[0].submitted_at = "2026-05-01T08:30:00Z".to_string();
        let checksum = authorize_sidecar(&db, &sidecar);

        db.conn_ref()
            .execute(
                "INSERT INTO version_events (
                    cursor, event_kind, claim_id, previous_version, current_version,
                    scope_redacted, created_at, actor_kind
                 ) VALUES (?1, 'claim.corrected', ?2, ?3, ?4, 0, ?5, 'user')",
                params![
                    "b821dbf6-9fb9-4235-a45e-0fcf9dc472aa",
                    fresh_claim_id,
                    1_i64,
                    2_i64,
                    "2026-05-01T08:30:00.500+00:00",
                ],
            )
            .expect("seed newer version event watermark");

        let report = replay_claim_file_sidecar_corrections(&ctx, &db, "run-1", &sidecar, &checksum)
            .expect("fractional version-event stale replay is terminalized");

        assert_eq!(report.failed_count, 1);
        assert_eq!(
            feedback_count_for_replay(&db, "fractional-stale-version-event"),
            0
        );
        let (status, reason_code, _) =
            replay_journal_status_and_reason(&db, "fractional-stale-version-event");
        assert_eq!(status, "failed");
        assert_eq!(reason_code.as_deref(), Some("stale_replay_watermark"));
    }

    #[test]
    fn dos832_replay_rejects_newer_existing_claim_update_watermark() {
        let db = test_db();
        let (clock, rng, external) = ctx_parts();
        let ctx = ServiceContext::test_live(&clock, &rng, &external);
        seed_account(&db);
        let fresh_claim_id = inserted_claim_id(
            commit_claim(&ctx, &db, proposal("Renewal risk is elevated"))
                .expect("commit fresh claim"),
        );
        let mut sidecar = sidecar_for_claim(
            &db,
            &fresh_claim_id,
            "old-runtime-claim-id",
            "newer-claim-update",
        );
        sidecar.claims[0].feedback_rows[0].submitted_at = REPLAYED_FEEDBACK_TS.to_string();
        let checksum = authorize_sidecar(&db, &sidecar);

        commit_claim(&ctx, &db, proposal("Renewal risk is elevated"))
            .expect("commit newer corroboration");
        let newer_existing_updates: i64 = db
            .conn_ref()
            .query_row(
                "SELECT count(*)
                 FROM version_events
                 WHERE claim_id = ?1
                   AND event_kind = 'claim.updated'
                   AND previous_version IS NOT NULL
                   AND created_at > ?2",
                params![&fresh_claim_id, REPLAYED_FEEDBACK_TS],
                |row| row.get(0),
            )
            .expect("count newer existing-claim updates");
        assert_eq!(newer_existing_updates, 1);

        let report = replay_claim_file_sidecar_corrections(&ctx, &db, "run-1", &sidecar, &checksum)
            .expect("stale replay is terminalized");

        assert_eq!(report.applied_count, 0);
        assert_eq!(report.failed_count, 1);
        assert_eq!(
            report.events[0].reason_code.as_deref(),
            Some("stale_replay_watermark")
        );
        assert_eq!(feedback_count_for_replay(&db, "newer-claim-update"), 0);
    }

    #[test]
    fn dos832_replay_failed_terminal_update_cannot_overwrite_applied() {
        let db = test_db();
        let (clock, rng, external) = ctx_parts();
        let ctx = ServiceContext::test_live(&clock, &rng, &external);
        seed_account(&db);
        let fresh_claim_id = inserted_claim_id(
            commit_claim(&ctx, &db, proposal("Renewal risk is elevated"))
                .expect("commit fresh claim"),
        );
        let sidecar = sidecar_for_claim(
            &db,
            &fresh_claim_id,
            "old-runtime-claim-id",
            "applied-event",
        );
        replay_authorized(&ctx, &db, "run-1", &sidecar).expect("first replay");

        mark_replay_event_terminal(
            &db,
            "applied-event",
            ReplayEventStatus::Failed.as_str(),
            Some("late_failure"),
            Some("late-failure-detail"),
            None,
            None,
        )
        .expect("late failed mark is ignored");

        let (status, reason_code, detail_hash) =
            replay_journal_status_and_reason(&db, "applied-event");
        assert_eq!(status, "applied");
        assert_eq!(reason_code, None);
        assert_eq!(detail_hash, None);
    }

    #[test]
    fn dos832_replay_repairs_partial_v282_claimed_content_hash_gap() {
        let db = test_db();
        let (clock, rng, external) = ctx_parts();
        let ctx = ServiceContext::test_live(&clock, &rng, &external);
        seed_account(&db);
        let fresh_claim_id = inserted_claim_id(
            commit_claim(&ctx, &db, proposal("Renewal risk is elevated"))
                .expect("commit fresh claim"),
        );
        let sidecar = sidecar_for_claim(
            &db,
            &fresh_claim_id,
            "old-runtime-claim-id",
            "partial-v282-event",
        );
        let sidecar_claim = &sidecar.claims[0];
        let feedback = &sidecar_claim.feedback_rows[0];
        db.conn_ref()
            .execute(
                "INSERT INTO rebuild_correction_replay_events (
                    sidecar_event_id, run_id, sidecar_schema_version, source_runtime_claim_id,
                    resolved_claim_id, action, status, reason_code, reason_detail_hash,
                    semantic_identity_hash, feedback_content_hash, applied_feedback_id,
                    attempt_count, claimed_at, applied_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'claimed', NULL, NULL, ?7, ?8, NULL, 1, ?9, NULL, ?9)",
                params![
                    &feedback.feedback_id,
                    "run-before-crash",
                    sidecar.schema_version as i64,
                    &sidecar_claim.runtime_claim_id,
                    &fresh_claim_id,
                    &feedback.action,
                    &sidecar_claim.semantic_identity.dedup_key_components_hash,
                    LEGACY_V282_UNSET_FEEDBACK_CONTENT_HASH,
                    TS,
                ],
            )
            .expect("insert partial v282 claimed row");

        let report =
            replay_authorized(&ctx, &db, "run-1", &sidecar).expect("replay repaired claimed row");

        assert_eq!(report.applied_count, 1);
        assert_eq!(feedback_count_for_replay(&db, "partial-v282-event"), 1);
        let repaired_hash: String = db
            .conn_ref()
            .query_row(
                "SELECT feedback_content_hash
                 FROM rebuild_correction_replay_events
                 WHERE sidecar_event_id = 'partial-v282-event'",
                [],
                |row| row.get(0),
            )
            .expect("read repaired hash");
        assert_ne!(repaired_hash, LEGACY_V282_UNSET_FEEDBACK_CONTENT_HASH);
        let (status, _, _) = replay_journal_status_and_reason(&db, "partial-v282-event");
        assert_eq!(status, "applied");
    }

    #[test]
    fn dos832_replay_claimed_legacy_collision_recovers_existing_feedback() {
        let db = test_db();
        let (clock, rng, external) = ctx_parts();
        let ctx = ServiceContext::test_live(&clock, &rng, &external);
        seed_account(&db);
        let fresh_claim_id = inserted_claim_id(
            commit_claim(&ctx, &db, proposal("Renewal risk is elevated"))
                .expect("commit fresh claim"),
        );
        let sidecar = sidecar_for_claim(
            &db,
            &fresh_claim_id,
            "old-runtime-claim-id",
            "legacy-collision-event",
        );
        let sidecar_claim = &sidecar.claims[0];
        let feedback = &sidecar_claim.feedback_rows[0];
        let action = feedback_action_from_slug(&feedback.action).expect("feedback action");
        let payload_json = feedback_payload_json(feedback).expect("payload json");
        let submitted_at =
            canonical_feedback_submitted_at(&feedback.submitted_at).expect("submitted at");
        let existing_hash = claim_feedback_replay_content_hash(
            action,
            &feedback.actor,
            feedback.actor_id.as_deref(),
            payload_json.as_deref(),
            &submitted_at,
        )
        .expect("existing content hash");
        let existing_feedback = record_claim_feedback_replay(
            &ctx,
            &db,
            ClaimFeedbackReplayInput {
                feedback: ClaimFeedbackInput {
                    claim_id: fresh_claim_id.clone(),
                    action,
                    actor: feedback.actor.clone(),
                    actor_id: feedback.actor_id.clone(),
                    payload_json: payload_json.clone(),
                },
                replay_event_id: feedback.feedback_id.clone(),
                submitted_at: submitted_at.clone(),
            },
        )
        .expect("record feedback before crash");
        let recorded_signal_id = claim_feedback_recorded_signal_id(&existing_feedback.feedback_id);
        assert_eq!(signal_count_for_id(&db, &recorded_signal_id), 1);
        db.conn_ref()
            .execute(
                "DELETE FROM signal_events WHERE id = ?1",
                params![&recorded_signal_id],
            )
            .expect("delete recorded feedback signal");
        assert_eq!(signal_count_for_id(&db, &recorded_signal_id), 0);
        db.conn_ref()
            .execute(
                "INSERT INTO rebuild_correction_replay_events (
                    sidecar_event_id, run_id, sidecar_schema_version, source_runtime_claim_id,
                    resolved_claim_id, action, status, reason_code, reason_detail_hash,
                    semantic_identity_hash, feedback_content_hash, applied_feedback_id,
                    attempt_count, claimed_at, applied_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'claimed', NULL, NULL, ?7, ?8, NULL, 1, ?9, NULL, ?9)",
                params![
                    &feedback.feedback_id,
                    "run-before-crash",
                    sidecar.schema_version as i64,
                    &sidecar_claim.runtime_claim_id,
                    &fresh_claim_id,
                    &feedback.action,
                    &sidecar_claim.semantic_identity.dedup_key_components_hash,
                    LEGACY_V282_UNSET_FEEDBACK_CONTENT_HASH,
                    TS,
                ],
            )
            .expect("insert claimed legacy journal row");

        let mut drifted_sidecar = sidecar.clone();
        drifted_sidecar.claims[0].claim_text =
            "Renewal risk is elevated in the sidecar projection".to_string();
        let drifted_checksum = authorize_sidecar(&db, &drifted_sidecar);
        assert_ne!(drifted_checksum, sidecar_checksum(&sidecar));

        let drifted_report = replay_claim_file_sidecar_corrections(
            &ctx,
            &db,
            "run-drifted",
            &drifted_sidecar,
            &drifted_checksum,
        )
        .expect("projection-only drifted retry recovers existing feedback");

        assert_eq!(drifted_report.already_applied_count, 1);
        assert_eq!(drifted_report.failed_count, 0);
        assert_eq!(feedback_count_for_replay(&db, "legacy-collision-event"), 1);
        let (stored_hash, stored_checksum, applied_feedback_id): (String, String, Option<String>) =
            db.conn_ref()
                .query_row(
                    "SELECT feedback_content_hash, sidecar_checksum, applied_feedback_id
                       FROM rebuild_correction_replay_events
                      WHERE sidecar_event_id = ?1",
                    params!["legacy-collision-event"],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
                .expect("read recovered journal");
        assert_eq!(stored_hash, existing_hash);
        assert_eq!(stored_checksum, LEGACY_V283_UNSET_SIDECAR_CHECKSUM);
        assert_eq!(signal_count_for_id(&db, &recorded_signal_id), 1);
        let repaired_payload: String = db
            .conn_ref()
            .query_row(
                "SELECT value FROM signal_events WHERE id = ?1",
                params![&recorded_signal_id],
                |row| row.get(0),
            )
            .expect("read repaired signal payload");
        assert!(
            repaired_payload.contains("\"recovered\":true"),
            "claimed collision recovery must repair the deterministic signal"
        );
        assert_eq!(
            applied_feedback_id.as_deref(),
            Some(existing_feedback.feedback_id.as_str())
        );

        let drifted_terminal_retry = replay_claim_file_sidecar_corrections(
            &ctx,
            &db,
            "run-drifted-terminal",
            &drifted_sidecar,
            &drifted_checksum,
        )
        .expect("projection-only terminal retry remains checksum-neutral");
        assert_eq!(drifted_terminal_retry.already_applied_count, 1);
        let terminal_retry_checksum: String = db
            .conn_ref()
            .query_row(
                "SELECT sidecar_checksum
                   FROM rebuild_correction_replay_events
                  WHERE sidecar_event_id = ?1",
                params!["legacy-collision-event"],
                |row| row.get(0),
            )
            .expect("read terminal retry checksum");
        assert_eq!(
            terminal_retry_checksum, LEGACY_V283_UNSET_SIDECAR_CHECKSUM,
            "terminal rows must stay checksum-neutral so retry order cannot lock out the original sidecar"
        );

        let original_checksum = authorize_sidecar(&db, &sidecar);
        let original_report = replay_claim_file_sidecar_corrections(
            &ctx,
            &db,
            "run-original",
            &sidecar,
            &original_checksum,
        )
        .expect("original retry remains recoverable");
        assert_eq!(original_report.already_applied_count, 1);
        assert_eq!(original_report.failed_count, 0);
    }

    #[test]
    fn dos832_replay_claimed_legacy_signal_repair_failure_remains_retryable() {
        let db = test_db();
        let (clock, rng, external) = ctx_parts();
        let ctx = ServiceContext::test_live(&clock, &rng, &external);
        seed_account(&db);
        let claim_id = inserted_claim_id(
            commit_claim(&ctx, &db, proposal("Renewal risk is elevated")).expect("commit claim"),
        );
        let sidecar = sidecar_for_claim(
            &db,
            &claim_id,
            "old-runtime-claim-id",
            "legacy-signal-failure-event",
        );
        let feedback = &sidecar.claims[0].feedback_rows[0];
        let action = feedback_action_from_slug(&feedback.action).expect("feedback action");
        let payload_json = feedback_payload_json(feedback).expect("payload json");
        let submitted_at =
            canonical_feedback_submitted_at(&feedback.submitted_at).expect("submitted at");
        let existing_feedback = record_claim_feedback_replay(
            &ctx,
            &db,
            ClaimFeedbackReplayInput {
                feedback: ClaimFeedbackInput {
                    claim_id: claim_id.clone(),
                    action,
                    actor: feedback.actor.clone(),
                    actor_id: feedback.actor_id.clone(),
                    payload_json,
                },
                replay_event_id: feedback.feedback_id.clone(),
                submitted_at,
            },
        )
        .expect("record feedback before crash");
        let recorded_signal_id = claim_feedback_recorded_signal_id(&existing_feedback.feedback_id);
        db.conn_ref()
            .execute(
                "DELETE FROM signal_events WHERE id = ?1",
                params![&recorded_signal_id],
            )
            .expect("delete recorded feedback signal");
        insert_claimed_legacy_replay_journal(&db, &sidecar, &claim_id);

        let mut drifted_sidecar = sidecar.clone();
        drifted_sidecar.claims[0].claim_text =
            "Renewal risk is elevated in the sidecar projection".to_string();
        let drifted_checksum = authorize_sidecar(&db, &drifted_sidecar);
        let escaped_signal_id = recorded_signal_id.replace('\'', "''");
        let fail_signal_trigger = format!(
            "CREATE TRIGGER fail_claim_feedback_signal_repair
             BEFORE INSERT ON signal_events
             WHEN NEW.id = '{escaped_signal_id}'
             BEGIN
                SELECT RAISE(FAIL, 'temporary signal repair failure');
             END;"
        );
        db.conn_ref()
            .execute_batch(&fail_signal_trigger)
            .expect("install signal repair failure trigger");

        let err = replay_claim_file_sidecar_corrections(
            &ctx,
            &db,
            "run-drifted-failure",
            &drifted_sidecar,
            &drifted_checksum,
        )
        .expect_err("signal repair failure remains retryable");
        assert!(
            matches!(err, RebuildError::ClaimReplay(message) if message.contains("temporary signal repair failure"))
        );
        let (status, reason_code, _) =
            replay_journal_status_and_reason(&db, "legacy-signal-failure-event");
        assert_eq!(status, "claimed");
        assert_eq!(reason_code, None);
        let stranded_checksum: String = db
            .conn_ref()
            .query_row(
                "SELECT sidecar_checksum
                   FROM rebuild_correction_replay_events
                  WHERE sidecar_event_id = ?1",
                params!["legacy-signal-failure-event"],
                |row| row.get(0),
            )
            .expect("read stranded checksum");
        assert_eq!(stranded_checksum, LEGACY_V283_UNSET_SIDECAR_CHECKSUM);

        db.conn_ref()
            .execute_batch("DROP TRIGGER fail_claim_feedback_signal_repair;")
            .expect("remove signal repair failure trigger");
        let original_checksum = authorize_sidecar(&db, &sidecar);
        let retry = replay_claim_file_sidecar_corrections(
            &ctx,
            &db,
            "run-original-retry",
            &sidecar,
            &original_checksum,
        )
        .expect("original retry remains recoverable");

        assert_eq!(retry.already_applied_count, 1);
        assert_eq!(retry.failed_count, 0);
        assert_eq!(signal_count_for_id(&db, &recorded_signal_id), 1);
    }

    #[test]
    fn dos832_replay_claimed_legacy_failed_terminal_update_keeps_content_retryable() {
        let db = test_db();
        let (clock, rng, external) = ctx_parts();
        let ctx = ServiceContext::test_live(&clock, &rng, &external);
        seed_account(&db);
        let claim_id = inserted_claim_id(
            commit_claim(&ctx, &db, proposal("Renewal risk is elevated")).expect("commit claim"),
        );
        let sidecar = sidecar_for_claim(
            &db,
            &claim_id,
            "old-runtime-claim-id",
            "legacy-terminal-failure-event",
        );
        let feedback = &sidecar.claims[0].feedback_rows[0];
        let action = feedback_action_from_slug(&feedback.action).expect("feedback action");
        let payload_json = feedback_payload_json(feedback).expect("payload json");
        let submitted_at =
            canonical_feedback_submitted_at(&feedback.submitted_at).expect("submitted at");
        record_claim_feedback_replay(
            &ctx,
            &db,
            ClaimFeedbackReplayInput {
                feedback: ClaimFeedbackInput {
                    claim_id: claim_id.clone(),
                    action,
                    actor: feedback.actor.clone(),
                    actor_id: feedback.actor_id.clone(),
                    payload_json,
                },
                replay_event_id: feedback.feedback_id.clone(),
                submitted_at,
            },
        )
        .expect("record feedback before crash");
        insert_claimed_legacy_replay_journal(&db, &sidecar, &claim_id);

        let mut bad_content_sidecar = sidecar.clone();
        bad_content_sidecar.claims[0].feedback_rows[0].submitted_at =
            "2026-05-01T08:30:01Z".to_string();
        let bad_checksum = authorize_sidecar(&db, &bad_content_sidecar);
        db.conn_ref()
            .execute_batch(
                "CREATE TRIGGER fail_replay_failed_terminal_update
                 BEFORE UPDATE ON rebuild_correction_replay_events
                 WHEN NEW.sidecar_event_id = 'legacy-terminal-failure-event'
                   AND NEW.status = 'failed'
                 BEGIN
                    SELECT RAISE(FAIL, 'temporary terminal update failure');
                 END;",
            )
            .expect("install failed terminal update trigger");

        let err = replay_claim_file_sidecar_corrections(
            &ctx,
            &db,
            "run-bad-content",
            &bad_content_sidecar,
            &bad_checksum,
        )
        .expect_err("failed terminal update remains retryable");
        assert!(
            format!("{err:?}").contains("temporary terminal update failure"),
            "unexpected error: {err:?}"
        );
        let (status, reason_code, _) =
            replay_journal_status_and_reason(&db, "legacy-terminal-failure-event");
        assert_eq!(status, "claimed");
        assert_eq!(reason_code, None);
        let (stored_hash, stored_checksum): (String, String) = db
            .conn_ref()
            .query_row(
                "SELECT feedback_content_hash, sidecar_checksum
                   FROM rebuild_correction_replay_events
                  WHERE sidecar_event_id = ?1",
                params!["legacy-terminal-failure-event"],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("read retryable claimed row");
        assert_eq!(stored_hash, LEGACY_V282_UNSET_FEEDBACK_CONTENT_HASH);
        assert_eq!(stored_checksum, LEGACY_V283_UNSET_SIDECAR_CHECKSUM);

        db.conn_ref()
            .execute_batch("DROP TRIGGER fail_replay_failed_terminal_update;")
            .expect("remove failed terminal update trigger");
        let original_checksum = authorize_sidecar(&db, &sidecar);
        let retry = replay_claim_file_sidecar_corrections(
            &ctx,
            &db,
            "run-original-retry",
            &sidecar,
            &original_checksum,
        )
        .expect("original retry remains recoverable");

        assert_eq!(retry.already_applied_count, 1);
        assert_eq!(retry.failed_count, 0);
    }

    #[test]
    fn dos832_replay_claimed_legacy_collision_refuses_different_existing_feedback() {
        let db = test_db();
        let (clock, rng, external) = ctx_parts();
        let ctx = ServiceContext::test_live(&clock, &rng, &external);
        seed_account(&db);
        let first_claim_id = inserted_claim_id(
            commit_claim(&ctx, &db, proposal("Renewal risk is elevated"))
                .expect("commit first claim"),
        );
        let first_sidecar = sidecar_for_claim(
            &db,
            &first_claim_id,
            "old-runtime-claim-id",
            "legacy-mismatch-event",
        );
        let first_sidecar_claim = &first_sidecar.claims[0];
        let first_feedback = &first_sidecar_claim.feedback_rows[0];
        let action = feedback_action_from_slug(&first_feedback.action).expect("feedback action");
        let payload_json = feedback_payload_json(first_feedback).expect("payload json");
        let submitted_at =
            canonical_feedback_submitted_at(&first_feedback.submitted_at).expect("submitted at");

        let mut second_proposal = proposal("Renewal risk is elevated for a different field");
        second_proposal.field_path = Some("health.other_risk".to_string());
        let second_claim_id = inserted_claim_id(
            commit_claim(&ctx, &db, second_proposal).expect("commit second claim"),
        );
        let second_feedback = record_claim_feedback_replay(
            &ctx,
            &db,
            ClaimFeedbackReplayInput {
                feedback: ClaimFeedbackInput {
                    claim_id: second_claim_id.clone(),
                    action,
                    actor: first_feedback.actor.clone(),
                    actor_id: first_feedback.actor_id.clone(),
                    payload_json: payload_json.clone(),
                },
                replay_event_id: first_feedback.feedback_id.clone(),
                submitted_at: submitted_at.clone(),
            },
        )
        .expect("record mismatched feedback before crash");
        db.conn_ref()
            .execute(
                "INSERT INTO rebuild_correction_replay_events (
                    sidecar_event_id, run_id, sidecar_schema_version, source_runtime_claim_id,
                    resolved_claim_id, action, status, reason_code, reason_detail_hash,
                    semantic_identity_hash, feedback_content_hash, applied_feedback_id,
                    attempt_count, claimed_at, applied_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'claimed', NULL, NULL, ?7, ?8, NULL, 1, ?9, NULL, ?9)",
                params![
                    &first_feedback.feedback_id,
                    "run-before-crash",
                    first_sidecar.schema_version as i64,
                    &first_sidecar_claim.runtime_claim_id,
                    &first_claim_id,
                    &first_feedback.action,
                    &first_sidecar_claim.semantic_identity.dedup_key_components_hash,
                    LEGACY_V282_UNSET_FEEDBACK_CONTENT_HASH,
                    TS,
                ],
            )
            .expect("insert claimed legacy journal row");
        let checksum = authorize_sidecar(&db, &first_sidecar);

        let report = replay_claim_file_sidecar_corrections(
            &ctx,
            &db,
            "run-mismatch",
            &first_sidecar,
            &checksum,
        )
        .expect("mismatched existing feedback is terminalized");

        assert_eq!(report.already_applied_count, 0);
        assert_eq!(report.failed_count, 1);
        assert_eq!(
            feedback_count_for_replay(&db, "legacy-mismatch-event"),
            1,
            "the pre-existing mismatched feedback row is left alone"
        );
        assert_eq!(
            feedback_id_for_replay(&db, "legacy-mismatch-event"),
            second_feedback.feedback_id
        );
        let (status, reason_code, detail_hash) =
            replay_journal_status_and_reason(&db, "legacy-mismatch-event");
        assert_eq!(status, "failed");
        assert_eq!(reason_code.as_deref(), Some("claim_feedback_replay_failed"));
        assert!(detail_hash.is_some());
        let applied_feedback_id: Option<String> = db
            .conn_ref()
            .query_row(
                "SELECT applied_feedback_id
                   FROM rebuild_correction_replay_events
                  WHERE sidecar_event_id = ?1",
                params!["legacy-mismatch-event"],
                |row| row.get(0),
            )
            .expect("read applied feedback id");
        assert_eq!(applied_feedback_id, None);
        assert_eq!(
            replay_journal_resolved_claim_id(&db, "legacy-mismatch-event").as_deref(),
            Some(first_claim_id.as_str())
        );
    }

    #[test]
    fn dos832_replay_claimed_legacy_collision_refuses_same_claim_different_action() {
        let db = test_db();
        let (clock, rng, external) = ctx_parts();
        let ctx = ServiceContext::test_live(&clock, &rng, &external);
        seed_account(&db);
        let claim_id = inserted_claim_id(
            commit_claim(&ctx, &db, proposal("Renewal risk is elevated")).expect("commit claim"),
        );
        let sidecar = sidecar_for_claim(
            &db,
            &claim_id,
            "old-runtime-claim-id",
            "legacy-action-mismatch-event",
        );
        let feedback = &sidecar.claims[0].feedback_rows[0];
        let payload_json = feedback_payload_json(feedback).expect("payload json");
        let submitted_at =
            canonical_feedback_submitted_at(&feedback.submitted_at).expect("submitted at");
        let existing_feedback = record_claim_feedback_replay(
            &ctx,
            &db,
            ClaimFeedbackReplayInput {
                feedback: ClaimFeedbackInput {
                    claim_id: claim_id.clone(),
                    action: FeedbackAction::ConfirmCurrent,
                    actor: feedback.actor.clone(),
                    actor_id: feedback.actor_id.clone(),
                    payload_json,
                },
                replay_event_id: feedback.feedback_id.clone(),
                submitted_at,
            },
        )
        .expect("record wrong-action feedback before crash");
        insert_claimed_legacy_replay_journal(&db, &sidecar, &claim_id);
        let checksum = authorize_sidecar(&db, &sidecar);

        let report = replay_claim_file_sidecar_corrections(
            &ctx,
            &db,
            "run-action-mismatch",
            &sidecar,
            &checksum,
        )
        .expect("wrong-action existing feedback is terminalized");

        assert_eq!(report.already_applied_count, 0);
        assert_eq!(report.failed_count, 1);
        assert_eq!(
            feedback_id_for_replay(&db, "legacy-action-mismatch-event"),
            existing_feedback.feedback_id
        );
        let (status, reason_code, detail_hash) =
            replay_journal_status_and_reason(&db, "legacy-action-mismatch-event");
        assert_eq!(status, "failed");
        assert_eq!(reason_code.as_deref(), Some("claim_feedback_replay_failed"));
        assert!(detail_hash.is_some());
        let applied_feedback_id: Option<String> = db
            .conn_ref()
            .query_row(
                "SELECT applied_feedback_id
                   FROM rebuild_correction_replay_events
                  WHERE sidecar_event_id = ?1",
                params!["legacy-action-mismatch-event"],
                |row| row.get(0),
            )
            .expect("read applied feedback id");
        assert_eq!(applied_feedback_id, None);
    }

    #[test]
    fn dos832_replay_claimed_legacy_collision_refuses_same_claim_different_content() {
        let db = test_db();
        let (clock, rng, external) = ctx_parts();
        let ctx = ServiceContext::test_live(&clock, &rng, &external);
        seed_account(&db);
        let claim_id = inserted_claim_id(
            commit_claim(&ctx, &db, proposal("Renewal risk is elevated")).expect("commit claim"),
        );
        let sidecar = sidecar_for_claim(
            &db,
            &claim_id,
            "old-runtime-claim-id",
            "legacy-content-mismatch-event",
        );
        let feedback = &sidecar.claims[0].feedback_rows[0];
        let action = feedback_action_from_slug(&feedback.action).expect("feedback action");
        let payload_json = feedback_payload_json(feedback).expect("payload json");
        let submitted_at =
            canonical_feedback_submitted_at(&feedback.submitted_at).expect("submitted at");
        let expected_content_hash =
            feedback_content_hash(feedback, action, payload_json.as_deref(), &submitted_at)
                .expect("expected content hash");
        let existing_feedback = record_claim_feedback_replay(
            &ctx,
            &db,
            ClaimFeedbackReplayInput {
                feedback: ClaimFeedbackInput {
                    claim_id: claim_id.clone(),
                    action,
                    actor: feedback.actor.clone(),
                    actor_id: feedback.actor_id.clone(),
                    payload_json: payload_json.clone(),
                },
                replay_event_id: feedback.feedback_id.clone(),
                submitted_at: "2026-05-01T08:30:01Z".to_string(),
            },
        )
        .expect("record mismatched-content feedback before crash");
        insert_claimed_legacy_replay_journal(&db, &sidecar, &claim_id);
        let checksum = authorize_sidecar(&db, &sidecar);

        let report = replay_claim_file_sidecar_corrections(
            &ctx,
            &db,
            "run-content-mismatch",
            &sidecar,
            &checksum,
        )
        .expect("mismatched-content existing feedback is terminalized");

        assert_eq!(report.already_applied_count, 0);
        assert_eq!(report.failed_count, 1);
        assert_eq!(
            feedback_id_for_replay(&db, "legacy-content-mismatch-event"),
            existing_feedback.feedback_id
        );
        let (status, reason_code, detail_hash) =
            replay_journal_status_and_reason(&db, "legacy-content-mismatch-event");
        assert_eq!(status, "failed");
        assert_eq!(reason_code.as_deref(), Some("claim_feedback_replay_failed"));
        assert!(detail_hash.is_some());
        let applied_feedback_id: Option<String> = db
            .conn_ref()
            .query_row(
                "SELECT applied_feedback_id
                   FROM rebuild_correction_replay_events
                  WHERE sidecar_event_id = ?1",
                params!["legacy-content-mismatch-event"],
                |row| row.get(0),
            )
            .expect("read applied feedback id");
        assert_eq!(applied_feedback_id, None);
        let stored_hash: String = db
            .conn_ref()
            .query_row(
                "SELECT feedback_content_hash
                   FROM rebuild_correction_replay_events
                  WHERE sidecar_event_id = ?1",
                params!["legacy-content-mismatch-event"],
                |row| row.get(0),
            )
            .expect("read terminal content hash");
        assert_eq!(stored_hash, expected_content_hash);
        assert_ne!(stored_hash, LEGACY_V282_UNSET_FEEDBACK_CONTENT_HASH);

        let retry = replay_claim_file_sidecar_corrections(
            &ctx,
            &db,
            "run-content-mismatch-retry",
            &sidecar,
            &checksum,
        )
        .expect("failed terminal retry is idempotent");
        assert_eq!(retry.failed_count, 1);
        assert_eq!(
            retry.events[0].reason_code.as_deref(),
            Some("claim_feedback_replay_failed")
        );
    }

    #[test]
    fn dos832_replay_rejects_retargeting_claimed_event_id() {
        let db = test_db();
        let (clock, rng, external) = ctx_parts();
        let ctx = ServiceContext::test_live(&clock, &rng, &external);
        seed_account(&db);
        let first_claim_id = inserted_claim_id(
            commit_claim(&ctx, &db, proposal("Renewal risk is elevated"))
                .expect("commit first claim"),
        );
        let first_sidecar = sidecar_for_claim(
            &db,
            &first_claim_id,
            "old-runtime-claim-id",
            "claimed-event",
        );
        let first_claim = &first_sidecar.claims[0];
        let first_feedback = &first_claim.feedback_rows[0];
        let action = feedback_action_from_slug(&first_feedback.action).expect("feedback action");
        let payload_json = feedback_payload_json(first_feedback).expect("payload json");
        let submitted_at =
            canonical_feedback_submitted_at(&first_feedback.submitted_at).expect("submitted at");
        let content_hash = feedback_content_hash(
            first_feedback,
            action,
            payload_json.as_deref(),
            &submitted_at,
        )
        .expect("content hash");
        let source_claim = SidecarClaimRef {
            runtime_claim_id: &first_claim.runtime_claim_id,
            identity: &first_claim.semantic_identity,
        };
        let claim = claim_replay_event(
            &db,
            ClaimReplayJournalInput {
                scope: ReplayScope {
                    run_id: "run-1",
                    sidecar_checksum: &sidecar_checksum(&first_sidecar),
                    sidecar_schema_version: first_sidecar.schema_version,
                },
                source_claim,
                resolved_claim_id: Some(first_claim_id.as_str()),
                sidecar_event_id: "claimed-event",
                action: &first_feedback.action,
                reason_code: None,
                feedback_content_hash: &content_hash,
            },
        )
        .expect("claim replay event");
        assert_eq!(claim, ReplayEventClaim::Claimed);

        let mut second_proposal = proposal("Renewal risk is elevated for a different field");
        second_proposal.field_path = Some("health.other_risk".to_string());
        let second_claim_id =
            inserted_claim_id(commit_claim(&ctx, &db, second_proposal).expect("commit second"));
        let second_sidecar = sidecar_for_claim(
            &db,
            &second_claim_id,
            "other-runtime-claim-id",
            "claimed-event",
        );

        let err = replay_authorized(&ctx, &db, "run-2", &second_sidecar)
            .expect_err("claimed event id must not retarget");

        assert!(
            matches!(err, RebuildError::InvalidSidecar(message) if message.contains("different target"))
        );
        assert_eq!(feedback_count_for_replay(&db, "claimed-event"), 0);
        assert_eq!(
            replay_journal_resolved_claim_id(&db, "claimed-event").as_deref(),
            Some(first_claim_id.as_str())
        );
    }

    #[test]
    fn dos832_replay_claimed_retry_does_not_self_stale_on_prior_version_event() {
        let db = test_db();
        let (clock, rng, external) = ctx_parts();
        let ctx = ServiceContext::test_live(&clock, &rng, &external);
        seed_account(&db);
        let fresh_claim_id = inserted_claim_id(
            commit_claim(&ctx, &db, proposal("Renewal risk is elevated"))
                .expect("commit fresh claim"),
        );
        let mut sidecar = sidecar_for_claim(
            &db,
            &fresh_claim_id,
            "old-runtime-claim-id",
            "claimed-retry-feedback",
        );
        sidecar.claims[0].feedback_rows[0].submitted_at = REPLAYED_FEEDBACK_TS.to_string();
        let checksum = authorize_sidecar(&db, &sidecar);

        let first = replay_claim_file_sidecar_corrections(&ctx, &db, "run-1", &sidecar, &checksum)
            .expect("first replay applies");
        assert_eq!(first.applied_count, 1);
        assert_eq!(feedback_count_for_replay(&db, "claimed-retry-feedback"), 1);
        let feedback_id = feedback_id_for_replay(&db, "claimed-retry-feedback");
        let recorded_signal_id = claim_feedback_recorded_signal_id(&feedback_id);
        assert_eq!(signal_count_for_id(&db, &recorded_signal_id), 1);
        db.conn_ref()
            .execute(
                "DELETE FROM signal_events WHERE id = ?1",
                params![&recorded_signal_id],
            )
            .expect("delete recorded feedback signal");
        assert_eq!(signal_count_for_id(&db, &recorded_signal_id), 0);
        let newer_replay_version_events: i64 = db
            .conn_ref()
            .query_row(
                "SELECT count(*)
                 FROM version_events
                 WHERE claim_id = ?1
                   AND event_kind = 'claim.corrected'
                   AND created_at > ?2",
                params![&fresh_claim_id, REPLAYED_FEEDBACK_TS],
                |row| row.get(0),
            )
            .expect("count replay-created version events");
        assert_eq!(newer_replay_version_events, 1);
        db.conn_ref()
            .execute(
                "UPDATE rebuild_correction_replay_events
                 SET status = 'claimed',
                     applied_feedback_id = NULL,
                     applied_at = NULL,
                     updated_at = ?2
                 WHERE sidecar_event_id = ?1",
                params!["claimed-retry-feedback", TS],
            )
            .expect("simulate crash before replay journal terminalization");

        let retry = replay_claim_file_sidecar_corrections(&ctx, &db, "run-2", &sidecar, &checksum)
            .expect("retry repairs claimed journal without self-stale");

        assert_eq!(retry.already_applied_count, 1);
        assert_eq!(retry.failed_count, 0);
        assert_eq!(feedback_count_for_replay(&db, "claimed-retry-feedback"), 1);
        assert_eq!(signal_count_for_id(&db, &recorded_signal_id), 1);
        let repaired_payload: String = db
            .conn_ref()
            .query_row(
                "SELECT value FROM signal_events WHERE id = ?1",
                params![&recorded_signal_id],
                |row| row.get(0),
            )
            .expect("read repaired signal payload");
        assert!(
            repaired_payload.contains("\"recovered\":true"),
            "retry must reach claim feedback repair path"
        );
        let (status, reason_code, _) =
            replay_journal_status_and_reason(&db, "claimed-retry-feedback");
        assert_eq!(status, "already_applied");
        assert_eq!(reason_code, None);
    }

    #[test]
    fn dos832_replay_same_sidecar_feedback_rows_do_not_self_stale() {
        let db = test_db();
        let (clock, rng, external) = ctx_parts();
        let ctx = ServiceContext::test_live(&clock, &rng, &external);
        seed_account(&db);
        let fresh_claim_id = inserted_claim_id(
            commit_claim(&ctx, &db, proposal("Renewal risk is elevated"))
                .expect("commit fresh claim"),
        );
        let mut sidecar = sidecar_for_claim(
            &db,
            &fresh_claim_id,
            "old-runtime-claim-id",
            "multi-feedback-1",
        );
        sidecar.claims[0].feedback_rows[0].submitted_at = REPLAYED_FEEDBACK_TS.to_string();
        let mut second_feedback = sidecar.claims[0].feedback_rows[0].clone();
        second_feedback.feedback_id = "multi-feedback-2".to_string();
        second_feedback.action = "confirm_current".to_string();
        second_feedback.submitted_at = "2026-05-01T08:31:00Z".to_string();
        sidecar.claims[0].feedback_rows.push(second_feedback);
        let checksum = authorize_sidecar(&db, &sidecar);

        let report = replay_claim_file_sidecar_corrections(&ctx, &db, "run-1", &sidecar, &checksum)
            .expect("same-sidecar feedback rows apply without self-stale");

        assert_eq!(report.applied_count, 2);
        assert_eq!(report.failed_count, 0);
        assert_eq!(feedback_count_for_replay(&db, "multi-feedback-1"), 1);
        assert_eq!(feedback_count_for_replay(&db, "multi-feedback-2"), 1);
        let (status, reason_code, _) = replay_journal_status_and_reason(&db, "multi-feedback-2");
        assert_eq!(status, "applied");
        assert_eq!(reason_code, None);
        let tagged_replay_version_events: i64 = db
            .conn_ref()
            .query_row(
                "SELECT count(*)
                 FROM version_events
                 WHERE claim_id = ?1
                   AND correction_event_log_id IN ('multi-feedback-1', 'multi-feedback-2')",
                params![&fresh_claim_id],
                |row| row.get(0),
            )
            .expect("count tagged replay version events");
        assert_eq!(tagged_replay_version_events, 2);
    }

    #[test]
    fn dos832_replay_mismatched_sidecar_event_does_not_mask_stale_later_row() {
        let db = test_db();
        let (clock, rng, external) = ctx_parts();
        let ctx = ServiceContext::test_live(&clock, &rng, &external);
        seed_account(&db);
        let fresh_claim_id = inserted_claim_id(
            commit_claim(&ctx, &db, proposal("Renewal risk is elevated"))
                .expect("commit fresh claim"),
        );
        let mut sidecar = sidecar_for_claim(
            &db,
            &fresh_claim_id,
            "old-runtime-claim-id",
            "mismatched-stale-1",
        );
        sidecar.claims[0].feedback_rows[0].submitted_at = "2026-05-01T08:30:00Z".to_string();
        let mut second_feedback = sidecar.claims[0].feedback_rows[0].clone();
        second_feedback.feedback_id = "mismatched-stale-2".to_string();
        second_feedback.action = "confirm_current".to_string();
        second_feedback.submitted_at = "2026-05-01T08:29:00Z".to_string();
        sidecar.claims[0].feedback_rows.push(second_feedback);

        let first_feedback = &sidecar.claims[0].feedback_rows[0];
        let action = feedback_action_from_slug(&first_feedback.action).expect("feedback action");
        let payload_json = feedback_payload_json(first_feedback).expect("payload json");
        record_claim_feedback_replay(
            &ctx,
            &db,
            ClaimFeedbackReplayInput {
                feedback: ClaimFeedbackInput {
                    claim_id: fresh_claim_id.clone(),
                    action,
                    actor: first_feedback.actor.clone(),
                    actor_id: first_feedback.actor_id.clone(),
                    payload_json,
                },
                replay_event_id: first_feedback.feedback_id.clone(),
                submitted_at: "2026-05-01T08:31:00Z".to_string(),
            },
        )
        .expect("record mismatched newer replay feedback");
        let checksum = authorize_sidecar(&db, &sidecar);

        let report = replay_claim_file_sidecar_corrections(
            &ctx,
            &db,
            "run-mismatched-stale",
            &sidecar,
            &checksum,
        )
        .expect("mismatched first row does not hide stale watermark");

        assert_eq!(report.applied_count, 0);
        assert_eq!(report.failed_count, 2);
        let (first_status, first_reason, _) =
            replay_journal_status_and_reason(&db, "mismatched-stale-1");
        assert_eq!(first_status, "failed");
        assert_eq!(
            first_reason.as_deref(),
            Some("claim_feedback_replay_failed")
        );
        let (second_status, second_reason, _) =
            replay_journal_status_and_reason(&db, "mismatched-stale-2");
        assert_eq!(second_status, "failed");
        assert_eq!(second_reason.as_deref(), Some("stale_replay_watermark"));
        assert_eq!(feedback_count_for_replay(&db, "mismatched-stale-1"), 1);
        assert_eq!(feedback_count_for_replay(&db, "mismatched-stale-2"), 0);
    }

    #[test]
    fn dos832_replay_rejects_sidecar_feedback_without_stable_event_id() {
        let db = test_db();
        let (clock, rng, external) = ctx_parts();
        let ctx = ServiceContext::test_live(&clock, &rng, &external);
        seed_account(&db);
        let fresh_claim_id = inserted_claim_id(
            commit_claim(&ctx, &db, proposal("Renewal risk is elevated"))
                .expect("commit fresh claim"),
        );
        let sidecar = sidecar_for_claim(&db, &fresh_claim_id, "old-runtime-claim-id", "");

        let err = replay_authorized(&ctx, &db, "run-1", &sidecar)
            .expect_err("missing event id must reject");

        assert!(matches!(err, RebuildError::InvalidSidecar(_)));
        assert_eq!(feedback_count_for_replay(&db, ""), 0);
        assert_eq!(replay_journal_count(&db), 0);
    }

    #[test]
    fn dos832_replay_rejects_sidecar_feedback_without_stable_actor_id() {
        let db = test_db();
        let (clock, rng, external) = ctx_parts();
        let ctx = ServiceContext::test_live(&clock, &rng, &external);
        seed_account(&db);
        let fresh_claim_id = inserted_claim_id(
            commit_claim(&ctx, &db, proposal("Renewal risk is elevated"))
                .expect("commit fresh claim"),
        );
        let mut sidecar = sidecar_for_claim(
            &db,
            &fresh_claim_id,
            "old-runtime-claim-id",
            "blank-actor-feedback",
        );
        sidecar.claims[0].feedback_rows[0].actor_id = Some("   ".to_string());
        let checksum = sidecar_checksum(&sidecar);

        let err = replay_claim_file_sidecar_corrections(&ctx, &db, "run-1", &sidecar, &checksum)
            .expect_err("blank actor_id must reject before replay writes");

        assert!(
            matches!(err, RebuildError::InvalidSidecar(message) if message.contains("stable actor_id"))
        );
        assert_eq!(feedback_count_for_replay(&db, "blank-actor-feedback"), 0);
        assert_eq!(replay_journal_count(&db), 0);
    }

    #[test]
    fn dos832_replay_rejects_unsupported_sidecar_schema_before_writes() {
        let db = test_db();
        let (clock, rng, external) = ctx_parts();
        let ctx = ServiceContext::test_live(&clock, &rng, &external);
        seed_account(&db);
        let fresh_claim_id = inserted_claim_id(
            commit_claim(&ctx, &db, proposal("Renewal risk is elevated"))
                .expect("commit fresh claim"),
        );
        let mut sidecar = sidecar_for_claim(
            &db,
            &fresh_claim_id,
            "old-runtime-claim-id",
            "old-feedback-schema",
        );
        sidecar.schema_version = CLAIM_FILE_SIDECAR_SCHEMA_VERSION + 1;

        let err = replay_authorized(&ctx, &db, "run-1", &sidecar)
            .expect_err("unsupported sidecar schema must reject");

        assert!(matches!(err, RebuildError::InvalidSidecar(_)));
        assert_eq!(feedback_count_for_replay(&db, "old-feedback-schema"), 0);
        assert_eq!(replay_journal_count(&db), 0);
    }

    #[test]
    fn dos832_replay_rejects_duplicate_feedback_ids_before_writes() {
        let db = test_db();
        let (clock, rng, external) = ctx_parts();
        let ctx = ServiceContext::test_live(&clock, &rng, &external);
        seed_account(&db);
        let fresh_claim_id = inserted_claim_id(
            commit_claim(&ctx, &db, proposal("Renewal risk is elevated"))
                .expect("commit fresh claim"),
        );
        let mut sidecar = sidecar_for_claim(
            &db,
            &fresh_claim_id,
            "old-runtime-claim-id",
            "dupe-feedback",
        );
        let duplicate_feedback = sidecar.claims[0].feedback_rows[0].clone();
        sidecar.claims[0].feedback_rows.push(duplicate_feedback);

        let err = replay_authorized(&ctx, &db, "run-1", &sidecar)
            .expect_err("duplicate event id must reject");

        assert!(matches!(err, RebuildError::InvalidSidecar(_)));
        assert_eq!(feedback_count_for_replay(&db, "dupe-feedback"), 0);
        assert_eq!(replay_journal_count(&db), 0);
    }

    #[test]
    fn dos832_replay_rejects_unsupported_later_feedback_action_before_writes() {
        let db = test_db();
        let (clock, rng, external) = ctx_parts();
        let ctx = ServiceContext::test_live(&clock, &rng, &external);
        seed_account(&db);
        let fresh_claim_id = inserted_claim_id(
            commit_claim(&ctx, &db, proposal("Renewal risk is elevated"))
                .expect("commit fresh claim"),
        );
        let mut sidecar = sidecar_for_claim(
            &db,
            &fresh_claim_id,
            "old-runtime-claim-id",
            "valid-before-bad-action",
        );
        let mut invalid_feedback = sidecar.claims[0].feedback_rows[0].clone();
        invalid_feedback.feedback_id = "bad-action-after-valid".to_string();
        invalid_feedback.action = "not_a_real_action".to_string();
        sidecar.claims[0].feedback_rows.push(invalid_feedback);
        let checksum = authorize_sidecar(&db, &sidecar);

        let err = replay_claim_file_sidecar_corrections(&ctx, &db, "run-1", &sidecar, &checksum)
            .expect_err("unsupported action must reject before any replay writes");

        assert!(
            matches!(err, RebuildError::InvalidSidecar(message) if message.contains("unsupported feedback action"))
        );
        assert_eq!(feedback_count_for_replay(&db, "valid-before-bad-action"), 0);
        assert_eq!(feedback_count_for_replay(&db, "bad-action-after-valid"), 0);
        assert_eq!(replay_journal_count(&db), 0);
    }

    #[test]
    fn dos832_replay_orphans_when_semantic_identity_is_missing() {
        let db = test_db();
        let (clock, rng, external) = ctx_parts();
        let ctx = ServiceContext::test_live(&clock, &rng, &external);
        seed_account(&db);
        let fresh_claim_id = inserted_claim_id(
            commit_claim(&ctx, &db, proposal("Renewal risk is elevated"))
                .expect("commit fresh claim"),
        );
        let mut sidecar = sidecar_for_claim(
            &db,
            &fresh_claim_id,
            "old-runtime-claim-id",
            "missing-feedback",
        );
        sidecar.claims[0]
            .semantic_identity
            .dedup_key_components_hash = "missing-semantic-identity".to_string();

        let report = replay_authorized(&ctx, &db, "run-1", &sidecar).expect("replay sidecar");

        assert_eq!(report.applied_count, 0);
        assert_eq!(report.orphaned_count, 1);
        assert_eq!(report.events[0].status, ReplayEventStatus::OrphanMissing);
        assert_eq!(
            report.events[0].reason_code.as_deref(),
            Some("semantic_identity_missing")
        );
        assert_eq!(feedback_count_for_replay(&db, "missing-feedback"), 0);
        let (status, reason_code, _) = replay_journal_status_and_reason(&db, "missing-feedback");
        assert_eq!(status, "orphan_missing");
        assert_eq!(reason_code.as_deref(), Some("semantic_identity_missing"));
    }

    #[test]
    fn dos832_replay_orphans_single_candidate_when_provenance_mismatches() {
        let db = test_db();
        let (clock, rng, external) = ctx_parts();
        let ctx = ServiceContext::test_live(&clock, &rng, &external);
        seed_account(&db);
        let fresh_claim_id = inserted_claim_id(
            commit_claim(&ctx, &db, proposal("Renewal risk is elevated"))
                .expect("commit fresh claim"),
        );
        let mut sidecar = sidecar_for_claim(
            &db,
            &fresh_claim_id,
            "old-runtime-claim-id",
            "source-mismatch-feedback",
        );
        sidecar.claims[0].feedback_rows[0].action = "wrong_source".to_string();
        sidecar.claims[0].feedback_rows[0].payload_json =
            Some(serde_json::json!({ "source_ref": "fixture://source-1" }));
        sidecar.claims[0].semantic_identity.source_ref = Some("fixture://source-2".to_string());
        sidecar.claims[0].semantic_identity.source_asof = Some("2026-06-05T13:00:00Z".to_string());

        let report = replay_authorized(&ctx, &db, "run-1", &sidecar).expect("replay sidecar");

        assert_eq!(report.applied_count, 0);
        assert_eq!(report.orphaned_count, 1);
        assert_eq!(report.events[0].status, ReplayEventStatus::OrphanMissing);
        assert_eq!(
            feedback_count_for_replay(&db, "source-mismatch-feedback"),
            0
        );
    }

    #[test]
    fn dos832_replay_recovers_orphan_when_same_sidecar_later_resolves() {
        let db = test_db();
        let builder_db = test_db();
        let (clock, rng, external) = ctx_parts();
        let ctx = ServiceContext::test_live(&clock, &rng, &external);
        seed_account(&db);
        seed_account(&builder_db);

        let mut matching_proposal = proposal("Renewal risk is elevated");
        matching_proposal.source_ref = Some("fixture://source-2".to_string());
        matching_proposal.source_asof = Some("2026-06-05T13:00:00Z".to_string());
        let future_claim_id = inserted_claim_id(
            commit_claim(&ctx, &builder_db, matching_proposal.clone())
                .expect("commit future sidecar identity claim"),
        );
        let sidecar = sidecar_for_claim(
            &builder_db,
            &future_claim_id,
            "old-runtime-claim-id",
            "recoverable-orphan-feedback",
        );

        let first = replay_authorized(&ctx, &db, "run-1", &sidecar).expect("first replay");
        assert_eq!(first.orphaned_count, 1);
        assert_eq!(
            feedback_count_for_replay(&db, "recoverable-orphan-feedback"),
            0
        );

        let resolved_claim_id = inserted_claim_id(
            commit_claim(&ctx, &db, matching_proposal).expect("commit matching claim"),
        );
        let second = replay_authorized(&ctx, &db, "run-2", &sidecar).expect("retry replay");

        assert_eq!(second.applied_count, 1);
        assert_eq!(second.orphaned_count, 0);
        assert_eq!(
            second.events[0].resolved_claim_id.as_deref(),
            Some(resolved_claim_id.as_str())
        );
        assert_eq!(
            feedback_count_for_replay(&db, "recoverable-orphan-feedback"),
            1
        );
        let (status, reason_code, _) =
            replay_journal_status_and_reason(&db, "recoverable-orphan-feedback");
        assert_eq!(status, "applied");
        assert_eq!(reason_code, None);
    }

    #[test]
    fn dos832_replay_recovers_migrated_orphan_with_legacy_content_placeholder() {
        let db = test_db();
        let builder_db = test_db();
        let (clock, rng, external) = ctx_parts();
        let ctx = ServiceContext::test_live(&clock, &rng, &external);
        seed_account(&db);
        seed_account(&builder_db);

        let mut matching_proposal = proposal("Renewal risk is elevated");
        matching_proposal.source_ref = Some("fixture://source-2".to_string());
        matching_proposal.source_asof = Some("2026-06-05T13:00:00Z".to_string());
        let future_claim_id = inserted_claim_id(
            commit_claim(&ctx, &builder_db, matching_proposal.clone())
                .expect("commit future sidecar identity claim"),
        );
        let sidecar = sidecar_for_claim(
            &builder_db,
            &future_claim_id,
            "old-runtime-claim-id",
            "legacy-placeholder-orphan-feedback",
        );

        let first = replay_authorized(&ctx, &db, "run-1", &sidecar).expect("first replay");
        assert_eq!(first.orphaned_count, 1);
        db.conn_ref()
            .execute(
                "UPDATE rebuild_correction_replay_events
                    SET feedback_content_hash = ?1,
                        sidecar_checksum = ?2
                  WHERE sidecar_event_id = ?3",
                params![
                    LEGACY_V282_UNSET_FEEDBACK_CONTENT_HASH,
                    LEGACY_V283_UNSET_SIDECAR_CHECKSUM,
                    "legacy-placeholder-orphan-feedback",
                ],
            )
            .expect("simulate v283 migrated orphan placeholder");

        let resolved_claim_id = inserted_claim_id(
            commit_claim(&ctx, &db, matching_proposal).expect("commit matching claim"),
        );
        let mut drifted_sidecar = sidecar.clone();
        drifted_sidecar.claims[0].claim_text =
            "Renewal risk is elevated in the sidecar projection".to_string();
        let drifted_checksum = authorize_sidecar(&db, &drifted_sidecar);
        let feedback = &sidecar.claims[0].feedback_rows[0];
        let action = feedback_action_from_slug(&feedback.action).expect("feedback action");
        let payload_json = feedback_payload_json(feedback).expect("payload json");
        let submitted_at =
            canonical_feedback_submitted_at(&feedback.submitted_at).expect("submitted at");
        let expected_content_hash =
            feedback_content_hash(feedback, action, payload_json.as_deref(), &submitted_at)
                .expect("expected content hash");

        let second = replay_claim_file_sidecar_corrections(
            &ctx,
            &db,
            "run-2",
            &drifted_sidecar,
            &drifted_checksum,
        )
        .expect("migrated orphan placeholder reclaims");

        assert_eq!(second.applied_count, 1);
        assert_eq!(second.orphaned_count, 0);
        assert_eq!(
            second.events[0].resolved_claim_id.as_deref(),
            Some(resolved_claim_id.as_str())
        );
        let (status, reason_code, _) =
            replay_journal_status_and_reason(&db, "legacy-placeholder-orphan-feedback");
        assert_eq!(status, "applied");
        assert_eq!(reason_code, None);
        let (stored_hash, stored_checksum): (String, String) = db
            .conn_ref()
            .query_row(
                "SELECT feedback_content_hash, sidecar_checksum
                   FROM rebuild_correction_replay_events
                  WHERE sidecar_event_id = ?1",
                params!["legacy-placeholder-orphan-feedback"],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("read migrated orphan terminal row");
        assert_eq!(stored_hash, expected_content_hash);
        assert_eq!(stored_checksum, LEGACY_V283_UNSET_SIDECAR_CHECKSUM);

        let original_checksum = authorize_sidecar(&db, &sidecar);
        let original_retry = replay_claim_file_sidecar_corrections(
            &ctx,
            &db,
            "run-original",
            &sidecar,
            &original_checksum,
        )
        .expect("original sidecar remains recoverable");
        assert_eq!(original_retry.already_applied_count, 1);
        assert_eq!(original_retry.failed_count, 0);

        let mut changed_content_sidecar = sidecar.clone();
        changed_content_sidecar.claims[0].feedback_rows[0].submitted_at =
            "2026-05-01T08:30:01Z".to_string();
        let changed_content_checksum = authorize_sidecar(&db, &changed_content_sidecar);
        let err = replay_claim_file_sidecar_corrections(
            &ctx,
            &db,
            "run-changed-content",
            &changed_content_sidecar,
            &changed_content_checksum,
        )
        .expect_err("checksum-neutral terminal row still rejects changed content");
        assert!(
            matches!(err, RebuildError::InvalidSidecar(message) if message.contains("feedback content"))
        );
        assert_eq!(
            feedback_count_for_replay(&db, "legacy-placeholder-orphan-feedback"),
            1
        );
        let (final_status, final_reason, _) =
            replay_journal_status_and_reason(&db, "legacy-placeholder-orphan-feedback");
        assert_eq!(final_status, "applied");
        assert_eq!(final_reason, None);
        let (final_hash, final_checksum): (String, String) = db
            .conn_ref()
            .query_row(
                "SELECT feedback_content_hash, sidecar_checksum
                   FROM rebuild_correction_replay_events
                  WHERE sidecar_event_id = ?1",
                params!["legacy-placeholder-orphan-feedback"],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("read unchanged migrated orphan terminal row");
        assert_eq!(final_hash, stored_hash);
        assert_eq!(final_checksum, stored_checksum);
    }

    #[test]
    fn dos832_replay_migrated_orphan_reclaim_failure_keeps_placeholders_retryable() {
        let db = test_db();
        let builder_db = test_db();
        let (clock, rng, external) = ctx_parts();
        let ctx = ServiceContext::test_live(&clock, &rng, &external);
        seed_account(&db);
        seed_account(&builder_db);

        let mut matching_proposal = proposal("Renewal risk is elevated");
        matching_proposal.source_ref = Some("fixture://source-2".to_string());
        matching_proposal.source_asof = Some("2026-06-05T13:00:00Z".to_string());
        let future_claim_id = inserted_claim_id(
            commit_claim(&ctx, &builder_db, matching_proposal.clone())
                .expect("commit future sidecar identity claim"),
        );
        let sidecar = sidecar_for_claim(
            &builder_db,
            &future_claim_id,
            "old-runtime-claim-id",
            "legacy-placeholder-orphan-failure",
        );

        let first = replay_authorized(&ctx, &db, "run-1", &sidecar).expect("first replay");
        assert_eq!(first.orphaned_count, 1);
        db.conn_ref()
            .execute(
                "UPDATE rebuild_correction_replay_events
                    SET feedback_content_hash = ?1,
                        sidecar_checksum = ?2
                  WHERE sidecar_event_id = ?3",
                params![
                    LEGACY_V282_UNSET_FEEDBACK_CONTENT_HASH,
                    LEGACY_V283_UNSET_SIDECAR_CHECKSUM,
                    "legacy-placeholder-orphan-failure",
                ],
            )
            .expect("simulate v283 migrated orphan placeholder");
        let resolved_claim_id = inserted_claim_id(
            commit_claim(&ctx, &db, matching_proposal).expect("commit matching claim"),
        );

        let mut bad_content_sidecar = sidecar.clone();
        bad_content_sidecar.claims[0].feedback_rows[0].submitted_at =
            "2026-05-01T08:30:01Z".to_string();
        let bad_checksum = authorize_sidecar(&db, &bad_content_sidecar);
        db.conn_ref()
            .execute_batch(
                "CREATE TRIGGER fail_migrated_orphan_replay_insert
                 BEFORE INSERT ON claim_feedback
                 WHEN NEW.replay_event_id = 'legacy-placeholder-orphan-failure'
                 BEGIN
                    SELECT RAISE(FAIL, 'temporary migrated orphan replay failure');
                 END;",
            )
            .expect("install migrated orphan replay failure trigger");

        let err = replay_claim_file_sidecar_corrections(
            &ctx,
            &db,
            "run-bad-content",
            &bad_content_sidecar,
            &bad_checksum,
        )
        .expect_err("transient replay failure remains retryable");
        assert!(
            matches!(err, RebuildError::ClaimReplay(message) if message.contains("temporary migrated orphan replay failure"))
        );
        assert_eq!(
            replay_journal_resolved_claim_id(&db, "legacy-placeholder-orphan-failure").as_deref(),
            Some(resolved_claim_id.as_str())
        );
        let (status, reason_code, _) =
            replay_journal_status_and_reason(&db, "legacy-placeholder-orphan-failure");
        assert_eq!(status, "claimed");
        assert_eq!(reason_code, None);
        let (stored_hash, stored_checksum): (String, String) = db
            .conn_ref()
            .query_row(
                "SELECT feedback_content_hash, sidecar_checksum
                   FROM rebuild_correction_replay_events
                  WHERE sidecar_event_id = ?1",
                params!["legacy-placeholder-orphan-failure"],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("read retryable migrated orphan row");
        assert_eq!(stored_hash, LEGACY_V282_UNSET_FEEDBACK_CONTENT_HASH);
        assert_eq!(stored_checksum, LEGACY_V283_UNSET_SIDECAR_CHECKSUM);

        db.conn_ref()
            .execute_batch("DROP TRIGGER fail_migrated_orphan_replay_insert;")
            .expect("remove migrated orphan replay failure trigger");
        let original_checksum = authorize_sidecar(&db, &sidecar);
        let retry = replay_claim_file_sidecar_corrections(
            &ctx,
            &db,
            "run-original",
            &sidecar,
            &original_checksum,
        )
        .expect("original sidecar remains recoverable");
        assert_eq!(retry.applied_count, 1);
        assert_eq!(retry.failed_count, 0);
    }

    #[test]
    fn dos832_replay_orphans_ambiguous_semantic_identity_without_mutating_feedback() {
        let db = test_db();
        let (clock, rng, external) = ctx_parts();
        let ctx = ServiceContext::test_live(&clock, &rng, &external);
        seed_account(&db);
        let fresh_claim_id = inserted_claim_id(
            commit_claim(&ctx, &db, proposal("Renewal risk is elevated"))
                .expect("commit fresh claim"),
        );
        let mut tombstone = proposal("Renewal risk is elevated");
        tombstone.tombstone = Some(TombstoneSpec {
            retraction_reason: "test tombstone shadow".to_string(),
            expires_at: None,
        });
        let tombstone_id =
            inserted_claim_id(commit_claim(&ctx, &db, tombstone).expect("commit tombstone claim"));
        assert_ne!(fresh_claim_id, tombstone_id);
        let sidecar = sidecar_for_claim(
            &db,
            &fresh_claim_id,
            "old-runtime-claim-id",
            "ambiguous-feedback",
        );

        let report = replay_authorized(&ctx, &db, "run-1", &sidecar).expect("replay sidecar");

        assert_eq!(report.applied_count, 0);
        assert_eq!(report.orphaned_count, 1);
        assert_eq!(report.events[0].status, ReplayEventStatus::OrphanAmbiguous);
        assert_eq!(
            report.events[0].reason_code.as_deref(),
            Some("semantic_identity_ambiguous_2")
        );
        assert_eq!(feedback_count_for_replay(&db, "ambiguous-feedback"), 0);
    }

    #[test]
    fn dos832_replay_marks_invalid_feedback_failed_and_continues() {
        let db = test_db();
        let (clock, rng, external) = ctx_parts();
        let ctx = ServiceContext::test_live(&clock, &rng, &external);
        seed_account(&db);
        let fresh_claim_id = inserted_claim_id(
            commit_claim(&ctx, &db, proposal("Renewal risk is elevated"))
                .expect("commit fresh claim"),
        );
        let mut sidecar = sidecar_for_claim(
            &db,
            &fresh_claim_id,
            "old-runtime-claim-id",
            "invalid-feedback",
        );
        sidecar.claims[0].feedback_rows[0].action = "wrong_source".to_string();
        let mut valid_feedback = sidecar.claims[0].feedback_rows[0].clone();
        valid_feedback.feedback_id = "valid-feedback-after-failure".to_string();
        valid_feedback.action = "cannot_verify".to_string();
        sidecar.claims[0].feedback_rows.push(valid_feedback);

        let report = replay_authorized(&ctx, &db, "run-1", &sidecar).expect("replay sidecar");

        assert_eq!(report.failed_count, 1);
        assert_eq!(report.applied_count, 1);
        assert_eq!(feedback_count_for_replay(&db, "invalid-feedback"), 0);
        assert_eq!(
            feedback_count_for_replay(&db, "valid-feedback-after-failure"),
            1
        );
        let (status, reason_code, detail_hash) =
            replay_journal_status_and_reason(&db, "invalid-feedback");
        assert_eq!(status, "failed");
        assert_eq!(reason_code.as_deref(), Some("claim_feedback_replay_failed"));
        assert!(detail_hash.is_some_and(|hash| hash.len() == 64));
    }

    #[test]
    fn dos832_replay_retryable_claim_service_failure_remains_claimed() {
        let db = test_db();
        let (clock, rng, external) = ctx_parts();
        let ctx = ServiceContext::test_live(&clock, &rng, &external);
        seed_account(&db);
        let fresh_claim_id = inserted_claim_id(
            commit_claim(&ctx, &db, proposal("Renewal risk is elevated"))
                .expect("commit fresh claim"),
        );
        let sidecar = sidecar_for_claim(
            &db,
            &fresh_claim_id,
            "old-runtime-claim-id",
            "retryable-db-error-feedback",
        );
        let checksum = authorize_sidecar(&db, &sidecar);
        db.conn_ref()
            .execute_batch(
                "CREATE TRIGGER fail_retryable_replay_insert
                 BEFORE INSERT ON claim_feedback
                 WHEN NEW.replay_event_id = 'retryable-db-error-feedback'
                 BEGIN
                    SELECT RAISE(FAIL, 'temporary replay insert failure');
                 END;",
            )
            .expect("install transient failure trigger");

        let first = replay_claim_file_sidecar_corrections(&ctx, &db, "run-1", &sidecar, &checksum)
            .expect_err("transient db failure should abort replay run");
        assert!(
            matches!(&first, RebuildError::ClaimReplay(message) if message.contains("temporary replay insert failure")),
            "unexpected error: {first:?}"
        );
        let (status, reason_code, _) =
            replay_journal_status_and_reason(&db, "retryable-db-error-feedback");
        assert_eq!(status, "claimed");
        assert_eq!(reason_code, None);
        assert_eq!(
            feedback_count_for_replay(&db, "retryable-db-error-feedback"),
            0
        );

        db.conn_ref()
            .execute_batch("DROP TRIGGER fail_retryable_replay_insert;")
            .expect("remove transient failure trigger");
        let second = replay_claim_file_sidecar_corrections(&ctx, &db, "run-2", &sidecar, &checksum)
            .expect("retry succeeds");

        assert_eq!(second.applied_count, 1);
        assert_eq!(
            feedback_count_for_replay(&db, "retryable-db-error-feedback"),
            1
        );
    }

    #[test]
    fn dos832_replay_rejects_retargeting_failed_event_id() {
        let db = test_db();
        let (clock, rng, external) = ctx_parts();
        let ctx = ServiceContext::test_live(&clock, &rng, &external);
        seed_account(&db);
        let first_claim_id = inserted_claim_id(
            commit_claim(&ctx, &db, proposal("Renewal risk is elevated"))
                .expect("commit first claim"),
        );
        let mut failed_sidecar =
            sidecar_for_claim(&db, &first_claim_id, "old-runtime-claim-id", "event-1");
        failed_sidecar.claims[0].feedback_rows[0].action = "wrong_source".to_string();
        let first_report = replay_authorized(&ctx, &db, "run-1", &failed_sidecar)
            .expect("first replay fails terminally");
        assert_eq!(first_report.failed_count, 1);

        let mut second_proposal = proposal("Renewal risk is elevated for a different field");
        second_proposal.field_path = Some("health.other_risk".to_string());
        let second_claim_id =
            inserted_claim_id(commit_claim(&ctx, &db, second_proposal).expect("commit second"));
        let second_sidecar =
            sidecar_for_claim(&db, &second_claim_id, "other-runtime-claim-id", "event-1");

        let err = replay_authorized(&ctx, &db, "run-2", &second_sidecar)
            .expect_err("failed event id must not retarget");

        assert!(
            matches!(err, RebuildError::InvalidSidecar(message) if message.contains("different target"))
        );
        assert_eq!(feedback_count_for_replay(&db, "event-1"), 0);
        let (status, reason_code, _) = replay_journal_status_and_reason(&db, "event-1");
        assert_eq!(status, "failed");
        assert_eq!(reason_code.as_deref(), Some("claim_feedback_replay_failed"));
        assert_eq!(
            replay_journal_resolved_claim_id(&db, "event-1").as_deref(),
            Some(first_claim_id.as_str())
        );
    }

    #[test]
    fn dos832_replay_rejects_retargeting_orphaned_event_id() {
        let db = test_db();
        let (clock, rng, external) = ctx_parts();
        let ctx = ServiceContext::test_live(&clock, &rng, &external);
        seed_account(&db);
        let first_claim_id = inserted_claim_id(
            commit_claim(&ctx, &db, proposal("Renewal risk is elevated"))
                .expect("commit first claim"),
        );
        let mut orphan_sidecar =
            sidecar_for_claim(&db, &first_claim_id, "old-runtime-claim-id", "event-2");
        orphan_sidecar.claims[0]
            .semantic_identity
            .dedup_key_components_hash = "missing-semantic-identity".to_string();
        let first_report =
            replay_authorized(&ctx, &db, "run-1", &orphan_sidecar).expect("first replay orphans");
        assert_eq!(first_report.orphaned_count, 1);

        let mut second_proposal = proposal("Renewal risk is elevated for a different field");
        second_proposal.field_path = Some("health.other_risk".to_string());
        let second_claim_id =
            inserted_claim_id(commit_claim(&ctx, &db, second_proposal).expect("commit second"));
        let second_sidecar =
            sidecar_for_claim(&db, &second_claim_id, "other-runtime-claim-id", "event-2");

        let err = replay_authorized(&ctx, &db, "run-2", &second_sidecar)
            .expect_err("orphaned event id must not retarget");

        assert!(
            matches!(err, RebuildError::InvalidSidecar(message) if message.contains("different target"))
        );
        assert_eq!(feedback_count_for_replay(&db, "event-2"), 0);
        let (status, reason_code, _) = replay_journal_status_and_reason(&db, "event-2");
        assert_eq!(status, "orphan_missing");
        assert_eq!(reason_code.as_deref(), Some("semantic_identity_missing"));
        assert_eq!(replay_journal_resolved_claim_id(&db, "event-2"), None);
    }

    #[test]
    fn dos832_replay_terminal_status_must_match_original_target() {
        let db = test_db();
        let (clock, rng, external) = ctx_parts();
        let ctx = ServiceContext::test_live(&clock, &rng, &external);
        seed_account(&db);
        let first_claim_id = inserted_claim_id(
            commit_claim(&ctx, &db, proposal("Renewal risk is elevated"))
                .expect("commit first claim"),
        );
        let mut second_proposal = proposal("Renewal risk is elevated for a different field");
        second_proposal.field_path = Some("health.other_risk".to_string());
        let second_claim_id =
            inserted_claim_id(commit_claim(&ctx, &db, second_proposal).expect("commit second"));
        let first_sidecar = sidecar_for_claim(
            &db,
            &first_claim_id,
            "old-runtime-claim-id",
            "reused-feedback",
        );
        replay_authorized(&ctx, &db, "run-1", &first_sidecar).expect("first replay");

        let second_sidecar = sidecar_for_claim(
            &db,
            &second_claim_id,
            "other-runtime-claim-id",
            "reused-feedback",
        );
        let err = replay_authorized(&ctx, &db, "run-2", &second_sidecar)
            .expect_err("reused terminal event id must reject target mismatch");

        assert!(
            matches!(err, RebuildError::InvalidSidecar(message) if message.contains("different target"))
        );
        assert_eq!(feedback_count_for_replay(&db, "reused-feedback"), 1);
    }
}
