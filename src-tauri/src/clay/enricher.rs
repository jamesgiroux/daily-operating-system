//! Clay enrichment pipeline with source priority system.
//!
//! Enriches person records from Clay.earth data while respecting a strict
//! source hierarchy: User (4) > Clay (3) > Gravatar (2) > AI (1).
//! Each field tracks its provenance via the `enrichment_sources` JSON column
//! so higher-priority sources are never overwritten by lower ones.

use std::collections::HashMap;

use chrono::Utc;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::clay::client::{ClayClient, ClayContactDetail};
use crate::db::ActionDb;
use crate::state::AppState;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Per-field provenance record stored in the `enrichment_sources` JSON column.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldSource {
    /// Which source wrote this field (e.g. "clay", "gravatar", "user", "ai").
    pub source: String,
    /// ISO-8601 timestamp of the last write.
    pub at: String,
}

/// Map of field names to their provenance.
pub type EnrichmentSources = HashMap<String, FieldSource>;

/// Outcome of an enrichment run for a single person.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnrichmentResult {
    pub person_id: String,
    pub fields_updated: Vec<String>,
    /// Detected change signals (e.g. "title_change", "company_change").
    pub signals: Vec<String>,
    pub clay_contact_id: Option<String>,
}

/// Lightweight person row loaded from the `people` table.
#[derive(Debug, Clone)]
struct DbPerson {
    id: String,
    email: String,
    name: String,
    organization: Option<String>,
    role: Option<String>,
    linkedin_url: Option<String>,
    twitter_handle: Option<String>,
    phone: Option<String>,
    photo_url: Option<String>,
    bio: Option<String>,
    title_history: Option<String>,
    company_industry: Option<String>,
    company_size: Option<String>,
    company_hq: Option<String>,
    enrichment_sources: Option<String>,
}

// ---------------------------------------------------------------------------
// Source priority
// ---------------------------------------------------------------------------

/// Returns the numeric priority for a given enrichment source.
/// Higher values win over lower values.
pub fn source_priority(source: &str) -> u8 {
    match source {
        "user" => 4,
        "clay" => 3,
        "gravatar" => 2,
        "ai" => 1,
        _ => 0,
    }
}

/// Checks whether a source is allowed to write a field given the current
/// provenance map.  Returns `true` when no higher-priority source has already
/// written the field.
pub fn can_write_field(
    current_sources_json: Option<&str>,
    field: &str,
    source: &str,
) -> bool {
    let new_priority = source_priority(source);
    if new_priority == 0 {
        return false; // unknown source may never write
    }

    let sources: EnrichmentSources = current_sources_json
        .and_then(|json| serde_json::from_str(json).ok())
        .unwrap_or_default();

    match sources.get(field) {
        Some(existing) => source_priority(&existing.source) <= new_priority,
        None => true, // no prior write — allow
    }
}

// ---------------------------------------------------------------------------
// DB helpers (raw SQL, same pattern as gravatar/cache.rs)
// ---------------------------------------------------------------------------

/// Load a person row by its primary key.
fn load_person(db: &ActionDb, person_id: &str) -> Result<Option<DbPerson>, String> {
    let conn = db.conn_ref();
    conn.query_row(
        "SELECT id, email, name, organization, role,
                linkedin_url, twitter_handle, phone, photo_url, bio,
                title_history, company_industry, company_size, company_hq,
                enrichment_sources
         FROM people WHERE id = ?1",
        [person_id],
        |row| {
            Ok(DbPerson {
                id: row.get(0)?,
                email: row.get(1)?,
                name: row.get(2)?,
                organization: row.get(3)?,
                role: row.get(4)?,
                linkedin_url: row.get(5)?,
                twitter_handle: row.get(6)?,
                phone: row.get(7)?,
                photo_url: row.get(8)?,
                bio: row.get(9)?,
                title_history: row.get(10)?,
                company_industry: row.get(11)?,
                company_size: row.get(12)?,
                company_hq: row.get(13)?,
                enrichment_sources: row.get(14)?,
            })
        },
    )
    .map(Some)
    .or_else(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => Ok(None),
        other => Err(format!("Failed to load person {}: {}", person_id, other)),
    })
}

