//! Database encryption key providers.
//!
//! `LocalKeychain` preserves the ADR-0092 SQLCipher key behavior: a 256-bit
//! random hex key is stored in the macOS Keychain under the existing service
//! and account, cached process-wide after first access, and never generated
//! for an encrypted database whose Keychain entry is missing.

use rand::Rng;
use rusqlite::Connection;
use std::fmt;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use parking_lot::RwLock;
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

use super::encryption;

const KEYCHAIN_SERVICE: &str = "com.dailyos.desktop.db";
const KEYCHAIN_ACCOUNT: &str = "sqlcipher-key";
const KEYCHAIN_ROTATION_ACCOUNT_PREFIX: &str = "sqlcipher-key.rotation";
const KEYCHAIN_ROTATION_IN_PROGRESS_ACCOUNT: &str = "sqlcipher-key.rotation_in_progress";
const KEYCHAIN_ROTATION_IN_PROGRESS_VALUE: &str =
    "0000000000000000000000000000000000000000000000000000000000000001";
const ROTATION_WAIT_TIMEOUT: Duration = Duration::from_secs(10);
const ROTATION_WAIT_POLL_INTERVAL: Duration = Duration::from_millis(10);

/// Set to `true` when a new key is generated (fresh install).
static KEY_WAS_GENERATED: AtomicBool = AtomicBool::new(false);

/// Process-wide rotation guard. This prevents `get_or_create_key` from
/// validating with stale cached key material while `rotate_key` is between
/// primary Keychain update and database rekey.
static ROTATION_IN_PROGRESS: AtomicBool = AtomicBool::new(false);

/// Process-wide open/rotation exclusion. `ActionDb::open` holds the read side
/// from key fetch through fresh connection acquisition; key rotation holds the
/// write side until the keychain and database have moved together.
static ROTATION_LOCK: RwLock<()> = parking_lot::const_rwlock(());

/// Process-wide cached key. This intentionally preserves the old
/// `OnceLock<String>` semantics for normal key loads: first successful key wins
/// and later duplicate set attempts are ignored. Rotation is the only path
/// allowed to replace the cached value.
static CACHED_KEY: OnceLock<RwLock<Option<EncryptionKey>>> = OnceLock::new();

pub(crate) fn rotation_lock_read() -> parking_lot::RwLockReadGuard<'static, ()> {
    ROTATION_LOCK.read()
}

pub(crate) fn rotation_lock_write() -> parking_lot::RwLockWriteGuard<'static, ()> {
    ROTATION_LOCK.write()
}

fn cached_key() -> &'static RwLock<Option<EncryptionKey>> {
    CACHED_KEY.get_or_init(|| RwLock::new(None))
}

fn read_cached_key() -> Option<EncryptionKey> {
    cached_key().read().clone()
}

fn set_cached_key_once(key: EncryptionKey) {
    let mut guard = cached_key().write();
    if guard.is_none() {
        *guard = Some(key);
    }
}

fn replace_cached_key(key: EncryptionKey) {
    *cached_key().write() = Some(key);
}

#[cfg(test)]
pub(crate) fn set_cached_db_key_for_tests(hex_key: &str) {
    set_cached_key_once(EncryptionKey::from_hex(hex_key.to_string()));
}

/// Whether the last `LocalKeychain::get_or_create_key` call generated a new key.
pub fn was_key_generated() -> bool {
    KEY_WAS_GENERATED.load(Ordering::Relaxed)
}

/// Identity metadata supplied to a `DbKeyProvider`.
///
/// Today the local provider only needs the database path because ADR-0092's
/// lost-key guard depends on whether that path already contains encrypted
/// bytes. Future tenant-aware providers can extend the construction site
/// without changing the `ActionDb` call shape.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UserIdentity {
    db_path: PathBuf,
}

impl UserIdentity {
    pub fn local(db_path: impl Into<PathBuf>) -> Self {
        Self {
            db_path: db_path.into(),
        }
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }
}

/// Opaque SQLCipher key material.
#[derive(Clone, PartialEq, Eq)]
pub struct EncryptionKey {
    hex: Zeroizing<String>,
}

impl EncryptionKey {
    pub(crate) fn from_hex(hex_key: String) -> Self {
        Self {
            hex: Zeroizing::new(hex_key),
        }
    }

    pub(crate) fn as_hex(&self) -> &str {
        self.hex.as_str()
    }

    /// Format this key into SQLCipher's raw-hex PRAGMA form.
    pub fn to_pragma(&self) -> SqlCipherPragma {
        SqlCipherPragma::new(encryption::key_to_pragma(self.as_hex()))
    }
}

impl Zeroize for EncryptionKey {
    fn zeroize(&mut self) {
        self.hex.zeroize();
    }
}

impl ZeroizeOnDrop for EncryptionKey {}

impl fmt::Debug for EncryptionKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("EncryptionKey([REDACTED])")
    }
}

/// SQLCipher PRAGMA text containing raw key material.
#[derive(Clone, PartialEq, Eq)]
pub struct SqlCipherPragma {
    sql: Zeroizing<String>,
}

impl SqlCipherPragma {
    fn new(sql: String) -> Self {
        Self {
            sql: Zeroizing::new(sql),
        }
    }

    pub fn as_str(&self) -> &str {
        self.sql.as_str()
    }
}

