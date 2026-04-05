# ADR-0094: Audit Log and Enterprise Observability

**Date:** 2026-02-24
**Status:** Accepted
**Target:** v0.16.1
**Relates to:** ADR-0092 (Data Security), ADR-0093 (Prompt Injection Hardening)

## Context

DailyOS operates with three existing audit mechanisms:

1. **`audit.rs`** -- writes raw Claude output as `.txt` files to `{workspace}/_audit/`. 30-day retention. Covers AI enrichment raw output for debugging.
2. **`enrichment_log` SQLite table** -- tracks enrichment events per entity with source, fields updated, and raw payload. Good for data provenance.
3. **`hygiene_actions_log` SQLite table** -- tracks hygiene actions linked to signal sources.

These cover specific pipelines but leave significant gaps:

- **No security event audit**: DB key access, auth grants/revocations, app unlock, failed DB opens
- **No AI operation audit**: what was sent to Claude, what came back, whether schema validation passed
- **No external API audit**: which third-party services were called, when, with what data category
- **No configuration change audit**: when workspace path changed, when AI provider switched, when settings were modified
- **No injection anomaly audit**: when a potential prompt injection was detected in external content
- The `_audit/` directory lives in the workspace -- iCloud risk if the workspace is under `~/Documents` (per ADR-0092 analysis)
- The SQLite tables are mutable -- records can be deleted or updated, which means they cannot satisfy AU-9 (Protection of Audit Information) without additional controls
- No structured format -- `_audit/` files are raw text blobs, not queryable or exportable

For a user at a FedRAMP Moderate organization, NIST 800-53 AU-2, AU-3, AU-9, and AU-12 are applicable controls:

- **AU-2**: Identify auditable events (auth, data access, config changes, anomalies)
- **AU-3**: Minimum record content (timestamp, event type, subject, source, success/failure)
- **AU-9**: Protect audit records from unauthorized modification or deletion
- **AU-12**: Generate audit records for the defined event types

For a single-user local desktop app, these controls are satisfied by a local append-only structured log with hash-chain tamper detection -- not by a SIEM or centralized log aggregation, which would be disproportionate.

## Decision

### 1. Append-Only Structured Audit Log at `~/.dailyos/audit.log`

A JSON-lines file at `~/.dailyos/audit.log`. Each line is one audit record. The file is written with `O_APPEND` semantics only -- never truncated, never read-then-write. File permissions: `0o600` (owner read/write only, same as the database).

**Record format:**
```json
{
  "ts": "2026-02-24T10:00:00.123Z",
  "v": 1,
  "category": "security",
  "event": "db_key_accessed",
  "detail": {"action": "retrieved_from_keychain"},
  "prev_hash": "a3f7c2b8e1d94012..."
}
```

Fields:
- `ts` -- RFC 3339 timestamp with millisecond precision
- `v` -- schema version (1 for this ADR)
- `category` -- `security` | `data_access` | `ai_operation` | `config_change` | `anomaly` | `lifecycle`
- `event` -- specific event name (enumerated below)
- `detail` -- event-specific payload; **never contains PII** (no names, emails, subjects -- only IDs, counts, and classifications)
- `prev_hash` -- SHA-256 of the previous record's raw JSON bytes; `null` for the first record

**Hash chain:** Each record's `prev_hash` is the SHA-256 of the raw bytes of the immediately preceding line (including the newline). Verifying the chain detects any record deletion or modification. The chain does not need to be verified in the normal write path -- only when performing an audit review or export.

```rust
use sha2::{Sha256, Digest};

pub struct AuditLogger {
    path: PathBuf,
    last_hash: Option<String>,
}

impl AuditLogger {
    pub fn append(&mut self, category: &str, event: &str, detail: serde_json::Value)
        -> Result<(), String>
    {
        let record = serde_json::json!({
            "ts": Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
            "v": 1,
            "category": category,
            "event": event,
            "detail": detail,
            "prev_hash": self.last_hash,
        });
        let line = serde_json::to_string(&record)
            .map_err(|e| format!("Audit serialize failed: {e}"))?;

        // Append-only: open with O_CREAT | O_APPEND | O_WRONLY
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|e| format!("Audit open failed: {e}"))?;

        use std::io::Write;
        writeln!(file, "{}", line)
            .map_err(|e| format!("Audit write failed: {e}"))?;

        // Update hash chain
        let mut hasher = Sha256::new();
        hasher.update(line.as_bytes());
        hasher.update(b"\n");
        self.last_hash = Some(hex::encode(hasher.finalize()));

        Ok(())
    }
}
```

`AuditLogger` lives in `AppState` as an `Arc<Mutex<AuditLogger>>`. The mutex serializes writes, which is correct since the append-only guarantee depends on no concurrent writers racing on the file offset.

### 2. Auditable Event Taxonomy

**Category: `security`**