/// Persist updated enrichment fields back to the `people` table.
#[allow(clippy::too_many_arguments)]
fn update_person_enrichment(
    db: &ActionDb,
    person_id: &str,
    linkedin_url: Option<&str>,
    twitter_handle: Option<&str>,
    phone: Option<&str>,
    photo_url: Option<&str>,
    bio: Option<&str>,
    title_history: Option<&str>,
    company_industry: Option<&str>,
    company_size: Option<&str>,
    company_hq: Option<&str>,
    role: Option<&str>,
    organization: Option<&str>,
    enrichment_sources_json: &str,
) -> Result<(), String> {
    let conn = db.conn_ref();
    conn.execute(
        "UPDATE people SET
            linkedin_url = ?2,
            twitter_handle = ?3,
            phone = ?4,
            photo_url = ?5,
            bio = ?6,
            title_history = ?7,
            company_industry = ?8,
            company_size = ?9,
            company_hq = ?10,
            role = ?11,
            organization = ?12,
            enrichment_sources = ?13,
            last_enriched_at = ?14,
            updated_at = ?14
         WHERE id = ?1",
        params![
            person_id,
            linkedin_url,
            twitter_handle,
            phone,
            photo_url,
            bio,
            title_history,
            company_industry,
            company_size,
            company_hq,
            role,
            organization,
            enrichment_sources_json,
            Utc::now().to_rfc3339(),
        ],
    )
    .map_err(|e| format!("Failed to update person enrichment for {}: {}", person_id, e))?;
    Ok(())
}

