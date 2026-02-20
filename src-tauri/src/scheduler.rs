//! Scheduler for cron-based workflow execution
//!
//! Manages scheduled jobs with support for:
//! - Cron expression parsing
//! - Timezone-aware scheduling
//! - Sleep/wake detection via time-jump polling
//! - Missed job handling (runs if within grace period)

use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use cron::Schedule;
use tokio::sync::mpsc;

use crate::error::ExecutionError;
use crate::state::AppState;
use crate::types::{ExecutionTrigger, ScheduleEntry, WorkflowId};

/// Grace period for missed jobs (2 hours)
const MISSED_JOB_GRACE_PERIOD_SECS: i64 = 7200;

/// Extended grace period for weekly jobs (24 hours) — catches Monday morning sleep/wake gaps
const MISSED_WEEKLY_JOB_GRACE_PERIOD_SECS: i64 = 86400;

/// Time jump threshold to detect sleep/wake (5 minutes)
const TIME_JUMP_THRESHOLD_SECS: i64 = 300;

/// Poll interval for scheduler loop (1 minute)
const POLL_INTERVAL_SECS: u64 = 60;

/// Message sent to trigger workflow execution
#[derive(Debug, Clone)]
pub struct SchedulerMessage {
    pub workflow: WorkflowId,
    pub trigger: ExecutionTrigger,
}

/// Scheduler for managing workflow execution times
pub struct Scheduler {
    state: Arc<AppState>,
    sender: mpsc::Sender<SchedulerMessage>,
}

impl Scheduler {
    pub fn new(state: Arc<AppState>, sender: mpsc::Sender<SchedulerMessage>) -> Self {
        Self { state, sender }
    }

    /// Start the scheduler loop
    ///
    /// This runs indefinitely, checking for due jobs every minute.
    /// It also handles sleep/wake detection.
    pub async fn run(&self) {
        let mut last_check = Utc::now();
        let mut last_proposed_archive = Utc::now();
        let mut last_pre_meeting_refresh = Utc::now();

        loop {
            tokio::time::sleep(Duration::from_secs(POLL_INTERVAL_SECS)).await;

            let now = Utc::now();

            // Detect sleep: time jumped more than 5 minutes
            let time_jump = (now - last_check).num_seconds();
            if time_jump > TIME_JUMP_THRESHOLD_SECS {
                log::info!(
                    "Detected system wake (time jumped {} seconds), checking for missed jobs",
                    time_jump
                );
                self.check_missed_jobs(now).await;
            }

            // Check and run due jobs
            self.check_and_run_due_jobs(now).await;

            // I305: Drain prep invalidation queue and trigger re-generation
            self.drain_prep_invalidation_queue().await;

            // Auto-archive stale proposed actions daily (I256)
            if (now - last_proposed_archive).num_hours() >= 24 {
                self.auto_archive_proposed_actions();
                last_proposed_archive = now;
            }

            // Pre-meeting auto-refresh every 30 minutes (Phase 4A)
            if (now - last_pre_meeting_refresh).num_minutes() >= 30 {
                self.check_pre_meeting_refresh().await;
                self.run_post_meeting_email_correlation();
                last_pre_meeting_refresh = now;
            }

            last_check = now;
        }
    }

    /// I305: Drain the prep invalidation queue and trigger a briefing refresh.
    ///
    /// When a user corrects a meeting's entity, the prep is invalidated (data
    /// cleared, file deleted) but nothing regenerates it. This method drains
    /// the queue and triggers a `today` workflow so preps regenerate with the
    /// correct entity context.
    async fn drain_prep_invalidation_queue(&self) {
        let meeting_ids: Vec<String> = {
            match self.state.prep_invalidation_queue.lock() {
                Ok(mut queue) => {
                    if queue.is_empty() {
                        return;
                    }
                    queue.drain(..).collect()
                }
                Err(_) => return,
            }
        };

        log::info!(
            "Prep invalidation: draining {} meetings, triggering briefing refresh: {:?}",
            meeting_ids.len(),
            meeting_ids
        );

        // Trigger a today workflow to regenerate preps with corrected entities.
        // The pipeline will re-classify, re-gather context, and re-generate
        // prep for all today's meetings including the invalidated ones.
        self.trigger_workflow(
            WorkflowId::Today,
            ExecutionTrigger::Missed, // Reuse Missed trigger — semantically "catch-up"
        )
        .await;
    }

