use super::*;
// =============================================================================
// I194: User Agenda + Notes Editability (ADR-0065)
// =============================================================================

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyPrepPrefillResult {
    pub meeting_id: String,
    pub added_agenda_items: usize,
    pub notes_appended: bool,
    pub mode: String,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgendaDraftResult {
    pub meeting_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    pub body: String,
}

fn is_meeting_user_layer_read_only(meeting: &crate::db::DbMeeting) -> bool {
    if meeting.prep_frozen_at.is_some() {
        return true;
    }
    let now = chrono::Utc::now();
    let end_dt = meeting
        .end_time
        .as_deref()
        .and_then(parse_meeting_datetime)
        .or_else(|| {
            parse_meeting_datetime(&meeting.start_time).map(|s| s + chrono::Duration::hours(1))
        });
    // Default to read-only when time can't be parsed — safer than allowing edits
    // on meetings whose temporal state is unknown.
    end_dt.is_none_or(|e| e < now)
}

fn normalized_item_key(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn merge_user_agenda(existing: &[String], incoming: &[String]) -> (Vec<String>, usize) {
    let mut merged = existing.to_vec();
    let mut seen: std::collections::HashSet<String> = existing
        .iter()
        .map(|item| normalized_item_key(item))
        .filter(|k| !k.is_empty())
        .collect();
    let mut added = 0usize;

    for item in incoming {
        let trimmed = item.trim();
        if trimmed.is_empty() {
            continue;
        }
        let key = normalized_item_key(trimmed);
        if key.is_empty() || seen.contains(&key) {
            continue;
        }
        merged.push(trimmed.to_string());
        seen.insert(key);
        added += 1;
    }

    (merged, added)
}

fn merge_user_notes(existing: Option<&str>, notes_append: &str) -> (Option<String>, bool) {
    let append = notes_append.trim();
    if append.is_empty() {
        return (existing.map(|s| s.to_string()), false);
    }

    match existing.map(str::trim).filter(|s| !s.is_empty()) {
        Some(current) if current.contains(append) => (Some(current.to_string()), false),
        Some(current) => (Some(format!("{}\n\n{}", current, append)), true),
        None => (Some(append.to_string()), true),
    }
}

fn apply_meeting_prep_prefill_inner(
    ctx: &crate::services::context::ServiceContext<'_>,
    db: &crate::db::ActionDb,
    engine: &crate::signals::propagation::PropagationEngine,
    meeting_id: &str,
    agenda_items: &[String],
    notes_append: &str,
) -> Result<ApplyPrepPrefillResult, String> {
    let meeting = db
        .get_meeting_intelligence_row(meeting_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Meeting not found: {}", meeting_id))?;

    if is_meeting_user_layer_read_only(&meeting) {
        return Err("Meeting user fields are read-only after freeze/past state".to_string());
    }

    let existing_agenda =
        parse_user_agenda_json(meeting.user_agenda_json.as_deref()).unwrap_or_default();
    let (merged_agenda, added_agenda_items) = merge_user_agenda(&existing_agenda, agenda_items);
    let agenda_json = if merged_agenda.is_empty() {
        None
    } else {
        Some(serde_json::to_string(&merged_agenda).map_err(|e| format!("Serialize error: {}", e))?)
    };

    let (merged_notes, notes_appended) =
        merge_user_notes(meeting.user_notes.as_deref(), notes_append);
    crate::services::mutations::update_meeting_user_layer(
        ctx,
        db,
        engine,
        meeting_id,
        agenda_json.as_deref(),
        merged_notes.as_deref(),
    )?;

    Ok(ApplyPrepPrefillResult {
        meeting_id: meeting_id.to_string(),
        added_agenda_items,
        notes_appended,
        mode: "append_dedupe".to_string(),
    })
}

fn generate_agenda_draft_body(
    title: &str,
    time_range: Option<&str>,
    agenda_items: &[String],
    context_hint: Option<&str>,
    context: Option<&str>,
) -> String {
    let mut body = String::new();
    body.push_str(&format!(
        "Hi all,\n\nAhead of {}, here is a proposed agenda",
        title
    ));
    if let Some(range) = time_range.filter(|value| !value.trim().is_empty()) {
        body.push_str(&format!(" for {}.", range));
    } else {
        body.push('.');
    }
    body.push_str("\n\n");

    if agenda_items.is_empty() {
        body.push_str("1. Confirm priorities and desired outcomes\n");
        body.push_str("2. Review current risks and blockers\n");
        body.push_str("3. Align on owners and next steps\n");
    } else {
        for (idx, item) in agenda_items.iter().enumerate() {
            body.push_str(&format!("{}. {}\n", idx + 1, item));
        }
    }

    if let Some(hint) = context_hint.map(str::trim).filter(|s| !s.is_empty()) {
        body.push_str(&format!("\nAdditional context to cover: {}\n", hint));
    }

    if let Some(summary) = context
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.lines().next().unwrap_or(s).trim())
        .filter(|s| !s.is_empty())
    {
        body.push_str(&format!("\nCurrent context: {}\n", summary));
    }

    body.push_str("\nPlease reply with additions or edits.\n\nThanks");
    body
}

fn build_agenda_draft_result(
    meeting: &crate::db::DbMeeting,
    prep: Option<&FullMeetingPrep>,
    context_hint: Option<&str>,
) -> AgendaDraftResult {
    let mut agenda_items: Vec<String> = Vec::new();
    if let Some(prep) = prep {
        if let Some(ref user_agenda) = prep.user_agenda {
            agenda_items.extend(user_agenda.iter().map(|item| item.trim().to_string()));
        }
        if agenda_items.is_empty() {
            if let Some(ref proposed) = prep.proposed_agenda {
                // Filter out talking_point source items to match frontend "Your Plan" display.
                // Fall back to all items if filtering leaves nothing.
                let non_talking: Vec<String> = proposed
                    .iter()
                    .filter(|item| item.source.as_deref() != Some("talking_point"))
                    .map(|item| item.topic.trim().to_string())
                    .filter(|item| !item.is_empty())
                    .collect();
                if non_talking.is_empty() {
                    agenda_items.extend(
                        proposed
                            .iter()
                            .map(|item| item.topic.trim().to_string())
                            .filter(|item| !item.is_empty()),
                    );
                } else {
                    agenda_items.extend(non_talking);
                }
            }
        }
    }
    agenda_items.retain(|item| !item.is_empty());
    let mut seen = std::collections::HashSet::new();
    agenda_items.retain(|item| seen.insert(normalized_item_key(item)));

    let title = prep
        .map(|p| p.title.as_str())
        .filter(|t| !t.trim().is_empty())
        .unwrap_or(&meeting.title);
    let time_range = prep.map(|p| p.time_range.as_str());
    let context = prep
        .and_then(|p| p.meeting_context.as_deref())
        .or(meeting.summary.as_deref());

    AgendaDraftResult {
        meeting_id: meeting.id.clone(),
        subject: Some(format!("Agenda for {}", title)),
        body: generate_agenda_draft_body(title, time_range, &agenda_items, context_hint, context),
    }
}

/// Apply AI-suggested prep additions in append + dedupe mode.
#[tauri::command]
pub async fn apply_meeting_prep_prefill(
    meeting_id: String,
    agenda_items: Vec<String>,
    notes_append: String,
    state: State<'_, Arc<AppState>>,
) -> Result<ApplyPrepPrefillResult, String> {
    let engine = state.signals.engine.clone();
    let mid = meeting_id.clone();
    let ai = agenda_items.clone();
    let na = notes_append.clone();
    let state_for_ctx = state.inner().clone();
    let result = state
        .db_write(move |db| {
            let ctx = state_for_ctx.live_service_context();
            apply_meeting_prep_prefill_inner(&ctx, db, &engine, &mid, &ai, &na)
        })
        .await?;

    // Mirror write to active prep JSON (best-effort) for immediate UI coherence.
    if let Ok(prep_path) = resolve_prep_path(&meeting_id, &state) {
        if let Ok(content) = std::fs::read_to_string(&prep_path) {
            if let Ok(mut json) = serde_json::from_str::<serde_json::Value>(&content) {
                let existing = json
                    .get("userAgenda")
                    .and_then(|v| v.as_array())
                    .map(|items| {
                        items
                            .iter()
                            .filter_map(|item| item.as_str().map(|s| s.to_string()))
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                let (merged_agenda, _) = merge_user_agenda(&existing, &agenda_items);
                if let Some(obj) = json.as_object_mut() {
                    if merged_agenda.is_empty() {
                        obj.remove("userAgenda");
                    } else {
                        obj.insert("userAgenda".to_string(), serde_json::json!(merged_agenda));
                    }
                    let existing_notes = obj.get("userNotes").and_then(|v| v.as_str());
                    let (merged_notes, _) = merge_user_notes(existing_notes, &notes_append);
                    if let Some(notes) = merged_notes {
                        obj.insert("userNotes".to_string(), serde_json::json!(notes));
                    }
                }
                if let Ok(updated) = serde_json::to_string_pretty(&json) {
                    let _ = std::fs::write(&prep_path, updated);
                }
            }
        }
    }

    Ok(result)
}

/// Generate a draft agenda message from current prep context.
#[tauri::command]
pub async fn generate_meeting_agenda_message_draft(
    meeting_id: String,
    context_hint: Option<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<AgendaDraftResult, String> {
    let config = state
        .config
        .read()
        .clone()
        .ok_or("No configuration loaded")?;
    let workspace_path = config.workspace_path.clone();

    state
        .db_read(move |db| {
            let workspace = Path::new(&workspace_path);
            let today_dir = workspace.join("_today");
            let meeting = db
                .get_meeting_intelligence_row(&meeting_id)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| format!("Meeting not found: {}", meeting_id))?;
            let prep = load_meeting_prep_from_sources(&today_dir, &meeting);

            Ok(build_agenda_draft_result(
                &meeting,
                prep.as_ref(),
                context_hint.as_deref(),
            ))
        })
        .await
}

/// Update user-authored agenda items on a meeting prep file.
#[tauri::command]
pub async fn update_meeting_user_agenda(
    meeting_id: String,
    agenda: Option<Vec<String>>,
    dismissed_topics: Option<Vec<String>>,
    hidden_attendees: Option<Vec<String>>,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let app_state = state.inner().clone();
    state
        .db_write(move |db| {
            crate::services::meetings::update_meeting_user_agenda(
                db,
                &app_state,
                &meeting_id,
                agenda,
                dismissed_topics,
                hidden_attendees,
            )
        })
        .await
}

/// Update user-authored notes on a meeting prep file.
#[tauri::command]
pub async fn update_meeting_user_notes(
    meeting_id: String,
    notes: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let app_state = state.inner().clone();
    state
        .db_write(move |db| {
            crate::services::meetings::update_meeting_user_notes(
                db,
                &app_state,
                &meeting_id,
                &notes,
            )
        })
        .await
}

/// Update a single field in a meeting's frozen prep JSON (user correction).
#[tauri::command]
pub async fn update_meeting_prep_field(
    meeting_id: String,
    field_path: String,
    value: String,
    target_person_id: Option<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let app_state = state.inner().clone();
    state
        .db_write(move |db| {
            crate::services::meetings::update_meeting_prep_field(
                db,
                &app_state,
                &meeting_id,
                &field_path,
                &value,
                target_person_id.as_deref(),
            )
        })
        .await
}

/// Resolve the on-disk path for a meeting's prep JSON file.
fn resolve_prep_path(meeting_id: &str, state: &AppState) -> Result<std::path::PathBuf, String> {
    let config = state
        .config
        .read()
        .clone()
        .ok_or("No configuration loaded")?;

    let workspace = Path::new(&config.workspace_path);
    let preps_dir = workspace.join("_today").join("data").join("preps");
    let clean_id = meeting_id.trim_end_matches(".json").trim_end_matches(".md");
    let path = preps_dir.join(format!("{}.json", clean_id));

    // Path containment check: prevent traversal outside preps directory
    if !path.starts_with(&preps_dir) {
        return Err("Invalid meeting ID".to_string());
    }

    if path.exists() {
        Ok(path)
    } else {
        Err(format!("Prep file not found: {}", path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{ActionDb, DbMeeting};
    use crate::services::context::{ExternalClients, FixedClock, SeedableRng, ServiceContext};
    use chrono::{TimeZone, Utc};
    use serde_json::json;
    use tempfile::tempdir;

    fn test_ctx<'a>(
        clock: &'a FixedClock,
        rng: &'a SeedableRng,
        ext: &'a ExternalClients,
    ) -> ServiceContext<'a> {
        ServiceContext::test_live(clock, rng, ext)
    }

    #[test]
    fn test_backfill_prep_semantics_value_derives_recent_wins_and_sources() {
        let mut prep = json!({
            "talkingPoints": [
                "Recent win: Sponsor re-engaged _(source: 2026-02-11-sync.md)_",
                "Win: Tier upgrade requested"
            ]
        });

        let changed = backfill_prep_semantics_value(&mut prep);
        assert!(changed);
        assert_eq!(prep["recentWins"][0], "Sponsor re-engaged");
        assert_eq!(prep["recentWins"][1], "Tier upgrade requested");
        assert_eq!(prep["recentWinSources"][0]["label"], "2026-02-11-sync.md");
        assert_eq!(prep["talkingPoints"][0], "Recent win: Sponsor re-engaged");
    }

    #[test]
    fn test_backfill_prep_files_in_dir_dry_run_counts() {
        let dir = tempdir().expect("tempdir");
        let preps_dir = dir.path().join("preps");
        fs::create_dir_all(&preps_dir).expect("create preps dir");

        fs::write(
            preps_dir.join("needs-backfill.json"),
            serde_json::to_string_pretty(&json!({
                "talkingPoints": ["Recent win: Sponsor re-engaged (source: notes.md)"]
            }))
            .unwrap(),
        )
        .unwrap();
        fs::write(
            preps_dir.join("already-new.json"),
            serde_json::to_string_pretty(&json!({
                "recentWins": ["Sponsor re-engaged"],
                "recentWinSources": [{"label": "notes.md", "path": "notes.md"}]
            }))
            .unwrap(),
        )
        .unwrap();
        fs::write(preps_dir.join("bad.json"), "{").unwrap();

        let counts = backfill_prep_files_in_dir(&preps_dir, true).expect("dry-run should succeed");
        assert_eq!(counts.candidate, 3);
        assert_eq!(counts.transformed, 1);
        assert_eq!(counts.skipped, 1);
        assert_eq!(counts.parse_errors, 1);

        let unchanged = fs::read_to_string(preps_dir.join("needs-backfill.json")).unwrap();
        assert!(unchanged.contains("talkingPoints"));
        assert!(!unchanged.contains("recentWins"));
    }

    #[test]
    fn test_backfill_prep_files_in_dir_apply_updates_file() {
        let dir = tempdir().expect("tempdir");
        let preps_dir = dir.path().join("preps");
        fs::create_dir_all(&preps_dir).expect("create preps dir");
        let path = preps_dir.join("meeting.json");
        fs::write(
            &path,
            serde_json::to_string_pretty(&json!({
                "talkingPoints": ["Recent win: Expansion approved (source: expansion.md)"]
            }))
            .unwrap(),
        )
        .unwrap();

        let counts = backfill_prep_files_in_dir(&preps_dir, false).expect("apply should succeed");
        assert_eq!(counts.transformed, 1);

        let updated: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(updated["recentWins"][0], "Expansion approved");
        assert_eq!(updated["recentWinSources"][0]["label"], "expansion.md");
    }

    #[test]
    fn test_backfill_db_prep_contexts_apply_updates_rows() {
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("test.db");
        let db = ActionDb::open_at_unencrypted(db_path).expect("open db");

        let meeting = DbMeeting {
            id: "mtg-1".to_string(),
            title: "Test Meeting".to_string(),
            meeting_type: "customer".to_string(),
            start_time: Utc::now().to_rfc3339(),
            end_time: None,
            attendees: None,
            notes_path: None,
            summary: None,
            created_at: Utc::now().to_rfc3339(),
            calendar_event_id: None,
            description: None,
            prep_context_json: Some(
                serde_json::to_string(&json!({
                    "talkingPoints": ["Recent win: Champion re-engaged (source: call.md)"]
                }))
                .unwrap(),
            ),
            user_agenda_json: None,
            user_notes: None,
            prep_frozen_json: None,
            prep_frozen_at: None,
            prep_snapshot_path: None,
            prep_snapshot_hash: None,
            transcript_path: None,
            transcript_processed_at: None,
            intelligence_state: None,
            intelligence_quality: None,
            last_enriched_at: None,
            signal_count: None,
            has_new_signals: None,
            last_viewed_at: None,
        };
        db.upsert_meeting(&meeting).expect("insert meeting");
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 4, 30, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);

        let dry_counts = backfill_db_prep_contexts(&ctx, &db, true).expect("dry-run");
        assert_eq!(dry_counts.candidate, 1);
        assert_eq!(dry_counts.transformed, 1);

        let before = db
            .get_meeting_by_id("mtg-1")
            .expect("meeting lookup")
            .expect("meeting exists")
            .prep_context_json
            .unwrap();
        assert!(!before.contains("recentWins"));

        let apply_counts = backfill_db_prep_contexts(&ctx, &db, false).expect("apply");
        assert_eq!(apply_counts.candidate, 1);
        assert_eq!(apply_counts.transformed, 1);

        let after = db
            .get_meeting_by_id("mtg-1")
            .expect("meeting lookup")
            .expect("meeting exists")
            .prep_context_json
            .unwrap();
        assert!(after.contains("recentWins"));
        assert!(after.contains("recentWinSources"));
    }

    #[test]
    fn test_apply_meeting_prep_prefill_additive_and_idempotent() {
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("test.db");
        let db = ActionDb::open_at_unencrypted(db_path).expect("open db");

        let meeting = DbMeeting {
            id: "mtg-prefill".to_string(),
            title: "Prefill Test".to_string(),
            meeting_type: "customer".to_string(),
            start_time: (Utc::now() + chrono::Duration::hours(2)).to_rfc3339(),
            end_time: Some((Utc::now() + chrono::Duration::hours(3)).to_rfc3339()),
            attendees: None,
            notes_path: None,
            summary: Some("Context summary".to_string()),
            created_at: Utc::now().to_rfc3339(),
            calendar_event_id: None,
            description: None,
            prep_context_json: None,
            user_agenda_json: None,
            user_notes: None,
            prep_frozen_json: None,
            prep_frozen_at: None,
            prep_snapshot_path: None,
            prep_snapshot_hash: None,
            transcript_path: None,
            transcript_processed_at: None,
            intelligence_state: None,
            intelligence_quality: None,
            last_enriched_at: None,
            signal_count: None,
            has_new_signals: None,
            last_viewed_at: None,
        };
        db.upsert_meeting(&meeting).expect("upsert meeting");
        let engine = crate::signals::propagation::PropagationEngine::new();
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 4, 30, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);

        let first = apply_meeting_prep_prefill_inner(
            &ctx,
            &db,
            &engine,
            "mtg-prefill",
            &["Confirm blockers".to_string(), "Agree owners".to_string()],
            "Bring latest renewal risk updates.",
        )
        .expect("first prefill");
        assert_eq!(first.added_agenda_items, 2);
        assert!(first.notes_appended);

        let second = apply_meeting_prep_prefill_inner(
            &ctx,
            &db,
            &engine,
            "mtg-prefill",
            &["Confirm blockers".to_string(), "Agree owners".to_string()],
            "Bring latest renewal risk updates.",
        )
        .expect("second prefill");
        assert_eq!(second.added_agenda_items, 0);
        assert!(!second.notes_appended);

        let updated = db
            .get_meeting_intelligence_row("mtg-prefill")
            .expect("load meeting")
            .expect("meeting exists");
        let agenda =
            parse_user_agenda_json(updated.user_agenda_json.as_deref()).unwrap_or_default();
        assert_eq!(agenda.len(), 2);
        assert!(updated
            .user_notes
            .unwrap_or_default()
            .contains("renewal risk updates"));
    }

    #[test]
    fn test_apply_meeting_prep_prefill_blocks_past_or_frozen() {
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("test.db");
        let db = ActionDb::open_at_unencrypted(db_path).expect("open db");

        let past = DbMeeting {
            id: "mtg-past".to_string(),
            title: "Past Meeting".to_string(),
            meeting_type: "customer".to_string(),
            start_time: (Utc::now() - chrono::Duration::hours(4)).to_rfc3339(),
            end_time: Some((Utc::now() - chrono::Duration::hours(3)).to_rfc3339()),
            attendees: None,
            notes_path: None,
            summary: None,
            created_at: Utc::now().to_rfc3339(),
            calendar_event_id: None,
            description: None,
            prep_context_json: None,
            user_agenda_json: None,
            user_notes: None,
            prep_frozen_json: None,
            prep_frozen_at: None,
            prep_snapshot_path: None,
            prep_snapshot_hash: None,
            transcript_path: None,
            transcript_processed_at: None,
            intelligence_state: None,
            intelligence_quality: None,
            last_enriched_at: None,
            signal_count: None,
            has_new_signals: None,
            last_viewed_at: None,
        };
        db.upsert_meeting(&past).expect("upsert past meeting");
        let engine = crate::signals::propagation::PropagationEngine::new();
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 4, 30, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);

        let err = apply_meeting_prep_prefill_inner(
            &ctx,
            &db,
            &engine,
            "mtg-past",
            &["Item".to_string()],
            "notes",
        )
        .expect_err("past meeting should be read-only");
        assert!(err.contains("read-only"));

        let frozen = DbMeeting {
            id: "mtg-frozen".to_string(),
            title: "Frozen Meeting".to_string(),
            meeting_type: "customer".to_string(),
            start_time: (Utc::now() + chrono::Duration::hours(2)).to_rfc3339(),
            end_time: Some((Utc::now() + chrono::Duration::hours(3)).to_rfc3339()),
            attendees: None,
            notes_path: None,
            summary: None,
            created_at: Utc::now().to_rfc3339(),
            calendar_event_id: None,
            description: None,
            prep_context_json: None,
            user_agenda_json: None,
            user_notes: None,
            prep_frozen_json: Some("{}".to_string()),
            prep_frozen_at: Some(Utc::now().to_rfc3339()),
            prep_snapshot_path: None,
            prep_snapshot_hash: None,
            transcript_path: None,
            transcript_processed_at: None,
            intelligence_state: None,
            intelligence_quality: None,
            last_enriched_at: None,
            signal_count: None,
            has_new_signals: None,
            last_viewed_at: None,
        };
        db.upsert_meeting(&frozen).expect("upsert frozen meeting");

        let frozen_err = apply_meeting_prep_prefill_inner(
            &ctx,
            &db,
            &engine,
            "mtg-frozen",
            &["Item".to_string()],
            "notes",
        )
        .expect_err("frozen meeting should be read-only");
        assert!(frozen_err.contains("read-only"));
    }

    #[test]
    fn test_generate_meeting_agenda_message_draft_deterministic_structure() {
        let meeting = DbMeeting {
            id: "mtg-draft".to_string(),
            title: "Acme Weekly".to_string(),
            meeting_type: "customer".to_string(),
            start_time: Utc::now().to_rfc3339(),
            end_time: None,
            attendees: None,
            notes_path: None,
            summary: Some("Renewal risk still elevated.".to_string()),
            created_at: Utc::now().to_rfc3339(),
            calendar_event_id: None,
            description: None,
            prep_context_json: None,
            user_agenda_json: None,
            user_notes: None,
            prep_frozen_json: None,
            prep_frozen_at: None,
            prep_snapshot_path: None,
            prep_snapshot_hash: None,
            transcript_path: None,
            transcript_processed_at: None,
            intelligence_state: None,
            intelligence_quality: None,
            last_enriched_at: None,
            signal_count: None,
            has_new_signals: None,
            last_viewed_at: None,
        };

        let prep = FullMeetingPrep {
            file_path: "preps/mtg-draft.json".to_string(),
            calendar_event_id: None,
            title: "Acme Weekly".to_string(),
            time_range: "Tuesday 2:00 PM".to_string(),
            meeting_context: Some("Renewal risk remains high.".to_string()),
            calendar_notes: None,
            account_snapshot: None,
            quick_context: None,
            user_agenda: None,
            user_notes: None,
            attendees: None,
            since_last: None,
            strategic_programs: None,
            current_state: None,
            open_items: None,
            risks: None,
            talking_points: None,
            recent_wins: None,
            recent_win_sources: None,
            questions: None,
            key_principles: None,
            references: None,
            raw_markdown: None,
            stakeholder_signals: None,
            attendee_context: None,
            proposed_agenda: Some(vec![
                crate::types::AgendaItem {
                    topic: "Align on renewal path".to_string(),
                    why: None,
                    source: None,
                },
                crate::types::AgendaItem {
                    topic: "Confirm owner handoffs".to_string(),
                    why: None,
                    source: None,
                },
            ]),
            intelligence_summary: None,
            entity_risks: None,
            entity_readiness: None,
            stakeholder_insights: None,
            recent_email_signals: None,
            email_digest: None,
            consistency_status: None,
            consistency_findings: Vec::new(),
        };

        let draft = build_agenda_draft_result(&meeting, Some(&prep), Some("Cover timeline risks"));
        assert_eq!(draft.subject.as_deref(), Some("Agenda for Acme Weekly"));
        assert!(draft.body.contains("1. Align on renewal path"));
        assert!(draft.body.contains("2. Confirm owner handoffs"));
        assert!(draft.body.contains("Cover timeline risks"));
        assert!(draft.body.contains("Please reply with additions or edits."));
    }
}

// ==================== Backfill ====================

/// Backfill historical meetings from filesystem into database.
///
/// Scans account/project directories for meeting files (transcripts, notes, summaries)
/// and creates database records + entity links for meetings not already in the system.
///
/// Returns (meetings_created, meetings_skipped, errors).
#[tauri::command]
pub async fn backfill_historical_meetings(
    state: State<'_, Arc<AppState>>,
) -> Result<(usize, usize, Vec<String>), String> {
    let config = state
        .config
        .read()
        .clone()
        .ok_or("Config not initialized")?;

    state
        .db_write(move |db| crate::backfill_meetings::backfill_historical_meetings(db, &config))
        .await
}

// ==================== Domain Backfill ====================

/// I660: Backfill account_domains from historical meeting→account links.
///
/// Walks all meeting_entities where entity_type='account', extracts attendee
/// email domains, and merges them into account_domains. This populates the
/// domain data that entity resolution and transcript routing depend on.
#[tauri::command]
pub async fn backfill_account_domains(
    state: State<'_, Arc<AppState>>,
) -> Result<(usize, usize, Vec<String>), String> {
    let user_domains = state
        .config
        .read()
        .as_ref()
        .map(|c| c.resolved_user_domains())
        .unwrap_or_default();

    state
        .db_write(move |db| {
            let pairs = db
                .get_account_meetings_for_domain_backfill()
                .map_err(|e| format!("Failed to query meetings: {e}"))?;

            let mut accounts_populated: std::collections::HashSet<String> =
                std::collections::HashSet::new();
            let mut domains_added: usize = 0;
            let mut errors: Vec<String> = Vec::new();

            for (account_id, attendees_raw) in &pairs {
                let attendees: Vec<String> =
                    if let Ok(arr) = serde_json::from_str::<Vec<String>>(attendees_raw) {
                        arr
                    } else {
                        attendees_raw
                            .split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect()
                    };

                let discovered = crate::signals::event_trigger::extract_domains_from_attendees(
                    &attendees,
                    &user_domains,
                );

                if discovered.is_empty() {
                    continue;
                }

                match db.merge_account_domains(account_id, &discovered) {
                    Ok(()) => {
                        domains_added += discovered.len();
                        accounts_populated.insert(account_id.clone());
                    }
                    Err(e) => {
                        errors.push(format!("Account {}: {}", account_id, e));
                    }
                }
            }

            log::info!(
                "I660 backfill: populated {} accounts with {} domains ({} errors)",
                accounts_populated.len(),
                domains_added,
                errors.len()
            );

            Ok((accounts_populated.len(), domains_added, errors))
        })
        .await
}

// ==================== Archive Recovery ====================

/// I662: Re-route stranded transcripts and meeting records from _archive/
/// to their correct entity directories.
#[tauri::command]
pub async fn recover_archived_transcripts(
    state: State<'_, Arc<AppState>>,
) -> Result<crate::workflow::recover::RecoveryReport, String> {
    let config = state
        .config
        .read()
        .clone()
        .ok_or("Config not initialized")?;

    let workspace = std::path::PathBuf::from(&config.workspace_path);
    let user_domains = config.resolved_user_domains();

    state
        .db_write(move |db| {
            crate::workflow::recover::recover_archived_transcripts(&workspace, db, &user_domains)
        })
        .await
}

// ==================== Risk Briefing ====================

/// Generate a strategic risk briefing for an account via AI.
/// All blocking work (DB lock + file I/O + PTY) runs in spawn_blocking
/// so the async runtime stays responsive and the UI can render the
/// progress page without beachballing.
#[tauri::command]
pub async fn generate_risk_briefing(
    state: State<'_, Arc<AppState>>,
    app_handle: tauri::AppHandle,
    account_id: String,
) -> Result<crate::types::RiskBriefing, String> {
    crate::services::intelligence::generate_risk_briefing(
        state.inner(),
        &account_id,
        Some(app_handle),
    )
    .await
}

/// Read a cached risk briefing for an account (fast, no AI).
#[tauri::command]
pub async fn get_risk_briefing(
    state: State<'_, Arc<AppState>>,
    account_id: String,
) -> Result<crate::types::RiskBriefing, String> {
    let app_state = state.inner().clone();
    state
        .db_read(move |db| {
            crate::services::intelligence::get_risk_briefing(db, &app_state, &account_id)
        })
        .await
}

// =============================================================================
// Reports (v0.15.0 — I397)
// =============================================================================

/// Generate a report for an entity (async, PTY enrichment).
/// I547: AppHandle passed through for BoB progressive event emission.
#[tauri::command]
pub async fn generate_report(
    state: State<'_, Arc<AppState>>,
    app_handle: tauri::AppHandle,
    entity_id: String,
    entity_type: String,
    report_type: String,
    spotlight_account_ids: Option<Vec<String>>,
) -> Result<crate::reports::ReportRow, String> {
    crate::services::reports::generate_report(
        state.inner(),
        &entity_id,
        &entity_type,
        &report_type,
        spotlight_account_ids.as_deref(),
        Some(app_handle),
    )
    .await
}

/// Read a cached report (fast, no AI).
#[tauri::command]
pub async fn get_report(
    state: State<'_, Arc<AppState>>,
    entity_id: String,
    entity_type: String,
    report_type: String,
) -> Result<Option<crate::reports::ReportRow>, String> {
    state
        .db_read(move |db| {
            crate::services::reports::get_report_cached(db, &entity_id, &entity_type, &report_type)
        })
        .await
}

/// Save user edits to a report (persists content_json back to DB).
#[tauri::command]
pub async fn save_report(
    state: State<'_, Arc<AppState>>,
    entity_id: String,
    entity_type: String,
    report_type: String,
    content_json: String,
) -> Result<(), String> {
    state
        .db_write(move |db| {
            crate::services::reports::save_report(
                db,
                &entity_id,
                &entity_type,
                &report_type,
                &content_json,
            )
        })
        .await
}

/// Fetch all reports for an entity.
#[tauri::command]
pub async fn get_reports_for_entity(
    state: State<'_, Arc<AppState>>,
    entity_id: String,
    entity_type: String,
) -> Result<Vec<crate::reports::ReportRow>, String> {
    state
        .db_read(move |db| {
            crate::services::reports::get_all_reports_for_entity(db, &entity_id, &entity_type)
        })
        .await
}
