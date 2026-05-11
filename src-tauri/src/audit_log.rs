//! Tamper-evident audit log for enterprise observability (ADR-0094).
//!
//! Appends JSON-lines to `~/.dailyos/audit.log` with a SHA-256 hash chain.
//! Each record links to the previous via `prev_hash`, making deletions or
//! insertions detectable. Records are rotated at 90 days on startup.
//!
//! ## Actor attribution (W1-A0)
//!
//! Per ADR-0102 §7.6 and ADR-0111 §8, every audit record optionally carries
//! actor attribution (`actor_kind`, `actor_instance`, `wp_user_id`,
//! `actor_scopes`). For [`abilities_runtime::Actor::SurfaceClient`] invocations,
//! these fields are MANDATORY and emission MUST route through the canonical
//! helper [`emit_surface_audit`] (or [`AuditLogger::append_with_actor`]) so
//! the SurfaceClient invariant — `actor_instance` populated AND `wp_user_id`
//! populated — is enforced at the wire boundary.
//!
//! The storage format is append-only JSONL, not a relational table; the AC's
//! `(wp_user_id, created_at)` "index" is realised as the `wp_user_id` field
//! being directly extractable from each record line for forensic grep / jq
//! queries (W6-A documents the full forensic-query exercise).

use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use abilities_runtime::abilities::registry::{Actor, ScopeSet, SurfaceClientId};

/// Retention period for audit log records.
const RETENTION_DAYS: i64 = 90;

/// A single audit log record.
///
/// The four `actor_*` fields and `wp_user_id` were added in W1-A0
/// (ADR-0102 §7.6 + ADR-0111 §8). They are additive and optional on the wire:
/// existing v1.4.1 records (and new non-SurfaceClient emissions) serialize
/// without these keys via `skip_serializing_if`. The
/// [`AuditLogger::append_with_actor`] / [`emit_surface_audit`] write path
/// populates them; [`AuditLogger::append`] preserves pre-W1-A0 semantics
/// (`actor_kind: None`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditRecord {
    /// ISO-8601 timestamp.
    pub ts: String,
    /// Schema version (always 1).
    pub v: u8,
    /// Event category: security, data_access, ai, anomaly, config, system.
    pub category: String,
    /// Event name (snake_case identifier).
    pub event: String,
    /// Structured detail (counts, IDs, classifications — never PII).
    pub detail: serde_json::Value,
    /// SHA-256 hex of the previous record's line (null for first record).
    pub prev_hash: Option<String>,

    // --- W1-A0 actor attribution (ADR-0102 §7.6, ADR-0111 §8) ---
    /// Kind tag for the invoking [`Actor`]: `"agent"`, `"user"`, `"admin"`,
    /// `"system"`, or `"surface_client"`. `None` for legacy (pre-W1-A0) or
    /// untagged emissions.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor_kind: Option<String>,
    /// Stable, non-PII per-instance identity for `Actor::SurfaceClient`.
    /// `None` for every other actor variant and for untagged emissions.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor_instance: Option<SurfaceClientId>,
    /// WordPress user id for a `SurfaceClient` invocation. MUST be
    /// `Some(_)` whenever `actor_kind == Some("surface_client")`. `None`
    /// otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wp_user_id: Option<u64>,
    /// Scope grant the SurfaceClient invocation carried, serialised as a
    /// deterministic (sorted) list. `None` for non-SurfaceClient emissions.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor_scopes: Option<Vec<String>>,
}

