//! SQLite-based local state management for actions, accounts, and meeting history.
//!
//! The database lives at `~/.dailyos/dailyos.db` and serves as the working store
//! for operational data (ADR-0048). The filesystem (markdown + JSON) is the durable
//! layer; SQLite enables fast queries, state tracking, and cross-entity intelligence.
//! SQLite is not disposable — important state lives here and is written back to the
//! filesystem at natural synchronization points (archive, dashboard regeneration).

use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
#[cfg(test)]
use std::sync::OnceLock;

use super::types::*;
use crate::db::encryption;
use crate::db::key_provider::{DbKeyProvider, EncryptionKey, LocalKeychain, UserIdentity};
use ring::hmac;
use rusqlite::{params, Connection, OpenFlags};
use sha2::{Digest, Sha256};

// ---------------------------------------------------------------------------
// Dev DB isolation
// ---------------------------------------------------------------------------

/// Process-wide DB-mode selector. Steers `ActionDb::db_path()`
/// between the production DB and isolated dev files, and — via the structural
/// guard below — forbids opening the production DB in any non-Live mode.
///
/// Background threads (executor, intel_queue, watcher, hygiene) and separate
/// binaries (MCP, doctor, maintenance) all call `ActionDb::open()` independently;
/// the process-wide value means each picks up the right path without plumbing.
/// Set ONCE at process bootstrap, before any thread spawn or open. `0` = unset
/// (resolves to the fail-closed default in `db_mode()`).
static DB_MODE: AtomicU8 = AtomicU8::new(0);

/// Which database a process operates against. Process-wide, set once at bootstrap.
/// Orthogonal to ADR-0104 `ExecutionMode` (request-scoped mutation-gating) — this
/// is process-wide path-selection. Do not merge the two.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DbMode {
    /// Production. `~/.dailyos/dailyos.db`. The ONLY mode allowed to open prod.
    Live,
    /// Production replica with real content. `~/.dailyos/dailyos-replica.db`.
    Replica,
    /// Fixtures / surface-state testing. `~/.dailyos/dailyos-dev.db`.
    Mock,
}

impl DbMode {
    fn as_u8(self) -> u8 {
        match self {
            DbMode::Live => 1,
            DbMode::Replica => 2,
            DbMode::Mock => 3,
        }
    }
    fn from_u8(v: u8) -> Option<Self> {
        match v {
            1 => Some(DbMode::Live),
            2 => Some(DbMode::Replica),
            3 => Some(DbMode::Mock),
            _ => None,
        }
    }

    fn from_process_arg(arg: &str) -> Option<Self> {
        match arg {
            "--live" => Some(DbMode::Live),
            "--replica" => Some(DbMode::Replica),
            "--mock" => Some(DbMode::Mock),
            _ => None,
        }
    }

    fn from_env_value(value: &str) -> Option<Self> {
        match value.trim() {
            "live" => Some(DbMode::Live),
            "replica" => Some(DbMode::Replica),
            "mock" => Some(DbMode::Mock),
            _ => None,
        }
    }
}
static WRITE_TRANSACTION_GATE: parking_lot::Mutex<()> = parking_lot::const_mutex(());
static WRITE_TRANSACTION_HOLDER: parking_lot::Mutex<Option<WriteTransactionHolder>> =
    parking_lot::const_mutex(None);
const WORKSPACE_GRAPH_DIAGNOSTIC_KEY_DERIVATION_DOMAIN: &[u8] =
    b"DAILYOS-WORKSPACE-GRAPH-DIAGNOSTIC-HANDLE-V1\n";

#[cfg(test)]
static TEST_DAILYOS_DATA_DIR: OnceLock<PathBuf> = OnceLock::new();

#[derive(Clone, Debug)]
struct WriteTransactionHolder {
    caller: String,
    acquired_at: std::time::Instant,
}

struct WriteTransactionHolderGuard;

impl WriteTransactionHolderGuard {
    fn set(caller: &'static std::panic::Location<'static>) -> Self {
        *WRITE_TRANSACTION_HOLDER.lock() = Some(WriteTransactionHolder {
            caller: format!("{}:{}", caller.file(), caller.line()),
            acquired_at: std::time::Instant::now(),
        });
        Self
    }
}

impl Drop for WriteTransactionHolderGuard {
    fn drop(&mut self) {
        *WRITE_TRANSACTION_HOLDER.lock() = None;
    }
}

fn write_transaction_holder_summary() -> String {
    WRITE_TRANSACTION_HOLDER
        .lock()
        .clone()
        .map(|holder| {
            format!(
                "{} held for {}ms",
                holder.caller,
                holder.acquired_at.elapsed().as_millis()
            )
        })
        .unwrap_or_else(|| "unknown holder".to_string())
}

/// Set the process-wide DB mode. Must be called once at bootstrap, before any
/// thread spawn or `ActionDb::open()`. Changing it after the first open is a bug.
pub fn set_db_mode(mode: DbMode) {
    DB_MODE.store(mode.as_u8(), Ordering::Release);
}

/// Resolve DB mode from process inputs and set it when explicitly provided.
///
/// CLI flags (`--live`, `--replica`, `--mock`) take precedence over
/// `DAILYOS_DB_MODE=live|replica|mock`. If neither is present, leave DB_MODE
/// unset so `db_mode()` keeps its fail-closed default.
pub fn resolve_and_set_db_mode_from_process() {
    if let Some(mode) = std::env::args().find_map(|arg| DbMode::from_process_arg(&arg)) {
        set_db_mode(mode);
        return;
    }

    if let Ok(value) = std::env::var("DAILYOS_DB_MODE") {
        if let Some(mode) = DbMode::from_env_value(&value) {
            set_db_mode(mode);
        }
    }
}

/// Resolve the active DB mode. **Fail-closed default when unset:** non-release
/// (`debug_assertions`) builds default to `Replica`, release builds to `Live`.
/// A no-env dev process can therefore never fall through to the production DB.
pub fn db_mode() -> DbMode {
    match DbMode::from_u8(DB_MODE.load(Ordering::Acquire)) {
        Some(mode) => mode,
        None if cfg!(debug_assertions) => DbMode::Replica,
        None => DbMode::Live,
    }
}

/// Legacy shim: "dev mode" == `Mock` (fixture DB). Preserved for callers not yet
/// migrated to `db_mode()`. Returns `false` in Replica/Live.
pub fn is_dev_db_mode() -> bool {
    db_mode() == DbMode::Mock
}

