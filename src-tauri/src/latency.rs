//! Lightweight in-memory latency rollups for hot command diagnostics.
//!
//! This keeps a bounded sample window per command so we can surface p95
//! diagnostics without introducing persistent storage or production UI coupling.
//!
//! W0-B extends the substrate to support the throughput measurement protocol:
//! - Sample window bumped from 256 → 4096 so 10–15 min scripted-load runs
//!   (and the AC6 30/40-entity growth-slope re-measure) don't evict tail
//!   samples that drive the p95/p99 gate decision.
//! - Lossless cumulative counters (`total_samples`, `cumulative_sum_ms`)
//!   accumulate independently of the sample window so long-window means and
//!   total request counts survive eviction.
//! - `snapshot_for_persistence` / `apply_persistent_snapshot` serialize the
//!   counters through `app_state_kv` via the writer queue (see
//!   `persist_rollups_to_kv` in `db_service.rs`), so a restart mid-measurement
//!   doesn't lose the long-window accumulation. Samples (for percentiles) are
//!   not persisted; only counters carry forward.

use parking_lot::Mutex;
use std::collections::{HashMap, VecDeque};
use std::sync::OnceLock;

use chrono::{DateTime, Utc};

const MAX_SAMPLES_PER_COMMAND: usize = 4096;

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LatencyCommandRollup {
    pub command: String,
    pub sample_count: usize,
    pub p50_ms: u128,
    pub p95_ms: u128,
    pub max_ms: u128,
    pub budget_ms: u128,
    pub budget_violations: u64,
    pub degraded_count: u64,
    pub last_recorded_at: Option<String>,
    /// Lossless lifetime count across the durable window. Independent of
    /// `sample_count` (which is bounded to the in-memory ring). Used by the
    /// W0 measurement protocol so total invocation counts survive sample
    /// eviction and restart.
    pub total_samples: u64,
    /// Sum of all sample values across the durable window. With
    /// `total_samples`, gives an exact long-window mean even when individual
    /// samples have been evicted from the in-memory ring.
    pub cumulative_sum_ms: u128,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LatencyRollupsPayload {
    pub generated_at: String,
    pub commands: Vec<LatencyCommandRollup>,
}

#[derive(Debug, Clone, Default)]
struct CommandLatencyWindow {
    samples_ms: VecDeque<u128>,
    budget_ms: u128,
    budget_violations: u64,
    degraded_count: u64,
    last_recorded_at: Option<DateTime<Utc>>,
    /// Total samples ever recorded for this command. Outlives the bounded
    /// `samples_ms` ring.
    total_samples: u64,
    /// Sum of every sample ever recorded for this command. With
    /// `total_samples` gives a long-window mean unaffected by sample eviction.
    cumulative_sum_ms: u128,
}

/// Persistable snapshot of the lossless counters for restart-durability.
/// Samples are not persisted (they're sample window-local), only the
/// accumulated counters so long-window means survive a process restart.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct LatencyPersistentSnapshot {
    pub generated_at: String,
    pub commands: Vec<LatencyPersistentEntry>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LatencyPersistentEntry {
    pub command: String,
    pub total_samples: u64,
    pub cumulative_sum_ms: u128,
    pub budget_violations: u64,
    pub degraded_count: u64,
    pub budget_ms: u128,
    pub last_recorded_at: Option<String>,
}

#[derive(Default)]
pub struct LatencyRecorder {
    windows: Mutex<HashMap<String, CommandLatencyWindow>>,
}

impl LatencyRecorder {
    fn global() -> &'static Self {
        static RECORDER: OnceLock<LatencyRecorder> = OnceLock::new();
        RECORDER.get_or_init(Self::default)
    }

    fn record_sample(&self, command: &str, elapsed_ms: u128, budget_ms: u128) {
        let mut windows = self.windows.lock();

        let window = windows.entry(command.to_string()).or_default();
        window.budget_ms = budget_ms;
        if elapsed_ms > budget_ms {
            window.budget_violations += 1;
        }
        if window.samples_ms.len() >= MAX_SAMPLES_PER_COMMAND {
            window.samples_ms.pop_front();
        }
        window.samples_ms.push_back(elapsed_ms);
        window.total_samples = window.total_samples.saturating_add(1);
        window.cumulative_sum_ms = window.cumulative_sum_ms.saturating_add(elapsed_ms);
        window.last_recorded_at = Some(Utc::now());
    }

    fn increment_degraded(&self, command: &str) {
        let mut windows = self.windows.lock();
        let window = windows.entry(command.to_string()).or_default();
        window.degraded_count += 1;
        if window.last_recorded_at.is_none() {
            window.last_recorded_at = Some(Utc::now());
        }
    }

    fn snapshot(&self) -> LatencyRollupsPayload {
        let windows = self.windows.lock();

        let mut commands: Vec<LatencyCommandRollup> = windows
            .iter()
            .map(|(command, window)| {
                let mut values: Vec<u128> = window.samples_ms.iter().copied().collect();
                values.sort_unstable();
                let sample_count = values.len();
                let p50 = percentile(&values, 50.0).unwrap_or(0);
                let p95 = percentile(&values, 95.0).unwrap_or(0);
                let max_ms = values.last().copied().unwrap_or(0);

                LatencyCommandRollup {
                    command: command.clone(),
                    sample_count,
                    p50_ms: p50,
                    p95_ms: p95,
                    max_ms,
                    budget_ms: window.budget_ms,
                    budget_violations: window.budget_violations,
                    degraded_count: window.degraded_count,
                    last_recorded_at: window.last_recorded_at.map(|dt| dt.to_rfc3339()),
                    total_samples: window.total_samples,
                    cumulative_sum_ms: window.cumulative_sum_ms,
                }
            })
            .collect();

        commands.sort_by(|a, b| b.p95_ms.cmp(&a.p95_ms).then(a.command.cmp(&b.command)));

        LatencyRollupsPayload {
            generated_at: Utc::now().to_rfc3339(),
            commands,
        }
    }
}

