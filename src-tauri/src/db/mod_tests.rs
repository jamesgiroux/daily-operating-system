use super::test_utils::test_db;
use super::*;
use crate::entity::{DbEntity, EntityType};
use chrono::Utc;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use strsim::jaro_winkler;

fn sample_action(id: &str, title: &str) -> DbAction {
    let now = Utc::now().to_rfc3339();
    DbAction {
        id: id.to_string(),
        title: title.to_string(),
        priority: crate::action_status::PRIORITY_MEDIUM,
        status: crate::action_status::UNSTARTED.to_string(),
        created_at: now.clone(),
        due_date: None,
        completed_at: None,
        account_id: None,
        project_id: None,
        source_type: None,
        source_id: None,
        source_label: None,
        context: None,
        waiting_on: None,
        updated_at: now,
        person_id: None,
        account_name: None,
        next_meeting_title: None,
        next_meeting_start: None,
        needs_decision: false,
        decision_owner: None,
        decision_stakes: None,
        linear_identifier: None,
        linear_url: None,
    }
}

fn sample_email(id: &str, thread_id: &str, subject: &str) -> DbEmail {
    let now = Utc::now().to_rfc3339();
    DbEmail {
        email_id: id.to_string(),
        thread_id: Some(thread_id.to_string()),
        sender_email: Some("owner@example.com".to_string()),
        sender_name: Some("Owner".to_string()),
        subject: Some(subject.to_string()),
        snippet: Some("snippet".to_string()),
        priority: Some("high".to_string()),
        is_unread: true,
        received_at: Some(now.clone()),
        enrichment_state: "enriched".to_string(),
        enrichment_attempts: 0,
        last_enrichment_at: None,
        enriched_at: Some(now.clone()),
        last_seen_at: Some(now.clone()),
        resolved_at: None,
        entity_id: Some("acc-1".to_string()),
        entity_type: Some("account".to_string()),
        contextual_summary: Some("summary".to_string()),
        sentiment: None,
        urgency: None,
        user_is_last_sender: false,
        last_sender_email: Some("owner@example.com".to_string()),
        message_count: 1,
        created_at: now.clone(),
        updated_at: now,
        relevance_score: Some(0.9),
        score_reason: Some("important".to_string()),
        pinned_at: None,
        commitments: None,
        questions: None,
    }
}

#[test]
fn test_open_creates_tables() {
    let db = test_db();
    // Verify tables exist by querying them (should not error)
    let count: i32 = db
        .conn
        .query_row("SELECT COUNT(*) FROM actions", [], |row| row.get(0))
        .expect("actions table should exist");
    assert_eq!(count, 0);

    let count: i32 = db
        .conn
        .query_row("SELECT COUNT(*) FROM accounts", [], |row| row.get(0))
        .expect("accounts table should exist");
    assert_eq!(count, 0);

    let count: i32 = db
        .conn
        .query_row("SELECT COUNT(*) FROM meetings", [], |row| row.get(0))
        .expect("meetings table should exist");
    assert_eq!(count, 0);
}

#[test]
fn test_upsert_and_query_action() {
    let db = test_db();

    let action = sample_action("act-001", "Follow up with Acme");
    db.upsert_action(&action).expect("upsert should succeed");

    let results = db.get_due_actions(7).expect("query should succeed");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, "act-001");
    assert_eq!(results[0].title, "Follow up with Acme");
}

#[test]
fn test_upsert_updates_existing() {
    let db = test_db();

    let mut action = sample_action("act-002", "Original title");
    db.upsert_action(&action).expect("first upsert");

    action.title = "Updated title".to_string();
    action.priority = crate::action_status::PRIORITY_URGENT;
    db.upsert_action(&action).expect("second upsert");

    let results = db.get_due_actions(7).expect("query");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].title, "Updated title");
    assert_eq!(results[0].priority, crate::action_status::PRIORITY_URGENT);
}

#[test]
fn test_complete_action() {
    let db = test_db();

    let action = sample_action("act-003", "Task to complete");
    db.upsert_action(&action).expect("upsert");

    db.complete_action("act-003").expect("complete");

    // Should no longer appear in pending results
    let results = db.get_due_actions(7).expect("query");
    assert_eq!(results.len(), 0);

    // Verify directly that status changed
    let status: String = db
        .conn
        .query_row(
            "SELECT status FROM actions WHERE id = 'act-003'",
            [],
            |row| row.get(0),
        )
        .expect("direct query");
    assert_eq!(status, "completed");

    // Verify completed_at was set
    let completed_at: Option<String> = db
        .conn
        .query_row(
            "SELECT completed_at FROM actions WHERE id = 'act-003'",
            [],
            |row| row.get(0),
        )
        .expect("direct query");
    assert!(completed_at.is_some());
}

#[test]
fn test_archive_email_archives_entire_thread() {
    let db = test_db();

    let latest = sample_email("em-thread-1-latest", "thread-1", "Latest");
    let older = sample_email("em-thread-1-older", "thread-1", "Older");
    db.upsert_email(&latest).expect("upsert latest");
    db.upsert_email(&older).expect("upsert older");

    db.archive_email("em-thread-1-latest")
        .expect("archive thread");

    let active = db.get_all_active_emails().expect("active emails");
    assert!(
        active.is_empty(),
        "thread should be fully hidden once archived"
    );

    db.unarchive_email("em-thread-1-latest")
        .expect("unarchive thread");

    let active = db.get_all_active_emails().expect("active after undo");
    assert_eq!(active.len(), 2, "undo should restore the full thread");
}

#[test]
fn test_dos31_last_successful_fetch_at_roundtrip() {
    // DOS-31: the `email_sync_meta` singleton row must be seeded by migration 094
    // and round-trip through the get/set API. Starts as None, becomes Some(now)
    // after a recorded successful fetch.
    let db = test_db();

    let initial = db
        .get_last_successful_fetch_at()
        .expect("meta query should succeed even when value is NULL");
    assert_eq!(initial, None, "meta row seeded with NULL last_successful_fetch_at");

    db.set_last_successful_fetch_at()
        .expect("set_last_successful_fetch_at should succeed");

    let after = db
        .get_last_successful_fetch_at()
        .expect("meta query after set");
    assert!(after.is_some(), "timestamp should be populated after set");
}

#[test]
fn test_dos31_sync_stats_includes_last_successful_fetch_at() {
    // DOS-31 AC: `get_email_sync_stats` exposes the separate
    // `last_successful_fetch_at` timestamp so the UI can distinguish a healthy
    // fetch from a stalled enrichment pipeline.
    let db = test_db();
    db.set_last_successful_fetch_at().expect("record fetch");

    let stats = db.get_email_sync_stats().expect("sync stats");
    assert!(
        stats.last_successful_fetch_at.is_some(),
        "stats must surface last_successful_fetch_at after a recorded fetch"
    );
}

/// Seed a single email row in the `failed` state at the 3-attempt cap.
/// Centralized so the DOS-226 rollback/finalize tests stay readable.
#[cfg(test)]
fn seed_failed_email(db: &ActionDb, email_id: &str) {
    let mut stuck = sample_email(email_id, "thread-stuck", "Stuck email");
    stuck.enrichment_state = "failed".to_string();
    stuck.enrichment_attempts = 3;
    db.upsert_email(&stuck).expect("upsert stuck");
    // upsert_email doesn't persist enrichment_state/attempts on conflict; force
    // the row into the expected failed/3 state.
    db.conn
        .execute(
            "UPDATE emails SET enrichment_state = 'failed', enrichment_attempts = 3 WHERE email_id = ?1",
            rusqlite::params![email_id],
        )
        .expect("seed failed state");
}

#[cfg(test)]
fn read_enrichment_state(db: &ActionDb, email_id: &str) -> (String, i32) {
    db.conn
        .query_row(
            "SELECT enrichment_state, enrichment_attempts FROM emails WHERE email_id = ?1",
            rusqlite::params![email_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("read back state")
}

#[test]
fn test_dos226_mark_failed_for_retry_preserves_attempts() {
    // DOS-226: the retry transition must preserve enrichment_attempts so a
    // rollback (refresh failure) can restore the row to exactly its pre-retry
    // state. Counting the row as failed in the UI also depends on the
    // attempts cap staying at 3.
    let db = test_db();
    seed_failed_email(&db, "em-stuck-1");

    let marked = db.mark_failed_for_retry("batch-a").expect("mark");
    assert_eq!(marked, 1);

    let (state, attempts) = read_enrichment_state(&db, "em-stuck-1");
    assert_eq!(state, "pending_retry");
    assert_eq!(attempts, 3, "attempts must be preserved for rollback");
}

#[test]
fn test_dos226_pending_retry_still_counts_as_failed_in_stats() {
    // DOS-226: while a retry is in flight, the UI must keep rendering the
    // Retry notice. That means `get_email_sync_stats().failed` needs to
    // include rows in the transitional `pending_retry` state.
    let db = test_db();
    seed_failed_email(&db, "em-stuck-1");
    db.mark_failed_for_retry("batch-b").expect("mark");

    let stats = db.get_email_sync_stats().expect("stats");
    assert_eq!(
        stats.failed, 1,
        "pending_retry must count as failed for UI purposes"
    );
}

#[test]
fn test_dos226_rollback_restores_failed_state() {
    // DOS-226: If the Gmail refresh fails mid-retry, the rows must return
    // to `failed` with their original `enrichment_attempts` so the user
    // can retry again (and the Retry notice never silently disappeared).
    let db = test_db();
    seed_failed_email(&db, "em-stuck-1");

    db.mark_failed_for_retry("batch-c").expect("mark");
    let rolled = db.rollback_pending_retry("batch-c").expect("rollback");
    assert_eq!(rolled, 1);

    let (state, attempts) = read_enrichment_state(&db, "em-stuck-1");
    assert_eq!(state, "failed");
    assert_eq!(attempts, 3, "attempts must be restored");
}

#[test]
fn test_dos226_finalize_success_promotes_to_pending_and_zeros_attempts() {
    // DOS-226: On refresh success, `pending_retry` rows must become `pending`
    // with zeroed attempts so the enrichment pipeline actually re-runs them
    // (it skips rows that have reached the 3-attempt cap).
    let db = test_db();
    seed_failed_email(&db, "em-stuck-1");

    db.mark_failed_for_retry("batch-d").expect("mark");
    let promoted = db.finalize_pending_retry_success("batch-d").expect("finalize");
    assert_eq!(promoted, 1);

    let (state, attempts) = read_enrichment_state(&db, "em-stuck-1");
    assert_eq!(state, "pending");
    assert_eq!(attempts, 0);
}

#[test]
fn test_dos226_finalize_is_scoped_to_batch_id() {
    // Codex finding 2: a finalize from refresh A must not accidentally
    // adopt rows owned by refresh B. If batching were ignored, a rapid
    // double-refresh could drop rows into `pending` with attempts=0 that
    // the first refresh had not yet finished processing.
    let db = test_db();
    seed_failed_email(&db, "em-batch-a-1");
    seed_failed_email(&db, "em-batch-a-2");

    db.mark_failed_for_retry("batch-A").expect("mark A");

    // Simulate a second refresh arriving before A finalizes.
    let bogus = db
        .finalize_pending_retry_success("batch-B")
        .expect("finalize B");
    assert_eq!(bogus, 0, "finalize for unknown batch must touch zero rows");

    // A's rows are still in pending_retry.
    let (state, _) = read_enrichment_state(&db, "em-batch-a-1");
    assert_eq!(state, "pending_retry");

    // A finalizes its own batch correctly.
    let promoted = db
        .finalize_pending_retry_success("batch-A")
        .expect("finalize A");
    assert_eq!(promoted, 2);
}

#[test]
fn test_dos226_rollback_is_scoped_to_batch_id() {
    // Codex finding 2: rollback from one batch must not clobber another
    // batch's in-flight rows.
    let db = test_db();
    seed_failed_email(&db, "em-rb-1");

    db.mark_failed_for_retry("batch-X").expect("mark X");
    let rolled = db.rollback_pending_retry("batch-Y").expect("rollback Y");
    assert_eq!(rolled, 0, "rollback for unknown batch is a no-op");

    let (state, _) = read_enrichment_state(&db, "em-rb-1");
    assert_eq!(state, "pending_retry", "X's row must be untouched");
}

#[test]
fn test_dos226_rollback_stale_pending_retry_recovers_orphans() {
    // Codex finding 2: simulate a crash between mark_failed_for_retry and
    // refresh completion. The row is stranded in pending_retry with a
    // batch_id that will never be finalized or rolled back by its owner.
    // Phase-0 recovery on the next refresh must promote it back to `failed`
    // so retry_failed_emails() will actually re-run it.
    let db = test_db();
    seed_failed_email(&db, "em-crashed");

    db.mark_failed_for_retry("batch-crashed").expect("mark");

    // Backdate retry_started_at to simulate staleness (older than the
    // recovery bound). Without this the row would be considered fresh.
    db.conn
        .execute(
            "UPDATE emails SET retry_started_at = '2020-01-01T00:00:00Z' \
             WHERE email_id = 'em-crashed'",
            [],
        )
        .expect("backdate");

    // `retry_failed_emails` counted only `failed` before the fix. Verify
    // the count_retriable helper includes pending_retry rows so a user
    // clicking Retry still triggers a refresh (the refresh's phase-0
    // recovery will then roll this row back to failed).
    let retriable = db.count_retriable_emails().expect("count retriable");
    assert_eq!(
        retriable, 1,
        "count_retriable must include pending_retry so orphans aren't invisible to Retry"
    );

    // Run the stale-recovery pass with a short bound.
    let recovered = db
        .rollback_stale_pending_retry(60)
        .expect("recover stale");
    assert_eq!(recovered, 1);

    let (state, attempts) = read_enrichment_state(&db, "em-crashed");
    assert_eq!(state, "failed", "stale pending_retry must be rolled back");
    assert_eq!(attempts, 3, "attempts untouched during recovery");

    // Columns cleared so the recovered row can participate in a fresh batch.
    let (batch_id, started_at): (Option<String>, Option<String>) = db
        .conn
        .query_row(
            "SELECT retry_batch_id, retry_started_at FROM emails WHERE email_id = 'em-crashed'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("read cols");
    assert!(batch_id.is_none(), "retry_batch_id cleared on recovery");
    assert!(started_at.is_none(), "retry_started_at cleared on recovery");
}

#[test]
fn test_dos226_rollback_stale_respects_fresh_batches() {
    // Codex finding 2: a concurrent refresh's fresh rows must survive the
    // recovery pass — only stale rows are rolled back.
    let db = test_db();
    seed_failed_email(&db, "em-fresh");

    db.mark_failed_for_retry("batch-fresh").expect("mark");

    let recovered = db
        .rollback_stale_pending_retry(600)
        .expect("recover stale");
    assert_eq!(recovered, 0, "fresh batches must not be disturbed");

    let (state, _) = read_enrichment_state(&db, "em-fresh");
    assert_eq!(state, "pending_retry");
}

#[test]
fn test_dos226_pending_retry_eligible_for_enrichment_after_finalize() {
    // Codex finding 1 regression guard: the whole retry loop hinges on
    // the retried row becoming eligible for the next enrichment pass.
    // Before this fix, `pending_retry` at attempts=3 would be filtered
    // out by `get_pending_enrichment` (which requires attempts < 3), so
    // the user-visible "retrying" state translated to zero work done.
    //
    // After finalize, the row must be state=pending + attempts=0 and
    // therefore selectable by `get_pending_enrichment`.
    let db = test_db();
    seed_failed_email(&db, "em-e2e");

    // Sanity: attempts=3 in `failed` is selected by get_pending_enrichment
    // (the query matches failed|pending|pending_retry). But the important
    // case for finding 1 is post-finalize eligibility, below.
    db.mark_failed_for_retry("batch-e2e").expect("mark");
    db.finalize_pending_retry_success("batch-e2e")
        .expect("finalize");

    let pending = db.get_pending_enrichment(10).expect("query pending");
    assert!(
        pending.iter().any(|e| e.email_id == "em-e2e"),
        "finalized retry row must be visible to enrichment; \
         otherwise the user-facing retry is a no-op"
    );
    let entry = pending.iter().find(|e| e.email_id == "em-e2e").unwrap();
    assert_eq!(entry.enrichment_state, "pending");
    assert_eq!(entry.enrichment_attempts, 0);
}

#[test]
fn test_dos226_set_last_successful_fetch_at_is_upsert() {
    // DOS-226: If the `email_sync_meta` singleton row is missing (fresh DB
    // that skipped the seed insert, partial restore, etc.), the timestamp
    // write must still materialize. The old UPDATE-only path silently
    // no-op'd, leaving the UI stuck on "never fetched".
    let db = test_db();

    // Simulate the missing-singleton case — this is the exact drift the fix
    // defends against.
    db.conn
        .execute("DELETE FROM email_sync_meta WHERE id = 1", [])
        .expect("delete seed");

    db.set_last_successful_fetch_at()
        .expect("upsert must succeed even when seed row is missing");

    let stamped = db
        .get_last_successful_fetch_at()
        .expect("read back")
        .expect("timestamp must be populated after upsert");
    assert!(!stamped.is_empty());
}

#[test]
fn test_dos226_get_email_sync_stats_propagates_errors() {
    // DOS-226: `get_email_sync_stats` previously swallowed errors from the
    // meta query as `Ok(None)`, masking schema drift (e.g. migration 094
    // never applied) as a benign "never fetched". Verify errors bubble up.
    let db = test_db();

    // Simulate schema drift: drop the meta table entirely. `get_last_...`
    // must now return Err, and the stats query must propagate it.
    db.conn
        .execute("DROP TABLE email_sync_meta", [])
        .expect("drop meta");

    let result = db.get_email_sync_stats();
    assert!(
        result.is_err(),
        "sync stats must propagate unexpected meta query errors"
    );
}

#[test]
fn test_get_account_actions() {
    let db = test_db();

    let mut action1 = sample_action("act-010", "Acme task");
    action1.account_id = Some("acme-corp".to_string());
    db.upsert_action(&action1).expect("upsert 1");

    let mut action2 = sample_action("act-011", "Beta task");
    action2.account_id = Some("beta-inc".to_string());
    db.upsert_action(&action2).expect("upsert 2");

    let mut action3 = sample_action("act-012", "Acme pending-delegated");
    action3.account_id = Some("acme-corp".to_string());
    action3.waiting_on = Some("John".to_string());
    db.upsert_action(&action3).expect("upsert 3");

    let mut action4 = sample_action("act-013", "Acme suggested");
    action4.account_id = Some("acme-corp".to_string());
    action4.status = crate::action_status::BACKLOG.to_string();
    db.upsert_action(&action4).expect("upsert 4");

    let results = db.get_account_actions("acme-corp").expect("account query");
    assert_eq!(results.len(), 3);
    // Backlog and unstarted should appear
    let statuses: Vec<&str> = results.iter().map(|a| a.status.as_str()).collect();
    assert!(statuses.contains(&crate::action_status::BACKLOG));
    assert!(statuses.contains(&crate::action_status::UNSTARTED));
}

#[test]
fn test_upsert_and_query_account() {
    let db = test_db();

    let now = Utc::now().to_rfc3339();
    let account = DbAccount {
        id: "acme-corp".to_string(),
        name: "Acme Corp".to_string(),
        lifecycle: Some("steady-state".to_string()),
        arr: Some(120_000.0),
        health: Some("green".to_string()),
        contract_start: Some("2025-01-01".to_string()),
        contract_end: Some("2026-01-01".to_string()),
        nps: None,
        tracker_path: Some("Accounts/acme-corp".to_string()),
        parent_id: None,
        account_type: crate::db::AccountType::Customer,
        updated_at: now,
        archived: false,
        keywords: None,
        keywords_extracted_at: None,
        metadata: None,
        ..Default::default()
    };

    db.upsert_account(&account).expect("upsert account");

    let result = db.get_account("acme-corp").expect("get account");
    assert!(result.is_some());
    let acct = result.unwrap();
    assert_eq!(acct.name, "Acme Corp");
    assert_eq!(acct.lifecycle, Some("steady-state".to_string()));
    assert_eq!(acct.arr, Some(120_000.0));
}

#[test]
fn test_get_account_not_found() {
    let db = test_db();
    let result = db.get_account("nonexistent").expect("get account");
    assert!(result.is_none());
}

#[test]
fn test_get_all_accounts_excludes_archived() {
    let db = test_db();
    let now = Utc::now().to_rfc3339();

    let active = DbAccount {
        id: "active-corp".to_string(),
        name: "Active Corp".to_string(),
        lifecycle: None,
        arr: None,
        health: None,
        contract_start: None,
        contract_end: None,
        nps: None,
        tracker_path: None,
        parent_id: None,
        account_type: crate::db::AccountType::Customer,
        updated_at: now.clone(),
        archived: false,
        keywords: None,
        keywords_extracted_at: None,
        metadata: None,
        ..Default::default()
    };

    let archived = DbAccount {
        id: "archived-corp".to_string(),
        name: "Archived Corp".to_string(),
        lifecycle: None,
        arr: None,
        health: None,
        contract_start: None,
        contract_end: None,
        nps: None,
        tracker_path: None,
        parent_id: None,
        account_type: crate::db::AccountType::Customer,
        updated_at: now,
        archived: true,
        keywords: None,
        keywords_extracted_at: None,
        metadata: None,
        ..Default::default()
    };

    db.upsert_account(&active).expect("upsert active");
    db.upsert_account(&archived).expect("upsert archived");

    let results = db.get_all_accounts().expect("get all");
    assert_eq!(results.len(), 1, "should only return active account");
    assert_eq!(results[0].id, "active-corp");
    assert!(!results[0].archived);
}

#[test]
fn test_upsert_and_query_meeting() {
    let db = test_db();

    let now = Utc::now().to_rfc3339();
    let meeting = DbMeeting {
        id: "mtg-001".to_string(),
        title: "Acme QBR".to_string(),
        meeting_type: "customer".to_string(),
        start_time: now.clone(),
        end_time: None,
        attendees: Some(r#"["alice@acme.com","bob@us.com"]"#.to_string()),
        notes_path: None,
        summary: Some("Discussed renewal".to_string()),
        created_at: now,
        calendar_event_id: Some("gcal-evt-001".to_string()),
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
    };

    db.upsert_meeting(&meeting).expect("upsert meeting");
    db.link_meeting_entity("mtg-001", "acme-corp", "account")
        .expect("link meeting entity");

    let results = db
        .get_meeting_history("acme-corp", 30, 10)
        .expect("meeting history");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].title, "Acme QBR");
    assert_eq!(results[0].summary, Some("Discussed renewal".to_string()));
}

#[test]
fn test_meeting_history_respects_limit() {
    let db = test_db();
    let now = Utc::now().to_rfc3339();

    for i in 0..5 {
        let mid = format!("mtg-{i:03}");
        let meeting = DbMeeting {
            id: mid.clone(),
            title: format!("Meeting {i}"),
            meeting_type: "customer".to_string(),
            start_time: now.clone(),
            end_time: None,
            attendees: None,
            notes_path: None,
            summary: None,
            created_at: now.clone(),
            calendar_event_id: None,
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
        };
        db.upsert_meeting(&meeting).expect("upsert");
        db.link_meeting_entity(&mid, "acme-corp", "account")
            .expect("link");
    }

    let results = db.get_meeting_history("acme-corp", 30, 3).expect("history");
    assert_eq!(results.len(), 3);
}

#[test]
fn test_due_actions_ordering() {
    let db = test_db();

    // P2 with no due date (should appear because due_date IS NULL)
    let action_no_date = sample_action("act-a", "No date task");
    db.upsert_action(&action_no_date).expect("upsert");

    // P1 with future due date
    let mut action_p1 = sample_action("act-b", "P1 future task");
    action_p1.priority = crate::action_status::PRIORITY_URGENT;
    action_p1.due_date = Some("2099-12-31".to_string());
    db.upsert_action(&action_p1).expect("upsert");

    // P1 overdue
    let mut action_overdue = sample_action("act-c", "Overdue task");
    action_overdue.priority = crate::action_status::PRIORITY_URGENT;
    action_overdue.due_date = Some("2020-01-01".to_string());
    db.upsert_action(&action_overdue).expect("upsert");

    let results = db.get_due_actions(365_000).expect("query");
    assert_eq!(results.len(), 3);
    // Overdue should be first
    assert_eq!(results[0].id, "act-c");
}

#[test]
fn test_mark_prep_reviewed() {
    let db = test_db();

    db.mark_prep_reviewed("gcal-evt-1", Some("gcal-evt-1"), "Acme Sync")
        .expect("mark reviewed");

    let reviewed = db.get_reviewed_preps().expect("get reviewed");
    assert_eq!(reviewed.len(), 1);
    assert!(reviewed.contains_key("gcal-evt-1"));
}

#[test]
fn test_mark_prep_reviewed_upsert() {
    let db = test_db();

    db.mark_prep_reviewed("0900-acme", None, "Acme")
        .expect("first mark");
    db.mark_prep_reviewed("0900-acme", Some("evt-1"), "Acme")
        .expect("second mark (upsert)");

    let reviewed = db.get_reviewed_preps().expect("get reviewed");
    assert_eq!(reviewed.len(), 1);
}

#[test]
fn test_freeze_meeting_prep_snapshot_is_idempotent() {
    let db = test_db();
    let meeting = DbMeeting {
        id: "evt-1".to_string(),
        title: "Acme Sync".to_string(),
        meeting_type: "customer".to_string(),
        start_time: Utc::now().to_rfc3339(),
        end_time: None,
        attendees: None,
        notes_path: None,
        summary: None,
        created_at: Utc::now().to_rfc3339(),
        calendar_event_id: Some("evt-1".to_string()),
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
    };
    db.upsert_meeting(&meeting).expect("upsert meeting");

    let first = db
        .freeze_meeting_prep_snapshot(
            "evt-1",
            "{\"k\":\"v\"}",
            "2026-02-12T10:00:00Z",
            "/tmp/snapshot.json",
            "hash-1",
        )
        .expect("first freeze");
    let second = db
        .freeze_meeting_prep_snapshot(
            "evt-1",
            "{\"k\":\"override\"}",
            "2026-02-12T11:00:00Z",
            "/tmp/snapshot-2.json",
            "hash-2",
        )
        .expect("second freeze");
    assert!(first);
    assert!(!second);

    let persisted = db
        .get_meeting_by_id("evt-1")
        .expect("query")
        .expect("row exists");
    assert_eq!(persisted.prep_snapshot_hash.as_deref(), Some("hash-1"));
}

#[test]
fn test_upsert_action_title_dedup() {
    let db = test_db();

    // Insert and complete an action under one ID
    let mut action = sample_action("briefing-001", "Follow up with Acme");
    action.account_id = Some("acme".to_string());
    db.upsert_action(&action).expect("insert");
    db.complete_action("briefing-001").expect("complete");

    // Try to insert the same action under a different ID (cross-source)
    let action2 = DbAction {
        id: "postmeet-999".to_string(),
        title: "Follow up with Acme".to_string(),
        account_id: Some("acme".to_string()),
        ..sample_action("postmeet-999", "Follow up with Acme")
    };
    db.upsert_action_if_not_completed(&action2)
        .expect("dedup upsert");

    // The new action should NOT have been inserted
    let result = db.get_action_by_id("postmeet-999").expect("query");
    assert!(result.is_none(), "Title-based dedup should prevent insert");
}

#[test]
fn test_upsert_action_title_dedup_pending() {
    let db = test_db();

    // Insert a PENDING action
    let action = sample_action("inbox-001", "Review contract");
    db.upsert_action_if_not_completed(&action).expect("insert");

    // Try to insert the same title under a different ID (re-processing same file)
    let action2 = DbAction {
        id: "inbox-002".to_string(),
        title: "Review contract".to_string(),
        ..sample_action("inbox-002", "Review contract")
    };
    db.upsert_action_if_not_completed(&action2)
        .expect("dedup upsert");

    // The duplicate should NOT have been inserted
    let result = db.get_action_by_id("inbox-002").expect("query");
    assert!(
        result.is_none(),
        "Title-based dedup should prevent duplicate pending actions"
    );
}

#[test]
fn test_upsert_action_transcript_dedup_is_meeting_scoped() {
    let db = test_db();

    let mut first = sample_action("transcript-m1-0", "Send recap");
    first.source_type = Some("transcript".to_string());
    first.source_id = Some("meeting-1".to_string());
    first.status = crate::action_status::BACKLOG.to_string();
    first.account_id = None;
    db.upsert_action_if_not_completed(&first)
        .expect("insert first transcript action");

    // Same title in a different meeting should NOT be suppressed.
    let mut second = sample_action("transcript-m2-0", "Send recap");
    second.source_type = Some("transcript".to_string());
    second.source_id = Some("meeting-2".to_string());
    second.status = crate::action_status::BACKLOG.to_string();
    second.account_id = None;
    db.upsert_action_if_not_completed(&second)
        .expect("insert second transcript action");

    let second_row = db
        .get_action_by_id("transcript-m2-0")
        .expect("query second transcript action");
    assert!(
        second_row.is_some(),
        "Transcript dedup must not suppress actions from other meetings"
    );

    // Same title in the same meeting/source should be suppressed.
    let mut duplicate_same_meeting = sample_action("transcript-m1-1", "Send recap");
    duplicate_same_meeting.source_type = Some("transcript".to_string());
    duplicate_same_meeting.source_id = Some("meeting-1".to_string());
    duplicate_same_meeting.status = crate::action_status::BACKLOG.to_string();
    duplicate_same_meeting.account_id = None;
    db.upsert_action_if_not_completed(&duplicate_same_meeting)
        .expect("attempt duplicate transcript action");

    let duplicate_row = db
        .get_action_by_id("transcript-m1-1")
        .expect("query duplicate transcript action");
    assert!(
        duplicate_row.is_none(),
        "Transcript dedup should still suppress duplicates within the same meeting"
    );
}

#[test]
fn test_get_non_briefing_pending_actions() {
    let db = test_db();

    // Insert a briefing-sourced action (should NOT appear)
    let mut briefing_action = sample_action("brief-001", "Briefing task");
    briefing_action.source_type = Some("briefing".to_string());
    db.upsert_action(&briefing_action).expect("insert");

    // Insert a post-meeting action (should appear)
    let mut pm_action = sample_action("pm-001", "Post-meeting task");
    pm_action.source_type = Some("post_meeting".to_string());
    db.upsert_action(&pm_action).expect("insert");

    // Insert an inbox action (should appear)
    let mut inbox_action = sample_action("inbox-001", "Inbox task");
    inbox_action.source_type = Some("inbox".to_string());
    db.upsert_action(&inbox_action).expect("insert");

    // Insert a completed post-meeting action (should NOT appear)
    let mut completed = sample_action("pm-002", "Done task");
    completed.source_type = Some("post_meeting".to_string());
    db.upsert_action(&completed).expect("insert");
    db.complete_action("pm-002").expect("complete");

    // Insert a pending inbox action with waiting_on (SHOULD appear)
    let mut waiting_action = sample_action("inbox-wait", "Waiting on legal");
    waiting_action.source_type = Some("inbox".to_string());
    waiting_action.waiting_on = Some("true".to_string());
    db.upsert_action(&waiting_action).expect("insert");

    let results = db.get_non_briefing_pending_actions().expect("query");
    assert_eq!(results.len(), 3);
    let ids: Vec<&str> = results.iter().map(|a| a.id.as_str()).collect();
    assert!(ids.contains(&"pm-001"));
    assert!(ids.contains(&"inbox-001"));
    assert!(ids.contains(&"inbox-wait"));
}

#[test]
fn test_get_actions_for_meeting_includes_post_meeting() {
    let db = test_db();

    let mut transcript_action = sample_action("mtg-act-001", "Transcript follow-up");
    transcript_action.source_type = Some("transcript".to_string());
    transcript_action.source_id = Some("meeting-123".to_string());
    db.upsert_action(&transcript_action)
        .expect("insert transcript action");

    let mut post_meeting_action = sample_action("mtg-act-002", "Manual capture follow-up");
    post_meeting_action.source_type = Some("post_meeting".to_string());
    post_meeting_action.source_id = Some("meeting-123".to_string());
    db.upsert_action(&post_meeting_action)
        .expect("insert post-meeting action");

    let mut unrelated = sample_action("mtg-act-003", "Other meeting");
    unrelated.source_type = Some("post_meeting".to_string());
    unrelated.source_id = Some("meeting-999".to_string());
    db.upsert_action(&unrelated)
        .expect("insert unrelated action");

    let actions = db
        .get_actions_for_meeting("meeting-123")
        .expect("query actions");
    let ids: Vec<&str> = actions.iter().map(|a| a.id.as_str()).collect();

    assert_eq!(actions.len(), 2);
    assert!(ids.contains(&"mtg-act-001"));
    assert!(ids.contains(&"mtg-act-002"));
    assert!(!ids.contains(&"mtg-act-003"));
}

#[test]
fn test_get_captures_for_account() {
    let db = test_db();

    // Insert captures for two accounts
    db.insert_capture(
        "mtg-1",
        "Acme QBR",
        Some("acme"),
        "win",
        "Expanded deployment",
    )
    .expect("insert capture 1");
    db.insert_capture("mtg-1", "Acme QBR", Some("acme"), "risk", "Budget freeze")
        .expect("insert capture 2");
    db.insert_capture(
        "mtg-2",
        "Beta Sync",
        Some("beta"),
        "win",
        "New champion identified",
    )
    .expect("insert capture 3");

    // Query for acme — should get 2
    let results = db
        .get_captures_for_account("acme", 30)
        .expect("query captures");
    assert_eq!(results.len(), 2);

    // Verify capture types are correct
    let types: Vec<&str> = results.iter().map(|c| c.capture_type.as_str()).collect();
    assert!(types.contains(&"win"));
    assert!(types.contains(&"risk"));

    // Query for beta — should get 1
    let results = db
        .get_captures_for_account("beta", 30)
        .expect("query captures");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].content, "New champion identified");

    // Query for nonexistent account — should get 0
    let results = db
        .get_captures_for_account("nonexistent", 30)
        .expect("query captures");
    assert_eq!(results.len(), 0);
}

