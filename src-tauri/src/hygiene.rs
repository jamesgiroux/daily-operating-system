//! Proactive intelligence maintenance (I145 — ADR-0058).
//!
//! The hygiene scanner detects data quality gaps across entity types and data
//! sources, then applies mechanical fixes (free, instant) before enqueuing
//! AI-budgeted enrichment for remaining gaps.
//!
//! Background loop: runs 30s after startup, then every 4 hours.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use crate::db::ActionDb;
use crate::state::AppState;
use crate::types::Config;

/// How long to wait after startup before the first scan.
const STARTUP_DELAY_SECS: u64 = 30;

/// Interval between scans (4 hours).
const SCAN_INTERVAL_SECS: u64 = 4 * 60 * 60;


/// Max people per domain for pairwise duplicate detection (prevents O(n²) explosion).
const MAX_DOMAIN_GROUP_SIZE: usize = 200;

/// Public interval getter for UI/command next-scan calculations.
/// Uses config value if provided, otherwise falls back to constant.
pub fn scan_interval_secs(config: Option<&Config>) -> u64 {
    config
        .map(|c| c.hygiene_scan_interval_hours as u64 * 3600)
        .unwrap_or(SCAN_INTERVAL_SECS)
}

/// A single narrative fix description for the UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HygieneFixDetail {
    pub fix_type: String,
    pub entity_name: Option<String>,
    pub description: String,
}

/// Maximum number of fix details to store per report.
const MAX_FIX_DETAILS: usize = 20;

/// Report from a hygiene scan: gaps detected + mechanical fixes applied.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HygieneReport {
    pub unnamed_people: usize,
    pub unknown_relationships: usize,
    pub missing_intelligence: usize,
    pub stale_intelligence: usize,
    pub unsummarized_files: usize,
    pub duplicate_people: usize,
    pub abandoned_quill_syncs: usize,
    /// Meetings with low-confidence entity matches (I305).
    pub low_confidence_entity_matches: usize,
    pub fixes: MechanicalFixes,
    pub fix_details: Vec<HygieneFixDetail>,
    pub scanned_at: String,
    pub scan_duration_ms: u64,
}

/// Counts of mechanical fixes applied during a hygiene scan.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MechanicalFixes {
    pub relationships_reclassified: usize,
    pub summaries_extracted: usize,
    pub meeting_counts_updated: usize,
    pub names_resolved: usize,
    pub people_linked_by_domain: usize,
    pub people_deduped_by_alias: usize,
    pub renewals_rolled_over: usize,
    pub ai_enrichments_enqueued: usize,
    pub quill_syncs_retried: usize,
    /// Entity suggestions created from low-confidence matches (I305).
    pub entity_suggestions_created: usize,
}

/// Run a full hygiene scan: detect gaps, apply mechanical fixes, return report.
/// If `budget` is provided, enqueue AI enrichment for remaining gaps.
pub fn run_hygiene_scan(
    db: &ActionDb,
    config: &Config,
    workspace: &Path,
    budget: Option<&crate::state::HygieneBudget>,
    queue: Option<&crate::intel_queue::IntelligenceQueue>,
    _first_run: bool,
    embedding_model: Option<&crate::embeddings::EmbeddingModel>,
) -> HygieneReport {
    let scan_start = std::time::Instant::now();
    let mut report = HygieneReport {
        scanned_at: Utc::now().to_rfc3339(),
        ..Default::default()
    };

    // --- Gap detection ---
    report.unnamed_people = db.get_unnamed_people().map(|v| v.len()).unwrap_or(0);
    report.unknown_relationships = db
        .get_unknown_relationship_people()
        .map(|v| v.len())
        .unwrap_or(0);
    report.missing_intelligence = db
        .get_entities_without_intelligence()
        .map(|v| v.len())
        .unwrap_or(0);
    report.stale_intelligence = db
        .get_stale_entity_intelligence(14)
        .map(|v| v.len())
        .unwrap_or(0);
    report.unsummarized_files = db
        .get_unsummarized_content_files()
        .map(|v| v.len())
        .unwrap_or(0);
    report.duplicate_people = detect_duplicate_people(db).map(|v| v.len()).unwrap_or(0);
    report.abandoned_quill_syncs = db.count_quill_syncs_by_state("abandoned").unwrap_or(0);

    // --- Phase 1: Mechanical fixes (free, instant) ---
    let user_domains = config.resolved_user_domains();
    let mut all_details: Vec<HygieneFixDetail> = Vec::new();

    let (count, details) = fix_unknown_relationships(db, &user_domains);
    report.fixes.relationships_reclassified = count;
    all_details.extend(details);

    let (count, details) = backfill_file_summaries(db);
    report.fixes.summaries_extracted = count;
    all_details.extend(details);

    let (count, details) = fix_meeting_counts(db);
    report.fixes.meeting_counts_updated = count;
    all_details.extend(details);

    let (count, details) = fix_renewal_rollovers(db);
    report.fixes.renewals_rolled_over = count;
    all_details.extend(details);

    let (count, details) = retry_abandoned_quill_syncs(db);
    report.fixes.quill_syncs_retried = count;
    all_details.extend(details);

    // --- Phase 2: Email name resolution + domain linking (free) ---
    let (count, details) = resolve_names_from_emails(db, workspace);
    report.fixes.names_resolved = count;
    all_details.extend(details);

    let (count, details) = auto_link_people_by_domain(db);
    report.fixes.people_linked_by_domain = count;
    all_details.extend(details);

    let (count, details) = dedup_people_by_domain_alias(db, &user_domains);
    report.fixes.people_deduped_by_alias = count;
    all_details.extend(details);

    // --- Phase 2b: Low-confidence entity match detection (I305) ---
    let accounts_dir = workspace.join("Accounts");
    let (count, details) = detect_low_confidence_matches(db, &accounts_dir, embedding_model);
    report.fixes.entity_suggestions_created = count;
    report.low_confidence_entity_matches = count;
    all_details.extend(details);

    // --- Phase 2c: Attendee group pattern mining (I307) ---
    match crate::signals::patterns::mine_attendee_patterns(db) {
        Ok(count) if count > 0 => {
            log::info!("Hygiene: mined {} attendee group pattern updates", count);
        }
        Err(e) => {
            log::warn!("Hygiene: attendee pattern mining failed: {}", e);
        }
        _ => {}
    }

    // --- Phase 3: AI-budgeted gap filling ---
    if let (Some(budget), Some(queue)) = (budget, queue) {
        report.fixes.ai_enrichments_enqueued = enqueue_ai_enrichments(db, budget, queue);
    }

    // Truncate details to max and store on report
    all_details.truncate(MAX_FIX_DETAILS);
    report.fix_details = all_details;

    // --- Re-count gaps after fixes so UI shows remaining problems, not stale pre-fix counts ---
    report.unnamed_people = db.get_unnamed_people().map(|v| v.len()).unwrap_or(0);
    report.unknown_relationships = db
        .get_unknown_relationship_people()
        .map(|v| v.len())
        .unwrap_or(0);
    report.unsummarized_files = db
        .get_unsummarized_content_files()
        .map(|v| v.len())
        .unwrap_or(0);
    // Intelligence gaps don't change from mechanical fixes — only AI enrichment resolves them.
    // duplicate_people is also unchanged (no auto-merge).
    report.abandoned_quill_syncs = db.count_quill_syncs_by_state("abandoned").unwrap_or(0);

    report.scan_duration_ms = scan_start.elapsed().as_millis() as u64;
    report
}

/// Reclassify people with "unknown" relationship using the user's domains (I171).
fn fix_unknown_relationships(db: &ActionDb, user_domains: &[String]) -> (usize, Vec<HygieneFixDetail>) {
    if user_domains.is_empty() {
        return (0, Vec::new());
    }

    let people = match db.get_unknown_relationship_people() {
        Ok(p) => p,
        Err(_) => return (0, Vec::new()),
    };

    let mut fixed = 0;
    let mut details = Vec::new();
    for person in &people {
        let new_rel = crate::util::classify_relationship_multi(&person.email, user_domains);
        if new_rel != "unknown" && db.update_person_relationship(&person.id, &new_rel).is_ok() {
            details.push(HygieneFixDetail {
                fix_type: "relationship_reclassified".to_string(),
                entity_name: Some(person.name.clone()),
                description: format!("Reclassified {} ({}) as {}", person.name, person.email, new_rel),
            });
            fixed += 1;
        }
    }
    (fixed, details)
}

/// Extract summaries for content files that have none.
fn backfill_file_summaries(db: &ActionDb) -> (usize, Vec<HygieneFixDetail>) {
    let files = match db.get_unsummarized_content_files() {
        Ok(f) => f,
        Err(_) => return (0, Vec::new()),
    };

    // Cap per scan to avoid blocking too long (mechanical extraction is fast but IO-bound)
    let batch_limit = 50;
    let mut extracted = 0;
    let mut details = Vec::new();
    let now = chrono::Utc::now().to_rfc3339();

    for file in files.iter().take(batch_limit) {
        let path = std::path::Path::new(&file.absolute_path);
        let filename = path
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_default();

        if !path.exists() {
            // File was deleted since indexing — mark so it exits the unsummarized pool
            let _ = db.conn_ref().execute(
                "UPDATE content_index SET extracted_at = ?1, summary = ?2 WHERE id = ?3",
                rusqlite::params![&now, "[file not found]", file.id],
            );
            continue;
        }

        let (extracted_at, summary) = crate::entity_intel::extract_and_summarize(path);
        match (extracted_at, summary) {
            (Some(ext_at), Some(summ)) => {
                let _ = db.conn_ref().execute(
                    "UPDATE content_index SET extracted_at = ?1, summary = ?2 WHERE id = ?3",
                    rusqlite::params![ext_at, summ, file.id],
                );
                if details.len() < 5 {
                    details.push(HygieneFixDetail {
                        fix_type: "summary_extracted".to_string(),
                        entity_name: Some(filename.clone()),
                        description: format!("Extracted summary for {}", filename),
                    });
                }
                extracted += 1;
            }
            _ => {
                // Extraction failed or returned empty — mark as attempted so the file
                // doesn't reappear as an unsummarized gap on every scan forever.
                let _ = db.conn_ref().execute(
                    "UPDATE content_index SET extracted_at = ?1, summary = ?2 WHERE id = ?3",
                    rusqlite::params![&now, "[extraction failed]", file.id],
                );
            }
        }
    }
    (extracted, details)
}

