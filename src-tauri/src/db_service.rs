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
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex as StdMutex};
use std::thread;

use rusqlite::Connection;
use tokio::sync::oneshot;

use crate::db::key_provider::{rekey_database_standalone, DbKeyProvider, EncryptionKey};
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
        #[allow(
            clippy::let_underscore_must_use,
            reason = "intentional best-effort discard; preserves existing non-blocking behavior"
        )]
        let _ = self.sender.send(CallMessage::Shutdown);
        let mut handle = self.handle.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(handle) = handle.take() {
            #[allow(
                clippy::let_underscore_must_use,
                reason = "intentional best-effort discard; preserves existing non-blocking behavior"
            )]
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
                            #[allow(clippy::let_underscore_must_use, reason = "intentional best-effort discard; preserves existing non-blocking behavior")]
                            let _ = respond_to.send(run_task(task, &mut conn));
                        }
                        CallMessage::Sync { task, respond_to } => {
                            #[allow(clippy::let_underscore_must_use, reason = "intentional best-effort discard; preserves existing non-blocking behavior")]
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

    fn split_payload<T: Send + 'static>(payload: CallResult) -> Result<T, PooledCallError> {
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
    encryption_key: &EncryptionKey,
) -> Result<(), rusqlite::Error> {
    conn.execute_batch(&encryption_key.to_pragma())?;
    conn.execute_batch("PRAGMA journal_mode = WAL;")?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;")?;
    conn.execute_batch("PRAGMA synchronous = NORMAL;")?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    if read_only {
        conn.execute_batch("PRAGMA query_only = ON;")?;
    }
    Ok(())
}

/// Open a fresh encrypted connection on the same initialization semantics as
/// `ActionDb::open` (key, verification query, WAL/busy/sync setup, migrations).
fn open_encrypted_fresh(
    path: &str,
    encryption_key: &EncryptionKey,
    read_only: bool,
) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    apply_pragmas(&conn, read_only, encryption_key)?;
    conn.query_row("SELECT count(*) FROM sqlite_master LIMIT 1", [], |row| {
        row.get::<_, i64>(0)
    })?;
    if !read_only {
        let has_accounts_table = conn
            .query_row(
                "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'accounts' LIMIT 1",
                [],
                |_| Ok(()),
            )
            .is_ok();
        if !has_accounts_table {
            crate::migrations::run_migrations_with_key(&conn, Some(encryption_key)).map_err(
                |e| rusqlite::Error::InvalidParameterName(format!("migration failed: {e}")),
            )?;
            conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        }
    }
    Ok(conn)
}

struct DbConnectionPool {
    writer: PooledConnection,
    readers: Vec<PooledConnection>,
}

impl DbConnectionPool {
    fn from_connections(writer: Connection, readers: Vec<Connection>) -> Result<Self, DbError> {
        let writer = PooledConnection::new(writer)?;
        let readers = readers
            .into_iter()
            .map(PooledConnection::new)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self { writer, readers })
    }

    fn open_existing(path: &Path, encryption_key: &EncryptionKey) -> Result<Self, DbError> {
        let path = path.to_string_lossy().to_string();
        let writer = open_encrypted_fresh(&path, encryption_key, false)?;
        let mut readers = Vec::with_capacity(NUM_READERS);
        for _ in 0..NUM_READERS {
            let conn = Connection::open(&path)?;
            apply_pragmas(&conn, true, encryption_key)?;
            readers.push(conn);
        }
        Self::from_connections(writer, readers)
    }

    fn shutdown(&self) {
        self.writer.shutdown();
        for reader in &self.readers {
            reader.shutdown();
        }
    }
}

/// The service itself. Hold as `Arc<DbService>` and share freely.
pub struct DbService {
    path: PathBuf,
    pool: parking_lot::RwLock<DbConnectionPool>,
    read_idx: AtomicUsize,
}

impl DbService {
    /// Open a DbService at the standard path.
    pub async fn open(key_provider: Arc<dyn DbKeyProvider>) -> Result<Arc<Self>, DbError> {
        let path = crate::db::ActionDb::db_path_public()?;
        Self::open_at(path, key_provider).await
    }

    /// Open a DbService at an explicit path. Encrypted via SQLCipher.
    pub async fn open_at(
        path: PathBuf,
        key_provider: Arc<dyn DbKeyProvider>,
    ) -> Result<Arc<Self>, DbError> {
        let path_for_writer = path.clone();
        let path_for_readers = path.to_string_lossy().to_string();
        let writer_key_provider = key_provider.clone();

        // Build the writer on a blocking thread so filesystem checks,
        // Keychain access, plaintext migration, open, and migrations do not
        // stall the Tokio runtime.
        let (writer, hex_key) = tokio::task::spawn_blocking(move || {
            crate::db::ActionDb::open_encrypted_connection(path_for_writer, writer_key_provider)
        })
        .await
        .map_err(|e| DbError::Migration(format!("writer spawn join: {e}")))??;

        let key_for_readers = hex_key.clone();

        // Readers: no migrations, just pragmas + query_only.
        let mut reader_conns = Vec::with_capacity(NUM_READERS);
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
            reader_conns.push(r);
        }

