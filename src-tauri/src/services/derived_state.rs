//! Derived-state projection ledger.
//!
//! `intelligence_claims` is the durable source of truth for claim-
//! shaped state. Several legacy consumers still read from cached
//! sibling tables and `intelligence.json`; commit_claim keeps them
//! current by running per-target projection rules right after the
//! claim insert.
//!
//! The projections are best-effort: one rule may fail without
//! aborting the authoritative claim, and a `claim_projection_status`
//! row records the outcome. Failed rows are the repair worklist a
//! sibling repair binary picks up.
//!
//! This module owns the substrate types and the status-write surface.
//! The actual projection rules and commit_claim integration are
//! sequenced after this lands so reviewers can audit the contract
//! shape without grepping the rule bodies.
//!
//! ## Invariants
//!
//! - Status writes are append-only per `(claim_id, projection_target)`
//!   primary key; an idempotent rerun of the same target overwrites
//!   the row but never deletes it.
//! - `attempted_at` is supplied by the caller (`ServiceContext.clock`)
//!   so backfill and tests get deterministic ordering.
//! - `succeeded_at` is set only on `committed` / `repaired` rows;
//!   `failed` rows leave it NULL.

use crate::db::ActionDb;
use crate::services::context::ServiceContext;

/// Projection targets for the v1.4.0 dual-projection window. The
/// label values are stable wire-format strings — repair tooling and
/// cross-version manifests reference them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProjectionTarget {
    /// Legacy `entity_assessment` + `entity_quality` tables that
    /// today's render path reads to reconstruct entity intelligence.
    EntityIntelligence,
    /// `success_plans` rows. Knock-on D in the storage-shape review:
    /// unowned legacy table that needs an explicit owner during the
    /// dual-projection window.
    SuccessPlans,
    /// Account AI columns (`company_overview`, `strategic_programs`,
    /// `notes`). Knock-on E: also unowned and needs an explicit owner.
    AccountsColumns,
    /// `intelligence.json` on disk. Sync-best-effort post-DB-commit;
    /// honors the existing schema-epoch fence so a stale worker
    /// can't overwrite a fresher projection.
    IntelligenceJson,
}

impl ProjectionTarget {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::EntityIntelligence => "entity_intelligence",
            Self::SuccessPlans => "success_plans",
            Self::AccountsColumns => "accounts_columns",
            Self::IntelligenceJson => "intelligence_json",
        }
    }

    pub fn try_from_str(s: &str) -> Option<Self> {
        Some(match s {
            "entity_intelligence" => Self::EntityIntelligence,
            "success_plans" => Self::SuccessPlans,
            "accounts_columns" => Self::AccountsColumns,
            "intelligence_json" => Self::IntelligenceJson,
            _ => return None,
        })
    }
}

/// Outcome status recorded per (claim, target) pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionStatus {
    /// Projection succeeded at commit time.
    Committed,
    /// Projection failed; row is on the repair worklist. The
    /// authoritative claim was already committed — failed projections
    /// are best-effort and don't roll back the claim.
    Failed,
    /// A previously-failed projection was successfully reprojected
    /// by the repair worker.
    Repaired,
}

impl ProjectionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Committed => "committed",
            Self::Failed => "failed",
            Self::Repaired => "repaired",
        }
    }

    pub fn try_from_str(s: &str) -> Option<Self> {
        Some(match s {
            "committed" => Self::Committed,
            "failed" => Self::Failed,
            "repaired" => Self::Repaired,
            _ => return None,
        })
    }
}

/// Outcome of a single projection rule. The aggregate of these
/// across all targets is what commit_claim returns to its caller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionOutcome {
    pub target: ProjectionTarget,
    pub status: ProjectionStatus,
    /// Recorded only for `Failed` rows; carries the error class so
    /// repair can branch on it. Customer text never appears here.
    pub error_message: Option<String>,
    pub attempted_at: String,
    pub succeeded_at: Option<String>,
}