fn percentile(values: &[u128], p: f64) -> Option<u128> {
    if values.is_empty() {
        return None;
    }
    let n = values.len();
    let rank = ((p / 100.0) * n as f64).ceil() as usize;
    let idx = rank.saturating_sub(1).min(n - 1);
    Some(values[idx])
}

pub fn record_latency(command: &str, elapsed_ms: u128, budget_ms: u128) {
    LatencyRecorder::global().record_sample(command, elapsed_ms, budget_ms);
}

pub fn increment_degraded(command: &str) {
    LatencyRecorder::global().increment_degraded(command);
}

pub fn get_rollups() -> LatencyRollupsPayload {
    LatencyRecorder::global().snapshot()
}

/// Snapshot the lossless counters for persistence. Samples (used for
/// percentile estimation) are NOT included — only the long-window counters
/// that need to survive sample eviction and process restart for the W0
/// measurement protocol.
pub fn snapshot_for_persistence() -> LatencyPersistentSnapshot {
    let windows = LatencyRecorder::global().windows.lock();
    let mut commands: Vec<LatencyPersistentEntry> = windows
        .iter()
        .map(|(command, window)| LatencyPersistentEntry {
            command: command.clone(),
            total_samples: window.total_samples,
            cumulative_sum_ms: window.cumulative_sum_ms,
            budget_violations: window.budget_violations,
            degraded_count: window.degraded_count,
            budget_ms: window.budget_ms,
            last_recorded_at: window.last_recorded_at.map(|dt| dt.to_rfc3339()),
        })
        .collect();
    commands.sort_by(|a, b| a.command.cmp(&b.command));
    LatencyPersistentSnapshot {
        generated_at: Utc::now().to_rfc3339(),
        commands,
    }
}

