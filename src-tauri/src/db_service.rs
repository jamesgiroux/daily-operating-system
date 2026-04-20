//! Unified async/sync database connection pool (DOS-* DbService refactor).
//!
//! Single source of truth for all DB access in the process. Replaces the old
//! dual model (tokio_rusqlite async pool + `ActionDb::open()` fresh-opens)
//! that caused WAL races under SQLCipher: two connections reading the same
//! mid-commit WAL frame would trigger HMAC verification failures ("file is
//! not a database") because each fresh `rusqlite::Connection::open()` gets
//! its own OS-level handle with no awareness of the pool writer's in-progress
//! commit stream.
//!
//! Architecture (ADR followup, not yet numbered):
//! - 1 writer connection and N readers, each owning a dedicated OS thread.
//! - `.call(|conn| ...).await` and `.call_sync(|conn| ...)` submit closures to
//!   the dedicated thread and await completion over channels.
//! - A process-wide `GLOBAL` singleton lets `ActionDb::open()` route through
//!   the pool instead of opening a fresh handle. If the pool is not yet
//!   initialized (startup, tests) `ActionDb::open()` falls back to the
//!   legacy fresh-open path.
//! - Fresh opens can also be serialized through `open_fresh_serialized` to avoid
//!   SQLCipher WAL read-verify races on `Connection::open()` verification.

use std::any::Any;
use std::path::PathBuf;
use std::panic::{self, AssertUnwindSafe};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex as StdMutex};
use std::thread;

use rusqlite::Connection;
use tokio::sync::oneshot;

use crate::db::DbError;

/// Number of read connections in the pool.
const NUM_READERS: usize = 2;

type CallResult = Result<Box<dyn Any + Send>, PooledCallError>;
type WorkerTask =
    Box<dyn FnOnce(&mut Connection) -> rusqlite::Result<Box<dyn Any + Send>> + Send + 'static>;

enum CallMessage {
    Async {
        task: WorkerTask,
        respond_to: oneshot::Sender<CallResult>,
    },
    Sync {
        task: WorkerTask,
        respond_to: mpsc::Sender<CallResult>,
    },
    Shutdown,
}

#[derive(Debug, thiserror::Error)]
pub enum PooledCallError {
    #[error("{0}")]
    Rusqlite(#[from] rusqlite::Error),
    #[error("pooled call result type mismatch")]
    TypeMismatch,
    #[error("pooled connection unavailable")]
    Closed,
    #[error("pooled call panicked: {0}")]
    Panic(String),
}

/// Shared worker internals.
struct PooledConnectionInner {
    sender: mpsc::Sender<CallMessage>,
    handle: StdMutex<Option<std::thread::JoinHandle<()>>>,
}

impl PooledConnectionInner {
    fn shutdown(&self) {
        let _ = self.sender.send(CallMessage::Shutdown);
        let mut handle = self.handle.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(handle) = handle.take() {
            let _ = handle.join();
        }
    }
}

/// A pooled connection handle. Clone-cheap (it's just an Arc under the hood).
#[derive(Clone)]
pub struct PooledConnection {
    inner: Arc<PooledConnectionInner>,
}

fn panic_to_string(payload: Box<dyn Any + Send>) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_string()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "pooled call panicked".to_string()
    }
}

fn run_task(task: WorkerTask, conn: &mut Connection) -> CallResult {
    match panic::catch_unwind(AssertUnwindSafe(|| task(conn))) {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(error)) => Err(PooledCallError::Rusqlite(error)),
        Err(payload) => Err(PooledCallError::Panic(panic_to_string(payload))),
    }
}

impl PooledConnection {
    fn new(conn: Connection) -> Result<Self, DbError> {
        let (sender, receiver) = mpsc::channel();
        let handle = thread::Builder::new()
            .name("dailyos-db-connection".to_string())
            .spawn(move || {
                let mut conn = conn;
                while let Ok(message) = receiver.recv() {
                    match message {
                        CallMessage::Async { task, respond_to } => {
                            let _ = respond_to.send(run_task(task, &mut conn));
                        }
                        CallMessage::Sync { task, respond_to } => {
                            let _ = respond_to.send(run_task(task, &mut conn));
                        }
                        CallMessage::Shutdown => {
                            break;
                        }
                    }
                }
            })
            .map_err(|e| DbError::Migration(format!("failed to start DB worker thread: {e}")))?;

        Ok(Self {
            inner: Arc::new(PooledConnectionInner {
                sender,
                handle: StdMutex::new(Some(handle)),
            }),
        })
    }

