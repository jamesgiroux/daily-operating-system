//! Observability module per ADR-0120.
//!
//! Provides InvocationRecord and an Evaluate-mode in-memory subscriber that
//! ability tests use to capture span fields. Production NDJSON subscriber
//! stub is included; full wiring lives in part 3.

use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvocationRecord {
    pub invocation_id: Uuid,
    pub ability_name: String,
    pub ability_category: String,
    pub actor: String,
    pub mode: String,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub ended_at: chrono::DateTime<chrono::Utc>,
    pub outcome: Outcome,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Outcome {
    Ok,
    Err { kind: String },
}

/// In-memory subscriber for Evaluate mode + tests.
pub struct EvaluateModeSubscriber {
    records: Mutex<Vec<InvocationRecord>>,
}

impl EvaluateModeSubscriber {
    pub fn new() -> Self {
        Self {
            records: Mutex::new(Vec::new()),
        }
    }

    pub fn record(&self, rec: InvocationRecord) {
        self.records.lock().unwrap().push(rec);
    }

    pub fn drain(&self) -> Vec<InvocationRecord> {
        std::mem::take(&mut *self.records.lock().unwrap())
    }

    pub fn snapshot(&self) -> Vec<InvocationRecord> {
        self.records.lock().unwrap().clone()
    }
}

impl Default for EvaluateModeSubscriber {
    fn default() -> Self {
        Self::new()
    }
}

/// NDJSON subscriber stub — wired in part 3.
pub struct NdjsonSubscriber;

impl NdjsonSubscriber {
    pub fn new() -> Self {
        Self
    }

    pub fn write(&self, _rec: &InvocationRecord) -> std::io::Result<()> {
        Ok(())
    }
}

impl Default for NdjsonSubscriber {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, TimeZone, Utc};

    fn record(outcome: Outcome) -> InvocationRecord {
        let started_at = Utc.with_ymd_and_hms(2026, 5, 1, 12, 0, 0).unwrap();
        let ended_at = started_at + Duration::milliseconds(42);
        InvocationRecord {
            invocation_id: Uuid::new_v4(),
            ability_name: "prepare_meeting".to_string(),
            ability_category: "Transform".to_string(),
            actor: "User".to_string(),
            mode: "evaluate".to_string(),
            started_at,
            ended_at,
            outcome,
            duration_ms: 42,
        }
    }

    #[test]
    fn invocation_record_captures_required_fields() {
        let rec = record(Outcome::Ok);

        assert_ne!(rec.invocation_id, Uuid::nil());
        assert_eq!(rec.ability_name, "prepare_meeting");
        assert_eq!(rec.ability_category, "Transform");
        assert_eq!(rec.actor, "User");
        assert_eq!(rec.mode, "evaluate");
        assert_eq!(
            rec.started_at,
            Utc.with_ymd_and_hms(2026, 5, 1, 12, 0, 0).unwrap()
        );
        assert_eq!(
            rec.ended_at,
            Utc.with_ymd_and_hms(2026, 5, 1, 12, 0, 0)
                .unwrap()
                + Duration::milliseconds(42)
        );
        assert_eq!(rec.outcome, Outcome::Ok);
        assert_eq!(rec.duration_ms, 42);
    }

    #[test]
    fn evaluate_mode_subscriber_records_to_in_memory_vec() {
        let subscriber = EvaluateModeSubscriber::new();
        subscriber.record(record(Outcome::Ok));
        subscriber.record(record(Outcome::Err {
            kind: "Validation".to_string(),
        }));

        assert_eq!(subscriber.snapshot().len(), 2);
        assert_eq!(subscriber.drain().len(), 2);
        assert!(subscriber.snapshot().is_empty());
    }

    #[test]
    fn outcome_kind_tag_is_present_for_err() {
        let value = serde_json::to_value(record(Outcome::Err {
            kind: "Capability".to_string(),
        }))
        .unwrap();

        assert_eq!(value["outcome"]["Err"]["kind"], "Capability");
    }
}