    /// Auto-archive proposed actions older than 7 days (I256).
    fn auto_archive_proposed_actions(&self) {
        match crate::db::ActionDb::open() {
            Ok(db) => match db.auto_archive_old_proposed(7) {
                Ok(count) if count > 0 => {
                    log::info!("Auto-archived {} stale proposed actions", count);
                }
                Ok(_) => {}
                Err(e) => {
                    log::warn!("Failed to auto-archive proposed actions: {}", e);
                }
            },
            Err(e) => {
                log::warn!("Failed to open DB for proposed action archival: {}", e);
            }
        }
    }

    /// Check for meetings starting in the next 2 hours that need intelligence refresh (Phase 4A).
    async fn check_pre_meeting_refresh(&self) {
        let meetings_to_refresh: Vec<String> = match crate::db::ActionDb::open() {
            Ok(db) => {
                let conn = db.conn_ref();
                let mut stmt = match conn.prepare(
                    "SELECT id FROM meetings_history
                     WHERE start_time > datetime('now')
                     AND start_time <= datetime('now', '+2 hours')
                     AND intelligence_state != 'archived'
                     AND (has_new_signals = 1
                          OR last_enriched_at IS NULL
                          OR last_enriched_at < datetime('now', '-12 hours'))"
                ) {
                    Ok(s) => s,
                    Err(e) => {
                        log::warn!("Pre-meeting refresh query failed: {}", e);
                        return;
                    }
                };
                stmt.query_map([], |row| row.get::<_, String>(0))
                    .map(|rows| rows.filter_map(|r| r.ok()).collect::<Vec<_>>())
                    .unwrap_or_default()
            }
            Err(e) => {
                log::warn!("Failed to open DB for pre-meeting refresh: {}", e);
                return;
            }
        };

        if meetings_to_refresh.is_empty() {
            return;
        }

        log::info!(
            "Pre-meeting refresh: {} meeting(s) need intelligence update",
            meetings_to_refresh.len()
        );

        for meeting_id in meetings_to_refresh {
            match crate::intelligence_lifecycle::generate_meeting_intelligence(
                &self.state, &meeting_id, false,
            )
            .await
            {
                Ok(quality) => {
                    log::info!(
                        "Pre-meeting refresh for {}: {:?}",
                        meeting_id,
                        quality.level
                    );
                }
                Err(e) => {
                    log::warn!("Pre-meeting refresh failed for {}: {}", meeting_id, e);
                }
            }
        }
    }

    /// Post-meeting email correlation (I308)
    fn run_post_meeting_email_correlation(&self) {
        match crate::db::ActionDb::open() {
            Ok(db) => {
                if let Err(e) = crate::signals::post_meeting::correlate_post_meeting_emails(&db) {
                    log::warn!("Post-meeting email correlation failed: {}", e);
                }
            }
            Err(e) => {
                log::warn!("Failed to open DB for post-meeting email correlation: {}", e);
            }
        }
    }

