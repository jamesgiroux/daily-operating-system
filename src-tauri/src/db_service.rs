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
use std::fmt;
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex as StdMutex};
use std::thread;
use std::time::Instant;

use rusqlite::Connection;
use tokio::sync::oneshot;

#[cfg(test)]
use crate::db::key_provider::UserIdentity;
use crate::db::key_provider::{rekey_database_standalone, DbKeyProvider, EncryptionKey};
use crate::db::DbError;

/// Number of read connections in the pool.
///
/// Sized to the N+1 latency-tier rule: foreground UI / foreground sync command /
/// background task / maintenance = 4 tiers. W0-B introduces the count without
/// tier ownership; W1-D adds strict ownership if telemetry shows wrong-tier
/// routing as a residual bottleneck. See ADR-0134 (reader pool sizing).
const NUM_READERS: usize = 4;
const DB_QUEUE_LATENCY_BUDGET_MS: u128 = 100;
const DB_EXECUTION_LATENCY_BUDGET_MS: u128 = 250;

/// Target WAL bytes before SQLite truncates via autocheckpoint. 200 frames ≈
/// 800 KB at 4 KB pages — small enough that readers don't walk a fat frame
/// index, large enough that a single enrichment burst doesn't thrash truncate.
const WAL_AUTOCHECKPOINT_FRAMES: i64 = 200;

/// Target mmap window for the SQLite page cache. Reduces userspace copy cost
/// on reads when the OS can serve pages from the unified buffer cache. Probed
/// post-set; if SQLCipher silently disables mmap the open fails so we don't
/// quietly run without the speedup (see ADR-0092 SQLCipher compatibility).
const MMAP_TARGET_BYTES: i64 = 268_435_456; // 256 MB

/// Negative cache_size means kibibytes (positive would mean pages). -64 MB
/// gives readers room to keep hot pages resident without ballooning RSS.
const CACHE_SIZE_KIB: i64 = -65_536;

/// Cap WAL growth even if a long-running writer prevents PASSIVE checkpoint
/// from truncating. Defends against the 18+ MB WAL bloat measured pre-W0.
const JOURNAL_SIZE_LIMIT_BYTES: i64 = 67_108_864; // 64 MB

/// Interval between PASSIVE checkpoints on the writer thread. 30 s is the
/// canonical SQLite forum recommendation under continuous write load — short
/// enough to keep WAL from accumulating mid-burst, long enough to avoid
/// thrashing truncate during user-active windows.
const WAL_CHECKPOINT_INTERVAL_SECS: u64 = 30;

type CallResult = Result<Box<dyn Any + Send>, PooledCallError>;
type WorkerTask =
    Box<dyn FnOnce(&mut Connection) -> rusqlite::Result<Box<dyn Any + Send>> + Send + 'static>;

enum CallMessage {
    Async {
        label: &'static str,
        tier_label: Option<&'static str>,
        enqueued_at: Instant,
        task: WorkerTask,
        respond_to: oneshot::Sender<CallResult>,
    },
    Sync {
        label: &'static str,
        tier_label: Option<&'static str>,
        enqueued_at: Instant,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DbAccessErrorClass {
    Retryable,
    Other,
}

#[derive(Debug)]
pub enum DbAccessError {
    Sqlite {
        context: Option<&'static str>,
        source: rusqlite::Error,
    },
    Other(String),
}

impl DbAccessError {
    pub fn db_read(error: PooledCallError) -> Self {
        Self::from_pooled_call("DB read error", error)
    }

    pub fn db_write(error: PooledCallError) -> Self {
        Self::from_pooled_call("DB write error", error)
    }

    fn from_pooled_call(context: &'static str, error: PooledCallError) -> Self {
        match error {
            PooledCallError::Rusqlite(source) => Self::Sqlite {
                context: Some(context),
                source,
            },
            other => Self::Other(format!("{context}: {other}")),
        }
    }

    pub fn class(&self) -> DbAccessErrorClass {
        match self.rusqlite_error() {
            Some(rusqlite::Error::SqliteFailure(sqlite_error, _))
                if matches!(
                    sqlite_error.code,
                    rusqlite::ErrorCode::DatabaseBusy
                        | rusqlite::ErrorCode::DatabaseLocked
                        | rusqlite::ErrorCode::NotADatabase
                ) =>
            {
                DbAccessErrorClass::Retryable
            }
            _ => DbAccessErrorClass::Other,
        }
    }

    pub fn is_retryable(&self) -> bool {
        self.class() == DbAccessErrorClass::Retryable
    }

    pub fn rusqlite_error(&self) -> Option<&rusqlite::Error> {
        match self {
            Self::Sqlite { source, .. } => Some(source),
            Self::Other(_) => None,
        }
    }
}

impl fmt::Display for DbAccessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sqlite {
                context: Some(context),
                source,
            } => write!(f, "{context}: {source}"),
            Self::Sqlite {
                context: None,
                source,
            } => write!(f, "{source}"),
            Self::Other(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for DbAccessError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Sqlite { source, .. } => Some(source),
            Self::Other(_) => None,
        }
    }
}