#[test]
fn test_get_captures_for_date() {
    let db = test_db();

    // Insert captures with explicit timestamps for today and yesterday
    let today = Utc::now().format("%Y-%m-%d").to_string();
    let today_ts = format!("{}T10:00:00+00:00", today);
    let yesterday = (Utc::now() - chrono::Duration::days(1))
        .format("%Y-%m-%d")
        .to_string();
    let yesterday_ts = format!("{}T10:00:00+00:00", yesterday);

    // Today's captures
    db.conn
            .execute(
                "INSERT INTO captures (id, meeting_id, meeting_title, account_id, capture_type, content, captured_at)
                 VALUES ('c1', 'mtg-1', 'Acme QBR', 'acme', 'win', 'Expanded deployment', ?1)",
                params![today_ts],
            )
            .expect("insert c1");
    db.conn
            .execute(
                "INSERT INTO captures (id, meeting_id, meeting_title, account_id, capture_type, content, captured_at)
                 VALUES ('c2', 'mtg-1', 'Acme QBR', 'acme', 'risk', 'Budget freeze', ?1)",
                params![today_ts],
            )
            .expect("insert c2");

    // Yesterday's capture (should NOT appear)
    db.conn
            .execute(
                "INSERT INTO captures (id, meeting_id, meeting_title, account_id, capture_type, content, captured_at)
                 VALUES ('c3', 'mtg-2', 'Beta Sync', 'beta', 'win', 'Old win', ?1)",
                params![yesterday_ts],
            )
            .expect("insert c3");

    let results = db.get_captures_for_date(&today).expect("query");
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].capture_type, "win");
    assert_eq!(results[1].capture_type, "risk");

    // Yesterday should have exactly 1
    let yesterday_results = db.get_captures_for_date(&yesterday).expect("query");
    assert_eq!(yesterday_results.len(), 1);

    // Nonexistent date returns empty
    let empty = db.get_captures_for_date("2020-01-01").expect("query");
    assert!(empty.is_empty());
}

#[test]
fn test_touch_account_last_contact_by_name() {
    let db = test_db();
    let account = DbAccount {
        id: "acme-corp".to_string(),
        name: "Acme Corp".to_string(),
        lifecycle: Some("steady-state".to_string()),
        arr: None,
        health: None,
        contract_start: None,
        contract_end: None,
        nps: None,
        tracker_path: None,
        parent_id: None,
        account_type: crate::db::AccountType::Customer,
        updated_at: "2020-01-01T00:00:00Z".to_string(),
        archived: false,
        keywords: None,
        keywords_extracted_at: None,
        metadata: None,
        ..Default::default()
    };
    db.upsert_account(&account).expect("upsert");

    // Touch by name (case-insensitive)
    let matched = db.touch_account_last_contact("acme corp").expect("touch");
    assert!(matched, "Should match by case-insensitive name");

    // Verify updated_at changed
    let acct = db.get_account("acme-corp").expect("get").unwrap();
    assert_ne!(acct.updated_at, "2020-01-01T00:00:00Z");
}

#[test]
fn test_touch_account_last_contact_by_id() {
    let db = test_db();
    let account = DbAccount {
        id: "acme-corp".to_string(),
        name: "Acme Corp".to_string(),
        updated_at: "2020-01-01T00:00:00Z".to_string(),
        ..Default::default()
    };
    db.upsert_account(&account).expect("upsert");

    let matched = db
        .touch_account_last_contact("acme-corp")
        .expect("touch by id");
    assert!(matched, "Should match by id");
}

#[test]
fn test_touch_account_last_contact_no_match() {
    let db = test_db();
    let matched = db.touch_account_last_contact("nonexistent").expect("touch");
    assert!(!matched, "Should return false when no account matches");
}

// =========================================================================
// Entity tests (ADR-0045)
// =========================================================================

#[test]
fn test_upsert_and_get_entity() {
    let db = test_db();

    let entity = DbEntity {
        id: "proj-alpha".to_string(),
        name: "Project Alpha".to_string(),
        entity_type: EntityType::Project,
        tracker_path: Some("Projects/alpha".to_string()),
        updated_at: "2025-01-01T00:00:00Z".to_string(),
    };
    db.upsert_entity(&entity).expect("upsert entity");

    let result = db.get_entity("proj-alpha").expect("get entity");
    assert!(result.is_some());
    let e = result.unwrap();
    assert_eq!(e.name, "Project Alpha");
    assert_eq!(e.entity_type, EntityType::Project);
    assert_eq!(e.tracker_path, Some("Projects/alpha".to_string()));

    // Not found
    let missing = db.get_entity("nonexistent").expect("get entity");
    assert!(missing.is_none());
}

#[test]
fn test_touch_entity_last_contact() {
    let db = test_db();

    let entity = DbEntity {
        id: "acme".to_string(),
        name: "Acme Corp".to_string(),
        entity_type: EntityType::Account,
        tracker_path: None,
        updated_at: "2020-01-01T00:00:00Z".to_string(),
    };
    db.upsert_entity(&entity).expect("upsert");

    // Touch by name (case-insensitive)
    let matched = db
        .touch_entity_last_contact("acme corp")
        .expect("touch by name");
    assert!(matched);

    let e = db.get_entity("acme").expect("get").unwrap();
    assert_ne!(e.updated_at, "2020-01-01T00:00:00Z");

    // Touch by ID
    let matched_id = db.touch_entity_last_contact("acme").expect("touch by id");
    assert!(matched_id);

    // No match
    let no_match = db.touch_entity_last_contact("nonexistent").expect("touch");
    assert!(!no_match);
}

#[test]
fn test_ensure_entity_for_account() {
    let db = test_db();

    let account = DbAccount {
        id: "beta-inc".to_string(),
        name: "Beta Inc".to_string(),
        lifecycle: Some("ramping".to_string()),
        arr: Some(50_000.0),
        health: Some("yellow".to_string()),
        contract_start: None,
        contract_end: None,
        nps: None,
        tracker_path: Some("Accounts/beta-inc".to_string()),
        parent_id: None,
        account_type: crate::db::AccountType::Customer,
        updated_at: "2025-06-01T00:00:00Z".to_string(),
        archived: false,
        keywords: None,
        keywords_extracted_at: None,
        metadata: None,
        ..Default::default()
    };

    // upsert_account now calls ensure_entity_for_account automatically
    db.upsert_account(&account).expect("upsert account");

    // Entity should exist with matching fields
    let entity = db.get_entity("beta-inc").expect("get entity").unwrap();
    assert_eq!(entity.name, "Beta Inc");
    assert_eq!(entity.entity_type, EntityType::Account);
    assert_eq!(entity.tracker_path, Some("Accounts/beta-inc".to_string()));
    assert_eq!(entity.updated_at, "2025-06-01T00:00:00Z");
}

#[test]
fn test_get_entities_by_type() {
    let db = test_db();

    let e1 = DbEntity {
        id: "acme".to_string(),
        name: "Acme".to_string(),
        entity_type: EntityType::Account,
        tracker_path: None,
        updated_at: Utc::now().to_rfc3339(),
    };
    let e2 = DbEntity {
        id: "beta".to_string(),
        name: "Beta".to_string(),
        entity_type: EntityType::Account,
        tracker_path: None,
        updated_at: Utc::now().to_rfc3339(),
    };
    let e3 = DbEntity {
        id: "proj-x".to_string(),
        name: "Project X".to_string(),
        entity_type: EntityType::Project,
        tracker_path: None,
        updated_at: Utc::now().to_rfc3339(),
    };

    db.upsert_entity(&e1).expect("upsert");
    db.upsert_entity(&e2).expect("upsert");
    db.upsert_entity(&e3).expect("upsert");

    let accounts = db.get_entities_by_type("account").expect("query");
    assert_eq!(accounts.len(), 2);

    let projects = db.get_entities_by_type("project").expect("query");
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].name, "Project X");
}

#[test]
fn test_idempotent_schema_application() {
    // Opening the same DB twice should not error (IF NOT EXISTS)
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("idempotent.db");

    let _db1 = ActionDb::open_at_unencrypted(path.clone()).expect("first open");
    let _db2 = ActionDb::open_at_unencrypted(path).expect("second open should not fail");
}

// =========================================================================
// Intelligence query tests (I42)
// =========================================================================

#[test]
fn test_get_stale_delegations() {
    let db = test_db();

    // Insert a pending action with waiting_on created 10 days ago (should be stale at 3-day threshold)
    let mut stale = sample_action("wait-001", "Waiting on legal review");
    stale.waiting_on = Some("Legal".to_string());
    stale.created_at = "2020-01-01T00:00:00Z".to_string(); // very old
    db.upsert_action(&stale).expect("insert stale");

    // Insert a pending action with waiting_on created now (should NOT be stale)
    let mut fresh = sample_action("wait-002", "Fresh delegation");
    fresh.waiting_on = Some("Bob".to_string());
    db.upsert_action(&fresh).expect("insert fresh");

    // Insert a pending action (not waiting — should NOT appear)
    let pending = sample_action("pend-001", "Pending task");
    db.upsert_action(&pending).expect("insert pending");

    let results = db.get_stale_delegations(3).expect("query");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, "wait-001");
    assert_eq!(results[0].waiting_on, Some("Legal".to_string()));
}

#[test]
fn test_get_stale_delegations_empty() {
    let db = test_db();
    let results = db.get_stale_delegations(3).expect("query");
    assert!(results.is_empty());
}

#[test]
fn test_flag_and_get_decisions() {
    let db = test_db();

    // Insert actions
    let mut act1 = sample_action("dec-001", "Decide on vendor");
    act1.due_date = Some("2099-12-31".to_string()); // future, within range
    db.upsert_action(&act1).expect("insert");

    let mut act2 = sample_action("dec-002", "Choose architecture");
    act2.due_date = Some("2099-12-31".to_string());
    db.upsert_action(&act2).expect("insert");

    let act3 = sample_action("dec-003", "Not flagged");
    db.upsert_action(&act3).expect("insert");

    // Flag only the first two
    assert!(db.flag_action_as_decision("dec-001").expect("flag"));
    assert!(db.flag_action_as_decision("dec-002").expect("flag"));

    // Non-existent action returns false
    assert!(!db.flag_action_as_decision("nonexistent").expect("flag"));

    // Query with large lookahead — should get both flagged actions
    let results = db.get_flagged_decisions(365_000).expect("query");
    assert_eq!(results.len(), 2);
    let ids: Vec<&str> = results.iter().map(|a| a.id.as_str()).collect();
    assert!(ids.contains(&"dec-001"));
    assert!(ids.contains(&"dec-002"));
}

#[test]
fn test_flagged_decisions_excludes_completed() {
    let db = test_db();

    let mut act = sample_action("dec-010", "Completed decision");
    act.due_date = Some("2099-12-31".to_string());
    db.upsert_action(&act).expect("insert");
    db.flag_action_as_decision("dec-010").expect("flag");
    db.complete_action("dec-010").expect("complete");

    let results = db.get_flagged_decisions(365_000).expect("query");
    assert!(results.is_empty(), "Completed actions should not appear");
}

