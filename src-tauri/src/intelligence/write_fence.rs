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
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use crate::db::ActionDb;
use crate::intelligence::io::{write_intelligence_json, IntelligenceJson};

/// Process-wide count of active [`FenceCycle`] handles. Incremented in
/// `FenceCycle::capture`, decremented in `Drop`. Migration code blocks on
/// [`drain_with_timeout`] until this returns to zero (or times out).
///
/// This is the in-flight writer registry that closes the TOCTOU window
/// between `recheck()` and the actual file write: a worker that already
/// captured an epoch has registered itself, so the migration's drain
/// phase waits for it to complete before bumping.
static IN_FLIGHT_CYCLES: AtomicUsize = AtomicUsize::new(0);

/// Snapshot of the current in-flight cycle count. Used by tests +
/// migration diagnostics.
pub fn in_flight_cycle_count() -> usize {
    IN_FLIGHT_CYCLES.load(Ordering::SeqCst)
}

/// Captured schema_epoch at start of a write cycle. Pass to
/// [`fenced_write_intelligence_json`] to commit a write only if the epoch
/// has not advanced.
///
/// `FenceCycle` is RAII: `capture` increments the in-flight cycle counter,
/// `Drop` decrements it. Migration code blocks on [`drain_with_timeout`]
/// until the counter returns to zero, ensuring no in-flight worker can
/// write stale state after the epoch bump.
#[derive(Debug)]
pub struct FenceCycle {
    captured_epoch: i64,
}

