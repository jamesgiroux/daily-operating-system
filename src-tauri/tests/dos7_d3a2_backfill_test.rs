//! Claims backfill D3a-2: backfill mechanisms 5-8 into intelligence_claims.
//!
//! Pattern per test: open in-memory DB, apply minimal legacy CREATE
//! TABLEs needed for each mechanism + D1 claims schema, INSERT mock
//! legacy rows, run the D3a-1 and D3a-2 backfill SQL, then assert.

use rusqlite::Connection;

const CLAIMS_SCHEMA_SQL: &str = include_str!("../src/migrations/129_dos_7_claims_schema.sql");
const D3A1_BACKFILL_SQL: &str = include_str!("../src/migrations/130_dos_7_claims_backfill_a1.sql");
const D3A2_BACKFILL_SQL: &str = include_str!("../src/migrations/131_dos_7_claims_backfill_a2.sql");

/// Inline minimal legacy schemas (just the columns the backfills read).
/// We deliberately do NOT run the full migration chain to keep tests
/// hermetic; backfill correctness is the concern, not legacy schema
/// integrity.
const LEGACY_SCHEMA_SQL: &str = "
CREATE TABLE accounts (id TEXT PRIMARY KEY);
CREATE TABLE people (id TEXT PRIMARY KEY, display_name TEXT);

CREATE TABLE suppression_tombstones (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    entity_id TEXT NOT NULL,
    field_key TEXT NOT NULL,
    item_key TEXT,
    item_hash TEXT,
    dismissed_at TEXT NOT NULL DEFAULT (datetime('now')),
    source_scope TEXT,
    expires_at TEXT,
    superseded_by_evidence_after TEXT
);

CREATE TABLE account_stakeholder_roles (
    account_id TEXT NOT NULL REFERENCES accounts(id),
    person_id TEXT NOT NULL REFERENCES people(id),
    role TEXT NOT NULL,
    data_source TEXT NOT NULL DEFAULT 'ai',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    dismissed_at TEXT,
    PRIMARY KEY (account_id, person_id, role)
);

