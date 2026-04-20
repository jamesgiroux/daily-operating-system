//! Unified async/sync database connection pool (DOS-* DbService refactor).
//!
//! Single source of truth for all DB access in the process. Replaces the old
//! dual model (tokio_rusqlite async pool + `ActionDb::open()` fresh-opens)
//! that caused WAL races under SQLCipher: two connections reading the same
//! mid-commit WAL frame would trigger HMAC verification failures ("file is
//! not a database") because each fresh `rusqlite::Connection::open()` gets
//! its own OS-level handle with no awareness of the pool writer's
//! in-progress frame.
//!
//! Architecture (ADR followup, not yet numbered):
//! - 1 writer connection, wrapped in `Arc<parking_lot::Mutex<Connection>>`
//! - N reader connections, each wrapped in its own `Arc<Mutex<Connection>>`
//! - Both sync and async APIs share the same underlying connection:
//!     - `.call(|conn| ...).await` — matches prior tokio_rusqlite signature,
//!       runs via `spawn_blocking` so the Tokio runtime is never blocked.
//!     - `.call_sync(|conn| ...)` — locks the mutex on the calling thread,
//!       for sync paths like `ActionDb::open()` in background tasks.
//! - A process-wide `GLOBAL` singleton lets `ActionDb::open()` route through
//!   the pool instead of opening a fresh handle. If the pool is not yet
//!   initialized (startup, tests) `ActionDb::open()` falls back to the
//!   legacy fresh-open path.

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use parking_lot::Mutex;
use rusqlite::Connection;

use crate::db::DbError;

/// Number of read connections in the pool.
const NUM_READERS: usize = 2;

/// Shared handle type: Arc<Mutex<Connection>>.
pub type ConnArc = Arc<Mutex<Connection>>;

/// Error type for pool calls. Mirrors the shape callers expected from
/// `tokio_rusqlite::Error` (Rusqlite variant + generic other).
#[derive(Debug, thiserror::Error)]
pub enum PooledCallError {
    #[error("{0}")]
    Rusqlite(#[from] rusqlite::Error),
    #[error("spawn_blocking join error: {0}")]
    Join(String),
}

/// A pooled connection handle. Clone-cheap (it's just an Arc under the hood).
#[derive(Clone)]
pub struct PooledConnection {
    inner: ConnArc,
}

impl PooledConnection {
    fn new(conn: Connection) -> Self {
        Self {
            inner: Arc::new(Mutex::new(conn)),
        }
    }

    /// Expose the underlying `Arc<Mutex<Connection>>` for callers that need
    /// to lock directly (e.g. `ActionDb::open()` pooled checkout via
    /// `lock_arc()` → `ArcMutexGuard`).
    pub fn arc(&self) -> ConnArc {
        Arc::clone(&self.inner)
    }

    /// Async call — locks the mutex on a dedicated blocking thread.
    /// Signature matches the prior `tokio_rusqlite::Connection::call` so
    /// existing `.call(move |conn| { ... }).await` call sites compile
    /// unchanged.
    pub async fn call<F, T>(&self, f: F) -> Result<T, PooledCallError>
    where
        F: FnOnce(&mut Connection) -> rusqlite::Result<T> + Send + 'static,
        T: Send + 'static,
    {
        let arc = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || {
            let mut guard = arc.lock();
            f(&mut guard)
        })
        .await
        .map_err(|e| PooledCallError::Join(e.to_string()))?
        .map_err(PooledCallError::Rusqlite)
    }

    /// Sync call — locks the mutex on the calling thread. Intended for
    /// callers not in an async context (startup, background worker threads).
    /// Never call this from within a Tokio runtime — it blocks the runtime
    /// thread. Use `.call(...).await` from async code.
    pub fn call_sync<F, T>(&self, f: F) -> rusqlite::Result<T>
    where
        F: FnOnce(&mut Connection) -> rusqlite::Result<T>,
    {
        let mut guard = self.inner.lock();
        f(&mut guard)
    }
}

