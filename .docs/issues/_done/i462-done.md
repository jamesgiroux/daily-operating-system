# I462 — SQLCipher Encryption at Rest

**Status:** Pending
**Priority:** P0
**Version:** 0.15.2
**Area:** Backend / Security
**ADR:** 0092

## Summary

`~/.dailyos/dailyos.db` is plaintext SQLite. Corporate intelligence — account ARR, renewal risk, meeting briefings, relationship maps, email summaries — is readable by any process or user with filesystem access. This issue adds SQLCipher AES-256 encryption, key storage in macOS Keychain, a one-time migration from plaintext, and encrypted backup.

## Acceptance Criteria

### Dependency change

1. `src-tauri/Cargo.toml` has `rusqlite = { version = "0.31", features = ["bundled-sqlcipher", "backup"] }` — NOT `"bundled"` alongside it. `cargo build` succeeds. No other Cargo.toml changes required for the crypto path (macOS system libcrypto is used).
2. `keyring = { version = "3", features = ["apple-native"] }` is added. `keyring::Entry::new(...)` compiles without error.

### Key generation and Keychain storage

3. `get_or_create_db_key()` in `db/mod.rs` (or a new `db/encryption.rs`): on first run (no Keychain entry), generates 32 random bytes via `rand::rngs::OsRng`, stores them as a binary secret at service `com.dailyos.desktop` / account `db-encryption-key`, and returns the key. Verify: `security find-generic-password -s com.dailyos.desktop -a db-encryption-key` returns a result after first launch.
4. On subsequent runs, `get_or_create_db_key()` retrieves the key from Keychain. No new key is generated. Verify: the Keychain entry from step 3 still exists after restarting the app.
5. If Keychain returns `NoEntry` AND `dailyos.db` already exists on disk, `open_or_handle_missing_key()` returns `DbError::KeyMissing` -- not a new key. The app surfaces a recovery screen (not a crash, not a silent empty DB).

### DB open sequence

6. `PRAGMA key = "x'<hex>'";` is executed **before** `PRAGMA journal_mode=WAL` and `PRAGMA foreign_keys=ON`. Any other order produces `SQLITE_NOTADB` on the first real query. Verify: app starts and all existing data loads correctly.
7. `SELECT cipher_version()` is called immediately after setting the key. A result is returned (e.g., "4.6.1 community"). If this call fails, `open_at()` returns an error before proceeding.
8. All existing DB operations (reads, writes, transactions, migrations) work correctly with the encrypted connection. No SQL changes are needed elsewhere in the codebase.

### One-time migration from plaintext

9. `is_database_plaintext()` correctly identifies an unencrypted `dailyos.db` by opening with empty key and checking `SELECT count(*) FROM sqlite_master`. Returns `true` for plaintext, `false` for encrypted.
10. When `is_database_plaintext()` returns true on startup: `migrate_to_encrypted()` runs. It:
    - Calls `PRAGMA wal_checkpoint(TRUNCATE)` on the plaintext connection
    - Runs `SELECT sqlcipher_export('encrypted')` via `ATTACH DATABASE ... AS encrypted KEY "x'<hex>'"`
    - Detaches, closes connections
    - Renames original to `dailyos.db.plaintext-backup`
    - Renames encrypted temp to `dailyos.db`
11. After migration: app opens the renamed `dailyos.db` with the key and all existing data is present. `SELECT count(*) FROM accounts` returns the same row count as before migration.
12. `dailyos.db.plaintext-backup` exists at `~/.dailyos/` after migration. It is deleted on the next app startup (not during the migration itself).

### Backup

13. `db_backup.rs:backup_database()` requires no code changes. Verify: after a backup run, `dailyos.db.bak` exists, `file ~/.dailyos/dailyos.db.bak` reports it is not an SQLite database (because it is encrypted). Attempting to open with `sqlite3 ~/.dailyos/dailyos.db.bak` returns "file is not a database" -- confirming encryption.

### MCP readonly path

14. `open_readonly()` called by the MCP subprocess uses the same `get_or_create_db_key()` path. The MCP sidecar starts correctly and can query the encrypted DB. Verify: run `pnpm dev` and trigger an enrichment that goes through the MCP path -- it completes without error.

### Error handling

15. If `get_or_create_db_key()` fails due to Keychain access error (not `NoEntry`), the error is surfaced in the app startup with a message: "DailyOS cannot open your data. The encryption key is unavailable — your macOS keychain may be locked." The app does not crash silently.
16. `DbError` gains two new variants: `KeyMissing { db_path: PathBuf }` and `Keychain(String)`. Both are handled in the startup path with user-facing messages.

## Files

### New
- `src-tauri/src/db/encryption.rs` — `get_or_create_db_key()`, `generate_and_store_key()`, `key_to_hex()`, `is_database_plaintext()`, `migrate_to_encrypted()`

### Modified
- `src-tauri/Cargo.toml` — feature flag change + `keyring` addition
- `src-tauri/src/db/mod.rs` — `open_at()` gains key setup before migrations; new `open_or_handle_missing_key()` entrypoint; migration check on startup
- `src-tauri/src/error.rs` or `db/mod.rs` — `DbError::KeyMissing`, `DbError::Keychain` variants
- `src-tauri/src/lib.rs` — startup path calls `open_or_handle_missing_key()`, handles `KeyMissing` by routing to recovery screen
- `src-tauri/src/mcp/main.rs` — uses `open_readonly()` which now requires key; call `get_or_create_db_key()` before opening

## Notes

- Do not specify both `bundled` and `bundled-sqlcipher` features -- `bundled-sqlcipher` is the replacement, not an addition.
- The raw hex key (`x'...'` format) bypasses PBKDF2 entirely. Do not use a passphrase format -- it adds ~300ms per open.
- `sqlcipher_export()` is the correct migration function -- it handles schema, indices, triggers, and virtual tables. Do not use `SELECT INTO` or manual table copy.
- `& -> < -> > -> "` escape order matters for other issues; this issue has no prompt-building surface.
- Performance expectation: ~3-5% overhead on Apple Silicon with hardware AES. Cold launch adds < 5ms for key retrieval from Keychain.
