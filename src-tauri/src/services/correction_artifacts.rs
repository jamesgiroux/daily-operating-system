use rusqlite::params;
use serde::Serialize;
use serde_json::json;
use sha2::Digest;
use uuid::Uuid;

use crate::db::ActionDb;
use crate::services::context::ServiceContext;

#[derive(Debug, thiserror::Error)]
pub enum CorrectionArtifactError {
    #[error("mutation rejected: {0}")]
    Mode(String),
    #[error("correction artifact lifecycle actor not allowed: {0}")]
    UnauthorizedActor(String),
    #[error("database error: {0}")]
    Db(String),
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

#[derive(Debug, Clone)]
pub struct RedactFeedbackArtifactsInput<'a> {
    pub feedback_id: &'a str,
    pub reason_code: &'a str,
}

#[derive(Debug, Clone)]
pub struct RemoveSourceArtifactsInput<'a> {
    pub source_key_hash: &'a str,
    pub reason_code: &'a str,
}

#[derive(Debug, Clone)]
pub struct RemoveMeetingArtifactsInput<'a> {
    pub meeting_stable_key: &'a str,
    pub meeting_id: Option<&'a str>,
    pub reason_code: &'a str,
}

#[derive(Debug, Clone)]
pub struct PurgeDos338ProofInput<'a> {
    pub run_id: &'a str,
    pub reason_code: &'a str,
}

#[derive(Debug, Clone)]
pub struct WorkspaceResetArtifactsInput<'a> {
    pub reason_code: &'a str,
}

#[derive(Debug, Clone)]
pub struct SubjectLifecycleArtifactsInput<'a> {
    pub subject_kind: &'a str,
    pub subject_id: &'a str,
    pub reason_code: &'a str,
    pub action: SubjectLifecycleAction<'a>,
}

#[derive(Debug, Clone)]
pub(crate) struct RebindSubjectArtifactsInput<'a> {
    pub subject_kind: &'a str,
    pub subject_id: &'a str,
    pub new_subject_kind: &'a str,
    pub new_subject_id: &'a str,
    pub reason_code: &'a str,
}

#[derive(Debug, Clone, Copy)]
pub enum SubjectLifecycleAction<'a> {
    Deleted,
    Orphaned,
    Rebound {
        new_subject_kind: &'a str,
        new_subject_id: &'a str,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct CorrectionArtifactLifecycleReport {
    pub feedback_payloads_redacted: usize,
    pub envelopes_redacted: usize,
    pub source_deltas_redacted: usize,
    pub source_aggregates_excluded: usize,
    pub subject_deltas_redacted: usize,
    pub subject_aggregates_excluded: usize,
    pub propagation_jobs_staled: usize,
    pub propagation_outcomes_recorded: usize,
    pub prep_journals_redacted: usize,
    pub declassification_decisions_revoked: usize,
    pub dos338_observations_marked: usize,
    pub lifecycle_events_recorded: usize,
    pub dos338_runs_purged: usize,
    pub workspace_rows_purged: usize,
}

pub fn redact_feedback_artifacts(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    input: RedactFeedbackArtifactsInput<'_>,
) -> Result<CorrectionArtifactLifecycleReport, CorrectionArtifactError> {
    ctx.check_mutation_allowed()
        .map_err(|error| CorrectionArtifactError::Mode(error.to_string()))?;
    authorize_lifecycle_actor(ctx.actor)?;
    validate_required(input.feedback_id, "feedback_id")?;
    validate_required(input.reason_code, "reason_code")?;
    let now = ctx.clock.now().to_rfc3339();
    db.with_transaction(|tx| {
        redact_feedback_artifacts_in_tx(tx, input.feedback_id, input.reason_code, ctx.actor, &now)
            .map_err(|error| error.to_string())
    })
    .map_err(CorrectionArtifactError::Db)
}

pub fn remove_source_artifacts(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    input: RemoveSourceArtifactsInput<'_>,
) -> Result<CorrectionArtifactLifecycleReport, CorrectionArtifactError> {
    ctx.check_mutation_allowed()
        .map_err(|error| CorrectionArtifactError::Mode(error.to_string()))?;
    authorize_lifecycle_actor(ctx.actor)?;
    validate_required(input.source_key_hash, "source_key_hash")?;
    validate_required(input.reason_code, "reason_code")?;
    let now = ctx.clock.now().to_rfc3339();
    db.with_transaction(|tx| {
        remove_source_artifacts_in_tx(
            tx,
            input.source_key_hash,
            input.reason_code,
            ctx.actor,
            &now,
        )
        .map_err(|error| error.to_string())
    })
    .map_err(CorrectionArtifactError::Db)
}

pub fn update_subject_lifecycle_artifacts(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    input: SubjectLifecycleArtifactsInput<'_>,
) -> Result<CorrectionArtifactLifecycleReport, CorrectionArtifactError> {
    ctx.check_mutation_allowed()
        .map_err(|error| CorrectionArtifactError::Mode(error.to_string()))?;
    authorize_lifecycle_actor(ctx.actor)?;
    validate_required(input.subject_kind, "subject_kind")?;
    validate_required(input.subject_id, "subject_id")?;
    validate_required(input.reason_code, "reason_code")?;
    if let SubjectLifecycleAction::Rebound {
        new_subject_kind,
        new_subject_id,
    } = input.action
    {
        validate_required(new_subject_kind, "new_subject_kind")?;
        validate_required(new_subject_id, "new_subject_id")?;
    }

    let now = ctx.clock.now().to_rfc3339();
    db.with_transaction(|tx| {
        update_subject_lifecycle_artifacts_in_tx(tx, input, ctx.actor, &now)
            .map_err(|error| error.to_string())
    })
    .map_err(CorrectionArtifactError::Db)
}

pub fn remove_meeting_artifacts(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    input: RemoveMeetingArtifactsInput<'_>,
) -> Result<CorrectionArtifactLifecycleReport, CorrectionArtifactError> {
    ctx.check_mutation_allowed()
        .map_err(|error| CorrectionArtifactError::Mode(error.to_string()))?;
    authorize_lifecycle_actor(ctx.actor)?;
    validate_required(input.meeting_stable_key, "meeting_stable_key")?;
    validate_required(input.reason_code, "reason_code")?;
    let now = ctx.clock.now().to_rfc3339();
    db.with_transaction(|tx| {
        remove_meeting_artifacts_in_tx(tx, input, ctx.actor, &now)
            .map_err(|error| error.to_string())
    })
    .map_err(CorrectionArtifactError::Db)
}

pub fn purge_dos338_proof_artifacts(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    input: PurgeDos338ProofInput<'_>,
) -> Result<CorrectionArtifactLifecycleReport, CorrectionArtifactError> {
    ctx.check_mutation_allowed()
        .map_err(|error| CorrectionArtifactError::Mode(error.to_string()))?;
    authorize_lifecycle_actor(ctx.actor)?;
    validate_required(input.run_id, "run_id")?;
    validate_required(input.reason_code, "reason_code")?;
    let now = ctx.clock.now().to_rfc3339();
    db.with_transaction(|tx| {
        purge_dos338_proof_artifacts_in_tx(tx, input.run_id, input.reason_code, ctx.actor, &now)
            .map_err(|error| error.to_string())
    })
    .map_err(CorrectionArtifactError::Db)
}

pub fn purge_workspace_correction_artifacts(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    input: WorkspaceResetArtifactsInput<'_>,
) -> Result<CorrectionArtifactLifecycleReport, CorrectionArtifactError> {
    ctx.check_mutation_allowed()
        .map_err(|error| CorrectionArtifactError::Mode(error.to_string()))?;
    authorize_lifecycle_actor(ctx.actor)?;
    validate_required(input.reason_code, "reason_code")?;
    let now = ctx.clock.now().to_rfc3339();
    db.with_transaction(|tx| {
        purge_workspace_correction_artifacts_in_tx(tx, input.reason_code, ctx.actor, &now)
            .map_err(|error| error.to_string())
    })
    .map_err(CorrectionArtifactError::Db)
}

pub(crate) fn authorize_lifecycle_actor(actor: &str) -> Result<(), CorrectionArtifactError> {
    let normalized = actor.trim().to_ascii_lowercase();
    let allowed = normalized == "user" || normalized.starts_with("user:");
    let rejected_prefix = ["agent", "system", "mcp", "surface_client", "runtime"]
        .iter()
        .any(|prefix| normalized.starts_with(prefix));
    if allowed && !rejected_prefix {
        Ok(())
    } else {
        Err(CorrectionArtifactError::UnauthorizedActor(
            actor.to_string(),
        ))
    }
}

pub(crate) fn authorize_service_lifecycle_actor(
    actor: &str,
) -> Result<(), CorrectionArtifactError> {
    if authorize_lifecycle_actor(actor).is_ok() {
        return Ok(());
    }
    let normalized = actor.trim().to_ascii_lowercase();
    let allowed_system = normalized == "system" || normalized.starts_with("system:");
    let rejected_prefix = ["agent", "mcp", "surface_client", "runtime"]
        .iter()
        .any(|prefix| normalized.starts_with(prefix));
    if allowed_system && !rejected_prefix {
        Ok(())
    } else {
        Err(CorrectionArtifactError::UnauthorizedActor(
            actor.to_string(),
        ))
    }
}

fn validate_required(value: &str, label: &str) -> Result<(), CorrectionArtifactError> {
    if value.trim().is_empty() {
        Err(CorrectionArtifactError::Db(format!(
            "{label} cannot be empty"
        )))
    } else {
        Ok(())
    }
}