/// Legacy shim. `true` → `Mock`, `false` → `Live`.
pub fn set_dev_db_mode(enabled: bool) {
    set_db_mode(if enabled { DbMode::Mock } else { DbMode::Live });
}

/// Resolve the root directory for DailyOS application data.
///
/// Production builds use the user's normal data directory. Test builds use a
/// process-unique directory under the system temp directory so tests cannot
/// resolve the real user data directory, even when the DB mode is forced Live.
#[cfg(not(test))]
pub fn dailyos_data_dir() -> Result<PathBuf, DbError> {
    dirs::home_dir()
        .map(|home| home.join(".dailyos"))
        .ok_or(DbError::HomeDirNotFound)
}

/// Resolve the root directory for DailyOS application data.
///
/// Production builds use the user's normal data directory. Test builds use a
/// process-unique directory under the system temp directory so tests cannot
/// resolve the real user data directory, even when the DB mode is forced Live.
#[cfg(test)]
pub fn dailyos_data_dir() -> Result<PathBuf, DbError> {
    // Resolve and create the isolated test root exactly once. Creating it inside
    // the OnceLock initializer avoids a per-call `create_dir_all` syscall on a
    // shared path, which adds filesystem contention when thousands of tests
    // resolve data paths concurrently.
    Ok(TEST_DAILYOS_DATA_DIR
        .get_or_init(|| {
            let unique = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0);
            let dir = std::env::temp_dir()
                .join(format!(
                    "dailyos-test-home-{}-{unique}",
                    std::process::id()
                ))
                .join(".dailyos");
            #[allow(
                clippy::let_underscore_must_use,
                reason = "best-effort create of the isolated test root; downstream writes surface real errors"
            )]
            let _ = std::fs::create_dir_all(&dir);
            dir
        })
        .clone())
}

/// The production database file paths (live + legacy). These may be opened ONLY
/// when `db_mode() == Live`; the guard below enforces it.
fn prod_db_paths() -> Vec<PathBuf> {
    dailyos_data_dir()
        .map(|dir| vec![dir.join("dailyos.db"), dir.join("actions.db")])
        .unwrap_or_default()
}

/// Structural prod-open deny. Called at every connection-open
/// chokepoint with the resolved path BEFORE the key is fetched or the file is
/// opened. The encryption key cache is path-blind, so the path layer is the only
/// barrier — it must be enforced here, not merely in `db_path()`.
pub(crate) fn guard_path_for_mode(path: &Path) -> Result<(), DbError> {
    let mode = db_mode();
    if mode == DbMode::Live {
        return Ok(());
    }
    let target_candidates = guarded_path_candidates(path);
    if prod_db_paths().iter().any(|prod_path| {
        let prod_candidates = guarded_path_candidates(prod_path);
        target_candidates.iter().any(|target| {
            prod_candidates
                .iter()
                .any(|prod| guarded_paths_equal(target, prod))
        })
    }) {
        return Err(DbError::ProdOpenDenied {
            mode: format!("{mode:?}"),
            path: path.display().to_string(),
        });
    }
    Ok(())
}

fn guarded_path_candidates(path: &Path) -> Vec<PathBuf> {
    let mut candidates = vec![lexically_normalized_path(path)];
    if let Ok(canonicalized) = path.canonicalize() {
        let canonicalized = lexically_normalized_path(&canonicalized);
        if !candidates
            .iter()
            .any(|candidate| guarded_paths_equal(candidate, &canonicalized))
        {
            candidates.push(canonicalized);
        }
    }
    candidates
}

fn lexically_normalized_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                let can_pop = matches!(
                    normalized.components().next_back(),
                    Some(Component::Normal(_))
                );
                if can_pop {
                    normalized.pop();
                } else if !normalized.has_root() {
                    normalized.push(component.as_os_str());
                }
            }
            Component::Prefix(_) | Component::RootDir | Component::Normal(_) => {
                normalized.push(component.as_os_str());
            }
        }
    }
    if normalized.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        normalized
    }
}

#[cfg(target_os = "macos")]
fn guarded_paths_equal(left: &Path, right: &Path) -> bool {
    left.as_os_str()
        .to_string_lossy()
        .eq_ignore_ascii_case(&right.as_os_str().to_string_lossy())
}

#[cfg(not(target_os = "macos"))]
fn guarded_paths_equal(left: &Path, right: &Path) -> bool {
    left == right
}

#[repr(transparent)]
pub struct ActionDb {
    pub(crate) conn: Connection,
}

#[cfg(test)]
struct FixtureDbKeyProvider {
    key: EncryptionKey,
}

#[cfg(test)]
impl FixtureDbKeyProvider {
    fn new() -> Self {
        Self {
            key: EncryptionKey::from_hex(
                "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
            ),
        }
    }
}

#[cfg(test)]
impl DbKeyProvider for FixtureDbKeyProvider {
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

pub(crate) fn local_db_keyed_audit_tag(
    tag_prefix: &str,
    domain: &str,
    components: &[&str],
) -> Result<String, String> {
    let db_path = ActionDb::db_path_public().map_err(|e| e.to_string())?;
    let provider = LocalKeychain::new();
    let key = provider.get_or_create_key(&UserIdentity::local(db_path))?;
    Ok(keyed_audit_tag(
        tag_prefix,
        domain,
        components,
        key.as_hex().as_bytes(),
    ))
}

/// Key-derived audit tagger captured when a DB connection is opened.
///
/// MCP registered write paths use this to avoid calling [`LocalKeychain`] during
/// a request, which can validate the key by reopening SQLite in the sidecar.
#[derive(Clone)]
pub(crate) struct LocalDbAuditTagger {
    key: EncryptionKey,
}

impl LocalDbAuditTagger {
    fn new(key: EncryptionKey) -> Self {
        Self { key }
    }

    #[cfg(test)]
    pub(crate) fn for_tests(secret: &str) -> Self {
        Self {
            key: EncryptionKey::from_hex(secret.to_string()),
        }
    }

    pub(crate) fn tag(&self, tag_prefix: &str, domain: &str, components: &[&str]) -> String {
        keyed_audit_tag(tag_prefix, domain, components, self.key.as_hex().as_bytes())
    }
}

impl std::fmt::Debug for LocalDbAuditTagger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("LocalDbAuditTagger([REDACTED])")
    }
}