    /// Check for jobs that should run now
    async fn check_and_run_due_jobs(&self, now: DateTime<Utc>) {
        let config = match self.state.config.read() {
            Ok(guard) => guard.clone(),
            Err(_) => return,
        };

        let Some(config) = config else { return };

        // Check today workflow
        if config.schedules.today.enabled {
            if let Ok(true) = self.should_run_now(&config.schedules.today, WorkflowId::Today, now) {
                self.trigger_workflow(WorkflowId::Today, ExecutionTrigger::Scheduled)
                    .await;
            }
        }

        // Check archive workflow
        if config.schedules.archive.enabled {
            if let Ok(true) =
                self.should_run_now(&config.schedules.archive, WorkflowId::Archive, now)
            {
                self.trigger_workflow(WorkflowId::Archive, ExecutionTrigger::Scheduled)
                    .await;
            }
        }

        // Check inbox batch workflow
        if config.schedules.inbox_batch.enabled {
            if let Ok(true) =
                self.should_run_now(&config.schedules.inbox_batch, WorkflowId::InboxBatch, now)
            {
                self.trigger_workflow(WorkflowId::InboxBatch, ExecutionTrigger::Scheduled)
                    .await;
            }
        }

        // Check week workflow
        if config.schedules.week.enabled {
            if let Ok(true) = self.should_run_now(&config.schedules.week, WorkflowId::Week, now) {
                self.trigger_workflow(WorkflowId::Week, ExecutionTrigger::Scheduled)
                    .await;
            }
        }
    }