#[test]
fn test_flagged_decisions_includes_no_due_date() {
    let db = test_db();

    // Action with no due date but flagged
    let act = sample_action("dec-020", "Open-ended decision");
    db.upsert_action(&act).expect("insert");
    db.flag_action_as_decision("dec-020").expect("flag");

    let results = db.get_flagged_decisions(3).expect("query");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, "dec-020");
}

#[test]
fn test_clear_decision_flags() {
    let db = test_db();

    let act = sample_action("dec-030", "Will be unflagged");
    db.upsert_action(&act).expect("insert");
    db.flag_action_as_decision("dec-030").expect("flag");

    // Verify flagged
    let before = db.get_flagged_decisions(365_000).expect("query");
    assert_eq!(before.len(), 1);

    // Clear
    db.clear_decision_flags().expect("clear");

    let after = db.get_flagged_decisions(365_000).expect("query");
    assert!(after.is_empty(), "All flags should be cleared");
}

#[test]
fn test_get_renewal_alerts() {
    let db = test_db();

    // Account renewing in 30 days (should appear at 60-day threshold)
    let soon = DbAccount {
        id: "renew-soon".to_string(),
        name: "Renewing Soon Corp".to_string(),
        lifecycle: Some("steady-state".to_string()),
        arr: Some(100_000.0),
        health: Some("green".to_string()),
        contract_start: Some("2025-01-01".to_string()),
        contract_end: Some(
            (Utc::now() + chrono::Duration::days(30))
                .format("%Y-%m-%d")
                .to_string(),
        ),
        nps: None,
        tracker_path: None,
        parent_id: None,
        account_type: crate::db::AccountType::Customer,
        updated_at: Utc::now().to_rfc3339(),
        archived: false,
        keywords: None,
        keywords_extracted_at: None,
        metadata: None,
        ..Default::default()
    };
    db.upsert_account(&soon).expect("insert");

    // Account with no contract_end (should NOT appear)
    let no_end = DbAccount {
        id: "no-end".to_string(),
        name: "No End Corp".to_string(),
        lifecycle: Some("ramping".to_string()),
        arr: None,
        health: None,
        contract_start: None,
        contract_end: None,
        nps: None,
        tracker_path: None,
        parent_id: None,
        account_type: crate::db::AccountType::Customer,
        updated_at: Utc::now().to_rfc3339(),
        archived: false,
        keywords: None,
        keywords_extracted_at: None,
        metadata: None,
        ..Default::default()
    };
    db.upsert_account(&no_end).expect("insert");

    // Account already expired (should NOT appear — contract_end < now)
    let expired = DbAccount {
        id: "expired".to_string(),
        name: "Expired Corp".to_string(),
        lifecycle: Some("onboarding".to_string()),
        arr: None,
        health: None,
        contract_start: None,
        contract_end: Some("2020-01-01".to_string()),
        nps: None,
        tracker_path: None,
        parent_id: None,
        account_type: crate::db::AccountType::Customer,
        updated_at: Utc::now().to_rfc3339(),
        archived: false,
        keywords: None,
        keywords_extracted_at: None,
        metadata: None,
        ..Default::default()
    };
    db.upsert_account(&expired).expect("insert");

    let results = db.get_renewal_alerts(60).expect("query");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, "renew-soon");
}

#[test]
fn test_get_stale_accounts() {
    let db = test_db();

    // Account updated 60 days ago (should be stale at 30-day threshold)
    let stale = DbAccount {
        id: "stale-acct".to_string(),
        name: "Stale Corp".to_string(),
        lifecycle: Some("ramping".to_string()),
        arr: None,
        health: None,
        contract_start: None,
        contract_end: None,
        nps: None,
        tracker_path: None,
        parent_id: None,
        account_type: crate::db::AccountType::Customer,
        updated_at: "2020-01-01T00:00:00Z".to_string(),
        archived: false,
        keywords: None,
        keywords_extracted_at: None,
        metadata: None,
        ..Default::default()
    };
    db.upsert_account(&stale).expect("insert");

    // Account updated just now (should NOT be stale)
    let fresh = DbAccount {
        id: "fresh-acct".to_string(),
        name: "Fresh Corp".to_string(),
        lifecycle: Some("steady-state".to_string()),
        arr: None,
        health: None,
        contract_start: None,
        contract_end: None,
        nps: None,
        tracker_path: None,
        parent_id: None,
        account_type: crate::db::AccountType::Customer,
        updated_at: Utc::now().to_rfc3339(),
        archived: false,
        keywords: None,
        keywords_extracted_at: None,
        metadata: None,
        ..Default::default()
    };
    db.upsert_account(&fresh).expect("insert");

    let results = db.get_stale_accounts(30).expect("query");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, "stale-acct");
}

#[test]
fn test_needs_decision_migration() {
    // Verify the needs_decision column exists after opening a fresh DB
    let db = test_db();
    let act = sample_action("mig-001", "Test migration");
    db.upsert_action(&act).expect("insert");

    // Should be able to flag it without error
    db.flag_action_as_decision("mig-001").expect("flag");

    // Verify directly
    let flagged: i32 = db
        .conn
        .query_row(
            "SELECT needs_decision FROM actions WHERE id = 'mig-001'",
            [],
            |row| row.get(0),
        )
        .expect("direct query");
    assert_eq!(flagged, 1);
}

// =========================================================================
// Stakeholder Signals tests (I43)
// =========================================================================

#[test]
fn test_stakeholder_signals_empty() {
    let db = test_db();
    let signals = db
        .get_stakeholder_signals("nonexistent-corp")
        .expect("should not error for missing account");
    assert_eq!(signals.meeting_frequency_30d, 0);
    assert_eq!(signals.meeting_frequency_90d, 0);
    assert!(signals.last_meeting.is_none());
    assert!(signals.last_contact.is_none());
    assert_eq!(signals.temperature, "cold");
    assert_eq!(signals.trend, "stable");
}

#[test]
fn test_stakeholder_signals_with_meetings() {
    let db = test_db();
    let now = Utc::now();

    // Insert recent meetings
    for i in 0..5 {
        let mid = format!("mtg-{}", i);
        let meeting = DbMeeting {
            id: mid.clone(),
            title: format!("Sync #{}", i),
            meeting_type: "customer".to_string(),
            start_time: (now - chrono::Duration::days(i * 5)).to_rfc3339(),
            end_time: None,
            attendees: None,
            notes_path: None,
            summary: None,
            created_at: now.to_rfc3339(),
            calendar_event_id: None,
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
        };
        db.upsert_meeting(&meeting).expect("insert meeting");
        db.link_meeting_entity(&mid, "acme-corp", "account")
            .expect("link");
    }

    let signals = db.get_stakeholder_signals("acme-corp").expect("signals");
    assert_eq!(signals.meeting_frequency_30d, 5);
    assert_eq!(signals.meeting_frequency_90d, 5);
    assert!(signals.last_meeting.is_some());
    assert_eq!(signals.temperature, "hot"); // most recent < 7 days ago
}

#[test]
fn test_stakeholder_signals_with_account_contact() {
    let db = test_db();

    let account = DbAccount {
        id: "acme-corp".to_string(),
        name: "Acme Corp".to_string(),
        lifecycle: None,
        arr: None,
        health: None,
        contract_start: None,
        contract_end: None,
        nps: None,
        tracker_path: None,
        parent_id: None,
        account_type: crate::db::AccountType::Customer,
        updated_at: Utc::now().to_rfc3339(),
        archived: false,
        keywords: None,
        keywords_extracted_at: None,
        metadata: None,
        ..Default::default()
    };
    db.upsert_account(&account).expect("insert account");

    let signals = db.get_stakeholder_signals("acme-corp").expect("signals");
    assert!(signals.last_contact.is_some());
}

#[test]
fn test_compute_temperature() {
    assert_eq!(super::compute_temperature(&Utc::now().to_rfc3339()), "hot");

    let days_ago_10 = (Utc::now() - chrono::Duration::days(10)).to_rfc3339();
    assert_eq!(super::compute_temperature(&days_ago_10), "warm");

    let days_ago_45 = (Utc::now() - chrono::Duration::days(45)).to_rfc3339();
    assert_eq!(super::compute_temperature(&days_ago_45), "cool");

    let days_ago_90 = (Utc::now() - chrono::Duration::days(90)).to_rfc3339();
    assert_eq!(super::compute_temperature(&days_ago_90), "cold");
}

#[test]
fn test_compute_trend() {
    // Even distribution: 3 in 30d out of 9 in 90d → stable
    assert_eq!(super::compute_trend(3, 9), "stable");

    // Increasing: 5 in 30d out of 6 in 90d (way above 1/3)
    assert_eq!(super::compute_trend(5, 6), "increasing");

    // Decreasing: 0 in 30d out of 9 in 90d (way below 1/3)
    assert_eq!(super::compute_trend(0, 9), "decreasing");

    // No data: should be stable
    assert_eq!(super::compute_trend(0, 0), "stable");
}

// =========================================================================
// People Tests (I51)
// =========================================================================

fn sample_person(email: &str) -> DbPerson {
    let now = Utc::now().to_rfc3339();
    DbPerson {
        id: crate::util::person_id_from_email(email),
        email: email.to_lowercase(),
        name: crate::util::name_from_email(email),
        organization: Some(crate::util::org_from_email(email)),
        role: None,
        relationship: "unknown".to_string(),
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
    }
}

#[test]
fn test_upsert_and_get_person() {
    let db = test_db();
    let person = sample_person("sarah.chen@acme.com");
    db.upsert_person(&person).expect("upsert person");

    let result = db.get_person(&person.id).expect("get person");
    assert!(result.is_some());
    let p = result.unwrap();
    assert_eq!(p.name, "Sarah Chen");
    assert_eq!(p.email, "sarah.chen@acme.com");
    assert_eq!(p.organization, Some("Acme".to_string()));
}

#[test]
fn test_get_person_by_email() {
    let db = test_db();
    let person = sample_person("bob@example.com");
    db.upsert_person(&person).expect("upsert");

    let result = db
        .get_person_by_email("BOB@EXAMPLE.COM")
        .expect("get by email");
    assert!(result.is_some());
    assert_eq!(result.unwrap().id, person.id);
}

#[test]
fn test_get_person_by_email_or_alias() {
    let db = test_db();
    let person = sample_person("alice@acme.com");
    db.upsert_person(&person).expect("upsert");

    // Exact match still works
    let result = db
        .get_person_by_email_or_alias("alice@acme.com")
        .expect("exact match");
    assert!(result.is_some());
    assert_eq!(result.as_ref().unwrap().id, person.id);

    // Add an alias
    db.add_person_email(&person.id, "alice@acmecorp.com", false)
        .expect("add alias");

    // Alias lookup works
    let result = db
        .get_person_by_email_or_alias("alice@acmecorp.com")
        .expect("alias match");
    assert!(result.is_some());
    assert_eq!(result.unwrap().id, person.id);

    // Unknown email returns None
    let result = db
        .get_person_by_email_or_alias("unknown@nowhere.com")
        .expect("no match");
    assert!(result.is_none());
}

#[test]
fn test_find_person_by_domain_alias() {
    let db = test_db();
    let person = sample_person("renan@globex.com");
    db.upsert_person(&person).expect("upsert");

    // Search for the same local part at a sibling domain
    let result = db
        .find_person_by_domain_alias("renan@globex-labs.com", &["globex.com".to_string()])
        .expect("domain alias search");
    assert!(result.is_some());
    assert_eq!(result.unwrap().id, person.id);

    // No match when sibling domains don't contain the person
    let result = db
        .find_person_by_domain_alias("renan@globex-labs.com", &["unknown.com".to_string()])
        .expect("no match");
    assert!(result.is_none());
}

#[test]
fn test_get_sibling_domains_for_email() {
    let db = test_db();

    // Set up an account with multiple domains
    setup_account(&db, "acc1", "Globex");
    db.set_account_domains(
        "acc1",
        &["globex.com".to_string(), "globex-labs.com".to_string()],
    )
    .expect("set domains");

    // Email at globex.com should return globex-labs.com as sibling
    let siblings = db
        .get_sibling_domains_for_email("renan@globex.com", &[])
        .expect("siblings");
    assert!(siblings.contains(&"globex-labs.com".to_string()));
    assert!(!siblings.contains(&"globex.com".to_string())); // self excluded

    // Personal email domains should return no siblings
    let siblings = db
        .get_sibling_domains_for_email("alice@gmail.com", &[])
        .expect("personal");
    assert!(siblings.is_empty());

    // user_domains path
    let user_domains = vec!["myco.com".to_string(), "myco.io".to_string()];
    let siblings = db
        .get_sibling_domains_for_email("alice@myco.com", &user_domains)
        .expect("user domains");
    assert!(siblings.contains(&"myco.io".to_string()));
}

#[test]
fn test_person_emails_crud() {
    let db = test_db();
    let person = sample_person("alice@acme.com");
    db.upsert_person(&person).expect("upsert");

    // upsert_person should auto-seed person_emails
    let emails = db.get_person_emails(&person.id).expect("list emails");
    assert_eq!(emails.len(), 1);
    assert_eq!(emails[0], "alice@acme.com");

    // Add an alias
    db.add_person_email(&person.id, "alice@acmecorp.com", false)
        .expect("add alias");
    let emails = db.get_person_emails(&person.id).expect("list emails");
    assert_eq!(emails.len(), 2);

    // Duplicate insert is idempotent
    db.add_person_email(&person.id, "alice@acmecorp.com", false)
        .expect("duplicate add");
    let emails = db.get_person_emails(&person.id).expect("list emails");
    assert_eq!(emails.len(), 2);
}

#[test]
fn test_merge_people_transfers_aliases() {
    let db = test_db();
    let p1 = sample_person("alice@acme.com");
    let p2 = sample_person("alice@acmecorp.com");
    db.upsert_person(&p1).expect("upsert p1");
    db.upsert_person(&p2).expect("upsert p2");

    // Both have their primary emails
    assert_eq!(db.get_person_emails(&p1.id).unwrap().len(), 1);
    assert_eq!(db.get_person_emails(&p2.id).unwrap().len(), 1);

    // Merge p2 into p1
    db.merge_people(&p1.id, &p2.id).expect("merge");

    // p1 should now have both emails
    let emails = db.get_person_emails(&p1.id).unwrap();
    assert!(emails.contains(&"alice@acme.com".to_string()));
    assert!(emails.contains(&"alice@acmecorp.com".to_string()));

    // p2 should be gone
    assert!(db.get_person(&p2.id).unwrap().is_none());
    assert!(db.get_person_emails(&p2.id).unwrap().is_empty());
}

#[test]
fn test_alias_aware_person_resolution_integration() {
    let db = test_db();

    // Set up account with two domains
    setup_account(&db, "acc1", "Globex");
    db.set_account_domains(
        "acc1",
        &["globex.com".to_string(), "globex-labs.com".to_string()],
    )
    .expect("set domains");

    // Create person from domain A
    let person = sample_person("renan@globex.com");
    db.upsert_person(&person).expect("upsert");

    // Simulate: calendar event arrives with renan@globex-labs.com
    let email = "renan@globex-labs.com";
    let found = db.get_person_by_email_or_alias(email).ok().flatten();
    assert!(found.is_none(), "no direct match yet");

    // Get siblings and try domain alias
    let siblings = db
        .get_sibling_domains_for_email(email, &[])
        .expect("siblings");
    assert!(!siblings.is_empty());

    let found = db
        .find_person_by_domain_alias(email, &siblings)
        .expect("domain alias");
    assert!(found.is_some());
    assert_eq!(found.as_ref().unwrap().id, person.id);

    // Record the alias
    db.add_person_email(&person.id, email, false)
        .expect("record alias");

    // Now direct alias lookup should work
    let found = db
        .get_person_by_email_or_alias(email)
        .expect("alias lookup");
    assert!(found.is_some());
    assert_eq!(found.unwrap().id, person.id);

    // person_emails should have both
    let emails = db.get_person_emails(&person.id).unwrap();
    assert_eq!(emails.len(), 2);
}

#[test]
fn test_get_people_with_filter() {
    let db = test_db();
    let mut p1 = sample_person("alice@myco.com");
    p1.relationship = "internal".to_string();
    let mut p2 = sample_person("bob@other.com");
    p2.relationship = "external".to_string();

    db.upsert_person(&p1).expect("upsert p1");
    db.upsert_person(&p2).expect("upsert p2");

    let all = db.get_people(None).expect("get all");
    assert_eq!(all.len(), 2);

    let internal = db.get_people(Some("internal")).expect("get internal");
    assert_eq!(internal.len(), 1);
    assert_eq!(internal[0].name, "Alice");

    let external = db.get_people(Some("external")).expect("get external");
    assert_eq!(external.len(), 1);
    assert_eq!(external[0].name, "Bob");
}

#[test]
fn test_get_people_excludes_archived() {
    let db = test_db();

    let mut active = sample_person("active@test.com");
    active.relationship = "external".to_string();

    let mut archived = sample_person("archived@test.com");
    archived.relationship = "external".to_string();
    archived.archived = true;

    db.upsert_person(&active).expect("upsert active");
    db.upsert_person(&archived).expect("upsert archived");

    // No filter — should exclude archived
    let all = db.get_people(None).expect("get all");
    assert_eq!(all.len(), 1, "should only return active person");
    assert_eq!(all[0].email, "active@test.com");

    // With relationship filter — should also exclude archived
    let filtered = db.get_people(Some("external")).expect("get external");
    assert_eq!(
        filtered.len(),
        1,
        "should only return active external person"
    );
    assert_eq!(filtered[0].email, "active@test.com");
}

#[test]
fn test_person_entity_linking() {
    let db = test_db();
    let person = sample_person("jane@acme.com");
    db.upsert_person(&person).expect("upsert person");

    let account = DbAccount {
        id: "acme-corp".to_string(),
        name: "Acme Corp".to_string(),
        lifecycle: None,
        arr: None,
        health: None,
        contract_start: None,
        contract_end: None,
        nps: None,
        tracker_path: None,
        parent_id: None,
        account_type: crate::db::AccountType::Customer,
        updated_at: Utc::now().to_rfc3339(),
        archived: false,
        keywords: None,
        keywords_extracted_at: None,
        metadata: None,
        ..Default::default()
    };
    db.upsert_account(&account).expect("upsert account");

    db.link_person_to_entity(&person.id, "acme-corp", "associated")
        .expect("link");

    let people = db
        .get_people_for_entity("acme-corp")
        .expect("people for entity");
    assert_eq!(people.len(), 1);
    assert_eq!(people[0].id, person.id);

    let entities = db
        .get_entities_for_person(&person.id)
        .expect("entities for person");
    assert_eq!(entities.len(), 1);
    assert_eq!(entities[0].id, "acme-corp");

    // Unlink
    db.unlink_person_from_entity(&person.id, "acme-corp")
        .expect("unlink");
    let people_after = db.get_people_for_entity("acme-corp").expect("after unlink");
    assert_eq!(people_after.len(), 0);
}

#[test]
fn test_meeting_attendance() {
    let db = test_db();
    let person = sample_person("attendee@test.com");
    db.upsert_person(&person).expect("upsert person");

    let now = Utc::now().to_rfc3339();
    let meeting = DbMeeting {
        id: "mtg-attend-001".to_string(),
        title: "Test Meeting".to_string(),
        meeting_type: "internal".to_string(),
        start_time: now.clone(),
        end_time: None,
        attendees: None,
        notes_path: None,
        summary: None,
        created_at: now,
        calendar_event_id: None,
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
    };
    db.upsert_meeting(&meeting).expect("upsert meeting");
    db.record_meeting_attendance("mtg-attend-001", &person.id)
        .expect("record attendance");

    // Check attendees for meeting
    let attendees = db
        .get_meeting_attendees("mtg-attend-001")
        .expect("get attendees");
    assert_eq!(attendees.len(), 1);
    assert_eq!(attendees[0].id, person.id);

    // Check meetings for person
    let meetings = db
        .get_person_meetings(&person.id, 10)
        .expect("person meetings");
    assert_eq!(meetings.len(), 1);
    assert_eq!(meetings[0].id, "mtg-attend-001");

    // Check meeting_count was incremented
    let updated = db.get_person(&person.id).expect("get updated").unwrap();
    assert_eq!(updated.meeting_count, 1);

    // Idempotent: recording again should not increment
    db.record_meeting_attendance("mtg-attend-001", &person.id)
        .expect("re-record");
    let same = db.get_person(&person.id).expect("get same").unwrap();
    assert_eq!(same.meeting_count, 1);
}

#[test]
fn test_get_person_meetings_includes_person_entity_link() {
    let db = test_db();
    let person = sample_person("linked-person@test.com");
    db.upsert_person(&person).expect("upsert person");

    let now = Utc::now().to_rfc3339();
    let meeting = DbMeeting {
        id: "mtg-person-link-001".to_string(),
        title: "1:1 Linked by Entity".to_string(),
        meeting_type: "one_on_one".to_string(),
        start_time: now.clone(),
        end_time: None,
        attendees: None,
        notes_path: None,
        summary: Some("Followed up on staffing plan".to_string()),
        created_at: now,
        calendar_event_id: None,
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
    };
    db.upsert_meeting(&meeting).expect("upsert meeting");
    db.link_meeting_entity(&meeting.id, &person.id, "person")
        .expect("link person entity");

    let meetings = db
        .get_person_meetings(&person.id, 10)
        .expect("person meetings");
    assert_eq!(meetings.len(), 1);
    assert_eq!(meetings[0].id, "mtg-person-link-001");
    assert_eq!(
        meetings[0].summary.as_deref(),
        Some("Followed up on staffing plan")
    );
}

#[test]
fn test_get_upcoming_meetings_for_person_includes_person_entity_link() {
    let db = test_db();
    let person = sample_person("linked-upcoming@test.com");
    db.upsert_person(&person).expect("upsert person");

    let start_time = (Utc::now() + chrono::Duration::days(1)).to_rfc3339();
    let meeting = DbMeeting {
        id: "mtg-person-link-002".to_string(),
        title: "Future 1:1 Linked by Entity".to_string(),
        meeting_type: "one_on_one".to_string(),
        start_time,
        end_time: None,
        attendees: None,
        notes_path: None,
        summary: None,
        created_at: Utc::now().to_rfc3339(),
        calendar_event_id: None,
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
    };
    db.upsert_meeting(&meeting).expect("upsert meeting");
    db.link_meeting_entity(&meeting.id, &person.id, "person")
        .expect("link person entity");

    let meetings = db
        .get_upcoming_meetings_for_person(&person.id, 10)
        .expect("upcoming meetings");
    assert_eq!(meetings.len(), 1);
    assert_eq!(meetings[0].id, "mtg-person-link-002");
}

