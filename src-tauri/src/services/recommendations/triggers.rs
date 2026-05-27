//! Trigger policy.
//!
//! Consumes upstream signals (claim changes, signal-bus events) and
//! emits `SalienceCandidateRefreshTriggered` when a candidate's
//! salience must be re-scored. Deterministic — no provider / LLM
//! call from this module.

use chrono::{DateTime, Duration, Utc};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};

use abilities_runtime::sensitivity::ClaimDismissalSurface;

use super::contracts::{ClaimId, TriggerKind, TriggerRef};
use super::surfacing::{
    evaluate_surfacing_for_claim, evidence_signature as claim_evidence_signature, SurfaceClass,
    SurfacingError, SurfacingEvaluationInput, SURFACING_EVALUATION_SCHEMA_VERSION,
};
use crate::db::ActionDb;
use crate::services::context::{ExecutionMode, ServiceContext, ServiceError};
use crate::signals::propagation::PropagationEngine;

pub const TRIGGER_POLICY_VERSION: &str = "recommendation_trigger_policy_v1";
pub const TRIGGER_SCAN_SCHEMA_VERSION: u32 = 1;
pub const SALIENCE_CANDIDATE_REFRESH_SIGNAL: &str = "salience_candidate_refresh_triggered";
const TRIGGER_SIGNAL_SOURCE: &str = "recommendation_trigger_policy";
const DOWNSTREAM_POLICY: &str = "recommendation_surfacing_policy";
const STARTED_COALESCE_SECS: i64 = 900;
const DEFAULT_MAX_CANDIDATES: usize = 25;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerClass {
    ScheduledFreshness,
    EventInvalidation,
    ManualRefresh,
    EntityChange,
    ClaimChange,
    SourceChange,
    OpenLoopChange,
    MeetingWindow,
    DecisionWindow,
    FeedbackEcho,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerDisposition {
    SilentPrepare,
    PrimaryCandidate,
    ReviewCandidate,
    QuietCandidate,
}

impl TriggerDisposition {
    const fn as_str(self) -> &'static str {
        match self {
            Self::SilentPrepare => "silent_prepare",
            Self::PrimaryCandidate => "primary_candidate",
            Self::ReviewCandidate => "review_candidate",
            Self::QuietCandidate => "quiet_candidate",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TriggerScanInput {
    pub schema_version: u32,
    pub trigger_class: TriggerClass,
    pub subject_kind: String,
    pub subject_id: String,
    pub render_surface: ClaimDismissalSurface,
    pub surface_class: SurfaceClass,
    pub run_id: Option<String>,
    pub source_signal_id: Option<String>,
    pub source_signal_type: Option<String>,
    pub source_asof: Option<DateTime<Utc>>,
    pub evidence_signature: Option<String>,
    pub subject_version: Option<i64>,
    pub max_candidates: Option<usize>,
}

impl TriggerScanInput {
    pub fn scheduled(subject_kind: impl Into<String>, subject_id: impl Into<String>) -> Self {
        Self {
            schema_version: TRIGGER_SCAN_SCHEMA_VERSION,
            trigger_class: TriggerClass::ScheduledFreshness,
            subject_kind: subject_kind.into(),
            subject_id: subject_id.into(),
            render_surface: ClaimDismissalSurface::Briefing,
            surface_class: SurfaceClass::Background,
            run_id: None,
            source_signal_id: None,
            source_signal_type: None,
            source_asof: None,
            evidence_signature: None,
            subject_version: None,
            max_candidates: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerScanStatus {
    Completed,
    CoalescedCompleted,
    CoalescedInProgress,
    CoalescedRetryNotDue,
    EvaluateOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TriggerScanOutcome {
    pub schema_version: u32,
    pub policy_version: String,
    pub run_id: String,
    pub status: TriggerScanStatus,
    pub signal_id: Option<String>,
    pub signal_coalesced: bool,
    pub derived_signal_ids: Vec<String>,
    pub candidate_claim_ids: Vec<String>,
    pub salience_evaluation_ids: Vec<String>,
    pub surfacing_decision_ids: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum TriggerError {
    #[error("unsupported schema_version `{0}` for run_trigger_scan")]
    UnsupportedSchemaVersion(u32),
    #[error("trigger subject kind/id must be non-empty")]
    EmptySubject,
    #[error("trigger {0} must be privacy-safe")]
    UnsafeInput(&'static str),
    #[error("trigger database error: {0}")]
    Database(String),
    #[error(transparent)]
    Service(#[from] ServiceError),
    #[error("trigger signal emit failed: {0}")]
    Signal(String),
    #[error(transparent)]
    Surfacing(#[from] SurfacingError),
    #[error("trigger serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
}

impl From<rusqlite::Error> for TriggerError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Database(error.to_string())
    }
}

#[derive(Debug, Clone, Copy)]
struct TriggerPolicy {
    trigger_kind: TriggerKind,
    trigger_disposition: TriggerDisposition,
    trust_floor: f64,
    freshness_window_secs: i64,
    suppression_window_secs: i64,
    reason_code: &'static str,
}

#[derive(Debug)]
struct ExistingTrigger {
    run_id: String,
    status: String,
    started_at: Option<DateTime<Utc>>,
    completed_at: Option<DateTime<Utc>>,
    next_retry_at: Option<DateTime<Utc>>,
    retry_count: i64,
    signal_id: Option<String>,
    signal_coalesced: bool,
    derived_signal_ids: Vec<String>,
    candidate_claim_ids: Vec<String>,
    salience_evaluation_ids: Vec<String>,
    surfacing_decision_ids: Vec<String>,
}

#[derive(Debug)]
struct CandidateClaim {
    claim_id: String,
    freshness_at: Option<DateTime<Utc>>,
    evidence_signature: Option<String>,
}

pub fn run_trigger_scan(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    engine: &PropagationEngine,
    input: TriggerScanInput,
) -> Result<TriggerScanOutcome, TriggerError> {
    validate_schema_version(input.schema_version)?;
    validate_subject(&input)?;
    let policy = policy_for_trigger(input.trigger_class);
    let now = ctx.clock.now();
    let run_id = input
        .run_id
        .as_deref()
        .map(|value| safe_storage_ref("run_ref", value))
        .unwrap_or_else(|| {
            if matches!(ctx.mode, ExecutionMode::Live) {
                format!("trigger-run-{}", uuid::Uuid::new_v4())
            } else {
                "evaluate-only".to_string()
            }
        });
    let source_signal_id = safe_optional_ref("signal_ref", input.source_signal_id.as_deref());
    let dedupe_key = dedupe_key(&input, policy, now, &run_id);
    let suppression_key = suppression_key(&input, policy);

    if !matches!(ctx.mode, ExecutionMode::Live) {
        return Ok(TriggerScanOutcome {
            schema_version: TRIGGER_SCAN_SCHEMA_VERSION,
            policy_version: TRIGGER_POLICY_VERSION.to_string(),
            run_id,
            status: TriggerScanStatus::EvaluateOnly,
            signal_id: None,
            signal_coalesced: false,
            derived_signal_ids: Vec::new(),
            candidate_claim_ids: Vec::new(),
            salience_evaluation_ids: Vec::new(),
            surfacing_decision_ids: Vec::new(),
        });
    }

    ctx.check_mutation_allowed()?;
    if let Some(existing) = coalesced_existing_trigger(db.conn_ref(), &dedupe_key, policy, now)? {
        return Ok(existing_outcome(existing));
    }

    let subject_version_changed = subject_version_changed_since_latest_trigger(
        db.conn_ref(),
        &suppression_key,
        input.subject_version,
    )?;
    let retry_count = latest_retry_count(db.conn_ref(), &dedupe_key)? + 1;
    insert_started_trigger(db, &run_id, &input, policy, &dedupe_key, retry_count, now)?;

    let signal_value = trigger_signal_value(&run_id, &input, policy)?;
    let signal_key = format!("{TRIGGER_POLICY_VERSION}:{run_id}");
    let (signal_outcome, derived_signal_ids) =
        match crate::services::signals::emit_once_for_key_and_propagate(
            ctx,
            db,
            engine,
            &signal_key,
            &input.subject_kind,
            &input.subject_id,
            SALIENCE_CANDIDATE_REFRESH_SIGNAL,
            TRIGGER_SIGNAL_SOURCE,
            Some(&signal_value),
            1.0,
        ) {
            Ok(outcome) => outcome,
            Err(error) => {
                mark_trigger_failed(db, &run_id, "signal_emit_failed", true, now)?;
                return Err(TriggerError::Signal(error));
            }
        };

    let candidates = find_candidate_claims(db.conn_ref(), &input, policy, now)?;
    let mut claim_ids = Vec::with_capacity(candidates.len());
    let mut salience_ids = Vec::with_capacity(candidates.len());
    let mut decision_ids = Vec::with_capacity(candidates.len());

    for candidate in candidates {
        claim_ids.push(candidate.claim_id.clone());
        let evidence_signature_changed = evidence_signature_changed(&input, &candidate);
        let evaluation_input = SurfacingEvaluationInput {
            schema_version: SURFACING_EVALUATION_SCHEMA_VERSION,
            claim_id: ClaimId(candidate.claim_id),
            render_surface: input.render_surface,
            surface_class: surface_class_for_disposition(policy.trigger_disposition),
            trigger_refs: vec![trigger_ref_for_input(&input, policy, now)],
            source_signal_id: source_signal_id.clone(),
            trigger_source_asof: input
                .source_asof
                .as_ref()
                .cloned()
                .or(candidate.freshness_at),
            evidence_signature_changed,
            subject_version_changed,
        };
        match evaluate_surfacing_for_claim(ctx, db, engine, evaluation_input) {
            Ok(evaluation) => {
                if let Some(id) = evaluation.salience_evaluation_id {
                    salience_ids.push(id);
                }
                if let Some(id) = evaluation.surfacing_decision_id {
                    decision_ids.push(id);
                }
            }
            Err(error) => {
                mark_trigger_failed(db, &run_id, closed_error_code(&error), true, now)?;
                return Err(error.into());
            }
        }
    }

    mark_trigger_completed(
        db,
        &run_id,
        CompletedTrigger {
            signal_id: &signal_outcome.id,
            signal_coalesced: signal_outcome.coalesced,
            derived_signal_ids: &derived_signal_ids,
            candidate_claim_ids: &claim_ids,
            salience_evaluation_ids: &salience_ids,
            surfacing_decision_ids: &decision_ids,
            result_kind: result_kind_for_completion(policy.trigger_disposition, &decision_ids),
            completed_at: now,
        },
    )?;

    Ok(TriggerScanOutcome {
        schema_version: TRIGGER_SCAN_SCHEMA_VERSION,
        policy_version: TRIGGER_POLICY_VERSION.to_string(),
        run_id,
        status: TriggerScanStatus::Completed,
        signal_id: Some(signal_outcome.id),
        signal_coalesced: signal_outcome.coalesced,
        derived_signal_ids,
        candidate_claim_ids: claim_ids,
        salience_evaluation_ids: salience_ids,
        surfacing_decision_ids: decision_ids,
    })
}

fn validate_schema_version(schema_version: u32) -> Result<(), TriggerError> {
    if schema_version == TRIGGER_SCAN_SCHEMA_VERSION {
        Ok(())
    } else {
        Err(TriggerError::UnsupportedSchemaVersion(schema_version))
    }
}

fn validate_subject(input: &TriggerScanInput) -> Result<(), TriggerError> {
    if input.subject_kind.trim().is_empty() || input.subject_id.trim().is_empty() {
        return Err(TriggerError::EmptySubject);
    }
    if !is_safe_storage_token(&input.subject_kind) {
        return Err(TriggerError::UnsafeInput("subject_kind"));
    }
    if !is_safe_storage_ref(&input.subject_id) {
        return Err(TriggerError::UnsafeInput("subject_id"));
    }
    if input
        .run_id
        .as_deref()
        .is_some_and(|run_id| run_id.trim().is_empty())
    {
        return Err(TriggerError::UnsafeInput("run_id"));
    }
    Ok(())
}

fn policy_for_trigger(trigger_class: TriggerClass) -> TriggerPolicy {
    match trigger_class {
        TriggerClass::ScheduledFreshness => TriggerPolicy {
            trigger_kind: TriggerKind::ScheduledScan,
            trigger_disposition: TriggerDisposition::SilentPrepare,
            trust_floor: 0.60,
            freshness_window_secs: 86_400,
            suppression_window_secs: 86_400,
            reason_code: "scheduled_freshness",
        },
        TriggerClass::EventInvalidation => TriggerPolicy {
            trigger_kind: TriggerKind::SignalArrival,
            trigger_disposition: TriggerDisposition::PrimaryCandidate,
            trust_floor: 0.70,
            freshness_window_secs: 21_600,
            suppression_window_secs: 21_600,
            reason_code: "event_invalidation",
        },
        TriggerClass::ManualRefresh => TriggerPolicy {
            trigger_kind: TriggerKind::SignalArrival,
            trigger_disposition: TriggerDisposition::ReviewCandidate,
            trust_floor: 0.50,
            freshness_window_secs: 0,
            suppression_window_secs: 60,
            reason_code: "manual_refresh",
        },
        TriggerClass::EntityChange => TriggerPolicy {
            trigger_kind: TriggerKind::EntityChange,
            trigger_disposition: TriggerDisposition::PrimaryCandidate,
            trust_floor: 0.65,
            freshness_window_secs: 21_600,
            suppression_window_secs: 21_600,
            reason_code: "entity_change",
        },
        TriggerClass::ClaimChange => TriggerPolicy {
            trigger_kind: TriggerKind::SignalArrival,
            trigger_disposition: TriggerDisposition::PrimaryCandidate,
            trust_floor: 0.70,
            freshness_window_secs: 21_600,
            suppression_window_secs: 3_600,
            reason_code: "claim_change",
        },
        TriggerClass::SourceChange => TriggerPolicy {
            trigger_kind: TriggerKind::SignalArrival,
            trigger_disposition: TriggerDisposition::ReviewCandidate,
            trust_floor: 0.75,
            freshness_window_secs: 21_600,
            suppression_window_secs: 21_600,
            reason_code: "source_change",
        },
        TriggerClass::OpenLoopChange => TriggerPolicy {
            trigger_kind: TriggerKind::SignalArrival,
            trigger_disposition: TriggerDisposition::QuietCandidate,
            trust_floor: 0.55,
            freshness_window_secs: 43_200,
            suppression_window_secs: 10_800,
            reason_code: "open_loop_change",
        },
        TriggerClass::MeetingWindow => TriggerPolicy {
            trigger_kind: TriggerKind::EntityChange,
            trigger_disposition: TriggerDisposition::PrimaryCandidate,
            trust_floor: 0.65,
            freshness_window_secs: 7_200,
            suppression_window_secs: 10_800,
            reason_code: "meeting_window",
        },
        TriggerClass::DecisionWindow => TriggerPolicy {
            trigger_kind: TriggerKind::ScheduledScan,
            trigger_disposition: TriggerDisposition::PrimaryCandidate,
            trust_floor: 0.70,
            freshness_window_secs: 43_200,
            suppression_window_secs: 21_600,
            reason_code: "decision_window",
        },
        TriggerClass::FeedbackEcho => TriggerPolicy {
            trigger_kind: TriggerKind::FeedbackEcho,
            trigger_disposition: TriggerDisposition::QuietCandidate,
            trust_floor: 0.50,
            freshness_window_secs: 604_800,
            suppression_window_secs: 300,
            reason_code: "feedback_echo",
        },
    }
}

fn coalesced_existing_trigger(
    conn: &rusqlite::Connection,
    dedupe_key: &str,
    policy: TriggerPolicy,
    now: DateTime<Utc>,
) -> Result<Option<ExistingTrigger>, TriggerError> {
    if !table_exists(conn, "triggers_log")? {
        return Ok(None);
    }
    let Some(existing) = load_latest_trigger(conn, dedupe_key)? else {
        return Ok(None);
    };

    let coalesces = match existing.status.as_str() {
        "completed" => existing.completed_at.is_some_and(|at| {
            now.signed_duration_since(at).num_seconds() <= policy.suppression_window_secs
        }),
        "started" => existing
            .started_at
            .is_some_and(|at| now.signed_duration_since(at).num_seconds() <= STARTED_COALESCE_SECS),
        "failed_retryable" => existing.next_retry_at.is_some_and(|at| at > now),
        "failed_terminal" => false,
        _ => false,
    };

    Ok(coalesces.then_some(existing))
}

fn load_latest_trigger(
    conn: &rusqlite::Connection,
    dedupe_key: &str,
) -> Result<Option<ExistingTrigger>, TriggerError> {
    conn.query_row(
        "SELECT run_id, status, started_at, completed_at, next_retry_at, retry_count,
                signal_id, signal_coalesced, derived_signal_ids_json,
                candidate_claim_ids_json, salience_evaluation_ids_json,
                surfacing_decision_ids_json
           FROM triggers_log
          WHERE policy_version = ?1 AND dedupe_key = ?2
          ORDER BY updated_at DESC
          LIMIT 1",
        params![TRIGGER_POLICY_VERSION, dedupe_key],
        |row| {
            Ok(ExistingTrigger {
                run_id: row.get(0)?,
                status: row.get(1)?,
                started_at: row
                    .get::<_, Option<String>>(2)?
                    .as_deref()
                    .and_then(parse_datetime),
                completed_at: row
                    .get::<_, Option<String>>(3)?
                    .as_deref()
                    .and_then(parse_datetime),
                next_retry_at: row
                    .get::<_, Option<String>>(4)?
                    .as_deref()
                    .and_then(parse_datetime),
                retry_count: row.get(5)?,
                signal_id: row.get(6)?,
                signal_coalesced: row.get::<_, i64>(7)? == 1,
                derived_signal_ids: decode_string_vec(row.get::<_, String>(8)?),
                candidate_claim_ids: decode_string_vec(row.get::<_, String>(9)?),
                salience_evaluation_ids: decode_string_vec(row.get::<_, String>(10)?),
                surfacing_decision_ids: decode_string_vec(row.get::<_, String>(11)?),
            })
        },
    )
    .optional()
    .map_err(TriggerError::from)
}

fn existing_outcome(existing: ExistingTrigger) -> TriggerScanOutcome {
    let status = match existing.status.as_str() {
        "completed" => TriggerScanStatus::CoalescedCompleted,
        "started" => TriggerScanStatus::CoalescedInProgress,
        "failed_retryable" => TriggerScanStatus::CoalescedRetryNotDue,
        _ => TriggerScanStatus::CoalescedCompleted,
    };

    TriggerScanOutcome {
        schema_version: TRIGGER_SCAN_SCHEMA_VERSION,
        policy_version: TRIGGER_POLICY_VERSION.to_string(),
        run_id: existing.run_id,
        status,
        signal_id: existing.signal_id,
        signal_coalesced: existing.signal_coalesced,
        derived_signal_ids: existing.derived_signal_ids,
        candidate_claim_ids: existing.candidate_claim_ids,
        salience_evaluation_ids: existing.salience_evaluation_ids,
        surfacing_decision_ids: existing.surfacing_decision_ids,
    }
}

fn latest_retry_count(conn: &rusqlite::Connection, dedupe_key: &str) -> Result<i64, TriggerError> {
    if !table_exists(conn, "triggers_log")? {
        return Ok(0);
    }
    let retry_count = conn
        .query_row(
            "SELECT retry_count
               FROM triggers_log
              WHERE policy_version = ?1 AND dedupe_key = ?2
              ORDER BY updated_at DESC
              LIMIT 1",
            params![TRIGGER_POLICY_VERSION, dedupe_key],
            |row| row.get::<_, i64>(0),
        )
        .optional()?
        .unwrap_or(0);
    Ok(retry_count)
}

fn insert_started_trigger(
    db: &ActionDb,
    run_id: &str,
    input: &TriggerScanInput,
    policy: TriggerPolicy,
    dedupe_key: &str,
    retry_count: i64,
    now: DateTime<Utc>,
) -> Result<(), TriggerError> {
    let suppression_key = suppression_key(input, policy);
    let source_signal_id = safe_optional_ref("signal_ref", input.source_signal_id.as_deref());
    let source_signal_type =
        safe_optional_token("signal_type", input.source_signal_type.as_deref());
    let evidence_signature = safe_optional_ref("evidence", input.evidence_signature.as_deref());
    db.conn_ref().execute(
        "INSERT INTO triggers_log (
             run_id, policy_version, trigger_class, trigger_kind, status,
             trigger_disposition, result_kind, downstream_policy,
             subject_kind, subject_id, entity_type, entity_id,
             reason_code, dedupe_key, suppression_key, trust_floor,
             freshness_window_secs, source_signal_id, source_signal_type,
             source_asof, evidence_signature, subject_version, retry_count,
             started_at, updated_at
         ) VALUES (
             ?1, ?2, ?3, ?4, 'started',
             ?5, 'prepared_silently', ?6,
             ?7, ?8, ?9, ?10,
             ?11, ?12, ?13, ?14,
             ?15, ?16, ?17,
             ?18, ?19, ?20, ?21,
             ?22, ?23
         )",
        params![
            run_id,
            TRIGGER_POLICY_VERSION,
            trigger_class_storage(input.trigger_class),
            trigger_kind_storage(policy.trigger_kind),
            policy.trigger_disposition.as_str(),
            DOWNSTREAM_POLICY,
            &input.subject_kind,
            &input.subject_id,
            &input.subject_kind,
            &input.subject_id,
            policy.reason_code,
            dedupe_key,
            suppression_key,
            policy.trust_floor,
            policy.freshness_window_secs,
            source_signal_id.as_deref(),
            source_signal_type.as_deref(),
            input.source_asof.as_ref().map(|dt| dt.to_rfc3339()),
            evidence_signature.as_deref(),
            input.subject_version,
            retry_count,
            now.to_rfc3339(),
            now.to_rfc3339(),
        ],
    )?;
    Ok(())
}

struct CompletedTrigger<'a> {
    signal_id: &'a str,
    signal_coalesced: bool,
    derived_signal_ids: &'a [String],
    candidate_claim_ids: &'a [String],
    salience_evaluation_ids: &'a [String],
    surfacing_decision_ids: &'a [String],
    result_kind: &'static str,
    completed_at: DateTime<Utc>,
}

fn mark_trigger_completed(
    db: &ActionDb,
    run_id: &str,
    completed: CompletedTrigger<'_>,
) -> Result<(), TriggerError> {
    db.conn_ref().execute(
        "UPDATE triggers_log
            SET status = 'completed',
                signal_id = ?2,
                signal_coalesced = ?3,
                derived_signal_ids_json = ?4,
                candidate_claim_ids_json = ?5,
                salience_evaluation_ids_json = ?6,
                surfacing_decision_ids_json = ?7,
                result_kind = ?8,
                completed_at = ?9,
                updated_at = ?9
          WHERE run_id = ?1",
        params![
            run_id,
            completed.signal_id,
            if completed.signal_coalesced { 1 } else { 0 },
            serde_json::to_string(completed.derived_signal_ids)?,
            serde_json::to_string(completed.candidate_claim_ids)?,
            serde_json::to_string(completed.salience_evaluation_ids)?,
            serde_json::to_string(completed.surfacing_decision_ids)?,
            completed.result_kind,
            completed.completed_at.to_rfc3339(),
        ],
    )?;
    Ok(())
}

fn mark_trigger_failed(
    db: &ActionDb,
    run_id: &str,
    error_code: &'static str,
    retryable: bool,
    now: DateTime<Utc>,
) -> Result<(), TriggerError> {
    let status = if retryable {
        "failed_retryable"
    } else {
        "failed_terminal"
    };
    let next_retry_at = retryable.then(|| (now + Duration::minutes(10)).to_rfc3339());
    db.conn_ref().execute(
        "UPDATE triggers_log
            SET status = ?2,
                error_code = ?3,
                next_retry_at = ?4,
                result_kind = 'failed',
                updated_at = ?5
          WHERE run_id = ?1",
        params![
            run_id,
            status,
            error_code,
            next_retry_at.as_deref(),
            now.to_rfc3339(),
        ],
    )?;
    Ok(())
}

fn find_candidate_claims(
    conn: &rusqlite::Connection,
    input: &TriggerScanInput,
    policy: TriggerPolicy,
    now: DateTime<Utc>,
) -> Result<Vec<CandidateClaim>, TriggerError> {
    let mut stmt = conn.prepare(
        "SELECT id, COALESCE(source_asof, observed_at, created_at) AS freshness_at,
                metadata_json
           FROM intelligence_claims
          WHERE claim_type = 'recommendation'
            AND claim_state = 'active'
            AND surfacing_state = 'active'
            AND json_valid(subject_ref) = 1
            AND lower(json_extract(subject_ref, '$.kind')) = lower(?1)
            AND json_extract(subject_ref, '$.id') = ?2
            AND COALESCE(trust_score, 0.0) >= ?3
          ORDER BY COALESCE(trust_score, 0.0) DESC, observed_at DESC, id ASC",
    )?;
    let rows = stmt.query_map(
        params![&input.subject_kind, &input.subject_id, policy.trust_floor],
        |row| {
            let metadata_json: Option<String> = row.get(2)?;
            Ok(CandidateClaim {
                claim_id: row.get(0)?,
                freshness_at: row
                    .get::<_, Option<String>>(1)?
                    .as_deref()
                    .and_then(parse_datetime),
                evidence_signature: claim_evidence_signature(&metadata_json),
            })
        },
    )?;

    let cutoff = (policy.freshness_window_secs > 0)
        .then(|| now - Duration::seconds(policy.freshness_window_secs));
    let mut candidates = Vec::new();
    for row in rows {
        let candidate = row?;
        let effective_freshness_at = effective_candidate_freshness_at(input, &candidate);
        if cutoff
            .zip(effective_freshness_at)
            .is_some_and(|(cutoff, freshness_at)| freshness_at < cutoff)
        {
            continue;
        }
        candidates.push(candidate);
        if candidates.len() >= input.max_candidates.unwrap_or(DEFAULT_MAX_CANDIDATES) {
            break;
        }
    }
    Ok(candidates)
}

fn trigger_ref_for_input(
    input: &TriggerScanInput,
    policy: TriggerPolicy,
    now: DateTime<Utc>,
) -> TriggerRef {
    TriggerRef {
        trigger_kind: policy.trigger_kind,
        at: input.source_asof.as_ref().cloned().unwrap_or(now),
        source: trigger_source_token(input.trigger_class).to_string(),
    }
}

fn trigger_signal_value(
    run_id: &str,
    input: &TriggerScanInput,
    policy: TriggerPolicy,
) -> Result<String, TriggerError> {
    Ok(serde_json::to_string(&json!({
        "runId": run_id,
        "policyVersion": TRIGGER_POLICY_VERSION,
        "triggerClass": trigger_class_storage(input.trigger_class),
        "triggerKind": trigger_kind_storage(policy.trigger_kind),
        "subject": {
            "kind": input.subject_kind,
            "id": input.subject_id,
        },
        "reasonCode": policy.reason_code,
        "triggerDisposition": policy.trigger_disposition.as_str(),
        "surfaceClass": surface_class_for_disposition(policy.trigger_disposition).as_str(),
    }))?)
}

fn surface_class_for_disposition(disposition: TriggerDisposition) -> SurfaceClass {
    match disposition {
        TriggerDisposition::SilentPrepare => SurfaceClass::Background,
        TriggerDisposition::PrimaryCandidate => SurfaceClass::Primary,
        TriggerDisposition::ReviewCandidate => SurfaceClass::Review,
        TriggerDisposition::QuietCandidate => SurfaceClass::Quiet,
    }
}

fn evidence_signature_changed(input: &TriggerScanInput, candidate: &CandidateClaim) -> bool {
    input
        .evidence_signature
        .as_deref()
        .is_some_and(|signature| candidate.evidence_signature.as_deref() != Some(signature))
}

fn subject_version_changed_since_latest_trigger(
    conn: &rusqlite::Connection,
    suppression_key: &str,
    current_subject_version: Option<i64>,
) -> Result<bool, TriggerError> {
    let Some(current_subject_version) = current_subject_version else {
        return Ok(false);
    };
    let previous_subject_version = conn
        .query_row(
            "SELECT subject_version
               FROM triggers_log
              WHERE policy_version = ?1
                AND suppression_key = ?2
                AND status = 'completed'
                AND subject_version IS NOT NULL
              ORDER BY COALESCE(completed_at, updated_at, started_at) DESC
              LIMIT 1",
            params![TRIGGER_POLICY_VERSION, suppression_key],
            |row| row.get::<_, i64>(0),
        )
        .optional()?;

    Ok(previous_subject_version.is_some_and(|previous| previous != current_subject_version))
}

fn dedupe_key(
    input: &TriggerScanInput,
    policy: TriggerPolicy,
    now: DateTime<Utc>,
    run_id: &str,
) -> String {
    let metadata_suffix = metadata_dedupe_suffix(input);
    match input.trigger_class {
        TriggerClass::ScheduledFreshness => format!(
            "{}:{}:{}:{}:{}:{}:{}",
            trigger_class_storage(input.trigger_class),
            policy.reason_code,
            input.subject_kind,
            input.subject_id,
            input.render_surface.as_str(),
            input.surface_class.as_str(),
            scheduled_day_dedupe_suffix(now, &metadata_suffix)
        ),
        TriggerClass::ManualRefresh => {
            format!("{}:{}", trigger_class_storage(input.trigger_class), run_id)
        }
        TriggerClass::EventInvalidation | TriggerClass::SourceChange => format!(
            "{}:{}:{}:{}:{}",
            trigger_class_storage(input.trigger_class),
            safe_optional_ref("signal_ref", input.source_signal_id.as_deref())
                .unwrap_or_else(|| "signal_ref_none".to_string()),
            input.subject_kind,
            input.subject_id,
            metadata_suffix
        ),
        _ => format!(
            "{}:{}:{}:{}:{}",
            trigger_class_storage(input.trigger_class),
            policy.reason_code,
            input.subject_kind,
            input.subject_id,
            metadata_suffix
        ),
    }
}

fn effective_candidate_freshness_at(
    input: &TriggerScanInput,
    candidate: &CandidateClaim,
) -> Option<DateTime<Utc>> {
    match input.trigger_class {
        TriggerClass::EventInvalidation | TriggerClass::SourceChange => {
            max_datetime(candidate.freshness_at, input.source_asof)
        }
        TriggerClass::ScheduledFreshness
        | TriggerClass::ManualRefresh
        | TriggerClass::EntityChange
        | TriggerClass::ClaimChange
        | TriggerClass::OpenLoopChange
        | TriggerClass::MeetingWindow
        | TriggerClass::DecisionWindow
        | TriggerClass::FeedbackEcho => candidate.freshness_at,
    }
}

fn scheduled_day_dedupe_suffix(now: DateTime<Utc>, metadata_suffix: &str) -> String {
    format!("day:{}:{}", now.date_naive(), metadata_suffix)
}

fn max_datetime(
    left: Option<DateTime<Utc>>,
    right: Option<DateTime<Utc>>,
) -> Option<DateTime<Utc>> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.max(right)),
        (Some(value), None) | (None, Some(value)) => Some(value),
        (None, None) => None,
    }
}

fn metadata_dedupe_suffix(input: &TriggerScanInput) -> String {
    if input.source_asof.is_none()
        && input.evidence_signature.is_none()
        && input.subject_version.is_none()
    {
        return "meta_none".to_string();
    }

    let mut hasher = Sha256::new();
    if let Some(source_asof) = input.source_asof.as_ref() {
        hasher.update(b"source_asof");
        hasher.update(source_asof.to_rfc3339().as_bytes());
    }
    if let Some(evidence_signature) = input.evidence_signature.as_deref() {
        hasher.update(b"evidence_signature");
        hasher.update(evidence_signature.as_bytes());
    }
    if let Some(subject_version) = input.subject_version {
        hasher.update(b"subject_version");
        hasher.update(subject_version.to_string().as_bytes());
    }

    format!("meta_{}", hex::encode(&hasher.finalize()[..16]))
}

fn suppression_key(input: &TriggerScanInput, policy: TriggerPolicy) -> String {
    match input.trigger_class {
        TriggerClass::EventInvalidation => format!(
            "signal:{}:{}:{}",
            safe_optional_token("signal_type", input.source_signal_type.as_deref())
                .unwrap_or_else(|| "signal_type_unknown".to_string()),
            input.subject_kind,
            input.subject_id
        ),
        TriggerClass::SourceChange => format!(
            "source:{}:{}:{}",
            safe_optional_ref("evidence", input.evidence_signature.as_deref())
                .unwrap_or_else(|| "evidence_unknown".to_string()),
            input.subject_kind,
            input.subject_id
        ),
        _ => format!(
            "{}:{}:{}",
            policy.reason_code, input.subject_kind, input.subject_id
        ),
    }
}

fn result_kind_for_completion(
    disposition: TriggerDisposition,
    surfacing_decision_ids: &[String],
) -> &'static str {
    if surfacing_decision_ids.is_empty() {
        return "prepared_silently";
    }
    match disposition {
        TriggerDisposition::SilentPrepare => "prepared_silently",
        TriggerDisposition::PrimaryCandidate => "render_decision_recorded",
        TriggerDisposition::ReviewCandidate => "held_for_review",
        TriggerDisposition::QuietCandidate => "stayed_quiet",
    }
}

fn safe_optional_token(prefix: &str, value: Option<&str>) -> Option<String> {
    value.map(|value| safe_storage_token(prefix, value))
}

fn safe_optional_ref(prefix: &str, value: Option<&str>) -> Option<String> {
    value.map(|value| safe_storage_ref(prefix, value))
}

fn safe_storage_token(prefix: &str, value: &str) -> String {
    let value = value.trim();
    if is_safe_storage_token(value) {
        value.to_string()
    } else {
        hashed_storage_ref(prefix, value)
    }
}

fn safe_storage_ref(prefix: &str, value: &str) -> String {
    let value = value.trim();
    if is_safe_storage_ref(value) {
        value.to_string()
    } else {
        hashed_storage_ref(prefix, value)
    }
}

fn is_safe_storage_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b':' | b'.'))
}