/// Recompute meeting counts for people whose counts may be stale.
/// Only fixes people whose stored count differs from actual attendee records.
fn fix_meeting_counts(db: &ActionDb) -> (usize, Vec<HygieneFixDetail>) {
    // Find people with mismatched counts via a single query
    let mismatched: Vec<(String, String, i64, i64)> = db
        .conn_ref()
        .prepare(
            "SELECT p.id, p.name, p.meeting_count, COALESCE(ma.actual, 0) FROM people p
             LEFT JOIN (
                 SELECT person_id, COUNT(*) AS actual FROM meeting_attendees GROUP BY person_id
             ) ma ON ma.person_id = p.id
             WHERE p.meeting_count != COALESCE(ma.actual, 0)",
        )
        .and_then(|mut stmt| {
            let rows = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            })?;
            Ok(rows.filter_map(|r| r.ok()).collect())
        })
        .unwrap_or_default();

    let mut fixed = 0;
    let mut details = Vec::new();
    for (person_id, name, old_count, new_count) in &mismatched {
        if db.recompute_person_meeting_count(person_id).is_ok() {
            details.push(HygieneFixDetail {
                fix_type: "meeting_count_updated".to_string(),
                entity_name: Some(name.clone()),
                description: format!("Updated meeting count for {}: {} \u{2192} {}", name, old_count, new_count),
            });
            fixed += 1;
        }
    }
    (fixed, details)
}

/// Auto-rollover renewal dates for accounts that passed their renewal without churning.
///
/// For each non-archived account whose `contract_end` is in the past:
///   1. Skip if the account has a 'churn' event (defensive — `get_accounts_past_renewal`
///      already filters these, but this guards against race conditions).
///   2. Record a 'renewal' event with the original contract_end date and current ARR.
///   3. Advance `contract_end` by 12 months.
///
/// This ensures renewals don't silently go stale when the user simply continues the
/// relationship without explicitly recording the event.
fn fix_renewal_rollovers(db: &ActionDb) -> (usize, Vec<HygieneFixDetail>) {
    let past_renewal = match db.get_accounts_past_renewal() {
        Ok(accounts) => accounts,
        Err(_) => return (0, Vec::new()),
    };

    let mut fixed = 0;
    let mut details = Vec::new();
    for account in &past_renewal {
        // Defensive: skip if a churn event exists
        if db.has_churn_event(&account.id).unwrap_or(false) {
            continue;
        }

        let renewal_date = match account.contract_end.as_deref() {
            Some(d) if !d.is_empty() => d,
            _ => continue,
        };

        let parsed = match chrono::NaiveDate::parse_from_str(renewal_date, "%Y-%m-%d") {
            Ok(d) => d,
            Err(_) => continue,
        };

        // Record the implicit renewal event
        if db
            .record_account_event(
                &account.id,
                "renewal",
                renewal_date,
                account.arr,
                Some("Auto-renewed (no churn recorded)"),
            )
            .is_err()
        {
            continue;
        }

        // Advance contract_end by 12 months
        let next = parsed + chrono::Months::new(12);
        let next_str = next.format("%Y-%m-%d").to_string();
        let _ = db.conn_ref().execute(
            "UPDATE accounts SET contract_end = ?1 WHERE id = ?2",
            rusqlite::params![next_str, account.id],
        );
        details.push(HygieneFixDetail {
            fix_type: "renewal_rolled_over".to_string(),
            entity_name: Some(account.name.clone()),
            description: format!("Rolled over {} renewal: {} \u{2192} {}", account.name, renewal_date, next_str),
        });
        fixed += 1;
    }

    (fixed, details)
}

/// Retry abandoned Quill syncs that are between 7 and 14 days old.
fn retry_abandoned_quill_syncs(db: &ActionDb) -> (usize, Vec<HygieneFixDetail>) {
    let syncs = match db.get_retryable_abandoned_quill_syncs(7, 14) {
        Ok(s) => s,
        Err(_) => return (0, Vec::new()),
    };

    let mut retried = 0;
    let mut details = Vec::new();
    for sync_row in &syncs {
        if db.reset_quill_sync_for_retry(&sync_row.id).is_ok() {
            details.push(HygieneFixDetail {
                fix_type: "quill_sync_retried".to_string(),
                entity_name: Some(sync_row.meeting_id.clone()),
                description: format!(
                    "Reset abandoned Quill sync for meeting {}",
                    sync_row.meeting_id
                ),
            });
            retried += 1;
        }
    }

    (retried, details)
}

// =============================================================================
// Phase 2: Email Name Resolution + Domain Linking (I146)
// =============================================================================

/// Resolve unnamed people from email From headers in emails.json.
///
/// Reads `_today/data/emails.json` (created by daily briefing), extracts display
/// names from From headers, and updates people who only have email-derived names.
pub fn resolve_names_from_emails(db: &ActionDb, workspace: &Path) -> (usize, Vec<HygieneFixDetail>) {
    let emails_path = workspace.join("_today").join("data").join("emails.json");
    let raw = match std::fs::read_to_string(&emails_path) {
        Ok(r) => r,
        Err(_) => return (0, Vec::new()),
    };

    let data: serde_json::Value = match serde_json::from_str(&raw) {
        Ok(d) => d,
        Err(_) => return (0, Vec::new()),
    };

    // Get unnamed people to match against
    let unnamed = match db.get_unnamed_people() {
        Ok(p) if !p.is_empty() => p,
        _ => return (0, Vec::new()),
    };
    let unnamed_emails: std::collections::HashSet<String> =
        unnamed.iter().map(|p| p.email.to_lowercase()).collect();

    let mut resolved = 0;
    let mut details = Vec::new();

    // Scan all email categories
    for key in &["highPriority", "mediumPriority", "lowPriority"] {
        if let Some(emails) = data.get(key).and_then(|v| v.as_array()) {
            for email in emails {
                let from = match email.get("from").and_then(|v| v.as_str()) {
                    Some(f) => f,
                    None => continue,
                };

                // Extract display name and email address
                let display_name = match crate::prepare::email_classify::extract_display_name(from)
                {
                    Some(n) => n,
                    None => continue,
                };
                let addr = crate::prepare::email_classify::extract_email_address(from);

                // Only update if this person is in our unnamed set
                if !unnamed_emails.contains(&addr) {
                    continue;
                }

                let person_id = crate::util::person_id_from_email(&addr);
                if db.update_person_name(&person_id, &display_name).is_ok() {
                    details.push(HygieneFixDetail {
                        fix_type: "name_resolved".to_string(),
                        entity_name: Some(display_name.clone()),
                        description: format!("Resolved {}'s name from {}", display_name, addr),
                    });
                    resolved += 1;
                }
            }
        }
    }

    (resolved, details)
}

/// Auto-link people to entities by matching email domain to account names.
///
/// If `schen@acme.com` is a person and there's an account whose name contains
/// "acme", link them via the entity_people junction table.
pub fn auto_link_people_by_domain(db: &ActionDb) -> (usize, Vec<HygieneFixDetail>) {
    let accounts = match db.get_all_accounts() {
        Ok(a) => a,
        Err(_) => return (0, Vec::new()),
    };
    if accounts.is_empty() {
        return (0, Vec::new());
    }

    // Build a domain-hint → (account_id, account_name) map
    // E.g., account "Acme Corp" → hint "acme"
    let account_hints: Vec<(String, String, String)> = accounts
        .iter()
        .map(|a| {
            let hint = a
                .name
                .to_lowercase()
                .split_whitespace()
                .next()
                .unwrap_or("")
                .chars()
                .filter(|c| c.is_alphanumeric())
                .collect::<String>();
            (hint, a.id.clone(), a.name.clone())
        })
        .filter(|(hint, _, _)| hint.len() >= 3)
        .collect();

    if account_hints.is_empty() {
        return (0, Vec::new());
    }

    // Get external people not yet linked to any entity
    let people = match db.get_people(Some("external")) {
        Ok(p) => p,
        Err(_) => return (0, Vec::new()),
    };

    let mut linked = 0;
    let mut details = Vec::new();
    for person in &people {
        // Check if already linked
        let already_linked = db
            .get_entities_for_person(&person.id)
            .map(|e| !e.is_empty())
            .unwrap_or(false);
        if already_linked {
            continue;
        }

        // Extract domain base from email
        let domain = crate::prepare::email_classify::extract_domain(&person.email);
        let domain_base = domain.split('.').next().unwrap_or("").to_lowercase();
        if domain_base.len() < 3 {
            continue;
        }

        // Match against account hints
        for (hint, account_id, account_name) in &account_hints {
            if (&domain_base == hint || (hint.len() >= 4 && domain_base.contains(hint.as_str())))
                && db
                    .link_person_to_entity(&person.id, account_id, "associated")
                    .is_ok()
            {
                details.push(HygieneFixDetail {
                    fix_type: "person_linked_by_domain".to_string(),
                    entity_name: Some(person.name.clone()),
                    description: format!("Linked {} to {} via {}", person.name, account_name, domain),
                });
                linked += 1;
                break; // One link per person
            }
        }
    }

    (linked, details)
}

// =============================================================================
// Duplicate People Detection (I172)
// =============================================================================

/// Merge duplicate people who share the same local part across aliased domains.
///
/// For each account with 2+ domains, groups people by `(local_part, domain_group)`
/// and merges duplicates. Uses existing `merge_people()` to transfer references.
fn dedup_people_by_domain_alias(
    db: &ActionDb,
    user_domains: &[String],
) -> (usize, Vec<HygieneFixDetail>) {
    let people = match db.get_people(None) {
        Ok(p) => p,
        Err(_) => return (0, Vec::new()),
    };
    let active: Vec<_> = people.into_iter().filter(|p| !p.archived).collect();
    if active.is_empty() {
        return (0, Vec::new());
    }

    // Build a map: domain → set of sibling domains (via account_domains + user_domains)
    let mut domain_siblings: HashMap<String, Vec<String>> = HashMap::new();

    for person in &active {
        let domain = crate::prepare::email_classify::extract_domain(&person.email);
        if domain.is_empty() {
            continue;
        }
        if domain_siblings.contains_key(&domain) {
            continue;
        }
        match db.get_sibling_domains_for_email(&person.email, user_domains) {
            Ok(siblings) if !siblings.is_empty() => {
                domain_siblings.insert(domain, siblings);
            }
            _ => {
                domain_siblings.insert(domain, Vec::new());
            }
        }
    }

    // Group people by (local_part, canonical_domain_group).
    // The canonical key is the sorted domain set so that `renan@wpvip.com` and `renan@a8c.com`
    // fall into the same group when those domains are siblings.
    let mut groups: HashMap<(String, String), Vec<&crate::db::DbPerson>> = HashMap::new();

    for person in &active {
        let domain = crate::prepare::email_classify::extract_domain(&person.email);
        let local_part = match person.email.rfind('@') {
            Some(pos) => person.email[..pos].to_lowercase(),
            None => continue,
        };

        // Build canonical domain set: this domain + its siblings, sorted
        let mut domain_set = vec![domain.clone()];
        if let Some(siblings) = domain_siblings.get(&domain) {
            domain_set.extend(siblings.iter().cloned());
        }
        domain_set.sort();
        domain_set.dedup();

        // Only consider domains that have siblings (otherwise no aliasing possible)
        if domain_set.len() < 2 {
            continue;
        }

        let key = (local_part, domain_set.join(","));
        groups.entry(key).or_default().push(person);
    }

    let mut merged = 0;
    let mut details = Vec::new();

    for ((_local_part, _domains), group) in &groups {
        if group.len() < 2 {
            continue;
        }

        // Keep the person with the highest meeting_count; tie-break by earliest first_seen
        let mut sorted: Vec<&&crate::db::DbPerson> = group.iter().collect();
        sorted.sort_by(|a, b| {
            b.meeting_count
                .cmp(&a.meeting_count)
                .then_with(|| a.first_seen.cmp(&b.first_seen))
        });

        let keep = sorted[0];
        for &remove in &sorted[1..] {
            if db.merge_people(&keep.id, &remove.id).is_ok() {
                if details.len() < 5 {
                    details.push(HygieneFixDetail {
                        fix_type: "people_deduped_by_alias".to_string(),
                        entity_name: Some(keep.name.clone()),
                        description: format!(
                            "Merged {} ({}) into {} ({})",
                            remove.name, remove.email, keep.name, keep.email
                        ),
                    });
                }
                merged += 1;
            }
        }
    }

    (merged, details)
}