pub(crate) fn local_db_workspace_graph_diagnostic_key_bytes() -> Result<[u8; 32], String> {
    let db_path = ActionDb::db_path_public().map_err(|e| e.to_string())?;
    let provider = LocalKeychain::new();
    let key = provider.get_or_create_key(&UserIdentity::local(db_path))?;
    Ok(workspace_graph_diagnostic_key_bytes(
        key.as_hex().as_bytes(),
    ))
}

#[cfg(test)]
pub(crate) fn local_db_keyed_audit_tag_for_tests(
    secret: &str,
    tag_prefix: &str,
    domain: &str,
    components: &[&str],
) -> String {
    keyed_audit_tag(tag_prefix, domain, components, secret.as_bytes())
}

fn keyed_audit_tag(tag_prefix: &str, domain: &str, components: &[&str], secret: &[u8]) -> String {
    let key = hmac::Key::new(hmac::HMAC_SHA256, secret);
    let mut context = hmac::Context::with_key(&key);
    context.update(domain.as_bytes());
    for component in components {
        context.update(&[0]);
        context.update(component.as_bytes());
    }
    let tag = context.sign();
    format!("{tag_prefix}_{}", hex::encode(&tag.as_ref()[..16]))
}

fn workspace_graph_diagnostic_key_bytes(secret: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(WORKSPACE_GRAPH_DIAGNOSTIC_KEY_DERIVATION_DOMAIN);
    hasher.update(secret);
    hasher.finalize().into()
}

impl ActionDb {
    /// Borrow the underlying connection for ad-hoc queries.
    pub fn conn_ref(&self) -> &Connection {
        &self.conn
    }

    /// Consume the wrapper and return the underlying connection.
    pub fn into_connection(self) -> Connection {
        self.conn
    }

    /// Borrow a `Connection` owned elsewhere as an `ActionDb` view.
    ///
    /// `ActionDb` is `repr(transparent)` over `rusqlite::Connection`, so this
    /// view has the same layout and a lifetime tied to the input borrow. The
    /// borrowed view cannot outlive `conn` or be moved into a `'static`
    /// closure, which keeps pooled `.call()` usage type-system bounded.
    pub fn from_conn(conn: &Connection) -> &Self {
        // SAFETY: `ActionDb` is `repr(transparent)` and its only field is
        // `Connection`, so `&Connection` and `&ActionDb` have identical layout.
        unsafe { &*(conn as *const Connection as *const Self) }
    }

