//! Tamper-evident audit log for enterprise observability (ADR-0094).
//!
//! Appends JSON-lines to `~/.dailyos/audit.log` with a SHA-256 hash chain.
//! Each record links to the previous via `prev_hash`, making deletions or
//! insertions detectable. Records are rotated at 90 days on startup.

use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Retention period for audit log records.
const RETENTION_DAYS: i64 = 90;

/// A single audit log record.
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

    /// Append a record to the audit log.
    ///
    /// Opens the file with O_APPEND for each write to avoid holding a file
    /// handle. Write failures are logged at WARN and never propagated.
    pub fn append(
        &mut self,
        category: &str,
        event: &str,
        detail: serde_json::Value,
    ) -> Result<(), String> {
        let record = AuditRecord {
            ts: Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            v: 1,
            category: category.to_string(),
            event: event.to_string(),
            detail,
            prev_hash: self.last_hash.clone(),
        };

        let line = serde_json::to_string(&record).map_err(|e| format!("Serialize error: {e}"))?;

        // Ensure parent directory exists
        if let Some(parent) = self.path.parent() {
            if !parent.exists() {
                let _ = fs::create_dir_all(parent);
            }
        }

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|e| format!("Failed to open audit log: {e}"))?;

        // Set permissions to owner-only on creation
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = file.set_permissions(fs::Permissions::from_mode(0o600));
        }

        writeln!(file, "{}", line).map_err(|e| format!("Failed to write audit log: {e}"))?;

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
}
