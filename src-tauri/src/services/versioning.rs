use chrono::{DateTime, Duration, Utc};
use rusqlite::{params, OptionalExtension};

use crate::db::ActionDb;

pub const MAX_PERSISTED_VERSION: u64 = i64::MAX as u64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignalCursor(String);

impl SignalCursor {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for SignalCursor {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for SignalCursor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MutationSubject {
    Claim(String),
    Composition(String),
}

impl MutationSubject {
    pub fn claim_id(&self) -> Option<&str> {
        match self {
            Self::Claim(id) => Some(id.as_str()),
            Self::Composition(_) => None,
        }
    }

    pub fn composition_id(&self) -> Option<&str> {
        match self {
            Self::Claim(_) => None,
            Self::Composition(id) => Some(id.as_str()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MutationAttempt {
    pub mutation_id: String,
    pub subject: MutationSubject,
    pub cursor: SignalCursor,
}

pub struct MutationGuard<'db> {
    db: &'db ActionDb,
    attempt: MutationAttempt,
    completed: bool,
}

impl<'db> MutationGuard<'db> {
    pub fn reserve(
        db: &'db ActionDb,
        claim_id: impl Into<String>,
        now: DateTime<Utc>,
    ) -> Result<Self, rusqlite::Error> {
        Self::reserve_for_subject(db, MutationSubject::Claim(claim_id.into()), now)
    }

    pub fn reserve_composition(
        db: &'db ActionDb,
        composition_id: impl Into<String>,
        now: DateTime<Utc>,
    ) -> Result<Self, rusqlite::Error> {
        Self::reserve_for_subject(db, MutationSubject::Composition(composition_id.into()), now)
    }

    fn reserve_for_subject(
        db: &'db ActionDb,
        subject: MutationSubject,
        now: DateTime<Utc>,
    ) -> Result<Self, rusqlite::Error> {
        let attempt = MutationAttempt {
            mutation_id: uuid::Uuid::new_v4().to_string(),
            subject,
            cursor: SignalCursor::new(),
        };
        db.with_transaction(|tx| {
            tx.conn_ref()
                .execute(
                    "INSERT INTO mutation_attempts \
                     (mutation_id, claim_id, composition_id, cursor, started_at, status, finalized_at) \
                     VALUES (?1, ?2, ?3, ?4, ?5, 'in_flight', NULL)",
                    params![
                        &attempt.mutation_id,
                        attempt.subject.claim_id(),
                        attempt.subject.composition_id(),
                        attempt.cursor.as_str(),
                        now.to_rfc3339(),
                    ],
                )
                .map_err(|e| e.to_string())?;
            Ok(())
        })
        .map_err(rusqlite::Error::InvalidParameterName)?;
        Ok(Self {
            db,
            attempt,
            completed: false,
        })
    }

    pub fn attempt(&self) -> &MutationAttempt {
        &self.attempt
    }

    pub fn cursor(&self) -> &SignalCursor {
        &self.attempt.cursor
    }

    pub fn mark_completed(&mut self) {
        self.completed = true;
    }
}

impl Drop for MutationGuard<'_> {
    fn drop(&mut self) {
        if !self.completed {
            let now = Utc::now();
            if let Err(error) = finalize_mutation_attempt_aborted(self.db, &self.attempt, now) {
                log::warn!(
                    "failed to finalize aborted mutation attempt mutation_id={} error={error}",
                    self.attempt.mutation_id
                );
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionEventKind {
    ClaimUpdated,
    ClaimCorrected,
    ClaimSuperseded,
    ClaimTombstoned,
    ClaimWriteRejected,
    ClaimConflictDetected,
    CompositionUpdated,
    CompositionWriteRejected,
    MutationAborted,
}

impl VersionEventKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ClaimUpdated => "claim.updated",
            Self::ClaimCorrected => "claim.corrected",
            Self::ClaimSuperseded => "claim.superseded",
            Self::ClaimTombstoned => "claim.tombstoned",
            Self::ClaimWriteRejected => "claim.write_rejected",
            Self::ClaimConflictDetected => "claim.conflict_detected",
            Self::CompositionUpdated => "composition.updated",
            Self::CompositionWriteRejected => "composition.write_rejected",
            Self::MutationAborted => "mutation_aborted",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionActorKind {
    User,
    Agent,
    Admin,
    System,
    SurfaceClient,
}

impl VersionActorKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Agent => "agent",
            Self::Admin => "admin",
            Self::System => "system",
            Self::SurfaceClient => "surface_client",
        }
    }

    pub fn from_service_actor(actor: &str) -> Self {
        // Canonical service-context actor strings carry an optional
        // `<kind>:<instance>` prefix (e.g. `agent:test`, `user:42`,
        // `surface_client:studio-instance-a`). Exact-match would
        // misattribute every prefixed instance to `System`. Parse the
        // kind prefix before `:`; fall back to exact match for the
        // bare kind tokens, then default to System.
        let kind = actor.split_once(':').map(|(prefix, _)| prefix).unwrap_or(actor);
        match kind {
            "user" => Self::User,
            "agent" => Self::Agent,
            "admin" => Self::Admin,
            "surface_client" | "surface" => Self::SurfaceClient,
            _ => Self::System,
        }
    }
}

#[derive(Debug, Clone)]
pub struct VersionEventInsert<'a> {
    pub cursor: &'a SignalCursor,
    pub event_kind: VersionEventKind,
    pub claim_id: Option<&'a str>,
    pub composition_id: Option<&'a str>,
    pub previous_version: Option<u64>,
    pub current_version: u64,
    pub reason: Option<&'a str>,
    pub scope_redacted: bool,
    pub correction_event_log_id: Option<&'a str>,
    pub mutation_id: Option<&'a str>,
    pub created_at: &'a str,
    pub actor_kind: VersionActorKind,
}

pub fn insert_version_event(
    tx: &ActionDb,
    event: VersionEventInsert<'_>,
) -> Result<(), rusqlite::Error> {
    tx.conn_ref().execute(
        "INSERT INTO version_events (
            cursor, event_kind, claim_id, composition_id, previous_version,
            current_version, reason, scope_redacted, correction_event_log_id,
            mutation_id, created_at, actor_kind
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            event.cursor.as_str(),
            event.event_kind.as_str(),
            event.claim_id,
            event.composition_id,
            event.previous_version.map(version_to_i64),
            version_to_i64(event.current_version),
            event.reason,
            if event.scope_redacted { 1 } else { 0 },
            event.correction_event_log_id,
            event.mutation_id,
            event.created_at,
            event.actor_kind.as_str(),
        ],
    )?;
    Ok(())
}

pub fn mark_mutation_attempt_committed(
    tx: &ActionDb,
    attempt: &MutationAttempt,
    actual_claim_id: &str,
    finalized_at: &str,
) -> Result<(), rusqlite::Error> {
    tx.conn_ref().execute(
        "UPDATE mutation_attempts
         SET claim_id = ?2,
             composition_id = NULL,
             status = 'committed',
             finalized_at = ?3
         WHERE mutation_id = ?1",
        params![&attempt.mutation_id, actual_claim_id, finalized_at],
    )?;
    Ok(())
}

pub fn mark_composition_mutation_attempt_committed(
    tx: &ActionDb,
    attempt: &MutationAttempt,
    composition_id: &str,
    finalized_at: &str,
) -> Result<(), rusqlite::Error> {
    tx.conn_ref().execute(
        "UPDATE mutation_attempts
         SET claim_id = NULL,
             composition_id = ?2,
             status = 'committed',
             finalized_at = ?3
         WHERE mutation_id = ?1",
        params![&attempt.mutation_id, composition_id, finalized_at],
    )?;
    Ok(())
}

pub fn finalize_mutation_attempt_aborted(
    db: &ActionDb,
    attempt: &MutationAttempt,
    now: DateTime<Utc>,
) -> Result<(), rusqlite::Error> {
    let finalized_at = now.to_rfc3339();
    db.with_transaction(|tx| {
        finalize_mutation_attempt_aborted_tx(tx, attempt, &finalized_at).map_err(|e| e.to_string())
    })
    .map_err(rusqlite::Error::InvalidParameterName)
}

fn finalize_mutation_attempt_aborted_tx(
    tx: &ActionDb,
    attempt: &MutationAttempt,
    finalized_at: &str,
) -> Result<(), rusqlite::Error> {
    let status: Option<String> = tx
        .conn_ref()
        .query_row(
            "SELECT status FROM mutation_attempts WHERE mutation_id = ?1",
            params![&attempt.mutation_id],
            |row| row.get(0),
        )
        .optional()?;
    if status.as_deref() != Some("in_flight") {
        return Ok(());
    }
    tx.conn_ref().execute(
        "UPDATE mutation_attempts
         SET status = 'aborted', finalized_at = ?2
         WHERE mutation_id = ?1",
        params![&attempt.mutation_id, finalized_at],
    )?;
    insert_version_event(
        tx,
        VersionEventInsert {
            cursor: &attempt.cursor,
            event_kind: VersionEventKind::MutationAborted,
            claim_id: attempt.subject.claim_id(),
            composition_id: attempt.subject.composition_id(),
            previous_version: None,
            current_version: 0,
            reason: Some("mutation_aborted"),
            scope_redacted: false,
            correction_event_log_id: None,
            mutation_id: Some(&attempt.mutation_id),
            created_at: finalized_at,
            actor_kind: VersionActorKind::System,
        },
    )?;
    Ok(())
}

pub fn recover_stuck_mutation_attempts(
    db: &ActionDb,
    now: DateTime<Utc>,
) -> Result<usize, rusqlite::Error> {
    let cutoff = (now - Duration::seconds(30)).to_rfc3339();
    let attempts = {
        let mut stmt = db.conn_ref().prepare(
            "SELECT mutation_id, claim_id, composition_id, cursor
             FROM mutation_attempts
             WHERE status = 'in_flight' AND started_at < ?1",
        )?;
        let rows = stmt.query_map(params![cutoff], |row| {
            let claim_id: Option<String> = row.get(1)?;
            let composition_id: Option<String> = row.get(2)?;
            let subject = match (claim_id, composition_id) {
                (Some(id), None) => MutationSubject::Claim(id),
                (None, Some(id)) => MutationSubject::Composition(id),
                _ => {
                    return Err(rusqlite::Error::InvalidColumnType(
                        1,
                        "mutation_subject".to_string(),
                        rusqlite::types::Type::Text,
                    ));
                }
            };
            Ok(MutationAttempt {
                mutation_id: row.get(0)?,
                subject,
                cursor: SignalCursor(row.get(3)?),
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>()?
    };
    let count = attempts.len();
    for attempt in attempts {
        finalize_mutation_attempt_aborted(db, &attempt, now)?;
    }
    Ok(count)
}

pub fn version_to_i64(version: u64) -> i64 {
    i64::try_from(version).unwrap_or(i64::MAX)
}

pub fn checked_next_version(current: u64) -> Option<u64> {
    current
        .checked_add(1)
        .filter(|next| *next <= MAX_PERSISTED_VERSION)
}