#[test]
fn test_get_person_actions_includes_person_entity_linked_meeting() {
    let db = test_db();
    let person = sample_person("linked-actions@test.com");
    db.upsert_person(&person).expect("upsert person");

    let meeting = DbMeeting {
        id: "mtg-person-link-003".to_string(),
        title: "1:1 Action Source".to_string(),
        meeting_type: "one_on_one".to_string(),
        start_time: Utc::now().to_rfc3339(),
        end_time: None,
        attendees: None,
        notes_path: None,
        summary: None,
        created_at: Utc::now().to_rfc3339(),
        calendar_event_id: None,
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
    };
    db.upsert_meeting(&meeting).expect("upsert meeting");
    db.link_meeting_entity(&meeting.id, &person.id, "person")
        .expect("link person entity");

    let mut action = sample_action("act-person-link-001", "Follow up with colleague");
    action.source_type = Some("post_meeting".to_string());
    action.source_id = Some(meeting.id.clone());
    db.upsert_action(&action).expect("upsert action");

    let actions = db.get_person_actions(&person.id).expect("person actions");
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].id, "act-person-link-001");
}

#[test]
fn test_search_people() {
    let db = test_db();
    db.upsert_person(&sample_person("alice@acme.com"))
        .expect("upsert");
    db.upsert_person(&sample_person("bob@bigcorp.io"))
        .expect("upsert");

    let results = db.search_people("acme", 10).expect("search");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "Alice");

    let results = db.search_people("bob", 10).expect("search");
    assert_eq!(results.len(), 1);
}

#[test]
fn test_update_person_field() {
    let db = test_db();
    let person = sample_person("field@test.com");
    db.upsert_person(&person).expect("upsert");

    db.update_person_field(&person.id, "role", "VP Engineering")
        .expect("update role");
    db.set_person_field_source(&person.id, "role", "user")
        .expect("set field source");
    let updated = db.get_person(&person.id).expect("get").unwrap();
    assert_eq!(updated.role, Some("VP Engineering".to_string()));
    let sources_json = updated
        .enrichment_sources
        .expect("enrichment_sources should exist");
    let sources: serde_json::Value =
        serde_json::from_str(&sources_json).expect("parse enrichment_sources");
    assert_eq!(sources["role"]["source"].as_str(), Some("user"));

    // Invalid field should error
    let err = db.update_person_field(&person.id, "invalid_field", "val");
    assert!(err.is_err());
}

#[test]
fn test_person_signals_empty() {
    let db = test_db();
    let person = sample_person("nobody@test.com");
    db.upsert_person(&person).expect("upsert");

    let signals = db.get_person_signals(&person.id).expect("signals");
    assert_eq!(signals.meeting_frequency_30d, 0);
    assert_eq!(signals.temperature, "cold");
    assert_eq!(signals.trend, "stable");
}

#[test]
fn test_people_table_created() {
    let db = test_db();
    let count: i32 = db
        .conn
        .query_row("SELECT COUNT(*) FROM people", [], |row| row.get(0))
        .expect("people table should exist");
    assert_eq!(count, 0);

    let count: i32 = db
        .conn
        .query_row("SELECT COUNT(*) FROM meeting_attendees", [], |row| {
            row.get(0)
        })
        .expect("meeting_attendees table should exist");
    assert_eq!(count, 0);

    let count: i32 = db
        .conn
        .query_row("SELECT COUNT(*) FROM account_stakeholders", [], |row| {
            row.get(0)
        })
        .expect("account_stakeholders table should exist");
    assert_eq!(count, 0);

    let count: i32 = db
        .conn
        .query_row("SELECT COUNT(*) FROM entity_members", [], |row| row.get(0))
        .expect("entity_members table should exist");
    assert_eq!(count, 0);

    let count: i32 = db
        .conn
        .query_row("SELECT COUNT(*) FROM meeting_entities", [], |row| {
            row.get(0)
        })
        .expect("meeting_entities table should exist");
    assert_eq!(count, 0);
}

// =========================================================================
// Merge + Delete People
// =========================================================================

fn make_meeting(db: &ActionDb, id: &str) {
    let now = Utc::now().to_rfc3339();
    let meeting = DbMeeting {
        id: id.to_string(),
        title: format!("Meeting {}", id),
        meeting_type: "internal".to_string(),
        start_time: now.clone(),
        end_time: None,
        attendees: None,
        notes_path: None,
        summary: None,
        created_at: now,
        calendar_event_id: None,
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
    };
    db.upsert_meeting(&meeting).expect("upsert meeting");
}

#[test]
fn test_merge_transfers_attendees() {
    let db = test_db();
    let keep = sample_person("keep@acme.com");
    let remove = sample_person("remove@other.com");
    db.upsert_person(&keep).expect("upsert keep");
    db.upsert_person(&remove).expect("upsert remove");

    make_meeting(&db, "mtg-a");
    make_meeting(&db, "mtg-b");
    db.record_meeting_attendance("mtg-a", &keep.id)
        .expect("attend");
    db.record_meeting_attendance("mtg-b", &remove.id)
        .expect("attend");

    db.merge_people(&keep.id, &remove.id).expect("merge");

    let meetings = db.get_person_meetings(&keep.id, 50).expect("meetings");
    assert_eq!(meetings.len(), 2, "kept person should attend both meetings");
}

#[test]
fn test_merge_transfers_entity_links() {
    let db = test_db();
    let keep = sample_person("keep@acme.com");
    let remove = sample_person("remove@other.com");
    db.upsert_person(&keep).expect("upsert");
    db.upsert_person(&remove).expect("upsert");

    let account = DbAccount {
        id: "acme".to_string(),
        name: "Acme".to_string(),
        lifecycle: None,
        arr: None,
        health: None,
        contract_start: None,
        contract_end: None,
        nps: None,
        tracker_path: None,
        parent_id: None,
        account_type: crate::db::AccountType::Customer,
        updated_at: Utc::now().to_rfc3339(),
        archived: false,
        keywords: None,
        keywords_extracted_at: None,
        metadata: None,
        ..Default::default()
    };
    db.upsert_account(&account).expect("upsert account");
    db.link_person_to_entity(&remove.id, "acme", "associated")
        .expect("link");

    db.merge_people(&keep.id, &remove.id).expect("merge");

    let entities = db.get_entities_for_person(&keep.id).expect("entities");
    assert_eq!(entities.len(), 1);
    assert_eq!(entities[0].id, "acme");
}

#[test]
fn test_merge_transfers_actions() {
    let db = test_db();
    let keep = sample_person("keep@acme.com");
    let remove = sample_person("remove@other.com");
    db.upsert_person(&keep).expect("upsert");
    db.upsert_person(&remove).expect("upsert");

    let mut action = sample_action("act-1", "Follow up");
    action.person_id = Some(remove.id.clone());
    db.upsert_action(&action).expect("upsert action");

    db.merge_people(&keep.id, &remove.id).expect("merge");

    let fetched = db.get_action_by_id("act-1").expect("get action").unwrap();
    assert_eq!(fetched.person_id, Some(keep.id));
}

#[test]
fn test_merge_handles_shared_meetings() {
    let db = test_db();
    let keep = sample_person("keep@acme.com");
    let remove = sample_person("remove@other.com");
    db.upsert_person(&keep).expect("upsert");
    db.upsert_person(&remove).expect("upsert");

    make_meeting(&db, "mtg-shared");
    db.record_meeting_attendance("mtg-shared", &keep.id)
        .expect("attend");
    db.record_meeting_attendance("mtg-shared", &remove.id)
        .expect("attend");

    // Should not fail despite both attending the same meeting
    db.merge_people(&keep.id, &remove.id)
        .expect("merge should succeed with shared meetings");

    let attendees = db.get_meeting_attendees("mtg-shared").expect("attendees");
    assert_eq!(attendees.len(), 1, "only kept person remains");
    assert_eq!(attendees[0].id, keep.id);
}

#[test]
fn test_merge_deletes_removed() {
    let db = test_db();
    let keep = sample_person("keep@acme.com");
    let remove = sample_person("remove@other.com");
    db.upsert_person(&keep).expect("upsert");
    db.upsert_person(&remove).expect("upsert");

    db.merge_people(&keep.id, &remove.id).expect("merge");

    assert!(
        db.get_person(&remove.id).expect("get").is_none(),
        "removed person should be gone"
    );
    assert!(
        db.get_person(&keep.id).expect("get").is_some(),
        "kept person should still exist"
    );
}

#[test]
fn test_merge_recomputes_count() {
    let db = test_db();
    let keep = sample_person("keep@acme.com");
    let remove = sample_person("remove@other.com");
    db.upsert_person(&keep).expect("upsert");
    db.upsert_person(&remove).expect("upsert");

    make_meeting(&db, "mtg-1");
    make_meeting(&db, "mtg-2");
    make_meeting(&db, "mtg-3");
    db.record_meeting_attendance("mtg-1", &keep.id)
        .expect("attend");
    db.record_meeting_attendance("mtg-2", &remove.id)
        .expect("attend");
    db.record_meeting_attendance("mtg-3", &remove.id)
        .expect("attend");

    db.merge_people(&keep.id, &remove.id).expect("merge");

    let person = db.get_person(&keep.id).expect("get").unwrap();
    assert_eq!(person.meeting_count, 3, "should have all 3 meetings");
}

#[test]
fn test_merge_nonexistent_fails() {
    let db = test_db();
    let keep = sample_person("keep@acme.com");
    db.upsert_person(&keep).expect("upsert");

    let err = db.merge_people(&keep.id, "nonexistent-id");
    assert!(
        err.is_err(),
        "merge should fail when remove_id doesn't exist"
    );

    let err = db.merge_people("nonexistent-id", &keep.id);
    assert!(err.is_err(), "merge should fail when keep_id doesn't exist");
}

#[test]
fn test_delete_person_cascades() {
    let db = test_db();
    let person = sample_person("doomed@test.com");
    db.upsert_person(&person).expect("upsert");

    make_meeting(&db, "mtg-doom");
    db.record_meeting_attendance("mtg-doom", &person.id)
        .expect("attend");

    let account = DbAccount {
        id: "doom-corp".to_string(),
        name: "Doom Corp".to_string(),
        lifecycle: None,
        arr: None,
        health: None,
        contract_start: None,
        contract_end: None,
        nps: None,
        tracker_path: None,
        parent_id: None,
        account_type: crate::db::AccountType::Customer,
        updated_at: Utc::now().to_rfc3339(),
        archived: false,
        keywords: None,
        keywords_extracted_at: None,
        metadata: None,
        ..Default::default()
    };
    db.upsert_account(&account).expect("upsert account");
    db.link_person_to_entity(&person.id, "doom-corp", "associated")
        .expect("link");

    let mut action = sample_action("act-doom", "Doomed action");
    action.person_id = Some(person.id.clone());
    db.upsert_action(&action).expect("upsert action");

    db.delete_person(&person.id).expect("delete");

    // Person gone
    assert!(db.get_person(&person.id).expect("get").is_none());

    // Attendance gone
    let attendees = db.get_meeting_attendees("mtg-doom").expect("attendees");
    assert_eq!(attendees.len(), 0);

    // Entity link gone
    let people = db.get_people_for_entity("doom-corp").expect("people");
    assert_eq!(people.len(), 0);

    // Action person_id nulled
    let action = db
        .get_action_by_id("act-doom")
        .expect("get action")
        .unwrap();
    assert!(
        action.person_id.is_none(),
        "person_id should be nulled, not left dangling"
    );
}

// =========================================================================
// Projects (I50)
// =========================================================================

#[test]
fn test_upsert_and_get_project() {
    let db = test_db();
    let now = Utc::now().to_rfc3339();

    let project = DbProject {
        id: "widget-v2".to_string(),
        name: "Widget v2".to_string(),
        status: "active".to_string(),
        milestone: Some("Beta Launch".to_string()),
        owner: Some("Alice".to_string()),
        target_date: Some("2026-06-01".to_string()),
        tracker_path: Some("Projects/Widget v2".to_string()),
        parent_id: None,
        updated_at: now,
        archived: false,
        keywords: None,
        keywords_extracted_at: None,
        ..Default::default()
    };

    db.upsert_project(&project).expect("upsert");

    let fetched = db.get_project("widget-v2").expect("get").unwrap();
    assert_eq!(fetched.name, "Widget v2");
    assert_eq!(fetched.status, "active");
    assert_eq!(fetched.milestone, Some("Beta Launch".to_string()));
}

#[test]
fn test_get_project_by_name() {
    let db = test_db();
    let now = Utc::now().to_rfc3339();

    let project = DbProject {
        id: "gadget".to_string(),
        name: "Gadget".to_string(),
        status: "active".to_string(),
        milestone: None,
        owner: None,
        target_date: None,
        tracker_path: None,
        parent_id: None,
        updated_at: now,
        archived: false,
        keywords: None,
        keywords_extracted_at: None,
        ..Default::default()
    };

    db.upsert_project(&project).expect("upsert");

    let fetched = db.get_project_by_name("gadget").expect("get").unwrap();
    assert_eq!(fetched.id, "gadget");

    // Case-insensitive
    let fetched = db.get_project_by_name("GADGET").expect("get").unwrap();
    assert_eq!(fetched.id, "gadget");
}

#[test]
fn test_get_all_projects() {
    let db = test_db();
    let now = Utc::now().to_rfc3339();

    for name in &["Alpha", "Beta", "Gamma"] {
        let project = DbProject {
            id: name.to_lowercase(),
            name: name.to_string(),
            status: "active".to_string(),
            updated_at: now.clone(),
            ..Default::default()
        };
        db.upsert_project(&project).expect("upsert");
    }

    let all = db.get_all_projects().expect("get all");
    assert_eq!(all.len(), 3);
    assert_eq!(all[0].name, "Alpha"); // Sorted by name
}

#[test]
fn test_update_project_field() {
    let db = test_db();
    let now = Utc::now().to_rfc3339();

    let project = DbProject {
        id: "proj-1".to_string(),
        name: "Proj 1".to_string(),
        status: "active".to_string(),
        milestone: None,
        owner: None,
        target_date: None,
        tracker_path: None,
        parent_id: None,
        updated_at: now,
        archived: false,
        keywords: None,
        keywords_extracted_at: None,
        ..Default::default()
    };
    db.upsert_project(&project).expect("upsert");

    db.update_project_field("proj-1", "status", "on_hold")
        .expect("update");

    let fetched = db.get_project("proj-1").expect("get").unwrap();
    assert_eq!(fetched.status, "on_hold");
}

#[test]
fn test_update_project_field_rejects_invalid() {
    let db = test_db();
    let now = Utc::now().to_rfc3339();

    let project = DbProject {
        id: "proj-1".to_string(),
        name: "Proj 1".to_string(),
        status: "active".to_string(),
        milestone: None,
        owner: None,
        target_date: None,
        tracker_path: None,
        parent_id: None,
        updated_at: now,
        archived: false,
        keywords: None,
        keywords_extracted_at: None,
        ..Default::default()
    };
    db.upsert_project(&project).expect("upsert");

    let result = db.update_project_field("proj-1", "id", "Hacked");
    assert!(result.is_err());
}

#[test]
fn test_ensure_entity_for_project() {
    let db = test_db();
    let now = Utc::now().to_rfc3339();

    let project = DbProject {
        id: "widget-v2".to_string(),
        name: "Widget v2".to_string(),
        status: "active".to_string(),
        milestone: None,
        owner: None,
        target_date: None,
        tracker_path: Some("Projects/Widget v2".to_string()),
        parent_id: None,
        updated_at: now,
        archived: false,
        keywords: None,
        keywords_extracted_at: None,
        ..Default::default()
    };

    db.upsert_project(&project).expect("upsert");

    let entity = db.get_entity("widget-v2").expect("get").unwrap();
    assert_eq!(entity.name, "Widget v2");
    assert_eq!(entity.entity_type, EntityType::Project);
}

#[test]
fn test_get_project_actions() {
    let db = test_db();
    let now = Utc::now().to_rfc3339();

    let project = DbProject {
        id: "proj-actions".to_string(),
        name: "Action Test".to_string(),
        status: "active".to_string(),
        milestone: None,
        owner: None,
        target_date: None,
        tracker_path: None,
        parent_id: None,
        updated_at: now.clone(),
        archived: false,
        keywords: None,
        keywords_extracted_at: None,
        ..Default::default()
    };
    db.upsert_project(&project).expect("upsert");

    // Insert pending action linked to project
    let pending_action = DbAction {
        id: "act-proj-1".to_string(),
        title: "Fix the widget".to_string(),
        priority: crate::action_status::PRIORITY_URGENT,
        status: crate::action_status::UNSTARTED.to_string(),
        created_at: now.clone(),
        due_date: None,
        completed_at: None,
        account_id: None,
        project_id: Some("proj-actions".to_string()),
        source_type: None,
        source_id: None,
        source_label: None,
        context: None,
        waiting_on: None,
        updated_at: now,
        person_id: None,
        account_name: None,
        next_meeting_title: None,
        next_meeting_start: None,
        needs_decision: false,
        decision_owner: None,
        decision_stakes: None,
        linear_identifier: None,
        linear_url: None,
    };
    db.upsert_action(&pending_action)
        .expect("upsert pending action");

    // Insert suggested action linked to project
    let suggested_action = DbAction {
        id: "act-proj-2".to_string(),
        title: "Draft rollout plan".to_string(),
        status: crate::action_status::BACKLOG.to_string(),
        ..pending_action.clone()
    };
    db.upsert_action(&suggested_action)
        .expect("upsert suggested action");

    let actions = db.get_project_actions("proj-actions").expect("get");
    assert_eq!(actions.len(), 2);
    let ids: Vec<&str> = actions.iter().map(|a| a.id.as_str()).collect();
    assert!(ids.contains(&"act-proj-1"));
    assert!(ids.contains(&"act-proj-2"));
}

#[test]
fn test_project_signals_empty() {
    let db = test_db();
    let signals = db.get_project_signals("nonexistent").expect("signals");
    assert_eq!(signals.meeting_frequency_30d, 0);
    assert_eq!(signals.meeting_frequency_90d, 0);
    assert_eq!(signals.open_action_count, 0);
    assert_eq!(signals.temperature, "cold");
}

#[test]
fn test_link_meeting_to_project_and_query() {
    let db = test_db();
    let now = Utc::now().to_rfc3339();

    let project = DbProject {
        id: "proj-mtg".to_string(),
        name: "Meeting Project".to_string(),
        status: "active".to_string(),
        milestone: None,
        owner: None,
        target_date: None,
        tracker_path: None,
        parent_id: None,
        updated_at: now.clone(),
        archived: false,
        keywords: None,
        keywords_extracted_at: None,
        ..Default::default()
    };
    db.upsert_project(&project).expect("upsert project");

    // Insert a meeting directly
    db.conn
        .execute(
            "INSERT INTO meetings (id, title, meeting_type, start_time, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
            params!["mtg-proj-001", "Sprint Review", "internal", &now, &now],
        )
        .expect("insert meeting");
    db.conn
        .execute(
            "INSERT OR IGNORE INTO meeting_prep (meeting_id) VALUES (?1)",
            params!["mtg-proj-001"],
        )
        .expect("insert meeting_prep");
    db.conn
        .execute(
            "INSERT OR IGNORE INTO meeting_transcripts (meeting_id) VALUES (?1)",
            params!["mtg-proj-001"],
        )
        .expect("insert meeting_transcripts");

    // Link it
    db.link_meeting_to_project("mtg-proj-001", "proj-mtg")
        .expect("link");

    // Query via project
    let meetings = db
        .get_meetings_for_project("proj-mtg", 10)
        .expect("get meetings");
    assert_eq!(meetings.len(), 1);
    assert_eq!(meetings[0].title, "Sprint Review");

    // Idempotent
    db.link_meeting_to_project("mtg-proj-001", "proj-mtg")
        .expect("re-link should not fail");
    let meetings = db
        .get_meetings_for_project("proj-mtg", 10)
        .expect("still 1");
    assert_eq!(meetings.len(), 1);
}

// =========================================================================
// I52: Meeting-entity M2M junction tests
// =========================================================================

#[test]
fn test_generic_link_unlink_meeting_entity() {
    let db = test_db();
    let now = Utc::now().to_rfc3339();

    // Create an entity (account)
    db.conn
        .execute(
            "INSERT INTO entities (id, name, entity_type, updated_at) VALUES (?1, ?2, ?3, ?4)",
            params!["acme-ent", "Acme", "account", &now],
        )
        .expect("insert entity");

    db.conn
        .execute(
            "INSERT INTO meetings (id, title, meeting_type, start_time, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
            params!["mtg-j1", "Acme QBR", "customer", &now, &now],
        )
        .expect("insert meeting");
    db.conn
        .execute(
            "INSERT OR IGNORE INTO meeting_prep (meeting_id) VALUES (?1)",
            params!["mtg-j1"],
        )
        .expect("insert meeting_prep");
    db.conn
        .execute(
            "INSERT OR IGNORE INTO meeting_transcripts (meeting_id) VALUES (?1)",
            params!["mtg-j1"],
        )
        .expect("insert meeting_transcripts");

    // Link
    db.link_meeting_entity("mtg-j1", "acme-ent", "account")
        .expect("link");
    let entities = db.get_meeting_entities("mtg-j1").expect("get entities");
    assert_eq!(entities.len(), 1);
    assert_eq!(entities[0].name, "Acme");

    // Unlink
    db.unlink_meeting_entity("mtg-j1", "acme-ent")
        .expect("unlink");
    let entities = db.get_meeting_entities("mtg-j1").expect("empty now");
    assert_eq!(entities.len(), 0);
}

#[test]
fn test_meeting_multi_entity_link() {
    let db = test_db();
    let now = Utc::now().to_rfc3339();

    // Create an account entity and a project entity
    db.conn
        .execute(
            "INSERT INTO entities (id, name, entity_type, updated_at) VALUES (?1, ?2, ?3, ?4)",
            params!["acme-m2m", "Acme", "account", &now],
        )
        .expect("insert account entity");

    let project = DbProject {
        id: "proj-m2m".to_string(),
        name: "Migration".to_string(),
        status: "active".to_string(),
        milestone: None,
        owner: None,
        target_date: None,
        tracker_path: None,
        parent_id: None,
        updated_at: now.clone(),
        archived: false,
        keywords: None,
        keywords_extracted_at: None,
        ..Default::default()
    };
    db.upsert_project(&project).expect("upsert project");

    db.conn
        .execute(
            "INSERT INTO meetings (id, title, meeting_type, start_time, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
            params!["mtg-m2m", "Migration Review", "customer", &now, &now],
        )
        .expect("insert meeting");
    db.conn
        .execute(
            "INSERT OR IGNORE INTO meeting_prep (meeting_id) VALUES (?1)",
            params!["mtg-m2m"],
        )
        .expect("insert meeting_prep");
    db.conn
        .execute(
            "INSERT OR IGNORE INTO meeting_transcripts (meeting_id) VALUES (?1)",
            params!["mtg-m2m"],
        )
        .expect("insert meeting_transcripts");

    // Link to both account and project
    db.link_meeting_entity("mtg-m2m", "acme-m2m", "account")
        .expect("link account");
    db.link_meeting_entity("mtg-m2m", "proj-m2m", "project")
        .expect("link project");

    let entities = db.get_meeting_entities("mtg-m2m").expect("get entities");
    assert_eq!(entities.len(), 2);

    // Generic get_meetings_for_entity works for both
    let acme_meetings = db
        .get_meetings_for_entity("acme-m2m", 10)
        .expect("acme meetings");
    assert_eq!(acme_meetings.len(), 1);
    let proj_meetings = db
        .get_meetings_for_entity("proj-m2m", 10)
        .expect("proj meetings");
    assert_eq!(proj_meetings.len(), 1);
}

