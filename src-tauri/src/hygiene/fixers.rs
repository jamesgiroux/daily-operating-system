//! Phase 1 and 1b mechanical fixes: free, instant data corrections.
//!
//! These functions detect and fix data quality issues without AI budget.

use crate::db::ActionDb;

use super::HygieneFixDetail;

/// Reclassify people with "unknown" relationship using the user's domains (I171).
pub(super) fn fix_unknown_relationships(
    db: &ActionDb,
    user_domains: &[String],
) -> (usize, Vec<HygieneFixDetail>) {
    if user_domains.is_empty() {
        return (0, Vec::new());
    }

    let people = match db.get_unknown_relationship_people() {
        Ok(p) => p,
        Err(_) => return (0, Vec::new()),
    };

    let mut fixed = 0;
    let mut details = Vec::new();
    let clock = crate::services::context::SystemClock;
    let rng = crate::services::context::SystemRng;
    let ext = crate::services::context::ExternalClients::default();
    let ctx = crate::services::context::ServiceContext::new_live(&clock, &rng, &ext);
    for person in &people {
        let new_rel = crate::util::classify_relationship_multi(&person.email, user_domains);
        if new_rel != "unknown"
            && crate::services::hygiene::update_person_relationship(
                &ctx, db, &person.id, &new_rel,
            )
                .is_ok()
        {
            details.push(HygieneFixDetail {
                fix_type: "relationship_reclassified".to_string(),
                entity_name: Some(person.name.clone()),
                description: format!(
                    "Reclassified {} ({}) as {}",
                    person.name, person.email, new_rel
                ),
            });
            fixed += 1;
        }
    }
    (fixed, details)
}

/// Extract summaries for content files that have none.
pub(super) fn backfill_file_summaries(db: &ActionDb) -> (usize, Vec<HygieneFixDetail>) {
    let files = match db.get_unsummarized_content_files() {
        Ok(f) => f,
        Err(_) => return (0, Vec::new()),
    };

    // Cap per scan to avoid blocking too long (mechanical extraction is fast but IO-bound)
    let batch_limit = 50;
    let mut extracted = 0;
    let mut details = Vec::new();
    let now = chrono::Utc::now().to_rfc3339();
    let clock = crate::services::context::SystemClock;
    let rng = crate::services::context::SystemRng;
    let ext = crate::services::context::ExternalClients::default();
    let ctx = crate::services::context::ServiceContext::new_live(&clock, &rng, &ext);

    for file in files.iter().take(batch_limit) {
        let path = std::path::Path::new(&file.absolute_path);
        let filename = path
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_default();

        if !path.exists() {
            // File was deleted since indexing -- mark so it exits the unsummarized pool
            let _ = crate::services::hygiene::mark_content_index_summary(
                &ctx,
                db,
                &file.id,
                &now,
                "[file not found]",
            );
            continue;
        }

        let (extracted_at, summary) = crate::intelligence::extract_and_summarize(path);
        match (extracted_at, summary) {
            (Some(ext_at), Some(summ)) => {
                let _ = crate::services::hygiene::mark_content_index_summary(
                    &ctx, db, &file.id, &ext_at, &summ,
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
                // Extraction failed or returned empty -- mark as attempted so the file
                // doesn't reappear as an unsummarized gap on every scan forever.
                let _ = crate::services::hygiene::mark_content_index_summary(
                    &ctx,
                    db,
                    &file.id,
                    &now,
                    "[extraction failed]",
                );
            }
        }
    }
    (extracted, details)
}

/// Recompute meeting counts for people whose counts may be stale.
/// Only fixes people whose stored count differs from actual attendee records.
pub(super) fn fix_meeting_counts(db: &ActionDb) -> (usize, Vec<HygieneFixDetail>) {
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
        if crate::services::hygiene::recompute_person_meeting_count(db, person_id).is_ok() {
            details.push(HygieneFixDetail {
                fix_type: "meeting_count_updated".to_string(),
                entity_name: Some(name.clone()),
                description: format!(
                    "Updated meeting count for {}: {} \u{2192} {}",
                    name, old_count, new_count
                ),
            });
            fixed += 1;
        }
    }
    (fixed, details)
}