CREATE TABLE email_dismissals (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    item_type TEXT NOT NULL,
    email_id TEXT NOT NULL,
    sender_domain TEXT,
    email_type TEXT,
    entity_id TEXT,
    item_text TEXT NOT NULL,
    dismissed_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE meeting_entity_dismissals (
    meeting_id TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    dismissed_at TEXT NOT NULL,
    dismissed_by TEXT,
    PRIMARY KEY (meeting_id, entity_id, entity_type)
);

CREATE TABLE linking_dismissals (
    owner_type TEXT NOT NULL,
    owner_id TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    dismissed_by TEXT,
    created_at TEXT NOT NULL,
    PRIMARY KEY (owner_type, owner_id, entity_id, entity_type)
);

CREATE TABLE briefing_callouts (
    id TEXT PRIMARY KEY,
    signal_id TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    entity_name TEXT,
    severity TEXT NOT NULL DEFAULT 'info',
    headline TEXT NOT NULL,
    detail TEXT,
    context_json TEXT,
    surfaced_at TEXT,
    dismissed_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE nudge_dismissals (
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    nudge_key TEXT NOT NULL,
    dismissed_at TEXT NOT NULL,
    PRIMARY KEY (entity_type, entity_id, nudge_key)
);

CREATE TABLE triage_snoozes (
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    triage_key TEXT NOT NULL,
    snoozed_until TEXT,
    resolved_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (entity_type, entity_id, triage_key)
);
";

fn fresh_db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(LEGACY_SCHEMA_SQL).unwrap();
    conn.execute_batch(CLAIMS_SCHEMA_SQL).unwrap();
    conn
}

fn run_backfills(conn: &Connection) {
    conn.execute_batch(D3A1_BACKFILL_SQL).unwrap();
    conn.execute_batch(D3A2_BACKFILL_SQL).unwrap();
}

// ---------------------------------------------------------------------------
// Mechanism 5 — linking_dismissals
// ---------------------------------------------------------------------------

#[test]
fn mechanism_5_linking_dismissals_backfills_with_owner_subject() {
    let conn = fresh_db();

    conn.execute(
        "INSERT INTO linking_dismissals \
         (owner_type, owner_id, entity_id, entity_type, dismissed_by, created_at) \
         VALUES ('email', 'email-1', 'acct-1', 'account', NULL, '2026-04-20T00:00:00Z')",
        [],
    )
    .unwrap();

    run_backfills(&conn);

    let (subject_ref, claim_type, field_path, text, dedup_key): (
        String,
        String,
        String,
        String,
        String,
    ) = conn
        .query_row(
            "SELECT subject_ref, claim_type, field_path, text, dedup_key \
             FROM intelligence_claims WHERE id LIKE 'm5-%' LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
        )
        .unwrap();
    assert!(subject_ref.contains("\"kind\":\"Email\""));
    assert!(subject_ref.contains("\"id\":\"email-1\""));
    assert_eq!(claim_type, "linking_dismissed");
    assert_eq!(field_path, "account");
    assert_eq!(text, "acct-1");
    assert_eq!(dedup_key, "acct-1:email:email-1:linking_dismissed:account");
}

#[test]
fn mechanism_5_newer_linking_wins_over_older_meeting_entity() {
    let conn = fresh_db();

    conn.execute(
        "INSERT INTO meeting_entity_dismissals \
         (meeting_id, entity_id, entity_type, dismissed_at, dismissed_by) \
         VALUES ('meeting-x', 'acct-1', 'account', '2026-04-20T00:00:00Z', NULL)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO linking_dismissals \
         (owner_type, owner_id, entity_id, entity_type, dismissed_by, created_at) \
         VALUES ('meeting', 'meeting-x', 'acct-1', 'account', NULL, '2026-04-21T00:00:00Z')",
        [],
    )
    .unwrap();

    run_backfills(&conn);

    let m5_count: i64 = conn
        .query_row(
            "SELECT count(*) FROM intelligence_claims \
             WHERE id = 'm5-meeting:meeting-x:acct-1:account'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(m5_count, 1, "newer linking dismissal should be backfilled");

    let (claim_id, source_mechanism): (String, String) = conn
        .query_row(
            "SELECT claim_id, source_mechanism FROM claim_corroborations \
             WHERE id = 'm4-m5-corr-meeting-x:acct-1:account'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(claim_id, "m5-meeting:meeting-x:acct-1:account");
    assert_eq!(source_mechanism, "meeting_entity_dismissals_dup");
}

#[test]
fn mechanism_5_older_linking_attaches_as_corroboration_to_meeting_entity_winner() {
    let conn = fresh_db();

    conn.execute(
        "INSERT INTO meeting_entity_dismissals \
         (meeting_id, entity_id, entity_type, dismissed_at, dismissed_by) \
         VALUES ('meeting-x', 'acct-1', 'account', '2026-04-21T00:00:00Z', NULL)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO linking_dismissals \
         (owner_type, owner_id, entity_id, entity_type, dismissed_by, created_at) \
         VALUES ('meeting', 'meeting-x', 'acct-1', 'account', NULL, '2026-04-20T00:00:00Z')",
        [],
    )
    .unwrap();

    run_backfills(&conn);

    let m5_count: i64 = conn
        .query_row(
            "SELECT count(*) FROM intelligence_claims WHERE id LIKE 'm5-%'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(m5_count, 0, "older linking duplicate should not win");

    let (claim_id, source_mechanism): (String, String) = conn
        .query_row(
            "SELECT claim_id, source_mechanism FROM claim_corroborations \
             WHERE id = 'm5-m4-corr-meeting-x:acct-1:account'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(claim_id, "m4-meeting-x:acct-1:account");
    assert_eq!(source_mechanism, "linking_dismissals_dup");
}

// ---------------------------------------------------------------------------
// Mechanism 6 — briefing_callouts.dismissed_at
// ---------------------------------------------------------------------------

#[test]
fn mechanism_6_briefing_callouts_backfills_only_dismissed_rows() {
    let conn = fresh_db();

    conn.execute_batch(
        "INSERT INTO briefing_callouts \
         (id, signal_id, entity_type, entity_id, entity_name, severity, headline, detail, dismissed_at, created_at) \
         VALUES ('email-1', 'nudge-key', 'account', 'acct-1', 'Test Account', 'info', \
                 'Dismissed headline', 'Dismissed detail', '2026-04-22T00:00:00Z', '2026-04-21T00:00:00Z');
         INSERT INTO briefing_callouts \
         (id, signal_id, entity_type, entity_id, entity_name, severity, headline, detail, dismissed_at, created_at) \
         VALUES ('meeting-x', 'triage-key', 'account', 'acct-1', 'Test Account', 'info', \
                 'Visible headline', 'Visible detail', NULL, '2026-04-21T00:00:00Z');",
    )
    .unwrap();

    run_backfills(&conn);

    let count: i64 = conn
        .query_row(
            "SELECT count(*) FROM intelligence_claims WHERE id LIKE 'm6-%'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 1, "only dismissed callouts should backfill");

    let (subject_ref, claim_type, text, dedup_key): (String, String, String, String) = conn
        .query_row(
            "SELECT subject_ref, claim_type, text, dedup_key \
             FROM intelligence_claims WHERE id = 'm6-email-1'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .unwrap();
    assert!(subject_ref.contains("\"kind\":\"Account\""));
    assert!(subject_ref.contains("\"id\":\"acct-1\""));
    assert_eq!(claim_type, "briefing_callout_dismissed");
    assert_eq!(text, "Dismissed headline");
    assert_eq!(dedup_key, "email-1:acct-1:briefing_callout_dismissed");
}

// ---------------------------------------------------------------------------
// Mechanism 7 — nudge_dismissals
// ---------------------------------------------------------------------------

#[test]
fn mechanism_7_nudge_dismissals_backfills_with_entity_subject() {
    let conn = fresh_db();

    conn.execute(
        "INSERT INTO nudge_dismissals (entity_type, entity_id, nudge_key, dismissed_at) \
         VALUES ('account', 'acct-1', 'nudge-key', '2026-04-23T00:00:00Z')",
        [],
    )
    .unwrap();

    run_backfills(&conn);

    let (subject_ref, claim_type, text, dedup_key): (String, String, String, String) = conn
        .query_row(
            "SELECT subject_ref, claim_type, text, dedup_key \
             FROM intelligence_claims WHERE id LIKE 'm7-%' LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .unwrap();
    assert!(subject_ref.contains("\"kind\":\"Account\""));
    assert!(subject_ref.contains("\"id\":\"acct-1\""));
    assert_eq!(claim_type, "nudge_dismissed");
    assert_eq!(text, "nudge-key");
    assert_eq!(dedup_key, "nudge-key:acct-1:nudge_dismissed");
}

// ---------------------------------------------------------------------------
// Mechanism 8 — triage_snoozes
// ---------------------------------------------------------------------------

#[test]
fn mechanism_8_triage_snoozes_backfills_with_expires_at_for_snooze() {
    let conn = fresh_db();

    conn.execute(
        "INSERT INTO triage_snoozes \
         (entity_type, entity_id, triage_key, snoozed_until, resolved_at, created_at, updated_at) \
         VALUES ('account', 'acct-1', 'triage-key', '2099-01-01T00:00:00Z', NULL, \
                 '2026-04-24T00:00:00Z', '2026-04-24T00:00:00Z')",
        [],
    )
    .unwrap();

    run_backfills(&conn);

    let (subject_ref, claim_type, text, expires_at, retraction_reason): (
        String,
        String,
        String,
        String,
        String,
    ) = conn
        .query_row(
            "SELECT subject_ref, claim_type, text, expires_at, retraction_reason \
             FROM intelligence_claims WHERE id LIKE 'm8-%' LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
        )
        .unwrap();
    assert!(subject_ref.contains("\"kind\":\"Account\""));
    assert!(subject_ref.contains("\"id\":\"acct-1\""));
    assert_eq!(claim_type, "triage_snooze");
    assert_eq!(text, "triage-key");
    assert_eq!(expires_at, "2099-01-01T00:00:00Z");
    assert_eq!(retraction_reason, "system_snooze");
}

#[test]
fn mechanism_8_triage_snoozes_backfills_resolved_rows_with_user_resolved_reason() {
    let conn = fresh_db();

    conn.execute(
        "INSERT INTO triage_snoozes \
         (entity_type, entity_id, triage_key, snoozed_until, resolved_at, created_at, updated_at) \
         VALUES ('account', 'acct-1', 'triage-key', NULL, '2026-04-25T00:00:00Z', \
                 '2026-04-24T00:00:00Z', '2026-04-25T00:00:00Z')",
        [],
    )
    .unwrap();

    run_backfills(&conn);

    let (claim_type, text, expires_at, retraction_reason): (
        String,
        String,
        Option<String>,
        String,
    ) = conn
        .query_row(
            "SELECT claim_type, text, expires_at, retraction_reason \
             FROM intelligence_claims WHERE id LIKE 'm8-%' LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .unwrap();
    assert_eq!(claim_type, "triage_snooze");
    assert_eq!(text, "triage-key");
    assert!(expires_at.is_none());
    assert_eq!(retraction_reason, "user_resolved");
}
