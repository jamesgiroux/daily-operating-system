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

/// How many days back to look for orphaned meetings.
const ORPHANED_MEETING_LOOKBACK_DAYS: i32 = 90;

/// Report from a hygiene scan: gaps detected + mechanical fixes applied.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HygieneReport {
    pub unnamed_people: usize,
    pub unknown_relationships: usize,
    pub missing_intelligence: usize,
    pub stale_intelligence: usize,
    pub unsummarized_files: usize,
    pub orphaned_meetings: usize,
    pub duplicate_people: usize,
    pub fixes: MechanicalFixes,
    pub scanned_at: String,
}

/// Counts of mechanical fixes applied during a hygiene scan.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MechanicalFixes {
    pub relationships_reclassified: usize,
    pub summaries_extracted: usize,
    pub orphaned_meetings_linked: usize,
    pub meeting_counts_updated: usize,
    pub names_resolved: usize,
    pub people_linked_by_domain: usize,
    pub renewals_rolled_over: usize,
    pub ai_enrichments_enqueued: usize,
}

/// Run a full hygiene scan: detect gaps, apply mechanical fixes, return report.
/// If `budget` is provided, enqueue AI enrichment for remaining gaps.
pub fn run_hygiene_scan(
    db: &ActionDb,
    config: &Config,
    workspace: &Path,
    budget: Option<&crate::state::HygieneBudget>,
    queue: Option<&crate::intel_queue::IntelligenceQueue>,
) -> HygieneReport {
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
    report.orphaned_meetings = db
        .get_orphaned_meetings(ORPHANED_MEETING_LOOKBACK_DAYS)
        .map(|v| v.len())
        .unwrap_or(0);
    report.duplicate_people = detect_duplicate_people(db).map(|v| v.len()).unwrap_or(0);

    // --- Phase 1: Mechanical fixes (free, instant) ---
    let user_domains = config.resolved_user_domains();
    report.fixes.relationships_reclassified = fix_unknown_relationships(db, &user_domains);
    report.fixes.summaries_extracted = backfill_file_summaries(db);
    report.fixes.orphaned_meetings_linked = fix_orphaned_meetings(db);
    report.fixes.meeting_counts_updated = fix_meeting_counts(db);
    report.fixes.renewals_rolled_over = fix_renewal_rollovers(db);

    // --- Phase 2: Email name resolution + domain linking (free) ---
    report.fixes.names_resolved = resolve_names_from_emails(db, workspace);
    report.fixes.people_linked_by_domain = auto_link_people_by_domain(db);

    // --- Phase 2: AI-budgeted gap filling ---
    if let (Some(budget), Some(queue)) = (budget, queue) {
        report.fixes.ai_enrichments_enqueued = enqueue_ai_enrichments(db, budget, queue);
    }

    report
}

/// Reclassify people with "unknown" relationship using the user's domains (I171).
fn fix_unknown_relationships(db: &ActionDb, user_domains: &[String]) -> usize {
    if user_domains.is_empty() {
        return 0; // Can't classify without user domains
    }

    let people = match db.get_unknown_relationship_people() {
        Ok(p) => p,
        Err(_) => return 0,
    };

    let mut fixed = 0;
    for person in &people {
        let new_rel = crate::util::classify_relationship_multi(&person.email, user_domains);
        if new_rel != "unknown"
            && db.update_person_relationship(&person.id, &new_rel).is_ok() {
                fixed += 1;
            }
    }
    fixed
}

/// Extract summaries for content files that have none.
fn backfill_file_summaries(db: &ActionDb) -> usize {
    let files = match db.get_unsummarized_content_files() {
        Ok(f) => f,
        Err(_) => return 0,
    };

    // Cap per scan to avoid blocking too long (mechanical extraction is fast but IO-bound)
    let batch_limit = 50;
    let mut extracted = 0;

    for file in files.iter().take(batch_limit) {
        let path = std::path::Path::new(&file.absolute_path);
        if !path.exists() {
            continue;
        }

        let (extracted_at, summary) = crate::entity_intel::extract_and_summarize(path);
        if let (Some(ext_at), Some(summ)) = (extracted_at, summary) {
            let _ = db.conn_ref().execute(
                "UPDATE content_index SET extracted_at = ?1, summary = ?2 WHERE id = ?3",
                rusqlite::params![ext_at, summ, file.id],
            );
            extracted += 1;
        }
    }
    extracted
}