/// Auto-rollover renewal dates for accounts that passed their renewal without churning.
///
/// For each non-archived account whose `contract_end` is in the past:
///   1. Skip if the account has a 'churn' event (defensive -- `get_accounts_past_renewal`
///      already filters these, but this guards against race conditions).
///   2. Record a 'renewal' event with the original contract_end date and current ARR.
///   3. Advance `contract_end` by 12 months.
///
/// This ensures renewals don't silently go stale when the user simply continues the
/// relationship without explicitly recording the event.
pub(super) fn fix_renewal_rollovers(db: &ActionDb) -> (usize, Vec<HygieneFixDetail>) {
    let past_renewal = match db.get_accounts_past_renewal() {
        Ok(accounts) => accounts,
        Err(_) => return (0, Vec::new()),
    };

    let mut fixed = 0;
    let mut details = Vec::new();
    let clock = crate::services::context::SystemClock;
    let rng = crate::services::context::SystemRng;
    let ext = crate::services::context::ExternalClients::default();
    let ctx = crate::services::context::ServiceContext::new_live(&clock, &rng, &ext);
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

        // Advance contract_end by 12 months
        let next = parsed + chrono::Months::new(12);
        let next_str = next.format("%Y-%m-%d").to_string();
        if crate::services::hygiene::rollover_account_renewal(
            &ctx,
            db,
            &account.id,
            &account.name,
            renewal_date,
            account.arr,
            &next_str,
        )
        .is_err()
        {
            continue;
        }
        details.push(HygieneFixDetail {
            fix_type: "renewal_rolled_over".to_string(),
            entity_name: Some(account.name.clone()),
            description: format!(
                "Rolled over {} renewal: {} \u{2192} {}",
                account.name, renewal_date, next_str
            ),
        });
        fixed += 1;
    }

    (fixed, details)
}