impl From<rusqlite::Error> for DbAccessError {
    fn from(source: rusqlite::Error) -> Self {
        Self::Sqlite {
            context: None,
            source,
        }
    }
}

impl From<String> for DbAccessError {
    fn from(message: String) -> Self {
        Self::Other(message)
    }
}

impl From<&str> for DbAccessError {
    fn from(message: &str) -> Self {
        Self::Other(message.to_string())
    }
}

impl From<DbAccessError> for String {
    fn from(error: DbAccessError) -> Self {
        error.to_string()
    }
}

/// Slot identity for telemetry. The split lets the W1 hard gate route between
/// W2 (writer-side dominant) and WX (reader-CPU dominant) instead of guessing
/// which side of the pool is saturated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SlotKind {
    Writer,
    Reader,
}

impl SlotKind {
    fn as_label(self) -> &'static str {
        match self {
            Self::Writer => "writer",
            Self::Reader => "reader",
        }
    }
}

/// Shared worker internals.
struct PooledConnectionInner {
    sender: mpsc::Sender<CallMessage>,
    handle: StdMutex<Option<std::thread::JoinHandle<()>>>,
    slot_kind: SlotKind,
    /// Optional caller-supplied tier annotation (e.g. "foreground_ui",
    /// "background"). When set, latency samples carry an extra
    /// `{label}.{phase}.tier.{tier}` rollup so W1-D's earn signal can
    /// distinguish wrong-tier routing from genuine pool saturation.
    tier_label: Option<&'static str>,
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

fn record_worker_latency(
    label: &'static str,
    phase: &str,
    slot_kind: SlotKind,
    tier_label: Option<&'static str>,
    elapsed_ms: u128,
    budget_ms: u128,
) {
    // Base rollup: backwards-compatible with existing dashboards.
    crate::latency::record_latency(&format!("{label}.{phase}"), elapsed_ms, budget_ms);
    // Writer/reader split: surface which side of the pool is saturated so the
    // W1 hard gate can route between W2 (writer-side dominant) and WX
    // (reader-CPU dominant).
    crate::latency::record_latency(
        &format!("{label}.{phase}.{}", slot_kind.as_label()),
        elapsed_ms,
        budget_ms,
    );
    // Tier-of-origin (optional): only emitted when the caller used a
    // tier-labeled accessor. Lets W1-D distinguish "foreground used a slot
    // also serving background" from "all slots saturated."
    if let Some(tier) = tier_label {
        crate::latency::record_latency(
            &format!("{label}.{phase}.tier.{tier}"),
            elapsed_ms,
            budget_ms,
        );
    }
}

fn run_timed_task(
    label: &'static str,
    slot_kind: SlotKind,
    tier_label: Option<&'static str>,
    enqueued_at: Instant,
    task: WorkerTask,
    conn: &mut Connection,
) -> CallResult {
    record_worker_latency(
        label,
        "queue_wait",
        slot_kind,
        tier_label,
        enqueued_at.elapsed().as_millis(),
        DB_QUEUE_LATENCY_BUDGET_MS,
    );
    let started = Instant::now();
    let result = run_task(task, conn);
    record_worker_latency(
        label,
        "execution",
        slot_kind,
        tier_label,
        started.elapsed().as_millis(),
        DB_EXECUTION_LATENCY_BUDGET_MS,
    );
    result
}

impl PooledConnection {
    fn new(conn: Connection, slot_kind: SlotKind) -> Result<Self, DbError> {
        let (sender, receiver) = mpsc::channel();
        let handle = thread::Builder::new()
            .name(format!("dailyos-db-{}", slot_kind.as_label()))
            .spawn(move || {
                let mut conn = conn;
                while let Ok(message) = receiver.recv() {
                    match message {
                        CallMessage::Async {
                            label,
                            tier_label,
                            enqueued_at,
                            task,
                            respond_to,
                        } => {
                            #[allow(clippy::let_underscore_must_use, reason = "intentional best-effort discard; preserves existing non-blocking behavior")]
                            let _ = respond_to.send(run_timed_task(label, slot_kind, tier_label, enqueued_at, task, &mut conn));
                        }
                        CallMessage::Sync {
                            label,
                            tier_label,
                            enqueued_at,
                            task,
                            respond_to,
                        } => {
                            #[allow(clippy::let_underscore_must_use, reason = "intentional best-effort discard; preserves existing non-blocking behavior")]
                            let _ = respond_to.send(run_timed_task(label, slot_kind, tier_label, enqueued_at, task, &mut conn));
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
                slot_kind,
                tier_label: None,
            }),
        })
    }

    /// Return a cheap clone of this connection annotated with a tier label.
    /// Subsequent `call*` invocations record latency under
    /// `{label}.{phase}.tier.{tier}` in addition to the base and slot rollups.
    ///
    /// Pre-W1-D this is opt-in: only the highest-frequency foreground call
    /// sites take a tier annotation, which is enough for the W1 hard gate to
    /// distinguish "tier confusion" from "pool saturation." Universal
    /// migration (197 sites) is W1-D's earn signal, not W0-B's.
    pub fn with_tier(&self, tier: &'static str) -> Self {
        Self {
            inner: Arc::new(PooledConnectionInner {
                sender: self.inner.sender.clone(),
                handle: StdMutex::new(None),
                slot_kind: self.inner.slot_kind,
                tier_label: Some(tier),
            }),
        }
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
        self.call_labeled("db.call", f).await
    }

    /// Async call with a stable PII-free latency label.
    pub async fn call_labeled<F, T>(&self, label: &'static str, f: F) -> Result<T, PooledCallError>
    where
        F: FnOnce(&mut Connection) -> rusqlite::Result<T> + Send + 'static,
        T: Send + 'static,
    {
        let (tx, rx) = oneshot::channel();
        let task: WorkerTask = Box::new(move |conn| f(conn).map(|value| Box::new(value) as Box<_>));
        self.inner
            .sender
            .send(CallMessage::Async {
                label,
                tier_label: self.inner.tier_label,
                enqueued_at: Instant::now(),
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
        self.call_sync_labeled("db.call_sync", f)
    }

    /// Sync call with a stable PII-free latency label.
    pub fn call_sync_labeled<F, T>(&self, label: &'static str, f: F) -> Result<T, PooledCallError>
    where
        F: FnOnce(&mut Connection) -> rusqlite::Result<T> + Send + 'static,
        T: Send + 'static,
    {
        let (tx, rx) = mpsc::channel();
        let task: WorkerTask = Box::new(move |conn| f(conn).map(|value| Box::new(value) as Box<_>));
        self.inner
            .sender
            .send(CallMessage::Sync {
                label,
                tier_label: self.inner.tier_label,
                enqueued_at: Instant::now(),
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
///
/// W0-A adds WAL throughput pragmas: `wal_autocheckpoint`, `mmap_size`,
/// `cache_size`, `journal_size_limit`. The `mmap_size` setting is probed
/// post-set and fails loud on 0 — SQLCipher silently disables mmap on builds
/// without `SQLITE_ENABLE_MMAP_SIZE`, and we want that surfaced at open rather
/// than discovered as missing read-path acceleration during a beachball.
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
    conn.execute_batch(&format!(
        "PRAGMA wal_autocheckpoint = {WAL_AUTOCHECKPOINT_FRAMES};"
    ))?;
    conn.execute_batch(&format!("PRAGMA mmap_size = {MMAP_TARGET_BYTES};"))?;
    conn.execute_batch(&format!("PRAGMA cache_size = {CACHE_SIZE_KIB};"))?;
    conn.execute_batch(&format!(
        "PRAGMA journal_size_limit = {JOURNAL_SIZE_LIMIT_BYTES};"
    ))?;
    let mmap_size: i64 = conn.query_row("PRAGMA mmap_size;", [], |row| row.get(0))?;
    if mmap_size == 0 {
        return Err(rusqlite::Error::InvalidParameterName(format!(
            "PRAGMA mmap_size returned 0 after setting {MMAP_TARGET_BYTES} — \
             SQLCipher build does not support mmap. Rebuild with \
             SQLITE_ENABLE_MMAP_SIZE or relax the W0-A mmap requirement."
        )));
    }
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
        let writer = PooledConnection::new(writer, SlotKind::Writer)?;
        let readers = readers
            .into_iter()
            .map(|conn| PooledConnection::new(conn, SlotKind::Reader))
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

#[cfg(test)]
struct FixtureDbServiceKeyProvider {
    key: EncryptionKey,
}

#[cfg(test)]
impl FixtureDbServiceKeyProvider {
    fn new() -> Self {
        Self {
            key: EncryptionKey::from_hex(
                "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
            ),
        }
    }
}

#[cfg(test)]
impl DbKeyProvider for FixtureDbServiceKeyProvider {
    fn get_or_create_key(
        &self,
        _user: &UserIdentity,
    ) -> crate::db::key_provider::Result<EncryptionKey> {
        Ok(self.key.clone())
    }

    fn rotate_key(&self, _user: &UserIdentity) -> crate::db::key_provider::Result<EncryptionKey> {
        Ok(self.key.clone())
    }
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
        crate::db::guard_path_for_mode(&path)?;

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
        let svc = Arc::new(Self {
            path,
            pool: parking_lot::RwLock::new(pool),
            read_idx: AtomicUsize::new(0),
        });
        // Restore long-window latency counters from the previous run before
        // any new samples land. Best-effort: missing rows / parse errors are
        // logged and we proceed with empty counters.
        hydrate_latency_snapshot_from_kv(&svc.reader()).await;
        Self::spawn_checkpoint_task(Arc::downgrade(&svc));
        Ok(svc)
    }

    /// Run `PRAGMA wal_checkpoint(PASSIVE)` on the writer thread every
    /// [`WAL_CHECKPOINT_INTERVAL_SECS`]. The task holds a `Weak<DbService>` so
    /// it exits cleanly when the last strong ref is dropped (DbService Drop ⇒
    /// pool shutdown ⇒ writer channel closed). PASSIVE is the right mode here
    /// because it never blocks readers or writers — if a writer is mid-commit,
    /// the call returns without truncating and we try again next interval.
    fn spawn_checkpoint_task(weak_svc: std::sync::Weak<DbService>) {
        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(std::time::Duration::from_secs(WAL_CHECKPOINT_INTERVAL_SECS));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            // First tick fires immediately; consume it so the first real
            // checkpoint happens one interval after open, not at t=0.
            interval.tick().await;
            let mut tick_counter: u32 = 0;
            loop {
                interval.tick().await;
                tick_counter = tick_counter.wrapping_add(1);
                let Some(svc) = weak_svc.upgrade() else {
                    log::debug!("db_service: checkpoint task exiting (DbService dropped)");
                    break;
                };
                let writer = svc.writer();
                drop(svc);
                let started = Instant::now();
                let result = writer
                    .call_labeled("db.wal_checkpoint_passive", |conn| {
                        conn.query_row("PRAGMA wal_checkpoint(PASSIVE);", [], |row| {
                            Ok((
                                row.get::<_, i64>(0)?,
                                row.get::<_, i64>(1)?,
                                row.get::<_, i64>(2)?,
                            ))
                        })
                    })
                    .await;
                let elapsed_ms = started.elapsed().as_millis();
                match result {
                    Ok((busy, log_frames, ckpt_frames)) => {
                        if busy != 0 || log_frames > WAL_AUTOCHECKPOINT_FRAMES.saturating_mul(4) {
                            log::warn!(
                                "db.wal_checkpoint_passive elevated: busy={busy} log_frames={log_frames} checkpointed={ckpt_frames} elapsed_ms={elapsed_ms}"
                            );
                        } else {
                            log::trace!(
                                "db.wal_checkpoint_passive: log_frames={log_frames} checkpointed={ckpt_frames} elapsed_ms={elapsed_ms}"
                            );
                        }
                    }
                    Err(PooledCallError::Closed) => {
                        log::debug!("db_service: checkpoint task exiting (writer closed)");
                        break;
                    }
                    Err(error) => {
                        log::warn!("db.wal_checkpoint_passive failed: {error}");
                    }
                }
                // Persist latency counters every N ticks for restart durability.
                if tick_counter.is_multiple_of(LATENCY_PERSIST_INTERVAL_TICKS) {
                    persist_latency_snapshot_to_kv(&writer).await;
                }
            }
        });
    }

    #[cfg(test)]
    pub async fn open_at_with_fixture_provider_for_tests(
        path: PathBuf,
    ) -> Result<Arc<Self>, DbError> {
        Self::open_at(path, Arc::new(FixtureDbServiceKeyProvider::new())).await
    }

    /// Unencrypted variant used by test harnesses that need `AppState`
    /// without touching the user's encrypted DailyOS database or Keychain.
    #[cfg(any(feature = "test-harness", feature = "bench-harness"))]
    #[doc(hidden)]
    pub async fn open_at_unencrypted_for_tests(path: PathBuf) -> Result<Arc<Self>, DbError> {
        Self::open_at_unencrypted_test_impl(path).await
    }

    #[cfg(any(test, feature = "test-harness", feature = "bench-harness"))]
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
            conn.execute_batch(&format!(
                "PRAGMA wal_autocheckpoint = {WAL_AUTOCHECKPOINT_FRAMES};"
            ))?;
            conn.execute_batch(&format!(
                "PRAGMA journal_size_limit = {JOURNAL_SIZE_LIMIT_BYTES};"
            ))?;
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
                conn.execute_batch(&format!(
                    "PRAGMA wal_autocheckpoint = {WAL_AUTOCHECKPOINT_FRAMES};"
                ))?;
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
        crate::db::guard_path_for_mode(&path)?;

        let started = Instant::now();
        let path = path.to_string_lossy().to_string();
        let writer = self.writer();
        let result = writer.call_sync_labeled("open_fresh_serialized", move |_| {
            open_encrypted_fresh(&path, &encryption_key, false)
        });
        crate::latency::record_latency(
            "open_fresh_serialized.total",
            started.elapsed().as_millis(),
            500,
        );
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

    /// Reader connection, round-robin, annotated with a tier label for
    /// telemetry. Equivalent to `reader().with_tier(tier)` but a single call
    /// for the hot foreground sites. See [`PooledConnection::with_tier`] for
    /// W0-B / W1-D earn-signal context.
    pub fn reader_for_tier(&self, tier: &'static str) -> PooledConnection {
        self.reader().with_tier(tier)
    }
}

/// app_state_kv key for the persisted latency-counter snapshot. Written every
/// `LATENCY_PERSIST_INTERVAL_TICKS` checkpoint cycles by `spawn_checkpoint_task`
/// and hydrated once from `open_at`. v1 schema: `LatencyPersistentSnapshot`
/// JSON. A schema change here requires bumping the key (`...v2`) so old keys
/// don't deserialize incorrectly.
const LATENCY_KV_KEY: &str = "db.latency.persistent_snapshot.v1";

/// One persist per N checkpoint cycles. 30 s × 2 = 60 s persist cadence,
/// which trades restart granularity against writer-thread budget.
const LATENCY_PERSIST_INTERVAL_TICKS: u32 = 2;

/// Persist current latency counters to `app_state_kv` via the writer queue.
/// Best-effort: a failure here is logged but does not propagate, because a
/// failed persist must not break a foreground command or stop the checkpoint
/// loop. The next tick will retry.
///
/// Wraps the write in `ActionDb::with_transaction` so the call satisfies
/// ADR-0133 §2's "explicit transaction wrapper" requirement, even though the
/// single INSERT OR REPLACE would auto-commit on its own.
async fn persist_latency_snapshot_to_kv(writer: &PooledConnection) {
    let snapshot = crate::latency::snapshot_for_persistence();
    let Ok(value_json) = serde_json::to_string(&snapshot) else {
        log::warn!("latency snapshot serialize failed");
        return;
    };
    let timestamp = chrono::Utc::now().to_rfc3339();
    let result = writer
        .call_labeled("db.latency.persist_snapshot", move |conn| {
            let db = crate::db::ActionDb::from_conn(conn);
            db.with_transaction(|inner| {
                inner
                    .conn_ref()
                    .execute(
                        "INSERT OR REPLACE INTO app_state_kv (key, value_json, updated_at) \
                         VALUES (?1, ?2, ?3)",
                        rusqlite::params![LATENCY_KV_KEY, value_json, timestamp],
                    )
                    .map(|_| ())
                    .map_err(|e| e.to_string())
            })
            .map_err(rusqlite::Error::InvalidParameterName)
        })
        .await;
    if let Err(error) = result {
        log::warn!("latency snapshot persist failed: {error}");
    }
}

/// Hydrate latency counters from the most recent persisted snapshot, if any.
/// Called once from `open_at` so the W0 measurement protocol's long-window
/// counters survive process restarts. Missing keys / parse errors are
/// non-fatal — startup must not block on telemetry plumbing.
async fn hydrate_latency_snapshot_from_kv(reader: &PooledConnection) {
    let row_result = reader
        .call_labeled("db.latency.hydrate_snapshot", |conn| {
            conn.query_row(
                "SELECT value_json FROM app_state_kv WHERE key = ?1",
                rusqlite::params![LATENCY_KV_KEY],
                |row| row.get::<_, String>(0),
            )
            .map(Some)
            .or_else(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(other),
            })
        })
        .await;
    let Ok(Some(value_json)) = row_result else {
        return;
    };
    let Ok(snapshot) =
        serde_json::from_str::<crate::latency::LatencyPersistentSnapshot>(&value_json)
    else {
        log::warn!("latency snapshot deserialize failed; ignoring");
        return;
    };
    crate::latency::apply_persistent_snapshot(snapshot);
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
            summary_context_prompt_version: None,
            summary_context_trust_band: None,
            summary_context_source_count: None,
            summary_context_source_keys_json: None,
            summary_context_generated_at: None,
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

    fn sqlite_error(code: std::os::raw::c_int, message: &str) -> rusqlite::Error {
        rusqlite::Error::SqliteFailure(rusqlite::ffi::Error::new(code), Some(message.to_string()))
    }

    #[test]
    fn db_access_error_classifies_database_busy_as_retryable() {
        let error = DbAccessError::from(sqlite_error(rusqlite::ffi::SQLITE_BUSY, "busy"));

        assert_eq!(error.class(), DbAccessErrorClass::Retryable);
        assert!(error.is_retryable());
    }

    #[test]
    fn db_access_error_classifies_database_locked_as_retryable() {
        let error = DbAccessError::from(sqlite_error(rusqlite::ffi::SQLITE_LOCKED, "locked"));

        assert_eq!(error.class(), DbAccessErrorClass::Retryable);
        assert!(error.is_retryable());
    }

    #[test]
    fn db_access_error_classifies_notadb_as_retryable() {
        let error =
            DbAccessError::from(sqlite_error(rusqlite::ffi::SQLITE_NOTADB, "not a database"));

        assert_eq!(error.class(), DbAccessErrorClass::Retryable);
        assert!(error.is_retryable());
    }

    #[test]
    fn db_access_error_classifies_constraint_violation_as_other() {
        let error =
            DbAccessError::from(sqlite_error(rusqlite::ffi::SQLITE_CONSTRAINT, "constraint"));

        assert_eq!(error.class(), DbAccessErrorClass::Other);
        assert!(!error.is_retryable());
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

        {
            let _rotation_test_guard = crate::db::key_provider::rotation_test_guard();
            install_global(svc.clone());
            let _global_guard = GlobalServiceGuard;
            let rotated = provider
                .rotate_key(&UserIdentity::local(path.clone()))
                .expect("rotate through global DbService");
            assert_eq!(rotated, EncryptionKey::from_hex(new_key.to_string()));
            uninstall_global();
        }

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

        {
            let _rotation_test_guard = crate::db::key_provider::rotation_test_guard();
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
        }
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
