use std::collections::HashSet;

use rusqlite::params;

use crate::db::ActionDb;
use crate::entity::EntityType;
use crate::google_api::classify::EntityHint;

/// Normalize a string for fuzzy matching: lowercase + ASCII alphanumeric only.
pub fn normalize_key(value: &str) -> String {
    value
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_lowercase()
}

/// Normalize a list of domain strings: trim, lowercase, dedupe, sort.
pub fn normalize_domains(domains: &[String]) -> Vec<String> {
    let mut out: Vec<String> = domains
        .iter()
        .map(|d| d.trim().to_lowercase())
        .filter(|d| !d.is_empty())
        .collect();
    out.sort();
    out.dedup();
    out
}

/// Build entity hints from DB for multi-entity meeting classification (I336).
pub fn build_entity_hints(db: &ActionDb) -> Vec<EntityHint> {
    let mut hints = Vec::new();

    // 1. Accounts: name slugs + domains (account_domains table) + keywords
    if let Ok(accounts) = db.get_all_accounts() {
        for acct in accounts.iter().filter(|a| !a.archived) {
            let domains = db.get_account_domains(&acct.id).unwrap_or_default();
            let keywords = acct.keywords.as_deref()
                .and_then(|k| serde_json::from_str::<Vec<String>>(k).ok())
                .unwrap_or_default();
            let slug = normalize_key(&acct.name);
            if slug.len() >= 3 || !domains.is_empty() || !keywords.is_empty() {
                hints.push(EntityHint {
                    id: acct.id.clone(),
                    entity_type: EntityType::Account,
                    name: acct.name.clone(),
                    slugs: if slug.len() >= 3 { vec![slug] } else { vec![] },
                    domains,
                    keywords,
                    emails: vec![],
                });
            }
        }
    }

    // 2. Projects: name slugs + keywords
    if let Ok(projects) = db.get_all_projects() {
        for proj in projects.iter().filter(|p| !p.archived) {
            let keywords = proj.keywords.as_deref()
                .and_then(|k| serde_json::from_str::<Vec<String>>(k).ok())
                .unwrap_or_default();
            let slug = normalize_key(&proj.name);
            if slug.len() >= 3 || !keywords.is_empty() {
                hints.push(EntityHint {
                    id: proj.id.clone(),
                    entity_type: EntityType::Project,
                    name: proj.name.clone(),
                    slugs: if slug.len() >= 3 { vec![slug] } else { vec![] },
                    domains: vec![],
                    keywords,
                    emails: vec![],
                });
            }
        }
    }

    // 3. People: email for 1:1 attendee matching
    if let Ok(people) = db.get_people(None) {
        for person in people.iter().filter(|p| !p.archived) {
            let mut emails = vec![person.email.clone()];
            // Also include aliases
            if let Ok(aliases) = db.get_person_emails(&person.id) {
                for alias in aliases {
                    if alias != person.email {
                        emails.push(alias);
                    }
                }
            }
            hints.push(EntityHint {
                id: person.id.clone(),
                entity_type: EntityType::Person,
                name: person.name.clone(),
                slugs: vec![],
                domains: vec![],
                keywords: vec![],
                emails,
            });
        }
    }

    hints
}

/// Build account hint set for email classification (backward compat). I336.
/// Extracts account slugs from entity hints for use by email_classify.
pub fn account_hints_from_entity_hints(entity_hints: &[EntityHint]) -> HashSet<String> {
    entity_hints.iter()
        .filter(|h| matches!(h.entity_type, EntityType::Account))
        .flat_map(|h| h.slugs.iter().cloned())
        .collect()
}

/// Build account hint set for meeting classification (legacy — delegates to entity hints).
pub fn build_external_account_hints(db: &ActionDb) -> HashSet<String> {
    account_hints_from_entity_hints(&build_entity_hints(db))
}

// ---------------------------------------------------------------------------
// Entity name resolution (unified from signals/callouts + proactive/detectors)
// ---------------------------------------------------------------------------

/// Resolve a display name for an entity from accounts, projects, or people tables.
///
/// Returns the entity name if found, or falls back to `entity_id` as a string.
pub fn resolve_entity_name(db: &ActionDb, entity_type: &str, entity_id: &str) -> String {
    let (table, col) = match entity_type {
        "account" => ("accounts", "name"),
        "project" => ("projects", "name"),
        "person" => ("people", "name"),
        _ => return entity_id.to_string(),
    };
    let sql = format!("SELECT {} FROM {} WHERE id = ?1", col, table);
    db.conn_ref()
        .query_row(&sql, params![entity_id], |row| row.get::<_, String>(0))
        .unwrap_or_else(|_| entity_id.to_string())
}

// ---------------------------------------------------------------------------
// Attendee email parsing (unified from signals/patterns, email_bridge, post_meeting)
// ---------------------------------------------------------------------------

/// Parse attendee emails from a DB-stored string (comma-separated or JSON array).
///
/// Normalizes to lowercase and filters to valid-looking email addresses.
pub fn parse_attendee_emails(raw: &str) -> Vec<String> {
    // Try JSON array first
    if let Ok(arr) = serde_json::from_str::<Vec<String>>(raw) {
        return arr
            .into_iter()
            .map(|e| e.trim().to_lowercase())
            .filter(|e| e.contains('@'))
            .collect();
    }
    // Fall back to comma-separated
    raw.split(',')
        .map(|s| s.trim().to_lowercase())
        .filter(|s| s.contains('@'))
        .collect()
}

/// Extract attendee emails from a meeting JSON value's "attendees" array field.
pub fn extract_attendee_emails(meeting: &serde_json::Value) -> Vec<String> {
    if let Some(arr) = meeting.get("attendees").and_then(|v| v.as_array()) {
        return arr
            .iter()
            .filter_map(|v| v.as_str())
            .map(|s| s.trim().to_lowercase())
            .filter(|s| s.contains('@'))
            .collect();
    }
    Vec::new()
}
