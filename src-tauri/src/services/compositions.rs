#[cfg(test)]
use abilities_runtime::abilities::composition::{AbilityRef, CompositionKind, CompositionMetadata};
use abilities_runtime::abilities::composition::{
    Composition, CompositionDocId, CompositionVersion,
};
#[cfg(test)]
use abilities_runtime::abilities::provenance::SchemaVersion;
use chrono::{DateTime, Utc};
use rusqlite::{params, OptionalExtension};
use thiserror::Error;

use crate::db::ActionDb;
use crate::services::context::ServiceContext;
use crate::services::versioning::{
    checked_next_version, insert_version_event, mark_composition_mutation_attempt_committed,
    version_to_i64, MutationGuard, VersionActorKind, VersionEventInsert, VersionEventKind,
};

#[derive(Debug, Clone)]
pub struct CompositionProposal {
    pub composition_id: CompositionDocId,
    pub expected_composition_version: u64,
    pub composition: Composition,
}

#[derive(Debug, Clone)]
pub struct CommittedComposition {
    pub composition_id: CompositionDocId,
    pub composition_version: u64,
    pub composition: Composition,
}

#[derive(Debug, Error)]
pub enum CompositionError {
    #[error("composition id is empty")]
    EmptyCompositionId,
    #[error(
        "stale composition version for {composition_id}: expected {expected}, current {current}"
    )]
    StaleVersion {
        composition_id: String,
        expected: u64,
        current: u64,
    },
    /// Caller-presented `expected_composition_version` is greater than the
    /// substrate's current value (fabricated). Wire-compatible with
    /// `StaleVersion` (HTTP 409 `stale_composition_watermark`), but emits
    /// `inflated_version_rejected` on the audit channel.
    #[error(
        "inflated composition version for {composition_id}: expected {expected}, current {current}"
    )]
    InflatedVersion {
        composition_id: String,
        expected: u64,
        current: u64,
    },
    #[error("composition version overflow for {composition_id}")]
    Overflow { composition_id: String },
    #[error("composition transaction failed: {0}")]
    Transaction(String),
    #[error("composition mutation blocked by mode: {0}")]
    Mode(String),
}

impl From<rusqlite::Error> for CompositionError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Transaction(error.to_string())
    }
}

pub fn commit_composition(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    proposal: CompositionProposal,
) -> Result<CommittedComposition, CompositionError> {
    ctx.check_mutation_allowed()
        .map_err(|error| CompositionError::Mode(error.to_string()))?;

    let composition_id = proposal.composition_id.as_str().trim().to_string();
    if composition_id.is_empty() {
        return Err(CompositionError::EmptyCompositionId);
    }

    let input_version = proposal.composition.metadata.composition_version.0;
    if input_version != 0 {
        log::warn!(
            "overwriting ability-supplied composition version composition_id={} supplied_version={}",
            composition_id,
            input_version
        );
    }

    let mut mutation_guard =
        MutationGuard::reserve_composition(db, composition_id.clone(), ctx.clock.now())?;
    let actor_kind = VersionActorKind::from_service_actor(ctx.actor);

    let committed = composition_transaction(db, || {
        let now = ctx.clock.now();
        commit_composition_tx(
            db,
            proposal,
            mutation_guard.attempt(),
            &composition_id,
            now,
            actor_kind,
            ctx,
        )
    });

    let committed = match committed {
        Ok(value) => value,
        Err(error) => {
            // Emit `composition.write_rejected` at the reserved cursor in a
            // dedicated Tx so the audit row is durable even though the
            // mutation Tx rolled back. Suppresses the Drop's
            // `mutation_aborted` for these classified rejection paths.
            let rejection_reason = match &error {
                CompositionError::StaleVersion { current, .. } => {
                    Some(("stale_composition_watermark", Some(*current)))
                }
                CompositionError::InflatedVersion { current, .. } => {
                    Some(("inflated_version_rejected", Some(*current)))
                }
                CompositionError::Overflow { .. } => {
                    Some(("composition_version_overflow", None))
                }
                _ => None,
            };
            if let Some((reason, current_version)) = rejection_reason {
                let now = ctx.clock.now().to_rfc3339();
                if let Err(audit_err) = record_composition_write_rejected_event(
                    db,
                    mutation_guard.attempt(),
                    &composition_id,
                    current_version,
                    reason,
                    &now,
                    actor_kind,
                ) {
                    log::warn!(
                        "failed to record composition.write_rejected event mutation_id={} error={audit_err}",
                        mutation_guard.attempt().mutation_id
                    );
                }
                mutation_guard.mark_completed();
            }
            return Err(error);
        }
    };

    mutation_guard.mark_completed();
    Ok(committed)
}

