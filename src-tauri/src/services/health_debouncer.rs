//! Regression guard: Backend-side debounce for post-edit health recompute.
//!
//! Previously, every health-relevant field edit synchronously invoked
//! `recompute_entity_health` inside `services::accounts::update_account_field`.
//! Ten rapid edits in under two seconds produced ten recomputes — wasteful and
//! racy when the scoring pass overlapped with subsequent writes. The frontend
//! debounce could not be trusted because:
//!   1. AI agents and automation can call `update_account_field` directly.
//!   2. Chat/correction backends  skip the UI debounce entirely.
//!   3. Health recompute cost scales with signal volume; we must coalesce.
//!
//! This module provides a per-account debouncer. Each call to
//! `schedule_recompute(account_id)` updates a "last requested at" timestamp
//! for that account and spawns a Tauri async task that sleeps for the debounce
//! window. When it wakes, it compares the stored timestamp against the one it
//! captured before sleeping — if any newer edit has landed, it exits without
//! recomputing. Exactly one recompute per quiet window.
//!
//! Exactly-one semantics per burst:
//! - Fire edits 1..10 at t0, t0+50ms, ..., t0+450ms.
//! - Each edit stores `Instant::now()` under its account_id and spawns a task.
//! - At t0+2000ms the first task wakes, sees its captured instant is stale
//!   (some other task wrote a newer one), and no-ops.
//! - Same for tasks 2..9.
//! - Task 10 wakes at t0+2450ms, finds its captured instant still matches,
//!   runs the recompute, and clears the entry.
//! - Final state reflects the LAST edit because recompute reads the account
//!   row from the DB after all writes have committed.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;

use crate::state::AppState;

/// Debounce window between the last health-relevant edit and the recompute.
/// Matches the prior frontend debounce — 2 seconds is long enough to absorb
/// a rapid slider/arrow-key storm without making the UI feel laggy.
pub const HEALTH_RECOMPUTE_DEBOUNCE_MS: u64 = 2_000;

/// Per-account last-requested timestamp. Kept deliberately small — the map
/// only holds accounts with a pending recompute and is cleared on flush.
#[derive(Default)]
pub struct HealthRecomputeDebouncer {
    pending: Mutex<HashMap<String, Instant>>,
}

impl HealthRecomputeDebouncer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of accounts with a pending recompute — test hook only.
    #[cfg(test)]
    pub fn pending_count(&self) -> usize {
        self.pending.lock().len()
    }

    /// Record an edit for `account_id`. Returns the timestamp that callers
    /// must present at flush time. If this timestamp is still the latest
    /// when the debounce window expires, the caller runs the recompute.
    fn record(&self, account_id: &str) -> Instant {
        let now = Instant::now();
        self.pending.lock().insert(account_id.to_string(), now);
        now
    }

    /// Attempt to claim the recompute slot for `account_id`.
    ///
    /// Returns true if `captured` is still the latest recorded timestamp for
    /// this account — the caller owns the recompute and the entry is cleared.
    /// Returns false if a newer edit has landed; the caller should bail.
    fn try_claim(&self, account_id: &str, captured: Instant) -> bool {
        let mut guard = self.pending.lock();
        match guard.get(account_id) {
            Some(latest) if *latest == captured => {
                guard.remove(account_id);
                true
            }
            _ => false,
        }
    }
}

/// Schedule a debounced health recompute for `account_id`.
///
/// Call this from mutation paths that were previously invoking
/// `recompute_entity_health` synchronously. The function returns immediately;
/// the recompute is executed on the Tauri async runtime after the debounce
/// window closes, and only if no newer edit has landed in the meantime.
///
/// this function is **timing/coalescing only**. The durable
/// `health_recompute_pending` marker MUST be written by the caller on the
/// same writer connection that committed the triggering mutation, BEFORE
/// calling this function. If we owned marker persistence here — even
/// synchronously via `db_write().await` — a crash or runtime shutdown
/// between the mutation's commit and the debouncer's marker write would
/// silently lose the pending recompute. The startup `drain_pending` is the
/// backstop, so the marker must be committed in the same transaction
/// boundary as the edit that triggered it.
pub fn schedule_recompute(
    ctx: &crate::services::context::ServiceContext<'_>,
    state: &Arc<AppState>,
    account_id: &str,
) {
    if ctx.check_mutation_allowed().is_err() {
        return;
    }
    let captured = state.health_recompute_debouncer.record(account_id);
    let state_clone = state.clone();
    let account_id = account_id.to_string();

    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_millis(HEALTH_RECOMPUTE_DEBOUNCE_MS)).await;

        if !state_clone
            .health_recompute_debouncer
            .try_claim(&account_id, captured)
        {
            log::debug!(
                "DOS-228: health recompute for {} superseded by newer edit",
                account_id
            );
            return;
        }

        // Flush through the db_write path so the recompute sees committed
        // writes from all coalesced edits.
        let id_for_write = account_id.clone();
        let write_result = state_clone
            .db_write(move |db| {
                let clock = crate::services::context::SystemClock;
                let rng = crate::services::context::SystemRng;
                let ext = crate::services::context::ExternalClients::default();
                let ctx = crate::services::context::ServiceContext::new_live(&clock, &rng, &ext);
                crate::services::intelligence::recompute_entity_health(&ctx, db, &id_for_write, "account")
            })
            .await;

        match write_result {
            Ok(()) => {
                log::info!(
                    "DOS-228: debounced health recompute complete for {}",
                    account_id
                );
                // Clear the durable marker so the next startup drain does
                // not redo this work.
                let clear_id = account_id.clone();
                let _ = state_clone
                    .db_write(move |db| {
                        db.clear_health_recompute_pending(&clear_id)
                            .map_err(|e| e.to_string())
                    })
                    .await;
            }
            Err(e) => log::warn!(
                "DOS-228: debounced health recompute failed for {}: {} (marker retained for startup retry)",
                account_id,
                e
            ),
        }
    });
}