/// Apply standard pragmas to a connection. `read_only` adds `query_only=ON`.
/// PRAGMA key MUST be first for SQLCipher (ADR-0092).
fn apply_pragmas(
    conn: &Connection,
    read_only: bool,
    hex_key: &str,
) -> Result<(), rusqlite::Error> {
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

/// The service itself. Hold as `Arc<DbService>` and share freely.
pub struct DbService {
    writer: PooledConnection,
    readers: Vec<PooledConnection>,
    read_idx: AtomicUsize,
}

impl DbService {
    /// Open a DbService at the standard path.
    pub async fn open() -> Result<Arc<Self>, DbError> {
        let path = crate::db::ActionDb::db_path_public()?;
        Self::open_at(path).await
    }

    /// Open a DbService at an explicit path. Encrypted via SQLCipher.
    pub async fn open_at(path: PathBuf) -> Result<Arc<Self>, DbError> {
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent).map_err(DbError::CreateDir)?;
            }
        }

        let hex_key =
            crate::db::encryption::get_or_create_db_key(&path).map_err(DbError::Encryption)?;

        if path.exists() && crate::db::encryption::is_database_plaintext(&path) {
            log::info!("DbService: detected plaintext DB, migrating to encrypted...");
            crate::db::encryption::migrate_to_encrypted(&path, &hex_key)
                .map_err(DbError::Encryption)?;
        }

        let path_str = path.to_string_lossy().to_string();
        let path_for_readers = path_str.clone();
        let key_for_readers = hex_key.clone();

        // Build the writer on a blocking thread so the open + migrations
        // don't stall the Tokio runtime.
        let writer = tokio::task::spawn_blocking(move || -> Result<Connection, DbError> {
            let conn = Connection::open(&path_str)?;
            apply_pragmas(&conn, false, &hex_key)?;
            crate::migrations::run_migrations(&conn).map_err(DbError::Migration)?;
            Ok(conn)
        })
        .await
        .map_err(|e| DbError::Migration(format!("writer spawn join: {e}")))??;

        // Readers: no migrations, just pragmas + query_only.
        let mut readers = Vec::with_capacity(NUM_READERS);
        for _ in 0..NUM_READERS {
            let path_clone = path_for_readers.clone();
            let key_clone = key_for_readers.clone();
            let r = tokio::task::spawn_blocking(move || -> Result<Connection, DbError> {
                let conn = Connection::open(&path_clone)?;
                apply_pragmas(&conn, true, &key_clone)?;
                Ok(conn)
            })
            .await
            .map_err(|e| DbError::Migration(format!("reader spawn join: {e}")))??;
            readers.push(PooledConnection::new(r));
        }

        Ok(Arc::new(Self {
            writer: PooledConnection::new(writer),
            readers,
            read_idx: AtomicUsize::new(0),
        }))
    }

    /// Unencrypted variant used only by tests.
    #[cfg(test)]
    pub async fn open_at_unencrypted(path: PathBuf) -> Result<Arc<Self>, DbError> {
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent).map_err(DbError::CreateDir)?;
            }
        }
        let path_str = path.to_string_lossy().to_string();
        let path_for_readers = path_str.clone();

        let writer = tokio::task::spawn_blocking(move || -> Result<Connection, DbError> {
            let conn = Connection::open(&path_str)?;
            conn.execute_batch("PRAGMA journal_mode = WAL;")?;
            conn.execute_batch("PRAGMA busy_timeout = 5000;")?;
            conn.execute_batch("PRAGMA synchronous = NORMAL;")?;
            conn.execute_batch("PRAGMA foreign_keys = ON;")?;
            crate::migrations::run_migrations(&conn).map_err(DbError::Migration)?;
            Ok(conn)
        })
        .await
        .map_err(|e| DbError::Migration(format!("writer spawn join: {e}")))??;

        let mut readers = Vec::with_capacity(NUM_READERS);
        for _ in 0..NUM_READERS {
            let path_clone = path_for_readers.clone();
            let r = tokio::task::spawn_blocking(move || -> Result<Connection, DbError> {
                let conn = Connection::open(&path_clone)?;
                conn.execute_batch("PRAGMA journal_mode = WAL;")?;
                conn.execute_batch("PRAGMA busy_timeout = 5000;")?;
                conn.execute_batch("PRAGMA synchronous = NORMAL;")?;
                conn.execute_batch("PRAGMA foreign_keys = ON;")?;
                conn.execute_batch("PRAGMA query_only = ON;")?;
                Ok(conn)
            })
            .await
            .map_err(|e| DbError::Migration(format!("reader spawn join: {e}")))??;
            readers.push(PooledConnection::new(r));
        }

        Ok(Arc::new(Self {
            writer: PooledConnection::new(writer),
            readers,
            read_idx: AtomicUsize::new(0),
        }))
    }

    /// Writer connection. Serialized: one write at a time.
    pub fn writer(&self) -> &PooledConnection {
        &self.writer
    }

    /// Reader connection, round-robin. Concurrent reads under WAL.
    pub fn reader(&self) -> &PooledConnection {
        let idx = self.read_idx.fetch_add(1, Ordering::Relaxed) % self.readers.len();
        &self.readers[idx]
    }
}

