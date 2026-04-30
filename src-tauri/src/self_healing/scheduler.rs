//! Self-healing scheduler and circuit breaker (I410).
//!
//! Manages the lifecycle of coherence checks after enrichment,
//! with a circuit breaker to prevent infinite re-enrichment loops.
//!
//! Circuit breaker rules:
//! - On coherence failure: start a window, increment retry count
//! - If 3 retries within 24h: block entity (circuit breaker trips)
//! - If window > 72h: auto-expire block, reset and allow retry
//! - If window > 24h but < 72h: reset count, start new window

use crate::db::ActionDb;
use crate::embeddings::EmbeddingModel;
use crate::intel_queue::IntelligenceQueue;

/// Max retries within a 24h window before blocking.
const MAX_RETRIES_PER_WINDOW: i64 = 3;

/// Check if an entity is coherence-blocked (circuit breaker tripped).
pub fn check_circuit_breaker(db: &ActionDb, entity_id: &str) -> bool {
    db.conn_ref()
        .query_row(
            "SELECT coherence_blocked FROM entity_quality WHERE entity_id = ?1",
            rusqlite::params![entity_id],
            |row| row.get::<_, i64>(0),
        )
        .unwrap_or(0)
        != 0
}

/// Reset circuit breaker for an entity (manual refresh override).
pub fn reset_circuit_breaker(db: &ActionDb, entity_id: &str) {
    let _ = db.conn_ref().execute(
        "UPDATE entity_quality SET coherence_blocked = 0, coherence_retry_count = 0,
         coherence_window_start = NULL, updated_at = datetime('now')
         WHERE entity_id = ?1",
        rusqlite::params![entity_id],
    );
}

