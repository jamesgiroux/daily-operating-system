//! Lightweight in-memory latency rollups for hot command diagnostics (I197).
//!
//! This keeps a bounded sample window per command so we can surface p95
//! diagnostics without introducing persistent storage or production UI coupling.

use std::collections::{HashMap, VecDeque};
use std::sync::{Mutex, OnceLock};

use chrono::{DateTime, Utc};

const MAX_SAMPLES_PER_COMMAND: usize = 256;

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
        let mut windows = match self.windows.lock() {
            Ok(guard) => guard,
            Err(_) => return,
        };

        let window = windows.entry(command.to_string()).or_default();
        window.budget_ms = budget_ms;
        if elapsed_ms > budget_ms {
            window.budget_violations += 1;
        }
        if window.samples_ms.len() >= MAX_SAMPLES_PER_COMMAND {
            window.samples_ms.pop_front();
        }
        window.samples_ms.push_back(elapsed_ms);
        window.last_recorded_at = Some(Utc::now());
    }

    fn increment_degraded(&self, command: &str) {
        let mut windows = match self.windows.lock() {
            Ok(guard) => guard,
            Err(_) => return,
        };
        let window = windows.entry(command.to_string()).or_default();
        window.degraded_count += 1;
        if window.last_recorded_at.is_none() {
            window.last_recorded_at = Some(Utc::now());
        }
    }

    fn snapshot(&self) -> LatencyRollupsPayload {
        let windows = match self.windows.lock() {
            Ok(guard) => guard,
            Err(_) => {
                return LatencyRollupsPayload {
                    generated_at: Utc::now().to_rfc3339(),
                    commands: Vec::new(),
                }
            }
        };

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
        for ms in 1..=300 {
            recorder.record_sample("test_cmd", ms, 100);
        }
        let snapshot = recorder.snapshot();
        let rollup = snapshot
            .commands
            .iter()
            .find(|c| c.command == "test_cmd")
            .expect("rollup");
        assert_eq!(rollup.sample_count, MAX_SAMPLES_PER_COMMAND);
        assert_eq!(rollup.max_ms, 300);
        assert!(rollup.p50_ms >= 170);
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