/// Retry abandoned Quill syncs that are between 7 and 14 days old.
pub(super) fn retry_abandoned_quill_syncs(db: &ActionDb) -> (usize, Vec<HygieneFixDetail>) {
    let syncs = match db.get_retryable_abandoned_quill_syncs(7, 14) {
        Ok(s) => s,
        Err(_) => return (0, Vec::new()),
    };

    let mut retried = 0;
    let mut details = Vec::new();
    let clock = crate::services::context::SystemClock;
    let rng = crate::services::context::SystemRng;
    let ext = crate::services::context::ExternalClients::default();
    let ctx = crate::services::context::ServiceContext::new_live(&clock, &rng, &ext);
    for sync_row in &syncs {
        if crate::services::hygiene::reset_quill_sync_for_retry(&ctx, db, &sync_row.id).is_ok() {
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
// Phase 1b: Account Cleanup
// =============================================================================

/// Archive phantom accounts -- structural folders (like "Internal") that were
/// incorrectly bootstrapped as standalone accounts during workspace sync.
/// A phantom account is: named "Internal", not flagged as internal, and has
/// no meetings, actions, or people linked.
pub(super) fn archive_phantom_accounts(db: &ActionDb) -> (usize, Vec<HygieneFixDetail>) {
    let conn = db.conn_ref();

    // Find accounts named "Internal" (case-insensitive) that are NOT the real
    // internal org root (account_type != 'internal') and have zero activity.
    let phantoms: Vec<(String, String)> = conn
        .prepare(
            "SELECT a.id, a.name FROM accounts a
             WHERE LOWER(a.name) = 'internal'
               AND a.account_type != 'internal'
               AND a.archived = 0
               AND NOT EXISTS (SELECT 1 FROM meeting_entities me WHERE me.entity_id = a.id AND me.entity_type = 'account')
               AND NOT EXISTS (SELECT 1 FROM actions act WHERE act.account_id = a.id)
               AND NOT EXISTS (SELECT 1 FROM account_stakeholders as_ WHERE as_.account_id = a.id)",
        )
        .and_then(|mut stmt| {
            stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
                .map(|rows| rows.filter_map(|r| r.ok()).collect())
        })
        .unwrap_or_default();

    let mut archived = 0;
    let mut details = Vec::new();
    let now = chrono::Utc::now().to_rfc3339();
    for (id, name) in &phantoms {
        if conn
            .execute(
                "UPDATE accounts SET archived = 1, updated_at = ?2 WHERE id = ?1",
                rusqlite::params![id, now],
            )
            .is_ok()
        {
            details.push(HygieneFixDetail {
                fix_type: "phantom_account_archived".to_string(),
                entity_name: Some(name.clone()),
                description: format!(
                    "Archived phantom account '{}' (structural folder, no activity)",
                    name
                ),
            });
            archived += 1;
        }
    }

    (archived, details)
}

/// Re-link orphan internal accounts to the internal root.
/// An orphan internal account has account_type = 'internal' but parent_id IS NULL and
/// is not the root account itself (i.e., there's another internal account that IS the root).
pub(super) fn relink_orphan_internal_accounts(db: &ActionDb) -> (usize, Vec<HygieneFixDetail>) {
    let root = match db.get_internal_root_account() {
        Ok(Some(r)) => r,
        _ => return (0, Vec::new()),
    };

    let conn = db.conn_ref();

    // Find internal accounts with no parent that aren't the root
    let orphans: Vec<(String, String)> = conn
        .prepare(
            "SELECT id, name FROM accounts
             WHERE account_type = 'internal' AND parent_id IS NULL AND archived = 0 AND id != ?1",
        )
        .and_then(|mut stmt| {
            stmt.query_map(rusqlite::params![root.id], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .map(|rows| rows.filter_map(|r| r.ok()).collect())
        })
        .unwrap_or_default();

    let mut relinked = 0;
    let mut details = Vec::new();
    let now = chrono::Utc::now().to_rfc3339();
    for (id, name) in &orphans {
        if conn
            .execute(
                "UPDATE accounts SET parent_id = ?2, updated_at = ?3 WHERE id = ?1",
                rusqlite::params![id, root.id, now],
            )
            .is_ok()
        {
            details.push(HygieneFixDetail {
                fix_type: "orphan_internal_relinked".to_string(),
                entity_name: Some(name.clone()),
                description: format!(
                    "Re-linked orphan internal account '{}' under '{}'",
                    name, root.name
                ),
            });
            relinked += 1;
        }
    }

    (relinked, details)
}

/// Archive empty shell accounts that have no meetings, no actions, no people,
/// and were created more than 30 days ago.
pub(super) fn archive_empty_shell_accounts(db: &ActionDb) -> (usize, Vec<HygieneFixDetail>) {
    let conn = db.conn_ref();

    let shells: Vec<(String, String)> = conn
        .prepare(
            "SELECT a.id, a.name FROM accounts a
             WHERE a.archived = 0
               AND a.updated_at <= datetime('now', '-30 days')
               AND NOT EXISTS (SELECT 1 FROM meeting_entities me WHERE me.entity_id = a.id AND me.entity_type = 'account')
               AND NOT EXISTS (SELECT 1 FROM actions act WHERE act.account_id = a.id)
               AND NOT EXISTS (SELECT 1 FROM account_stakeholders as_ WHERE as_.account_id = a.id)
               AND NOT EXISTS (SELECT 1 FROM account_events ae WHERE ae.account_id = a.id)
               AND NOT EXISTS (SELECT 1 FROM email_signals es WHERE es.entity_id = a.id AND es.entity_type = 'account' AND es.deactivated_at IS NULL)",
        )
        .and_then(|mut stmt| {
            stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
                .map(|rows| rows.filter_map(|r| r.ok()).collect())
        })
        .unwrap_or_default();

    let mut archived = 0;
    let mut details = Vec::new();
    let now = chrono::Utc::now().to_rfc3339();
    for (id, name) in &shells {
        if conn
            .execute(
                "UPDATE accounts SET archived = 1, updated_at = ?2 WHERE id = ?1",
                rusqlite::params![id, now],
            )
            .is_ok()
        {
            details.push(HygieneFixDetail {
                fix_type: "empty_shell_archived".to_string(),
                entity_name: Some(name.clone()),
                description: format!(
                    "Archived empty shell account '{}' (no activity after 30 days)",
                    name
                ),
            });
            archived += 1;
        }
    }

    (archived, details)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_utils::test_db;
    use crate::hygiene::tests_common::{
        default_test_config, seed_account_with_renewal, seed_person,
    };
    use chrono::Utc;
    use std::path::Path;

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

    // --- Renewal auto-rollover tests (I143) ---

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
        assert_eq!(report.fixes.renewals_rolled_over, 1);
    }

    #[test]
    fn test_full_scan_empty_db() {
        let db = test_db();
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

        // Fixes applied
        assert_eq!(report.fixes.relationships_reclassified, 2);

        // Post-fix gap count: both resolved, so 0 remaining
        assert_eq!(report.unknown_relationships, 0);

        // Verify actual state
        let p1 = db.get_person("p1").unwrap().unwrap();
        assert_eq!(p1.relationship, "internal");
    }
}
