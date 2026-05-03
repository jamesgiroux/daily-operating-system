//! Tombstone-resurrection reconcile fixtures.
//!
//! The 3 named reconcile fixtures:
//!   - tombstoned-correctly-hidden     → 0 findings (clean)
//!   - tombstoned-with-new-evidence    → 0 findings (newer evidence wins)
//!   - tombstoned-resurrected          → 2 findings (1 dedup_key match + 1 item_hash fallback match)
//!
//! Each test seeds the fixture into an in-memory SQLite DB (test-only
//! scaffolding schema in `dos311_fixtures/schema.sql`) and runs the
//! reconcile SQL from `scripts/reconcile_ghost_resurrection.sql`. The
//! production `intelligence_claims` table + `legacy_projection_state`
//! view ship with (W3); these fixtures use the W1-time scaffolding
//! that mirrors the reconcile fixture column shapes.
//!
//! When  lands the production schema, this test should re-run
//! against that schema (the fixture SQL might need column-name updates;
//! the reconcile SQL is shared).

use rusqlite::Connection;
use std::path::PathBuf;

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn fresh_db() -> Connection {
    let conn = Connection::open_in_memory().expect("open in-memory db");
    let schema = std::fs::read_to_string(
        project_root().join("src-tauri/tests/dos311_fixtures/schema.sql"),
    )
    .expect("read schema.sql");
    conn.execute_batch(&schema).expect("apply schema");
    conn
}

fn load_fixture(conn: &Connection, name: &str) {
    let path = project_root()
        .join("src-tauri/tests/dos311_fixtures")
        .join(format!("{name}.sql"));
    let sql = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read fixture {}: {e}", path.display()));
    conn.execute_batch(&sql)
        .unwrap_or_else(|e| panic!("apply fixture {name}: {e}"));
}

fn run_reconcile(conn: &Connection) -> usize {
    let sql = std::fs::read_to_string(
        project_root().join("scripts/reconcile_ghost_resurrection.sql"),
    )
    .expect("read reconcile_ghost_resurrection.sql");
    let mut stmt = conn.prepare(&sql).expect("prepare reconcile SQL");
    let count = stmt
        .query_map([], |_row| Ok(()))
        .expect("execute reconcile")
        .count();
    count
}

#[test]
fn dos311_fixture_a_tombstoned_correctly_hidden_zero_findings() {
    let conn = fresh_db();
    load_fixture(&conn, "tombstoned_correctly_hidden");
    let findings = run_reconcile(&conn);
    assert_eq!(
        findings, 0,
        "tombstoned-correctly-hidden must produce 0 reconcile findings"
    );
}

#[test]
fn dos311_fixture_b_tombstoned_with_new_evidence_zero_findings() {
    let conn = fresh_db();
    load_fixture(&conn, "tombstoned_with_new_evidence");
    let findings = run_reconcile(&conn);
    assert_eq!(
        findings, 0,
        "tombstoned-with-new-evidence must produce 0 reconcile findings \
         (newer sourced_at overrides tombstone.dismissed_at)"
    );
}

#[test]
fn dos311_fixture_c_tombstoned_resurrected_two_findings() {
    let conn = fresh_db();
    load_fixture(&conn, "tombstoned_resurrected");
    let findings = run_reconcile(&conn);
    // Two findings:
    //   1. claim-3 / dedup-stale-renewal — matched via dedup_key
    //   2. claim-4 / hash-shared-content — matched via item_hash fallback
    //      (the dedup_key shifted; only item_hash still aligns)
    assert_eq!(
        findings, 2,
        "tombstoned-resurrected must produce 2 reconcile findings (1 dedup_key + 1 item_hash fallback)"
    );
}

#[test]
fn dos311_reconcile_match_path_distinguishes_dedup_vs_hash() {
    // Verify the `match_path` column produced by reconcile correctly
    // distinguishes which match path fired. Operators rely on this for
    // diagnosing data drift.
    let conn = fresh_db();
    load_fixture(&conn, "tombstoned_resurrected");
    let sql = std::fs::read_to_string(
        project_root().join("scripts/reconcile_ghost_resurrection.sql"),
    )
    .expect("read reconcile SQL");
    let mut stmt = conn.prepare(&sql).expect("prepare");
    let mut paths: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>("match_path"))
        .expect("query")
        .collect::<Result<Vec<_>, _>>()
        .expect("collect");
    paths.sort();
    assert_eq!(
        paths,
        vec!["dedup_key".to_string(), "item_hash".to_string()],
        "match_path column must distinguish dedup_key vs item_hash matches"
    );
}