#[test]
fn test_link_meeting_entity_manual() {
    let db = test_db();
    let now = Utc::now().to_rfc3339();

    let meeting = DbMeeting {
        id: "mtg-link".to_string(),
        title: "Link Test".to_string(),
        meeting_type: "customer".to_string(),
        start_time: now.clone(),
        end_time: None,
        attendees: None,
        notes_path: None,
        summary: None,
        created_at: now.clone(),
        calendar_event_id: None,
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
    };
    db.upsert_meeting(&meeting).expect("upsert");
    db.link_meeting_entity("mtg-link", "acme-auto", "account")
        .expect("link");

    // Junction should contain the link
    let count: i32 = db
        .conn
        .query_row(
            "SELECT COUNT(*) FROM meeting_entities WHERE meeting_id = ?1 AND entity_id = ?2",
            params!["mtg-link", "acme-auto"],
            |row| row.get(0),
        )
        .expect("count");
    assert_eq!(count, 1);
}

#[test]
fn test_captures_with_project_id() {
    let db = test_db();

    db.insert_capture_with_project(
        "mtg-p1",
        "Sprint Review",
        None,
        Some("proj-cap"),
        "win",
        "Feature shipped",
    )
    .expect("insert");

    let captures = db.get_captures_for_project("proj-cap", 30).expect("query");
    assert_eq!(captures.len(), 1);
    assert_eq!(captures[0].project_id.as_deref(), Some("proj-cap"));
    assert_eq!(captures[0].content, "Feature shipped");

    // Regular insert_capture still works (project_id = None)
    db.insert_capture("mtg-p2", "Acme QBR", Some("acme"), "risk", "Budget freeze")
        .expect("insert without project");
    let proj_captures = db.get_captures_for_project("acme", 30).expect("query");
    assert_eq!(proj_captures.len(), 0); // acme is account_id, not project_id
}

// =========================================================================
// I124: Content Index tests
// =========================================================================

#[test]
fn test_upsert_and_get_content_files() {
    let db = test_db();
    let now = Utc::now().to_rfc3339();

    let file = DbContentFile {
        id: "acme/notes-md".to_string(),
        entity_id: "acme".to_string(),
        entity_type: "account".to_string(),
        filename: "notes.md".to_string(),
        relative_path: "Accounts/Acme/notes.md".to_string(),
        absolute_path: "/tmp/workspace/Accounts/Acme/notes.md".to_string(),
        format: "Markdown".to_string(),
        file_size: 1234,
        modified_at: now.clone(),
        indexed_at: now.clone(),
        extracted_at: None,
        summary: None,
        embeddings_generated_at: None,
        content_type: "notes".to_string(),
        priority: 7,
    };

    db.upsert_content_file(&file).unwrap();

    let files = db.get_entity_files("acme").unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].filename, "notes.md");
    assert_eq!(files[0].file_size, 1234);
    assert_eq!(files[0].format, "Markdown");
}

#[test]
fn test_delete_content_file() {
    let db = test_db();
    let now = Utc::now().to_rfc3339();

    let file = DbContentFile {
        id: "beta/report-pdf".to_string(),
        entity_id: "beta".to_string(),
        entity_type: "account".to_string(),
        filename: "report.pdf".to_string(),
        relative_path: "Accounts/Beta/report.pdf".to_string(),
        absolute_path: "/tmp/workspace/Accounts/Beta/report.pdf".to_string(),
        format: "Pdf".to_string(),
        file_size: 50000,
        modified_at: now.clone(),
        indexed_at: now.clone(),
        extracted_at: None,
        summary: None,
        embeddings_generated_at: None,
        content_type: "general".to_string(),
        priority: 5,
    };

    db.upsert_content_file(&file).unwrap();
    assert_eq!(db.get_entity_files("beta").unwrap().len(), 1);

    db.delete_content_file("beta/report-pdf").unwrap();
    assert_eq!(db.get_entity_files("beta").unwrap().len(), 0);
}

#[test]
fn test_coalesce_preserves_extraction() {
    let db = test_db();
    let now = Utc::now().to_rfc3339();

    // Insert with extraction data
    let file = DbContentFile {
        id: "gamma/doc-md".to_string(),
        entity_id: "gamma".to_string(),
        entity_type: "account".to_string(),
        filename: "doc.md".to_string(),
        relative_path: "Accounts/Gamma/doc.md".to_string(),
        absolute_path: "/tmp/workspace/Accounts/Gamma/doc.md".to_string(),
        format: "Markdown".to_string(),
        file_size: 500,
        modified_at: now.clone(),
        indexed_at: now.clone(),
        extracted_at: Some(now.clone()),
        summary: Some("Important document about things.".to_string()),
        embeddings_generated_at: Some(now.clone()),
        content_type: "general".to_string(),
        priority: 5,
    };
    db.upsert_content_file(&file).unwrap();

    // Upsert again without extraction data (simulating a re-scan)
    let file_rescan = DbContentFile {
        id: "gamma/doc-md".to_string(),
        entity_id: "gamma".to_string(),
        entity_type: "account".to_string(),
        filename: "doc.md".to_string(),
        relative_path: "Accounts/Gamma/doc.md".to_string(),
        absolute_path: "/tmp/workspace/Accounts/Gamma/doc.md".to_string(),
        format: "Markdown".to_string(),
        file_size: 600, // size changed
        modified_at: now.clone(),
        indexed_at: now.clone(),
        extracted_at: None, // Not re-extracted
        summary: None,      // Not re-extracted
        embeddings_generated_at: None,
        content_type: "general".to_string(),
        priority: 5,
    };
    db.upsert_content_file(&file_rescan).unwrap();

    // Extraction data should be preserved via COALESCE
    let files = db.get_entity_files("gamma").unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].file_size, 600); // Size updated
    assert!(files[0].extracted_at.is_some()); // Preserved
    assert_eq!(
        files[0].summary.as_deref(),
        Some("Important document about things.")
    ); // Preserved
}

#[test]
fn test_get_files_needing_embeddings() {
    let db = test_db();
    let now = Utc::now().to_rfc3339();

    let mut ready_file = DbContentFile {
        id: "emb/ready".to_string(),
        entity_id: "emb".to_string(),
        entity_type: "account".to_string(),
        filename: "ready.md".to_string(),
        relative_path: "Accounts/Emb/ready.md".to_string(),
        absolute_path: "/tmp/workspace/Accounts/Emb/ready.md".to_string(),
        format: "Markdown".to_string(),
        file_size: 100,
        modified_at: now.clone(),
        indexed_at: now.clone(),
        extracted_at: None,
        summary: None,
        embeddings_generated_at: Some(now.clone()),
        content_type: "general".to_string(),
        priority: 5,
    };
    db.upsert_content_file(&ready_file).unwrap();

    ready_file.id = "emb/stale".to_string();
    ready_file.filename = "stale.md".to_string();
    ready_file.relative_path = "Accounts/Emb/stale.md".to_string();
    ready_file.absolute_path = "/tmp/workspace/Accounts/Emb/stale.md".to_string();
    ready_file.embeddings_generated_at = None;
    db.upsert_content_file(&ready_file).unwrap();

    let needing = db.get_files_needing_embeddings(10).unwrap();
    assert_eq!(needing.len(), 1);
    assert_eq!(needing[0].id, "emb/stale");
}

#[test]
fn test_replace_content_embeddings_for_file() {
    let db = test_db();
    let now = Utc::now().to_rfc3339();

    let file = DbContentFile {
        id: "emb/file".to_string(),
        entity_id: "emb".to_string(),
        entity_type: "account".to_string(),
        filename: "file.md".to_string(),
        relative_path: "Accounts/Emb/file.md".to_string(),
        absolute_path: "/tmp/workspace/Accounts/Emb/file.md".to_string(),
        format: "Markdown".to_string(),
        file_size: 100,
        modified_at: now.clone(),
        indexed_at: now.clone(),
        extracted_at: None,
        summary: None,
        embeddings_generated_at: None,
        content_type: "general".to_string(),
        priority: 5,
    };
    db.upsert_content_file(&file).unwrap();

    let chunk = DbContentEmbedding {
        id: "chunk-1".to_string(),
        content_file_id: file.id.clone(),
        chunk_index: 0,
        chunk_text: "hello world".to_string(),
        embedding: vec![0, 1, 2, 3],
        created_at: now.clone(),
    };
    db.replace_content_embeddings_for_file(&file.id, &[chunk])
        .unwrap();
    db.set_embeddings_generated_at(&file.id, Some(&now))
        .unwrap();

    let chunks = db.get_entity_embedding_chunks("emb").unwrap();
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].chunk_text, "hello world");
}

#[test]
fn test_chat_session_roundtrip() {
    let db = test_db();
    let now = Utc::now().to_rfc3339();

    let session = db
        .create_chat_session("sess-1", Some("acme"), Some("account"), &now, &now)
        .unwrap();
    assert_eq!(session.id, "sess-1");

    let open = db
        .get_open_chat_session(Some("acme"), Some("account"))
        .unwrap()
        .unwrap();
    assert_eq!(open.id, "sess-1");

    let idx = db.get_next_chat_turn_index("sess-1").unwrap();
    assert_eq!(idx, 0);

    db.append_chat_turn("turn-1", "sess-1", idx, "user", "hi", &now)
        .unwrap();
    db.bump_chat_session_stats("sess-1", 1, Some("hi")).unwrap();

    let turns = db.get_chat_session_turns("sess-1", 10).unwrap();
    assert_eq!(turns.len(), 1);
    assert_eq!(turns[0].content, "hi");
}

// === I127/I128: Manual action creation & editing tests ===

#[test]
fn test_create_action_all_fields() {
    let db = test_db();
    let now = Utc::now().to_rfc3339();

    let action = DbAction {
        id: "manual-001".to_string(),
        title: "Call Jane about renewal".to_string(),
        priority: crate::action_status::PRIORITY_URGENT,
        status: crate::action_status::UNSTARTED.to_string(),
        created_at: now.clone(),
        due_date: Some("2026-02-15".to_string()),
        completed_at: None,
        account_id: Some("acme-corp".to_string()),
        project_id: Some("proj-q1".to_string()),
        source_type: Some("manual".to_string()),
        source_id: None,
        source_label: Some("Slack #cs-team".to_string()),
        context: Some("Jane mentioned churn risk in standup".to_string()),
        waiting_on: None,
        updated_at: now.clone(),
        person_id: Some("person-jane".to_string()),
        account_name: None,
        next_meeting_title: None,
        next_meeting_start: None,
        needs_decision: false,
        decision_owner: None,
        decision_stakes: None,
        linear_identifier: None,
        linear_url: None,
    };
    db.upsert_action(&action).unwrap();

    let fetched = db.get_action_by_id("manual-001").unwrap().unwrap();
    assert_eq!(fetched.title, "Call Jane about renewal");
    assert_eq!(fetched.priority, crate::action_status::PRIORITY_URGENT);
    assert_eq!(fetched.status, crate::action_status::UNSTARTED);
    assert_eq!(fetched.due_date.as_deref(), Some("2026-02-15"));
    assert_eq!(fetched.account_id.as_deref(), Some("acme-corp"));
    assert_eq!(fetched.project_id.as_deref(), Some("proj-q1"));
    assert_eq!(fetched.source_type.as_deref(), Some("manual"));
    assert_eq!(fetched.source_label.as_deref(), Some("Slack #cs-team"));
    assert_eq!(
        fetched.context.as_deref(),
        Some("Jane mentioned churn risk in standup")
    );
    assert_eq!(fetched.person_id.as_deref(), Some("person-jane"));
}

#[test]
fn test_create_action_defaults() {
    let db = test_db();

    // Simulate creating with title only — mirroring the create_action command defaults
    let now = Utc::now().to_rfc3339();
    let action = DbAction {
        id: "manual-002".to_string(),
        title: "Quick follow-up".to_string(),
        priority: crate::action_status::PRIORITY_MEDIUM,
        status: crate::action_status::UNSTARTED.to_string(),
        created_at: now.clone(),
        due_date: None,
        completed_at: None,
        account_id: None,
        project_id: None,
        source_type: Some("manual".to_string()),
        source_id: None,
        source_label: None,
        context: None,
        waiting_on: None,
        updated_at: now,
        person_id: None,
        account_name: None,
        next_meeting_title: None,
        next_meeting_start: None,
        needs_decision: false,
        decision_owner: None,
        decision_stakes: None,
        linear_identifier: None,
        linear_url: None,
    };
    db.upsert_action(&action).unwrap();

    let fetched = db.get_action_by_id("manual-002").unwrap().unwrap();
    assert_eq!(fetched.priority, crate::action_status::PRIORITY_MEDIUM);
    assert_eq!(fetched.status, crate::action_status::UNSTARTED);
    assert_eq!(fetched.source_type.as_deref(), Some("manual"));
    assert!(fetched.due_date.is_none());
    assert!(fetched.account_id.is_none());
    assert!(fetched.person_id.is_none());
    assert!(fetched.context.is_none());
}

#[test]
fn test_update_action_fields() {
    let db = test_db();

    // Create initial action
    let action = sample_action("update-001", "Original title");
    db.upsert_action(&action).unwrap();

    // Update specific fields (mirroring update_action command logic)
    let mut updated = db.get_action_by_id("update-001").unwrap().unwrap();
    updated.title = "Updated title".to_string();
    updated.due_date = Some("2026-03-01".to_string());
    updated.context = Some("New context added".to_string());
    updated.account_id = Some("acme".to_string());
    updated.person_id = Some("person-bob".to_string());
    updated.updated_at = Utc::now().to_rfc3339();
    db.upsert_action(&updated).unwrap();

    // Verify updates applied and other fields preserved
    let fetched = db.get_action_by_id("update-001").unwrap().unwrap();
    assert_eq!(fetched.title, "Updated title");
    assert_eq!(fetched.due_date.as_deref(), Some("2026-03-01"));
    assert_eq!(fetched.context.as_deref(), Some("New context added"));
    assert_eq!(fetched.account_id.as_deref(), Some("acme"));
    assert_eq!(fetched.person_id.as_deref(), Some("person-bob"));
    // Unchanged fields preserved
    assert_eq!(fetched.priority, crate::action_status::PRIORITY_MEDIUM);
    assert_eq!(fetched.status, crate::action_status::UNSTARTED);
}

#[test]
fn test_update_action_clear_fields() {
    let db = test_db();

    // Create action with fields populated
    let now = Utc::now().to_rfc3339();
    let action = DbAction {
        id: "clear-001".to_string(),
        title: "Action with fields".to_string(),
        priority: crate::action_status::PRIORITY_URGENT,
        status: crate::action_status::UNSTARTED.to_string(),
        created_at: now.clone(),
        due_date: Some("2026-02-20".to_string()),
        completed_at: None,
        account_id: Some("acme".to_string()),
        project_id: Some("proj-1".to_string()),
        source_type: Some("manual".to_string()),
        source_id: None,
        source_label: Some("Call".to_string()),
        context: Some("Some context".to_string()),
        waiting_on: None,
        updated_at: now,
        person_id: Some("person-alice".to_string()),
        account_name: None,
        next_meeting_title: None,
        next_meeting_start: None,
        needs_decision: false,
        decision_owner: None,
        decision_stakes: None,
        linear_identifier: None,
        linear_url: None,
    };
    db.upsert_action(&action).unwrap();

    // Clear specific fields (mirroring clear_* flags in update_action command)
    let mut cleared = db.get_action_by_id("clear-001").unwrap().unwrap();
    cleared.due_date = None;
    cleared.account_id = None;
    cleared.person_id = None;
    cleared.updated_at = Utc::now().to_rfc3339();
    db.upsert_action(&cleared).unwrap();

    let fetched = db.get_action_by_id("clear-001").unwrap().unwrap();
    assert!(fetched.due_date.is_none(), "due_date should be cleared");
    assert!(fetched.account_id.is_none(), "account_id should be cleared");
    assert!(fetched.person_id.is_none(), "person_id should be cleared");
    // Non-cleared fields preserved
    assert_eq!(fetched.title, "Action with fields");
    assert_eq!(fetched.priority, crate::action_status::PRIORITY_URGENT);
    assert_eq!(fetched.context.as_deref(), Some("Some context"));
    assert_eq!(fetched.source_label.as_deref(), Some("Call"));
    assert_eq!(fetched.project_id.as_deref(), Some("proj-1"));
}

#[test]
fn test_person_id_column() {
    let db = test_db();

    // Insert without person_id
    let action = sample_action("pid-001", "No person");
    db.upsert_action(&action).unwrap();

    let fetched = db.get_action_by_id("pid-001").unwrap().unwrap();
    assert!(fetched.person_id.is_none());

    // Update to add person_id
    let mut with_person = fetched;
    with_person.person_id = Some("person-charlie".to_string());
    with_person.updated_at = Utc::now().to_rfc3339();
    db.upsert_action(&with_person).unwrap();

    let fetched2 = db.get_action_by_id("pid-001").unwrap().unwrap();
    assert_eq!(fetched2.person_id.as_deref(), Some("person-charlie"));

    // Verify person_id appears in get_due_actions results too
    let due = db.get_due_actions(90).unwrap();
    let found = due.iter().find(|a| a.id == "pid-001").unwrap();
    assert_eq!(found.person_id.as_deref(), Some("person-charlie"));

    // Clear person_id
    let mut cleared = fetched2;
    cleared.person_id = None;
    cleared.updated_at = Utc::now().to_rfc3339();
    db.upsert_action(&cleared).unwrap();

    let fetched3 = db.get_action_by_id("pid-001").unwrap().unwrap();
    assert!(fetched3.person_id.is_none());
}

#[test]
fn test_manual_actions_in_non_briefing_query() {
    let db = test_db();

    // Manual action should appear in get_non_briefing_pending_actions
    let now = Utc::now().to_rfc3339();
    let action = DbAction {
        id: "manual-nbp".to_string(),
        title: "Manual task".to_string(),
        priority: crate::action_status::PRIORITY_MEDIUM,
        status: crate::action_status::UNSTARTED.to_string(),
        created_at: now.clone(),
        due_date: None,
        completed_at: None,
        account_id: None,
        project_id: None,
        source_type: Some("manual".to_string()),
        source_id: None,
        source_label: None,
        context: None,
        waiting_on: None,
        updated_at: now,
        person_id: None,
        account_name: None,
        next_meeting_title: None,
        next_meeting_start: None,
        needs_decision: false,
        decision_owner: None,
        decision_stakes: None,
        linear_identifier: None,
        linear_url: None,
    };
    db.upsert_action(&action).unwrap();

    let non_briefing = db.get_non_briefing_pending_actions().unwrap();
    let found = non_briefing.iter().find(|a| a.id == "manual-nbp");
    assert!(
        found.is_some(),
        "Manual actions should appear in non-briefing pending query"
    );
}

#[test]
fn test_get_latest_processing_status() {
    let db = test_db();

    // Insert two entries for the same file — only latest should be returned
    let entry1 = DbProcessingLog {
        id: "log-1".to_string(),
        filename: "report.pdf".to_string(),
        source_path: "/inbox/report.pdf".to_string(),
        destination_path: None,
        classification: "document".to_string(),
        status: "error".to_string(),
        processed_at: None,
        error_message: Some("parse failed".to_string()),
        created_at: "2025-01-01T00:00:00Z".to_string(),
    };
    db.insert_processing_log(&entry1).unwrap();

    let entry2 = DbProcessingLog {
        id: "log-2".to_string(),
        filename: "report.pdf".to_string(),
        source_path: "/inbox/report.pdf".to_string(),
        destination_path: Some("/accounts/acme/report.pdf".to_string()),
        classification: "document".to_string(),
        status: "completed".to_string(),
        processed_at: Some("2025-01-02T00:00:00Z".to_string()),
        error_message: None,
        created_at: "2025-01-02T00:00:00Z".to_string(),
    };
    db.insert_processing_log(&entry2).unwrap();

    // Insert a separate file with error status
    let entry3 = DbProcessingLog {
        id: "log-3".to_string(),
        filename: "notes.md".to_string(),
        source_path: "/inbox/notes.md".to_string(),
        destination_path: None,
        classification: "meeting".to_string(),
        status: "error".to_string(),
        processed_at: None,
        error_message: Some("AI enrichment timed out".to_string()),
        created_at: "2025-01-03T00:00:00Z".to_string(),
    };
    db.insert_processing_log(&entry3).unwrap();

    let map = db.get_latest_processing_status().unwrap();

    // Should have exactly 2 filenames
    assert_eq!(map.len(), 2);

    // report.pdf should show the LATEST entry (completed, no error)
    let (status, error) = map.get("report.pdf").expect("report.pdf should be in map");
    assert_eq!(status, "completed");
    assert!(error.is_none());

    // notes.md should show error with message
    let (status, error) = map.get("notes.md").expect("notes.md should be in map");
    assert_eq!(status, "error");
    assert_eq!(error.as_deref(), Some("AI enrichment timed out"));
}

// =========================================================================
// cascade_meeting_entity_to_people (I184)
// =========================================================================

/// Helper: create an account and ensure its entity row exists.
fn setup_account(db: &ActionDb, id: &str, name: &str) {
    let now = Utc::now().to_rfc3339();
    let account = DbAccount {
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
        account_type: crate::db::AccountType::Customer,
        updated_at: now,
        archived: false,
        keywords: None,
        keywords_extracted_at: None,
        metadata: None,
        ..Default::default()
    };
    db.upsert_account(&account).expect("upsert account");
    db.ensure_entity_for_account(&account)
        .expect("ensure entity");
}

