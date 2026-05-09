//! SQLCipher encryption helpers (ADR-0092).
//!
//! Key retrieval and rotation live in `db::key_provider`; this module keeps the
//! SQLCipher formatting, plaintext detection, and migration helpers.

use std::sync::atomic::{AtomicBool, Ordering};

/// Set to `true` when a plaintext→encrypted migration is performed.
static MIGRATION_PERFORMED: AtomicBool = AtomicBool::new(false);

/// Whether the last local key-provider call generated a new key.
pub fn was_key_generated() -> bool {
    crate::db::key_provider::was_key_generated()
}

/// Whether `migrate_to_encrypted` ran during this process.
pub fn was_migration_performed() -> bool {
    MIGRATION_PERFORMED.load(Ordering::Relaxed)
}

#[cfg(test)]
pub(crate) fn set_cached_db_key_for_tests(hex_key: &str) {
    crate::db::key_provider::set_cached_db_key_for_tests(hex_key);
}

/// Retrieve the existing DB key without creating a Keychain entry.
pub fn get_existing_db_key() -> Result<crate::db::EncryptionKey, String> {
    crate::db::LocalKeychain::new().get_existing_key()
}

/// Check if a key exists in the Keychain (without retrieving it).
pub fn has_db_key() -> bool {
    crate::db::LocalKeychain::new().has_key()
}

/// Delete the DB key from Keychain. Used for testing/recovery only.
#[must_use = "check whether keychain entry was deleted before treating encrypted DB as reset"]
pub fn delete_db_key() -> Result<(), String> {
    crate::db::LocalKeychain::new().delete_key()
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
#[must_use = "check whether plaintext database was migrated before opening encrypted storage"]
pub fn migrate_to_encrypted(plaintext_path: &std::path::Path, hex_key: &str) -> Result<(), String> {
    use rusqlite::Connection;

    let encrypted_path = plaintext_path.with_extension("db.encrypted");

    // Open plaintext DB
    let plain_conn = Connection::open(plaintext_path)
        .map_err(|e| format!("Failed to open plaintext DB: {e}"))?;

    // Checkpoint WAL to ensure all data is in the main file
    #[allow(
        clippy::let_underscore_must_use,
        reason = "intentional best-effort discard; preserves existing non-blocking behavior"
    )]
    // best-effort: encryption export reads the main DB and handles an absent/empty WAL.
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
    #[allow(
        clippy::let_underscore_must_use,
        reason = "intentional best-effort discard; preserves existing non-blocking behavior"
    )]
    let _ = std::fs::remove_file(plaintext_path.with_extension("db-wal"));
    #[allow(
        clippy::let_underscore_must_use,
        reason = "intentional best-effort discard; preserves existing non-blocking behavior"
    )]
    let _ = std::fs::remove_file(plaintext_path.with_extension("db-shm"));
    // Remove the plaintext backup after successful swap
    #[allow(
        clippy::let_underscore_must_use,
        reason = "intentional best-effort discard; preserves existing non-blocking behavior"
    )]
    let _ = std::fs::remove_file(&backup_path);

    MIGRATION_PERFORMED.store(true, Ordering::Relaxed);
    log::info!("Database migrated to SQLCipher encryption");
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