/// Evaluate whether a signal-triggered entity should be re-enriched.
/// Returns true if the entity was enqueued.
pub fn evaluate_on_signal(
    db: &ActionDb,
    entity_id: &str,
    entity_type: &str,
    queue: &IntelligenceQueue,
) -> Result<bool, String> {
    let score = super::remediation::compute_enrichment_trigger_score(db, entity_id, entity_type);

    if score > 0.7 && !check_circuit_breaker(db, entity_id) {
        let _ = queue.enqueue(crate::intel_queue::IntelRequest::new(            entity_id.to_string(),
            entity_type.to_string(),
            crate::intel_queue::IntelPriority::ContentChange,
        ));
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Called after enrichment completes. Runs coherence check and manages circuit breaker.
pub fn on_enrichment_complete(
    db: &ActionDb,
    embedding_model: Option<&EmbeddingModel>,
    entity_id: &str,
    entity_type: &str,
    queue: &IntelligenceQueue,
    signal_engine: Option<&crate::signals::propagation::PropagationEngine>,
) -> Result<(), String> {
    let result = super::detector::run_coherence_check(db, embedding_model, entity_id)?;

    if result.passed {
        // If previously flagged, clear coherence state
        let was_flagged: bool = db
            .conn_ref()
            .query_row(
                "SELECT coherence_retry_count > 0 OR coherence_blocked = 1
                 FROM entity_quality WHERE entity_id = ?1",
                rusqlite::params![entity_id],
                |row| row.get::<_, bool>(0),
            )
            .unwrap_or(false);

        if was_flagged {
            reset_circuit_breaker(db, entity_id);
        }
        return Ok(());
    }

    // Coherence failed — emit signal (I407 AC#6)
    if let Some(engine) = signal_engine {
        let clock = crate::services::context::SystemClock;
        let rng = crate::services::context::SystemRng;
        let ext = crate::services::context::ExternalClients::default();
        let ctx = crate::services::context::ServiceContext::new_live(&clock, &rng, &ext);
        let _ = crate::services::signals::emit_and_propagate(
            &ctx,
            db,
            engine,
            entity_type,
            entity_id,
            "entity_coherence_flagged",
            "self_healing",
            Some(&format!("{{\"coherence_score\":{:.3}}}", result.score)),
            0.3,
        );
    }

    // Coherence failed — manage circuit breaker
    manage_circuit_breaker(db, entity_id, entity_type, queue)
}

/// Circuit breaker state machine for coherence failures.
fn manage_circuit_breaker(
    db: &ActionDb,
    entity_id: &str,
    entity_type: &str,
    queue: &IntelligenceQueue,
) -> Result<(), String> {
    let (retry_count, window_start, blocked) = db
        .conn_ref()
        .query_row(
            "SELECT coherence_retry_count, coherence_window_start, coherence_blocked
             FROM entity_quality WHERE entity_id = ?1",
            rusqlite::params![entity_id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            },
        )
        .unwrap_or((0, None, 0));

    if blocked != 0 {
        // Already blocked — check for auto-expiry (72h)
        if let Some(ref ws) = window_start {
            let hours_since_window = window_hours_ago(db, ws);
            if hours_since_window > 72.0 {
                // Auto-expire: reset and re-enqueue
                reset_circuit_breaker(db, entity_id);
                start_new_window(db, entity_id);
                enqueue_retry(queue, entity_id, entity_type);
            }
        }
        return Ok(());
    }

    match window_start {
        None => {
            // First failure: start window, retry_count = 1
            start_new_window(db, entity_id);
            enqueue_retry(queue, entity_id, entity_type);
        }
        Some(ref ws) => {
            let hours = window_hours_ago(db, ws);

            if hours < 24.0 && retry_count + 1 >= MAX_RETRIES_PER_WINDOW {
                // 3 retries in <24h → trip circuit breaker
                let _ = db.conn_ref().execute(
                    "UPDATE entity_quality SET coherence_blocked = 1, updated_at = datetime('now')
                     WHERE entity_id = ?1",
                    rusqlite::params![entity_id],
                );
                log::warn!(
                    "SelfHealing: circuit breaker tripped for entity {}",
                    entity_id
                );
            } else if hours > 24.0 {
                // Window expired (>24h): reset count, start new window
                start_new_window(db, entity_id);
                enqueue_retry(queue, entity_id, entity_type);
            } else {
                // Within window, haven't hit limit: increment and retry
                let _ = db.conn_ref().execute(
                    "UPDATE entity_quality SET coherence_retry_count = coherence_retry_count + 1,
                     updated_at = datetime('now')
                     WHERE entity_id = ?1",
                    rusqlite::params![entity_id],
                );
                enqueue_retry(queue, entity_id, entity_type);
            }
        }
    }

    Ok(())
}

/// Start a new circuit breaker window.
fn start_new_window(db: &ActionDb, entity_id: &str) {
    let _ = db.conn_ref().execute(
        "UPDATE entity_quality SET coherence_window_start = datetime('now'),
         coherence_retry_count = 1, updated_at = datetime('now')
         WHERE entity_id = ?1",
        rusqlite::params![entity_id],
    );
}

/// Calculate hours since a window started.
fn window_hours_ago(db: &ActionDb, window_start: &str) -> f64 {
    db.conn_ref()
        .query_row(
            "SELECT (julianday('now') - julianday(?1)) * 24.0",
            rusqlite::params![window_start],
            |row| row.get::<_, f64>(0),
        )
        .unwrap_or(0.0)
}

/// Enqueue an entity for re-enrichment at ProactiveHygiene priority.
fn enqueue_retry(queue: &IntelligenceQueue, entity_id: &str, entity_type: &str) {
    let _ = queue.enqueue(crate::intel_queue::IntelRequest::new(        entity_id.to_string(),
        entity_type.to_string(),
        crate::intel_queue::IntelPriority::ProactiveHygiene,
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::ActionDb;
    use crate::self_healing::quality;

    fn test_db() -> ActionDb {
        crate::db::test_utils::test_db()
    }

    #[test]
    fn test_circuit_breaker_default_not_blocked() {
        let db = test_db();
        quality::ensure_quality_row(&db, "acme", "account");
        assert!(!check_circuit_breaker(&db, "acme"));
    }

    #[test]
    fn test_circuit_breaker_blocked_after_set() {
        let db = test_db();
        quality::ensure_quality_row(&db, "acme", "account");

        db.conn_ref()
            .execute(
                "UPDATE entity_quality SET coherence_blocked = 1 WHERE entity_id = 'acme'",
                [],
            )
            .unwrap();

        assert!(check_circuit_breaker(&db, "acme"));
    }

    #[test]
    fn test_reset_circuit_breaker() {
        let db = test_db();
        quality::ensure_quality_row(&db, "acme", "account");

        db.conn_ref()
            .execute(
                "UPDATE entity_quality SET coherence_blocked = 1, coherence_retry_count = 3
                 WHERE entity_id = 'acme'",
                [],
            )
            .unwrap();

        reset_circuit_breaker(&db, "acme");

        let q = quality::get_quality(&db, "acme").unwrap();
        assert!(!q.coherence_blocked);
        assert_eq!(q.coherence_retry_count, 0);
        assert!(q.coherence_window_start.is_none());
    }

    #[test]
    fn test_circuit_breaker_trips_after_max_retries() {
        let db = test_db();
        quality::ensure_quality_row(&db, "acme", "account");
        let queue = IntelligenceQueue::new();

        // Simulate window started now with 2 retries already
        db.conn_ref()
            .execute(
                "UPDATE entity_quality SET coherence_window_start = datetime('now'),
                 coherence_retry_count = 2
                 WHERE entity_id = 'acme'",
                [],
            )
            .unwrap();

        // This should trip the breaker (retry_count + 1 = 3 >= MAX_RETRIES_PER_WINDOW)
        manage_circuit_breaker(&db, "acme", "account", &queue).unwrap();

        assert!(check_circuit_breaker(&db, "acme"));
    }

    #[test]
    fn test_first_failure_starts_window() {
        let db = test_db();
        quality::ensure_quality_row(&db, "acme", "account");
        let queue = IntelligenceQueue::new();

        manage_circuit_breaker(&db, "acme", "account", &queue).unwrap();

        let q = quality::get_quality(&db, "acme").unwrap();
        assert!(q.coherence_window_start.is_some());
        assert_eq!(q.coherence_retry_count, 1);
        assert!(!q.coherence_blocked);
        // Should have enqueued a retry
        assert_eq!(queue.len(), 1);
    }

    #[test]
    fn test_evaluate_on_signal_skips_blocked() {
        let db = test_db();
        quality::ensure_quality_row(&db, "acme", "account");
        let queue = IntelligenceQueue::new();

        db.conn_ref()
            .execute(
                "UPDATE entity_quality SET coherence_blocked = 1 WHERE entity_id = 'acme'",
                [],
            )
            .unwrap();

        let enqueued = evaluate_on_signal(&db, "acme", "account", &queue).unwrap();
        assert!(!enqueued);
    }

    #[test]
    fn test_nonexistent_entity_not_blocked() {
        let db = test_db();
        // No quality row exists — should not be considered blocked
        assert!(!check_circuit_breaker(&db, "nonexistent"));
    }
}
