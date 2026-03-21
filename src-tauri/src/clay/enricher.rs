//! Clay enrichment pipeline.
//!
//! Fetches contact data from Clay via Smithery MCP and writes it to the person
//! profile through the unified `update_person_profile()` path, which handles
//! source priority, provenance tracking, and audit logging.

use serde::{Deserialize, Serialize};

use crate::clay::client::ClayClient;
use crate::db::people::ProfileUpdate;
use crate::state::AppState;

// Re-export source priority types from db layer for backward compat
pub use crate::db::people::{can_write_field, source_priority, FieldSource};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Outcome of an enrichment run for a single person.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnrichmentResult {
    pub person_id: String,
    pub fields_updated: Vec<String>,
    pub signals: Vec<String>,
    pub clay_contact_id: Option<String>,
}

// ---------------------------------------------------------------------------
// Main enrichment flow
// ---------------------------------------------------------------------------

/// Enrich a person from Clay via Smithery MCP.
///
/// Three-phase lock pattern: read person (lock) → Clay API calls (no lock) →
/// write profile update (lock).
pub async fn enrich_person_from_clay_with_client(
    state: &AppState,
    person_id: &str,
    client: &ClayClient,
) -> Result<EnrichmentResult, String> {
    // Phase 1: Load person under lock, then release
    let (email, name, org, old_title_history, old_org, old_linkedin, old_twitter) = {
        let db = crate::db::ActionDb::open().map_err(|e| format!("DB open failed: {e}"))?;
        let person = db
            .get_person(person_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Person not found: {}", person_id))?;
        (
            person.email.clone(),
            person.name.clone(),
            person.organization.clone(),
            person.title_history.clone(),
            person.organization.clone(),
            person.linkedin_url.clone(),
            person.twitter_handle.clone(),
        )
    };

    // Step 2: Async Clay calls (no lock held)
    let mut search_results = client
        .search_contact(&email)
        .await
        .map_err(|e| e.to_string())?;
    if search_results.is_empty() {
        if let Some(ref org_name) = org {
            if !org_name.is_empty() {
                let query = format!("{} {}", name, org_name);
                search_results = client
                    .search_contact(&query)
                    .await
                    .map_err(|e| e.to_string())?;
            }
        }
    }
    if search_results.is_empty() {
        return Ok(EnrichmentResult {
            person_id: person_id.to_string(),
            fields_updated: vec![],
            signals: vec![],
            clay_contact_id: None,
        });
    }

    // Match: email in email/name field, then by actual name, then first result
    let best = search_results
        .iter()
        .find(|r| {
            r.email
                .as_deref()
                .map(|e| e.eq_ignore_ascii_case(&email))
                .unwrap_or(false)
        })
        .or_else(|| {
            search_results.iter().find(|r| {
                r.name
                    .as_deref()
                    .map(|n| n.eq_ignore_ascii_case(&email))
                    .unwrap_or(false)
            })
        })
        .or_else(|| {
            search_results.iter().find(|r| {
                r.name
                    .as_deref()
                    .map(|n| n.eq_ignore_ascii_case(&name))
                    .unwrap_or(false)
            })
        })
        .or_else(|| search_results.first())
        .ok_or("No contact selected")?;
    let clay_id = best.id_str();

    if clay_id.is_empty() {
        return Err(format!(
            "Clay search returned contact with empty id. person={}, best_name={:?}, results={}",
            person_id,
            best.name,
            search_results.len()
        ));
    }

    let detail = client.get_contact_detail(&clay_id).await.map_err(|e| {
        format!(
            "getContact failed for clay_id='{}' person={}: {}",
            clay_id, person_id, e
        )
    })?;

    // Build title history JSON
    let title_history_json = if detail.title_history.is_empty() {
        None
    } else {
        serde_json::to_string(&detail.title_history).ok()
    };

    // Detect change signals before writing
    let signals = super::signals::detect_changes(
        old_title_history.as_deref(),
        title_history_json.as_deref(),
        old_org.as_deref(),
        detail.company.as_deref(),
        old_linkedin.as_deref(),
        detail.linkedin_url.as_deref(),
        old_twitter.as_deref(),
        detail.twitter_handle.as_deref(),
    );

    // Phase 3: Write through unified profile update (under lock)
    let update = ProfileUpdate {
        linkedin_url: detail.linkedin_url.clone(),
        twitter_handle: detail.twitter_handle.clone(),
        phone: detail.phone.clone(),
        photo_url: detail.photo_url.clone(),
        bio: detail.bio.clone(),
        title_history: title_history_json,
        organization: detail.company.clone(),
        role: detail.title.clone(),
        company_industry: detail.company_industry.clone(),
        company_size: detail.company_size.clone(),
        company_hq: detail.company_hq.clone(),
    };

    let fields_updated = {
        let db = crate::db::ActionDb::open().map_err(|e| format!("DB open failed: {e}"))?;
        let result = db
            .update_person_profile(person_id, &update, "clay")
            .map_err(|e| e.to_string())?;
        result.fields_updated
    };

    // Emit change signals to the signal bus
    let signal_names: Vec<String> = signals.iter().map(|s| s.signal_type.clone()).collect();
    {
        let db = crate::db::ActionDb::open().map_err(|e| format!("DB open failed: {e}"))?;
        for signal in &signals {
            let value = serde_json::json!({
                "description": signal.description,
                "old_value": signal.old_value,
                "new_value": signal.new_value,
            })
            .to_string();
            let _ = crate::services::signals::emit_and_propagate(
                &db,
                &state.signals.engine,
                "person",
                person_id,
                &signal.signal_type,
                "clay",
                Some(&value),
                0.85,
            );
        }
    }

    Ok(EnrichmentResult {
        person_id: person_id.to_string(),
        fields_updated,
        signals: signal_names,
        clay_contact_id: Some(clay_id),
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_priority_values() {
        assert_eq!(source_priority("user"), 4);
        assert_eq!(source_priority("clay"), 3);
        assert_eq!(source_priority("gravatar"), 2);
        assert_eq!(source_priority("ai"), 1);
        assert_eq!(source_priority("unknown_source"), 0);
    }

    #[test]
    fn test_can_write_field_empty_sources() {
        // No existing sources — any valid source can write
        assert!(can_write_field(None, "bio", "clay"));
        assert!(can_write_field(Some("{}"), "bio", "ai"));
        assert!(can_write_field(Some("{}"), "bio", "user"));
    }

    #[test]
    fn test_can_write_field_lower_priority_blocked() {
        let sources = r#"{"bio":{"source":"clay","at":"2026-01-01T00:00:00Z"}}"#;
        // AI (1) cannot overwrite Clay (3)
        assert!(!can_write_field(Some(sources), "bio", "ai"));
        // Gravatar (2) cannot overwrite Clay (3)
        assert!(!can_write_field(Some(sources), "bio", "gravatar"));
    }

    #[test]
    fn test_can_write_field_equal_priority_allowed() {
        let sources = r#"{"bio":{"source":"clay","at":"2026-01-01T00:00:00Z"}}"#;
        // Same source can re-write (refresh)
        assert!(can_write_field(Some(sources), "bio", "clay"));
    }

    #[test]
    fn test_can_write_field_higher_priority_allowed() {
        let sources = r#"{"bio":{"source":"clay","at":"2026-01-01T00:00:00Z"}}"#;
        // User (4) can overwrite Clay (3)
        assert!(can_write_field(Some(sources), "bio", "user"));
    }

    #[test]
    fn test_can_write_field_different_field_unaffected() {
        let sources = r#"{"bio":{"source":"user","at":"2026-01-01T00:00:00Z"}}"#;
        // "role" has no entry, so even AI can write it
        assert!(can_write_field(Some(sources), "role", "ai"));
    }

    #[test]
    fn test_unknown_source_blocked() {
        assert!(!can_write_field(None, "bio", "random"));
        assert!(!can_write_field(Some("{}"), "bio", ""));
    }

    #[test]
    fn test_malformed_json_treated_as_empty() {
        // Bad JSON should not block writes
        assert!(can_write_field(Some("not json"), "bio", "clay"));
    }
}
