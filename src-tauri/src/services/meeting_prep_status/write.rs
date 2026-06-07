//! Mutating writers for meeting prep status.
//!
//! All paths in this module are out of the ADR-0102 §3 Read-ability
//! call graph. Callers are non-Read abilities (background workers,
//! manual entity linking, user-facing write commands) — never
//! `get_daily_briefing` or any sibling read-ability.
//!
//! # Write surfaces (L0 packet §5.5)
//!
//! - [`enqueue_refresh`] — schedule a re-generate. Sets status to
//!   `Queued` (from a legal predecessor) and emits the
//!   `meeting_prep_status_changed` signal.
//! - [`record_user_authored`] — persist user-authored agenda / notes /
//!   preparation_text / hidden_attendees. Emits Decision / Commitment
//!   claims via the existing claim store (eventual consistency through
//!   the claim-lifecycle-signal substrate — AC-335.13 Part B).
//! - [`record_dismissal`] / [`clear_dismissal`] — UserSuppressed /
//!   UserDismissed lifecycle. Backs the v242 dismissals table.
//! - [`transition_status`] — explicit state-machine guarded transition
//!   primitive. Returns [`PrepStatusError::IllegalTransition`] on an
//!   illegal jump (AC-335.14).
//!
//! # Signal contract (AC-335.15)
//!
//! Every successful transition emits a
//! `meeting_prep_status_changed` signal whose value payload is the
//! JSON-encoded `{from, to, meeting_id}` tuple. Consumers re-render
//! through the existing signal substrate; this module does not
//! arrange consumer notification beyond the signal emit.

use std::collections::BTreeMap;
use std::sync::Arc;

use chrono::Duration;
use chrono::Utc;
use rusqlite::OptionalExtension;
use serde_json::json;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::{PrepStatus, PrepStatusError, RefreshReason, UserAuthoredFields};
use crate::db::ActionDb;
use crate::services::context::ServiceContext;
use crate::signals::bus::{self, SignalEmission};
use crate::state::AppState;

const SIGNAL_TYPE: &str = "meeting_prep_status_changed";
const SIGNAL_SOURCE: &str = "service:meeting_prep_status";
const STARTUP_PREP_REGEN_DRAIN_LIMIT: usize = 100;
const PREP_REGEN_WORKER_IDLE_POLL_MS: u64 = 250;
const PREP_REGEN_WORKER_ERROR_POLL_MS: u64 = 2_000;
const PREP_REGEN_RUNNING_LEASE_SECONDS: i64 = 300;

struct SerializedUserAuthoredFields<'a> {
    agenda: Option<&'a str>,
    notes: Option<&'a str>,
    preparation_text: Option<&'a str>,
    hidden_attendees_json: Option<String>,
    decisions_json: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RecordUserAuthoredOptions {
    pub update_hidden_attendees: bool,
    pub update_decisions: bool,
}

