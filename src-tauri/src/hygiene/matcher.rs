//! Entity matching, enrichment orchestration, and linking (hygiene scan step 2).
//!
//! Functions that resolve names, link people to entities, deduplicate
//! records, and enqueue AI enrichment for remaining gaps.

use std::collections::HashMap;
use std::path::Path;

use crate::db::ActionDb;

use super::HygieneFixDetail;

/// Resolve unnamed people from sender names stored in the emails table.
///
/// Uses SQLite as the single source of truth and never depends on `_today/data/emails.json`.
pub fn resolve_names_from_emails(
    db: &ActionDb,
    _workspace: &Path,
) -> (usize, Vec<HygieneFixDetail>) {
    // Get unnamed people to match against
    let unnamed = match db.get_unnamed_people() {
        Ok(p) if !p.is_empty() => p,
        _ => return (0, Vec::new()),
    };
    let unnamed_emails: std::collections::HashSet<String> =
        unnamed.iter().map(|p| p.email.to_lowercase()).collect();

    let mut resolved = 0;
    let mut details = Vec::new();

    let mut stmt = match db.conn_ref().prepare(
        "SELECT DISTINCT sender_email, sender_name
         FROM emails
         WHERE sender_email IS NOT NULL
           AND sender_name IS NOT NULL
           AND TRIM(sender_name) != ''",
    ) {
        Ok(stmt) => stmt,
        Err(_) => return (0, Vec::new()),
    };

    let rows = match stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    }) {
        Ok(rows) => rows,
        Err(_) => return (0, Vec::new()),
    };

    for row in rows.flatten() {
        let (addr, display_name) = row;
        let addr = addr.to_lowercase();
        if !unnamed_emails.contains(&addr) {
            continue;
        }

        let cleaned_name = display_name.trim();
        if cleaned_name.is_empty() {
            continue;
        }

        let person_id = crate::util::person_id_from_email(&addr);
        if crate::services::hygiene::update_person_name(db, &person_id, cleaned_name).is_ok() {
            details.push(HygieneFixDetail {
                fix_type: "name_resolved".to_string(),
                entity_name: Some(cleaned_name.to_string()),
                description: format!("Resolved {}'s name from {}", cleaned_name, addr),
            });
            resolved += 1;
        }
    }

    (resolved, details)
}

/// Auto-link people to entities by matching email domain to account names.
///
/// If `schen@acme.com` is a person and there's an account whose name contains
/// "acme", link them via account_stakeholders.
pub fn auto_link_people_by_domain(db: &ActionDb) -> (usize, Vec<HygieneFixDetail>) {
    let accounts = match db.get_all_accounts() {
        Ok(a) => a,
        Err(_) => return (0, Vec::new()),
    };
    if accounts.is_empty() {
        return (0, Vec::new());
    }

    // Build a domain-hint -> (account_id, account_name) map
    // E.g., account "Acme Corp" -> hint "acme"
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
                && crate::services::hygiene::link_person_to_entity(
                    db,
                    &person.id,
                    account_id,
                    "associated",
                    0.75,
                    &format!("domain:{} account:{}", domain, account_name),
                )
                .is_ok()
            {
                details.push(HygieneFixDetail {
                    fix_type: "person_linked_by_domain".to_string(),
                    entity_name: Some(person.name.clone()),
                    description: format!(
                        "Linked {} to {} via {}",
                        person.name, account_name, domain
                    ),
                });
                linked += 1;
                break; // One link per person
            }
        }
    }

    (linked, details)
}

