//! Linear issue state provenance for entity chapters, signal emission, and
//! meeting callouts: retain the upstream update timestamp + assignee identity
//! so claim freshness and trust factors can read source-of-truth state.
//!
//! Idempotent variant of the original SQL migration so test fixtures that
//! roll `schema_version` back without rolling the actual schema (the v157 /
//! v162 repair regression tests) can re-apply v178 without tripping
//! `duplicate column name`.

use rusqlite::{Connection, OptionalExtension};

use super::MigrationError;

const LINEAR_ISSUES_TABLE: &str = "linear_issues";

pub(super) fn migrate_v178(conn: &Connection) -> Result<(), MigrationError> {
    if !table_exists(conn, LINEAR_ISSUES_TABLE)? {
        return Ok(());
    }

    add_column_if_missing(conn, "linear_updated_at", "TEXT")?;
    add_column_if_missing(conn, "assignee_id", "TEXT")?;
    add_column_if_missing(conn, "assignee_name", "TEXT")?;

    conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_linear_issues_updated \
         ON linear_issues(linear_updated_at DESC);",
    )
    .map_err(|e| format!("create idx_linear_issues_updated: {e}"))?;

    Ok(())
}

fn table_exists(conn: &Connection, table_name: &str) -> Result<bool, MigrationError> {
    let exists: i64 = conn
        .query_row(
            "SELECT COUNT(*)
             FROM sqlite_master
             WHERE type = 'table' AND name = ?1",
            [table_name],
            |row| row.get(0),
        )
        .map_err(|e| format!("query table existence for {table_name}: {e}"))?;
    Ok(exists > 0)
}

fn add_column_if_missing(
    conn: &Connection,
    column_name: &str,
    column_type: &str,
) -> Result<(), MigrationError> {
    if column_exists(conn, LINEAR_ISSUES_TABLE, column_name)? {
        return Ok(());
    }
    conn.execute(
        &format!("ALTER TABLE {LINEAR_ISSUES_TABLE} ADD COLUMN {column_name} {column_type}"),
        [],
    )
    .map_err(|e| format!("add {LINEAR_ISSUES_TABLE}.{column_name}: {e}"))?;
    Ok(())
}

fn table_exists(conn: &Connection, table_name: &str) -> Result<bool, MigrationError> {
    conn.query_row(
        "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1",
        [table_name],
        |_| Ok(()),
    )
    .optional()
    .map(|row| row.is_some())
    .map_err(|e| format!("check table {table_name}: {e}"))
}

fn column_exists(
    conn: &Connection,
    table_name: &str,
    column_name: &str,
) -> Result<bool, MigrationError> {
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info({table_name})"))
        .map_err(|e| format!("prepare table info for {table_name}: {e}"))?;
    let mut rows = stmt
        .query([])
        .map_err(|e| format!("query table info for {table_name}: {e}"))?;
    while let Some(row) = rows
        .next()
        .map_err(|e| format!("read table info for {table_name}: {e}"))?
    {
        let name: String = row
            .get(1)
            .map_err(|e| format!("read column name for {table_name}: {e}"))?;
        if name == column_name {
            return Ok(true);
        }
    }
    Ok(false)
}
