use std::path::PathBuf;
use std::sync::Arc;

use chrono::{Duration, Utc};
use rusqlite::params;
use serde::Deserialize;

use crate::db::{ActionDb, LocalKeychain};

#[derive(Debug, Default, PartialEq, Eq)]
pub struct WatermarkDoctorReport {
    pub claims_below_floor: i64,
    pub compositions_below_floor: i64,
    pub zombie_attempts: i64,
    pub claims_missing_outbox: i64,
    pub compositions_missing_outbox: i64,
}

impl WatermarkDoctorReport {
    pub fn is_clean(&self) -> bool {
        self.claims_below_floor == 0
            && self.compositions_below_floor == 0
            && self.zombie_attempts == 0
            && self.claims_missing_outbox == 0
            && self.compositions_missing_outbox == 0
    }
}

pub fn run_from_args<I>(args: I) -> Option<i32>
where
    I: IntoIterator<Item = String>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if args.get(1).map(String::as_str) != Some("doctor") {
        return None;
    }

    let subcommand = args.get(2).map(String::as_str).unwrap_or("all");
    match subcommand {
        "watermarks" => Some(run_watermarks_cli()),
        "pairing" => Some(run_pairing_cli()),
        "all" => {
            let watermarks = run_watermarks_cli();
            println!();
            let pairing = run_pairing_cli();
            // Combined exit code: max-of-any so callers see failure if anything failed.
            Some(watermarks.max(pairing))
        }
        other => {
            eprintln!(
                "unknown doctor subcommand `{other}`; expected `watermarks`, `pairing`, or `all`"
            );
            Some(2)
        }
    }
}

fn run_watermarks_cli() -> i32 {
    match run_watermark_doctor() {
        Ok(report) if report.is_clean() => {
            println!("dailyos doctor watermarks: ok");
            0
        }
        Ok(report) => {
            println!("dailyos doctor watermarks: failed");
            println!("claims_below_floor={}", report.claims_below_floor);
            println!(
                "compositions_below_floor={}",
                report.compositions_below_floor
            );
            println!("zombie_attempts={}", report.zombie_attempts);
            println!("claims_missing_outbox={}", report.claims_missing_outbox);
            println!(
                "compositions_missing_outbox={}",
                report.compositions_missing_outbox
            );
            1
        }
        Err(error) => {
            eprintln!("dailyos doctor watermarks failed to run: {error}");
            1
        }
    }
}

fn run_pairing_cli() -> i32 {
    let report = inspect_pairing();
    if report.is_clean() {
        println!("dailyos doctor pairing: ok");
        println!("runtime_endpoint={}", report.runtime_endpoint_summary());
        println!("audit_log={}", report.audit_log_summary());
        0
    } else {
        println!("dailyos doctor pairing: needs attention");
        for issue in &report.issues {
            println!("issue: {issue}");
        }
        println!("runtime_endpoint={}", report.runtime_endpoint_summary());
        println!("audit_log={}", report.audit_log_summary());
        for remediation in &report.remediations {
            println!("remediation: {remediation}");
        }
        1
    }
}

pub fn run_watermark_doctor() -> Result<WatermarkDoctorReport, String> {
    // Open without startup recovery so zombie `mutation_attempts` rows can
    // be counted before they're auto-aborted. The doctor reports state;
    // it must not heal what it's inspecting (packet ac §36 + L2 cycle-2 P2).
    let db = ActionDb::open_for_inspection(Arc::new(LocalKeychain::new()))
        .map_err(|error| error.to_string())?;
    inspect_watermarks(&db)
}

pub fn inspect_watermarks(db: &ActionDb) -> Result<WatermarkDoctorReport, String> {
    let cutoff = (Utc::now() - Duration::seconds(60)).to_rfc3339();
    Ok(WatermarkDoctorReport {
        claims_below_floor: count_i64(
            db,
            "SELECT COUNT(*) FROM intelligence_claims WHERE claim_version < 1",
            [],
        )?,
        compositions_below_floor: count_i64(
            db,
            "SELECT COUNT(*) FROM composition_versions WHERE composition_version < 1",
            [],
        )?,
        zombie_attempts: count_i64(
            db,
            "SELECT COUNT(*) FROM mutation_attempts
             WHERE status = 'in_flight' AND started_at < ?1",
            params![cutoff],
        )?,
        // claim_version = 1 is the post-migration baseline. Migration 172
        // backfills pre-existing rows to v=1 via a single summary event
        // (sentinel '__migration_172_backfill__'); per-row events would
        // fire a false invalidation storm. Outbox-integrity only checks
        // mutations from v>=2 onwards (every subsequent commit_claim has
        // its own version_events row at the matching current_version).
        claims_missing_outbox: count_i64(
            db,
            "SELECT COUNT(*)
             FROM intelligence_claims c
             WHERE c.claim_version >= 2
               AND NOT EXISTS (
                 SELECT 1
                 FROM version_events ve
                 JOIN mutation_attempts ma ON ma.mutation_id = ve.mutation_id
                 WHERE ve.claim_id = c.id
                   AND ve.current_version = c.claim_version
                   AND ve.cursor = ma.cursor
                   AND ma.status IN ('committed', 'aborted')
               )",
            [],
        )?,
        compositions_missing_outbox: count_i64(
            db,
            "SELECT COUNT(*)
             FROM composition_versions cv
             WHERE cv.composition_version >= 1
               AND NOT EXISTS (
                 SELECT 1
                 FROM version_events ve
                 JOIN mutation_attempts ma ON ma.mutation_id = ve.mutation_id
                 WHERE ve.composition_id = cv.composition_id
                   AND ve.current_version = cv.composition_version
                   AND ve.cursor = ma.cursor
                   AND ma.status IN ('committed', 'aborted')
               )",
            [],
        )?,
    })
}