/// Drain persisted `health_recompute_pending`
/// markers on app startup. Any row surviving a crash gets its recompute run
/// synchronously (from the caller's perspective — each is a separate
/// `db_write`) and, on success, its marker cleared. Failures are logged
/// and the marker is retained so the NEXT startup tries again.
///
/// Call this once during `AppState` initialization, after migrations have
/// run and before any user-facing command handlers are registered.
pub async fn drain_pending(
    ctx: &crate::services::context::ServiceContext<'_>,
    state: &Arc<AppState>,
) {
    if ctx.check_mutation_allowed().is_err() {
        return;
    }
    let pending = match state
        .db_read(|db| db.list_health_recompute_pending().map_err(|e| e.to_string()))
        .await
    {
        Ok(v) => v,
        Err(e) => {
            log::warn!(
                "DOS-228: failed to list pending health recomputes on startup: {}",
                e
            );
            return;
        }
    };

    if pending.is_empty() {
        return;
    }

    log::info!(
        "DOS-228: draining {} pending health recompute(s) on startup",
        pending.len()
    );

    for account_id in pending {
        let id_for_write = account_id.clone();
        let recompute_result = state
            .db_write(move |db| {
                let clock = crate::services::context::SystemClock;
                let rng = crate::services::context::SystemRng;
                let ext = crate::services::context::ExternalClients::default();
                let ctx = crate::services::context::ServiceContext::new_live(&clock, &rng, &ext);
                crate::services::intelligence::recompute_entity_health(
                    &ctx,
                    db,
                    &id_for_write,
                    "account",
                )
            })
            .await;

        match recompute_result {
            Ok(()) => {
                let clear_id = account_id.clone();
                let _ = state
                    .db_write(move |db| {
                        db.clear_health_recompute_pending(&clear_id)
                            .map_err(|e| e.to_string())
                    })
                    .await;
                log::info!(
                    "DOS-228: startup-drained health recompute for {}",
                    account_id
                );
            }
            Err(e) => log::warn!(
                "DOS-228: startup drain failed for {}: {} (marker retained)",
                account_id,
                e
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_and_claim_happy_path() {
        let d = HealthRecomputeDebouncer::new();
        let ts = d.record("a");
        assert_eq!(d.pending_count(), 1);
        assert!(d.try_claim("a", ts), "claim should succeed");
        assert_eq!(d.pending_count(), 0, "claim clears the entry");
    }

    #[test]
    fn newer_edit_supersedes_older_claim() {
        let d = HealthRecomputeDebouncer::new();
        let first = d.record("a");
        // Force time to advance so the two Instants differ. Instant is
        // monotonic; sleeping 1ms is sufficient.
        std::thread::sleep(Duration::from_millis(1));
        let _second = d.record("a");
        assert!(
            !d.try_claim("a", first),
            "old claim must NOT win over newer edit"
        );
        assert_eq!(
            d.pending_count(),
            1,
            "failed claim must NOT clear the pending entry"
        );
    }

    #[test]
    fn rapid_burst_only_latest_claims() {
        // Simulate 10 rapid edits on the same account — only the last one
        // should successfully claim the recompute slot.
        let d = HealthRecomputeDebouncer::new();
        let mut stamps = Vec::new();
        for _ in 0..10 {
            stamps.push(d.record("a"));
            std::thread::sleep(Duration::from_millis(1));
        }
        let last = *stamps.last().expect("stamps not empty");
        for older in &stamps[..9] {
            assert!(
                !d.try_claim("a", *older),
                "older edits must not claim the recompute"
            );
        }
        assert!(d.try_claim("a", last), "last edit must claim");
        assert_eq!(d.pending_count(), 0);
    }

    #[test]
    fn different_accounts_are_independent() {
        let d = HealthRecomputeDebouncer::new();
        let a = d.record("a");
        let b = d.record("b");
        assert!(d.try_claim("a", a));
        assert!(d.try_claim("b", b));
    }

    /// the debouncer is timing-only. `record` must not touch
    /// the database — all persistence now lives in the calling mutation's
    /// writer closure. This test codifies that separation of concerns: a bare
    /// `HealthRecomputeDebouncer` is fully usable with no DB handle at all.
    #[test]
    fn record_is_timing_only_no_db_dependency() {
        let d = HealthRecomputeDebouncer::new();
        // No AppState, no ActionDb, no connection — just an in-memory map.
        let ts = d.record("acct-w0f");
        assert_eq!(d.pending_count(), 1);
        assert!(d.try_claim("acct-w0f", ts));
        assert_eq!(d.pending_count(), 0);
    }
}
