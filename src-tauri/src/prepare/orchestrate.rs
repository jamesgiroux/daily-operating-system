//! Thin orchestrators that compose Phase 1 ops and write directive JSON.
//!
//! Ported from prepare_today.py, prepare_week.py, refresh_emails.py, deliver_week.py.
//! Each orchestrator is async (for Google API calls) and writes a directive JSON
//! that the Rust delivery pipeline (deliver.rs) consumes.

use std::collections::HashSet;
use std::path::Path;

use chrono::{Datelike, NaiveDate, Utc};
use serde_json::{json, Value};

use crate::error::ExecutionError;
use crate::google_api;
use crate::state::AppState;

use super::actions;
use super::constants::DAY_NAMES;
use super::email_classify;
use super::gaps;
use super::meeting_context;

// ============================================================================
// prepare_today
// ============================================================================

/// Replaces prepare_today.py. Writes: _today/data/today-directive.json
pub async fn prepare_today(state: &AppState, workspace: &Path) -> Result<(), ExecutionError> {
    let now = Utc::now();
    // Use local date — not UTC — to determine "today" from the user's perspective.
    // Without this, a Sunday 8pm EST user gets Monday's briefing (UTC is already Monday).
    let today = chrono::Local::now().date_naive();

    // Load config
    let (profile, user_domains, user_focus) = get_config(state);
    let primary_user_domain = user_domains.first().cloned().unwrap_or_default();

    log::info!(
        "prepare_today: workspace={}, profile={}, domain={}",
        workspace.display(),
        profile,
        if user_domains.is_empty() {
            "(unknown)"
        } else {
            &primary_user_domain
        }
    );

    // Step 1: Context metadata
    let (iso_year, iso_week, _) = today.iso_week_fields();
    let mut context = json!({
        "date": today.to_string(),
        "day_of_week": today.format("%A").to_string(),
        "week_number": iso_week,
        "year": iso_year,
        "profile": profile,
    });
    if let Some(ref focus) = user_focus {
        context
            .as_object_mut()
            .unwrap()
            .insert("focus".to_string(), json!(focus));
    }

    // Step 2: Fetch calendar events + classify (I336: entity-generic)
    let entity_hints = {
        let db_guard = state.db.lock().ok();
        let db_ref = db_guard.as_ref().and_then(|g| g.as_ref());
        match db_ref {
            Some(db) => crate::helpers::build_entity_hints(db),
            None => {
                // Fallback: build from filesystem (legacy)
                let fs_hints = build_account_domain_hints(workspace);
                fs_hints.into_iter().map(|slug| crate::google_api::classify::EntityHint {
                    id: slug.clone(),
                    entity_type: crate::entity::EntityType::Account,
                    name: slug.clone(),
                    slugs: vec![slug],
                    domains: vec![],
                    keywords: vec![],
                    emails: vec![],
                }).collect()
            }
        }
    };
    let account_hints = crate::helpers::account_hints_from_entity_hints(&entity_hints);

    let (classified, events, meetings_by_type, time_status) =
        fetch_and_classify_today(today, &user_domains, &entity_hints).await;

    log::info!("prepare_today: {} events fetched", events.len());

    // Step 3: Calendar gaps
    let gap_list = gaps::compute_gaps(&events, today, None);
    let total_gap_minutes: i64 = gap_list
        .iter()
        .filter_map(|g| g.get("duration_minutes").and_then(|v| v.as_i64()))
        .sum();
    log::info!(
        "prepare_today: {} gaps, {} min focus time",
        gap_list.len(),
        total_gap_minutes
    );

    // Step 4: Fetch and classify emails
    let customer_domains = extract_customer_domains(&meetings_by_type);
    let preset_email_keywords: Vec<String> = state
        .active_preset
        .read()
        .ok()
        .and_then(|guard| guard.as_ref().map(|p| p.email_priority_keywords.clone()))
        .unwrap_or_default();
    let mut email_result =
        fetch_and_classify_emails(&primary_user_domain, &customer_domains, &account_hints, &preset_email_keywords).await;
    if let Some(ref sync_error) = email_result.sync_error {
        log::warn!(
            "prepare_today: email sync degraded [{}] {}",
            sync_error.code,
            sync_error.message
        );
    }
    log::info!(
        "prepare_today: {} emails ({} high, {} medium, {} low)",
        email_result.all.len(),
        email_result.high.len(),
        email_result.medium_count,
        email_result.low_count,
    );

    // Step 4a2: Signal-context boosting (I320 — boost medium emails with entity signals)
    {
        let boost_guard = state.db.lock().ok();
        if let Some(db) = boost_guard.as_ref().and_then(|g| g.as_ref()) {
            let mut boosted_count = 0u32;
            let mut new_high = Vec::new();
            let mut new_all = Vec::new();

            for email_val in &email_result.all {
                let priority = email_val.get("priority").and_then(|v| v.as_str()).unwrap_or("medium");
                let from_email = email_val.get("from_email").and_then(|v| v.as_str()).unwrap_or("");

                if let Some(boost) = email_classify::boost_with_entity_context(from_email, priority, db) {
                    // Clone and update priority
                    let mut boosted = email_val.clone();
                    if let Some(obj) = boosted.as_object_mut() {
                        obj.insert("priority".to_string(), json!("high"));
                        obj.insert("boost_reason".to_string(), json!(boost.reason));
                    }
                    new_high.push(boosted.clone());
                    new_all.push(boosted);
                    boosted_count += 1;
                } else {
                    new_all.push(email_val.clone());
                    if priority == "high" {
                        new_high.push(email_val.clone());
                    }
                }
            }

            if boosted_count > 0 {
                email_result.all = new_all;
                email_result.high = new_high;
                email_result.medium_count = email_result.medium_count.saturating_sub(boosted_count as u64);
                log::info!("prepare_today: boosted {} medium emails to high via entity signals", boosted_count);
            }
        }
    }

    // Step 4a3: Email commitment extraction (I321 — extract actions from high-priority email bodies)
    {
        let body_access_enabled = {
            let config_guard = state.config.read().ok();
            config_guard
                .as_ref()
                .and_then(|g| g.as_ref())
                .map(|c| crate::types::is_feature_enabled(c, "emailBodyAccess"))
                .unwrap_or(false)
        };

        if body_access_enabled {
            let access_token = google_api::get_valid_access_token().await.ok();
            if let Some(token) = access_token {
                // Phase 1: Fetch bodies (async, no db lock held)
                let mut fetched_bodies: Vec<(String, String, String, String)> = Vec::new();
                for email_val in &email_result.high {
                    let email_id =
                        email_val.get("id").and_then(|v| v.as_str()).unwrap_or("");
                    let subject =
                        email_val.get("subject").and_then(|v| v.as_str()).unwrap_or("");
                    let from_email = email_val
                        .get("from_email")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");

                    if email_id.is_empty() {
                        continue;
                    }

                    match google_api::gmail::fetch_message_body(&token, email_id).await {
                        Ok(Some(body)) => {
                            fetched_bodies.push((
                                email_id.to_string(),
                                subject.to_string(),
                                from_email.to_string(),
                                body,
                            ));
                        }
                        Ok(None) => {
                            log::debug!("prepare_today: no body for email {}", email_id);
                        }
                        Err(e) => {
                            log::debug!(
                                "prepare_today: failed to fetch body for {}: {}",
                                email_id,
                                e
                            );
                        }
                    }
                }

                // Phase 2: Extract commitments (sync, with db lock)
                if !fetched_bodies.is_empty() {
                    let commitment_guard = state.db.lock().ok();
                    if let Some(db) = commitment_guard.as_ref().and_then(|g| g.as_ref()) {
                        let mut total_commitments = 0usize;
                        for (email_id, subject, from_email, body) in &fetched_bodies {
                            let commitments =
                                crate::processor::email_actions::extract_email_commitments(
                                    body, email_id, subject, from_email, db,
                                );
                            total_commitments += commitments.len();
                        }
                        if total_commitments > 0 {
                            log::info!(
                                "prepare_today: extracted {} commitments from {} high-priority emails",
                                total_commitments,
                                email_result.high.len()
                            );
                        }
                    }
                }
            }
        }
    }

    // Step 4b: Email-meeting bridge (I306 — correlate email signals with upcoming meetings)
    {
        let bridge_guard = state.db.lock().ok();
        if let Some(db) = bridge_guard.as_ref().and_then(|g| g.as_ref()) {
            match crate::signals::email_bridge::run_email_meeting_bridge(db) {
                Ok(correlations) if !correlations.is_empty() => {
                    log::info!("prepare_today: {} email-meeting correlations", correlations.len());
                }
                Err(e) => {
                    log::warn!("prepare_today: email-meeting bridge failed: {}", e);
                }
                _ => {}
            }
        }
    }

    // Step 4b2: Email cadence monitoring (I319 — anomaly detection)
    let cadence_anomalies = {
        let cadence_guard = state.db.lock().ok();
        if let Some(db) = cadence_guard.as_ref().and_then(|g| g.as_ref()) {
            let anomalies = crate::signals::cadence::compute_and_emit_cadence_anomalies(db);
            if !anomalies.is_empty() {
                log::info!("prepare_today: {} cadence anomalies detected", anomalies.len());
            }
            anomalies
        } else {
            Vec::new()
        }
    };

    // Step 4b3: Auto-archive low-priority emails (I323)
    let mut archived_count = 0u64;
    {
        let auto_archive_enabled = {
            let config_guard = state.config.read().ok();
            config_guard
                .as_ref()
                .and_then(|g| g.as_ref())
                .map(|c| crate::types::is_feature_enabled(c, "autoArchiveEnabled"))
                .unwrap_or(false)
        };

        if auto_archive_enabled && !email_result.low.is_empty() {
            match google_api::get_valid_access_token().await {
                Ok(token) => {
                    let low_ids: Vec<String> = email_result
                        .low
                        .iter()
                        .filter_map(|e| e.get("id").and_then(|v| v.as_str()).map(String::from))
                        .collect();

                    if !low_ids.is_empty() {
                        match google_api::gmail::archive_emails(&token, &low_ids).await {
                            Ok(count) => {
                                log::info!(
                                    "prepare_today: auto-archived {} low-priority emails",
                                    count
                                );
                                archived_count = count as u64;

                                // Write disposition manifest
                                let manifest: Vec<Value> = email_result
                                    .low
                                    .iter()
                                    .map(|e| {
                                        json!({
                                            "id": e.get("id"),
                                            "from": e.get("from"),
                                            "subject": e.get("subject"),
                                            "reason": "low_priority_auto_archive",
                                            "archived_at": chrono::Utc::now().to_rfc3339(),
                                        })
                                    })
                                    .collect();

                                let manifest_path = workspace
                                    .join("_today")
                                    .join("data")
                                    .join("email-disposition.json");
                                if let Ok(manifest_json) = serde_json::to_string_pretty(&manifest)
                                {
                                    if let Err(e) =
                                        crate::util::atomic_write_str(&manifest_path, &manifest_json)
                                    {
                                        log::warn!(
                                            "prepare_today: failed to write disposition manifest: {}",
                                            e
                                        );
                                    }
                                }
                            }
                            Err(e) => {
                                log::warn!("prepare_today: auto-archive failed: {}", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    log::warn!("prepare_today: auto-archive skipped (auth failed): {}", e);
                }
            }
        }
    }

    // Step 4c: Thread position tracking (I318 — "ball in your court")
    // Only track high-priority email threads for "ball in your court" detection
    let replies_needed = {
        let thread_guard = state.db.lock().ok();
        if let Some(db) = thread_guard.as_ref().and_then(|g| g.as_ref()) {
            let high_priority_emails: Vec<Value> = email_result.all
                .iter()
                .filter(|e| e.get("priority").and_then(|v| v.as_str()) == Some("high"))
                .cloned()
                .collect();
            let tracked = track_thread_positions(
                &high_priority_emails,
                &primary_user_domain,
                db,
            );
            log::info!("prepare_today: tracked {} high-priority threads, {} awaiting reply", tracked.0, tracked.1);
            // Fetch threads awaiting reply for the directive
            let now = Utc::now();
            db.get_threads_awaiting_reply()
                .unwrap_or_default()
                .iter()
                .map(|(tid, subject, sender, date)| {
                    let wait_duration = compute_wait_duration(date, &now);
                    json!({
                        "thread_id": tid,
                        "subject": subject,
                        "from": sender,
                        "date": date,
                        "waitDuration": wait_duration,
                    })
                })
                .collect::<Vec<Value>>()
        } else {
            Vec::new()
        }
    };

    // Step 5: Collect actions (workspace markdown + SQLite)
    let db_guard = state.db.lock().ok();
    let db_ref = db_guard.as_ref().and_then(|g| g.as_ref());
    let action_result = actions::collect_all_actions(workspace, db_ref);
    let actions_dict = action_result.to_value();

    // Step 6: Meeting contexts (I305: thread embedding model for entity resolution)
    let embedding_ref = state.embedding_model.as_ref();
    let meeting_contexts =
        meeting_context::gather_all_meeting_contexts(&classified, workspace, db_ref, Some(embedding_ref));
    // Drop DB guard before any further awaits
    drop(db_guard);

    // Step 6a: Run proactive detection scan (I260)
    {
        let scan_guard = state.db.lock().ok();
        if let Some(db) = scan_guard.as_ref().and_then(|g| g.as_ref()) {
            let (profile, user_domains, _) = get_config(state);
            let scan_ctx = crate::proactive::engine::DetectorContext {
                today,
                user_domains,
                profile,
            };
            let engine = crate::proactive::engine::default_engine();
            match engine.run_scan(db, &scan_ctx) {
                Ok(n) if n > 0 => log::info!("prepare_today: {} proactive insights detected", n),
                Err(e) => log::warn!("prepare_today: proactive scan failed: {}", e),
                _ => {}
            }
        }
    }

    // Step 6b: Signal-driven briefing callouts (I308)
    let callouts = {
        let callout_guard = state.db.lock().ok();
        let callout_db = callout_guard.as_ref().and_then(|g| g.as_ref());
        match callout_db {
            Some(db) => {
                let model_ref = state.embedding_model.as_ref();
                crate::signals::callouts::generate_callouts(db, Some(model_ref), &classified)
            }
            None => Vec::new(),
        }
    };

    // Step 6c: Person intelligence enrichment triggers (I338)
    {
        let person_guard = state.db.lock().ok();
        if let Some(db) = person_guard.as_ref().and_then(|g| g.as_ref()) {
            let n = queue_person_intelligence(&meeting_contexts, workspace, db, &state.intel_queue);
            if n > 0 {
                log::info!("prepare_today: queued {} person intelligence enrichments", n);
            }
        }
    }

    // Step 7: File inventory
    let existing_today = inventory_today_files(workspace);
    let inbox_pending = count_inbox_pending(workspace);

    // Step 8: Generate AI tasks
    let ai_tasks = generate_ai_tasks(&classified, &time_status, &email_result.high);

    // Step 7b: Synthesize email briefing narrative (I322)
    let email_narrative = synthesize_email_narrative(
        &replies_needed,
        &email_result.high,
        &meeting_contexts,
        &cadence_anomalies,
    );

    // Build lean events (strip attendees)
    let lean_events: Vec<Value> = events
        .iter()
        .map(|ev| {
            json!({
                "id": ev.get("id"),
                "summary": ev.get("summary").or_else(|| ev.get("title")),
                "start": ev.get("start"),
                "end": ev.get("end"),
            })
        })
        .collect();

    // Strip attendees from meetings_by_type
    let lean_meetings = lean_meetings_by_type(&meetings_by_type);

    let directive = json!({
        "command": "today",
        "generated_at": now.to_rfc3339(),
        "context": context,
        "calendar": {
            "events": lean_events,
            "past": time_status.get("past").cloned().unwrap_or_default(),
            "in_progress": time_status.get("in_progress").cloned().unwrap_or_default(),
            "upcoming": time_status.get("upcoming").cloned().unwrap_or_default(),
            "gaps": gap_list,
        },
        "meetings": lean_meetings,
        "meeting_contexts": meeting_contexts,
        "actions": actions_dict,
        "emails": {
            "high_priority": email_result.high,
            "classified": email_result.all,
            "medium_count": email_result.medium_count,
            "low_count": email_result.low_count,
            "sync_error": email_result.sync_error.as_ref().map(EmailSyncFailure::to_json),
            "repliesNeeded": replies_needed,
            "narrative": email_narrative,
            "disposition": if archived_count > 0 {
                json!({
                    "archived_count": archived_count,
                    "manifest_path": "data/email-disposition.json",
                })
            } else {
                json!(null)
            },
        },
        "files": {
            "existing_today": existing_today,
            "inbox_pending": inbox_pending,
        },
        "ai_tasks": ai_tasks,
        "callouts": callouts,
    });

    // Write output
    let output_path = workspace
        .join("_today")
        .join("data")
        .join("today-directive.json");
    write_directive(&output_path, &directive)?;

    log::info!(
        "prepare_today: directive written to {}",
        output_path.display()
    );
    Ok(())
}

// ============================================================================
// prepare_week
// ============================================================================

/// Replaces prepare_week.py. Writes: _today/data/week-directive.json
pub async fn prepare_week(state: &AppState, workspace: &Path) -> Result<(), ExecutionError> {
    let now = Utc::now();
    let today = chrono::Local::now().date_naive();

    let (profile, user_domains, _user_focus) = get_config(state);

    // Week bounds — on weekends, anchor to NEXT Monday (forecast the upcoming week)
    let monday = if today.weekday() == chrono::Weekday::Sat {
        today + chrono::Duration::days(2)
    } else if today.weekday() == chrono::Weekday::Sun {
        today + chrono::Duration::days(1)
    } else {
        today - chrono::Duration::days(today.weekday().num_days_from_monday() as i64)
    };
    let friday = monday + chrono::Duration::days(4);
    let (iso_year, iso_week, _) = monday.iso_week_fields();
    let week_label = format!("W{:02}", iso_week);
    let date_range = format_date_range(monday, friday);

    log::info!("prepare_week: {} ({})", week_label, date_range);

    let context = json!({
        "weekNumber": week_label,
        "year": iso_year,
        "monday": monday.to_string(),
        "friday": friday.to_string(),
        "dateRange": date_range,
        "profile": profile,
    });

    // Fetch and classify calendar events for the week (I336: entity-generic)
    let entity_hints = {
        let db_guard = state.db.lock().ok();
        let db_ref = db_guard.as_ref().and_then(|g| g.as_ref());
        match db_ref {
            Some(db) => crate::helpers::build_entity_hints(db),
            None => Vec::new(),
        }
    };
    let (classified, _events, _meetings_by_type, _time_status, events_by_day) =
        fetch_and_classify_week(monday, friday, &user_domains, &entity_hints).await;

    // Actions from SQLite
    let db_guard = state.db.lock().ok();
    let db_ref = db_guard.as_ref().and_then(|g| g.as_ref());
    let actions_data = match db_ref {
        Some(db) => actions::fetch_actions_from_db(db),
        None => json!({"overdue": [], "thisWeek": []}),
    };

    // Meeting contexts (I305: thread embedding model for entity resolution)
    let embedding_ref_week = state.embedding_model.as_ref();
    let meeting_contexts =
        meeting_context::gather_all_meeting_contexts(&classified, workspace, db_ref, Some(embedding_ref_week));
    drop(db_guard);

    // Gap analysis — resolve user timezone from schedule config for accurate UTC→local conversion
    let user_tz: Option<chrono_tz::Tz> = state
        .config
        .read()
        .ok()
        .and_then(|g| g.as_ref().and_then(|c| c.schedules.today.timezone.parse().ok()));
    let gaps_by_day = gaps::compute_all_gaps(&events_by_day, monday, user_tz);
    let suggestions = gaps::suggest_focus_blocks(&gaps_by_day);

    // Build lean events by day (strip attendees)
    let mut serializable_by_day = serde_json::Map::new();
    if let Some(obj) = events_by_day.as_object() {
        for (day_name, day_events) in obj {
            let lean: Vec<Value> = day_events
                .as_array()
                .unwrap_or(&Vec::new())
                .iter()
                .map(|ev| {
                    json!({
                        "id": ev.get("id"),
                        "title": ev.get("title").or_else(|| ev.get("summary")),
                        "start": ev.get("start"),
                        "end": ev.get("end"),
                        "type": ev.get("type"),
                        "external_domains": ev.get("external_domains"),
                    })
                })
                .collect();
            serializable_by_day.insert(day_name.clone(), json!(lean));
        }
    }

    // Proactive scan + callouts for week view (I260)
    let week_callouts = {
        let callout_guard = state.db.lock().ok();
        let callout_db = callout_guard.as_ref().and_then(|g| g.as_ref());
        match callout_db {
            Some(db) => {
                // Run proactive scan first
                let scan_ctx = crate::proactive::engine::DetectorContext {
                    today,
                    user_domains: user_domains.clone(),
                    profile: profile.clone(),
                };
                let engine = crate::proactive::engine::default_engine();
                let _ = engine.run_scan(db, &scan_ctx);

                // Generate callouts
                let model_ref = state.embedding_model.as_ref();
                crate::signals::callouts::generate_callouts(db, Some(model_ref), &classified)
            }
            None => Vec::new(),
        }
    };

    // Person intelligence enrichment triggers (I338)
    {
        let person_guard = state.db.lock().ok();
        if let Some(db) = person_guard.as_ref().and_then(|g| g.as_ref()) {
            let n = queue_person_intelligence(&meeting_contexts, workspace, db, &state.intel_queue);
            if n > 0 {
                log::info!("prepare_week: queued {} person intelligence enrichments", n);
            }
        }
    }

    let directive = json!({
        "command": "week",
        "generatedAt": now.to_rfc3339(),
        "context": context,
        "meetingsByDay": Value::Object(serializable_by_day),
        "meetingContexts": meeting_contexts,
        "actions": actions_data,
        "callouts": week_callouts,
        "timeBlocks": {
            "gapsByDay": gaps_by_day,
            "suggestions": suggestions,
        },
    });

    let output_path = workspace
        .join("_today")
        .join("data")
        .join("week-directive.json");
    write_directive(&output_path, &directive)?;

    log::info!(
        "prepare_week: directive written to {}",
        output_path.display()
    );
    Ok(())
}

// ============================================================================
// refresh_emails
// ============================================================================

/// Replaces refresh_emails.py. Writes: _today/data/email-refresh-directive.json
pub async fn refresh_emails(state: &AppState, workspace: &Path) -> Result<(), ExecutionError> {
    let (_profile, user_domains, _user_focus) = get_config(state);
    let primary_user_domain = user_domains.first().cloned().unwrap_or_default();
    let account_hints = {
        let db_guard = state.db.lock().ok();
        let db_ref = db_guard.as_ref().and_then(|g| g.as_ref());
        match db_ref {
            Some(db) => crate::helpers::build_external_account_hints(db),
            None => build_account_domain_hints(workspace),
        }
    };

    // Extract customer domains from morning's schedule.json if available
    let mut customer_domains = HashSet::new();
    let schedule_path = workspace.join("_today").join("data").join("schedule.json");
    if let Ok(content) = std::fs::read_to_string(&schedule_path) {
        if let Ok(schedule) = serde_json::from_str::<Value>(&content) {
            if let Some(meetings) = schedule.get("meetings").and_then(|v| v.as_array()) {
                for meeting in meetings {
                    if let Some(attendees) = meeting.get("attendees").and_then(|v| v.as_array()) {
                        for attendee in attendees {
                            if let Some(email) = attendee.as_str() {
                                if email.contains('@') {
                                    let domain =
                                        email.split('@').nth(1).unwrap_or("").to_lowercase();
                                    customer_domains.insert(domain);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let preset_email_keywords: Vec<String> = state
        .active_preset
        .read()
        .ok()
        .and_then(|guard| guard.as_ref().map(|p| p.email_priority_keywords.clone()))
        .unwrap_or_default();
    let email_result =
        fetch_and_classify_emails(&primary_user_domain, &customer_domains, &account_hints, &preset_email_keywords).await;
    if let Some(sync_error) = email_result.sync_error {
        return Err(ExecutionError::NetworkError(sync_error.message));
    }

    // Build refresh directive matching the shape Rust expects
    let mut high_priority = Vec::new();
    let mut classified = Vec::new();
    for email in &email_result.all {
        let priority = email
            .get("priority")
            .and_then(|v| v.as_str())
            .unwrap_or("medium");
        let entry = json!({
            "id": email.get("id"),
            "from": email.get("from"),
            "from_email": email.get("from_email"),
            "subject": email.get("subject"),
            "snippet": email.get("snippet"),
            "priority": priority,
        });
        if priority == "high" {
            high_priority.push(entry);
        } else {
            classified.push(entry);
        }
    }

    let directive = json!({
        "source": "email-refresh",
        "emails": {
            "highPriority": high_priority,
            "classified": classified,
            "mediumCount": email_result.medium_count,
            "lowCount": email_result.low_count,
        },
    });

    let output_path = workspace
        .join("_today")
        .join("data")
        .join("email-refresh-directive.json");
    write_directive(&output_path, &directive)?;

    log::info!(
        "refresh_emails: {} emails ({} high)",
        email_result.all.len(),
        email_result.high.len(),
    );
    Ok(())
}

// ============================================================================
// deliver_week (Phase 3)
// ============================================================================

/// Replaces deliver_week.py. Reads week-directive.json, writes week-overview.json.
///
/// This is Phase 3 of the week workflow — transforms the directive into the
/// format the frontend consumes (matching the Rust WeekOverview struct).
pub fn deliver_week(workspace: &Path) -> Result<(), String> {
    let data_dir = workspace.join("_today").join("data");
    let directive_path = data_dir.join("week-directive.json");

    let directive: Value = if directive_path.exists() {
        let raw = std::fs::read_to_string(&directive_path)
            .map_err(|e| format!("Failed to read week directive: {}", e))?;
        serde_json::from_str(&raw).map_err(|e| format!("Failed to parse week directive: {}", e))?
    } else {
        return Err("Week directive not found".to_string());
    };

    let overview = build_week_overview(&directive, &data_dir);
    let output_path = data_dir.join("week-overview.json");
    let content = serde_json::to_string_pretty(&overview)
        .map_err(|e| format!("JSON serialization failed: {}", e))?;

    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create dir: {}", e))?;
    }
    std::fs::write(&output_path, content)
        .map_err(|e| format!("Failed to write week overview: {}", e))?;

    log::info!("deliver_week: wrote {}", output_path.display());

    // Write human-readable markdown alongside JSON
    let md_content = build_week_markdown(&overview);
    let md_path = workspace.join("_today").join("week-overview.md");
    std::fs::write(&md_path, md_content)
        .map_err(|e| format!("Failed to write week-overview.md: {}", e))?;
    log::info!("deliver_week: wrote {}", md_path.display());

    Ok(())
}

// ============================================================================
// Person intelligence enrichment (I338)
// ============================================================================

/// Scan meeting contexts for person entities and trigger intelligence enrichment.
///
/// For each unique person entity with confidence >= 0.60:
/// 1. Write/update `dashboard.json` via `write_person_dashboard_json()`
/// 2. Check `intelligence.json` freshness (skip if < 2 hours old)
/// 3. If stale/missing: enqueue for AI enrichment
///
/// Returns the number of person entities enqueued.
fn queue_person_intelligence(
    meeting_contexts: &[Value],
    workspace: &Path,
    db: &crate::db::ActionDb,
    queue: &crate::intel_queue::IntelligenceQueue,
) -> usize {
    let mut seen = HashSet::new();
    let mut enqueued = 0;

    for ctx in meeting_contexts {
        let pe = match ctx.get("primary_entity") {
            Some(pe) => pe,
            None => continue,
        };

        let entity_type = pe.get("entity_type").and_then(|v| v.as_str()).unwrap_or("");
        if entity_type != "person" {
            continue;
        }

        let confidence = pe.get("confidence").and_then(|v| v.as_f64()).unwrap_or(0.0);
        if confidence < 0.60 {
            continue;
        }

        let entity_id = match pe.get("entity_id").and_then(|v| v.as_str()) {
            Some(id) => id.to_string(),
            None => continue,
        };

        if !seen.insert(entity_id.clone()) {
            continue;
        }

        // Write/update dashboard.json
        if let Ok(Some(person)) = db.get_person(&entity_id) {
            let _ = crate::people::write_person_dashboard_json(workspace, &person, db);
        }

        // Check intelligence.json freshness
        let entity_name = pe.get("entity_name").and_then(|v| v.as_str()).unwrap_or(&entity_id);
        let person_dir = crate::people::person_dir(workspace, entity_name);

        if let Ok(intel) = crate::entity_intel::read_intelligence_json(&person_dir) {
            if let Ok(ts) = chrono::DateTime::parse_from_rfc3339(&intel.enriched_at) {
                let age_secs = (Utc::now() - ts.with_timezone(&Utc)).num_seconds();
                if age_secs < 7200 {
                    log::debug!(
                        "queue_person_intelligence: skipping {} (enriched {}s ago)",
                        entity_id,
                        age_secs,
                    );
                    continue;
                }
            }
        }

        queue.enqueue(crate::intel_queue::IntelRequest {
            entity_id,
            entity_type: "person".to_string(),
            priority: crate::intel_queue::IntelPriority::CalendarChange,
            requested_at: std::time::Instant::now(),
        });
        enqueued += 1;
    }

    enqueued
}

// ============================================================================
// Shared helpers
// ============================================================================

fn get_config(state: &AppState) -> (String, Vec<String>, Option<String>) {
    let config_guard = state.config.read().ok();
    let config = config_guard.as_ref().and_then(|g| g.as_ref());

    let profile = config
        .map(|c| c.profile.clone())
        .unwrap_or_else(|| "general".to_string());

    let user_domains = config
        .map(|c| c.resolved_user_domains())
        .unwrap_or_default();

    let user_focus = config.and_then(|c| c.user_focus.clone());

    (profile, user_domains, user_focus)
}

/// Build account domain hints from Accounts/ directory.
fn build_account_domain_hints(workspace: &Path) -> HashSet<String> {
    let accounts_dir = workspace.join("Accounts");
    if !accounts_dir.is_dir() {
        return HashSet::new();
    }

    let mut hints = HashSet::new();
    if let Ok(entries) = std::fs::read_dir(&accounts_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false)
                && !name_str.starts_with('.')
                && !name_str.starts_with('_')
            {
                let slug: String = name_str
                    .to_lowercase()
                    .chars()
                    .filter(|c| c.is_alphanumeric())
                    .collect();
                if slug.len() >= 3 {
                    hints.insert(slug);
                }
            }
        }
    }
    hints
}

/// Fetch calendar events for today, classify, and compute time status.
async fn fetch_and_classify_today(
    today: NaiveDate,
    user_domains: &[String],
    entity_hints: &[crate::google_api::classify::EntityHint],
) -> (
    Vec<Value>,
    Vec<Value>,
    Value,
    serde_json::Map<String, Value>,
) {
    let access_token = match google_api::get_valid_access_token().await {
        Ok(t) => t,
        Err(e) => {
            log::warn!(
                "prepare_today: Google auth failed ({}), calendar will be empty",
                e
            );
            return (Vec::new(), Vec::new(), json!({}), serde_json::Map::new());
        }
    };

    let raw_events = match google_api::calendar::fetch_events(&access_token, today, today).await {
        Ok(e) => e,
        Err(e) => {
            log::warn!(
                "prepare_today: Calendar fetch failed ({}), calendar will be empty",
                e
            );
            return (Vec::new(), Vec::new(), json!({}), serde_json::Map::new());
        }
    };

    // Classify each event
    let mut classified = Vec::new();
    let mut events = Vec::new();
    for raw in &raw_events {
        let cm = google_api::classify::classify_meeting_multi(raw, user_domains, entity_hints);
        let ev = cm.to_calendar_event();
        classified.push(json!({
            "id": ev.id,
            "title": ev.title,
            "summary": ev.title,
            "start": ev.start,
            "end": ev.end,
            "type": ev.meeting_type,
            "attendees": raw.attendees,
            "organizer": raw.organizer,
            "external_domains": cm.external_domains,
            "is_recurring": raw.is_recurring,
            "account": cm.account(),
            "entities": cm.resolved_entities,
            "description": cm.description,
        }));
        events.push(json!({
            "id": ev.id,
            "summary": ev.title,
            "start": ev.start,
            "end": ev.end,
        }));
    }

    // Bucket by type
    let mut meetings_by_type = serde_json::Map::new();
    for ev in &classified {
        let mt = ev
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("external");
        meetings_by_type
            .entry(mt.to_string())
            .or_insert_with(|| json!([]))
            .as_array_mut()
            .unwrap()
            .push(ev.clone());
    }

    // Time status
    let now = Utc::now();
    let mut time_status = serde_json::Map::new();
    time_status.insert("past".to_string(), json!([]));
    time_status.insert("in_progress".to_string(), json!([]));
    time_status.insert("upcoming".to_string(), json!([]));

    for ev in &classified {
        let event_id = ev.get("id").and_then(|v| v.as_str()).unwrap_or("");
        let start_str = ev.get("start").and_then(|v| v.as_str()).unwrap_or("");
        let end_str = ev.get("end").and_then(|v| v.as_str()).unwrap_or("");

        let start_dt = chrono::DateTime::parse_from_rfc3339(&start_str.replace('Z', "+00:00"))
            .ok()
            .or_else(|| chrono::DateTime::parse_from_rfc3339(start_str).ok());
        let end_dt = chrono::DateTime::parse_from_rfc3339(&end_str.replace('Z', "+00:00"))
            .ok()
            .or_else(|| chrono::DateTime::parse_from_rfc3339(end_str).ok());

        let bucket = match (start_dt, end_dt) {
            (Some(_s), Some(e)) if now >= e => "past",
            (Some(s), end_opt) if s <= now && end_opt.is_none_or(|e| now < e) => "in_progress",
            _ => "upcoming",
        };

        time_status
            .get_mut(bucket)
            .unwrap()
            .as_array_mut()
            .unwrap()
            .push(json!(event_id));
    }

    (
        classified,
        events,
        Value::Object(meetings_by_type),
        time_status,
    )
}

/// Fetch calendar events for a week, classify, and organize by day.
async fn fetch_and_classify_week(
    monday: NaiveDate,
    friday: NaiveDate,
    user_domains: &[String],
    entity_hints: &[crate::google_api::classify::EntityHint],
) -> (
    Vec<Value>,
    Vec<Value>,
    Value,
    serde_json::Map<String, Value>,
    Value,
) {
    let access_token = match google_api::get_valid_access_token().await {
        Ok(t) => t,
        Err(e) => {
            log::warn!("prepare_week: Google auth failed ({})", e);
            let empty_days = json!({
                "Monday": [], "Tuesday": [], "Wednesday": [],
                "Thursday": [], "Friday": [],
            });
            return (
                Vec::new(),
                Vec::new(),
                json!({}),
                serde_json::Map::new(),
                empty_days,
            );
        }
    };

    let raw_events = match google_api::calendar::fetch_events(&access_token, monday, friday).await {
        Ok(e) => e,
        Err(e) => {
            log::warn!("prepare_week: Calendar fetch failed ({})", e);
            let empty_days = json!({
                "Monday": [], "Tuesday": [], "Wednesday": [],
                "Thursday": [], "Friday": [],
            });
            return (
                Vec::new(),
                Vec::new(),
                json!({}),
                serde_json::Map::new(),
                empty_days,
            );
        }
    };

    let mut classified = Vec::new();
    let mut events = Vec::new();
    for raw in &raw_events {
        let cm = google_api::classify::classify_meeting_multi(raw, user_domains, entity_hints);
        let ev = cm.to_calendar_event();
        classified.push(json!({
            "id": ev.id,
            "title": ev.title,
            "summary": ev.title,
            "start": ev.start,
            "end": ev.end,
            "type": ev.meeting_type,
            "attendees": raw.attendees,
            "organizer": raw.organizer,
            "external_domains": cm.external_domains,
            "is_recurring": raw.is_recurring,
            "account": cm.account(),
            "entities": cm.resolved_entities,
            "description": cm.description,
        }));
        events.push(json!({
            "id": ev.id,
            "summary": ev.title,
            "start": ev.start,
            "end": ev.end,
        }));
    }

    // Bucket by type
    let mut meetings_by_type = serde_json::Map::new();
    for ev in &classified {
        let mt = ev
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("external");
        meetings_by_type
            .entry(mt.to_string())
            .or_insert_with(|| json!([]))
            .as_array_mut()
            .unwrap()
            .push(ev.clone());
    }

    // Organize by day
    let mut events_by_day = serde_json::Map::new();
    for day_name in DAY_NAMES {
        events_by_day.insert(day_name.to_string(), json!([]));
    }
    for ev in &classified {
        let start_str = ev.get("start").and_then(|v| v.as_str()).unwrap_or("");
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&start_str.replace('Z', "+00:00")) {
            let weekday = dt.weekday().num_days_from_monday() as usize;
            if weekday < 5 {
                events_by_day
                    .get_mut(DAY_NAMES[weekday])
                    .unwrap()
                    .as_array_mut()
                    .unwrap()
                    .push(ev.clone());
            }
        }
    }

    let time_status = serde_json::Map::new(); // Week doesn't need time_status

    (
        classified,
        events,
        Value::Object(meetings_by_type),
        time_status,
        Value::Object(events_by_day),
    )
}

/// Fetch and classify emails (async, uses google_api).
struct EmailResult {
    all: Vec<Value>,
    high: Vec<Value>,
    low: Vec<Value>,
    medium_count: u64,
    low_count: u64,
    sync_error: Option<EmailSyncFailure>,
}

#[derive(Debug, Clone)]
struct EmailSyncFailure {
    stage: &'static str,
    code: &'static str,
    message: String,
}

impl EmailSyncFailure {
    fn to_json(&self) -> Value {
        json!({
            "stage": self.stage,
            "code": self.code,
            "message": self.message,
        })
    }
}

async fn fetch_and_classify_emails(
    user_domain: &str,
    customer_domains: &HashSet<String>,
    account_hints: &HashSet<String>,
    extra_high_keywords: &[String],
) -> EmailResult {
    let access_token = match google_api::get_valid_access_token().await {
        Ok(t) => t,
        Err(e) => {
            log::warn!("Email fetch: auth failed ({}), emails will be empty", e);
            return EmailResult {
                all: Vec::new(),
                high: Vec::new(),
                low: Vec::new(),
                medium_count: 0,
                low_count: 0,
                sync_error: Some(EmailSyncFailure {
                    stage: "fetch",
                    code: "gmail_auth_failed",
                    message: format!("Email fetch authentication failed: {}", e),
                }),
            };
        }
    };

    let raw_emails = match google_api::gmail::fetch_unread_emails(&access_token, 30).await {
        Ok(e) => e,
        Err(e) => {
            log::warn!("Email fetch failed ({}), emails will be empty", e);
            return EmailResult {
                all: Vec::new(),
                high: Vec::new(),
                low: Vec::new(),
                medium_count: 0,
                low_count: 0,
                sync_error: Some(EmailSyncFailure {
                    stage: "fetch",
                    code: "gmail_fetch_failed",
                    message: format!("Email fetch failed: {}", e),
                }),
            };
        }
    };

    let mut all = Vec::new();
    let mut high = Vec::new();
    let mut low = Vec::new();
    let mut medium_count: u64 = 0;
    let mut low_count: u64 = 0;

    for email in &raw_emails {
        let priority = email_classify::classify_email_priority_with_extras(
            &email.from,
            &email.subject,
            &email.list_unsubscribe,
            &email.precedence,
            customer_domains,
            user_domain,
            account_hints,
            extra_high_keywords,
        );

        let from_email = email_classify::extract_email_address(&email.from);

        let obj = json!({
            "id": email.id,
            "thread_id": email.thread_id,
            "from": email.from,
            "from_email": from_email,
            "subject": email.subject,
            "snippet": email.snippet,
            "date": email.date,
            "priority": priority,
        });

        all.push(obj.clone());
        match priority {
            "high" => high.push(obj),
            "medium" => medium_count += 1,
            _ => {
                low_count += 1;
                low.push(obj);
            }
        }
    }

    EmailResult {
        all,
        high,
        low,
        medium_count,
        low_count,
        sync_error: None,
    }
}

/// I318: Track thread positions from fetched high-priority emails.
/// Groups emails by thread_id, determines last sender per thread,
/// persists to email_threads table, and emits thread_position signals.
/// Returns (total_tracked, awaiting_reply_count).
fn track_thread_positions(
    emails: &[Value],
    user_domain: &str,
    db: &crate::db::ActionDb,
) -> (usize, usize) {
    use std::collections::HashMap;

    // Group emails by thread_id, keeping the most recent and counting per thread
    let mut threads: HashMap<String, (&Value, String, usize)> = HashMap::new(); // -> (email, date, count)

    for email in emails {
        let thread_id = email.get("thread_id").and_then(|v| v.as_str()).unwrap_or("");
        if thread_id.is_empty() {
            continue;
        }
        let date = email.get("date").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let entry = threads.entry(thread_id.to_string()).or_insert((email, date.clone(), 0));
        entry.2 += 1; // increment message count
        // Keep the email with the later date
        if date > entry.1 {
            entry.0 = email;
            entry.1 = date;
        }
    }

    let mut total = 0;
    let mut awaiting = 0;

    for (thread_id, (email, date, msg_count)) in &threads {
        let subject = email.get("subject").and_then(|v| v.as_str()).unwrap_or("");
        let from_email = email.get("from_email").and_then(|v| v.as_str()).unwrap_or("");
        let sender_domain = from_email.split('@').nth(1).unwrap_or("");
        let user_is_last_sender = !sender_domain.is_empty()
            && !user_domain.is_empty()
            && sender_domain.eq_ignore_ascii_case(user_domain);

        if !user_is_last_sender {
            awaiting += 1;
        }

        let _ = db.upsert_email_thread(
            thread_id,
            subject,
            from_email,
            date,
            *msg_count as i32,
            user_is_last_sender,
        );

        // Emit thread_position signal
        let signal_value = if user_is_last_sender {
            "waiting_on_them"
        } else {
            "awaiting_reply"
        };
        let _ = crate::signals::bus::emit_signal(
            db,
            "thread",
            thread_id,
            "thread_position",
            "email_thread",
            Some(signal_value),
            if user_is_last_sender { 0.7 } else { 0.85 },
        );

        total += 1;
    }

    (total, awaiting)
}

/// Compute human-readable wait duration from a date string.
fn compute_wait_duration(date_str: &str, now: &chrono::DateTime<Utc>) -> String {
    // Try parsing various date formats
    if let Ok(dt) = chrono::DateTime::parse_from_rfc2822(date_str) {
        let days = (*now - dt.with_timezone(&Utc)).num_days();
        return match days {
            0 => "today".to_string(),
            1 => "1 day".to_string(),
            n if n > 0 => format!("{} days", n),
            _ => String::new(),
        };
    }
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(date_str) {
        let days = (*now - dt.with_timezone(&Utc)).num_days();
        return match days {
            0 => "today".to_string(),
            1 => "1 day".to_string(),
            n if n > 0 => format!("{} days", n),
            _ => String::new(),
        };
    }
    String::new()
}

/// I322: Synthesize a 2-4 sentence email briefing narrative from:
/// - Thread positions (replies_needed from I318)
/// - High-priority emails
/// - Meeting contexts with email digests (I317)
///
/// Returns None if there's nothing meaningful to say.
fn synthesize_email_narrative(
    replies_needed: &[Value],
    high_priority: &[Value],
    meeting_contexts: &[Value],
    cadence_anomalies: &[crate::signals::cadence::CadenceAnomaly],
) -> Option<String> {
    let mut sentences = Vec::new();

    // Thread position sentence
    if !replies_needed.is_empty() {
        let count = replies_needed.len();
        if count == 1 {
            let subject = replies_needed[0]
                .get("subject")
                .and_then(|v| v.as_str())
                .unwrap_or("a thread");
            let from = replies_needed[0]
                .get("from")
                .and_then(|v| v.as_str())
                .unwrap_or("someone");
            sentences.push(format!("{} replied to \"{}\" — awaiting your response.", from, subject));
        } else {
            sentences.push(format!("{} threads are awaiting your reply.", count));
        }
    }

    // High-priority count
    if high_priority.len() > 2 {
        sentences.push(format!("{} high-priority emails today.", high_priority.len()));
    } else if high_priority.len() == 1 {
        let from = high_priority[0]
            .get("from")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown");
        let subject = high_priority[0]
            .get("subject")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if !subject.is_empty() {
            sentences.push(format!("Priority email from {}: \"{}\".", from, subject));
        }
    }

    // Meeting-linked emails
    let meetings_with_email: Vec<&str> = meeting_contexts
        .iter()
        .filter(|mc| {
            mc.get("pre_meeting_email_context")
                .and_then(|v| v.as_array())
                .is_some_and(|a| !a.is_empty())
        })
        .filter_map(|mc| {
            mc.get("title")
                .or_else(|| mc.get("account"))
                .and_then(|v| v.as_str())
        })
        .collect();

    if !meetings_with_email.is_empty() {
        if meetings_with_email.len() == 1 {
            sentences.push(format!(
                "Email threads connected to your {} meeting.",
                meetings_with_email[0]
            ));
        } else {
            sentences.push(format!(
                "Email threads connected to {} of today's meetings.",
                meetings_with_email.len()
            ));
        }
    }

    // Cadence anomalies (I319)
    let quiet_count = cadence_anomalies
        .iter()
        .filter(|a| a.anomaly_type == "gone_quiet")
        .count();
    let spike_count = cadence_anomalies
        .iter()
        .filter(|a| a.anomaly_type == "activity_spike")
        .count();
    if quiet_count > 0 {
        if quiet_count == 1 {
            let a = cadence_anomalies.iter().find(|a| a.anomaly_type == "gone_quiet").unwrap();
            sentences.push(format!(
                "{} email volume dropped {:.0}% this week.",
                a.entity_id,
                (1.0 - a.current_count as f64 / a.rolling_avg) * 100.0,
            ));
        } else {
            sentences.push(format!("{} accounts have gone quiet on email.", quiet_count));
        }
    }
    if spike_count > 0 {
        if spike_count == 1 {
            let a = cadence_anomalies.iter().find(|a| a.anomaly_type == "activity_spike").unwrap();
            sentences.push(format!(
                "Unusual email surge from {} ({} vs {:.0}/week average).",
                a.entity_id, a.current_count, a.rolling_avg,
            ));
        } else {
            sentences.push(format!("{} accounts spiking in email volume.", spike_count));
        }
    }

    if sentences.is_empty() {
        None
    } else {
        Some(sentences.join(" "))
    }
}

/// Extract customer domains from classified meetings.
fn extract_customer_domains(meetings_by_type: &Value) -> HashSet<String> {
    let mut domains = HashSet::new();
    if let Some(customer_meetings) = meetings_by_type.get("customer").and_then(|v| v.as_array()) {
        for ev in customer_meetings {
            if let Some(ext_domains) = ev.get("external_domains").and_then(|v| v.as_array()) {
                for d in ext_domains {
                    if let Some(s) = d.as_str() {
                        domains.insert(s.to_string());
                    }
                }
            }
        }
    }
    domains
}

/// List existing files in {workspace}/_today/.
fn inventory_today_files(workspace: &Path) -> Vec<String> {
    let today_dir = workspace.join("_today");
    if !today_dir.is_dir() {
        return Vec::new();
    }
    let mut files: Vec<String> = std::fs::read_dir(&today_dir)
        .ok()
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| {
            e.file_type().map(|ft| ft.is_file()).unwrap_or(false)
                && !e.file_name().to_string_lossy().starts_with('.')
        })
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    files.sort();
    files
}

/// Count files pending in {workspace}/_inbox/.
fn count_inbox_pending(workspace: &Path) -> usize {
    let inbox_dir = workspace.join("_inbox");
    if !inbox_dir.is_dir() {
        return 0;
    }
    std::fs::read_dir(&inbox_dir)
        .ok()
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| {
            e.file_type().map(|ft| ft.is_file()).unwrap_or(false)
                && !e.file_name().to_string_lossy().starts_with('.')
        })
        .count()
}

/// Generate AI task list for Phase 2 enrichment.
fn generate_ai_tasks(
    classified: &[Value],
    time_status: &serde_json::Map<String, Value>,
    emails_high: &[Value],
) -> Vec<Value> {
    let mut tasks = Vec::new();
    let past_ids: HashSet<String> = time_status
        .get("past")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    for meeting in classified {
        let event_id = meeting.get("id").and_then(|v| v.as_str()).unwrap_or("");
        let meeting_type = meeting.get("type").and_then(|v| v.as_str()).unwrap_or("");

        if past_ids.contains(event_id) {
            continue;
        }
        if meeting_type == "personal" || meeting_type == "all_hands" {
            continue;
        }

        match meeting_type {
            "customer" | "qbr" => {
                tasks.push(json!({
                    "type": "generate_meeting_prep",
                    "event_id": event_id,
                    "meeting_type": meeting_type,
                    "priority": "high",
                }));
            }
            "training" => {
                tasks.push(json!({
                    "type": "generate_meeting_prep",
                    "event_id": event_id,
                    "meeting_type": meeting_type,
                    "priority": "medium",
                }));
            }
            "external" => {
                let has_unknown = meeting
                    .get("external_domains")
                    .and_then(|v| v.as_array())
                    .map(|a| !a.is_empty())
                    .unwrap_or(false);
                tasks.push(json!({
                    "type": if has_unknown { "research_unknown_meeting" } else { "generate_meeting_prep" },
                    "event_id": event_id,
                    "meeting_type": meeting_type,
                    "priority": if has_unknown { "medium" } else { "low" },
                }));
            }
            "internal" | "team_sync" | "one_on_one" => {
                tasks.push(json!({
                    "type": "generate_meeting_prep",
                    "event_id": event_id,
                    "meeting_type": meeting_type,
                    "priority": "low",
                }));
            }
            _ => {}
        }
    }

    // Email summaries for high-priority emails
    for email in emails_high {
        tasks.push(json!({
            "type": "summarize_email",
            "email_id": email.get("id"),
            "thread_id": email.get("thread_id"),
            "priority": "medium",
        }));
    }

    // Generate daily briefing narrative
    tasks.push(json!({
        "type": "generate_briefing_narrative",
        "priority": "high",
    }));

    tasks
}

/// Strip attendees from meetings_by_type for lean directive output.
fn lean_meetings_by_type(meetings_by_type: &Value) -> Value {
    let mut lean = serde_json::Map::new();
    if let Some(obj) = meetings_by_type.as_object() {
        for (mt, meetings) in obj {
            let lean_meetings: Vec<Value> = meetings
                .as_array()
                .unwrap_or(&Vec::new())
                .iter()
                .map(|m| {
                    let mut cloned = m.clone();
                    if let Some(obj) = cloned.as_object_mut() {
                        obj.remove("attendees");
                    }
                    cloned
                })
                .collect();
            lean.insert(mt.clone(), json!(lean_meetings));
        }
    }
    Value::Object(lean)
}

/// Write directive JSON to disk, creating parent dirs as needed.
fn write_directive(path: &Path, data: &Value) -> Result<(), ExecutionError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| ExecutionError::ScriptFailed {
            code: 1,
            stderr: format!("Failed to create dir {}: {}", parent.display(), e),
        })?;
    }
    let content = serde_json::to_string_pretty(data)
        .map_err(|e| ExecutionError::ParseError(format!("JSON serialization failed: {}", e)))?;
    std::fs::write(path, content).map_err(|e| ExecutionError::ScriptFailed {
        code: 1,
        stderr: format!("Failed to write {}: {}", path.display(), e),
    })?;
    Ok(())
}

/// Human-friendly date range string, e.g. "February 2-6, 2026".
fn format_date_range(monday: NaiveDate, friday: NaiveDate) -> String {
    if monday.month() == friday.month() {
        format!(
            "{} {}-{}, {}",
            monday.format("%B"),
            monday.day(),
            friday.day(),
            friday.year(),
        )
    } else {
        format!(
            "{} {} - {} {}, {}",
            monday.format("%B"),
            monday.day(),
            friday.format("%B"),
            friday.day(),
            friday.year(),
        )
    }
}

// ============================================================================
// deliver_week helpers (ported from deliver_week.py)
// ============================================================================

fn build_week_overview(directive: &Value, data_dir: &Path) -> Value {
    let context = directive.get("context").cloned().unwrap_or(json!({}));

    let week_number_raw = context
        .get("weekNumber")
        .or_else(|| context.get("week_number"))
        .cloned()
        .unwrap_or(json!(0));
    let week_number = if let Some(s) = week_number_raw.as_str() {
        if s.starts_with('W') {
            s.to_string()
        } else {
            format!("W{:02}", s.parse::<u32>().unwrap_or(0))
        }
    } else {
        format!("W{:02}", week_number_raw.as_u64().unwrap_or(0))
    };

    let date_range = context
        .get("dateRange")
        .or_else(|| context.get("date_range_display"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let monday_str = context.get("monday").and_then(|v| v.as_str()).unwrap_or("");

    // Build days array
    let meetings_by_day = directive.get("meetingsByDay").cloned().unwrap_or(json!({}));

    let mut days = Vec::new();
    for (i, day_name) in DAY_NAMES.iter().enumerate() {
        let day_date = if !monday_str.is_empty() {
            NaiveDate::parse_from_str(monday_str, "%Y-%m-%d")
                .ok()
                .map(|m| (m + chrono::Duration::days(i as i64)).to_string())
                .unwrap_or_default()
        } else {
            String::new()
        };

        let day_meetings = meetings_by_day
            .get(day_name)
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        days.push(build_week_day(&day_date, day_name, &day_meetings, data_dir));
    }

    let action_summary = build_action_summary(directive, data_dir);
    let focus_areas = build_focus_areas(directive);
    let time_blocks = build_time_blocks(directive);
    let readiness_checks = build_readiness_checks(directive, data_dir);
    let day_shapes = build_day_shapes(directive, data_dir);

    let mut result = json!({
        "weekNumber": week_number,
        "dateRange": date_range,
        "days": days,
        "actionSummary": action_summary,
        "hygieneAlerts": build_hygiene_alerts(directive),
        "focusAreas": focus_areas,
        "availableTimeBlocks": time_blocks,
        "dayShapes": day_shapes,
    });

    if !readiness_checks.is_empty() {
        result["readinessChecks"] = json!(readiness_checks);
    }
    // weekNarrative and topPriority left null — I94 adds them via AI

    result
}

fn build_week_day(date: &str, day_name: &str, meetings_raw: &[Value], data_dir: &Path) -> Value {
    let mut meetings = Vec::new();
    for m in meetings_raw {
        let meeting_type =
            normalise_meeting_type(m.get("type").and_then(|v| v.as_str()).unwrap_or("internal"));
        if meeting_type == "personal" {
            continue;
        }

        let start = m.get("start").and_then(|v| v.as_str()).unwrap_or("");
        let time_display = format_time_display(start);

        let meeting_id = m.get("id").and_then(|v| v.as_str()).unwrap_or("");
        let prep_status = resolve_prep_status(meeting_id, &meeting_type, data_dir);

        meetings.push(json!({
            "time": if time_display.is_empty() { "TBD".to_string() } else { time_display },
            "title": m.get("title").or_else(|| m.get("summary")).and_then(|v| v.as_str()).unwrap_or("Meeting"),
            "account": m.get("account"),
            "type": meeting_type,
            "prepStatus": prep_status,
        }));
    }

    json!({
        "date": date,
        "dayName": day_name,
        "meetings": meetings,
    })
}

/// Resolve actual prep status for a meeting by checking prep files on disk.
fn resolve_prep_status(meeting_id: &str, meeting_type: &str, data_dir: &Path) -> String {
    let prep_eligible = [
        "customer",
        "qbr",
        "partnership",
        "internal",
        "team_sync",
        "one_on_one",
    ];
    if meeting_id.is_empty() || !prep_eligible.contains(&meeting_type) {
        // Non-prep-eligible meetings don't need prep
        return "done".to_string();
    }

    let prep_path = data_dir.join("preps").join(format!("{}.json", meeting_id));
    if !prep_path.exists() {
        return "prep_needed".to_string();
    }

    // Check prep file content quality
    match std::fs::read_to_string(&prep_path) {
        Ok(content) => {
            if let Ok(prep) = serde_json::from_str::<Value>(&content) {
                let has_agenda = prep
                    .get("proposedAgenda")
                    .and_then(|v| v.as_array())
                    .is_some_and(|a| !a.is_empty());
                let has_talking_points = prep
                    .get("talkingPoints")
                    .and_then(|v| v.as_array())
                    .is_some_and(|a| !a.is_empty());
                if has_agenda || has_talking_points {
                    "prep_ready".to_string()
                } else {
                    "context_needed".to_string()
                }
            } else {
                "context_needed".to_string()
            }
        }
        Err(_) => "prep_needed".to_string(),
    }
}

fn build_action_summary(directive: &Value, data_dir: &Path) -> Value {
    let actions = directive.get("actions").cloned().unwrap_or(json!({}));
    let overdue = actions
        .get("overdue")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let mut this_week = actions
        .get("thisWeek")
        .or_else(|| actions.get("this_week"))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    // Fallback to actions.json
    if overdue.is_empty() && this_week.is_empty() {
        let actions_path = data_dir.join("actions.json");
        if let Ok(content) = std::fs::read_to_string(&actions_path) {
            if let Ok(today_actions) = serde_json::from_str::<Value>(&content) {
                if let Some(all) = today_actions.get("actions").and_then(|v| v.as_array()) {
                    this_week = all
                        .iter()
                        .filter(|a| {
                            !a.get("isOverdue")
                                .and_then(|v| v.as_bool())
                                .unwrap_or(false)
                        })
                        .cloned()
                        .collect();
                }
            }
        }
    }

    let critical_items: Vec<String> = overdue
        .iter()
        .take(10)
        .filter_map(|task| {
            let title = task.get("title").and_then(|v| v.as_str()).unwrap_or("");
            let account = task.get("account").and_then(|v| v.as_str()).unwrap_or("");
            if title.is_empty() {
                None
            } else if account.is_empty() {
                Some(title.to_string())
            } else {
                Some(format!("{} - {}", title, account))
            }
        })
        .collect();

    // Build title→id lookup from actions.json so week items link to real action IDs
    let action_id_by_title: std::collections::HashMap<String, String> = {
        let mut map = std::collections::HashMap::new();
        let actions_path = data_dir.join("actions.json");
        if let Ok(content) = std::fs::read_to_string(&actions_path) {
            if let Ok(parsed) = serde_json::from_str::<Value>(&content) {
                if let Some(all) = parsed.get("actions").and_then(|v| v.as_array()) {
                    for a in all {
                        if let (Some(id), Some(title)) = (
                            a.get("id").and_then(|v| v.as_str()),
                            a.get("title").and_then(|v| v.as_str()),
                        ) {
                            map.insert(title.to_string(), id.to_string());
                        }
                    }
                }
            }
        }
        map
    };

    let overdue_items: Vec<Value> = overdue
        .iter()
        .take(20)
        .enumerate()
        .map(|(i, a)| {
            let title = a
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let due_date = a
                .get("due_date")
                .or_else(|| a.get("dueDate"))
                .and_then(|v| v.as_str());
            let days_overdue = due_date.and_then(|d| {
                NaiveDate::parse_from_str(d, "%Y-%m-%d")
                    .ok()
                    .map(|dd| (Utc::now().date_naive() - dd).num_days())
            });
            let id = a
                .get("id")
                .and_then(|v| v.as_str())
                .map(String::from)
                .or_else(|| action_id_by_title.get(&title).cloned())
                .unwrap_or_else(|| format!("overdue-{}", i));
            json!({
                "id": id,
                "title": title,
                "account": a.get("account"),
                "dueDate": due_date,
                "priority": a.get("priority").and_then(|v| v.as_str()).unwrap_or("P3"),
                "daysOverdue": days_overdue,
            })
        })
        .collect();

    let due_this_week_items: Vec<Value> = this_week
        .iter()
        .take(20)
        .enumerate()
        .map(|(i, a)| {
            let title = a
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let id = a
                .get("id")
                .and_then(|v| v.as_str())
                .map(String::from)
                .or_else(|| action_id_by_title.get(&title).cloned())
                .unwrap_or_else(|| format!("week-{}", i));
            json!({
                "id": id,
                "title": title,
                "account": a.get("account"),
                "dueDate": a.get("due_date").or_else(|| a.get("dueDate")),
                "priority": a.get("priority").and_then(|v| v.as_str()).unwrap_or("P3"),
            })
        })
        .collect();

    json!({
        "overdueCount": overdue.len(),
        "dueThisWeek": this_week.len(),
        "criticalItems": critical_items,
        "overdue": overdue_items,
        "dueThisWeekItems": due_this_week_items,
    })
}

/// Build readiness checks: surfaces prep gaps, overdue actions, and stale contacts.
fn build_readiness_checks(directive: &Value, data_dir: &Path) -> Vec<Value> {
    let mut checks = Vec::new();
    let prep_eligible = [
        "customer",
        "qbr",
        "partnership",
        "internal",
        "team_sync",
        "one_on_one",
    ];

    // 1. Meetings without preps
    let meetings_by_day = directive.get("meetingsByDay").cloned().unwrap_or(json!({}));
    if let Some(obj) = meetings_by_day.as_object() {
        for day_meetings in obj.values() {
            if let Some(arr) = day_meetings.as_array() {
                for m in arr {
                    let meeting_type = normalise_meeting_type(
                        m.get("type").and_then(|v| v.as_str()).unwrap_or("internal"),
                    );
                    if !prep_eligible.contains(&meeting_type.as_str()) {
                        continue;
                    }
                    let meeting_id = m.get("id").and_then(|v| v.as_str()).unwrap_or("");
                    if meeting_id.is_empty() {
                        continue;
                    }
                    let prep_path = data_dir.join("preps").join(format!("{}.json", meeting_id));
                    if !prep_path.exists() {
                        let title = m
                            .get("title")
                            .or_else(|| m.get("summary"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("Meeting");
                        checks.push(json!({
                            "checkType": "no_prep",
                            "message": format!("{} has no prep", title),
                            "severity": "action_needed",
                            "meetingId": meeting_id,
                        }));
                    }
                }
            }
        }
    }

    // 2. Overdue actions
    let actions = directive.get("actions").cloned().unwrap_or(json!({}));
    let overdue = actions.get("overdue").and_then(|v| v.as_array());
    if let Some(o) = overdue {
        if !o.is_empty() {
            checks.push(json!({
                "checkType": "overdue_action",
                "message": format!("{} overdue action{} need{} attention",
                    o.len(),
                    if o.len() == 1 { "" } else { "s" },
                    if o.len() == 1 { "s" } else { "" },
                ),
                "severity": "action_needed",
            }));
        }
    }

    // 3. Stale contacts — check meetingContexts for last meeting > 30 days
    let mut seen_accounts = HashSet::new();
    if let Some(contexts) = directive.get("meetingContexts").and_then(|v| v.as_array()) {
        for ctx in contexts {
            let account = ctx.get("account").and_then(|v| v.as_str()).unwrap_or("");
            if account.is_empty() || seen_accounts.contains(account) {
                continue;
            }
            seen_accounts.insert(account.to_string());

            if let Some(last_meeting) = ctx
                .get("lastMeetingDate")
                .or_else(|| ctx.get("last_meeting_date"))
                .and_then(|v| v.as_str())
            {
                if let Ok(last_date) = NaiveDate::parse_from_str(last_meeting, "%Y-%m-%d") {
                    let days_since = (Utc::now().date_naive() - last_date).num_days();
                    if days_since > 30 {
                        checks.push(json!({
                            "checkType": "stale_contact",
                            "message": format!("{} — last meeting {} days ago", account, days_since),
                            "severity": "heads_up",
                            "accountId": account,
                        }));
                    }
                }
            }
        }
    }

    checks
}

/// Build per-day shape with density classification and meeting details.
fn build_day_shapes(directive: &Value, data_dir: &Path) -> Vec<Value> {
    let context = directive.get("context").cloned().unwrap_or(json!({}));
    let monday_str = context.get("monday").and_then(|v| v.as_str()).unwrap_or("");
    let meetings_by_day = directive.get("meetingsByDay").cloned().unwrap_or(json!({}));
    let time_blocks_raw = directive
        .get("timeBlocks")
        .or_else(|| directive.get("time_blocks"))
        .cloned()
        .unwrap_or(json!({}));
    let gaps_by_day = time_blocks_raw
        .get("gapsByDay")
        .or_else(|| time_blocks_raw.get("gaps_by_day"))
        .cloned()
        .unwrap_or(json!({}));
    // Generic suggestions ("Deep Work" / "Admin / Follow-up") keyed by (day, start)
    let generic_suggestions: std::collections::HashMap<(String, String), String> = time_blocks_raw
        .get("suggestions")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|s| {
                    let day = s.get("day").and_then(|v| v.as_str())?.to_string();
                    let start = s.get("start").and_then(|v| v.as_str())?.to_string();
                    let use_str = s.get("suggested_use").and_then(|v| v.as_str())?.to_string();
                    Some(((day, start), use_str))
                })
                .collect()
        })
        .unwrap_or_default();

    let mut shapes = Vec::new();

    for (i, day_name) in DAY_NAMES.iter().enumerate() {
        let day_date = if !monday_str.is_empty() {
            NaiveDate::parse_from_str(monday_str, "%Y-%m-%d")
                .ok()
                .map(|m| (m + chrono::Duration::days(i as i64)).to_string())
                .unwrap_or_default()
        } else {
            String::new()
        };

        let day_meetings_raw = meetings_by_day
            .get(day_name)
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        // Build meeting list (reusing same logic as build_week_day)
        let mut meetings = Vec::new();
        let mut total_minutes: u32 = 0;

        for m in &day_meetings_raw {
            let meeting_type = normalise_meeting_type(
                m.get("type").and_then(|v| v.as_str()).unwrap_or("internal"),
            );
            if meeting_type == "personal" {
                continue;
            }

            let start_str = m.get("start").and_then(|v| v.as_str()).unwrap_or("");
            let end_str = m.get("end").and_then(|v| v.as_str()).unwrap_or("");
            let time_display = format_time_display(start_str);

            // Calculate duration
            let duration = compute_meeting_duration(start_str, end_str);
            total_minutes += duration;

            let meeting_id = m.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let prep_status = resolve_prep_status(meeting_id, &meeting_type, data_dir);
            let stable_meeting_id = crate::workflow::deliver::meeting_primary_id(
                if meeting_id.is_empty() {
                    None
                } else {
                    Some(meeting_id)
                },
                m.get("title")
                    .or_else(|| m.get("summary"))
                    .and_then(|v| v.as_str())
                    .unwrap_or(""),
                start_str,
                &meeting_type,
            );

            meetings.push(json!({
                "time": if time_display.is_empty() { "TBD".to_string() } else { time_display },
                "title": m.get("title").or_else(|| m.get("summary")).and_then(|v| v.as_str()).unwrap_or("Meeting"),
                "account": m.get("account"),
                "meetingId": if stable_meeting_id.is_empty() { Value::Null } else { json!(stable_meeting_id) },
                "type": meeting_type,
                "prepStatus": prep_status,
            }));
        }

        // Density classification
        let density = match total_minutes {
            0..=90 => "light",
            91..=180 => "moderate",
            181..=300 => "busy",
            _ => "packed",
        };

        // Available blocks for this day
        let mut available_blocks = Vec::new();
        if let Some(day_gaps) = gaps_by_day.get(day_name).and_then(|v| v.as_array()) {
            for gap in day_gaps {
                let dur = gap
                    .get("duration_minutes")
                    .or_else(|| gap.get("durationMinutes"))
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                if dur < 30 {
                    continue;
                }
                let start = gap
                    .get("start")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let end = gap
                    .get("end")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                if !start.is_empty() && !end.is_empty() {
                    let mut block = json!({
                        "day": day_name,
                        "start": start.clone(),
                        "end": end,
                        "durationMinutes": dur,
                    });
                    // Apply generic suggestion if available
                    if let Some(suggestion) =
                        generic_suggestions.get(&(day_name.to_string(), start))
                    {
                        block
                            .as_object_mut()
                            .unwrap()
                            .insert("suggestedUse".to_string(), json!(suggestion));
                    }
                    available_blocks.push(block);
                }
            }
        }

        shapes.push(json!({
            "dayName": day_name,
            "date": day_date,
            "meetingCount": meetings.len(),
            "meetingMinutes": total_minutes,
            "density": density,
            "meetings": meetings,
            "availableBlocks": available_blocks,
        }));
    }

    shapes
}

/// Compute meeting duration in minutes from ISO start/end strings.
fn compute_meeting_duration(start: &str, end: &str) -> u32 {
    if start.is_empty() || end.is_empty() {
        return 30; // Default assumption
    }
    let parse = |s: &str| {
        chrono::DateTime::parse_from_rfc3339(&s.replace('Z', "+00:00"))
            .ok()
            .or_else(|| chrono::DateTime::parse_from_rfc3339(s).ok())
    };
    match (parse(start), parse(end)) {
        (Some(s), Some(e)) => ((e - s).num_minutes().max(0)) as u32,
        _ => 30,
    }
}

fn build_focus_areas(directive: &Value) -> Vec<String> {
    let mut areas = Vec::new();

    let meetings_by_day = directive.get("meetingsByDay").cloned().unwrap_or(json!({}));
    let empty_arr = Vec::new();
    let customer_count: usize = meetings_by_day
        .as_object()
        .map(|obj| {
            obj.values()
                .flat_map(|day| day.as_array().unwrap_or(&empty_arr).iter())
                .filter(|m| m.get("type").and_then(|v| v.as_str()) == Some("customer"))
                .count()
        })
        .unwrap_or(0);
    if customer_count > 0 {
        areas.push(format!("Customer meetings ({})", customer_count));
    }

    let actions = directive.get("actions").cloned().unwrap_or(json!({}));
    let overdue = actions.get("overdue").and_then(|v| v.as_array());
    if let Some(o) = overdue {
        if !o.is_empty() {
            areas.push(format!("Overdue items ({})", o.len()));
        }
    }

    let this_week = actions
        .get("thisWeek")
        .or_else(|| actions.get("this_week"))
        .and_then(|v| v.as_array());
    if let Some(tw) = this_week {
        if !tw.is_empty() {
            areas.push(format!("Due this week ({})", tw.len()));
        }
    }

    if areas.is_empty() {
        areas.push("Review weekly overview".to_string());
    }

    areas
}

fn build_time_blocks(directive: &Value) -> Vec<Value> {
    let mut blocks = Vec::new();
    let time_blocks_raw = directive
        .get("timeBlocks")
        .or_else(|| directive.get("time_blocks"))
        .cloned()
        .unwrap_or(json!({}));

    let suggestions = time_blocks_raw
        .get("suggestions")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    for s in &suggestions {
        let day = s.get("day").and_then(|v| v.as_str()).unwrap_or("");
        let mut start = s
            .get("start")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let mut end = s
            .get("end")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let duration = s
            .get("duration_minutes")
            .or_else(|| s.get("duration"))
            .and_then(|v| v.as_i64())
            .unwrap_or(30);

        // Extract HH:MM from ISO datetime
        if start.contains('T') {
            start = start.split('T').nth(1).unwrap_or("")[..5].to_string();
        }
        if end.contains('T') {
            end = end.split('T').nth(1).unwrap_or("")[..5].to_string();
        }

        let suggested_use = s
            .get("suggested_use")
            .or_else(|| s.get("block_type"))
            .and_then(|v| v.as_str())
            .unwrap_or("Focus")
            .to_string();

        if !day.is_empty() && !start.is_empty() && !end.is_empty() {
            blocks.push(json!({
                "day": day,
                "start": start,
                "end": end,
                "durationMinutes": duration,
                "suggestedUse": suggested_use,
            }));
        }
    }

    // If no suggestions, use raw gaps
    if blocks.is_empty() {
        let gaps_by_day = time_blocks_raw
            .get("gapsByDay")
            .or_else(|| time_blocks_raw.get("gaps_by_day"))
            .cloned()
            .unwrap_or(json!({}));

        for day_name in DAY_NAMES {
            if let Some(day_gaps) = gaps_by_day.get(day_name).and_then(|v| v.as_array()) {
                for gap in day_gaps {
                    let duration = gap
                        .get("duration_minutes")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0);
                    if duration < 30 {
                        continue;
                    }
                    let mut start = gap
                        .get("start")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let mut end = gap
                        .get("end")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    if start.contains('T') {
                        start = start.split('T').nth(1).unwrap_or("")[..5].to_string();
                    }
                    if end.contains('T') {
                        end = end.split('T').nth(1).unwrap_or("")[..5].to_string();
                    }
                    if !start.is_empty() && !end.is_empty() {
                        blocks.push(json!({
                            "day": day_name,
                            "start": start,
                            "end": end,
                            "durationMinutes": duration,
                            "suggestedUse": "Deep work",
                        }));
                    }
                }
            }
        }
    }

    blocks.truncate(10);
    blocks
}

/// Generate human-readable markdown from the week overview JSON.
fn build_week_markdown(overview: &Value) -> String {
    let mut md = String::new();

    // Header
    let week_number = overview
        .get("weekNumber")
        .and_then(|v| v.as_str())
        .unwrap_or("W??");
    let date_range = overview
        .get("dateRange")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    md.push_str(&format!("# Week {} — {}\n\n", week_number, date_range));

    // Readiness
    if let Some(checks) = overview.get("readinessChecks").and_then(|v| v.as_array()) {
        if !checks.is_empty() {
            md.push_str("## Readiness\n\n");
            for check in checks {
                let msg = check.get("message").and_then(|v| v.as_str()).unwrap_or("");
                let severity = check.get("severity").and_then(|v| v.as_str()).unwrap_or("");
                let icon = if severity == "action_needed" {
                    "\u{26a0}\u{fe0f}"
                } else {
                    "\u{2139}\u{fe0f}"
                };
                md.push_str(&format!("- {} {}\n", icon, msg));
            }
            md.push('\n');
        }
    }

    // Week Shape table
    if let Some(shapes) = overview.get("dayShapes").and_then(|v| v.as_array()) {
        md.push_str("## Week Shape\n\n");
        md.push_str("| Day | Meetings | Density | Focus |\n");
        md.push_str("|-----|----------|---------|-------|\n");
        for shape in shapes {
            let day = shape.get("dayName").and_then(|v| v.as_str()).unwrap_or("");
            let count = shape
                .get("meetingCount")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let density = shape.get("density").and_then(|v| v.as_str()).unwrap_or("");
            let density_cap = if density.is_empty() {
                String::new()
            } else {
                let mut c = density.chars();
                match c.next() {
                    Some(first) => first.to_uppercase().to_string() + c.as_str(),
                    None => String::new(),
                }
            };
            // Sum available blocks for focus time
            let focus_min: u64 = shape
                .get("availableBlocks")
                .and_then(|v| v.as_array())
                .map(|blocks| {
                    blocks
                        .iter()
                        .filter_map(|b| b.get("durationMinutes").and_then(|v| v.as_u64()))
                        .sum()
                })
                .unwrap_or(0);
            let focus_display = if focus_min >= 60 {
                format!("{}h{}m", focus_min / 60, focus_min % 60)
            } else {
                format!("{}m", focus_min)
            };
            md.push_str(&format!(
                "| {} | {} | {} | {} |\n",
                day, count, density_cap, focus_display
            ));
        }
        md.push('\n');
    }

    // Actions
    let action_summary = overview.get("actionSummary").cloned().unwrap_or(json!({}));
    let overdue_count = action_summary
        .get("overdueCount")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let due_count = action_summary
        .get("dueThisWeek")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    if overdue_count > 0 || due_count > 0 {
        md.push_str("## Actions\n\n");

        if let Some(overdue) = action_summary.get("overdue").and_then(|v| v.as_array()) {
            if !overdue.is_empty() {
                md.push_str(&format!("### Overdue ({})\n\n", overdue.len()));
                for item in overdue {
                    let title = item.get("title").and_then(|v| v.as_str()).unwrap_or("");
                    let account = item.get("account").and_then(|v| v.as_str()).unwrap_or("");
                    let priority = item
                        .get("priority")
                        .and_then(|v| v.as_str())
                        .unwrap_or("P3");
                    let days = item.get("daysOverdue").and_then(|v| v.as_i64());
                    let days_str = days
                        .map(|d| format!(" ({}d overdue)", d))
                        .unwrap_or_default();
                    if account.is_empty() {
                        md.push_str(&format!("- [{}] {}{}\n", priority, title, days_str));
                    } else {
                        md.push_str(&format!(
                            "- [{}] {} — {}{}\n",
                            priority, title, account, days_str
                        ));
                    }
                }
                md.push('\n');
            }
        }

        if let Some(due) = action_summary
            .get("dueThisWeekItems")
            .and_then(|v| v.as_array())
        {
            if !due.is_empty() {
                md.push_str(&format!("### Due This Week ({})\n\n", due.len()));
                for item in due {
                    let title = item.get("title").and_then(|v| v.as_str()).unwrap_or("");
                    let account = item.get("account").and_then(|v| v.as_str()).unwrap_or("");
                    let priority = item
                        .get("priority")
                        .and_then(|v| v.as_str())
                        .unwrap_or("P3");
                    let due_date = item.get("dueDate").and_then(|v| v.as_str()).unwrap_or("");
                    // Extract day name from date if possible
                    let day_label = NaiveDate::parse_from_str(due_date, "%Y-%m-%d")
                        .ok()
                        .map(|d| d.format("%a").to_string())
                        .unwrap_or_default();
                    let suffix = if !day_label.is_empty() {
                        format!(" ({})", day_label)
                    } else {
                        String::new()
                    };
                    if account.is_empty() {
                        md.push_str(&format!("- [{}] {}{}\n", priority, title, suffix));
                    } else {
                        md.push_str(&format!(
                            "- [{}] {} — {}{}\n",
                            priority, title, account, suffix
                        ));
                    }
                }
                md.push('\n');
            }
        }
    }

    // Account Health (hygiene alerts)
    if let Some(alerts) = overview.get("hygieneAlerts").and_then(|v| v.as_array()) {
        if !alerts.is_empty() {
            md.push_str("## Account Health\n\n");
            for alert in alerts {
                let account = alert.get("account").and_then(|v| v.as_str()).unwrap_or("");
                let lifecycle = alert
                    .get("lifecycle")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let arr = alert.get("arr").and_then(|v| v.as_str()).unwrap_or("");
                let issue = alert.get("issue").and_then(|v| v.as_str()).unwrap_or("");
                let meta = [lifecycle, arr]
                    .iter()
                    .filter(|s| !s.is_empty())
                    .copied()
                    .collect::<Vec<_>>()
                    .join(", ");
                if meta.is_empty() {
                    md.push_str(&format!("- **{}** — {}\n", account, issue));
                } else {
                    md.push_str(&format!("- **{}** ({}) — {}\n", account, meta, issue));
                }
            }
            md.push('\n');
        }
    }

    md
}

/// Build hygiene alerts from meeting contexts — flags at-risk/churned accounts.
fn build_hygiene_alerts(directive: &Value) -> Vec<Value> {
    let mut alerts = Vec::new();
    let mut seen_accounts = HashSet::new();

    if let Some(contexts) = directive.get("meetingContexts").and_then(|v| v.as_array()) {
        for ctx in contexts {
            let account = ctx.get("account").and_then(|v| v.as_str()).unwrap_or("");
            if account.is_empty() || seen_accounts.contains(account) {
                continue;
            }
            seen_accounts.insert(account.to_string());

            let account_data = ctx.get("account_data").cloned().unwrap_or(json!({}));
            let lifecycle = account_data
                .get("lifecycle")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let health = account_data
                .get("health")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let arr_raw = account_data.get("arr");
            let narrative = ctx.get("narrative").and_then(|v| v.as_str()).unwrap_or("");

            let needs_alert =
                matches!(health, "yellow" | "red") || lifecycle == "churned";

            if needs_alert {
                let severity = if health == "red" || lifecycle == "churned" {
                    "critical"
                } else {
                    "warning"
                };

                let issue = if !narrative.is_empty() {
                    format!("Health is {} — {}", health.to_uppercase(), narrative)
                } else {
                    format!(
                        "Health is {}, lifecycle {}",
                        health.to_uppercase(),
                        lifecycle
                    )
                };

                alerts.push(json!({
                    "account": account,
                    "lifecycle": lifecycle,
                    "arr": format_arr(arr_raw),
                    "issue": issue,
                    "severity": severity,
                }));
            }
        }
    }

    alerts
}

/// Format ARR value for display: number → "$1.2M" / "$350K" / "$50K", string passthrough.
fn format_arr(raw: Option<&Value>) -> String {
    match raw {
        Some(Value::Number(n)) => {
            if let Some(v) = n.as_f64() {
                if v >= 1_000_000.0 {
                    let m = v / 1_000_000.0;
                    if (m * 10.0).fract().abs() < 0.001 {
                        format!("${:.1}M", m)
                    } else {
                        format!("${:.2}M", m)
                    }
                } else if v >= 1_000.0 {
                    format!("${}K", (v / 1_000.0) as u64)
                } else {
                    format!("${}", v as u64)
                }
            } else {
                String::new()
            }
        }
        Some(Value::String(s)) => s.clone(),
        _ => String::new(),
    }
}

fn normalise_meeting_type(raw: &str) -> String {
    let normalised = raw.to_lowercase().replace([' ', '-'], "_");
    let valid = [
        "customer",
        "qbr",
        "training",
        "internal",
        "team_sync",
        "one_on_one",
        "partnership",
        "all_hands",
        "external",
        "personal",
    ];
    if valid.contains(&normalised.as_str()) {
        normalised
    } else {
        "internal".to_string()
    }
}

fn format_time_display(iso_string: &str) -> String {
    if iso_string.is_empty() || !iso_string.contains('T') {
        return String::new();
    }
    chrono::DateTime::parse_from_rfc3339(&iso_string.replace('Z', "+00:00"))
        .ok()
        .or_else(|| chrono::DateTime::parse_from_rfc3339(iso_string).ok())
        .map(|dt| {
            // Convert to system local time for display
            let local = dt.with_timezone(&chrono::Local);
            let h = local.format("%-I:%M %p").to_string();
            h
        })
        .unwrap_or_default()
}

/// Helper trait for NaiveDate ISO week access.
trait IsoWeekFields {
    fn iso_week_fields(&self) -> (i32, u32, u32);
}

impl IsoWeekFields for NaiveDate {
    fn iso_week_fields(&self) -> (i32, u32, u32) {
        let iso = self.iso_week();
        (
            iso.year(),
            iso.week(),
            self.weekday().num_days_from_monday() + 1,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_readiness_checks_no_prep() {
        let tmp = TempDir::new().unwrap();
        let data_dir = tmp.path();
        std::fs::create_dir_all(data_dir.join("preps")).unwrap();

        let directive = json!({
            "meetingsByDay": {
                "Monday": [
                    {"id": "m1", "title": "Customer Sync", "type": "customer", "start": "2025-02-10T13:00:00Z", "end": "2025-02-10T13:45:00Z"},
                    {"id": "m2", "title": "Team Standup", "type": "team_sync", "start": "2025-02-10T14:00:00Z", "end": "2025-02-10T14:15:00Z"}
                ]
            },
            "actions": {}
        });

        let checks = build_readiness_checks(&directive, data_dir);
        // I159: both customer and team_sync are now prep-eligible
        assert_eq!(checks.len(), 2);
        assert_eq!(checks[0]["checkType"], "no_prep");
        assert!(checks[0]["message"]
            .as_str()
            .unwrap()
            .contains("Customer Sync"));
        assert_eq!(checks[1]["checkType"], "no_prep");
        assert!(checks[1]["message"]
            .as_str()
            .unwrap()
            .contains("Team Standup"));
    }

    #[test]
    fn test_readiness_checks_with_prep() {
        let tmp = TempDir::new().unwrap();
        let data_dir = tmp.path();
        let preps_dir = data_dir.join("preps");
        std::fs::create_dir_all(&preps_dir).unwrap();
        // Write a prep file
        std::fs::write(
            preps_dir.join("m1.json"),
            r#"{"proposedAgenda": [{"topic": "Review"}]}"#,
        )
        .unwrap();

        let directive = json!({
            "meetingsByDay": {
                "Monday": [
                    {"id": "m1", "title": "Customer Sync", "type": "customer"}
                ]
            },
            "actions": {}
        });

        let checks = build_readiness_checks(&directive, data_dir);
        // No check for m1 since prep exists
        assert!(checks.is_empty());
    }

    #[test]
    fn test_readiness_checks_overdue_actions() {
        let tmp = TempDir::new().unwrap();
        let directive = json!({
            "meetingsByDay": {},
            "actions": {
                "overdue": [
                    {"title": "Task 1"},
                    {"title": "Task 2"}
                ]
            }
        });

        let checks = build_readiness_checks(&directive, tmp.path());
        assert_eq!(checks.len(), 1);
        assert_eq!(checks[0]["checkType"], "overdue_action");
        assert!(checks[0]["message"]
            .as_str()
            .unwrap()
            .contains("2 overdue actions"));
    }

    #[test]
    fn test_day_shapes_density() {
        let tmp = TempDir::new().unwrap();
        let data_dir = tmp.path();
        std::fs::create_dir_all(data_dir.join("preps")).unwrap();

        let directive = json!({
            "context": {"monday": "2025-02-10"},
            "meetingsByDay": {
                "Monday": [
                    {"id": "m1", "title": "Meeting 1", "type": "internal",
                     "start": "2025-02-10T09:00:00Z", "end": "2025-02-10T10:00:00Z"},
                    {"id": "m2", "title": "Meeting 2", "type": "internal",
                     "start": "2025-02-10T10:30:00Z", "end": "2025-02-10T11:30:00Z"},
                    {"id": "m3", "title": "Meeting 3", "type": "internal",
                     "start": "2025-02-10T14:00:00Z", "end": "2025-02-10T15:00:00Z"},
                    {"id": "m4", "title": "Meeting 4", "type": "customer",
                     "start": "2025-02-10T15:30:00Z", "end": "2025-02-10T16:30:00Z"}
                ],
                "Tuesday": [],
                "Wednesday": [],
                "Thursday": [],
                "Friday": []
            },
            "timeBlocks": {"gapsByDay": {}}
        });

        let shapes = build_day_shapes(&directive, data_dir);
        assert_eq!(shapes.len(), 5);

        // Monday: 4 meetings × 60 min = 240 min → "busy"
        assert_eq!(shapes[0]["dayName"], "Monday");
        assert_eq!(shapes[0]["density"], "busy");
        assert_eq!(shapes[0]["meetingCount"], 4);
        assert_eq!(shapes[0]["meetingMinutes"], 240);

        // Tuesday: 0 meetings → "light"
        assert_eq!(shapes[1]["dayName"], "Tuesday");
        assert_eq!(shapes[1]["density"], "light");
        assert_eq!(shapes[1]["meetingCount"], 0);
    }

    #[test]
    fn test_resolve_prep_status_non_eligible() {
        let tmp = TempDir::new().unwrap();
        // I159: internal, team_sync, one_on_one are now person-prep eligible
        assert_eq!(resolve_prep_status("m1", "personal", tmp.path()), "done");
        assert_eq!(resolve_prep_status("m1", "all_hands", tmp.path()), "done");
    }

    #[test]
    fn test_resolve_prep_status_no_file() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join("preps")).unwrap();
        assert_eq!(
            resolve_prep_status("m1", "customer", tmp.path()),
            "prep_needed"
        );
    }

    #[test]
    fn test_resolve_prep_status_with_agenda() {
        let tmp = TempDir::new().unwrap();
        let preps = tmp.path().join("preps");
        std::fs::create_dir_all(&preps).unwrap();
        std::fs::write(
            preps.join("m1.json"),
            r#"{"proposedAgenda": [{"topic": "Review"}]}"#,
        )
        .unwrap();
        assert_eq!(
            resolve_prep_status("m1", "customer", tmp.path()),
            "prep_ready"
        );
    }

    #[test]
    fn test_resolve_prep_status_minimal_file() {
        let tmp = TempDir::new().unwrap();
        let preps = tmp.path().join("preps");
        std::fs::create_dir_all(&preps).unwrap();
        std::fs::write(preps.join("m1.json"), r#"{"title": "Meeting"}"#).unwrap();
        assert_eq!(
            resolve_prep_status("m1", "customer", tmp.path()),
            "context_needed"
        );
    }

    #[test]
    fn test_compute_meeting_duration() {
        assert_eq!(
            compute_meeting_duration("2025-02-10T09:00:00Z", "2025-02-10T10:00:00Z"),
            60
        );
        assert_eq!(
            compute_meeting_duration("2025-02-10T14:00:00Z", "2025-02-10T14:30:00Z"),
            30
        );
        // Fallback for empty
        assert_eq!(compute_meeting_duration("", ""), 30);
    }

    #[test]
    fn test_action_summary_includes_items() {
        let tmp = TempDir::new().unwrap();
        let directive = json!({
            "actions": {
                "overdue": [
                    {"title": "Overdue Task", "account": "Acme", "due_date": "2025-01-01", "priority": "P1"}
                ],
                "thisWeek": [
                    {"title": "Week Task", "account": "Globex", "due_date": "2025-02-12", "priority": "P2"}
                ]
            }
        });

        let summary = build_action_summary(&directive, tmp.path());
        assert_eq!(summary["overdueCount"], 1);
        assert_eq!(summary["dueThisWeek"], 1);

        let overdue = summary["overdue"].as_array().unwrap();
        assert_eq!(overdue.len(), 1);
        assert_eq!(overdue[0]["title"], "Overdue Task");
        assert_eq!(overdue[0]["priority"], "P1");

        let due = summary["dueThisWeekItems"].as_array().unwrap();
        assert_eq!(due.len(), 1);
        assert_eq!(due[0]["title"], "Week Task");
    }

    #[test]
    fn test_hygiene_alerts_yellow_health() {
        let directive = json!({
            "meetingContexts": [
                {
                    "account": "Globex Industries",
                    "account_data": {"lifecycle": "renewal", "arr": 800000, "health": "yellow"},
                    "narrative": "Team B usage declining."
                }
            ]
        });
        let alerts = build_hygiene_alerts(&directive);
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0]["account"], "Globex Industries");
        assert_eq!(alerts[0]["severity"], "warning");
        assert_eq!(alerts[0]["arr"], "$800K");
        assert!(alerts[0]["issue"].as_str().unwrap().contains("YELLOW"));
    }

    #[test]
    fn test_hygiene_alerts_red_health() {
        let directive = json!({
            "meetingContexts": [
                {
                    "account": "BadCo",
                    "account_data": {"lifecycle": "churned", "arr": 50000, "health": "red"},
                    "narrative": "Account lost."
                }
            ]
        });
        let alerts = build_hygiene_alerts(&directive);
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0]["severity"], "critical");
    }

    #[test]
    fn test_hygiene_alerts_green_skipped() {
        let directive = json!({
            "meetingContexts": [
                {
                    "account": "Acme Corp",
                    "account_data": {"lifecycle": "steady-state", "arr": 1200000, "health": "green"},
                    "narrative": "All good."
                }
            ]
        });
        let alerts = build_hygiene_alerts(&directive);
        assert!(alerts.is_empty());
    }

    #[test]
    fn test_hygiene_alerts_deduplicates_by_account() {
        let directive = json!({
            "meetingContexts": [
                {
                    "account": "Globex",
                    "account_data": {"health": "yellow"},
                    "narrative": "First."
                },
                {
                    "account": "Globex",
                    "account_data": {"health": "yellow"},
                    "narrative": "Second."
                }
            ]
        });
        let alerts = build_hygiene_alerts(&directive);
        assert_eq!(alerts.len(), 1);
    }

    #[test]
    fn test_format_arr_millions() {
        assert_eq!(format_arr(Some(&json!(1200000))), "$1.2M");
        assert_eq!(format_arr(Some(&json!(2000000))), "$2.0M");
    }

    #[test]
    fn test_format_arr_thousands() {
        assert_eq!(format_arr(Some(&json!(800000))), "$800K");
        assert_eq!(format_arr(Some(&json!(350000))), "$350K");
    }

    #[test]
    fn test_format_arr_string_passthrough() {
        assert_eq!(format_arr(Some(&json!("$1.5M"))), "$1.5M");
    }

    #[test]
    fn test_format_arr_none() {
        assert_eq!(format_arr(None), "");
    }

    #[test]
    fn test_build_week_markdown_includes_sections() {
        let overview = json!({
            "weekNumber": "W06",
            "dateRange": "2025-02-10 – 2025-02-14",
            "readinessChecks": [
                {"checkType": "no_prep", "message": "Globex Check-in has no prep", "severity": "action_needed"}
            ],
            "dayShapes": [
                {"dayName": "Monday", "date": "2025-02-10", "meetingCount": 3, "meetingMinutes": 120,
                 "density": "moderate", "meetings": [], "availableBlocks": [
                    {"day": "Monday", "start": "11:00 AM", "end": "1:00 PM", "durationMinutes": 120}
                 ]}
            ],
            "actionSummary": {
                "overdueCount": 1, "dueThisWeek": 1,
                "overdue": [{"title": "Task A", "account": "Acme", "priority": "P1", "daysOverdue": 2}],
                "dueThisWeekItems": [{"title": "Task B", "priority": "P2", "dueDate": "2025-02-12"}]
            },
            "hygieneAlerts": [
                {"account": "Globex", "lifecycle": "at-risk", "arr": "$800K", "issue": "Health declining", "severity": "warning"}
            ]
        });

        let md = build_week_markdown(&overview);
        assert!(md.contains("# Week W06"));
        assert!(md.contains("## Readiness"));
        assert!(md.contains("Globex Check-in has no prep"));
        assert!(md.contains("## Week Shape"));
        assert!(md.contains("| Monday |"));
        assert!(md.contains("## Actions"));
        assert!(md.contains("[P1] Task A"));
        assert!(md.contains("## Account Health"));
        assert!(md.contains("**Globex**"));
    }

    #[test]
    fn test_deliver_week_writes_json_and_md() {
        let tmp = TempDir::new().unwrap();
        let workspace = tmp.path();
        let data_dir = workspace.join("_today").join("data");
        std::fs::create_dir_all(data_dir.join("preps")).unwrap();

        let directive = json!({
            "context": {"weekNumber": "W06", "monday": "2025-02-10", "friday": "2025-02-14",
                        "dateRange": "2025-02-10 – 2025-02-14"},
            "meetingsByDay": {
                "Monday": [{"id": "m1", "title": "Sync", "type": "customer",
                            "start": "2025-02-10T13:00:00Z", "end": "2025-02-10T13:45:00Z"}],
                "Tuesday": [], "Wednesday": [], "Thursday": [], "Friday": []
            },
            "meetingContexts": [],
            "actions": {},
            "timeBlocks": {"gapsByDay": {}}
        });

        std::fs::write(
            data_dir.join("week-directive.json"),
            serde_json::to_string_pretty(&directive).unwrap(),
        )
        .unwrap();

        let result = deliver_week(workspace);
        assert!(result.is_ok(), "deliver_week failed: {:?}", result);
        assert!(data_dir.join("week-overview.json").exists());
        assert!(workspace.join("_today").join("week-overview.md").exists());

        let md =
            std::fs::read_to_string(workspace.join("_today").join("week-overview.md")).unwrap();
        assert!(md.contains("# Week W06"));
    }

    #[test]
    fn test_week_date_anchoring_weekday() {
        // Wednesday 2026-02-11 → Monday of same week = 2026-02-09
        let today = NaiveDate::from_ymd_opt(2026, 2, 11).unwrap();
        let monday = if today.weekday() == chrono::Weekday::Sat {
            today + chrono::Duration::days(2)
        } else if today.weekday() == chrono::Weekday::Sun {
            today + chrono::Duration::days(1)
        } else {
            today - chrono::Duration::days(today.weekday().num_days_from_monday() as i64)
        };
        assert_eq!(monday, NaiveDate::from_ymd_opt(2026, 2, 9).unwrap());
    }

    #[test]
    fn test_week_date_anchoring_saturday() {
        // Saturday 2026-02-14 → NEXT Monday = 2026-02-16
        let today = NaiveDate::from_ymd_opt(2026, 2, 14).unwrap();
        let monday = if today.weekday() == chrono::Weekday::Sat {
            today + chrono::Duration::days(2)
        } else if today.weekday() == chrono::Weekday::Sun {
            today + chrono::Duration::days(1)
        } else {
            today - chrono::Duration::days(today.weekday().num_days_from_monday() as i64)
        };
        assert_eq!(monday, NaiveDate::from_ymd_opt(2026, 2, 16).unwrap());
    }

    #[test]
    fn test_week_date_anchoring_sunday() {
        // Sunday 2026-02-15 → NEXT Monday = 2026-02-16
        let today = NaiveDate::from_ymd_opt(2026, 2, 15).unwrap();
        let monday = if today.weekday() == chrono::Weekday::Sat {
            today + chrono::Duration::days(2)
        } else if today.weekday() == chrono::Weekday::Sun {
            today + chrono::Duration::days(1)
        } else {
            today - chrono::Duration::days(today.weekday().num_days_from_monday() as i64)
        };
        assert_eq!(monday, NaiveDate::from_ymd_opt(2026, 2, 16).unwrap());
    }

    #[test]
    fn test_week_date_anchoring_monday() {
        // Monday 2026-02-09 → same Monday = 2026-02-09
        let today = NaiveDate::from_ymd_opt(2026, 2, 9).unwrap();
        let monday = if today.weekday() == chrono::Weekday::Sat {
            today + chrono::Duration::days(2)
        } else if today.weekday() == chrono::Weekday::Sun {
            today + chrono::Duration::days(1)
        } else {
            today - chrono::Duration::days(today.weekday().num_days_from_monday() as i64)
        };
        assert_eq!(monday, NaiveDate::from_ymd_opt(2026, 2, 9).unwrap());
    }

    // ── I338: queue_person_intelligence tests ────────────────────────────

    fn test_db() -> crate::db::ActionDb {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("test.db");
        std::mem::forget(dir);
        let db = crate::db::ActionDb::open_at(path).expect("open test DB");
        db.conn_ref()
            .execute_batch("PRAGMA foreign_keys = OFF;")
            .expect("disable FK");
        db
    }

    fn person_meeting_context(entity_id: &str, entity_name: &str, confidence: f64) -> Value {
        json!({
            "primary_entity": {
                "entity_id": entity_id,
                "entity_type": "person",
                "entity_name": entity_name,
                "confidence": confidence,
            }
        })
    }

    #[test]
    fn test_queue_person_intelligence_from_contexts() {
        let db = test_db();
        let queue = crate::intel_queue::IntelligenceQueue::new();
        let workspace = TempDir::new().unwrap();

        // Insert person into DB so dashboard can be written
        let person = crate::db::DbPerson {
            id: "person_bob_example_com".to_string(),
            email: "bob@example.com".to_string(),
            name: "Bob Smith".to_string(),
            organization: None,
            role: None,
            relationship: "external".to_string(),
            notes: None,
            tracker_path: None,
            last_seen: None,
            first_seen: None,
            meeting_count: 5,
            updated_at: Utc::now().to_rfc3339(),
            archived: false,
            linkedin_url: None,
            twitter_handle: None,
            phone: None,
            photo_url: None,
            bio: None,
            title_history: None,
            company_industry: None,
            company_size: None,
            company_hq: None,
            last_enriched_at: None,
            enrichment_sources: None,
        };
        let _ = db.upsert_person(&person);

        let contexts = vec![
            person_meeting_context("person_bob_example_com", "Bob Smith", 0.85),
        ];

        let n = queue_person_intelligence(&contexts, workspace.path(), &db, &queue);
        assert_eq!(n, 1, "should enqueue 1 person intelligence request");
        assert_eq!(queue.len(), 1);
    }

    #[test]
    fn test_queue_person_intelligence_skips_low_confidence() {
        let db = test_db();
        let queue = crate::intel_queue::IntelligenceQueue::new();
        let workspace = TempDir::new().unwrap();

        let contexts = vec![
            person_meeting_context("person_low_example_com", "Low Conf", 0.40),
        ];

        let n = queue_person_intelligence(&contexts, workspace.path(), &db, &queue);
        assert_eq!(n, 0, "should skip low-confidence person entities");
        assert!(queue.is_empty());
    }

    #[test]
    fn test_queue_person_intelligence_skips_fresh() {
        let db = test_db();
        let queue = crate::intel_queue::IntelligenceQueue::new();
        let workspace = TempDir::new().unwrap();

        // Create a fresh intelligence.json
        let person_dir = crate::people::person_dir(workspace.path(), "Fresh Person");
        std::fs::create_dir_all(&person_dir).unwrap();
        let intel_json = json!({
            "version": 1,
            "entityId": "person_fresh_example_com",
            "entityType": "person",
            "enrichedAt": Utc::now().to_rfc3339(),
            "executiveAssessment": "Fresh assessment.",
        });
        let intel_str = serde_json::to_string_pretty(&intel_json).unwrap();
        crate::util::atomic_write_str(&person_dir.join("intelligence.json"), &intel_str).unwrap();

        let contexts = vec![
            person_meeting_context("person_fresh_example_com", "Fresh Person", 0.90),
        ];

        let n = queue_person_intelligence(&contexts, workspace.path(), &db, &queue);
        assert_eq!(n, 0, "should skip freshly-enriched person entities");
        assert!(queue.is_empty());
    }
}