fn is_safe_storage_ref(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 160
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b':' | b'.'))
}

fn hashed_storage_ref(prefix: &str, value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(prefix.as_bytes());
    hasher.update(b":");
    hasher.update(value.as_bytes());
    format!("{prefix}_{}", hex::encode(&hasher.finalize()[..16]))
}

fn table_exists(conn: &rusqlite::Connection, table: &str) -> Result<bool, TriggerError> {
    conn.query_row(
        "SELECT COUNT(*)
           FROM sqlite_master
          WHERE type = 'table' AND name = ?1",
        [table],
        |row| row.get::<_, i64>(0).map(|count| count > 0),
    )
    .map_err(TriggerError::from)
}

fn decode_string_vec(raw: String) -> Vec<String> {
    serde_json::from_str::<Vec<String>>(&raw).unwrap_or_default()
}

fn parse_datetime(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .or_else(|_| {
            chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S")
                .map(|value| value.and_utc())
        })
        .ok()
}

fn closed_error_code(error: &SurfacingError) -> &'static str {
    match error {
        SurfacingError::UnsupportedSchemaVersion(_) => "unsupported_schema_version",
        SurfacingError::ClaimNotFound(_) => "claim_not_found",
        SurfacingError::NotRecommendationClaim(_) => "not_recommendation_claim",
        SurfacingError::ClaimNotVisible(_) => "claim_not_visible",
        SurfacingError::MissingStoredSalience(_) => "missing_stored_salience",
        SurfacingError::Database(_) => "database_error",
        SurfacingError::Service(_) => "service_error",
        SurfacingError::Salience(_) => "salience_error",
        SurfacingError::Signal(_) => "signal_error",
        SurfacingError::Serialization(_) => "serialization_error",
    }
}