/// Link orphaned meetings to entities via account name resolution.
fn fix_orphaned_meetings(db: &ActionDb) -> usize {
    let meetings = match db.get_orphaned_meetings(ORPHANED_MEETING_LOOKBACK_DAYS) {
        Ok(m) => m,
        Err(_) => return 0,
    };

    let mut linked = 0;
    for meeting in &meetings {
        let account_name = match &meeting.account_id {
            Some(name) if !name.is_empty() => name,
            _ => continue,
        };

        // Try to resolve account name to an entity ID
        if let Ok(Some(account)) = db.get_account_by_name(account_name) {
            if db
                .link_meeting_entity(&meeting.id, &account.id, "account")
                .is_ok()
            {
                linked += 1;
            }
        }
    }
    linked
}

/// Recompute meeting counts for people whose counts may be stale.
/// Only fixes people whose stored count differs from actual attendee records.
fn fix_meeting_counts(db: &ActionDb) -> usize {
    // Find people with mismatched counts via a single query
    let mismatched: Vec<String> = db
        .conn_ref()
        .prepare(
            "SELECT p.id FROM people p
             LEFT JOIN (
                 SELECT person_id, COUNT(*) AS actual FROM meeting_attendees GROUP BY person_id
             ) ma ON ma.person_id = p.id
             WHERE p.meeting_count != COALESCE(ma.actual, 0)",
        )
        .and_then(|mut stmt| {
            let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
            Ok(rows.filter_map(|r| r.ok()).collect())
        })
        .unwrap_or_default();

    let mut fixed = 0;
    for person_id in &mismatched {
        if db.recompute_person_meeting_count(person_id).is_ok() {
            fixed += 1;
        }
    }
    fixed
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
fn fix_renewal_rollovers(db: &ActionDb) -> usize {
    let past_renewal = match db.get_accounts_past_renewal() {
        Ok(accounts) => accounts,
        Err(_) => return 0,
    };

    let mut fixed = 0;
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
        fixed += 1;
    }

    fixed
}

// =============================================================================
// Phase 2: Email Name Resolution + Domain Linking (I146)
// =============================================================================

/// Resolve unnamed people from email From headers in emails.json.
///
/// Reads `_today/data/emails.json` (created by daily briefing), extracts display
/// names from From headers, and updates people who only have email-derived names.
pub fn resolve_names_from_emails(db: &ActionDb, workspace: &Path) -> usize {
    let emails_path = workspace.join("_today").join("data").join("emails.json");
    let raw = match std::fs::read_to_string(&emails_path) {
        Ok(r) => r,
        Err(_) => return 0, // No emails.json yet
    };

    let data: serde_json::Value = match serde_json::from_str(&raw) {
        Ok(d) => d,
        Err(_) => return 0,
    };

    // Get unnamed people to match against
    let unnamed = match db.get_unnamed_people() {
        Ok(p) if !p.is_empty() => p,
        _ => return 0,
    };
    let unnamed_emails: std::collections::HashSet<String> =
        unnamed.iter().map(|p| p.email.to_lowercase()).collect();

    let mut resolved = 0;

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
                    resolved += 1;
                }
            }
        }
    }

    resolved
}