/// Side-Tx helper to mark a composition mutation attempt aborted and emit a
/// `composition.write_rejected` event at the reserved cursor. Mirrors the
/// claim-side `record_claim_write_rejected_event` helper.
fn record_composition_write_rejected_event(
    db: &ActionDb,
    attempt: &crate::services::versioning::MutationAttempt,
    composition_id: &str,
    current_version: Option<u64>,
    reason: &str,
    now: &str,
    actor_kind: VersionActorKind,
) -> Result<(), CompositionError> {
    db.with_transaction(|tx| {
        tx.conn_ref()
            .execute(
                "UPDATE mutation_attempts \
                 SET status = 'aborted', finalized_at = ?2 \
                 WHERE mutation_id = ?1 AND status = 'in_flight'",
                params![&attempt.mutation_id, now],
            )
            .map_err(|e| e.to_string())?;
        insert_version_event(
            tx,
            VersionEventInsert {
                cursor: &attempt.cursor,
                event_kind: VersionEventKind::CompositionWriteRejected,
                claim_id: None,
                composition_id: Some(composition_id),
                previous_version: None,
                current_version: current_version.unwrap_or(0),
                reason: Some(reason),
                scope_redacted: false,
                correction_event_log_id: None,
                mutation_id: Some(&attempt.mutation_id),
                created_at: now,
                actor_kind,
            },
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    })
    .map_err(CompositionError::Transaction)
}

fn composition_transaction<T, F>(db: &ActionDb, f: F) -> Result<T, CompositionError>
where
    F: FnOnce() -> Result<T, CompositionError>,
{
    if !db.conn_ref().is_autocommit() {
        return f();
    }

    db.conn_ref().execute_batch("BEGIN IMMEDIATE")?;
    match f() {
        Ok(value) => {
            db.conn_ref().execute_batch("COMMIT")?;
            Ok(value)
        }
        Err(error) => {
            if let Err(rollback_error) = db.conn_ref().execute_batch("ROLLBACK") {
                log::warn!("failed to roll back composition transaction: {rollback_error}");
            }
            Err(error)
        }
    }
}

fn commit_composition_tx(
    tx: &ActionDb,
    mut proposal: CompositionProposal,
    attempt: &crate::services::versioning::MutationAttempt,
    composition_id: &str,
    now: DateTime<Utc>,
    actor_kind: VersionActorKind,
    ctx: &ServiceContext<'_>,
) -> Result<CommittedComposition, CompositionError> {
    let current = tx
        .conn_ref()
        .query_row(
            "SELECT composition_version FROM composition_versions WHERE composition_id = ?1",
            params![composition_id],
            |row| row.get::<_, i64>(0),
        )
        .optional()?
        .map(|value| {
            u64::try_from(value).map_err(|_| {
                CompositionError::Transaction(format!(
                    "composition {composition_id} has negative composition_version {value}"
                ))
            })
        })
        .transpose()?;

    let (previous_version, assigned_version) = match current {
        Some(current_version) if proposal.expected_composition_version == current_version => {
            let next = checked_next_version(current_version).ok_or_else(|| {
                CompositionError::Overflow {
                    composition_id: composition_id.to_string(),
                }
            })?;
            tx.conn_ref().execute(
                "UPDATE composition_versions
                 SET composition_version = ?2,
                     generated_at = ?3,
                     generated_by_invocation_id = ?4,
                     generated_by_actor_kind = ?5
                 WHERE composition_id = ?1",
                params![
                    composition_id,
                    version_to_i64(next),
                    now.to_rfc3339(),
                    generated_by_invocation_id(ctx, &proposal.composition),
                    actor_kind.as_str(),
                ],
            )?;
            (Some(current_version), next)
        }
        Some(current_version) => {
            // Branch inflated (expected > current) from stale (expected <
            // current). Both still surface as HTTP 409
            // `stale_composition_watermark`, but the trust-system-facing
            // audit event_kind differs per packet §6 / §6.5 / ac §34.
            if proposal.expected_composition_version > current_version {
                return Err(CompositionError::InflatedVersion {
                    composition_id: composition_id.to_string(),
                    expected: proposal.expected_composition_version,
                    current: current_version,
                });
            }
            return Err(CompositionError::StaleVersion {
                composition_id: composition_id.to_string(),
                expected: proposal.expected_composition_version,
                current: current_version,
            });
        }
        None if proposal.expected_composition_version == 0 => {
            tx.conn_ref().execute(
                "INSERT INTO composition_versions
                 (composition_id, composition_version, generated_at,
                  generated_by_invocation_id, generated_by_actor_kind)
                 VALUES (?1, 1, ?2, ?3, ?4)",
                params![
                    composition_id,
                    now.to_rfc3339(),
                    generated_by_invocation_id(ctx, &proposal.composition),
                    actor_kind.as_str(),
                ],
            )?;
            (None, 1)
        }
        None => {
            // No row yet, caller asserted a non-zero expected — that's
            // an inflated/fabricated version (substrate has no history
            // to be stale against).
            return Err(CompositionError::InflatedVersion {
                composition_id: composition_id.to_string(),
                expected: proposal.expected_composition_version,
                current: 0,
            });
        }
    };

    proposal.composition.id = proposal.composition_id.clone();
    proposal.composition.generated_at = now;
    proposal.composition.metadata.generated_at = now;
    proposal.composition.metadata.composition_version = CompositionVersion::new(assigned_version);

    let created_at = now.to_rfc3339();
    insert_version_event(
        tx,
        VersionEventInsert {
            cursor: &attempt.cursor,
            event_kind: VersionEventKind::CompositionUpdated,
            claim_id: None,
            composition_id: Some(composition_id),
            previous_version,
            current_version: assigned_version,
            reason: if previous_version.is_none() {
                Some("composition_version_bootstrap")
            } else {
                None
            },
            scope_redacted: false,
            correction_event_log_id: None,
            mutation_id: Some(&attempt.mutation_id),
            created_at: &created_at,
            actor_kind,
        },
    )?;
    mark_composition_mutation_attempt_committed(tx, attempt, composition_id, &created_at)?;

    Ok(CommittedComposition {
        composition_id: proposal.composition_id,
        composition_version: assigned_version,
        composition: proposal.composition,
    })
}

fn generated_by_invocation_id(ctx: &ServiceContext<'_>, composition: &Composition) -> String {
    ctx.ability_id
        .map(str::to_string)
        .unwrap_or_else(|| composition.generated_by.as_str().to_string())
}

#[cfg(test)]
fn empty_composition(id: &str, version: u64, generated_at: DateTime<Utc>) -> Composition {
    Composition {
        id: CompositionDocId::new(id),
        kind: CompositionKind::EntityPage,
        subject: None,
        sections: Vec::new(),
        salience: Default::default(),
        generated_at,
        generated_by: AbilityRef::new("test_composer"),
        metadata: CompositionMetadata {
            schema_version: SchemaVersion(1),
            generated_at,
            composition_version: CompositionVersion::new(version),
            generated_by: "test_composer".to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::ActionDb;
    use crate::services::context::{
        Clock, ExternalClients, FixedClock, SeedableRng, ServiceContext,
    };
    use chrono::TimeZone;

    fn test_ctx<'a>(
        clock: &'a FixedClock,
        rng: &'a SeedableRng,
        ext: &'a ExternalClients,
    ) -> ServiceContext<'a> {
        ServiceContext::test_live(clock, rng, ext).with_actor("agent:test_composer")
    }

    fn test_db() -> ActionDb {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("composition.sqlite");
        std::mem::forget(dir);
        ActionDb::open_at_unencrypted(path).expect("open test db")
    }

    #[test]
    fn commit_composition_bootstraps_then_rejects_stale_version() {
        let db = test_db();
        let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 13, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(7);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);

        let proposal = CompositionProposal {
            composition_id: CompositionDocId::new("composition-1"),
            expected_composition_version: 0,
            composition: empty_composition("composition-1", 99, clock.now()),
        };
        let committed = commit_composition(&ctx, &db, proposal).expect("bootstrap composition");
        assert_eq!(committed.composition_version, 1);
        assert_eq!(committed.composition.metadata.composition_version.0, 1);

        let stale = CompositionProposal {
            composition_id: CompositionDocId::new("composition-1"),
            expected_composition_version: 0,
            composition: empty_composition("composition-1", 1, clock.now()),
        };
        let error = commit_composition(&ctx, &db, stale).expect_err("stale version rejected");
        assert!(matches!(
            error,
            CompositionError::StaleVersion {
                expected: 0,
                current: 1,
                ..
            }
        ));
    }

    #[test]
    fn commit_composition_writes_version_event_atomically() {
        let db = test_db();
        let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 13, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(7);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);

        let proposal = CompositionProposal {
            composition_id: CompositionDocId::new("composition-events"),
            expected_composition_version: 0,
            composition: empty_composition("composition-events", 0, clock.now()),
        };
        commit_composition(&ctx, &db, proposal).expect("commit composition");

        let (event_kind, current_version, attempts): (String, i64, i64) = db
            .conn_ref()
            .query_row(
                "SELECT ve.event_kind, ve.current_version, COUNT(ma.mutation_id)
                 FROM version_events ve
                 JOIN mutation_attempts ma ON ma.mutation_id = ve.mutation_id
                 WHERE ve.composition_id = 'composition-events'
                 GROUP BY ve.event_kind, ve.current_version",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("event row");
        assert_eq!(event_kind, "composition.updated");
        assert_eq!(current_version, 1);
        assert_eq!(attempts, 1);
    }
}