/// Helper: create a meeting.
fn setup_meeting(db: &ActionDb, id: &str, title: &str) {
    let now = Utc::now().to_rfc3339();
    let meeting = DbMeeting {
        id: id.to_string(),
        title: title.to_string(),
        meeting_type: "external".to_string(),
        start_time: now.clone(),
        end_time: None,
        attendees: None,
        notes_path: None,
        summary: None,
        created_at: now,
        calendar_event_id: None,
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
    };
    db.upsert_meeting(&meeting).expect("upsert meeting");
}

#[test]
fn test_cascade_meeting_entity_to_people_external_only() {
    let db = test_db();
    setup_account(&db, "acc1", "Acme Corp");
    setup_meeting(&db, "m1", "Acme QBR");
    setup_meeting(&db, "m2", "Acme Follow-up");

    // External person attends 2 meetings with same account (I652: 2+ threshold)
    let mut external = sample_person("jane@acme.com");
    external.relationship = "external".to_string();
    db.upsert_person(&external).expect("upsert external");
    db.record_meeting_attendance("m1", &external.id)
        .expect("attend m1");
    db.record_meeting_attendance("m2", &external.id)
        .expect("attend m2");
    db.link_meeting_entity("m1", "acc1", "account")
        .expect("link m1");
    db.link_meeting_entity("m2", "acc1", "account")
        .expect("link m2");

    // Internal person → should NOT be linked regardless of meeting count
    let mut internal = sample_person("john@mycompany.com");
    internal.relationship = "internal".to_string();
    db.upsert_person(&internal).expect("upsert internal");
    db.record_meeting_attendance("m1", &internal.id)
        .expect("attend");

    // Cascade on m2 — jane has 2 meetings with acc1, threshold met
    let linked = db
        .cascade_meeting_entity_to_people("m2", Some("acc1"), None)
        .expect("cascade");
    assert!(linked >= 1, "external with 2+ meetings should be linked");

    // External person linked
    let team = db.get_account_team("acc1").expect("team");
    assert!(
        team.iter().any(|t| t.person_id == external.id),
        "jane should be in team"
    );

    // Internal person NOT linked
    assert!(
        !team.iter().any(|t| t.person_id == internal.id),
        "internal should not be in team"
    );
}

#[test]
fn test_cascade_meeting_entity_to_people_idempotent() {
    let db = test_db();
    setup_account(&db, "acc1", "Acme Corp");
    setup_meeting(&db, "m1", "Acme QBR");

    let mut person = sample_person("jane@acme.com");
    person.relationship = "external".to_string();
    db.upsert_person(&person).expect("upsert");
    db.record_meeting_attendance("m1", &person.id)
        .expect("attend");

    // Manually link person first
    db.link_person_to_entity(&person.id, "acc1", "associated")
        .expect("manual link");

    // Cascade should detect existing link, return 0 new
    let linked = db
        .cascade_meeting_entity_to_people("m1", Some("acc1"), None)
        .expect("cascade");
    assert_eq!(linked, 0);

    // Still only one link
    let entities = db.get_entities_for_person(&person.id).expect("entities");
    assert_eq!(entities.len(), 1);
}

#[test]
fn test_cascade_meeting_entity_to_people_no_entity() {
    let db = test_db();
    setup_meeting(&db, "m1", "Internal Sync");

    let mut person = sample_person("someone@test.com");
    person.relationship = "external".to_string();
    db.upsert_person(&person).expect("upsert");
    db.record_meeting_attendance("m1", &person.id)
        .expect("attend");

    // Cascade with no entity → 0 links
    let linked = db
        .cascade_meeting_entity_to_people("m1", None, None)
        .expect("cascade");
    assert_eq!(linked, 0);
}

// =========================================================================
// Domain reclassification tests (I184)
// =========================================================================

#[test]
fn test_reclassify_people_for_domains() {
    let db = test_db();

    // Create two people: one external, one unknown
    let mut p1 = sample_person("alice@subsidiary.com");
    p1.relationship = "external".to_string();
    db.upsert_person(&p1).expect("upsert");

    let mut p2 = sample_person("bob@vendor.com");
    p2.relationship = "external".to_string();
    db.upsert_person(&p2).expect("upsert");

    // Add subsidiary.com as internal domain
    let domains = vec!["myco.com".to_string(), "subsidiary.com".to_string()];
    let changed = db
        .reclassify_people_for_domains(&domains)
        .expect("reclassify");

    // alice should flip to internal, bob stays external
    assert_eq!(changed, 1);

    let alice = db.get_person(&p1.id).expect("get").unwrap();
    assert_eq!(alice.relationship, "internal");

    let bob = db.get_person(&p2.id).expect("get").unwrap();
    assert_eq!(bob.relationship, "external");
}

#[test]
fn test_reclassify_meeting_types_from_attendees() {
    let db = test_db();
    // DOS-225: Path A now requires full attendee coverage via the `attendees`
    // JSON blob, so the meeting must have it populated explicitly rather than
    // relying on junction-join coverage alone.
    setup_meeting_with_attendees(&db, "m1", "customer", r#"["alice@subsidiary.com"]"#);

    // Create person who is currently external
    let mut p1 = sample_person("alice@subsidiary.com");
    p1.relationship = "external".to_string();
    db.upsert_person(&p1).expect("upsert");
    db.record_meeting_attendance("m1", &p1.id).expect("attend");

    // Now reclassify alice as internal
    let domains = vec!["myco.com".to_string(), "subsidiary.com".to_string()];
    db.reclassify_people_for_domains(&domains)
        .expect("reclassify people");

    // Reclassify meetings
    let changed = db
        .reclassify_meeting_types_from_attendees()
        .expect("reclassify meetings");
    assert_eq!(changed, 1);

    let meeting: String = db
        .conn
        .query_row(
            "SELECT meeting_type FROM meetings WHERE id = 'm1'",
            [],
            |row| row.get(0),
        )
        .expect("query");
    assert_eq!(meeting, "internal");
}

/// DOS-206 regression: stale meeting rows must be swept even when no people
/// rows changed during the current reclassification pass.
///
/// Scenario: user adds a new user_domain. People records were already
/// correctly classified (alice@subsidiary.com was `internal` before this
/// sweep). But an old meeting is still stuck at `customer` because it was
/// classified before the people correction. Re-running
/// `reclassify_meeting_types_from_attendees()` must flip it to `internal`.
#[test]
fn test_dos206_reclassify_catches_stale_customer_meetings() {
    let db = test_db();
    // DOS-225: populate the attendees JSON blob — the new Path A enforces
    // full-attendee-coverage against this canonical list.
    setup_meeting_with_attendees(
        &db,
        "m_stale",
        "customer",
        r#"["alice@internal.co"]"#,
    );

    // Alice is ALREADY internal before the sweep — this mirrors the case
    // where domains were set correctly but an older meeting was misclassified.
    let mut alice = sample_person("alice@internal.co");
    alice.relationship = "internal".to_string();
    db.upsert_person(&alice).expect("upsert");
    db.record_meeting_attendance("m_stale", &alice.id)
        .expect("attend");

    // Reclassify: no people changed, but the meeting must still be flipped.
    let changed = db
        .reclassify_meeting_types_from_attendees()
        .expect("reclassify");
    assert!(
        changed >= 1,
        "stale customer meeting with all-internal attendees must be reclassified"
    );

    let meeting_type: String = db
        .conn
        .query_row(
            "SELECT meeting_type FROM meetings WHERE id = 'm_stale'",
            [],
            |row| row.get(0),
        )
        .expect("query");
    assert_eq!(meeting_type, "internal");
}

#[test]
fn test_reclassify_preserves_title_based_types() {
    let db = test_db();
    setup_meeting(&db, "m1", "All Hands");

    // Even with attendee changes, all_hands should not be touched
    db.conn
        .execute(
            "UPDATE meetings SET meeting_type = 'all_hands' WHERE id = 'm1'",
            [],
        )
        .expect("set type");

    let changed = db
        .reclassify_meeting_types_from_attendees()
        .expect("reclassify");
    assert_eq!(changed, 0);

    let meeting_type: String = db
        .conn
        .query_row(
            "SELECT meeting_type FROM meetings WHERE id = 'm1'",
            [],
            |row| row.get(0),
        )
        .expect("query");
    assert_eq!(meeting_type, "all_hands");
}

fn sample_account(id: &str, name: &str) -> DbAccount {
    DbAccount {
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
        account_type: crate::db::AccountType::Customer,
        updated_at: Utc::now().to_rfc3339(),
        archived: false,
        keywords: None,
        keywords_extracted_at: None,
        metadata: None,
        ..Default::default()
    }
}

#[test]
fn test_get_all_accounts_with_domains_single_query() {
    let db = test_db();

    let acct = sample_account("acme", "Acme Corp");
    db.upsert_account(&acct).unwrap();
    db.set_account_domains("acme", &["acme.com".to_string(), "acme.io".to_string()])
        .unwrap();

    let acct2 = sample_account("globex", "Globex Inc");
    db.upsert_account(&acct2).unwrap();
    db.set_account_domains("globex", &["globex.com".to_string()])
        .unwrap();

    let results = db.get_all_accounts_with_domains(false).unwrap();
    assert_eq!(results.len(), 2);

    // Find acme
    let acme = results.iter().find(|(a, _)| a.id == "acme").unwrap();
    assert_eq!(acme.0.name, "Acme Corp");
    assert_eq!(acme.1.len(), 2);
    assert!(acme.1.contains(&"acme.com".to_string()));
    assert!(acme.1.contains(&"acme.io".to_string()));

    // Find globex
    let globex = results.iter().find(|(a, _)| a.id == "globex").unwrap();
    assert_eq!(globex.1.len(), 1);
    assert_eq!(globex.1[0], "globex.com");
}

#[test]
fn test_get_all_accounts_with_domains_no_domains() {
    let db = test_db();

    let acct = sample_account("solo", "Solo Corp");
    db.upsert_account(&acct).unwrap();

    let results = db.get_all_accounts_with_domains(false).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].0.id, "solo");
    assert!(results[0].1.is_empty());
}

#[test]
fn test_get_all_accounts_with_domains_filters_archived() {
    let db = test_db();

    let active = sample_account("active", "Active Corp");
    db.upsert_account(&active).unwrap();
    db.set_account_domains("active", &["active.com".to_string()])
        .unwrap();

    let mut archived = sample_account("old", "Old Corp");
    archived.archived = true;
    db.upsert_account(&archived).unwrap();
    db.set_account_domains("old", &["old.com".to_string()])
        .unwrap();

    // Exclude archived
    let results = db.get_all_accounts_with_domains(false).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].0.id, "active");

    // Include archived
    let results = db.get_all_accounts_with_domains(true).unwrap();
    assert_eq!(results.len(), 2);
}

#[test]
fn test_merge_account_domains_additive() {
    let db = test_db();

    let acct = sample_account("acme", "Acme Corp");
    db.upsert_account(&acct).unwrap();

    // First merge: add two domains
    db.merge_account_domains("acme", &["acme.com".to_string(), "acme.io".to_string()])
        .unwrap();
    let domains = db.get_account_domains("acme").unwrap();
    assert_eq!(domains.len(), 2);

    // Second merge: add one new, one duplicate
    db.merge_account_domains("acme", &["acme.com".to_string(), "acme.co.uk".to_string()])
        .unwrap();
    let domains = db.get_account_domains("acme").unwrap();
    assert_eq!(
        domains.len(),
        3,
        "should add new without clobbering existing"
    );
    assert!(domains.contains(&"acme.com".to_string()));
    assert!(domains.contains(&"acme.io".to_string()));
    assert!(domains.contains(&"acme.co.uk".to_string()));
}

#[test]
fn test_insert_and_query_email_signals() {
    let db = test_db();
    setup_account(&db, "acc1", "Acme Corp");

    db.upsert_email_signal(&crate::db::signals::EmailSignalInput {
        email_id: "email-1",
        sender_email: Some("owner@acme.com"),
        person_id: None,
        entity_id: "acc1",
        entity_type: "account",
        signal_type: "timeline",
        signal_text: "Customer asked to move launch date by two weeks",
        confidence: Some(0.86),
        sentiment: Some("neutral"),
        urgency: Some("high"),
        detected_at: Some("2026-02-12T09:00:00Z"),
        source: None,
    })
    .expect("insert signal");

    // Duplicate should be ignored by dedupe unique index.
    db.upsert_email_signal(&crate::db::signals::EmailSignalInput {
        email_id: "email-1",
        sender_email: Some("owner@acme.com"),
        person_id: None,
        entity_id: "acc1",
        entity_type: "account",
        signal_type: "timeline",
        signal_text: "Customer asked to move launch date by two weeks",
        confidence: Some(0.86),
        sentiment: Some("neutral"),
        urgency: Some("high"),
        detected_at: Some("2026-02-12T09:00:00Z"),
        source: None,
    })
    .expect("insert duplicate signal");

    let signals = db
        .list_recent_email_signals_for_entity("acc1", 10)
        .expect("list signals");
    assert_eq!(signals.len(), 1);
    assert_eq!(signals[0].signal_type, "timeline");
    assert!(signals[0].signal_text.contains("launch date"));
    assert_eq!(signals[0].source, "email_enrichment");
}

#[test]
fn test_email_signal_source_attribution_roundtrip() {
    let db = test_db();
    setup_account(&db, "acc1", "Acme Corp");

    db.upsert_email_signal(&crate::db::signals::EmailSignalInput {
        email_id: "email-source",
        sender_email: Some("owner@acme.com"),
        person_id: None,
        entity_id: "acc1",
        entity_type: "account",
        signal_type: "timeline",
        signal_text: "Updated timeline requested",
        confidence: Some(0.8),
        sentiment: Some("neutral"),
        urgency: Some("medium"),
        detected_at: Some("2026-02-20T10:00:00Z"),
        source: Some("ai_classification"),
    })
    .expect("insert signal with source");

    let source = db
        .get_email_signal_source_for_feedback("email-source")
        .expect("lookup source");
    assert_eq!(source.as_deref(), Some("ai_classification"));
}

#[test]
fn test_email_signal_source_lookup_ignores_feedback_rows() {
    let db = test_db();
    setup_account(&db, "acc1", "Acme Corp");

    db.upsert_email_signal(&crate::db::signals::EmailSignalInput {
        email_id: "email-feedback",
        sender_email: Some("owner@acme.com"),
        person_id: None,
        entity_id: "acc1",
        entity_type: "account",
        signal_type: "timeline",
        signal_text: "Original signal",
        confidence: Some(0.8),
        sentiment: Some("neutral"),
        urgency: Some("medium"),
        detected_at: Some("2026-02-20T10:00:00Z"),
        source: Some("ai_classification"),
    })
    .expect("insert original signal");
    db.upsert_email_signal(&crate::db::signals::EmailSignalInput {
        email_id: "email-feedback",
        sender_email: None,
        person_id: None,
        entity_id: "system",
        entity_type: "account",
        signal_type: "feedback",
        signal_text: "User corrected priority",
        confidence: Some(1.0),
        sentiment: None,
        urgency: None,
        detected_at: Some("2026-02-20T11:00:00Z"),
        source: Some("user_feedback"),
    })
    .expect("insert feedback row");

    let source = db
        .get_email_signal_source_for_feedback("email-feedback")
        .expect("lookup source");
    assert_eq!(source.as_deref(), Some("ai_classification"));
}

#[test]
fn test_domain_based_account_lookup() {
    let db = test_db();
    setup_account(&db, "acc1", "Acme Corp");
    db.set_account_domains("acc1", &["acme.com".to_string()])
        .expect("set domains");

    let candidates = db
        .lookup_account_candidates_by_domain("acme.com")
        .expect("lookup domain");
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].id, "acc1");
}

// =========================================================================
// Email signal pipeline integration tests (S3)
//
// These test the same person-resolution → domain-fallback → signal-upsert
// pipeline used by Executor::sync_email_signals_from_payload, exercised
// at the DB layer to avoid needing a Tauri AppHandle.
// =========================================================================

#[test]
fn test_email_signal_pipeline_person_direct_match() {
    let db = test_db();
    setup_account(&db, "acc1", "Acme Corp");

    // Create person linked to account
    let person = sample_person("alice@acme.com");
    db.upsert_person(&person).expect("upsert person");
    db.link_person_to_entity(&person.id, "acc1", "contact")
        .expect("link person");

    // Simulate: person lookup → entity resolution → signal insert
    let sender = "alice@acme.com";
    let found = db
        .get_person_by_email(sender)
        .expect("lookup")
        .expect("person should exist");
    let entities = db.get_entities_for_person(&found.id).expect("get entities");
    assert_eq!(entities.len(), 1);
    assert_eq!(entities[0].id, "acc1");

    let inserted = db
        .upsert_email_signal(&crate::db::signals::EmailSignalInput {
            email_id: "email-1",

            sender_email: Some(sender),

            person_id: Some(&found.id),

            entity_id: &entities[0].id,

            entity_type: entities[0].entity_type.as_str(),

            signal_type: "expansion",

            signal_text: "Wants to add 50 seats in Q2",

            confidence: Some(0.85),

            sentiment: Some("positive"),

            urgency: Some("medium"),

            detected_at: Some("2026-02-13T10:00:00Z"),

            source: None,
        })
        .expect("insert signal");
    assert!(inserted);

    let signals = db
        .list_recent_email_signals_for_entity("acc1", 10)
        .expect("list signals");
    assert_eq!(signals.len(), 1);
    assert_eq!(signals[0].signal_type, "expansion");
    assert_eq!(signals[0].entity_id, "acc1");
    assert_eq!(signals[0].person_id, Some(found.id));
}

#[test]
fn test_email_signal_pipeline_domain_fallback() {
    let db = test_db();
    setup_account(&db, "acc1", "Acme Corp");
    db.set_account_domains("acc1", &["acme.com".to_string()])
        .expect("set domains");

    // No person record — simulate domain fallback
    let sender = "unknown@acme.com";
    let person = db.get_person_by_email(sender).expect("lookup");
    assert!(person.is_none(), "no person should match");

    // Domain fallback
    let domain = sender.split('@').nth(1).unwrap();
    let candidates = db
        .lookup_account_candidates_by_domain(domain)
        .expect("lookup domain");
    assert_eq!(candidates.len(), 1);

    let inserted = db
        .upsert_email_signal(&crate::db::signals::EmailSignalInput {

            email_id: "email-2",

            sender_email: Some(sender),

            person_id: None,

            entity_id: // no person_id
            &candidates[0].id,

            entity_type: "account",

            signal_type: "question",

            signal_text: "Asking about enterprise pricing",

            confidence: Some(0.75),

            sentiment: Some("neutral"),

            urgency: Some("low"),

            detected_at: None,

            source: None,

        })
        .expect("insert signal");
    assert!(inserted);

    let signals = db
        .list_recent_email_signals_for_entity("acc1", 10)
        .expect("list signals");
    assert_eq!(signals.len(), 1);
    assert_eq!(signals[0].signal_type, "question");
    assert!(signals[0].person_id.is_none());
}

#[test]
fn test_email_signal_pipeline_deduplication() {
    let db = test_db();
    setup_account(&db, "acc1", "Acme Corp");

    // Insert same signal twice (same email_id + entity)
    let first = db
        .upsert_email_signal(&crate::db::signals::EmailSignalInput {
            email_id: "email-dup",

            sender_email: Some("alice@acme.com"),

            person_id: None,

            entity_id: "acc1",

            entity_type: "account",

            signal_type: "expansion",

            signal_text: "Wants to expand",

            confidence: Some(0.85),

            sentiment: Some("positive"),

            urgency: Some("high"),

            detected_at: Some("2026-02-13T10:00:00Z"),

            source: None,
        })
        .expect("first insert");
    assert!(first);

    let second = db
        .upsert_email_signal(&crate::db::signals::EmailSignalInput {
            email_id: "email-dup",

            sender_email: Some("alice@acme.com"),

            person_id: None,

            entity_id: "acc1",

            entity_type: "account",

            signal_type: "expansion",

            signal_text: "Wants to expand",

            confidence: Some(0.85),

            sentiment: Some("positive"),

            urgency: Some("high"),

            detected_at: Some("2026-02-13T10:00:00Z"),

            source: None,
        })
        .expect("second insert");
    assert!(!second, "duplicate should return false");

    let signals = db
        .list_recent_email_signals_for_entity("acc1", 10)
        .expect("list");
    assert_eq!(signals.len(), 1, "only one signal despite two inserts");
}

#[test]
fn test_email_signal_pipeline_multi_entity_targets() {
    let db = test_db();
    setup_account(&db, "acc1", "Acme Corp");
    setup_account(&db, "acc2", "Acme Sub");

    // Person linked to two accounts
    let person = sample_person("alice@acme.com");
    db.upsert_person(&person).expect("upsert person");
    db.link_person_to_entity(&person.id, "acc1", "contact")
        .expect("link 1");
    db.link_person_to_entity(&person.id, "acc2", "contact")
        .expect("link 2");

    let entities = db
        .get_entities_for_person(&person.id)
        .expect("get entities");
    assert_eq!(entities.len(), 2);

    // Insert signal for each entity (mirrors executor loop)
    for entity in &entities {
        db.upsert_email_signal(&crate::db::signals::EmailSignalInput {
            email_id: "email-multi",
            sender_email: Some("alice@acme.com"),
            person_id: Some(&person.id),
            entity_id: &entity.id,
            entity_type: entity.entity_type.as_str(),
            signal_type: "feedback",
            signal_text: "Great experience with the new feature",
            confidence: Some(0.9),
            sentiment: Some("positive"),
            urgency: None,
            detected_at: Some("2026-02-13T11:00:00Z"),
            source: None,
        })
        .expect("insert");
    }

    let signals_acc1 = db
        .list_recent_email_signals_for_entity("acc1", 10)
        .expect("list acc1");
    let signals_acc2 = db
        .list_recent_email_signals_for_entity("acc2", 10)
        .expect("list acc2");
    assert_eq!(signals_acc1.len(), 1);
    assert_eq!(signals_acc2.len(), 1);
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MockInsertStatement {
    table: String,
    columns: Vec<String>,
    value_arities: Vec<usize>,
}

fn extract_rust_string_literals(source: &str) -> Vec<String> {
    let bytes = source.as_bytes();
    let mut literals = Vec::new();
    let mut i = 0usize;

    while i < bytes.len() {
        if bytes[i] == b'r' {
            let mut hashes = 0usize;
            let mut j = i + 1;
            let mut matched_raw = false;
            while j < bytes.len() && bytes[j] == b'#' {
                hashes += 1;
                j += 1;
            }
            if j < bytes.len() && bytes[j] == b'"' {
                let start = j + 1;
                let mut end = start;
                while end < bytes.len() {
                    if bytes[end] == b'"' {
                        let hash_slice_end = end + 1 + hashes;
                        if hash_slice_end <= bytes.len()
                            && bytes[end + 1..hash_slice_end].iter().all(|b| *b == b'#')
                        {
                            literals.push(source[start..end].to_string());
                            i = hash_slice_end;
                            matched_raw = true;
                            break;
                        }
                    }
                    end += 1;
                }
                if matched_raw {
                    continue;
                }
            }
        }

        if bytes[i] == b'"' {
            let mut value = String::new();
            let mut j = i + 1;
            let mut closed = false;
            while j < bytes.len() {
                match bytes[j] {
                    b'\\' => {
                        j += 1;
                        if j >= bytes.len() {
                            break;
                        }
                        match bytes[j] {
                            b'n' => value.push('\n'),
                            b'r' => value.push('\r'),
                            b't' => value.push('\t'),
                            b'"' => value.push('"'),
                            b'\\' => value.push('\\'),
                            b'\n' => {
                                j += 1;
                                while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                                    j += 1;
                                }
                                continue;
                            }
                            other => value.push(other as char),
                        }
                    }
                    b'"' => {
                        literals.push(value);
                        i = j + 1;
                        closed = true;
                        break;
                    }
                    other => value.push(other as char),
                }
                j += 1;
            }
            if closed {
                continue;
            }
        }

        i += 1;
    }

    literals
}

