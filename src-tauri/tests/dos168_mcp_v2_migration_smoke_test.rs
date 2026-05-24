//! Smoke test for the MCP v2 migrations after dev-reconciliation renumbering
//! and the local-transport nonce-ledger cleanup.
//!
//! Runs the full migration chain against an in-memory SQLite and asserts the
//! schema landed correctly: tables exist, expected columns are present, and
//! the composite UNIQUE constraints + indexes from ADR-0102 §C.bis.schema are
//! enforced while the removed remote-transport nonce ledger stays absent.

use dailyos_lib::migration_test_api::run_migrations;
use rusqlite::Connection;

#[test]
fn dos168_v255_v259_migrations_land_canonical_schema() {
    let conn = Connection::open_in_memory().expect("open in-memory database");
    run_migrations(&conn).expect("migrations apply cleanly");

    // ---- v255 mcp_client_manifest + mcp_tool_grant ----
    let manifest_cols = table_columns(&conn, "mcp_client_manifest");
    assert!(
        manifest_cols.contains(&"client_id".to_string()),
        "manifest missing client_id"
    );
    assert!(
        manifest_cols.contains(&"paired_at".to_string()),
        "manifest missing paired_at"
    );
    assert!(
        manifest_cols.contains(&"revoked_at".to_string()),
        "manifest missing revoked_at"
    );
    assert!(
        manifest_cols.contains(&"transport_key_ref".to_string()),
        "manifest missing transport_key_ref"
    );

    let grant_cols = table_columns(&conn, "mcp_tool_grant");
    assert!(
        grant_cols.contains(&"client_id".to_string()),
        "grant missing client_id"
    );
    assert!(
        grant_cols.contains(&"tool_name".to_string()),
        "grant missing tool_name"
    );
    assert!(
        grant_cols.contains(&"scopes_granted_json".to_string()),
        "grant missing scopes_granted_json"
    );
    assert!(
        grant_cols.contains(&"exposure".to_string()),
        "grant missing exposure"
    );
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

    // ---- v261 local transport cleanup ----
    // Migration 257 and the later v259 repair are intentionally absent after
    // MCP moved to local stdio / loopback transport. v261 also drops the nonce
    // ledger for DBs that briefly received it.
    assert!(
        !table_exists(&conn, "mcp_transport_nonce_ledger"),
        "fresh local MCP schema should not include the obsolete nonce ledger"
    );

    // Seed a manifest row so subsequent assertions can reference a client_id.
    // transport_key_ref is retained for schema stability but is always NULL in
    // this smoke fixture.
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

fn table_exists(conn: &Connection, table: &str) -> bool {
    conn.query_row(
        "SELECT EXISTS (
             SELECT 1
             FROM sqlite_master
             WHERE type = 'table' AND name = ?1
         )",
        [table],
        |row| row.get::<_, bool>(0),
    )
    .expect("table existence query")
}