/// Merge duplicate people who share the same local part across aliased domains.
///
/// For each account with 2+ domains, groups people by `(local_part, domain_group)`
/// and merges duplicates. Uses existing `merge_people()` to transfer references.
pub(super) fn dedup_people_by_domain_alias(
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

    // Build a map: domain -> set of sibling domains (via account_domains + user_domains)
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
    // The canonical key is the sorted domain set so that `user@subsidiary.com` and `user@parent.com`
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
            if crate::services::hygiene::merge_people(db, &keep.id, &remove.id, "hygiene_alias")
                .is_ok()
            {
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

/// Auto-merge duplicate people with confidence >= 0.95.
///
/// Uses `detect_duplicate_people()` to find candidates, then merges each pair
/// via the people-merge persistence path. Keeps the person with the higher meeting count.
/// Capped at 10 merges per scan to prevent cascades.
pub(super) fn fix_auto_merge_duplicates(db: &ActionDb) -> (usize, Vec<HygieneFixDetail>) {
    let candidates = match super::detectors::detect_duplicate_people(db) {
        Ok(c) => c,
        Err(_) => return (0, Vec::new()),
    };

    let mut merged = 0;
    let mut details = Vec::new();
    let mut already_merged: std::collections::HashSet<String> = std::collections::HashSet::new();
    const MAX_MERGES: usize = 10;

    for candidate in &candidates {
        if merged >= MAX_MERGES {
            break;
        }
        if candidate.confidence < 0.95 {
            break; // sorted descending, no more high-confidence candidates
        }

        // Skip if either person was already merged this scan
        if already_merged.contains(&candidate.person1_id)
            || already_merged.contains(&candidate.person2_id)
        {
            continue;
        }

        // Determine keep vs. remove: higher meeting_count wins
        let (keep_id, keep_name, remove_id, remove_name) = {
            let p1 = db.get_person(&candidate.person1_id).ok().flatten();
            let p2 = db.get_person(&candidate.person2_id).ok().flatten();
            match (p1, p2) {
                (Some(p1), Some(p2)) => {
                    if p1.meeting_count >= p2.meeting_count {
                        (p1.id, p1.name, p2.id, p2.name)
                    } else {
                        (p2.id, p2.name, p1.id, p1.name)
                    }
                }
                _ => continue,
            }
        };

        if crate::services::hygiene::merge_people(db, &keep_id, &remove_id, "hygiene_auto_merge")
            .is_ok()
        {
            // Emit audit signal on the kept person
            if let Err(err) = crate::services::signals::emit(
                db,
                "person",
                &keep_id,
                "auto_merged",
                "hygiene",
                Some(&format!("merged {} into {}", remove_name, keep_name)),
                candidate.confidence as f64,
            ) {
                log::warn!("hygiene auto-merged signal failed for {}: {}", keep_id, err);
            }

            already_merged.insert(remove_id.clone());
            already_merged.insert(keep_id.clone());

            details.push(HygieneFixDetail {
                fix_type: "people_auto_merged".to_string(),
                entity_name: Some(keep_name.clone()),
                description: format!(
                    "Auto-merged duplicate: {} into {} ({:.0}% confidence)",
                    remove_name,
                    keep_name,
                    candidate.confidence * 100.0
                ),
            });
            merged += 1;
        }
    }

    (merged, details)
}

/// Link people to accounts based on meeting co-attendance patterns.
///
/// If a person attends 3+ meetings that are linked to an account (via
/// `meeting_entities`) but has no `account_stakeholders` link to that account,
/// create the link automatically.
pub(super) fn fix_co_attendance_links(db: &ActionDb) -> (usize, Vec<HygieneFixDetail>) {
    // Find (person_id, entity_id, shared_meeting_count) where the person
    // co-attends meetings linked to an account but has no account_stakeholders link.
    let candidates: Vec<(String, String, String, String, i64)> = db
        .conn_ref()
        .prepare(
            "SELECT ma.person_id, p.name, me.entity_id, a.name, COUNT(*) AS shared
             FROM meeting_attendees ma
             JOIN meeting_entities me ON me.meeting_id = ma.meeting_id AND me.entity_type = 'account'
             JOIN people p ON p.id = ma.person_id AND p.archived = 0 AND p.relationship = 'external'
             JOIN accounts a ON a.id = me.entity_id AND a.archived = 0
             WHERE NOT EXISTS (
                 SELECT 1 FROM account_stakeholders as_
                 WHERE as_.person_id = ma.person_id AND as_.account_id = me.entity_id
             )
             GROUP BY ma.person_id, me.entity_id
             HAVING shared >= 3
             ORDER BY shared DESC",
        )
        .and_then(|mut stmt| {
            let rows = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, i64>(4)?,
                ))
            })?;
            Ok(rows.filter_map(|r| r.ok()).collect())
        })
        .unwrap_or_default();

    let mut linked = 0;
    let mut details = Vec::new();

    for (person_id, person_name, entity_id, account_name, shared_count) in &candidates {
        if crate::services::hygiene::link_person_to_entity(
            db,
            person_id,
            entity_id,
            "co-attendee",
            match *shared_count {
                3..=4 => 0.75,
                5..=9 => 0.85,
                _ => 0.95,
            },
            &format!("{} meetings with {}", shared_count, account_name),
        )
        .is_ok()
        {
            if details.len() < 5 {
                details.push(HygieneFixDetail {
                    fix_type: "person_linked_co_attendance".to_string(),
                    entity_name: Some(person_name.clone()),
                    description: format!(
                        "Linked {} to {} ({} shared meetings)",
                        person_name, account_name, shared_count
                    ),
                });
            }
            linked += 1;
        }
    }

    (linked, details)
}

