//! DOS-311: schema-epoch fence for `intelligence.json` writes.
//!
//! The fence prevents stale writes during the W3 / DOS-7 cutover. The flow:
//!
//! 1. A worker captures the current `migration_state.schema_epoch` at
//!    job pickup via [`FenceCycle::capture`].
//! 2. The worker does its enrichment / mutation work (potentially seconds
//!    to minutes for PTY/Glean paths).
//! 3. Before writing `intelligence.json`, the worker passes its `FenceCycle`
//!    to [`fenced_write_intelligence_json`]. The fence re-reads the epoch;
//!    if it has advanced (because DOS-7's migration ran mid-flight), the
//!    write is rejected with [`FenceError::EpochAdvanced`].
//! 4. The caller treats `EpochAdvanced` as a soft skip: log, do not roll
//!    back DB state (DB is canonical), and re-enqueue the work for the
//!    next cycle.
//!
//! ## Cross-issue dependency note
//!
//! The live DOS-311 ticket also requires `--repair` mode that consumes
//! `services/claims.rs::commit_claim` (DOS-7) and a reconcile pass over
//! `intelligence_claims` (DOS-7 schema). Both ship in W3. This module
//! ships the substrate primitive (epoch capture + recheck on write); the
//! reconcile + repair binary land alongside DOS-7.

use std::path::Path;

use crate::db::ActionDb;
use crate::intelligence::io::{write_intelligence_json, IntelligenceJson};

/// Captured schema_epoch at start of a write cycle. Pass to
/// [`fenced_write_intelligence_json`] to commit a write only if the epoch
/// has not advanced.
#[derive(Debug, Clone, Copy)]
pub struct FenceCycle {
    captured_epoch: i64,
}

impl FenceCycle {
    /// Read the current `migration_state.schema_epoch` and bind it to a
    /// `FenceCycle` handle. Workers call this at job pickup; the handle
    /// flows through enrichment until write-back.
    pub fn capture(db: &ActionDb) -> Result<Self, String> {
        let captured_epoch: i64 = db
            .conn_ref()
            .query_row(
                "SELECT value FROM migration_state WHERE key = 'schema_epoch'",
                [],
                |r| r.get(0),
            )
            .map_err(|e| format!("schema_epoch capture: {e}"))?;
        Ok(Self { captured_epoch })
    }

    /// The captured epoch value. Useful for diagnostics; callers normally
    /// pass the whole [`FenceCycle`] to [`fenced_write_intelligence_json`].
    pub fn captured_epoch(&self) -> i64 {
        self.captured_epoch
    }

    /// Re-read the current epoch and compare against the captured value.
    /// `Ok(())` if unchanged; `Err(FenceError::EpochAdvanced)` otherwise.
    pub fn recheck(&self, db: &ActionDb) -> Result<(), FenceError> {
        let current: i64 = db
            .conn_ref()
            .query_row(
                "SELECT value FROM migration_state WHERE key = 'schema_epoch'",
                [],
                |r| r.get(0),
            )
            .map_err(|e| FenceError::DbRead(e.to_string()))?;
        if current != self.captured_epoch {
            return Err(FenceError::EpochAdvanced {
                captured: self.captured_epoch,
                current,
            });
        }
        Ok(())
    }
}

/// Bump `migration_state.schema_epoch`. Called by DOS-7's migration script
/// at step 2 of the 7-step sequence (pre-flight log → bump → drain → backfill →
/// requeue → reconcile → resume). Must only be called from migration code.
#[must_use = "schema_epoch bumps must be propagated; silent discard breaks cutover safety"]
pub fn bump_schema_epoch(db: &ActionDb) -> Result<i64, String> {
    db.conn_ref()
        .execute(
            "UPDATE migration_state SET value = value + 1 WHERE key = 'schema_epoch'",
            [],
        )
        .map_err(|e| format!("bump_schema_epoch UPDATE: {e}"))?;
    let new_value: i64 = db
        .conn_ref()
        .query_row(
            "SELECT value FROM migration_state WHERE key = 'schema_epoch'",
            [],
            |r| r.get(0),
        )
        .map_err(|e| format!("bump_schema_epoch read-back: {e}"))?;
    Ok(new_value)
}

