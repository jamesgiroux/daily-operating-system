//! Migration round-trip tests for v1.4.5 W1-A + W1-B + W1-C SQL.
//! Pre-stages the L1 acceptance criteria from each lane's L0 packet §8.

use rusqlite::Connection;

fn table_columns(conn: &Connection, table: &str) -> Vec<String> {
    conn.prepare(&format!("PRAGMA table_info({table})"))
        .unwrap_or_else(|error| panic!("prepare columns for {table}: {error}"))
        .query_map([], |row| row.get::<_, String>(1))
        .unwrap_or_else(|error| panic!("query columns for {table}: {error}"))
        .filter_map(Result::ok)
        .collect()
}

fn assert_has_columns(conn: &Connection, table: &str, required: &[&str]) {
    let columns = table_columns(conn, table);
    for column in required {
        assert!(
            columns.iter().any(|found| found == column),
            "{table} missing required column `{column}`; got {columns:?}"
        );
    }
}

fn assert_forbidden_columns_absent(conn: &Connection, table: &str) {
    let columns = table_columns(conn, table);
    for forbidden in &[
        "canonical_path",
        "relative_path",
        "filename",
        "absolute_path",
        "raw_path",
        "claim_text",
        "file_content",
        "prompt",
        "output_body",
    ] {
        assert!(
            !columns.iter().any(|column| column == forbidden),
            "{table} must not store raw surfaced field `{forbidden}`"
        );
    }
}

fn index_names(conn: &Connection, table: &str) -> Vec<String> {
    conn.prepare(&format!("PRAGMA index_list({table})"))
        .unwrap_or_else(|error| panic!("prepare indexes for {table}: {error}"))
        .query_map([], |row| row.get::<_, String>(1))
        .unwrap_or_else(|error| panic!("query indexes for {table}: {error}"))
        .filter_map(Result::ok)
        .collect()
}

#[test]
fn w1_migrations_v250_through_v254_apply_in_order() {
    let conn = Connection::open_in_memory().expect("open in-memory sqlite");
    for sql in [
        include_str!("../src/migrations/250_workspace_file_lifecycle.sql"),
        include_str!("../src/migrations/251_workspace_file_lifecycle_category.sql"),
        include_str!("../src/migrations/252_workspace_source_registry.sql"),
        include_str!("../src/migrations/253_document_ingestion_runs.sql"),
        include_str!("../src/migrations/254_document_entity_links.sql"),
    ] {
        conn.execute_batch(sql)
            .unwrap_or_else(|e| panic!("migration apply: {e}"));
    }

    let table_names: Vec<String> = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name LIKE 'workspace%' OR name LIKE 'document_%' ORDER BY name")
        .expect("prepare")
        .query_map([], |row| row.get::<_, String>(0))
        .expect("query")
        .filter_map(Result::ok)
        .collect();

    for required in &[
        "workspace_file_lifecycle",
        "workspace_source_registry",
        "workspace_category_registry",
        "document_ingestion_runs",
        "document_entity_links",
    ] {
        assert!(
            table_names.iter().any(|t| t == required),
            "missing required table `{required}`; got {table_names:?}"
        );
    }
}