/// Resolve unnamed people from calendar attendee display names.
///
/// Google Calendar provides display names like "James Giroux" for attendees,
/// but the person record may only have an email-derived name like "jgiroux".
/// This function matches unnamed people against the `attendee_display_names`
/// table (populated during calendar sync) and updates their names.
pub(super) fn resolve_names_from_calendar(db: &ActionDb) -> (usize, Vec<HygieneFixDetail>) {
    // Find unnamed people whose email exists in attendee_display_names
    // with a proper display name (contains a space, no @ sign).
    let candidates: Vec<(String, String, String)> = db
        .conn_ref()
        .prepare(
            "SELECT p.id, p.email, adn.display_name
             FROM people p
             JOIN attendee_display_names adn ON LOWER(adn.email) = LOWER(p.email)
             WHERE p.archived = 0
               AND p.name NOT LIKE '% %'
               AND adn.display_name LIKE '% %'
               AND adn.display_name NOT LIKE '%@%'",
        )
        .and_then(|mut stmt| {
            let rows = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })?;
            Ok(rows.filter_map(|r| r.ok()).collect())
        })
        .unwrap_or_default();

    let mut resolved = 0;
    let mut details = Vec::new();

    for (person_id, email, display_name) in &candidates {
        if crate::services::hygiene::update_person_name(db, person_id, display_name).is_ok() {
            details.push(HygieneFixDetail {
                fix_type: "name_resolved_calendar".to_string(),
                entity_name: Some(display_name.clone()),
                description: format!("Resolved {}'s name from calendar: {}", email, display_name),
            });
            resolved += 1;
        }
    }

    (resolved, details)
}

