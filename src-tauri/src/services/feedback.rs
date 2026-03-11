//! Intelligence quality feedback service (I529).
//!
//! Records user feedback on intelligence fields and adjusts source weights
//! via the Bayesian signal weight system.

use crate::db::ActionDb;

/// Submit feedback on an intelligence field for an entity.
///
/// Records the feedback, adjusts source weights (Bayesian alpha/beta),
/// and emits a signal for downstream propagation.
#[allow(clippy::too_many_arguments)]
pub fn submit_intelligence_feedback(
    db: &ActionDb,
    entity_id: &str,
    entity_type: &str,
    field: &str,
    feedback_type: &str,
    context: Option<&str>,
) -> Result<(), String> {
    let id = uuid::Uuid::new_v4().to_string();

    // Insert feedback record
    db.insert_intelligence_feedback(&id, entity_id, entity_type, field, feedback_type, None, context)?;

    // Try to identify the source that produced this field from enrichment_sources.
    // Currently only people have enrichment_sources; accounts do not.
    let prior_source = if entity_type == "person" {
        db.get_person(entity_id)
            .ok()
            .flatten()
            .and_then(|person| person.enrichment_sources)
            .and_then(|sources_json| serde_json::from_str::<serde_json::Value>(&sources_json).ok())
            .and_then(|sources| {
                sources[field]
                    .get("source")
                    .and_then(|s| s.as_str())
                    .map(|s| s.to_string())
            })
    } else {
        None
    };

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

/// Map an intelligence field name to a signal weight category.
fn field_to_signal_category(field: &str) -> String {
    match field {
        "role" | "organization" | "linkedin_url" | "name" | "title" => {
            "profile_enrichment".to_string()
        }
        "health_score" | "health_assessment" | "risk_level" => "health_scoring".to_string(),
        "relationship_strength" | "sentiment" | "engagement" => "relationship_intel".to_string(),
        _ => format!("intelligence_{field}"),
    }
}
