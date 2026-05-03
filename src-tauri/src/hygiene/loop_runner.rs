//! Background hygiene loop: startup delay, periodic scans, overnight enrichment,
//! age-based purge, and proactive detection.

use std::sync::Arc;

use chrono::Utc;
use tauri::{AppHandle, Emitter};

use crate::state::AppState;

use super::{narrative, run_hygiene_scan, HygieneReport, SCAN_INTERVAL_SECS, STARTUP_DELAY_SECS};

fn is_overnight_window() -> bool {
    use chrono::Local;
    let hour = Local::now()
        .format("%H")
        .to_string()
        .parse::<u32>()
        .unwrap_or(12);
    (2..=3).contains(&hour)
}

/// Background loop: runs scan on startup (30s delay), then every 4 hours.
pub async fn run_hygiene_loop(state: Arc<AppState>, app: AppHandle) {
    // Wait for startup to complete
    tokio::time::sleep(std::time::Duration::from_secs(STARTUP_DELAY_SECS)).await;

    // Log DB size at startup and emit warning if needed
    let startup_size = crate::db::data_lifecycle::log_db_size_at_startup();
    if startup_size >= 500_000_000 {
        let _ = app.emit("db-size-warning", startup_size);
    }

    log::info!("HygieneLoop: started");

    // Track last purge date to run age-based purge once per day
    let mut last_purge_date: Option<chrono::NaiveDate> = None;

    loop {
        // Dev mode isolation: pause background processing while dev sandbox is active
        if crate::db::is_dev_db_mode() {
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            continue;
        }

        // Read config-driven interval each iteration (changes take effect next cycle)
        let interval = state
            .config
            .read()
            .as_ref()
            .map(|c| c.hygiene_scan_interval_hours as u64 * 3600)
            .unwrap_or(SCAN_INTERVAL_SECS);

        // Prevent overlap with manual scan runs.
        let began_scan = state
            .hygiene
            .scan_running
            .compare_exchange(
                false,
                true,
                std::sync::atomic::Ordering::AcqRel,
                std::sync::atomic::Ordering::Acquire,
            )
            .is_ok();

        if !began_scan {
            log::debug!("HygieneLoop: skipping scan (another hygiene scan is already running)");
            tokio::time::sleep(std::time::Duration::from_secs(interval)).await;
            continue;
        }

        // Hygiene is lowest priority -- skip if heavy work is in progress.
        let permit = state.permits.pty.try_acquire();
        if permit.is_err() {
            log::debug!("HygieneLoop: skipping scan -- heavy work in progress");
            state
                .hygiene
                .scan_running
                .store(false, std::sync::atomic::Ordering::Release);
            tokio::time::sleep(std::time::Duration::from_secs(interval)).await;
            continue;
        }

        // Check for overnight window -- use expanded scan with higher AI budget
        if is_overnight_window() {
            let overnight = try_run_overnight(&state);
            if let Some(report) = overnight {
                log::info!(
                    "HygieneLoop: overnight scan -- {} entities refreshed, {} names resolved",
                    report.entities_refreshed,
                    report.names_resolved,
                );
            }
        }

        // Run regular scan synchronously (all locks drop before the next await)
        let report = try_run_scan(&state);

        // Release permit after scan completes
        drop(permit);

        if let Some(report) = report {
            log_scan_report(&report);

            // Store report for frontend access
            {
                let mut guard = state.hygiene.report.lock();
                *guard = Some(report);
            }
            {
                let mut guard = state.hygiene.last_scan_at.lock();
                *guard = Some(Utc::now().to_rfc3339());
            }
        }

        // Run proactive detection scan after hygiene fixes
        match crate::proactive::scanner::run_proactive_scan(&state) {
            Ok(n) if n > 0 => log::info!("HygieneLoop: {} proactive insights detected", n),
            Err(e) => log::warn!("HygieneLoop: proactive scan failed: {}", e),
            _ => {}
        }

        // Prune old audit trail files
        if let Some(config) = state.config.read().clone() {
            let workspace = std::path::Path::new(&config.workspace_path);
            let pruned = crate::audit::prune_audit_files(workspace);
            if pruned > 0 {
                log::info!("HygieneLoop: pruned {} old audit files", pruned);
            }
        }

        // Age-based purge -- runs once per day, not every scan cycle
        let today = chrono::Local::now().date_naive();
        let should_purge = last_purge_date.is_none_or(|d| d < today);
        if should_purge {
            run_daily_purge(&state, &app);
            last_purge_date = Some(today);
        }

        {
            let mut guard = state.hygiene.next_scan_at.lock();
            *guard = Some((Utc::now() + chrono::Duration::seconds(interval as i64)).to_rfc3339());
        }
        state
            .hygiene
            .scan_running
            .store(false, std::sync::atomic::Ordering::Release);

        tokio::time::sleep(std::time::Duration::from_secs(interval)).await;
    }
}