    /// Check if a workflow should run at the given time
    fn should_run_now(
        &self,
        entry: &ScheduleEntry,
        workflow: WorkflowId,
        now: DateTime<Utc>,
    ) -> Result<bool, ExecutionError> {
        let schedule = parse_cron(&entry.cron)?;
        let tz: Tz = entry.timezone.parse().map_err(|_| {
            ExecutionError::ConfigurationError(format!("Invalid timezone: {}", entry.timezone))
        })?;

        // Convert now to the configured timezone
        let now_local = now.with_timezone(&tz);

        // Get the last scheduled run time
        let last_run = self.state.get_last_scheduled_run(workflow);

        // Find the most recent scheduled time that's <= now
        let mut scheduled_times = schedule.after(&(now_local - chrono::Duration::minutes(2)));

        if let Some(next_time) = scheduled_times.next() {
            // Check if this minute matches
            let next_utc = next_time.with_timezone(&Utc);
            let diff = (now - next_utc).num_seconds().abs();

            // Within 2 minutes of scheduled time (I67: wider window for sleep/wake)
            if diff < 120 {
                // Check if we already ran this scheduled time
                if let Some(last) = last_run {
                    if (last - next_utc).num_seconds().abs() < 60 {
                        return Ok(false); // Already ran
                    }
                }
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Check for jobs that were missed during sleep
    async fn check_missed_jobs(&self, now: DateTime<Utc>) {
        let config = match self.state.config.read() {
            Ok(guard) => guard.clone(),
            Err(_) => return,
        };

        let Some(config) = config else { return };

        // Check today workflow
        if config.schedules.today.enabled {
            if let Ok(Some(_)) =
                self.find_missed_job(&config.schedules.today, WorkflowId::Today, now)
            {
                log::info!("Found missed 'today' job, running now");
                self.trigger_workflow(WorkflowId::Today, ExecutionTrigger::Missed)
                    .await;
            }
        }

        // Check archive workflow
        if config.schedules.archive.enabled {
            if let Ok(Some(_)) =
                self.find_missed_job(&config.schedules.archive, WorkflowId::Archive, now)
            {
                log::info!("Found missed 'archive' job, running now");
                self.trigger_workflow(WorkflowId::Archive, ExecutionTrigger::Missed)
                    .await;
            }
        }

        // Check inbox batch workflow
        if config.schedules.inbox_batch.enabled {
            if let Ok(Some(_)) =
                self.find_missed_job(&config.schedules.inbox_batch, WorkflowId::InboxBatch, now)
            {
                log::info!("Found missed 'inbox_batch' job, running now");
                self.trigger_workflow(WorkflowId::InboxBatch, ExecutionTrigger::Missed)
                    .await;
            }
        }

        // Check week workflow
        if config.schedules.week.enabled {
            if let Ok(Some(_)) = self.find_missed_job(&config.schedules.week, WorkflowId::Week, now)
            {
                log::info!("Found missed 'week' job, running now");
                self.trigger_workflow(WorkflowId::Week, ExecutionTrigger::Missed)
                    .await;
            }
        }
    }

    /// Find a missed job within the grace period.
    /// Weekly jobs use an extended 24-hour grace period to catch Monday sleep/wake gaps.
    fn find_missed_job(
        &self,
        entry: &ScheduleEntry,
        workflow: WorkflowId,
        now: DateTime<Utc>,
    ) -> Result<Option<DateTime<Utc>>, ExecutionError> {
        let schedule = parse_cron(&entry.cron)?;
        let tz: Tz = entry.timezone.parse().map_err(|_| {
            ExecutionError::ConfigurationError(format!("Invalid timezone: {}", entry.timezone))
        })?;

        let now_local = now.with_timezone(&tz);
        let grace_secs = match workflow {
            WorkflowId::Week => MISSED_WEEKLY_JOB_GRACE_PERIOD_SECS,
            _ => MISSED_JOB_GRACE_PERIOD_SECS,
        };
        let grace_period = chrono::Duration::seconds(grace_secs);
        let grace_start = now_local - grace_period;

        // Get last run time
        let last_run = self.state.get_last_scheduled_run(workflow);

        // Look for scheduled times in the grace period
        let iter = schedule.after(&grace_start);

        for scheduled in iter {
            let scheduled_utc = scheduled.with_timezone(&Utc);

            // Stop if we've passed now
            if scheduled_utc > now {
                break;
            }

            // Check if this was missed
            if let Some(last) = last_run {
                if last >= scheduled_utc {
                    continue; // Already ran
                }
            }

            // Found a missed job
            return Ok(Some(scheduled_utc));
        }

        Ok(None)
    }

    /// Trigger a workflow execution
    async fn trigger_workflow(&self, workflow: WorkflowId, trigger: ExecutionTrigger) {
        if self
            .sender
            .send(SchedulerMessage { workflow, trigger })
            .await
            .is_err()
        {
            log::error!("Failed to send scheduler message for {:?}", workflow);
        }
    }
}

/// Parse a cron expression
pub fn parse_cron(expr: &str) -> Result<Schedule, ExecutionError> {
    // The cron crate expects 6 fields (with seconds), but we use 5-field format
    // Add "0" for seconds at the start
    let full_expr = format!("0 {}", expr);

    full_expr.parse::<Schedule>().map_err(|e| {
        ExecutionError::ConfigurationError(format!("Invalid cron expression '{}': {}", expr, e))
    })
}

/// Get the next scheduled time for a workflow
pub fn get_next_run_time(entry: &ScheduleEntry) -> Result<DateTime<Utc>, ExecutionError> {
    let schedule = parse_cron(&entry.cron)?;
    let tz: Tz = entry.timezone.parse().map_err(|_| {
        ExecutionError::ConfigurationError(format!("Invalid timezone: {}", entry.timezone))
    })?;

    let _now_local = Utc::now().with_timezone(&tz);
    let next = schedule.upcoming(tz).next().ok_or_else(|| {
        ExecutionError::ConfigurationError("No upcoming scheduled time".to_string())
    })?;

    Ok(next.with_timezone(&Utc))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cron_weekdays_8am() {
        let result = parse_cron("0 8 * * 1-5");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_cron_midnight() {
        let result = parse_cron("0 0 * * *");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_cron_invalid() {
        let result = parse_cron("not a cron");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_next_run_time() {
        let entry = ScheduleEntry {
            enabled: true,
            cron: "0 8 * * 1-5".to_string(),
            timezone: "America/New_York".to_string(),
        };

        let result = get_next_run_time(&entry);
        assert!(result.is_ok());
    }
}