fn count_i64<P>(db: &ActionDb, sql: &str, params: P) -> Result<i64, String>
where
    P: rusqlite::Params,
{
    db.conn_ref()
        .query_row(sql, params, |row| row.get(0))
        .map_err(|error| error.to_string())
}

/// Sentinel file contract — kept in sync with `surface_runtime::runtime_sentinel_path`.
/// Doctor mirrors the path computation rather than depending on the module to keep the
/// doctor CLI usable when the surface_runtime isn't fully bootable (the user is running
/// `dailyos doctor` precisely because the runtime can't pair).
fn doctor_runtime_sentinel_path() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|home| {
        let mut path = PathBuf::from(home);
        path.push(".dailyos");
        path.push("runtime-endpoint.json");
        path
    })
}

#[derive(Debug, Deserialize)]
struct SentinelPayload {
    #[serde(default)]
    port: Option<u16>,
    #[serde(default)]
    runtime_version: Option<String>,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct PairingDoctorReport {
    pub sentinel_present: bool,
    pub sentinel_path_known: bool,
    pub sentinel_port: Option<u16>,
    pub sentinel_runtime_version: Option<String>,
    pub audit_log_writable: bool,
    pub issues: Vec<String>,
    pub remediations: Vec<String>,
}

impl PairingDoctorReport {
    pub fn is_clean(&self) -> bool {
        self.issues.is_empty()
    }

    pub fn runtime_endpoint_summary(&self) -> String {
        match (self.sentinel_present, self.sentinel_port) {
            (true, Some(port)) => format!(
                "present (port={port}, runtime_version={})",
                self.sentinel_runtime_version
                    .as_deref()
                    .unwrap_or("unknown")
            ),
            (true, None) => "present-but-unparseable".to_string(),
            (false, _) => "absent".to_string(),
        }
    }

    pub fn audit_log_summary(&self) -> &'static str {
        if self.audit_log_writable {
            "writable"
        } else {
            "not-writable"
        }
    }
}