/// Error returned by the [`emit_surface_audit`] /
/// [`AuditLogger::append_with_actor`] write path.
///
/// Distinct from the existing `append(...) -> Result<(), String>` shape: the
/// W1-A0 helper has two failure modes that must be distinguishable for
/// callers — the runtime contract (`SurfaceClientMissingWpUserId`) versus the
/// serialise/IO path (`Write`). Callers that need to soft-fail on IO but
/// hard-fail on contract violations match on the variant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuditError {
    /// The caller passed `Actor::SurfaceClient { .. }` but `wp_user_id` was
    /// `None` in [`AuditFields`]. ADR-0111 §8 forbids this — every paired
    /// SurfaceClient request carries a WP user id from the endpoint, and an
    /// emission without one means the request context was dropped between
    /// extraction and emission. Surfaced as a runtime contract error rather
    /// than swallowed.
    SurfaceClientMissingWpUserId,
    /// The serialise / append path failed (disk full, permission denied,
    /// JSON encoding error). Carries the underlying message for logging.
    Write(String),
}

impl std::fmt::Display for AuditError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuditError::SurfaceClientMissingWpUserId => f.write_str(
                "emit_surface_audit contract violation: Actor::SurfaceClient \
                 requires AuditFields.wp_user_id to be Some(_)",
            ),
            AuditError::Write(msg) => write!(f, "audit log write failed: {msg}"),
        }
    }
}

impl std::error::Error for AuditError {}

/// Structured fields for [`emit_surface_audit`] / [`AuditLogger::append_with_actor`].
///
/// Built via [`AuditFields::new`] (category + event + detail are mandatory) and
/// fluently extended with [`AuditFields::with_wp_user_id`]. The
/// [`Actor::SurfaceClient`] contract — `wp_user_id` MUST be `Some(_)` — is
/// validated at emission time, not in this builder, because the actor and
/// fields are independent inputs to the helper and only the helper sees both.
///
/// The builder intentionally does NOT accept `actor_instance` /
/// `actor_scopes` from the caller: those are derived from the `&Actor`
/// argument inside the helper so they cannot drift from the variant.
#[derive(Debug, Clone)]
pub struct AuditFields {
    /// Event category — same vocabulary as [`AuditLogger::append`]
    /// (`security`, `data_access`, `ai`, `anomaly`, `config`, `system`).
    pub category: String,
    /// Structured detail payload (counts, IDs, classifications — never PII).
    pub detail: serde_json::Value,
    /// WordPress user id from the SurfaceClient request context. REQUIRED
    /// for `Actor::SurfaceClient`; ignored for other actors (set to `None`).
    pub wp_user_id: Option<u64>,
}

impl AuditFields {
    /// Construct an [`AuditFields`] with category + detail. `wp_user_id`
    /// defaults to `None`; chain [`AuditFields::with_wp_user_id`] when the
    /// actor is a SurfaceClient.
    pub fn new(category: impl Into<String>, detail: serde_json::Value) -> Self {
        Self {
            category: category.into(),
            detail,
            wp_user_id: None,
        }
    }

    /// Attach the SurfaceClient invocation's `wp_user_id`. Required for any
    /// `Actor::SurfaceClient` emission; omitting it routes through
    /// [`AuditError::SurfaceClientMissingWpUserId`].
    #[must_use]
    pub fn with_wp_user_id(mut self, wp_user_id: u64) -> Self {
        self.wp_user_id = Some(wp_user_id);
        self
    }
}

/// Canonical kind tag for an [`Actor`] variant. Stable wire string — written
/// into `AuditRecord::actor_kind` and consumed by forensic queries / the
/// W6-A CI lint.
fn actor_kind_tag(actor: &Actor) -> &'static str {
    match actor {
        Actor::Agent => "agent",
        Actor::User => "user",
        Actor::Admin => "admin",
        Actor::System => "system",
        Actor::SurfaceClient { .. } => "surface_client",
    }
}

/// Render a [`ScopeSet`] as a deterministic, sorted `Vec<String>` for audit
/// emission. [`ScopeSet::iter`] yields in sorted order by construction; we
/// preserve it to keep forensic grep across log lines deterministic.
fn serialize_scopes(scopes: &ScopeSet) -> Vec<String> {
    scopes.iter().map(|s| s.to_string()).collect()
}

