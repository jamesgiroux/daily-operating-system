//! First-class rebuild helpers for DOS-832.
//!
//! This module owns rebuild replay planning and reporting. The L1a slice is
//! deliberately scoped to current-encrypted/Replica proof: it replays W3 claim
//! file correction sidecars through the claim service, and it records which
//! storage/cutover claims remain unproven until DOS-831 lands.

use std::collections::HashSet;

use chrono::Utc;
use rusqlite::{params, OptionalExtension};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::abilities::feedback::FeedbackAction;
use crate::db::ActionDb;
use crate::services::claim_files::{
    semantic_identity_for_loaded_claim, source_content_hash_for_claim, ClaimFileClaim,
    ClaimFileFeedbackRow, ClaimFileSidecar, ClaimSemanticIdentityV1,
    CLAIM_FILE_LEGACY_SIDECAR_SCHEMA_VERSION, CLAIM_FILE_SIDECAR_SCHEMA_VERSION,
};
use crate::services::claims::{
    claim_feedback_replay_content_hash, load_claim_by_id, record_claim_feedback_replay,
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
) -> Result<CorrectionReplayReport, RebuildError> {
    if run_id.trim().is_empty() {
        return Err(RebuildError::InvalidSidecar(
            "run_id is required for rebuild replay".to_string(),
        ));
    }
    validate_replay_sidecar(sidecar)?;

    let scope = ReplayScope {
        run_id,
        sidecar_schema_version: sidecar.schema_version,
    };
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
                Resolution::Resolved(claim_id) => {
                    replay_feedback_event(ctx, db, scope, source_claim, claim_id, feedback_ref)?
                }
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

fn sidecar_feedback_event_id(
    sidecar: &ClaimFileSidecar,
    claim: &ClaimFileClaim,
    feedback: &ClaimFileFeedbackRow,
    feedback_index: usize,
) -> Result<String, RebuildError> {
    let explicit = feedback.feedback_id.trim();
    if !explicit.is_empty() {
        return Ok(explicit.to_string());
    }
    if sidecar.schema_version != CLAIM_FILE_LEGACY_SIDECAR_SCHEMA_VERSION {
        return Err(RebuildError::InvalidSidecar(
            "sidecar feedback row missing stable feedback_id".to_string(),
        ));
    }
    let action = feedback_action_from_slug(&feedback.action)?;
    let payload_json = feedback_payload_json(feedback)?;
    let content_hash = feedback_content_hash(feedback, action, payload_json.as_deref())?;
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
    hasher.update(feedback.submitted_at.as_bytes());
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
) -> Result<String, RebuildError> {
    claim_feedback_replay_content_hash(
        action,
        &feedback.actor,
        feedback.actor_id.as_deref(),
        payload_json,
    )
    .map_err(|error| RebuildError::InvalidSidecar(error.to_string()))
}

fn replay_feedback_event(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    scope: ReplayScope<'_>,
    source_claim: SidecarClaimRef<'_>,
    resolved_claim_id: &str,
    feedback: SidecarFeedbackRef<'_>,
) -> Result<ReplayEventOutcome, RebuildError> {
    let payload_json = feedback_payload_json(feedback.row)?;
    let content_hash =
        feedback_content_hash(feedback.row, feedback.action, payload_json.as_deref())?;
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

    let outcome = match record_claim_feedback_replay(
        ctx,
        db,
        ClaimFeedbackReplayInput {
            feedback: ClaimFeedbackInput {
                claim_id: resolved_claim_id.to_string(),
                action: feedback.action,
                actor: feedback.row.actor.clone(),
                actor_id: feedback.row.actor_id.clone(),
                payload_json,
            },
            replay_event_id: feedback.event_id.to_string(),
        },
    ) {
        Ok(outcome) => outcome,
        Err(error) => {
            let reason_code = "claim_feedback_replay_failed";
            let reason_detail_hash = reason_detail_hash(&error.to_string());
            mark_replay_event_terminal(
                db,
                feedback.event_id,
                ReplayEventStatus::Failed.as_str(),
                Some(reason_code),
                Some(&reason_detail_hash),
                None,
            )?;
            return Ok(ReplayEventOutcome {
                sidecar_event_id: feedback.event_id.to_string(),
                source_runtime_claim_id: source_claim.runtime_claim_id.to_string(),
                resolved_claim_id: Some(resolved_claim_id.to_string()),
                action: feedback.row.action.clone(),
                status: ReplayEventStatus::Failed,
                reason_code: Some(reason_code.to_string()),
                applied_feedback_id: None,
            });
        }
    };

    let status = if outcome.applied_at_pending {
        ReplayEventStatus::Applied
    } else {
        ReplayEventStatus::AlreadyApplied
    };
    mark_replay_event_terminal(
        db,
        feedback.event_id,
        status.as_str(),
        None,
        None,
        Some(&outcome.feedback_id),
    )?;
    Ok(ReplayEventOutcome {
        sidecar_event_id: feedback.event_id.to_string(),
        source_runtime_claim_id: source_claim.runtime_claim_id.to_string(),
        resolved_claim_id: Some(resolved_claim_id.to_string()),
        action: feedback.row.action.clone(),
        status,
        reason_code: None,
        applied_feedback_id: Some(outcome.feedback_id),
    })
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
    let content_hash =
        feedback_content_hash(feedback.row, feedback.action, payload_json.as_deref())?;
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
            "SELECT sidecar_schema_version, source_runtime_claim_id, resolved_claim_id,
                    action, status, reason_code, semantic_identity_hash,
                    feedback_content_hash, applied_feedback_id
             FROM rebuild_correction_replay_events
             WHERE sidecar_event_id = ?1",
            params![sidecar_event_id],
            |row| {
                let schema_version = row.get::<_, i64>(0)?;
                Ok(ExistingReplayEvent {
                    sidecar_schema_version: schema_version as u32,
                    source_runtime_claim_id: row.get(1)?,
                    resolved_claim_id: row.get(2)?,
                    action: row.get(3)?,
                    status: row.get(4)?,
                    reason_code: row.get(5)?,
                    semantic_identity_hash: row.get(6)?,
                    feedback_content_hash: row.get(7)?,
                    applied_feedback_id: row.get(8)?,
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
        repair_claimed_legacy_content_hash(db, &existing, &input)?;
        increment_claimed_attempt_count(db, input.sidecar_event_id)?;
        return replay_event_claim_from_existing(existing);
    }

    let now = Utc::now().to_rfc3339();
    let inserted = db.conn_ref().execute(
        "INSERT INTO rebuild_correction_replay_events (
            sidecar_event_id, run_id, sidecar_schema_version, source_runtime_claim_id,
            resolved_claim_id, action, status, reason_code, semantic_identity_hash,
            feedback_content_hash, attempt_count, claimed_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'claimed', ?7, ?8, ?9, 1, ?10, ?10)
         ON CONFLICT(sidecar_event_id) DO NOTHING",
        params![
            input.sidecar_event_id,
            input.scope.run_id,
            input.scope.sidecar_schema_version as i64,
            input.source_claim.runtime_claim_id,
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
    repair_claimed_legacy_content_hash(db, &existing, &input)?;
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
        || existing.resolved_claim_id.as_deref() != input.resolved_claim_id
        || existing.action != input.action
        || existing.semantic_identity_hash != input.source_claim.identity.dedup_key_components_hash
    {
        return Err(RebuildError::InvalidSidecar(
            "sidecar replay event already recorded for a different target or feedback content"
                .to_string(),
        ));
    }
    let content_hash_matches = existing.feedback_content_hash == input.feedback_content_hash
        || (existing.status == "claimed"
            && existing.feedback_content_hash == LEGACY_V282_UNSET_FEEDBACK_CONTENT_HASH);
    if !content_hash_matches {
        return Err(RebuildError::InvalidSidecar(
            "sidecar replay event already recorded for a different target or feedback content"
                .to_string(),
        ));
    }
    Ok(())
}

fn repair_claimed_legacy_content_hash(
    db: &ActionDb,
    existing: &ExistingReplayEvent,
    input: &ClaimReplayJournalInput<'_>,
) -> Result<(), RebuildError> {
    if existing.status != "claimed"
        || existing.feedback_content_hash != LEGACY_V282_UNSET_FEEDBACK_CONTENT_HASH
    {
        return Ok(());
    }
    db.conn_ref().execute(
        "UPDATE rebuild_correction_replay_events
         SET feedback_content_hash = ?1,
             updated_at = ?2
         WHERE sidecar_event_id = ?3
           AND status = 'claimed'
           AND feedback_content_hash = ?4",
        params![
            input.feedback_content_hash,
            Utc::now().to_rfc3339(),
            input.sidecar_event_id,
            LEGACY_V282_UNSET_FEEDBACK_CONTENT_HASH,
        ],
    )?;
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

fn mark_replay_event_terminal(
    db: &ActionDb,
    sidecar_event_id: &str,
    status: &str,
    reason_code: Option<&str>,
    reason_detail_hash: Option<&str>,
    applied_feedback_id: Option<&str>,
) -> Result<(), RebuildError> {
    let now = Utc::now().to_rfc3339();
    db.conn_ref().execute(
        "UPDATE rebuild_correction_replay_events
         SET status = ?1,
             reason_code = COALESCE(?2, reason_code),
             reason_detail_hash = COALESCE(?3, reason_detail_hash),
             applied_feedback_id = COALESCE(?4, applied_feedback_id),
             applied_at = CASE WHEN ?1 IN ('applied', 'already_applied') THEN ?5 ELSE applied_at END,
             updated_at = ?5
         WHERE sidecar_event_id = ?6
           AND NOT (
             status IN ('applied', 'already_applied')
             AND ?1 = 'failed'
           )",
        params![
            status,
            reason_code,
            reason_detail_hash,
            applied_feedback_id,
            &now,
            sidecar_event_id
        ],
    )?;
    Ok(())
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
    use crate::services::claims::{commit_claim, ClaimProposal, CommittedClaim, TombstoneSpec};
    use crate::services::context::{ExternalClients, FixedClock, SeedableRng, ServiceContext};
    use abilities_runtime::types::{ClaimSensitivity, TemporalScope};

    const TS: &str = "2026-06-05T12:00:00Z";
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

    fn feedback_count_for_replay(db: &ActionDb, replay_event_id: &str) -> i64 {
        db.conn_ref()
            .query_row(
                "SELECT count(*) FROM claim_feedback WHERE replay_event_id = ?1",
                params![replay_event_id],
                |row| row.get(0),
            )
            .expect("count replay feedback")
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
        let sidecar = sidecar_for_claim(
            &db,
            &fresh_claim_id,
            "old-runtime-claim-id",
            "old-feedback-1",
        );

        let report = replay_claim_file_sidecar_corrections(&ctx, &db, "run-1", &sidecar)
            .expect("replay sidecar");

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
        let (verification_state, claim_version) =
            claim_verification_state_and_version(&db, &fresh_claim_id);
        assert_eq!(verification_state, "contested");
        assert_eq!(claim_version, 2);
        assert_eq!(targeted_repair_job_count(&db, &fresh_claim_id), 1);
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

        replay_claim_file_sidecar_corrections(&ctx, &db, "run-1", &sidecar).expect("first replay");
        let second = replay_claim_file_sidecar_corrections(&ctx, &db, "run-1", &sidecar)
            .expect("second replay");

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

        let report = replay_claim_file_sidecar_corrections(&ctx, &db, "run-1", &sidecar)
            .expect("replay legacy sidecar");

        assert_eq!(report.applied_count, 1);
        assert!(report.events[0].sidecar_event_id.starts_with("legacy-v1:"));
        assert_eq!(
            feedback_count_for_replay(&db, &report.events[0].sidecar_event_id),
            1
        );
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

        let err = replay_claim_file_sidecar_corrections(&ctx, &db, "run-1", &sidecar)
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
        replay_claim_file_sidecar_corrections(&ctx, &db, "run-1", &first_sidecar)
            .expect("first replay");

        let mut second_sidecar = first_sidecar;
        second_sidecar.claims[0].feedback_rows[0].payload_json =
            Some(serde_json::json!({ "corrected_text": "Risk is limited to the renewal window" }));
        let err = replay_claim_file_sidecar_corrections(&ctx, &db, "run-2", &second_sidecar)
            .expect_err("same replay id must not hide changed payload");

        assert!(
            matches!(err, RebuildError::InvalidSidecar(message) if message.contains("feedback content"))
        );
        assert_eq!(feedback_count_for_replay(&db, "payload-event"), 1);
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
        replay_claim_file_sidecar_corrections(&ctx, &db, "run-1", &sidecar).expect("first replay");

        mark_replay_event_terminal(
            &db,
            "applied-event",
            ReplayEventStatus::Failed.as_str(),
            Some("late_failure"),
            Some("late-failure-detail"),
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

        let report = replay_claim_file_sidecar_corrections(&ctx, &db, "run-1", &sidecar)
            .expect("replay repaired claimed row");

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
        let content_hash = feedback_content_hash(first_feedback, action, payload_json.as_deref())
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

        let err = replay_claim_file_sidecar_corrections(&ctx, &db, "run-2", &second_sidecar)
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

        let err = replay_claim_file_sidecar_corrections(&ctx, &db, "run-1", &sidecar)
            .expect_err("missing event id must reject");

        assert!(matches!(err, RebuildError::InvalidSidecar(_)));
        assert_eq!(feedback_count_for_replay(&db, ""), 0);
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

        let err = replay_claim_file_sidecar_corrections(&ctx, &db, "run-1", &sidecar)
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

        let err = replay_claim_file_sidecar_corrections(&ctx, &db, "run-1", &sidecar)
            .expect_err("duplicate event id must reject");

        assert!(matches!(err, RebuildError::InvalidSidecar(_)));
        assert_eq!(feedback_count_for_replay(&db, "dupe-feedback"), 0);
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

        let report = replay_claim_file_sidecar_corrections(&ctx, &db, "run-1", &sidecar)
            .expect("replay sidecar");

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

        let report = replay_claim_file_sidecar_corrections(&ctx, &db, "run-1", &sidecar)
            .expect("replay sidecar");

        assert_eq!(report.applied_count, 0);
        assert_eq!(report.orphaned_count, 1);
        assert_eq!(report.events[0].status, ReplayEventStatus::OrphanMissing);
        assert_eq!(
            feedback_count_for_replay(&db, "source-mismatch-feedback"),
            0
        );
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

        let report = replay_claim_file_sidecar_corrections(&ctx, &db, "run-1", &sidecar)
            .expect("replay sidecar");

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

        let report = replay_claim_file_sidecar_corrections(&ctx, &db, "run-1", &sidecar)
            .expect("replay sidecar");

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
        let first_report =
            replay_claim_file_sidecar_corrections(&ctx, &db, "run-1", &failed_sidecar)
                .expect("first replay fails terminally");
        assert_eq!(first_report.failed_count, 1);

        let mut second_proposal = proposal("Renewal risk is elevated for a different field");
        second_proposal.field_path = Some("health.other_risk".to_string());
        let second_claim_id =
            inserted_claim_id(commit_claim(&ctx, &db, second_proposal).expect("commit second"));
        let second_sidecar =
            sidecar_for_claim(&db, &second_claim_id, "other-runtime-claim-id", "event-1");

        let err = replay_claim_file_sidecar_corrections(&ctx, &db, "run-2", &second_sidecar)
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
            replay_claim_file_sidecar_corrections(&ctx, &db, "run-1", &orphan_sidecar)
                .expect("first replay orphans");
        assert_eq!(first_report.orphaned_count, 1);

        let mut second_proposal = proposal("Renewal risk is elevated for a different field");
        second_proposal.field_path = Some("health.other_risk".to_string());
        let second_claim_id =
            inserted_claim_id(commit_claim(&ctx, &db, second_proposal).expect("commit second"));
        let second_sidecar =
            sidecar_for_claim(&db, &second_claim_id, "other-runtime-claim-id", "event-2");

        let err = replay_claim_file_sidecar_corrections(&ctx, &db, "run-2", &second_sidecar)
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
        replay_claim_file_sidecar_corrections(&ctx, &db, "run-1", &first_sidecar)
            .expect("first replay");

        let second_sidecar = sidecar_for_claim(
            &db,
            &second_claim_id,
            "other-runtime-claim-id",
            "reused-feedback",
        );
        let err = replay_claim_file_sidecar_corrections(&ctx, &db, "run-2", &second_sidecar)
            .expect_err("reused terminal event id must reject target mismatch");

        assert!(
            matches!(err, RebuildError::InvalidSidecar(message) if message.contains("different target"))
        );
        assert_eq!(feedback_count_for_replay(&db, "reused-feedback"), 1);
    }
}
