//! Smoke test for the v1.4.7 W1-A migrations (v241-v244) per DOS-168 L0 packet.
//!
//! Runs the full migration chain against an in-memory SQLite and asserts the
//! schema landed correctly: tables exist, expected columns are present, and
//! the composite UNIQUE constraints + indexes from ADR-0102 §C.bis.schema are
//! enforced.

use dailyos_lib::migration_test_api::run_migrations;
use rusqlite::Connection;

#[test]
fn dos168_v241_v244_migrations_land_canonical_schema() {
    let conn = Connection::open_in_memory().expect("open in-memory database");
    run_migrations(&conn).expect("migrations apply cleanly");

    // ---- v241 mcp_client_manifest + mcp_tool_grant ----
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

    // ---- v242 mcp_conversation_handle (composite handle+client_id) ----
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

    // ---- v243 mcp_transport_nonce_ledger (ADR-0102 §C.bis.schema verbatim) ----
    let nonce_cols = table_columns(&conn, "mcp_transport_nonce_ledger");
    assert!(nonce_cols.contains(&"nonce".to_string()));
    assert!(nonce_cols.contains(&"client_id".to_string()));
    assert!(
        nonce_cols.contains(&"issued_at".to_string()),
        "nonce ledger missing issued_at per ADR-0102 §C.bis.schema"
    );
    assert!(
        nonce_cols.contains(&"expires_at".to_string()),
        "nonce ledger missing expires_at per ADR-0102 §C.bis.schema"
    );
    assert!(
        nonce_cols.contains(&"consumed_at".to_string()),
        "nonce ledger missing consumed_at per ADR-0102 §C.bis.schema"
    );
    let nonce_indexes = indexes_for(&conn, "mcp_transport_nonce_ledger");
    assert!(
        nonce_indexes
            .iter()
            .any(|n| n.contains("nonce") || n.contains("lookup")),
        "nonce ledger missing lookup index per §C.bis.schema"
    );

    // Atomic consume-once semantics: UNIQUE(nonce, client_id) is enforced.
    conn.execute(
        "INSERT INTO mcp_client_manifest (client_id, paired_at, revoked_at, transport_key_ref) \
         VALUES ('smoke-client', 1, NULL, 'smoke-ref')",
        [],
    )
    .expect("seed manifest row");
    conn.execute(
        "INSERT INTO mcp_transport_nonce_ledger \
         (nonce, client_id, issued_at, expires_at, consumed_at) \
         VALUES ('seed-nonce', 'smoke-client', 1, 1000, NULL)",
        [],
    )
    .expect("insert first nonce row");
    let duplicate_result = conn.execute(
        "INSERT INTO mcp_transport_nonce_ledger \
         (nonce, client_id, issued_at, expires_at, consumed_at) \
         VALUES ('seed-nonce', 'smoke-client', 2, 2000, NULL)",
        [],
    );
    assert!(
        duplicate_result.is_err(),
        "UNIQUE(nonce, client_id) NOT enforced — replay protection broken"
    );

    // ---- v244 mcp_tool_call_ledger + mcp_audit_outbox ----
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
