# I471 — AuditLogger Core

**Status:** Pending
**Priority:** P1
**Version:** 0.15.3
**Area:** Backend / Security
**ADR:** 0094

## Summary

DailyOS has no tamper-evident record of what it does: no security event log, no AI operation audit, no external API call log. This issue adds the core `AuditLogger` infrastructure: an append-only JSON-lines file at `~/.dailyos/audit.log` with a SHA-256 hash chain linking each record to the prior one. Tampering (deletion or modification of records) is detectable by chain verification. Retention is 90 days, enforced by rotation on startup.

## Acceptance Criteria

### Data structure and format

1. `AuditLogger` struct in a new `src-tauri/src/audit_log.rs` module. Fields: `path: PathBuf`, `last_hash: Option<String>`. The `last_hash` is the SHA-256 of the previous record's raw bytes (JSON string + newline).
2. Each record is a single JSON object on one line (JSON-lines format). Fields: `ts` (RFC 3339 with milliseconds), `v` (integer, value `1`), `category` (string), `event` (string), `detail` (JSON object), `prev_hash` (string or null for first record).
3. Example record: `{"ts":"2026-02-24T10:00:00.123Z","v":1,"category":"security","event":"db_key_accessed","detail":{"action":"retrieved_from_keychain"},"prev_hash":null}` -- valid JSON on one line, terminated with `\n`.

### Append-only write

4. `AuditLogger::append(category, event, detail)` opens the file with `O_CREAT | O_APPEND | O_WRONLY` -- never `O_TRUNC`. Verify: calling `append()` twice writes two lines; calling it a third time adds a third line without removing the first two.
5. The file is never read-then-written for normal appending. Only rotation reads the file (see criterion 10).
6. File permissions on creation: `0o600`. The containing `~/.dailyos/` is already `0o700` per I463.
7. `AuditLogger` is initialized with `last_hash = None`. On the second call to `append()`, `prev_hash` in the record is the SHA-256 of the first record's raw JSON+newline bytes. Verify: parse both records; `hex(SHA-256(first_line + "\n"))` == `second_record.prev_hash`.
8. Write failures are logged at `WARN` level and never propagate to the caller. The audit logger must not block normal app operation if the log file is unavailable (e.g., disk full).

### AppState integration

9. `AppState` gains `audit_log: Arc<Mutex<AuditLogger>>`. Initialized in `lib.rs` after the DB opens and before the scheduler starts. The `AuditLogger` is constructed from `~/.dailyos/audit.log`; on init, it reads the last line of the existing file (if any) to set `last_hash` and resume the chain.
10. On init, if the log file exists, read its last line, parse the `prev_hash` field, and store it. This allows the chain to be continuous across app restarts. Verify: kill and restart the app; the first new record written after restart has a `prev_hash` matching the hash of the last pre-restart record.

### 90-day rotation

11. On app startup, after `AuditLogger` is initialized, `rotate_audit_log()` runs. It reads all lines from the existing log, filters to records where `ts` is within the last 90 days, writes the filtered lines to `audit.log.rotating`, renames `audit.log.rotating` to `audit.log`, and resets `last_hash` from the last retained record.
12. If fewer than 90 days of records exist (including first run with empty file), rotation is a no-op. Verify: a log with 20 records all from today retains all 20 after rotation.
13. `rotation_completed` event is appended to the log after rotation with `detail: {"records_pruned": N, "bytes_freed": N}`. If 0 records are pruned, the event is still written (as a startup marker).
14. Rotation is synchronous and runs before any other startup events are logged. It blocks the startup sequence for at most 100ms -- for a 90-day log at 50 events/day that is 4,500 records at ~200 bytes each = ~900KB, which reads and writes in well under 100ms.

### Hash chain verification

15. A `verify_audit_log() -> Result<usize, (usize, String)>` function reads all records from the log and verifies the hash chain. Returns `Ok(N)` where N is the record count if the chain is intact. Returns `Err((line_number, message))` at the first broken link. This function is used by the Settings UI (I473) to report chain integrity; it is not called on every startup.
16. Verification correctly detects: a deleted middle record (the following record's `prev_hash` won't match), a modified record (its computed hash won't match the next record's `prev_hash`), an appended-to record (same).

### Event taxonomy (first events logged by this issue)

17. `app_started` is logged on every startup with `detail: {"version": "<semver>", "db_encrypted": true/false}`. Verify by inspecting `audit.log` after launch.
18. `audit_log_rotated` is logged after rotation (always, even if 0 pruned). Detail: `{"records_pruned": N, "bytes_freed": N}`.
19. `db_key_generated` and `db_key_accessed` from I462's `get_or_create_db_key()` are logged here. If I462 ships first with stub log calls (`log::info!`), this issue upgrades them to real audit calls. Either order works.
20. `db_migration_started` and `db_migration_completed` are logged by the migration path in I462 if I471 is available; otherwise `log::info!()` is the stub.

### Unit tests

21. `test_append_creates_chain`: append 3 records, parse the file, verify `record[1].prev_hash == sha256(record[0] line)` and `record[2].prev_hash == sha256(record[1] line)`.
22. `test_rotation_prunes_old`: write 5 records with timestamps 100 days ago and 3 records from today. After rotation, only 3 records remain (plus the `audit_log_rotated` record = 4 total). The 5 old records are gone.
23. `test_verify_detects_deletion`: write 3 records to a file, delete the middle line, call `verify_audit_log()`, verify it returns `Err`.
24. `test_write_failure_does_not_panic`: simulate a write failure (e.g., set file path to a non-writable location after init), call `append()`, verify it returns normally (no panic, no propagated error).
25. `cargo test` passes.

## Files

### New
- `src-tauri/src/audit_log.rs` — `AuditLogger`, `AuditEvent`, `rotate_audit_log()`, `verify_audit_log()`

### Modified
- `src-tauri/src/state.rs` — `audit_log: Arc<Mutex<AuditLogger>>` field on `AppState`
- `src-tauri/src/lib.rs` — initialize `AuditLogger` in startup sequence; call `rotate_audit_log()` on startup; log `app_started` event
- `src-tauri/src/db/encryption.rs` (I462) — upgrade `log::info!()` stubs to `audit_log.append()` calls
- `src-tauri/Cargo.toml` — add `sha2 = "0.10"` if not already present

## Notes

- `sha2` may already be in the dependency tree via `sqlcipher` or other crates. Check `cargo tree | grep sha2` before adding a new dependency.
- The `audit_log.rs` module is distinct from the existing `audit.rs` (which writes raw AI output to `{workspace}/_audit/`). Both coexist. `audit.rs` is for debugging AI output; `audit_log.rs` is for security and compliance events. Do not merge them.
- The rotation implementation reads the entire log into memory. At 90 days × 50 events/day × 200 bytes = ~900KB, this is acceptable. If the log is ever significantly larger (suggests a bug -- 50 events/day is the design ceiling), rotation should still not fail -- just be slower.
- `AuditLogger::append()` acquires the `Arc<Mutex<AuditLogger>>` lock. Since audit writes are infrequent (not hot-path), mutex contention is not a concern.