impl RecordUserAuthoredOptions {
    fn infer_from_fields(fields: &UserAuthoredFields) -> Self {
        Self {
            update_hidden_attendees: !fields.hidden_attendees.is_empty(),
            update_decisions: !fields.decisions.is_empty(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrepCorrectionReplayReport {
    pub rebuild_replay_id: String,
    pub replayed_entries: usize,
    pub orphaned_entries: usize,
    pub skipped_entries: usize,
}

#[derive(Debug, Clone)]
struct PrepReplayJournalRow {
    id: String,
    meeting_stable_key: String,
    meeting_id: Option<String>,
    field_path: String,
    payload_json: String,
}

#[derive(Debug, Clone)]
struct ReplayUserAuthoredFields {
    fields: UserAuthoredFields,
    options: RecordUserAuthoredOptions,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrepRegenerationProcessOutcome {
    NoJob,
    Completed { job_id: String },
    Stale { job_id: String },
    RetryScheduled { job_id: String },
    DeadLettered { job_id: String },
}

#[derive(Debug, Clone)]
struct PrepRegenerationJob {
    id: String,
    meeting_stable_key: String,
    retry_count: i64,
    max_attempts: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PrepRegenerationCompletion {
    Completed(&'static str),
    Stale(&'static str),
}

/// Enqueue a prep refresh for `meeting_id`.
///
/// Persists the queued state in `meeting_prep_status_dismissals` is
/// NOT the right home; the v241 view derives Queued/Running/Failed
/// from the existing prep_invalidation queue substrate. W1 surfaces
/// the API; the queue plumbing lands at Stage 1c when the meeting
/// prep queue read API stabilizes (the existing `meeting_prep_queue`
/// module is the producer).
///
/// For W1 this primitive:
/// - validates that `from → Queued` is a legal transition,
/// - delegates queue enqueue to the existing `meeting_prep_queue`
///   producer (callers must follow up with a queue push — left
///   explicit so consumers can decide their priority + dedup window),
/// - emits the `meeting_prep_status_changed` signal so subscribers
///   re-render.
pub fn enqueue_refresh(
    meeting_id: &str,
    from: PrepStatus,
    _reason: RefreshReason,
    db: &ActionDb,
) -> Result<(), PrepStatusError> {
    transition_status(meeting_id, from, PrepStatus::Queued, db)
}

/// Persist user-authored agenda / notes / preparation_text /
/// hidden_attendees for `meeting_id` (AC-335.8 — survives recompute).
///
/// # AC-335.13 contract
///
/// - **Part A (commutes):** Plain user-authored fields written to
///   `meeting_prep` columns disjoint from the status/queue columns.
///   The property test in `mod.rs` `state_machine_tests` covers this.
/// - **Part B (eventual consistency):** Decision / Commitment claim
///   emission goes through the existing claim store substrate. This
///   module wires the persistence (agenda/notes etc.); the
///   decision-claim emission is handled by the standard commitment /
///   decision claim writer ([`crate::services::claims`] +
///   [`crate::services::commitment_bridge`]). Status recompute
///   observes the new claim through the standard claim-lifecycle-
///   signal → invalidation path.
pub fn record_user_authored(
    ctx: &ServiceContext<'_>,
    meeting_id: &str,
    fields: &UserAuthoredFields,
    db: &ActionDb,
) -> Result<(), PrepStatusError> {
    record_user_authored_with_options(
        ctx,
        meeting_id,
        fields,
        RecordUserAuthoredOptions::infer_from_fields(fields),
        db,
    )
}

pub fn record_user_authored_with_options(
    ctx: &ServiceContext<'_>,
    meeting_id: &str,
    fields: &UserAuthoredFields,
    options: RecordUserAuthoredOptions,
    db: &ActionDb,
) -> Result<(), PrepStatusError> {
    authorize_user_authored_actor(ctx)?;
    let now = ctx.clock.now().to_rfc3339();
    let serialized = serialize_user_authored_fields(fields, options)?;

    db.with_transaction(|tx| {
        let conn = tx.conn_ref();
        // Upsert into meeting_prep. `meeting_id` is PK; if the row
        // does not yet exist we insert; otherwise we update the
        // disjoint user-authored columns ONLY. Status / queue columns
        // are untouched — AC-335.13 Part A.
        conn.execute(
            "INSERT INTO meeting_prep (
                meeting_id,
                user_agenda_json,
                user_notes,
                user_preparation_text,
                user_hidden_attendees_json,
                user_decisions_json
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(meeting_id) DO UPDATE SET
                user_agenda_json = excluded.user_agenda_json,
                user_notes       = excluded.user_notes,
                user_preparation_text = COALESCE(
                    excluded.user_preparation_text,
                    meeting_prep.user_preparation_text
                ),
                user_hidden_attendees_json = COALESCE(
                    excluded.user_hidden_attendees_json,
                    meeting_prep.user_hidden_attendees_json
                ),
                user_decisions_json = COALESCE(
                    excluded.user_decisions_json,
                    meeting_prep.user_decisions_json
                )",
            rusqlite::params![
                meeting_id,
                serialized.agenda,
                serialized.notes,
                serialized.preparation_text,
                serialized.hidden_attendees_json.as_deref(),
                serialized.decisions_json.as_deref()
            ],
        )
        .map_err(|e| e.to_string())?;
        persist_prep_correction_replay_artifacts(tx, meeting_id, fields, options, &now)
            .map_err(|e| e.to_string())?;
        Ok(())
    })
    .map_err(PrepStatusError::Db)?;

    // Emit the signal so subscribers re-render. The transition itself
    // (Ready → Ready) is a no-op at the status level; the signal
    // carries the user_authored-changed event so consumers know to
    // re-render. Use a no-op transition payload (status unchanged) so
    // the receiver can distinguish from a state-machine transition.
    emit_status_signal(meeting_id, None, None, "user_authored_changed", db, now)?;
    Ok(())
}

pub fn replay_active_prep_correction_journal(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    rebuild_replay_id: &str,
) -> Result<PrepCorrectionReplayReport, PrepStatusError> {
    ctx.check_mutation_allowed()
        .map_err(|error| PrepStatusError::Db(error.to_string()))?;
    let rows = load_active_replay_rows(db)?;
    let mut report = PrepCorrectionReplayReport {
        rebuild_replay_id: rebuild_replay_id.to_string(),
        replayed_entries: 0,
        orphaned_entries: 0,
        skipped_entries: 0,
    };

    let resolver = MeetingReplayResolver::load(db)?;
    let mut rows_by_meeting = BTreeMap::<String, Vec<PrepReplayJournalRow>>::new();
    for row in rows {
        match resolver.resolve(&row) {
            Some(meeting_id) => rows_by_meeting.entry(meeting_id).or_default().push(row),
            None => {
                mark_prep_replay_orphaned(db, &row.id, rebuild_replay_id, "meeting_not_rebuilt")?;
                report.orphaned_entries += 1;
            }
        };
    }

    for (meeting_id, rows) in rows_by_meeting {
        let replay = user_authored_fields_from_replay_rows(&rows)?;
        let now = ctx.clock.now().to_rfc3339();
        db.with_transaction(|tx| {
            apply_user_authored_fields_in_tx(tx, &meeting_id, &replay.fields, replay.options)
                .map_err(|error| error.to_string())?;
            mark_prep_replay_rows_replayed_in_tx(tx, &rows, rebuild_replay_id, &now)
                .map_err(|error| error.to_string())?;
            Ok(())
        })
        .map_err(PrepStatusError::Db)?;
        report.replayed_entries += rows.len();
    }

    Ok(report)
}

fn authorize_user_authored_actor(ctx: &ServiceContext<'_>) -> Result<(), PrepStatusError> {
    ctx.check_mutation_allowed()
        .map_err(|error| PrepStatusError::Db(error.to_string()))?;
    crate::services::correction_artifacts::authorize_lifecycle_actor(ctx.actor)
        .map_err(|error| PrepStatusError::Db(error.to_string()))
}

pub fn process_one_prep_regeneration_job(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    worker_id: &str,
) -> Result<PrepRegenerationProcessOutcome, PrepStatusError> {
    ctx.check_mutation_allowed()
        .map_err(|error| PrepStatusError::Db(error.to_string()))?;
    let Some(job) = claim_next_prep_regeneration_job(ctx, db, worker_id)? else {
        return Ok(PrepRegenerationProcessOutcome::NoJob);
    };
    let job_id = job.id.clone();
    let now = ctx.clock.now().to_rfc3339();
    let completion = db.with_transaction(|tx| {
        let completion =
            run_prep_regeneration_job(ctx, tx, &job, &now).map_err(|e| e.to_string())?;
        mark_prep_regeneration_job_complete(tx, &job, completion, &now)
            .map_err(|e| e.to_string())?;
        Ok(completion)
    });

    match completion {
        Ok(PrepRegenerationCompletion::Completed(_)) => {
            Ok(PrepRegenerationProcessOutcome::Completed { job_id })
        }
        Ok(PrepRegenerationCompletion::Stale(_)) => {
            Ok(PrepRegenerationProcessOutcome::Stale { job_id })
        }
        Err(error) => mark_prep_regeneration_job_failed(ctx, db, &job, &error),
    }
}

pub async fn drain_pending_prep_regeneration_jobs(state: &Arc<AppState>) {
    let worker_id = format!("prep-regeneration-startup-{}", Uuid::new_v4());
    for _ in 0..STARTUP_PREP_REGEN_DRAIN_LIMIT {
        let worker_id = worker_id.clone();
        let result = state
            .db_write(move |db| {
                let clock = crate::services::context::SystemClock;
                let rng = crate::services::context::SystemRng;
                let ext = crate::services::context::ExternalClients::default();
                let ctx = ServiceContext::new_live(&clock, &rng, &ext);
                process_one_prep_regeneration_job(&ctx, db, &worker_id)
                    .map_err(|error| error.to_string())
            })
            .await;

        match result {
            Ok(PrepRegenerationProcessOutcome::NoJob) => break,
            Ok(outcome) => log::info!("Prep regeneration drain processed {outcome:?}"),
            Err(error) => {
                let message = error.to_string();
                state
                    .recover_db_service_after_access_error(&error, "Prep regeneration drain")
                    .await;
                log::warn!("Prep regeneration drain stopped: {message}");
                break;
            }
        }
    }
}

pub async fn run_prep_regeneration_worker(state: Arc<AppState>) {
    let worker_id = format!("prep-regeneration-worker-{}", Uuid::new_v4());
    loop {
        if state.is_database_recovery_required() {
            log::warn!("Prep regeneration worker stopped: database recovery required");
            break;
        }
        let worker_id_for_db = worker_id.clone();
        let result = state
            .db_write(move |db| {
                let clock = crate::services::context::SystemClock;
                let rng = crate::services::context::SystemRng;
                let ext = crate::services::context::ExternalClients::default();
                let ctx = ServiceContext::new_live(&clock, &rng, &ext);
                process_one_prep_regeneration_job(&ctx, db, &worker_id_for_db)
                    .map_err(|error| error.to_string())
            })
            .await;

        match result {
            Ok(PrepRegenerationProcessOutcome::NoJob) => {
                tokio::time::sleep(std::time::Duration::from_millis(
                    PREP_REGEN_WORKER_IDLE_POLL_MS,
                ))
                .await;
            }
            Ok(outcome) => log::info!("Prep regeneration worker processed {outcome:?}"),
            Err(error) => {
                let message = error.to_string();
                state
                    .recover_db_service_after_access_error(&error, "Prep regeneration worker")
                    .await;
                if error.is_retryable() && !message.contains("file is not a database") {
                    log::debug!(
                        "Prep regeneration worker iteration retrying after transient DB contention: {message}"
                    );
                } else {
                    log::warn!("Prep regeneration worker iteration failed: {message}");
                }
                tokio::time::sleep(std::time::Duration::from_millis(
                    PREP_REGEN_WORKER_ERROR_POLL_MS,
                ))
                .await;
            }
        }
    }
}

fn claim_next_prep_regeneration_job(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    worker_id: &str,
) -> Result<Option<PrepRegenerationJob>, PrepStatusError> {
    let now_value = ctx.clock.now();
    let now = now_value.to_rfc3339();
    let lease_expired_before =
        (now_value - Duration::seconds(PREP_REGEN_RUNNING_LEASE_SECONDS)).to_rfc3339();
    let run_marker = format!("{worker_id}:{}", Uuid::new_v4());
    db.with_transaction(|tx| {
        reclaim_expired_prep_regeneration_jobs(tx, &lease_expired_before, &now)
            .map_err(|error| error.to_string())?;
        let job_id = tx
            .conn_ref()
            .query_row(
                "SELECT id
                   FROM meeting_prep_regeneration_jobs
                  WHERE status = 'pending'
                    AND datetime(next_run_at) <= datetime(?1)
                  ORDER BY created_at ASC
                  LIMIT 1",
                rusqlite::params![&now],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|error| error.to_string())?;
        let Some(job_id) = job_id else {
            return Ok(None);
        };

        tx.conn_ref()
            .execute(
                "UPDATE meeting_prep_regeneration_jobs
                    SET status = 'running',
                        retry_count = retry_count + 1,
                        failure_reason_code = ?2,
                        updated_at = ?3
                  WHERE id = ?1
                    AND status = 'pending'",
                rusqlite::params![&job_id, &run_marker, &now],
            )
            .map_err(|error| error.to_string())?;
        read_prep_regeneration_job(tx, &job_id).map_err(|error| error.to_string())
    })
    .map_err(PrepStatusError::Db)
}

fn reclaim_expired_prep_regeneration_jobs(
    tx: &ActionDb,
    lease_expired_before: &str,
    now: &str,
) -> Result<(), PrepStatusError> {
    tx.conn_ref()
        .execute(
            "UPDATE meeting_prep_regeneration_jobs
                SET status = 'dead_lettered',
                    failure_reason_code = 'worker_lease_expired',
                    dead_lettered_at = ?2,
                    updated_at = ?2
              WHERE status = 'running'
                AND updated_at <= ?1
                AND retry_count >= max_attempts",
            rusqlite::params![lease_expired_before, now],
        )
        .map_err(|error| PrepStatusError::Db(error.to_string()))?;
    tx.conn_ref()
        .execute(
            "UPDATE meeting_prep_regeneration_jobs
                SET status = 'pending',
                    stale_reason = NULL,
                    failure_reason_code = 'worker_lease_expired',
                    next_run_at = ?2,
                    updated_at = ?2
              WHERE status = 'running'
                AND updated_at <= ?1
                AND retry_count < max_attempts",
            rusqlite::params![lease_expired_before, now],
        )
        .map_err(|error| PrepStatusError::Db(error.to_string()))?;
    Ok(())
}

fn read_prep_regeneration_job(
    db: &ActionDb,
    job_id: &str,
) -> Result<Option<PrepRegenerationJob>, PrepStatusError> {
    db.conn_ref()
        .query_row(
            "SELECT id, meeting_stable_key, retry_count, max_attempts
               FROM meeting_prep_regeneration_jobs
              WHERE id = ?1",
            rusqlite::params![job_id],
            |row| {
                Ok(PrepRegenerationJob {
                    id: row.get(0)?,
                    meeting_stable_key: row.get(1)?,
                    retry_count: row.get(2)?,
                    max_attempts: row.get(3)?,
                })
            },
        )
        .optional()
        .map_err(|error| PrepStatusError::Db(error.to_string()))
}

fn run_prep_regeneration_job(
    _ctx: &ServiceContext<'_>,
    tx: &ActionDb,
    job: &PrepRegenerationJob,
    now: &str,
) -> Result<PrepRegenerationCompletion, PrepStatusError> {
    let rows = load_active_replay_rows_for_stable_key(tx, &job.meeting_stable_key)?;
    if rows.is_empty() {
        return Ok(PrepRegenerationCompletion::Stale(
            "prep_regeneration_no_active_replay_rows",
        ));
    }

    let resolver = MeetingReplayResolver::load(tx)?;
    let mut rows_by_meeting = BTreeMap::<String, Vec<PrepReplayJournalRow>>::new();
    let mut orphaned = 0usize;
    for row in rows {
        match resolver.resolve(&row) {
            Some(meeting_id) => rows_by_meeting.entry(meeting_id).or_default().push(row),
            None => {
                mark_prep_replay_orphaned_in_tx(
                    tx,
                    &row.id,
                    &format!("prep-regeneration:{}", job.id),
                    "meeting_not_rebuilt",
                    now,
                )?;
                orphaned += 1;
            }
        };
    }

    let mut replayed = 0usize;
    for (meeting_id, rows) in rows_by_meeting {
        let replay = user_authored_fields_from_replay_rows(&rows)?;
        apply_user_authored_fields_in_tx(tx, &meeting_id, &replay.fields, replay.options)?;
        mark_prep_replay_rows_replayed_in_tx(
            tx,
            &rows,
            &format!("prep-regeneration:{}", job.id),
            now,
        )?;
        replayed += rows.len();
    }

    if replayed > 0 {
        Ok(PrepRegenerationCompletion::Completed(
            "prep_regeneration_replayed_user_layer",
        ))
    } else if orphaned > 0 {
        Ok(PrepRegenerationCompletion::Stale(
            "prep_regeneration_orphaned_replay_rows",
        ))
    } else {
        Ok(PrepRegenerationCompletion::Stale(
            "prep_regeneration_no_effect",
        ))
    }
}

fn mark_prep_regeneration_job_complete(
    tx: &ActionDb,
    job: &PrepRegenerationJob,
    completion: PrepRegenerationCompletion,
    now: &str,
) -> Result<(), PrepStatusError> {
    let (status, reason_code, completed_at) = match completion {
        PrepRegenerationCompletion::Completed(reason) => ("completed", reason, Some(now)),
        PrepRegenerationCompletion::Stale(reason) => ("stale", reason, None),
    };
    tx.conn_ref()
        .execute(
            "UPDATE meeting_prep_regeneration_jobs
                SET status = ?2,
                    stale_reason = CASE WHEN ?2 = 'stale' THEN ?3 ELSE NULL END,
                    failure_reason_code = NULL,
                    completed_at = ?4,
                    updated_at = ?5
              WHERE id = ?1
                AND status = 'running'",
            rusqlite::params![&job.id, status, reason_code, completed_at, now],
        )
        .map_err(|error| PrepStatusError::Db(error.to_string()))?;
    Ok(())
}

fn mark_prep_regeneration_job_failed(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    job: &PrepRegenerationJob,
    error: &str,
) -> Result<PrepRegenerationProcessOutcome, PrepStatusError> {
    let now = ctx.clock.now().to_rfc3339();
    let next_run_at = (ctx.clock.now()
        + Duration::seconds(prep_regeneration_retry_backoff_seconds(job.retry_count)))
    .to_rfc3339();
    let terminal = job.retry_count >= job.max_attempts;
    db.with_transaction(|tx| {
        if terminal {
            tx.conn_ref()
                .execute(
                    "UPDATE meeting_prep_regeneration_jobs
                        SET status = 'dead_lettered',
                            failure_reason_code = ?2,
                            dead_lettered_at = ?3,
                            updated_at = ?3
                      WHERE id = ?1",
                    rusqlite::params![&job.id, error, &now],
                )
                .map_err(|e| e.to_string())?;
        } else {
            tx.conn_ref()
                .execute(
                    "UPDATE meeting_prep_regeneration_jobs
                        SET status = 'pending',
                            failure_reason_code = ?2,
                            updated_at = ?3,
                            next_run_at = ?4
                      WHERE id = ?1",
                    rusqlite::params![&job.id, error, &now, &next_run_at],
                )
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    })
    .map_err(PrepStatusError::Db)?;

    if terminal {
        Ok(PrepRegenerationProcessOutcome::DeadLettered {
            job_id: job.id.clone(),
        })
    } else {
        Ok(PrepRegenerationProcessOutcome::RetryScheduled {
            job_id: job.id.clone(),
        })
    }
}

fn prep_regeneration_retry_backoff_seconds(attempts: i64) -> i64 {
    let exponent = attempts.saturating_sub(1).min(4) as u32;
    4_i64.pow(exponent)
}

fn load_active_replay_rows(db: &ActionDb) -> Result<Vec<PrepReplayJournalRow>, PrepStatusError> {
    let mut stmt = db
        .conn_ref()
        .prepare(
            "SELECT id, meeting_stable_key, meeting_id, field_path, payload_json
               FROM meeting_prep_correction_journal
              WHERE lifecycle_state = 'active'
              ORDER BY meeting_stable_key, field_path, updated_at",
        )
        .map_err(|error| PrepStatusError::Db(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(PrepReplayJournalRow {
                id: row.get(0)?,
                meeting_stable_key: row.get(1)?,
                meeting_id: row.get(2)?,
                field_path: row.get(3)?,
                payload_json: row.get(4)?,
            })
        })
        .map_err(|error| PrepStatusError::Db(error.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| PrepStatusError::Db(error.to_string()))?;
    Ok(rows)
}

fn load_active_replay_rows_for_stable_key(
    db: &ActionDb,
    meeting_stable_key: &str,
) -> Result<Vec<PrepReplayJournalRow>, PrepStatusError> {
    let mut stmt = db
        .conn_ref()
        .prepare(
            "SELECT id, meeting_stable_key, meeting_id, field_path, payload_json
               FROM meeting_prep_correction_journal
              WHERE lifecycle_state = 'active'
                AND meeting_stable_key = ?1
              ORDER BY field_path, updated_at",
        )
        .map_err(|error| PrepStatusError::Db(error.to_string()))?;
    let rows = stmt
        .query_map([meeting_stable_key], |row| {
            Ok(PrepReplayJournalRow {
                id: row.get(0)?,
                meeting_stable_key: row.get(1)?,
                meeting_id: row.get(2)?,
                field_path: row.get(3)?,
                payload_json: row.get(4)?,
            })
        })
        .map_err(|error| PrepStatusError::Db(error.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| PrepStatusError::Db(error.to_string()))?;
    Ok(rows)
}

fn meeting_stable_key_for_db(db: &ActionDb, meeting_id: &str) -> Result<String, PrepStatusError> {
    let row = db
        .conn_ref()
        .query_row(
            "SELECT calendar_event_id, start_time, attendees
               FROM meetings
              WHERE id = ?1",
            [meeting_id],
            |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                ))
            },
        )
        .optional()
        .map_err(|error| PrepStatusError::Db(error.to_string()))?;

    let Some((calendar_event_id, start_time, attendees_json)) = row else {
        return pii_safe_hash(
            "meeting_prep",
            "dailyos.w4.meeting_prep.stable_key",
            &["missing_meeting_id_fallback", meeting_id],
        );
    };
    meeting_stable_key_from_parts(
        meeting_id,
        calendar_event_id.as_deref(),
        &start_time,
        attendees_json.as_deref(),
    )
}

fn meeting_stable_key_from_parts(
    meeting_id: &str,
    calendar_event_id: Option<&str>,
    start_time: &str,
    attendees_json: Option<&str>,
) -> Result<String, PrepStatusError> {
    if let Some(calendar_event_id) = calendar_event_id {
        let calendar_event_id = calendar_event_id.trim();
        if !calendar_event_id.is_empty() {
            let key = pii_safe_hash(
                "meeting_prep",
                "dailyos.w4.meeting_prep.stable_key",
                &["calendar_event_id", calendar_event_id],
            )?;
            return Ok(format!("calendar:{key}"));
        }
    }
    let identity = meeting_manual_identity(start_time, attendees_json)?;
    let discriminator = pii_safe_hash(
        "meeting_prep",
        "dailyos.w4.meeting_prep.stable_key",
        &["manual_discriminator", meeting_id],
    )?;
    let strong_family = identity
        .strong_family
        .as_deref()
        .unwrap_or(&identity.loose_family);
    Ok(format!(
        "manual:{strong_family}:{}:{discriminator}",
        identity.loose_family
    ))
}

#[derive(Debug, Clone)]
struct MeetingReplayCandidate {
    id: String,
    stable_key: String,
}

#[derive(Debug, Default)]
struct MeetingReplayResolver {
    by_id: BTreeMap<String, MeetingReplayCandidate>,
    by_stable_key: BTreeMap<String, Vec<String>>,
    by_strong_family: BTreeMap<String, Vec<String>>,
    by_loose_family: BTreeMap<String, Vec<String>>,
}

impl MeetingReplayResolver {
    fn load(db: &ActionDb) -> Result<Self, PrepStatusError> {
        let mut stmt = db
            .conn_ref()
            .prepare(
                "SELECT id, calendar_event_id, start_time, attendees
                   FROM meetings
                  ORDER BY id",
            )
            .map_err(|error| PrepStatusError::Db(error.to_string()))?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            })
            .map_err(|error| PrepStatusError::Db(error.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| PrepStatusError::Db(error.to_string()))?;

        let mut resolver = Self::default();
        for (id, calendar_event_id, start_time, attendees_json) in rows {
            let Ok(stable_key) = meeting_stable_key_from_parts(
                &id,
                calendar_event_id.as_deref(),
                &start_time,
                attendees_json.as_deref(),
            ) else {
                continue;
            };
            let manual_identity = manual_identity_from_stable_key(&stable_key);
            resolver
                .by_stable_key
                .entry(stable_key.clone())
                .or_default()
                .push(id.clone());
            if let Some(identity) = manual_identity.as_ref() {
                if let Some(strong_family) = identity.strong_family.as_ref() {
                    resolver
                        .by_strong_family
                        .entry(strong_family.clone())
                        .or_default()
                        .push(id.clone());
                }
                resolver
                    .by_loose_family
                    .entry(identity.loose_family.clone())
                    .or_default()
                    .push(id.clone());
            }
            resolver
                .by_id
                .insert(id.clone(), MeetingReplayCandidate { id, stable_key });
        }
        Ok(resolver)
    }

    fn resolve(&self, row: &PrepReplayJournalRow) -> Option<String> {
        if let Some(meeting_id) = row.meeting_id.as_deref() {
            if let Some(candidate) = self.by_id.get(meeting_id) {
                if candidate.stable_key == row.meeting_stable_key {
                    return Some(candidate.id.clone());
                }
            }
        }

        if let Some(match_id) = unique_match(self.by_stable_key.get(&row.meeting_stable_key)) {
            return Some(match_id);
        }

        let stored_identity = manual_identity_from_stable_key(&row.meeting_stable_key)?;
        if let Some(strong_family) = stored_identity.strong_family.as_ref() {
            if let Some(match_id) = unique_match(self.by_strong_family.get(strong_family)) {
                return Some(match_id);
            }
        }
        unique_match(self.by_loose_family.get(&stored_identity.loose_family))
    }
}

fn unique_match(matches: Option<&Vec<String>>) -> Option<String> {
    match matches {
        Some(matches) if matches.len() == 1 => matches.first().cloned(),
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ManualMeetingIdentity {
    strong_family: Option<String>,
    loose_family: String,
}

fn meeting_manual_identity(
    start_time: &str,
    attendees_json: Option<&str>,
) -> Result<ManualMeetingIdentity, PrepStatusError> {
    let attendees = normalized_attendees_identity(attendees_json)?;
    let normalized_start_time = start_time.trim();
    let strong_family = if normalized_start_time.is_empty() {
        None
    } else {
        Some(pii_safe_hash(
            "meeting_prep",
            "dailyos.w4.meeting_prep.stable_family",
            &["manual_family_v2", normalized_start_time, &attendees],
        )?)
    };
    let loose_family = pii_safe_hash(
        "meeting_prep",
        "dailyos.w4.meeting_prep.stable_family",
        &["manual_family", &attendees],
    )?;
    Ok(ManualMeetingIdentity {
        strong_family,
        loose_family,
    })
}

fn manual_identity_from_stable_key(stable_key: &str) -> Option<ManualMeetingIdentity> {
    let mut parts = stable_key.split(':');
    match (parts.next(), parts.next(), parts.next(), parts.next()) {
        (Some("manual"), Some(loose_family), Some(_discriminator), None) => {
            Some(ManualMeetingIdentity {
                strong_family: None,
                loose_family: loose_family.to_string(),
            })
        }
        (Some("manual"), Some(strong_family), Some(loose_family), Some(_discriminator)) => {
            if parts.next().is_none() {
                Some(ManualMeetingIdentity {
                    strong_family: Some(strong_family.to_string()),
                    loose_family: loose_family.to_string(),
                })
            } else {
                None
            }
        }
        _ => None,
    }
}

fn normalized_attendees_identity(attendees_json: Option<&str>) -> Result<String, PrepStatusError> {
    let Some(raw) = attendees_json
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok("[]".to_string());
    };
    let parsed: serde_json::Value =
        serde_json::from_str(raw).map_err(|error| PrepStatusError::Db(error.to_string()))?;
    let mut attendees = parsed
        .as_array()
        .map(|values| {
            values
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_ascii_lowercase)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    attendees.sort();
    attendees.dedup();
    serde_json::to_string(&attendees).map_err(|error| PrepStatusError::Db(error.to_string()))
}

fn user_authored_fields_from_replay_rows(
    rows: &[PrepReplayJournalRow],
) -> Result<ReplayUserAuthoredFields, PrepStatusError> {
    let mut fields = UserAuthoredFields::default();
    let mut options = RecordUserAuthoredOptions::default();
    for row in rows {
        let payload = replay_payload_value(&row.payload_json)?;
        match row.field_path.as_str() {
            "user_agenda_json" => fields.agenda = optional_string_payload(payload)?,
            "user_notes" => fields.notes = optional_string_payload(payload)?,
            "preparation_text" => fields.preparation_text = optional_string_payload(payload)?,
            "hidden_attendees" => {
                options.update_hidden_attendees = true;
                fields.hidden_attendees = serde_json::from_value(payload).map_err(|error| {
                    PrepStatusError::Db(format!("invalid hidden_attendees replay payload: {error}"))
                })?;
            }
            "decisions" => {
                options.update_decisions = true;
                fields.decisions = serde_json::from_value(payload).map_err(|error| {
                    PrepStatusError::Db(format!("invalid decisions replay payload: {error}"))
                })?;
            }
            _ => {}
        }
    }
    Ok(ReplayUserAuthoredFields { fields, options })
}

fn replay_payload_value(raw: &str) -> Result<serde_json::Value, PrepStatusError> {
    let payload: serde_json::Value =
        serde_json::from_str(raw).map_err(|error| PrepStatusError::Db(error.to_string()))?;
    Ok(payload
        .get("value")
        .cloned()
        .unwrap_or(serde_json::Value::Null))
}

fn optional_string_payload(payload: serde_json::Value) -> Result<Option<String>, PrepStatusError> {
    match payload {
        serde_json::Value::Null => Ok(None),
        serde_json::Value::String(value) => Ok(Some(value)),
        other => Err(PrepStatusError::Db(format!(
            "invalid string replay payload: {other}"
        ))),
    }
}

fn mark_prep_replay_rows_replayed(
    db: &ActionDb,
    rows: &[PrepReplayJournalRow],
    rebuild_replay_id: &str,
) -> Result<(), PrepStatusError> {
    let now = Utc::now().to_rfc3339();
    db.with_transaction(|tx| {
        mark_prep_replay_rows_replayed_in_tx(tx, rows, rebuild_replay_id, &now)
            .map_err(|error| error.to_string())?;
        Ok(())
    })
    .map_err(PrepStatusError::Db)
}

fn mark_prep_replay_rows_replayed_in_tx(
    tx: &ActionDb,
    rows: &[PrepReplayJournalRow],
    rebuild_replay_id: &str,
    now: &str,
) -> Result<(), PrepStatusError> {
    for row in rows {
        tx.conn_ref()
            .execute(
                "UPDATE meeting_prep_correction_journal
                    SET lifecycle_state = 'active',
                        replay_attempt_count = replay_attempt_count + 1,
                        rebuild_replay_id = ?2,
                        replayed_at = ?3,
                        updated_at = ?3
                  WHERE id = ?1",
                rusqlite::params![&row.id, rebuild_replay_id, now],
            )
            .map_err(|error| PrepStatusError::Db(error.to_string()))?;
        tx.conn_ref()
            .execute(
                "UPDATE meeting_prep_regeneration_jobs
                    SET status = 'completed',
                        completed_at = ?2,
                        stale_reason = NULL,
                        failure_reason_code = NULL,
                        updated_at = ?2
                  WHERE journal_id = ?1
                    AND status IN ('pending', 'running', 'stale')",
                rusqlite::params![&row.id, now],
            )
            .map_err(|error| PrepStatusError::Db(error.to_string()))?;
    }
    Ok(())
}

fn mark_prep_replay_orphaned(
    db: &ActionDb,
    journal_id: &str,
    rebuild_replay_id: &str,
    reason_code: &str,
) -> Result<(), PrepStatusError> {
    let now = Utc::now().to_rfc3339();
    db.with_transaction(|tx| {
        mark_prep_replay_orphaned_in_tx(tx, journal_id, rebuild_replay_id, reason_code, &now)
            .map_err(|error| error.to_string())?;
        Ok(())
    })
    .map_err(PrepStatusError::Db)
}

fn mark_prep_replay_orphaned_in_tx(
    tx: &ActionDb,
    journal_id: &str,
    rebuild_replay_id: &str,
    reason_code: &str,
    now: &str,
) -> Result<(), PrepStatusError> {
    tx.conn_ref()
        .execute(
            "UPDATE meeting_prep_correction_journal
                SET lifecycle_state = 'orphaned',
                    replay_attempt_count = replay_attempt_count + 1,
                    rebuild_replay_id = ?2,
                    orphan_reason = ?3,
                    orphaned_at = ?4,
                    updated_at = ?4
              WHERE id = ?1",
            rusqlite::params![journal_id, rebuild_replay_id, reason_code, now],
        )
        .map_err(|error| PrepStatusError::Db(error.to_string()))?;
    tx.conn_ref()
        .execute(
            "UPDATE meeting_prep_regeneration_jobs
                SET status = 'stale',
                    stale_reason = ?2,
                    updated_at = ?3
              WHERE journal_id = ?1
                AND status IN ('pending', 'running')",
            rusqlite::params![journal_id, reason_code, now],
        )
        .map_err(|error| PrepStatusError::Db(error.to_string()))?;
    Ok(())
}

fn apply_user_authored_fields_in_tx(
    tx: &ActionDb,
    meeting_id: &str,
    fields: &UserAuthoredFields,
    options: RecordUserAuthoredOptions,
) -> Result<(), PrepStatusError> {
    let serialized = serialize_user_authored_fields(fields, options)?;
    tx.conn_ref()
        .execute(
            "INSERT INTO meeting_prep (
                meeting_id,
                user_agenda_json,
                user_notes,
                user_preparation_text,
                user_hidden_attendees_json,
                user_decisions_json
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(meeting_id) DO UPDATE SET
                user_agenda_json = excluded.user_agenda_json,
                user_notes       = excluded.user_notes,
                user_preparation_text = COALESCE(
                    excluded.user_preparation_text,
                    meeting_prep.user_preparation_text
                ),
                user_hidden_attendees_json = COALESCE(
                    excluded.user_hidden_attendees_json,
                    meeting_prep.user_hidden_attendees_json
                ),
                user_decisions_json = COALESCE(
                    excluded.user_decisions_json,
                    meeting_prep.user_decisions_json
                )",
            rusqlite::params![
                meeting_id,
                serialized.agenda,
                serialized.notes,
                serialized.preparation_text,
                serialized.hidden_attendees_json.as_deref(),
                serialized.decisions_json.as_deref()
            ],
        )
        .map_err(|error| PrepStatusError::Db(error.to_string()))?;
    Ok(())
}

fn serialize_user_authored_fields(
    fields: &UserAuthoredFields,
    options: RecordUserAuthoredOptions,
) -> Result<SerializedUserAuthoredFields<'_>, PrepStatusError> {
    Ok(SerializedUserAuthoredFields {
        agenda: fields.agenda.as_deref(),
        notes: fields.notes.as_deref(),
        preparation_text: fields.preparation_text.as_deref(),
        hidden_attendees_json: if options.update_hidden_attendees {
            Some(
                serde_json::to_string(&fields.hidden_attendees)
                    .map_err(|error| PrepStatusError::Db(error.to_string()))?,
            )
        } else {
            None
        },
        decisions_json: if options.update_decisions {
            Some(
                serde_json::to_string(&fields.decisions)
                    .map_err(|error| PrepStatusError::Db(error.to_string()))?,
            )
        } else {
            None
        },
    })
}

fn persist_prep_correction_replay_artifacts(
    tx: &ActionDb,
    meeting_id: &str,
    fields: &UserAuthoredFields,
    options: RecordUserAuthoredOptions,
    now: &str,
) -> Result<(), PrepStatusError> {
    let meeting_stable_key = meeting_stable_key_for_db(tx, meeting_id)?;
    for (field_path, payload) in prep_replay_payloads(fields, options)? {
        let payload_json = stable_json_string(&payload)?;
        let payload_hash = stable_json_hash(&payload)?;
        let replay_key = pii_safe_hash(
            "prep_replay",
            "dailyos.w4.meeting_prep.replay_key",
            &[&meeting_stable_key, field_path],
        )?;
        let journal_id = Uuid::new_v4().to_string();
        tx.conn_ref()
            .execute(
                "INSERT INTO meeting_prep_correction_journal (
                    id, feedback_id, meeting_stable_key, meeting_id, field_path,
                    actor, surface, source_asof, sensitivity, replay_key,
                    payload_json, payload_hash, lifecycle_state, created_at, updated_at
                 ) VALUES (
                    ?1, NULL, ?2, ?3, ?4,
                    'user', 'tauri', ?5, 'user_only', ?6,
                    ?7, ?8, 'active', ?5, ?5
                 )
                 ON CONFLICT(replay_key, field_path) DO UPDATE SET
                    meeting_id = excluded.meeting_id,
                    actor = excluded.actor,
                    surface = excluded.surface,
                    source_asof = excluded.source_asof,
                    sensitivity = 'user_only',
                    payload_json = excluded.payload_json,
                    payload_hash = excluded.payload_hash,
                    lifecycle_state = 'active',
                    lifecycle_version = lifecycle_version + 1,
                    redacted_at = NULL,
                    redaction_reason_code = NULL,
                    updated_at = excluded.updated_at",
                rusqlite::params![
                    journal_id,
                    &meeting_stable_key,
                    meeting_id,
                    field_path,
                    now,
                    &replay_key,
                    &payload_json,
                    &payload_hash,
                ],
            )
            .map_err(|e| PrepStatusError::Db(e.to_string()))?;

        let persisted_journal_id: String = tx
            .conn_ref()
            .query_row(
                "SELECT id
                   FROM meeting_prep_correction_journal
                  WHERE replay_key = ?1
                    AND field_path = ?2",
                rusqlite::params![&replay_key, field_path],
                |row| row.get(0),
            )
            .map_err(|e| PrepStatusError::Db(e.to_string()))?;
        upsert_prep_regeneration_job(
            tx,
            Some(&persisted_journal_id),
            &meeting_stable_key,
            field_path,
            now,
        )?;
    }
    Ok(())
}

fn prep_replay_payloads(
    fields: &UserAuthoredFields,
    options: RecordUserAuthoredOptions,
) -> Result<Vec<(&'static str, serde_json::Value)>, PrepStatusError> {
    let mut payloads = vec![
        ("user_agenda_json", json!({ "value": fields.agenda })),
        ("user_notes", json!({ "value": fields.notes })),
    ];
    if fields.preparation_text.is_some() {
        payloads.push((
            "preparation_text",
            json!({ "value": fields.preparation_text }),
        ));
    }
    if options.update_hidden_attendees {
        payloads.push((
            "hidden_attendees",
            json!({ "value": fields.hidden_attendees }),
        ));
    }
    if options.update_decisions {
        payloads.push((
            "decisions",
            json!({ "value": serde_json::to_value(&fields.decisions)
                .map_err(|e| PrepStatusError::Db(e.to_string()))? }),
        ));
    }
    Ok(payloads)
}

fn upsert_prep_regeneration_job(
    tx: &ActionDb,
    journal_id: Option<&str>,
    meeting_stable_key: &str,
    field_path: &str,
    now: &str,
) -> Result<(), PrepStatusError> {
    let job_id = Uuid::new_v4().to_string();
    let coalescing_key = format!("prep:{meeting_stable_key}:{field_path}");
    let inserted = tx
        .conn_ref()
        .execute(
            "INSERT OR IGNORE INTO meeting_prep_regeneration_jobs (
                id, journal_id, meeting_stable_key, field_path, status,
                coalescing_key, next_run_at, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, 'pending', ?5, ?6, ?6, ?6)",
            rusqlite::params![
                job_id,
                journal_id,
                meeting_stable_key,
                field_path,
                &coalescing_key,
                now,
            ],
        )
        .map_err(|e| PrepStatusError::Db(e.to_string()))?;
    if inserted == 0 {
        tx.conn_ref()
            .execute(
                "UPDATE meeting_prep_regeneration_jobs
                    SET journal_id = ?2,
                        next_run_at = CASE
                            WHEN status = 'pending' AND datetime(next_run_at) > datetime(?3) THEN ?3
                            ELSE next_run_at
                        END,
                        updated_at = ?3
                  WHERE coalescing_key = ?1
                    AND status IN ('pending', 'running')",
                rusqlite::params![&coalescing_key, journal_id, now],
            )
            .map_err(|e| PrepStatusError::Db(e.to_string()))?;
    }
    Ok(())
}

pub(crate) fn enqueue_prep_regeneration_job_in_tx(
    tx: &ActionDb,
    meeting_id: &str,
    field_path: Option<&str>,
    now: &str,
) -> Result<(), PrepStatusError> {
    let meeting_stable_key = meeting_stable_key_for_db(tx, meeting_id)?;
    upsert_prep_regeneration_job(
        tx,
        None,
        &meeting_stable_key,
        field_path.unwrap_or("*"),
        now,
    )
}

fn stable_json_string(value: &serde_json::Value) -> Result<String, PrepStatusError> {
    serde_json::to_string(value).map_err(|e| PrepStatusError::Db(e.to_string()))
}

fn stable_json_hash(value: &serde_json::Value) -> Result<String, PrepStatusError> {
    let raw = stable_json_string(value)?;
    Ok(format!(
        "sha256:{}",
        hex::encode(Sha256::digest(raw.as_bytes()))
    ))
}

fn pii_safe_hash(
    prefix: &str,
    domain: &str,
    components: &[&str],
) -> Result<String, PrepStatusError> {
    #[cfg(test)]
    {
        Ok(crate::db::local_db_keyed_audit_tag_for_tests(
            "w4-meeting-prep-test-secret",
            prefix,
            domain,
            components,
        ))
    }

    #[cfg(not(test))]
    {
        crate::db::local_db_keyed_audit_tag(prefix, domain, components)
            .map_err(|error| PrepStatusError::Db(format!("derive PII-safe hash: {error}")))
    }
}

/// Record a UserSuppressed / UserDismissed dismissal for a meeting.
///
/// Persists to v242 `meeting_prep_status_dismissals`. Transitions the
/// in-memory status snapshot — the v241 view-driven read path picks
/// up the dismissal at next `compute_status`.
pub fn record_dismissal(
    meeting_id: &str,
    from: PrepStatus,
    kind: DismissalKind,
    actor: &str,
    reason: Option<&str>,
    db: &ActionDb,
) -> Result<(), PrepStatusError> {
    let target = match kind {
        DismissalKind::UserSuppressed => PrepStatus::UserSuppressed,
        DismissalKind::UserDismissed => PrepStatus::UserDismissed,
    };
    if !from.can_transition_to(target) {
        return Err(PrepStatusError::IllegalTransition { from, to: target });
    }

    let now = Utc::now().to_rfc3339();
    let id = uuid::Uuid::new_v4().to_string();

    db.with_transaction(|tx| {
        let conn = tx.conn_ref();
        conn.execute(
            "INSERT INTO meeting_prep_status_dismissals (
                id, meeting_id, dismissal_kind, dismissal_reason,
                actor, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
            rusqlite::params![id, meeting_id, kind.as_str(), reason, actor, now],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    })
    .map_err(PrepStatusError::Db)?;

    emit_status_signal(
        meeting_id,
        Some(from),
        Some(target),
        kind.as_str(),
        db,
        Utc::now().to_rfc3339(),
    )?;
    Ok(())
}

/// Clear an active dismissal (un-suppress / un-dismiss).
///
/// Transitions UserSuppressed / UserDismissed → PrepNeeded.
pub fn clear_dismissal(
    meeting_id: &str,
    from: PrepStatus,
    db: &ActionDb,
) -> Result<(), PrepStatusError> {
    if !matches!(from, PrepStatus::UserSuppressed | PrepStatus::UserDismissed) {
        return Err(PrepStatusError::IllegalTransition {
            from,
            to: PrepStatus::PrepNeeded,
        });
    }
    let now = Utc::now().to_rfc3339();
    db.with_transaction(|tx| {
        let conn = tx.conn_ref();
        conn.execute(
            "UPDATE meeting_prep_status_dismissals
                SET resolved_at = ?1, updated_at = ?1
                WHERE meeting_id = ?2 AND resolved_at IS NULL",
            rusqlite::params![now, meeting_id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    })
    .map_err(PrepStatusError::Db)?;

    emit_status_signal(
        meeting_id,
        Some(from),
        Some(PrepStatus::PrepNeeded),
        "dismissal_cleared",
        db,
        Utc::now().to_rfc3339(),
    )?;
    Ok(())
}

/// Explicit state-machine transition primitive.
///
/// AC-335.14: validates `from → to` against
/// [`PrepStatus::legal_transitions`] before emitting the signal.
/// Returns [`PrepStatusError::IllegalTransition`] when the transition
/// is illegal.
pub fn transition_status(
    meeting_id: &str,
    from: PrepStatus,
    to: PrepStatus,
    db: &ActionDb,
) -> Result<(), PrepStatusError> {
    if !from.can_transition_to(to) {
        return Err(PrepStatusError::IllegalTransition { from, to });
    }
    let now = Utc::now().to_rfc3339();
    emit_status_signal(meeting_id, Some(from), Some(to), "transition", db, now)?;
    Ok(())
}

/// Dismissal kind for [`record_dismissal`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DismissalKind {
    UserSuppressed,
    UserDismissed,
}

impl DismissalKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UserSuppressed => "user_suppressed",
            Self::UserDismissed => "user_dismissed",
        }
    }
}

/// Emit `meeting_prep_status_changed` with the `{from, to,
/// meeting_id, reason}` payload.
fn emit_status_signal(
    meeting_id: &str,
    from: Option<PrepStatus>,
    to: Option<PrepStatus>,
    reason_tag: &str,
    db: &ActionDb,
    at: String,
) -> Result<(), PrepStatusError> {
    let payload = json!({
        "meeting_id": meeting_id,
        "from": from,
        "to": to,
        "reason": reason_tag,
        "at": at,
    })
    .to_string();
    bus::emit(
        db,
        SignalEmission {
            entity_type: "meeting",
            entity_id: meeting_id,
            signal_type: SIGNAL_TYPE,
            source: SIGNAL_SOURCE,
            value: Some(&payload),
            confidence: 1.0,
            source_context: None,
        },
    )
    .map(|_| ())
    .map_err(|e| PrepStatusError::SignalEmit(e.to_string()))
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_utils::test_db;
    use crate::services::context::{ExternalClients, FixedClock, SeedableRng};
    use crate::services::meeting_prep_status::DecisionRef;
    use chrono::{TimeZone, Utc};

    struct UserCtxFixture {
        clock: crate::services::context::SystemClock,
        rng: crate::services::context::SystemRng,
        external: crate::services::context::ExternalClients,
    }

    impl UserCtxFixture {
        fn new() -> Self {
            Self {
                clock: crate::services::context::SystemClock,
                rng: crate::services::context::SystemRng,
                external: crate::services::context::ExternalClients::default(),
            }
        }

        fn ctx(&self) -> ServiceContext<'_> {
            ServiceContext::new_live(&self.clock, &self.rng, &self.external).with_actor("user:test")
        }
    }

    fn w4_user_authored_fixture() -> UserAuthoredFields {
        UserAuthoredFields {
            agenda: Some("[\"Discuss renewal risk\"]".to_string()),
            notes: Some("User-authored prep note".to_string()),
            preparation_text: Some("Review account state before the call".to_string()),
            hidden_attendees: vec!["Hidden attendee".to_string()],
            decisions: vec![DecisionRef {
                claim_id: "decision-claim-fixture-1".to_string(),
                text: "Continue the pilot rollout".to_string(),
            }],
        }
    }

    fn assert_meeting_prep_user_authored_fields(
        db: &ActionDb,
        meeting_id: &str,
        fields: &UserAuthoredFields,
    ) {
        let (agenda, notes, preparation_text, hidden_attendees_json, decisions_json): (
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
        ) = db
            .conn_ref()
            .query_row(
                "SELECT user_agenda_json,
                        user_notes,
                        user_preparation_text,
                        user_hidden_attendees_json,
                        user_decisions_json
                   FROM meeting_prep
                  WHERE meeting_id = ?1",
                [meeting_id],
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
            .expect("read persisted user-authored prep fields");

        assert_eq!(agenda, fields.agenda);
        assert_eq!(notes, fields.notes);
        assert_eq!(preparation_text, fields.preparation_text);
        let expected_hidden_attendees_json = serde_json::to_string(&fields.hidden_attendees)
            .expect("serialize hidden attendees fixture");
        let expected_decisions_json =
            serde_json::to_string(&fields.decisions).expect("serialize decisions fixture");
        assert_eq!(
            hidden_attendees_json.as_deref(),
            Some(expected_hidden_attendees_json.as_str())
        );
        assert_eq!(
            decisions_json.as_deref(),
            Some(expected_decisions_json.as_str())
        );
    }

    fn clear_meeting_prep_user_authored_fields(db: &ActionDb, meeting_id: &str) {
        db.conn_ref()
            .execute(
                "UPDATE meeting_prep
                    SET user_agenda_json = NULL,
                        user_notes = NULL,
                        user_preparation_text = NULL,
                        user_hidden_attendees_json = NULL,
                        user_decisions_json = NULL
                  WHERE meeting_id = ?1",
                [meeting_id],
            )
            .expect("simulate rebuilt generated prep without replayed user layer");
    }

    fn delete_meeting_prep_row(db: &ActionDb, meeting_id: &str) {
        db.conn_ref()
            .execute(
                "DELETE FROM meeting_prep WHERE meeting_id = ?1",
                [meeting_id],
            )
            .expect("simulate rebuilt meeting without prep row");
    }

    fn insert_meeting_fixture(db: &ActionDb, meeting_id: &str) {
        db.conn_ref()
            .execute(
                "INSERT INTO meetings (id, title, meeting_type, start_time, created_at)
                 VALUES (?1, 'Fixture meeting', 'customer', '2026-06-06T00:00:00Z', '2026-06-06T00:00:00Z')",
                [meeting_id],
            )
            .expect("insert meeting fixture");
    }

    fn insert_manual_meeting_fixture(
        db: &ActionDb,
        meeting_id: &str,
        title: &str,
        start_time: &str,
        attendees_json: &str,
    ) {
        db.conn_ref()
            .execute(
                "INSERT INTO meetings (id, title, meeting_type, start_time, attendees, created_at)
                 VALUES (?1, ?2, 'customer', ?3, ?4, ?3)",
                rusqlite::params![meeting_id, title, start_time, attendees_json],
            )
            .expect("insert manual meeting fixture");
    }

    #[test]
    fn transition_status_rejects_illegal_transition() {
        // We don't need a DB for the pre-check; transition_status
        // returns the IllegalTransition error before any DB call.
        // Build an ActionDb stub via in-memory connection.
        let conn = rusqlite::Connection::open_in_memory().expect("open in-memory");
        let db = ActionDb::from_conn(&conn);

        // PrepNeeded → Ready is illegal.
        let result = transition_status("m1", PrepStatus::PrepNeeded, PrepStatus::Ready, db);
        match result {
            Err(PrepStatusError::IllegalTransition { from, to }) => {
                assert_eq!(from, PrepStatus::PrepNeeded);
                assert_eq!(to, PrepStatus::Ready);
            }
            other => panic!("expected IllegalTransition, got {other:?}"),
        }
    }

    #[test]
    fn dismissal_kind_str_roundtrips_schema_check() {
        // The CHECK constraint on meeting_prep_status_dismissals
        // enumerates exactly these two strings; if this test
        // diverges, the v242 schema CHECK will reject writes at
        // runtime. Keep them aligned.
        assert_eq!(DismissalKind::UserSuppressed.as_str(), "user_suppressed");
        assert_eq!(DismissalKind::UserDismissed.as_str(), "user_dismissed");
    }

    #[test]
    fn record_user_authored_writes_replay_journal_and_regeneration_jobs() {
        let db = test_db();
        let ctx_fixture = UserCtxFixture::new();
        let ctx = ctx_fixture.ctx();
        let fields = w4_user_authored_fixture();

        record_user_authored(&ctx, "meeting-fixture-1", &fields, &db)
            .expect("record user-authored prep");
        assert_meeting_prep_user_authored_fields(&db, "meeting-fixture-1", &fields);

        let (journal_count, user_only_count): (i64, i64) = db
            .conn_ref()
            .query_row(
                "SELECT count(*),
                        sum(CASE WHEN sensitivity = 'user_only' THEN 1 ELSE 0 END)
                   FROM meeting_prep_correction_journal
                  WHERE meeting_id = 'meeting-fixture-1'
                    AND lifecycle_state = 'active'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("count prep journal rows");
        assert_eq!(journal_count, 5);
        assert_eq!(user_only_count, journal_count);

        let regeneration_count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT count(*)
                   FROM meeting_prep_regeneration_jobs
                  WHERE status = 'pending'
                    AND journal_id IN (
                        SELECT id
                          FROM meeting_prep_correction_journal
                         WHERE meeting_id = 'meeting-fixture-1'
                    )",
                [],
                |row| row.get(0),
            )
            .expect("count prep regeneration jobs");
        assert_eq!(regeneration_count, journal_count);
    }

    #[test]
    fn w4_replay_key_distinguishes_manual_meetings_same_time_and_attendees() {
        let db = test_db();
        let attendees = r#"["person-a@example.com","person-b@example.com"]"#;
        insert_manual_meeting_fixture(
            &db,
            "meeting-fixture-1",
            "First planning session",
            "2026-06-06T12:00:00Z",
            attendees,
        );
        insert_manual_meeting_fixture(
            &db,
            "meeting-fixture-2",
            "Second planning session",
            "2026-06-06T12:00:00Z",
            attendees,
        );
        let ctx_fixture = UserCtxFixture::new();
        let ctx = ctx_fixture.ctx();

        let first_fields = UserAuthoredFields {
            agenda: None,
            notes: Some("First meeting notes".to_string()),
            preparation_text: None,
            hidden_attendees: vec![],
            decisions: vec![],
        };
        let second_fields = UserAuthoredFields {
            agenda: None,
            notes: Some("Second meeting notes".to_string()),
            preparation_text: None,
            hidden_attendees: vec![],
            decisions: vec![],
        };
        record_user_authored(&ctx, "meeting-fixture-1", &first_fields, &db)
            .expect("record first manual meeting prep");
        record_user_authored(&ctx, "meeting-fixture-2", &second_fields, &db)
            .expect("record second manual meeting prep");

        let mut stmt = db
            .conn_ref()
            .prepare(
                "SELECT meeting_id, meeting_stable_key, payload_json
                   FROM meeting_prep_correction_journal
                  WHERE field_path = 'user_notes'
                  ORDER BY meeting_id",
            )
            .expect("prepare replay rows");
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .expect("query replay rows")
            .collect::<Result<Vec<_>, _>>()
            .expect("map replay rows");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].0.as_deref(), Some("meeting-fixture-1"));
        assert_eq!(rows[1].0.as_deref(), Some("meeting-fixture-2"));
        assert_ne!(
            rows[0].1, rows[1].1,
            "manual meetings with the same time and attendees must not collide in the replay journal"
        );
        assert!(rows[0].2.contains("First meeting notes"));
        assert!(rows[1].2.contains("Second meeting notes"));
    }

    #[test]
    fn w4_replay_key_survives_manual_meeting_content_drift() {
        let db = test_db();
        let attendees = r#"["person-a@example.com","person-b@example.com"]"#;
        insert_manual_meeting_fixture(
            &db,
            "meeting-fixture-1",
            "Original planning session",
            "2026-06-06T12:00:00Z",
            attendees,
        );
        let ctx_fixture = UserCtxFixture::new();
        let seed_ctx = ctx_fixture.ctx();
        let fields = w4_user_authored_fixture();
        record_user_authored(&seed_ctx, "meeting-fixture-1", &fields, &db)
            .expect("record user-authored manual meeting prep");

        delete_meeting_prep_row(&db, "meeting-fixture-1");
        db.conn_ref()
            .execute("DELETE FROM meetings WHERE id = ?1", ["meeting-fixture-1"])
            .expect("simulate manual meeting row being rebuilt");
        insert_manual_meeting_fixture(
            &db,
            "meeting-rebuilt-fixture-1",
            "Renamed planning session",
            "2026-06-06T12:30:00Z",
            attendees,
        );

        let clock = crate::services::context::SystemClock;
        let rng = crate::services::context::SystemRng;
        let external = crate::services::context::ExternalClients::default();
        let ctx = ServiceContext::new_live(&clock, &rng, &external).with_actor("user:test");
        let report = replay_active_prep_correction_journal(&ctx, &db, "manual-drift-rebuild")
            .expect("replay prep corrections after manual meeting content drift");

        assert_eq!(report.replayed_entries, 5);
        assert_eq!(report.orphaned_entries, 0);
        assert_meeting_prep_user_authored_fields(&db, "meeting-rebuilt-fixture-1", &fields);
    }

    #[test]
    fn w4_replay_resolver_ignores_unrelated_legacy_csv_attendees() {
        let db = test_db();
        let attendees = r#"["person-a@example.com","person-b@example.com"]"#;
        insert_manual_meeting_fixture(
            &db,
            "meeting-fixture-1",
            "Planning session",
            "2026-06-06T12:00:00Z",
            attendees,
        );
        let ctx_fixture = UserCtxFixture::new();
        let seed_ctx = ctx_fixture.ctx();
        let fields = w4_user_authored_fixture();
        record_user_authored(&seed_ctx, "meeting-fixture-1", &fields, &db)
            .expect("record user-authored manual meeting prep");

        insert_manual_meeting_fixture(
            &db,
            "unrelated-legacy-meeting",
            "Legacy attendee fixture",
            "2026-06-07T12:00:00Z",
            "person-a@example.com, person-b@example.com",
        );
        delete_meeting_prep_row(&db, "meeting-fixture-1");

        let clock = crate::services::context::SystemClock;
        let rng = crate::services::context::SystemRng;
        let external = crate::services::context::ExternalClients::default();
        let ctx = ServiceContext::new_live(&clock, &rng, &external).with_actor("user:test");
        let report = replay_active_prep_correction_journal(&ctx, &db, "legacy-csv-rebuild")
            .expect("unrelated malformed attendees should not abort replay");

        assert_eq!(report.replayed_entries, 5);
        assert_eq!(report.orphaned_entries, 0);
        assert_meeting_prep_user_authored_fields(&db, "meeting-fixture-1", &fields);
    }

    #[test]
    fn w4_manual_replay_resolves_same_attendees_by_start_time_family() {
        let db = test_db();
        let attendees = r#"["person-a@example.com","person-b@example.com"]"#;
        insert_manual_meeting_fixture(
            &db,
            "meeting-fixture-1",
            "Morning planning session",
            "2026-06-06T12:00:00Z",
            attendees,
        );
        insert_manual_meeting_fixture(
            &db,
            "meeting-fixture-2",
            "Afternoon planning session",
            "2026-06-06T15:00:00Z",
            attendees,
        );
        let ctx_fixture = UserCtxFixture::new();
        let seed_ctx = ctx_fixture.ctx();
        let fields = w4_user_authored_fixture();
        record_user_authored(&seed_ctx, "meeting-fixture-1", &fields, &db)
            .expect("record user-authored manual prep");
        let expected_replay_entries: i64 = db
            .conn_ref()
            .query_row(
                "SELECT count(*)
                   FROM meeting_prep_correction_journal
                  WHERE meeting_id = ?1
                    AND lifecycle_state = 'active'",
                ["meeting-fixture-1"],
                |row| row.get(0),
            )
            .expect("count active replay journal rows");

        delete_meeting_prep_row(&db, "meeting-fixture-1");
        db.conn_ref()
            .execute(
                "DELETE FROM meetings WHERE id IN (?1, ?2)",
                ["meeting-fixture-1", "meeting-fixture-2"],
            )
            .expect("simulate manual meeting rows being rebuilt");
        insert_manual_meeting_fixture(
            &db,
            "meeting-rebuilt-fixture-1",
            "Renamed morning planning session",
            "2026-06-06T12:00:00Z",
            attendees,
        );
        insert_manual_meeting_fixture(
            &db,
            "meeting-rebuilt-fixture-2",
            "Renamed afternoon planning session",
            "2026-06-06T15:00:00Z",
            attendees,
        );

        let clock = crate::services::context::SystemClock;
        let rng = crate::services::context::SystemRng;
        let external = crate::services::context::ExternalClients::default();
        let ctx = ServiceContext::new_live(&clock, &rng, &external).with_actor("user:test");
        let report = replay_active_prep_correction_journal(&ctx, &db, "manual-family-rebuild")
            .expect("replay prep corrections after manual meeting rebuild");

        assert_eq!(
            i64::try_from(report.replayed_entries).unwrap(),
            expected_replay_entries
        );
        assert_eq!(report.orphaned_entries, 0);
        assert_meeting_prep_user_authored_fields(&db, "meeting-rebuilt-fixture-1", &fields);
        let afternoon_notes: Option<String> = db
            .conn_ref()
            .query_row(
                "SELECT user_notes FROM meeting_prep WHERE meeting_id = ?1",
                ["meeting-rebuilt-fixture-2"],
                |row| row.get(0),
            )
            .optional()
            .expect("load afternoon prep fields")
            .flatten();
        assert_eq!(afternoon_notes, None);
    }

    #[test]
    fn compute_status_reads_all_user_authored_fields() {
        let db = test_db();
        insert_meeting_fixture(&db, "meeting-fixture-1");
        let ctx_fixture = UserCtxFixture::new();
        let ctx = ctx_fixture.ctx();
        let fields = w4_user_authored_fixture();
        record_user_authored(&ctx, "meeting-fixture-1", &fields, &db)
            .expect("record user-authored prep");

        let snapshot =
            crate::services::meeting_prep_status::read::compute_status("meeting-fixture-1", &db)
                .expect("compute meeting prep status");

        assert_eq!(snapshot.user_authored, fields);
    }

    #[test]
    fn record_user_authored_preserves_w4_fields_on_partial_update() {
        let db = test_db();
        let ctx_fixture = UserCtxFixture::new();
        let ctx = ctx_fixture.ctx();
        let fields = w4_user_authored_fixture();
        record_user_authored(&ctx, "meeting-fixture-1", &fields, &db)
            .expect("record full user layer");

        record_user_authored(
            &ctx,
            "meeting-fixture-1",
            &UserAuthoredFields {
                agenda: fields.agenda.clone(),
                notes: Some("Updated notes only".to_string()),
                preparation_text: None,
                hidden_attendees: vec![],
                decisions: vec![],
            },
            &db,
        )
        .expect("record partial notes update");

        let mut expected = fields;
        expected.notes = Some("Updated notes only".to_string());
        assert_meeting_prep_user_authored_fields(&db, "meeting-fixture-1", &expected);
    }

    #[test]
    fn w4_record_user_authored_with_options_clears_empty_hidden_attendees() {
        let db = test_db();
        let ctx_fixture = UserCtxFixture::new();
        let ctx = ctx_fixture.ctx();
        let fields = w4_user_authored_fixture();
        record_user_authored(&ctx, "meeting-fixture-1", &fields, &db)
            .expect("record full user layer");

        let mut clear_hidden = fields.clone();
        clear_hidden.hidden_attendees = vec![];
        record_user_authored_with_options(
            &ctx,
            "meeting-fixture-1",
            &clear_hidden,
            RecordUserAuthoredOptions {
                update_hidden_attendees: true,
                update_decisions: false,
            },
            &db,
        )
        .expect("clear hidden attendees");

        let mut expected = fields;
        expected.hidden_attendees = vec![];
        assert_meeting_prep_user_authored_fields(&db, "meeting-fixture-1", &expected);

        let payload_json: String = db
            .conn_ref()
            .query_row(
                "SELECT payload_json
                   FROM meeting_prep_correction_journal
                  WHERE meeting_id = 'meeting-fixture-1'
                    AND field_path = 'hidden_attendees'",
                [],
                |row| row.get(0),
            )
            .expect("read hidden-attendees replay payload");
        assert_eq!(payload_json, r#"{"value":[]}"#);
    }

    #[test]
    fn replay_active_prep_correction_journal_restores_user_authored_fields() {
        let db = test_db();
        insert_meeting_fixture(&db, "meeting-fixture-1");
        let ctx_fixture = UserCtxFixture::new();
        let seed_ctx = ctx_fixture.ctx();
        let fields = w4_user_authored_fixture();
        record_user_authored(&seed_ctx, "meeting-fixture-1", &fields, &db)
            .expect("record user-authored prep");

        delete_meeting_prep_row(&db, "meeting-fixture-1");

        let clock = crate::services::context::SystemClock;
        let rng = crate::services::context::SystemRng;
        let external = crate::services::context::ExternalClients::default();
        let ctx = ServiceContext::new_live(&clock, &rng, &external).with_actor("user:test");
        let report = replay_active_prep_correction_journal(&ctx, &db, "rebuild-fixture-1")
            .expect("replay prep corrections");

        assert_eq!(report.replayed_entries, 5);
        assert_eq!(report.orphaned_entries, 0);

        assert_meeting_prep_user_authored_fields(&db, "meeting-fixture-1", &fields);

        let (active_count, replayed_at_count, first_attempt_count, completed_jobs): (
            i64,
            i64,
            i64,
            i64,
        ) = db
            .conn_ref()
            .query_row(
                "SELECT
                    sum(CASE WHEN j.lifecycle_state = 'active' THEN 1 ELSE 0 END),
                    sum(CASE WHEN j.replayed_at IS NOT NULL THEN 1 ELSE 0 END),
                    sum(CASE WHEN j.replay_attempt_count = 1 THEN 1 ELSE 0 END),
                    sum(CASE WHEN r.status = 'completed' THEN 1 ELSE 0 END)
                   FROM meeting_prep_correction_journal j
                   LEFT JOIN meeting_prep_regeneration_jobs r ON r.journal_id = j.id
                  WHERE j.meeting_id = 'meeting-fixture-1'
                    AND j.rebuild_replay_id = 'rebuild-fixture-1'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("read replay terminal state");
        assert_eq!(active_count, 5);
        assert_eq!(replayed_at_count, 5);
        assert_eq!(first_attempt_count, 5);
        assert_eq!(completed_jobs, 5);

        clear_meeting_prep_user_authored_fields(&db, "meeting-fixture-1");
        let second_report = replay_active_prep_correction_journal(&ctx, &db, "rebuild-fixture-2")
            .expect("replay active prep corrections again");
        assert_eq!(second_report.replayed_entries, 5);
        assert_eq!(second_report.orphaned_entries, 0);

        let second_attempt_count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT sum(CASE WHEN replay_attempt_count = 2 THEN 1 ELSE 0 END)
                   FROM meeting_prep_correction_journal
                  WHERE meeting_id = 'meeting-fixture-1'",
                [],
                |row| row.get(0),
            )
            .expect("read second replay attempt count");
        assert_meeting_prep_user_authored_fields(&db, "meeting-fixture-1", &fields);
        assert_eq!(second_attempt_count, 5);
    }

    #[test]
    fn prep_regeneration_worker_replays_pending_user_authored_layer() {
        let db = test_db();
        insert_meeting_fixture(&db, "meeting-fixture-1");
        let ctx_fixture = UserCtxFixture::new();
        let seed_ctx = ctx_fixture.ctx();
        let fields = w4_user_authored_fixture();
        record_user_authored(&seed_ctx, "meeting-fixture-1", &fields, &db)
            .expect("record user-authored prep");

        delete_meeting_prep_row(&db, "meeting-fixture-1");

        let clock = crate::services::context::SystemClock;
        let rng = crate::services::context::SystemRng;
        let external = crate::services::context::ExternalClients::default();
        let ctx = ServiceContext::new_live(&clock, &rng, &external).with_actor("system:worker");
        let outcome = process_one_prep_regeneration_job(&ctx, &db, "prep-worker")
            .expect("process prep regeneration job");
        assert!(matches!(
            outcome,
            PrepRegenerationProcessOutcome::Completed { .. }
        ));

        let completed_jobs: i64 = db
            .conn_ref()
            .query_row(
                "SELECT count(*)
                   FROM meeting_prep_regeneration_jobs
                  WHERE status = 'completed'",
                [],
                |row| row.get(0),
            )
            .expect("read replayed prep state");
        assert_meeting_prep_user_authored_fields(&db, "meeting-fixture-1", &fields);
        assert_eq!(completed_jobs, 5);
    }

    #[test]
    fn failed_prep_regeneration_job_schedules_due_retry_without_immediate_reclaim() {
        let db = test_db();
        db.conn_ref()
            .execute(
                "INSERT INTO meeting_prep_regeneration_jobs (
                    id, journal_id, meeting_stable_key, field_path, status,
                    coalescing_key, retry_count, max_attempts, next_run_at,
                    created_at, updated_at
                 ) VALUES (
                    'prep-job-delayed-retry', NULL, 'manual:meeting-key',
                    'user_notes', 'running', 'prep:manual:meeting-key:user_notes',
                    1, 5, '2026-06-06T12:00:00+00:00',
                    '2026-06-06T11:00:00+00:00',
                    '2026-06-06T12:00:00+00:00'
                 )",
                [],
            )
            .expect("seed running prep regeneration job");

        let fail_clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 6, 6, 12, 0, 0).unwrap());
        let fail_rng = SeedableRng::new(71);
        let fail_external = ExternalClients::default();
        let fail_ctx = ServiceContext::test_live(&fail_clock, &fail_rng, &fail_external)
            .with_actor("system:worker");
        let job = read_prep_regeneration_job(&db, "prep-job-delayed-retry")
            .expect("read prep job")
            .expect("prep job exists");
        let outcome =
            mark_prep_regeneration_job_failed(&fail_ctx, &db, &job, "transient_dependency")
                .expect("schedule prep retry");
        assert!(matches!(
            outcome,
            PrepRegenerationProcessOutcome::RetryScheduled { ref job_id }
                if job_id == "prep-job-delayed-retry"
        ));
        let (status, next_run_at): (String, String) = db
            .conn_ref()
            .query_row(
                "SELECT status, next_run_at
                   FROM meeting_prep_regeneration_jobs
                  WHERE id = 'prep-job-delayed-retry'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("read scheduled prep retry");
        assert_eq!(status, "pending");
        assert_eq!(
            next_run_at,
            Utc.with_ymd_and_hms(2026, 6, 6, 12, 0, 1)
                .unwrap()
                .to_rfc3339()
        );

        let immediate = claim_next_prep_regeneration_job(&fail_ctx, &db, "prep-worker-now")
            .expect("check immediate prep claim");
        assert!(
            immediate.is_none(),
            "scheduled prep retry must not be claimable before next_run_at"
        );

        let due_clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 6, 6, 12, 0, 1).unwrap());
        let due_rng = SeedableRng::new(72);
        let due_external = ExternalClients::default();
        let due_ctx = ServiceContext::test_live(&due_clock, &due_rng, &due_external)
            .with_actor("system:worker");
        let claimed = claim_next_prep_regeneration_job(&due_ctx, &db, "prep-worker-due")
            .expect("claim due prep retry")
            .expect("prep retry should be due");
        assert_eq!(claimed.id, "prep-job-delayed-retry");
        assert_eq!(claimed.retry_count, 2);
    }