    /// Execute a closure within a SQLite transaction.
    /// Commits on Ok, rolls back on Err.
    #[must_use = "the closure may write to the DB; dropping this Result silently swallows transaction failure or rollback"]
    #[track_caller]
    pub fn with_transaction<F, T>(&self, f: F) -> Result<T, String>
    where
        F: FnOnce(&Self) -> Result<T, String>,
    {
        // Nested transaction support: if we're already inside a transaction on this
        // connection, execute the closure directly so all writes stay in the
        // caller's transaction boundary.
        if !self.conn.is_autocommit() {
            return f(self);
        }

        let gate_started = std::time::Instant::now();
        // W0-B: log gate waits beginning at 250 ms (was 5 s). The previous
        // threshold only surfaced storms; the lower one surfaces the routine
        // contention that drives 8.8 s foreground beachballs.
        let mut next_wait_log_at = std::time::Duration::from_millis(250);
        let wait_limit = std::time::Duration::from_secs(120);
        let _write_gate = loop {
            if let Some(gate) =
                WRITE_TRANSACTION_GATE.try_lock_for(std::time::Duration::from_millis(250))
            {
                break gate;
            }
            let waited = gate_started.elapsed();
            if waited >= next_wait_log_at {
                log::warn!(
                    "write transaction gate busy for {}ms before BEGIN IMMEDIATE at {}:{}; current holder: {}",
                    waited.as_millis(),
                    std::panic::Location::caller().file(),
                    std::panic::Location::caller().line(),
                    write_transaction_holder_summary(),
                );
                next_wait_log_at += std::time::Duration::from_secs(5);
            }
            if waited >= wait_limit {
                return Err(format!(
                    "Timed out waiting for process write transaction gate before BEGIN IMMEDIATE; waited={}ms; current holder: {}",
                    waited.as_millis(),
                    write_transaction_holder_summary()
                ));
            }
        };
        let _holder_guard = WriteTransactionHolderGuard::set(std::panic::Location::caller());
        // W0-B: budget tightened from 500 ms to 100 ms so the latency rollup's
        // budget_violations counter surfaces routine contention. The 250 ms
        // AC threshold is captured by the separate `_over_250ms` rollup below.
        let gate_wait_ms = gate_started.elapsed().as_millis();
        crate::latency::record_latency("action_db.write_transaction_gate_wait", gate_wait_ms, 100);
        // Dedicated rollup for the AC4 threshold (zero gate-waits > 250 ms at
        // user-active times). Records only when above the threshold so the
        // rollup's sample count IS the violation count.
        if gate_wait_ms > 250 {
            crate::latency::record_latency(
                "action_db.write_transaction_gate_wait_over_250ms",
                gate_wait_ms,
                250,
            );
        }

        self.conn
            .execute_batch("BEGIN IMMEDIATE")
            .map_err(|e| format!("Failed to begin transaction: {e}"))?;
        match f(self) {
            Ok(val) => {
                self.conn
                    .execute_batch("COMMIT")
                    .map_err(|e| format!("Failed to commit transaction: {e}"))?;
                Ok(val)
            }
            Err(e) => {
                #[allow(
                    clippy::let_underscore_must_use,
                    reason = "intentional best-effort discard; preserves existing non-blocking behavior"
                )]
                // best-effort: preserve the original transaction error if rollback itself fails.
                let _ = self.conn.execute_batch("ROLLBACK");
                Err(e)
            }
        }
    }

    fn recover_stuck_version_mutations_logged(db: &ActionDb) {
        match crate::services::versioning::recover_stuck_mutation_attempts(db, chrono::Utc::now()) {
            Ok(0) => {}
            Ok(count) => log::warn!(
                "recovered {count} stale in-flight version mutation attempt(s) at startup"
            ),
            Err(error) => log::warn!("version mutation startup recovery scan failed: {error}"),
        }
    }

    fn map_key_error(error: String) -> DbError {
        if error.starts_with("KEY_MISSING:") {
            DbError::KeyMissing {
                db_path: error.trim_start_matches("KEY_MISSING:").to_string(),
            }
        } else {
            DbError::Encryption(error)
        }
    }

    fn prepare_encrypted_connection(
        path: &Path,
        key_provider: Arc<dyn DbKeyProvider>,
    ) -> Result<(Connection, EncryptionKey), DbError> {
        // structural prod-open deny — before key fetch or file open.
        guard_path_for_mode(path)?;

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent).map_err(DbError::CreateDir)?;
            }
        }

        // Get or create encryption key from Keychain
        let user = UserIdentity::local(path.to_path_buf());
        let encryption_key = key_provider
            .get_or_create_key(&user)
            .map_err(Self::map_key_error)?;

        // Migrate plaintext DB if it exists (ADR-0092)
        if path.exists() && encryption::is_database_plaintext(path) {
            log::info!("Detected plaintext database, migrating to encrypted...");
            encryption::migrate_to_encrypted(path, encryption_key.as_hex())
                .map_err(DbError::Encryption)?;
        }

        let conn = Connection::open(path)?;

        // PRAGMA key MUST be first — before any other PRAGMA (ADR-0092)
        conn.execute_batch(&encryption_key.to_pragma())?;

        // Validate that the key can read the database by touching schema metadata.
        // This avoids engine-specific SQLCipher functions (e.g. sqlcipher_version)
        // that may not exist in all bundled builds.
        conn.query_row("SELECT count(*) FROM sqlite_master LIMIT 1", [], |row| {
            row.get::<_, i64>(0)
        })
        .map_err(|e| {
            DbError::Encryption(format!(
                "SQLCipher key verification failed (database unreadable): {e}"
            ))
        })?;

        // Enable WAL mode for better concurrent read performance
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;

        // Retry for up to 5s on SQLITE_BUSY instead of failing immediately.
        // Without this, background tasks opening their own connections cause
        // immediate failures when the main connection holds a write lock.
        conn.execute_batch("PRAGMA busy_timeout = 5000;")?;

        // NORMAL sync is safe with WAL — only fsyncs on checkpoint, not every commit.
        // ~3x write throughput improvement over the default FULL.
        conn.execute_batch("PRAGMA synchronous = NORMAL;")?;

        // Run schema migrations (ADR-0071)
        crate::migrations::run_migrations_with_key(&conn, Some(&encryption_key))
            .map_err(DbError::Migration)?;

        Self::recover_stuck_version_mutations_logged(Self::from_conn(&conn));

        // Enable FK constraint enforcement. Set after migrations since
        // migration 010 uses PRAGMA foreign_keys = OFF for table recreation.
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;

        // Legacy data repairs — idempotent Rust code, safe to run every startup.
        // Will be removed once all alpha users are past v0.7.3.
        #[allow(
            clippy::let_underscore_must_use,
            reason = "intentional best-effort discard; preserves existing non-blocking behavior"
        )]
        let _ = Self::normalize_reviewed_prep_keys(&conn);
        #[allow(
            clippy::let_underscore_must_use,
            reason = "intentional best-effort discard; preserves existing non-blocking behavior"
        )]
        let _ = Self::backfill_meeting_identity(&conn);
        #[allow(
            clippy::let_underscore_must_use,
            reason = "intentional best-effort discard; preserves existing non-blocking behavior"
        )]
        let _ = Self::backfill_meeting_user_layer(&conn);
        #[allow(
            clippy::let_underscore_must_use,
            reason = "intentional best-effort discard; preserves existing non-blocking behavior"
        )]
        let _ = Self::backfill_stakeholder_columns(&conn);
        #[allow(
            clippy::let_underscore_must_use,
            reason = "intentional best-effort discard; preserves existing non-blocking behavior"
        )]
        let _ = Self::dismiss_internal_stakeholder_suggestions(&conn);

        Ok((conn, encryption_key))
    }

    pub(crate) fn open_encrypted_connection(
        path: PathBuf,
        key_provider: Arc<dyn DbKeyProvider>,
    ) -> Result<(Connection, EncryptionKey), DbError> {
        let (conn, encryption_key) = Self::prepare_encrypted_connection(&path, key_provider)?;
        let db = Self { conn };

        // One-time initialization tasks (guarded by init_tasks table).
        // These run exactly once per database and are safe to call on every startup.
        #[allow(
            clippy::let_underscore_must_use,
            reason = "intentional best-effort discard; preserves existing non-blocking behavior"
        )]
        let _ = db.run_guarded_init_backfill_account_domains();

        Ok((db.conn, encryption_key))
    }

    /// Open (or create) the database at `~/.dailyos/dailyos.db` and apply the schema.
    ///
    /// Every call creates a fresh `rusqlite::Connection` via direct open. When
    /// a global `DbService` is installed, the fresh-open path is executed on
    /// the writer's dedicated thread to avoid SQLCipher WAL key-verification races
    /// (SQLITE_NOTADB) while preserving a non-shared ownership contract.
    pub fn open(key_provider: Arc<dyn DbKeyProvider>) -> Result<Self, DbError> {
        let path = Self::db_path()?;
        Self::open_resolved_path(path, key_provider)
    }

    /// Open the database for diagnostic inspection WITHOUT running startup
    /// recovery for stuck mutation attempts.
    ///
    /// `ActionDb::open` calls `recover_stuck_mutation_attempts` during
    /// connection bring-up, which aborts any `mutation_attempts` row whose
    /// `in_flight` lease exceeds 30 seconds. `dailyos doctor watermarks`
    /// needs to inventory those zombie rows BEFORE recovery wipes them;
    /// otherwise the doctor's own action mutates the state it's reporting.
    /// Per packet ac §36 + L2 cycle-2 P2 (codex): the doctor must read,
    /// not heal.
    pub fn open_for_inspection(key_provider: Arc<dyn DbKeyProvider>) -> Result<Self, DbError> {
        let path = Self::db_path()?;
        let (conn, _key) = Self::prepare_encrypted_connection_no_recovery(&path, key_provider)?;
        Ok(Self { conn })
    }

    /// Variant of `prepare_encrypted_connection` that runs migrations but
    /// skips startup recovery for in-flight mutation attempts. Used by
    /// `open_for_inspection`. The two should diverge in EXACTLY that line
    /// of behaviour; centralised so the encryption + migration setup
    /// cannot drift between paths.
    fn prepare_encrypted_connection_no_recovery(
        path: &Path,
        key_provider: Arc<dyn DbKeyProvider>,
    ) -> Result<(Connection, EncryptionKey), DbError> {
        // structural prod-open deny — before key fetch or file open.
        guard_path_for_mode(path)?;

        if let Some(parent) = path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent).map_err(DbError::CreateDir)?;
            }
        }

        let user = UserIdentity::local(path.to_path_buf());
        let encryption_key = key_provider
            .get_or_create_key(&user)
            .map_err(Self::map_key_error)?;

        if path.exists() && encryption::is_database_plaintext(path) {
            log::info!("Detected plaintext database, migrating to encrypted...");
            encryption::migrate_to_encrypted(path, encryption_key.as_hex())
                .map_err(DbError::Encryption)?;
        }

        let conn = Connection::open(path)?;
        conn.execute_batch(&encryption_key.to_pragma())?;
        conn.query_row("SELECT count(*) FROM sqlite_master LIMIT 1", [], |row| {
            row.get::<_, i64>(0)
        })
        .map_err(|e| {
            DbError::Encryption(format!(
                "SQLCipher key verification failed (database unreadable): {e}"
            ))
        })?;
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        conn.execute_batch("PRAGMA busy_timeout = 5000;")?;
        conn.execute_batch("PRAGMA synchronous = NORMAL;")?;

        crate::migrations::run_migrations_with_key(&conn, Some(&encryption_key))
            .map_err(DbError::Migration)?;

        // Intentionally skip recover_stuck_version_mutations_logged so the
        // doctor inspection can count zombie attempts. No legacy backfill
        // either — those are healing operations.
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;

        Ok((conn, encryption_key))
    }

    fn open_resolved_path(
        path: PathBuf,
        key_provider: Arc<dyn DbKeyProvider>,
    ) -> Result<Self, DbError> {
        Self::open_resolved_path_with_key(path, key_provider).map(|(db, _key)| db)
    }

    fn open_resolved_path_with_key(
        path: PathBuf,
        key_provider: Arc<dyn DbKeyProvider>,
    ) -> Result<(Self, EncryptionKey), DbError> {
        // structural prod-open deny — covers the svc.open_fresh_serialized
        // branch, which does not route through prepare_encrypted_connection.
        guard_path_for_mode(&path)?;
        let rotation_lock = crate::db::key_provider::rotation_lock_read();
        if let Some(svc) = crate::db_service::try_global() {
            let user = UserIdentity::local(path.clone());
            let encryption_key = key_provider
                .get_or_create_key(&user)
                .map_err(Self::map_key_error)?;
            let conn = svc.open_fresh_serialized(path.clone(), encryption_key.clone())?;
            drop(rotation_lock);
            // Startup initialization already runs through the global DbService.
            // Fresh handles should not add best-effort writes outside that path.
            return Ok((Self { conn }, encryption_key));
        }

        let (conn, encryption_key) = Self::open_encrypted_connection(path, key_provider)?;
        drop(rotation_lock);
        Ok((Self { conn }, encryption_key))
    }

    #[cfg(test)]
    pub(crate) fn open_resolved_path_for_tests(
        path: PathBuf,
        key_provider: Arc<dyn DbKeyProvider>,
    ) -> Result<Self, DbError> {
        Self::open_resolved_path(path, key_provider)
    }

    pub(crate) fn open_with_audit_tagger(
        key_provider: Arc<dyn DbKeyProvider>,
    ) -> Result<(Self, LocalDbAuditTagger), DbError> {
        let path = Self::db_path()?;
        let (db, key) = Self::open_resolved_path_with_key(path, key_provider)?;
        Ok((db, LocalDbAuditTagger::new(key)))
    }

    #[cfg(test)]
    pub(crate) fn open_resolved_path_with_fixture_provider_for_tests(
        path: PathBuf,
    ) -> Result<Self, DbError> {
        Self::open_resolved_path(path, Arc::new(FixtureDbKeyProvider::new()))
    }

    /// Open a database at an explicit path. Useful for testing.
    pub(crate) fn open_at(
        path: PathBuf,
        key_provider: Arc<dyn DbKeyProvider>,
    ) -> Result<Self, DbError> {
        let (conn, _) = Self::open_encrypted_connection(path, key_provider)?;
        Ok(Self { conn })
    }

    /// Open without encryption. Used for tests only.
    #[cfg(test)]
    pub(crate) fn open_at_unencrypted(path: PathBuf) -> Result<Self, DbError> {
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent).map_err(DbError::CreateDir)?;
            }
        }
        let conn = Connection::open(&path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        conn.execute_batch("PRAGMA busy_timeout = 5000;")?;
        conn.execute_batch("PRAGMA synchronous = NORMAL;")?;
        crate::migrations::run_migrations(&conn).map_err(DbError::Migration)?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        let _ = Self::normalize_reviewed_prep_keys(&conn);
        let _ = Self::backfill_meeting_identity(&conn);
        let _ = Self::backfill_meeting_user_layer(&conn);
        let _ = Self::backfill_stakeholder_columns(&conn);

        let db = Self { conn };
        Self::recover_stuck_version_mutations_logged(&db);
        let _ = db.run_guarded_init_backfill_account_domains();
        Ok(db)
    }

    /// Open the database in read-only mode. Used by the MCP binary for safe
    /// concurrent reads while the Tauri app owns writes.
    pub fn open_readonly(key_provider: Arc<dyn DbKeyProvider>) -> Result<Self, DbError> {
        let path = Self::db_path()?;
        Self::open_readonly_at(&path, key_provider)
    }

    /// Open a database at an explicit path in read-only mode.
    pub fn open_readonly_at(
        path: &std::path::Path,
        key_provider: Arc<dyn DbKeyProvider>,
    ) -> Result<Self, DbError> {
        guard_path_for_mode(path)?;

        let user = UserIdentity::local(path.to_path_buf());
        let encryption_key = key_provider.get_or_create_key(&user).map_err(|e| {
            if e.starts_with("KEY_MISSING:") {
                DbError::KeyMissing {
                    db_path: e.trim_start_matches("KEY_MISSING:").to_string(),
                }
            } else {
                DbError::Encryption(e)
            }
        })?;

        let conn = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;

        // PRAGMA key MUST be first
        conn.execute_batch(&encryption_key.to_pragma())?;
        conn.query_row("SELECT count(*) FROM sqlite_master LIMIT 1", [], |row| {
            row.get::<_, i64>(0)
        })
        .map_err(|e| {
            DbError::Encryption(format!(
                "SQLCipher read-only key verification failed (database unreadable): {e}"
            ))
        })?;

        conn.execute_batch("PRAGMA busy_timeout = 5000;")?;
        conn.execute_batch("PRAGMA query_only = ON;")?;
        Ok(Self { conn })
    }

    #[cfg(any(test, feature = "test-harness", feature = "bench-harness"))]
    #[doc(hidden)]
    pub fn from_connection_for_tests(conn: Connection) -> Self {
        Self { conn }
    }

    #[cfg(any(test, feature = "test-harness"))]
    #[doc(hidden)]
    pub fn open_unencrypted_readonly_at_for_tests(path: &std::path::Path) -> Result<Self, DbError> {
        let conn = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        conn.execute_batch("PRAGMA busy_timeout = 5000;")?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        conn.execute_batch("PRAGMA query_only = ON;")?;
        Ok(Self { conn })
    }

    /// Resolve the default database path: `~/.dailyos/dailyos.db`.
    ///
    /// When dev-mode DB isolation is active (`set_dev_db_mode(true)`), returns
    /// `~/.dailyos/dailyos-dev.db` instead. Migration logic only applies to the
    /// live path — the dev DB is always created fresh.
    /// Public accessor for the resolved DB path. Used by `DbService` to open
    /// connections at the same path as `ActionDb::open()`.
    pub fn db_path_public() -> Result<PathBuf, DbError> {
        Self::db_path()
    }

    fn db_path() -> Result<PathBuf, DbError> {
        let dailyos_dir = dailyos_data_dir()?;

        // DB-mode isolation: non-Live modes resolve to isolated files
        // and never to the production DB.
        match db_mode() {
            DbMode::Replica => return Ok(dailyos_dir.join("dailyos-replica.db")),
            DbMode::Mock => return Ok(dailyos_dir.join("dailyos-dev.db")),
            DbMode::Live => {}
        }

        let new_path = dailyos_dir.join("dailyos.db");
        let legacy_path = dailyos_dir.join("actions.db");

        // One-time migration: rename actions.db → dailyos.db
        if !new_path.exists() && legacy_path.exists() {
            // Checkpoint WAL into the main file before renaming, otherwise
            // data written to the WAL but not yet flushed would be lost.
            if let Ok(conn) = Connection::open(&legacy_path) {
                #[allow(
                    clippy::let_underscore_must_use,
                    reason = "intentional best-effort discard; preserves existing non-blocking behavior"
                )]
                // best-effort: rename migration can still proceed if no WAL frames need flushing.
                let _ = conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");
                drop(conn);
            }

            if let Err(e) = std::fs::rename(&legacy_path, &new_path) {
                log::warn!(
                    "Failed to rename actions.db → dailyos.db: {}. Will open at legacy path.",
                    e
                );
                return Ok(legacy_path);
            }
            // Clean up WAL/SHM files (SQLite recreates them under the new name)
            #[allow(
                clippy::let_underscore_must_use,
                reason = "intentional best-effort discard; preserves existing non-blocking behavior"
            )]
            let _ = std::fs::remove_file(dailyos_dir.join("actions.db-wal"));
            #[allow(
                clippy::let_underscore_must_use,
                reason = "intentional best-effort discard; preserves existing non-blocking behavior"
            )]
            let _ = std::fs::remove_file(dailyos_dir.join("actions.db-shm"));
            log::info!("Migrated database: actions.db → dailyos.db");
        }

        Ok(new_path)
    }

    /// Convert reviewed-prep keys from legacy prep file paths to meeting IDs.
    fn normalize_reviewed_prep_keys(conn: &Connection) -> Result<(), DbError> {
        let rows: Vec<(String, Option<String>, String, Option<String>)> = {
            let mut stmt = conn.prepare(
                "SELECT prep_file, calendar_event_id, reviewed_at, title
                 FROM meeting_prep_state",
            )?;
            let mapped = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            })?;
            let mut items = Vec::new();
            for row in mapped {
                items.push(row?);
            }
            items
        };

        for (legacy_key, calendar_event_id, reviewed_at, title) in rows {
            let canonical = if let Some(ref cal_id) = calendar_event_id {
                if !cal_id.trim().is_empty() {
                    Self::sanitize_calendar_event_id(cal_id)
                } else {
                    Self::extract_meeting_id_from_review_key(&legacy_key)
                }
            } else {
                Self::extract_meeting_id_from_review_key(&legacy_key)
            };
            if canonical.is_empty() || canonical == legacy_key {
                continue;
            }
            conn.execute(
                "INSERT INTO meeting_prep_state (prep_file, calendar_event_id, reviewed_at, title)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(prep_file) DO UPDATE SET
                    reviewed_at = CASE
                        WHEN excluded.reviewed_at > meeting_prep_state.reviewed_at
                        THEN excluded.reviewed_at
                        ELSE meeting_prep_state.reviewed_at
                    END,
                    calendar_event_id = COALESCE(excluded.calendar_event_id, meeting_prep_state.calendar_event_id),
                    title = COALESCE(excluded.title, meeting_prep_state.title)",
                params![canonical, calendar_event_id, reviewed_at, title],
            )?;
            conn.execute(
                "DELETE FROM meeting_prep_state WHERE prep_file = ?1",
                params![legacy_key],
            )?;
        }
        Ok(())
    }

    fn extract_meeting_id_from_review_key(key: &str) -> String {
        let trimmed = key.trim();
        let without_prefix = trimmed.strip_prefix("preps/").unwrap_or(trimmed);
        without_prefix
            .trim_end_matches(".json")
            .trim_end_matches(".md")
            .to_string()
    }

    pub(super) fn sanitize_calendar_event_id(calendar_event_id: &str) -> String {
        calendar_event_id.replace('@', "_at_")
    }

    /// One-time backfill of dashboard.json narrative fields into DB columns.
    ///
    /// Iterates accounts with `tracker_path IS NOT NULL AND company_overview IS NULL`,
    /// reads their dashboard.json, and writes the fields to DB.
    /// Same for projects.
    #[must_use = "check how many dashboard fields were backfilled before trusting DB narrative columns"]
    pub fn backfill_dashboard_json_to_db(&self, workspace: &Path) -> Result<usize, DbError> {
        const TASK_NAME: &str = "backfill_dashboard_json_to_db_v1";

        if Self::is_init_task_completed(&self.conn, TASK_NAME)? {
            return Ok(0);
        }

        let mut count = 0usize;

        // Backfill accounts
        let accounts: Vec<(String, String)> = {
            let mut stmt = self.conn.prepare(
                "SELECT id, name FROM accounts \
                 WHERE tracker_path IS NOT NULL AND company_overview IS NULL",
            )?;
            let rows = stmt.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;
            rows.collect::<Result<Vec<_>, _>>()?
        };

        for (account_id, _account_name) in &accounts {
            if let Ok(Some(account)) = self.get_account(account_id) {
                let account_dir = crate::accounts::resolve_account_dir(workspace, &account);
                let json_path = account_dir.join("dashboard.json");
                if json_path.exists() {
                    match crate::accounts::read_account_json(&json_path) {
                        Ok(result) => {
                            let ov_json = result
                                .json
                                .company_overview
                                .and_then(|ov| serde_json::to_string(&ov).ok());
                            let prg_json = if result.json.strategic_programs.is_empty() {
                                None
                            } else {
                                serde_json::to_string(&result.json.strategic_programs).ok()
                            };
                            let notes = result.json.notes;
                            let now = chrono::Utc::now().to_rfc3339();
                            if let Err(e) =
                                crate::services::derived_state::update_account_ai_columns_projection(
                                    self,
                                    account_id,
                                    ov_json.as_deref(),
                                    prg_json.as_deref(),
                                    notes.as_deref(),
                                    &now,
                                )
                            {
                                log::warn!(
                                    "I644 backfill: failed to update account {}: {}",
                                    account_id,
                                    e
                                );
                            } else {
                                count += 1;
                            }
                        }
                        Err(e) => {
                            log::warn!(
                                "I644 backfill: failed to read dashboard.json for account {}: {}",
                                account_id,
                                e
                            );
                        }
                    }
                }
            }
        }

        // Backfill projects
        let projects: Vec<(String, String)> = {
            let mut stmt = self.conn.prepare(
                "SELECT id, name FROM projects \
                 WHERE tracker_path IS NOT NULL AND description IS NULL",
            )?;
            let rows = stmt.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;
            rows.collect::<Result<Vec<_>, _>>()?
        };

        for (project_id, project_name) in &projects {
            let project_dir = crate::projects::project_dir(workspace, project_name);
            let json_path = project_dir.join("dashboard.json");
            if json_path.exists() {
                match crate::projects::read_project_json(&json_path) {
                    Ok(result) => {
                        let ms_json = if result.json.milestones.is_empty() {
                            None
                        } else {
                            serde_json::to_string(&result.json.milestones).ok()
                        };
                        let now = chrono::Utc::now().to_rfc3339();
                        if let Err(e) = self.conn.execute(
                            "UPDATE projects SET description = ?1, milestones = ?2, \
                             notes = ?3, updated_at = ?4 WHERE id = ?5",
                            rusqlite::params![
                                result.json.description,
                                ms_json,
                                result.json.notes,
                                now,
                                project_id
                            ],
                        ) {
                            log::warn!(
                                "I644 backfill: failed to update project {}: {}",
                                project_id,
                                e
                            );
                        } else {
                            count += 1;
                        }
                    }
                    Err(e) => {
                        log::warn!(
                            "I644 backfill: failed to read dashboard.json for project {}: {}",
                            project_id,
                            e
                        );
                    }
                }
            }
        }

        Self::mark_init_task_completed(&self.conn, TASK_NAME)?;

        Ok(count)
    }
}