fn log_scan_report(report: &HygieneReport) {
    let total_gaps = report.unnamed_people
        + report.unknown_relationships
        + report.missing_intelligence
        + report.stale_intelligence
        + report.unsummarized_files;

    let total_fixes = report.fixes.relationships_reclassified
        + report.fixes.summaries_extracted
        + report.fixes.meeting_counts_updated
        + report.fixes.names_resolved
        + report.fixes.people_linked_by_domain
        + report.fixes.renewals_rolled_over
        + report.fixes.ai_enrichments_enqueued;

    if total_gaps > 0 || total_fixes > 0 {
        log::info!(
            "HygieneLoop: {} gaps detected, {} fixes applied \
             (relationships={}, summaries={}, counts={}, \
             names={}, domain_links={}, renewals={}, ai_enqueued={})",
            total_gaps,
            total_fixes,
            report.fixes.relationships_reclassified,
            report.fixes.summaries_extracted,
            report.fixes.meeting_counts_updated,
            report.fixes.names_resolved,
            report.fixes.people_linked_by_domain,
            report.fixes.renewals_rolled_over,
            report.fixes.ai_enrichments_enqueued,
        );
    } else {
        log::debug!("HygieneLoop: clean -- no gaps detected");
    }
}

fn run_daily_purge(state: &AppState, app: &AppHandle) {
    if let Ok(db) = crate::db::ActionDb::open() {
        let purge_report = crate::db::data_lifecycle::run_age_based_purge(&db);
        if purge_report.total() > 0 {
            log::info!(
                "HygieneLoop: age-based purge -- signals={}, email_signals={}, emails={}, embeddings={}",
                purge_report.signals_purged,
                purge_report.email_signals_purged,
                purge_report.emails_purged,
                purge_report.embeddings_purged,
            );
            {
                let mut audit = state.audit_log.lock();
                let _ = audit.append(
                    "system",
                    "age_based_purge",
                    serde_json::json!({
                        "signals_purged": purge_report.signals_purged,
                        "email_signals_purged": purge_report.email_signals_purged,
                        "emails_purged": purge_report.emails_purged,
                        "embeddings_purged": purge_report.embeddings_purged,
                    }),
                );
            }
        } else {
            log::debug!("HygieneLoop: age-based purge -- nothing to purge");
        }

        let size = crate::db::data_lifecycle::db_file_size_bytes();
        if size >= 500_000_000 {
            let _ = app.emit("db-size-warning", size);
        }
    }
}

/// Run overnight scan with expanded budget.
fn try_run_overnight(state: &AppState) -> Option<narrative::OvernightReport> {
    let config = state.config.read().clone()?;
    let db = crate::db::ActionDb::open().ok()?;
    let workspace = std::path::Path::new(&config.workspace_path);
    Some(narrative::run_overnight_scan(
        &db,
        &config,
        workspace,
        &state.intel_queue,
    ))
}

/// Synchronous scan attempt — releases everything when done.
fn try_run_scan(state: &AppState) -> Option<HygieneReport> {
    let config = state.config.read().clone()?;
    let db = crate::db::ActionDb::open().ok()?;

    let first_run = !state
        .hygiene
        .full_orphan_scan_done
        .swap(true, std::sync::atomic::Ordering::AcqRel);

    let workspace = std::path::Path::new(&config.workspace_path);
    Some(run_hygiene_scan(
        &db,
        &config,
        workspace,
        Some(&state.hygiene.budget),
        Some(&state.intel_queue),
        first_run,
        Some(state.embedding_model.as_ref()),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_overnight_window_returns_bool() {
        // Can't control the clock, but verify it doesn't panic
        let _result = is_overnight_window();
    }
}