fn redact_feedback_artifacts_in_tx(
    tx: &ActionDb,
    feedback_id: &str,
    reason_code: &str,
    actor: &str,
    now: &str,
) -> Result<CorrectionArtifactLifecycleReport, CorrectionArtifactError> {
    let mut report = CorrectionArtifactLifecycleReport::default();
    let redacted_payload = json!({
        "redacted": true,
        "reason_code": reason_code,
    })
    .to_string();

    report.feedback_payloads_redacted = tx
        .conn_ref()
        .execute(
            "UPDATE claim_feedback
                SET payload_json = ?2
              WHERE id = ?1
                AND payload_json IS NOT NULL",
            params![feedback_id, &redacted_payload],
        )
        .map_err(|error| CorrectionArtifactError::Db(error.to_string()))?;

    report.envelopes_redacted = tx
        .conn_ref()
        .execute(
            "UPDATE claim_feedback_correction_envelopes
                SET target_receipt_json = NULL,
                    corrected_subject_ref_json = NULL,
                    source_ref = NULL,
                    action_metadata_json = ?2,
                    lifecycle_state = 'redacted',
                    lifecycle_reason_code = ?3,
                    redacted_at = ?4,
                    updated_at = ?4
              WHERE feedback_id = ?1
                AND lifecycle_state != 'redacted'",
            params![feedback_id, &redacted_payload, reason_code, now],
        )
        .map_err(|error| CorrectionArtifactError::Db(error.to_string()))?;

    report.source_deltas_redacted = tx
        .conn_ref()
        .execute(
            "UPDATE source_reliability_feedback_deltas
                SET status = 'redacted'
              WHERE feedback_id = ?1
                AND status != 'redacted'",
            params![feedback_id],
        )
        .map_err(|error| CorrectionArtifactError::Db(error.to_string()))?;

    report.source_aggregates_excluded = tx
        .conn_ref()
        .execute(
            "UPDATE source_claim_type_reliability
                SET alpha = 1.0 + COALESCE((
                        SELECT SUM(delta.alpha_delta)
                          FROM source_reliability_feedback_deltas delta
                         WHERE delta.source_key_version = source_claim_type_reliability.source_key_version
                           AND delta.source_key_epoch_hash = source_claim_type_reliability.source_key_epoch_hash
                           AND delta.source_key_hash = source_claim_type_reliability.source_key_hash
                           AND delta.claim_type = source_claim_type_reliability.claim_type
                           AND delta.signal_type = source_claim_type_reliability.signal_type
                           AND delta.status = 'applied'
                    ), 0.0),
                    beta = 1.0 + COALESCE((
                        SELECT SUM(delta.beta_delta)
                          FROM source_reliability_feedback_deltas delta
                         WHERE delta.source_key_version = source_claim_type_reliability.source_key_version
                           AND delta.source_key_epoch_hash = source_claim_type_reliability.source_key_epoch_hash
                           AND delta.source_key_hash = source_claim_type_reliability.source_key_hash
                           AND delta.claim_type = source_claim_type_reliability.claim_type
                           AND delta.signal_type = source_claim_type_reliability.signal_type
                           AND delta.status = 'applied'
                    ), 0.0),
                    update_count = (
                        SELECT COUNT(*)
                          FROM source_reliability_feedback_deltas delta
                         WHERE delta.source_key_version = source_claim_type_reliability.source_key_version
                           AND delta.source_key_epoch_hash = source_claim_type_reliability.source_key_epoch_hash
                           AND delta.source_key_hash = source_claim_type_reliability.source_key_hash
                           AND delta.claim_type = source_claim_type_reliability.claim_type
                           AND delta.signal_type = source_claim_type_reliability.signal_type
                           AND delta.status = 'applied'
                    ),
                    excluded_at = CASE
                        WHEN (
                            SELECT COUNT(*)
                              FROM source_reliability_feedback_deltas delta
                             WHERE delta.source_key_version = source_claim_type_reliability.source_key_version
                               AND delta.source_key_epoch_hash = source_claim_type_reliability.source_key_epoch_hash
                               AND delta.source_key_hash = source_claim_type_reliability.source_key_hash
                               AND delta.claim_type = source_claim_type_reliability.claim_type
                               AND delta.signal_type = source_claim_type_reliability.signal_type
                               AND delta.status = 'applied'
                        ) = 0 THEN COALESCE(excluded_at, ?2)
                        ELSE NULL
                    END,
                    updated_at = ?2
              WHERE EXISTS (
                    SELECT 1
                      FROM source_reliability_feedback_deltas redacted
                     WHERE redacted.feedback_id = ?1
                       AND redacted.source_key_version = source_claim_type_reliability.source_key_version
                       AND redacted.source_key_epoch_hash = source_claim_type_reliability.source_key_epoch_hash
                       AND redacted.source_key_hash = source_claim_type_reliability.source_key_hash
                       AND redacted.claim_type = source_claim_type_reliability.claim_type
                       AND redacted.signal_type = source_claim_type_reliability.signal_type
              )",
            params![feedback_id, now],
        )
        .map_err(|error| CorrectionArtifactError::Db(error.to_string()))?;

    report.subject_deltas_redacted = tx
        .conn_ref()
        .execute(
            "UPDATE subject_inference_reliability_deltas
                SET status = 'redacted'
              WHERE feedback_id = ?1
                AND status != 'redacted'",
            params![feedback_id],
        )
        .map_err(|error| CorrectionArtifactError::Db(error.to_string()))?;

    report.subject_aggregates_excluded = tx
        .conn_ref()
        .execute(
            "UPDATE subject_inference_reliability
                SET alpha = 1.0 + COALESCE((
                        SELECT SUM(delta.alpha_delta)
                          FROM subject_inference_reliability_deltas delta
                         WHERE delta.subject_ref_hash = subject_inference_reliability.subject_ref_hash
                           AND delta.claim_type = subject_inference_reliability.claim_type
                           AND delta.signal_type = subject_inference_reliability.signal_type
                           AND delta.status = 'applied'
                    ), 0.0),
                    beta = 1.0 + COALESCE((
                        SELECT SUM(delta.beta_delta)
                          FROM subject_inference_reliability_deltas delta
                         WHERE delta.subject_ref_hash = subject_inference_reliability.subject_ref_hash
                           AND delta.claim_type = subject_inference_reliability.claim_type
                           AND delta.signal_type = subject_inference_reliability.signal_type
                           AND delta.status = 'applied'
                    ), 0.0),
                    update_count = (
                        SELECT COUNT(*)
                          FROM subject_inference_reliability_deltas delta
                         WHERE delta.subject_ref_hash = subject_inference_reliability.subject_ref_hash
                           AND delta.claim_type = subject_inference_reliability.claim_type
                           AND delta.signal_type = subject_inference_reliability.signal_type
                           AND delta.status = 'applied'
                    ),
                    lifecycle_state = CASE
                        WHEN (
                            SELECT COUNT(*)
                              FROM subject_inference_reliability_deltas delta
                             WHERE delta.subject_ref_hash = subject_inference_reliability.subject_ref_hash
                               AND delta.claim_type = subject_inference_reliability.claim_type
                               AND delta.signal_type = subject_inference_reliability.signal_type
                               AND delta.status = 'applied'
                        ) = 0 THEN 'redacted'
                        ELSE 'active'
                    END,
                    excluded_at = CASE
                        WHEN (
                            SELECT COUNT(*)
                              FROM subject_inference_reliability_deltas delta
                             WHERE delta.subject_ref_hash = subject_inference_reliability.subject_ref_hash
                               AND delta.claim_type = subject_inference_reliability.claim_type
                               AND delta.signal_type = subject_inference_reliability.signal_type
                               AND delta.status = 'applied'
                        ) = 0 THEN COALESCE(excluded_at, ?2)
                        ELSE NULL
                    END,
                    updated_at = ?2
              WHERE EXISTS (
                    SELECT 1
                      FROM subject_inference_reliability_deltas redacted
                     WHERE redacted.feedback_id = ?1
                       AND redacted.subject_ref_hash = subject_inference_reliability.subject_ref_hash
                       AND redacted.claim_type = subject_inference_reliability.claim_type
                       AND redacted.signal_type = subject_inference_reliability.signal_type
              )",
            params![feedback_id, now],
        )
        .map_err(|error| CorrectionArtifactError::Db(error.to_string()))?;

    tx.conn_ref()
        .execute(
            "UPDATE claim_feedback_propagation_jobs
                SET scope_json = ?2,
                    cursor_json = NULL,
                    coalescing_key = 'redacted:' || id,
                    updated_at = ?3
              WHERE feedback_id = ?1",
            params![feedback_id, &redacted_payload, now],
        )
        .map_err(|error| CorrectionArtifactError::Db(error.to_string()))?;

    report.propagation_jobs_staled = tx
        .conn_ref()
        .execute(
            "UPDATE claim_feedback_propagation_jobs
                SET status = 'stale',
                    stale_reason = ?2,
                    updated_at = ?3
              WHERE feedback_id = ?1
                AND status IN ('pending', 'running', 'coalesced')",
            params![feedback_id, reason_code, now],
        )
        .map_err(|error| CorrectionArtifactError::Db(error.to_string()))?;
    report.propagation_outcomes_recorded =
        record_redaction_propagation_outcomes(tx, feedback_id, reason_code, now)?;

    report.prep_journals_redacted = tx
        .conn_ref()
        .execute(
            "UPDATE meeting_prep_correction_journal
                SET payload_json = ?2,
                    payload_hash = ?3,
                    lifecycle_state = 'redacted',
                    redacted_at = ?4,
                    redaction_reason_code = ?5,
                    updated_at = ?4
              WHERE feedback_id = ?1
                AND lifecycle_state != 'redacted'",
            params![
                feedback_id,
                &redacted_payload,
                stable_json_hash(&redacted_payload),
                now,
                reason_code
            ],
        )
        .map_err(|error| CorrectionArtifactError::Db(error.to_string()))?;

    report.declassification_decisions_revoked = tx
        .conn_ref()
        .execute(
            "UPDATE correction_artifact_declassification_decisions
                SET status = 'revoked',
                    revoked_reason_code = ?2,
                    revoked_at = ?3
              WHERE artifact_id = ?1
                AND status = 'active'",
            params![feedback_id, reason_code, now],
        )
        .map_err(|error| CorrectionArtifactError::Db(error.to_string()))?;

    report.dos338_observations_marked = tx
        .conn_ref()
        .execute(
            "UPDATE dos338_stickiness_observations
                SET reason_code = COALESCE(reason_code, ?2),
                    dead_letter_reason = COALESCE(dead_letter_reason, ?2)
              WHERE feedback_id = ?1",
            params![feedback_id, reason_code],
        )
        .map_err(|error| CorrectionArtifactError::Db(error.to_string()))?;

    report.lifecycle_events_recorded = insert_lifecycle_event(
        tx,
        "claim_feedback",
        feedback_id,
        "redacted",
        actor,
        reason_code,
        now,
    )?;
    Ok(report)
}

fn remove_source_artifacts_in_tx(
    tx: &ActionDb,
    source_key_hash: &str,
    reason_code: &str,
    actor: &str,
    now: &str,
) -> Result<CorrectionArtifactLifecycleReport, CorrectionArtifactError> {
    let mut report = CorrectionArtifactLifecycleReport::default();
    let marker = lifecycle_marker("source_removed", reason_code);
    let feedback_ids = feedback_ids_for_source_key(tx, source_key_hash)?;

    report.feedback_payloads_redacted = tx
        .conn_ref()
        .execute(
            "UPDATE claim_feedback
                SET payload_json = ?2
              WHERE id IN (
                    SELECT feedback_id
                      FROM claim_feedback_correction_envelopes
                     WHERE source_key_hash = ?1
                    UNION
                    SELECT feedback_id
                      FROM source_reliability_feedback_deltas
                     WHERE source_key_hash = ?1
                )
                AND payload_json IS NOT NULL",
            params![source_key_hash, &marker],
        )
        .map_err(|error| CorrectionArtifactError::Db(error.to_string()))?;

    report.envelopes_redacted = tx
        .conn_ref()
        .execute(
            "UPDATE claim_feedback_correction_envelopes
                SET target_receipt_json = NULL,
                    corrected_subject_ref_json = NULL,
                    source_ref = NULL,
                    source_ref_hash = NULL,
                    action_metadata_json = ?2,
                    lifecycle_state = 'source_removed',
                    lifecycle_reason_code = ?3,
                    source_removed_at = ?4,
                    updated_at = ?4
              WHERE source_key_hash = ?1
                AND lifecycle_state != 'source_removed'",
            params![source_key_hash, &marker, reason_code, now],
        )
        .map_err(|error| CorrectionArtifactError::Db(error.to_string()))?;

    report.source_deltas_redacted = tx
        .conn_ref()
        .execute(
            "UPDATE source_reliability_feedback_deltas
                SET status = 'source_removed'
              WHERE source_key_hash = ?1
                AND status != 'source_removed'",
            params![source_key_hash],
        )
        .map_err(|error| CorrectionArtifactError::Db(error.to_string()))?;

    report.source_aggregates_excluded = tx
        .conn_ref()
        .execute(
            "UPDATE source_claim_type_reliability
                SET excluded_at = COALESCE(excluded_at, ?2),
                    updated_at = ?2
              WHERE source_key_hash = ?1
                AND excluded_at IS NULL",
            params![source_key_hash, now],
        )
        .map_err(|error| CorrectionArtifactError::Db(error.to_string()))?;

    report.propagation_jobs_staled =
        stale_propagation_for_feedback_ids(tx, &feedback_ids, &marker, reason_code, now)?;
    report.propagation_outcomes_recorded =
        record_stale_outcomes_for_feedback_ids(tx, &feedback_ids, reason_code, now)?;

    report.declassification_decisions_revoked = tx
        .conn_ref()
        .execute(
            "UPDATE correction_artifact_declassification_decisions
                SET status = 'revoked',
                    revoked_reason_code = ?2,
                    revoked_at = ?3,
                    parent_lifecycle_state = 'source_removed'
              WHERE status = 'active'
                AND (
                    source_artifact_hash = ?1
                    OR artifact_id IN (
                        SELECT feedback_id
                          FROM claim_feedback_correction_envelopes
                         WHERE source_key_hash = ?1
                        UNION
                        SELECT feedback_id
                          FROM source_reliability_feedback_deltas
                         WHERE source_key_hash = ?1
                    )
                )",
            params![source_key_hash, reason_code, now],
        )
        .map_err(|error| CorrectionArtifactError::Db(error.to_string()))?;

    report.dos338_observations_marked = tx
        .conn_ref()
        .execute(
            "UPDATE dos338_stickiness_observations
                SET reason_code = COALESCE(reason_code, ?2),
                    dead_letter_reason = COALESCE(dead_letter_reason, ?2)
              WHERE feedback_id IN (
                    SELECT feedback_id
                      FROM claim_feedback_correction_envelopes
                     WHERE source_key_hash = ?1
                    UNION
                    SELECT feedback_id
                      FROM source_reliability_feedback_deltas
                     WHERE source_key_hash = ?1
                )",
            params![source_key_hash, reason_code],
        )
        .map_err(|error| CorrectionArtifactError::Db(error.to_string()))?;

    report.lifecycle_events_recorded = insert_lifecycle_event(
        tx,
        "source_reliability_key",
        source_key_hash,
        "source_removed",
        actor,
        reason_code,
        now,
    )?;
    Ok(report)
}

pub(crate) fn remove_data_source_artifacts_for_source_purge_in_tx(
    tx: &ActionDb,
    data_source: &str,
    reason_code: &str,
    actor: &str,
    now: &str,
) -> Result<CorrectionArtifactLifecycleReport, CorrectionArtifactError> {
    authorize_service_lifecycle_actor(actor)?;
    validate_required(data_source, "data_source")?;
    validate_required(reason_code, "reason_code")?;
    let source_keys = source_keys_for_data_source(tx, data_source)?;
    let mut report = CorrectionArtifactLifecycleReport::default();
    for source_key_hash in source_keys {
        let source_report =
            remove_source_artifacts_in_tx(tx, &source_key_hash, reason_code, actor, now)?;
        report.feedback_payloads_redacted += source_report.feedback_payloads_redacted;
        report.envelopes_redacted += source_report.envelopes_redacted;
        report.source_deltas_redacted += source_report.source_deltas_redacted;
        report.source_aggregates_excluded += source_report.source_aggregates_excluded;
        report.propagation_jobs_staled += source_report.propagation_jobs_staled;
        report.propagation_outcomes_recorded += source_report.propagation_outcomes_recorded;
        report.declassification_decisions_revoked +=
            source_report.declassification_decisions_revoked;
        report.dos338_observations_marked += source_report.dos338_observations_marked;
        report.lifecycle_events_recorded += source_report.lifecycle_events_recorded;
    }
    Ok(report)
}

pub(crate) fn delete_subject_artifacts_in_tx(
    tx: &ActionDb,
    subject_kind: &str,
    subject_id: &str,
    reason_code: &str,
    actor: &str,
    now: &str,
) -> Result<CorrectionArtifactLifecycleReport, CorrectionArtifactError> {
    authorize_service_lifecycle_actor(actor)?;
    update_subject_lifecycle_artifacts_in_tx(
        tx,
        SubjectLifecycleArtifactsInput {
            subject_kind,
            subject_id,
            reason_code,
            action: SubjectLifecycleAction::Deleted,
        },
        actor,
        now,
    )
}