/// Canonical emission helper for actor-attributed audit events
/// (W1-A0; ADR-0102 §7.6; ADR-0111 §8).
///
/// Every audit emission that runs on behalf of an [`Actor::SurfaceClient`]
/// MUST route through this helper (or [`AuditLogger::append_with_actor`],
/// which is the underlying primitive). The helper:
///
/// 1. Tags the record with `actor_kind` derived from the variant.
/// 2. For `Actor::SurfaceClient`, populates `actor_instance` and
///    `actor_scopes` from the variant's payload and REQUIRES
///    `fields.wp_user_id` to be `Some(_)`. A missing `wp_user_id` returns
///    [`AuditError::SurfaceClientMissingWpUserId`] without writing the
///    record — a paired SurfaceClient invocation without a WP user id
///    means request-context propagation broke somewhere upstream.
/// 3. For every other actor variant, `actor_instance`, `wp_user_id`, and
///    `actor_scopes` are written as `None`. `wp_user_id` on `AuditFields`
///    is ignored when the actor is not a SurfaceClient.
///
/// W2-C (pairing), W2-D (rate-limit denial), W3-C (MCP invocation), W4-E
/// (nonce event), and W5-A (feedback application) all bind to this helper
/// per their issue acceptance criteria; downstream callers wire in their
/// own waves.
pub fn emit_surface_audit(
    logger: &mut AuditLogger,
    event_kind: &str,
    actor: &Actor,
    fields: AuditFields,
) -> Result<(), AuditError> {
    logger.append_with_actor(event_kind, actor, fields)
}

/// Append-only audit logger with hash chain continuity.
pub struct AuditLogger {
    path: PathBuf,
    last_hash: Option<String>,
}

impl AuditLogger {
    /// Create a new AuditLogger, resuming the hash chain from an existing file.
    pub fn new(path: PathBuf) -> Self {
        let last_hash = read_last_line_hash(&path);
        Self { path, last_hash }
    }

    /// Append a record to the audit log (legacy, pre-W1-A0 emission path).
    ///
    /// Opens the file with O_APPEND for each write to avoid holding a file
    /// handle. Write failures are logged at WARN and never propagated.
    ///
    /// This path emits with no actor attribution — `actor_kind`,
    /// `actor_instance`, `wp_user_id`, and `actor_scopes` are all `None`.
    /// Use [`AuditLogger::append_with_actor`] (or the free function
    /// [`emit_surface_audit`]) for any new emission site that runs on
    /// behalf of an [`Actor`]. Existing v1.4.0/v1.4.1 callers continue to
    /// route through this path; the W6-A migration sweep tightens that.
    pub fn append(
        &mut self,
        category: &str,
        event: &str,
        detail: serde_json::Value,
    ) -> Result<(), String> {
        self.write_record(category, event, detail, None, None, None, None)
            .map_err(|err| match err {
                AuditError::Write(msg) => msg,
                AuditError::SurfaceClientMissingWpUserId => {
                    "internal: append() emitted SurfaceClient contract error".to_string()
                }
            })
    }

