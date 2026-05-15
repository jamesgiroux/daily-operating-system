//! Substrate concurrency contract: server-assigned `claim_version` watermark,
//! `mutation_attempts` chokepoint, `composition_versions` watermark table,
//! and `version_events` outbox.
//!
//! Idempotent variant of the original SQL migration so test fixtures that
//! roll `schema_version` back without rolling the actual schema (the v157 /
//! v162 repair regression tests) can re-apply v172 without tripping
//! `duplicate column name` / `table already exists`.

use rusqlite::Connection;

use super::MigrationError;

const CLAIMS_TABLE: &str = "intelligence_claims";

pub(super) fn migrate_v172(conn: &Connection) -> Result<(), MigrationError> {
    if !table_exists(conn, CLAIMS_TABLE)? {
        return Err(format!("required table {CLAIMS_TABLE} is missing"));
    }

    execute_batch(conn, "BEGIN IMMEDIATE;", "begin immediate transaction")?;
    let result = migrate_in_transaction(conn);
    match result {
        Ok(()) => execute_batch(conn, "COMMIT;", "commit transaction"),
        Err(error) => {
            #[allow(
                clippy::let_underscore_must_use,
                reason = "intentional best-effort cleanup after migration failure"
            )]
            let _ = conn.execute_batch("ROLLBACK;");
            Err(error)
        }
    }
}

fn migrate_in_transaction(conn: &Connection) -> Result<(), MigrationError> {
    add_claim_version_column_if_missing(conn)?;

    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS mutation_attempts (
            mutation_id TEXT PRIMARY KEY,
            claim_id TEXT,
            composition_id TEXT,
            cursor TEXT NOT NULL UNIQUE,
            started_at TEXT NOT NULL,
            status TEXT NOT NULL CHECK (status IN ('in_flight', 'committed', 'aborted')),
            finalized_at TEXT,
            CHECK (
                (status = 'in_flight' AND finalized_at IS NULL)
                OR (status != 'in_flight' AND finalized_at IS NOT NULL)
            ),
            CHECK ((claim_id IS NOT NULL) != (composition_id IS NOT NULL))
        );

        CREATE INDEX IF NOT EXISTS idx_mutation_attempts_in_flight
            ON mutation_attempts (started_at)
            WHERE status = 'in_flight';

        CREATE TABLE IF NOT EXISTS composition_versions (
            composition_id TEXT PRIMARY KEY,
            composition_version INTEGER NOT NULL,
            generated_at TEXT NOT NULL,
            generated_by_invocation_id TEXT NOT NULL,
            generated_by_actor_kind TEXT NOT NULL,
            CHECK (composition_version BETWEEN 1 AND 9223372036854775807)
        );

        CREATE TABLE IF NOT EXISTS version_events (
            event_seq INTEGER PRIMARY KEY AUTOINCREMENT,
            cursor TEXT NOT NULL UNIQUE CHECK (length(cursor) = 36 AND cursor GLOB '*-*-*-*-*'),
            event_kind TEXT NOT NULL CHECK (event_kind IN (
                'claim.updated',
                'claim.corrected',
                'claim.superseded',
                'claim.tombstoned',
                'claim.write_rejected',
                'claim.conflict_detected',
                'composition.updated',
                'composition.write_rejected',
                'mutation_aborted'
            )),
            claim_id TEXT,
            composition_id TEXT,
            previous_version INTEGER,
            current_version INTEGER NOT NULL,
            reason TEXT,
            scope_redacted INTEGER NOT NULL CHECK (scope_redacted IN (0, 1)),
            correction_event_log_id TEXT,
            mutation_id TEXT,
            created_at TEXT NOT NULL,
            actor_kind TEXT NOT NULL CHECK (actor_kind IN ('user', 'agent', 'admin', 'system', 'surface_client')),
            CHECK ((claim_id IS NOT NULL) != (composition_id IS NOT NULL))
        );

        CREATE INDEX IF NOT EXISTS idx_version_events_claim
            ON version_events (claim_id, current_version);

        CREATE INDEX IF NOT EXISTS idx_version_events_composition
            ON version_events (composition_id, current_version);
        ",
    )
    .map_err(|e| format!("create v172 substrate tables: {e}"))?;

    conn.execute_batch(
        "UPDATE intelligence_claims
         SET claim_version = 1
         WHERE claim_version = 0;",
    )
    .map_err(|e| format!("backfill claim_version baseline: {e}"))?;

    // Single-row backfill audit pair (see original SQL for rationale). Both
    // INSERTs are guarded by `WHERE NOT EXISTS` so re-running the migration
    // (test rollback scenarios) is a no-op rather than a unique-constraint
    // panic.
    conn.execute_batch(
        "INSERT INTO mutation_attempts (
            mutation_id,
            claim_id,
            composition_id,
            cursor,
            started_at,
            status,
            finalized_at
        )
        SELECT
            'migration-172-backfill-summary',
            '__migration_172_backfill__',
            NULL,
            lower(
                hex(randomblob(4)) || '-' ||
                hex(randomblob(2)) || '-' ||
                '4' || substr(hex(randomblob(2)), 2) || '-' ||
                substr('89ab', abs(random()) % 4 + 1, 1) || substr(hex(randomblob(2)), 2) || '-' ||
                hex(randomblob(6))
            ),
            datetime('now'),
            'committed',
            datetime('now')
        WHERE NOT EXISTS (
            SELECT 1 FROM mutation_attempts WHERE mutation_id = 'migration-172-backfill-summary'
        );

        INSERT INTO version_events (
            cursor,
            event_kind,
            claim_id,
            previous_version,
            current_version,
            reason,
            scope_redacted,
            mutation_id,
            created_at,
            actor_kind
        )
        SELECT
            ma.cursor,
            'claim.updated',
            '__migration_172_backfill__',
            0,
            1,
            'claim_version_backfill:row_count=' || (
                SELECT COUNT(*) FROM intelligence_claims WHERE claim_version = 1
            ) || ':migration_version=172',
            0,
            ma.mutation_id,
            ma.finalized_at,
            'system'
        FROM mutation_attempts ma
        WHERE ma.mutation_id = 'migration-172-backfill-summary'
          AND NOT EXISTS (
              SELECT 1 FROM version_events ve
              WHERE ve.mutation_id = ma.mutation_id
          );",
    )
    .map_err(|e| format!("seed v172 backfill audit pair: {e}"))?;

    Ok(())
}

fn add_claim_version_column_if_missing(conn: &Connection) -> Result<(), MigrationError> {
    if column_exists(conn, CLAIMS_TABLE, "claim_version")? {
        return Ok(());
    }
    conn.execute(
        &format!(
            "ALTER TABLE {CLAIMS_TABLE} \
             ADD COLUMN claim_version INTEGER NOT NULL DEFAULT 0 \
             CHECK (claim_version BETWEEN 0 AND 9223372036854775807)"
        ),
        [],
    )
    .map_err(|e| format!("add {CLAIMS_TABLE}.claim_version: {e}"))?;
    Ok(())
}

fn table_exists(conn: &Connection, table_name: &str) -> Result<bool, MigrationError> {
    conn.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1
        )",
        [table_name],
        |row| row.get::<_, i64>(0),
    )
    .map(|count| count != 0)
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

fn execute_batch(conn: &Connection, sql: &str, label: &str) -> Result<(), MigrationError> {
    conn.execute_batch(sql).map_err(|e| format!("{label}: {e}"))
}