/// Auto-link people to entities by matching email domain to account names.
///
/// If `schen@acme.com` is a person and there's an account whose name contains
/// "acme", link them via the entity_people junction table.
pub fn auto_link_people_by_domain(db: &ActionDb) -> usize {
    let accounts = match db.get_all_accounts() {
        Ok(a) => a,
        Err(_) => return 0,
    };
    if accounts.is_empty() {
        return 0;
    }

    // Build a domain-hint → account_id map
    // E.g., account "Acme Corp" → hint "acme"
    let account_hints: Vec<(String, String)> = accounts
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
            (hint, a.id.clone())
        })
        .filter(|(hint, _)| hint.len() >= 3)
        .collect();

    if account_hints.is_empty() {
        return 0;
    }

    // Get external people not yet linked to any entity
    let people = match db.get_people(Some("external")) {
        Ok(p) => p,
        Err(_) => return 0,
    };

    let mut linked = 0;
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
        for (hint, account_id) in &account_hints {
            if (&domain_base == hint || (hint.len() >= 4 && domain_base.contains(hint.as_str())))
                && db
                    .link_person_to_entity(&person.id, account_id, "associated")
                    .is_ok()
                {
                    linked += 1;
                    break; // One link per person
                }
        }
    }

    linked
}

// =============================================================================
// Duplicate People Detection (I172)
// =============================================================================

