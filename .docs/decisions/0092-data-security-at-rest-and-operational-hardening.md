# ADR-0092: Data Security at Rest and Operational Hardening

**Date:** 2026-02-24
**Status:** Accepted
**Target:** v0.16.1
**Relates to:** ADR-0091 (IntelligenceProvider -- local-first AI path)

## Context

DailyOS stores organizationally sensitive data locally: account intelligence (ARR, renewal risk, health scores), meeting briefings, relationship graphs, email summaries, and enriched PII (names, emails, roles, work history). This is not just personal data -- it is corporate intelligence that belongs to the user's employer.

The current storage posture is:
- SQLite at `~/.dailyos/dailyos.db` -- plaintext, no encryption at rest
- Backup at `~/.dailyos/dailyos.db.bak` -- also plaintext
- WAL/SHM files at `~/.dailyos/dailyos.db-wal`, `dailyos.db-shm` -- also plaintext
- Workspace JSON files (`_today/data/preps/`, `schedule.json`, `Accounts/*/dashboard.json`) -- plaintext in the user filesystem
- Google OAuth token -- correctly stored in macOS Keychain
- Smithery API key -- correctly stored in macOS Keychain

Three operational risks compound the plaintext storage problem:

**1. Unauthorized local access.** No app-level authentication exists. Anyone with filesystem access (shared device, unlocked laptop, shoulder surfing) can read all data via the DB file or workspace JSONs.

**2. Backup exfiltration.** macOS Time Machine backs up `~/.dailyos/` by default. If the backup target is an unencrypted external drive or network share, all corporate intelligence leaves the machine in plaintext. Additionally, if the user's workspace is under `~/Documents` and iCloud Desktop & Documents is enabled, workspace files are already syncing to Apple's servers silently. Note: `~/.dailyos/` itself is a dotfolder in HOME and is NOT in iCloud scope -- the workspace path is the risk.

**3. Physical media exposure.** Lost or stolen device with unencrypted storage, or forensic analysis of decommissioned hardware, exposes all stored intelligence.

For users at FedRAMP Moderate organizations, SC-28 (Protection of Information at Rest) and SC-13 (Cryptographic Protection) require encryption for CUI stored at rest. These controls are not satisfied by the current plaintext SQLite.

### Corporate SSL inspection (Automattic / Autoproxxy)

**Scope: Automattic-managed devices only.** This does not apply to non-Automattic users or personal devices. On a device without the corporate CA installed, DailyOS's API calls go direct to their destinations with standard TLS certificates issued by the respective service's CA (Google Trust Services, etc.).

On Automattic-managed devices, Autoproxxy performs SSL inspection of outbound HTTPS traffic using a JAMF-deployed corporate CA ("Automattic Inc. JSS Built-in Certificate Authority"). This has been confirmed by inspecting the TLS certificate issuer for external API calls from a managed device.

The actual data flow for DailyOS on a managed Automattic device is:

```
DailyOS → Autoproxxy (TLS terminated, plaintext visible) → Anthropic API
DailyOS → Autoproxxy (TLS terminated, plaintext visible) → Google APIs
DailyOS → Autoproxxy (TLS terminated, plaintext visible) → Smithery / Clay
```

This means: full Claude prompts (meeting context, account names, relationship notes, email summaries), Google API responses (calendar events, email metadata), and Clay enrichment queries pass through Automattic's proxy infrastructure before reaching their destinations.

**Implications:**

1. **This is expected and acceptable** for work data on a managed work device. Automattic's acceptable use policy governs this traffic; employees on managed devices should expect corporate visibility into work-related API calls.

2. **The FedRAMP exception for Anthropic** (enterprise agreement, separation between FedRAMP and non-FedRAMP customers) covers Anthropic's processing of the data. It does not cover Autoproxxy's position in the chain. The systems security team exception should be scoped to acknowledge the proxy layer.