// =============================================================================
// Shared test utilities
// =============================================================================

#[cfg(test)]
pub mod test_utils {
    use super::ActionDb;

    /// Create an isolated migrated in-memory database for testing.
    ///
    /// FK enforcement is disabled so unit tests can insert rows without satisfying
    /// every foreign key constraint.
    pub fn test_db() -> ActionDb {
        let db =
            ActionDb::from_connection_for_tests(crate::migrations::migrated_in_memory_for_tests());
        db.conn_ref()
            .execute_batch("PRAGMA foreign_keys = OFF;")
            .expect("disable FK for tests");
        db
    }
}

#[cfg(test)]
mod db_mode_tests {
    use super::*;

    static DB_MODE_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    struct ResetDbMode;

    impl Drop for ResetDbMode {
        fn drop(&mut self) {
            set_db_mode(DbMode::Live);
        }
    }

    fn production_db_path() -> PathBuf {
        dailyos_data_dir().expect("data dir").join("dailyos.db")
    }

    fn production_db_path_with_redundant_dot_segment() -> PathBuf {
        dailyos_data_dir()
            .expect("data dir")
            .join(".")
            .join("dailyos.db")
    }

    fn assert_prod_open_denied(mode: DbMode) {
        set_db_mode(mode);
        let err = guard_path_for_mode(&production_db_path())
            .expect_err("non-Live mode must deny production DB path");
        assert!(
            matches!(err, DbError::ProdOpenDenied { .. }),
            "expected ProdOpenDenied, got {err:?}"
        );
    }

