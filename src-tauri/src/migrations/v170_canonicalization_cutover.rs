use rusqlite::Connection;

use super::MigrationError;
use crate::services::comparator_thresholds::COMPARATOR_THRESHOLD_VERSION;

const CLAIMS_TABLE: &str = "intelligence_claims";
const CUTOVER_TABLE: &str = "canonicalization_cutover";

// Frozen at v170 application time per ADR-0131 §6. The runtime
// CLAIM_EMBEDDING_MODEL_VERSION constant may move forward later;
// this row records what was authoritative at cutover.
const EMBEDDING_MODEL_VERSION_AT_CUTOVER: &str = "nomic-embed-text-v1.5-Q";

pub(super) fn migrate_v170(conn: &Connection) -> Result<(), MigrationError> {
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
    let pending_backfill: i64 = conn
        .query_row(
            "SELECT count(*) FROM intelligence_claims WHERE canonical_status = 'pending_backfill'",
            [],
            |row| row.get(0),
        )
        .map_err(|e| format!("count pending_backfill claims: {e}"))?;
    if pending_backfill > 0 {
        return Err(format!(
            "ADR-0131 §7 Phase C cutover precondition not met: {pending_backfill} claim(s) remain in pending_backfill state. \
             Backfill must transition every row to either 'live' or 'legacy_unmigrated' before v2 becomes authoritative."
        ));
    }

    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS canonicalization_cutover (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            cutover_at TIMESTAMP NOT NULL,
            embedding_model_version TEXT NOT NULL,
            comparator_threshold_version TEXT NOT NULL
        );
        ",
    )
    .map_err(|e| format!("create {CUTOVER_TABLE}: {e}"))?;

    conn.execute(
        "INSERT OR IGNORE INTO canonicalization_cutover \
            (id, cutover_at, embedding_model_version, comparator_threshold_version) \
         VALUES (1, datetime('now'), ?1, ?2)",
        rusqlite::params![EMBEDDING_MODEL_VERSION_AT_CUTOVER, COMPARATOR_THRESHOLD_VERSION],
    )
    .map_err(|e| format!("insert cutover row: {e}"))?;

    Ok(())
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
    .map_err(|e| format!("check table {table_name}: {e}"))
}

fn execute_batch(conn: &Connection, sql: &str, label: &str) -> Result<(), MigrationError> {
    conn.execute_batch(sql).map_err(|e| format!("{label}: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open_with_claims_table() -> Connection {
        let conn = Connection::open_in_memory().expect("open in-memory sqlite");
        conn.execute_batch(
            "CREATE TABLE intelligence_claims (
                id TEXT PRIMARY KEY,
                canonical_status TEXT NOT NULL DEFAULT 'live'
            );",
        )
        .expect("create intelligence_claims fixture");
        conn
    }

    fn cutover_row_count(conn: &Connection) -> i64 {
        conn.query_row(
            "SELECT count(*) FROM canonicalization_cutover",
            [],
            |row| row.get::<_, i64>(0),
        )
        .expect("count cutover rows")
    }

    #[test]
    fn happy_path_inserts_single_cutover_row() {
        let conn = open_with_claims_table();
        migrate_v170(&conn).expect("migration succeeds with no pending_backfill rows");

        assert_eq!(cutover_row_count(&conn), 1);

        let (model, threshold): (String, String) = conn
            .query_row(
                "SELECT embedding_model_version, comparator_threshold_version \
                 FROM canonicalization_cutover WHERE id = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("read cutover row");
        assert_eq!(model, EMBEDDING_MODEL_VERSION_AT_CUTOVER);
        assert_eq!(threshold, COMPARATOR_THRESHOLD_VERSION);
    }

    #[test]
    fn precondition_blocks_when_pending_backfill_present() {
        let conn = open_with_claims_table();
        conn.execute(
            "INSERT INTO intelligence_claims (id, canonical_status) VALUES (?1, 'pending_backfill')",
            ["claim-pending-1"],
        )
        .expect("insert pending_backfill fixture");

        let err = migrate_v170(&conn).expect_err("migration must fail when pending_backfill > 0");
        assert!(err.contains("Phase C cutover precondition"));
        assert!(err.contains("1 claim"));

        let table_exists_after: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='canonicalization_cutover'",
                [],
                |row| row.get(0),
            )
            .expect("query sqlite_master");
        assert_eq!(table_exists_after, 0, "cutover table must not be created when precondition fails");
    }

    #[test]
    fn idempotent_on_repeat_application() {
        let conn = open_with_claims_table();
        migrate_v170(&conn).expect("first apply");
        migrate_v170(&conn).expect("second apply");
        assert_eq!(cutover_row_count(&conn), 1);
    }
}
