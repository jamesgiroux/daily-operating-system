//! Async database service using tokio-rusqlite.
//!
//! Provides read/write separation for SQLite in WAL mode:
//! - 1 writer connection (serialized via tokio-rusqlite's internal channel)
//! - 2 reader connections (concurrent under WAL, round-robin dispatched)
//!
//! All SQLite I/O runs on dedicated OS threads, never blocking the Tokio
//! runtime. This eliminates the beachball caused by the old
//! `Mutex<Option<ActionDb>>` pattern where background tasks holding the
//! lock for seconds would block every Tauri command handler.

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use tokio_rusqlite::Connection;

use crate::db::DbError;

/// Number of read connections in the pool. 2 is plenty for a desktop app —
/// WAL mode allows concurrent readers so this gives us two parallel reads
/// while a write is in progress.
const NUM_READERS: usize = 2;

/// Async database connection pool with read/write separation.
///
/// The writer connection is serialized: all closures submitted via `writer()`
/// execute sequentially on a single dedicated thread. Reader connections
/// share WAL snapshots and can execute concurrently with the writer and
/// each other.
pub struct DbService {
    writer: Connection,
    readers: Vec<Connection>,
    read_idx: AtomicUsize,
}

/// Apply standard pragmas to a connection. Both readers and writers get
/// busy_timeout and WAL mode; readers additionally get query_only.
/// PRAGMA key is set first for SQLCipher (ADR-0092).
fn apply_pragmas(
    conn: &rusqlite::Connection,
    read_only: bool,
    hex_key: &str,
) -> Result<(), rusqlite::Error> {
    // PRAGMA key MUST be first — before any other PRAGMA (ADR-0092)
    conn.execute_batch(&crate::db::encryption::key_to_pragma(hex_key))?;
    conn.execute_batch("PRAGMA journal_mode = WAL;")?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;")?;
    conn.execute_batch("PRAGMA synchronous = NORMAL;")?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    if read_only {
        conn.execute_batch("PRAGMA query_only = ON;")?;
    }
    Ok(())
}

impl DbService {
    /// Open a new DbService at the standard database path.
    ///
    /// Runs migrations on the writer connection, then opens reader connections.
    /// Must be called from an async context (Tokio runtime).
    pub async fn open() -> Result<Self, DbError> {
        let path = crate::db::ActionDb::db_path_public()?;
        Self::open_at(path).await
    }