    fn assert_readonly_prod_open_denied(path: &Path, mode: DbMode) {
        set_db_mode(mode);
        match ActionDb::open_readonly_at(path, Arc::new(FixtureDbKeyProvider::new())) {
            Err(DbError::ProdOpenDenied { .. }) => {}
            Err(err) => panic!("expected ProdOpenDenied, got {err:?}"),
            Ok(_) => panic!("non-Live mode must deny production DB read path"),
        }
    }

    fn create_encrypted_prod_db(path: &Path) {
        std::fs::create_dir_all(path.parent().expect("prod db parent"))
            .expect("create prod db parent");
        let provider = FixtureDbKeyProvider::new();
        let conn = Connection::open(path).expect("create encrypted prod db");
        conn.execute_batch(&provider.key.to_pragma())
            .expect("apply fixture key");
        conn.execute_batch("CREATE TABLE readonly_smoke (id INTEGER PRIMARY KEY);")
            .expect("create smoke table");
    }

    #[test]
    fn replica_mode_refuses_production_db_path() {
        let _lock = DB_MODE_TEST_LOCK.lock().expect("db mode test lock");
        let _reset = ResetDbMode;

        assert_prod_open_denied(DbMode::Replica);
    }

    #[test]
    fn mock_mode_refuses_production_db_path() {
        let _lock = DB_MODE_TEST_LOCK.lock().expect("db mode test lock");
        let _reset = ResetDbMode;

        assert_prod_open_denied(DbMode::Mock);
    }

