//! Migration round-trip tests for v1.4.5 W1-A + W1-B + W1-C SQL.
//! Pre-stages the L1 acceptance criteria from each lane's L0 packet §8.

use rusqlite::Connection;

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
    assert_eq!(seeded, expected, "expected 7 canonical WorkspaceFileKind seeds in alphabetical order");
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