impl Deref for SqlCipherPragma {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl AsRef<str> for SqlCipherPragma {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Zeroize for SqlCipherPragma {
    fn zeroize(&mut self) {
        self.sql.zeroize();
    }
}

impl ZeroizeOnDrop for SqlCipherPragma {}

impl fmt::Debug for SqlCipherPragma {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SqlCipherPragma([REDACTED])")
    }
}

pub type Result<T> = std::result::Result<T, String>;

pub trait DbKeyProvider: Send + Sync {
    #[must_use = "check whether DB key was loaded or created before opening encrypted database"]
    fn get_or_create_key(&self, user: &UserIdentity) -> Result<EncryptionKey>;
    #[must_use = "check whether DB key rotation completed before relying on the returned key"]
    fn rotate_key(&self, user: &UserIdentity) -> Result<EncryptionKey>;
}

trait KeychainBackend: Send + Sync {
    fn get_key(&self, account: &str) -> Result<EncryptionKey>;
    fn upsert_key(&self, account: &str, key: &EncryptionKey) -> Result<()>;
    fn delete_key(&self, account: &str) -> Result<()>;

    fn rotation_accounts(&self) -> Result<Vec<String>> {
        Ok(vec![KEYCHAIN_ROTATION_ACCOUNT_PREFIX.to_string()])
    }
}

#[derive(Debug, Default)]
struct SecurityCliKeychain;

impl KeychainBackend for SecurityCliKeychain {
    fn get_key(&self, account: &str) -> Result<EncryptionKey> {
        get_key_from_keychain_account(account)
    }

    fn upsert_key(&self, account: &str, key: &EncryptionKey) -> Result<()> {
        upsert_keychain_account(account, key)
    }

    fn delete_key(&self, account: &str) -> Result<()> {
        delete_keychain_account(account)
    }

    fn rotation_accounts(&self) -> Result<Vec<String>> {
        let mut accounts = Vec::new();
        if self.get_key(KEYCHAIN_ROTATION_ACCOUNT_PREFIX).is_ok() {
            accounts.push(KEYCHAIN_ROTATION_ACCOUNT_PREFIX.to_string());
        }

        if let Ok(output) = std::process::Command::new("security")
            .arg("dump-keychain")
            .output()
        {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                accounts.extend(parse_rotation_accounts_from_keychain_dump(&stdout));
            }
        }

        accounts.sort();
        accounts.dedup();
        Ok(accounts)
    }
}

pub struct LocalKeychain {
    keychain: Arc<dyn KeychainBackend>,
    use_process_cache: bool,
}

impl fmt::Debug for LocalKeychain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LocalKeychain").finish_non_exhaustive()
    }
}

impl Default for LocalKeychain {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum KeyAccessMode {
    Normal,
    DuringRotation,
    RecoveringRotation,
}

impl KeyAccessMode {
    fn is_recovering_rotation(self) -> bool {
        self == Self::RecoveringRotation
    }
}

struct RotationGuard<'a> {
    provider: &'a LocalKeychain,
}

impl Drop for RotationGuard<'_> {
    fn drop(&mut self) {
        self.provider.clear_rotation_in_progress_flag_logged();
        ROTATION_IN_PROGRESS.store(false, Ordering::Release);
    }
}

impl LocalKeychain {
    pub fn new() -> Self {
        Self {
            keychain: Arc::new(SecurityCliKeychain),
            use_process_cache: true,
        }
    }

    #[cfg(test)]
    fn with_keychain_for_tests(keychain: Arc<dyn KeychainBackend>) -> Self {
        Self {
            keychain,
            use_process_cache: false,
        }
    }

    /// Retrieve the existing DB key without creating a Keychain entry.
    pub fn get_existing_key(&self) -> Result<EncryptionKey> {
        if self.use_process_cache {
            if let Some(key) = read_cached_key() {
                return Ok(key);
            }
        }

        let key = self.get_existing_key_uncached()?;
        self.cache_key_once(key.clone());
        Ok(key)
    }

    /// Check if a key exists in the Keychain without using the process cache.
    pub fn has_key(&self) -> bool {
        self.get_existing_key_uncached().is_ok()
    }

    /// Delete the DB key from Keychain. Used for testing/recovery only.
    #[must_use = "check whether keychain entry was deleted before treating encrypted DB as reset"]
    pub fn delete_key(&self) -> Result<()> {
        self.keychain.delete_key(KEYCHAIN_ACCOUNT)
    }

    fn get_existing_key_uncached(&self) -> Result<EncryptionKey> {
        self.keychain.get_key(KEYCHAIN_ACCOUNT)
    }

    fn cache_key_once(&self, key: EncryptionKey) {
        if self.use_process_cache {
            set_cached_key_once(key);
        }
    }

    fn replace_cached_key(&self, key: EncryptionKey) {
        if self.use_process_cache {
            replace_cached_key(key);
        }
    }

    fn wait_for_rotation_to_finish(&self) -> Result<()> {
        let deadline = Instant::now() + ROTATION_WAIT_TIMEOUT;
        while ROTATION_IN_PROGRESS.load(Ordering::Acquire) {
            if Instant::now() >= deadline {
                return Err(format!(
                    "Timed out waiting {}s for DB key rotation to finish",
                    ROTATION_WAIT_TIMEOUT.as_secs()
                ));
            }
            thread::sleep(ROTATION_WAIT_POLL_INTERVAL);
        }
        Ok(())
    }