// -----------------------------------------------------------------------
// Process-wide singleton so sync `ActionDb::open()` can route through the
// pool instead of opening a fresh handle.
// -----------------------------------------------------------------------

static GLOBAL: parking_lot::Mutex<Option<Arc<DbService>>> = parking_lot::Mutex::new(None);

/// Get a cloned Arc to the global DbService, if one is installed.
pub fn try_global() -> Option<Arc<DbService>> {
    GLOBAL.lock().clone()
}

/// Install (or replace) the global DbService. Called once from state init
/// and again on dev-mode transitions.
pub fn install_global(svc: Arc<DbService>) {
    *GLOBAL.lock() = Some(svc);
}

/// Remove the global DbService. Subsequent `ActionDb::open()` calls fall
/// back to the legacy fresh-open path until a new service is installed.
pub fn uninstall_global() {
    *GLOBAL.lock() = None;
}

#[cfg(test)]
mod tests {
    //! DOS-229 — verify that writes are immediately visible to subsequent
    //! reads through the long-lived reader pool. Without the fix, the
    //! `query_only=ON` reader connections could serve a stale WAL snapshot.
    use super::*;
    use crate::db::ActionDb;

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

        svc.writer()
            .call(|conn| {
                let db = ActionDb::from_conn(conn);
                db.update_email_entity("em-dos229-1", Some("acc-new"), Some("account"))
                    .expect("update");
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
            assert_eq!(rows.len(), 1, "iter {i}");
            assert_eq!(
                rows[0].entity_id.as_deref(),
                Some("acc-new"),
                "iter {i}: stale entity_id"
            );
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn sync_and_async_share_connection_state() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("sync_async.db");
        let svc = DbService::open_at_unencrypted(path)
            .await
            .expect("open svc");

        // Write via async API.
        let email = sample_email("em-sync-1", "acc-1");
        svc.writer()
            .call(move |conn| {
                let db = ActionDb::from_conn(conn);
                db.upsert_email(&email).expect("upsert");
                Ok(())
            })
            .await
            .expect("writer call");

        // Read via sync API on a blocking thread (simulating ActionDb::open
        // from a background worker).
        let reader = svc.reader().clone();
        let rows = tokio::task::spawn_blocking(move || -> Result<Vec<_>, String> {
            reader
                .call_sync(|conn| {
                    let db = ActionDb::from_conn(conn);
                    Ok(db.get_all_active_emails().expect("read"))
                })
                .map_err(|e| e.to_string())
        })
        .await
        .expect("join")
        .expect("sync read");

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].entity_id.as_deref(), Some("acc-1"));
    }
}
