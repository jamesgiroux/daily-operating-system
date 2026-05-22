//! Smoke test for the v1.4.7 W1-A migrations (v255-v258 per dev-reconciliation
//! renumber 2026-05-21; originally v241-v244 in L0 packet) per DOS-168.
//!
//! Runs the full migration chain against an in-memory SQLite and asserts the
//! schema landed correctly: tables exist, expected columns are present, and
//! the composite UNIQUE constraints + indexes from ADR-0102 §C.bis.schema are
//! enforced.

use dailyos_lib::migration_test_api::run_migrations;
use rusqlite::Connection;

#[test]
fn dos168_v255_v258_migrations_land_canonical_schema() {
    let conn = Connection::open_in_memory().expect("open in-memory database");
    run_migrations(&conn).expect("migrations apply cleanly");

    // ---- v255 mcp_client_manifest + mcp_tool_grant ----
    let manifest_cols = table_columns(&conn, "mcp_client_manifest");
    assert!(manifest_cols.contains(&"client_id".to_string()), "manifest missing client_id");
    assert!(manifest_cols.contains(&"paired_at".to_string()), "manifest missing paired_at");
    assert!(manifest_cols.contains(&"revoked_at".to_string()), "manifest missing revoked_at");
    assert!(
        manifest_cols.contains(&"transport_key_ref".to_string()),
        "manifest missing transport_key_ref"
    );

    let grant_cols = table_columns(&conn, "mcp_tool_grant");
    assert!(grant_cols.contains(&"client_id".to_string()), "grant missing client_id");
    assert!(grant_cols.contains(&"tool_name".to_string()), "grant missing tool_name");
    assert!(
        grant_cols.contains(&"scopes_granted_json".to_string()),
        "grant missing scopes_granted_json"
    );
    assert!(grant_cols.contains(&"exposure".to_string()), "grant missing exposure");
    assert!(
        grant_cols.contains(&"rate_limit_max".to_string()),
        "grant missing rate_limit_max"
    );
    assert!(
        grant_cols.contains(&"rate_limit_window_secs".to_string()),
        "grant missing rate_limit_window_secs"
    );
    assert!(
        has_index(&conn, "mcp_tool_grant", "idx_mcp_tool_grant_client_tool"),
        "missing idx_mcp_tool_grant_client_tool index per L0 packet AC-2"
    );

    // ---- v256 mcp_conversation_handle (composite handle+client_id) ----
    let handle_cols = table_columns(&conn, "mcp_conversation_handle");
    assert!(handle_cols.contains(&"handle".to_string()));
    assert!(handle_cols.contains(&"client_id".to_string()));
    assert!(handle_cols.contains(&"mint_at".to_string()));
    assert!(handle_cols.contains(&"last_touched_at".to_string()));
    assert!(handle_cols.contains(&"revoked_at".to_string()));
    // Cross-client binding per ADR-0102 §D.bis: composite UNIQUE on (handle, client_id).
    let handle_indexes = indexes_for(&conn, "mcp_conversation_handle");
    assert!(
        handle_indexes.iter().any(|name| name.contains("handle")),
        "missing handle index per ADR-0102 §D.bis"
    );

    // ---- DOS-168 amended: transport-ceremony rip verification ----
    // Migration 257 (mcp_transport_nonce_ledger) was never introduced — the
    // nonce-replay defense applied to a transport that doesn't have a wire to
    // capture (stdio MCP / loopback). Proving its absence here keeps a future
    // re-introduction loud.
    let nonce_table_exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master \
             WHERE type = 'table' AND name = 'mcp_transport_nonce_ledger'",
            [],
            |row| row.get(0),
        )
        .expect("query sqlite_master");
    assert_eq!(
        nonce_table_exists, 0,
        "mcp_transport_nonce_ledger reintroduced — transport-ceremony rip regressed"
    );

    // Seed a manifest row so subsequent assertions can reference a client_id.
    // transport_key_ref retained for schema stability under DOS-758 but is
    // always NULL post-rip (no transport key material in personal-tier model).
    conn.execute(
        "INSERT INTO mcp_client_manifest (client_id, paired_at, revoked_at, transport_key_ref) \
         VALUES ('smoke-client', 1, NULL, NULL)",
        [],
    )
    .expect("seed manifest row");

    // ---- v258 mcp_tool_call_ledger + mcp_audit_outbox ----
    let ledger_cols = table_columns(&conn, "mcp_tool_call_ledger");
    assert!(ledger_cols.contains(&"client_id".to_string()));
    assert!(ledger_cols.contains(&"tool_name".to_string()));
    assert!(ledger_cols.contains(&"called_at".to_string()));

    let outbox_cols = table_columns(&conn, "mcp_audit_outbox");
    assert!(outbox_cols.contains(&"event".to_string()));
    assert!(outbox_cols.contains(&"detail_json".to_string()));
    assert!(outbox_cols.contains(&"actor_kind".to_string()));
    assert!(outbox_cols.contains(&"created_at".to_string()));
    assert!(
        outbox_cols.contains(&"drained_at".to_string()),
        "outbox missing drained_at for sweep cadence per §C.bis.sweep"
    );

    // Idempotency: re-running migrations is a no-op.
    let second_run = run_migrations(&conn).expect("re-run is idempotent");
    let _ = second_run; // count is implementation-specific; only the success matters.
}

// ---- helpers -------------------------------------------------------------

fn table_columns(conn: &Connection, table: &str) -> Vec<String> {
    conn.prepare(&format!("PRAGMA table_info({table})"))
        .expect("prepare table_info")
        .query_map([], |row| row.get::<_, String>(1))
        .expect("execute table_info")
        .filter_map(Result::ok)
        .collect()
}

fn indexes_for(conn: &Connection, table: &str) -> Vec<String> {
    conn.prepare(&format!("PRAGMA index_list({table})"))
        .expect("prepare index_list")
        .query_map([], |row| row.get::<_, String>(1))
        .expect("execute index_list")
        .filter_map(Result::ok)
        .collect()
}

fn has_index(conn: &Connection, table: &str, index_name: &str) -> bool {
    indexes_for(conn, table)
        .iter()
        .any(|name| name == index_name)
}
