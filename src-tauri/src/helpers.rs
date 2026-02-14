use std::collections::HashSet;

use crate::db::ActionDb;

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

/// Build account hint set for meeting classification.
/// Contains normalized account IDs, names, and domain prefixes for external accounts.
pub fn build_external_account_hints(db: &ActionDb) -> HashSet<String> {
    let mut hints = HashSet::new();
    if let Ok(accounts) = db.get_all_accounts() {
        for account in accounts.into_iter().filter(|a| !a.is_internal && !a.archived) {
            let id_key = normalize_key(&account.id);
            if id_key.len() >= 3 {
                hints.insert(id_key);
            }
            let name_key = normalize_key(&account.name);
            if name_key.len() >= 3 {
                hints.insert(name_key);
            }
            if let Ok(domains) = db.get_account_domains(&account.id) {
                for domain in domains {
                    let base = domain.split('.').next().unwrap_or("").to_lowercase();
                    let base_key = normalize_key(&base);
                    if base_key.len() >= 3 {
                        hints.insert(base_key);
                    }
                }
            }
        }
    }
    hints
}