        let pool = DbConnectionPool::from_connections(writer, reader_conns)?;
        Ok(Arc::new(Self {
            path,
            pool: parking_lot::RwLock::new(pool),
            read_idx: AtomicUsize::new(0),
        }))
    }

    /// Unencrypted variant used by test harnesses that need `AppState`
    /// without touching the user's encrypted DailyOS database or Keychain.
    #[cfg(feature = "test-harness")]
    #[doc(hidden)]
    pub async fn open_at_unencrypted_for_tests(path: PathBuf) -> Result<Arc<Self>, DbError> {
        Self::open_at_unencrypted_test_impl(path).await
    }

    #[cfg(any(test, feature = "test-harness"))]
    async fn open_at_unencrypted_test_impl(path: PathBuf) -> Result<Arc<Self>, DbError> {
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

        let mut reader_conns = Vec::with_capacity(NUM_READERS);
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
            reader_conns.push(r);
        }

        let pool = DbConnectionPool::from_connections(writer, reader_conns)?;
        Ok(Arc::new(Self {
            path,
            pool: parking_lot::RwLock::new(pool),
            read_idx: AtomicUsize::new(0),
        }))
    }

    /// Unencrypted variant used only by unit tests.
    #[cfg(test)]
    pub async fn open_at_unencrypted(path: PathBuf) -> Result<Arc<Self>, DbError> {
        Self::open_at_unencrypted_test_impl(path).await
    }

    /// Open a fresh encrypted connection through the writer thread so SQLCipher
    /// verification executes in series with WAL writes.
    pub fn open_fresh_serialized(
        &self,
        path: PathBuf,
        encryption_key: EncryptionKey,
    ) -> Result<Connection, DbError> {
        let path = path.to_string_lossy().to_string();
        let writer = self.writer();
        let result = writer.call_sync(move |_| open_encrypted_fresh(&path, &encryption_key, false));
        match result {
            Ok(conn) => Ok(conn),
            Err(PooledCallError::Rusqlite(error)) => Err(DbError::Sqlite(error)),
            Err(PooledCallError::Closed) => Err(DbError::Migration(
                "pooled writer thread not available".to_string(),
            )),
            Err(PooledCallError::Panic(message)) => Err(DbError::Migration(format!(
                "open_fresh_serialized panic: {message}"
            ))),
            Err(PooledCallError::TypeMismatch) => Err(DbError::Migration(
                "open_fresh_serialized result type mismatch".to_string(),
            )),
        }
    }

    pub fn db_path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn rekey_database(
        &self,
        db_path: &Path,
        old_key: &EncryptionKey,
        new_key: &EncryptionKey,
    ) -> Result<(), String> {
        if self.path != db_path {
            return Err(format!(
                "DbService rotation path mismatch: service={}, requested={}",
                self.path.display(),
                db_path.display()
            ));
        }

        let mut pool = self.pool.write();
        pool.shutdown();

        match rekey_database_standalone(db_path, old_key, new_key) {
            Ok(()) => match DbConnectionPool::open_existing(db_path, new_key) {
                Ok(new_pool) => {
                    *pool = new_pool;
                    self.read_idx.store(0, Ordering::Relaxed);
                    Ok(())
                }
                Err(reopen_new_error) => {
                    let rollback = rekey_database_standalone(db_path, new_key, old_key);
                    match rollback {
                        Ok(()) => match DbConnectionPool::open_existing(db_path, old_key) {
                            Ok(old_pool) => {
                                *pool = old_pool;
                                self.read_idx.store(0, Ordering::Relaxed);
                                Err(format!(
                                    "DB rekey succeeded but reopening DbService with the new key failed: {reopen_new_error}; rollback to original key succeeded"
                                ))
                            }
                            Err(reopen_old_error) => Err(format!(
                                "DB rekey succeeded but reopening DbService with the new key failed: {reopen_new_error}; rollback to original key succeeded but reopening the original pool failed: {reopen_old_error}"
                            )),
                        },
                        Err(rollback_error) => Err(format!(
                            "DB rekey succeeded but reopening DbService with the new key failed: {reopen_new_error}; rollback to original key failed: {rollback_error}"
                        )),
                    }
                }
            },
            Err(rekey_error) => match DbConnectionPool::open_existing(db_path, old_key) {
                Ok(old_pool) => {
                    *pool = old_pool;
                    self.read_idx.store(0, Ordering::Relaxed);
                    Err(rekey_error)
                }
                Err(reopen_old_error) => Err(format!(
                    "{rekey_error}; failed to reopen DbService with the original key after failed rotation: {reopen_old_error}"
                )),
            },
        }
    }

    /// Writer connection. Serialized: one write at a time.
    pub fn writer(&self) -> PooledConnection {
        self.pool.read().writer.clone()
    }

    /// Reader connection, round-robin. Concurrent reads under WAL.
    pub fn reader(&self) -> PooledConnection {
        let pool = self.pool.read();
        let idx = self.read_idx.fetch_add(1, Ordering::Relaxed) % pool.readers.len();
        pool.readers[idx].clone()
    }
}