| Event | When | Detail fields |
|-------|------|---------------|
| `db_key_generated` | First run -- new key generated | `{"action": "generated_and_stored"}` |
| `db_key_accessed` | DB opened successfully | `{"action": "retrieved_from_keychain"}` |
| `db_key_missing` | DB exists but key not in Keychain | `{"db_exists": true, "action": "blocked"}` |
| `db_migration_started` | Plaintext → encrypted migration begins | `{"db_size_bytes": N}` |
| `db_migration_completed` | Migration succeeded | `{"duration_ms": N}` |
| `app_unlock_attempted` | Touch ID / password prompt shown | `{"trigger": "idle_timeout"}` |
| `app_unlock_succeeded` | Touch ID accepted | `{}` |
| `app_unlock_failed` | Touch ID rejected or cancelled | `{"reason": "cancelled"}` |
| `oauth_connected` | Google OAuth grant completed | `{"scopes": ["calendar", "gmail", ...]}` |
| `oauth_revoked` | User disconnected Google account | `{}` |
| `icloud_warning_shown` | iCloud workspace scope detected | `{"workspace_path_category": "documents"}` |
| `icloud_warning_dismissed` | User dismissed warning | `{}` |

**Category: `data_access`**

| Event | When | Detail fields |
|-------|------|---------------|
| `google_calendar_sync` | Calendar fetch completed | `{"events_fetched": N, "range_days": 14}` |
| `gmail_sync` | Gmail fetch completed | `{"emails_fetched": N, "threads": N}` |
| `google_drive_import` | Drive document imported | `{"file_type": "docx", "size_bytes": N}` |
| `google_drive_watch_started` | Drive watch registered | `{}` |
| `clay_enrichment` | Clay API call made | `{"entity_type": "person", "tool": "searchContacts"}` |
| `gravatar_lookup` | Gravatar fetch made | `{"count": N}` |
| `linear_sync` | Linear sync completed | `{"issues_fetched": N}` |
| `granola_sync` | Granola transcript fetch | `{"transcripts_fetched": N}` |

Note: detail fields contain **counts and categories only** -- no names, emails, subject lines, or content. This satisfies the log PII hygiene rule from ADR-0092.

**Category: `ai_operation`**

| Event | When | Detail fields |
|-------|------|---------------|
| `entity_enrichment_started` | Intel queue dequeues an entity | `{"entity_type": "account", "model_tier": "Synthesis", "is_incremental": true}` |
| `entity_enrichment_completed` | Enrichment succeeded + schema validation passed | `{"entity_type": "account", "duration_ms": N, "token_estimate": N}` |
| `entity_enrichment_failed` | PTY error or schema validation failed | `{"entity_type": "account", "error_category": "schema_validation"}` |
| `email_enrichment_batch` | Email enrichment batch completed | `{"emails_processed": N, "failed": N, "model_tier": "Extraction"}` |
| `meeting_prep_generated` | Meeting prep frozen | `{"model_tier": "Synthesis", "duration_ms": N}` |
| `daily_workflow_completed` | Today workflow finished | `{"phases": ["prepare", "enrich", "deliver"], "duration_ms": N}` |

**Category: `anomaly`**

| Event | When | Detail fields |
|-------|------|---------------|
| `injection_tag_escape_detected` | `</user_data>` found in external content, was escaped | `{"source": "email_subject", "escaped": true}` |
| `injection_instruction_in_output` | AI output contained system prompt terms | `{"event_type": "enrichment", "terms_detected": ["chief of staff"]}` |
| `schema_validation_failed` | AI response did not conform to expected schema | `{"entity_type": "account", "error": "unexpected_fields"}` |
| `api_error_suspicious` | External API returned unexpected auth error | `{"service": "clay", "http_status": 401}` |

**Category: `config_change`**

| Event | When | Detail fields |
|-------|------|---------------|
| `workspace_path_changed` | User changed workspace location | `{"from_category": "documents", "to_category": "home"}` |
| `ai_provider_changed` | AI provider switched | `{"from": "claude_code", "to": "ollama"}` |
| `app_lock_timeout_changed` | Idle lock timeout updated | `{"minutes": 15}` |
| `enrichment_service_toggled` | Clay/Gravatar/Linear enabled or disabled | `{"service": "clay", "enabled": true}` |

**Category: `lifecycle`**

| Event | When | Detail fields |
|-------|------|---------------|
| `app_started` | App process started | `{"version": "0.16.1", "db_encrypted": true}` |
| `app_stopped` | App process stopping cleanly | `{"uptime_minutes": N}` |
| `audit_log_rotated` | Log rotation (>90 days entries pruned) | `{"records_pruned": N, "bytes_freed": N}` |
| `db_backup_completed` | Database backup completed | `{"backup_size_bytes": N}` |

### 3. Retention and Rotation

**Retention:** 90 days rolling (3x the existing `_audit/` 30-day retention). On app startup, records older than 90 days are pruned. Since the file is append-only and cannot have individual lines deleted, rotation works by:

1. On startup, read current `audit.log`
2. Filter to records newer than 90 days
3. Write the filtered records to `audit.log.new`
4. Atomic rename: `audit.log.new` -> `audit.log`
5. Recompute `last_hash` from the last remaining record

