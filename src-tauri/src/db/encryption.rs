//! SQLCipher key management via macOS Keychain (ADR-0092).
//!
//! The database encryption key is a 256-bit random hex string stored in the
//! macOS Keychain under `com.dailyos.desktop.db`. Raw hex format (`x'...'`)
//! bypasses SQLCipher's PBKDF2, avoiding the 300ms open-time overhead.

use rand::Rng;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

const KEYCHAIN_SERVICE: &str = "com.dailyos.desktop.db";
const KEYCHAIN_ACCOUNT: &str = "sqlcipher-key";

/// Set to `true` when a new key is generated (fresh install).
static KEY_WAS_GENERATED: AtomicBool = AtomicBool::new(false);

/// Set to `true` when a plaintext→encrypted migration is performed.
static MIGRATION_PERFORMED: AtomicBool = AtomicBool::new(false);

/// Whether the last `get_or_create_db_key` call generated a new key.
pub fn was_key_generated() -> bool {
    KEY_WAS_GENERATED.load(Ordering::Relaxed)
}

/// Whether `migrate_to_encrypted` ran during this process.
pub fn was_migration_performed() -> bool {
    MIGRATION_PERFORMED.load(Ordering::Relaxed)
}

/// Process-wide cached key. Set once on first Keychain read, reused for all
/// subsequent DB opens. This avoids hitting the Keychain on every background
/// thread's `open_at()` call (which would trigger repeated macOS permission
/// dialogs on first launch or after code-signing changes).
static CACHED_KEY: OnceLock<String> = OnceLock::new();

/// Retrieve the existing DB key from Keychain, or generate and store a new one
/// if no database exists yet. The key is cached in memory after the first
/// successful access — subsequent calls never touch the Keychain.
///
/// **Critical safety rule:** If an encrypted database already exists but no
/// Keychain entry is found, this returns `Err` instead of generating a new key.
/// A new key would silently fail to decrypt the existing data. The caller must
/// surface this as a recovery screen, not swallow it.
pub fn get_or_create_db_key(db_path: &std::path::Path) -> Result<String, String> {
    let key = match get_existing_db_key() {
        Ok(key) => key,
        Err(_e) => {
            // DB exists and is not plaintext → encrypted with a lost key.
            // Return a KEY_MISSING marker so callers can distinguish this from
            // other encryption errors and show a recovery screen.
            if db_path.exists() && !is_database_plaintext(db_path) {
                return Err(format!("KEY_MISSING:{}", db_path.display()));
            }
            // No DB yet (fresh install) or plaintext DB (pre-migration) → safe to create key
            let new_key = generate_key();
            store_key_in_keychain(&new_key)?;
            KEY_WAS_GENERATED.store(true, Ordering::Relaxed);
            new_key
        }
    };

    // Cache for all future callers (race-safe: OnceLock ignores duplicate sets)
    let _ = CACHED_KEY.set(key.clone());
    Ok(key)
}

/// Retrieve the existing DB key without creating a Keychain entry.
pub fn get_existing_db_key() -> Result<String, String> {
    if let Some(key) = CACHED_KEY.get() {
        return Ok(key.clone());
    }

    let key = get_key_from_keychain()?;
    let _ = CACHED_KEY.set(key.clone());
    Ok(key)
}

/// Check if a key exists in the Keychain (without retrieving it).
pub fn has_db_key() -> bool {
    get_key_from_keychain().is_ok()
}