    #[test]
    fn prep_regeneration_worker_stales_job_without_active_replay_rows() {
        let db = test_db();
        db.with_transaction(|tx| {
            enqueue_prep_regeneration_job_in_tx(
                tx,
                "meeting-without-journal",
                Some("user_notes"),
                "2026-06-06T00:00:00Z",
            )
            .map_err(|error| error.to_string())?;
            Ok(())
        })
        .expect("enqueue prep regeneration job");

        let clock = crate::services::context::SystemClock;
        let rng = crate::services::context::SystemRng;
        let external = crate::services::context::ExternalClients::default();
        let ctx = ServiceContext::new_live(&clock, &rng, &external).with_actor("system:worker");
        let outcome = process_one_prep_regeneration_job(&ctx, &db, "prep-worker")
            .expect("process prep regeneration job");
        assert!(matches!(
            outcome,
            PrepRegenerationProcessOutcome::Stale { .. }
        ));

        let (status, stale_reason): (String, Option<String>) = db
            .conn_ref()
            .query_row(
                "SELECT status, stale_reason
                   FROM meeting_prep_regeneration_jobs
                  LIMIT 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("read stale prep job");
        assert_eq!(status, "stale");
        assert_eq!(
            stale_reason.as_deref(),
            Some("prep_regeneration_no_active_replay_rows")
        );
    }
}