/// Enqueue AI enrichment for entities with missing or stale intelligence.
/// Respects the daily budget. Returns number of enrichments enqueued.
pub(super) fn enqueue_ai_enrichments(
    db: &ActionDb,
    budget: &crate::state::HygieneBudget,
    queue: &crate::intel_queue::IntelligenceQueue,
) -> usize {
    use crate::intel_queue::{IntelPriority, IntelRequest};

    let mut enqueued = 0;

    // First: entities with no intelligence at all (never enriched)
    if let Ok(missing) = db.get_entities_without_intelligence() {
        for (entity_id, entity_type) in missing {
            // DOS-286: skip archived entities — they shouldn't consume budget
            if db.is_entity_archived(&entity_id, &entity_type) {
                continue;
            }
            if !budget.try_consume() {
                log::debug!(
                    "HygieneLoop: AI budget exhausted ({} used)",
                    budget.used_today()
                );
                return enqueued;
            }
            queue.enqueue(IntelRequest::new(
                entity_id,
                entity_type,
                IntelPriority::ProactiveHygiene,
            ));
            enqueued += 1;
        }
    }

    // Second: entities with stale intelligence (>14 days with new content)
    if let Ok(stale) = db.get_stale_entity_intelligence(14) {
        for (entity_id, entity_type, _enriched_at) in stale {
            // DOS-286: skip archived entities — they shouldn't consume budget
            if db.is_entity_archived(&entity_id, &entity_type) {
                continue;
            }
            if !budget.try_consume() {
                log::debug!(
                    "HygieneLoop: AI budget exhausted ({} used)",
                    budget.used_today()
                );
                return enqueued;
            }
            queue.enqueue(IntelRequest::new(
                entity_id,
                entity_type,
                IntelPriority::ProactiveHygiene,
            ));
            enqueued += 1;
        }
    }

    enqueued
}

/// Enqueue targeted Glean gap-fills for accounts with empty risks.
pub(super) fn enqueue_glean_risk_gap_fills(
    db: &ActionDb,
    budget: &crate::state::HygieneBudget,
    queue: &crate::intel_queue::IntelligenceQueue,
) -> (usize, Vec<HygieneFixDetail>) {
    if !matches!(
        crate::context_provider::read_context_mode(db),
        crate::context_provider::ContextMode::Glean { .. }
    ) {
        return (0, Vec::new());
    }

    let mut stmt = match db.conn_ref().prepare(
        "SELECT a.id, a.name
         FROM accounts a
         INNER JOIN entity_assessment ea ON ea.entity_id = a.id AND ea.entity_type = 'account'
         WHERE a.archived = 0
           AND (
               ea.risks_json IS NULL
               OR TRIM(ea.risks_json) = ''
               OR TRIM(ea.risks_json) = '[]'
           )
         ORDER BY a.updated_at DESC",
    ) {
        Ok(stmt) => stmt,
        Err(_) => return (0, Vec::new()),
    };

    let candidates: Vec<(String, String)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .map(|rows| rows.filter_map(|row| row.ok()).collect())
        .unwrap_or_default();

    let mut enqueued = 0usize;
    let mut details = Vec::new();

    for (entity_id, name) in candidates {
        if !budget.try_consume() {
            break;
        }

        queue.enqueue(crate::intel_queue::IntelRequest::new(
            entity_id,
            "account".to_string(),
            crate::intel_queue::IntelPriority::ProactiveHygiene,
        ));
        details.push(HygieneFixDetail {
            fix_type: "glean_risk_gap_fill".to_string(),
            entity_name: Some(name.clone()),
            description: format!(
                "Queued targeted Glean gap-fill for '{}' because risks were empty",
                name
            ),
        });
        enqueued += 1;
    }

    (enqueued, details)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_utils::test_db;
    use crate::hygiene::tests_common::{seed_account, seed_person};

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

        db.conn_ref().execute(
            "INSERT INTO emails (email_id, thread_id, sender_email, sender_name, subject, snippet, priority, is_unread, received_at, created_at, updated_at)
             VALUES ('em-1', 'th-1', 'jane@customer.com', 'Jane Doe', 'Hi', '', 'high_priority', 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
            [],
        ).unwrap();

        let (resolved, _) = resolve_names_from_emails(&db, Path::new("/unused"));
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

        db.conn_ref().execute(
            "INSERT INTO emails (email_id, thread_id, sender_email, sender_name, subject, snippet, priority, is_unread, received_at, created_at, updated_at)
             VALUES ('em-2', 'th-2', 'jane@customer.com', 'Jane Doe', 'Hi', '', 'high_priority', 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
            [],
        ).unwrap();

        // "Jane Doe" has spaces so not in unnamed set
        let (resolved, _) = resolve_names_from_emails(&db, Path::new("/unused"));
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
}