impl Drop for DbService {
    fn drop(&mut self) {
        self.pool.write().shutdown();
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
    //! verify that writes are immediately visible to subsequent
    //! reads through the long-lived reader pool. Without the fix, the
    //! `query_only=ON` reader connections could serve a stale WAL snapshot.
    use super::*;
    use crate::db::{ActionDb, DbKeyProvider, LocalKeychain, UserIdentity};
    use parking_lot::Mutex;
    use std::sync::mpsc;
    use std::time::Duration;

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
            to_recipients: None,
            cc_recipients: None,
        }
    }

    struct GetBlocker {
        key_fetched: mpsc::Sender<()>,
        release_get: mpsc::Receiver<()>,
    }

    struct RotatingFixtureKeyProvider {
        current: Mutex<EncryptionKey>,
        next: EncryptionKey,
        block_next_get: Mutex<Option<GetBlocker>>,
    }

    impl RotatingFixtureKeyProvider {
        fn new(current: &str, next: &str) -> Self {
            Self {
                current: Mutex::new(EncryptionKey::from_hex(current.to_string())),
                next: EncryptionKey::from_hex(next.to_string()),
                block_next_get: Mutex::new(None),
            }
        }

        fn block_next_get(&self, key_fetched: mpsc::Sender<()>, release_get: mpsc::Receiver<()>) {
            *self.block_next_get.lock() = Some(GetBlocker {
                key_fetched,
                release_get,
            });
        }
    }

    impl DbKeyProvider for RotatingFixtureKeyProvider {
        fn get_or_create_key(
            &self,
            _user: &UserIdentity,
        ) -> crate::db::key_provider::Result<EncryptionKey> {
            let key = self.current.lock().clone();
            let blocker = self.block_next_get.lock().take();
            if let Some(blocker) = blocker {
                blocker.key_fetched.send(()).expect("signal key fetched");
                blocker
                    .release_get
                    .recv()
                    .expect("wait for get_or_create release");
            }
            Ok(key)
        }

        fn rotate_key(
            &self,
            user: &UserIdentity,
        ) -> crate::db::key_provider::Result<EncryptionKey> {
            let _rotation_lock = crate::db::key_provider::rotation_lock_write();
            let mut current = self.current.lock();
            crate::db::key_provider::rekey_database(user.db_path(), &current, &self.next)?;
            *current = self.next.clone();
            Ok(current.clone())
        }
    }

    struct GlobalServiceGuard;

    impl Drop for GlobalServiceGuard {
        fn drop(&mut self) {
            uninstall_global();
        }
    }

    fn encrypted_db_can_read(path: &std::path::Path, key: &EncryptionKey) -> bool {
        let Ok(conn) = Connection::open(path) else {
            return false;
        };
        if conn.execute_batch(&key.to_pragma()).is_err() {
            return false;
        }
        conn.query_row("SELECT count(*) FROM sqlite_master LIMIT 1", [], |row| {
            row.get::<_, i64>(0)
        })
        .is_ok()
    }

    #[cfg(target_os = "macos")]
    fn seed_sqlcipher_key_for_keychainless_tests() {
        crate::db::encryption::set_cached_db_key_for_tests(
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        );
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

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn key_rotation_reopens_active_db_service_pool() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("pool_rotation.db");
        let old_key = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let new_key = "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";
        let provider = Arc::new(RotatingFixtureKeyProvider::new(old_key, new_key));
        let svc = DbService::open_at(path.clone(), provider.clone())
            .await
            .expect("open svc");

        let email = sample_email("em-rotate-before", "acc-before");
        svc.writer()
            .call(move |conn| {
                let db = ActionDb::from_conn(conn);
                db.upsert_email(&email).expect("upsert before rotation");
                Ok(())
            })
            .await
            .expect("writer call before rotation");

        install_global(svc.clone());
        let _global_guard = GlobalServiceGuard;
        let rotated = provider
            .rotate_key(&UserIdentity::local(path.clone()))
            .expect("rotate through global DbService");
        assert_eq!(rotated, EncryptionKey::from_hex(new_key.to_string()));
        uninstall_global();

        let email = sample_email("em-rotate-after", "acc-after");
        svc.writer()
            .call(move |conn| {
                let db = ActionDb::from_conn(conn);
                db.upsert_email(&email).expect("upsert after rotation");
                Ok(())
            })
            .await
            .expect("writer call after rotation");

        let rows = svc
            .reader()
            .call(|conn| {
                let db = ActionDb::from_conn(conn);
                Ok(db.get_all_active_emails().expect("read after rotation"))
            })
            .await
            .expect("reader call after rotation");
        assert_eq!(rows.len(), 2);
        assert!(!encrypted_db_can_read(
            &path,
            &EncryptionKey::from_hex(old_key.to_string())
        ));
        assert!(encrypted_db_can_read(
            &path,
            &EncryptionKey::from_hex(new_key.to_string())
        ));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn action_db_open_key_fetch_and_fresh_open_are_rotation_atomic() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("open_rotation_atomic.db");
        let old_key = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let new_key = "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";
        let provider = Arc::new(RotatingFixtureKeyProvider::new(old_key, new_key));
        let svc = DbService::open_at(path.clone(), provider.clone())
            .await
            .expect("open svc");

        install_global(svc);
        let _global_guard = GlobalServiceGuard;

        let (key_fetched_tx, key_fetched_rx) = mpsc::channel();
        let (release_get_tx, release_get_rx) = mpsc::channel();
        provider.block_next_get(key_fetched_tx, release_get_rx);

        let open_provider = provider.clone();
        let open_path = path.clone();
        let open_handle = std::thread::spawn(move || {
            let db = ActionDb::open_resolved_path_for_tests(open_path, open_provider)?;
            drop(db);
            Ok::<(), DbError>(())
        });

        key_fetched_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("open fetched key before rotation attempt");

        let rotate_provider = provider.clone();
        let rotate_user = UserIdentity::local(path.clone());
        let (rotation_started_tx, rotation_started_rx) = mpsc::channel();
        let (rotation_done_tx, rotation_done_rx) = mpsc::channel();
        let rotate_handle = std::thread::spawn(move || {
            rotation_started_tx
                .send(())
                .expect("signal rotation started");
            let result = rotate_provider.rotate_key(&rotate_user);
            rotation_done_tx.send(()).expect("signal rotation done");
            result
        });

        rotation_started_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("rotation thread started");
        assert!(
            rotation_done_rx
                .recv_timeout(Duration::from_millis(100))
                .is_err(),
            "rotation completed while ActionDb::open held a fetched key"
        );

        release_get_tx.send(()).expect("release blocked key fetch");
        open_handle
            .join()
            .expect("open thread joined")
            .expect("open should complete with the pre-rotation key");

        let rotated = rotate_handle
            .join()
            .expect("rotation thread joined")
            .expect("rotation completed after open connection acquisition");
        assert_eq!(rotated, EncryptionKey::from_hex(new_key.to_string()));
        assert!(!encrypted_db_can_read(
            &path,
            &EncryptionKey::from_hex(old_key.to_string())
        ));
        assert!(encrypted_db_can_read(
            &path,
            &EncryptionKey::from_hex(new_key.to_string())
        ));
    }

    #[cfg(target_os = "macos")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn dos_229_sqlcipher_open_fresh_serialized_no_notadb() {
        seed_sqlcipher_key_for_keychainless_tests();

        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("fresh_open_fallback.db");

        let provider = Arc::new(LocalKeychain::new());
        let svc = DbService::open_at(path.clone(), provider.clone())
            .await
            .expect("open svc");
        let user = UserIdentity::local(path.clone());
        let encryption_key = provider.get_or_create_key(&user).expect("db key");

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
            let key = encryption_key.clone();
            let path = path.clone();
            open_tasks.push(tokio::spawn(async move {
                svc.open_fresh_serialized(path.clone(), key)
                    .map_err(|e| e.to_string())
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

        writer_task
            .await
            .expect("writer task")
            .expect("writer task error");
    }

    #[cfg(target_os = "macos")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn open_fresh_serialized_initializes_schema_for_new_path() {
        seed_sqlcipher_key_for_keychainless_tests();

        let dir = tempfile::tempdir().expect("tempdir");
        let service_path = dir.path().join("service.db");
        let fresh_path = dir.path().join("fresh_missing_schema.db");

        let provider = Arc::new(LocalKeychain::new());
        let svc = DbService::open_at(service_path, provider.clone())
            .await
            .expect("open svc");
        let user = UserIdentity::local(fresh_path.clone());
        let encryption_key = provider.get_or_create_key(&user).expect("db key");

        let conn = svc
            .open_fresh_serialized(fresh_path, encryption_key)
            .expect("fresh serialized open");

        let account_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM accounts", [], |row| row.get(0))
            .expect("accounts table should exist");
        assert_eq!(account_count, 0);
    }
}