pub(crate) fn rebind_subject_artifacts_in_tx(
    tx: &ActionDb,
    input: RebindSubjectArtifactsInput<'_>,
    actor: &str,
    now: &str,
) -> Result<CorrectionArtifactLifecycleReport, CorrectionArtifactError> {
    authorize_service_lifecycle_actor(actor)?;
    update_subject_lifecycle_artifacts_in_tx(
        tx,
        SubjectLifecycleArtifactsInput {
            subject_kind: input.subject_kind,
            subject_id: input.subject_id,
            reason_code: input.reason_code,
            action: SubjectLifecycleAction::Rebound {
                new_subject_kind: input.new_subject_kind,
                new_subject_id: input.new_subject_id,
            },
        },
        actor,
        now,
    )
}

pub(crate) fn remove_meeting_artifacts_for_meeting_id_in_tx(
    tx: &ActionDb,
    meeting_id: &str,
    reason_code: &str,
    actor: &str,
    now: &str,
) -> Result<CorrectionArtifactLifecycleReport, CorrectionArtifactError> {
    authorize_service_lifecycle_actor(actor)?;
    validate_required(meeting_id, "meeting_id")?;
    validate_required(reason_code, "reason_code")?;
    let mut stable_keys = Vec::new();
    let mut stmt = tx
        .conn_ref()
        .prepare(
            "SELECT DISTINCT meeting_stable_key
               FROM meeting_prep_correction_journal
              WHERE meeting_id = ?1",
        )
        .map_err(|error| CorrectionArtifactError::Db(error.to_string()))?;
    let rows = stmt
        .query_map(params![meeting_id], |row| row.get::<_, String>(0))
        .map_err(|error| CorrectionArtifactError::Db(error.to_string()))?;
    for row in rows {
        stable_keys.push(row.map_err(|error| CorrectionArtifactError::Db(error.to_string()))?);
    }

    let mut report = CorrectionArtifactLifecycleReport::default();
    for stable_key in stable_keys {
        let meeting_report = remove_meeting_artifacts_in_tx(
            tx,
            RemoveMeetingArtifactsInput {
                meeting_stable_key: &stable_key,
                meeting_id: Some(meeting_id),
                reason_code,
            },
            actor,
            now,
        )?;
        report.feedback_payloads_redacted += meeting_report.feedback_payloads_redacted;
        report.envelopes_redacted += meeting_report.envelopes_redacted;
        report.subject_deltas_redacted += meeting_report.subject_deltas_redacted;
        report.subject_aggregates_excluded += meeting_report.subject_aggregates_excluded;
        report.propagation_jobs_staled += meeting_report.propagation_jobs_staled;
        report.propagation_outcomes_recorded += meeting_report.propagation_outcomes_recorded;
        report.prep_journals_redacted += meeting_report.prep_journals_redacted;
        report.declassification_decisions_revoked +=
            meeting_report.declassification_decisions_revoked;
        report.dos338_observations_marked += meeting_report.dos338_observations_marked;
        report.lifecycle_events_recorded += meeting_report.lifecycle_events_recorded;
    }

    if report.lifecycle_events_recorded == 0 {
        let subject_report = update_subject_lifecycle_artifacts_in_tx(
            tx,
            SubjectLifecycleArtifactsInput {
                subject_kind: "meeting",
                subject_id: meeting_id,
                reason_code,
                action: SubjectLifecycleAction::Orphaned,
            },
            actor,
            now,
        )?;
        report.feedback_payloads_redacted += subject_report.feedback_payloads_redacted;
        report.envelopes_redacted += subject_report.envelopes_redacted;
        report.subject_deltas_redacted += subject_report.subject_deltas_redacted;
        report.subject_aggregates_excluded += subject_report.subject_aggregates_excluded;
        report.propagation_outcomes_recorded += subject_report.propagation_outcomes_recorded;
        report.declassification_decisions_revoked +=
            subject_report.declassification_decisions_revoked;
        report.dos338_observations_marked += subject_report.dos338_observations_marked;
    }

    Ok(report)
}

fn update_subject_lifecycle_artifacts_in_tx(
    tx: &ActionDb,
    input: SubjectLifecycleArtifactsInput<'_>,
    actor: &str,
    now: &str,
) -> Result<CorrectionArtifactLifecycleReport, CorrectionArtifactError> {
    let mut report = CorrectionArtifactLifecycleReport::default();
    let subject_kind = normalized_subject_kind(input.subject_kind);
    let subject_json = canonical_subject_json(&subject_kind, input.subject_id);
    let subject_hash = subject_hash_for_json(&subject_json)?;
    let feedback_ids = feedback_ids_for_subject(tx, &subject_kind, input.subject_id)?;
    let mut subject_hashes = subject_hashes_for_feedback_ids(tx, &feedback_ids)?;
    if !subject_hashes.iter().any(|hash| hash == &subject_hash) {
        subject_hashes.push(subject_hash.clone());
    }

    match input.action {
        SubjectLifecycleAction::Deleted | SubjectLifecycleAction::Orphaned => {
            let (lifecycle_state, delta_status, event_type) = match input.action {
                SubjectLifecycleAction::Deleted => {
                    ("subject_deleted", "subject_deleted", "subject_deleted")
                }
                SubjectLifecycleAction::Orphaned => {
                    ("subject_orphaned", "subject_orphaned", "subject_deleted")
                }
                SubjectLifecycleAction::Rebound { .. } => unreachable!(),
            };
            let marker = lifecycle_marker(lifecycle_state, input.reason_code);
            report.envelopes_redacted = tx
                .conn_ref()
                .execute(
                    "UPDATE claim_feedback_correction_envelopes
                        SET asserted_subject_ref_json = CASE
                                WHEN asserted_subject_kind = ?1 AND asserted_subject_id = ?2 THEN ?3
                                ELSE asserted_subject_ref_json
                            END,
                            asserted_subject_kind = CASE
                                WHEN asserted_subject_kind = ?1 AND asserted_subject_id = ?2 THEN ?4
                                ELSE asserted_subject_kind
                            END,
                            asserted_subject_id = CASE
                                WHEN asserted_subject_kind = ?1 AND asserted_subject_id = ?2 THEN NULL
                                ELSE asserted_subject_id
                            END,
                            corrected_subject_ref_json = CASE
                                WHEN corrected_subject_ref_json = ?8
                                  OR (
                                      json_valid(corrected_subject_ref_json) = 1
                                      AND json_extract(corrected_subject_ref_json, '$.kind') = ?1
                                      AND json_extract(corrected_subject_ref_json, '$.id') = ?2
                                  )
                                  OR (
                                      json_valid(action_metadata_json) = 1
                                      AND (
                                          (
                                              json_extract(action_metadata_json, '$.corrected_subject_ref.kind') = ?1
                                              AND json_extract(action_metadata_json, '$.corrected_subject_ref.id') = ?2
                                          )
                                          OR (
                                              json_extract(action_metadata_json, '$.corrected_subject.kind') = ?1
                                              AND json_extract(action_metadata_json, '$.corrected_subject.id') = ?2
                                          )
                                      )
                                  )
                                THEN NULL
                                ELSE corrected_subject_ref_json
                            END,
                            action_metadata_json = ?3,
                            lifecycle_state = ?5,
                            lifecycle_reason_code = ?6,
                            subject_orphaned_at = ?7,
                            updated_at = ?7
                      WHERE (
                            (
                                asserted_subject_kind = ?1
                                AND asserted_subject_id = ?2
                            )
                            OR corrected_subject_ref_json = ?8
                            OR (
                                json_valid(corrected_subject_ref_json) = 1
                                AND json_extract(corrected_subject_ref_json, '$.kind') = ?1
                                AND json_extract(corrected_subject_ref_json, '$.id') = ?2
                            )
                            OR (
                                json_valid(action_metadata_json) = 1
                                AND (
                                    (
                                        json_extract(action_metadata_json, '$.corrected_subject_ref.kind') = ?1
                                        AND json_extract(action_metadata_json, '$.corrected_subject_ref.id') = ?2
                                    )
                                    OR (
                                        json_extract(action_metadata_json, '$.corrected_subject.kind') = ?1
                                        AND json_extract(action_metadata_json, '$.corrected_subject.id') = ?2
                                    )
                                )
                            )
                         )
                        AND lifecycle_state != ?5",
                    params![
                        &subject_kind,
                        input.subject_id,
                        &marker,
                        lifecycle_state,
                        lifecycle_state,
                        input.reason_code,
                        now,
                        &subject_json
                    ],
                )
                .map_err(|error| CorrectionArtifactError::Db(error.to_string()))?;
            for hash in &subject_hashes {
                report.subject_deltas_redacted += tx
                    .conn_ref()
                    .execute(
                        "UPDATE subject_inference_reliability_deltas
	                            SET status = ?2,
	                                corrected_subject_ref_hash = NULL
	                          WHERE (subject_ref_hash = ?1 OR corrected_subject_ref_hash = ?1)
	                            AND status != ?2",
                        params![hash, delta_status],
                    )
                    .map_err(|error| CorrectionArtifactError::Db(error.to_string()))?;
                report.subject_aggregates_excluded += tx
                    .conn_ref()
                    .execute(
                        "UPDATE subject_inference_reliability
                            SET lifecycle_state = ?2,
                                excluded_at = COALESCE(excluded_at, ?3),
                                updated_at = ?3
                          WHERE subject_ref_hash = ?1
                            AND lifecycle_state != ?2",
                        params![hash, lifecycle_state, now],
                    )
                    .map_err(|error| CorrectionArtifactError::Db(error.to_string()))?;
            }
            report.propagation_jobs_staled = stale_propagation_for_feedback_ids(
                tx,
                &feedback_ids,
                &marker,
                input.reason_code,
                now,
            )?;
            report.propagation_outcomes_recorded =
                record_stale_outcomes_for_feedback_ids(tx, &feedback_ids, input.reason_code, now)?;
            report.declassification_decisions_revoked = revoke_subject_declassifications(
                tx,
                &feedback_ids,
                &subject_hashes,
                lifecycle_state,
                input.reason_code,
                now,
            )?;
            report.dos338_observations_marked =
                mark_dos338_for_subject(tx, &feedback_ids, &subject_hashes, input.reason_code)?;
            report.lifecycle_events_recorded = insert_lifecycle_event(
                tx,
                "subject_ref",
                &subject_hash,
                event_type,
                actor,
                input.reason_code,
                now,
            )?;
        }
        SubjectLifecycleAction::Rebound {
            new_subject_kind,
            new_subject_id,
        } => {
            let new_kind = normalized_subject_kind(new_subject_kind);
            let new_json = canonical_subject_json(&new_kind, new_subject_id);
            let new_hash = subject_hash_for_json(&new_json)?;
            let marker = lifecycle_marker("subject_rebound", input.reason_code);
            report.envelopes_redacted = tx
                .conn_ref()
                .execute(
                    "UPDATE claim_feedback_correction_envelopes
                        SET asserted_subject_ref_json = CASE
                                WHEN asserted_subject_kind = ?1 AND asserted_subject_id = ?2 THEN ?3
                                ELSE asserted_subject_ref_json
                            END,
                            asserted_subject_kind = CASE
                                WHEN asserted_subject_kind = ?1 AND asserted_subject_id = ?2 THEN ?4
                                ELSE asserted_subject_kind
                            END,
                            asserted_subject_id = CASE
                                WHEN asserted_subject_kind = ?1 AND asserted_subject_id = ?2 THEN ?5
                                ELSE asserted_subject_id
                            END,
                            corrected_subject_ref_json = CASE
                                WHEN corrected_subject_ref_json = ?8
                                  OR (
                                      json_valid(corrected_subject_ref_json) = 1
                                      AND json_extract(corrected_subject_ref_json, '$.kind') = ?1
                                      AND json_extract(corrected_subject_ref_json, '$.id') = ?2
                                  )
                                  OR (
                                      json_valid(action_metadata_json) = 1
                                      AND (
                                          (
                                              json_extract(action_metadata_json, '$.corrected_subject_ref.kind') = ?1
                                              AND json_extract(action_metadata_json, '$.corrected_subject_ref.id') = ?2
                                          )
                                          OR (
                                              json_extract(action_metadata_json, '$.corrected_subject.kind') = ?1
                                              AND json_extract(action_metadata_json, '$.corrected_subject.id') = ?2
                                          )
                                      )
                                  )
                                THEN ?3
                                ELSE corrected_subject_ref_json
                            END,
                            action_metadata_json = ?9,
                            parent_lifecycle_state = 'subject_rebound',
                            lifecycle_reason_code = ?6,
                            updated_at = ?7
                      WHERE (
                            asserted_subject_kind = ?1
                            AND asserted_subject_id = ?2
                         )
                         OR corrected_subject_ref_json = ?8
                         OR (
                            json_valid(corrected_subject_ref_json) = 1
                            AND json_extract(corrected_subject_ref_json, '$.kind') = ?1
                            AND json_extract(corrected_subject_ref_json, '$.id') = ?2
                         )
                         OR (
                            json_valid(action_metadata_json) = 1
                            AND (
                                (
                                    json_extract(action_metadata_json, '$.corrected_subject_ref.kind') = ?1
                                    AND json_extract(action_metadata_json, '$.corrected_subject_ref.id') = ?2
                                )
                                OR (
                                    json_extract(action_metadata_json, '$.corrected_subject.kind') = ?1
                                    AND json_extract(action_metadata_json, '$.corrected_subject.id') = ?2
                                )
                            )
                         )",
                    params![
                        &subject_kind,
                        input.subject_id,
                        &new_json,
                        &new_kind,
                        new_subject_id,
                        input.reason_code,
                        now,
                        &subject_json,
                        &marker
                    ],
                )
                .map_err(|error| CorrectionArtifactError::Db(error.to_string()))?;
            for hash in &subject_hashes {
                report.subject_deltas_redacted += tx
                    .conn_ref()
                    .execute(
                        "UPDATE subject_inference_reliability_deltas
	                            SET subject_ref_hash = ?2
	                          WHERE subject_ref_hash = ?1
	                            AND status = 'applied'",
                        params![hash, &new_hash],
                    )
                    .map_err(|error| CorrectionArtifactError::Db(error.to_string()))?;
                report.subject_deltas_redacted += tx
                    .conn_ref()
                    .execute(
                        "UPDATE subject_inference_reliability_deltas
                            SET corrected_subject_ref_hash = ?2
                          WHERE corrected_subject_ref_hash = ?1
                            AND status = 'applied'",
                        params![hash, &new_hash],
                    )
                    .map_err(|error| CorrectionArtifactError::Db(error.to_string()))?;
                report.subject_aggregates_excluded += tx
                    .conn_ref()
                    .execute(
                        "UPDATE subject_inference_reliability
                            SET lifecycle_state = 'subject_rebound',
                                excluded_at = COALESCE(excluded_at, ?2),
                                updated_at = ?2
                          WHERE subject_ref_hash = ?1",
                        params![hash, now],
                    )
                    .map_err(|error| CorrectionArtifactError::Db(error.to_string()))?;
            }
            rebuild_subject_reliability_aggregate(tx, &new_hash, now)?;
            report.propagation_jobs_staled = stale_propagation_for_feedback_ids(
                tx,
                &feedback_ids,
                &marker,
                input.reason_code,
                now,
            )?;
            report.propagation_outcomes_recorded =
                record_stale_outcomes_for_feedback_ids(tx, &feedback_ids, input.reason_code, now)?;
            report.declassification_decisions_revoked = revoke_subject_declassifications(
                tx,
                &feedback_ids,
                &subject_hashes,
                "subject_rebound",
                input.reason_code,
                now,
            )?;
            report.dos338_observations_marked =
                mark_dos338_for_subject(tx, &feedback_ids, &subject_hashes, input.reason_code)?;
            report.lifecycle_events_recorded = insert_lifecycle_event(
                tx,
                "subject_ref",
                &subject_hash,
                "subject_rebound",
                actor,
                input.reason_code,
                now,
            )?;
        }
    }
    Ok(report)
}

