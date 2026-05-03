//!  post-migration reconcile + repair binary.
//!
//! Runs the reconcile SQL from `scripts/reconcile_ghost_resurrection.sql`
//! against the workspace DB and reports/repairs ghost-resurrection
//! findings.
//!
//! Usage:
//!   reconcile_post_migration              — read-only reconcile; report findings.
//!   reconcile_post_migration --repair     — apply repair (re-tombstone via commit_claim).
//!
//! ## Status (W1 ship)
//!
//! Skeleton form. The binary fully detects + runs the reconcile SQL when
//! `intelligence_claims` exists; per-finding repair logic that consumes
//! `services::claims::commit_claim` lands when  (W3) ships that
//! module. Until then `--repair` is a no-op with a clear log message.

use std::path::PathBuf;
use std::process::ExitCode;

const RECONCILE_SQL_PATH: &str = "scripts/reconcile_ghost_resurrection.sql";

fn main() -> ExitCode {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    let args: Vec<String> = std::env::args().collect();
    let repair_mode = args.iter().any(|a| a == "--repair");

    log::info!(
        "DOS-311 reconcile post-migration: {}",
        if repair_mode { "REPAIR mode" } else { "REPORT mode" }
    );

    let sql_path = locate_reconcile_sql();
    let sql = match std::fs::read_to_string(&sql_path) {
        Ok(s) => s,
        Err(e) => {
            log::error!(
                "Failed to read reconcile SQL at {}: {e}",
                sql_path.display()
            );
            return ExitCode::from(2);
        }
    };

    let db = match dailyos_lib::db::ActionDb::open() {
        Ok(d) => d,
        Err(e) => {
            log::error!("Failed to open workspace DB: {e}");
            return ExitCode::from(2);
        }
    };

    // precondition check: intelligence_claims table must exist.
    let claims_exists: bool = db
        .conn_ref()
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name='intelligence_claims'",
            [],
            |_| Ok(true),
        )
        .unwrap_or(false);

    if !claims_exists {
        log::info!(
            "intelligence_claims table not present (DOS-7 / W3 not yet shipped). \
             Reconcile is a no-op until DOS-7 introduces the table. \
             Reconcile SQL is committed at {} for when that happens.",
            sql_path.display()
        );
        return ExitCode::from(0);
    }

    let mut stmt = match db.conn_ref().prepare(&sql) {
        Ok(s) => s,
        Err(e) => {
            log::error!("Failed to prepare reconcile SQL: {e}");
            return ExitCode::from(2);
        }
    };

    let findings: Vec<ReconcileFinding> = match stmt.query_map([], |row| {
        Ok(ReconcileFinding {
            subject_ref: row.get("subject_ref")?,
            claim_type: row.get("claim_type")?,
            field_path: row.get("field_path")?,
            dedup_key: row.get::<_, Option<String>>("dedup_key")?,
            item_hash: row.get::<_, Option<String>>("item_hash")?,
            projection_target: row.get("projection_target")?,
            dismissed_at: row.get("dismissed_at")?,
            sourced_at: row.get::<_, Option<String>>("sourced_at")?,
            match_path: row.get("match_path")?,
        })
    }) {
        Ok(rows) => match rows.collect::<Result<Vec<_>, _>>() {
            Ok(v) => v,
            Err(e) => {
                log::error!("Failed to collect reconcile rows: {e}");
                return ExitCode::from(2);
            }
        },
        Err(e) => {
            log::error!("Failed to query reconcile SQL: {e}");
            return ExitCode::from(2);
        }
    };

    if findings.is_empty() {
        log::info!("Reconcile clean: 0 ghost-resurrection findings.");
        return ExitCode::from(0);
    }

    log::warn!(
        "Reconcile found {} ghost-resurrection finding(s):",
        findings.len()
    );
    for (i, f) in findings.iter().enumerate() {
        log::warn!(
            "  [{}/{}] subject={} claim_type={} field={} match_path={} dedup_key={:?} item_hash={:?} dismissed_at={} sourced_at={:?} projection={}",
            i + 1, findings.len(),
            f.subject_ref, f.claim_type, f.field_path, f.match_path,
            f.dedup_key, f.item_hash, f.dismissed_at, f.sourced_at, f.projection_target,
        );
    }

    if !repair_mode {
        log::info!(
            "Run with --repair to re-apply tombstones via services::claims::commit_claim."
        );
        return ExitCode::from(1);
    }

    log::error!(
        "--repair: services::claims::commit_claim not yet available \
         (ships with DOS-7 / W3). Skeleton binary records findings but does \
         not yet apply repairs. Each finding's tombstone re-application \
         lands when DOS-7 introduces the commit_claim API per ADR-0113."
    );
    log::info!(
        "Until then, operators can: \
         (a) re-run reconcile after DOS-7 ships to apply repairs; \
         (b) manually re-tombstone via the dismiss UI; \
         (c) wait for DOS-301's projection sweep to repair drift on next claim touch."
    );
    ExitCode::from(2)
}

fn locate_reconcile_sql() -> PathBuf {
    let cwd_relative = PathBuf::from(RECONCILE_SQL_PATH);
    if cwd_relative.exists() {
        return cwd_relative;
    }
    let from_src_tauri = PathBuf::from("..").join(RECONCILE_SQL_PATH);
    if from_src_tauri.exists() {
        return from_src_tauri;
    }
    if let Ok(p) = std::env::var("DOS311_RECONCILE_SQL") {
        return PathBuf::from(p);
    }
    cwd_relative
}

#[derive(Debug)]
struct ReconcileFinding {
    subject_ref: String,
    claim_type: String,
    field_path: String,
    dedup_key: Option<String>,
    item_hash: Option<String>,
    projection_target: String,
    dismissed_at: String,
    sourced_at: Option<String>,
    match_path: String,
}
