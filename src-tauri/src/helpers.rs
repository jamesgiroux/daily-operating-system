use std::collections::HashSet;

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