#[test]
fn w5_a_backfill_state_migration_creates_privacy_safe_run_item_operation_tables() {
    let conn = Connection::open_in_memory().expect("open in-memory sqlite");
    conn.execute_batch("PRAGMA foreign_keys = ON")
        .expect("foreign keys");
    for sql in [
        include_str!("../src/migrations/250_workspace_file_lifecycle.sql"),
        include_str!("../src/migrations/264_workspace_backfill_state.sql"),
    ] {
        conn.execute_batch(sql)
            .unwrap_or_else(|e| panic!("migration apply: {e}"));
    }

    let tables: Vec<String> = conn
        .prepare(
            "SELECT name FROM sqlite_master
             WHERE type='table' AND name LIKE 'workspace_backfill_%'
             ORDER BY name",
        )
        .expect("prepare")
        .query_map([], |row| row.get::<_, String>(0))
        .expect("query")
        .filter_map(Result::ok)
        .collect();
    assert_eq!(
        tables,
        vec![
            "workspace_backfill_items",
            "workspace_backfill_operations",
            "workspace_backfill_runs",
        ]
    );

    assert_has_columns(
        &conn,
        "workspace_backfill_runs",
        &[
            "run_id",
            "mode",
            "status",
            "workspace_root_fingerprint",
            "actor",
            "reason_counts_json",
            "source_class_counts_json",
            "divergence_counts_json",
            "started_at",
            "completed_at",
            "created_at",
            "updated_at",
        ],
    );
    assert_has_columns(
        &conn,
        "workspace_backfill_items",
        &[
            "run_id",
            "source_handle",
            "item_handle",
            "file_id",
            "content_sha256",
            "duplicate_group_handle",
            "candidate_kind",
            "entity_type",
            "entity_id",
            "category",
            "exposure_state",
            "source_time_basis",
            "source_time_confidence",
            "backfill_observed_at",
            "status",
            "reason_code",
            "created_at",
            "updated_at",
        ],
    );
    assert_has_columns(
        &conn,
        "workspace_backfill_operations",
        &[
            "id",
            "run_id",
            "source_handle",
            "operation_kind",
            "status",
            "created_lifecycle",
            "updated_lifecycle_fields",
            "created_link_handle",
            "reason_code",
            "created_at",
            "updated_at",
        ],
    );
    for table in &[
        "workspace_backfill_runs",
        "workspace_backfill_items",
        "workspace_backfill_operations",
    ] {
        assert_forbidden_columns_absent(&conn, table);
    }

    for (table, index) in [
        ("workspace_backfill_runs", "idx_wbr_status"),
        ("workspace_backfill_items", "idx_wbi_source_handle"),
        ("workspace_backfill_items", "idx_wbi_file_id"),
        ("workspace_backfill_items", "idx_wbi_status"),
        ("workspace_backfill_operations", "idx_wbo_run_source"),
        ("workspace_backfill_operations", "idx_wbo_kind_status"),
    ] {
        let indexes = index_names(&conn, table);
        assert!(
            indexes.iter().any(|found| found == index),
            "{table} missing index {index}; got {indexes:?}"
        );
    }

    conn.execute(
        "INSERT INTO workspace_backfill_runs
         (run_id, mode, status, workspace_root_fingerprint, actor)
         VALUES ('run-1', 'apply', 'running', 'root:v1:test', 'system:workspace_backfill:v1')",
        [],
    )
    .expect("valid run");
    conn.execute(
        "INSERT INTO workspace_backfill_items
         (run_id, source_handle, item_handle, file_id, candidate_kind, exposure_state,
          source_time_basis, source_time_confidence, backfill_observed_at, status)
         VALUES ('run-1', 'source:v1:test', 'item:v1:test', 'file-1', 'entity_doc',
          'pending_review', 'filesystem_mtime', 'filesystem_unverified',
          '2026-05-25T00:00:00.000Z', 'planned')",
        [],
    )
    .expect("valid item");
    conn.execute(
        "INSERT INTO workspace_backfill_operations
         (run_id, source_handle, operation_kind, status)
         VALUES ('run-1', 'source:v1:test', 'register_source', 'planned')",
        [],
    )
    .expect("valid operation");

    assert!(
        conn.execute(
            "INSERT INTO workspace_backfill_runs
             (run_id, mode, status, workspace_root_fingerprint, actor)
             VALUES ('run-bad-mode', 'preview', 'running', 'root:v1:test', 'system')",
            [],
        )
        .is_err(),
        "run mode CHECK must reject non-dry_run/apply values"
    );
    assert!(
        conn.execute(
            "INSERT INTO workspace_backfill_runs
             (run_id, mode, status, workspace_root_fingerprint, actor)
             VALUES ('run-bad-status', 'apply', 'queued', 'root:v1:test', 'system')",
            [],
        )
        .is_err(),
        "run status CHECK must reject untracked states"
    );
    assert!(
        conn.execute(
            "INSERT INTO workspace_backfill_items
             (run_id, source_handle, item_handle, file_id, candidate_kind, exposure_state,
              backfill_observed_at, status)
             VALUES ('run-1', 'source:v1:bad-exposure', 'item:v1:bad', 'file-2',
              'entity_doc', 'trusted', '2026-05-25T00:00:00.000Z', 'planned')",
            [],
        )
        .is_err(),
        "item exposure CHECK must reject trusted promotion vocabulary"
    );
    assert!(
        conn.execute(
            "INSERT INTO workspace_backfill_items
             (run_id, source_handle, item_handle, file_id, candidate_kind,
              backfill_observed_at, status)
             VALUES ('run-1', 'source:v1:bad-status', 'item:v1:bad', 'file-2',
              'entity_doc', '2026-05-25T00:00:00.000Z', 'queued')",
            [],
        )
        .is_err(),
        "item status CHECK must reject untracked states"
    );
    assert!(
        conn.execute(
            "INSERT INTO workspace_backfill_operations
             (run_id, source_handle, operation_kind, status)
             VALUES ('run-1', 'source:v1:test', 'register_source', 'queued')",
            [],
        )
        .is_err(),
        "operation status CHECK must reject untracked states"
    );
    assert!(
        conn.execute(
            "INSERT INTO workspace_backfill_operations
             (run_id, source_handle, operation_kind, status)
             VALUES ('run-1', 'source:v1:missing', 'register_source', 'planned')",
            [],
        )
        .is_err(),
        "operation rows must reference an existing backfill item"
    );
}