    /// Append a record with [`Actor`] attribution per W1-A0.
    ///
    /// This is the canonical write primitive behind [`emit_surface_audit`].
    /// It pattern-matches the actor variant, derives `actor_kind`, and for
    /// `Actor::SurfaceClient { instance, scopes }` requires
    /// `fields.wp_user_id` to be `Some(_)` (returns
    /// [`AuditError::SurfaceClientMissingWpUserId`] without writing
    /// otherwise). For non-SurfaceClient actors, `wp_user_id` /
    /// `actor_instance` / `actor_scopes` are written as `None` regardless of
    /// whether `fields.wp_user_id` was supplied — only SurfaceClient
    /// invocations carry WP identity.
    pub fn append_with_actor(
        &mut self,
        event: &str,
        actor: &Actor,
        fields: AuditFields,
    ) -> Result<(), AuditError> {
        let kind = actor_kind_tag(actor);
        let (actor_instance, wp_user_id, actor_scopes) = match actor {
            Actor::SurfaceClient { instance, scopes } => {
                let wp_user_id = fields
                    .wp_user_id
                    .ok_or(AuditError::SurfaceClientMissingWpUserId)?;
                (
                    Some(instance.clone()),
                    Some(wp_user_id),
                    Some(serialize_scopes(scopes)),
                )
            }
            // Per AC line 293, `wp_user_id` is SurfaceClient-only: even if
            // the caller supplied one in `AuditFields` for a non-SurfaceClient
            // actor, we drop it so the schema cannot be misused as a generic
            // user-id channel for non-paired actors.
            Actor::Agent | Actor::User | Actor::Admin | Actor::System => (None, None, None),
        };

        self.write_record(
            &fields.category,
            event,
            fields.detail,
            Some(kind.to_string()),
            actor_instance,
            wp_user_id,
            actor_scopes,
        )
    }

    /// Shared write primitive used by both [`AuditLogger::append`] and
    /// [`AuditLogger::append_with_actor`]. Centralising the JSONL encode +
    /// file open + permission set + hash-chain advance means the two public
    /// entrypoints cannot drift.
    #[allow(clippy::too_many_arguments)]
    fn write_record(
        &mut self,
        category: &str,
        event: &str,
        detail: serde_json::Value,
        actor_kind: Option<String>,
        actor_instance: Option<SurfaceClientId>,
        wp_user_id: Option<u64>,
        actor_scopes: Option<Vec<String>>,
    ) -> Result<(), AuditError> {
        let record = AuditRecord {
            ts: Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            v: 1,
            category: category.to_string(),
            event: event.to_string(),
            detail,
            prev_hash: self.last_hash.clone(),
            actor_kind,
            actor_instance,
            wp_user_id,
            actor_scopes,
        };

        let line = serde_json::to_string(&record)
            .map_err(|e| AuditError::Write(format!("Serialize error: {e}")))?;

        // Ensure parent directory exists
        if let Some(parent) = self.path.parent() {
            if !parent.exists() {
                #[allow(
                    clippy::let_underscore_must_use,
                    reason = "intentional best-effort discard; preserves existing non-blocking behavior"
                )]
                let _ = fs::create_dir_all(parent);
            }
        }

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|e| AuditError::Write(format!("Failed to open audit log: {e}")))?;

        // Set permissions to owner-only on creation
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            #[allow(
                clippy::let_underscore_must_use,
                reason = "intentional best-effort discard; preserves existing non-blocking behavior"
            )]
            let _ = file.set_permissions(fs::Permissions::from_mode(0o600));
        }

        writeln!(file, "{}", line)
            .map_err(|e| AuditError::Write(format!("Failed to write audit log: {e}")))?;

        // Update hash chain
        self.last_hash = Some(hash_line(&line));

        Ok(())
    }

    /// Get the file path for this audit log.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Rotate the audit log, removing records older than 90 days.