/// Delete the DB key from Keychain. Used for testing/recovery only.
pub fn delete_db_key() -> Result<(), String> {
    let output = std::process::Command::new("security")
        .args([
            "delete-generic-password",
            "-s",
            KEYCHAIN_SERVICE,
            "-a",
            KEYCHAIN_ACCOUNT,
        ])
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

/// Format a hex key string into the SQLCipher PRAGMA format.
pub fn key_to_pragma(hex_key: &str) -> String {
    format!("PRAGMA key = \"x'{hex_key}'\"")
}

/// Check if a database file is plaintext SQLite (not encrypted).
pub fn is_database_plaintext(path: &std::path::Path) -> bool {
    if let Ok(bytes) = std::fs::read(path) {
        // SQLite3 magic header: "SQLite format 3\0"
        bytes.len() >= 16 && &bytes[..16] == b"SQLite format 3\0"
    } else {
        false
    }
}

/// Migrate a plaintext database to an encrypted one using sqlcipher_export().
pub fn migrate_to_encrypted(plaintext_path: &std::path::Path, hex_key: &str) -> Result<(), String> {
    use rusqlite::Connection;

    let encrypted_path = plaintext_path.with_extension("db.encrypted");

    // Open plaintext DB
    let plain_conn = Connection::open(plaintext_path)
        .map_err(|e| format!("Failed to open plaintext DB: {e}"))?;

    // Checkpoint WAL to ensure all data is in the main file
    let _ = plain_conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");

    // Attach encrypted target
    plain_conn
        .execute_batch(&format!(
            "ATTACH DATABASE '{}' AS encrypted KEY \"x'{hex_key}'\";",
            encrypted_path.display()
        ))
        .map_err(|e| format!("Failed to attach encrypted DB: {e}"))?;

    // Export all data
    plain_conn
        .execute_batch("SELECT sqlcipher_export('encrypted');")
        .map_err(|e| format!("sqlcipher_export failed: {e}"))?;

    plain_conn
        .execute_batch("DETACH DATABASE encrypted;")
        .map_err(|e| format!("Failed to detach: {e}"))?;

    drop(plain_conn);

    // Atomic swap: rename encrypted over plaintext
    let backup_path = plaintext_path.with_extension("db.plaintext-backup");
    std::fs::rename(plaintext_path, &backup_path)
        .map_err(|e| format!("Failed to backup plaintext DB: {e}"))?;
    std::fs::rename(&encrypted_path, plaintext_path)
        .map_err(|e| format!("Failed to swap encrypted DB: {e}"))?;

    // Clean up WAL/SHM from the plaintext version
    let _ = std::fs::remove_file(plaintext_path.with_extension("db-wal"));
    let _ = std::fs::remove_file(plaintext_path.with_extension("db-shm"));
    // Remove the plaintext backup after successful swap
    let _ = std::fs::remove_file(&backup_path);

    MIGRATION_PERFORMED.store(true, Ordering::Relaxed);
    log::info!("Database migrated to SQLCipher encryption");
    Ok(())
}

fn generate_key() -> String {
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}

/// Read the encryption key from macOS Keychain via the `security` CLI.
///
/// Using the `security` binary instead of the `keyring` crate avoids the
/// repeated password prompt during development — `security` is a trusted
/// system binary, so macOS doesn't re-prompt when the app binary changes
/// on every recompile.
fn get_key_from_keychain() -> Result<String, String> {
    let output = std::process::Command::new("security")
        .args([
            "find-generic-password",
            "-s",
            KEYCHAIN_SERVICE,
            "-a",
            KEYCHAIN_ACCOUNT,
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
    Ok(key)
}

/// Store the encryption key in macOS Keychain via the `security` CLI.
fn store_key_in_keychain(key: &str) -> Result<(), String> {
    // Delete existing entry first (add-generic-password fails if it exists)
    let _ = std::process::Command::new("security")
        .args([
            "delete-generic-password",
            "-s",
            KEYCHAIN_SERVICE,
            "-a",
            KEYCHAIN_ACCOUNT,
        ])
        .output();

    let output = std::process::Command::new("security")
        .args([
            "add-generic-password",
            "-s",
            KEYCHAIN_SERVICE,
            "-a",
            KEYCHAIN_ACCOUNT,
            "-w",
            key,
            "-U", // update if exists
        ])
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
    use super::{is_database_plaintext, key_to_pragma};
    use std::fs;

    #[test]
    fn key_to_pragma_formats_raw_hex_key_for_sqlcipher() {
        assert_eq!(
            key_to_pragma("0123456789abcdef"),
            "PRAGMA key = \"x'0123456789abcdef'\""
        );
    }

    #[test]
    fn is_database_plaintext_returns_true_for_valid_sqlite_header() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let db_path = temp_dir.path().join("plain.db");

        fs::write(&db_path, b"SQLite format 3\0remaining bytes").expect("write sqlite header");

        assert!(is_database_plaintext(&db_path));
    }

    #[test]
    fn is_database_plaintext_returns_false_for_non_sqlite_file() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let db_path = temp_dir.path().join("encrypted.db");

        fs::write(&db_path, b"not sqlite header and likely encrypted").expect("write non-sqlite");

        assert!(!is_database_plaintext(&db_path));
    }

    #[test]
    fn is_database_plaintext_returns_false_for_empty_file() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let db_path = temp_dir.path().join("empty.db");

        fs::write(&db_path, []).expect("write empty file");

        assert!(!is_database_plaintext(&db_path));
    }

    #[test]
    fn is_database_plaintext_returns_false_for_missing_file() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let db_path = temp_dir.path().join("missing.db");

        assert!(!is_database_plaintext(&db_path));
    }

    #[test]
    fn is_database_plaintext_returns_false_for_file_shorter_than_sqlite_header() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let db_path = temp_dir.path().join("short.db");

        fs::write(&db_path, b"SQLite format 3").expect("write short header");

        assert!(!is_database_plaintext(&db_path));
    }
}
