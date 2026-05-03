//! Gap detection functions for hygiene scanning.
//!
//! These functions detect data quality issues without modifying data.

use std::collections::HashMap;

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::db::ActionDb;
use crate::types::Config;

/// Max people per domain for pairwise duplicate detection (prevents O(n^2) explosion).
const MAX_DOMAIN_GROUP_SIZE: usize = 200;

/// Hours before a meeting to trigger refresh.
const PRE_MEETING_WINDOW_HOURS: i64 = 2;

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
pub fn score_name_similarity(name1: &str, name2: &str) -> Option<(f32, String)> {
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

    let window_hours = config
        .map(|c| c.hygiene_pre_meeting_hours as i64)
        .unwrap_or(PRE_MEETING_WINDOW_HOURS);
    let window_end = Utc::now() + chrono::Duration::hours(window_hours);

    // Find meetings in the next window
    let upcoming: Vec<crate::db::DbMeeting> = db
        .conn_ref()
        .prepare(
            "SELECT m.id, m.title, m.meeting_type, m.start_time, m.end_time,
                    m.attendees, m.notes_path, mt.summary,
                    m.created_at, m.calendar_event_id
             FROM meetings m
             LEFT JOIN meeting_transcripts mt ON mt.meeting_id = m.id
             WHERE m.start_time > datetime('now')
               AND m.start_time <= ?1
             ORDER BY m.start_time ASC",
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
                    intelligence_state: None,
                    intelligence_quality: None,
                    last_enriched_at: None,
                    signal_count: None,
                    has_new_signals: None,
                    last_viewed_at: None,
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
            let entity_type = format!("{:?}", entity.entity_type).to_lowercase();
            // Use the continuous trigger score  -- meeting_imminence will be high
            // since these meetings are within the pre-meeting window. Combined with
            // staleness, this replaces the binary PRE_MEETING_STALE_DAYS check.
            let trigger_score = crate::self_healing::remediation::compute_enrichment_trigger_score(
                db,
                &entity.id,
                &entity_type,
            );
            // Pre-meeting window: enqueue if trigger score >= 0.4 (lower than the
            // signal-driven 0.7 threshold because we're in a time-critical window)
            if trigger_score >= 0.4 {
                let _ = queue.enqueue(IntelRequest::new(                    entity.id.clone(),
                    entity_type,
                    IntelPriority::CalendarChange,
                ));
                enqueued_ids.push(entity.id.clone());
                log::debug!(
                    "PreMeeting: enqueued {} (trigger_score={:.2})",
                    entity.id,
                    trigger_score,
                );
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

/// Count empty shell accounts for gap reporting (before fixes run).
pub(super) fn count_empty_shell_accounts(db: &ActionDb) -> usize {
    db.conn_ref()
        .query_row(
            "SELECT COUNT(*) FROM accounts a
             WHERE a.archived = 0
               AND a.updated_at <= datetime('now', '-30 days')
               AND NOT EXISTS (SELECT 1 FROM meeting_entities me WHERE me.entity_id = a.id AND me.entity_type = 'account')
               AND NOT EXISTS (SELECT 1 FROM actions act WHERE act.account_id = a.id)
               AND NOT EXISTS (SELECT 1 FROM account_stakeholders as_ WHERE as_.account_id = a.id)
               AND NOT EXISTS (SELECT 1 FROM account_events ae WHERE ae.account_id = a.id)
               AND NOT EXISTS (SELECT 1 FROM email_signals es WHERE es.entity_id = a.id AND es.entity_type = 'account' AND es.deactivated_at IS NULL)",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_utils::test_db;
    use crate::hygiene::tests_common::{
        default_test_config, link_meeting_entity, seed_entity, seed_entity_intelligence,
        seed_person, seed_upcoming_meeting,
    };
    use chrono::Utc;
    use std::path::Path;

    // --- Duplicate People Detection tests  ---

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
        seed_person(&db, "jane-1", "jane@acme.com", "Jane Doe", "external");
        seed_person(&db, "jane-2", "jane@other.com", "Jane Doe", "external");

        let dupes = detect_duplicate_people(&db).unwrap();
        // Different domains, so they're in different groups -- no match
        assert!(dupes.is_empty());
    }

    #[test]
    fn test_detect_duplicates_cross_domain_no_match() {
        let db = test_db();
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

    // --- Pre-meeting readiness tests ---

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
    fn test_pre_meeting_check_enqueues_fresh_entity_with_imminent_meeting() {
        let db = test_db();
        let queue = crate::intel_queue::IntelligenceQueue::new();

        seed_entity(&db, "acme", "Acme Corp", "account");
        let fresh_time = (Utc::now() - chrono::Duration::days(1)).to_rfc3339();
        seed_entity_intelligence(&db, "acme", &fresh_time);

        // Meeting in 1 hour, linked to entity
        seed_upcoming_meeting(&db, "m1", 1);
        link_meeting_entity(&db, "m1", "acme");

        let enqueued = check_upcoming_meeting_readiness(&db, &queue, None);
        assert_eq!(
            enqueued.len(),
            1,
            "fresh entity with imminent meeting should be enqueued (trigger score driven)"
        );
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
    fn test_detect_duplicates_in_hygiene_report() {
        let db = test_db();
        seed_person(&db, "jane-1", "jane.doe@acme.com", "Jane Doe", "external");
        seed_person(&db, "jane-2", "jdoe@acme.com", "Jane Doe", "external");

        let config = crate::types::Config {
            workspace_path: "/tmp/nonexistent".to_string(),
            user_domain: Some("myco.com".to_string()),
            ..default_test_config()
        };

        let report = crate::hygiene::run_hygiene_scan(
            &db,
            &config,
            Path::new("/tmp/nonexistent"),
            None,
            None,
            false,
            None,
        );
        // After auto-merge of high-confidence duplicates (>=0.95), the re-count should be 0
        assert_eq!(report.duplicate_people, 0);
        assert_eq!(report.fixes.people_auto_merged, 1);
    }
}