/// Inspect pairing-adjacent state without leaking secrets. Reports:
/// - Runtime sentinel file presence + parsed shape (port + runtime_version only)
/// - Audit log writeability (basic IO sanity)
///
/// Notes on what's NOT here:
/// - WP-side pairing marker lives in WordPress wp_options, which Tauri can't read directly.
///   For end-to-end pairing diagnosis, this doctor is paired with the Studio-side runbook
///   that walks the user through inspecting wp_options via WP-CLI or browser devtools.
/// - HMAC session keys live in keychain. The doctor MUST NOT print them; it only reports
///   "present" / "absent" if a keychain probe is added in a follow-up.
pub fn inspect_pairing() -> PairingDoctorReport {
    let mut report = PairingDoctorReport::default();

    match doctor_runtime_sentinel_path() {
        Some(path) => {
            report.sentinel_path_known = true;
            match std::fs::read_to_string(&path) {
                Ok(contents) => {
                    report.sentinel_present = true;
                    match serde_json::from_str::<SentinelPayload>(&contents) {
                        Ok(parsed) => {
                            report.sentinel_port = parsed.port;
                            report.sentinel_runtime_version = parsed.runtime_version;
                            if parsed.port.is_none() {
                                report.issues.push(
                                    "sentinel file present but `port` field is missing or invalid"
                                        .to_string(),
                                );
                                report.remediations.push(
                                    "Restart the DailyOS app to rewrite the sentinel file."
                                        .to_string(),
                                );
                            }
                        }
                        Err(_) => {
                            report.issues.push(
                                "sentinel file present but JSON parse failed".to_string(),
                            );
                            report.remediations.push(
                                "Delete ~/.dailyos/runtime-endpoint.json and restart the DailyOS app.".to_string(),
                            );
                        }
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    report.sentinel_present = false;
                    report.issues.push(
                        "sentinel file absent — DailyOS runtime is not running".to_string(),
                    );
                    report.remediations.push(
                        "Launch the DailyOS app. The sentinel file is written on bind.".to_string(),
                    );
                }
                Err(error) => {
                    report.issues.push(format!("sentinel read error: {error}"));
                    report.remediations.push(
                        "Check ~/.dailyos/ permissions; parent dir must be 0700 owned by you."
                            .to_string(),
                    );
                }
            }
        }
        None => {
            report.issues.push(
                "HOME env var unset; cannot derive sentinel path".to_string(),
            );
            report.remediations.push(
                "Set HOME or run dailyos doctor from a user shell.".to_string(),
            );
        }
    }

    // Audit log writeability: try to open ~/.dailyos/audit.log with append; the audit
    // logger uses 0600 perms via O_APPEND. The doctor probe just opens for-append to confirm
    // the runtime would be able to emit audit rows; it does NOT write a probe record.
    if let Some(home) = std::env::var_os("HOME") {
        let mut audit_path = PathBuf::from(home);
        audit_path.push(".dailyos");
        audit_path.push("audit.log");
        report.audit_log_writable = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&audit_path)
            .is_ok();
        if !report.audit_log_writable {
            report.issues.push(
                "audit log file at ~/.dailyos/audit.log cannot be opened for append".to_string(),
            );
            report.remediations.push(
                "Check ~/.dailyos/ permissions and disk space.".to_string(),
            );
        }
    }

    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static HOME_ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn clean_empty_watermark_schema_passes() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db = ActionDb::open_at_unencrypted(dir.path().join("doctor.sqlite")).expect("db");
        let report = inspect_watermarks(&db).expect("inspect");
        assert!(report.is_clean(), "unexpected report: {report:?}");
    }

    #[test]
    fn pairing_doctor_reports_absent_sentinel_with_remediation() {
        let _guard = HOME_ENV_LOCK.lock().unwrap();
        let dir = tempfile::tempdir().expect("tempdir");
        // Point HOME at a fresh empty dir; sentinel path resolves to <home>/.dailyos/runtime-endpoint.json
        // which doesn't exist → "absent" branch.
        let prior_home = std::env::var_os("HOME");
        // SAFETY: protected by HOME_ENV_LOCK; restored at end of test.
        unsafe {
            std::env::set_var("HOME", dir.path());
        }

        let report = inspect_pairing();

        // Restore HOME before assertions so a failing assert doesn't poison the global env.
        unsafe {
            match prior_home {
                Some(value) => std::env::set_var("HOME", value),
                None => std::env::remove_var("HOME"),
            }
        }

        assert!(report.sentinel_path_known);
        assert!(!report.sentinel_present);
        assert!(!report.is_clean());
        assert!(report.issues.iter().any(|i| i.contains("absent")));
        assert!(report.remediations.iter().any(|r| r.contains("Launch")));
        assert_eq!(report.runtime_endpoint_summary(), "absent");
    }

    #[test]
    fn pairing_doctor_parses_valid_sentinel() {
        let _guard = HOME_ENV_LOCK.lock().unwrap();
        let dir = tempfile::tempdir().expect("tempdir");
        let dailyos_dir = dir.path().join(".dailyos");
        std::fs::create_dir_all(&dailyos_dir).expect("mkdir");
        std::fs::write(
            dailyos_dir.join("runtime-endpoint.json"),
            r#"{"port":54321,"runtime_version":"1.4.3"}"#,
        )
        .expect("write sentinel");

        let prior_home = std::env::var_os("HOME");
        unsafe {
            std::env::set_var("HOME", dir.path());
        }

        let report = inspect_pairing();

        unsafe {
            match prior_home {
                Some(value) => std::env::set_var("HOME", value),
                None => std::env::remove_var("HOME"),
            }
        }

        assert!(report.sentinel_present);
        assert_eq!(report.sentinel_port, Some(54321));
        assert_eq!(report.sentinel_runtime_version.as_deref(), Some("1.4.3"));
        // No issues from sentinel parsing; audit_log writeability may or may not pass
        // depending on temp dir permissions but the sentinel parse path is what we're asserting.
        assert!(
            report
                .issues
                .iter()
                .all(|i| !i.contains("sentinel")),
            "unexpected sentinel issue: {:?}",
            report.issues
        );
    }

    #[test]
    fn pairing_doctor_flags_unparseable_sentinel() {
        let _guard = HOME_ENV_LOCK.lock().unwrap();
        let dir = tempfile::tempdir().expect("tempdir");
        let dailyos_dir = dir.path().join(".dailyos");
        std::fs::create_dir_all(&dailyos_dir).expect("mkdir");
        std::fs::write(dailyos_dir.join("runtime-endpoint.json"), "not json")
            .expect("write sentinel");

        let prior_home = std::env::var_os("HOME");
        unsafe {
            std::env::set_var("HOME", dir.path());
        }

        let report = inspect_pairing();

        unsafe {
            match prior_home {
                Some(value) => std::env::set_var("HOME", value),
                None => std::env::remove_var("HOME"),
            }
        }

        assert!(report.sentinel_present);
        assert!(report.sentinel_port.is_none());
        assert!(report.issues.iter().any(|i| i.contains("JSON parse failed")));
        assert!(report.remediations.iter().any(|r| r.contains("Delete")));
    }
}
