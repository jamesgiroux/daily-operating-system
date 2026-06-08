//! Keychain-backed non-DB secrets and retired DB-key compatibility shims.
//!
//! DailyOS v1.4.9 moved the active database to a plain SQLite storage boundary.
//! Normal database opens must not resolve a Keychain database key, rotate one,
//! or apply key material to SQLite. This module keeps the old trait shape for
//! call-site compatibility while making DB-key operations fail loudly.

use rand::Rng;
use sha2::Digest;
use std::fmt;
use std::path::{Path, PathBuf};

use parking_lot::RwLock;
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

const KEYCHAIN_SERVICE: &str = "com.dailyos.desktop.db";
const SURFACE_RUNTIME_ANCHOR_ACCOUNT: &str = "surface-runtime-anchor";
const RETIRED_DB_KEY_ERROR: &str =
    "DB key material is retired for the v1.4.9 plain-SQLite storage boundary";

static ROTATION_LOCK: RwLock<()> = parking_lot::const_rwlock(());

pub(crate) fn rotation_lock_read() -> parking_lot::RwLockReadGuard<'static, ()> {
    ROTATION_LOCK.read()
}

pub(crate) fn rotation_lock_write() -> parking_lot::RwLockWriteGuard<'static, ()> {
    ROTATION_LOCK.write()
}

/// Identity metadata retained for compatibility with the retired provider API.
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

/// Opaque secret material.
///
/// Used only for Keychain-backed non-DB secrets and retired DB-key compatibility
/// shims. Active DB opens must not use it as SQLite key material.
#[derive(Clone, PartialEq, Eq)]
pub struct KeychainSecret {
    hex: Zeroizing<String>,
}

impl KeychainSecret {
    pub(crate) fn from_hex(hex_key: String) -> Self {
        Self {
            hex: Zeroizing::new(hex_key),
        }
    }

    pub(crate) fn as_hex(&self) -> &str {
        self.hex.as_str()
    }
}

impl Zeroize for KeychainSecret {
    fn zeroize(&mut self) {
        self.hex.zeroize();
    }
}

impl ZeroizeOnDrop for KeychainSecret {}

impl fmt::Debug for KeychainSecret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("KeychainSecret([REDACTED])")
    }
}

pub type Result<T> = std::result::Result<T, String>;

pub(crate) fn get_or_create_non_db_secret(account: &str) -> Result<String> {
    get_or_create_non_db_secret_with(
        account,
        || read_keychain_account(account),
        |key| upsert_keychain_account(account, key),
    )
}

fn get_or_create_non_db_secret_with(
    account: &str,
    read_key: impl FnOnce() -> std::result::Result<KeychainSecret, KeychainReadError>,
    upsert_key: impl FnOnce(&KeychainSecret) -> Result<()>,
) -> Result<String> {
    validate_non_db_secret_account(account)?;
    match read_key() {
        Ok(existing) => Ok(existing.as_hex().to_string()),
        Err(KeychainReadError::NotFound(_)) => {
            let key = generate_key();
            upsert_key(&key)?;
            Ok(key.as_hex().to_string())
        }
        Err(error) => Err(error.to_string()),
    }
}

fn validate_non_db_secret_account(account: &str) -> Result<()> {
    let trimmed = account.trim();
    if trimmed.is_empty() {
        return Err("non-DB secret account must be named".to_string());
    }

    let lower = trimmed.to_ascii_lowercase();
    if lower.contains("db-key") || lower.contains("cipher-key") || lower.contains(".rotation") {
        return Err("non-DB secret account must not reuse retired DB-key accounts".to_string());
    }

    Ok(())
}

pub trait DbKeyProvider: Send + Sync {
    #[must_use = "DB-key provider calls are retired and must fail loudly if reached"]
    fn get_or_create_key(&self, user: &UserIdentity) -> Result<KeychainSecret>;

    #[must_use = "DB-key rotation is retired and must fail loudly if reached"]
    fn rotate_key(&self, user: &UserIdentity) -> Result<KeychainSecret>;
}

#[derive(Debug, Default)]
pub struct LocalKeychain;

impl LocalKeychain {
    pub fn new() -> Self {
        Self
    }
}

impl DbKeyProvider for LocalKeychain {
    fn get_or_create_key(&self, user: &UserIdentity) -> Result<KeychainSecret> {
        Err(retired_db_key_error(user.db_path()))
    }

    fn rotate_key(&self, user: &UserIdentity) -> Result<KeychainSecret> {
        Err(retired_db_key_error(user.db_path()))
    }
}

#[must_use = "retired DB-key operation failures must be handled"]
pub(crate) fn rekey_database(
    db_path: &Path,
    _old_key: &KeychainSecret,
    _new_key: &KeychainSecret,
) -> Result<()> {
    Err(retired_db_key_error(db_path))
}