    /// Open a DbService at an explicit path. Used for testing.
    pub async fn open_at(path: PathBuf) -> Result<Self, DbError> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent).map_err(DbError::CreateDir)?;
            }
        }

        // Get or create encryption key from Keychain (ADR-0092)
        let hex_key =
            crate::db::encryption::get_or_create_db_key(&path).map_err(DbError::Encryption)?;

        // Migrate plaintext DB if it exists
        if path.exists() && crate::db::encryption::is_database_plaintext(&path) {
            log::info!("DbService: Detected plaintext database, migrating to encrypted...");
            crate::db::encryption::migrate_to_encrypted(&path, &hex_key)
                .map_err(DbError::Encryption)?;
        }

        let path_str = path.to_string_lossy().to_string();
        let writer_key = hex_key.clone();

        // Open the writer connection — this is where migrations run.
        let writer = Connection::open(&path_str)
            .await
            .map_err(|e| DbError::Migration(format!("Failed to open writer: {e}")))?;

        // Apply pragmas and run migrations on the writer.
        writer
            .call(move |conn| {
                apply_pragmas(conn, false, &writer_key)?;
                crate::migrations::run_migrations(conn)
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(e.into()))?;
                Ok(())
            })
            .await
            .map_err(|e| DbError::Migration(e.to_string()))?;

        // Open reader connections — no migrations, just pragmas.
        let mut readers = Vec::with_capacity(NUM_READERS);
        for _ in 0..NUM_READERS {
            let r = Connection::open(&path_str)
                .await
                .map_err(|e| DbError::Migration(format!("Failed to open reader: {e}")))?;

            let reader_key = hex_key.clone();
            r.call(move |conn| {
                apply_pragmas(conn, true, &reader_key)?;
                Ok(())
            })
            .await
            .map_err(|e| DbError::Migration(e.to_string()))?;

            readers.push(r);
        }

        Ok(Self {
            writer,
            readers,
            read_idx: AtomicUsize::new(0),
        })
    }

    /// Open an unencrypted DbService at an explicit path. Tests only.
    #[cfg(test)]
    pub async fn open_at_unencrypted(path: PathBuf) -> Result<Self, DbError> {
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent).map_err(DbError::CreateDir)?;
            }
        }

        let path_str = path.to_string_lossy().to_string();

        let writer = Connection::open(&path_str)
            .await
            .map_err(|e| DbError::Migration(format!("Failed to open writer: {e}")))?;

        writer
            .call(|conn| {
                conn.execute_batch("PRAGMA journal_mode = WAL;")?;
                conn.execute_batch("PRAGMA busy_timeout = 5000;")?;
                conn.execute_batch("PRAGMA synchronous = NORMAL;")?;
                conn.execute_batch("PRAGMA foreign_keys = ON;")?;
                crate::migrations::run_migrations(conn)
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(e.into()))?;
                Ok(())
            })
            .await
            .map_err(|e| DbError::Migration(e.to_string()))?;

        let mut readers = Vec::with_capacity(NUM_READERS);
        for _ in 0..NUM_READERS {
            let r = Connection::open(&path_str)
                .await
                .map_err(|e| DbError::Migration(format!("Failed to open reader: {e}")))?;
            r.call(|conn| {
                conn.execute_batch("PRAGMA journal_mode = WAL;")?;
                conn.execute_batch("PRAGMA busy_timeout = 5000;")?;
                conn.execute_batch("PRAGMA synchronous = NORMAL;")?;
                conn.execute_batch("PRAGMA foreign_keys = ON;")?;
                conn.execute_batch("PRAGMA query_only = ON;")?;
                Ok(())
            })
            .await
            .map_err(|e| DbError::Migration(e.to_string()))?;
            readers.push(r);
        }

        Ok(Self {
            writer,
            readers,
            read_idx: AtomicUsize::new(0),
        })
    }

    /// Get a reader connection (round-robin). Use for SELECT-only queries
    /// from Tauri command handlers. Never blocks the writer.
    pub fn reader(&self) -> &Connection {
        let idx = self.read_idx.fetch_add(1, Ordering::Relaxed) % self.readers.len();
        &self.readers[idx]
    }

    /// Get the writer connection. Use for INSERT/UPDATE/DELETE operations.
    /// All writes are serialized through this single connection, preventing
    /// WAL contention and SQLITE_BUSY retry storms.
    pub fn writer(&self) -> &Connection {
        &self.writer
    }
}

#[cfg(test)]
mod tests {
    //! DOS-229 — verify that writes are immediately visible to subsequent
    //! reads through the long-lived reader pool. Without the fix, the
    //! `query_only=ON` reader connections can serve a stale WAL snapshot
    //! when the round-robin happens to reuse a connection that is still
    //! holding an older mxFrame mark.
    use super::*;
    use crate::db::ActionDb;

    /// Build a minimal email row for the visibility tests below.
    fn sample_email(id: &str, entity_id: &str) -> crate::db::DbEmail {
        let now = chrono::Utc::now().to_rfc3339();
        crate::db::DbEmail {
            email_id: id.to_string(),
            thread_id: Some(format!("thread-{id}")),
            sender_email: Some("owner@example.com".to_string()),
            sender_name: Some("Owner".to_string()),
            subject: Some("Subject".to_string()),
            snippet: Some("snip".to_string()),
            priority: Some("high".to_string()),
            is_unread: true,
            received_at: Some(now.clone()),
            enrichment_state: "enriched".to_string(),
            enrichment_attempts: 0,
            last_enrichment_at: None,
            enriched_at: Some(now.clone()),
            last_seen_at: Some(now.clone()),
            resolved_at: None,
            entity_id: Some(entity_id.to_string()),
            entity_type: Some("account".to_string()),
            contextual_summary: Some("ctx".to_string()),
            sentiment: None,
            urgency: None,
            user_is_last_sender: false,
            last_sender_email: Some("owner@example.com".to_string()),
            message_count: 1,
            created_at: now.clone(),
            updated_at: now,
            relevance_score: Some(0.5),
            score_reason: Some("test".to_string()),
            pinned_at: None,
            commitments: None,
            questions: None,
            is_noise: false,
        }
    }