3. **SQLCipher (this ADR's primary decision) protects against a different threat** -- at-rest access to the local DB by unauthorized parties. It does not protect and is not intended to protect the in-transit layer. The SSL inspection is an in-transit consideration, not an at-rest one.

4. **DailyOS requires no code changes** for this. The macOS system certificate store trusts the Automattic CA, so all TLS connections succeed transparently. This is documented here for accuracy, not as a remediation item.

5. **ADR-0091 (Ollama local provider)** remains the path to eliminating Anthropic API calls from the data flow entirely. With Ollama selected, the only outbound calls are to Google APIs (which Autoproxxy also sees, but which are inherent to the calendar/email integration).

## Decision

### 1. SQLite Encryption via SQLCipher

Add SQLCipher encryption to `~/.dailyos/dailyos.db` and its backup.

**Dependency changes** (`src-tauri/Cargo.toml`):

`bundled-sqlcipher` replaces `bundled` -- do NOT specify both features:

```toml
# Before:
# rusqlite = { version = "0.31", features = ["bundled", "backup"] }
# After:
rusqlite = { version = "0.31", features = ["bundled-sqlcipher", "backup"] }

# Add for Keychain access:
keyring = { version = "3", features = ["apple-native"] }
```

`bundled-sqlcipher` compiles SQLCipher from source and links against macOS system libcrypto (Security.framework). The `apple-native` feature on `keyring` is required -- without it the crate defaults to a mock in-memory store.

SQLCipher uses AES-256 by default, which is FIPS 140-2 aligned. The encryption is transparent to all existing SQL operations -- no query changes required.

**Key generation and Keychain storage:**

On first run, generate a 32-byte random key via `rand::rngs::OsRng` (OS CSPRNG, already in Cargo.toml) and store it in the macOS login keychain via the `keyring` crate. Using a raw 32-byte random key stored in Keychain bypasses SQLCipher's PBKDF2 key derivation entirely -- the key is passed with the `x'...'` raw hex format. This eliminates the open-time key derivation overhead (otherwise ~300ms for 256,000 PBKDF2 iterations). On Apple Silicon with hardware AES, per-read/write overhead is ~3-5%.

```rust
use keyring::Entry;

const KEYCHAIN_SERVICE: &str = "com.dailyos.desktop";
const KEYCHAIN_ACCOUNT: &str = "db-encryption-key";

pub fn get_or_create_db_key() -> Result<[u8; 32], DbError> {
    let entry = Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT)
        .map_err(|e| DbError::Keychain(e.to_string()))?;
    match entry.get_secret() {
        Ok(bytes) if bytes.len() == 32 => {
            let mut key = [0u8; 32];
            key.copy_from_slice(&bytes);
            Ok(key)
        }
        Ok(_) => {
            log::warn!("Stored DB key has unexpected length -- regenerating");
            generate_and_store_key(&entry)
        }
        Err(keyring::Error::NoEntry) => generate_and_store_key(&entry),
        Err(e) => Err(DbError::Keychain(e.to_string())),
    }
}

fn generate_and_store_key(entry: &Entry) -> Result<[u8; 32], DbError> {
    use rand::RngCore;
    let mut key = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut key);
    entry.set_secret(&key)
        .map_err(|e| DbError::Keychain(e.to_string()))?;
    log::info!("Generated and stored new DB encryption key in Keychain");
    Ok(key)
}
```

**DB open sequence** (`db/mod.rs`):

The key PRAGMA **must be set first** -- before `journal_mode`, `foreign_keys`, or any other operation. SQLCipher applies just-in-time key derivation at first DB access; any operation before `PRAGMA key` will fail with a corrupt-database error.

```rust
fn open_at_with_key(path: PathBuf, key: [u8; 32]) -> Result<Self, DbError> {
    let hex_key = hex::encode(key);  // hex 0.4 already in Cargo.toml
    let conn = Connection::open(&path)?;

    // Key MUST be first -- before any other PRAGMA or query
    conn.execute_batch(&format!("PRAGMA key = \"x'{hex_key}'\";"))?;

    // Verify key accepted: returns SQLCipher version on success,
    // SQLITE_NOTADB error if wrong key
    conn.query_row("SELECT cipher_version()", [], |r| r.get::<_, String>(0))?;

    // Standard initialization (unchanged from current code)
    conn.execute_batch("PRAGMA journal_mode=WAL;")?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    run_migrations(&conn)?;
    Ok(Self { conn })
}
```

`open_readonly()` (used by the MCP subprocess) also requires the key. The MCP subprocess runs as the same macOS user, so Keychain access is available. No architectural change needed.

**Migration path for existing installations:**

On startup, detect whether the DB is plaintext and run a one-time `sqlcipher_export()` migration if so. `sqlcipher_export()` is the correct migration API -- it copies all schema, triggers, virtual tables, and data to a new encrypted database atomically.

```rust
fn is_database_plaintext(path: &Path) -> bool {
    let Ok(conn) = Connection::open(path) else { return false };
    // Empty key = plaintext mode in SQLCipher
    let _ = conn.execute_batch("PRAGMA key = '';");
    // Plaintext DB returns a count; encrypted DB returns SQLITE_NOTADB
    conn.query_row(
        "SELECT count(*) FROM sqlite_master",
        [],
        |r| r.get::<_, i64>(0),
    ).is_ok()
}

fn migrate_to_encrypted(plaintext_path: &Path, hex_key: &str) -> Result<(), DbError> {
    let conn = Connection::open(plaintext_path)?;
    conn.execute_batch("PRAGMA key = '';")?;
    // Checkpoint WAL before export to ensure all data is in the main file
    conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;
    let encrypted_tmp = plaintext_path.with_extension("db.encrypting");
    conn.execute_batch(&format!(
        "ATTACH DATABASE '{}' AS encrypted KEY \"x'{hex_key}'\";",
        encrypted_tmp.display()
    ))?;
    conn.execute_batch("SELECT sqlcipher_export('encrypted');")?;
    conn.execute_batch("DETACH DATABASE encrypted;")?;
    drop(conn);
    // Atomic swap: preserve plaintext backup for one release cycle
    std::fs::rename(
        plaintext_path,
        plaintext_path.with_extension("db.plaintext-backup")
    )?;
    std::fs::rename(&encrypted_tmp, plaintext_path)?;
    log::info!("One-time migration to encrypted DB complete");
    Ok(())
}
```

The plaintext backup (`dailyos.db.plaintext-backup`) is deleted on the next app startup after migration.

**Backup implications:** `db_backup.rs` uses `rusqlite::backup::Backup`, which copies raw (already encrypted) pages. When the source DB is SQLCipher-encrypted, the backup is automatically written encrypted with the same key. No changes to `db_backup.rs` are required.

**Key recovery:**

In Settings -> Data, expose an "Export Recovery Key" action that retrieves the 64-character hex key from Keychain and displays it for the user to save to a password manager. This is the only recovery path if the user sets up a new machine without Migration Assistant and restores the DB file manually.

If `get_or_create_db_key()` returns `NoEntry` AND the database file already exists (key-loss scenario), the app must NOT silently regenerate a key -- that produces a new empty database. Instead, surface a recovery screen:

> "DailyOS cannot read your data because the encryption key is unavailable. This can happen when migrating to a new Mac without using Migration Assistant. If you exported a recovery key, you can enter it here to restore access."

### 2. File Permission Hardening

Set explicit restrictive permissions on creation of all sensitive paths:

| Path | Mode | Existing |
|------|------|----------|
| `~/.dailyos/` | `0o700` | Not enforced |
| `~/.dailyos/dailyos.db` | `0o600` | Not enforced |
| `~/.dailyos/dailyos.db.bak` | `0o600` | Not enforced |
| Workspace `_today/data/` | `0o700` | Not enforced |

`token_store.rs` already implements this pattern for OAuth files. Apply the same `set_permissions` call in `db/mod.rs` on directory and file creation.

### 3. Time Machine Exclusion

Exclude `~/.dailyos/` from Time Machine backups on app startup using `tmutil addexclusion`, which sets the `com.apple.metadata:com_apple_backup_excludeItem` extended attribute (sticky, survives file moves, no sudo required):

```rust
fn exclude_from_time_machine(path: &Path) -> Result<(), std::io::Error> {
    let status = std::process::Command::new("tmutil")
        .arg("addexclusion")
        .arg(path)
        .status()?;
    if status.success() {
        log::info!("Excluded from Time Machine: {}", path.display());
        Ok(())
    } else {
        log::warn!("tmutil addexclusion failed for: {}", path.display());
        Ok(())  // Non-fatal: best-effort exclusion
    }
}
```

Call this immediately after `std::fs::create_dir_all(&dailyos_dir)` in `db_path()`. This is idempotent -- calling it on an already-excluded path is a no-op.

Apply to `~/.dailyos/` (the entire data directory). Do **not** exclude the workspace -- the workspace contains user-authored markdown files (notes, actions, meeting summaries) that the user may want backed up. The DB holds derived intelligence; the workspace is the user's own writing. Respect this distinction.

Verify with: `xattr -p com.apple.metadata:com_apple_backup_excludeItem ~/.dailyos`

### 4. iCloud Drive Detection and Warning

`~/.dailyos/` is a dotfolder in HOME and is NOT in iCloud Drive scope regardless of user settings. The risk is the **workspace path**: if the workspace is under `~/Documents` and the user has "Desktop and Documents Folders" iCloud sync enabled, workspace files (meeting preps, schedule, account dashboards) are syncing to Apple's servers.

On app startup and Settings open, detect if the configured workspace path is under an iCloud-synced directory:

```rust
pub fn is_under_icloud_scope(path: &Path) -> bool {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return false,
    };
    let icloud_root = home.join("Library/Mobile Documents/com~apple~CloudDocs");
    // Desktop/Documents when iCloud "Desktop and Documents Folders" is on
    let icloud_desktop = home.join("Library/Mobile Documents/com~apple~CloudDocs/Desktop");
    let icloud_docs = home.join("Library/Mobile Documents/com~apple~CloudDocs/Documents");

    path.starts_with(&icloud_root)
        || path.starts_with(&icloud_desktop)
        || path.starts_with(&icloud_docs)
        || path.starts_with(home.join("Desktop"))
        || path.starts_with(home.join("Documents"))
}
```

If detected, show a one-time modal in Settings -> Data:

> **Your workspace may be syncing to iCloud**
>
> DailyOS stores meeting briefings and account intelligence in your workspace folder, which appears to be inside an iCloud-synced directory. This means prep documents, account health data, and relationship notes may be uploading to Apple's servers.
>
> Recommended: Move your workspace to a location outside iCloud, such as ~/DailyOS.
> [Open Settings] [Dismiss -- Don't Show Again]

This check is informational -- the app does not block or force a move.

### 5. App Lock on Idle (Touch ID / macOS Auth)

Implement an inactivity lock using macOS LocalAuthentication:

- **Lock trigger**: App window goes to background for more than 15 minutes (configurable in Settings -> Security; options: 5 / 15 / 30 / Never)
- **Lock screen**: Full-window overlay, app name only, no data visible
- **Unlock**: macOS Touch ID / password via `LAContext.evaluatePolicy(.deviceOwnerAuthenticationWithBiometricsOrPassword)` invoked from Rust via `tauri-plugin-biometric` or a direct macOS Security framework call
- **Implementation**: Rust-side `NSApplicationDelegate` notification on `applicationDidResignActive` + timer; lock state held in `AppState` and checked by frontend before rendering any data

This is defense-in-depth against the "stepped away from unlocked laptop" scenario.

### 6. PII Hygiene in Logs

Current `log::info!` / `log::warn!` calls may emit entity names, email addresses, or account data. Log the *shape* of data, not the *content*.

```rust
// Wrong -- leaks entity name (PII):
log::info!("Enriching entity: {} ({})", entity_name, entity_id);

// Correct -- UUID is not PII, type is not PII:
log::info!("Enriching entity id={} type={}", entity_id, entity_type);
```

Apply this rule to new log statements going forward. Do not do a mass retrofit -- that risks introducing bugs. When the diagnostic export feature (I429) is built, apply PII scrubbing to log output at that point.

## What Is Not Decided Here

**Workspace file encryption**: Workspace markdown and JSON files are not encrypted. Encrypting them would break the "workspace as plain text, readable by Claude Code" contract. The workspace is user-authored content; the DB (derived intelligence, enrichment, email metadata) is the higher-sensitivity target and is addressed here.

**Key rotation**: Not implemented. SQLCipher supports `PRAGMA rekey` for key rotation. If a key is suspected compromised, rotation can be added as a Settings -> Security action in a future update.

**End-to-end encrypted sync**: Out of scope. DailyOS is local-first with no sync layer.

## Consequences

- `rusqlite` feature flag changes from `bundled` to `bundled-sqlcipher`. This is a compile-time switch -- no runtime API changes to existing code.
- `keyring` crate added (version 3, `apple-native` feature). Minimal binary size increase.
- First launch after upgrade: one-time DB migration (encrypt-in-place via `sqlcipher_export`). Expected duration: < 2 seconds for a typical DB (< 50MB) on Apple Silicon.
- If Keychain is unavailable (locked keychain, new machine without migration), the app surfaces a recovery screen rather than crashing or silently creating a new empty DB.
- The `.bak` backup is encrypted automatically -- no code changes to `db_backup.rs`.
- `open_readonly()` (MCP subprocess) requires the same key from Keychain. The subprocess runs as the same user -- Keychain access is available. No architectural change.
- Time Machine exclusion is best-effort. If `tmutil` is unavailable, log a warning and continue -- don't fail startup.
- iCloud detection is informational only. No blocking. The `~/.dailyos/` directory is not in iCloud scope regardless.
