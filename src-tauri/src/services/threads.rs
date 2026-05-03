use chrono::{DateTime, Utc};
use rusqlite::types::Type;
use rusqlite::{params, OptionalExtension};

use crate::abilities::provenance::ThreadId;
use crate::abilities::threads::ThreadMetadata;
use crate::db::ActionDb;
use crate::services::context::ServiceContext;

#[derive(Debug, thiserror::Error)]
pub enum ThreadError {
    #[error("thread mutation blocked by execution mode: {0}")]
    Mode(String),
    #[error(transparent)]
    Rusqlite(#[from] rusqlite::Error),
}

pub fn save_thread(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    thread: &ThreadMetadata,
) -> Result<(), ThreadError> {
    ctx.check_mutation_allowed()
        .map_err(|e| ThreadError::Mode(e.to_string()))?;

    with_write_transaction(db, |tx| {
        tx.conn_ref().execute(
            "INSERT OR IGNORE INTO thread_metadata \
             (thread_id, created_at, created_by, display_label) \
             VALUES (?1, ?2, ?3, ?4)",
            params![
                thread.id.0.to_string(),
                thread.created_at.to_rfc3339(),
                thread.created_by.as_str(),
                thread.display_label.as_deref(),
            ],
        )?;
        Ok(())
    })
}

pub fn get_thread(db: &ActionDb, id: &ThreadId) -> Result<Option<ThreadMetadata>, ThreadError> {
    db.conn_ref()
        .query_row(
            "SELECT thread_id, created_at, created_by, display_label \
             FROM thread_metadata \
             WHERE thread_id = ?1",
            params![id.0.to_string()],
            row_to_thread_metadata,
        )
        .optional()
        .map_err(ThreadError::from)
}

pub fn list_threads_for_claim(
    db: &ActionDb,
    claim_id: &str,
) -> Result<Vec<ThreadMetadata>, ThreadError> {
    let mut stmt = db.conn_ref().prepare(
        "SELECT tm.thread_id, tm.created_at, tm.created_by, tm.display_label \
         FROM intelligence_claims ic \
         JOIN thread_metadata tm ON tm.thread_id = ic.thread_id \
         WHERE ic.id = ?1 \
         ORDER BY tm.created_at ASC, tm.thread_id ASC",
    )?;
    let rows = stmt.query_map(params![claim_id], row_to_thread_metadata)?;
    let mut threads = Vec::new();
    for row in rows {
        threads.push(row?);
    }
    Ok(threads)
}

fn with_write_transaction<F>(db: &ActionDb, f: F) -> Result<(), ThreadError>
where
    F: FnOnce(&ActionDb) -> Result<(), ThreadError>,
{
    let started = db.conn_ref().is_autocommit();
    if started {
        db.conn_ref().execute_batch("BEGIN IMMEDIATE")?;
    }

    match f(db) {
        Ok(()) => {
            if started {
                if let Err(err) = db.conn_ref().execute_batch("COMMIT") {
                    let _ = db.conn_ref().execute_batch("ROLLBACK");
                    return Err(ThreadError::Rusqlite(err));
                }
            }
            Ok(())
        }
        Err(err) => {
            if started {
                let _ = db.conn_ref().execute_batch("ROLLBACK");
            }
            Err(err)
        }
    }
}

fn row_to_thread_metadata(row: &rusqlite::Row<'_>) -> rusqlite::Result<ThreadMetadata> {
    let raw_id: String = row.get(0)?;
    let raw_created_at: String = row.get(1)?;
    Ok(ThreadMetadata {
        id: parse_thread_id(raw_id, 0)?,
        created_at: parse_created_at(raw_created_at, 1)?,
        created_by: row.get(2)?,
        display_label: row.get(3)?,
    })
}

fn parse_thread_id(raw: String, column: usize) -> rusqlite::Result<ThreadId> {
    ThreadId::parse(&raw)
        .map_err(|err| rusqlite::Error::FromSqlConversionFailure(column, Type::Text, Box::new(err)))
}

fn parse_created_at(raw: String, column: usize) -> rusqlite::Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(&raw)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|err| rusqlite::Error::FromSqlConversionFailure(column, Type::Text, Box::new(err)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abilities::threads::create_thread;
    use crate::db::test_utils::test_db;
    use crate::services::context::{Clock, ExternalClients, FixedClock, SeedableRng, SeededRng};
    use chrono::TimeZone;

    fn test_ctx<'a>(
        clock: &'a FixedClock,
        rng: &'a SeedableRng,
        ext: &'a ExternalClients,
    ) -> ServiceContext<'a> {
        ServiceContext::test_live(clock, rng, ext).with_actor("agent:test")
    }

    fn expected_thread_id_from_seed(seed: u64) -> ThreadId {
        let rng = SeedableRng::new(seed);
        let mut bytes = [0u8; 16];
        bytes[..8].copy_from_slice(&rng.random_u64().to_be_bytes());
        bytes[8..].copy_from_slice(&rng.random_u64().to_be_bytes());
        bytes[6] = (bytes[6] & 0x0f) | 0x40;
        bytes[8] = (bytes[8] & 0x3f) | 0x80;
        ThreadId::new(uuid::Uuid::from_bytes(bytes))
    }

    fn fixed_clock() -> FixedClock {
        FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 3, 14, 30, 0).unwrap())
    }

    fn seed_claim(db: &ActionDb, claim_id: &str, thread_id: Option<&ThreadId>) {
        db.conn_ref()
            .execute(
                "INSERT INTO intelligence_claims \
                 (id, subject_ref, claim_type, text, dedup_key, actor, data_source, \
                  observed_at, provenance_json, thread_id) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    claim_id,
                    r#"{"kind":"Account","id":"acct-1"}"#,
                    "account_state",
                    "Renewal strategy is active",
                    format!("dedup-{claim_id}"),
                    "agent:test",
                    "manual",
                    "2026-05-03T14:30:00+00:00",
                    r#"{"provenance_schema_version":1}"#,
                    thread_id.map(|id| id.0.to_string()),
                ],
            )
            .expect("seed claim");
    }

    #[test]
    fn create_thread_uses_ctx_rng_not_global() {
        let clock = fixed_clock();
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);

        let thread = create_thread(&ctx, None);

        assert_eq!(thread.id, expected_thread_id_from_seed(42));
    }

    #[test]
    fn create_thread_captures_clock_and_actor() {
        let clock = fixed_clock();
        let rng = SeedableRng::new(7);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);

        let thread = create_thread(&ctx, Some("Q4 renewal strategy"));

        assert_eq!(thread.created_at, clock.now());
        assert_eq!(thread.created_by, "agent:test");
        assert_eq!(thread.display_label.as_deref(), Some("Q4 renewal strategy"));
    }

    #[test]
    fn save_thread_then_get_thread_round_trips() {
        let db = test_db();
        let clock = fixed_clock();
        let rng = SeedableRng::new(11);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);
        let thread = create_thread(&ctx, Some("Expansion plan"));

        save_thread(&ctx, &db, &thread).expect("save thread");
        let read_back = get_thread(&db, &thread.id).expect("get thread");

        assert_eq!(read_back, Some(thread));
    }

    #[test]
    fn list_threads_for_claim_returns_associated_threads() {
        let db = test_db();
        let clock = fixed_clock();
        let rng = SeedableRng::new(12);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);
        let thread = create_thread(&ctx, Some("Budget risk"));
        save_thread(&ctx, &db, &thread).expect("save thread");
        seed_claim(&db, "claim-with-thread", Some(&thread.id));

        let threads = list_threads_for_claim(&db, "claim-with-thread").expect("list threads");

        assert_eq!(threads, vec![thread]);
    }

    #[test]
    fn list_threads_for_claim_returns_empty_when_no_thread() {
        let db = test_db();
        seed_claim(&db, "claim-without-thread", None);

        let threads = list_threads_for_claim(&db, "claim-without-thread").expect("list threads");

        assert!(threads.is_empty());
    }

    #[test]
    fn save_thread_idempotent() {
        let db = test_db();
        let clock = fixed_clock();
        let rng = SeedableRng::new(13);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);
        let original = create_thread(&ctx, Some("Original label"));
        let duplicate = ThreadMetadata {
            display_label: Some("Updated label should be ignored".to_string()),
            ..original.clone()
        };

        save_thread(&ctx, &db, &original).expect("save original");
        save_thread(&ctx, &db, &duplicate).expect("save duplicate");

        let read_back = get_thread(&db, &original.id)
            .expect("get thread")
            .expect("thread exists");
        assert_eq!(read_back, original);
    }
}