/// Errors callers may see from this module. Distinct from a
/// `Failed` status: those are recorded outcomes; these are
/// substrate-level problems (DB unavailable, malformed input).
#[derive(Debug, thiserror::Error)]
pub enum DerivedStateError {
    #[error("ServiceContext mutation gate: {0}")]
    Mode(String),
    #[error("rusqlite error: {0}")]
    Rusqlite(#[from] rusqlite::Error),
}

/// Record a projection outcome. Upserts on the
/// `(claim_id, projection_target)` primary key so a repair pass
/// idempotently overwrites a prior failed row without losing the
/// audit ordering (`attempted_at` is the most-recent attempt).
///
/// `succeeded_at` should be `Some` for `Committed` / `Repaired` and
/// `None` for `Failed`. The contract isn't enforced at the SQL layer
/// because backfill and out-of-band repair flows may want flexibility,
/// but production callers go through the `mark_*` helpers below.
pub fn record_projection_outcome(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    claim_id: &str,
    outcome: &ProjectionOutcome,
) -> Result<(), DerivedStateError> {
    ctx.check_mutation_allowed()
        .map_err(|e| DerivedStateError::Mode(e.to_string()))?;
    db.conn_ref().execute(
        "INSERT INTO claim_projection_status \
         (claim_id, projection_target, status, error_message, attempted_at, succeeded_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6) \
         ON CONFLICT (claim_id, projection_target) DO UPDATE SET \
             status = excluded.status, \
             error_message = excluded.error_message, \
             attempted_at = excluded.attempted_at, \
             succeeded_at = excluded.succeeded_at",
        rusqlite::params![
            claim_id,
            outcome.target.as_str(),
            outcome.status.as_str(),
            outcome.error_message.as_deref(),
            outcome.attempted_at,
            outcome.succeeded_at.as_deref(),
        ],
    )?;
    Ok(())
}

/// Convenience: record a successful projection at the supplied
/// `attempted_at` (typically `ctx.clock.now().to_rfc3339()`).
pub fn mark_committed(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    claim_id: &str,
    target: ProjectionTarget,
    attempted_at: &str,
) -> Result<(), DerivedStateError> {
    record_projection_outcome(
        ctx,
        db,
        claim_id,
        &ProjectionOutcome {
            target,
            status: ProjectionStatus::Committed,
            error_message: None,
            attempted_at: attempted_at.to_string(),
            succeeded_at: Some(attempted_at.to_string()),
        },
    )
}

/// Convenience: record a failed projection. The error class string
/// must NOT contain customer text — it's a class label like
/// `validation_error`, `target_table_locked`, `fence_advanced`.
pub fn mark_failed(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    claim_id: &str,
    target: ProjectionTarget,
    attempted_at: &str,
    error_class: &str,
) -> Result<(), DerivedStateError> {
    record_projection_outcome(
        ctx,
        db,
        claim_id,
        &ProjectionOutcome {
            target,
            status: ProjectionStatus::Failed,
            error_message: Some(error_class.to_string()),
            attempted_at: attempted_at.to_string(),
            succeeded_at: None,
        },
    )
}

/// Convenience: mark a previously-failed projection as repaired.
pub fn mark_repaired(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    claim_id: &str,
    target: ProjectionTarget,
    attempted_at: &str,
) -> Result<(), DerivedStateError> {
    record_projection_outcome(
        ctx,
        db,
        claim_id,
        &ProjectionOutcome {
            target,
            status: ProjectionStatus::Repaired,
            error_message: None,
            attempted_at: attempted_at.to_string(),
            succeeded_at: Some(attempted_at.to_string()),
        },
    )
}

/// Read-side: enumerate the failed-projection worklist for a target.
/// Returns `(claim_id, error_message_or_class)` pairs. The repair
/// binary uses this to drive idempotent reprojection.
pub fn list_failed_projections(
    db: &ActionDb,
    target: ProjectionTarget,
) -> Result<Vec<(String, Option<String>)>, rusqlite::Error> {
    let mut stmt = db.conn_ref().prepare(
        "SELECT claim_id, error_message \
         FROM claim_projection_status \
         WHERE projection_target = ?1 AND status = 'failed' \
         ORDER BY attempted_at",
    )?;
    let rows = stmt.query_map(rusqlite::params![target.as_str()], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
    })?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_utils::test_db;
    use crate::services::context::{ExternalClients, FixedClock, SeedableRng, ServiceContext};
    use chrono::TimeZone;
    use rusqlite::params;