/// Apply a previously-persisted snapshot, additively. Counters merge by the
/// stored totals so a restart mid-measurement preserves long-window
/// accumulation. Samples are not restored (they're window-local).
///
/// At-most-once per process via the static `HYDRATED` guard: dev-mode
/// `reinit_db_service` reopens the pool (dev_apply_scenario, dev_restore_live,
/// dev_onboarding_scenario), which would re-fire `hydrate_latency_snapshot_from_kv`
/// and double-count the persisted counters into the live in-memory state. The
/// guard makes additional calls no-ops with a debug log; production lifecycle
/// is unaffected.
pub fn apply_persistent_snapshot(snapshot: LatencyPersistentSnapshot) {
    static HYDRATED: std::sync::OnceLock<()> = std::sync::OnceLock::new();
    if HYDRATED.set(()).is_err() {
        log::debug!(
            "latency hydrate skipped: already applied in this process (snapshot dropped)"
        );
        return;
    }
    let mut windows = LatencyRecorder::global().windows.lock();
    for entry in snapshot.commands {
        let window = windows.entry(entry.command).or_default();
        window.total_samples = window.total_samples.saturating_add(entry.total_samples);
        window.cumulative_sum_ms = window
            .cumulative_sum_ms
            .saturating_add(entry.cumulative_sum_ms);
        window.budget_violations = window
            .budget_violations
            .saturating_add(entry.budget_violations);
        window.degraded_count = window.degraded_count.saturating_add(entry.degraded_count);
        if window.budget_ms == 0 {
            window.budget_ms = entry.budget_ms;
        }
        if window.last_recorded_at.is_none() {
            window.last_recorded_at = entry
                .last_recorded_at
                .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                .map(|dt| dt.with_timezone(&Utc));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_percentile_empty() {
        assert_eq!(percentile(&[], 95.0), None);
    }

    #[test]
    fn test_percentile_small_sample_sizes() {
        let values = vec![10_u128, 20, 30];
        assert_eq!(percentile(&values, 50.0), Some(20));
        assert_eq!(percentile(&values, 95.0), Some(30));
    }

    #[test]
    fn test_ring_buffer_eviction() {
        let recorder = LatencyRecorder::default();
        // Push enough samples to exceed the window cap and force eviction.
        // W0-B raised the cap to 4096; this loop overshoots by ~22% so the
        // tail of the window holds the highest values after eviction.
        let total = MAX_SAMPLES_PER_COMMAND + 1000;
        for ms in 1..=(total as u128) {
            recorder.record_sample("test_cmd", ms, 100);
        }
        let snapshot = recorder.snapshot();
        let rollup = snapshot
            .commands
            .iter()
            .find(|c| c.command == "test_cmd")
            .expect("rollup");
        assert_eq!(rollup.sample_count, MAX_SAMPLES_PER_COMMAND);
        assert_eq!(rollup.max_ms, total as u128);
        // After eviction the window holds the most recent MAX_SAMPLES samples
        // (values `total-MAX_SAMPLES+1 ..= total`), so p50 sits ~mid-window.
        let window_min = (total - MAX_SAMPLES_PER_COMMAND) as u128;
        assert!(rollup.p50_ms >= window_min + (MAX_SAMPLES_PER_COMMAND as u128) / 2);
        // Lossless counters track every sample, not just the window.
        assert_eq!(rollup.total_samples, total as u64);
        assert_eq!(
            rollup.cumulative_sum_ms,
            (1..=(total as u128)).sum::<u128>()
        );
    }

    #[test]
    fn test_budget_violations_increment_only_on_exceed() {
        let recorder = LatencyRecorder::default();
        recorder.record_sample("budget_cmd", 95, 100);
        recorder.record_sample("budget_cmd", 100, 100);
        recorder.record_sample("budget_cmd", 101, 100);
        recorder.record_sample("budget_cmd", 300, 100);

        let snapshot = recorder.snapshot();
        let rollup = snapshot
            .commands
            .iter()
            .find(|c| c.command == "budget_cmd")
            .expect("rollup");
        assert_eq!(rollup.budget_violations, 2);
    }
}