///
/// Returns `(records_pruned, bytes_freed)`.
pub fn rotate_audit_log(logger: &mut AuditLogger) -> (usize, u64) {
    let path = logger.path.clone();
    if !path.exists() {
        return (0, 0);
    }

    let original_size = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);

    let cutoff = Utc::now() - chrono::Duration::days(RETENTION_DAYS);
    let cutoff_str = cutoff.to_rfc3339();

    // Read all lines, keep only those within retention
    let lines: Vec<String> = match fs::read_to_string(&path) {
        Ok(content) => content.lines().map(String::from).collect(),
        Err(e) => {
            log::warn!("Failed to read audit log for rotation: {e}");
            return (0, 0);
        }
    };

    let mut retained = Vec::new();
    let mut pruned = 0usize;

    for line in &lines {
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<AuditRecord>(line) {
            Ok(record) => {
                if record.ts >= cutoff_str {
                    retained.push(line.clone());
                } else {
                    pruned += 1;
                }
            }
            Err(_) => {
                // Keep unparseable lines (don't silently drop data)
                retained.push(line.clone());
            }
        }
    }

    if pruned == 0 {
        return (0, 0);
    }

    // Rewrite the hash chain for retained records
    let mut rewritten = Vec::new();
    let mut prev_hash: Option<String> = None;

    for line in &retained {
        match serde_json::from_str::<AuditRecord>(line) {
            Ok(mut record) => {
                record.prev_hash = prev_hash;
                let new_line = serde_json::to_string(&record).unwrap_or_else(|_| line.clone());
                prev_hash = Some(hash_line(&new_line));
                rewritten.push(new_line);
            }
            Err(_) => {
                prev_hash = Some(hash_line(line));
                rewritten.push(line.clone());
            }
        }
    }

    // Atomic write: write to temp, rename over original
    let rotating_path = path.with_extension("log.rotating");
    match File::create(&rotating_path) {
        Ok(mut f) => {
            for line in &rewritten {
                if writeln!(f, "{}", line).is_err() {
                    log::warn!("Failed to write rotating audit log");
                    #[allow(
                        clippy::let_underscore_must_use,
                        reason = "intentional best-effort discard; preserves existing non-blocking behavior"
                    )]
                    let _ = fs::remove_file(&rotating_path);
                    return (0, 0);
                }
            }
        }
        Err(e) => {
            log::warn!("Failed to create rotating audit log: {e}");
            return (0, 0);
        }
    }

    if let Err(e) = fs::rename(&rotating_path, &path) {
        log::warn!("Failed to rename rotating audit log: {e}");
        #[allow(
            clippy::let_underscore_must_use,
            reason = "intentional best-effort discard; preserves existing non-blocking behavior"
        )]
        let _ = fs::remove_file(&rotating_path);
        return (0, 0);
    }

    // Update logger's hash chain from last retained record
    logger.last_hash = prev_hash;

    let new_size = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    let bytes_freed = original_size.saturating_sub(new_size);

    (pruned, bytes_freed)
}

/// Verify the hash chain integrity of an audit log file.
///
/// Returns `Ok(count)` if all records verify, or `Err((line_number, message))`
/// on the first broken link.
pub fn verify_audit_log(path: &Path) -> Result<usize, (usize, String)> {
    let file = File::open(path).map_err(|e| (0, format!("Cannot open audit log: {e}")))?;
    let reader = BufReader::new(file);

    let mut prev_hash: Option<String> = None;
    let mut count = 0usize;

    for (idx, line_result) in reader.lines().enumerate() {
        let line = line_result.map_err(|e| (idx + 1, format!("Read error: {e}")))?;
        if line.trim().is_empty() {
            continue;
        }

        let record: AuditRecord =
            serde_json::from_str(&line).map_err(|e| (idx + 1, format!("Parse error: {e}")))?;

        if record.prev_hash != prev_hash {
            return Err((
                idx + 1,
                format!(
                    "Hash chain broken: expected {:?}, found {:?}",
                    prev_hash, record.prev_hash
                ),
            ));
        }

        prev_hash = Some(hash_line(&line));
        count += 1;
    }

    Ok(count)
}

