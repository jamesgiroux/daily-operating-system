//! Intelligence quality feedback service (I529).
//!
//! Records user feedback on intelligence fields and adjusts source weights
//! via the Bayesian signal weight system.
//!
//! Source attribution strategy (coarse, per I529 spec):
//! 1. People: read `enrichment_sources[field]["source"]` for precise field-level attribution
//! 2. All entities: fall back to most recent enrichment signal source for the entity
//! 3. If no source identifiable: record feedback with `source = null` (signal still emitted)

use crate::db::ActionDb;

/// Submit feedback on an intelligence field for an entity.
///
/// Records the feedback, adjusts source weights (Bayesian alpha/beta),
/// and emits a signal for downstream propagation.
pub fn submit_intelligence_feedback(
    db: &ActionDb,
    entity_id: &str,
    entity_type: &str,
    field: &str,
    feedback_type: &str,
    context: Option<&str>,
) -> Result<(), String> {
    let id = uuid::Uuid::new_v4().to_string();

    // Insert or replace feedback record (UNIQUE on entity_id+entity_type+field per AC16)
    db.insert_intelligence_feedback(
        &id,
        entity_id,
        entity_type,
        field,
        feedback_type,
        None,
        context,
    )?;

    // Resolve the source that produced this intelligence.
    let prior_source = resolve_intelligence_source(db, entity_id, entity_type, field);

    // Adjust Bayesian source weights based on feedback direction.
    // negative → beta++ (penalize source), positive → alpha++ (reward source)
    if let Some(ref source) = prior_source {
        let field_category = field_to_signal_category(field);
        match feedback_type {
            "negative" => {
                let _ = db.upsert_signal_weight(source, entity_type, &field_category, 0.0, 1.0);
            }
            "positive" => {
                let _ = db.upsert_signal_weight(source, entity_type, &field_category, 1.0, 0.0);
            }
            _ => {}
        }
    }

    // Emit signal for intelligence feedback
    let signal_type = if feedback_type == "positive" {
        "intelligence_confirmed"
    } else {
        "intelligence_rejected"
    };
    let value_json = serde_json::json!({
        "field": field,
        "feedback_type": feedback_type,
        "source": prior_source,
    })
    .to_string();
    let _ = crate::services::signals::emit(
        db,
        entity_type,
        entity_id,
        signal_type,
        "user_feedback",
        Some(&value_json),
        0.8,
    );

    Ok(())
}

/// Resolve the source that produced an intelligence field.
///
/// Strategy:
/// 1. For people: check `enrichment_sources[field]["source"]` (precise attribution)
/// 2. For all entities: check the most recent enrichment-class signal for the entity
///    (covers Glean-sourced intelligence for accounts/projects)
/// 3. Returns None if no source identifiable
fn resolve_intelligence_source(
    db: &ActionDb,
    entity_id: &str,
    entity_type: &str,
    field: &str,
) -> Option<String> {
    // 1. Precise: person profile field attribution
    if entity_type == "person" {
        let precise = db
            .get_person(entity_id)
            .ok()
            .flatten()
            .and_then(|person| person.enrichment_sources)
            .and_then(|sources_json| serde_json::from_str::<serde_json::Value>(&sources_json).ok())
            .and_then(|sources| {
                sources[field]
                    .get("source")
                    .and_then(|s| s.as_str())
                    .map(|s| s.to_string())
            });
        if precise.is_some() {
            return precise;
        }
    }

    // 2. Coarse: most recent enrichment-class signal for this entity.
    // Covers Glean-sourced items on accounts/projects and chapter-level fields on people.
    // Enrichment signals have source in: glean, intel_queue, clay, gravatar, ai
    let coarse = db.conn_ref()
        .query_row(
            "SELECT source FROM signal_events \
             WHERE entity_id = ?1 AND entity_type = ?2 \
             AND signal_type IN ('entity_enriched', 'intelligence_refreshed', 'glean_document', 'enrichment_complete') \
             AND superseded_by IS NULL \
             ORDER BY created_at DESC LIMIT 1",
            rusqlite::params![entity_id, entity_type],
            |row| row.get::<_, String>(0),
        )
        .ok();

    coarse
}

/// Map an intelligence field name to a signal weight category.
fn field_to_signal_category(field: &str) -> String {
    match field {
        // Person profile fields
        "role" | "organization" | "linkedin_url" | "name" | "title" => {
            "profile_enrichment".to_string()
        }
        // Health scoring
        "health_score" | "health_assessment" | "risk_level" => "health_scoring".to_string(),
        // Relationship intelligence
        "relationship_strength" | "sentiment" | "engagement" => "relationship_intel".to_string(),
        // Chapter-level intelligence sections
        "state_of_play" | "watch_list" | "risks" | "plan" => "intelligence_assessment".to_string(),
        // Fallback
        _ => format!("intelligence_{field}"),
    }
}
