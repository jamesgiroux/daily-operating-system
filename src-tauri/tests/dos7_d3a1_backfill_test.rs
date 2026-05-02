//! DOS-7 D3a-1: backfill mechanisms 1-4 into intelligence_claims.
//!
//! Pattern per test: open in-memory DB, apply minimal legacy CREATE
//! TABLEs needed for each mechanism + D1 claims schema, INSERT mock
//! legacy rows, run the D3a-1 backfill SQL, then assert.

use rusqlite::Connection;

const CLAIMS_SCHEMA_SQL: &str = include_str!("../src/migrations/129_dos_7_claims_schema.sql");
const D3A1_BACKFILL_SQL: &str = include_str!("../src/migrations/130_dos_7_claims_backfill_a1.sql");

/// Inline minimal legacy schemas (just the columns the backfill reads).
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
";

fn fresh_db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(LEGACY_SCHEMA_SQL).unwrap();
    conn.execute_batch(CLAIMS_SCHEMA_SQL).unwrap();
    conn
}

fn run_backfill(conn: &Connection) {
    conn.execute_batch(D3A1_BACKFILL_SQL).unwrap();
}

// ---------------------------------------------------------------------------
// Mechanism 1 — suppression_tombstones
// ---------------------------------------------------------------------------

#[test]
fn mechanism_1_suppression_tombstones_backfills_to_tombstone_claim() {
    let conn = fresh_db();

    conn.execute(
        "INSERT INTO suppression_tombstones (entity_id, field_key, item_key, item_hash, dismissed_at) \
         VALUES ('acct-1', 'risks', 'risk-text', 'h-1', '2026-04-01T00:00:00Z')",
        [],
    )
    .unwrap();

    run_backfill(&conn);

    let count: i64 = conn
        .query_row(
            "SELECT count(*) FROM intelligence_claims \
             WHERE claim_state = 'tombstoned' AND retraction_reason = 'user_removal' \
               AND data_source = 'legacy_dismissal' AND subject_ref LIKE '%acct-1%'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 1, "expected one tombstone claim from mechanism 1");

    let (claim_type, dedup_key, field_path, item_hash, expires_at): (
        String,
        String,
        String,
        String,
        Option<String>,
    ) = conn
        .query_row(
            "SELECT claim_type, dedup_key, field_path, item_hash, expires_at \
             FROM intelligence_claims WHERE id LIKE 'm1-%' LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
        )
        .unwrap();
    assert_eq!(claim_type, "risk");
    assert_eq!(field_path, "risks");
    assert_eq!(item_hash, "h-1");
    assert!(dedup_key.contains("h-1"));
    assert!(dedup_key.contains("acct-1"));
    assert!(expires_at.is_none());
}

#[test]
fn mechanism_1_duplicate_keeps_latest_attaches_prior_as_corroboration() {
    let conn = fresh_db();

    conn.execute(
        "INSERT INTO suppression_tombstones (id, entity_id, field_key, item_key, item_hash, dismissed_at) \
         VALUES (10, 'acct-1', 'risks', 'risk-x', 'h-x', '2026-03-01T00:00:00Z')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO suppression_tombstones (id, entity_id, field_key, item_key, item_hash, dismissed_at) \
         VALUES (11, 'acct-1', 'risks', 'risk-x', 'h-x', '2026-04-01T00:00:00Z')",
        [],
    )
    .unwrap();

    run_backfill(&conn);

    let claim_count: i64 = conn
        .query_row(
            "SELECT count(*) FROM intelligence_claims WHERE id LIKE 'm1-%'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(claim_count, 1);

    let claim_id: String = conn
        .query_row(
            "SELECT id FROM intelligence_claims WHERE id LIKE 'm1-%' LIMIT 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(claim_id, "m1-11", "winner should be the newer row");

    let corr_count: i64 = conn
        .query_row(
            "SELECT count(*) FROM claim_corroborations \
             WHERE claim_id = 'm1-11' AND source_mechanism = 'suppression_tombstones_dup'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(corr_count, 1);
}

// ---------------------------------------------------------------------------
// Mechanism 2 — account_stakeholder_roles.dismissed_at
// ---------------------------------------------------------------------------

#[test]
fn mechanism_2_stakeholder_role_dismissed_at_backfills_with_person_subject() {
    let conn = fresh_db();

    conn.execute_batch(
        "INSERT INTO accounts (id) VALUES ('acct-1');
         INSERT INTO people (id, display_name) VALUES ('person-1', 'Test Person');
         INSERT INTO account_stakeholder_roles (account_id, person_id, role, data_source, dismissed_at) \
            VALUES ('acct-1', 'person-1', 'champion', 'user', '2026-04-15T00:00:00Z');",
    )
    .unwrap();

    run_backfill(&conn);

    let (subject_ref, claim_type, text, dedup_key): (String, String, String, String) = conn
        .query_row(
            "SELECT subject_ref, claim_type, text, dedup_key \
             FROM intelligence_claims WHERE id LIKE 'm2-%' LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .unwrap();
    assert!(subject_ref.contains("\"kind\":\"Person\""));
    assert!(subject_ref.contains("\"id\":\"person-1\""));
    assert_eq!(claim_type, "stakeholder_role");
    assert_eq!(text, "champion");
    assert_eq!(dedup_key, "champion:acct-1:person-1:stakeholder_role");
}

// ---------------------------------------------------------------------------
// Mechanism 3 — email_dismissals
// ---------------------------------------------------------------------------

#[test]
fn mechanism_3_email_dismissals_backfills_with_email_subject() {
    let conn = fresh_db();

    conn.execute(
        "INSERT INTO email_dismissals (item_type, email_id, sender_domain, email_type, item_text, dismissed_at) \
         VALUES ('commitment', 'email-1', 'example.com', 'inbound', 'Will follow up next week', '2026-04-10T00:00:00Z')",
        [],
    )
    .unwrap();

    run_backfill(&conn);

    let (subject_ref, claim_type, field_path, text): (String, String, String, String) = conn
        .query_row(
            "SELECT subject_ref, claim_type, field_path, text \
             FROM intelligence_claims WHERE id LIKE 'm3-%' LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .unwrap();
    assert!(subject_ref.contains("\"kind\":\"Email\""));
    assert!(subject_ref.contains("\"id\":\"email-1\""));
    assert_eq!(claim_type, "email_dismissed");
    assert_eq!(field_path, "commitment");
    assert_eq!(text, "Will follow up next week");
}

// ---------------------------------------------------------------------------
// Mechanism 4 — meeting_entity_dismissals
// ---------------------------------------------------------------------------

#[test]
fn mechanism_4_meeting_entity_dismissals_backfills_with_meeting_subject() {
    let conn = fresh_db();

    conn.execute(
        "INSERT INTO meeting_entity_dismissals (meeting_id, entity_id, entity_type, dismissed_at, dismissed_by) \
         VALUES ('meeting-x', 'acct-y', 'account', '2026-04-20T00:00:00Z', 'user-1')",
        [],
    )
    .unwrap();

    run_backfill(&conn);

    let (subject_ref, claim_type, field_path, text, dedup_key): (String, String, String, String, String) = conn
        .query_row(
            "SELECT subject_ref, claim_type, field_path, text, dedup_key \
             FROM intelligence_claims WHERE id LIKE 'm4-%' LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
        )
        .unwrap();
    assert!(subject_ref.contains("\"kind\":\"Meeting\""));
    assert!(subject_ref.contains("\"id\":\"meeting-x\""));
    assert_eq!(claim_type, "meeting_entity_dismissed");
    assert_eq!(field_path, "account");
    assert_eq!(text, "acct-y");
    assert_eq!(dedup_key, "acct-y:meeting-x:meeting_entity_dismissed:account");
}

// ---------------------------------------------------------------------------
// Idempotency
// ---------------------------------------------------------------------------

#[test]
fn backfill_a1_migration_is_idempotent() {
    let conn = fresh_db();

    conn.execute(
        "INSERT INTO suppression_tombstones (entity_id, field_key, item_key, item_hash, dismissed_at) \
         VALUES ('acct-1', 'risks', 'risk-text', 'h-1', '2026-04-01T00:00:00Z')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO email_dismissals (item_type, email_id, item_text, dismissed_at) \
         VALUES ('reply_needed', 'email-z', 'Reply by Monday', '2026-04-12T00:00:00Z')",
        [],
    )
    .unwrap();

    run_backfill(&conn);
    let count_after_first: i64 = conn
        .query_row(
            "SELECT count(*) FROM intelligence_claims WHERE id LIKE 'm%'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    run_backfill(&conn);
    let count_after_second: i64 = conn
        .query_row(
            "SELECT count(*) FROM intelligence_claims WHERE id LIKE 'm%'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(
        count_after_first, count_after_second,
        "backfill must be idempotent"
    );
}