fn remove_meeting_artifacts_in_tx(
    tx: &ActionDb,
    input: RemoveMeetingArtifactsInput<'_>,
    actor: &str,
    now: &str,
) -> Result<CorrectionArtifactLifecycleReport, CorrectionArtifactError> {
    let mut report = CorrectionArtifactLifecycleReport::default();
    let marker = lifecycle_marker("meeting_removed", input.reason_code);
    let marker_hash = stable_json_hash(&marker);
    report.prep_journals_redacted = tx
        .conn_ref()
        .execute(
            "UPDATE meeting_prep_correction_journal
                SET payload_json = ?2,
                    payload_hash = ?3,
                    lifecycle_state = 'meeting_removed',
                    meeting_id = NULL,
                    meeting_removed_reason_code = ?4,
                    updated_at = ?5
              WHERE meeting_stable_key = ?1
                AND lifecycle_state != 'meeting_removed'",
            params![
                input.meeting_stable_key,
                &marker,
                marker_hash,
                input.reason_code,
                now
            ],
        )
        .map_err(|error| CorrectionArtifactError::Db(error.to_string()))?;
    report.propagation_jobs_staled = tx
        .conn_ref()
        .execute(
            "UPDATE meeting_prep_regeneration_jobs
                SET status = 'stale',
                    stale_reason = ?2,
                    updated_at = ?3
              WHERE meeting_stable_key = ?1
                AND status IN ('pending', 'running', 'coalesced')",
            params![input.meeting_stable_key, input.reason_code, now],
        )
        .map_err(|error| CorrectionArtifactError::Db(error.to_string()))?;

    if let Some(meeting_id) = input.meeting_id {
        let subject_input = SubjectLifecycleArtifactsInput {
            subject_kind: "meeting",
            subject_id: meeting_id,
            reason_code: input.reason_code,
            action: SubjectLifecycleAction::Orphaned,
        };
        let subject_report =
            update_subject_lifecycle_artifacts_in_tx(tx, subject_input, actor, now)?;
        report.feedback_payloads_redacted += subject_report.feedback_payloads_redacted;
        report.envelopes_redacted += subject_report.envelopes_redacted;
        report.subject_deltas_redacted += subject_report.subject_deltas_redacted;
        report.subject_aggregates_excluded += subject_report.subject_aggregates_excluded;
        report.declassification_decisions_revoked +=
            subject_report.declassification_decisions_revoked;
        report.dos338_observations_marked += subject_report.dos338_observations_marked;
        report.propagation_outcomes_recorded += subject_report.propagation_outcomes_recorded;
    }
    report.lifecycle_events_recorded += insert_lifecycle_event(
        tx,
        "meeting_prep",
        input.meeting_stable_key,
        "meeting_removed",
        actor,
        input.reason_code,
        now,
    )?;
    Ok(report)
}

fn purge_dos338_proof_artifacts_in_tx(
    tx: &ActionDb,
    run_id: &str,
    reason_code: &str,
    actor: &str,
    now: &str,
) -> Result<CorrectionArtifactLifecycleReport, CorrectionArtifactError> {
    let dos338_observations_marked = tx
        .conn_ref()
        .execute(
            "UPDATE dos338_stickiness_observations
	                SET direct_surface = 'purged',
	                    indirect_surface = 'purged',
	                    direct_surface_before_hash = NULL,
	                    direct_surface_after_hash = NULL,
	                    indirect_surface_before_hash = NULL,
	                    indirect_surface_after_hash = NULL,
	                    pre_reenrichment_state_hash = NULL,
                    post_reenrichment_state_hash = NULL,
                    post_rebuild_state_hash = NULL,
                    trust_band_before = NULL,
                    trust_band_after = NULL,
                    recompute_job_id = NULL,
                    repair_job_id = NULL,
                    dead_letter_reason = ?2,
                    reason_code = ?2
              WHERE run_id = ?1",
            params![run_id, reason_code],
        )
        .map_err(|error| CorrectionArtifactError::Db(error.to_string()))?;
    let dos338_runs_purged = tx
        .conn_ref()
        .execute(
            "UPDATE dos338_stickiness_runs
                SET status = 'purged',
                    purge_state = 'purged',
                    report_hash = NULL,
                    reason_code = ?2,
                    purged_at = ?3,
                    updated_at = ?3
              WHERE id = ?1
                AND purge_state != 'purged'",
            params![run_id, reason_code, now],
        )
        .map_err(|error| CorrectionArtifactError::Db(error.to_string()))?;
    let lifecycle_events_recorded = insert_lifecycle_event(
        tx,
        "dos338_stickiness_run",
        run_id,
        "proof_purged",
        actor,
        reason_code,
        now,
    )?;
    Ok(CorrectionArtifactLifecycleReport {
        dos338_observations_marked,
        dos338_runs_purged,
        lifecycle_events_recorded,
        ..Default::default()
    })
}

fn purge_workspace_correction_artifacts_in_tx(
    tx: &ActionDb,
    reason_code: &str,
    actor: &str,
    now: &str,
) -> Result<CorrectionArtifactLifecycleReport, CorrectionArtifactError> {
    let mut report = CorrectionArtifactLifecycleReport::default();
    let marker = lifecycle_marker("workspace_reset", reason_code);
    report.feedback_payloads_redacted = tx
        .conn_ref()
        .execute(
            "UPDATE claim_feedback
                SET payload_json = ?1
              WHERE id IN (SELECT feedback_id FROM claim_feedback_correction_envelopes)
                AND payload_json IS NOT NULL",
            params![&marker],
        )
        .map_err(|error| CorrectionArtifactError::Db(error.to_string()))?;
    report.envelopes_redacted = tx
        .conn_ref()
        .execute(
            "UPDATE claim_feedback_correction_envelopes
                SET target_receipt_json = NULL,
                    asserted_subject_ref_json = ?1,
                    asserted_subject_kind = 'workspace_reset',
                    asserted_subject_id = NULL,
                    corrected_subject_ref_json = NULL,
                    source_ref = NULL,
                    source_ref_hash = NULL,
                    action_metadata_json = ?1,
                    lifecycle_state = 'workspace_reset',
                    lifecycle_reason_code = ?2,
                    updated_at = ?3
              WHERE lifecycle_state != 'workspace_reset'",
            params![&marker, reason_code, now],
        )
        .map_err(|error| CorrectionArtifactError::Db(error.to_string()))?;
    report.source_deltas_redacted = tx
        .conn_ref()
        .execute(
            "UPDATE source_reliability_feedback_deltas
                SET status = 'stale_key_version'
              WHERE status != 'stale_key_version'",
            [],
        )
        .map_err(|error| CorrectionArtifactError::Db(error.to_string()))?;
    report.source_aggregates_excluded = tx
        .conn_ref()
        .execute(
            "UPDATE source_claim_type_reliability
                SET excluded_at = COALESCE(excluded_at, ?1),
                    stale_key_version = source_key_version,
                    updated_at = ?1
              WHERE excluded_at IS NULL",
            params![now],
        )
        .map_err(|error| CorrectionArtifactError::Db(error.to_string()))?;
    report.subject_deltas_redacted = tx
        .conn_ref()
        .execute(
            "UPDATE subject_inference_reliability_deltas
                SET status = 'subject_orphaned'
              WHERE status = 'applied'",
            [],
        )
        .map_err(|error| CorrectionArtifactError::Db(error.to_string()))?;
    report.subject_aggregates_excluded = tx
        .conn_ref()
        .execute(
            "UPDATE subject_inference_reliability
                SET lifecycle_state = 'subject_orphaned',
                    excluded_at = COALESCE(excluded_at, ?1),
                    updated_at = ?1
              WHERE lifecycle_state = 'active'",
            params![now],
        )
        .map_err(|error| CorrectionArtifactError::Db(error.to_string()))?;
    report.propagation_jobs_staled = tx
        .conn_ref()
        .execute(
            "UPDATE claim_feedback_propagation_jobs
                SET status = 'stale',
                    scope_json = ?1,
                    cursor_json = NULL,
                    stale_reason = ?2,
                    updated_at = ?3
              WHERE status IN ('pending', 'running', 'coalesced')",
            params![&marker, reason_code, now],
        )
        .map_err(|error| CorrectionArtifactError::Db(error.to_string()))?;
    report.prep_journals_redacted = tx
        .conn_ref()
        .execute(
            "UPDATE meeting_prep_correction_journal
                SET payload_json = ?1,
                    payload_hash = ?2,
                    lifecycle_state = 'orphaned',
                    orphan_reason = ?3,
                    updated_at = ?4
              WHERE lifecycle_state != 'orphaned'",
            params![&marker, stable_json_hash(&marker), reason_code, now],
        )
        .map_err(|error| CorrectionArtifactError::Db(error.to_string()))?;
    report.declassification_decisions_revoked = tx
        .conn_ref()
        .execute(
            "UPDATE correction_artifact_declassification_decisions
                SET status = 'revoked',
                    revoked_reason_code = ?1,
                    revoked_at = ?2,
                    parent_lifecycle_state = 'workspace_reset'
              WHERE status = 'active'",
            params![reason_code, now],
        )
        .map_err(|error| CorrectionArtifactError::Db(error.to_string()))?;
    let proof_report = purge_all_dos338_proof_artifacts_in_tx(tx, reason_code, now)?;
    report.dos338_observations_marked = proof_report.dos338_observations_marked;
    report.dos338_runs_purged = proof_report.dos338_runs_purged;
    report.workspace_rows_purged = report.feedback_payloads_redacted
        + report.envelopes_redacted
        + report.source_deltas_redacted
        + report.source_aggregates_excluded
        + report.subject_deltas_redacted
        + report.subject_aggregates_excluded
        + report.propagation_jobs_staled
        + report.prep_journals_redacted
        + report.declassification_decisions_revoked
        + report.dos338_runs_purged;
    report.lifecycle_events_recorded = insert_lifecycle_event(
        tx,
        "workspace",
        "local_workspace",
        "workspace_reset",
        actor,
        reason_code,
        now,
    )?;
    Ok(report)
}