    const TS: &str = "2026-05-03T12:00:00+00:00";

    fn ctx_parts() -> (FixedClock, SeedableRng, ExternalClients) {
        (
            FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 3, 12, 0, 0).unwrap()),
            SeedableRng::new(7),
            ExternalClients::default(),
        )
    }

    fn live_ctx<'a>(
        clock: &'a FixedClock,
        rng: &'a SeedableRng,
        external: &'a ExternalClients,
    ) -> ServiceContext<'a> {
        ServiceContext::test_live(clock, rng, external)
    }

    fn seed_claim(db: &ActionDb, claim_id: &str) {
        // Minimal subject + claim row so the FK on
        // claim_projection_status.claim_id has something to point at.
        db.conn_ref()
            .execute(
                "INSERT OR IGNORE INTO accounts (id, name, updated_at) VALUES (?1, ?2, ?3)",
                params!["acct-1", "Acme", TS],
            )
            .unwrap();
        db.conn_ref()
            .execute(
                "INSERT INTO intelligence_claims /* dos7-allowed: test seed for FK target */ (\
                    id, subject_ref, claim_type, text, dedup_key, actor, \
                    data_source, observed_at, provenance_json) \
                 VALUES (?1, ?2, 'risk', 'r', ?3, 'system', 'manual', ?4, '{}')",
                params![
                    claim_id,
                    "{\"kind\":\"account\",\"id\":\"acct-1\"}",
                    format!("hash:{claim_id}"),
                    TS,
                ],
            )
            .unwrap();
    }

    #[test]
    fn projection_target_strings_round_trip() {
        for t in [
            ProjectionTarget::EntityIntelligence,
            ProjectionTarget::SuccessPlans,
            ProjectionTarget::AccountsColumns,
            ProjectionTarget::IntelligenceJson,
        ] {
            assert_eq!(ProjectionTarget::try_from_str(t.as_str()), Some(t));
        }
        assert_eq!(ProjectionTarget::try_from_str("not_a_target"), None);
    }

    #[test]
    fn projection_status_strings_round_trip() {
        for s in [
            ProjectionStatus::Committed,
            ProjectionStatus::Failed,
            ProjectionStatus::Repaired,
        ] {
            assert_eq!(ProjectionStatus::try_from_str(s.as_str()), Some(s));
        }
        assert_eq!(ProjectionStatus::try_from_str("queued"), None);
    }

    #[test]
    fn mark_committed_writes_a_committed_row() {
        let db = test_db();
        seed_claim(&db, "claim-1");
        let (clock, rng, ext) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &ext);

        mark_committed(
            &ctx,
            &db,
            "claim-1",
            ProjectionTarget::EntityIntelligence,
            TS,
        )
        .unwrap();

        let (status, succeeded_at, attempted_at, err_msg): (
            String,
            Option<String>,
            String,
            Option<String>,
        ) = db
            .conn_ref()
            .query_row(
                "SELECT status, succeeded_at, attempted_at, error_message \
                 FROM claim_projection_status \
                 WHERE claim_id = 'claim-1' AND projection_target = 'entity_intelligence'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap();
        assert_eq!(status, "committed");
        assert_eq!(succeeded_at.as_deref(), Some(TS));
        assert_eq!(attempted_at, TS);
        assert!(err_msg.is_none());
    }

    #[test]
    fn mark_failed_records_error_class_without_succeeded_at() {
        let db = test_db();
        seed_claim(&db, "claim-2");
        let (clock, rng, ext) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &ext);

        mark_failed(
            &ctx,
            &db,
            "claim-2",
            ProjectionTarget::IntelligenceJson,
            TS,
            "fence_advanced",
        )
        .unwrap();

        let (status, succeeded_at, err_msg): (String, Option<String>, Option<String>) = db
            .conn_ref()
            .query_row(
                "SELECT status, succeeded_at, error_message \
                 FROM claim_projection_status \
                 WHERE claim_id = 'claim-2' AND projection_target = 'intelligence_json'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(status, "failed");
        assert_eq!(succeeded_at, None);
        assert_eq!(err_msg.as_deref(), Some("fence_advanced"));
    }

    #[test]
    fn mark_repaired_overwrites_prior_failed_row_idempotently() {
        let db = test_db();
        seed_claim(&db, "claim-3");
        let (clock, rng, ext) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &ext);

        // First attempt fails.
        mark_failed(
            &ctx,
            &db,
            "claim-3",
            ProjectionTarget::SuccessPlans,
            TS,
            "validation_error",
        )
        .unwrap();
        // Repair attempt succeeds at a later attempted_at.
        let later = "2026-05-03T13:00:00+00:00";
        mark_repaired(
            &ctx,
            &db,
            "claim-3",
            ProjectionTarget::SuccessPlans,
            later,
        )
        .unwrap();

        // ON CONFLICT should have UPDATED the row, not inserted a
        // second one.
        let count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT count(*) FROM claim_projection_status \
                 WHERE claim_id = 'claim-3' AND projection_target = 'success_plans'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
        let (status, succeeded_at, attempted_at, err_msg): (
            String,
            Option<String>,
            String,
            Option<String>,
        ) = db
            .conn_ref()
            .query_row(
                "SELECT status, succeeded_at, attempted_at, error_message \
                 FROM claim_projection_status \
                 WHERE claim_id = 'claim-3' AND projection_target = 'success_plans'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap();
        assert_eq!(status, "repaired");
        assert_eq!(succeeded_at.as_deref(), Some(later));
        assert_eq!(attempted_at, later);
        // Error class is cleared on repair.
        assert!(err_msg.is_none());
    }

    #[test]
    fn list_failed_projections_returns_only_failed_rows_for_target() {
        let db = test_db();
        seed_claim(&db, "claim-4");
        seed_claim(&db, "claim-5");
        seed_claim(&db, "claim-6");
        let (clock, rng, ext) = ctx_parts();
        let ctx = live_ctx(&clock, &rng, &ext);

        // Two failed for IntelligenceJson, one committed.
        mark_failed(
            &ctx,
            &db,
            "claim-4",
            ProjectionTarget::IntelligenceJson,
            TS,
            "fence_advanced",
        )
        .unwrap();
        mark_failed(
            &ctx,
            &db,
            "claim-5",
            ProjectionTarget::IntelligenceJson,
            TS,
            "io_error",
        )
        .unwrap();
        mark_committed(
            &ctx,
            &db,
            "claim-6",
            ProjectionTarget::IntelligenceJson,
            TS,
        )
        .unwrap();
        // One failed for a different target should not appear.
        mark_failed(
            &ctx,
            &db,
            "claim-4",
            ProjectionTarget::SuccessPlans,
            TS,
            "validation_error",
        )
        .unwrap();

        let mut rows = list_failed_projections(&db, ProjectionTarget::IntelligenceJson).unwrap();
        rows.sort_by(|a, b| a.0.cmp(&b.0));
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].0, "claim-4");
        assert_eq!(rows[0].1.as_deref(), Some("fence_advanced"));
        assert_eq!(rows[1].0, "claim-5");
        assert_eq!(rows[1].1.as_deref(), Some("io_error"));
    }
}