#[test]
fn w1_b_source_registry_seeded_with_seven_workspace_file_kinds() {
    let conn = Connection::open_in_memory().expect("open in-memory sqlite");
    conn.execute_batch(include_str!(
        "../src/migrations/252_workspace_source_registry.sql"
    ))
    .expect("v252 apply");

    let seeded: Vec<String> = conn
        .prepare("SELECT source_type FROM workspace_source_registry ORDER BY source_type")
        .expect("prepare")
        .query_map([], |row| row.get::<_, String>(0))
        .expect("query")
        .filter_map(Result::ok)
        .collect();

    let expected = vec![
        "drive_sync",
        "entity_doc",
        "granola_transcript",
        "inbox",
        "mcp_placement",
        "quill_transcript",
        "user_attachment",
    ];
    assert_eq!(
        seeded, expected,
        "expected 7 canonical WorkspaceFileKind seeds in alphabetical order"
    );
}

#[test]
fn w1_b_category_registry_seeded_with_18_default_pairs() {
    let conn = Connection::open_in_memory().expect("open in-memory sqlite");
    conn.execute_batch(include_str!(
        "../src/migrations/252_workspace_source_registry.sql"
    ))
    .expect("v252 apply");

    let count: i64 = conn
        .query_row(
            "SELECT count(*) FROM workspace_category_registry",
            [],
            |row| row.get(0),
        )
        .expect("count");
    assert_eq!(
        count, 18,
        "expected 18 (entity_type, category_slug) pairs: 3 entity types × 6 categories"
    );

    // Person must have all 6 categories per V1.1 fold #18 correction.
    let person_count: i64 = conn
        .query_row(
            "SELECT count(*) FROM workspace_category_registry WHERE entity_type = 'person'",
            [],
            |row| row.get(0),
        )
        .expect("person count");
    assert_eq!(person_count, 6, "Person must have all 6 categories");
}

#[test]
fn w1_b_source_registry_data_source_json_uses_externally_tagged_serde_shape() {
    let conn = Connection::open_in_memory().expect("open in-memory sqlite");
    conn.execute_batch(include_str!(
        "../src/migrations/252_workspace_source_registry.sql"
    ))
    .expect("v252 apply");

    let inbox_json: String = conn
        .query_row(
            "SELECT data_source_json FROM workspace_source_registry WHERE source_type = 'inbox'",
            [],
            |row| row.get(0),
        )
        .expect("inbox row");
    assert_eq!(
        inbox_json, r#"{"workspace_file":{"kind":"inbox"}}"#,
        "data_source_json must use canonical externally-tagged serde shape per cycle 3 codex finding"
    );
}

#[test]
fn w1_c_partial_unique_indexes_present() {
    let conn = Connection::open_in_memory().expect("open in-memory sqlite");
    conn.execute_batch(include_str!(
        "../src/migrations/250_workspace_file_lifecycle.sql"
    ))
    .expect("v250 apply (FK target)");
    conn.execute_batch(include_str!(
        "../src/migrations/253_document_ingestion_runs.sql"
    ))
    .expect("v253 apply");
    conn.execute_batch(include_str!(
        "../src/migrations/254_document_entity_links.sql"
    ))
    .expect("v254 apply");

    let indexes: Vec<String> = conn
        .prepare(
            "SELECT name FROM sqlite_master WHERE type='index' AND \
             (name = 'idx_dir_idempotency_unique' OR name = 'idx_del_active_unique')",
        )
        .expect("prepare")
        .query_map([], |row| row.get::<_, String>(0))
        .expect("query")
        .filter_map(Result::ok)
        .collect();

    assert!(
        indexes.contains(&"idx_dir_idempotency_unique".to_string()),
        "v253 must create UNIQUE partial index `idx_dir_idempotency_unique` for idempotency"
    );
    assert!(
        indexes.contains(&"idx_del_active_unique".to_string()),
        "v254 must create UNIQUE partial index `idx_del_active_unique` for tombstone semantics"
    );
}