fn record_redaction_propagation_outcomes(
    tx: &ActionDb,
    feedback_id: &str,
    reason_code: &str,
    now: &str,
) -> Result<usize, CorrectionArtifactError> {
    let mut stmt = tx
        .conn_ref()
        .prepare(
            "SELECT id, target_kind, operation, sync_class
               FROM claim_feedback_propagation_jobs
              WHERE feedback_id = ?1",
        )
        .map_err(|error| CorrectionArtifactError::Db(error.to_string()))?;
    let jobs = stmt
        .query_map(params![feedback_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .map_err(|error| CorrectionArtifactError::Db(error.to_string()))?;
    let mut recorded = 0usize;
    for job in jobs {
        let (job_id, target_kind, operation, sync_class) =
            job.map_err(|error| CorrectionArtifactError::Db(error.to_string()))?;
        tx.conn_ref()
            .execute(
                "INSERT INTO claim_feedback_propagation_outcomes (
                    id, job_id, feedback_id, target_kind, operation, sync_class,
                    status, reason_code, observed_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'stale', ?7, ?8)",
                params![
                    Uuid::new_v4().to_string(),
                    job_id,
                    feedback_id,
                    target_kind,
                    operation,
                    sync_class,
                    reason_code,
                    now
                ],
            )
            .map_err(|error| CorrectionArtifactError::Db(error.to_string()))?;
        recorded += 1;
    }
    Ok(recorded)
}

fn feedback_ids_for_source_key(
    tx: &ActionDb,
    source_key_hash: &str,
) -> Result<Vec<String>, CorrectionArtifactError> {
    let mut stmt = tx
        .conn_ref()
        .prepare(
            "SELECT feedback_id
               FROM claim_feedback_correction_envelopes
              WHERE source_key_hash = ?1
             UNION
             SELECT feedback_id
               FROM source_reliability_feedback_deltas
              WHERE source_key_hash = ?1",
        )
        .map_err(|error| CorrectionArtifactError::Db(error.to_string()))?;
    let rows = stmt
        .query_map(params![source_key_hash], |row| row.get::<_, String>(0))
        .map_err(|error| CorrectionArtifactError::Db(error.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| CorrectionArtifactError::Db(error.to_string()))?;
    Ok(rows)
}

fn source_keys_for_data_source(
    tx: &ActionDb,
    data_source: &str,
) -> Result<Vec<String>, CorrectionArtifactError> {
    let mut stmt = tx
        .conn_ref()
        .prepare(
            "SELECT DISTINCT source_key_hash
               FROM claim_feedback_correction_envelopes
              WHERE source_key_hash IS NOT NULL
                AND source_key_hash != ''
                AND (
                    data_source = ?1
                    OR substr(data_source, 1, length(?1) + 1) = ?1 || '_'
                    OR substr(data_source, 1, length(?1) + 1) = ?1 || ':'
                )
             UNION
             SELECT DISTINCT source_key_hash
               FROM source_claim_type_reliability
              WHERE source_key_hash IS NOT NULL
                AND source_key_hash != ''
                AND (
                    data_source = ?1
                    OR substr(data_source, 1, length(?1) + 1) = ?1 || '_'
                    OR substr(data_source, 1, length(?1) + 1) = ?1 || ':'
                )
             UNION
             SELECT DISTINCT source_key_hash
               FROM source_reliability_feedback_deltas
              WHERE source_key_hash IS NOT NULL
                AND source_key_hash != ''
                AND (
                    data_source = ?1
                    OR substr(data_source, 1, length(?1) + 1) = ?1 || '_'
                    OR substr(data_source, 1, length(?1) + 1) = ?1 || ':'
                )",
        )
        .map_err(|error| CorrectionArtifactError::Db(error.to_string()))?;
    let rows = stmt
        .query_map(params![data_source], |row| row.get::<_, String>(0))
        .map_err(|error| CorrectionArtifactError::Db(error.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| CorrectionArtifactError::Db(error.to_string()))?;
    Ok(rows)
}

fn feedback_ids_for_subject(
    tx: &ActionDb,
    subject_kind: &str,
    subject_id: &str,
) -> Result<Vec<String>, CorrectionArtifactError> {
    let subject_json = canonical_subject_json(subject_kind, subject_id);
    let mut stmt = tx
        .conn_ref()
        .prepare(
            "SELECT feedback_id
               FROM claim_feedback_correction_envelopes
              WHERE asserted_subject_kind = ?1
                AND asserted_subject_id = ?2
             UNION
             SELECT feedback_id
               FROM claim_feedback_correction_envelopes
              WHERE corrected_subject_ref_json = ?3
                 OR (
                    json_valid(corrected_subject_ref_json) = 1
                    AND json_extract(corrected_subject_ref_json, '$.kind') = ?1
                    AND json_extract(corrected_subject_ref_json, '$.id') = ?2
                 )
                 OR (
                    json_valid(action_metadata_json) = 1
                    AND (
                        (
                            json_extract(action_metadata_json, '$.corrected_subject_ref.kind') = ?1
                            AND json_extract(action_metadata_json, '$.corrected_subject_ref.id') = ?2
                        )
                        OR (
                            json_extract(action_metadata_json, '$.corrected_subject.kind') = ?1
                            AND json_extract(action_metadata_json, '$.corrected_subject.id') = ?2
                        )
                    )
                 )",
        )
        .map_err(|error| CorrectionArtifactError::Db(error.to_string()))?;
    let rows = stmt
        .query_map(params![subject_kind, subject_id, &subject_json], |row| {
            row.get::<_, String>(0)
        })
        .map_err(|error| CorrectionArtifactError::Db(error.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| CorrectionArtifactError::Db(error.to_string()))?;
    Ok(rows)
}

fn subject_hashes_for_feedback_ids(
    tx: &ActionDb,
    feedback_ids: &[String],
) -> Result<Vec<String>, CorrectionArtifactError> {
    let mut hashes = Vec::new();
    for feedback_id in feedback_ids {
        let mut stmt = tx
            .conn_ref()
            .prepare(
                "SELECT DISTINCT json_extract(scope_json, '$.subject_ref_hash')
                   FROM claim_feedback_propagation_jobs
                  WHERE feedback_id = ?1
                    AND json_extract(scope_json, '$.subject_ref_hash') IS NOT NULL
                 UNION
	                 SELECT DISTINCT subject_ref_hash
	                   FROM subject_inference_reliability_deltas
	                  WHERE feedback_id = ?1
	                 UNION
	                 SELECT DISTINCT corrected_subject_ref_hash
	                   FROM subject_inference_reliability_deltas
	                  WHERE feedback_id = ?1
	                    AND corrected_subject_ref_hash IS NOT NULL",
            )
            .map_err(|error| CorrectionArtifactError::Db(error.to_string()))?;
        let rows = stmt
            .query_map(params![feedback_id], |row| row.get::<_, String>(0))
            .map_err(|error| CorrectionArtifactError::Db(error.to_string()))?;
        for row in rows {
            let hash = row.map_err(|error| CorrectionArtifactError::Db(error.to_string()))?;
            if !hashes.iter().any(|existing| existing == &hash) {
                hashes.push(hash);
            }
        }
    }
    Ok(hashes)
}

fn stale_propagation_for_feedback_ids(
    tx: &ActionDb,
    feedback_ids: &[String],
    marker: &str,
    reason_code: &str,
    now: &str,
) -> Result<usize, CorrectionArtifactError> {
    let mut updated = 0usize;
    for feedback_id in feedback_ids {
        tx.conn_ref()
            .execute(
                "UPDATE claim_feedback_propagation_jobs
                    SET scope_json = ?2,
                        cursor_json = NULL,
                        coalescing_key = 'lifecycle:' || id,
                        updated_at = ?3
                  WHERE feedback_id = ?1",
                params![feedback_id, marker, now],
            )
            .map_err(|error| CorrectionArtifactError::Db(error.to_string()))?;
        updated += tx
            .conn_ref()
            .execute(
                "UPDATE claim_feedback_propagation_jobs
                    SET status = 'stale',
                        stale_reason = ?2,
                        updated_at = ?3
                  WHERE feedback_id = ?1
                    AND status IN ('pending', 'running', 'coalesced')",
                params![feedback_id, reason_code, now],
            )
            .map_err(|error| CorrectionArtifactError::Db(error.to_string()))?;
    }
    Ok(updated)
}

fn record_stale_outcomes_for_feedback_ids(
    tx: &ActionDb,
    feedback_ids: &[String],
    reason_code: &str,
    now: &str,
) -> Result<usize, CorrectionArtifactError> {
    let mut recorded = 0usize;
    for feedback_id in feedback_ids {
        recorded += record_redaction_propagation_outcomes(tx, feedback_id, reason_code, now)?;
    }
    Ok(recorded)
}

fn revoke_subject_declassifications(
    tx: &ActionDb,
    feedback_ids: &[String],
    subject_hashes: &[String],
    parent_lifecycle_state: &str,
    reason_code: &str,
    now: &str,
) -> Result<usize, CorrectionArtifactError> {
    let mut revoked = 0usize;
    for feedback_id in feedback_ids {
        revoked += tx
            .conn_ref()
            .execute(
                "UPDATE correction_artifact_declassification_decisions
                    SET status = 'revoked',
                        revoked_reason_code = ?2,
                        revoked_at = ?3,
                        parent_lifecycle_state = ?4
                  WHERE artifact_id = ?1
                    AND status = 'active'",
                params![feedback_id, reason_code, now, parent_lifecycle_state],
            )
            .map_err(|error| CorrectionArtifactError::Db(error.to_string()))?;
    }
    for subject_hash in subject_hashes {
        revoked += tx
            .conn_ref()
            .execute(
                "UPDATE correction_artifact_declassification_decisions
                    SET status = 'revoked',
                        revoked_reason_code = ?2,
                        revoked_at = ?3,
                        parent_lifecycle_state = ?4
                  WHERE source_artifact_hash = ?1
                    AND status = 'active'",
                params![subject_hash, reason_code, now, parent_lifecycle_state],
            )
            .map_err(|error| CorrectionArtifactError::Db(error.to_string()))?;
    }
    Ok(revoked)
}

fn mark_dos338_for_subject(
    tx: &ActionDb,
    feedback_ids: &[String],
    subject_hashes: &[String],
    reason_code: &str,
) -> Result<usize, CorrectionArtifactError> {
    let mut marked = 0usize;
    for feedback_id in feedback_ids {
        marked += tx
            .conn_ref()
            .execute(
                "UPDATE dos338_stickiness_observations
                    SET reason_code = COALESCE(reason_code, ?2),
                        dead_letter_reason = COALESCE(dead_letter_reason, ?2)
                  WHERE feedback_id = ?1",
                params![feedback_id, reason_code],
            )
            .map_err(|error| CorrectionArtifactError::Db(error.to_string()))?;
    }
    for subject_hash in subject_hashes {
        marked += tx
            .conn_ref()
            .execute(
                "UPDATE dos338_stickiness_observations
                    SET reason_code = COALESCE(reason_code, ?2),
                        dead_letter_reason = COALESCE(dead_letter_reason, ?2)
                  WHERE subject_ref_hash = ?1",
                params![subject_hash, reason_code],
            )
            .map_err(|error| CorrectionArtifactError::Db(error.to_string()))?;
    }
    Ok(marked)
}

fn rebuild_subject_reliability_aggregate(
    tx: &ActionDb,
    subject_ref_hash: &str,
    now: &str,
) -> Result<(), CorrectionArtifactError> {
    tx.conn_ref()
        .execute(
            "INSERT INTO subject_inference_reliability (
                subject_ref_hash, claim_type, signal_type, alpha, beta,
                update_count, lifecycle_state, excluded_at, updated_at
             )
             SELECT subject_ref_hash,
                    claim_type,
                    signal_type,
                    1.0 + COALESCE(SUM(alpha_delta), 0.0),
                    1.0 + COALESCE(SUM(beta_delta), 0.0),
                    COUNT(*),
                    'active',
                    NULL,
                    ?2
               FROM subject_inference_reliability_deltas
              WHERE subject_ref_hash = ?1
                AND status = 'applied'
              GROUP BY subject_ref_hash, claim_type, signal_type
             ON CONFLICT(subject_ref_hash, claim_type, signal_type)
             DO UPDATE SET
                alpha = excluded.alpha,
                beta = excluded.beta,
                update_count = excluded.update_count,
                lifecycle_state = 'active',
                excluded_at = NULL,
                updated_at = ?2",
            params![subject_ref_hash, now],
        )
        .map_err(|error| CorrectionArtifactError::Db(error.to_string()))?;
    Ok(())
}

fn purge_all_dos338_proof_artifacts_in_tx(
    tx: &ActionDb,
    reason_code: &str,
    now: &str,
) -> Result<CorrectionArtifactLifecycleReport, CorrectionArtifactError> {
    let dos338_observations_marked = tx
        .conn_ref()
        .execute(
            "UPDATE dos338_stickiness_observations
                SET direct_surface = 'purged',
                    indirect_surface = 'purged',
                    direct_surface_before_hash = NULL,
                    direct_surface_after_hash = NULL,
                    indirect_surface_before_hash = NULL,
                    indirect_surface_after_hash = NULL,
                    pre_reenrichment_state_hash = NULL,
                    post_reenrichment_state_hash = NULL,
                    post_rebuild_state_hash = NULL,
                    trust_band_before = NULL,
                    trust_band_after = NULL,
                    recompute_job_id = NULL,
                    repair_job_id = NULL,
                    dead_letter_reason = ?1,
                    reason_code = ?1",
            params![reason_code],
        )
        .map_err(|error| CorrectionArtifactError::Db(error.to_string()))?;
    let dos338_runs_purged = tx
        .conn_ref()
        .execute(
            "UPDATE dos338_stickiness_runs
                SET status = 'purged',
                    purge_state = 'purged',
                    report_hash = NULL,
                    reason_code = ?1,
                    purged_at = ?2,
                    updated_at = ?2
              WHERE purge_state != 'purged'",
            params![reason_code, now],
        )
        .map_err(|error| CorrectionArtifactError::Db(error.to_string()))?;
    Ok(CorrectionArtifactLifecycleReport {
        dos338_observations_marked,
        dos338_runs_purged,
        ..Default::default()
    })
}

fn normalized_subject_kind(subject_kind: &str) -> String {
    match subject_kind.trim().to_ascii_lowercase().as_str() {
        "accounts" => "account".to_string(),
        "meetings" => "meeting".to_string(),
        "people" => "person".to_string(),
        "projects" => "project".to_string(),
        "emails" => "email".to_string(),
        "actions" => "action".to_string(),
        other => other.to_string(),
    }
}

fn canonical_subject_json(subject_kind: &str, subject_id: &str) -> String {
    json!({"kind": subject_kind, "id": subject_id}).to_string()
}

fn subject_hash_for_json(subject_json: &str) -> Result<String, CorrectionArtifactError> {
    pii_safe_hash("subject", "dailyos.w4.correction.subject", &[subject_json])
}

fn lifecycle_marker(lifecycle_state: &str, reason_code: &str) -> String {
    json!({
        "lifecycle_state": lifecycle_state,
        "reason_code": reason_code,
    })
    .to_string()
}

fn insert_lifecycle_event(
    tx: &ActionDb,
    artifact_kind: &str,
    artifact_id: &str,
    event_type: &str,
    actor: &str,
    reason_code: &str,
    now: &str,
) -> Result<usize, CorrectionArtifactError> {
    let artifact_id_hash = pii_safe_hash(
        "correction_artifact",
        "dailyos.w4.correction_artifact.lifecycle.artifact",
        &[artifact_kind, artifact_id],
    )?;
    let detail_hash = pii_safe_hash(
        "correction_artifact",
        "dailyos.w4.correction_artifact.lifecycle.detail",
        &[artifact_kind, &artifact_id_hash, reason_code],
    )?;
    let actor_class = lifecycle_actor_class(actor);
    tx.conn_ref()
        .execute(
            "INSERT INTO correction_artifact_lifecycle_events (
                id, artifact_kind, artifact_id, event_type, actor,
                reason_code, pii_safe_detail_hash, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                Uuid::new_v4().to_string(),
                artifact_kind,
                artifact_id_hash,
                event_type,
                actor_class,
                reason_code,
                detail_hash,
                now,
            ],
        )
        .map_err(|error| CorrectionArtifactError::Db(error.to_string()))
}

fn lifecycle_actor_class(actor: &str) -> &'static str {
    if actor.trim().to_ascii_lowercase().starts_with("user") {
        "user"
    } else {
        "unknown"
    }
}

fn pii_safe_hash(
    prefix: &str,
    domain: &str,
    components: &[&str],
) -> Result<String, CorrectionArtifactError> {
    #[cfg(test)]
    {
        Ok(crate::db::local_db_keyed_audit_tag_for_tests(
            "w4-correction-artifacts-test-secret",
            prefix,
            domain,
            components,
        ))
    }

    #[cfg(not(test))]
    {
        crate::db::local_db_keyed_audit_tag(prefix, domain, components)
            .map_err(|error| CorrectionArtifactError::Db(format!("derive PII-safe hash: {error}")))
    }
}

fn stable_json_hash(raw: &str) -> String {
    format!(
        "sha256:{}",
        hex::encode(sha2::Sha256::digest(raw.as_bytes()))
    )
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};
    use rusqlite::params;

    use super::*;
    use crate::abilities::feedback::FeedbackAction;
    use crate::db::test_utils::test_db;
    use crate::db::DbAccount;
    use crate::services::claims::{
        commit_claim, record_claim_feedback, ClaimFeedbackInput, ClaimProposal, CommittedClaim,
    };
    use crate::services::context::{ExternalClients, FixedClock, SeedableRng};

    fn test_ctx<'a>(
        clock: &'a FixedClock,
        rng: &'a SeedableRng,
        external: &'a ExternalClients,
        actor: &'a str,
    ) -> ServiceContext<'a> {
        ServiceContext::test_live(clock, rng, external).with_actor(actor)
    }

    fn seed_account(db: &ActionDb) {
        db.upsert_account(&DbAccount {
            id: "acct-redaction".to_string(),
            name: "Account Redaction".to_string(),
            updated_at: "2026-06-05T00:00:00Z".to_string(),
            ..Default::default()
        })
        .expect("seed account");
    }

    fn proposal() -> ClaimProposal {
        ClaimProposal {
            id: None,
            expected_claim_version: None,
            subject_ref: json!({"kind": "account", "id": "acct-redaction"}).to_string(),
            claim_type: "risk".to_string(),
            field_path: Some("health.risk".to_string()),
            topic_key: None,
            text: "Synthetic risk claim".to_string(),
            actor: "agent:test".to_string(),
            data_source: "unit_test".to_string(),
            source_ref: Some("fixture://source-redaction".to_string()),
            source_asof: Some("2026-06-05T00:00:00Z".to_string()),
            observed_at: "2026-06-05T00:00:00Z".to_string(),
            provenance_json: "{}".to_string(),
            metadata_json: None,
            thread_id: None,
            temporal_scope: Some(crate::db::claims::TemporalScope::State),
            sensitivity: Some(crate::db::claims::ClaimSensitivity::Internal),
            supersedes: None,
            tombstone: None,
        }
    }

    fn inserted_claim_id(result: CommittedClaim) -> String {
        match result {
            CommittedClaim::Inserted { claim }
            | CommittedClaim::Reinforced { claim, .. }
            | CommittedClaim::Tombstoned { claim }
            | CommittedClaim::Forked {
                primary_claim: claim,
                ..
            } => claim.id,
        }
    }

    fn seed_declassification_decision(db: &ActionDb, artifact_id: &str, source_hash: &str) {
        db.conn_ref()
            .execute(
                "INSERT INTO correction_artifact_declassification_decisions (
                    id, artifact_kind, artifact_id, source_artifact_hash,
                    source_artifact_version, derived_field, destination_surface,
                    decision_version, source_sensitivity, requester_actor, status,
                    reason_code, parent_version_hash
                 ) VALUES (?1, 'claim_feedback', ?2, ?3, '1',
                    'has_user_correction', 'mcp', 1, 'user_only',
                    'user', 'active', 'fixture_declassification', 'parent_hash_01')",
                params![Uuid::new_v4().to_string(), artifact_id, source_hash],
            )
            .expect("seed declassification decision");
    }

    fn seed_stickiness_observation(
        db: &ActionDb,
        feedback_id: Option<&str>,
        subject_ref_hash: &str,
    ) -> String {
        let run_id = Uuid::new_v4().to_string();
        db.conn_ref()
            .execute(
                "INSERT INTO dos338_stickiness_runs (
                    id, fixture_id_hash, entry_point, status, report_hash,
                    started_at, completed_at, created_at, updated_at
                 ) VALUES (?1, 'fixture_hash_01', 'app', 'completed',
                    'sha256:fixture_report', '2026-06-05T00:00:00Z',
                    '2026-06-05T00:00:00Z', '2026-06-05T00:00:00Z',
                    '2026-06-05T00:00:00Z')",
                params![&run_id],
            )
            .expect("seed stickiness run");
        db.conn_ref()
            .execute(
                "INSERT INTO dos338_stickiness_observations (
                    id, run_id, feedback_id, entry_point, action, subject_kind,
                    subject_ref_hash, direct_surface, indirect_surface,
                    direct_surface_before_hash, direct_surface_after_hash,
                    indirect_surface_before_hash, indirect_surface_after_hash,
                    pre_reenrichment_state_hash, post_reenrichment_state_hash,
                    post_rebuild_state_hash, trust_band_before, trust_band_after,
                    recompute_job_id, repair_job_id, sensitivity_gate_result,
                    result, observed_at
                 ) VALUES (?1, ?2, ?3, 'app', 'wrong_source', 'account',
                    ?4, 'claim_receipt', 'meeting_readiness',
                    'sha256:direct-before', 'sha256:direct-after',
                    'sha256:indirect-before', 'sha256:indirect-after',
                    'sha256:pre', 'sha256:post', 'sha256:rebuild',
                    'likely_current', 'use_with_caution', 'job_01', NULL,
                    'local_only_allowed', 'passed', '2026-06-05T00:00:00Z')",
                params![
                    Uuid::new_v4().to_string(),
                    &run_id,
                    feedback_id,
                    subject_ref_hash
                ],
            )
            .expect("seed stickiness observation");
        run_id
    }

    fn envelope_source_key(db: &ActionDb, feedback_id: &str) -> String {
        db.conn_ref()
            .query_row(
                "SELECT source_key_hash
                   FROM claim_feedback_correction_envelopes
                  WHERE feedback_id = ?1",
                params![feedback_id],
                |row| row.get(0),
            )
            .expect("read source key hash")
    }

    fn subject_hash_for_feedback(db: &ActionDb, feedback_id: &str) -> String {
        db.conn_ref()
            .query_row(
                "SELECT json_extract(scope_json, '$.subject_ref_hash')
                   FROM claim_feedback_propagation_jobs
                  WHERE feedback_id = ?1
                    AND json_extract(scope_json, '$.subject_ref_hash') IS NOT NULL
                  LIMIT 1",
                params![feedback_id],
                |row| row.get(0),
            )
            .expect("read subject hash")
    }

    #[test]
    fn redact_feedback_artifacts_scrubs_payloads_and_stales_propagation() {
        let db = test_db();
        seed_account(&db);
        let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 6, 5, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(7);
        let external = ExternalClients::default();
        let write_ctx = test_ctx(&clock, &rng, &external, "user:test");
        let claim_id = inserted_claim_id(commit_claim(&write_ctx, &db, proposal()).unwrap());
        let feedback = record_claim_feedback(
            &write_ctx,
            &db,
            ClaimFeedbackInput {
                claim_id,
                action: FeedbackAction::WrongSource,
                actor: "user".to_string(),
                actor_id: Some("user-fixture".to_string()),
                payload_json: Some(
                    json!({
                        "surface": "entity_detail",
                        "source_ref": "fixture://source-redaction",
                        "note": "sensitive local correction"
                    })
                    .to_string(),
                ),
            },
        )
        .expect("record feedback");

        let report = redact_feedback_artifacts(
            &write_ctx,
            &db,
            RedactFeedbackArtifactsInput {
                feedback_id: &feedback.feedback_id,
                reason_code: "user_requested_redaction",
            },
        )
        .expect("redact feedback artifacts");
        assert_eq!(report.feedback_payloads_redacted, 1);
        assert_eq!(report.envelopes_redacted, 1);
        assert!(report.source_deltas_redacted >= 1);
        assert!(report.propagation_jobs_staled >= 1);
        assert!(report.propagation_outcomes_recorded >= 1);
        assert_eq!(report.lifecycle_events_recorded, 1);

        let (payload_json, envelope_state, source_ref): (String, String, Option<String>) = db
            .conn_ref()
            .query_row(
                "SELECT feedback.payload_json, envelope.lifecycle_state, envelope.source_ref
                   FROM claim_feedback feedback
                   JOIN claim_feedback_correction_envelopes envelope
                     ON envelope.feedback_id = feedback.id
                  WHERE feedback.id = ?1",
                params![&feedback.feedback_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("read redacted artifacts");
        assert!(payload_json.contains("\"redacted\":true"));
        assert_eq!(envelope_state, "redacted");
        assert_eq!(source_ref, None);

        let (scope_json, cursor_json, coalescing_key): (String, Option<String>, String) = db
            .conn_ref()
            .query_row(
                "SELECT scope_json, cursor_json, coalescing_key
                   FROM claim_feedback_propagation_jobs
                  WHERE feedback_id = ?1
                  LIMIT 1",
                params![&feedback.feedback_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("read redacted propagation payloads");
        assert!(scope_json.contains("\"redacted\":true"));
        assert_eq!(cursor_json, None);
        assert!(coalescing_key.starts_with("redacted:"));

        let (artifact_id, actor): (String, String) = db
            .conn_ref()
            .query_row(
                "SELECT artifact_id, actor
                   FROM correction_artifact_lifecycle_events
                  WHERE artifact_kind = 'claim_feedback'
                  LIMIT 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("read lifecycle event");
        assert_ne!(artifact_id, feedback.feedback_id);
        assert_eq!(actor, "user");
    }

    #[test]
    fn redact_feedback_artifacts_preserves_other_source_reliability_evidence() {
        let db = test_db();
        seed_account(&db);
        let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 6, 5, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(7);
        let external = ExternalClients::default();
        let write_ctx = test_ctx(&clock, &rng, &external, "user:test");
        let first_claim_id = inserted_claim_id(commit_claim(&write_ctx, &db, proposal()).unwrap());
        let mut second = proposal();
        second.text = "Synthetic second risk claim".to_string();
        let second_claim_id = inserted_claim_id(commit_claim(&write_ctx, &db, second).unwrap());

        let first_feedback = record_claim_feedback(
            &write_ctx,
            &db,
            ClaimFeedbackInput {
                claim_id: first_claim_id,
                action: FeedbackAction::WrongSource,
                actor: "user".to_string(),
                actor_id: Some("user-fixture".to_string()),
                payload_json: Some(
                    json!({
                        "surface": "entity_detail",
                        "source_ref": "fixture://source-redaction"
                    })
                    .to_string(),
                ),
            },
        )
        .expect("record first feedback");
        let second_feedback = record_claim_feedback(
            &write_ctx,
            &db,
            ClaimFeedbackInput {
                claim_id: second_claim_id,
                action: FeedbackAction::WrongSource,
                actor: "user".to_string(),
                actor_id: Some("user-fixture".to_string()),
                payload_json: Some(
                    json!({
                        "surface": "entity_detail",
                        "source_ref": "fixture://source-redaction"
                    })
                    .to_string(),
                ),
            },
        )
        .expect("record second feedback");

        redact_feedback_artifacts(
            &write_ctx,
            &db,
            RedactFeedbackArtifactsInput {
                feedback_id: &first_feedback.feedback_id,
                reason_code: "user_requested_redaction",
            },
        )
        .expect("redact first feedback artifacts");

        let (alpha, beta, update_count, excluded_at): (f64, f64, i64, Option<String>) = db
            .conn_ref()
            .query_row(
                "SELECT alpha, beta, update_count, excluded_at
                   FROM source_claim_type_reliability
                  WHERE claim_type = 'risk'
                    AND signal_type = 'user_feedback'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("read recomputed source reliability aggregate");
        assert_eq!(alpha, 1.0);
        assert_eq!(beta, 2.0);
        assert_eq!(update_count, 1);
        assert_eq!(
            excluded_at, None,
            "aggregate must stay active while another applied delta remains"
        );

        let (redacted_count, applied_count): (i64, i64) = db
            .conn_ref()
            .query_row(
                "SELECT
                    sum(CASE WHEN feedback_id = ?1 AND status = 'redacted' THEN 1 ELSE 0 END),
                    sum(CASE WHEN feedback_id = ?2 AND status = 'applied' THEN 1 ELSE 0 END)
                   FROM source_reliability_feedback_deltas
                  WHERE feedback_id IN (?1, ?2)",
                params![&first_feedback.feedback_id, &second_feedback.feedback_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("read redacted and retained delta counts");
        assert_eq!(redacted_count, 1);
        assert_eq!(applied_count, 1);
    }

    #[test]
    fn redact_feedback_artifacts_rejects_agent_actor() {
        let db = test_db();
        let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 6, 5, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(7);
        let external = ExternalClients::default();
        let agent_ctx = test_ctx(&clock, &rng, &external, "agent:test");

        let err = redact_feedback_artifacts(
            &agent_ctx,
            &db,
            RedactFeedbackArtifactsInput {
                feedback_id: "feedback-fixture",
                reason_code: "user_requested_redaction",
            },
        )
        .expect_err("agent actor must be rejected");
        assert!(matches!(err, CorrectionArtifactError::UnauthorizedActor(_)));
    }

    #[test]
    fn redact_feedback_artifacts_rejects_admin_actor() {
        let db = test_db();
        let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 6, 5, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(7);
        let external = ExternalClients::default();
        let admin_ctx = test_ctx(&clock, &rng, &external, "admin:test");

        let err = redact_feedback_artifacts(
            &admin_ctx,
            &db,
            RedactFeedbackArtifactsInput {
                feedback_id: "feedback-fixture",
                reason_code: "user_requested_redaction",
            },
        )
        .expect_err("admin actor must be rejected");
        assert!(matches!(err, CorrectionArtifactError::UnauthorizedActor(_)));
    }

    #[test]
    fn w4_source_removal_scrubs_source_refs_and_excludes_reliability() {
        let db = test_db();
        seed_account(&db);
        let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 6, 5, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(7);
        let external = ExternalClients::default();
        let write_ctx = test_ctx(&clock, &rng, &external, "user:test");
        let claim_id = inserted_claim_id(commit_claim(&write_ctx, &db, proposal()).unwrap());
        let feedback = record_claim_feedback(
            &write_ctx,
            &db,
            ClaimFeedbackInput {
                claim_id,
                action: FeedbackAction::WrongSource,
                actor: "user".to_string(),
                actor_id: Some("user-fixture".to_string()),
                payload_json: Some(
                    json!({
                        "surface": "entity_detail",
                        "source_ref": "fixture://source-removal"
                    })
                    .to_string(),
                ),
            },
        )
        .expect("record feedback");
        let source_key_hash = envelope_source_key(&db, &feedback.feedback_id);
        seed_declassification_decision(&db, &feedback.feedback_id, &source_key_hash);
        seed_stickiness_observation(&db, Some(&feedback.feedback_id), "subject_hash_01");

        let report = remove_source_artifacts(
            &write_ctx,
            &db,
            RemoveSourceArtifactsInput {
                source_key_hash: &source_key_hash,
                reason_code: "source_removed_by_user",
            },
        )
        .expect("remove source artifacts");

        assert_eq!(report.envelopes_redacted, 1);
        assert!(report.source_deltas_redacted >= 1);
        assert_eq!(report.source_aggregates_excluded, 1);
        assert_eq!(report.declassification_decisions_revoked, 1);
        assert_eq!(report.dos338_observations_marked, 1);

        let (state, source_ref, source_ref_hash, excluded_at, delta_status): (
            String,
            Option<String>,
            Option<String>,
            Option<String>,
            String,
        ) = db
            .conn_ref()
            .query_row(
                "SELECT envelope.lifecycle_state,
                        envelope.source_ref,
                        envelope.source_ref_hash,
                        aggregate.excluded_at,
                        delta.status
                   FROM claim_feedback_correction_envelopes envelope
                   JOIN source_claim_type_reliability aggregate
                     ON aggregate.source_key_hash = envelope.source_key_hash
                   JOIN source_reliability_feedback_deltas delta
                     ON delta.feedback_id = envelope.feedback_id
                  WHERE envelope.feedback_id = ?1",
                params![&feedback.feedback_id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .expect("read source lifecycle state");
        assert_eq!(state, "source_removed");
        assert_eq!(source_ref, None);
        assert_eq!(source_ref_hash, None);
        assert!(excluded_at.is_some());
        assert_eq!(delta_status, "source_removed");
    }

    #[test]
    fn w4_source_removal_redacts_historical_feedback_payload_without_envelope() {
        let db = test_db();
        seed_account(&db);
        let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 6, 5, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(7);
        let external = ExternalClients::default();
        let write_ctx = test_ctx(&clock, &rng, &external, "user:test");
        let mut source_proposal = proposal();
        source_proposal.source_ref = Some("fixture://historical-source-removal".to_string());
        let claim_id = inserted_claim_id(commit_claim(&write_ctx, &db, source_proposal).unwrap());
        let feedback = record_claim_feedback(
            &write_ctx,
            &db,
            ClaimFeedbackInput {
                claim_id,
                action: FeedbackAction::WrongSource,
                actor: "user".to_string(),
                actor_id: Some("user-fixture".to_string()),
                payload_json: Some(
                    json!({
                        "surface": "entity_detail",
                        "source_ref": "fixture://historical-source-removal"
                    })
                    .to_string(),
                ),
            },
        )
        .expect("record feedback");
        let source_key_hash: String = db
            .conn_ref()
            .query_row(
                "SELECT source_key_hash
                   FROM source_reliability_feedback_deltas
                  WHERE feedback_id = ?1",
                params![&feedback.feedback_id],
                |row| row.get(0),
            )
            .expect("read source key hash from source reliability delta");
        db.conn_ref()
            .execute(
                "DELETE FROM claim_feedback_correction_envelopes WHERE feedback_id = ?1",
                params![&feedback.feedback_id],
            )
            .expect("delete envelope to model historical backfilled feedback");

        let report = remove_source_artifacts(
            &write_ctx,
            &db,
            RemoveSourceArtifactsInput {
                source_key_hash: &source_key_hash,
                reason_code: "source_removed_by_user",
            },
        )
        .expect("remove source artifacts");

        assert_eq!(report.envelopes_redacted, 0);
        assert_eq!(report.feedback_payloads_redacted, 1);
        assert!(report.source_deltas_redacted >= 1);

        let (payload_json, delta_status): (String, String) = db
            .conn_ref()
            .query_row(
                "SELECT feedback.payload_json, delta.status
                   FROM claim_feedback feedback
                   JOIN source_reliability_feedback_deltas delta
                     ON delta.feedback_id = feedback.id
                  WHERE feedback.id = ?1",
                params![&feedback.feedback_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("read redacted historical feedback");
        assert!(!payload_json.contains("historical-source-removal"));
        assert!(payload_json.contains("source_removed"));
        assert_eq!(delta_status, "source_removed");
    }

    #[test]
    fn w4_data_source_purge_marks_correction_artifacts_in_production_path() {
        let db = test_db();
        seed_account(&db);
        let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 6, 5, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(7);
        let external = ExternalClients::default();
        let write_ctx = test_ctx(&clock, &rng, &external, "user:test");
        let mut glean_proposal = proposal();
        glean_proposal.data_source = "glean_crm".to_string();
        glean_proposal.source_ref = Some("fixture://glean-source-purge".to_string());
        let claim_id = inserted_claim_id(commit_claim(&write_ctx, &db, glean_proposal).unwrap());
        let feedback = record_claim_feedback(
            &write_ctx,
            &db,
            ClaimFeedbackInput {
                claim_id,
                action: FeedbackAction::WrongSource,
                actor: "user".to_string(),
                actor_id: Some("user-fixture".to_string()),
                payload_json: Some(
                    json!({
                        "surface": "entity_detail",
                        "source_ref": "fixture://glean-source-purge"
                    })
                    .to_string(),
                ),
            },
        )
        .expect("record feedback");

        let purge_report = crate::db::data_lifecycle::purge_source(
            &db,
            crate::db::data_lifecycle::DataSource::Glean,
        )
        .expect("purge Glean source");

        assert!(
            purge_report.correction_artifacts_lifecycle_marked > 0,
            "source purge must mark W4 correction artifacts through production purge path"
        );
        let state: String = db
            .conn_ref()
            .query_row(
                "SELECT lifecycle_state
                   FROM claim_feedback_correction_envelopes
                  WHERE feedback_id = ?1",
                params![&feedback.feedback_id],
                |row| row.get(0),
            )
            .expect("read source-purged envelope");
        assert_eq!(state, "source_removed");
    }

    #[test]
    fn w4_data_source_purge_matches_literal_source_family_separator() {
        let db = test_db();
        seed_account(&db);
        let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 6, 5, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(7);
        let external = ExternalClients::default();
        let write_ctx = test_ctx(&clock, &rng, &external, "user:test");

        let mut glean_proposal = proposal();
        glean_proposal.text = "Synthetic Glean risk claim".to_string();
        glean_proposal.data_source = "glean_crm".to_string();
        glean_proposal.source_ref = Some("fixture://literal-glean-source-purge".to_string());
        let glean_claim_id =
            inserted_claim_id(commit_claim(&write_ctx, &db, glean_proposal).unwrap());

        let mut sibling_proposal = proposal();
        sibling_proposal.text = "Synthetic sibling risk claim".to_string();
        sibling_proposal.data_source = "gleaner".to_string();
        sibling_proposal.source_ref = Some("fixture://literal-gleaner-source".to_string());
        let sibling_claim_id =
            inserted_claim_id(commit_claim(&write_ctx, &db, sibling_proposal).unwrap());

        db.conn_ref()
            .execute_batch(
                "INSERT INTO claim_feedback (
                    id, claim_id, feedback_type, actor, actor_id, payload_json,
                    submitted_at, applied_at
                 ) VALUES
                    (
                        'feedback-literal-glean', 'claim-literal-glean',
                        'wrong_source', 'user', 'user-fixture',
                        '{\"surface\":\"entity_detail\",\"source_ref\":\"fixture://literal-glean-source-purge\"}',
                        '2026-06-05T12:00:00Z', '2026-06-05T12:00:00Z'
                    ),
                    (
                        'feedback-literal-gleaner', 'claim-literal-gleaner',
                        'wrong_source', 'user', 'user-fixture',
                        '{\"surface\":\"entity_detail\",\"source_ref\":\"fixture://literal-gleaner-source\"}',
                        '2026-06-05T12:00:00Z', '2026-06-05T12:00:00Z'
                    );",
            )
            .expect("seed feedback rows for literal source family regression");
        db.conn_ref()
            .execute(
                "UPDATE claim_feedback SET claim_id = ?1 WHERE id = 'feedback-literal-glean'",
                params![&glean_claim_id],
            )
            .expect("bind glean feedback to committed claim");
        db.conn_ref()
            .execute(
                "UPDATE claim_feedback SET claim_id = ?1 WHERE id = 'feedback-literal-gleaner'",
                params![&sibling_claim_id],
            )
            .expect("bind sibling feedback to committed claim");
        db.conn_ref()
            .execute_batch(
                "INSERT INTO claim_feedback_correction_envelopes (
                    feedback_id, claim_id, action, actor, actor_id, surface,
                    asserted_subject_ref_json, asserted_subject_kind, asserted_subject_id,
                    field_path, data_source, source_ref, source_ref_hash, source_asof,
                    source_key_version, source_key_epoch_hash, source_key_hash,
                    claim_type, sensitivity, replay_key, action_metadata_json
                 ) VALUES
                    (
                        'feedback-literal-glean', 'claim-literal-glean',
                        'wrong_source', 'user', 'user-fixture', 'entity_detail',
                        '{\"kind\":\"account\",\"id\":\"acct-redaction\"}',
                        'account', 'acct-redaction', 'health.risk', 'glean_crm',
                        'fixture://literal-glean-source-purge', 'source-ref-hash-glean',
                        '2026-06-05T00:00:00Z', 1, 'epoch-literal',
                        'source-key-literal-glean', 'risk', 'internal',
                        'replay-literal-glean', '{}'
                    ),
                    (
                        'feedback-literal-gleaner', 'claim-literal-gleaner',
                        'wrong_source', 'user', 'user-fixture', 'entity_detail',
                        '{\"kind\":\"account\",\"id\":\"acct-redaction\"}',
                        'account', 'acct-redaction', 'health.risk', 'gleaner',
                        'fixture://literal-gleaner-source', 'source-ref-hash-gleaner',
                        '2026-06-05T00:00:00Z', 1, 'epoch-literal',
                        'source-key-literal-gleaner', 'risk', 'internal',
                        'replay-literal-gleaner', '{}'
                    );
                 INSERT INTO source_claim_type_reliability (
                    source_key_version, source_key_epoch_hash, source_key_hash,
                    data_source, source_key_kind, claim_type, signal_type,
                    alpha, beta, update_count
                 ) VALUES
                    (
                        1, 'epoch-literal', 'source-key-literal-glean',
                        'glean_crm', 'source_content_hash', 'risk',
                        'user_feedback', 1.0, 2.0, 1
                    ),
                    (
                        1, 'epoch-literal', 'source-key-literal-gleaner',
                        'gleaner', 'source_content_hash', 'risk',
                        'user_feedback', 1.0, 2.0, 1
                    );
                 INSERT INTO source_reliability_feedback_deltas (
                    id, feedback_id, source_key_version, source_key_epoch_hash,
                    source_key_hash, data_source, source_key_kind, claim_type,
                    signal_type, effect_kind, alpha_delta, beta_delta
                 ) VALUES
                    (
                        'delta-literal-glean', 'feedback-literal-glean', 1,
                        'epoch-literal', 'source-key-literal-glean',
                        'glean_crm', 'source_content_hash', 'risk',
                        'user_feedback', 'wrong_source', 0.0, 1.0
                    ),
                    (
                        'delta-literal-gleaner', 'feedback-literal-gleaner', 1,
                        'epoch-literal', 'source-key-literal-gleaner',
                        'gleaner', 'source_content_hash', 'risk',
                        'user_feedback', 'wrong_source', 0.0, 1.0
                    );",
            )
            .expect("seed correction artifacts for literal source family regression");

        let purge_report = crate::db::data_lifecycle::purge_source(
            &db,
            crate::db::data_lifecycle::DataSource::Glean,
        )
        .expect("purge Glean source");

        assert!(
            purge_report.correction_artifacts_lifecycle_marked > 0,
            "source purge must mark the literal Glean source family"
        );
        let (glean_state, sibling_state, sibling_payload, sibling_excluded_at): (
            String,
            String,
            String,
            Option<String>,
        ) = db
            .conn_ref()
            .query_row(
                "SELECT glean_envelope.lifecycle_state,
                        sibling_envelope.lifecycle_state,
                        sibling_feedback.payload_json,
                        sibling_aggregate.excluded_at
                   FROM claim_feedback_correction_envelopes glean_envelope
                   JOIN claim_feedback_correction_envelopes sibling_envelope
                     ON sibling_envelope.feedback_id = ?2
                   JOIN claim_feedback sibling_feedback
                     ON sibling_feedback.id = sibling_envelope.feedback_id
                   JOIN source_claim_type_reliability sibling_aggregate
                     ON sibling_aggregate.source_key_hash = sibling_envelope.source_key_hash
                  WHERE glean_envelope.feedback_id = ?1",
                params!["feedback-literal-glean", "feedback-literal-gleaner"],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("read literal source family purge state");

        assert_eq!(glean_state, "source_removed");
        assert_eq!(sibling_state, "active");
        assert!(
            sibling_payload.contains("literal-gleaner-source"),
            "sibling source payload must not be redacted by Glean purge"
        );
        assert_eq!(sibling_excluded_at, None);
    }

    #[test]
    fn w4_subject_deletion_scrubs_subject_refs_and_blocks_replay() {
        let db = test_db();
        seed_account(&db);
        let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 6, 5, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(7);
        let external = ExternalClients::default();
        let write_ctx = test_ctx(&clock, &rng, &external, "user:test");
        let claim_id = inserted_claim_id(commit_claim(&write_ctx, &db, proposal()).unwrap());
        let feedback = record_claim_feedback(
            &write_ctx,
            &db,
            ClaimFeedbackInput {
                claim_id,
                action: FeedbackAction::WrongSubject,
                actor: "user".to_string(),
                actor_id: Some("user-fixture".to_string()),
                payload_json: Some(
                    json!({
                        "corrected_subject_ref": {"kind": "account", "id": "account_02"}
                    })
                    .to_string(),
                ),
            },
        )
        .expect("record wrong-subject feedback");
        let subject_hash = subject_hash_for_feedback(&db, &feedback.feedback_id);
        seed_declassification_decision(&db, &feedback.feedback_id, &subject_hash);
        seed_stickiness_observation(&db, Some(&feedback.feedback_id), &subject_hash);

        let report = update_subject_lifecycle_artifacts(
            &write_ctx,
            &db,
            SubjectLifecycleArtifactsInput {
                subject_kind: "account",
                subject_id: "acct-redaction",
                reason_code: "subject_deleted_by_user",
                action: SubjectLifecycleAction::Deleted,
            },
        )
        .expect("delete subject artifacts");

        assert_eq!(report.envelopes_redacted, 1);
        assert_eq!(report.subject_deltas_redacted, 1);
        assert_eq!(report.subject_aggregates_excluded, 1);
        assert_eq!(report.declassification_decisions_revoked, 1);
        assert!(report.propagation_jobs_staled >= 1);

        let (envelope_state, subject_kind, subject_id, delta_status, aggregate_state): (
            String,
            Option<String>,
            Option<String>,
            String,
            String,
        ) = db
            .conn_ref()
            .query_row(
                "SELECT envelope.lifecycle_state,
                        envelope.asserted_subject_kind,
                        envelope.asserted_subject_id,
                        delta.status,
                        aggregate.lifecycle_state
                   FROM claim_feedback_correction_envelopes envelope
                   JOIN subject_inference_reliability_deltas delta
                     ON delta.feedback_id = envelope.feedback_id
                   JOIN subject_inference_reliability aggregate
                     ON aggregate.subject_ref_hash = delta.subject_ref_hash
                  WHERE envelope.feedback_id = ?1",
                params![&feedback.feedback_id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .expect("read subject lifecycle state");
        assert_eq!(envelope_state, "subject_deleted");
        assert_eq!(subject_kind.as_deref(), Some("subject_deleted"));
        assert_eq!(subject_id, None);
        assert_eq!(delta_status, "subject_deleted");
        assert_eq!(aggregate_state, "subject_deleted");
    }

    #[test]
    fn w4_meeting_removal_redacts_prep_journal_and_stales_regeneration() {
        let db = test_db();
        let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 6, 5, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(7);
        let external = ExternalClients::default();
        let write_ctx = test_ctx(&clock, &rng, &external, "user:test");

        db.conn_ref()
            .execute(
                "INSERT INTO meeting_prep_correction_journal (
                    id, meeting_stable_key, meeting_id, field_path, actor, surface,
                    source_asof, sensitivity, replay_key, payload_json, payload_hash
                 ) VALUES ('journal_01', 'meeting_stable_01', 'meeting_01',
                    'user_preparation_text', 'user', 'meeting_prep',
                    '2026-06-05T00:00:00Z', 'user_only', 'replay_01',
                    '{\"text\":\"fixture\"}', 'sha256:fixture')",
                [],
            )
            .expect("seed prep journal");
        db.conn_ref()
            .execute(
                "INSERT INTO meeting_prep_regeneration_jobs (
                    id, journal_id, meeting_stable_key, field_path, status,
                    coalescing_key
                 ) VALUES ('prep_job_01', 'journal_01', 'meeting_stable_01',
                    'user_preparation_text', 'pending', 'meeting_stable_01:user_preparation_text')",
                [],
            )
            .expect("seed prep job");

        let report = remove_meeting_artifacts(
            &write_ctx,
            &db,
            RemoveMeetingArtifactsInput {
                meeting_stable_key: "meeting_stable_01",
                meeting_id: None,
                reason_code: "meeting_removed_by_user",
            },
        )
        .expect("remove meeting artifacts");

        assert_eq!(report.prep_journals_redacted, 1);
        assert_eq!(report.propagation_jobs_staled, 1);

        let (journal_state, meeting_id, job_status): (String, Option<String>, String) = db
            .conn_ref()
            .query_row(
                "SELECT journal.lifecycle_state, journal.meeting_id, job.status
                   FROM meeting_prep_correction_journal journal
                   JOIN meeting_prep_regeneration_jobs job
                     ON job.journal_id = journal.id
                  WHERE journal.id = 'journal_01'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("read meeting lifecycle state");
        assert_eq!(journal_state, "meeting_removed");
        assert_eq!(meeting_id, None);
        assert_eq!(job_status, "stale");
    }

    #[test]
    fn w4_proof_purge_removes_dos338_surface_hashes() {
        let db = test_db();
        let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 6, 5, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(7);
        let external = ExternalClients::default();
        let write_ctx = test_ctx(&clock, &rng, &external, "user:test");
        let run_id = seed_stickiness_observation(&db, None, "subject_hash_01");

        let report = purge_dos338_proof_artifacts(
            &write_ctx,
            &db,
            PurgeDos338ProofInput {
                run_id: &run_id,
                reason_code: "proof_purged_by_user",
            },
        )
        .expect("purge proof artifacts");

        assert_eq!(report.dos338_runs_purged, 1);
        assert_eq!(report.dos338_observations_marked, 1);

        let (status, purge_state, report_hash, direct_surface, post_hash): (
            String,
            String,
            Option<String>,
            String,
            Option<String>,
        ) = db
            .conn_ref()
            .query_row(
                "SELECT run.status,
                        run.purge_state,
                        run.report_hash,
                        observation.direct_surface,
                        observation.post_rebuild_state_hash
                   FROM dos338_stickiness_runs run
                   JOIN dos338_stickiness_observations observation
                     ON observation.run_id = run.id
                  WHERE run.id = ?1",
                params![&run_id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .expect("read purged proof artifacts");
        assert_eq!(status, "purged");
        assert_eq!(purge_state, "purged");
        assert_eq!(report_hash, None);
        assert_eq!(direct_surface, "purged");
        assert_eq!(post_hash, None);
    }

    #[test]
    fn workspace_reset_scrubs_w4_artifacts_without_runtime_actor() {
        let db = test_db();
        seed_account(&db);
        let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 6, 5, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(7);
        let external = ExternalClients::default();
        let runtime_ctx = test_ctx(&clock, &rng, &external, "runtime:test");

        let rejected = purge_workspace_correction_artifacts(
            &runtime_ctx,
            &db,
            WorkspaceResetArtifactsInput {
                reason_code: "workspace_reset_by_user",
            },
        )
        .expect_err("runtime actor cannot purge workspace artifacts");
        assert!(matches!(
            rejected,
            CorrectionArtifactError::UnauthorizedActor(_)
        ));

        let write_ctx = test_ctx(&clock, &rng, &external, "user:test");
        let claim_id = inserted_claim_id(commit_claim(&write_ctx, &db, proposal()).unwrap());
        let feedback = record_claim_feedback(
            &write_ctx,
            &db,
            ClaimFeedbackInput {
                claim_id,
                action: FeedbackAction::WrongSource,
                actor: "user".to_string(),
                actor_id: Some("user-fixture".to_string()),
                payload_json: Some(
                    json!({
                        "surface": "entity_detail",
                        "source_ref": "fixture://workspace-reset"
                    })
                    .to_string(),
                ),
            },
        )
        .expect("record feedback");
        let run_id =
            seed_stickiness_observation(&db, Some(&feedback.feedback_id), "subject_hash_01");

        let report = purge_workspace_correction_artifacts(
            &write_ctx,
            &db,
            WorkspaceResetArtifactsInput {
                reason_code: "workspace_reset_by_user",
            },
        )
        .expect("workspace reset artifacts");

        assert!(report.workspace_rows_purged > 0);
        assert_eq!(report.dos338_runs_purged, 1);

        let (
            envelope_state,
            run_state,
            direct_before_hash,
            direct_after_hash,
            indirect_before_hash,
            indirect_after_hash,
        ): (
            String,
            String,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
        ) = db
            .conn_ref()
            .query_row(
                "SELECT envelope.lifecycle_state, run.purge_state,
                        observation.direct_surface_before_hash,
                        observation.direct_surface_after_hash,
                        observation.indirect_surface_before_hash,
                        observation.indirect_surface_after_hash
                   FROM claim_feedback_correction_envelopes envelope,
                        dos338_stickiness_runs run,
                        dos338_stickiness_observations observation
                  WHERE envelope.feedback_id = ?1
                    AND run.id = ?2
                    AND observation.run_id = run.id",
                params![&feedback.feedback_id, &run_id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                    ))
                },
            )
            .expect("read workspace reset artifacts");
        assert_eq!(envelope_state, "workspace_reset");
        assert_eq!(run_state, "purged");
        assert_eq!(direct_before_hash, None);
        assert_eq!(direct_after_hash, None);
        assert_eq!(indirect_before_hash, None);
        assert_eq!(indirect_after_hash, None);
    }
}