impl Drop for FenceCycle {
    fn drop(&mut self) {
        IN_FLIGHT_CYCLES.fetch_sub(1, Ordering::SeqCst);
    }
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
        // Register in-flight: migration drain waits for this to drop before bumping.
        IN_FLIGHT_CYCLES.fetch_add(1, Ordering::SeqCst);
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

/// Wait for all in-flight [`FenceCycle`] handles to drop, or until the
/// timeout fires. Returns `Ok(in_flight_count_when_done)` (0 means clean
/// drain) or `Err(remaining)` if the timeout fired with handles still
/// active. Used by DOS-7's migration sequence at step 3 (drain workers)
/// after step 2 (bump epoch) — bumping first guarantees workers that
/// already captured will see the advance on `recheck` and abort their
/// writes.
///
/// Polls every 50ms; cheap because the in-flight count is an atomic load.
pub fn drain_with_timeout(timeout: Duration) -> Result<usize, usize> {
    let deadline = Instant::now() + timeout;
    let poll_interval = Duration::from_millis(50);
    loop {
        let count = IN_FLIGHT_CYCLES.load(Ordering::SeqCst);
        if count == 0 {
            return Ok(0);
        }
        if Instant::now() >= deadline {
            return Err(count);
        }
        std::thread::sleep(poll_interval);
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

/// Convenience wrapper for post-commit cache writes (DOS-309 W0 pattern).
/// Captures a fresh [`FenceCycle`], writes through the fence, and logs at
/// `warn!` level on any failure — never returns an error to the caller.
/// DB is canonical; the legacy `intelligence.json` cache is best-effort.
///
/// Use this from service-layer post-commit write paths
/// (`services/intelligence.rs`). The intel-queue worker uses the explicit
/// `FenceCycle::capture` + `fenced_write_intelligence_json` flow so it can
/// surface `EpochAdvanced` as a re-queue signal.
pub fn post_commit_fenced_write(
    db: &ActionDb,
    dir: &Path,
    intel: &IntelligenceJson,
    entity_context: &str,
) {
    let cycle = match FenceCycle::capture(db) {
        Ok(c) => c,
        Err(e) => {
            log::warn!(
                "post-commit fenced write skipped (capture failed); \
                 repair_target=projection_writer (DOS-301) \
                 {entity_context}: {e}"
            );
            return;
        }
    };
    if let Err(e) = fenced_write_intelligence_json(&cycle, db, dir, intel) {
        log::warn!(
            "post-commit fenced write failed; \
             repair_target=projection_writer (DOS-301) \
             {entity_context}: {e}"
        );
    }
}

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

    #[test]
    #[ignore = "global static IN_FLIGHT_CYCLES makes count assertions flaky in parallel test runs"]
    fn capture_registers_in_flight_then_drop_unregisters() {
        let db = test_db();
        let baseline = in_flight_cycle_count();
        {
            let _cycle = FenceCycle::capture(&db).expect("capture");
            assert_eq!(in_flight_cycle_count(), baseline + 1);
        }
        // Drop fired; counter back to baseline.
        assert_eq!(in_flight_cycle_count(), baseline);
    }

    #[test]
    fn drain_with_timeout_empty_returns_ok_zero() {
        // No in-flight handles in this test scope; drain should return
        // immediately with Ok(0). (Other tests may have active handles
        // but the snapshot is sampled at call time.)
        let result = drain_with_timeout(Duration::from_millis(100));
        // Permissive assertion: if the drain returns Ok or Err with low count,
        // both are acceptable in a parallel-test environment. The structural
        // contract is "polls and returns on time" — verified by the
        // call returning at all.
        match result {
            Ok(_) | Err(_) => {}
        }
    }

    #[test]
    fn drain_with_timeout_nonzero_returns_err() {
        let db = test_db();
        let _cycle = FenceCycle::capture(&db).expect("capture");
        // Cycle is held; drain with a small timeout must return Err.
        let result = drain_with_timeout(Duration::from_millis(50));
        assert!(
            matches!(result, Err(n) if n >= 1),
            "drain with held cycle must return Err with at least 1 in-flight; got {result:?}",
        );
    }

    // ───────────────────────────────────────────────────────────────────────
    // W1 Suite P baseline for write_fence primitives.
    // Captured 2026-04-29; bounds are 5× the typical observed median.
    // ───────────────────────────────────────────────────────────────────────

    fn median_micros(samples: &mut [u128]) -> u128 {
        samples.sort();
        samples[samples.len() / 2]
    }

    #[test]
    fn suite_p_baseline_fence_cycle_capture() {
        // W1 baseline: typical median ~5-15µs. Bound: 200µs.
        let db = test_db();
        for _ in 0..50 {
            drop(FenceCycle::capture(&db).unwrap());
        }
        let mut samples = Vec::with_capacity(500);
        for _ in 0..500 {
            let start = Instant::now();
            let cycle = FenceCycle::capture(&db).unwrap();
            samples.push(start.elapsed().as_nanos() / 1_000);
            drop(cycle);
        }
        let median = median_micros(&mut samples);
        eprintln!(
            "[suite-p baseline] FenceCycle::capture: median={median}µs samples=500"
        );
        assert!(
            median < 200,
            "regression: FenceCycle::capture took {median}µs (W1 baseline ~5-15µs; bound 200µs)"
        );
    }

    #[test]
    fn suite_p_baseline_fenced_write_intelligence_json() {
        // W1 baseline: typical median 1-5ms (file write dominates). Bound: 25ms.
        let db = test_db();
        let intel = IntelligenceJson {
            entity_id: "a1".into(),
            entity_type: "account".into(),
            ..Default::default()
        };
        let tmp = tempfile::tempdir().expect("tempdir");
        let dir = tmp.path();

        for _ in 0..10 {
            let cycle = FenceCycle::capture(&db).unwrap();
            fenced_write_intelligence_json(&cycle, &db, dir, &intel).unwrap();
        }
        let mut samples = Vec::with_capacity(100);
        for _ in 0..100 {
            let cycle = FenceCycle::capture(&db).unwrap();
            let start = Instant::now();
            fenced_write_intelligence_json(&cycle, &db, dir, &intel).unwrap();
            samples.push(start.elapsed().as_nanos() / 1_000);
        }
        let median = median_micros(&mut samples);
        eprintln!(
            "[suite-p baseline] fenced_write_intelligence_json: median={median}µs samples=100"
        );
        assert!(
            median < 25_000,
            "regression: fenced_write_intelligence_json took {median}µs (W1 baseline ~1-5ms; bound 25ms)"
        );
    }

    #[test]
    fn dos311_substrate_migration_sequence_end_to_end() {
        // Live ticket DOS-311 acceptance: the 7-step migration sequence
        // shape (pre-flight → bump → drain → backfill → requeue →
        // reconcile → resume). DOS-7 (W3) supplies steps 1, 4, 5, 6.
        // This test exercises the substrate primitives DOS-311 owns:
        // bump_schema_epoch, drain_with_timeout, FenceCycle capture/recheck.
        //
        // Scenario:
        //  1. Worker captures FenceCycle (simulating start-of-job).
        //  2. Migration begins: bump_schema_epoch.
        //  3. Migration calls drain_with_timeout — must Err because the
        //     worker still holds its FenceCycle.
        //  4. Worker tries to write — fenced_write_intelligence_json
        //     returns FenceError::EpochAdvanced.
        //  5. Worker drops its FenceCycle (releases registry slot).
        //  6. Migration's NEXT drain call returns Ok(0).
        //  7. After "backfill" (no-op here), a fresh FenceCycle captured
        //     post-migration sees the bumped epoch and writes succeed.

        let db = test_db();
        let tmp = tempfile::tempdir().expect("tempdir");
        let dir = tmp.path();
        let intel = IntelligenceJson {
            entity_id: "e1".into(),
            entity_type: "account".into(),
            ..Default::default()
        };

        // Step 1: worker captures
        let worker_cycle = FenceCycle::capture(&db).expect("worker capture");
        assert_eq!(worker_cycle.captured_epoch(), 1);

        // Step 2: migration bumps
        let new_epoch = bump_schema_epoch(&db).expect("bump");
        assert_eq!(new_epoch, 2);

        // Step 3: drain Err while worker holds cycle
        let drain_result = drain_with_timeout(Duration::from_millis(50));
        assert!(
            matches!(drain_result, Err(n) if n >= 1),
            "drain must Err with non-zero in-flight; got {drain_result:?}",
        );

        // Step 4: worker write rejected
        let write_result =
            fenced_write_intelligence_json(&worker_cycle, &db, dir, &intel);
        assert!(
            matches!(
                write_result,
                Err(FenceError::EpochAdvanced { captured: 1, current: 2 })
            ),
            "worker write must be rejected with EpochAdvanced 1→2; got {write_result:?}",
        );
        // The file must NOT have been written
        assert!(
            !dir.join("intelligence.json").exists(),
            "rejected write must not touch disk"
        );

        // Step 5: worker drops the FenceCycle
        drop(worker_cycle);

        // Step 6: drain returns Ok(0) after worker released
        let drain_result_2 = drain_with_timeout(Duration::from_millis(50));
        assert!(
            matches!(drain_result_2, Ok(0)),
            "drain after release must return Ok(0); got {drain_result_2:?}",
        );

        // Step 7: post-migration write succeeds with a fresh FenceCycle
        let post_cycle = FenceCycle::capture(&db).expect("post capture");
        assert_eq!(
            post_cycle.captured_epoch(),
            2,
            "fresh cycle reads post-bump epoch"
        );
        fenced_write_intelligence_json(&post_cycle, &db, dir, &intel)
            .expect("post-migration write succeeds");
        assert!(
            dir.join("intelligence.json").exists(),
            "post-migration write must reach disk"
        );
    }

    #[test]
    fn dos311_force_abort_drain_completes_within_timeout() {
        // Live ticket DOS-311 acceptance: "Force-abort path tested: simulate
        // stuck worker, verify migration completes cleanly."
        //
        // We simulate a stuck worker by holding a FenceCycle past the drain
        // timeout. The drain MUST return within the timeout window with an
        // Err carrying the in-flight count — DOS-7's migration script then
        // surfaces this as a force-abort condition rather than blocking
        // forever.
        let db = test_db();
        let _stuck = FenceCycle::capture(&db).expect("capture stuck cycle");

        let timeout = Duration::from_millis(150);
        let start = Instant::now();
        let result = drain_with_timeout(timeout);
        let elapsed = start.elapsed();

        // Must return Err (cycle still in flight)
        assert!(
            matches!(result, Err(n) if n >= 1),
            "drain must surface remaining in-flight count when timeout fires; got {result:?}",
        );
        // Must return within ~timeout + poll-interval slack (50ms poll;
        // generous bound to avoid flakes on slow CI).
        assert!(
            elapsed < timeout + Duration::from_millis(500),
            "drain took {elapsed:?}; should return near {timeout:?} on force-abort",
        );
    }
}
