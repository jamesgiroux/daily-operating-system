//! Background meeting prep queue.
//!
//! Generates mechanical meeting briefings for future meetings using the
//! existing `gather_meeting_context()` pipeline — the same code `prepare_today`
//! uses. Produces `FullMeetingPrep`-compatible JSON from entity intelligence,
//! account dashboards, open actions, and meeting history without any AI call.
//!
//! Modeled on `intel_queue.rs`: priority queue, dedup, sequential processing,
//! split-lock DB access so the UI stays responsive.

use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;
use std::time::Instant;

use std::sync::Arc;

use serde_json::json;
use tauri::{AppHandle, Emitter};

use crate::state::AppState;

/// How often the background processor checks for work.
const POLL_INTERVAL_SECS: u64 = 5;

/// Debounce window — skip re-queueing the same meeting within this window.
const DEBOUNCE_SECS: u64 = 60;

/// Priority levels for meeting prep generation.
/// Higher numeric value = higher priority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PrepPriority {
    /// Pre-generation from weekly workflow — lowest priority.
    Background = 0,
    /// User opened Week page, meeting has no prep.
    PageLoad = 1,
    /// User clicked Refresh on meeting detail — highest priority.
    Manual = 2,
}

/// A request to generate prep for a meeting.
#[derive(Debug, Clone)]
pub struct PrepRequest {
    pub meeting_id: String,
    pub priority: PrepPriority,
    pub requested_at: Instant,
}

/// Thread-safe meeting prep queue with deduplication and debounce.
pub struct MeetingPrepQueue {
    queue: Mutex<VecDeque<PrepRequest>>,
    last_enqueued: Mutex<HashMap<String, Instant>>,
}

impl Default for MeetingPrepQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl MeetingPrepQueue {
    pub fn new() -> Self {
        Self {
            queue: Mutex::new(VecDeque::new()),
            last_enqueued: Mutex::new(HashMap::new()),
        }
    }

    /// Enqueue a prep request.
    ///
    /// Deduplicates by meeting_id: if already queued, higher priority wins.
    /// Debounces Background/PageLoad requests within `DEBOUNCE_SECS`.
    pub fn enqueue(&self, request: PrepRequest) {
        // Debounce low-priority triggers
        if request.priority == PrepPriority::Background || request.priority == PrepPriority::PageLoad
        {
            if let Ok(last) = self.last_enqueued.lock() {
                if let Some(last_time) = last.get(&request.meeting_id) {
                    if last_time.elapsed().as_secs() < DEBOUNCE_SECS {
                        log::debug!(
                            "MeetingPrepQueue: debounced {} ({}s since last)",
                            request.meeting_id,
                            last_time.elapsed().as_secs()
                        );
                        return;
                    }
                }
            }
        }

        let mut queue = match self.queue.lock() {
            Ok(q) => q,
            Err(_) => return,
        };

        // Dedup: if meeting already in queue, keep higher priority
        if let Some(existing) = queue
            .iter_mut()
            .find(|r| r.meeting_id == request.meeting_id)
        {
            if request.priority > existing.priority {
                existing.priority = request.priority;
                log::debug!(
                    "MeetingPrepQueue: upgraded priority for {} to {:?}",
                    request.meeting_id,
                    request.priority
                );
            }
            return;
        }

        log::info!(
            "MeetingPrepQueue: enqueued {} priority={:?}",
            request.meeting_id,
            request.priority
        );

        queue.push_back(request.clone());

        // Update debounce tracker
        if let Ok(mut last) = self.last_enqueued.lock() {
            last.insert(request.meeting_id, Instant::now());
        }
    }

    /// Dequeue the highest-priority request.
    pub fn dequeue(&self) -> Option<PrepRequest> {
        let mut queue = self.queue.lock().ok()?;
        if queue.is_empty() {
            return None;
        }

        let best_idx = queue
            .iter()
            .enumerate()
            .max_by_key(|(_, r)| r.priority)
            .map(|(i, _)| i)?;

        queue.remove(best_idx)
    }

    /// Current queue depth (for diagnostics).
    pub fn len(&self) -> usize {
        self.queue.lock().map(|q| q.len()).unwrap_or(0)
    }