fn strip_sql_comments(sql: &str) -> String {
    let mut result = String::with_capacity(sql.len());
    let chars: Vec<char> = sql.chars().collect();
    let mut i = 0usize;
    let mut in_single_quote = false;

    while i < chars.len() {
        let ch = chars[i];
        if ch == '\'' {
            result.push(ch);
            if in_single_quote && i + 1 < chars.len() && chars[i + 1] == '\'' {
                result.push(chars[i + 1]);
                i += 2;
                continue;
            }
            in_single_quote = !in_single_quote;
            i += 1;
            continue;
        }

        if !in_single_quote && ch == '-' && i + 1 < chars.len() && chars[i + 1] == '-' {
            i += 2;
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }

        result.push(ch);
        i += 1;
    }

    result
}

fn split_top_level_csv(input: &str) -> Vec<String> {
    let mut items = Vec::new();
    let mut current = String::new();
    let mut depth = 0i32;
    let mut in_single_quote = false;
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0usize;

    while i < chars.len() {
        let ch = chars[i];
        if ch == '\'' {
            current.push(ch);
            if in_single_quote && i + 1 < chars.len() && chars[i + 1] == '\'' {
                current.push(chars[i + 1]);
                i += 2;
                continue;
            }
            in_single_quote = !in_single_quote;
            i += 1;
            continue;
        }

        if !in_single_quote {
            match ch {
                '(' => depth += 1,
                ')' => depth -= 1,
                ',' if depth == 0 => {
                    let trimmed = current.trim();
                    if !trimmed.is_empty() {
                        items.push(trimmed.to_string());
                    }
                    current.clear();
                    i += 1;
                    continue;
                }
                _ => {}
            }
        }

        current.push(ch);
        i += 1;
    }

    let trimmed = current.trim();
    if !trimmed.is_empty() {
        items.push(trimmed.to_string());
    }
    items
}

fn parse_values_groups(values_block: &str) -> Vec<String> {
    let mut groups = Vec::new();
    let chars: Vec<char> = values_block.chars().collect();
    let mut in_single_quote = false;
    let mut depth = 0i32;
    let mut start: Option<usize> = None;
    let mut i = 0usize;

    while i < chars.len() {
        let ch = chars[i];
        if ch == '\'' {
            if in_single_quote && i + 1 < chars.len() && chars[i + 1] == '\'' {
                i += 2;
                continue;
            }
            in_single_quote = !in_single_quote;
            i += 1;
            continue;
        }

        if !in_single_quote {
            if ch == '(' {
                if depth == 0 {
                    start = Some(i);
                }
                depth += 1;
            } else if ch == ')' {
                depth -= 1;
                if depth == 0 {
                    if let Some(group_start) = start.take() {
                        groups.push(chars[group_start..=i].iter().collect::<String>());
                    }
                }
            }
        }

        i += 1;
    }

    groups
}

fn extract_insert_statements(source: &str) -> Vec<MockInsertStatement> {
    let insert_re = Regex::new(
        r"(?is)^\s*INSERT(?:\s+OR\s+(?:IGNORE|REPLACE))?\s+INTO\s+([A-Za-z_][A-Za-z0-9_]*)\s*\((.*?)\)\s*VALUES\s*(.+?)(?:\s+ON\s+CONFLICT\b.*)?\s*$",
    )
    .expect("insert regex");

    extract_rust_string_literals(source)
        .into_iter()
        .filter_map(|literal| {
            let normalized = strip_sql_comments(&literal);
            let caps = insert_re.captures(normalized.trim())?;
            let table = caps.get(1)?.as_str().trim().to_string();
            let columns = split_top_level_csv(caps.get(2)?.as_str())
                .into_iter()
                .map(|c| {
                    c.trim()
                        .trim_matches('"')
                        .trim_matches('`')
                        .trim_matches('[')
                        .trim_matches(']')
                        .to_string()
                })
                .collect::<Vec<_>>();
            let value_groups = parse_values_groups(caps.get(3)?.as_str());
            if columns.is_empty() || value_groups.is_empty() {
                return None;
            }
            let value_arities = value_groups
                .into_iter()
                .map(|group| {
                    let trimmed = group.trim();
                    let inner = trimmed
                        .strip_prefix('(')
                        .and_then(|s| s.strip_suffix(')'))
                        .unwrap_or(trimmed);
                    split_top_level_csv(inner).len()
                })
                .collect::<Vec<_>>();

            Some(MockInsertStatement {
                table,
                columns,
                value_arities,
            })
        })
        .collect()
}

fn schema_columns(db: &ActionDb) -> HashMap<String, HashSet<String>> {
    let mut stmt = db
        .conn
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table'")
        .expect("prepare schema query");
    let tables = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .expect("query tables")
        .collect::<Result<Vec<_>, _>>()
        .expect("collect tables");

    let mut schema = HashMap::new();
    for table in tables {
        if table.starts_with("sqlite_") {
            continue;
        }
        let pragma = format!("PRAGMA table_info({table})");
        let mut column_stmt = db.conn.prepare(&pragma).expect("prepare table info");
        let columns = column_stmt
            .query_map([], |row| row.get::<_, String>(1))
            .expect("query columns")
            .collect::<Result<HashSet<_>, _>>()
            .expect("collect columns");
        schema.insert(table, columns);
    }

    schema
}

fn closest_match(target: &str, candidates: impl IntoIterator<Item = String>) -> Option<String> {
    let mut best: Option<(String, f64)> = None;
    for candidate in candidates {
        let mut score = jaro_winkler(target, &candidate);
        let target_parts: HashSet<&str> = target.split('_').collect();
        let candidate_parts: HashSet<&str> = candidate.split('_').collect();
        if target.contains(&candidate) || candidate.contains(target) {
            score += 0.25;
        }
        if target_parts.contains(candidate.as_str()) || candidate_parts.contains(target) {
            score = score.max(0.85);
        }
        if target_parts.intersection(&candidate_parts).next().is_some() {
            score += 0.1;
        }
        if score >= 0.72 {
            match &best {
                Some((_, best_score)) if *best_score >= score => {}
                _ => best = Some((candidate, score)),
            }
        }
    }
    best.map(|(candidate, _)| candidate)
}

fn validate_mock_insert_source(
    source_name: &str,
    source: &str,
    schema: &HashMap<String, HashSet<String>>,
) -> Vec<String> {
    let mut errors = Vec::new();

    for statement in extract_insert_statements(source) {
        let Some(table_columns) = schema.get(&statement.table) else {
            let suggestion = closest_match(&statement.table, schema.keys().cloned())
                .map(|candidate| format!(" Did you mean '{candidate}'?"))
                .unwrap_or_default();
            errors.push(format!(
                "{source_name}: Mock data references table '{}' which does not exist.{suggestion}",
                statement.table
            ));
            continue;
        };

        for column in &statement.columns {
            if !table_columns.contains(column) {
                let suggestion = closest_match(column, table_columns.iter().cloned())
                    .map(|candidate| format!(" Did you mean '{candidate}'?"))
                    .unwrap_or_default();
                errors.push(format!(
                    "{source_name}: Mock data references column '{}.{}' which does not exist.{suggestion}",
                    statement.table, column
                ));
            }
        }

        for arity in &statement.value_arities {
            if *arity != statement.columns.len() {
                errors.push(format!(
                    "{source_name}: INSERT INTO {} has {} columns but {} values in one tuple.",
                    statement.table,
                    statement.columns.len(),
                    arity
                ));
            }
        }
    }

    errors
}

fn sample_db_meeting(id: &str, title: &str, start_time: &str) -> DbMeeting {
    DbMeeting {
        id: id.to_string(),
        title: title.to_string(),
        meeting_type: "customer".to_string(),
        start_time: start_time.to_string(),
        end_time: Some(start_time.to_string()),
        attendees: None,
        notes_path: None,
        summary: None,
        created_at: start_time.to_string(),
        calendar_event_id: None,
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
    }
}

#[test]
fn test_mock_data_insert_statements_match_current_schema() {
    let db = test_db();
    let schema = schema_columns(&db);
    let mut errors = Vec::new();

    errors.extend(validate_mock_insert_source(
        "src-tauri/src/demo.rs",
        include_str!("../demo.rs"),
        &schema,
    ));
    errors.extend(validate_mock_insert_source(
        "src-tauri/src/devtools/mod.rs",
        include_str!("../devtools/mod.rs"),
        &schema,
    ));

    assert!(
        errors.is_empty(),
        "mock data validation failed:\n{}",
        errors.join("\n")
    );
}

#[test]
fn test_mock_validator_reports_missing_table_with_suggestion() {
    let db = test_db();
    let schema = schema_columns(&db);
    let errors = validate_mock_insert_source(
        "bad-table.sql",
        r#""INSERT INTO enriched_captures (id, meeting_id) VALUES (?1, ?2)""#,
        &schema,
    );

    assert_eq!(errors.len(), 1);
    assert!(errors[0].contains("enriched_captures"));
    assert!(errors[0].contains("Did you mean 'captures'?"));
}

#[test]
fn test_mock_validator_reports_missing_column_with_suggestion() {
    let db = test_db();
    let schema = schema_columns(&db);
    let errors = validate_mock_insert_source(
        "bad-column.sql",
        r#""INSERT INTO captures (id, meeting_id, meeting_title, bogus_column) VALUES (?1, ?2, ?3, ?4)""#,
        &schema,
    );

    assert_eq!(errors.len(), 1);
    assert!(errors[0].contains("captures.bogus_column"));
}

#[test]
fn test_mock_validator_reports_value_arity_mismatch() {
    let db = test_db();
    let schema = schema_columns(&db);
    let errors = validate_mock_insert_source(
        "bad-arity.sql",
        r#""INSERT INTO captures (id, meeting_id, meeting_title) VALUES (?1, ?2)""#,
        &schema,
    );

    assert_eq!(errors.len(), 1);
    assert!(errors[0].contains("has 3 columns but 2 values"));
}

#[test]
fn test_get_previous_meeting_for_entity_returns_none_for_first_meeting() {
    let db = test_db();
    setup_account(&db, "acc-first", "First Corp");

    let meeting = sample_db_meeting("mtg-first", "First Corp Kickoff", "2026-03-20T15:00:00Z");
    db.upsert_meeting(&meeting).expect("upsert current meeting");
    db.link_meeting_entity(&meeting.id, "acc-first", "account")
        .expect("link meeting");

    let previous = db
        .get_previous_meeting_for_entity("acc-first", "account", &meeting.start_time)
        .expect("lookup previous meeting");

    assert!(previous.is_none());
}

#[test]
fn test_get_continuity_thread_includes_actions_health_delta_and_new_attendees() {
    let db = test_db();
    setup_account(&db, "acc-thread", "Thread Corp");

    let previous = sample_db_meeting(
        "mtg-prev",
        "Thread Corp Weekly Sync",
        "2026-03-10T15:00:00Z",
    );
    let current = sample_db_meeting(
        "mtg-current",
        "Thread Corp Weekly Sync",
        "2026-03-20T15:00:00Z",
    );
    db.upsert_meeting(&previous)
        .expect("upsert previous meeting");
    db.upsert_meeting(&current).expect("upsert current meeting");
    db.link_meeting_entity(&previous.id, "acc-thread", "account")
        .expect("link previous");
    db.link_meeting_entity(&current.id, "acc-thread", "account")
        .expect("link current");

    let existing = sample_person("taylor@thread.com");
    let newcomer = sample_person("jordan@thread.com");
    db.upsert_person(&existing)
        .expect("upsert existing attendee");
    db.upsert_person(&newcomer)
        .expect("upsert newcomer attendee");
    db.record_meeting_attendance(&previous.id, &existing.id)
        .expect("record previous attendance");
    db.record_meeting_attendance(&current.id, &existing.id)
        .expect("record repeated attendance");
    db.record_meeting_attendance(&current.id, &newcomer.id)
        .expect("record new attendance");

    let mut completed = sample_action("act-thread-complete", "Finalize mutual action plan");
    completed.account_id = Some("acc-thread".to_string());
    completed.status = "completed".to_string();
    completed.completed_at = Some("2026-03-15T10:00:00Z".to_string());
    db.upsert_action(&completed)
        .expect("upsert completed action");

    let mut open = sample_action("act-thread-open", "Review pricing addendum");
    open.account_id = Some("acc-thread".to_string());
    open.due_date = Some("2026-03-25".to_string());
    db.upsert_action(&open).expect("upsert open action");

    db.conn
        .execute(
            "INSERT INTO health_score_history (account_id, score, band, confidence, computed_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params!["acc-thread", 72.0, "yellow", 0.8, "2026-03-09T09:00:00Z"],
        )
        .expect("insert previous health score");
    db.conn
        .execute(
            "INSERT INTO health_score_history (account_id, score, band, confidence, computed_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params!["acc-thread", 84.0, "green", 0.9, "2026-03-19T09:00:00Z"],
        )
        .expect("insert current health score");

    let thread = db
        .get_continuity_thread(
            "acc-thread",
            &current.id,
            &previous.id,
            &previous.start_time,
            &current.start_time,
        )
        .expect("continuity thread");

    assert_eq!(thread.actions_completed.len(), 1);
    assert_eq!(
        thread.actions_completed[0].title,
        "Finalize mutual action plan"
    );
    assert_eq!(thread.actions_open.len(), 1);
    assert_eq!(thread.actions_open[0].title, "Review pricing addendum");
    assert_eq!(thread.actions_open[0].date.as_deref(), Some("2026-03-25"));
    assert_eq!(thread.new_attendees, vec![newcomer.name]);

    let health_delta = thread.health_delta.expect("health delta");
    assert_eq!(health_delta.previous, 72.0);
    assert_eq!(health_delta.current, 84.0);
    assert!(!thread.is_first_meeting);
}

/// DOS-74 regression: when a meeting's linked_entities junction contains a
/// high-confidence primary plus a low-confidence sibling (a "suggestion"),
/// `get_meeting_linked_entities` returns them in primary-first, confidence-
/// descending order and flags the sibling as `suggested`.
#[test]
fn test_dos74_get_meeting_linked_entities_primary_then_suggestions() {
    let db = test_db();

    // Seed two accounts + their junction entity rows.
    let parent = sample_account("parent", "Parent Co");
    db.upsert_account(&parent).expect("upsert parent");
    let sub = sample_account("sub", "Subsidiary BU");
    db.upsert_account(&sub).expect("upsert sub");

    setup_meeting(&db, "m_dos74", "Joint Strategy Sync");

    // High-confidence primary link (e.g. resolved from strong domain signal).
    db.link_meeting_entity_with_confidence("m_dos74", "parent", "account", 0.95, true)
        .expect("link primary");
    // Low-confidence sibling link (domain_sibling tier, DOS-74 suggestion).
    db.link_meeting_entity_with_confidence("m_dos74", "sub", "account", 0.45, false)
        .expect("link suggestion");

    let linked = db
        .get_meeting_linked_entities("m_dos74")
        .expect("get linked");
    assert_eq!(linked.len(), 2);

    // [0] must be the primary, [1] the suggestion.
    assert_eq!(linked[0].id, "parent");
    assert!(linked[0].is_primary);
    assert!(!linked[0].suggested);
    assert!(linked[0].confidence >= 0.9);

    assert_eq!(linked[1].id, "sub");
    assert!(!linked[1].is_primary);
    assert!(linked[1].suggested, "low-confidence sibling must render as suggestion");
    assert!(linked[1].confidence < 0.60);
}

/// DOS-74 regression: `link_meeting_entity_with_confidence` must NEVER
/// downgrade an existing high-confidence link. If a manual/junction
/// resolution already wrote confidence 0.95 / is_primary=true, a later
/// background sweep at 0.45 must not stomp that row.
#[test]
fn test_dos74_link_confidence_never_downgrades() {
    let db = test_db();
    let acct = sample_account("acct1", "Acct One");
    db.upsert_account(&acct).expect("upsert");
    setup_meeting(&db, "m1", "Sync");

    // First: low confidence sibling write.
    db.link_meeting_entity_with_confidence("m1", "acct1", "account", 0.45, false)
        .expect("initial weak link");
    // Second: high confidence primary write — should upgrade.
    db.link_meeting_entity_with_confidence("m1", "acct1", "account", 0.95, true)
        .expect("upgrade to primary");

    let linked = db.get_meeting_linked_entities("m1").expect("get");
    assert_eq!(linked.len(), 1);
    assert!(linked[0].is_primary);
    assert!(linked[0].confidence >= 0.9);

    // Third: another weak write should NOT downgrade.
    db.link_meeting_entity_with_confidence("m1", "acct1", "account", 0.30, false)
        .expect("weak retry");
    let linked = db.get_meeting_linked_entities("m1").expect("get");
    assert!(linked[0].is_primary, "primary flag must not be cleared");
    assert!(linked[0].confidence >= 0.9, "confidence must not decrease");
}

#[test]
fn test_dos228_sentiment_note_bound_to_current_value() {
    // DOS-228 Fix 1: get_latest_sentiment_note must return None when the
    // account's current sentiment has no associated note, even if older
    // sentiment values have notes in the journal history.
    let db = test_db();
    let acct = sample_account("acct-sent", "Sent Co");
    db.upsert_account(&acct).expect("upsert");

    // Step 1: set sentiment to at_risk with a note
    db.update_account_field("acct-sent", "user_health_sentiment", "at_risk")
        .expect("set at_risk");
    db.insert_sentiment_journal_entry(
        "acct-sent",
        "at_risk",
        Some("churn risk — exec escalation"),
        None,
        None,
    )
    .expect("journal at_risk");

    let note = db
        .get_latest_sentiment_note("acct-sent")
        .expect("lookup1");
    assert!(
        note.as_ref().map(|(n, _)| n.as_str()) == Some("churn risk — exec escalation"),
        "should return the at_risk note while at_risk is current, got {:?}",
        note
    );

    // Step 2: transition to on_track with NO note
    db.update_account_field("acct-sent", "user_health_sentiment", "on_track")
        .expect("set on_track");
    db.insert_sentiment_journal_entry("acct-sent", "on_track", None, None, None)
        .expect("journal on_track");

    // Assertion: the stale at_risk note must NOT leak through.
    let note = db
        .get_latest_sentiment_note("acct-sent")
        .expect("lookup2");
    assert_eq!(
        note, None,
        "note must be None when current sentiment has no note attached"
    );

    // Step 3: add a note against on_track — should surface now
    db.insert_sentiment_journal_entry(
        "acct-sent",
        "on_track",
        Some("stabilized after QBR"),
        None,
        None,
    )
    .expect("journal on_track with note");
    let note = db
        .get_latest_sentiment_note("acct-sent")
        .expect("lookup3");
    assert_eq!(
        note.map(|(n, _)| n),
        Some("stabilized after QBR".to_string())
    );
}

#[test]
fn test_dos228_sentiment_note_none_when_no_current_sentiment() {
    // DOS-228 Fix 1: when user_health_sentiment is NULL, return None even if
    // historical journal entries exist.
    let db = test_db();
    let acct = sample_account("acct-no-sent", "NoSent Co");
    db.upsert_account(&acct).expect("upsert");

    // Historical note against at_risk, but no current sentiment set.
    db.insert_sentiment_journal_entry(
        "acct-no-sent",
        "at_risk",
        Some("legacy note"),
        None,
        None,
    )
    .expect("journal");

    let note = db
        .get_latest_sentiment_note("acct-no-sent")
        .expect("lookup");
    assert_eq!(note, None);
}

#[test]
fn test_dos228_risk_briefing_job_lifecycle() {
    // DOS-228 Fix 3: job progresses enqueued → running → complete.
    let db = test_db();
    let acct = sample_account("acct-rb", "RB Co");
    db.upsert_account(&acct).expect("upsert");

    // Initially no job exists.
    assert!(db
        .get_risk_briefing_job("acct-rb")
        .expect("get")
        .is_none());

    // Enqueue.
    db.upsert_risk_briefing_job_enqueued("acct-rb")
        .expect("enqueue");
    let job = db
        .get_risk_briefing_job("acct-rb")
        .expect("get")
        .expect("present");
    assert_eq!(job.status, "enqueued");
    assert!(job.completed_at.is_none());
    assert!(job.error_message.is_none());

    // Running.
    db.mark_risk_briefing_job_running("acct-rb")
        .expect("running");
    let job = db
        .get_risk_briefing_job("acct-rb")
        .expect("get")
        .expect("present");
    assert_eq!(job.status, "running");

    // Complete.
    db.mark_risk_briefing_job_complete("acct-rb")
        .expect("complete");
    let job = db
        .get_risk_briefing_job("acct-rb")
        .expect("get")
        .expect("present");
    assert_eq!(job.status, "complete");
    assert!(job.completed_at.is_some());
    assert!(job.error_message.is_none());
}

