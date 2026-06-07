//! Correction stickiness measurement.
//!
//! The mutation phase writes only fixture-safe hashes/ids. The read/eval
//! phase can then use ADR-0110 ability evaluation to compare rendered
//! surfaces without pretending evaluate-mode services may mutate state.

use std::sync::Arc;

use rusqlite::{params, OptionalExtension};
use serde::Serialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::abilities::feedback::FeedbackAction;
use crate::db::ActionDb;
use crate::services::claim_receipt::contracts::{ReceiptTarget, SurfaceContext};
use crate::services::context::{
    ClaimReceiptReadError, ClaimReceiptReadFuture, ClaimReceiptReadHandle, ClaimReceiptSnapshot,
    ServiceContext,
};

#[derive(Debug, thiserror::Error)]
pub enum StickinessEvalError {
    #[error("service mode rejected correction stickiness mutation: {0}")]
    Mode(String),
    #[error("database error: {0}")]
    Db(String),
    #[error("invalid correction stickiness observation: {0}")]
    InvalidObservation(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StickinessEntryPoint {
    App,
    FileProjection,
    Mcp,
}

impl StickinessEntryPoint {
    fn as_str(self) -> &'static str {
        match self {
            Self::App => "app",
            Self::FileProjection => "file_projection",
            Self::Mcp => "mcp",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StickinessObservationResult {
    Passed,
    Failed,
    BlockedByW5,
    BlockedByPrivacyGate,
}

impl StickinessObservationResult {
    fn as_str(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::BlockedByW5 => "blocked_by_w5",
            Self::BlockedByPrivacyGate => "blocked_by_privacy_gate",
        }
    }
}

#[derive(Debug, Clone)]
struct StickinessRunInput {
    fixture_id: String,
    entry_point: StickinessEntryPoint,
    observations: Vec<StickinessObservationInput>,
}

#[derive(Debug, Clone)]
struct StickinessObservationInput {
    feedback_id: Option<String>,
    action: FeedbackAction,
    subject_kind: String,
    subject_ref: String,
    direct_surface: String,
    indirect_surface: String,
    direct_surface_before_hash: Option<String>,
    direct_surface_after_hash: Option<String>,
    indirect_surface_before_hash: Option<String>,
    indirect_surface_after_hash: Option<String>,
    pre_reenrichment_state_hash: Option<String>,
    post_reenrichment_state_hash: Option<String>,
    post_rebuild_state_hash: Option<String>,
    trust_band_before: Option<String>,
    trust_band_after: Option<String>,
    recompute_job_id: Option<String>,
    repair_job_id: Option<String>,
    dead_letter_reason: Option<String>,
    sensitivity_gate_result: String,
    result: StickinessObservationResult,
    reason_code: Option<String>,
}

pub struct StickinessHarnessRunInput {
    pub fixture_id: String,
    pub entry_point: StickinessEntryPoint,
    pub snapshots: StickinessHarnessSnapshots,
    pub cases: Vec<StickinessHarnessCaseInput>,
}

#[derive(Clone)]
pub struct StickinessHarnessSnapshots {
    pub before_correction: Arc<ActionDb>,
    pub after_correction: Arc<ActionDb>,
    pub after_reenrichment: Arc<ActionDb>,
    pub after_rebuild: Arc<ActionDb>,
}

#[derive(Debug, Clone)]
pub struct StickinessHarnessCaseInput {
    pub feedback_id: String,
    pub action: FeedbackAction,
    pub subject_kind: String,
    pub subject_ref: String,
    pub direct_surface: StickinessSurfaceProbe,
    pub indirect_surface: StickinessSurfaceProbe,
}

#[derive(Debug, Clone)]
pub enum StickinessSurfaceProbe {
    ClaimReceipt { surface: SurfaceContext },
    ClaimFileProjection,
    MeetingPrepStatus { meeting_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StickinessRunReport {
    pub run_id: String,
    pub status: String,
    pub report_hash: String,
    pub passed_observations: usize,
    pub failed_observations: usize,
    pub blocked_observations: usize,
}

pub fn classify_stickiness_result(
    _entry_point: StickinessEntryPoint,
    correction_applied: bool,
    indirect_surface_changed: bool,
    direct_surface_rerendered: bool,
    re_enrichment_preserved: bool,
    rebuild_preserved: bool,
    no_forbidden_surface_leak: bool,
) -> StickinessObservationResult {
    if !no_forbidden_surface_leak {
        return StickinessObservationResult::BlockedByPrivacyGate;
    }
    if correction_applied
        && indirect_surface_changed
        && direct_surface_rerendered
        && re_enrichment_preserved
        && rebuild_preserved
    {
        StickinessObservationResult::Passed
    } else {
        StickinessObservationResult::Failed
    }
}

pub fn record_stickiness_harness_run(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    input: StickinessHarnessRunInput,
) -> Result<StickinessRunReport, StickinessEvalError> {
    ctx.check_mutation_allowed()
        .map_err(|error| StickinessEvalError::Mode(error.to_string()))?;
    if input.cases.is_empty() {
        return Err(StickinessEvalError::InvalidObservation(
            "at least one DOS-338 harness case is required".to_string(),
        ));
    }
    validate_harness_snapshots(&input.snapshots)?;

    let mut observations = Vec::with_capacity(input.cases.len());
    for case in input.cases {
        validate_harness_case(&case)?;
        let envelope = feedback_correction_envelope(db, &case.feedback_id)?;
        let Some(envelope) = envelope else {
            observations.push(missing_feedback_observation(input.entry_point, case));
            continue;
        };
        validate_case_matches_envelope(&case, &envelope)?;
        validate_harness_phase_progress(
            &input.snapshots.after_reenrichment,
            &case.feedback_id,
            "after_reenrichment",
        )?;
        validate_harness_phase_progress(
            &input.snapshots.after_rebuild,
            &case.feedback_id,
            "after_rebuild",
        )?;
        validate_rebuild_surface_proof(
            &input.snapshots.after_rebuild,
            &case.feedback_id,
            &case.indirect_surface,
            &envelope,
        )?;

        let proof = capture_harness_proof(&input.snapshots, &case, &envelope)?;
        let correction_applied = envelope.lifecycle_state == "active";
        let direct_surface_rerendered =
            proof.direct_before.state_hash != proof.direct_after.state_hash;
        let indirect_surface_changed =
            proof.indirect_before.state_hash != proof.indirect_after.state_hash;
        let re_enrichment_preserved =
            proof.post_reenrichment.state_hash == proof.indirect_after.state_hash;
        let rebuild_preserved = proof.post_rebuild.state_hash == proof.post_reenrichment.state_hash;
        let no_forbidden_surface_leak = !proof.privacy_blocked;
        let result = classify_stickiness_result(
            input.entry_point,
            correction_applied,
            indirect_surface_changed,
            direct_surface_rerendered,
            re_enrichment_preserved,
            rebuild_preserved,
            no_forbidden_surface_leak,
        );
        let reason_code = harness_reason_code(
            result,
            correction_applied,
            indirect_surface_changed,
            direct_surface_rerendered,
            re_enrichment_preserved,
            rebuild_preserved,
            no_forbidden_surface_leak,
        );
        let job_proof = feedback_job_proof(db, &case.feedback_id)?;
        observations.push(StickinessObservationInput {
            feedback_id: Some(case.feedback_id),
            action: case.action,
            subject_kind: case.subject_kind,
            subject_ref: case.subject_ref,
            direct_surface: proof.direct_before.surface_name,
            indirect_surface: proof.indirect_before.surface_name,
            direct_surface_before_hash: Some(proof.direct_before.state_hash),
            direct_surface_after_hash: Some(proof.direct_after.state_hash),
            indirect_surface_before_hash: Some(proof.indirect_before.state_hash),
            indirect_surface_after_hash: Some(proof.indirect_after.state_hash.clone()),
            pre_reenrichment_state_hash: Some(proof.indirect_after.state_hash),
            post_reenrichment_state_hash: Some(proof.post_reenrichment.state_hash),
            post_rebuild_state_hash: Some(proof.post_rebuild.state_hash),
            trust_band_before: proof.direct_before.trust_band,
            trust_band_after: proof.direct_after.trust_band,
            recompute_job_id: job_proof.recompute_job_id,
            repair_job_id: job_proof.repair_job_id,
            dead_letter_reason: job_proof.dead_letter_reason,
            sensitivity_gate_result: proof.sensitivity_gate_result,
            result,
            reason_code,
        });
    }

    record_stickiness_run(
        ctx,
        db,
        StickinessRunInput {
            fixture_id: input.fixture_id,
            entry_point: input.entry_point,
            observations,
        },
    )
}

fn validate_harness_snapshots(
    snapshots: &StickinessHarnessSnapshots,
) -> Result<(), StickinessEvalError> {
    let phases = [
        ("before_correction", &snapshots.before_correction),
        ("after_correction", &snapshots.after_correction),
        ("after_reenrichment", &snapshots.after_reenrichment),
        ("after_rebuild", &snapshots.after_rebuild),
    ];
    for (left_index, (left_name, left_db)) in phases.iter().enumerate() {
        for (right_name, right_db) in phases.iter().skip(left_index + 1) {
            if Arc::ptr_eq(left_db, right_db) {
                return Err(StickinessEvalError::InvalidObservation(format!(
                    "DOS-338 harness phase {left_name} must use a distinct service snapshot from {right_name}"
                )));
            }
        }
    }
    Ok(())
}

fn validate_harness_phase_progress(
    db: &Arc<ActionDb>,
    feedback_id: &str,
    phase: &str,
) -> Result<(), StickinessEvalError> {
    let (direct_total, direct_terminal, direct_active): (i64, i64, i64) = db
        .conn_ref()
        .query_row(
            "SELECT
                count(*),
                COALESCE(sum(CASE WHEN status IN ('completed', 'stale', 'dead_lettered') THEN 1 ELSE 0 END), 0),
                COALESCE(sum(CASE WHEN status IN ('pending', 'running', 'coalesced') THEN 1 ELSE 0 END), 0)
               FROM claim_feedback_propagation_jobs
              WHERE feedback_id = ?1",
            params![feedback_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(|error| StickinessEvalError::Db(error.to_string()))?;
    let (coalesced_total, coalesced_terminal, coalesced_active): (i64, i64, i64) = db
        .conn_ref()
        .query_row(
            "SELECT
                count(*),
                COALESCE(sum(CASE WHEN parent.status IN ('completed', 'stale', 'dead_lettered') THEN 1 ELSE 0 END), 0),
                COALESCE(sum(CASE WHEN parent.status IN ('pending', 'running', 'coalesced') THEN 1 ELSE 0 END), 0)
               FROM claim_feedback_propagation_outcomes outcome
               JOIN claim_feedback_propagation_jobs parent ON parent.id = outcome.job_id
              WHERE outcome.feedback_id = ?1
                AND outcome.status = 'coalesced'",
            params![feedback_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(|error| StickinessEvalError::Db(error.to_string()))?;
    let total = direct_total + coalesced_total;
    let terminal = direct_terminal + coalesced_terminal;
    let active = direct_active + coalesced_active;
    if total == 0 {
        return Err(StickinessEvalError::InvalidObservation(format!(
            "DOS-338 harness phase {phase} missing feedback propagation proof"
        )));
    }
    if active > 0 {
        return Err(StickinessEvalError::InvalidObservation(format!(
            "DOS-338 harness phase {phase} has unprocessed feedback propagation jobs"
        )));
    }
    if terminal == 0 {
        return Err(StickinessEvalError::InvalidObservation(format!(
            "DOS-338 harness phase {phase} has no terminal feedback propagation jobs"
        )));
    }
    Ok(())
}

fn validate_rebuild_surface_proof(
    db: &Arc<ActionDb>,
    feedback_id: &str,
    probe: &StickinessSurfaceProbe,
    envelope: &FeedbackCorrectionEnvelope,
) -> Result<(), StickinessEvalError> {
    match probe {
        StickinessSurfaceProbe::ClaimFileProjection => {
            let rebuilt_count: i64 = db
                .conn_ref()
                .query_row(
                    "SELECT count(*)
                       FROM claim_file_projection_runs run
                       JOIN claim_file_projection_run_claims run_claim
                         ON run_claim.run_id = run.id
                       JOIN claim_feedback feedback
                         ON feedback.id = ?3
                      WHERE run.entity_subject_ref_json = ?1
                        AND run_claim.claim_id = ?2
                        AND run.status IN ('committed', 'repaired')
                        AND run.succeeded_at IS NOT NULL
                        AND datetime(run.attempted_at) >= datetime(COALESCE(feedback.applied_at, feedback.submitted_at))
                        AND datetime(run.succeeded_at) >= datetime(COALESCE(feedback.applied_at, feedback.submitted_at))",
                    params![
                        &envelope.asserted_subject_ref_json,
                        &envelope.claim_id,
                        feedback_id,
                    ],
                    |row| row.get(0),
                )
                .map_err(|error| StickinessEvalError::Db(error.to_string()))?;
            if rebuilt_count == 0 {
                return Err(StickinessEvalError::InvalidObservation(
                    "DOS-338 harness after_rebuild missing post-feedback claim file projection rebuild proof"
                        .to_string(),
                ));
            }
        }
        StickinessSurfaceProbe::MeetingPrepStatus { meeting_id } => {
            let replayed_count: i64 = db
                .conn_ref()
                .query_row(
                    "SELECT count(*)
                       FROM meeting_prep_correction_journal journal
                       JOIN claim_feedback feedback
                         ON feedback.id = ?2
                      WHERE journal.meeting_id = ?1
                        AND journal.replayed_at IS NOT NULL
                        AND journal.replay_attempt_count > 0
                        AND journal.rebuild_replay_id IS NOT NULL
                        AND datetime(journal.replayed_at) >= datetime(COALESCE(feedback.applied_at, feedback.submitted_at))",
                    params![meeting_id, feedback_id],
                    |row| row.get(0),
                )
                .map_err(|error| StickinessEvalError::Db(error.to_string()))?;
            if replayed_count == 0 {
                return Err(StickinessEvalError::InvalidObservation(
                    "DOS-338 harness after_rebuild missing post-feedback meeting prep replay proof"
                        .to_string(),
                ));
            }
        }
        StickinessSurfaceProbe::ClaimReceipt { .. } => {}
    }
    Ok(())
}

fn record_stickiness_run(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    input: StickinessRunInput,
) -> Result<StickinessRunReport, StickinessEvalError> {
    ctx.check_mutation_allowed()
        .map_err(|error| StickinessEvalError::Mode(error.to_string()))?;
    validate_run_input(&input)?;

    let now = ctx.clock.now().to_rfc3339();
    let run_id = Uuid::new_v4().to_string();
    let fixture_id_hash = pii_safe_hash(
        "dos338_fixture",
        "dailyos.w4.dos338.fixture",
        &[&input.fixture_id],
    )?;
    let counts = observation_counts(&input.observations);
    let status = if counts.failed > 0 {
        "failed"
    } else {
        "completed"
    };
    let reason_code = if counts.failed > 0 {
        Some("stickiness_gate_failed")
    } else if counts.blocked > 0 {
        Some("cross_wave_or_privacy_blocked")
    } else {
        None
    };
    let report_hash = stable_json_hash(&json!({
        "fixture_id_hash": fixture_id_hash,
        "entry_point": input.entry_point.as_str(),
        "observations": input.observations.len(),
        "passed": counts.passed,
        "failed": counts.failed,
        "blocked": counts.blocked,
        "status": status,
        "reason_code": reason_code,
    }))?;

    db.with_transaction(|tx| {
        tx.conn_ref()
            .execute(
                "INSERT INTO dos338_stickiness_runs (
                    id, fixture_id_hash, entry_point, status, report_hash,
                    reason_code, started_at, completed_at, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7, ?7, ?7)",
                params![
                    &run_id,
                    &fixture_id_hash,
                    input.entry_point.as_str(),
                    status,
                    &report_hash,
                    reason_code,
                    &now,
                ],
            )
            .map_err(|error| error.to_string())?;

        for observation in &input.observations {
            let observation_id = Uuid::new_v4().to_string();
            let subject_ref_hash = pii_safe_hash(
                "dos338_subject",
                "dailyos.w4.dos338.subject",
                &[&observation.subject_kind, &observation.subject_ref],
            )
            .map_err(|error| error.to_string())?;
            tx.conn_ref()
                .execute(
                    "INSERT INTO dos338_stickiness_observations (
	                        id, run_id, feedback_id, entry_point, action, subject_kind,
	                        subject_ref_hash, direct_surface, indirect_surface,
	                        direct_surface_before_hash, direct_surface_after_hash,
	                        indirect_surface_before_hash, indirect_surface_after_hash,
	                        pre_reenrichment_state_hash, post_reenrichment_state_hash,
	                        post_rebuild_state_hash, trust_band_before, trust_band_after,
	                        recompute_job_id, repair_job_id, dead_letter_reason,
	                        sensitivity_gate_result, result, reason_code, observed_at
	                    ) VALUES (
	                        ?1, ?2, ?3, ?4, ?5, ?6,
	                        ?7, ?8, ?9,
	                        ?10, ?11,
	                        ?12, ?13,
	                        ?14, ?15,
	                        ?16, ?17, ?18,
	                        ?19, ?20, ?21,
	                        ?22, ?23, ?24, ?25
	                    )",
                    params![
                        observation_id,
                        &run_id,
                        observation.feedback_id.as_deref(),
                        input.entry_point.as_str(),
                        observation.action.as_str(),
                        &observation.subject_kind,
                        subject_ref_hash,
                        &observation.direct_surface,
                        &observation.indirect_surface,
                        observation.direct_surface_before_hash.as_deref(),
                        observation.direct_surface_after_hash.as_deref(),
                        observation.indirect_surface_before_hash.as_deref(),
                        observation.indirect_surface_after_hash.as_deref(),
                        observation.pre_reenrichment_state_hash.as_deref(),
                        observation.post_reenrichment_state_hash.as_deref(),
                        observation.post_rebuild_state_hash.as_deref(),
                        observation.trust_band_before.as_deref(),
                        observation.trust_band_after.as_deref(),
                        observation.recompute_job_id.as_deref(),
                        observation.repair_job_id.as_deref(),
                        observation.dead_letter_reason.as_deref(),
                        &observation.sensitivity_gate_result,
                        observation.result.as_str(),
                        observation.reason_code.as_deref(),
                        &now,
                    ],
                )
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    })
    .map_err(StickinessEvalError::Db)?;

    Ok(StickinessRunReport {
        run_id,
        status: status.to_string(),
        report_hash,
        passed_observations: counts.passed,
        failed_observations: counts.failed,
        blocked_observations: counts.blocked,
    })
}

fn validate_harness_case(case: &StickinessHarnessCaseInput) -> Result<(), StickinessEvalError> {
    let required = [
        ("feedback_id", case.feedback_id.as_str()),
        ("subject_kind", case.subject_kind.as_str()),
        ("subject_ref", case.subject_ref.as_str()),
    ];
    for (field, value) in required {
        if value.trim().is_empty() {
            return Err(StickinessEvalError::InvalidObservation(format!(
                "{field} is required"
            )));
        }
    }
    validate_surface_probe(&case.direct_surface, "direct_surface")?;
    validate_surface_probe(&case.indirect_surface, "indirect_surface")?;
    Ok(())
}

fn validate_surface_probe(
    probe: &StickinessSurfaceProbe,
    field_name: &str,
) -> Result<(), StickinessEvalError> {
    if let StickinessSurfaceProbe::MeetingPrepStatus { meeting_id } = probe {
        if meeting_id.trim().is_empty() {
            return Err(StickinessEvalError::InvalidObservation(format!(
                "{field_name}.meeting_id is required"
            )));
        }
    }
    Ok(())
}

#[derive(Debug, Clone)]
struct FeedbackCorrectionEnvelope {
    claim_id: String,
    action: FeedbackAction,
    asserted_subject_ref_json: String,
    asserted_subject_kind: Option<String>,
    asserted_subject_id: Option<String>,
    field_path: Option<String>,
    sensitivity: String,
    lifecycle_state: String,
}

#[derive(Debug, Clone)]
struct SurfaceSnapshot {
    surface_name: String,
    state_hash: String,
    trust_band: Option<String>,
    privacy_blocked: bool,
}

#[derive(Debug, Clone)]
struct HarnessProof {
    direct_before: SurfaceSnapshot,
    direct_after: SurfaceSnapshot,
    indirect_before: SurfaceSnapshot,
    indirect_after: SurfaceSnapshot,
    post_reenrichment: SurfaceSnapshot,
    post_rebuild: SurfaceSnapshot,
    sensitivity_gate_result: String,
    privacy_blocked: bool,
}

#[derive(Debug, Clone, Default)]
struct FeedbackJobProof {
    recompute_job_id: Option<String>,
    repair_job_id: Option<String>,
    dead_letter_reason: Option<String>,
}

fn feedback_correction_envelope(
    db: &ActionDb,
    feedback_id: &str,
) -> Result<Option<FeedbackCorrectionEnvelope>, StickinessEvalError> {
    db.conn_ref()
        .query_row(
            "SELECT claim_id, action, asserted_subject_ref_json,
                    asserted_subject_kind, asserted_subject_id, field_path,
                    sensitivity, lifecycle_state
               FROM claim_feedback_correction_envelopes
              WHERE feedback_id = ?1",
            params![feedback_id],
            |row| {
                let action_raw: String = row.get(1)?;
                let action = serde_json::from_value::<FeedbackAction>(json!(action_raw)).map_err(
                    |error| {
                        rusqlite::Error::FromSqlConversionFailure(
                            1,
                            rusqlite::types::Type::Text,
                            Box::new(error),
                        )
                    },
                )?;
                Ok(FeedbackCorrectionEnvelope {
                    claim_id: row.get(0)?,
                    action,
                    asserted_subject_ref_json: row.get(2)?,
                    asserted_subject_kind: row.get(3)?,
                    asserted_subject_id: row.get(4)?,
                    field_path: row.get(5)?,
                    sensitivity: row.get(6)?,
                    lifecycle_state: row.get(7)?,
                })
            },
        )
        .optional()
        .map_err(|error| StickinessEvalError::Db(error.to_string()))
}

fn validate_case_matches_envelope(
    case: &StickinessHarnessCaseInput,
    envelope: &FeedbackCorrectionEnvelope,
) -> Result<(), StickinessEvalError> {
    if case.action != envelope.action {
        return Err(StickinessEvalError::InvalidObservation(format!(
            "case action {} does not match feedback envelope action {}",
            case.action.as_str(),
            envelope.action.as_str()
        )));
    }
    let (envelope_kind, envelope_id) = envelope_subject_kind_id(envelope)?;
    if normalize_subject_kind(&case.subject_kind) != envelope_kind {
        return Err(StickinessEvalError::InvalidObservation(format!(
            "case subject kind {} does not match feedback envelope subject kind {}",
            case.subject_kind, envelope_kind
        )));
    }
    if case.subject_ref.trim() != envelope_id
        && case.subject_ref.trim() != envelope.asserted_subject_ref_json.trim()
    {
        return Err(StickinessEvalError::InvalidObservation(
            "case subject ref does not match feedback envelope subject".to_string(),
        ));
    }
    Ok(())
}

fn capture_harness_proof(
    snapshots: &StickinessHarnessSnapshots,
    case: &StickinessHarnessCaseInput,
    envelope: &FeedbackCorrectionEnvelope,
) -> Result<HarnessProof, StickinessEvalError> {
    let direct_before =
        capture_surface_snapshot(&snapshots.before_correction, &case.direct_surface, envelope)?;
    let direct_after =
        capture_surface_snapshot(&snapshots.after_correction, &case.direct_surface, envelope)?;
    let indirect_before = capture_surface_snapshot(
        &snapshots.before_correction,
        &case.indirect_surface,
        envelope,
    )?;
    let indirect_after = capture_surface_snapshot(
        &snapshots.after_correction,
        &case.indirect_surface,
        envelope,
    )?;
    let post_reenrichment = capture_surface_snapshot(
        &snapshots.after_reenrichment,
        &case.indirect_surface,
        envelope,
    )?;
    let post_rebuild =
        capture_surface_snapshot(&snapshots.after_rebuild, &case.indirect_surface, envelope)?;
    let privacy_blocked = [
        &direct_before,
        &direct_after,
        &indirect_before,
        &indirect_after,
        &post_reenrichment,
        &post_rebuild,
    ]
    .iter()
    .any(|snapshot| snapshot.privacy_blocked);
    let sensitivity_gate_result = if privacy_blocked {
        "blocked_by_privacy_gate"
    } else if envelope.sensitivity == "user_only" || envelope.sensitivity == "confidential" {
        "local_only_allowed"
    } else {
        "render_allowed"
    }
    .to_string();

    Ok(HarnessProof {
        direct_before,
        direct_after,
        indirect_before,
        indirect_after,
        post_reenrichment,
        post_rebuild,
        sensitivity_gate_result,
        privacy_blocked,
    })
}

fn capture_surface_snapshot(
    db: &Arc<ActionDb>,
    probe: &StickinessSurfaceProbe,
    envelope: &FeedbackCorrectionEnvelope,
) -> Result<SurfaceSnapshot, StickinessEvalError> {
    match probe {
        StickinessSurfaceProbe::ClaimReceipt { surface } => {
            capture_claim_receipt_snapshot(Arc::clone(db), *surface, probe, envelope)
        }
        StickinessSurfaceProbe::ClaimFileProjection => {
            let snapshot = crate::services::claim_files::render_entity_claim_file_eval_snapshot_db(
                db.as_ref(),
                &envelope.asserted_subject_ref_json,
            )
            .map_err(StickinessEvalError::Db)?;
            Ok(SurfaceSnapshot {
                surface_name: surface_probe_name(probe)?,
                state_hash: stable_serialize_hash(&snapshot)?,
                trust_band: None,
                privacy_blocked: false,
            })
        }
        StickinessSurfaceProbe::MeetingPrepStatus { meeting_id } => {
            let snapshot =
                crate::services::meeting_prep_status::read::compute_status(meeting_id, db.as_ref())
                    .map_err(|error| {
                        StickinessEvalError::InvalidObservation(format!(
                            "meeting prep status capture failed: {error}"
                        ))
                    })?;
            Ok(SurfaceSnapshot {
                surface_name: surface_probe_name(probe)?,
                state_hash: stable_serialize_hash(&snapshot)?,
                trust_band: None,
                privacy_blocked: false,
            })
        }
    }
}

struct StaticClaimReceiptEvalReader {
    result: Result<ClaimReceiptSnapshot, ClaimReceiptReadError>,
}

impl ClaimReceiptReadHandle for StaticClaimReceiptEvalReader {
    fn read_claim_receipt<'a>(
        &'a self,
        _target: crate::services::context::ClaimReceiptTarget,
        _surface: crate::services::context::ClaimReceiptSurfaceContext,
    ) -> ClaimReceiptReadFuture<'a> {
        let result = self.result.clone();
        Box::pin(async move { result })
    }
}

fn capture_claim_receipt_snapshot(
    db: Arc<ActionDb>,
    surface: SurfaceContext,
    probe: &StickinessSurfaceProbe,
    envelope: &FeedbackCorrectionEnvelope,
) -> Result<SurfaceSnapshot, StickinessEvalError> {
    let app_target = receipt_target_for_envelope(envelope)?;
    let target = crate::services::context::app_target_to_ability(&app_target);
    let app_surface = surface;
    let surface = crate::services::context::app_surface_to_ability(app_surface);
    let result = match crate::services::claim_receipt::render::render_receipt_for_db(
        db.as_ref(),
        app_target,
        app_surface,
    ) {
        Ok(receipt) => Ok(crate::services::context::app_receipt_to_ability(receipt)),
        Err(crate::services::claim_receipt::render::RenderError::TargetNotFound) => {
            Err(ClaimReceiptReadError::TargetNotFound)
        }
        Err(crate::services::claim_receipt::render::RenderError::PrivacyDrop) => {
            Err(ClaimReceiptReadError::PrivacyDrop)
        }
        Err(error) => Err(ClaimReceiptReadError::ReadFailed(error.to_string())),
    };
    let clock = crate::services::context::SystemClock;
    let rng = crate::services::context::SeedableRng::new(338);
    let ctx = ServiceContext::new_evaluate_default(&clock, &rng)
        .with_actor("eval:dos338_stickiness")
        .with_claim_receipt_reader(Arc::new(StaticClaimReceiptEvalReader { result }));
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| StickinessEvalError::Db(format!("claim receipt eval runtime: {error}")))?;
    match runtime.block_on(ctx.read_claim_receipt(target, surface)) {
        Ok(receipt) => {
            let trust_band = serialize_label(&receipt.trust.band)?;
            Ok(SurfaceSnapshot {
                surface_name: surface_probe_name(probe)?,
                state_hash: stable_serialize_hash(&receipt)?,
                trust_band: Some(trust_band),
                privacy_blocked: false,
            })
        }
        Err(ClaimReceiptReadError::PrivacyDrop) => Ok(SurfaceSnapshot {
            surface_name: surface_probe_name(probe)?,
            state_hash: stable_json_hash(&json!({
                "surface": "claim_receipt",
                "surface_context": surface_context_label(app_surface)?,
                "privacy": "drop"
            }))?,
            trust_band: None,
            privacy_blocked: true,
        }),
        Err(error) => Err(StickinessEvalError::InvalidObservation(format!(
            "claim receipt eval capture failed: {error}"
        ))),
    }
}

fn feedback_job_proof(
    db: &ActionDb,
    feedback_id: &str,
) -> Result<FeedbackJobProof, StickinessEvalError> {
    let recompute_job_id = db
        .conn_ref()
        .query_row(
            "SELECT id
               FROM claim_feedback_propagation_jobs
              WHERE feedback_id = ?1
                AND target_kind = 'claim_recompute'
              ORDER BY created_at ASC, id ASC
              LIMIT 1",
            params![feedback_id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| StickinessEvalError::Db(error.to_string()))?;
    let repair_job_id = db
        .conn_ref()
        .query_row(
            "SELECT json_extract(scope_json, '$.repair_job_id')
               FROM claim_feedback_propagation_jobs
              WHERE feedback_id = ?1
                AND target_kind = 'targeted_repair'
                AND json_extract(scope_json, '$.repair_job_id') IS NOT NULL
              ORDER BY created_at ASC, id ASC
              LIMIT 1",
            params![feedback_id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| StickinessEvalError::Db(error.to_string()))?;
    let dead_letter_reason = db
        .conn_ref()
        .query_row(
            "SELECT COALESCE(failure_reason_code, stale_reason, 'dead_lettered')
               FROM claim_feedback_propagation_jobs
              WHERE feedback_id = ?1
                AND status = 'dead_lettered'
              ORDER BY updated_at DESC, id DESC
              LIMIT 1",
            params![feedback_id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| StickinessEvalError::Db(error.to_string()))?;
    Ok(FeedbackJobProof {
        recompute_job_id,
        repair_job_id,
        dead_letter_reason,
    })
}

fn missing_feedback_observation(
    entry_point: StickinessEntryPoint,
    case: StickinessHarnessCaseInput,
) -> StickinessObservationInput {
    let result = classify_stickiness_result(entry_point, false, false, false, false, false, true);
    StickinessObservationInput {
        feedback_id: Some(case.feedback_id),
        action: case.action,
        subject_kind: case.subject_kind,
        subject_ref: case.subject_ref,
        direct_surface: surface_probe_name(&case.direct_surface)
            .unwrap_or_else(|_| "unknown_direct_surface".to_string()),
        indirect_surface: surface_probe_name(&case.indirect_surface)
            .unwrap_or_else(|_| "unknown_indirect_surface".to_string()),
        direct_surface_before_hash: None,
        direct_surface_after_hash: None,
        indirect_surface_before_hash: None,
        indirect_surface_after_hash: None,
        pre_reenrichment_state_hash: None,
        post_reenrichment_state_hash: None,
        post_rebuild_state_hash: None,
        trust_band_before: None,
        trust_band_after: None,
        recompute_job_id: None,
        repair_job_id: None,
        dead_letter_reason: None,
        sensitivity_gate_result: "not_evaluated_missing_feedback_envelope".to_string(),
        result,
        reason_code: Some("correction_not_applied".to_string()),
    }
}

fn receipt_target_for_envelope(
    envelope: &FeedbackCorrectionEnvelope,
) -> Result<ReceiptTarget, StickinessEvalError> {
    let (subject_kind, subject_id) = envelope_subject_kind_id(envelope)?;
    Ok(ReceiptTarget::Claim {
        claim_id: envelope.claim_id.clone(),
        subject: receipt_subject_ref(&subject_kind, &subject_id),
        field_path: envelope.field_path.clone(),
    })
}

fn envelope_subject_kind_id(
    envelope: &FeedbackCorrectionEnvelope,
) -> Result<(String, String), StickinessEvalError> {
    if let (Some(kind), Some(id)) = (
        envelope.asserted_subject_kind.as_deref(),
        envelope.asserted_subject_id.as_deref(),
    ) {
        return Ok((normalize_subject_kind(kind), id.to_string()));
    }
    let value: serde_json::Value = serde_json::from_str(&envelope.asserted_subject_ref_json)
        .map_err(|error| {
            StickinessEvalError::InvalidObservation(format!(
                "invalid feedback envelope subject: {error}"
            ))
        })?;
    let kind = value
        .get("kind")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            StickinessEvalError::InvalidObservation(
                "feedback envelope subject kind is missing".to_string(),
            )
        })?;
    let id = value
        .get("id")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            StickinessEvalError::InvalidObservation(
                "feedback envelope subject id is missing".to_string(),
            )
        })?;
    Ok((normalize_subject_kind(kind), id.to_string()))
}

fn receipt_subject_ref(
    subject_kind: &str,
    subject_id: &str,
) -> abilities_runtime::abilities::provenance::SubjectRef {
    use abilities_runtime::abilities::provenance::SubjectRef;
    match normalize_subject_kind(subject_kind).as_str() {
        "account" => SubjectRef::Account(subject_id.to_string()),
        "project" => SubjectRef::Project(subject_id.to_string()),
        "person" => SubjectRef::Person(subject_id.to_string()),
        "action" => SubjectRef::Action(subject_id.to_string()),
        "meeting" => SubjectRef::Meeting(subject_id.to_string()),
        "user" => SubjectRef::User(subject_id.to_string()),
        "global" => SubjectRef::Global,
        _ => SubjectRef::Unknown,
    }
}

fn normalize_subject_kind(kind: &str) -> String {
    kind.trim().to_ascii_lowercase()
}

fn surface_probe_name(probe: &StickinessSurfaceProbe) -> Result<String, StickinessEvalError> {
    match probe {
        StickinessSurfaceProbe::ClaimReceipt { surface } => Ok(format!(
            "claim_receipt:{}",
            surface_context_label(*surface)?
        )),
        StickinessSurfaceProbe::ClaimFileProjection => Ok("claim_file_projection".to_string()),
        StickinessSurfaceProbe::MeetingPrepStatus { .. } => Ok("meeting_prep_status".to_string()),
    }
}

fn surface_context_label(surface: SurfaceContext) -> Result<String, StickinessEvalError> {
    serialize_label(&surface)
}

fn serialize_label<T: Serialize>(value: &T) -> Result<String, StickinessEvalError> {
    match serde_json::to_value(value).map_err(|error| StickinessEvalError::Db(error.to_string()))? {
        serde_json::Value::String(label) => Ok(label),
        other => serde_json::to_string(&other)
            .map_err(|error| StickinessEvalError::Db(error.to_string())),
    }
}

fn stable_serialize_hash<T: Serialize>(value: &T) -> Result<String, StickinessEvalError> {
    let value =
        serde_json::to_value(value).map_err(|error| StickinessEvalError::Db(error.to_string()))?;
    stable_json_hash(&value)
}

fn harness_reason_code(
    result: StickinessObservationResult,
    correction_applied: bool,
    indirect_surface_changed: bool,
    direct_surface_rerendered: bool,
    re_enrichment_preserved: bool,
    rebuild_preserved: bool,
    no_forbidden_surface_leak: bool,
) -> Option<String> {
    match result {
        StickinessObservationResult::Passed => None,
        StickinessObservationResult::BlockedByW5 => Some("blocked_by_w5".to_string()),
        StickinessObservationResult::BlockedByPrivacyGate => {
            Some("blocked_by_privacy_gate".to_string())
        }
        StickinessObservationResult::Failed => {
            if !correction_applied {
                Some("correction_not_applied".to_string())
            } else if !direct_surface_rerendered {
                Some("direct_surface_not_rerendered".to_string())
            } else if !indirect_surface_changed {
                Some("indirect_surface_not_changed".to_string())
            } else if !re_enrichment_preserved {
                Some("reenrichment_not_preserved".to_string())
            } else if !rebuild_preserved {
                Some("rebuild_not_preserved".to_string())
            } else if !no_forbidden_surface_leak {
                Some("forbidden_surface_leak".to_string())
            } else {
                Some("stickiness_gate_failed".to_string())
            }
        }
    }
}

fn validate_run_input(input: &StickinessRunInput) -> Result<(), StickinessEvalError> {
    if input.observations.is_empty() {
        return Err(StickinessEvalError::InvalidObservation(
            "at least one DOS-338 observation is required".to_string(),
        ));
    }
    for observation in &input.observations {
        if observation.direct_surface.trim().is_empty()
            || observation.indirect_surface.trim().is_empty()
            || observation.subject_kind.trim().is_empty()
            || observation.subject_ref.trim().is_empty()
            || observation.sensitivity_gate_result.trim().is_empty()
        {
            return Err(StickinessEvalError::InvalidObservation(
                "surface, subject, and sensitivity gate fields are required".to_string(),
            ));
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy)]
struct ObservationCounts {
    passed: usize,
    failed: usize,
    blocked: usize,
}

fn observation_counts(observations: &[StickinessObservationInput]) -> ObservationCounts {
    let mut counts = ObservationCounts {
        passed: 0,
        failed: 0,
        blocked: 0,
    };
    for observation in observations {
        match observation.result {
            StickinessObservationResult::Passed => counts.passed += 1,
            StickinessObservationResult::Failed => counts.failed += 1,
            StickinessObservationResult::BlockedByW5
            | StickinessObservationResult::BlockedByPrivacyGate => counts.blocked += 1,
        }
    }
    counts
}

fn stable_json_hash(value: &serde_json::Value) -> Result<String, StickinessEvalError> {
    let raw =
        serde_json::to_string(value).map_err(|error| StickinessEvalError::Db(error.to_string()))?;
    Ok(format!(
        "sha256:{}",
        hex::encode(Sha256::digest(raw.as_bytes()))
    ))
}

fn pii_safe_hash(
    prefix: &str,
    domain: &str,
    components: &[&str],
) -> Result<String, StickinessEvalError> {
    #[cfg(test)]
    {
        Ok(crate::db::local_db_keyed_audit_tag_for_tests(
            "w4-dos338-stickiness-test-secret",
            prefix,
            domain,
            components,
        ))
    }

    #[cfg(not(test))]
    {
        crate::db::local_db_keyed_audit_tag(prefix, domain, components).map_err(|error| {
            StickinessEvalError::Db(format!("derive DOS-338 PII-safe hash: {error}"))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use rusqlite::{params, Connection};

    use crate::db::test_utils::test_db;
    use crate::db::DbAccount;
    use crate::services::claims::{
        commit_claim, record_claim_feedback, ClaimFeedbackInput, ClaimProposal, CommittedClaim,
        DeterministicInsertProposal,
    };
    use crate::services::context::{ExternalClients, FixedClock, SeedableRng};

    fn live_ctx<'a>(
        clock: &'a crate::services::context::SystemClock,
        rng: &'a crate::services::context::SystemRng,
        external: &'a crate::services::context::ExternalClients,
    ) -> ServiceContext<'a> {
        ServiceContext::new_live(clock, rng, external).with_actor("user:test")
    }

    fn test_ctx<'a>(
        clock: &'a FixedClock,
        rng: &'a SeedableRng,
        external: &'a ExternalClients,
    ) -> ServiceContext<'a> {
        ServiceContext::test_live(clock, rng, external).with_actor("user:test")
    }

    fn seed_account(db: &ActionDb) {
        db.upsert_account(&DbAccount {
            id: "account_01".to_string(),
            name: "Account 01".to_string(),
            updated_at: "2026-06-05T00:00:00Z".to_string(),
            ..Default::default()
        })
        .expect("seed account");
    }

    fn proposal() -> ClaimProposal {
        ClaimProposal {
            id: None,
            expected_claim_version: None,
            subject_ref: json!({"kind": "account", "id": "account_01"}).to_string(),
            claim_type: "risk".to_string(),
            field_path: Some("health.risk".to_string()),
            topic_key: None,
            text: "Synthetic risk claim".to_string(),
            actor: "agent:test".to_string(),
            data_source: "unit_test".to_string(),
            source_ref: Some("fixture://dos338-source".to_string()),
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

    fn fixture_ctx() -> ServiceContext<'static> {
        let clock = Box::leak(Box::new(FixedClock::new(
            Utc.with_ymd_and_hms(2026, 6, 5, 12, 0, 0).unwrap(),
        )));
        let rng = Box::leak(Box::new(SeedableRng::new(7)));
        let external = Box::leak(Box::new(ExternalClients::default()));
        test_ctx(clock, rng, external)
    }

    fn seed_fixture_claim(db: &ActionDb, ctx: &ServiceContext<'_>) -> String {
        seed_account(db);
        inserted_claim_id(
            commit_claim(
                ctx,
                db,
                DeterministicInsertProposal::new("claim_dos338_01".to_string(), proposal()),
            )
            .unwrap(),
        )
    }

    fn record_fixture_feedback_with_action(
        db: &ActionDb,
        action: FeedbackAction,
    ) -> (ServiceContext<'static>, String) {
        let ctx = fixture_ctx();
        let claim_id = seed_fixture_claim(db, &ctx);
        let feedback = record_claim_feedback(
            &ctx,
            db,
            ClaimFeedbackInput {
                claim_id,
                action,
                actor: "user".to_string(),
                actor_id: Some("user-fixture".to_string()),
                payload_json: Some(json!({"surface": "claim_receipt"}).to_string()),
            },
        )
        .expect("record fixture feedback");
        (ctx, feedback.feedback_id)
    }

    fn uncorrected_snapshot() -> Arc<ActionDb> {
        let db = Arc::new(test_db());
        let ctx = fixture_ctx();
        seed_fixture_claim(&db, &ctx);
        db
    }

    fn corrected_snapshot(
        action: FeedbackAction,
    ) -> (Arc<ActionDb>, ServiceContext<'static>, String) {
        let db = Arc::new(test_db());
        let (ctx, feedback_id) = record_fixture_feedback_with_action(&db, action);
        (db, ctx, feedback_id)
    }

    fn cloned_snapshot(source: &Arc<ActionDb>) -> Arc<ActionDb> {
        let mut cloned = Connection::open_in_memory().expect("open cloned DB");
        let backup = rusqlite::backup::Backup::new(source.conn_ref(), &mut cloned)
            .expect("initialize DB snapshot clone");
        backup.step(-1).expect("copy DB snapshot");
        drop(backup);
        cloned
            .execute_batch("PRAGMA foreign_keys = OFF;")
            .expect("disable FK for cloned tests");
        Arc::new(ActionDb::from_connection_for_tests(cloned))
    }

    fn reenriched_snapshot(
        source: &Arc<ActionDb>,
        ctx: &ServiceContext<'_>,
        feedback_id: &str,
    ) -> Arc<ActionDb> {
        let snapshot = cloned_snapshot(source);
        drain_feedback_propagation(snapshot.as_ref(), ctx, feedback_id, "reenrichment");
        snapshot
    }

    fn rebuilt_claim_file_snapshot(source: &Arc<ActionDb>, feedback_id: &str) -> Arc<ActionDb> {
        let snapshot = cloned_snapshot(source);
        insert_claim_file_projection_rebuild_at(
            snapshot.as_ref(),
            feedback_id,
            "2026-06-05T12:10:00+00:00",
        );
        snapshot
    }

    fn drain_feedback_propagation(
        db: &ActionDb,
        ctx: &ServiceContext<'_>,
        feedback_id: &str,
        phase: &str,
    ) {
        for _ in 0..64 {
            let outcome =
                crate::services::claim_feedback_propagation::process_one_feedback_propagation_job(
                    ctx,
                    db,
                    &format!("dos338-{phase}"),
                )
                .expect("process feedback propagation job");
            if matches!(
                outcome,
                crate::services::claim_feedback_propagation::FeedbackPropagationProcessOutcome::NoJob
            ) {
                break;
            }
        }
        let active_jobs: i64 = db
            .conn_ref()
            .query_row(
                "SELECT count(*)
                   FROM claim_feedback_propagation_jobs
                  WHERE feedback_id = ?1
                    AND status IN ('pending', 'running', 'coalesced')",
                params![feedback_id],
                |row| row.get(0),
            )
            .expect("count active feedback propagation jobs");
        assert_eq!(
            active_jobs, 0,
            "{phase} phase should drain feedback propagation jobs"
        );
    }

    fn insert_claim_file_projection_rebuild_at(
        db: &ActionDb,
        feedback_id: &str,
        observed_at: &str,
    ) {
        let subject_ref = proposal().subject_ref;
        db.conn_ref()
            .execute(
                "INSERT INTO claim_file_projection_runs (
                    id, entity_subject_ref_json, entity_subject_compact, projection_root,
                    markdown_rel_path, sidecar_rel_path, projection_version,
                    sidecar_schema_version, entity_claim_invalidation_version,
                    claim_watermark, markdown_checksum, sidecar_checksum, status,
                    attempted_at, succeeded_at, created_at, updated_at
                 ) VALUES (
                    ?1, ?2, ?3, '_dailyos_claims',
                    '_dailyos_claims/account/account-01/claims.md',
                    '_dailyos_claims/account/account-01/claims.corrections.json',
                    1, 1, 0, 'claim_dos338_01:v1',
                    'markdown-checksum', 'sidecar-checksum', 'committed',
                    ?4, ?4, ?4, ?4
                 )",
                params![
                    format!("dos338-rebuild-{feedback_id}"),
                    &subject_ref,
                    r#"{"id":"account_01","kind":"account"}"#,
                    observed_at,
                ],
            )
            .expect("insert claim-file rebuild run");
        db.conn_ref()
            .execute(
                "INSERT INTO claim_file_projection_run_claims (
                    run_id, claim_id, claim_version, semantic_identity_json,
                    trust_band, sensitivity
                 ) VALUES (?1, 'claim_dos338_01', 1, '{}', 'likely_current', 'internal')",
                params![format!("dos338-rebuild-{feedback_id}")],
            )
            .expect("insert claim-file rebuild membership");
    }

    fn harness_case(feedback_id: &str) -> StickinessHarnessCaseInput {
        harness_case_with_action(feedback_id, FeedbackAction::ConfirmCurrent)
    }

    fn harness_case_with_action(
        feedback_id: &str,
        action: FeedbackAction,
    ) -> StickinessHarnessCaseInput {
        StickinessHarnessCaseInput {
            feedback_id: feedback_id.to_string(),
            action,
            subject_kind: "account".to_string(),
            subject_ref: "account_01".to_string(),
            direct_surface: StickinessSurfaceProbe::ClaimReceipt {
                surface: SurfaceContext::EntityDetail,
            },
            indirect_surface: StickinessSurfaceProbe::ClaimFileProjection,
        }
    }

    fn record_fixture_feedback(db: &ActionDb) -> (ServiceContext<'static>, String) {
        record_fixture_feedback_with_action(db, FeedbackAction::ConfirmCurrent)
    }

    fn observation(result: StickinessObservationResult) -> StickinessObservationInput {
        StickinessObservationInput {
            feedback_id: None,
            action: FeedbackAction::WrongSource,
            subject_kind: "account".to_string(),
            subject_ref: "account_01".to_string(),
            direct_surface: "claim_receipt".to_string(),
            indirect_surface: "meeting_readiness".to_string(),
            direct_surface_before_hash: Some("sha256:direct-pre".to_string()),
            direct_surface_after_hash: Some("sha256:direct-post".to_string()),
            indirect_surface_before_hash: Some("sha256:indirect-pre".to_string()),
            indirect_surface_after_hash: Some("sha256:indirect-post".to_string()),
            pre_reenrichment_state_hash: Some("sha256:pre".to_string()),
            post_reenrichment_state_hash: Some("sha256:post".to_string()),
            post_rebuild_state_hash: Some("sha256:rebuild".to_string()),
            trust_band_before: Some("likely_current".to_string()),
            trust_band_after: Some("use_with_caution".to_string()),
            recompute_job_id: Some("job_01".to_string()),
            repair_job_id: None,
            dead_letter_reason: None,
            sensitivity_gate_result: "local_only_allowed".to_string(),
            result,
            reason_code: None,
        }
    }

    #[test]
    fn record_stickiness_run_persists_fixture_safe_report() {
        let db = test_db();
        let clock = crate::services::context::SystemClock;
        let rng = crate::services::context::SystemRng;
        let external = crate::services::context::ExternalClients::default();
        let ctx = live_ctx(&clock, &rng, &external);

        let report = record_stickiness_run(
            &ctx,
            &db,
            StickinessRunInput {
                fixture_id: "fixture_account_01".to_string(),
                entry_point: StickinessEntryPoint::App,
                observations: vec![observation(StickinessObservationResult::Passed)],
            },
        )
        .expect("record stickiness run");

        assert_eq!(report.status, "completed");
        assert_eq!(report.passed_observations, 1);
        assert!(report.report_hash.starts_with("sha256:"));

        let (fixture_hash, result): (String, String) = db
            .conn_ref()
            .query_row(
                "SELECT runs.fixture_id_hash, observations.result
                   FROM dos338_stickiness_runs runs
                   JOIN dos338_stickiness_observations observations
                     ON observations.run_id = runs.id
                  WHERE runs.id = ?1",
                [&report.run_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("read stickiness run");
        assert!(!fixture_hash.is_empty());
        assert!(!fixture_hash.contains("fixture_account_01"));
        assert_eq!(result, "passed");
    }

    #[test]
    fn mcp_observation_can_claim_pass_after_w5() {
        let db = test_db();
        let clock = crate::services::context::SystemClock;
        let rng = crate::services::context::SystemRng;
        let external = crate::services::context::ExternalClients::default();
        let ctx = live_ctx(&clock, &rng, &external);

        let report = record_stickiness_run(
            &ctx,
            &db,
            StickinessRunInput {
                fixture_id: "fixture_mcp_01".to_string(),
                entry_point: StickinessEntryPoint::Mcp,
                observations: vec![observation(StickinessObservationResult::Passed)],
            },
        )
        .expect("W5 MCP parity can be measured by the same stickiness gate");

        assert_eq!(report.status, "completed");
        assert_eq!(report.passed_observations, 1);
    }

    #[test]
    fn evaluate_mode_cannot_write_stickiness_runs() {
        let db = test_db();
        let clock = crate::services::context::SystemClock;
        let rng = crate::services::context::SystemRng;
        let ctx = ServiceContext::new_evaluate_default(&clock, &rng).with_actor("eval:test");

        let error = record_stickiness_run(
            &ctx,
            &db,
            StickinessRunInput {
                fixture_id: "fixture_eval_01".to_string(),
                entry_point: StickinessEntryPoint::App,
                observations: vec![observation(StickinessObservationResult::Passed)],
            },
        )
        .expect_err("evaluate mode must not write DOS-338 rows");

        assert!(error.to_string().contains("rejected"));
    }

    #[test]
    fn w4_dos338_harness_computes_pass_from_service_read_snapshots() {
        let before_db = uncorrected_snapshot();
        let (after_db, ctx, feedback_id) = corrected_snapshot(FeedbackAction::MarkFalse);
        let after_reenrichment_db = reenriched_snapshot(&after_db, &ctx, &feedback_id);
        let after_rebuild_db = rebuilt_claim_file_snapshot(&after_reenrichment_db, &feedback_id);

        let report = record_stickiness_harness_run(
            &ctx,
            after_db.as_ref(),
            StickinessHarnessRunInput {
                fixture_id: "fixture_account_01".to_string(),
                entry_point: StickinessEntryPoint::App,
                snapshots: StickinessHarnessSnapshots {
                    before_correction: Arc::clone(&before_db),
                    after_correction: Arc::clone(&after_db),
                    after_reenrichment: Arc::clone(&after_reenrichment_db),
                    after_rebuild: Arc::clone(&after_rebuild_db),
                },
                cases: vec![harness_case_with_action(
                    &feedback_id,
                    FeedbackAction::MarkFalse,
                )],
            },
        )
        .expect("record harness run");

        let (result, reason_code, direct_before, direct_after, indirect_before, indirect_after): (
            String,
            Option<String>,
            String,
            String,
            String,
            String,
        ) = after_db
            .conn_ref()
            .query_row(
                "SELECT result, reason_code,
	                        direct_surface_before_hash, direct_surface_after_hash,
	                        indirect_surface_before_hash, indirect_surface_after_hash
	                   FROM dos338_stickiness_observations
	                  WHERE run_id = ?1",
                params![&report.run_id],
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
            .expect("read computed observation hashes");
        assert_eq!(report.status, "completed", "reason={reason_code:?}");
        assert_eq!(report.passed_observations, 1);
        assert_eq!(result, "passed");
        assert_eq!(reason_code, None);
        assert!(direct_before.starts_with("sha256:"));
        assert!(direct_after.starts_with("sha256:"));
        assert_ne!(direct_before, direct_after);
        assert_ne!(indirect_before, indirect_after);
    }

    #[test]
    fn w4_dos338_harness_fails_when_service_snapshots_do_not_change() {
        let (before_db, _, _) = corrected_snapshot(FeedbackAction::ConfirmCurrent);
        let (after_db, ctx, feedback_id) = corrected_snapshot(FeedbackAction::ConfirmCurrent);
        let after_reenrichment_db = reenriched_snapshot(&after_db, &ctx, &feedback_id);
        let after_rebuild_db = rebuilt_claim_file_snapshot(&after_reenrichment_db, &feedback_id);

        let report = record_stickiness_harness_run(
            &ctx,
            after_db.as_ref(),
            StickinessHarnessRunInput {
                fixture_id: "fixture_account_01".to_string(),
                entry_point: StickinessEntryPoint::App,
                snapshots: StickinessHarnessSnapshots {
                    before_correction: Arc::clone(&before_db),
                    after_correction: Arc::clone(&after_db),
                    after_reenrichment: Arc::clone(&after_reenrichment_db),
                    after_rebuild: Arc::clone(&after_rebuild_db),
                },
                cases: vec![harness_case(&feedback_id)],
            },
        )
        .expect("record failed harness run");

        assert_eq!(report.status, "failed");
        assert_eq!(report.failed_observations, 1);
        let reason_code: String = after_db
            .conn_ref()
            .query_row(
                "SELECT reason_code
	                   FROM dos338_stickiness_observations
	                  WHERE run_id = ?1",
                params![&report.run_id],
                |row| row.get(0),
            )
            .expect("read reason code");
        assert_eq!(reason_code, "direct_surface_not_rerendered");
    }

    #[test]
    fn w4_dos338_harness_rejects_reused_service_snapshots() {
        let db = Arc::new(test_db());
        let (ctx, feedback_id) = record_fixture_feedback(&db);

        let error = record_stickiness_harness_run(
            &ctx,
            db.as_ref(),
            StickinessHarnessRunInput {
                fixture_id: "fixture_account_01".to_string(),
                entry_point: StickinessEntryPoint::App,
                snapshots: StickinessHarnessSnapshots {
                    before_correction: Arc::clone(&db),
                    after_correction: Arc::clone(&db),
                    after_reenrichment: Arc::clone(&db),
                    after_rebuild: Arc::clone(&db),
                },
                cases: vec![harness_case(&feedback_id)],
            },
        )
        .expect_err("reused snapshots must not satisfy DOS-338 proof");

        assert!(error
            .to_string()
            .contains("must use a distinct service snapshot"));
    }

    #[test]
    fn w4_dos338_harness_rejects_distinct_synthetic_phase_snapshots() {
        let before_db = uncorrected_snapshot();
        let (after_db, ctx, feedback_id) = corrected_snapshot(FeedbackAction::MarkFalse);
        let (after_reenrichment_db, _, _) = corrected_snapshot(FeedbackAction::MarkFalse);
        let (after_rebuild_db, _, _) = corrected_snapshot(FeedbackAction::MarkFalse);

        let error = record_stickiness_harness_run(
            &ctx,
            after_db.as_ref(),
            StickinessHarnessRunInput {
                fixture_id: "fixture_account_01".to_string(),
                entry_point: StickinessEntryPoint::App,
                snapshots: StickinessHarnessSnapshots {
                    before_correction: Arc::clone(&before_db),
                    after_correction: Arc::clone(&after_db),
                    after_reenrichment: Arc::clone(&after_reenrichment_db),
                    after_rebuild: Arc::clone(&after_rebuild_db),
                },
                cases: vec![harness_case_with_action(
                    &feedback_id,
                    FeedbackAction::MarkFalse,
                )],
            },
        )
        .expect_err("distinct synthetic snapshots must not satisfy DOS-338 proof");

        assert!(error
            .to_string()
            .contains("missing feedback propagation proof"));
    }

    #[test]
    fn w4_dos338_harness_rejects_stale_claim_file_projection_rebuild_proof() {
        let before_db = uncorrected_snapshot();
        let (after_db, ctx, feedback_id) = corrected_snapshot(FeedbackAction::MarkFalse);
        let after_reenrichment_db = reenriched_snapshot(&after_db, &ctx, &feedback_id);
        let after_rebuild_db = cloned_snapshot(&after_reenrichment_db);
        insert_claim_file_projection_rebuild_at(
            after_rebuild_db.as_ref(),
            &feedback_id,
            "2026-06-05T11:59:59+00:00",
        );

        let error = record_stickiness_harness_run(
            &ctx,
            after_db.as_ref(),
            StickinessHarnessRunInput {
                fixture_id: "fixture_account_01".to_string(),
                entry_point: StickinessEntryPoint::App,
                snapshots: StickinessHarnessSnapshots {
                    before_correction: Arc::clone(&before_db),
                    after_correction: Arc::clone(&after_db),
                    after_reenrichment: Arc::clone(&after_reenrichment_db),
                    after_rebuild: Arc::clone(&after_rebuild_db),
                },
                cases: vec![harness_case_with_action(
                    &feedback_id,
                    FeedbackAction::MarkFalse,
                )],
            },
        )
        .expect_err("stale claim-file projection proof must not satisfy DOS-338 rebuild phase");

        assert!(error
            .to_string()
            .contains("missing post-feedback claim file projection rebuild proof"));
    }

    #[test]
    fn w4_dos338_harness_rejects_stale_meeting_prep_rebuild_proof() {
        let (after_db, _, feedback_id) = corrected_snapshot(FeedbackAction::MarkFalse);
        let envelope = feedback_correction_envelope(after_db.as_ref(), &feedback_id)
            .expect("load feedback envelope")
            .expect("feedback envelope exists");
        let probe = StickinessSurfaceProbe::MeetingPrepStatus {
            meeting_id: "meeting-fixture-1".to_string(),
        };

        let error = validate_rebuild_surface_proof(&after_db, &feedback_id, &probe, &envelope)
            .expect_err("meeting prep rebuild proof must require post-feedback replay evidence");

        assert!(error
            .to_string()
            .contains("missing post-feedback meeting prep replay proof"));

        after_db
            .conn_ref()
            .execute(
                "INSERT INTO meeting_prep_correction_journal (
                    id,
                    feedback_id,
                    meeting_stable_key,
                    meeting_id,
                    field_path,
                    actor,
                    surface,
                    source_asof,
                    sensitivity,
                    replay_key,
                    payload_json,
                    payload_hash,
                    lifecycle_state,
                    replay_attempt_count,
                    rebuild_replay_id,
                    replayed_at,
                    created_at,
                    updated_at
                 ) VALUES (
                    'journal-dos338-meeting-proof',
                    ?1,
                    'stable-key-dos338',
                    'meeting-fixture-1',
                    'user_notes',
                    'user',
                    'tauri',
                    '2026-06-05T12:10:00+00:00',
                    'user_only',
                    'replay-key-dos338',
                    '{\"value\":\"Replayed meeting prep note\"}',
                    'payload-hash-dos338',
                    'active',
                    1,
                    'rebuild-dos338-meeting-proof',
                    '2026-06-05T12:10:00+00:00',
                    '2026-06-05T12:10:00+00:00',
                    '2026-06-05T12:10:00+00:00'
                 )",
                params![&feedback_id],
            )
            .expect("insert post-feedback meeting prep replay proof");

        validate_rebuild_surface_proof(&after_db, &feedback_id, &probe, &envelope)
            .expect("post-feedback meeting prep replay proof satisfies rebuild gate");
    }

    #[test]
    fn w4_dos338_harness_rejects_active_coalesced_parent_job() {
        let (after_db, _, feedback_id) = corrected_snapshot(FeedbackAction::MarkFalse);
        after_db
            .conn_ref()
            .execute(
                "UPDATE claim_feedback_propagation_jobs
                    SET status = 'completed',
                        completed_at = '2026-06-05T12:01:00+00:00'
                  WHERE feedback_id = ?1",
                params![&feedback_id],
            )
            .expect("terminalize direct jobs");
        after_db
            .conn_ref()
            .execute(
                "INSERT INTO claim_feedback_propagation_jobs (
                    id, feedback_id, action, target_kind, operation, sync_class,
                    status, coalescing_key, scope_json, created_at, updated_at
                 ) VALUES (
                    'coalesced-parent-job', 'older-feedback-id', 'mark_false',
                    'claim_recompute', 'recompute_claim_trust', 'async',
                    'pending', 'shared-coalescing-key', '{}',
                    '2026-06-05T12:00:00+00:00', '2026-06-05T12:00:00+00:00'
                 )",
                [],
            )
            .expect("insert active coalesced parent job");
        after_db
            .conn_ref()
            .execute(
                "INSERT INTO claim_feedback_propagation_outcomes (
                    id, job_id, feedback_id, target_kind, operation, sync_class,
                    status, reason_code, observed_at
                 ) VALUES (
                    'coalesced-outcome-for-current-feedback', 'coalesced-parent-job',
                    ?1, 'claim_recompute', 'recompute_claim_trust', 'async',
                    'coalesced', 'coalesced_into_existing_target',
                    '2026-06-05T12:00:01+00:00'
                 )",
                params![&feedback_id],
            )
            .expect("insert coalesced outcome for current feedback");

        let error = validate_harness_phase_progress(&after_db, &feedback_id, "after_reenrichment")
            .expect_err("active coalesced parent job must block DOS-338 phase proof");
        assert!(error
            .to_string()
            .contains("unprocessed feedback propagation jobs"));
    }

    #[test]
    fn w4_dos338_harness_cannot_claim_pass_without_feedback_envelope() {
        let before_db = uncorrected_snapshot();
        let after_db = uncorrected_snapshot();
        let after_reenrichment_db = uncorrected_snapshot();
        let after_rebuild_db = uncorrected_snapshot();
        let ctx = fixture_ctx();

        let report = record_stickiness_harness_run(
            &ctx,
            after_db.as_ref(),
            StickinessHarnessRunInput {
                fixture_id: "fixture_account_01".to_string(),
                entry_point: StickinessEntryPoint::App,
                snapshots: StickinessHarnessSnapshots {
                    before_correction: Arc::clone(&before_db),
                    after_correction: Arc::clone(&after_db),
                    after_reenrichment: Arc::clone(&after_reenrichment_db),
                    after_rebuild: Arc::clone(&after_rebuild_db),
                },
                cases: vec![harness_case("missing_feedback_id")],
            },
        )
        .expect("record failed harness run");

        assert_eq!(report.status, "failed");
        let reason_code: String = after_db
            .conn_ref()
            .query_row(
                "SELECT reason_code
                   FROM dos338_stickiness_observations
                  WHERE run_id = ?1",
                params![&report.run_id],
                |row| row.get(0),
            )
            .expect("read reason code");
        assert_eq!(reason_code, "correction_not_applied");
    }

    #[test]
    fn w5_dos338_harness_scores_mcp_with_stickiness_criteria() {
        let before_db = uncorrected_snapshot();
        let (after_db, ctx, feedback_id) = corrected_snapshot(FeedbackAction::MarkFalse);
        let after_reenrichment_db = reenriched_snapshot(&after_db, &ctx, &feedback_id);
        let after_rebuild_db = rebuilt_claim_file_snapshot(&after_reenrichment_db, &feedback_id);

        let report = record_stickiness_harness_run(
            &ctx,
            after_db.as_ref(),
            StickinessHarnessRunInput {
                fixture_id: "fixture_mcp_01".to_string(),
                entry_point: StickinessEntryPoint::Mcp,
                snapshots: StickinessHarnessSnapshots {
                    before_correction: Arc::clone(&before_db),
                    after_correction: Arc::clone(&after_db),
                    after_reenrichment: Arc::clone(&after_reenrichment_db),
                    after_rebuild: Arc::clone(&after_rebuild_db),
                },
                cases: vec![harness_case_with_action(
                    &feedback_id,
                    FeedbackAction::MarkFalse,
                )],
            },
        )
        .expect("record mcp harness run");

        assert_eq!(report.status, "completed");
        assert_eq!(report.passed_observations, 1);
        assert_eq!(report.blocked_observations, 0);
        let (result, reason_code): (String, Option<String>) = after_db
            .conn_ref()
            .query_row(
                "SELECT result, reason_code
                   FROM dos338_stickiness_observations
                  WHERE run_id = ?1",
                params![&report.run_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("read mcp observation");
        assert_eq!(result, "passed");
        assert_eq!(reason_code, None);
    }

    #[test]
    fn w4_dos338_harness_rejects_action_subject_claims_not_bound_to_envelope() {
        let before_db = uncorrected_snapshot();
        let (after_db, ctx, feedback_id) = corrected_snapshot(FeedbackAction::ConfirmCurrent);
        let (after_reenrichment_db, _, _) = corrected_snapshot(FeedbackAction::ConfirmCurrent);
        let (after_rebuild_db, _, _) = corrected_snapshot(FeedbackAction::ConfirmCurrent);
        let mut mismatched = harness_case(&feedback_id);
        mismatched.action = FeedbackAction::WrongSource;

        let error = record_stickiness_harness_run(
            &ctx,
            after_db.as_ref(),
            StickinessHarnessRunInput {
                fixture_id: "fixture_account_01".to_string(),
                entry_point: StickinessEntryPoint::App,
                snapshots: StickinessHarnessSnapshots {
                    before_correction: Arc::clone(&before_db),
                    after_correction: Arc::clone(&after_db),
                    after_reenrichment: Arc::clone(&after_reenrichment_db),
                    after_rebuild: Arc::clone(&after_rebuild_db),
                },
                cases: vec![mismatched],
            },
        )
        .expect_err("case must be bound to envelope action");

        assert!(error
            .to_string()
            .contains("does not match feedback envelope action"));
    }
}