    /// Whether the queue is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Remove stale entries from the debounce tracker.
    pub fn prune_stale_entries(&self) {
        let stale_threshold_secs = DEBOUNCE_SECS * 10;
        if let Ok(mut last) = self.last_enqueued.lock() {
            let before = last.len();
            last.retain(|_, instant| instant.elapsed().as_secs() < stale_threshold_secs);
            let pruned = before - last.len();
            if pruned > 0 {
                log::debug!(
                    "MeetingPrepQueue: pruned {} stale debounce entries",
                    pruned
                );
            }
        }
    }
}

/// Payload emitted when meeting prep is ready.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrepReadyPayload {
    pub meeting_id: String,
}

/// Background meeting prep processor.
///
/// Runs in a loop, checking the queue every `POLL_INTERVAL_SECS`.
/// When a request is found:
/// 1. Opens own DB connection (split-lock) to load meeting data
/// 2. Checks if fresh prep already exists
/// 3. Builds classified meeting JSON for `gather_meeting_context`
/// 4. Calls `gather_meeting_context` (mechanical — no AI)
/// 5. Converts context to `FullMeetingPrep` via `build_prep_json`
/// 6. Writes result to `prep_frozen_json` in DB
/// 7. Emits `prep-ready` event
fn sweep_meetings_needing_prep(state: &AppState) {
    let db_guard = match state.db.lock() {
        Ok(g) => g,
        Err(_) => {
            log::warn!("MeetingPrepSweep: DB lock poisoned");
            return;
        }
    };
    let db = match db_guard.as_ref() {
        Some(d) => d,
        None => return,
    };

    // Find future meetings that have at least one linked entity but no prep
    let sql = "SELECT DISTINCT mh.id
               FROM meetings_history mh
               INNER JOIN meeting_entities me ON mh.id = me.meeting_id
               WHERE mh.start_time > datetime('now')
                 AND mh.prep_frozen_json IS NULL
                 AND (mh.intelligence_state IS NULL OR mh.intelligence_state != 'archived')";

    let meeting_ids: Vec<String> = {
        let conn = db.conn_ref();
        let mut stmt = match conn.prepare(sql) {
            Ok(s) => s,
            Err(e) => {
                log::warn!("MeetingPrepSweep: query error: {}", e);
                return;
            }
        };
        stmt.query_map([], |row| row.get::<_, String>(0))
            .ok()
            .map(|rows| rows.filter_map(|r| r.ok()).collect())
            .unwrap_or_default()
    };

    if meeting_ids.is_empty() {
        log::info!("MeetingPrepSweep: all future meetings have prep");
        return;
    }

    log::info!(
        "MeetingPrepSweep: enqueuing {} meetings for mechanical prep",
        meeting_ids.len()
    );

    // Drop DB lock before enqueuing
    drop(db_guard);

    for mid in &meeting_ids {
        state.meeting_prep_queue.enqueue(PrepRequest {
            meeting_id: mid.clone(),
            priority: PrepPriority::Background,
            requested_at: Instant::now(),
        });
    }

    log::info!("MeetingPrepSweep: enqueued {} meetings", meeting_ids.len());
}

pub async fn run_meeting_prep_processor(state: Arc<AppState>, app: AppHandle) {
    log::info!("MeetingPrepProcessor: started");

    // Startup sweep: enqueue all future meetings that have linked entities but no prep.
    // This ensures every meeting with entity intelligence gets a mechanical briefing
    // before the user ever opens it. ADR-0086: meeting prep is a consumer of entity intel.
    sweep_meetings_needing_prep(&state);

    let mut polls_since_prune: u64 = 0;
    let prune_interval = 60 / POLL_INTERVAL_SECS;

    loop {
        tokio::time::sleep(std::time::Duration::from_secs(POLL_INTERVAL_SECS)).await;

        // Periodic pruning
        polls_since_prune += 1;
        if polls_since_prune >= prune_interval {
            state.meeting_prep_queue.prune_stale_entries();
            polls_since_prune = 0;
        }

        let request = match state.meeting_prep_queue.dequeue() {
            Some(r) => r,
            None => continue,
        };

        log::info!(
            "MeetingPrepProcessor: processing {} (priority={:?})",
            request.meeting_id,
            request.priority
        );

        // Run the blocking prep generation on a thread pool to avoid blocking
        // the tokio runtime (gather_meeting_context does synchronous file I/O).
        let state_clone = Arc::clone(&state);
        let meeting_id = request.meeting_id.clone();

        let result = tokio::task::spawn_blocking(move || {
            generate_mechanical_prep(&state_clone, &meeting_id)
        })
        .await;

        match result {
            Ok(Ok(())) => {
                let _ = app.emit(
                    "prep-ready",
                    PrepReadyPayload {
                        meeting_id: request.meeting_id.clone(),
                    },
                );
                log::info!(
                    "MeetingPrepProcessor: completed {}",
                    request.meeting_id
                );
            }
            Ok(Err(e)) => {
                log::warn!(
                    "MeetingPrepProcessor: failed for {}: {}",
                    request.meeting_id,
                    e
                );
            }
            Err(e) => {
                log::warn!(
                    "MeetingPrepProcessor: task panicked for {}: {}",
                    request.meeting_id,
                    e
                );
            }
        }
    }
}