#[test]
fn test_dos228_risk_briefing_job_failure_path() {
    // DOS-228 Fix 3: failed jobs persist an error_message so the UI can
    // explain the retry prompt.
    let db = test_db();
    let acct = sample_account("acct-rb-fail", "Fail Co");
    db.upsert_account(&acct).expect("upsert");

    db.upsert_risk_briefing_job_enqueued("acct-rb-fail")
        .expect("enqueue");
    db.mark_risk_briefing_job_running("acct-rb-fail")
        .expect("running");
    db.mark_risk_briefing_job_failed("acct-rb-fail", "Claude timeout after 30s")
        .expect("failed");

    let job = db
        .get_risk_briefing_job("acct-rb-fail")
        .expect("get")
        .expect("present");
    assert_eq!(job.status, "failed");
    assert_eq!(
        job.error_message.as_deref(),
        Some("Claude timeout after 30s")
    );
    assert!(job.completed_at.is_some());

    // Retry: re-enqueuing must reset status and clear the error.
    db.upsert_risk_briefing_job_enqueued("acct-rb-fail")
        .expect("retry enqueue");
    let job = db
        .get_risk_briefing_job("acct-rb-fail")
        .expect("get")
        .expect("present");
    assert_eq!(job.status, "enqueued");
    assert!(
        job.error_message.is_none(),
        "re-enqueue must clear prior error_message"
    );
    assert!(job.completed_at.is_none());
}

#[test]
fn test_dos228_risk_briefing_job_error_truncation() {
    // DOS-228 Fix 3: huge error blobs are truncated to 2000 chars so a
    // runaway PTY stderr cannot bloat the DB.
    let db = test_db();
    let acct = sample_account("acct-rb-big", "Big Co");
    db.upsert_account(&acct).expect("upsert");

    db.upsert_risk_briefing_job_enqueued("acct-rb-big")
        .expect("enqueue");
    let huge = "x".repeat(10_000);
    db.mark_risk_briefing_job_failed("acct-rb-big", &huge)
        .expect("failed");

    let job = db
        .get_risk_briefing_job("acct-rb-big")
        .expect("get")
        .expect("present");
    assert_eq!(
        job.error_message.as_ref().map(|s| s.chars().count()),
        Some(2_000)
    );
}

// ---------------------------------------------------------------------------
// DOS-240: meeting-entity dismissal dictionary
// ---------------------------------------------------------------------------

#[test]
fn test_dos240_dismissal_roundtrip() {
    let db = test_db();
    assert!(!db
        .is_meeting_entity_dismissed("m1", "acct1", "account")
        .expect("probe"));

    db.record_meeting_entity_dismissal("m1", "acct1", "account", Some("user"))
        .expect("record");
    assert!(db
        .is_meeting_entity_dismissed("m1", "acct1", "account")
        .expect("probe"));

    let set = db
        .list_dismissed_meeting_entities("m1")
        .expect("list");
    assert!(set.contains(&("acct1".to_string(), "account".to_string())));

    // Undo / restore
    let removed = db
        .remove_meeting_entity_dismissal("m1", "acct1", "account")
        .expect("remove");
    assert!(removed);
    assert!(!db
        .is_meeting_entity_dismissed("m1", "acct1", "account")
        .expect("probe after remove"));
}

// ---------------------------------------------------------------------------
// DOS-224: single-primary invariant on scored junction writes
// ---------------------------------------------------------------------------

#[test]
fn test_dos224_new_primary_demotes_stale_primary() {
    // Codex: the previous `ON CONFLICT DO UPDATE SET is_primary = MAX(..)`
    // could never demote an existing primary. If batch N elected A as
    // primary and batch N+1 elected B, both ended up `is_primary = 1` for
    // the same (meeting_id, entity_type). Fix runs demote-then-upsert in a
    // transaction so exactly one row has `is_primary = 1` per
    // (meeting_id, entity_type).
    let db = test_db();
    let meeting_id = "mtg-dos224";

    // Batch N: stale primary A at 0.95.
    db.link_meeting_entity_with_confidence(meeting_id, "acct-a", "account", 0.95, true)
        .expect("seed primary A");

    // Batch N+1: a different candidate B wins primary at 0.98.
    db.link_meeting_entity_with_confidence(meeting_id, "acct-b", "account", 0.98, true)
        .expect("promote primary B");

    // Exactly one row must have is_primary = 1 for (meeting_id, entity_type).
    let primaries: Vec<(String, i64)> = db
        .conn_ref()
        .prepare(
            "SELECT entity_id, is_primary FROM meeting_entities
             WHERE meeting_id = ?1 AND entity_type = 'account'
             ORDER BY entity_id",
        )
        .expect("prepare")
        .query_map(rusqlite::params![meeting_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })
        .expect("query")
        .collect::<Result<Vec<_>, _>>()
        .expect("collect");

    assert_eq!(primaries.len(), 2, "both rows should still exist");
    let primary_count: usize = primaries.iter().filter(|(_, p)| *p == 1).count();
    assert_eq!(
        primary_count, 1,
        "exactly one row must be is_primary = 1 per (meeting_id, entity_type)",
    );
    let (only_primary, _) = primaries
        .iter()
        .find(|(_, p)| *p == 1)
        .expect("one primary");
    assert_eq!(only_primary, "acct-b", "newer candidate B must be the primary");
}

#[test]
fn test_dos224_suggestion_write_does_not_demote_primary() {
    // A non-primary (suggestion) write must never demote an existing
    // primary on the same (meeting_id, entity_type).
    let db = test_db();
    let meeting_id = "mtg-dos224-sugg";

    db.link_meeting_entity_with_confidence(meeting_id, "acct-a", "account", 0.95, true)
        .expect("seed primary A");
    db.link_meeting_entity_with_confidence(meeting_id, "acct-b", "account", 0.70, false)
        .expect("write suggestion B");

    let primary_a: i64 = db
        .conn_ref()
        .query_row(
            "SELECT is_primary FROM meeting_entities WHERE meeting_id = ?1 AND entity_id = 'acct-a'",
            rusqlite::params![meeting_id],
            |row| row.get(0),
        )
        .expect("get A");
    let primary_b: i64 = db
        .conn_ref()
        .query_row(
            "SELECT is_primary FROM meeting_entities WHERE meeting_id = ?1 AND entity_id = 'acct-b'",
            rusqlite::params![meeting_id],
            |row| row.get(0),
        )
        .expect("get B");
    assert_eq!(primary_a, 1, "existing primary A must remain primary");
    assert_eq!(primary_b, 0, "suggestion B must not be promoted");
}

#[test]
fn test_dos224_primary_upsert_does_not_downgrade_confidence() {
    // When the same entity is re-asserted as primary at lower confidence,
    // the stored confidence must not decrease (existing MAX(..) guarantee
    // still holds post-fix).
    let db = test_db();
    let meeting_id = "mtg-dos224-conf";

    db.link_meeting_entity_with_confidence(meeting_id, "acct-a", "account", 0.95, true)
        .expect("seed primary A @ 0.95");
    db.link_meeting_entity_with_confidence(meeting_id, "acct-a", "account", 0.60, true)
        .expect("re-assert A @ 0.60");

    let conf: f64 = db
        .conn_ref()
        .query_row(
            "SELECT confidence FROM meeting_entities WHERE meeting_id = ?1 AND entity_id = 'acct-a'",
            rusqlite::params![meeting_id],
            |row| row.get(0),
        )
        .expect("get");
    assert!(conf >= 0.95, "confidence must not be downgraded, got {}", conf);
}

// ---------------------------------------------------------------------------
// DOS-225: Path A reclassify — full-attendee-coverage regression
// ---------------------------------------------------------------------------

fn setup_meeting_with_attendees(
    db: &ActionDb,
    id: &str,
    meeting_type: &str,
    attendees_json: &str,
) {
    let now = Utc::now().to_rfc3339();
    let meeting = DbMeeting {
        id: id.to_string(),
        title: "test".to_string(),
        meeting_type: meeting_type.to_string(),
        start_time: now.clone(),
        end_time: None,
        attendees: Some(attendees_json.to_string()),
        notes_path: None,
        summary: None,
        created_at: now,
        calendar_event_id: None,
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
    };
    db.upsert_meeting(&meeting).expect("upsert meeting");
}

/// DOS-225 regression: a meeting whose raw-email attendee list contains
/// BOTH an internal attendee (with a people row) AND an external attendee
/// (no people row, unknown domain) must NOT flip to `internal`. The prior
/// Path A implementation JOIN-dropped the external attendee and the
/// aggregate falsely reported all-internal coverage.
#[test]
fn test_dos225_mixed_attendees_blocks_internal_flip() {
    let db = test_db();
    // Internal attendee — has a people row, relationship=internal.
    let mut internal = sample_person("alice@company.com");
    internal.relationship = "internal".to_string();
    db.upsert_person(&internal).expect("upsert internal");

    // Attendee JSON contains the internal email AND an external email that
    // has NO people row (simulates a fresh Gmail attendee).
    let attendees_json = r#"["alice@company.com","stranger@external.com"]"#;
    setup_meeting_with_attendees(&db, "m_mixed", "customer", attendees_json);

    // Link only the internal attendee via the junction (the external never
    // materialized into `people`).
    db.record_meeting_attendance("m_mixed", &internal.id)
        .expect("record");

    let _changed = db
        .reclassify_meeting_types_from_attendees()
        .expect("reclassify");

    // Must stay customer — unresolved external blocks the flip.
    let row = db.get_meeting_by_id("m_mixed").expect("get").expect("some");
    assert_eq!(
        row.meeting_type, "customer",
        "DOS-225: unresolved external attendee must block Path A flip"
    );
}

/// DOS-225 regression: a fully-internal meeting (every raw attendee has a
/// people row with relationship=internal) must still flip to `internal`.
#[test]
fn test_dos225_fully_internal_still_flips() {
    let db = test_db();
    let mut alice = sample_person("alice@company.com");
    alice.relationship = "internal".to_string();
    db.upsert_person(&alice).expect("upsert alice");
    let mut bob = sample_person("bob@company.com");
    bob.relationship = "internal".to_string();
    db.upsert_person(&bob).expect("upsert bob");

    let attendees_json = r#"["alice@company.com","bob@company.com"]"#;
    setup_meeting_with_attendees(&db, "m_internal", "customer", attendees_json);
    db.record_meeting_attendance("m_internal", &alice.id)
        .expect("rec a");
    db.record_meeting_attendance("m_internal", &bob.id)
        .expect("rec b");

    let _ = db
        .reclassify_meeting_types_from_attendees()
        .expect("reclassify");

    let row = db.get_meeting_by_id("m_internal").expect("get").expect("some");
    assert_eq!(
        row.meeting_type, "internal",
        "DOS-225: all-internal coverage must flip"
    );
}

// ---------------------------------------------------------------------------
// DOS-224: scored persistence — calendar-sync path
// ---------------------------------------------------------------------------

use crate::google_api::classify::ResolvedMeetingEntity as Rme;

fn rme(id: &str, confidence: f64, source: &str) -> Rme {
    Rme {
        entity_id: id.to_string(),
        entity_type: "account".to_string(),
        name: id.to_string(),
        confidence,
        source: source.to_string(),
    }
}

/// DOS-224 regression: an all-internal meeting that title-matched a known
/// account slug (confidence 0.50 / source="title") must NOT produce an
/// `is_primary = 1` junction row on the calendar-sync path. The entity
/// still persists — as a non-primary suggestion — so the UI can surface it
/// in the affordance strip, but the briefing must not treat it as the
/// meeting's primary account.
#[test]
fn test_dos224_title_only_never_primary_on_calendar_sync() {
    let db = test_db();
    let acct = sample_account("acme", "Acme Corp");
    db.upsert_account(&acct).expect("upsert");
    setup_meeting(&db, "m_sync_1", "Acme Ops Planning");

    let entities = vec![rme("acme", 0.50, "title")];
    let linked = crate::services::meetings::persist_classification_entities_scored(
        &db,
        "m_sync_1",
        &entities,
    )
    .expect("persist");
    assert_eq!(linked, 1, "title-only resolution still persists as suggestion");

    let rows = db.get_meeting_linked_entities("m_sync_1").expect("linked");
    assert_eq!(rows.len(), 1);
    assert!(
        !rows[0].is_primary,
        "DOS-224: title-only (<0.70) must NEVER be is_primary=1 on calendar-sync"
    );
    assert!(rows[0].confidence < 0.70);
}

/// DOS-224 regression: when multiple domain-matched account entities land
/// (e.g., parent BU + subsidiary sharing a domain), the calendar-sync
/// persistence path must still pick exactly one primary and persist the
/// others as suggestions. Mirrors the single-primary rule from the
/// resolver path.
#[test]
fn test_dos224_multi_bu_single_primary_on_calendar_sync() {
    let db = test_db();
    let parent = sample_account("parent-bu", "Parent BU");
    db.upsert_account(&parent).expect("upsert parent");
    let sub = sample_account("sub-bu", "Subsidiary BU");
    db.upsert_account(&sub).expect("upsert sub");
    setup_meeting(&db, "m_sync_2", "Shared Domain Sync");

    let entities = vec![
        rme("parent-bu", 0.85, "domain"),
        rme("sub-bu", 0.80, "domain"),
    ];
    let linked = crate::services::meetings::persist_classification_entities_scored(
        &db,
        "m_sync_2",
        &entities,
    )
    .expect("persist");
    assert_eq!(linked, 2);

    let rows = db.get_meeting_linked_entities("m_sync_2").expect("linked");
    assert_eq!(rows.len(), 2);
    let primaries: Vec<&_> = rows.iter().filter(|r| r.is_primary).collect();
    assert_eq!(
        primaries.len(),
        1,
        "DOS-224: calendar-sync must pick exactly one primary per entity_type"
    );
    assert_eq!(primaries[0].id, "parent-bu", "highest-confidence wins");
}

/// DOS-240 regression: a dismissed entity must NOT be re-linked by the
/// calendar-sync persistence path, even if the resolver still thinks the
/// match is valid. The legacy behavior (auto-relink on every sync) is the
/// bug we're fixing.
#[test]
fn test_dos240_dismissal_blocks_calendar_sync_relink() {
    let db = test_db();
    let acct = sample_account("acme", "Acme Corp");
    db.upsert_account(&acct).expect("upsert");
    setup_meeting(&db, "m_dismiss_1", "Acme Planning");

    // User previously dismissed this account from this meeting.
    db.record_meeting_entity_dismissal("m_dismiss_1", "acme", "account", None)
        .expect("record dismissal");

    let entities = vec![rme("acme", 0.85, "domain")];
    let linked = crate::services::meetings::persist_classification_entities_scored(
        &db,
        "m_dismiss_1",
        &entities,
    )
    .expect("persist");
    assert_eq!(linked, 0, "DOS-240: dismissed entity must be skipped");

    let rows = db.get_meeting_linked_entities("m_dismiss_1").expect("linked");
    assert!(rows.is_empty(), "no junction row should have been written");
}

/// DOS-240 regression: after `restore_meeting_entity` removes the dismissal
/// record, the next persistence pass re-links the entity normally.
#[test]
fn test_dos240_restore_allows_relink() {
    let db = test_db();
    let acct = sample_account("acme", "Acme Corp");
    db.upsert_account(&acct).expect("upsert");
    setup_meeting(&db, "m_dismiss_2", "Acme Planning");

    db.record_meeting_entity_dismissal("m_dismiss_2", "acme", "account", None)
        .expect("dismiss");
    let entities = vec![rme("acme", 0.85, "domain")];
    let linked = crate::services::meetings::persist_classification_entities_scored(
        &db,
        "m_dismiss_2",
        &entities,
    )
    .expect("persist");
    assert_eq!(linked, 0);

    // Restore: undo the dismissal.
    let removed = db
        .remove_meeting_entity_dismissal("m_dismiss_2", "acme", "account")
        .expect("remove");
    assert!(removed);

    // Next sync re-matches and this time the link lands.
    let linked = crate::services::meetings::persist_classification_entities_scored(
        &db,
        "m_dismiss_2",
        &entities,
    )
    .expect("persist after restore");
    assert_eq!(linked, 1, "DOS-240: after restore the entity re-links");
    let rows = db
        .get_meeting_linked_entities("m_dismiss_2")
        .expect("linked");
    assert_eq!(rows.len(), 1);
    assert!(rows[0].is_primary);
}

// ---------------------------------------------------------------------------
// DOS-232 Codex fix: account-specific recentMeetings must include non-primary
// high-confidence account links. DOS-224 persists exactly one primary per
// meeting even when multiple accounts share that meeting, so gating on
// `is_primary = 1` hid legitimate secondary accounts from their own dossier.
// ---------------------------------------------------------------------------

/// A meeting linked to account A at is_primary=1 confidence=0.95 AND account
/// B at is_primary=0 confidence=0.80 must appear in BOTH accounts'
/// `get_meetings_for_account_with_prep` results.
#[test]
fn test_dos232_recent_meetings_includes_non_primary_high_confidence() {
    let db = test_db();
    let a = sample_account("acct-a", "Account A");
    let b = sample_account("acct-b", "Account B");
    db.upsert_account(&a).expect("upsert a");
    db.upsert_account(&b).expect("upsert b");
    setup_meeting(&db, "m_shared", "Shared Meeting");

    db.link_meeting_entity_with_confidence("m_shared", "acct-a", "account", 0.95, true)
        .expect("link primary");
    db.link_meeting_entity_with_confidence("m_shared", "acct-b", "account", 0.80, false)
        .expect("link secondary");

    let rows_a = db
        .get_meetings_for_account_with_prep("acct-a", 10)
        .expect("a record");
    assert_eq!(rows_a.len(), 1, "primary account sees the meeting");
    assert_eq!(rows_a[0].id, "m_shared");

    let rows_b = db
        .get_meetings_for_account_with_prep("acct-b", 10)
        .expect("b record");
    assert_eq!(
        rows_b.len(),
        1,
        "DOS-232: secondary account above confidence floor must also see the meeting"
    );
    assert_eq!(rows_b[0].id, "m_shared");
}

/// The 0.70 confidence floor still applies — a speculative domain-match at
/// confidence 0.50 must not surface on the account's Record, is_primary or
/// not.
#[test]
fn test_dos232_recent_meetings_still_filters_below_confidence_floor() {
    let db = test_db();
    let a = sample_account("acct-lowconf", "Low Confidence Account");
    db.upsert_account(&a).expect("upsert");
    setup_meeting(&db, "m_weak", "Weak Link");

    db.link_meeting_entity_with_confidence("m_weak", "acct-lowconf", "account", 0.50, false)
        .expect("link weak");

    let rows = db
        .get_meetings_for_account_with_prep("acct-lowconf", 10)
        .expect("query");
    assert!(
        rows.is_empty(),
        "sub-0.70 junctions must remain excluded from The Record"
    );
}

// ---------------------------------------------------------------------------
// DOS-233 Codex fix: unbounded count queries for About-this-dossier totals.
// ---------------------------------------------------------------------------

/// `get_meeting_count_for_account` must return COUNT(*) without applying the
/// preview-list LIMIT of 10. `get_transcript_count_for_account` must return
/// the distinct number of meetings that have a transcript on record.
#[test]
fn test_dos233_account_meeting_and_transcript_counts_unbounded() {
    let db = test_db();
    let a = sample_account("acct-big", "Big Account");
    db.upsert_account(&a).expect("upsert");

    // 12 meetings linked at confidence 0.95 — well above the LIMIT 10 of the
    // preview query — half carry transcripts, half don't.
    for i in 0..12 {
        let mid = format!("m_big_{i}");
        setup_meeting(&db, &mid, &format!("Meeting {i}"));
        db.link_meeting_entity_with_confidence(&mid, "acct-big", "account", 0.95, i == 0)
            .expect("link");
        if i % 2 == 0 {
            db.conn_ref()
                .execute(
                    "UPDATE meeting_transcripts SET transcript_path = ?1 WHERE meeting_id = ?2",
                    rusqlite::params![format!("/tmp/transcript_{i}.txt"), mid],
                )
                .expect("mark transcript");
        }
    }

    // And one low-confidence junction that must NOT count.
    setup_meeting(&db, "m_weak", "Speculative Match");
    db.link_meeting_entity_with_confidence("m_weak", "acct-big", "account", 0.50, false)
        .expect("link weak");

    let meeting_count = db
        .get_total_meeting_count_for_account("acct-big")
        .expect("meeting count");
    assert_eq!(
        meeting_count, 12,
        "DOS-233: meeting total must be 12 (unbounded) — LIMIT 10 was not applied, 0.50 link excluded"
    );

    let transcript_count = db
        .get_total_transcript_count_for_account("acct-big")
        .expect("transcript count");
    assert_eq!(
        transcript_count, 6,
        "DOS-233: transcript total must be 6 — every other meeting carries a transcript"
    );
}

// ---------------------------------------------------------------------------
// DOS-231 Codex fix: `update_technical_footprint_field` persists a single
// whitelisted column on `account_technical_footprint`. Creates the row if it
// does not yet exist; stamps `source = 'user_edit'`.
// ---------------------------------------------------------------------------

#[test]
fn test_dos231_update_technical_footprint_field_creates_row_and_writes_text() {
    let db = test_db();
    let a = sample_account("acct-tf", "TF Account");
    db.upsert_account(&a).expect("upsert");

    db.update_technical_footprint_field("acct-tf", "usage_tier", "enterprise")
        .expect("write usage tier");

    let tf = db
        .get_account_technical_footprint("acct-tf")
        .expect("query")
        .expect("row exists");
    assert_eq!(tf.usage_tier.as_deref(), Some("enterprise"));
    assert_eq!(tf.source, "user_edit");
}

#[test]
fn test_dos231_update_technical_footprint_field_parses_numerics() {
    let db = test_db();
    let a = sample_account("acct-tf2", "TF Account 2");
    db.upsert_account(&a).expect("upsert");

    db.update_technical_footprint_field("acct-tf2", "active_users", "1250")
        .expect("int");
    db.update_technical_footprint_field("acct-tf2", "csat_score", "4.3")
        .expect("real");
    db.update_technical_footprint_field("acct-tf2", "adoption_score", "0.82")
        .expect("real");

    let tf = db
        .get_account_technical_footprint("acct-tf2")
        .expect("q")
        .expect("row");
    assert_eq!(tf.active_users, Some(1250));
    assert!((tf.csat_score.unwrap() - 4.3).abs() < 1e-6);
    assert!((tf.adoption_score.unwrap() - 0.82).abs() < 1e-6);
}

#[test]
fn test_dos231_update_technical_footprint_field_rejects_unknown_field() {
    let db = test_db();
    let a = sample_account("acct-tf3", "TF Account 3");
    db.upsert_account(&a).expect("upsert");

    let err = db
        .update_technical_footprint_field("acct-tf3", "integrations_json", "garbage")
        .unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("unsupported field"),
        "unsupported fields must be rejected, got: {msg}"
    );
}

#[test]
fn test_dos231_update_technical_footprint_field_rejects_bad_numeric() {
    let db = test_db();
    let a = sample_account("acct-tf4", "TF Account 4");
    db.upsert_account(&a).expect("upsert");

    let err = db
        .update_technical_footprint_field("acct-tf4", "active_users", "not-a-number")
        .unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("not a valid integer"),
        "bad integer must be rejected, got: {msg}"
    );
}