    fn begin_rotation(&self) -> Result<RotationGuard<'_>> {
        let deadline = Instant::now() + ROTATION_WAIT_TIMEOUT;
        loop {
            if ROTATION_IN_PROGRESS
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                break;
            }

            if Instant::now() >= deadline {
                return Err(format!(
                    "Timed out waiting {}s to start DB key rotation",
                    ROTATION_WAIT_TIMEOUT.as_secs()
                ));
            }
            thread::sleep(ROTATION_WAIT_POLL_INTERVAL);
        }

        if let Err(error) = self.set_rotation_in_progress_flag() {
            ROTATION_IN_PROGRESS.store(false, Ordering::Release);
            return Err(format!(
                "Failed to set DB key rotation-in-progress flag: {error}"
            ));
        }

        Ok(RotationGuard { provider: self })
    }

    fn set_rotation_in_progress_flag(&self) -> Result<()> {
        let flag_value = EncryptionKey::from_hex(KEYCHAIN_ROTATION_IN_PROGRESS_VALUE.to_string());
        self.keychain
            .upsert_key(KEYCHAIN_ROTATION_IN_PROGRESS_ACCOUNT, &flag_value)
    }

    fn clear_rotation_in_progress_flag(&self) -> Result<()> {
        if self
            .keychain
            .get_key(KEYCHAIN_ROTATION_IN_PROGRESS_ACCOUNT)
            .is_err()
        {
            return Ok(());
        }
        self.keychain
            .delete_key(KEYCHAIN_ROTATION_IN_PROGRESS_ACCOUNT)
    }

    fn clear_rotation_in_progress_flag_logged(&self) {
        if let Err(error) = self.clear_rotation_in_progress_flag() {
            log::warn!("Failed to clear DB key rotation-in-progress flag: {error}");
        }
    }

    fn keychain_rotation_in_progress(&self) -> bool {
        self.keychain
            .get_key(KEYCHAIN_ROTATION_IN_PROGRESS_ACCOUNT)
            .is_ok()
    }

    fn staged_cleanup_blocked_by_rotation(&self) -> bool {
        ROTATION_IN_PROGRESS.load(Ordering::Acquire) || self.keychain_rotation_in_progress()
    }

    fn finish_rotation_recovery(&self, staged_accounts: &[String]) -> Result<()> {
        self.clear_rotation_in_progress_flag()
            .map_err(|error| format!("DB key rotation recovery succeeded, but failed to clear rotation-in-progress flag: {error}"))?;
        self.cleanup_staged_keychain_keys_unchecked(staged_accounts);
        Ok(())
    }

    fn get_or_create_key_inner(
        &self,
        user: &UserIdentity,
        mode: KeyAccessMode,
    ) -> Result<EncryptionKey> {
        let db_path = user.db_path();
        let key = match self.get_existing_key() {
            Ok(key) => key,
            Err(_e) => {
                // DB exists and is not plaintext -> encrypted with a lost key.
                // Return a KEY_MISSING marker so callers can distinguish this
                // from other encryption errors and show a recovery screen.
                if db_path.exists() && !encryption::is_database_plaintext(db_path) {
                    return Err(format!("KEY_MISSING:{}", db_path.display()));
                }
                // No DB yet (fresh install) or plaintext DB (pre-migration) -> safe to create key.
                let new_key = generate_key();
                self.keychain.upsert_key(KEYCHAIN_ACCOUNT, &new_key)?;
                KEY_WAS_GENERATED.store(true, Ordering::Relaxed);
                new_key
            }
        };

        let should_validate = if mode.is_recovering_rotation() {
            encrypted_database_exists(db_path)
        } else {
            should_validate_existing_database_key(db_path)
        };

        if should_validate {
            self.verify_or_recover_existing_key(db_path, key, mode)
        } else {
            self.cache_key_once(key.clone());
            if mode.is_recovering_rotation() {
                self.finish_rotation_recovery(&[])?;
            }
            Ok(key)
        }
    }

    fn verify_or_recover_existing_key(
        &self,
        db_path: &Path,
        candidate_key: EncryptionKey,
        mode: KeyAccessMode,
    ) -> Result<EncryptionKey> {
        match verify_database_key(db_path, &candidate_key) {
            Ok(()) => {
                if mode.is_recovering_rotation() {
                    self.finish_rotation_recovery(&[KEYCHAIN_ROTATION_ACCOUNT_PREFIX.to_string()])?;
                } else {
                    self.cleanup_staged_keychain_key_if_present(KEYCHAIN_ROTATION_ACCOUNT_PREFIX);
                }
                self.cache_key_once(candidate_key.clone());
                Ok(candidate_key)
            }
            Err(candidate_error) => {
                let primary_key = self.get_existing_key_uncached().map_err(|error| {
                    format!(
                        "SQLCipher primary key verification failed: {candidate_error}; failed to reload primary Keychain account: {error}"
                    )
                })?;

                if primary_key != candidate_key
                    && verify_database_key(db_path, &primary_key).is_ok()
                {
                    if mode.is_recovering_rotation() {
                        self.finish_rotation_recovery(&[
                            KEYCHAIN_ROTATION_ACCOUNT_PREFIX.to_string()
                        ])?;
                    } else {
                        self.cleanup_staged_keychain_key_if_present(
                            KEYCHAIN_ROTATION_ACCOUNT_PREFIX,
                        );
                    }
                    self.replace_cached_key(primary_key.clone());
                    return Ok(primary_key);
                }

                self.recover_key_from_staged_key(db_path, &primary_key, candidate_error, mode)
            }
        }
    }

    fn recover_key_from_staged_key(
        &self,
        db_path: &Path,
        primary_key: &EncryptionKey,
        primary_error: String,
        mode: KeyAccessMode,
    ) -> Result<EncryptionKey> {
        let staged_keys = self.staged_keychain_keys().map_err(|error| {
            format!(
                "SQLCipher primary key verification failed: {primary_error}; failed to read staged rotation keys: {error}"
            )
        })?;

        if staged_keys.is_empty() {
            return Err(format!(
                "SQLCipher primary key verification failed: {primary_error}; no staged rotation key was available"
            ));
        }

        let staged_accounts: Vec<String> = staged_keys
            .iter()
            .map(|staged| staged.account.clone())
            .collect();
        let mut last_staged_error = None;

        for staged in staged_keys {
            match verify_database_key(db_path, &staged.key) {
                Ok(()) => {
                    if staged.key != *primary_key {
                        match rekey_database(db_path, &staged.key, primary_key) {
                            Ok(()) => {}
                            Err(rekey_error) => {
                                if let Err(verify_error) = verify_database_key(db_path, primary_key)
                                {
                                    return Err(format!(
                                        "SQLCipher primary key verification failed: {primary_error}; staged rotation key {} opened the database, but recovery rekey to primary failed: {rekey_error}; primary verification after recovery failed: {verify_error}",
                                        staged.account
                                    ));
                                }
                            }
                        }
                    }

                    if mode.is_recovering_rotation() {
                        self.finish_rotation_recovery(&staged_accounts)?;
                    } else {
                        self.cleanup_staged_keychain_keys(&staged_accounts);
                    }
                    self.replace_cached_key(primary_key.clone());
                    return Ok(primary_key.clone());
                }
                Err(error) => {
                    last_staged_error = Some(format!("{}: {error}", staged.account));
                }
            }
        }

        let staged_detail = last_staged_error
            .map(|error| format!("; last staged key verification failed: {error}"))
            .unwrap_or_default();
        Err(format!(
            "SQLCipher primary key verification failed: {primary_error}; staged rotation keys did not open the database{staged_detail}"
        ))
    }

    fn staged_keychain_keys(&self) -> Result<Vec<StagedKeychainKey>> {
        let staged_keys = self
            .keychain
            .rotation_accounts()?
            .into_iter()
            .filter(|account| account.starts_with(KEYCHAIN_ROTATION_ACCOUNT_PREFIX))
            .filter(|account| account != KEYCHAIN_ROTATION_IN_PROGRESS_ACCOUNT)
            .filter_map(|account| match self.keychain.get_key(&account) {
                Ok(key) => Some(StagedKeychainKey { account, key }),
                Err(error) => {
                    log::warn!("Failed to read staged DB keychain entry {account}: {error}");
                    None
                }
            })
            .collect::<Vec<_>>();
        Ok(staged_keys)
    }

    fn cleanup_staged_keychain_key_if_present(&self, account: &str) {
        if self.staged_cleanup_blocked_by_rotation() {
            log::info!("Preserving staged DB keychain entry {account} during key rotation");
            return;
        }

        if self.keychain.get_key(account).is_ok() {
            self.cleanup_staged_keychain_key(account);
        }
    }

    fn cleanup_staged_keychain_keys(&self, accounts: &[String]) {
        if self.staged_cleanup_blocked_by_rotation() {
            log::info!("Preserving staged DB keychain entries during key rotation");
            return;
        }

        self.cleanup_staged_keychain_keys_unchecked(accounts);
    }

    fn cleanup_staged_keychain_keys_unchecked(&self, accounts: &[String]) {
        for account in accounts {
            self.cleanup_staged_keychain_key(account);
        }
    }

    fn cleanup_staged_keychain_key(&self, account: &str) {
        if let Err(error) = self.keychain.delete_key(account) {
            log::warn!("Failed to delete staged DB keychain entry {account}: {error}");
        }
    }
}