/// A candidate pair of potentially duplicate people.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateCandidate {
    pub person1_id: String,
    pub person1_name: String,
    pub person2_id: String,
    pub person2_name: String,
    pub confidence: f32,
    pub reason: String,
}

/// Detect potential duplicate people using name similarity within email domain groups.
///
/// Strategy:
/// 1. Query all non-archived people from the database
/// 2. Group people by email domain
/// 3. Within each domain group, compare normalized names pairwise
/// 4. Score based on:
///    - Exact normalized name match (different emails) -> 0.95
///    - Same first name + last name initial match -> 0.7
///    - First 3 chars of both first and last name match -> 0.6
///    - Same last name + same domain -> 0.4
/// 5. Return candidates sorted by confidence descending
///
/// People with the same email are the same person and are NOT flagged.
/// Only compares within the same email domain to keep the comparison set manageable.
pub fn detect_duplicate_people(db: &ActionDb) -> Result<Vec<DuplicateCandidate>, String> {
    // Get all non-archived people
    let people = db
        .get_people(None)
        .map_err(|e| format!("Failed to get people: {e}"))?;

    let active: Vec<_> = people.into_iter().filter(|p| !p.archived).collect();

    // Group by email domain
    let mut domain_groups: HashMap<String, Vec<&crate::db::DbPerson>> = HashMap::new();
    for person in &active {
        let domain = crate::prepare::email_classify::extract_domain(&person.email);
        if domain.is_empty() {
            continue;
        }
        domain_groups.entry(domain).or_default().push(person);
    }

    let mut candidates: Vec<DuplicateCandidate> = Vec::new();

    for (domain, group) in domain_groups.iter() {
        if group.len() < 2 {
            continue;
        }

        if group.len() > MAX_DOMAIN_GROUP_SIZE {
            log::warn!(
                "Skipping duplicate detection for domain {} ({} people exceeds limit of {})",
                domain,
                group.len(),
                MAX_DOMAIN_GROUP_SIZE
            );
            continue;
        }

        // Pairwise comparison within the domain group
        for i in 0..group.len() {
            for j in (i + 1)..group.len() {
                let p1 = group[i];
                let p2 = group[j];

                // Same email = same person, not a duplicate
                if p1.email.to_lowercase() == p2.email.to_lowercase() {
                    continue;
                }

                if let Some((confidence, reason)) = score_name_similarity(&p1.name, &p2.name) {
                    candidates.push(DuplicateCandidate {
                        person1_id: p1.id.clone(),
                        person1_name: p1.name.clone(),
                        person2_id: p2.id.clone(),
                        person2_name: p2.name.clone(),
                        confidence,
                        reason,
                    });
                }
            }
        }
    }

    // Sort by confidence descending
    candidates.sort_by(|a, b| {
        b.confidence
            .partial_cmp(&a.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    Ok(candidates)
}

/// Normalized name parts: (first, last) both lowercase and trimmed.
/// Returns None if the name is empty or clearly not a real name (e.g. just an email).
fn split_name(name: &str) -> Option<(String, String)> {
    let normalized = name.trim().to_lowercase();
    // Skip email-as-name entries
    if normalized.contains('@') {
        return None;
    }

    let parts: Vec<&str> = normalized.split_whitespace().collect();
    match parts.len() {
        0 => None,
        1 => Some((parts[0].to_string(), String::new())),
        _ => {
            let first = parts[0].to_string();
            // Join remaining parts as last name (handles "Van Der Berg" etc.)
            let last = parts[1..].join(" ");
            Some((first, last))
        }
    }
}

/// Score name similarity between two people. Returns (confidence, reason) if
/// the names are similar enough to flag, or None if no match.
///
/// Thresholds:
/// - 0.95: Exact normalized name match (different emails)
/// - 0.70: Same first name + last initial matches
/// - 0.60: First 3 chars of both first and last name match
/// - 0.40: Same last name (at least 3 chars)
fn score_name_similarity(name1: &str, name2: &str) -> Option<(f32, String)> {
    let (first1, last1) = split_name(name1)?;
    let (first2, last2) = split_name(name2)?;

    // Both must have at least a first name
    if first1.is_empty() || first2.is_empty() {
        return None;
    }

    // Exact full-name match
    let full1 = if last1.is_empty() {
        first1.clone()
    } else {
        format!("{first1} {last1}")
    };
    let full2 = if last2.is_empty() {
        first2.clone()
    } else {
        format!("{first2} {last2}")
    };
    if full1 == full2 {
        return Some((0.95, format!("Exact name match: \"{full1}\"")));
    }

    // From here, require both names to have first and last parts
    if last1.is_empty() || last2.is_empty() {
        return None;
    }

    // Same first name + last name initial matches
    if first1 == first2 {
        let last1_initial = last1.chars().next().unwrap_or(' ');
        let last2_initial = last2.chars().next().unwrap_or(' ');
        if last1_initial == last2_initial {
            return Some((
                0.70,
                format!("Same first name \"{first1}\" + last initial '{last1_initial}'"),
            ));
        }
    }

    // First 3 chars of both first and last match (catches typos/abbreviations)
    let prefix_len = 3;
    let f1_prefix: String = first1.chars().take(prefix_len).collect();
    let f2_prefix: String = first2.chars().take(prefix_len).collect();
    let l1_prefix: String = last1.chars().take(prefix_len).collect();
    let l2_prefix: String = last2.chars().take(prefix_len).collect();

    if f1_prefix.len() >= prefix_len
        && l1_prefix.len() >= prefix_len
        && f1_prefix == f2_prefix
        && l1_prefix == l2_prefix
    {
        return Some((
            0.60,
            format!("Similar names: \"{first1} {last1}\" ~ \"{first2} {last2}\""),
        ));
    }

    // Same last name (at least 3 chars) on same domain
    if last1.len() >= 3 && last1 == last2 {
        return Some((0.40, format!("Same last name \"{last1}\" on same domain")));
    }

    None
}

/// Enqueue AI enrichment for entities with missing or stale intelligence.
/// Respects the daily budget. Returns number of enrichments enqueued.
fn enqueue_ai_enrichments(
    db: &ActionDb,
    budget: &crate::state::HygieneBudget,
    queue: &crate::intel_queue::IntelligenceQueue,
) -> usize {
    use crate::intel_queue::{IntelPriority, IntelRequest};
    use std::time::Instant;

    let mut enqueued = 0;

    // First: entities with no intelligence at all (never enriched)
    if let Ok(missing) = db.get_entities_without_intelligence() {
        for (entity_id, entity_type) in missing {
            if !budget.try_consume() {
                log::debug!(
                    "HygieneLoop: AI budget exhausted ({} used)",
                    budget.used_today()
                );
                return enqueued;
            }
            queue.enqueue(IntelRequest {
                entity_id,
                entity_type,
                priority: IntelPriority::ProactiveHygiene,
                requested_at: Instant::now(),
            });
            enqueued += 1;
        }
    }

    // Second: entities with stale intelligence (>14 days with new content)
    if let Ok(stale) = db.get_stale_entity_intelligence(14) {
        for (entity_id, entity_type, _enriched_at) in stale {
            if !budget.try_consume() {
                log::debug!(
                    "HygieneLoop: AI budget exhausted ({} used)",
                    budget.used_today()
                );
                return enqueued;
            }
            queue.enqueue(IntelRequest {
                entity_id,
                entity_type,
                priority: IntelPriority::ProactiveHygiene,
                requested_at: Instant::now(),
            });
            enqueued += 1;
        }
    }

    enqueued
}

// =============================================================================
// Phase 3: Pre-Meeting Intelligence Refresh + Overnight Batch (I147)
// =============================================================================

/// Staleness threshold for pre-meeting refresh (7 days).
const PRE_MEETING_STALE_DAYS: i64 = 7;

/// Hours before a meeting to trigger refresh.
const PRE_MEETING_WINDOW_HOURS: i64 = 2;

/// Default overnight AI budget (higher than daytime).
const OVERNIGHT_AI_BUDGET: u32 = 20;

/// Check upcoming meetings and enqueue intelligence refresh for stale linked entities.
///
/// Called after each calendar poll. Looks for meetings within the configured
/// pre-meeting window (default 12h) with linked entities whose intelligence
/// is older than 7 days.
pub fn check_upcoming_meeting_readiness(
    db: &ActionDb,
    queue: &crate::intel_queue::IntelligenceQueue,
    config: Option<&Config>,
) -> Vec<String> {
    use crate::intel_queue::{IntelPriority, IntelRequest};
    use std::time::Instant;

    let window_hours = config
        .map(|c| c.hygiene_pre_meeting_hours as i64)
        .unwrap_or(PRE_MEETING_WINDOW_HOURS);
    let window_end = Utc::now() + chrono::Duration::hours(window_hours);
    let stale_threshold = Utc::now() - chrono::Duration::days(PRE_MEETING_STALE_DAYS);
    let stale_str = stale_threshold.to_rfc3339();

    // Find meetings in the next window
    let upcoming: Vec<crate::db::DbMeeting> = db
        .conn_ref()
        .prepare(
            "SELECT id, title, meeting_type, start_time, end_time,
                    attendees, notes_path, summary,
                    created_at, calendar_event_id
             FROM meetings_history
             WHERE start_time > datetime('now')
               AND start_time <= ?1
             ORDER BY start_time ASC",
        )
        .and_then(|mut stmt| {
            let rows = stmt.query_map(rusqlite::params![window_end.to_rfc3339()], |row| {
                Ok(crate::db::DbMeeting {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    meeting_type: row.get(2)?,
                    start_time: row.get(3)?,
                    end_time: row.get(4)?,
                    attendees: row.get(5)?,
                    notes_path: row.get(6)?,
                    summary: row.get(7)?,
                    created_at: row.get(8)?,
                    calendar_event_id: row.get(9)?,
                    description: None,
                    prep_context_json: None,
                    user_agenda_json: None,
                    user_notes: None,
                    prep_frozen_json: None,
                    prep_frozen_at: None,
                    prep_snapshot_path: None,
                    prep_snapshot_hash: None,
                    transcript_path: None,
                    transcript_processed_at: None,
                })
            })?;
            Ok(rows.filter_map(|r| r.ok()).collect())
        })
        .unwrap_or_default();

    let mut enqueued_ids = Vec::new();

    for meeting in &upcoming {
        // Get linked entities via junction table
        let entities = match db.get_meeting_entities(&meeting.id) {
            Ok(e) => e,
            Err(_) => continue,
        };

        for entity in &entities {
            // Check if intelligence is stale
            let is_stale = match db.get_entity_intelligence(&entity.id) {
                Ok(Some(intel)) => intel.enriched_at < stale_str,
                Ok(None) => true, // Never enriched
                Err(_) => continue,
            };

            if is_stale {
                let entity_type = format!("{:?}", entity.entity_type).to_lowercase();
                queue.enqueue(IntelRequest {
                    entity_id: entity.id.clone(),
                    entity_type,
                    priority: IntelPriority::CalendarChange,
                    requested_at: Instant::now(),
                });
                enqueued_ids.push(entity.id.clone());
            }
        }
    }

    if !enqueued_ids.is_empty() {
        log::info!(
            "PreMeetingScan: enqueued {} entity refreshes for upcoming meetings",
            enqueued_ids.len()
        );
    }

    enqueued_ids
}

/// Overnight maintenance report.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OvernightReport {
    pub ran_at: String,
    pub entities_refreshed: usize,
    pub names_resolved: usize,
    pub summaries_extracted: usize,
    pub relationships_reclassified: usize,
}

/// Run an expanded overnight scan with higher AI budget.
/// Writes maintenance.json for the morning briefing to reference.
pub fn run_overnight_scan(
    db: &ActionDb,
    config: &Config,
    workspace: &Path,
    queue: &crate::intel_queue::IntelligenceQueue,
) -> OvernightReport {
    // Use expanded overnight budget (2x daytime from config)
    let overnight_limit = config.hygiene_ai_budget.saturating_mul(2).max(OVERNIGHT_AI_BUDGET);
    let overnight_budget = crate::state::HygieneBudget::new(overnight_limit);

    let report = run_hygiene_scan(db, config, workspace, Some(&overnight_budget), Some(queue), false, None);

    let overnight = OvernightReport {
        ran_at: Utc::now().to_rfc3339(),
        entities_refreshed: report.fixes.ai_enrichments_enqueued,
        names_resolved: report.fixes.names_resolved,
        summaries_extracted: report.fixes.summaries_extracted,
        relationships_reclassified: report.fixes.relationships_reclassified,
    };

    // Write maintenance.json for morning briefing
    let maintenance_path = workspace
        .join("_today")
        .join("data")
        .join("maintenance.json");
    if let Ok(json) = serde_json::to_string_pretty(&overnight) {
        let _ = crate::util::atomic_write_str(&maintenance_path, &json);
    }

    overnight
}

/// Check if current time is in the overnight window (2-3 AM local time).
fn is_overnight_window() -> bool {
    use chrono::Local;
    let hour = Local::now().format("%H").to_string().parse::<u32>().unwrap_or(12);
    (2..=3).contains(&hour)
}

/// Background loop: runs scan on startup (30s delay), then every 4 hours.
pub async fn run_hygiene_loop(state: Arc<AppState>, _app: AppHandle) {
    // Wait for startup to complete
    tokio::time::sleep(std::time::Duration::from_secs(STARTUP_DELAY_SECS)).await;

    log::info!("HygieneLoop: started");

    loop {
        // Read config-driven interval each iteration (changes take effect next cycle)
        let interval = state
            .config
            .read()
            .ok()
            .and_then(|g| g.as_ref().map(|c| c.hygiene_scan_interval_hours as u64 * 3600))
            .unwrap_or(SCAN_INTERVAL_SECS);

        // Prevent overlap with manual scan runs.
        let began_scan = state
            .hygiene_scan_running
            .compare_exchange(
                false,
                true,
                std::sync::atomic::Ordering::AcqRel,
                std::sync::atomic::Ordering::Acquire,
            )
            .is_ok();

        if !began_scan {
            log::debug!("HygieneLoop: skipping scan (another hygiene scan is already running)");
            tokio::time::sleep(std::time::Duration::from_secs(interval)).await;
            continue;
        }

        // Check for overnight window — use expanded scan with higher AI budget
        if is_overnight_window() {
            let overnight = try_run_overnight(&state);
            if let Some(report) = overnight {
                log::info!(
                    "HygieneLoop: overnight scan — {} entities refreshed, {} names resolved",
                    report.entities_refreshed,
                    report.names_resolved,
                );
            }
        }

        // Run regular scan synchronously (all locks drop before the next await)
        let report = try_run_scan(&state);

        if let Some(report) = report {
            let total_gaps = report.unnamed_people
                + report.unknown_relationships
                + report.missing_intelligence
                + report.stale_intelligence
                + report.unsummarized_files;

            let total_fixes = report.fixes.relationships_reclassified
                + report.fixes.summaries_extracted
                + report.fixes.meeting_counts_updated
                + report.fixes.names_resolved
                + report.fixes.people_linked_by_domain
                + report.fixes.renewals_rolled_over
                + report.fixes.ai_enrichments_enqueued;

            if total_gaps > 0 || total_fixes > 0 {
                log::info!(
                    "HygieneLoop: {} gaps detected, {} fixes applied \
                     (relationships={}, summaries={}, counts={}, \
                     names={}, domain_links={}, renewals={}, ai_enqueued={})",
                    total_gaps,
                    total_fixes,
                    report.fixes.relationships_reclassified,
                    report.fixes.summaries_extracted,
                    report.fixes.meeting_counts_updated,
                    report.fixes.names_resolved,
                    report.fixes.people_linked_by_domain,
                    report.fixes.renewals_rolled_over,
                    report.fixes.ai_enrichments_enqueued,
                );
            } else {
                log::debug!("HygieneLoop: clean — no gaps detected");
            }

            // Store report for frontend access (Phase 4)
            if let Ok(mut guard) = state.last_hygiene_report.lock() {
                *guard = Some(report);
            }
            if let Ok(mut guard) = state.last_hygiene_scan_at.lock() {
                *guard = Some(Utc::now().to_rfc3339());
            }
        }

        // Run proactive detection scan after hygiene fixes (I260)
        match crate::proactive::scanner::run_proactive_scan(&state) {
            Ok(n) if n > 0 => log::info!("HygieneLoop: {} proactive insights detected", n),
            Err(e) => log::warn!("HygieneLoop: proactive scan failed: {}", e),
            _ => {}
        }

        // Prune old audit trail files (I297)
        if let Some(config) = state.config.read().ok().and_then(|g| g.clone()) {
            let workspace = std::path::Path::new(&config.workspace_path);
            let pruned = crate::audit::prune_audit_files(workspace);
            if pruned > 0 {
                log::info!("HygieneLoop: pruned {} old audit files", pruned);
            }
        }

        if let Ok(mut guard) = state.next_hygiene_scan_at.lock() {
            *guard = Some(
                (Utc::now() + chrono::Duration::seconds(interval as i64)).to_rfc3339(),
            );
        }
        state
            .hygiene_scan_running
            .store(false, std::sync::atomic::Ordering::Release);

        tokio::time::sleep(std::time::Duration::from_secs(interval)).await;
    }
}

/// Run overnight scan with expanded budget.
/// Opens own DB connection to avoid holding state.db Mutex during scan.
fn try_run_overnight(state: &AppState) -> Option<OvernightReport> {
    let config = state.config.read().ok()?.clone()?;

    let db = crate::db::ActionDb::open().ok()?;

    let workspace = std::path::Path::new(&config.workspace_path);
    Some(run_overnight_scan(
        &db,
        &config,
        workspace,
        &state.intel_queue,
    ))
}

/// Synchronous scan attempt — releases everything when done.
/// Opens own DB connection to avoid holding state.db Mutex during scan.
fn try_run_scan(state: &AppState) -> Option<HygieneReport> {
    let config = state.config.read().ok()?.clone()?;

    let db = crate::db::ActionDb::open().ok()?;

    // First run does a full orphan scan (no lookback limit)
    let first_run = !state
        .hygiene_full_orphan_scan_done
        .swap(true, std::sync::atomic::Ordering::AcqRel);

    let workspace = std::path::Path::new(&config.workspace_path);
    Some(run_hygiene_scan(
        &db,
        &config,
        workspace,
        Some(&state.hygiene_budget),
        Some(&state.intel_queue),
        first_run,
        Some(state.embedding_model.as_ref()),
    ))
}

// =============================================================================
// Hygiene Status View Model (consumed by Tauri commands)
// =============================================================================

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HygieneFixView {
    pub key: String,
    pub label: String,
    pub count: usize,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HygieneGapActionView {
    pub kind: String,
    pub label: String,
    pub route: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HygieneGapView {
    pub key: String,
    pub label: String,
    pub count: usize,
    pub impact: String,
    pub description: String,
    pub action: HygieneGapActionView,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HygieneBudgetView {
    pub used_today: u32,
    pub daily_limit: u32,
    pub queued_for_next_budget: usize,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HygieneStatusView {
    pub status: String,
    pub status_label: String,
    pub last_scan_time: Option<String>,
    pub next_scan_time: Option<String>,
    pub total_gaps: usize,
    pub total_fixes: usize,
    pub is_running: bool,
    pub fixes: Vec<HygieneFixView>,
    pub fix_details: Vec<HygieneFixDetail>,
    pub gaps: Vec<HygieneGapView>,
    pub budget: HygieneBudgetView,
    pub scan_duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HygieneNarrativeView {
    pub narrative: String,
    pub remaining_gaps: Vec<HygieneGapSummary>,
    pub last_scan_time: Option<String>,
    pub total_fixes: usize,
    pub total_remaining_gaps: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HygieneGapSummary {
    pub label: String,
    pub count: usize,
    pub severity: String, // "critical" | "medium" | "low"
}

/// Join items into prose: "a, b, and c"
fn join_prose_list(items: &[String]) -> String {
    match items.len() {
        0 => String::new(),
        1 => items[0].clone(),
        2 => format!("{} and {}", items[0], items[1]),
        _ => {
            let (last, rest) = items.split_last().unwrap();
            format!("{}, and {}", rest.join(", "), last)
        }
    }
}

pub fn build_hygiene_narrative(report: &HygieneReport) -> Option<HygieneNarrativeView> {
    // Build fix descriptions
    let mut fix_parts: Vec<String> = Vec::new();
    let fixes = &report.fixes;
    if fixes.names_resolved > 0 {
        fix_parts.push(format!(
            "resolved {} unnamed {}",
            fixes.names_resolved,
            if fixes.names_resolved == 1 { "person" } else { "people" }
        ));
    }
    if fixes.relationships_reclassified > 0 {
        fix_parts.push(format!(
            "reclassified {} {}",
            fixes.relationships_reclassified,
            if fixes.relationships_reclassified == 1 { "relationship" } else { "relationships" }
        ));
    }
    if fixes.summaries_extracted > 0 {
        fix_parts.push(format!(
            "extracted {} {}",
            fixes.summaries_extracted,
            if fixes.summaries_extracted == 1 { "summary" } else { "summaries" }
        ));
    }
    if fixes.meeting_counts_updated > 0 {
        fix_parts.push(format!(
            "updated {} meeting {}",
            fixes.meeting_counts_updated,
            if fixes.meeting_counts_updated == 1 { "count" } else { "counts" }
        ));
    }
    if fixes.people_linked_by_domain > 0 {
        fix_parts.push(format!(
            "linked {} {} by domain",
            fixes.people_linked_by_domain,
            if fixes.people_linked_by_domain == 1 { "person" } else { "people" }
        ));
    }
    if fixes.renewals_rolled_over > 0 {
        fix_parts.push(format!(
            "rolled over {} {}",
            fixes.renewals_rolled_over,
            if fixes.renewals_rolled_over == 1 { "renewal" } else { "renewals" }
        ));
    }
    if fixes.ai_enrichments_enqueued > 0 {
        fix_parts.push(format!(
            "queued {} intelligence {}",
            fixes.ai_enrichments_enqueued,
            if fixes.ai_enrichments_enqueued == 1 { "refresh" } else { "refreshes" }
        ));
    }

    // Build gap summaries
    let mut remaining_gaps: Vec<HygieneGapSummary> = Vec::new();
    let gap_rows: Vec<(&str, usize, &str)> = vec![
        ("unnamed people", report.unnamed_people, "critical"),
        ("duplicate people", report.duplicate_people, "critical"),
        ("unknown relationships", report.unknown_relationships, "medium"),
        ("missing intelligence", report.missing_intelligence, "medium"),
        ("stale intelligence", report.stale_intelligence, "low"),
        ("unsummarized files", report.unsummarized_files, "medium"),
    ];
    for (label, count, severity) in &gap_rows {
        if *count > 0 {
            remaining_gaps.push(HygieneGapSummary {
                label: format!("{} {}", count, label),
                count: *count,
                severity: severity.to_string(),
            });
        }
    }

    let total_fix_count = fixes.names_resolved
        + fixes.relationships_reclassified
        + fixes.summaries_extracted
        + fixes.meeting_counts_updated
        + fixes.people_linked_by_domain
        + fixes.renewals_rolled_over
        + fixes.ai_enrichments_enqueued;
    let total_remaining_gaps: usize = remaining_gaps.iter().map(|g| g.count).sum();

    // Return None when nothing happened
    if total_fix_count == 0 && total_remaining_gaps == 0 {
        return None;
    }

    // Build narrative prose
    let mut narrative = String::new();
    if !fix_parts.is_empty() {
        let capitalized = {
            let joined = join_prose_list(&fix_parts);
            let mut chars = joined.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => c.to_uppercase().to_string() + chars.as_str(),
            }
        };
        narrative.push_str(&format!("{}.", capitalized));
    }
    if total_remaining_gaps > 0 {
        if !narrative.is_empty() {
            narrative.push(' ');
        }
        narrative.push_str(&format!(
            "{} {} remaining.",
            total_remaining_gaps,
            if total_remaining_gaps == 1 { "gap" } else { "gaps" }
        ));
    } else if !narrative.is_empty() {
        narrative.push_str(" All clear.");
    }

    Some(HygieneNarrativeView {
        narrative,
        remaining_gaps,
        last_scan_time: Some(report.scanned_at.clone()),
        total_fixes: total_fix_count,
        total_remaining_gaps,
    })
}

fn hygiene_gap_action(key: &str) -> HygieneGapActionView {
    match key {
        "unnamed_people" => HygieneGapActionView {
            kind: "navigate".to_string(),
            label: "View People".to_string(),
            route: Some("/people?hygiene=unnamed".to_string()),
        },
        "unknown_relationships" => HygieneGapActionView {
            kind: "navigate".to_string(),
            label: "Review Relationships".to_string(),
            route: Some("/people?relationship=unknown".to_string()),
        },
        "duplicate_people" => HygieneGapActionView {
            kind: "navigate".to_string(),
            label: "Review Duplicates".to_string(),
            route: Some("/people?hygiene=duplicates".to_string()),
        },
        _ => HygieneGapActionView {
            kind: "run_scan_now".to_string(),
            label: "Run Hygiene Scan Now".to_string(),
            route: None,
        },
    }
}

/// Build the hygiene status view model from app state and an optional report.
pub fn build_intelligence_hygiene_status(
    state: &AppState,
    report: Option<&HygieneReport>,
) -> HygieneStatusView {
    let unnamed_people = report.map(|r| r.unnamed_people).unwrap_or(0);
    let unknown_relationships = report.map(|r| r.unknown_relationships).unwrap_or(0);
    let missing_intelligence = report.map(|r| r.missing_intelligence).unwrap_or(0);
    let stale_intelligence = report.map(|r| r.stale_intelligence).unwrap_or(0);
    let unsummarized_files = report.map(|r| r.unsummarized_files).unwrap_or(0);
    let duplicate_people = report.map(|r| r.duplicate_people).unwrap_or(0);

    let fixes = report
        .map(|r| {
            vec![
                HygieneFixView {
                    key: "relationships_reclassified".to_string(),
                    label: "Relationships reclassified".to_string(),
                    count: r.fixes.relationships_reclassified,
                },
                HygieneFixView {
                    key: "summaries_extracted".to_string(),
                    label: "Summaries extracted".to_string(),
                    count: r.fixes.summaries_extracted,
                },
                HygieneFixView {
                    key: "meeting_counts_updated".to_string(),
                    label: "Meeting counts updated".to_string(),
                    count: r.fixes.meeting_counts_updated,
                },
                HygieneFixView {
                    key: "names_resolved".to_string(),
                    label: "Names resolved".to_string(),
                    count: r.fixes.names_resolved,
                },
                HygieneFixView {
                    key: "people_linked_by_domain".to_string(),
                    label: "People linked by domain".to_string(),
                    count: r.fixes.people_linked_by_domain,
                },
                HygieneFixView {
                    key: "renewals_rolled_over".to_string(),
                    label: "Renewals rolled over".to_string(),
                    count: r.fixes.renewals_rolled_over,
                },
                HygieneFixView {
                    key: "ai_enrichments_enqueued".to_string(),
                    label: "AI enrichments enqueued".to_string(),
                    count: r.fixes.ai_enrichments_enqueued,
                },
            ]
        })
        .unwrap_or_default()
        .into_iter()
        .filter(|fix| fix.count > 0)
        .collect::<Vec<_>>();

    let mut gaps = Vec::new();
    let gap_rows = vec![
        (
            "unnamed_people",
            "Unnamed people",
            unnamed_people,
            "critical",
            "Missing names make prep less personal.",
        ),
        (
            "unknown_relationships",
            "Unknown relationships",
            unknown_relationships,
            "medium",
            "Unknown relationships reduce meeting classification accuracy.",
        ),
        (
            "duplicate_people",
            "Duplicate people",
            duplicate_people,
            "critical",
            "Duplicate records fragment context and meeting history.",
        ),
        (
            "missing_intelligence",
            "Missing intelligence",
            missing_intelligence,
            "medium",
            "Entities without intelligence produce sparse prep.",
        ),
        (
            "stale_intelligence",
            "Stale intelligence",
            stale_intelligence,
            "low",
            "Older intelligence can miss recent customer signals.",
        ),
        (
            "unsummarized_files",
            "Unsummarized files",
            unsummarized_files,
            "medium",
            "Summaries speed up context retrieval during prep.",
        ),
    ];

    for (key, label, count, impact, description) in gap_rows {
        if count == 0 {
            continue;
        }
        gaps.push(HygieneGapView {
            key: key.to_string(),
            label: label.to_string(),
            count,
            impact: impact.to_string(),
            description: description.to_string(),
            action: hygiene_gap_action(key),
        });
    }

    let total_gaps = unnamed_people
        + unknown_relationships
        + missing_intelligence
        + stale_intelligence
        + unsummarized_files
        + duplicate_people;

    let total_fixes = report
        .map(|r| {
            r.fixes.relationships_reclassified
                + r.fixes.summaries_extracted
                + r.fixes.meeting_counts_updated
                + r.fixes.names_resolved
                + r.fixes.people_linked_by_domain
                + r.fixes.renewals_rolled_over
                + r.fixes.ai_enrichments_enqueued
        })
        .unwrap_or(0);

    let is_running = state
        .hygiene_scan_running
        .load(std::sync::atomic::Ordering::Acquire);

    let (status, status_label) = if is_running {
        ("running".to_string(), "Running".to_string())
    } else if total_gaps == 0 {
        ("healthy".to_string(), "Healthy".to_string())
    } else {
        ("needs_attention".to_string(), "Needs Attention".to_string())
    };

    let queued_for_next_budget = report
        .map(|r| {
            (r.missing_intelligence + r.stale_intelligence)
                .saturating_sub(r.fixes.ai_enrichments_enqueued)
        })
        .unwrap_or(0);

    let last_scan_time = state
        .last_hygiene_scan_at
        .lock()
        .ok()
        .and_then(|g| g.clone())
        .or_else(|| report.map(|r| r.scanned_at.clone()));
    let next_scan_time = state
        .next_hygiene_scan_at
        .lock()
        .ok()
        .and_then(|g| g.clone());

    let fix_details = report
        .map(|r| r.fix_details.clone())
        .unwrap_or_default();

    HygieneStatusView {
        status,
        status_label,
        last_scan_time,
        next_scan_time,
        total_gaps,
        total_fixes,
        is_running,
        fixes,
        fix_details,
        gaps,
        budget: HygieneBudgetView {
            used_today: state.hygiene_budget.used_today(),
            daily_limit: state.hygiene_budget.daily_limit,
            queued_for_next_budget,
        },
        scan_duration_ms: report.map(|r| r.scan_duration_ms),
    }
}

// =============================================================================
// Low-confidence entity match detection (I305)
// =============================================================================

/// Detect meetings from the last 14 days with no entity links that could
/// be matched to entities with low confidence (0.30–0.60 = suggestion).
/// Returns up to 10 suggestions as HygieneFixDetail entries.
fn detect_low_confidence_matches(
    db: &ActionDb,
    accounts_dir: &std::path::Path,
    embedding_model: Option<&crate::embeddings::EmbeddingModel>,
) -> (usize, Vec<HygieneFixDetail>) {
    use crate::prepare::entity_resolver::{resolve_meeting_entities, ResolutionOutcome};

    let lookback = (Utc::now() - chrono::Duration::days(14)).to_rfc3339();
    let meetings = db.get_unlinked_meetings(&lookback, 50).unwrap_or_default();

    let mut suggestions = Vec::new();
    let max_suggestions = 10;

    for (_meeting_id, title, calendar_event_id, _start_time) in meetings {
        if suggestions.len() >= max_suggestions {
            break;
        }

        let event_id = calendar_event_id.as_deref().unwrap_or("");
        let meeting_json = serde_json::json!({
            "title": title,
            "id": event_id,
        });

        let outcomes =
            resolve_meeting_entities(db, event_id, &meeting_json, accounts_dir, embedding_model);

        for outcome in &outcomes {
            if let ResolutionOutcome::Suggestion(entity) = outcome {
                suggestions.push(HygieneFixDetail {
                    fix_type: "low_confidence_entity_match".to_string(),
                    entity_name: Some(entity.entity_id.clone()),
                    description: format!(
                        "Meeting \"{}\" may be related to entity (confidence: {:.0}%, source: {})",
                        title,
                        entity.confidence * 100.0,
                        entity.source,
                    ),
                });

                // I306: Emit low_confidence_match signal to bus
                let value = serde_json::json!({
                    "meeting_title": title,
                    "source": entity.source,
                })
                .to_string();
                let _ = crate::signals::bus::emit_signal(
                    db,
                    entity.entity_type.as_str(),
                    &entity.entity_id,
                    "low_confidence_match",
                    "heuristic",
                    Some(&value),
                    entity.confidence,
                );

                if suggestions.len() >= max_suggestions {
                    break;
                }
            }
        }
    }

    let count = suggestions.len();
    (count, suggestions)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn test_db() -> ActionDb {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("test.db");
        std::mem::forget(dir);
        ActionDb::open_at(path).expect("open")
    }

    fn seed_person(db: &ActionDb, id: &str, email: &str, name: &str, relationship: &str) {
        let now = Utc::now().to_rfc3339();
        let person = crate::db::DbPerson {
            id: id.to_string(),
            email: email.to_string(),
            name: name.to_string(),
            organization: None,
            role: None,
            relationship: relationship.to_string(),
            notes: None,
            tracker_path: None,
            last_seen: None,
            first_seen: Some(now.clone()),
            meeting_count: 0,
            updated_at: now,
            archived: false,
            linkedin_url: None,
            twitter_handle: None,
            phone: None,
            photo_url: None,
            bio: None,
            title_history: None,
            company_industry: None,
            company_size: None,
            company_hq: None,
            last_enriched_at: None,
            enrichment_sources: None,
        };
        db.upsert_person(&person).expect("upsert person");
    }

    fn seed_account(db: &ActionDb, id: &str, name: &str) {
        let now = Utc::now().to_rfc3339();
        let account = crate::db::DbAccount {
            id: id.to_string(),
            name: name.to_string(),
            lifecycle: None,
            arr: None,
            health: None,
            contract_start: None,
            contract_end: None,
            nps: None,
            tracker_path: None,
            parent_id: None,
            is_internal: false,
            updated_at: now,
            archived: false,
            keywords: None,
            keywords_extracted_at: None,
        };
        db.upsert_account(&account).expect("upsert account");
    }

    // --- Gap detection tests ---

    #[test]
    fn test_get_unnamed_people_single_word() {
        let db = test_db();
        seed_person(&db, "jdoe", "jdoe@acme.com", "Jdoe", "external");
        seed_person(
            &db,
            "sarah-chen",
            "sarah@acme.com",
            "Sarah Chen",
            "external",
        );

        let unnamed = db.get_unnamed_people().unwrap();
        assert_eq!(unnamed.len(), 1);
        assert_eq!(unnamed[0].id, "jdoe");
    }

    #[test]
    fn test_get_unnamed_people_email_as_name() {
        let db = test_db();
        seed_person(&db, "raw-email", "raw@test.com", "raw@test.com", "external");

        let unnamed = db.get_unnamed_people().unwrap();
        assert_eq!(unnamed.len(), 1);
        assert_eq!(unnamed[0].name, "raw@test.com");
    }

    #[test]
    fn test_get_unknown_relationship_people() {
        let db = test_db();
        seed_person(&db, "p1", "a@test.com", "A", "unknown");
        seed_person(&db, "p2", "b@test.com", "B", "internal");
        seed_person(&db, "p3", "c@test.com", "C", "unknown");

        let unknown = db.get_unknown_relationship_people().unwrap();
        assert_eq!(unknown.len(), 2);
    }

    #[test]
    fn test_get_entities_without_intelligence_empty() {
        let db = test_db();
        let result = db.get_entities_without_intelligence().unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_get_unsummarized_content_files_empty() {
        let db = test_db();
        let result = db.get_unsummarized_content_files().unwrap();
        assert!(result.is_empty());
    }

    // --- Mechanical fix tests ---

    #[test]
    fn test_fix_unknown_relationships_reclassifies() {
        let db = test_db();
        seed_person(&db, "p1", "me@myco.com", "Me", "unknown");
        seed_person(&db, "p2", "them@other.com", "Them", "unknown");

        let domains = vec!["myco.com".to_string()];
        let (fixed, _) = fix_unknown_relationships(&db, &domains);
        assert_eq!(fixed, 2);

        let p1 = db.get_person("p1").unwrap().unwrap();
        assert_eq!(p1.relationship, "internal");

        let p2 = db.get_person("p2").unwrap().unwrap();
        assert_eq!(p2.relationship, "external");
    }

    #[test]
    fn test_fix_unknown_relationships_no_domain() {
        let db = test_db();
        seed_person(&db, "p1", "me@myco.com", "Me", "unknown");

        let (fixed, _) = fix_unknown_relationships(&db, &[]);
        assert_eq!(fixed, 0);
    }

    #[test]
    fn test_fix_unknown_relationships_idempotent() {
        let db = test_db();
        seed_person(&db, "p1", "me@myco.com", "Me", "unknown");

        let domains = vec!["myco.com".to_string()];
        let _ = fix_unknown_relationships(&db, &domains);
        // Second run: person is now "internal", not "unknown", so shouldn't be re-processed
        let (fixed, _) = fix_unknown_relationships(&db, &domains);
        assert_eq!(fixed, 0);
    }

    #[test]
    fn test_fix_meeting_counts() {
        let db = test_db();
        seed_person(&db, "p1", "a@test.com", "A Test", "external");

        // Manually set a wrong meeting count
        db.conn_ref()
            .execute("UPDATE people SET meeting_count = 99 WHERE id = 'p1'", [])
            .unwrap();

        let (fixed, _) = fix_meeting_counts(&db);
        assert_eq!(fixed, 1);

        let person = db.get_person("p1").unwrap().unwrap();
        assert_eq!(person.meeting_count, 0); // No actual attendee records
    }

    #[test]
    fn test_fix_meeting_counts_idempotent() {
        let db = test_db();
        seed_person(&db, "p1", "a@test.com", "A Test", "external");

        // Count is already correct (0 meetings, 0 count)
        let (fixed, _) = fix_meeting_counts(&db);
        assert_eq!(fixed, 0);
    }

    #[test]
    fn test_full_scan_empty_db() {
        let db = test_db();
        let config = Config {
            workspace_path: "/tmp/nonexistent".to_string(),
            user_domain: Some("myco.com".to_string()),
            ..default_test_config()
        };

        let report = run_hygiene_scan(&db, &config, Path::new("/tmp/nonexistent"), None, None, false, None);

        assert_eq!(report.unnamed_people, 0);
        assert_eq!(report.unknown_relationships, 0);
        assert_eq!(report.missing_intelligence, 0);
        assert_eq!(report.stale_intelligence, 0);
        assert_eq!(report.unsummarized_files, 0);
        assert!(!report.scanned_at.is_empty());
    }

    #[test]
    fn test_full_scan_detects_and_fixes() {
        let db = test_db();
        seed_person(&db, "p1", "me@myco.com", "Me", "unknown");
        seed_person(&db, "p2", "them@other.com", "Them", "unknown");

        let config = Config {
            workspace_path: "/tmp/nonexistent".to_string(),
            user_domain: Some("myco.com".to_string()),
            ..default_test_config()
        };

        let report = run_hygiene_scan(&db, &config, Path::new("/tmp/nonexistent"), None, None, false, None);

        // Fixes applied
        assert_eq!(report.fixes.relationships_reclassified, 2);

        // Post-fix gap count: both resolved, so 0 remaining
        assert_eq!(report.unknown_relationships, 0);

        // Verify actual state
        let p1 = db.get_person("p1").unwrap().unwrap();
        assert_eq!(p1.relationship, "internal");
    }

    // --- Phase 2: Email name resolution tests ---

    #[test]
    fn test_resolve_names_from_emails() {
        let db = test_db();
        seed_person(
            &db,
            "jane-customer-com",
            "jane@customer.com",
            "Jane",
            "external",
        );

        // Create workspace with emails.json
        let workspace = tempfile::tempdir().unwrap();
        let data_dir = workspace.path().join("_today").join("data");
        std::fs::create_dir_all(&data_dir).unwrap();
        std::fs::write(
            data_dir.join("emails.json"),
            r#"{"highPriority": [{"from": "Jane Doe <jane@customer.com>", "subject": "Hi"}]}"#,
        )
        .unwrap();

        let (resolved, _) = resolve_names_from_emails(&db, workspace.path());
        assert_eq!(resolved, 1);

        let person = db.get_person("jane-customer-com").unwrap().unwrap();
        assert_eq!(person.name, "Jane Doe");
    }

    #[test]
    fn test_resolve_names_no_emails_file() {
        let db = test_db();
        let (resolved, _) = resolve_names_from_emails(&db, Path::new("/nonexistent"));
        assert_eq!(resolved, 0);
    }

    #[test]
    fn test_resolve_names_skips_already_named() {
        let db = test_db();
        seed_person(
            &db,
            "jane-customer-com",
            "jane@customer.com",
            "Jane Doe",
            "external",
        );

        let workspace = tempfile::tempdir().unwrap();
        let data_dir = workspace.path().join("_today").join("data");
        std::fs::create_dir_all(&data_dir).unwrap();
        std::fs::write(
            data_dir.join("emails.json"),
            r#"{"highPriority": [{"from": "Jane Doe <jane@customer.com>", "subject": "Hi"}]}"#,
        )
        .unwrap();

        // "Jane Doe" has spaces so not in unnamed set
        let (resolved, _) = resolve_names_from_emails(&db, workspace.path());
        assert_eq!(resolved, 0);
    }

    #[test]
    fn test_auto_link_people_by_domain() {
        let db = test_db();
        seed_account(&db, "acme-corp", "Acme Corp");
        seed_person(
            &db,
            "jane-acme-com",
            "jane@acme.com",
            "Jane Doe",
            "external",
        );

        let (linked, _) = auto_link_people_by_domain(&db);
        assert_eq!(linked, 1);

        let entities = db.get_entities_for_person("jane-acme-com").unwrap();
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].id, "acme-corp");
    }

    #[test]
    fn test_auto_link_people_idempotent() {
        let db = test_db();
        seed_account(&db, "acme-corp", "Acme Corp");
        seed_person(
            &db,
            "jane-acme-com",
            "jane@acme.com",
            "Jane Doe",
            "external",
        );

        let _ = auto_link_people_by_domain(&db);
        let (linked, _) = auto_link_people_by_domain(&db);
        assert_eq!(linked, 0); // Already linked
    }

    #[test]
    fn test_auto_link_skips_internal() {
        let db = test_db();
        seed_account(&db, "acme-corp", "Acme Corp");
        seed_person(&db, "me-acme-com", "me@acme.com", "Me", "internal");

        let (linked, _) = auto_link_people_by_domain(&db);
        assert_eq!(linked, 0); // Internal people not auto-linked
    }

    // --- Phase 2: Budget tests ---

    #[test]
    fn test_hygiene_budget_try_consume() {
        let budget = crate::state::HygieneBudget::new(3);
        assert!(budget.try_consume());
        assert!(budget.try_consume());
        assert!(budget.try_consume());
        assert!(!budget.try_consume()); // Exhausted
        assert_eq!(budget.used_today(), 3);
    }

    #[test]
    fn test_enqueue_ai_enrichments_respects_budget() {
        let db = test_db();
        let budget = crate::state::HygieneBudget::new(0); // Zero budget
        let queue = crate::intel_queue::IntelligenceQueue::new();

        let enqueued = enqueue_ai_enrichments(&db, &budget, &queue);
        assert_eq!(enqueued, 0);
    }

    // --- Phase 3: Pre-meeting + overnight tests ---

    fn seed_entity(db: &ActionDb, id: &str, name: &str, entity_type: &str) {
        let now = Utc::now().to_rfc3339();
        db.conn_ref()
            .execute(
                "INSERT OR REPLACE INTO entities (id, name, entity_type, updated_at)
                 VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![id, name, entity_type, now],
            )
            .unwrap();
    }

    fn seed_upcoming_meeting(db: &ActionDb, meeting_id: &str, hours_from_now: i64) {
        let start = Utc::now() + chrono::Duration::hours(hours_from_now);
        db.conn_ref()
            .execute(
                "INSERT INTO meetings_history (id, title, meeting_type, start_time, created_at)
                 VALUES (?1, 'Test Meeting', 'customer', ?2, ?2)",
                rusqlite::params![meeting_id, start.to_rfc3339()],
            )
            .unwrap();
    }

    fn link_meeting_entity(db: &ActionDb, meeting_id: &str, entity_id: &str) {
        db.conn_ref()
            .execute(
                "INSERT INTO meeting_entities (meeting_id, entity_id, entity_type)
                 VALUES (?1, ?2, 'account')",
                rusqlite::params![meeting_id, entity_id],
            )
            .unwrap();
    }

    fn seed_entity_intelligence(db: &ActionDb, entity_id: &str, enriched_at: &str) {
        db.conn_ref()
            .execute(
                "INSERT OR REPLACE INTO entity_intelligence (entity_id, entity_type, enriched_at)
                 VALUES (?1, 'account', ?2)",
                rusqlite::params![entity_id, enriched_at],
            )
            .unwrap();
    }

    #[test]
    fn test_pre_meeting_check_enqueues_stale_entity() {
        let db = test_db();
        let queue = crate::intel_queue::IntelligenceQueue::new();

        // Entity with stale intelligence (30 days ago)
        seed_entity(&db, "acme", "Acme Corp", "account");
        let stale_time = (Utc::now() - chrono::Duration::days(30)).to_rfc3339();
        seed_entity_intelligence(&db, "acme", &stale_time);

        // Meeting in 1 hour, linked to entity
        seed_upcoming_meeting(&db, "m1", 1);
        link_meeting_entity(&db, "m1", "acme");

        let enqueued = check_upcoming_meeting_readiness(&db, &queue, None);
        assert_eq!(enqueued.len(), 1);
        assert_eq!(enqueued[0], "acme");
    }

    #[test]
    fn test_pre_meeting_check_skips_fresh_entity() {
        let db = test_db();
        let queue = crate::intel_queue::IntelligenceQueue::new();

        // Entity with fresh intelligence (1 day ago)
        seed_entity(&db, "acme", "Acme Corp", "account");
        let fresh_time = (Utc::now() - chrono::Duration::days(1)).to_rfc3339();
        seed_entity_intelligence(&db, "acme", &fresh_time);

        // Meeting in 1 hour, linked to entity
        seed_upcoming_meeting(&db, "m1", 1);
        link_meeting_entity(&db, "m1", "acme");

        let enqueued = check_upcoming_meeting_readiness(&db, &queue, None);
        assert!(enqueued.is_empty());
    }

    #[test]
    fn test_pre_meeting_check_skips_distant_meetings() {
        let db = test_db();
        let queue = crate::intel_queue::IntelligenceQueue::new();

        // Entity with stale intelligence
        seed_entity(&db, "acme", "Acme Corp", "account");
        let stale_time = (Utc::now() - chrono::Duration::days(30)).to_rfc3339();
        seed_entity_intelligence(&db, "acme", &stale_time);

        // Meeting in 5 hours (outside 2-hour window)
        seed_upcoming_meeting(&db, "m1", 5);
        link_meeting_entity(&db, "m1", "acme");

        let enqueued = check_upcoming_meeting_readiness(&db, &queue, None);
        assert!(enqueued.is_empty());
    }

    #[test]
    fn test_pre_meeting_check_enqueues_never_enriched() {
        let db = test_db();
        let queue = crate::intel_queue::IntelligenceQueue::new();

        // Entity with NO intelligence row at all
        seed_entity(&db, "acme", "Acme Corp", "account");

        // Meeting in 1 hour, linked to entity
        seed_upcoming_meeting(&db, "m1", 1);
        link_meeting_entity(&db, "m1", "acme");

        let enqueued = check_upcoming_meeting_readiness(&db, &queue, None);
        assert_eq!(enqueued.len(), 1);
    }

    #[test]
    fn test_overnight_scan_produces_maintenance_json() {
        let db = test_db();
        let queue = crate::intel_queue::IntelligenceQueue::new();
        let workspace = tempfile::tempdir().unwrap();

        // Create _today/data dir
        let data_dir = workspace.path().join("_today").join("data");
        std::fs::create_dir_all(&data_dir).unwrap();

        let config = Config {
            workspace_path: workspace.path().to_string_lossy().to_string(),
            user_domain: Some("myco.com".to_string()),
            ..default_test_config()
        };

        let report = run_overnight_scan(&db, &config, workspace.path(), &queue);

        // Report should have a timestamp
        assert!(!report.ran_at.is_empty());

        // maintenance.json should exist
        let maint_path = data_dir.join("maintenance.json");
        assert!(maint_path.exists());

        // Should be valid JSON
        let content = std::fs::read_to_string(&maint_path).unwrap();
        let parsed: OvernightReport = serde_json::from_str(&content).unwrap();
        assert!(!parsed.ran_at.is_empty());
    }

    #[test]
    fn test_overnight_budget_higher_than_daytime() {
        assert_eq!(OVERNIGHT_AI_BUDGET, 20);
        // Daytime default is 10 (from HygieneBudget::default())
        let daytime = crate::state::HygieneBudget::default();
        assert!(OVERNIGHT_AI_BUDGET > daytime.daily_limit);
    }

    #[test]
    fn test_is_overnight_window_returns_bool() {
        // We can't control the clock, but we can verify it returns a bool
        // and doesn't panic
        let _result = is_overnight_window();
    }

    // --- Duplicate People Detection tests (I172) ---

    #[test]
    fn test_split_name_basic() {
        let (first, last) = split_name("Jane Doe").unwrap();
        assert_eq!(first, "jane");
        assert_eq!(last, "doe");
    }

    #[test]
    fn test_split_name_single() {
        let (first, last) = split_name("Jane").unwrap();
        assert_eq!(first, "jane");
        assert_eq!(last, "");
    }

    #[test]
    fn test_split_name_multi_part_last() {
        let (first, last) = split_name("Jan Van Der Berg").unwrap();
        assert_eq!(first, "jan");
        assert_eq!(last, "van der berg");
    }

    #[test]
    fn test_split_name_email_rejected() {
        assert!(split_name("jane@acme.com").is_none());
    }

    #[test]
    fn test_split_name_empty() {
        assert!(split_name("").is_none());
    }

    #[test]
    fn test_score_exact_match() {
        let result = score_name_similarity("Jane Doe", "jane doe");
        assert!(result.is_some());
        let (confidence, reason) = result.unwrap();
        assert!((confidence - 0.95).abs() < f32::EPSILON);
        assert!(reason.contains("Exact name match"));
    }

    #[test]
    fn test_score_first_name_plus_initial() {
        let result = score_name_similarity("Jane Doe", "Jane Davis");
        assert!(result.is_some());
        let (confidence, _) = result.unwrap();
        assert!((confidence - 0.70).abs() < f32::EPSILON);
    }

    #[test]
    fn test_score_prefix_match() {
        let result = score_name_similarity("Jonathan Smith", "Jonny Smithson");
        assert!(result.is_some());
        let (confidence, _) = result.unwrap();
        assert!((confidence - 0.60).abs() < f32::EPSILON);
    }

    #[test]
    fn test_score_same_last_name() {
        let result = score_name_similarity("Alice Smith", "Bob Smith");
        assert!(result.is_some());
        let (confidence, _) = result.unwrap();
        assert!((confidence - 0.40).abs() < f32::EPSILON);
    }

    #[test]
    fn test_score_no_match() {
        let result = score_name_similarity("Jane Doe", "Bob Smith");
        assert!(result.is_none());
    }

    #[test]
    fn test_score_both_single_name_different() {
        // Single-word names that differ should not match
        let result = score_name_similarity("Alice", "Bob");
        assert!(result.is_none());
    }

    #[test]
    fn test_score_email_names_rejected() {
        let result = score_name_similarity("jane@acme.com", "jane@other.com");
        assert!(result.is_none());
    }

    #[test]
    fn test_detect_duplicates_exact_name_different_emails() {
        let db = test_db();
        seed_person(
            &db,
            "jane-doe-1",
            "jane.doe@acme.com",
            "Jane Doe",
            "external",
        );
        seed_person(&db, "jane-doe-2", "jdoe@acme.com", "Jane Doe", "external");

        let dupes = detect_duplicate_people(&db).unwrap();
        assert_eq!(dupes.len(), 1);
        assert!((dupes[0].confidence - 0.95).abs() < f32::EPSILON);
        assert!(dupes[0].reason.contains("Exact name match"));
    }

    #[test]
    fn test_detect_duplicates_same_email_not_flagged() {
        let db = test_db();
        // Same email means same person — not a duplicate
        seed_person(&db, "jane-1", "jane@acme.com", "Jane Doe", "external");
        // This would normally be caught by upsert, but test the detection logic
        // seed a person with the same email but different id would be same person
        // Instead, two people with genuinely different emails on different domains
        seed_person(&db, "jane-2", "jane@other.com", "Jane Doe", "external");

        let dupes = detect_duplicate_people(&db).unwrap();
        // Different domains, so they're in different groups — no match
        assert!(dupes.is_empty());
    }

    #[test]
    fn test_detect_duplicates_cross_domain_no_match() {
        let db = test_db();
        // Same name but different domains — not flagged (different domain groups)
        seed_person(&db, "jane-acme", "jane@acme.com", "Jane Doe", "external");
        seed_person(&db, "jane-other", "jane@other.com", "Jane Doe", "external");

        let dupes = detect_duplicate_people(&db).unwrap();
        assert!(dupes.is_empty());
    }

    #[test]
    fn test_detect_duplicates_archived_excluded() {
        let db = test_db();
        seed_person(
            &db,
            "jane-doe-1",
            "jane.doe@acme.com",
            "Jane Doe",
            "external",
        );
        seed_person(&db, "jane-doe-2", "jdoe@acme.com", "Jane Doe", "external");

        // Archive one
        db.conn_ref()
            .execute("UPDATE people SET archived = 1 WHERE id = 'jane-doe-2'", [])
            .unwrap();

        let dupes = detect_duplicate_people(&db).unwrap();
        assert!(dupes.is_empty());
    }

    #[test]
    fn test_detect_duplicates_empty_db() {
        let db = test_db();
        let dupes = detect_duplicate_people(&db).unwrap();
        assert!(dupes.is_empty());
    }

    #[test]
    fn test_detect_duplicates_singleton_domain() {
        let db = test_db();
        seed_person(&db, "jane", "jane@acme.com", "Jane Doe", "external");

        let dupes = detect_duplicate_people(&db).unwrap();
        assert!(dupes.is_empty());
    }

    #[test]
    fn test_detect_duplicates_sorted_by_confidence() {
        let db = test_db();
        // Exact match pair: 0.95
        seed_person(&db, "jane-1", "jane.doe@acme.com", "Jane Doe", "external");
        seed_person(&db, "jane-2", "jdoe@acme.com", "Jane Doe", "external");
        // Same-last-name pair: 0.40
        seed_person(&db, "bob-doe", "bob.doe@acme.com", "Bob Doe", "external");

        let dupes = detect_duplicate_people(&db).unwrap();
        // Should have at least 2 candidates (exact match + same last name pairs)
        assert!(dupes.len() >= 2);
        // First should be highest confidence
        assert!(dupes[0].confidence >= dupes[1].confidence);
    }

    #[test]
    fn test_detect_duplicates_in_hygiene_report() {
        let db = test_db();
        seed_person(&db, "jane-1", "jane.doe@acme.com", "Jane Doe", "external");
        seed_person(&db, "jane-2", "jdoe@acme.com", "Jane Doe", "external");

        let config = Config {
            workspace_path: "/tmp/nonexistent".to_string(),
            user_domain: Some("myco.com".to_string()),
            ..default_test_config()
        };

        let report = run_hygiene_scan(&db, &config, Path::new("/tmp/nonexistent"), None, None, false, None);
        assert_eq!(report.duplicate_people, 1);
    }

    #[test]
    fn test_detect_duplicates_first_name_initial_match() {
        let db = test_db();
        seed_person(&db, "jane-doe", "jane.doe@acme.com", "Jane Doe", "external");
        seed_person(
            &db,
            "jane-davis",
            "jane.davis@acme.com",
            "Jane Davis",
            "external",
        );

        let dupes = detect_duplicate_people(&db).unwrap();
        // Should match: same first name + last initial 'D'
        let matching: Vec<_> = dupes
            .iter()
            .filter(|d| (d.confidence - 0.70).abs() < f32::EPSILON)
            .collect();
        assert_eq!(matching.len(), 1);
    }

    // --- Renewal auto-rollover tests (I143) ---

    fn seed_account_with_renewal(
        db: &ActionDb,
        id: &str,
        name: &str,
        contract_end: &str,
        arr: Option<f64>,
    ) {
        let now = Utc::now().to_rfc3339();
        let account = crate::db::DbAccount {
            id: id.to_string(),
            name: name.to_string(),
            lifecycle: None,
            arr,
            health: None,
            contract_start: None,
            contract_end: Some(contract_end.to_string()),
            nps: None,
            tracker_path: None,
            parent_id: None,
            is_internal: false,
            updated_at: now,
            archived: false,
            keywords: None,
            keywords_extracted_at: None,
        };
        db.upsert_account(&account).expect("upsert account");
    }

    #[test]
    fn test_renewal_rollover_advances_date_and_records_event() {
        let db = test_db();
        // Account with contract_end 6 months in the past, no churn
        let past = (Utc::now() - chrono::Duration::days(180))
            .format("%Y-%m-%d")
            .to_string();
        seed_account_with_renewal(&db, "acme", "Acme Corp", &past, Some(120_000.0));

        let (fixed, _) = fix_renewal_rollovers(&db);
        assert_eq!(fixed, 1);

        // Verify the renewal event was recorded
        let events = db.get_account_events("acme").unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "renewal");
        assert_eq!(events[0].event_date, past);
        assert_eq!(events[0].arr_impact, Some(120_000.0));
        assert!(events[0].notes.as_deref().unwrap().contains("Auto-renewed"));

        // Verify contract_end advanced by 12 months
        let past_date = chrono::NaiveDate::parse_from_str(&past, "%Y-%m-%d").unwrap();
        let expected_next = (past_date + chrono::Months::new(12))
            .format("%Y-%m-%d")
            .to_string();

        let updated: String = db
            .conn_ref()
            .query_row(
                "SELECT contract_end FROM accounts WHERE id = 'acme'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(updated, expected_next);
    }

    #[test]
    fn test_renewal_rollover_skips_churned_account() {
        let db = test_db();
        let past = (Utc::now() - chrono::Duration::days(30))
            .format("%Y-%m-%d")
            .to_string();
        seed_account_with_renewal(&db, "churned-co", "Churned Co", &past, Some(50_000.0));

        // Record a churn event
        db.record_account_event("churned-co", "churn", &past, Some(50_000.0), Some("Lost"))
            .unwrap();

        let (fixed, _) = fix_renewal_rollovers(&db);
        assert_eq!(fixed, 0);

        // contract_end should be unchanged
        let contract_end: String = db
            .conn_ref()
            .query_row(
                "SELECT contract_end FROM accounts WHERE id = 'churned-co'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(contract_end, past);
    }

    #[test]
    fn test_renewal_rollover_idempotent() {
        let db = test_db();
        let past = (Utc::now() - chrono::Duration::days(60))
            .format("%Y-%m-%d")
            .to_string();
        seed_account_with_renewal(&db, "acme", "Acme Corp", &past, None);

        let (fixed1, _) = fix_renewal_rollovers(&db);
        assert_eq!(fixed1, 1);

        // Second run: contract_end is now in the future, so no rollover
        let (fixed2, _) = fix_renewal_rollovers(&db);
        assert_eq!(fixed2, 0);
    }

    #[test]
    fn test_renewal_rollover_in_hygiene_report() {
        let db = test_db();
        let past = (Utc::now() - chrono::Duration::days(30))
            .format("%Y-%m-%d")
            .to_string();
        seed_account_with_renewal(&db, "acme", "Acme Corp", &past, Some(100_000.0));

        let config = Config {
            workspace_path: "/tmp/nonexistent".to_string(),
            user_domain: Some("myco.com".to_string()),
            ..default_test_config()
        };

        let report = run_hygiene_scan(&db, &config, Path::new("/tmp/nonexistent"), None, None, false, None);
        assert_eq!(report.fixes.renewals_rolled_over, 1);
    }

    // --- Hygiene Narrative tests (I273) ---

    #[test]
    fn test_join_prose_list_empty() {
        assert_eq!(join_prose_list(&[]), "");
    }

    #[test]
    fn test_join_prose_list_one() {
        assert_eq!(join_prose_list(&["a".to_string()]), "a");
    }

    #[test]
    fn test_join_prose_list_two() {
        assert_eq!(
            join_prose_list(&["a".to_string(), "b".to_string()]),
            "a and b"
        );
    }

    #[test]
    fn test_join_prose_list_three() {
        assert_eq!(
            join_prose_list(&["a".to_string(), "b".to_string(), "c".to_string()]),
            "a, b, and c"
        );
    }

    #[test]
    fn test_build_narrative_empty_report() {
        let report = HygieneReport::default();
        assert!(build_hygiene_narrative(&report).is_none());
    }

    #[test]
    fn test_build_narrative_only_fixes() {
        let mut report = HygieneReport::default();
        report.fixes.names_resolved = 3;
        report.scanned_at = "2026-01-15T10:00:00Z".to_string();
        let view = build_hygiene_narrative(&report).unwrap();
        assert!(view.narrative.contains("Resolved 3 unnamed people"));
        assert!(view.narrative.contains("All clear."));
        assert_eq!(view.total_fixes, 3);
        assert_eq!(view.total_remaining_gaps, 0);
        assert!(view.remaining_gaps.is_empty());
    }

    #[test]
    fn test_build_narrative_only_gaps() {
        let report = HygieneReport {
            unnamed_people: 4,
            scanned_at: "2026-01-15T10:00:00Z".to_string(),
            ..Default::default()
        };
        let view = build_hygiene_narrative(&report).unwrap();
        assert!(view.narrative.contains("4 gaps remaining"));
        assert_eq!(view.total_fixes, 0);
        assert_eq!(view.total_remaining_gaps, 4);
        assert_eq!(view.remaining_gaps.len(), 1);
    }

    #[test]
    fn test_build_narrative_fixes_and_gaps() {
        let mut report = HygieneReport::default();
        report.fixes.relationships_reclassified = 2;
        report.missing_intelligence = 3;
        report.scanned_at = "2026-01-15T10:00:00Z".to_string();
        let view = build_hygiene_narrative(&report).unwrap();
        assert!(view.narrative.contains("Reclassified 2 relationships"));
        assert!(view.narrative.contains("3 gaps remaining"));
        assert_eq!(view.total_fixes, 2);
        assert_eq!(view.total_remaining_gaps, 3);
    }

    fn default_test_config() -> Config {
        Config {
            workspace_path: String::new(),
            schedules: crate::types::Schedules::default(),
            profile: "customer-success".to_string(),
            profile_config: None,
            entity_mode: "account".to_string(),
            google: crate::types::GoogleConfig::default(),
            post_meeting_capture: crate::types::PostMeetingCaptureConfig::default(),
            quill: crate::quill::QuillConfig::default(),
            granola: crate::granola::GranolaConfig::default(),
            gravatar: crate::gravatar::GravatarConfig::default(),
            clay: crate::clay::ClayConfig::default(),
            features: std::collections::HashMap::new(),
            user_domain: None,
            user_domains: None,
            user_name: None,
            user_company: None,
            user_title: None,
            user_focus: None,
            internal_team_setup_completed: false,
            internal_team_setup_version: 0,
            internal_org_account_id: None,
            developer_mode: false,
            personality: "professional".to_string(),
            ai_models: crate::types::AiModelConfig::default(),
            embeddings: crate::types::EmbeddingConfig::default(),
            hygiene_scan_interval_hours: 4,
            hygiene_ai_budget: 10,
            hygiene_pre_meeting_hours: 12,
        }
    }
}