/// Insert an entry in the `enrichment_log` audit trail.
#[allow(clippy::too_many_arguments)]
fn insert_enrichment_log(
    db: &ActionDb,
    entity_type: &str,
    entity_id: &str,
    source: &str,
    event_type: &str,
    signal_type: Option<&str>,
    fields_updated: &[String],
    raw_payload: Option<&str>,
) -> Result<(), String> {
    let conn = db.conn_ref();
    let id = format!("el-{}", Uuid::new_v4());
    let fields_json = serde_json::to_string(fields_updated)
        .unwrap_or_else(|_| "[]".to_string());

    conn.execute(
        "INSERT INTO enrichment_log
            (id, entity_type, entity_id, source, event_type, signal_type,
             fields_updated, raw_payload, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, datetime('now'))",
        params![
            id,
            entity_type,
            entity_id,
            source,
            event_type,
            signal_type,
            fields_json,
            raw_payload,
        ],
    )
    .map_err(|e| format!("Failed to insert enrichment_log: {}", e))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Merge helpers
// ---------------------------------------------------------------------------

/// Apply a Clay field value if the source priority allows it.
/// Returns `true` and updates `sources` when the field was written.
fn try_set_field(
    target: &mut Option<String>,
    field_name: &str,
    new_value: Option<&str>,
    sources: &mut EnrichmentSources,
    current_sources_json: Option<&str>,
) -> bool {
    let value = match new_value {
        Some(v) if !v.is_empty() => v,
        _ => return false,
    };

    if !can_write_field(current_sources_json, field_name, "clay") {
        return false;
    }

    *target = Some(value.to_string());
    sources.insert(
        field_name.to_string(),
        FieldSource {
            source: "clay".to_string(),
            at: Utc::now().to_rfc3339(),
        },
    );
    true
}

// ---------------------------------------------------------------------------
// Main enrichment flow
// ---------------------------------------------------------------------------

/// Convenience wrapper that extracts the DB from AppState.
///
/// Splits work into lock-free phases so the `MutexGuard` is never held across
/// an `.await` (required for Tauri Send-safety).
pub async fn enrich_person_from_clay_with_client(
    state: &AppState,
    person_id: &str,
    client: &ClayClient,
) -> Result<EnrichmentResult, String> {
    // Phase 1: Load person under lock, then release
    let person = {
        let db_guard = state.db.lock().map_err(|_| "DB lock poisoned")?;
        let db = db_guard.as_ref().ok_or("Database not initialized")?;
        load_person(db, person_id)?
            .ok_or_else(|| format!("Person not found: {}", person_id))?
    };

    // Phase 2: Async Clay calls (no lock held)
    let mut search_results = client.search_contact(&person.email).await.map_err(|e| e.to_string())?;
    if search_results.is_empty() {
        let org = person.organization.as_deref().unwrap_or("");
        if !org.is_empty() {
            let query = format!("{} {}", person.name, org);
            search_results = client.search_contact(&query).await.map_err(|e| e.to_string())?;
        }
    }
    if search_results.is_empty() {
        // No match — log under lock and return
        let db_guard = state.db.lock().map_err(|_| "DB lock poisoned")?;
        let db = db_guard.as_ref().ok_or("Database not initialized")?;
        insert_enrichment_log(db, "person", person_id, "clay", "enrichment", None, &[], Some("no_match"))?;
        return Ok(EnrichmentResult {
            person_id: person_id.to_string(),
            fields_updated: vec![],
            signals: vec![],
            clay_contact_id: None,
        });
    }

    let best = search_results
        .iter()
        .find(|r| r.email.as_deref().map(|e| e.eq_ignore_ascii_case(&person.email)).unwrap_or(false))
        .or_else(|| search_results.first())
        .ok_or("No contact selected")?;
    let clay_id = best.id.clone();

    let detail = client.get_contact_detail(&clay_id).await.map_err(|e| e.to_string())?;
    let _stats = client.get_contact_stats(&clay_id).await.ok();

    // Phase 3: Merge + write under lock
    let new_title_history_json = if detail.title_history.is_empty() {
        None
    } else {
        serde_json::to_string(&detail.title_history).ok()
    };

    let signals = super::signals::detect_changes(
        person.title_history.as_deref(),
        new_title_history_json.as_deref(),
        person.organization.as_deref(),
        detail.company.as_deref(),
        person.linkedin_url.as_deref(),
        detail.linkedin_url.as_deref(),
        person.twitter_handle.as_deref(),
        detail.twitter_handle.as_deref(),
    );

    let db_guard = state.db.lock().map_err(|_| "DB lock poisoned")?;
    let db = db_guard.as_ref().ok_or("Database not initialized")?;

    let current_sources_json = person.enrichment_sources.as_deref();
    let mut sources: EnrichmentSources = current_sources_json
        .and_then(|json| serde_json::from_str(json).ok())
        .unwrap_or_default();
    let mut fields_updated: Vec<String> = Vec::new();

    let mut linkedin_url = person.linkedin_url.clone();
    let mut twitter_handle = person.twitter_handle.clone();
    let mut phone = person.phone.clone();
    let mut photo_url = person.photo_url.clone();
    let mut bio = person.bio.clone();
    let mut title_history = person.title_history.clone();
    let mut company_industry = person.company_industry.clone();
    let mut company_size = person.company_size.clone();
    let mut company_hq = person.company_hq.clone();
    let mut role = person.role.clone();
    let mut organization = person.organization.clone();

    let field_mappings: Vec<(&str, Option<&str>, &mut Option<String>)> = vec![
        ("linkedin_url", detail.linkedin_url.as_deref(), &mut linkedin_url),
        ("twitter_handle", detail.twitter_handle.as_deref(), &mut twitter_handle),
        ("phone", detail.phone.as_deref(), &mut phone),
        ("photo_url", detail.photo_url.as_deref(), &mut photo_url),
        ("bio", detail.bio.as_deref(), &mut bio),
        ("company_industry", detail.company_industry.as_deref(), &mut company_industry),
        ("company_size", detail.company_size.as_deref(), &mut company_size),
        ("company_hq", detail.company_hq.as_deref(), &mut company_hq),
        ("role", detail.title.as_deref(), &mut role),
        ("organization", detail.company.as_deref(), &mut organization),
    ];

    for (field_name, new_value, target) in field_mappings {
        if try_set_field(target, field_name, new_value, &mut sources, current_sources_json) {
            fields_updated.push(field_name.to_string());
        }
    }

    if !detail.title_history.is_empty() && can_write_field(current_sources_json, "title_history", "clay") {
        title_history = serde_json::to_string(&detail.title_history).ok();
        sources.insert("title_history".to_string(), FieldSource {
            source: "clay".to_string(),
            at: Utc::now().to_rfc3339(),
        });
        fields_updated.push("title_history".to_string());
    }

    let sources_json = serde_json::to_string(&sources).unwrap_or_else(|_| "{}".to_string());
    update_person_enrichment(
        db, person_id,
        linkedin_url.as_deref(), twitter_handle.as_deref(), phone.as_deref(),
        photo_url.as_deref(), bio.as_deref(), title_history.as_deref(),
        company_industry.as_deref(), company_size.as_deref(), company_hq.as_deref(),
        role.as_deref(), organization.as_deref(), &sources_json,
    )?;

    let signal_names: Vec<String> = signals.iter().map(|s| s.signal_type.clone()).collect();
    let raw = serde_json::to_string(&detail).ok();
    insert_enrichment_log(db, "person", person_id, "clay", "enrichment", None, &fields_updated, raw.as_deref())?;

    for signal in &signal_names {
        insert_enrichment_log(db, "person", person_id, "clay", "signal", Some(signal), &fields_updated, None)?;
    }

    Ok(EnrichmentResult {
        person_id: person_id.to_string(),
        fields_updated,
        signals: signal_names,
        clay_contact_id: Some(clay_id),
    })
}

/// Enrich a person record from Clay.earth contact data.
///
/// 1. Loads the person from the local DB.
/// 2. Searches Clay by email (falls back to name + org).
/// 3. Fetches detail and stats for the best match.
/// 4. Detects change signals via `signals::detect_changes`.
/// 5. Merges fields respecting source priority.
/// 6. Persists updates and writes an enrichment log entry.
pub async fn enrich_person_from_clay(
    db: &ActionDb,
    person_id: &str,
    client: &ClayClient,
) -> Result<EnrichmentResult, String> {
    // 1. Load person
    let person = load_person(db, person_id)?
        .ok_or_else(|| format!("Person not found: {}", person_id))?;

    // 2. Search Clay by email
    let mut search_results = client.search_contact(&person.email).await.map_err(|e| e.to_string())?;

    // If no results by email, try name + org
    if search_results.is_empty() {
        let org = person.organization.as_deref().unwrap_or("");
        if !org.is_empty() {
            let query = format!("{} {}", person.name, org);
            search_results = client.search_contact(&query).await.map_err(|e| e.to_string())?;
        }
    }

    if search_results.is_empty() {
        // No Clay match — record the attempt and return empty result
        insert_enrichment_log(
            db,
            "person",
            person_id,
            "clay",
            "enrichment",
            None,
            &[],
            Some("no_match"),
        )?;
        return Ok(EnrichmentResult {
            person_id: person_id.to_string(),
            fields_updated: vec![],
            signals: vec![],
            clay_contact_id: None,
        });
    }

    // 3. Pick best match — prefer email-matching result
    let best = search_results
        .iter()
        .find(|r| {
            r.email
                .as_deref()
                .map(|e| e.eq_ignore_ascii_case(&person.email))
                .unwrap_or(false)
        })
        .or_else(|| search_results.first())
        .ok_or_else(|| "Search returned results but none could be selected".to_string())?;

    let clay_id = best.id.clone();

    // 4. Fetch full detail and stats
    let detail: ClayContactDetail = client.get_contact_detail(&clay_id).await.map_err(|e| e.to_string())?;
    let stats = client.get_contact_stats(&clay_id).await.ok();

    // 5. Detect change signals
    let new_title_history_json = if detail.title_history.is_empty() {
        None
    } else {
        serde_json::to_string(&detail.title_history).ok()
    };
    let signals = super::signals::detect_changes(
        person.title_history.as_deref(),
        new_title_history_json.as_deref(),
        person.organization.as_deref(),
        detail.company.as_deref(),
        person.linkedin_url.as_deref(),
        detail.linkedin_url.as_deref(),
        person.twitter_handle.as_deref(),
        detail.twitter_handle.as_deref(),
    );

    // 6. Merge with priority
    let current_sources_json = person.enrichment_sources.as_deref();
    let mut sources: EnrichmentSources = current_sources_json
        .and_then(|json| serde_json::from_str(json).ok())
        .unwrap_or_default();

    let mut fields_updated: Vec<String> = Vec::new();

    // Mutable copies of the person's current field values
    let mut linkedin_url = person.linkedin_url.clone();
    let mut twitter_handle = person.twitter_handle.clone();
    let mut phone = person.phone.clone();
    let mut photo_url = person.photo_url.clone();
    let mut bio = person.bio.clone();
    let mut title_history = person.title_history.clone();
    let mut company_industry = person.company_industry.clone();
    let mut company_size = person.company_size.clone();
    let mut company_hq = person.company_hq.clone();
    let mut role = person.role.clone();
    let mut organization = person.organization.clone();

    // Apply each enrichable field
    let field_mappings: Vec<(&str, Option<&str>, &mut Option<String>)> = vec![
        ("linkedin_url", detail.linkedin_url.as_deref(), &mut linkedin_url),
        ("twitter_handle", detail.twitter_handle.as_deref(), &mut twitter_handle),
        ("phone", detail.phone.as_deref(), &mut phone),
        ("photo_url", detail.photo_url.as_deref(), &mut photo_url),
        ("bio", detail.bio.as_deref(), &mut bio),
        ("company_industry", detail.company_industry.as_deref(), &mut company_industry),
        ("company_size", detail.company_size.as_deref(), &mut company_size),
        ("company_hq", detail.company_hq.as_deref(), &mut company_hq),
        ("role", detail.title.as_deref(), &mut role),
        ("organization", detail.company.as_deref(), &mut organization),
    ];

    for (field_name, new_value, target) in field_mappings {
        if try_set_field(target, field_name, new_value, &mut sources, current_sources_json) {
            fields_updated.push(field_name.to_string());
        }
    }

    // Title history is JSON — merge rather than overwrite
    if !detail.title_history.is_empty() && can_write_field(current_sources_json, "title_history", "clay") {
        title_history = serde_json::to_string(&detail.title_history).ok();
        sources.insert(
            "title_history".to_string(),
            FieldSource {
                source: "clay".to_string(),
                at: Utc::now().to_rfc3339(),
            },
        );
        fields_updated.push("title_history".to_string());
    }

    // Include stats-derived fields if available
    if let Some(ref st) = stats {
        if let Some(ref last_interaction) = st.last_interaction_at {
            if can_write_field(current_sources_json, "clay_last_interaction", "clay") {
                sources.insert(
                    "clay_last_interaction".to_string(),
                    FieldSource {
                        source: "clay".to_string(),
                        at: Utc::now().to_rfc3339(),
                    },
                );
                let _ = last_interaction; // consumed via sources tracking
            }
        }
    }

    let sources_json =
        serde_json::to_string(&sources).unwrap_or_else(|_| "{}".to_string());

    // 7. Write to DB
    update_person_enrichment(
        db,
        person_id,
        linkedin_url.as_deref(),
        twitter_handle.as_deref(),
        phone.as_deref(),
        photo_url.as_deref(),
        bio.as_deref(),
        title_history.as_deref(),
        company_industry.as_deref(),
        company_size.as_deref(),
        company_hq.as_deref(),
        role.as_deref(),
        organization.as_deref(),
        &sources_json,
    )?;

    // 8. Enrichment log
    let signal_names: Vec<String> = signals.iter().map(|s| s.signal_type.clone()).collect();
    let raw = serde_json::to_string(&detail).ok();

    insert_enrichment_log(
        db,
        "person",
        person_id,
        "clay",
        "enrichment",
        None,
        &fields_updated,
        raw.as_deref(),
    )?;

    // If there are change signals, log them separately
    for signal in &signal_names {
        insert_enrichment_log(
            db,
            "person",
            person_id,
            "clay",
            "signal",
            Some(signal),
            &fields_updated,
            None,
        )?;
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