fn retired_db_key_error(db_path: &Path) -> String {
    format!("{RETIRED_DB_KEY_ERROR}: {}", db_path.display())
}

/// Runtime keychain anchor for paired local surfaces.
///
/// W2-C treats the macOS Keychain entry as the runtime authority anchor. The
/// anchor id is a digest of the keychain-only secret; callers use the id in DB
/// rows and audit records, never the secret itself.
pub(crate) fn get_or_create_surface_runtime_anchor_id() -> Result<String> {
    get_or_create_surface_runtime_anchor_id_with(
        || read_keychain_account(SURFACE_RUNTIME_ANCHOR_ACCOUNT),
        |key| upsert_keychain_account(SURFACE_RUNTIME_ANCHOR_ACCOUNT, key),
    )
}

fn get_or_create_surface_runtime_anchor_id_with(
    read_key: impl FnOnce() -> std::result::Result<KeychainSecret, KeychainReadError>,
    upsert_key: impl FnOnce(&KeychainSecret) -> Result<()>,
) -> Result<String> {
    let key = match read_key() {
        Ok(key) => key,
        Err(KeychainReadError::NotFound(_)) => {
            let key = generate_key();
            upsert_key(&key)?;
            key
        }
        Err(error) => return Err(error.to_string()),
    };
    Ok(surface_runtime_anchor_id_from_key(&key))
}

fn surface_runtime_anchor_id_from_key(key: &KeychainSecret) -> String {
    let mut hasher = sha2::Sha256::new();
    hasher.update(b"DAILYOS-SURFACE-RUNTIME-ANCHOR-V1\n");
    hasher.update(key.as_hex().as_bytes());
    hex::encode(hasher.finalize())
}

fn generate_key() -> KeychainSecret {
    let mut bytes = [0_u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    KeychainSecret::from_hex(hex::encode(bytes))
}

#[derive(Debug, PartialEq, Eq)]
enum KeychainReadError {
    NotFound(String),
    Failed(String),
}

impl fmt::Display for KeychainReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound(error) | Self::Failed(error) => f.write_str(error),
        }
    }
}