impl DbKeyProvider for LocalKeychain {
    fn get_or_create_key(&self, user: &UserIdentity) -> Result<EncryptionKey> {
        self.wait_for_rotation_to_finish()?;
        let mode = if self.keychain_rotation_in_progress() {
            KeyAccessMode::RecoveringRotation
        } else {
            KeyAccessMode::Normal
        };
        self.get_or_create_key_inner(user, mode)
    }

    fn rotate_key(&self, user: &UserIdentity) -> Result<EncryptionKey> {
        let _rotation_lock = rotation_lock_write();
        let _rotation_guard = self.begin_rotation()?;
        let db_path = user.db_path();
        let old_key = self.get_or_create_key_inner(user, KeyAccessMode::DuringRotation)?;
        let new_key = generate_key();
        let db_needs_rekey = encrypted_database_exists(db_path);
        let staged_account = if db_needs_rekey {
            Some(stage_key_in_keychain(self.keychain.as_ref(), &old_key)?)
        } else {
            None
        };

        if let Err(store_error) = self.keychain.upsert_key(KEYCHAIN_ACCOUNT, &new_key) {
            if let Some(account) = &staged_account {
                self.cleanup_staged_keychain_key(account);
            }
            return Err(store_error);
        }

        if db_needs_rekey {
            if let Err(error) = rekey_database(db_path, &old_key, &new_key) {
                if verify_database_key(db_path, &old_key).is_ok() {
                    if let Err(restore_error) = self.keychain.upsert_key(KEYCHAIN_ACCOUNT, &old_key)
                    {
                        let account = staged_account.as_deref().unwrap_or("<none>");
                        return Err(format!(
                            "{error}; database still opens with the original key, but restoring the primary Keychain account failed: {restore_error}; staged original key remains in Keychain account {account}"
                        ));
                    }
                    if let Some(account) = &staged_account {
                        self.cleanup_staged_keychain_key(account);
                    }
                    self.replace_cached_key(old_key.clone());
                    return Err(format!(
                        "{error}; restored primary Keychain account to the original key"
                    ));
                }

                if verify_database_key(db_path, &new_key).is_ok() {
                    if let Some(account) = &staged_account {
                        self.cleanup_staged_keychain_key(account);
                    }
                    self.replace_cached_key(new_key.clone());
                    return Err(format!(
                        "{error}; database opens with the new key and primary Keychain account already contains it"
                    ));
                }

                let account = staged_account.as_deref().unwrap_or("<none>");
                return Err(format!(
                    "{error}; staged original key remains in Keychain account {account}"
                ));
            }
        }

        if let Some(account) = &staged_account {
            self.cleanup_staged_keychain_key(account);
        }
        self.replace_cached_key(new_key.clone());
        Ok(new_key)
    }
}