/// Generate mechanical prep for a single meeting.
///
/// Uses own DB connection (split-lock pattern) to avoid blocking UI.
fn generate_mechanical_prep(state: &AppState, meeting_id: &str) -> Result<(), String> {
    // Phase 1: Load meeting from DB (own connection)
    let db = crate::db::ActionDb::open().map_err(|e| format!("Failed to open DB: {}", e))?;

    let meeting = db
        .get_meeting_by_id(meeting_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Meeting not found: {}", meeting_id))?;

    // Phase 2: Check if prep already exists and is fresh
    if meeting.prep_frozen_json.is_some() {
        log::debug!(
            "MeetingPrepQueue: {} already has prep_frozen_json, skipping",
            meeting_id
        );
        return Ok(());
    }

    // Resolve workspace path for context gathering
    let workspace = {
        let config_guard = state.config.read().map_err(|_| "Config lock poisoned")?;
        let config = config_guard.as_ref().ok_or("No config")?;
        std::path::PathBuf::from(&config.workspace_path)
    };

    // Phase 3: Build classified meeting JSON for gather_meeting_context
    let classified = json!({
        "id": meeting.id,
        "title": meeting.title,
        "type": meeting.meeting_type,
        "start": meeting.start_time,
        "description": meeting.description.as_deref().unwrap_or(""),
    });

    // Phase 4: Gather context (mechanical — no AI)
    let embedding_model = if state.embedding_model.is_ready() {
        Some(state.embedding_model.as_ref())
    } else {
        None
    };

    let ctx = crate::prepare::meeting_context::gather_meeting_context_single(
        &classified,
        &workspace,
        Some(&db),
        embedding_model,
    );

    // Phase 5: Build FullMeetingPrep JSON via deliver.rs
    let directive_ctx: crate::json_loader::DirectiveMeetingContext =
        serde_json::from_value(ctx).unwrap_or_default();

    let directive_meeting = crate::json_loader::DirectiveMeeting {
        id: Some(meeting.id.clone()),
        event_id: meeting.calendar_event_id.clone(),
        summary: Some(meeting.title.clone()),
        title: Some(meeting.title.clone()),
        start: Some(meeting.start_time.clone()),
        end: meeting.end_time.clone(),
        account: directive_ctx.account.clone(),
        start_display: None,
        end_display: None,
        meeting_type: Some(meeting.meeting_type.clone()),
        entities: Vec::new(),
    };

    let mut prep_json = crate::workflow::deliver::build_prep_json_public(
        &directive_meeting,
        &meeting.meeting_type,
        meeting_id,
        Some(&directive_ctx),
    );

    // Ensure required fields for FullMeetingPrep deserialization.
    // build_prep_json produces disk-oriented JSON that lacks filePath and may lack timeRange.
    if let Some(obj) = prep_json.as_object_mut() {
        if !obj.contains_key("filePath") {
            obj.insert(
                "filePath".to_string(),
                json!(format!("prep_frozen:{}", meeting_id)),
            );
        }
        if !obj.contains_key("timeRange") {
            obj.insert("timeRange".to_string(), json!(""));
        }
    }

    // Phase 6: Write result to prep_frozen_json in DB.
    // Deliberately does NOT set prep_frozen_at — that column is owned by the AI
    // workflow (reconcile.rs freeze_meeting_prep_snapshot) and gates on IS NULL.
    // Setting it here would prevent the workflow from ever writing real AI content.
    let frozen_str =
        serde_json::to_string(&prep_json).map_err(|e| format!("Serialize error: {}", e))?;

    db.conn_ref()
        .execute(
            "UPDATE meetings_history SET prep_frozen_json = ?1 WHERE id = ?2",
            rusqlite::params![frozen_str, meeting_id],
        )
        .map_err(|e| format!("Failed to write prep: {}", e))?;

    log::info!(
        "MeetingPrepQueue: wrote prep_frozen_json for {} ({} bytes)",
        meeting_id,
        frozen_str.len()
    );

    Ok(())
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prep_queue_enqueue_dequeue() {
        let queue = MeetingPrepQueue::new();

        queue.enqueue(PrepRequest {
            meeting_id: "mtg-1".to_string(),
            priority: PrepPriority::PageLoad,
            requested_at: Instant::now(),
        });

        assert_eq!(queue.len(), 1);

        let req = queue.dequeue().unwrap();
        assert_eq!(req.meeting_id, "mtg-1");
        assert_eq!(req.priority, PrepPriority::PageLoad);
        assert!(queue.is_empty());
    }

    #[test]
    fn test_prep_queue_dedup_keeps_higher_priority() {
        let queue = MeetingPrepQueue::new();

        queue.enqueue(PrepRequest {
            meeting_id: "mtg-1".to_string(),
            priority: PrepPriority::PageLoad,
            requested_at: Instant::now(),
        });

        // Same meeting, higher priority → should upgrade
        queue.enqueue(PrepRequest {
            meeting_id: "mtg-1".to_string(),
            priority: PrepPriority::Manual,
            requested_at: Instant::now(),
        });

        assert_eq!(queue.len(), 1);
        let req = queue.dequeue().unwrap();
        assert_eq!(req.priority, PrepPriority::Manual);
    }

    #[test]
    fn test_prep_queue_priority_ordering() {
        let queue = MeetingPrepQueue::new();

        queue.enqueue(PrepRequest {
            meeting_id: "alpha".to_string(),
            priority: PrepPriority::Background,
            requested_at: Instant::now(),
        });

        queue.enqueue(PrepRequest {
            meeting_id: "beta".to_string(),
            priority: PrepPriority::Manual,
            requested_at: Instant::now(),
        });

        queue.enqueue(PrepRequest {
            meeting_id: "gamma".to_string(),
            priority: PrepPriority::PageLoad,
            requested_at: Instant::now(),
        });

        // Should dequeue in priority order: Manual > PageLoad > Background
        let first = queue.dequeue().unwrap();
        assert_eq!(first.meeting_id, "beta");

        let second = queue.dequeue().unwrap();
        assert_eq!(second.meeting_id, "gamma");

        let third = queue.dequeue().unwrap();
        assert_eq!(third.meeting_id, "alpha");
    }

    #[test]
    fn test_prep_queue_manual_bypasses_debounce() {
        let queue = MeetingPrepQueue::new();

        // First: page load
        queue.enqueue(PrepRequest {
            meeting_id: "mtg-1".to_string(),
            priority: PrepPriority::PageLoad,
            requested_at: Instant::now(),
        });

        // Dequeue it
        let _ = queue.dequeue();

        // Manual request should bypass debounce
        queue.enqueue(PrepRequest {
            meeting_id: "mtg-1".to_string(),
            priority: PrepPriority::Manual,
            requested_at: Instant::now(),
        });

        assert_eq!(queue.len(), 1);
    }

    #[test]
    fn test_prep_queue_dequeue_empty() {
        let queue = MeetingPrepQueue::new();
        assert!(queue.dequeue().is_none());
    }

    #[test]
    fn test_prep_queue_prune_stale_entries() {
        let queue = MeetingPrepQueue::new();

        {
            let mut last = queue.last_enqueued.lock().unwrap();
            last.insert("fresh".to_string(), Instant::now());
        }

        queue.prune_stale_entries();

        let last = queue.last_enqueued.lock().unwrap();
        assert!(
            last.contains_key("fresh"),
            "fresh entry should survive pruning"
        );
    }
}
