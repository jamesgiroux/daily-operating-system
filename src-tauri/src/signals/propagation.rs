//! Signal propagation engine (I308 — ADR-0080 Phase 4).
//!
//! When a signal is emitted, the propagation engine evaluates registered rules
//! to derive new signals on related entities. For example, a `title_change` on
//! a person propagates `stakeholder_change` to all linked accounts.

use rusqlite::params;
use uuid::Uuid;

use crate::db::{ActionDb, DbError};

use super::bus::SignalEvent;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A signal derived from a source signal via a propagation rule.
#[derive(Debug, Clone)]
pub struct DerivedSignal {
    pub entity_type: String,
    pub entity_id: String,
    pub signal_type: String,
    pub source: String,
    pub value: Option<String>,
    pub confidence: f64,
    pub rule_name: String,
}

/// A named propagation rule: given a source signal and DB access, returns
/// zero or more derived signals on related entities.
pub type PropagationRule = fn(&SignalEvent, &ActionDb) -> Vec<DerivedSignal>;

/// Registry of propagation rules evaluated after each signal emission.
pub struct PropagationEngine {
    rules: Vec<(String, PropagationRule)>,
}

impl Default for PropagationEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl PropagationEngine {
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    /// Register a named propagation rule.
    pub fn register(&mut self, name: &str, rule: PropagationRule) {
        self.rules.push((name.to_string(), rule));
    }

    /// Evaluate all rules against a source signal, emit derived signals,
    /// and record derivation links.
    pub fn propagate(
        &self,
        db: &ActionDb,
        source_signal: &SignalEvent,
    ) -> Result<Vec<String>, DbError> {
        let mut derived_ids = Vec::new();

        for (rule_name, rule_fn) in &self.rules {
            let derived_signals = rule_fn(source_signal, db);

            for ds in derived_signals {
                let id = format!("sig-{}", Uuid::new_v4());
                let half_life = super::bus::default_half_life(&ds.source);

                db.insert_signal_event(
                    &id,
                    &ds.entity_type,
                    &ds.entity_id,
                    &ds.signal_type,
                    &ds.source,
                    ds.value.as_deref(),
                    ds.confidence,
                    half_life,
                )?;

                db.insert_signal_derivation(
                    &source_signal.id,
                    &id,
                    rule_name,
                )?;

                derived_ids.push(id);
            }
        }

        Ok(derived_ids)
    }
}

/// Construct a propagation engine with all default rules.
pub fn default_engine() -> PropagationEngine {
    let mut engine = PropagationEngine::new();

    engine.register("rule_person_job_change", super::rules::rule_person_job_change);
    engine.register("rule_meeting_frequency_drop", super::rules::rule_meeting_frequency_drop);
    engine.register("rule_overdue_actions", super::rules::rule_overdue_actions);
    engine.register("rule_champion_sentiment", super::rules::rule_champion_sentiment);
    engine.register("rule_departure_renewal", super::rules::rule_departure_renewal);

    engine
}

// ---------------------------------------------------------------------------
// ActionDb methods
// ---------------------------------------------------------------------------

impl ActionDb {
    /// Insert a signal derivation record linking source → derived signal.
    pub fn insert_signal_derivation(
        &self,
        source_signal_id: &str,
        derived_signal_id: &str,
        rule_name: &str,
    ) -> Result<(), DbError> {
        let id = format!("sd-{}", Uuid::new_v4());
        self.conn_ref().execute(
            "INSERT INTO signal_derivations (id, source_signal_id, derived_signal_id, rule_name)
             VALUES (?1, ?2, ?3, ?4)",
            params![id, source_signal_id, derived_signal_id, rule_name],
        )?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn test_db() -> ActionDb {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("test.db");
        std::mem::forget(dir);
        ActionDb::open_at(path).expect("open")
    }

    #[test]
    fn test_propagation_engine_empty() {
        let db = test_db();
        let engine = PropagationEngine::new();

        let signal = SignalEvent {
            id: "sig-test".to_string(),
            entity_type: "person".to_string(),
            entity_id: "p1".to_string(),
            signal_type: "title_change".to_string(),
            source: "clay".to_string(),
            value: None,
            confidence: 0.85,
            decay_half_life_days: 90,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            superseded_by: None,
            source_context: None,
        };

        let derived = engine.propagate(&db, &signal).expect("propagate");
        assert!(derived.is_empty(), "no rules = no derived signals");
    }

    #[test]
    fn test_propagation_engine_with_rule() {
        let db = test_db();
        let mut engine = PropagationEngine::new();

        // Simple test rule: always emit one derived signal
        fn test_rule(_signal: &SignalEvent, _db: &ActionDb) -> Vec<DerivedSignal> {
            vec![DerivedSignal {
                entity_type: "account".to_string(),
                entity_id: "a1".to_string(),
                signal_type: "test_derived".to_string(),
                source: "propagation".to_string(),
                value: Some("test".to_string()),
                confidence: 0.75,
                rule_name: "test_rule".to_string(),
            }]
        }

        engine.register("test_rule", test_rule);

        // Insert source signal first
        let _ = super::super::bus::emit_signal(
            &db, "person", "p1", "title_change", "clay", None, 0.85,
        )
        .expect("emit");

        let signal = SignalEvent {
            id: "sig-source".to_string(),
            entity_type: "person".to_string(),
            entity_id: "p1".to_string(),
            signal_type: "title_change".to_string(),
            source: "clay".to_string(),
            value: None,
            confidence: 0.85,
            decay_half_life_days: 90,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            superseded_by: None,
            source_context: None,
        };

        let derived = engine.propagate(&db, &signal).expect("propagate");
        assert_eq!(derived.len(), 1);
        assert!(derived[0].starts_with("sig-"));

        // Verify derived signal exists in DB
        let signals = super::super::bus::get_active_signals(&db, "account", "a1").expect("get");
        assert_eq!(signals.len(), 1);
        assert_eq!(signals[0].signal_type, "test_derived");
    }

    #[test]
    fn test_insert_signal_derivation() {
        let db = test_db();
        db.insert_signal_derivation("sig-source", "sig-derived", "test_rule")
            .expect("insert derivation");

        let count: i32 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM signal_derivations WHERE source_signal_id = 'sig-source'",
                [],
                |row| row.get(0),
            )
            .expect("count");
        assert_eq!(count, 1);
    }
}