struct StagedKeychainKey {
    account: String,
    key: EncryptionKey,
}

#[must_use = "database rekey failures must be handled"]
pub(crate) fn rekey_database(
    db_path: &Path,
    old_key: &EncryptionKey,
    new_key: &EncryptionKey,
) -> Result<()> {
    if let Some(svc) = crate::db_service::try_global() {
        if svc.db_path() == db_path {
            return svc.rekey_database(db_path, old_key, new_key);
        }
    }

    rekey_database_standalone(db_path, old_key, new_key)
}

#[must_use = "database rekey failures must be handled"]
pub(crate) fn rekey_database_standalone(
    db_path: &Path,
    old_key: &EncryptionKey,
    new_key: &EncryptionKey,
) -> Result<()> {
    rekey_database_standalone_inner(db_path, old_key, new_key, true)
}

fn rekey_database_standalone_inner(
    db_path: &Path,
    old_key: &EncryptionKey,
    new_key: &EncryptionKey,
    rollback_after_rekey_failure: bool,
) -> Result<()> {
    let conn = Connection::open(db_path)
        .map_err(|e| format!("Failed to open encrypted DB for key rotation: {e}"))?;
    conn.execute_batch(&old_key.to_pragma())
        .map_err(|e| format!("Failed to apply existing DB key for rotation: {e}"))?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;")
        .map_err(|e| format!("Failed to set busy timeout before key rotation: {e}"))?;
    conn.execute_batch("PRAGMA locking_mode = EXCLUSIVE;")
        .map_err(|e| format!("Failed to take exclusive DB lock for key rotation: {e}"))?;
    conn.query_row("SELECT count(*) FROM sqlite_master LIMIT 1", [], |row| {
        row.get::<_, i64>(0)
    })
    .map_err(|e| format!("SQLCipher key verification failed before rotation: {e}"))?;
    conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
        .map_err(|e| format!("Failed to checkpoint WAL before key rotation: {e}"))?;
    conn.execute_batch(&key_to_rekey_pragma(new_key))
        .map_err(|e| format!("SQLCipher rekey failed: {e}"))?;

    if let Err(error) = conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);") {
        drop(conn);
        return rollback_after_completed_rekey(
            db_path,
            new_key,
            old_key,
            format!("Failed to checkpoint WAL after key rotation: {error}"),
            rollback_after_rekey_failure,
        );
    }

    drop(conn);
    if let Err(error) = verify_database_key(db_path, new_key) {
        return rollback_after_completed_rekey(
            db_path,
            new_key,
            old_key,
            format!("SQLCipher key verification failed after rotation: {error}"),
            rollback_after_rekey_failure,
        );
    }
    Ok(())
}

fn rollback_after_completed_rekey(
    db_path: &Path,
    current_key: &EncryptionKey,
    previous_key: &EncryptionKey,
    original_error: String,
    rollback_after_rekey_failure: bool,
) -> Result<()> {
    if !rollback_after_rekey_failure {
        return Err(original_error);
    }

    match rekey_database_standalone_inner(db_path, current_key, previous_key, false) {
        Ok(()) => Err(format!(
            "{original_error}; rollback to original DB key succeeded"
        )),
        Err(rollback_error) => Err(format!(
            "{original_error}; rollback to original DB key failed: {rollback_error}"
        )),
    }
}

fn verify_database_key(db_path: &Path, key: &EncryptionKey) -> Result<()> {
    let conn = Connection::open(db_path)
        .map_err(|e| format!("Failed to reopen encrypted DB for key verification: {e}"))?;
    conn.execute_batch(&key.to_pragma())
        .map_err(|e| format!("Failed to apply DB key for verification: {e}"))?;
    conn.query_row("SELECT count(*) FROM sqlite_master LIMIT 1", [], |row| {
        row.get::<_, i64>(0)
    })
    .map_err(|e| format!("SQLCipher key verification query failed: {e}"))?;
    Ok(())
}

fn encrypted_database_exists(db_path: &Path) -> bool {
    db_path.exists() && !encryption::is_database_plaintext(db_path)
}

fn should_validate_existing_database_key(db_path: &Path) -> bool {
    encrypted_database_exists(db_path) && !global_db_service_owns_path(db_path)
}