    #[test]
    fn live_mode_allows_production_db_path() {
        let _lock = DB_MODE_TEST_LOCK.lock().expect("db mode test lock");
        let _reset = ResetDbMode;

        set_db_mode(DbMode::Live);

        guard_path_for_mode(&production_db_path())
            .expect("Live mode must allow production DB path");
    }

    #[test]
    fn unrelated_temp_path_is_allowed_in_every_mode() {
        let _lock = DB_MODE_TEST_LOCK.lock().expect("db mode test lock");
        let _reset = ResetDbMode;
        let temp = tempfile::tempdir().expect("tempdir");
        let unrelated_path = temp.path().join("dailyos.db");

        for mode in [DbMode::Live, DbMode::Replica, DbMode::Mock] {
            set_db_mode(mode);
            guard_path_for_mode(&unrelated_path)
                .unwrap_or_else(|err| panic!("{mode:?} should allow unrelated temp path: {err}"));
        }
    }

    #[test]
    fn open_readonly_at_refuses_prod_path_in_replica_and_mock() {
        let _lock = DB_MODE_TEST_LOCK.lock().expect("db mode test lock");
        let _reset = ResetDbMode;
        let prod_path = production_db_path();

        assert_readonly_prod_open_denied(&prod_path, DbMode::Replica);
        assert_readonly_prod_open_denied(&prod_path, DbMode::Mock);
    }