    fn split_payload<T: Send + 'static>(
        payload: CallResult,
    ) -> Result<T, PooledCallError> {
        let payload = payload?;
        payload
            .downcast::<T>()
            .map(|value| *value)
            .map_err(|_| PooledCallError::TypeMismatch)
    }

    /// Async call — submits closure to the dedicated thread.
    pub async fn call<F, T>(&self, f: F) -> Result<T, PooledCallError>
    where
        F: FnOnce(&mut Connection) -> rusqlite::Result<T> + Send + 'static,
        T: Send + 'static,
    {
        let (tx, rx) = oneshot::channel();
        let task: WorkerTask = Box::new(move |conn| f(conn).map(|value| Box::new(value) as Box<_>));
        self.inner
            .sender
            .send(CallMessage::Async {
                task,
                respond_to: tx,
            })
            .map_err(|_| PooledCallError::Closed)?;
        Self::split_payload(rx.await.map_err(|_| PooledCallError::Closed)?)
    }

    /// Sync call — submits closure to the dedicated thread and blocks on the
    /// response channel. Intended for sync startup/background paths.
    pub fn call_sync<F, T>(&self, f: F) -> Result<T, PooledCallError>
    where
        F: FnOnce(&mut Connection) -> rusqlite::Result<T> + Send + 'static,
        T: Send + 'static,
    {
        let (tx, rx) = mpsc::channel();
        let task: WorkerTask = Box::new(move |conn| f(conn).map(|value| Box::new(value) as Box<_>));
        self.inner
            .sender
            .send(CallMessage::Sync {
                task,
                respond_to: tx,
            })
            .map_err(|_| PooledCallError::Closed)?;
        Self::split_payload(rx.recv().map_err(|_| PooledCallError::Closed)?)
    }

    pub(crate) fn shutdown(&self) {
        self.inner.shutdown();
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

/// Open a fresh encrypted connection on the same validation semantics as
/// `ActionDb::open` (key, verification query, WAL/busy/sync setup).
fn open_encrypted_fresh(path: &str, hex_key: &str, read_only: bool) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    conn.execute_batch(&crate::db::encryption::key_to_pragma(hex_key))?;
    conn.query_row("SELECT count(*) FROM sqlite_master LIMIT 1", [], |row| {
        row.get::<_, i64>(0)
    })?;
    apply_pragmas(&conn, read_only, hex_key)?;
    Ok(conn)
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

        // Build the writer on a blocking thread so open + migrations
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
            readers.push(PooledConnection::new(r)?);
        }

        Ok(Arc::new(Self {
            writer: PooledConnection::new(writer)?,
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
            readers.push(PooledConnection::new(r)?);
        }

        Ok(Arc::new(Self {
            writer: PooledConnection::new(writer)?,
            readers,
            read_idx: AtomicUsize::new(0),
        }))
    }

    /// Open a fresh encrypted connection through the writer thread so SQLCipher
    /// verification executes in series with WAL writes.
    pub fn open_fresh_serialized(
        &self,
        path: PathBuf,
        hex_key: String,
    ) -> Result<Connection, DbError> {
        let path = path.to_string_lossy().to_string();
        let result = self.writer.call_sync(move |_| open_encrypted_fresh(&path, &hex_key, false));
        match result {
            Ok(conn) => Ok(conn),
            Err(PooledCallError::Rusqlite(error)) => Err(DbError::Sqlite(error)),
            Err(PooledCallError::Closed) => {
                Err(DbError::Migration("pooled writer thread not available".to_string()))
            }
            Err(PooledCallError::Panic(message)) => {
                Err(DbError::Migration(format!("open_fresh_serialized panic: {message}")))
            }
            Err(PooledCallError::TypeMismatch) => {
                Err(DbError::Migration(
                    "open_fresh_serialized result type mismatch".to_string(),
                ))
            }
        }
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

impl Drop for DbService {
    fn drop(&mut self) {
        self.writer.shutdown();
        for reader in &self.readers {
            reader.shutdown();
        }
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

    #[cfg(target_os = "macos")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn dos_229_sqlcipher_open_fresh_serialized_no_notadb() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("fresh_open_fallback.db");

        let svc = DbService::open_at(path.clone())
            .await
            .expect("open svc");
        let hex_key = crate::db::encryption::get_or_create_db_key(&path)
            .expect("db key");

        let writer = svc.clone();
        let writer_task = tokio::task::spawn_blocking(move || -> Result<(), String> {
            for i in 0..500 {
                let email = sample_email(&format!("em-race-{i}"), "acc-race");
                writer
                    .writer()
                    .call_sync(move |conn| {
                        let db = ActionDb::from_conn(conn);
                        db.upsert_email(&email).expect("upsert");
                        Ok(())
                    })
                    .map_err(|e| e.to_string())?;
            }
            Ok(())
        });

        let mut open_tasks = Vec::with_capacity(200);
        for _ in 0..200 {
            let svc = svc.clone();
            let key = hex_key.clone();
            let path = path.clone();
            open_tasks.push(tokio::spawn(async move {
                svc.open_fresh_serialized(path.clone(), key).map_err(|e| e.to_string())
            }));
        }

        let mut notadb_errors = 0usize;
        for join in open_tasks {
            let opened = join.await.expect("join");
            if let Err(error) = opened {
                if error.contains("not a database") || error.contains("SQLITE_NOTADB") {
                    notadb_errors += 1;
                } else {
                    panic!("unexpected open error: {error}");
                }
            }
        }

        assert_eq!(
            notadb_errors, 0,
            "SQLCipher fresh-open race produced SQLITE_NOTADB"
        );

        writer_task.await.expect("writer task").expect("writer task error");
    }
}