/// Read audit log records from file (most recent last).
///
/// Optionally filters by category and limits the result count.
pub fn read_records(path: &Path, limit: usize, category_filter: Option<&str>) -> Vec<AuditRecord> {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let mut records: Vec<AuditRecord> = content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();

    if let Some(cat) = category_filter {
        records.retain(|r| r.category == cat);
    }

    // Return the most recent `limit` records (tail of the file)
    let start = records.len().saturating_sub(limit);
    records[start..].to_vec()
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Compute SHA-256 hex hash of a line (without trailing newline).
fn hash_line(line: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(line.as_bytes());
    hasher.update(b"\n");
    hex::encode(hasher.finalize())
}

/// Read the last non-empty line of a file and compute its hash.
fn read_last_line_hash(path: &Path) -> Option<String> {
    let content = fs::read_to_string(path).ok()?;
    let last_line = content.lines().rfind(|l| !l.trim().is_empty())?;
    Some(hash_line(last_line))
}

/// Get the default audit log path (~/.dailyos/audit.log).
pub fn default_audit_log_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join(".dailyos")
        .join("audit.log")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_logger() -> (tempfile::TempDir, AuditLogger) {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("audit.log");
        let logger = AuditLogger::new(path);
        (dir, logger)
    }

    #[test]
    fn test_append_creates_chain() {
        let (_dir, mut logger) = temp_logger();

        logger
            .append("system", "event_a", serde_json::json!({"n": 1}))
            .unwrap();
        logger
            .append("system", "event_b", serde_json::json!({"n": 2}))
            .unwrap();
        logger
            .append("system", "event_c", serde_json::json!({"n": 3}))
            .unwrap();

        let count = verify_audit_log(logger.path()).unwrap();
        assert_eq!(count, 3);
    }

    #[test]
    fn test_rotation_prunes_old() {
        let (_dir, mut logger) = temp_logger();

        // Write 5 records with old timestamps manually
        let old_ts = (Utc::now() - chrono::Duration::days(100))
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        for i in 0..5 {
            let record = AuditRecord {
                ts: old_ts.clone(),
                v: 1,
                category: "system".to_string(),
                event: format!("old_{i}"),
                detail: serde_json::json!({}),
                prev_hash: logger.last_hash.clone(),
                actor_kind: None,
                actor_instance: None,
                wp_user_id: None,
                actor_scopes: None,
            };
            let line = serde_json::to_string(&record).unwrap();
            let mut f = OpenOptions::new()
                .create(true)
                .append(true)
                .open(logger.path())
                .unwrap();
            writeln!(f, "{}", line).unwrap();
            logger.last_hash = Some(hash_line(&line));
        }

        // Write 3 recent records
        for i in 0..3 {
            logger
                .append("system", &format!("recent_{i}"), serde_json::json!({}))
                .unwrap();
        }

        let (pruned, _) = rotate_audit_log(&mut logger);
        assert_eq!(pruned, 5);

        // Verify chain is still valid after rotation
        let count = verify_audit_log(logger.path()).unwrap();
        assert_eq!(count, 3);
    }

    #[test]
    fn test_verify_detects_deletion() {
        let (_dir, mut logger) = temp_logger();

        for i in 0..5 {
            logger
                .append("system", &format!("event_{i}"), serde_json::json!({}))
                .unwrap();
        }

        // Delete the middle line (line 3 of 5)
        let content = fs::read_to_string(logger.path()).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        let mut modified = Vec::new();
        for (i, line) in lines.iter().enumerate() {
            if i != 2 {
                modified.push(*line);
            }
        }
        fs::write(logger.path(), modified.join("\n") + "\n").unwrap();

        let result = verify_audit_log(logger.path());
        assert!(result.is_err());
        let (line_no, _msg) = result.unwrap_err();
        // Line 3 (1-indexed) should fail because its prev_hash references the deleted line 2
        assert_eq!(line_no, 3);
    }

    #[test]
    fn test_write_failure_does_not_panic() {
        // Path to a non-existent deeply nested directory that can't be created
        let mut logger = AuditLogger::new(PathBuf::from("/dev/null/impossible/audit.log"));
        let result = logger.append("system", "test", serde_json::json!({}));
        // Should return Err, not panic
        assert!(result.is_err());
    }

    #[test]
    fn test_resume_chain_across_restarts() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("audit.log");

        // Session 1
        {
            let mut logger = AuditLogger::new(path.clone());
            logger
                .append("system", "event_1", serde_json::json!({}))
                .unwrap();
            logger
                .append("system", "event_2", serde_json::json!({}))
                .unwrap();
        }

        // Session 2 (new logger instance resumes chain)
        {
            let mut logger = AuditLogger::new(path.clone());
            logger
                .append("system", "event_3", serde_json::json!({}))
                .unwrap();
        }

        // Verify full chain
        let count = verify_audit_log(&path).unwrap();
        assert_eq!(count, 3);
    }

    #[test]
    fn test_read_records_with_filter() {
        let (_dir, mut logger) = temp_logger();

        logger
            .append("security", "login", serde_json::json!({}))
            .unwrap();
        logger
            .append("system", "started", serde_json::json!({}))
            .unwrap();
        logger
            .append("security", "logout", serde_json::json!({}))
            .unwrap();

        let all = read_records(logger.path(), 100, None);
        assert_eq!(all.len(), 3);

        let security = read_records(logger.path(), 100, Some("security"));
        assert_eq!(security.len(), 2);

        let limited = read_records(logger.path(), 1, None);
        assert_eq!(limited.len(), 1);
        assert_eq!(limited[0].event, "logout");
    }

    // -----------------------------------------------------------------
    // W1-A0 — emit_surface_audit + actor attribution tests
    // -----------------------------------------------------------------

    use abilities_runtime::abilities::registry::{ScopeSet, SurfaceClientId, SurfaceScope};

    /// Build a [`ScopeSet`] from a fixed scope name. The W1-A construction
    /// path is lenient when the allowlist has not been initialised — which is
    /// the state under `cargo test --lib` since the macro registry boot does
    /// not run — so unknown scopes are accepted here.
    fn scope_set(scope: &str) -> ScopeSet {
        ScopeSet::new([SurfaceScope::new(scope)]).expect("non-empty scope set")
    }

    #[test]
    fn emit_surface_audit_with_wp_user_id_writes_record() {
        let (_dir, mut logger) = temp_logger();
        let actor = Actor::SurfaceClient {
            instance: SurfaceClientId::new("studio-instance-a"),
            scopes: scope_set("read.composition"),
        };
        let fields = AuditFields::new("data_access", serde_json::json!({"page": "compose"}))
            .with_wp_user_id(42);

        emit_surface_audit(&mut logger, "composition_view", &actor, fields)
            .expect("emit OK for SurfaceClient + wp_user_id");

        let records = read_records(logger.path(), 10, None);
        assert_eq!(records.len(), 1);
        let r = &records[0];
        assert_eq!(r.event, "composition_view");
        assert_eq!(r.actor_kind.as_deref(), Some("surface_client"));
        assert_eq!(
            r.actor_instance.as_ref().map(|id| id.to_string()),
            Some("studio-instance-a".to_string())
        );
        assert_eq!(r.wp_user_id, Some(42));
        assert_eq!(
            r.actor_scopes.as_deref(),
            Some(&["read.composition".to_string()][..])
        );
    }

    #[test]
    fn emit_surface_audit_rejects_surface_client_without_wp_user_id() {
        let (_dir, mut logger) = temp_logger();
        let actor = Actor::SurfaceClient {
            instance: SurfaceClientId::new("studio-instance-b"),
            scopes: scope_set("submit.feedback"),
        };
        let fields = AuditFields::new("security", serde_json::json!({}));

        let err = emit_surface_audit(&mut logger, "feedback_applied", &actor, fields)
            .expect_err("contract violation should reject");
        assert_eq!(err, AuditError::SurfaceClientMissingWpUserId);

        // The contract violation must NOT have written a record.
        let records = read_records(logger.path(), 10, None);
        assert!(records.is_empty(), "no record should be written on reject");
    }

    #[test]
    fn emit_surface_audit_accepts_non_surface_client_without_wp_user_id() {
        let (_dir, mut logger) = temp_logger();
        let fields = AuditFields::new("system", serde_json::json!({"phase": "boot"}));

        emit_surface_audit(&mut logger, "started", &Actor::User, fields)
            .expect("non-SurfaceClient does not require wp_user_id");

        let records = read_records(logger.path(), 10, None);
        assert_eq!(records.len(), 1);
        let r = &records[0];
        assert_eq!(r.actor_kind.as_deref(), Some("user"));
        // Non-SurfaceClient: every SurfaceClient-only field stays None.
        assert!(r.actor_instance.is_none());
        assert!(r.wp_user_id.is_none());
        assert!(r.actor_scopes.is_none());
    }

    #[test]
    fn emit_surface_audit_drops_wp_user_id_for_non_surface_client() {
        // Per AC line 293, wp_user_id is SurfaceClient-only — a caller that
        // mistakenly supplies one for an Agent/User/Admin/System actor must
        // have it dropped, not written, so the schema cannot be repurposed.
        let (_dir, mut logger) = temp_logger();
        let fields = AuditFields::new("ai", serde_json::json!({})).with_wp_user_id(7);

        emit_surface_audit(&mut logger, "agent_event", &Actor::Agent, fields)
            .expect("non-SurfaceClient emission with stray wp_user_id is OK");

        let records = read_records(logger.path(), 10, None);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].actor_kind.as_deref(), Some("agent"));
        assert!(records[0].wp_user_id.is_none());
    }

    #[test]
    fn audit_record_round_trips_actor_fields_through_jsonl() {
        let (_dir, mut logger) = temp_logger();
        let actor = Actor::SurfaceClient {
            instance: SurfaceClientId::new("studio-instance-c"),
            scopes: ScopeSet::new([
                SurfaceScope::new("read.account_overview"),
                SurfaceScope::new("submit.feedback"),
            ])
            .expect("non-empty scope set"),
        };
        let fields = AuditFields::new(
            "data_access",
            serde_json::json!({"surface": "account_overview"}),
        )
        .with_wp_user_id(99);

        emit_surface_audit(&mut logger, "account_view", &actor, fields).unwrap();

        // Read raw bytes from disk and parse a fresh AuditRecord — this is
        // the forensic round-trip path (AC line 294 / W6-A forensic skeleton).
        let raw = fs::read_to_string(logger.path()).expect("read audit log");
        let line = raw.lines().next().expect("at least one line");
        let parsed: AuditRecord = serde_json::from_str(line).expect("parse audit record");

        assert_eq!(parsed.actor_kind.as_deref(), Some("surface_client"));
        assert_eq!(
            parsed.actor_instance.as_ref().map(|id| id.to_string()),
            Some("studio-instance-c".to_string())
        );
        assert_eq!(parsed.wp_user_id, Some(99));
        // ScopeSet iterates in sorted order — serialised list preserves it.
        assert_eq!(
            parsed.actor_scopes.as_deref(),
            Some(
                &[
                    "read.account_overview".to_string(),
                    "submit.feedback".to_string(),
                ][..]
            )
        );

        // Hash chain still verifies with the new fields present.
        let count = verify_audit_log(logger.path()).expect("chain verifies");
        assert_eq!(count, 1);
    }

    #[test]
    fn legacy_records_without_actor_fields_still_parse() {
        // Simulate a pre-W1-A0 record on disk: the four actor_* keys are
        // absent. Serde must default them to None via `#[serde(default)]`.
        let legacy = r#"{"ts":"2026-05-10T00:00:00.000Z","v":1,"category":"system","event":"legacy","detail":{},"prev_hash":null}"#;
        let parsed: AuditRecord = serde_json::from_str(legacy).expect("legacy parses");
        assert!(parsed.actor_kind.is_none());
        assert!(parsed.actor_instance.is_none());
        assert!(parsed.wp_user_id.is_none());
        assert!(parsed.actor_scopes.is_none());
    }
}