This breaks the hash chain at the rotation boundary (the oldest retained record's `prev_hash` may point to a pruned record). This is expected and documented -- chain verification covers the retained window only.

**Size:** JSON-lines format at ~200 bytes per record. At 50 events per day, 90 days = 4,500 records = ~900KB. The log file will not meaningfully impact storage.

### 4. Location and Protection

`~/.dailyos/audit.log` -- inside the `~/.dailyos/` directory, which is:
- Excluded from Time Machine per ADR-0092
- Not in iCloud Drive scope (dotfolder in HOME)
- `0o700` directory permissions
- File permissions: `0o600` (owner read/write only)

This satisfies AU-9 against other OS users and processes that don't have the login credentials. It does not protect against the owner themselves deleting the file, which is acceptable for a single-user local app -- the user owns their data.

### 5. Relationship to Existing Audit Mechanisms

**`audit.rs` / `_audit/` directory:** Retained as-is for AI raw output debugging. This is a developer/debugging facility, not a security audit trail. Its location in the workspace and plain-text format are appropriate for its purpose (a developer inspecting "what did Claude actually output?"). The two systems are complementary:
- `audit.rs` answers: "what did Claude say for entity X?"
- `audit.log` answers: "when did Claude process entity X, did schema validation pass, was there an anomaly?"

**`enrichment_log` SQLite table:** Retained for data provenance queries (what fields were updated by which source, when). The audit log's `ai_operation` events complement it with timing and validation outcomes. The SQLite tables are mutable and not AU-9 compliant -- they are for operational queries, not compliance audit.

**`hygiene_actions_log` and `processing_log`:** Retained for their specific pipeline tracking. Not merged into the audit log -- they serve different consumers (pipeline monitoring vs security review).

### 6. Settings UI: Activity Log

Settings --> Data --> Activity Log:

- Shows the last 100 audit records grouped by day
- Filter by category: Security / Data Access / AI Operations / Anomalies / All
- Anomaly records are visually flagged (amber color, anomaly icon)
- "Export" button writes the full retained log to a user-selected path as JSON-lines
- "Verify integrity" checks the hash chain and reports: "Log integrity verified" or "Warning: N records may have been modified" with the approximate date of the break

The export format is the raw JSON-lines file -- no transformation. This is the format a compliance reviewer can work with.

### 7. What Is Not Audited

- **Message content** -- email subjects, meeting titles, document content, entity names never appear in audit records. IDs, counts, and classifications only.
- **Individual signals** -- the signal bus emits hundreds of events per session. These are not audit log events; the existing `signals` SQLite table is the right store for signal history.
- **Frontend user interactions** -- page navigations, clicks, UI state. This is application telemetry, not audit trail. No telemetry is collected.
- **Model prompt content** -- prompts sent to Claude are not logged in the audit trail (they exist in `_audit/` raw output files for debugging). The audit trail logs that an enrichment happened and its outcome, not what was in the prompt.

## What Is Not Decided Here

**Remote audit log shipping**: Sending audit records to a SIEM or remote log aggregator (Splunk, Datadog, etc.). Out of scope -- DailyOS is local-first. If an enterprise deployment scenario emerges post-1.0, this could be added as an optional exporter.

**Real-time anomaly alerting**: Triggering a macOS notification when an injection anomaly is detected. Useful but not in scope for the initial implementation. The anomaly record in the audit log is the baseline; notifications can be added later.

**Log signing with a certificate**: Signing the audit log with a user-held private key for non-repudiation. The hash chain provides tamper detection; full cryptographic signing is disproportionate for the current threat model.

**Multi-user support**: The audit model assumes a single macOS user. If DailyOS ever adds team/shared deployments, the audit model needs revisiting -- each actor would need a distinct identity in the records.

## Consequences

- `AuditLogger` is added to `AppState` as `Arc<Mutex<AuditLogger>>`, initialized on startup after the DB opens.
- `~/.dailyos/audit.log` is created with `0o600` permissions on first write. The containing `~/.dailyos/` directory is already `0o700` per ADR-0092.
- Adding a `sha2` dependency (or using the `hex` + `sha2` already referenced in the SQLCipher work). Binary size impact: negligible.
- All pipeline code that currently generates enrichment events (`intel_queue.rs`, `prepare/email_enrich.rs`, `workflow/deliver.rs`, `executor.rs`) needs an `AuditLogger::append()` call at start and completion. These are fire-and-forget -- audit write failures are logged at WARN but never propagate to the caller.
- Audit writes are synchronous but append-only and small. Expected overhead: < 1ms per record. Not on the hot path.
- `_audit/` raw output files are retained. Retention stays at 30 days. The audit log has a separate 90-day retention.
- The Activity Log UI in Settings surfaces audit data to the user. This is the first place in the app where operational history is user-visible -- it directly addresses the transparency requirement for trust.
- The hash chain provides forensic tamper detection. The verification tool in Settings ("Verify integrity") makes this user-accessible without requiring technical expertise.