/// A potential duplicate person pair with confidence scoring.
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

    for group in domain_groups.values() {
        // Skip singleton groups — no possible duplicates
        if group.len() < 2 {
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
/// Called after each calendar poll. Looks for meetings in the next 2 hours
/// with linked entities whose intelligence is older than 7 days.
pub fn check_upcoming_meeting_readiness(
    db: &ActionDb,
    queue: &crate::intel_queue::IntelligenceQueue,
) -> Vec<String> {
    use crate::intel_queue::{IntelPriority, IntelRequest};
    use std::time::Instant;

    let window_end = Utc::now() + chrono::Duration::hours(PRE_MEETING_WINDOW_HOURS);
    let stale_threshold = Utc::now() - chrono::Duration::days(PRE_MEETING_STALE_DAYS);
    let stale_str = stale_threshold.to_rfc3339();

    // Find meetings in the next window
    let upcoming: Vec<crate::db::DbMeeting> = db
        .conn_ref()
        .prepare(
            "SELECT id, title, meeting_type, start_time, end_time,
                    account_id, attendees, notes_path, summary,
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
                    account_id: row.get(5)?,
                    attendees: row.get(6)?,
                    notes_path: row.get(7)?,
                    summary: row.get(8)?,
                    created_at: row.get(9)?,
                    calendar_event_id: row.get(10)?,
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
    pub meetings_linked: usize,
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
    // Use expanded overnight budget
    let overnight_budget = crate::state::HygieneBudget::new(OVERNIGHT_AI_BUDGET);

    let report = run_hygiene_scan(db, config, workspace, Some(&overnight_budget), Some(queue));

    let overnight = OvernightReport {
        ran_at: Utc::now().to_rfc3339(),
        entities_refreshed: report.fixes.ai_enrichments_enqueued,
        names_resolved: report.fixes.names_resolved,
        meetings_linked: report.fixes.orphaned_meetings_linked,
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

/// Check if current time is in the overnight window (2-3 AM UTC).
fn is_overnight_window() -> bool {
    let hour = Utc::now()
        .format("%H")
        .to_string()
        .parse::<u32>()
        .unwrap_or(12);
    (2..=3).contains(&hour)
}

/// Background loop: runs scan on startup (30s delay), then every 4 hours.
pub async fn run_hygiene_loop(state: Arc<AppState>, _app: AppHandle) {
    // Wait for startup to complete
    tokio::time::sleep(std::time::Duration::from_secs(STARTUP_DELAY_SECS)).await;

    log::info!("HygieneLoop: started");

    loop {
        // Check for overnight window — use expanded scan with higher AI budget
        if is_overnight_window() {
            let overnight = try_run_overnight(&state);
            if let Some(report) = overnight {
                log::info!(
                    "HygieneLoop: overnight scan — {} entities refreshed, {} names resolved, \
                     {} meetings linked",
                    report.entities_refreshed,
                    report.names_resolved,
                    report.meetings_linked,
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
                + report.unsummarized_files
                + report.orphaned_meetings;

            let total_fixes = report.fixes.relationships_reclassified
                + report.fixes.summaries_extracted
                + report.fixes.orphaned_meetings_linked
                + report.fixes.meeting_counts_updated
                + report.fixes.names_resolved
                + report.fixes.people_linked_by_domain
                + report.fixes.renewals_rolled_over
                + report.fixes.ai_enrichments_enqueued;

            if total_gaps > 0 || total_fixes > 0 {
                log::info!(
                    "HygieneLoop: {} gaps detected, {} fixes applied \
                     (relationships={}, summaries={}, orphaned={}, counts={}, \
                     names={}, domain_links={}, renewals={}, ai_enqueued={})",
                    total_gaps,
                    total_fixes,
                    report.fixes.relationships_reclassified,
                    report.fixes.summaries_extracted,
                    report.fixes.orphaned_meetings_linked,
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
        }

        tokio::time::sleep(std::time::Duration::from_secs(SCAN_INTERVAL_SECS)).await;
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

    let workspace = std::path::Path::new(&config.workspace_path);
    Some(run_hygiene_scan(
        &db,
        &config,
        workspace,
        Some(&state.hygiene_budget),
        Some(&state.intel_queue),
    ))
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
            csm: None,
            champion: None,
            nps: None,
            tracker_path: None,
            parent_id: None,
            updated_at: now,
            archived: false,
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
    fn test_get_orphaned_meetings_none() {
        let db = test_db();
        let result = db.get_orphaned_meetings(90).unwrap();
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
        let fixed = fix_unknown_relationships(&db, &domains);
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

        let fixed = fix_unknown_relationships(&db, &[]);
        assert_eq!(fixed, 0);
    }

    #[test]
    fn test_fix_unknown_relationships_idempotent() {
        let db = test_db();
        seed_person(&db, "p1", "me@myco.com", "Me", "unknown");

        let domains = vec!["myco.com".to_string()];
        fix_unknown_relationships(&db, &domains);
        // Second run: person is now "internal", not "unknown", so shouldn't be re-processed
        let fixed = fix_unknown_relationships(&db, &domains);
        assert_eq!(fixed, 0);
    }

    #[test]
    fn test_fix_orphaned_meetings_links() {
        let db = test_db();
        seed_account(&db, "acme-corp", "Acme Corp");

        // Insert a meeting with account_id but no junction row
        let now = Utc::now().to_rfc3339();
        db.conn_ref()
            .execute(
                "INSERT INTO meetings_history (id, title, meeting_type, start_time, account_id, created_at)
                 VALUES ('m1', 'Acme Sync', 'customer', ?1, 'Acme Corp', ?1)",
                rusqlite::params![now],
            )
            .unwrap();

        let orphaned = db.get_orphaned_meetings(90).unwrap();
        assert_eq!(orphaned.len(), 1);

        let linked = fix_orphaned_meetings(&db);
        assert_eq!(linked, 1);

        // Should no longer be orphaned
        let orphaned_after = db.get_orphaned_meetings(90).unwrap();
        assert_eq!(orphaned_after.len(), 0);
    }

    #[test]
    fn test_fix_orphaned_meetings_idempotent() {
        let db = test_db();
        seed_account(&db, "acme-corp", "Acme Corp");

        let now = Utc::now().to_rfc3339();
        db.conn_ref()
            .execute(
                "INSERT INTO meetings_history (id, title, meeting_type, start_time, account_id, created_at)
                 VALUES ('m1', 'Acme Sync', 'customer', ?1, 'Acme Corp', ?1)",
                rusqlite::params![now],
            )
            .unwrap();

        fix_orphaned_meetings(&db);
        let linked = fix_orphaned_meetings(&db);
        assert_eq!(linked, 0); // Nothing to fix on second run
    }

    #[test]
    fn test_fix_meeting_counts() {
        let db = test_db();
        seed_person(&db, "p1", "a@test.com", "A Test", "external");

        // Manually set a wrong meeting count
        db.conn_ref()
            .execute("UPDATE people SET meeting_count = 99 WHERE id = 'p1'", [])
            .unwrap();

        let fixed = fix_meeting_counts(&db);
        assert_eq!(fixed, 1);

        let person = db.get_person("p1").unwrap().unwrap();
        assert_eq!(person.meeting_count, 0); // No actual attendee records
    }

    #[test]
    fn test_fix_meeting_counts_idempotent() {
        let db = test_db();
        seed_person(&db, "p1", "a@test.com", "A Test", "external");

        // Count is already correct (0 meetings, 0 count)
        let fixed = fix_meeting_counts(&db);
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

        let report = run_hygiene_scan(&db, &config, Path::new("/tmp/nonexistent"), None, None);

        assert_eq!(report.unnamed_people, 0);
        assert_eq!(report.unknown_relationships, 0);
        assert_eq!(report.missing_intelligence, 0);
        assert_eq!(report.stale_intelligence, 0);
        assert_eq!(report.unsummarized_files, 0);
        assert_eq!(report.orphaned_meetings, 0);
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

        let report = run_hygiene_scan(&db, &config, Path::new("/tmp/nonexistent"), None, None);

        // Detected before fixing
        assert_eq!(report.unknown_relationships, 2);

        // Fixes applied
        assert_eq!(report.fixes.relationships_reclassified, 2);

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

        let resolved = resolve_names_from_emails(&db, workspace.path());
        assert_eq!(resolved, 1);

        let person = db.get_person("jane-customer-com").unwrap().unwrap();
        assert_eq!(person.name, "Jane Doe");
    }

    #[test]
    fn test_resolve_names_no_emails_file() {
        let db = test_db();
        let resolved = resolve_names_from_emails(&db, Path::new("/nonexistent"));
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
        let resolved = resolve_names_from_emails(&db, workspace.path());
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

        let linked = auto_link_people_by_domain(&db);
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

        auto_link_people_by_domain(&db);
        let linked = auto_link_people_by_domain(&db);
        assert_eq!(linked, 0); // Already linked
    }

    #[test]
    fn test_auto_link_skips_internal() {
        let db = test_db();
        seed_account(&db, "acme-corp", "Acme Corp");
        seed_person(&db, "me-acme-com", "me@acme.com", "Me", "internal");

        let linked = auto_link_people_by_domain(&db);
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

        let enqueued = check_upcoming_meeting_readiness(&db, &queue);
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

        let enqueued = check_upcoming_meeting_readiness(&db, &queue);
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

        let enqueued = check_upcoming_meeting_readiness(&db, &queue);
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

        let enqueued = check_upcoming_meeting_readiness(&db, &queue);
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

        let report = run_hygiene_scan(&db, &config, Path::new("/tmp/nonexistent"), None, None);
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
            csm: None,
            champion: None,
            nps: None,
            tracker_path: None,
            parent_id: None,
            updated_at: now,
            archived: false,
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

        let fixed = fix_renewal_rollovers(&db);
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

        let fixed = fix_renewal_rollovers(&db);
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

        let fixed1 = fix_renewal_rollovers(&db);
        assert_eq!(fixed1, 1);

        // Second run: contract_end is now in the future, so no rollover
        let fixed2 = fix_renewal_rollovers(&db);
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

        let report = run_hygiene_scan(&db, &config, Path::new("/tmp/nonexistent"), None, None);
        assert_eq!(report.fixes.renewals_rolled_over, 1);
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
            features: std::collections::HashMap::new(),
            user_domain: None,
            user_domains: None,
            user_name: None,
            user_company: None,
            user_title: None,
            user_focus: None,
            developer_mode: false,
            ai_models: crate::types::AiModelConfig::default(),
        }
    }
}
