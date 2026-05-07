use std::collections::HashSet;

use rusqlite::Connection;

use super::MigrationError;

const TABLE: &str = "sensitivity_reveal_audit";
const REBUILD_TABLE: &str = "sensitivity_reveal_audit_new";
const ACTION_INDEX: &str = "idx_sensitivity_reveal_audit_action_token";

pub(super) fn migrate_v144_audit_action_token(conn: &Connection) -> Result<(), MigrationError> {
    execute_batch(conn, "PRAGMA foreign_keys = OFF;", "disable foreign keys")?;

    let migration_result = run_with_transaction(conn);
    let restore_result = execute_batch(conn, "PRAGMA foreign_keys = ON;", "restore foreign keys");

    match (migration_result, restore_result) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(error), _) => Err(error),
        (Ok(()), Err(error)) => Err(error),
    }
}

fn run_with_transaction(conn: &Connection) -> Result<(), MigrationError> {
    execute_batch(conn, "BEGIN IMMEDIATE;", "begin immediate transaction")?;

    let result = migrate_in_transaction(conn);
    match result {
        Ok(()) => {
            if let Err(error) = execute_batch(conn, "COMMIT;", "commit transaction") {
                let _ = conn.execute_batch("ROLLBACK;");
                return Err(error);
            }
            Ok(())
        }
        Err(error) => {
            let _ = conn.execute_batch("ROLLBACK;");
            Err(error)
        }
    }
}

fn migrate_in_transaction(conn: &Connection) -> Result<(), MigrationError> {
    let source_exists = table_exists(conn, TABLE)?;
    let rebuild_exists = table_exists(conn, REBUILD_TABLE)?;

    if !source_exists {
        if rebuild_exists {
            promote_rebuild_table(conn)?;
            return Ok(());
        }

        return Err(format!("required table {TABLE} is missing"));
    }

    let columns = table_columns(conn, TABLE)?;
    let has_reveal_action_id = columns.contains("reveal_action_id");
    let has_reveal_session_id = columns.contains("reveal_session_id");
    let has_audit_bucket = columns.contains("audit_bucket");
    let has_action_index = index_exists(conn, ACTION_INDEX)?;

    if has_reveal_action_id && !has_reveal_session_id && !has_audit_bucket && has_action_index {
        return Ok(());
    }

    execute_batch(
        conn,
        "DROP INDEX IF EXISTS idx_sensitivity_reveal_audit_reveal_session;
         DROP INDEX IF EXISTS idx_sensitivity_reveal_audit_audit_bucket;
         DROP TABLE IF EXISTS sensitivity_reveal_audit_new;

         CREATE TABLE sensitivity_reveal_audit_new (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            claim_id TEXT NOT NULL,
            user_id TEXT NOT NULL,
            revealed_at TEXT NOT NULL,
            reveal_action_id TEXT NOT NULL DEFAULT '',
            FOREIGN KEY (claim_id) REFERENCES intelligence_claims(id) ON DELETE CASCADE
         );",
        "prepare rebuild table",
    )?;

    let reveal_action_expr = if has_reveal_action_id {
        "COALESCE(reveal_action_id, '')"
    } else {
        "''"
    };
    let insert_sql = format!(
        "INSERT INTO {REBUILD_TABLE} (id, claim_id, user_id, revealed_at, reveal_action_id)
         SELECT id, claim_id, user_id, revealed_at, {reveal_action_expr}
         FROM {TABLE};"
    );
    execute_batch(conn, &insert_sql, "copy reveal audit rows")?;

    execute_batch(
        conn,
        "DROP TABLE sensitivity_reveal_audit;
         ALTER TABLE sensitivity_reveal_audit_new RENAME TO sensitivity_reveal_audit;",
        "replace reveal audit table",
    )?;
    create_canonical_indexes(conn)
}

fn promote_rebuild_table(conn: &Connection) -> Result<(), MigrationError> {
    let columns = table_columns(conn, REBUILD_TABLE)?;
    for required in [
        "id",
        "claim_id",
        "user_id",
        "revealed_at",
        "reveal_action_id",
    ] {
        if !columns.contains(required) {
            return Err(format!(
                "cannot recover partial v144 state: {REBUILD_TABLE}.{required} is missing"
            ));
        }
    }

    execute_batch(
        conn,
        "ALTER TABLE sensitivity_reveal_audit_new RENAME TO sensitivity_reveal_audit;",
        "promote partial rebuild table",
    )?;
    create_canonical_indexes(conn)
}

fn create_canonical_indexes(conn: &Connection) -> Result<(), MigrationError> {
    execute_batch(
        conn,
        "CREATE INDEX IF NOT EXISTS idx_sensitivity_reveal_audit_claim
            ON sensitivity_reveal_audit(claim_id, revealed_at);
         CREATE INDEX IF NOT EXISTS idx_sensitivity_reveal_audit_user
            ON sensitivity_reveal_audit(user_id, revealed_at);
         CREATE UNIQUE INDEX IF NOT EXISTS idx_sensitivity_reveal_audit_action_token
            ON sensitivity_reveal_audit(claim_id, user_id, reveal_action_id)
            WHERE reveal_action_id != '';",
        "create canonical indexes",
    )
}

fn table_exists(conn: &Connection, table_name: &str) -> Result<bool, MigrationError> {
    conn.query_row(
        "SELECT EXISTS(
            SELECT 1
            FROM sqlite_master
            WHERE type = 'table' AND name = ?1
        )",
        [table_name],
        |row| row.get::<_, i64>(0),
    )
    .map(|count| count != 0)
    .map_err(|error| format!("v144 table existence check failed for {table_name}: {error}"))
}

fn table_columns(conn: &Connection, table_name: &str) -> Result<HashSet<String>, MigrationError> {
    let pragma = format!("PRAGMA table_info('{table_name}')");
    let mut stmt = conn
        .prepare(&pragma)
        .map_err(|error| format!("v144 column inspection failed for {table_name}: {error}"))?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|error| format!("v144 column query failed for {table_name}: {error}"))?;

    let mut columns = HashSet::new();
    for row in rows {
        columns.insert(row.map_err(|error| {
            format!("v144 column metadata read failed for {table_name}: {error}")
        })?);
    }

    Ok(columns)
}

fn index_exists(conn: &Connection, index_name: &str) -> Result<bool, MigrationError> {
    conn.query_row(
        "SELECT EXISTS(
            SELECT 1
            FROM sqlite_master
            WHERE type = 'index' AND name = ?1
        )",
        [index_name],
        |row| row.get::<_, i64>(0),
    )
    .map(|count| count != 0)
    .map_err(|error| format!("v144 index existence check failed for {index_name}: {error}"))
}

fn execute_batch(
    conn: &Connection,
    sql: &str,
    context: &'static str,
) -> Result<(), MigrationError> {
    conn.execute_batch(sql)
        .map_err(|error| format!("v144 action token migration failed to {context}: {error}"))
}