fn trigger_class_storage(trigger_class: TriggerClass) -> &'static str {
    match trigger_class {
        TriggerClass::ScheduledFreshness => "scheduled_freshness",
        TriggerClass::EventInvalidation => "event_invalidation",
        TriggerClass::ManualRefresh => "manual_refresh",
        TriggerClass::EntityChange => "entity_change",
        TriggerClass::ClaimChange => "claim_change",
        TriggerClass::SourceChange => "source_change",
        TriggerClass::OpenLoopChange => "open_loop_change",
        TriggerClass::MeetingWindow => "meeting_window",
        TriggerClass::DecisionWindow => "decision_window",
        TriggerClass::FeedbackEcho => "feedback_echo",
    }
}

fn trigger_kind_storage(trigger_kind: TriggerKind) -> &'static str {
    match trigger_kind {
        TriggerKind::SignalArrival => "signal_arrival",
        TriggerKind::EntityChange => "entity_change",
        TriggerKind::ScheduledScan => "scheduled_scan",
        TriggerKind::FeedbackEcho => "feedback_echo",
    }
}

fn trigger_source_token(trigger_class: TriggerClass) -> &'static str {
    match trigger_class {
        TriggerClass::ScheduledFreshness => "scheduled_scan",
        TriggerClass::EventInvalidation => "signal_event",
        TriggerClass::ManualRefresh => "manual_refresh",
        TriggerClass::EntityChange => "entity_change",
        TriggerClass::ClaimChange => "claim_change",
        TriggerClass::SourceChange => "source_change",
        TriggerClass::OpenLoopChange => "open_loop_change",
        TriggerClass::MeetingWindow => "meeting_window",
        TriggerClass::DecisionWindow => "decision_window",
        TriggerClass::FeedbackEcho => "feedback_echo",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use chrono::TimeZone;

    use abilities_runtime::abilities::provenance::subject::SubjectRef;
    use abilities_runtime::abilities::trust::types::TrustBand;
    use abilities_runtime::types::{ClaimSensitivity, TemporalScope};

    use crate::abilities::claims::ClaimType;
    use crate::services::claims::{
        commit_claim, update_claim_trust, ClaimProposal, DeterministicInsertProposal, TrustScore,
    };
    use crate::services::context::{ExternalClients, FixedClock, SeedableRng};
    use crate::services::recommendations::contracts::{
        EvidenceRef, FactorRationale, RecommendationDraft, RecommendedAction, SalienceFactor,
        SalienceFactorKind, SalienceScore,
    };
    use crate::services::recommendations::recommendation::metadata_envelope;

    fn test_db() -> ActionDb {
        ActionDb::from_connection_for_tests(crate::migrations::migrated_in_memory_for_tests())
    }

    fn test_ctx() -> (FixedClock, SeedableRng, ExternalClients) {
        (
            FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 26, 12, 0, 0).unwrap()),
            SeedableRng::new(23),
            ExternalClients::default(),
        )
    }

    fn live_ctx<'a>(
        clock: &'a FixedClock,
        rng: &'a SeedableRng,
        external: &'a ExternalClients,
    ) -> ServiceContext<'a> {
        ServiceContext::test_live(clock, rng, external).with_actor("system:test")
    }

    fn insert_recommendation_claim(db: &ActionDb, id: &str, trust_score: f64, source_asof: &str) {
        let (clock, rng, external) = test_ctx();
        let ctx = live_ctx(&clock, &rng, &external);
        let draft = RecommendationDraft {
            subject: SubjectRef::Account("acct-example".to_string()),
            recommended_action: RecommendedAction::ReviewClaim {
                claim_id: ClaimId("claim-source".to_string()),
                reason: "verify".to_string(),
            },
            evidence: vec![EvidenceRef {
                source: "claim:source-1".to_string(),
                chunk: None,
            }],
            provenance_json: r#"{"sources":[]}"#.to_string(),
            source_ref: Some("run:test".to_string()),
            source_asof: Some(source_asof.parse::<DateTime<Utc>>().unwrap()),
            observed_at: "2026-05-26T10:00:00Z".parse::<DateTime<Utc>>().unwrap(),
            text: "Review the account before the next customer conversation.".to_string(),
            salience: SalienceScore {
                total: 0.75,
                factors: vec![SalienceFactor {
                    kind: SalienceFactorKind::Trust,
                    value: Some(0.9),
                    weight: 0.1,
                    rationale: FactorRationale::Trust {
                        trust_band: TrustBand::LikelyCurrent,
                    },
                }],
            },
        };
        let proposal = ClaimProposal {
            id: None,
            expected_claim_version: None,
            subject_ref: r#"{"kind":"account","id":"acct-example"}"#.to_string(),
            claim_type: ClaimType::Recommendation.as_str().to_string(),
            field_path: Some("recommendation.reviewClaim".to_string()),
            topic_key: Some("reviewClaim".to_string()),
            text: draft.text.clone(),
            actor: "agent".to_string(),
            data_source: "recommendation".to_string(),
            source_ref: Some("run:test".to_string()),
            source_asof: Some(source_asof.to_string()),
            observed_at: "2026-05-26T10:00:00Z".to_string(),
            provenance_json: r#"{"sources":[]}"#.to_string(),
            metadata_json: Some(serde_json::to_string(&metadata_envelope(&draft)).unwrap()),
            thread_id: None,
            temporal_scope: Some(TemporalScope::State),
            sensitivity: Some(ClaimSensitivity::Internal),
            supersedes: None,
            tombstone: None,
        };
        commit_claim(
            &ctx,
            db,
            DeterministicInsertProposal::new(id.to_string(), proposal),
        )
        .expect("insert recommendation claim");
        update_claim_trust(db, id, TrustScore(trust_score), 1, &ctx).expect("seed trust score");
    }

    fn stored_evidence_signature(db: &ActionDb, claim_id: &str) -> String {
        let metadata_json: Option<String> = db
            .conn_ref()
            .query_row(
                "SELECT metadata_json FROM intelligence_claims WHERE id = ?1",
                [claim_id],
                |row| row.get(0),
            )
            .expect("read recommendation metadata");
        claim_evidence_signature(&metadata_json).expect("metadata has evidence signature")
    }

    #[test]
    fn trigger_policy_defaults_match_l0_contract() {
        let cases = [
            (
                TriggerClass::ScheduledFreshness,
                TriggerKind::ScheduledScan,
                TriggerDisposition::SilentPrepare,
                0.60,
                86_400,
            ),
            (
                TriggerClass::EventInvalidation,
                TriggerKind::SignalArrival,
                TriggerDisposition::PrimaryCandidate,
                0.70,
                21_600,
            ),
            (
                TriggerClass::ManualRefresh,
                TriggerKind::SignalArrival,
                TriggerDisposition::ReviewCandidate,
                0.50,
                0,
            ),
            (
                TriggerClass::EntityChange,
                TriggerKind::EntityChange,
                TriggerDisposition::PrimaryCandidate,
                0.65,
                21_600,
            ),
            (
                TriggerClass::ClaimChange,
                TriggerKind::SignalArrival,
                TriggerDisposition::PrimaryCandidate,
                0.70,
                21_600,
            ),
            (
                TriggerClass::SourceChange,
                TriggerKind::SignalArrival,
                TriggerDisposition::ReviewCandidate,
                0.75,
                21_600,
            ),
            (
                TriggerClass::OpenLoopChange,
                TriggerKind::SignalArrival,
                TriggerDisposition::QuietCandidate,
                0.55,
                43_200,
            ),
            (
                TriggerClass::MeetingWindow,
                TriggerKind::EntityChange,
                TriggerDisposition::PrimaryCandidate,
                0.65,
                7_200,
            ),
            (
                TriggerClass::DecisionWindow,
                TriggerKind::ScheduledScan,
                TriggerDisposition::PrimaryCandidate,
                0.70,
                43_200,
            ),
            (
                TriggerClass::FeedbackEcho,
                TriggerKind::FeedbackEcho,
                TriggerDisposition::QuietCandidate,
                0.50,
                604_800,
            ),
        ];

        for (trigger_class, trigger_kind, trigger_disposition, trust_floor, freshness_window) in
            cases
        {
            let policy = policy_for_trigger(trigger_class);
            assert_eq!(policy.trigger_kind, trigger_kind);
            assert_eq!(policy.trigger_disposition, trigger_disposition);
            assert!((policy.trust_floor - trust_floor).abs() < f64::EPSILON);
            assert_eq!(policy.freshness_window_secs, freshness_window);
        }
    }

    #[test]
    fn scheduled_refresh_logs_trigger_and_invokes_w2a_evaluation() {
        let db = test_db();
        insert_recommendation_claim(&db, "claim-trigger-1", 0.95, "2026-05-26T10:00:00Z");
        let (clock, rng, external) = test_ctx();
        let ctx = live_ctx(&clock, &rng, &external);
        let engine = PropagationEngine::new();

        let outcome = run_trigger_scan(
            &ctx,
            &db,
            &engine,
            TriggerScanInput::scheduled("account", "acct-example"),
        )
        .expect("trigger scan succeeds");

        assert_eq!(outcome.status, TriggerScanStatus::Completed);
        assert_eq!(outcome.candidate_claim_ids, vec!["claim-trigger-1"]);
        assert_eq!(outcome.salience_evaluation_ids.len(), 1);
        assert_eq!(outcome.surfacing_decision_ids.len(), 1);
        let stored_salience_ids: String = db
            .conn_ref()
            .query_row(
                "SELECT salience_evaluation_ids_json FROM triggers_log WHERE run_id = ?1",
                [&outcome.run_id],
                |row| row.get(0),
            )
            .expect("read trigger log salience ids");
        assert!(stored_salience_ids.contains(&outcome.salience_evaluation_ids[0]));
        let decision_salience_id: String = db
            .conn_ref()
            .query_row(
                "SELECT salience_evaluation_id FROM surfacing_decisions WHERE id = ?1",
                [&outcome.surfacing_decision_ids[0]],
                |row| row.get(0),
            )
            .expect("read surfacing decision");
        assert_eq!(decision_salience_id, outcome.salience_evaluation_ids[0]);
        let decision_surface_class: String = db
            .conn_ref()
            .query_row(
                "SELECT surface_class FROM surfacing_decisions WHERE id = ?1",
                [&outcome.surfacing_decision_ids[0]],
                |row| row.get(0),
            )
            .expect("read surfacing class");
        assert_eq!(decision_surface_class, "background");
        let (trigger_disposition, result_kind, downstream_policy): (String, String, String) = db
            .conn_ref()
            .query_row(
                "SELECT trigger_disposition, result_kind, downstream_policy
                   FROM triggers_log
                  WHERE run_id = ?1",
                [&outcome.run_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("read trigger disposition");
        assert_eq!(trigger_disposition, "silent_prepare");
        assert_eq!(result_kind, "prepared_silently");
        assert_eq!(downstream_policy, DOWNSTREAM_POLICY);
    }

    #[test]
    fn duplicate_completed_scan_coalesces_without_new_log_row() {
        let db = test_db();
        insert_recommendation_claim(&db, "claim-trigger-dup", 0.95, "2026-05-26T10:00:00Z");
        let (clock, rng, external) = test_ctx();
        let ctx = live_ctx(&clock, &rng, &external);
        let engine = PropagationEngine::new();
        let input = TriggerScanInput::scheduled("account", "acct-example");

        let first = run_trigger_scan(&ctx, &db, &engine, input.clone()).expect("first scan");
        let second = run_trigger_scan(&ctx, &db, &engine, input).expect("second scan");

        assert_eq!(second.status, TriggerScanStatus::CoalescedCompleted);
        assert_eq!(second.run_id, first.run_id);
        let log_rows: i64 = db
            .conn_ref()
            .query_row("SELECT COUNT(*) FROM triggers_log", [], |row| row.get(0))
            .expect("count trigger logs");
        assert_eq!(log_rows, 1);
    }

    #[test]
    fn scheduled_refresh_dedupe_is_per_day() {
        let db = test_db();
        insert_recommendation_claim(&db, "claim-trigger-daily", 0.95, "2026-05-26T10:00:00Z");
        let rng = SeedableRng::new(17);
        let external = ExternalClients::default();
        let first_clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 26, 23, 59, 0).unwrap());
        let second_clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 27, 0, 1, 0).unwrap());
        let first_ctx = live_ctx(&first_clock, &rng, &external);
        let second_ctx = live_ctx(&second_clock, &rng, &external);
        let engine = PropagationEngine::new();
        let input = TriggerScanInput::scheduled("account", "acct-example");

        let first = run_trigger_scan(&first_ctx, &db, &engine, input.clone()).expect("first scan");
        let second = run_trigger_scan(&second_ctx, &db, &engine, input).expect("second scan");

        assert_eq!(first.status, TriggerScanStatus::Completed);
        assert_eq!(second.status, TriggerScanStatus::Completed);
        assert_ne!(first.run_id, second.run_id);
        let log_rows: i64 = db
            .conn_ref()
            .query_row("SELECT COUNT(*) FROM triggers_log", [], |row| row.get(0))
            .expect("count trigger logs");
        assert_eq!(log_rows, 2);
    }

    #[test]
    fn manual_refresh_dedupe_uses_explicit_run_id() {
        let db = test_db();
        insert_recommendation_claim(&db, "claim-trigger-manual", 0.95, "2026-05-20T10:00:00Z");
        let (clock, rng, external) = test_ctx();
        let ctx = live_ctx(&clock, &rng, &external);
        let engine = PropagationEngine::new();
        let mut first = TriggerScanInput::scheduled("account", "acct-example");
        first.trigger_class = TriggerClass::ManualRefresh;
        first.run_id = Some("manual-run-1".to_string());
        let mut second = first.clone();
        second.run_id = Some("manual-run-2".to_string());

        let first_outcome = run_trigger_scan(&ctx, &db, &engine, first).expect("first manual run");
        let second_outcome =
            run_trigger_scan(&ctx, &db, &engine, second).expect("second manual run");

        assert_eq!(first_outcome.status, TriggerScanStatus::Completed);
        assert_eq!(second_outcome.status, TriggerScanStatus::Completed);
        assert_ne!(first_outcome.run_id, second_outcome.run_id);
        let log_rows: i64 = db
            .conn_ref()
            .query_row("SELECT COUNT(*) FROM triggers_log", [], |row| row.get(0))
            .expect("count trigger logs");
        assert_eq!(log_rows, 2);
    }

    #[test]
    fn unchanged_evidence_signature_does_not_override_recent_feedback() {
        let db = test_db();
        insert_recommendation_claim(&db, "claim-trigger-feedback", 0.95, "2026-05-26T10:00:00Z");
        let signature = stored_evidence_signature(&db, "claim-trigger-feedback");
        db.conn_ref()
            .execute(
                "INSERT INTO claim_feedback (
                     id, claim_id, feedback_type, actor, actor_id, submitted_at
                 ) VALUES (
                     'feedback-trigger-suppress-1', 'claim-trigger-feedback',
                     'not_relevant_here', 'user', 'user-1', '2026-05-26T11:00:00Z'
                 )",
                [],
            )
            .expect("insert feedback");
        let (clock, rng, external) = test_ctx();
        let ctx = live_ctx(&clock, &rng, &external);
        let engine = PropagationEngine::new();
        let mut input = TriggerScanInput::scheduled("account", "acct-example");
        input.trigger_class = TriggerClass::SourceChange;
        input.run_id = Some("trigger-source-same-evidence".to_string());
        input.evidence_signature = Some(signature);

        let outcome = run_trigger_scan(&ctx, &db, &engine, input).expect("source trigger");

        assert_eq!(outcome.status, TriggerScanStatus::Completed);
        let (decision_kind, suppress_reason): (String, Option<String>) = db
            .conn_ref()
            .query_row(
                "SELECT decision_kind, suppress_reason
                   FROM surfacing_decisions
                  WHERE id = ?1",
                [&outcome.surfacing_decision_ids[0]],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("read surfacing decision");
        assert_eq!(decision_kind, "suppress");
        assert_eq!(suppress_reason.as_deref(), Some("dismissed_recently"));
    }

    #[test]
    fn entity_change_subject_version_breaks_trigger_dedupe() {
        let db = test_db();
        insert_recommendation_claim(&db, "claim-trigger-versioned", 0.95, "2026-05-26T10:00:00Z");
        let (clock, rng, external) = test_ctx();
        let ctx = live_ctx(&clock, &rng, &external);
        let engine = PropagationEngine::new();
        let mut first = TriggerScanInput::scheduled("account", "acct-example");
        first.trigger_class = TriggerClass::EntityChange;
        first.run_id = Some("trigger-entity-version-1".to_string());
        first.subject_version = Some(1);
        let mut second = first.clone();
        second.run_id = Some("trigger-entity-version-2".to_string());
        second.subject_version = Some(2);

        let first_outcome = run_trigger_scan(&ctx, &db, &engine, first).expect("first trigger");
        let second_outcome = run_trigger_scan(&ctx, &db, &engine, second).expect("second trigger");

        assert_eq!(first_outcome.status, TriggerScanStatus::Completed);
        assert_eq!(second_outcome.status, TriggerScanStatus::Completed);
        assert_ne!(first_outcome.run_id, second_outcome.run_id);
        let log_rows: i64 = db
            .conn_ref()
            .query_row("SELECT COUNT(*) FROM triggers_log", [], |row| row.get(0))
            .expect("count trigger logs");
        assert_eq!(log_rows, 2);
    }

    #[test]
    fn stale_source_is_not_selected_for_routine_refresh() {
        let db = test_db();
        insert_recommendation_claim(&db, "claim-stale", 0.95, "2026-05-20T10:00:00Z");
        let (clock, rng, external) = test_ctx();
        let ctx = live_ctx(&clock, &rng, &external);
        let engine = PropagationEngine::new();

        let outcome = run_trigger_scan(
            &ctx,
            &db,
            &engine,
            TriggerScanInput::scheduled("account", "acct-example"),
        )
        .expect("scan succeeds");

        assert!(outcome.candidate_claim_ids.is_empty());
        assert!(outcome.salience_evaluation_ids.is_empty());
    }

    #[test]
    fn source_change_uses_trigger_freshness_for_stale_candidate() {
        let db = test_db();
        insert_recommendation_claim(
            &db,
            "claim-stale-source-change",
            0.95,
            "2026-05-20T10:00:00Z",
        );
        let (clock, rng, external) = test_ctx();
        let ctx = live_ctx(&clock, &rng, &external);
        let engine = PropagationEngine::new();
        let mut input = TriggerScanInput::scheduled("account", "acct-example");
        input.trigger_class = TriggerClass::SourceChange;
        input.run_id = Some("trigger-source-fresh-stale-claim".to_string());
        input.source_signal_id = Some("signal-source-fresh".to_string());
        input.source_asof = Some(Utc.with_ymd_and_hms(2026, 5, 26, 11, 30, 0).unwrap());

        let outcome = run_trigger_scan(&ctx, &db, &engine, input).expect("source trigger");

        assert_eq!(outcome.status, TriggerScanStatus::Completed);
        assert_eq!(
            outcome.candidate_claim_ids,
            vec!["claim-stale-source-change"]
        );
        assert_eq!(outcome.salience_evaluation_ids.len(), 1);
        let decision_surface_class: String = db
            .conn_ref()
            .query_row(
                "SELECT surface_class FROM surfacing_decisions WHERE id = ?1",
                [&outcome.surfacing_decision_ids[0]],
                |row| row.get(0),
            )
            .expect("read source-change surfacing class");
        assert_eq!(decision_surface_class, "review");
    }

    #[test]
    fn signal_emit_failure_marks_started_trigger_retryable() {
        let db = test_db();
        insert_recommendation_claim(
            &db,
            "claim-trigger-signal-error",
            0.95,
            "2026-05-26T10:00:00Z",
        );
        db.conn_ref()
            .execute("DROP TABLE signal_events", [])
            .expect("remove signal table");
        let (clock, rng, external) = test_ctx();
        let ctx = live_ctx(&clock, &rng, &external);
        let engine = PropagationEngine::new();
        let mut input = TriggerScanInput::scheduled("account", "acct-example");
        input.run_id = Some("trigger-signal-error-run".to_string());

        let error = run_trigger_scan(&ctx, &db, &engine, input).expect_err("signal emit fails");

        assert!(matches!(error, TriggerError::Signal(_)));
        let (status, error_code): (String, Option<String>) = db
            .conn_ref()
            .query_row(
                "SELECT status, error_code
                   FROM triggers_log
                  WHERE run_id = 'trigger-signal-error-run'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("read failed trigger");
        assert_eq!(status, "failed_retryable");
        assert_eq!(error_code.as_deref(), Some("signal_emit_failed"));
    }

    #[test]
    fn trigger_log_hashes_untrusted_source_fields() {
        let db = test_db();
        insert_recommendation_claim(&db, "claim-trigger-privacy", 0.95, "2026-05-20T10:00:00Z");
        let (clock, rng, external) = test_ctx();
        let ctx = live_ctx(&clock, &rng, &external);
        let engine = PropagationEngine::new();
        let mut input = TriggerScanInput::scheduled("account", "acct-example");
        input.trigger_class = TriggerClass::SourceChange;
        input.run_id = Some("trigger-privacy-run".to_string());
        input.source_signal_id = Some("/Users/example/raw-workspace-note.md".to_string());
        input.source_signal_type = Some("prompt body from provider output".to_string());
        input.evidence_signature = Some("provider output /Users/example/raw body".to_string());
        input.source_asof = Some(Utc.with_ymd_and_hms(2026, 5, 26, 11, 30, 0).unwrap());

        let outcome = run_trigger_scan(&ctx, &db, &engine, input).expect("privacy trigger");

        assert_eq!(outcome.status, TriggerScanStatus::Completed);
        let (
            source_signal_id,
            source_signal_type,
            evidence_signature,
            dedupe_key,
            suppression_key,
            result_kind,
        ): (String, String, String, String, String, String) = db
            .conn_ref()
            .query_row(
                "SELECT source_signal_id, source_signal_type, evidence_signature,
                        dedupe_key, suppression_key, result_kind
                   FROM triggers_log
                  WHERE run_id = ?1",
                [&outcome.run_id],
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
            .expect("read trigger privacy fields");
        let decision_source_signal_id: String = db
            .conn_ref()
            .query_row(
                "SELECT source_signal_id
                   FROM surfacing_decisions
                  WHERE id = ?1",
                [&outcome.surfacing_decision_ids[0]],
                |row| row.get(0),
            )
            .expect("read surfacing source signal");
        let signal_values: Vec<String> = {
            let mut stmt = db
                .conn_ref()
                .prepare(
                    "SELECT COALESCE(value, '')
                       FROM signal_events
                      WHERE data_source IN (
                          'recommendation_trigger_policy',
                          'recommendation_surfacing_policy'
                      )
                      ORDER BY created_at, id",
                )
                .expect("prepare signal value query");
            stmt.query_map([], |row| row.get::<_, String>(0))
                .expect("query signal values")
                .collect::<Result<Vec<_>, _>>()
                .expect("collect signal values")
        };

        assert!(source_signal_id.starts_with("signal_ref_"));
        assert!(decision_source_signal_id.starts_with("signal_ref_"));
        assert!(source_signal_type.starts_with("signal_type_"));
        assert!(evidence_signature.starts_with("evidence_"));
        assert_eq!(result_kind, "held_for_review");

        let persisted = format!(
            "{source_signal_id} {decision_source_signal_id} {source_signal_type} \
             {evidence_signature} {dedupe_key} {suppression_key} {}",
            signal_values.join(" ")
        );
        for forbidden in [
            "/Users/example",
            "raw-workspace-note",
            "prompt body",
            "provider output",
            "raw body",
        ] {
            assert!(
                !persisted.contains(forbidden),
                "persisted trigger artifacts leaked `{forbidden}`: {persisted}"
            );
        }
    }

    #[test]
    fn evaluate_mode_does_not_write_or_emit() {
        let db = test_db();
        insert_recommendation_claim(&db, "claim-trigger-eval", 0.95, "2026-05-26T10:00:00Z");
        let (clock, rng, _external) = test_ctx();
        let ctx = ServiceContext::new_evaluate_default(&clock, &rng).with_actor("system:test");
        let engine = PropagationEngine::new();

        let outcome = run_trigger_scan(
            &ctx,
            &db,
            &engine,
            TriggerScanInput::scheduled("account", "acct-example"),
        )
        .expect("evaluate mode returns no-op outcome");

        assert_eq!(outcome.status, TriggerScanStatus::EvaluateOnly);
        let trigger_rows: i64 = db
            .conn_ref()
            .query_row("SELECT COUNT(*) FROM triggers_log", [], |row| row.get(0))
            .expect("count trigger logs");
        let signal_rows: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM signal_events WHERE signal_type = 'salience_candidate_refresh_triggered'",
                [],
                |row| row.get(0),
            )
            .expect("count signals");
        assert_eq!(trigger_rows, 0);
        assert_eq!(signal_rows, 0);
    }
}