    #[test]
    fn open_readonly_at_refuses_prod_path_with_redundant_dot_segment() {
        let _lock = DB_MODE_TEST_LOCK.lock().expect("db mode test lock");
        let _reset = ResetDbMode;
        let redundant_path = production_db_path_with_redundant_dot_segment();

        assert_readonly_prod_open_denied(&redundant_path, DbMode::Replica);
    }

    #[test]
    fn open_readonly_at_allows_prod_path_in_live() {
        let _lock = DB_MODE_TEST_LOCK.lock().expect("db mode test lock");
        let _reset = ResetDbMode;
        let prod_path = production_db_path();
        create_encrypted_prod_db(&prod_path);

        set_db_mode(DbMode::Live);
        ActionDb::open_readonly_at(&prod_path, Arc::new(FixtureDbKeyProvider::new()))
            .expect("Live mode must allow production DB read path");
    }

    #[test]
    fn live_mode_db_paths_stay_under_test_data_dir() {
        let _lock = DB_MODE_TEST_LOCK.lock().expect("db mode test lock");
        let _reset = ResetDbMode;

        set_db_mode(DbMode::Live);

        let test_data_dir = dailyos_data_dir().expect("test data dir");
        let db_path = ActionDb::db_path().expect("db path");
        let prod_paths = prod_db_paths();
        let real_dailyos_dir = dirs::home_dir().expect("home dir").join(".dailyos");

        assert!(test_data_dir.starts_with(std::env::temp_dir()));
        assert_ne!(test_data_dir, real_dailyos_dir);
        assert!(db_path.starts_with(&test_data_dir));
        assert!(!db_path.starts_with(&real_dailyos_dir));
        assert!(!prod_paths.is_empty());
        for prod_path in prod_paths {
            assert!(prod_path.starts_with(&test_data_dir));
            assert!(!prod_path.starts_with(&real_dailyos_dir));
        }
    }

    #[test]
    fn live_mode_resolves_config_workspace_and_google_token_to_live_paths() {
        let _lock = DB_MODE_TEST_LOCK.lock().expect("db mode test lock");
        let _reset = ResetDbMode;

        set_db_mode(DbMode::Live);
        let home = dirs::home_dir().expect("home dir");
        let dailyos_dir = dailyos_data_dir().expect("data dir");
        let configured_workspace = home.join("Documents").join("DailyOS");
        let configured_workspace_str = configured_workspace.to_string_lossy().to_string();

        assert_eq!(
            crate::state::config_path().expect("config path"),
            dailyos_dir.join("config.json")
        );
        assert_eq!(
            crate::state::resolved_workspace_path(Some(&configured_workspace_str))
                .expect("workspace path"),
            configured_workspace
        );
        assert_eq!(
            crate::state::google_token_path(),
            dailyos_dir.join("google").join("token.json")
        );
        assert_eq!(
            crate::google_api::token_path(),
            crate::state::google_token_path()
        );
    }

    #[test]
    fn non_live_modes_resolve_config_workspace_and_google_token_to_isolated_paths() {
        let _lock = DB_MODE_TEST_LOCK.lock().expect("db mode test lock");
        let _reset = ResetDbMode;

        let home = dirs::home_dir().expect("home dir");
        let dailyos_dir = dailyos_data_dir().expect("data dir");
        let live_config = dailyos_dir.join("config.json");
        let live_workspace = home.join("Documents").join("DailyOS");
        let live_workspace_str = live_workspace.to_string_lossy().to_string();
        let live_token = dailyos_dir.join("google").join("token.json");

        for (mode, config_name, workspace_name, state_name) in [
            (
                DbMode::Replica,
                "config-replica.json",
                "replica-workspace",
                "replica",
            ),
            (DbMode::Mock, "config-dev.json", "dev-workspace", "dev"),
        ] {
            set_db_mode(mode);

            let config_path = crate::state::config_path().expect("config path");
            let workspace_path = crate::state::resolved_workspace_path(Some(&live_workspace_str))
                .expect("workspace path");
            let google_token_path = crate::state::google_token_path();
            let state_dir = dailyos_dir.join(state_name);

            assert_eq!(config_path, dailyos_dir.join(config_name));
            assert_eq!(workspace_path, dailyos_dir.join(workspace_name));
            assert_eq!(
                google_token_path,
                state_dir.join("google").join("token.json")
            );
            assert_eq!(crate::google_api::token_path(), google_token_path);
            assert_eq!(
                crate::audit_log::default_audit_log_path(),
                state_dir.join("audit.log")
            );

            assert_ne!(config_path, live_config);
            assert_ne!(workspace_path, live_workspace);
            assert_ne!(google_token_path, live_token);
            assert!(workspace_path.starts_with(&dailyos_dir));
            assert!(google_token_path.starts_with(&state_dir));
        }
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