/// Errors from the write fence.
#[derive(Debug)]
pub enum FenceError {
    /// `migration_state.schema_epoch` could not be read.
    DbRead(String),
    /// The epoch advanced between [`FenceCycle::capture`] and the write.
    /// The write was NOT performed; caller logs + re-queues.
    EpochAdvanced { captured: i64, current: i64 },
    /// The underlying [`write_intelligence_json`] call failed (disk full,
    /// permissions, etc.). Treat as best-effort cache write per the
    /// post-W0 DB-first contract.
    WriteFailed(String),
}

impl std::fmt::Display for FenceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DbRead(e) => write!(f, "fence DB read failed: {e}"),
            Self::EpochAdvanced { captured, current } => write!(
                f,
                "fence epoch advanced (captured={captured}, current={current}); \
                 migration ran mid-cycle; caller should re-queue work"
            ),
            Self::WriteFailed(e) => write!(f, "fence-wrapped write_intelligence_json failed: {e}"),
        }
    }
}

impl std::error::Error for FenceError {}

/// Write `intelligence.json` IF the schema_epoch is unchanged since
/// `cycle.capture`. Otherwise return [`FenceError::EpochAdvanced`].
///
/// This is the universal write fence: every production caller of
/// `write_intelligence_json` SHOULD route through this function. The bash
/// CI lint `scripts/check_write_fence_usage.sh` enforces (post-W3 cleanup
/// removes any transitional allowlist entries).
#[must_use = "fence-wrapped write results must be propagated; silent discard regresses cutover safety"]
pub fn fenced_write_intelligence_json(
    cycle: &FenceCycle,
    db: &ActionDb,
    dir: &Path,
    intel: &IntelligenceJson,
) -> Result<(), FenceError> {
    cycle.recheck(db)?;
    write_intelligence_json(dir, intel).map_err(FenceError::WriteFailed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_utils::test_db;

    #[test]
    fn capture_reads_initial_epoch_one() {
        let db = test_db();
        let cycle = FenceCycle::capture(&db).expect("capture");
        assert_eq!(cycle.captured_epoch(), 1);
    }

    #[test]
    fn recheck_succeeds_when_epoch_unchanged() {
        let db = test_db();
        let cycle = FenceCycle::capture(&db).expect("capture");
        assert!(cycle.recheck(&db).is_ok());
    }

    #[test]
    fn recheck_fails_when_epoch_advanced() {
        let db = test_db();
        let cycle = FenceCycle::capture(&db).expect("capture");
        let new_value = bump_schema_epoch(&db).expect("bump");
        assert_eq!(new_value, 2);
        match cycle.recheck(&db) {
            Err(FenceError::EpochAdvanced { captured, current }) => {
                assert_eq!(captured, 1);
                assert_eq!(current, 2);
            }
            other => panic!("expected EpochAdvanced, got {other:?}"),
        }
    }

    #[test]
    fn fenced_write_rejects_when_epoch_advanced() {
        let db = test_db();
        let cycle = FenceCycle::capture(&db).expect("capture");
        bump_schema_epoch(&db).expect("bump");

        let tmp = tempfile::tempdir().expect("tempdir");
        let intel = IntelligenceJson {
            entity_id: "e1".into(),
            entity_type: "account".into(),
            ..Default::default()
        };
        match fenced_write_intelligence_json(&cycle, &db, tmp.path(), &intel) {
            Err(FenceError::EpochAdvanced { .. }) => {}
            other => panic!("expected EpochAdvanced, got {other:?}"),
        }
        // The file must NOT have been written when the fence rejected.
        assert!(
            !tmp.path().join("intelligence.json").exists(),
            "fence-rejected write must not touch disk"
        );
    }

    #[test]
    fn fenced_write_succeeds_when_epoch_unchanged() {
        let db = test_db();
        let cycle = FenceCycle::capture(&db).expect("capture");

        let tmp = tempfile::tempdir().expect("tempdir");
        let intel = IntelligenceJson {
            entity_id: "e1".into(),
            entity_type: "account".into(),
            ..Default::default()
        };
        fenced_write_intelligence_json(&cycle, &db, tmp.path(), &intel)
            .expect("happy path write");
        assert!(tmp.path().join("intelligence.json").exists());
    }

    #[test]
    fn bump_increments_epoch() {
        let db = test_db();
        assert_eq!(bump_schema_epoch(&db).expect("bump 1"), 2);
        assert_eq!(bump_schema_epoch(&db).expect("bump 2"), 3);
    }
}
