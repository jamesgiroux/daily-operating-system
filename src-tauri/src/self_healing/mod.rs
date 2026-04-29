//! Intelligence self-healing (I406–I410).
//!
//! Detects quality degradation in entity intelligence, triggers re-enrichment
//! based on a continuous priority function, and wires user corrections back
//! to source reliability via the signal bus.

pub mod detector;
pub mod feedback;
pub mod quality;
pub mod remediation;
pub mod scheduler;

use crate::db::ActionDb;
use crate::intel_queue::IntelligenceQueue;
use crate::state::HygieneBudget;

/// Evaluate the full entity portfolio for self-healing enrichment.
///
/// Called from hygiene Phase 3 instead of the old `enqueue_ai_enrichments`.
/// Returns the number of enrichments enqueued.
pub fn evaluate_portfolio(
    db: &ActionDb,
    budget: &HygieneBudget,
    queue: &IntelligenceQueue,
    _embedding_model: Option<&crate::embeddings::EmbeddingModel>,
) -> usize {
    // Ensure every known entity has a quality row (idempotent)
    quality::initialize_quality_scores(db);

    // Get prioritized enrichment candidates
    let candidates = remediation::prioritize_enrichment_queue(db);

    let mut enqueued = 0;
    for (entity_id, entity_type, score) in &candidates {
        log::debug!(
            "SelfHealing: {} ({}) trigger_score={:.3}",
            entity_id,
            entity_type,
            score,
        );

        if !budget.try_consume() {
            log::debug!(
                "SelfHealing: budget exhausted ({} used)",
                budget.used_today()
            );
            break;
        }

        // Skip coherence-blocked entities
        if scheduler::check_circuit_breaker(db, entity_id) {
            log::debug!("SelfHealing: {} coherence-blocked, skipping", entity_id);
            continue;
        }

        let _ = queue.enqueue(crate::intel_queue::IntelRequest::new(            entity_id.clone(),
            entity_type.clone(),
            crate::intel_queue::IntelPriority::ProactiveHygiene,
        ));
        enqueued += 1;
    }

    enqueued
}