fn read_keychain_account(account: &str) -> std::result::Result<KeychainSecret, KeychainReadError> {
    let output = std::process::Command::new("security")
        .args([
            "find-generic-password",
            "-s",
            KEYCHAIN_SERVICE,
            "-a",
            account,
            "-w",
        ])
        .output()
        .map_err(|e| KeychainReadError::Failed(format!("Failed to run security CLI: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let detail = format!("Keychain read failed: {}", stderr.trim());
        if is_keychain_item_not_found(&stderr) {
            return Err(KeychainReadError::NotFound(detail));
        }
        return Err(KeychainReadError::Failed(detail));
    }

    let key = String::from_utf8(output.stdout)
        .map_err(|e| KeychainReadError::Failed(format!("Keychain returned non-UTF-8 secret: {e}")))?
        .trim()
        .to_string();
    if key.is_empty() {
        return Err(KeychainReadError::Failed(
            "Keychain returned empty secret".to_string(),
        ));
    }
    Ok(KeychainSecret::from_hex(key))
}

fn is_keychain_item_not_found(stderr: &str) -> bool {
    let stderr = stderr.to_ascii_lowercase();
    stderr.contains("could not be found")
        || stderr.contains("item not found")
        || stderr.contains("-25300")
}

fn upsert_keychain_account(account: &str, key: &KeychainSecret) -> Result<()> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use zeroize::ZeroizeOnDrop;

    fn fixed_secret(hex: &str) -> KeychainSecret {
        KeychainSecret::from_hex(hex.to_string())
    }

    #[test]
    fn surface_runtime_anchor_reuses_existing_keychain_anchor() {
        let existing =
            fixed_secret("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef");
        let upsert_called = AtomicBool::new(false);

        let anchor_id = get_or_create_surface_runtime_anchor_id_with(
            || Ok(existing.clone()),
            |_| {
                upsert_called.store(true, Ordering::Release);
                Ok(())
            },
        )
        .expect("existing anchor id is returned");

        assert_eq!(anchor_id, surface_runtime_anchor_id_from_key(&existing));
        assert!(!upsert_called.load(Ordering::Acquire));
    }

    #[test]
    fn surface_runtime_anchor_creates_keychain_anchor_only_when_missing() {
        let stored_anchor = parking_lot::Mutex::new(None);

        let anchor_id = get_or_create_surface_runtime_anchor_id_with(
            || {
                Err(KeychainReadError::NotFound(
                    "Keychain read failed: The specified item could not be found in the keychain."
                        .to_string(),
                ))
            },
            |key| {
                *stored_anchor.lock() = Some(key.clone());
                Ok(())
            },
        )
        .expect("missing anchor is created");

        let stored_anchor = stored_anchor
            .lock()
            .clone()
            .expect("generated anchor was stored");
        assert_eq!(
            anchor_id,
            surface_runtime_anchor_id_from_key(&stored_anchor)
        );
    }

    #[test]
    fn surface_runtime_anchor_fails_closed_on_keychain_read_failures() {
        for error in [
            "Failed to run security CLI: No such file or directory",
            "Keychain read failed: User interaction is not allowed.",
            "Keychain returned non-UTF-8 secret: invalid utf-8 sequence",
            "Keychain returned empty secret",
        ] {
            let upsert_called = AtomicBool::new(false);

            let observed = get_or_create_surface_runtime_anchor_id_with(
                || Err(KeychainReadError::Failed(error.to_string())),
                |_| {
                    upsert_called.store(true, Ordering::Release);
                    Ok(())
                },
            )
            .expect_err("non-missing keychain failures do not rotate the anchor");

            assert_eq!(observed, error);
            assert!(!upsert_called.load(Ordering::Acquire));
        }
    }

    #[test]
    fn non_db_secret_reuses_existing_secret() {
        let existing =
            fixed_secret("abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789");
        let upsert_called = AtomicBool::new(false);

        let observed = get_or_create_non_db_secret_with(
            "audit-pseudonymization-test",
            || Ok(existing.clone()),
            |_| {
                upsert_called.store(true, Ordering::Release);
                Ok(())
            },
        )
        .expect("existing secret");

        assert_eq!(observed, existing.as_hex());
        assert!(!upsert_called.load(Ordering::Acquire));
    }

    #[test]
    fn non_db_secret_creates_secret_only_when_missing() {
        let stored_secret = parking_lot::Mutex::new(None);

        let observed = get_or_create_non_db_secret_with(
            "workspace-graph-diagnostic-test",
            || Err(KeychainReadError::NotFound("missing".to_string())),
            |key| {
                *stored_secret.lock() = Some(key.clone());
                Ok(())
            },
        )
        .expect("created secret");

        let stored_secret = stored_secret
            .lock()
            .clone()
            .expect("generated secret was stored");
        assert_eq!(observed, stored_secret.as_hex());
    }

    #[test]
    fn non_db_secret_rejects_retired_db_key_account_shapes() {
        for account in [
            "",
            "dailyos-db-key",
            "legacy-cipher-key",
            "dailyos.rotation",
        ] {
            let error = get_or_create_non_db_secret_with(
                account,
                || panic!("validator should run before keychain read"),
                |_| panic!("validator should run before keychain write"),
            )
            .expect_err("retired account shape rejected");
            assert!(error.contains("non-DB secret account"));
        }
    }

    #[test]
    fn local_keychain_db_key_calls_are_retired() {
        let provider = LocalKeychain::new();
        let user = UserIdentity::local("/tmp/dailyos.db");

        let get_error = provider
            .get_or_create_key(&user)
            .expect_err("DB key fetch is retired");
        let rotate_error = provider
            .rotate_key(&user)
            .expect_err("DB key rotation is retired");

        assert!(get_error.contains(RETIRED_DB_KEY_ERROR));
        assert!(rotate_error.contains(RETIRED_DB_KEY_ERROR));
    }

    #[test]
    fn rekey_database_is_retired() {
        let old = fixed_secret("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef");
        let new = fixed_secret("abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789");

        let error = rekey_database(Path::new("/tmp/dailyos.db"), &old, &new)
            .expect_err("DB rekey is retired");

        assert!(error.contains(RETIRED_DB_KEY_ERROR));
    }

    #[test]
    fn keychain_not_found_classifier_matches_security_cli_absence_errors() {
        assert!(is_keychain_item_not_found(
            "security: SecKeychainSearchCopyNext: The specified item could not be found in the keychain."
        ));
        assert!(is_keychain_item_not_found("security: item not found"));
        assert!(is_keychain_item_not_found("security: error -25300"));
        assert!(!is_keychain_item_not_found(
            "security: SecKeychainItemCopyContent: User interaction is not allowed."
        ));
    }

    #[test]
    fn keychain_secret_debug_redacts_key_material() {
        let key = fixed_secret("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef");

        assert_eq!(format!("{key:?}"), "KeychainSecret([REDACTED])");
    }

    #[test]
    fn local_keychain_implements_provider_trait() {
        fn assert_provider<T: DbKeyProvider>() {}
        assert_provider::<LocalKeychain>();
    }

    #[test]
    fn keychain_secret_is_zeroize_on_drop() {
        fn assert_zeroize_on_drop<T: ZeroizeOnDrop>() {}

        assert_zeroize_on_drop::<KeychainSecret>();
    }
}