    /// DOS-229 repro #1: update_email_entity through writer must be visible
    /// to a subsequent get_all_active_emails on every reader connection.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn dos_229_email_entity_update_visible_to_readers() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("dos229.db");
        let svc = DbService::open_at_unencrypted(path)
            .await
            .expect("open svc");

        let email = sample_email("em-dos229-1", "acc-old");
        svc.writer()
            .call(move |conn| {
                let db = ActionDb::from_conn(conn);
                db.upsert_email(&email).expect("upsert");
                Ok(())
            })
            .await
            .expect("writer call");

        // Prime BOTH reader connections so each opens a WAL snapshot before
        // the next write. Without the fix, the snapshot is held and the
        // following update is invisible until the connection is recycled.
        for _ in 0..(NUM_READERS * 2) {
            let r = svc.reader();
            r.call(|conn| {
                let db = ActionDb::from_conn(conn);
                let _ = db.get_all_active_emails().expect("read");
                Ok(())
            })
            .await
            .expect("reader call");
        }

        // Now flip the entity_id through the writer.
        svc.writer()
            .call(|conn| {
                let db = ActionDb::from_conn(conn);
                db.update_email_entity("em-dos229-1", Some("acc-new"), Some("account"))
                    .expect("update");
                Ok(())
            })
            .await
            .expect("writer call");

