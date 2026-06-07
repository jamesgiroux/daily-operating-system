use rusqlite::{Connection, OptionalExtension};

pub(crate) fn unresolved_contradiction_count(
    conn: &Connection,
    claim_id: &str,
) -> Result<u32, rusqlite::Error> {
    if !table_exists(conn, "claim_contradictions")? {
        return Ok(0);
    }
    let count: i64 = conn.query_row(
        "SELECT COUNT(*)
           FROM claim_contradictions
          WHERE reconciled_at IS NULL
            AND (primary_claim_id = ?1 OR contradicting_claim_id = ?1)",
        [claim_id],
        |row| row.get(0),
    )?;
    Ok(u32::try_from(count.max(0)).unwrap_or(u32::MAX))
}

pub(crate) fn contradiction_caveat(count: u32) -> Option<String> {
    match count {
        0 => None,
        1 => Some("unresolved contradiction present".to_string()),
        n => Some(format!("{n} unresolved contradictions present")),
    }
}

fn table_exists(conn: &Connection, table: &str) -> Result<bool, rusqlite::Error> {
    conn.query_row(
        "SELECT 1
           FROM sqlite_master
          WHERE type = 'table'
            AND name = ?1",
        [table],
        |_| Ok(()),
    )
    .optional()
    .map(|value| value.is_some())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unresolved_count_handles_absent_table() {
        let conn = Connection::open_in_memory().unwrap();
        assert_eq!(unresolved_contradiction_count(&conn, "claim-1").unwrap(), 0);
    }

    #[test]
    fn unresolved_count_checks_both_edge_sides() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE claim_contradictions (
                id TEXT PRIMARY KEY,
                primary_claim_id TEXT NOT NULL,
                contradicting_claim_id TEXT NOT NULL,
                reconciled_at TEXT
            );",
        )
        .unwrap();
        conn.execute(
            "INSERT INTO claim_contradictions -- dos7-allowed: unit-test fixture seed; production path is read-only
                (id, primary_claim_id, contradicting_claim_id, reconciled_at)
             VALUES
                ('edge-1', 'claim-1', 'claim-2', NULL),
                ('edge-2', 'claim-3', 'claim-1', NULL),
                ('edge-3', 'claim-1', 'claim-4', '2026-06-07T00:00:00Z')",
            [],
        )
        .unwrap();

        assert_eq!(unresolved_contradiction_count(&conn, "claim-1").unwrap(), 2);
    }

    #[test]
    fn caveat_is_stable_and_pluralized() {
        assert_eq!(contradiction_caveat(0), None);
        assert_eq!(
            contradiction_caveat(1).as_deref(),
            Some("unresolved contradiction present")
        );
        assert_eq!(
            contradiction_caveat(2).as_deref(),
            Some("2 unresolved contradictions present")
        );
    }
}