fn global_db_service_owns_path(db_path: &Path) -> bool {
    crate::db_service::try_global().is_some_and(|svc| svc.db_path() == db_path)
}

pub(crate) fn key_to_rekey_pragma(key: &EncryptionKey) -> SqlCipherPragma {
    SqlCipherPragma::new(format!("PRAGMA rekey = \"x'{}'\"", key.as_hex()))
}

fn generate_key() -> EncryptionKey {
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    EncryptionKey::from_hex(hex::encode(bytes))
}

fn parse_rotation_accounts_from_keychain_dump(dump: &str) -> Vec<String> {
    let mut accounts = Vec::new();
    let mut current_account: Option<String> = None;
    let mut current_service: Option<String> = None;

    for line in dump.lines().chain(std::iter::once("")) {
        let line = line.trim_start();
        if line.starts_with("class:") || line.is_empty() {
            collect_rotation_account(
                &mut accounts,
                current_service.take(),
                current_account.take(),
            );
            continue;
        }

        if let Some(value) = parse_security_blob_attribute(line, "\"acct\"<blob>=") {
            current_account = Some(value);
        } else if let Some(value) = parse_security_blob_attribute(line, "\"svce\"<blob>=") {
            current_service = Some(value);
        }
    }

    accounts.sort();
    accounts.dedup();
    accounts
}

fn collect_rotation_account(
    accounts: &mut Vec<String>,
    service: Option<String>,
    account: Option<String>,
) {
    let Some(service) = service else {
        return;
    };
    let Some(account) = account else {
        return;
    };

    if service == KEYCHAIN_SERVICE
        && account.starts_with(KEYCHAIN_ROTATION_ACCOUNT_PREFIX)
        && account != KEYCHAIN_ROTATION_IN_PROGRESS_ACCOUNT
    {
        accounts.push(account);
    }
}

fn parse_security_blob_attribute(line: &str, prefix: &str) -> Option<String> {
    let value = line.strip_prefix(prefix)?.trim();
    if value == "<NULL>" {
        return None;
    }
    Some(value.trim_matches('"').to_string())
}

/// Read the encryption key from macOS Keychain via the `security` CLI.
///
/// Using the `security` binary instead of the `keyring` crate avoids the
/// repeated password prompt during development -- `security` is a trusted
/// system binary, so macOS doesn't re-prompt when the app binary changes
/// on every recompile.
fn get_key_from_keychain_account(account: &str) -> Result<EncryptionKey> {
    let output = std::process::Command::new("security")
        .args([
            "find-generic-password",
            "-s",
            KEYCHAIN_SERVICE,
            "-a",
            account,
            "-w", // output password only
        ])
        .output()
        .map_err(|e| format!("Failed to run security CLI: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Keychain read failed: {}", stderr.trim()));
    }

    let key = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if key.is_empty() {
        return Err("Keychain returned empty key".to_string());
    }
    Ok(EncryptionKey::from_hex(key))
}

fn stage_key_in_keychain(keychain: &dyn KeychainBackend, key: &EncryptionKey) -> Result<String> {
    let account = KEYCHAIN_ROTATION_ACCOUNT_PREFIX.to_string();
    keychain.upsert_key(&account, key)?;
    Ok(account)
}

fn upsert_keychain_account(account: &str, key: &EncryptionKey) -> Result<()> {
    let output = std::process::Command::new("security")
        .arg("add-generic-password")
        .arg("-s")
        .arg(KEYCHAIN_SERVICE)
        .arg("-a")
        .arg(account)
        .arg("-w")
        .arg(key.as_hex())
        .arg("-U")
        .output()
        .map_err(|e| format!("Failed to run security CLI: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Keychain write failed: {}", stderr.trim()));
    }
    Ok(())
}

