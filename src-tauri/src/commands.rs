use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use chrono::TimeZone;
use regex::Regex;
use tauri::{Emitter, Manager, State};

use crate::executor::request_workflow_execution;
use crate::hygiene::{build_intelligence_hygiene_status, HygieneStatusView};
use crate::json_loader::load_emails_json;
use crate::parser::list_inbox_files;
use crate::scheduler::get_next_run_time as scheduler_get_next_run_time;
use crate::state::{reload_config, AppState};
use crate::types::{
    CalendarEvent, CapturedOutcome, Config, EmailBriefingData, ExecutionRecord, FullMeetingPrep,
    GoogleAuthStatus, InboxFile, LiveProactiveSuggestion, MeetingIntelligence,
    PostMeetingCaptureConfig, SourceReference, WorkflowId, WorkflowStatus,
};
use crate::SchedulerSender;

pub(crate) static DEV_CLAUDE_OVERRIDE: AtomicU8 = AtomicU8::new(0);
pub(crate) static DEV_GOOGLE_OVERRIDE: AtomicU8 = AtomicU8::new(0);

pub use crate::services::actions::ActionsResult;
pub use crate::services::dashboard::{DashboardResult, WeekResult};

mod accounts_content_chat;
mod actions_calendar;
mod app_support;
mod core;
mod integrations;
mod people_entities;
mod planning_reports;
mod projects_data;
mod success_plans;
mod workspace;

pub use accounts_content_chat::*;
pub use actions_calendar::*;
pub use app_support::*;
pub use core::*;
pub use integrations::*;
pub use people_entities::*;
pub use planning_reports::*;
pub use projects_data::*;
pub use success_plans::*;
pub use workspace::*;

pub(crate) use app_support::{
    __cmd__clear_intelligence, __cmd__delete_all_data, __cmd__export_all_data,
    __cmd__get_data_summary, __cmd__get_sync_freshness, __cmd__rebuild_search_index,
    __cmd__search_global,
};
#[allow(unused_imports)]
pub(crate) use core::{
    backfill_db_prep_contexts, backfill_prep_files_in_dir, backfill_prep_semantics_value,
    collect_meeting_outcomes_from_db, load_meeting_prep_from_sources, parse_meeting_datetime,
    parse_user_agenda_json,
};

const READ_CMD_LATENCY_BUDGET_MS: u128 = 100;
const CLAUDE_STATUS_CACHE_TTL_SECS: u64 = 300;

fn log_command_latency(command: &str, started: std::time::Instant, budget_ms: u128) {
    let elapsed_ms = started.elapsed().as_millis();
    crate::latency::record_latency(command, elapsed_ms, budget_ms);
    if elapsed_ms > budget_ms {
        log::warn!(
            "{} exceeded latency budget: {}ms > {}ms",
            command,
            elapsed_ms,
            budget_ms
        );
    } else {
        log::debug!("{} completed in {}ms", command, elapsed_ms);
    }
}
