//! DOS-7 D1: schema-only test for the claims commit substrate. Verifies
//! that the D1 migration SQL creates all 6 tables + indexes idempotently.

use rusqlite::Connection;

const CLAIMS_SCHEMA_SQL: &str = include_str!("../src/migrations/129_dos_7_claims_schema.sql");

fn mem_db() -> Connection {
    Connection::open_in_memory().unwrap()
}

fn apply_claims_schema(conn: &Connection) {
    conn.execute_batch(CLAIMS_SCHEMA_SQL).unwrap();
}

#[test]
fn migration_130_creates_intelligence_claims_table() {
    let conn = mem_db();
    apply_claims_schema(&conn);

    let row_count: i64 = conn
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='intelligence_claims'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(row_count, 1);
}

#[test]
fn migration_130_creates_all_six_claim_tables() {
    let conn = mem_db();
    apply_claims_schema(&conn);

    for table in &[
        "intelligence_claims",
        "claim_corroborations",
        "claim_contradictions",
        "agent_trust_ledger",
        "claim_feedback",
        "claim_repair_job",
    ] {
        let exists: bool = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name=?1",
                [table],
                |row| row.get::<_, i64>(0).map(|c| c > 0),
            )
            .unwrap();
        assert!(exists, "table {table} should exist after migration 130");
    }
}

#[test]
fn migration_130_creates_required_indexes() {
    let conn = mem_db();
    apply_claims_schema(&conn);

    for index in &[
        "idx_claims_default_read",
        "idx_claims_suppression_lookup",
        "idx_claims_dedup_key",
        "idx_claims_thread_id",
        "idx_claims_superseded_by",
        "idx_corroborations_claim",
        "idx_corroborations_source",
        "idx_contradictions_primary",
        "idx_contradictions_unreconciled",
        "idx_agent_trust_lookup",
        "idx_feedback_claim",
        "idx_feedback_type",
        "idx_repair_pending",
    ] {
        let exists: bool = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='index' AND name=?1",
                [index],
                |row| row.get::<_, i64>(0).map(|c| c > 0),
            )
            .unwrap();
        assert!(exists, "index {index} should exist after migration 130");
    }
}

#[test]
fn migration_130_is_idempotent() {
    let conn = mem_db();
    apply_claims_schema(&conn);
    apply_claims_schema(&conn);

    for table in &[
        "intelligence_claims",
        "claim_corroborations",
        "claim_contradictions",
        "agent_trust_ledger",
        "claim_feedback",
        "claim_repair_job",
    ] {
        let row_count: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name=?1",
                [table],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(row_count, 1, "table {table} should be created once");
    }
}

#[test]
fn intelligence_claims_check_constraints_enforce_enum_values() {
    let conn = mem_db();
    apply_claims_schema(&conn);

    let result = conn.execute(
        "INSERT INTO intelligence_claims \
         (id, subject_ref, claim_type, text, dedup_key, actor, data_source, observed_at, provenance_json, claim_state) \
         VALUES ('test-id', '{}', 'risk', 'text', 'dedup', 'agent', 'glean', '2026-05-02T00:00:00Z', '{}', 'invalid_state')",
        [],
    );
    assert!(
        result.is_err(),
        "CHECK constraint should reject invalid claim_state"
    );
}

#[test]
fn sibling_check_constraints_enforce_enum_values_and_ranges() {
    let conn = mem_db();
    apply_claims_schema(&conn);

    conn.execute(
        "INSERT INTO intelligence_claims \
         (id, subject_ref, claim_type, text, dedup_key, actor, data_source, observed_at, provenance_json) \
         VALUES ('claim-1', '{}', 'risk', 'text', 'dedup', 'agent', 'glean', '2026-05-02T00:00:00Z', '{}')",
        [],
    )
    .unwrap();

    let bad_strength = conn.execute(
        "INSERT INTO claim_corroborations (id, claim_id, data_source, strength) \
         VALUES ('corr-1', 'claim-1', 'glean', 1.5)",
        [],
    );
    assert!(
        bad_strength.is_err(),
        "CHECK constraint should reject corroboration strength > 1.0"
    );

    let bad_branch = conn.execute(
        "INSERT INTO claim_contradictions \
         (id, primary_claim_id, contradicting_claim_id, branch_kind) \
         VALUES ('contr-1', 'claim-1', 'claim-1', 'invalid')",
        [],
    );
    assert!(
        bad_branch.is_err(),
        "CHECK constraint should reject invalid branch_kind"
    );

    let bad_feedback = conn.execute(
        "INSERT INTO claim_feedback (id, claim_id, feedback_type, actor) \
         VALUES ('fb-1', 'claim-1', 'invalid', 'user')",
        [],
    );
    assert!(
        bad_feedback.is_err(),
        "CHECK constraint should reject invalid feedback_type"
    );

    let bad_repair_state = conn.execute(
        "INSERT INTO claim_repair_job (id, claim_id, state) \
         VALUES ('repair-1', 'claim-1', 'invalid')",
        [],
    );
    assert!(
        bad_repair_state.is_err(),
        "CHECK constraint should reject invalid repair job state"
    );
}

#[test]
fn intelligence_claims_default_state_values_apply() {
    let conn = mem_db();
    apply_claims_schema(&conn);

    conn.execute(
        "INSERT INTO intelligence_claims \
         (id, subject_ref, claim_type, text, dedup_key, actor, data_source, observed_at, provenance_json) \
         VALUES ('test-id', '{}', 'risk', 'text', 'dedup', 'agent', 'glean', '2026-05-02T00:00:00Z', '{}')",
        [],
    )
    .unwrap();

    let (claim_state, surfacing_state, temporal_scope, sensitivity): (
        String,
        String,
        String,
        String,
    ) = conn
        .query_row(
            "SELECT claim_state, surfacing_state, temporal_scope, sensitivity \
             FROM intelligence_claims WHERE id = 'test-id'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .unwrap();

    assert_eq!(claim_state, "active");
    assert_eq!(surfacing_state, "active");
    assert_eq!(temporal_scope, "state");
    assert_eq!(sensitivity, "internal");
}

#[test]
fn sibling_default_values_apply() {
    let conn = mem_db();
    apply_claims_schema(&conn);

    conn.execute(
        "INSERT INTO intelligence_claims \
         (id, subject_ref, claim_type, text, dedup_key, actor, data_source, observed_at, provenance_json) \
         VALUES ('claim-1', '{}', 'risk', 'text', 'dedup', 'agent', 'glean', '2026-05-02T00:00:00Z', '{}')",
        [],
    )
    .unwrap();

    conn.execute(
        "INSERT INTO claim_corroborations (id, claim_id, data_source) \
         VALUES ('corr-1', 'claim-1', 'glean')",
        [],
    )
    .unwrap();
    let (strength, reinforcement_count): (f64, i64) = conn
        .query_row(
            "SELECT strength, reinforcement_count FROM claim_corroborations WHERE id = 'corr-1'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(strength, 0.5);
    assert_eq!(reinforcement_count, 1);

    conn.execute(
        "INSERT INTO claim_repair_job (id, claim_id) VALUES ('repair-1', 'claim-1')",
        [],
    )
    .unwrap();
    let (state, attempts, max_attempts): (String, i64, i64) = conn
        .query_row(
            "SELECT state, attempts, max_attempts FROM claim_repair_job WHERE id = 'repair-1'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(state, "pending");
    assert_eq!(attempts, 0);
    assert_eq!(max_attempts, 3);
}