        // Hit every reader at least once and assert the new value is visible.
        for i in 0..(NUM_READERS * 2) {
            let r = svc.reader();
            let rows = r
                .call(|conn| {
                    let db = ActionDb::from_conn(conn);
                    Ok(db.get_all_active_emails().expect("read"))
                })
                .await
                .expect("reader call");
            assert_eq!(rows.len(), 1, "iteration {i}: expected 1 row");
            assert_eq!(
                rows[0].entity_id.as_deref(),
                Some("acc-new"),
                "iteration {i}: reader returned stale entity_id"
            );
        }
    }

    /// DOS-229 repro #1b: same as #1 but with overlapping reads in flight
    /// at the moment the write commits. This stresses the snapshot lifecycle
    /// and is the closest analog to a UI that fans out many `db_read` calls
    /// (entity names, signals, commitments, threads) while the user clicks
    /// "save" on a chip.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn dos_229_email_entity_update_visible_after_concurrent_reads() {
        use std::sync::Arc;
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("dos229_concurrent.db");
        let svc = Arc::new(
            DbService::open_at_unencrypted(path)
                .await
                .expect("open svc"),
        );

        let email = sample_email("em-dos229-3", "acc-old");
        svc.writer()
            .call(move |conn| {
                let db = ActionDb::from_conn(conn);
                db.upsert_email(&email).expect("upsert");
                Ok(())
            })
            .await
            .expect("writer call");

        // Spawn a flurry of background reads to keep the reader pool busy.
        let mut handles = Vec::new();
        for _ in 0..32 {
            let svc2 = Arc::clone(&svc);
            handles.push(tokio::spawn(async move {
                let r = svc2.reader();
                r.call(|conn| {
                    let db = ActionDb::from_conn(conn);
                    let _ = db.get_all_active_emails().expect("read");
                    Ok(())
                })
                .await
                .expect("reader call");
            }));
        }

        // Write while reads are in flight.
        svc.writer()
            .call(|conn| {
                let db = ActionDb::from_conn(conn);
                db.update_email_entity("em-dos229-3", Some("acc-new"), Some("account"))
                    .expect("update");
                Ok(())
            })
            .await
            .expect("writer call");

        for h in handles {
            let _ = h.await;
        }

        for i in 0..(NUM_READERS * 4) {
            let r = svc.reader();
            let rows = r
                .call(|conn| {
                    let db = ActionDb::from_conn(conn);
                    Ok(db.get_all_active_emails().expect("read"))
                })
                .await
                .expect("reader call");
            assert_eq!(rows.len(), 1, "iter {i}");
            assert_eq!(
                rows[0].entity_id.as_deref(),
                Some("acc-new"),
                "iter {i}: stale entity_id from reader pool"
            );
        }
    }

    /// DOS-229 repro #1c: reader connection that has sat idle since a prior
    /// read. The prior read closed its statement (drop) but the connection
    /// has been alive on its dedicated tokio_rusqlite thread.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn dos_229_reader_after_idle_sees_writer_update() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("dos229_idle.db");
        let svc = DbService::open_at_unencrypted(path)
            .await
            .expect("open svc");

        let email = sample_email("em-dos229-4", "acc-old");
        svc.writer()
            .call(move |conn| {
                let db = ActionDb::from_conn(conn);
                db.upsert_email(&email).expect("upsert");
                Ok(())
            })
            .await
            .expect("writer call");

        // Hit each reader once.
        for _ in 0..NUM_READERS {
            svc.reader()
                .call(|conn| {
                    let db = ActionDb::from_conn(conn);
                    let _ = db.get_all_active_emails().expect("read");
                    Ok(())
                })
                .await
                .expect("reader call");
        }

        // Let readers go idle.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Write update.
        svc.writer()
            .call(|conn| {
                let db = ActionDb::from_conn(conn);
                db.update_email_entity("em-dos229-4", Some("acc-new"), Some("account"))
                    .expect("update");
                Ok(())
            })
            .await
            .expect("writer call");

        // Idle longer.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Round-robin both readers and assert visibility.
        for i in 0..(NUM_READERS * 4) {
            let r = svc.reader();
            let rows = r
                .call(|conn| {
                    let db = ActionDb::from_conn(conn);
                    Ok(db.get_all_active_emails().expect("read"))
                })
                .await
                .expect("reader call");
            assert_eq!(
                rows[0].entity_id.as_deref(),
                Some("acc-new"),
                "iter {i}: stale entity_id from idle reader"
            );
        }
    }

    /// DOS-229 repro #2: a sentiment column update through the writer must
    /// be visible to readers immediately. Same root cause, different column.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn dos_229_sentiment_update_visible_to_readers() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("dos229_sentiment.db");
        let svc = DbService::open_at_unencrypted(path)
            .await
            .expect("open svc");

        let mut email = sample_email("em-dos229-2", "acc-1");
        email.sentiment = Some("neutral".to_string());
        svc.writer()
            .call(move |conn| {
                let db = ActionDb::from_conn(conn);
                db.upsert_email(&email).expect("upsert");
                Ok(())
            })
            .await
            .expect("writer call");

        // Prime readers.
        for _ in 0..(NUM_READERS * 2) {
            let r = svc.reader();
            r.call(|conn| {
                let db = ActionDb::from_conn(conn);
                let _ = db.get_all_active_emails().expect("read");
                Ok(())
            })
            .await
            .expect("reader call");
        }

        // Update sentiment through the writer using a raw UPDATE.
        svc.writer()
            .call(|conn| {
                conn.execute(
                    "UPDATE emails SET sentiment = ?1, updated_at = ?2 WHERE email_id = ?3",
                    rusqlite::params!["positive", chrono::Utc::now().to_rfc3339(), "em-dos229-2"],
                )?;
                Ok(())
            })
            .await
            .expect("writer call");

        for i in 0..(NUM_READERS * 2) {
            let r = svc.reader();
            let rows = r
                .call(|conn| {
                    let db = ActionDb::from_conn(conn);
                    Ok(db.get_all_active_emails().expect("read"))
                })
                .await
                .expect("reader call");
            assert_eq!(rows.len(), 1, "iteration {i}");
            assert_eq!(
                rows[0].sentiment.as_deref(),
                Some("positive"),
                "iteration {i}: reader returned stale sentiment"
            );
        }
    }
}
