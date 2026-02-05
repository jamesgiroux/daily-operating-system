//! Scheduler for cron-based workflow execution
//!
//! Manages scheduled jobs with support for:
//! - Cron expression parsing
//! - Timezone-aware scheduling
//! - Sleep/wake detection via time-jump polling
//! - Missed job handling (runs if within grace period)

use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, TimeZone, Utc};
use chrono_tz::Tz;
use cron::Schedule;
use tokio::sync::mpsc;

use crate::error::ExecutionError;
use crate::state::AppState;
use crate::types::{ExecutionTrigger, ScheduleEntry, WorkflowId};

/// Grace period for missed jobs (2 hours)
const MISSED_JOB_GRACE_PERIOD_SECS: i64 = 7200;

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

            last_check = now;
        }
    }

    /// Check for jobs that should run now
    async fn check_and_run_due_jobs(&self, now: DateTime<Utc>) {
        let config = match self.state.config.lock() {
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
    }

    /// Check if a workflow should run at the given time
    fn should_run_now(
        &self,
        entry: &ScheduleEntry,
        workflow: WorkflowId,
        now: DateTime<Utc>,
    ) -> Result<bool, ExecutionError> {
        let schedule = parse_cron(&entry.cron)?;
        let tz: Tz = entry
            .timezone
            .parse()
            .map_err(|_| ExecutionError::ConfigurationError(format!("Invalid timezone: {}", entry.timezone)))?;

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

            // Within 1 minute of scheduled time
            if diff < 60 {
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
        let config = match self.state.config.lock() {
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
    }

    /// Find a missed job within the grace period
    fn find_missed_job(
        &self,
        entry: &ScheduleEntry,
        workflow: WorkflowId,
        now: DateTime<Utc>,
    ) -> Result<Option<DateTime<Utc>>, ExecutionError> {
        let schedule = parse_cron(&entry.cron)?;
        let tz: Tz = entry
            .timezone
            .parse()
            .map_err(|_| ExecutionError::ConfigurationError(format!("Invalid timezone: {}", entry.timezone)))?;

        let now_local = now.with_timezone(&tz);
        let grace_period = chrono::Duration::seconds(MISSED_JOB_GRACE_PERIOD_SECS);
        let grace_start = now_local - grace_period;

        // Get last run time
        let last_run = self.state.get_last_scheduled_run(workflow);

        // Look for scheduled times in the grace period
        let mut iter = schedule.after(&grace_start);

        while let Some(scheduled) = iter.next() {
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

    full_expr
        .parse::<Schedule>()
        .map_err(|e| ExecutionError::ConfigurationError(format!("Invalid cron expression '{}': {}", expr, e)))
}

/// Get the next scheduled time for a workflow
pub fn get_next_run_time(entry: &ScheduleEntry) -> Result<DateTime<Utc>, ExecutionError> {
    let schedule = parse_cron(&entry.cron)?;
    let tz: Tz = entry
        .timezone
        .parse()
        .map_err(|_| ExecutionError::ConfigurationError(format!("Invalid timezone: {}", entry.timezone)))?;

    let now_local = Utc::now().with_timezone(&tz);
    let next = schedule
        .upcoming(tz)
        .next()
        .ok_or_else(|| ExecutionError::ConfigurationError("No upcoming scheduled time".to_string()))?;

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