fn delete_keychain_account(account: &str) -> Result<()> {
    let output = std::process::Command::new("security")
        .arg("delete-generic-password")
        .arg("-s")
        .arg(KEYCHAIN_SERVICE)
        .arg("-a")
        .arg(account)
        .output()
        .map_err(|e| format!("Failed to run security CLI: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "Failed to delete keychain entry: {}",
            stderr.trim()
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use parking_lot::Mutex;
    use rusqlite::Connection;
    use std::collections::HashMap;
    use std::sync::mpsc;
    use std::thread;
    use zeroize::ZeroizeOnDrop;

    #[derive(Debug, Default)]
    struct FakeKeychain {
        accounts: Mutex<HashMap<String, EncryptionKey>>,
    }

    impl KeychainBackend for FakeKeychain {
        fn get_key(&self, account: &str) -> Result<EncryptionKey> {
            self.accounts
                .lock()
                .get(account)
                .cloned()
                .ok_or_else(|| format!("missing fake keychain account: {account}"))
        }

        fn upsert_key(&self, account: &str, key: &EncryptionKey) -> Result<()> {
            self.accounts
                .lock()
                .insert(account.to_string(), key.clone());
            Ok(())
        }

        fn delete_key(&self, account: &str) -> Result<()> {
            self.accounts
                .lock()
                .remove(account)
                .map(|_| ())
                .ok_or_else(|| format!("missing fake keychain account: {account}"))
        }

        fn rotation_accounts(&self) -> Result<Vec<String>> {
            let mut accounts = self
                .accounts
                .lock()
                .keys()
                .filter(|account| account.starts_with(KEYCHAIN_ROTATION_ACCOUNT_PREFIX))
                .cloned()
                .collect::<Vec<_>>();
            accounts.sort();
            Ok(accounts)
        }
    }

    struct BlockingPrimaryUpsertKeychain {
        inner: FakeKeychain,
        primary_upsert_started: Mutex<Option<mpsc::Sender<()>>>,
        release_primary_upsert: Mutex<mpsc::Receiver<()>>,
        rotation_flag_seen: AtomicBool,
    }

    impl BlockingPrimaryUpsertKeychain {
        fn new(
            primary_upsert_started: mpsc::Sender<()>,
            release_primary_upsert: mpsc::Receiver<()>,
        ) -> Self {
            Self {
                inner: FakeKeychain::default(),
                primary_upsert_started: Mutex::new(Some(primary_upsert_started)),
                release_primary_upsert: Mutex::new(release_primary_upsert),
                rotation_flag_seen: AtomicBool::new(false),
            }
        }

        fn maybe_block_primary_upsert(&self, account: &str) {
            if account == KEYCHAIN_ROTATION_IN_PROGRESS_ACCOUNT {
                self.rotation_flag_seen.store(true, Ordering::Release);
                return;
            }

            if account == KEYCHAIN_ACCOUNT
                && self.rotation_flag_seen.load(Ordering::Acquire)
                && self.inner.get_key(KEYCHAIN_ACCOUNT).is_ok()
            {
                if let Some(sender) = self.primary_upsert_started.lock().take() {
                    sender
                        .send(())
                        .expect("signal blocked primary upsert started");
                }
                self.release_primary_upsert
                    .lock()
                    .recv()
                    .expect("release blocked primary upsert");
            }
        }
    }

    impl KeychainBackend for BlockingPrimaryUpsertKeychain {
        fn get_key(&self, account: &str) -> Result<EncryptionKey> {
            self.inner.get_key(account)
        }

        fn upsert_key(&self, account: &str, key: &EncryptionKey) -> Result<()> {
            self.maybe_block_primary_upsert(account);
            self.inner.upsert_key(account, key)
        }

        fn delete_key(&self, account: &str) -> Result<()> {
            self.inner.delete_key(account)
        }

        fn rotation_accounts(&self) -> Result<Vec<String>> {
            self.inner.rotation_accounts()
        }
    }

    fn fixed_key(hex: &str) -> EncryptionKey {
        EncryptionKey::from_hex(hex.to_string())
    }

    fn create_encrypted_test_database(db_path: &Path, key: &EncryptionKey) {
        let conn = Connection::open(db_path).expect("open encrypted test database");
        conn.execute_batch(&key.to_pragma())
            .expect("apply encryption key");
        conn.execute_batch(
            "CREATE TABLE recovery_probe (id INTEGER PRIMARY KEY, value TEXT NOT NULL);
             INSERT INTO recovery_probe (value) VALUES ('ok');",
        )
        .expect("initialize encrypted test database");
        drop(conn);

        verify_database_key(db_path, key).expect("created database opens with key");
    }

    #[test]
    fn get_or_create_waits_for_in_flight_rotation() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("blocked_rotation.db");
        let old_key = fixed_key("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef");
        let (primary_started_tx, primary_started_rx) = mpsc::channel();
        let (release_primary_tx, release_primary_rx) = mpsc::channel();
        let keychain = Arc::new(BlockingPrimaryUpsertKeychain::new(
            primary_started_tx,
            release_primary_rx,
        ));

        create_encrypted_test_database(&db_path, &old_key);
        keychain
            .upsert_key(KEYCHAIN_ACCOUNT, &old_key)
            .expect("store primary old key");

        let rotate_provider = LocalKeychain::with_keychain_for_tests(keychain.clone());
        let rotate_user = UserIdentity::local(db_path.clone());
        let rotate_handle = thread::spawn(move || rotate_provider.rotate_key(&rotate_user));

        primary_started_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("rotation reached blocked primary upsert");

        let get_provider = LocalKeychain::with_keychain_for_tests(keychain.clone());
        let get_user = UserIdentity::local(db_path.clone());
        let (get_done_tx, get_done_rx) = mpsc::channel();
        let get_handle = thread::spawn(move || {
            let key = get_provider.get_or_create_key(&get_user);
            get_done_tx.send(key).expect("send get_or_create result");
        });

        let early_get_result = get_done_rx.recv_timeout(Duration::from_millis(100));
        release_primary_tx
            .send(())
            .expect("release blocked primary upsert");

        assert!(
            early_get_result.is_err(),
            "get_or_create_key returned before rotation completed"
        );

        let rotated_key = rotate_handle
            .join()
            .expect("rotation thread joined")
            .expect("rotation completed");
        let observed_key = get_done_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("get_or_create returned after rotation")
            .expect("get_or_create succeeded");
        get_handle.join().expect("get_or_create thread joined");

        assert_eq!(observed_key, rotated_key);
        assert!(keychain
            .get_key(KEYCHAIN_ROTATION_IN_PROGRESS_ACCOUNT)
            .is_err());
        assert!(keychain.get_key(KEYCHAIN_ROTATION_ACCOUNT_PREFIX).is_err());
        verify_database_key(&db_path, &rotated_key).expect("DB uses rotated key");
        assert!(verify_database_key(&db_path, &old_key).is_err());
    }

    #[test]
    fn encryption_key_debug_redacts_key_material() {
        let key = EncryptionKey::from_hex(
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
        );

        assert_eq!(format!("{key:?}"), "EncryptionKey([REDACTED])");
    }

    #[test]
    fn local_keychain_implements_provider_trait() {
        fn assert_provider<T: DbKeyProvider>() {}
        assert_provider::<LocalKeychain>();
    }

    #[test]
    fn encryption_key_and_pragma_are_zeroize_on_drop() {
        fn assert_zeroize_on_drop<T: ZeroizeOnDrop>() {}

        assert_zeroize_on_drop::<EncryptionKey>();
        assert_zeroize_on_drop::<SqlCipherPragma>();
    }

    #[test]
    fn startup_recovers_crash_after_rekey_before_primary_key_update() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("rotation_recovery.db");
        let old_key = fixed_key("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef");
        let new_key = fixed_key("abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789");
        let keychain = Arc::new(FakeKeychain::default());

        create_encrypted_test_database(&db_path, &old_key);
        keychain
            .upsert_key(KEYCHAIN_ACCOUNT, &old_key)
            .expect("store primary old key");
        keychain
            .upsert_key(KEYCHAIN_ROTATION_ACCOUNT_PREFIX, &new_key)
            .expect("stage new key");

        rekey_database_standalone(&db_path, &old_key, &new_key)
            .expect("simulate crash after DB rekey");
        assert!(verify_database_key(&db_path, &old_key).is_err());
        verify_database_key(&db_path, &new_key).expect("DB uses staged new key");

        let provider = LocalKeychain::with_keychain_for_tests(keychain.clone());
        let recovered = provider
            .get_or_create_key(&UserIdentity::local(db_path.clone()))
            .expect("startup recovers from staged key");

        assert_eq!(recovered, old_key);
        assert_eq!(
            keychain
                .get_key(KEYCHAIN_ACCOUNT)
                .expect("primary key remains present"),
            old_key
        );
        assert!(keychain.get_key(KEYCHAIN_ROTATION_ACCOUNT_PREFIX).is_err());
        verify_database_key(&db_path, &old_key).expect("DB was rekeyed back to primary");
        assert!(verify_database_key(&db_path, &new_key).is_err());
    }

    #[test]
    fn startup_recovers_crash_after_primary_key_update_before_rekey() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("primary_first_rotation_recovery.db");
        let old_key = fixed_key("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef");
        let new_key = fixed_key("abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789");
        let keychain = Arc::new(FakeKeychain::default());

        create_encrypted_test_database(&db_path, &old_key);
        keychain
            .upsert_key(KEYCHAIN_ACCOUNT, &new_key)
            .expect("store primary new key");
        keychain
            .upsert_key(KEYCHAIN_ROTATION_ACCOUNT_PREFIX, &old_key)
            .expect("stage old key");
        keychain
            .upsert_key(
                KEYCHAIN_ROTATION_IN_PROGRESS_ACCOUNT,
                &fixed_key(KEYCHAIN_ROTATION_IN_PROGRESS_VALUE),
            )
            .expect("store rotation-in-progress flag");

        let provider = LocalKeychain::with_keychain_for_tests(keychain.clone());
        let recovered = provider
            .get_or_create_key(&UserIdentity::local(db_path.clone()))
            .expect("startup recovers from staged old key");

        assert_eq!(recovered, new_key);
        assert_eq!(
            keychain
                .get_key(KEYCHAIN_ACCOUNT)
                .expect("primary key remains present"),
            new_key
        );
        assert!(keychain.get_key(KEYCHAIN_ROTATION_ACCOUNT_PREFIX).is_err());
        assert!(keychain
            .get_key(KEYCHAIN_ROTATION_IN_PROGRESS_ACCOUNT)
            .is_err());
        verify_database_key(&db_path, &new_key).expect("DB was rekeyed to primary");
        assert!(verify_database_key(&db_path, &old_key).is_err());
    }

    #[test]
    fn verify_existing_key_preserves_staged_entry_while_rotation_flag_is_set() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("preserve_staged_during_rotation.db");
        let old_key = fixed_key("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef");
        let staged_key =
            fixed_key("abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789");
        let keychain = Arc::new(FakeKeychain::default());

        create_encrypted_test_database(&db_path, &old_key);
        keychain
            .upsert_key(KEYCHAIN_ACCOUNT, &old_key)
            .expect("store primary old key");
        keychain
            .upsert_key(KEYCHAIN_ROTATION_ACCOUNT_PREFIX, &staged_key)
            .expect("stage recovery key");
        keychain
            .upsert_key(
                KEYCHAIN_ROTATION_IN_PROGRESS_ACCOUNT,
                &fixed_key(KEYCHAIN_ROTATION_IN_PROGRESS_VALUE),
            )
            .expect("store rotation-in-progress flag");

        let provider = LocalKeychain::with_keychain_for_tests(keychain.clone());
        let verified = provider
            .verify_or_recover_existing_key(&db_path, old_key.clone(), KeyAccessMode::Normal)
            .expect("existing key verifies");

        assert_eq!(verified, old_key);
        assert_eq!(
            keychain
                .get_key(KEYCHAIN_ROTATION_ACCOUNT_PREFIX)
                .expect("staged key remains while rotation flag is set"),
            staged_key
        );
        assert!(keychain
            .get_key(KEYCHAIN_ROTATION_IN_PROGRESS_ACCOUNT)
            .is_ok());
    }
}
