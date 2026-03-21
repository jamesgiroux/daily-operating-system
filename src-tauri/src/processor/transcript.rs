//! Meeting-scoped transcript processing (I44 / ADR-0044).
//!
//! Processes a transcript file with full meeting context, extracting outcomes
//! (summary, wins, risks, decisions, actions) and routing the file to its
//! proper workspace location.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

use crate::db::{ActionDb, DbProcessingLog};
use crate::pty::{ModelTier, PtyManager};
use crate::types::AiModelConfig;
use crate::types::{
    CalendarEvent, CapturedAction, ChampionHealth, CompetitorMention, EngagementSignals,
    EscalationSignal, InteractionDynamics, MeetingType, RoleChange, SpeakerSentiment,
    TranscriptCommitment, TranscriptResult, TranscriptSentiment,
};
use crate::util::{
    encode_high_risk_field, sanitize_external_field, wrap_user_data, INJECTION_PREAMBLE,
};

use super::enrich::parse_enrichment_response;
use super::hooks;

/// Per-phase timeout for phased transcript processing (60s each, 3 phases = 180s max)
const TRANSCRIPT_PHASE_TIMEOUT_SECS: u64 = 60;

/// Maximum transcript content sent to AI (covers ~75 min calls).
const TRANSCRIPT_MAX_CHARS: usize = 60_000;

/// Head portion kept for tail-biased truncation (attendee context, meeting opening).
const TRANSCRIPT_HEAD_KEEP: usize = 3_000;
/// Timeout for the post-extraction role-attribution review pass.
const TRANSCRIPT_ROLE_REVIEW_TIMEOUT_SECS: u64 = 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TranscriptContentKind {
    Transcript,
    Notes,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TranscriptProgressPayload {
    meeting_id: String,
    phase: String,
    completed: u32,
    total: u32,
    summary_ready: bool,
    outcomes_ready: bool,
    post_intel_ready: bool,
    actions_count: usize,
    wins_count: usize,
    risks_count: usize,
    decisions_count: usize,
    commitments_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AttendeeRoleHint {
    name: String,
    email: String,
    side: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TranscriptRoleReviewPayload {
    summary: String,
    #[serde(default)]
    discussion: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    analysis: Option<String>,
    #[serde(default)]
    wins: Vec<String>,
    #[serde(default)]
    risks: Vec<String>,
    #[serde(default)]
    decisions: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    champion_health: Option<ChampionHealth>,
}

#[allow(clippy::too_many_arguments)]
fn emit_transcript_progress(
    app_handle: Option<&AppHandle>,
    meeting_id: &str,
    phase: &str,
    completed: u32,
    summary_ready: bool,
    outcomes_ready: bool,
    post_intel_ready: bool,
    actions_count: usize,
    wins_count: usize,
    risks_count: usize,
    decisions_count: usize,
    commitments_count: usize,
) {
    if let Some(app_handle) = app_handle {
        let _ = app_handle.emit(
            "transcript-progress",
            TranscriptProgressPayload {
                meeting_id: meeting_id.to_string(),
                phase: phase.to_string(),
                completed,
                total: 3,
                summary_ready,
                outcomes_ready,
                post_intel_ready,
                actions_count,
                wins_count,
                risks_count,
                decisions_count,
                commitments_count,
            },
        );
    }
}

/// Process a transcript file with meeting context.
///
/// 1. Read the source file
/// 2. Route to account dir or archive with YAML frontmatter
/// 3. Send to Claude for extraction with meeting context
/// 4. Store outcomes (wins/risks/decisions as captures, actions to SQLite)
/// 5. Run post-enrichment hooks
pub fn process_transcript(
    workspace: &Path,
    file_path: &str,
    meeting: &CalendarEvent,
    app_handle: Option<&AppHandle>,
    db: Option<&ActionDb>,
    profile: &str,
    ai_config: Option<&AiModelConfig>,
) -> TranscriptResult {
    process_transcript_with_kind(
        workspace,
        file_path,
        meeting,
        app_handle,
        db,
        profile,
        ai_config,
        TranscriptContentKind::Transcript,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn process_transcript_with_kind(
    workspace: &Path,
    file_path: &str,
    meeting: &CalendarEvent,
    app_handle: Option<&AppHandle>,
    db: Option<&ActionDb>,
    profile: &str,
    ai_config: Option<&AiModelConfig>,
    content_kind: TranscriptContentKind,
) -> TranscriptResult {
    let source = Path::new(file_path);

    // 1. Read the source file
    let content = match std::fs::read_to_string(source) {
        Ok(c) => c,
        Err(e) => {
            return TranscriptResult {
                status: "error".to_string(),
                message: Some(format!("Failed to read transcript: {}", e)),
                ..TranscriptResult::default()
            };
        }
    };

    // 2. Generate destination path and copy with frontmatter
    let date = meeting.end.format("%Y-%m-%d").to_string();
    let slug = slugify(&meeting.title);
    let dest_filename = format!("{}-{}-transcript.md", date, slug);

    // I631: Priority-ordered routing: account > project > person (1:1) > archive
    let destination = if let Some(ref account) = meeting.account {
        // Validate account exists in DB before routing to its folder
        let account_exists = db
            .and_then(|db| db.get_account_by_name(account).ok().flatten())
            .is_some();
        if account_exists {
            let account_dir = sanitize_account_dir(account);
            workspace
                .join("Accounts")
                .join(&account_dir)
                .join("Call-Transcripts")
                .join(&dest_filename)
        } else {
            log::info!(
                "Account '{}' not found in DB — routing transcript to archive",
                account
            );
            workspace.join("_archive").join(&date).join(&dest_filename)
        }
    } else if let Some(dest) = route_to_project(meeting, db, workspace, &dest_filename) {
        dest
    } else if let Some(dest) = route_to_person(meeting, db, workspace, &dest_filename) {
        dest
    } else {
        workspace.join("_archive").join(&date).join(&dest_filename)
    };

    // Build frontmatter
    let frontmatter = build_frontmatter(meeting, &date);
    let content_with_frontmatter = format!("{}\n{}", frontmatter, content);

    // Create dirs and write
    if let Some(parent) = destination.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            return TranscriptResult {
                status: "error".to_string(),
                message: Some(format!("Failed to create directory: {}", e)),
                ..TranscriptResult::default()
            };
        }
    }

    if let Err(e) = std::fs::write(&destination, &content_with_frontmatter) {
        return TranscriptResult {
            status: "error".to_string(),
            message: Some(format!("Failed to write transcript: {}", e)),
            ..TranscriptResult::default()
        };
    }

    log::info!(
        "Transcript for '{}' written to '{}'",
        meeting.title,
        destination.display()
    );

    // 3. Phased transcript processing (AC 67d)
    //
    // Phase 1: Core extraction (summary + actions + decisions) — ~30s
    // Phase 2: Intelligence extraction (wins + risks + sentiment + champion) — ~30s
    // Phase 3: Deep analysis (dynamics + commitments + role changes) — ~30s
    //
    // Each phase persists immediately. If Phase 1 fails, abort all.
    // If Phase 2 or 3 fails, Phase 1 results are already persisted (partial success).

    let default_config = AiModelConfig::default();
    let effective_config = ai_config.unwrap_or(&default_config);

    // I535 Step 10: Inject Gong call summaries as supplementary context when in Glean mode
    let gong_context = build_gong_pre_context(db, meeting);

    // ── Phase 1: Core extraction (summary, discussion, analysis, actions) ──
    let mut phase1_prompt = build_phase1_prompt(meeting, &content, content_kind);
    if let Some(ref ctx) = gong_context {
        phase1_prompt = format!("{}\n\n{}", ctx, phase1_prompt);
    }
    let pty1 = PtyManager::for_tier(ModelTier::Extraction, effective_config)
        .with_timeout(TRANSCRIPT_PHASE_TIMEOUT_SECS);
    let phase1_output = match pty1.spawn_claude(workspace, &phase1_prompt) {
        Ok(o) => o.stdout,
        Err(e) => {
            log::error!(
                "Phase 1 (core extraction) failed for '{}': {}",
                meeting.title,
                e
            );
            return TranscriptResult {
                status: "success".to_string(),
                summary: None,
                destination: Some(destination.display().to_string()),
                message: Some(format!("Transcript saved but AI extraction failed: {}", e)),
                ..TranscriptResult::default()
            };
        }
    };

    // Audit trail (I297) — Phase 1
    let _ =
        crate::audit::write_audit_entry(workspace, "transcript-p1", &meeting.id, &phase1_output);

    log::info!(
        "Phase 1 output for '{}' ({} bytes): {}",
        meeting.title,
        phase1_output.len(),
        if phase1_output.len() > 500 {
            &phase1_output[..500]
        } else {
            &phase1_output
        }
    );

    // Parse Phase 1
    let parsed_p1 = parse_enrichment_response(&phase1_output);
    let mut summary = parsed_p1.summary.clone();
    let mut discussion = parsed_p1.discussion.clone();
    let mut analysis = parsed_p1.analysis.clone();

    // Clear any prior extraction data on THIS connection to avoid dedup guard
    // rejecting re-inserted actions during reprocessing (WAL visibility issue
    // between db_write and dedicated ActionDb::open connections).
    if let Some(db) = db {
        let _ = db.conn.execute(
            "DELETE FROM actions WHERE source_id = ?1 AND source_type IN ('transcript', 'post_meeting')",
            rusqlite::params![meeting.id],
        );
    }

    // Persist Phase 1: actions to SQLite
    let mut extracted_actions = Vec::new();
    if parsed_p1.actions_text.is_none() {
        log::info!(
            "Phase 1 for '{}': no ACTIONS section found in AI output (may be expected for internal meetings)",
            meeting.title
        );
    }
    if let Some(ref actions_text) = parsed_p1.actions_text {
        log::info!(
            "Phase 1 for '{}': found ACTIONS section ({} bytes, {} lines)",
            meeting.title,
            actions_text.len(),
            actions_text.lines().count()
        );
        if let Some(db) = db {
            extract_transcript_actions(
                actions_text,
                &meeting.id,
                &meeting.title,
                db,
                meeting.account.as_deref(),
            );
        }
        for line in actions_text.lines() {
            let Some(raw) = parse_action_line(line) else {
                continue;
            };
            if !raw.is_empty() {
                let meta = super::metadata::parse_action_metadata(raw);
                extracted_actions.push(CapturedAction {
                    title: meta.clean_title,
                    owner: meta.account.clone(),
                    due_date: meta.due_date,
                    priority: meta.priority,
                    context: meta.context,
                    account: meta.account,
                });
            }
        }
    }

    if let Some(db) = db {
        let processed_at = Utc::now().to_rfc3339();
        let summary_ref = (!summary.trim().is_empty()).then_some(summary.as_str());
        // DIRECT_DB_ALLOWED: Transcript processing persists derived metadata before
        // any user-facing surfaces reload; this write is part of the processor pipeline.
        if let Err(e) = db.update_meeting_transcript_metadata(
            &meeting.id,
            &destination.display().to_string(),
            &processed_at,
            summary_ref,
        ) {
            log::warn!(
                "Failed to persist phase 1 transcript metadata for {}: {}",
                meeting.id,
                e
            );
        }
    }
    emit_transcript_progress(
        app_handle,
        &meeting.id,
        "phase1",
        1,
        !summary.trim().is_empty(),
        false,
        false,
        extracted_actions.len(),
        0,
        0,
        0,
        0,
    );

    // ── Phase 2: Intelligence extraction (wins, risks, decisions, sentiment, champion) ──
    let phase2_prompt = build_phase2_prompt(meeting, &content, content_kind, &summary);
    let pty2 = PtyManager::for_tier(ModelTier::Extraction, effective_config)
        .with_timeout(TRANSCRIPT_PHASE_TIMEOUT_SECS);

    let (mut wins, mut risks, mut decisions, sentiment, mut champion_health) =
        match pty2.spawn_claude(workspace, &phase2_prompt) {
            Ok(o) => {
                let phase2_output = o.stdout;
                let _ = crate::audit::write_audit_entry(
                    workspace,
                    "transcript-p2",
                    &meeting.id,
                    &phase2_output,
                );
                log::info!(
                    "Phase 2 output for '{}' ({} bytes): {}",
                    meeting.title,
                    phase2_output.len(),
                    if phase2_output.len() > 500 {
                        &phase2_output[..500]
                    } else {
                        &phase2_output
                    }
                );

                let parsed_p2 = parse_enrichment_response(&phase2_output);
                let sentiment = parse_sentiment_block(&phase2_output);
                let champion_health = parse_champion_health_block(&phase2_output);
                (
                    parsed_p2.wins,
                    parsed_p2.risks,
                    parsed_p2.decisions,
                    sentiment,
                    champion_health,
                )
            }
            Err(e) => {
                log::warn!(
                "Phase 2 (intelligence extraction) failed for '{}': {} — Phase 1 results preserved",
                meeting.title,
                e
            );
                (Vec::new(), Vec::new(), Vec::new(), None, None)
            }
        };

    // Persist Phase 2: transcript captures + outcomes signal
    if let Some(db) = db {
        let entity_type = meeting
            .linked_entities
            .as_ref()
            .and_then(|e| e.first())
            .map(|e| e.entity_type.as_str())
            .unwrap_or("account");
        let entity_id = meeting
            .linked_entities
            .as_ref()
            .and_then(|e| e.first())
            .map(|e| e.id.as_str())
            .or(meeting.account.as_deref())
            .unwrap_or(&meeting.id);

        if !wins.is_empty() || !risks.is_empty() || !decisions.is_empty() {
            if let Err(e) = crate::services::mutations::persist_transcript_outcomes(
                db,
                entity_type,
                entity_id,
                &meeting.id,
                &meeting.title,
                meeting.account.as_deref(),
                &wins,
                &risks,
                &decisions,
            ) {
                log::warn!(
                    "Failed to persist transcript captures/outcomes signal transactionally: {}",
                    e
                );
            }
        }

        // Persist champion health from Phase 2
        if let Some(ref health) = champion_health {
            let db_health = crate::db::types::ChampionHealthAssessment {
                meeting_id: meeting.id.clone(),
                champion_name: Some(health.champion_name.clone()),
                champion_status: health.champion_status.clone(),
                champion_evidence: health.champion_evidence.clone(),
                champion_risk: health.champion_risk.clone(),
            };
            // DIRECT_DB_ALLOWED: Transcript extraction owns persistence of per-meeting
            // champion health artifacts before downstream signal propagation runs.
            if let Err(e) = db.upsert_champion_health(&meeting.id, &db_health) {
                log::warn!(
                    "Failed to persist champion health for {}: {}",
                    meeting.id,
                    e
                );
            }
        }
    }
    emit_transcript_progress(
        app_handle,
        &meeting.id,
        "phase2",
        2,
        !summary.trim().is_empty(),
        true,
        false,
        extracted_actions.len(),
        wins.len(),
        risks.len(),
        decisions.len(),
        0,
    );

    // ── Phase 3: Deep analysis (dynamics, commitments, role changes) ──
    let phase3_prompt = build_phase3_prompt(meeting, &content, content_kind, &summary);
    let pty3 = PtyManager::for_tier(ModelTier::Extraction, effective_config)
        .with_timeout(TRANSCRIPT_PHASE_TIMEOUT_SECS);

    let (interaction_dynamics, role_changes, commitments) =
        match pty3.spawn_claude(workspace, &phase3_prompt) {
            Ok(o) => {
                let phase3_output = o.stdout;
                let _ = crate::audit::write_audit_entry(
                    workspace,
                    "transcript-p3",
                    &meeting.id,
                    &phase3_output,
                );
                log::info!(
                    "Phase 3 output for '{}' ({} bytes): {}",
                    meeting.title,
                    phase3_output.len(),
                    if phase3_output.len() > 500 {
                        &phase3_output[..500]
                    } else {
                        &phase3_output
                    }
                );

                let interaction_dynamics = parse_interaction_dynamics(&phase3_output);
                let role_changes = parse_role_changes_block(&phase3_output);
                let commitments = parse_commitments_block(&phase3_output);
                (interaction_dynamics, role_changes, commitments)
            }
            Err(e) => {
                log::warn!(
                    "Phase 3 (deep analysis) failed for '{}': {} — Phase 1/2 results preserved",
                    meeting.title,
                    e
                );
                (None, Vec::new(), Vec::new())
            }
        };

    if let Some(reviewed) = review_transcript_role_attribution(
        workspace,
        meeting,
        db,
        effective_config,
        &summary,
        &discussion,
        analysis.as_deref(),
        &wins,
        &risks,
        &decisions,
        champion_health.as_ref(),
    ) {
        summary = reviewed.summary;
        discussion = reviewed.discussion;
        analysis = reviewed.analysis;
        wins = reviewed.wins;
        risks = reviewed.risks;
        decisions = reviewed.decisions;
        champion_health = reviewed.champion_health;

        if let Some(db) = db {
            let processed_at = Utc::now().to_rfc3339();
            let summary_ref = (!summary.trim().is_empty()).then_some(summary.as_str());
            if let Err(e) = crate::services::mutations::persist_transcript_metadata(
                db,
                &meeting.id,
                &destination.display().to_string(),
                &processed_at,
                summary_ref,
            ) {
                log::warn!(
                    "Failed to persist reviewed transcript summary for {}: {}",
                    meeting.id,
                    e
                );
            }
            let mut captures = Vec::new();
            for win in &wins {
                let (content, sub_type, evidence_quote) = parse_reviewed_win_metadata(win);
                captures.push(crate::services::mutations::ParsedCapture {
                    capture_type: "win",
                    content,
                    sub_type,
                    urgency: None,
                    evidence_quote,
                });
            }
            for risk in &risks {
                let (content, urgency, evidence_quote) = parse_reviewed_risk_metadata(risk);
                captures.push(crate::services::mutations::ParsedCapture {
                    capture_type: "risk",
                    content,
                    sub_type: None,
                    urgency,
                    evidence_quote,
                });
            }
            for decision in &decisions {
                let (content, evidence_quote) = parse_reviewed_evidence_quote(decision);
                captures.push(crate::services::mutations::ParsedCapture {
                    capture_type: "decision",
                    content,
                    sub_type: None,
                    urgency: None,
                    evidence_quote,
                });
            }
            if let Err(e) = crate::services::mutations::replace_transcript_outcome_captures(
                db,
                &meeting.id,
                &meeting.title,
                meeting.account.as_deref(),
                &captures,
            ) {
                log::warn!(
                    "Failed to persist reviewed transcript outcomes for {}: {}",
                    meeting.id,
                    e
                );
            }
            if let Some(ref health) = champion_health {
                let db_health = crate::db::types::ChampionHealthAssessment {
                    meeting_id: meeting.id.clone(),
                    champion_name: Some(health.champion_name.clone()),
                    champion_status: health.champion_status.clone(),
                    champion_evidence: health.champion_evidence.clone(),
                    champion_risk: health.champion_risk.clone(),
                };
                if let Err(e) = crate::services::mutations::persist_champion_health(
                    db,
                    &meeting.id,
                    &db_health,
                ) {
                    log::warn!(
                        "Failed to persist reviewed champion health for {}: {}",
                        meeting.id,
                        e
                    );
                }
            } else if let Err(e) =
                crate::services::mutations::clear_champion_health(db, &meeting.id)
            {
                log::warn!(
                    "Failed to clear reviewed champion health for {}: {}",
                    meeting.id,
                    e
                );
            }
        }
    }

    // Persist Phase 3: dynamics, role changes, commitments
    if let Some(db) = db {
        persist_enriched_transcript_data(
            db,
            &meeting.id,
            &meeting.title,
            meeting.account.as_deref(),
            interaction_dynamics.as_ref(),
            None, // champion_health already persisted in Phase 2
            &role_changes,
            &commitments,
        );
    }
    emit_transcript_progress(
        app_handle,
        &meeting.id,
        "phase3",
        3,
        !summary.trim().is_empty(),
        true,
        true,
        extracted_actions.len(),
        wins.len(),
        risks.len(),
        decisions.len(),
        commitments.len(),
    );

    // Recompute health after transcript phases write champion_health + interaction_dynamics
    if let Some(db) = db {
        let transcript_entity_id = meeting
            .linked_entities
            .as_ref()
            .and_then(|e| e.first())
            .map(|e| (e.id.as_str(), e.entity_type.as_str()));

        if let Some((eid, etype)) = transcript_entity_id {
            if etype == "account" {
                if let Err(e) =
                    crate::services::intelligence::recompute_entity_health(db, eid, "account")
                {
                    log::warn!(
                        "Health recompute failed for {} after transcript: {}",
                        eid,
                        e
                    );
                }
            }
        }
    }

    // Post-phase hooks and logging (run after all phases)
    if let Some(db) = db {
        let ctx = hooks::EnrichmentContext {
            workspace: workspace.to_path_buf(),
            filename: dest_filename.clone(),
            classification: "transcript".to_string(),
            account: meeting.account.clone(),
            summary: summary.clone(),
            actions: Vec::new(),
            destination_path: Some(destination.display().to_string()),
            profile: profile.to_string(),
            wins: wins.clone(),
            risks: risks.clone(),
            entity_type: None,
        };
        let hook_results = hooks::run_post_enrichment_hooks(&ctx, db);
        for hr in &hook_results {
            log::info!(
                "Transcript hook '{}': {} — {}",
                hr.hook_name,
                if hr.success { "OK" } else { "FAILED" },
                hr.message.as_deref().unwrap_or("")
            );
        }
    }

    // Log to processing_log
    if let Some(db) = db {
        let log_entry = DbProcessingLog {
            id: uuid::Uuid::new_v4().to_string(),
            filename: dest_filename,
            source_path: file_path.to_string(),
            destination_path: Some(destination.display().to_string()),
            classification: "transcript".to_string(),
            status: "completed".to_string(),
            processed_at: Some(Utc::now().to_rfc3339()),
            error_message: None,
            created_at: Utc::now().to_rfc3339(),
        };
        if let Err(e) = crate::services::mutations::insert_processing_log(db, &log_entry) {
            log::warn!("Failed to log transcript processing: {}", e);
        }
    }

    // I636: Generate structured meeting record markdown
    if let Some(db) = db {
        generate_and_persist_meeting_record(
            workspace,
            meeting,
            &summary,
            &destination,
            &wins,
            &risks,
            &decisions,
            &extracted_actions,
            &commitments,
            db,
        );
    }

    // Append wins to impact log
    if !wins.is_empty() {
        append_to_impact_log(workspace, meeting, &wins);
    }

    // If summary is empty after parsing, include truncated raw output for debugging
    let debug_message = if summary.is_empty() {
        let preview = if phase1_output.len() > 200 {
            format!("{}...", &phase1_output[..200])
        } else {
            phase1_output.clone()
        };
        Some(format!("Empty parse result. Raw output: {}", preview))
    } else {
        None
    };

    TranscriptResult {
        status: "success".to_string(),
        summary: Some(summary),
        destination: Some(destination.display().to_string()),
        wins,
        risks,
        decisions,
        actions: extracted_actions,
        discussion,
        analysis,
        message: debug_message,
        sentiment,
        interaction_dynamics,
        champion_health,
        role_changes,
        commitments,
    }
}

/// I636: Generate a structured meeting record markdown file.
///
/// Produces a consolidated intelligence output document with YAML frontmatter,
/// executive summary, key findings, commitments, actions, and attendees.
fn generate_meeting_record_markdown(
    meeting: &CalendarEvent,
    summary: &str,
    wins: &[String],
    risks: &[String],
    decisions: &[String],
    actions: &[CapturedAction],
    commitments: &[TranscriptCommitment],
) -> String {
    let date = meeting.end.format("%Y-%m-%d").to_string();
    let time = meeting.start.format("%H:%M").to_string();
    let duration_mins = (meeting.end - meeting.start).num_minutes();
    let meeting_type = format!("{:?}", meeting.meeting_type).to_lowercase();
    let now = Utc::now().to_rfc3339();

    let entity_name = meeting.account.as_deref().unwrap_or("");
    let attendees_yaml = if meeting.attendees.is_empty() {
        "  - (none recorded)".to_string()
    } else {
        meeting
            .attendees
            .iter()
            .map(|a| format!("  - \"{}\"", a.replace('"', "\\\"")))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let mut md = String::with_capacity(2048);

    // YAML frontmatter
    md.push_str("---\n");
    md.push_str(&format!("meeting_id: \"{}\"\n", meeting.id));
    md.push_str(&format!(
        "title: \"{}\"\n",
        meeting.title.replace('"', "\\\"")
    ));
    if !entity_name.is_empty() {
        md.push_str(&format!(
            "entity: \"{}\"\n",
            entity_name.replace('"', "\\\"")
        ));
    }
    md.push_str(&format!("date: \"{}\"\n", date));
    md.push_str(&format!("time: \"{}\"\n", time));
    md.push_str(&format!("duration_minutes: {}\n", duration_mins));
    md.push_str(&format!("type: \"{}\"\n", meeting_type));
    md.push_str("attendees:\n");
    md.push_str(&attendees_yaml);
    md.push('\n');
    md.push_str(&format!("generated_at: \"{}\"\n", now));
    md.push_str("---\n\n");

    // Title heading
    let entity_suffix = if entity_name.is_empty() {
        String::new()
    } else {
        format!(" | {}", entity_name)
    };
    md.push_str(&format!(
        "# {}\n\n{} {} UTC{}\n\n",
        meeting.title, date, time, entity_suffix
    ));

    // Summary
    md.push_str("## Summary\n\n");
    if summary.is_empty() {
        md.push_str("*No summary available.*\n\n");
    } else {
        md.push_str(summary);
        md.push_str("\n\n");
    }

    // Key Findings (grouped by type)
    let has_findings = !wins.is_empty() || !risks.is_empty() || !decisions.is_empty();
    if has_findings {
        md.push_str("## Key Findings\n\n");

        if !wins.is_empty() {
            md.push_str("### Wins\n\n");
            for win in wins {
                md.push_str(&format!("- {}\n", win));
            }
            md.push('\n');
        }

        if !risks.is_empty() {
            md.push_str("### Risks\n\n");
            for risk in risks {
                md.push_str(&format!("- {}\n", risk));
            }
            md.push('\n');
        }

        if !decisions.is_empty() {
            md.push_str("### Decisions\n\n");
            for decision in decisions {
                md.push_str(&format!("- {}\n", decision));
            }
            md.push('\n');
        }
    }

    // Commitments
    if !commitments.is_empty() {
        md.push_str("## Commitments\n\n");
        for c in commitments {
            let mut line = format!("- {}", c.commitment);
            if let Some(ref owner) = c.owned_by {
                line.push_str(&format!(" ({})", owner));
            }
            if let Some(ref target) = c.target_date {
                line.push_str(&format!(" — by {}", target));
            }
            md.push_str(&line);
            md.push('\n');
            if let Some(ref criteria) = c.success_criteria {
                md.push_str(&format!("  - Success criteria: {}\n", criteria));
            }
        }
        md.push('\n');
    }

    // Actions
    if !actions.is_empty() {
        md.push_str("## Actions\n\n");
        for action in actions {
            let mut line = format!("- [ ] {}", action.title);
            if let Some(ref owner) = action.owner {
                line.push_str(&format!(" @{}", owner));
            }
            if let Some(ref due) = action.due_date {
                line.push_str(&format!(" (due: {})", due));
            }
            md.push_str(&line);
            md.push('\n');
        }
        md.push('\n');
    }

    // Attendees
    md.push_str("## Attendees\n\n");
    if meeting.attendees.is_empty() {
        md.push_str("*No attendees recorded.*\n");
    } else {
        for attendee in &meeting.attendees {
            md.push_str(&format!("- {}\n", attendee));
        }
    }

    md
}

/// I636: Determine the meeting record destination path, mirroring transcript
/// routing (account > project > person > archive) but under `Meeting-Records/`.
fn compute_meeting_record_path(
    workspace: &Path,
    meeting: &CalendarEvent,
    db: &crate::db::ActionDb,
) -> PathBuf {
    let date = meeting.end.format("%Y-%m-%d").to_string();
    let slug = crate::util::slugify(&meeting.title);
    let record_filename = format!("{}-{}-record.md", date, slug);

    // Same priority routing as transcript: account > project > person > archive
    if let Some(ref account) = meeting.account {
        let account_exists = db.get_account_by_name(account).ok().flatten().is_some();
        if account_exists {
            let account_dir = sanitize_account_dir(account);
            return workspace
                .join("Accounts")
                .join(&account_dir)
                .join("Meeting-Records")
                .join(&record_filename);
        }
    }

    // Project routing
    if let Some(ref entities) = meeting.linked_entities {
        if let Some(project) = entities.iter().find(|e| e.entity_type == "project") {
            if db.get_project(&project.id).ok().flatten().is_some() {
                let project_dir = sanitize_account_dir(&project.name);
                return workspace
                    .join("Projects")
                    .join(&project_dir)
                    .join("Meeting-Records")
                    .join(&record_filename);
            }
        }
    }

    // Person routing (1:1 only)
    if meeting.meeting_type == MeetingType::OneOnOne {
        if let Some(ref entities) = meeting.linked_entities {
            if let Some(person) = entities.iter().find(|e| e.entity_type == "person") {
                if db.get_person(&person.id).ok().flatten().is_some() {
                    let person_dir = sanitize_account_dir(&person.name);
                    return workspace
                        .join("People")
                        .join(&person_dir)
                        .join("Meeting-Records")
                        .join(&record_filename);
                }
            }
        }
    }

    // Archive fallback
    workspace
        .join("_archive")
        .join(&date)
        .join(&record_filename)
}

/// I636: Generate, write, persist, and index the meeting record.
#[allow(clippy::too_many_arguments)]
fn generate_and_persist_meeting_record(
    workspace: &Path,
    meeting: &CalendarEvent,
    summary: &str,
    _transcript_destination: &Path,
    wins: &[String],
    risks: &[String],
    decisions: &[String],
    actions: &[CapturedAction],
    commitments: &[TranscriptCommitment],
    db: &crate::db::ActionDb,
) {
    let record_path = compute_meeting_record_path(workspace, meeting, db);

    // Generate markdown content
    let markdown = generate_meeting_record_markdown(
        meeting,
        summary,
        wins,
        risks,
        decisions,
        actions,
        commitments,
    );

    // Create directory and write file
    if let Some(parent) = record_path.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            log::warn!(
                "I636: Failed to create Meeting-Records dir for {}: {}",
                meeting.id,
                e
            );
            return;
        }
    }

    if let Err(e) = std::fs::write(&record_path, &markdown) {
        log::warn!(
            "I636: Failed to write meeting record for {}: {}",
            meeting.id,
            e
        );
        return;
    }

    let record_path_str = record_path.display().to_string();
    log::info!(
        "I636: Meeting record for '{}' written to '{}'",
        meeting.title,
        record_path_str
    );

    // Store record_path in DB
    // DIRECT_DB_ALLOWED: Transcript processor pipeline owns meeting record persistence.
    if let Err(e) = db.conn.execute(
        "UPDATE meeting_transcripts SET record_path = ?1 WHERE meeting_id = ?2",
        rusqlite::params![record_path_str, meeting.id],
    ) {
        log::warn!(
            "I636: Failed to persist record_path for {}: {}",
            meeting.id,
            e
        );
    }

    // Content indexing for MCP search_content
    let entity_type = meeting
        .linked_entities
        .as_ref()
        .and_then(|e| e.first())
        .map(|e| e.entity_type.as_str())
        .unwrap_or("account");
    let entity_id = meeting
        .linked_entities
        .as_ref()
        .and_then(|e| e.first())
        .map(|e| e.id.as_str())
        .or(meeting.account.as_deref())
        .unwrap_or(&meeting.id);

    let now = Utc::now().to_rfc3339();
    let file_size = markdown.len() as i64;
    let filename = record_path
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_default();
    let relative_path = record_path
        .strip_prefix(workspace)
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| record_path_str.clone());

    let record = crate::db::types::DbContentFile {
        id: format!("meeting-record-{}", meeting.id),
        entity_id: entity_id.to_string(),
        entity_type: entity_type.to_string(),
        filename,
        relative_path,
        absolute_path: record_path_str,
        format: "Markdown".to_string(),
        file_size,
        modified_at: now.clone(),
        indexed_at: now.clone(),
        extracted_at: Some(now),
        summary: if summary.is_empty() {
            None
        } else {
            Some(summary.to_string())
        },
        embeddings_generated_at: None,
        content_type: "meeting_record".to_string(),
        priority: 8, // High priority — intelligence output
    };

    if let Err(e) = db.upsert_content_file(&record) {
        log::warn!(
            "I636: Failed to index meeting record for {}: {}",
            meeting.id,
            e
        );
    }
}

/// Parse a single action line from AI output.
///
/// Accepts common list formats so transcript extraction remains robust across
/// provider/model style changes:
/// - checkbox bullets (`- [ ]`, `- [x]`)
/// - plain bullets (`-`, `*`, `•`)
/// - numbered lists (`1.`, `2)`)
fn parse_action_line(line: &str) -> Option<&str> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }

    for prefix in ["- [ ] ", "- [x] ", "- [X] ", "- ", "* ", "• "] {
        if let Some(rest) = trimmed.strip_prefix(prefix) {
            let title = rest.trim();
            return (!title.is_empty()).then_some(title);
        }
    }

    parse_numbered_action_line(trimmed)
}

/// Parse numbered action items like `1. Follow up` or `2) Send recap`.
fn parse_numbered_action_line(line: &str) -> Option<&str> {
    let digit_count = line.chars().take_while(|c| c.is_ascii_digit()).count();
    if digit_count == 0 {
        return None;
    }

    let marker = line.chars().nth(digit_count)?;
    if marker != '.' && marker != ')' {
        return None;
    }

    let rest = line[(digit_count + marker.len_utf8())..].trim_start();
    (!rest.is_empty()).then_some(rest)
}

/// Resolve a meeting/account identifier (ID or display name) to a canonical account ID.
fn resolve_account_id(db: &ActionDb, candidate: &str) -> Option<String> {
    let normalized = candidate.trim();
    if normalized.is_empty() {
        return None;
    }

    db.get_account(normalized)
        .ok()
        .flatten()
        .map(|a| a.id)
        .or_else(|| {
            db.get_account_by_name(normalized)
                .ok()
                .flatten()
                .map(|a| a.id)
        })
}

#[allow(clippy::too_many_arguments)]
fn review_transcript_role_attribution(
    workspace: &Path,
    meeting: &CalendarEvent,
    db: Option<&ActionDb>,
    ai_config: &AiModelConfig,
    summary: &str,
    discussion: &[String],
    analysis: Option<&str>,
    wins: &[String],
    risks: &[String],
    decisions: &[String],
    champion_health: Option<&ChampionHealth>,
) -> Option<TranscriptRoleReviewPayload> {
    let config = crate::state::load_config().ok()?;
    let user_domains = config.resolved_user_domains();
    if user_domains.is_empty() {
        return None;
    }

    let attendee_hints = build_attendee_role_hints(db, meeting, &user_domains);
    if attendee_hints.is_empty() {
        return None;
    }

    let has_internal = attendee_hints.iter().any(|hint| hint.side == "internal");
    let has_external = attendee_hints.iter().any(|hint| hint.side == "external");
    if !has_internal || !has_external {
        return None;
    }

    let payload = TranscriptRoleReviewPayload {
        summary: summary.to_string(),
        discussion: discussion.to_vec(),
        analysis: analysis.map(|s| s.to_string()),
        wins: wins.to_vec(),
        risks: risks.to_vec(),
        decisions: decisions.to_vec(),
        champion_health: champion_health.cloned(),
    };

    let context_json = serde_json::json!({
        "meetingTitle": meeting.title,
        "internalDomains": user_domains,
        "attendees": attendee_hints,
        "extracted": payload,
    });
    let wrapped_context = wrap_user_data(&serde_json::to_string_pretty(&context_json).ok()?);
    let prompt = format!(
        r#"{preamble}You are reviewing extracted meeting intelligence for attendee-role accuracy only.

Correct only obvious mistakes where a person was assigned to the wrong side
(internal team vs customer/external attendee), or where the attendee roster makes
the intended person's name obvious.

Rules:
- Treat attendees whose email domains match `internalDomains` as internal.
- Treat all other human attendees as external/customer-side.
- Preserve the original meaning and wording as much as possible.
- Do NOT add new facts, re-summarize, or rewrite for style.
- Do NOT change actions, dates, priorities, or business claims unless they depend on the wrong person being named.
- If uncertain, leave the text unchanged.
- Return ONLY valid JSON matching the `extracted` object schema.

Review context:
{context}
"#,
        preamble = INJECTION_PREAMBLE,
        context = wrapped_context,
    );

    let pty = PtyManager::for_tier(ModelTier::Mechanical, ai_config)
        .with_timeout(TRANSCRIPT_ROLE_REVIEW_TIMEOUT_SECS);
    let output = match pty.spawn_claude(workspace, &prompt) {
        Ok(o) => o.stdout,
        Err(e) => {
            log::warn!(
                "Transcript role review failed for '{}': {}",
                meeting.title,
                e
            );
            return None;
        }
    };

    let parsed = parse_transcript_role_review_response(&output);
    if parsed.is_none() {
        log::warn!(
            "Transcript role review returned unparseable output for '{}'",
            meeting.title
        );
    }
    parsed
}

fn build_attendee_role_hints(
    db: Option<&ActionDb>,
    meeting: &CalendarEvent,
    user_domains: &[String],
) -> Vec<AttendeeRoleHint> {
    let mut seen = HashSet::new();
    meeting
        .attendees
        .iter()
        .filter_map(|email| {
            let email = email.trim().to_lowercase();
            if email.is_empty()
                || !email.contains('@')
                || email.ends_with("@group.calendar.google.com")
                || !seen.insert(email.clone())
            {
                return None;
            }

            let name = db
                .and_then(|db| db.get_person_by_email_or_alias(&email).ok().flatten())
                .and_then(|person| {
                    let trimmed = person.name.trim();
                    (!trimmed.is_empty()).then_some(trimmed.to_string())
                })
                .unwrap_or_else(|| humanize_attendee_email(&email));

            Some(AttendeeRoleHint {
                name,
                side: crate::util::classify_relationship_multi(&email, user_domains),
                email,
            })
        })
        .collect()
}

fn humanize_attendee_email(email: &str) -> String {
    let local = email.split('@').next().unwrap_or(email);
    local
        .split(['.', '_', '-'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn parse_transcript_role_review_response(output: &str) -> Option<TranscriptRoleReviewPayload> {
    let trimmed = output.trim();
    if trimmed.is_empty() {
        return None;
    }

    let start = trimmed.find('{')?;
    let end = trimmed.rfind('}')?;
    if end <= start {
        return None;
    }

    serde_json::from_str(&trimmed[start..=end]).ok()
}

fn parse_reviewed_win_metadata(raw: &str) -> (&str, Option<&str>, Option<&str>) {
    let (text, evidence) = parse_reviewed_evidence_quote(raw);
    let trimmed = text.trim();
    if let Some(rest) = trimmed.strip_prefix('[') {
        if let Some(end) = rest.find(']') {
            let sub_type = &rest[..end];
            let content = rest[end + 1..].trim();
            let sub_type_lower = sub_type.to_lowercase();
            let valid = matches!(
                sub_type_lower.as_str(),
                "adoption"
                    | "expansion"
                    | "value_realized"
                    | "relationship"
                    | "commercial"
                    | "advocacy"
            );
            if valid {
                return (content, Some(sub_type), evidence);
            }
        }
    }

    (text, None, evidence)
}

fn parse_reviewed_risk_metadata(raw: &str) -> (&str, Option<String>, Option<&str>) {
    let (text, evidence) = parse_reviewed_evidence_quote(raw);
    let trimmed = text.trim();
    if let Some(rest) = trimmed.strip_prefix('[') {
        if let Some(end) = rest.find(']') {
            let urgency = rest[..end].trim().to_lowercase();
            let content = rest[end + 1..].trim();
            let valid = matches!(urgency.as_str(), "red" | "yellow" | "green_watch");
            if valid {
                return (content, Some(urgency), evidence);
            }
        }
    }

    (text, None, evidence)
}

fn parse_reviewed_evidence_quote(raw: &str) -> (&str, Option<&str>) {
    if let Some(hash_idx) = raw.rfind("#\"") {
        let main = raw[..hash_idx].trim();
        let quote_start = hash_idx + 2;
        if let Some(end_idx) = raw[quote_start..].find('"') {
            let quote = &raw[quote_start..quote_start + end_idx];
            (main, Some(quote))
        } else {
            let quote = raw[quote_start..].trim_end_matches('"');
            (main, Some(quote))
        }
    } else {
        (raw, None)
    }
}

/// Extract actions from AI output, using meeting ID as source_id for meeting-scoped queries.
fn extract_transcript_actions(
    actions_text: &str,
    meeting_id: &str,
    meeting_title: &str,
    db: &ActionDb,
    account_fallback: Option<&str>,
) {
    let now = Utc::now().to_rfc3339();
    let mut attempted = 0;
    let mut written = 0;
    let mut skipped = 0;

    for line in actions_text.lines() {
        let Some(raw_title) = parse_action_line(line) else {
            continue;
        };

        if raw_title.is_empty() {
            continue;
        }

        let meta = super::metadata::parse_action_metadata(raw_title);

        let status = if meta.is_waiting {
            "waiting".to_string()
        } else {
            "proposed".to_string()
        };

        // Resolve @Tag to a real account ID; fall back to meeting-level account.
        // If the tag doesn't match any account, use None to avoid FK violations.
        let account_id = meta
            .account
            .as_deref()
            .and_then(|tag| resolve_account_id(db, tag))
            .or_else(|| account_fallback.and_then(|fallback| resolve_account_id(db, fallback)));

        attempted += 1;

        let action = crate::db::DbAction {
            id: format!("transcript-{}-{}", meeting_id, attempted - 1),
            title: meta.clean_title,
            priority: meta.priority.unwrap_or_else(|| "P2".to_string()),
            status,
            created_at: now.clone(),
            due_date: meta.due_date,
            completed_at: None,
            account_id,
            project_id: None,
            source_type: Some("transcript".to_string()),
            source_id: Some(meeting_id.to_string()),
            source_label: Some(meeting_title.to_string()),
            context: meta.context,
            waiting_on: if meta.is_waiting {
                Some("true".to_string())
            } else {
                None
            },
            updated_at: now.clone(),
            person_id: None,
            account_name: None,
            next_meeting_title: None,
            next_meeting_start: None,
        };

        match crate::services::mutations::upsert_action_if_not_completed(db, &action) {
            Err(e) => {
                log::warn!("Failed to insert transcript action: {}", e);
            }
            Ok(true) => {
                written += 1;
            }
            Ok(false) => {
                skipped += 1;
            }
        }
    }

    log::info!(
        "Transcript action extraction for '{}': {} attempted, {} written, {} dedup-skipped",
        meeting_title,
        attempted,
        written,
        skipped
    );
}

/// Persist I555 enriched transcript data (interaction dynamics, champion health,
/// role changes, commitments) to their respective tables.
///
/// Backwards-compatible: silently skips any block that wasn't parsed (None/empty).
#[allow(clippy::too_many_arguments)]
fn persist_enriched_transcript_data(
    db: &crate::db::ActionDb,
    meeting_id: &str,
    meeting_title: &str,
    account_id: Option<&str>,
    interaction_dynamics: Option<&InteractionDynamics>,
    champion_health: Option<&ChampionHealth>,
    role_changes: &[RoleChange],
    commitments: &[TranscriptCommitment],
) {
    // Persist interaction dynamics
    if let Some(dynamics) = interaction_dynamics {
        let db_dynamics = convert_dynamics_to_db(meeting_id, dynamics);
        // DIRECT_DB_ALLOWED: Background transcript processing writes meeting-level
        // dynamics directly in the processor pipeline.
        if let Err(e) = db.upsert_interaction_dynamics(meeting_id, &db_dynamics) {
            log::warn!(
                "Failed to persist interaction dynamics for {}: {}",
                meeting_id,
                e
            );
        }
    }

    // Persist champion health
    if let Some(health) = champion_health {
        let db_health = crate::db::types::ChampionHealthAssessment {
            meeting_id: meeting_id.to_string(),
            champion_name: Some(health.champion_name.clone()),
            champion_status: health.champion_status.clone(),
            champion_evidence: health.champion_evidence.clone(),
            champion_risk: health.champion_risk.clone(),
        };
        // DIRECT_DB_ALLOWED: Background transcript processing writes meeting-level
        // champion health directly in the processor pipeline.
        if let Err(e) = db.upsert_champion_health(meeting_id, &db_health) {
            log::warn!(
                "Failed to persist champion health for {}: {}",
                meeting_id,
                e
            );
        }
    }

    // Persist role changes
    if !role_changes.is_empty() {
        let db_changes: Vec<crate::db::types::RoleChange> = role_changes
            .iter()
            .map(|rc| crate::db::types::RoleChange {
                id: uuid::Uuid::new_v4().to_string(),
                meeting_id: meeting_id.to_string(),
                person_name: rc.person_name.clone(),
                old_status: rc.old_status.clone(),
                new_status: rc.new_status.clone(),
                evidence_quote: rc.evidence.clone(),
            })
            .collect();
        // DIRECT_DB_ALLOWED: Transcript extraction owns persistence of structured
        // role-change artifacts before downstream enrichment consumes them.
        if let Err(e) = db.insert_role_changes(meeting_id, &db_changes) {
            log::warn!("Failed to persist role changes for {}: {}", meeting_id, e);
        }
    }

    // Persist commitments as dual-write: captured_commitments + captures table
    if !commitments.is_empty() {
        for commitment in commitments {
            // Write to captured_commitments (structured, for objective suggestion pipeline)
            if let Some(acct_id) = account_id {
                let commit_id = uuid::Uuid::new_v4().to_string();
                let now = chrono::Utc::now().to_rfc3339();
                let source_label = format!("transcript:{}", meeting_title);
                let owned_by = commitment.owned_by.as_deref().unwrap_or("joint");
                // DIRECT_DB_ALLOWED: Transcript extraction dual-writes commitments as
                // structured artifacts for the success-plan suggestion pipeline.
                if let Err(e) = db.conn_ref().execute(
                    "INSERT OR IGNORE INTO captured_commitments (id, account_id, meeting_id, title, owner, target_date, confidence, source, consumed, created_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'medium', ?7, 0, ?8)",
                    rusqlite::params![
                        commit_id,
                        acct_id,
                        meeting_id,
                        commitment.commitment,
                        owned_by,
                        commitment.target_date,
                        source_label,
                        now,
                    ],
                ) {
                    log::warn!("Failed to insert captured_commitment: {}", e);
                }
            }

            // Also write to captures table with capture_type='commitment' for automatic
            // flow into intel context and meeting prep via existing queries
            // DIRECT_DB_ALLOWED: Transcript extraction dual-writes commitments into
            // captures so existing prep/intelligence queries can consume them immediately.
            if let Err(e) = db.insert_capture_enriched(
                meeting_id,
                meeting_title,
                account_id,
                "commitment",
                &commitment.commitment,
                None,
                None,
                commitment.success_criteria.as_deref(),
            ) {
                log::warn!("Failed to insert commitment capture: {}", e);
            }
        }
    }
}

/// Convert the parsed `InteractionDynamics` (types.rs) to the DB version (db/types.rs).
fn convert_dynamics_to_db(
    meeting_id: &str,
    dynamics: &InteractionDynamics,
) -> crate::db::types::InteractionDynamics {
    // Parse talk_balance string like "60/40" into customer/internal percentages
    let (customer_pct, internal_pct) = dynamics
        .talk_balance
        .as_deref()
        .and_then(|tb| {
            let parts: Vec<&str> = tb.split('/').collect();
            if parts.len() == 2 {
                let c = parts[0].trim().parse::<i32>().ok()?;
                let i = parts[1].trim().parse::<i32>().ok()?;
                Some((Some(c), Some(i)))
            } else {
                None
            }
        })
        .unwrap_or((None, None));

    // Convert speaker sentiments
    let speaker_sentiments: Vec<crate::db::types::SpeakerSentiment> = dynamics
        .speaker_sentiment
        .iter()
        .map(|ss| crate::db::types::SpeakerSentiment {
            name: ss.name.clone(),
            sentiment: ss.sentiment.clone(),
            evidence: ss.evidence.clone().unwrap_or_default(),
        })
        .collect();

    // Convert competitor mentions (same type name, different module)
    let competitor_mentions: Vec<crate::db::types::CompetitorMention> = dynamics
        .competitor_mentions
        .iter()
        .map(|cm| crate::db::types::CompetitorMention {
            competitor: cm.competitor.clone(),
            context: cm.context.clone(),
        })
        .collect();

    // Convert escalation signals to escalation quotes
    let escalation_language: Vec<crate::db::types::EscalationQuote> = dynamics
        .escalation_signals
        .iter()
        .map(|es| crate::db::types::EscalationQuote {
            quote: es.quote.clone(),
            speaker: es.speaker.clone().unwrap_or_default(),
        })
        .collect();

    // Extract engagement signals
    let (question_density, decision_maker_active, forward_looking, monologue_risk) =
        if let Some(ref eng) = dynamics.engagement_signals {
            (
                eng.question_density.clone(),
                eng.decision_maker_active.clone(),
                eng.forward_looking.clone(),
                eng.monologue_risk.unwrap_or(false),
            )
        } else {
            (None, None, None, false)
        };

    crate::db::types::InteractionDynamics {
        meeting_id: meeting_id.to_string(),
        talk_balance_customer_pct: customer_pct,
        talk_balance_internal_pct: internal_pct,
        speaker_sentiments,
        question_density,
        decision_maker_active,
        forward_looking,
        monologue_risk,
        competitor_mentions,
        escalation_language,
    }
}

/// Truncate transcript content with a tail-biased strategy.
///
/// For very long transcripts (>60K chars), keeps the first 3K chars (attendee
/// context, meeting opening) plus the last 57K chars (substantive discussion).
/// This prevents social preamble from dominating the AI's analysis window.
fn truncate_transcript(content: &str) -> String {
    if content.len() <= TRANSCRIPT_MAX_CHARS {
        return content.to_string();
    }

    // Find valid UTF-8 boundaries for the head slice
    let mut head_end = TRANSCRIPT_HEAD_KEEP;
    while head_end > 0 && !content.is_char_boundary(head_end) {
        head_end -= 1;
    }

    // Find valid UTF-8 boundary for the tail slice
    let tail_len = TRANSCRIPT_MAX_CHARS - head_end - 30; // 30 chars for the splice marker
    let mut tail_start = content.len() - tail_len;
    while tail_start < content.len() && !content.is_char_boundary(tail_start) {
        tail_start += 1;
    }

    format!(
        "{}\n\n[... truncated {} chars ...]\n\n{}",
        &content[..head_end],
        tail_start - head_end,
        &content[tail_start..],
    )
}

/// Build the common prompt header (preamble + meeting context + source intro).
///
/// Shared across all 3 phases to maintain consistent meeting context framing.
fn build_prompt_header(
    meeting: &CalendarEvent,
    content: &str,
    content_kind: TranscriptContentKind,
) -> (String, String) {
    let truncated = truncate_transcript(content);
    let meeting_type = format!("{:?}", meeting.meeting_type).to_lowercase();
    let title = if meeting.title.trim().is_empty() {
        "Untitled meeting"
    } else {
        &meeting.title
    };
    let account_line = match meeting.account.as_deref() {
        Some(a) if !a.trim().is_empty() => format!("Account: {}\n", sanitize_external_field(a)),
        _ => String::new(),
    };
    let date = meeting.end.format("%Y-%m-%d").to_string();
    let source_intro = match content_kind {
        TranscriptContentKind::Transcript => {
            format!("You are analyzing a transcript from a {meeting_type} meeting.")
        }
        TranscriptContentKind::Notes => format!(
            "You are analyzing meeting notes from a {meeting_type} meeting. These notes may already be condensed, so extract actions, decisions, wins, and risks from what is actually present. Do not repeat the notes verbatim or invent detail that is not supported."
        ),
    };

    let header = format!(
        r#"{preamble}{source_intro}

Meeting: "{title}"
{account_line}Date: {date}

IMPORTANT: Focus on the substantive business discussion. Skip social chitchat,
internal team banter, and small talk that typically occurs at the start of calls.
Prioritize customer-facing content — what the customer said, asked, or committed to."#,
        preamble = INJECTION_PREAMBLE,
        source_intro = source_intro,
        title = encode_high_risk_field(title),
        account_line = account_line,
        date = date,
    );

    (header, wrap_user_data(&truncated).to_string())
}

/// Phase 1 prompt: Core extraction — SUMMARY, DISCUSSION, ANALYSIS, ACTIONS.
///
/// This is the "meeting is done, what happened?" fast response (~30s).
fn build_phase1_prompt(
    meeting: &CalendarEvent,
    content: &str,
    content_kind: TranscriptContentKind,
) -> String {
    let (header, wrapped_content) = build_prompt_header(meeting, content, content_kind);

    format!(
        r#"{header}

Respond in exactly this format:

SUMMARY: <2-3 sentence executive summary focused on business outcomes and decisions, not a chronological recap. Who met, what was substantively discussed, key outcomes.>

DISCUSSION:
- <Topic 1>: <What was discussed, decided, or committed to. Include direct customer quotes where they reveal priorities, concerns, or sentiment.>
- <Topic 2>: ...
END_DISCUSSION

ANALYSIS: <1-2 sentences of strategic TAM-perspective insight — connect what happened in this meeting to account health, expansion potential, or renewal risk.>

ACTIONS:
- <concise action title> P1/P2/P3 @Account due: YYYY-MM-DD #"context sentence"
END_ACTIONS

Rules for actions:
- TITLE MUST be concise and imperative: verb + object, max 10 words. Not a sentence, not a description — a task.
  - Good: "Follow up on renewal pricing"
  - Bad: "Follow up with the client regarding the renewal discussion they mentioned during the quarterly business review"
- Include priority when urgency is inferable (P1=urgent, P2=normal, P3=low)
- Include @AccountName when action relates to a specific customer/account
- Include due: YYYY-MM-DD when a deadline is mentioned or implied
- Include #"context" with a short sentence explaining WHY this action matters or WHAT was discussed. Use quotes around multi-word context.
  - Good: #"Renewal decision pending CFO approval, budget freeze risk"
  - Bad: #billing
- Use "waiting" or "blocked" in the title if action depends on someone else
- If no metadata can be inferred, just write the action text plainly
- Example: Follow up on renewal P1 @Acme due: 2026-03-15 #"CFO needs pricing comparison before Q2 budget lock"

Transcript:
{content}
"#,
        header = header,
        content = wrapped_content,
    )
}

/// Phase 2 prompt: Intelligence extraction — WINS, RISKS, DECISIONS, SENTIMENT, CHAMPION_HEALTH.
///
/// Includes Phase 1 summary as context for coherence (~30s).
fn build_phase2_prompt(
    meeting: &CalendarEvent,
    content: &str,
    content_kind: TranscriptContentKind,
    phase1_summary: &str,
) -> String {
    let (header, wrapped_content) = build_prompt_header(meeting, content, content_kind);

    let summary_context = if phase1_summary.is_empty() {
        String::new()
    } else {
        format!(
            "\nPrevious analysis summary (for context): {}\n",
            phase1_summary
        )
    };

    format!(
        r#"{header}
{summary_context}
Respond in exactly this format:

WINS:
Extract only verifiable positive outcomes — not vague sentiment. Each win MUST include
a specific, observable event. "Customer seems happy" is NOT a win.

Sub-types (tag each):
- ADOPTION: milestone crossed, feature activated, user activation target met, integration completed
- EXPANSION: interest in additional scope, new department/team, usage ceiling hit, cross-functional mention
- VALUE_REALIZED: customer articulates ROI in their own words, KPI improvement attributed to product, results shared with leadership
- RELATIONSHIP: executive sponsor actively engaged, new champion identified, reference/case study agreement, advisory board join
- COMMERCIAL: renewal confirmed (especially early), upsell/cross-sell, multi-year commitment, budget increase
- ADVOCACY: public endorsement, referral, conference speaking, internal win-sharing to leadership

Format: - [SUB_TYPE] <specific win with evidence> #"verbatim quote if available"
END_WINS
RISKS:
Categorize each risk by urgency. Be specific — name the person, the competitor, the timeline.

RED (critical — requires immediate action):
- Champion departure or executive sponsor disengagement
- Active competitor evaluation or piloting
- Severe usage collapse (<50% utilization mentioned)
- Active escalation (unresolved critical issue)
- Budget elimination or review
- Explicit renewal doubt

YELLOW (moderate — needs a recovery plan):
- Usage decline mentioned but not severe
- Champion role change (internal move)
- Delayed implementation or milestone pushback
- Organizational restructuring affecting ownership
- Repeated feature complaints without resolution
- Reduced meeting attendance by key stakeholders

GREEN_WATCH (early warning — monitor):
- Vague dissatisfaction without specific cause
- New leadership reviewing vendor relationships
- Industry/company headwinds (layoffs, funding concerns)
- Reduced energy or engagement without stated reason

Format: - [RED|YELLOW|GREEN_WATCH] <specific risk with named people/timelines> #"verbatim quote"
END_RISKS
DECISIONS:
- [CUSTOMER_COMMITMENT|INTERNAL_DECISION|JOINT_AGREEMENT] <decision> @owner #"verbatim quote"
END_DECISIONS

CHAMPION_HEALTH:
- champion_name: <name or "unidentified">
- champion_status: strong|weak|lost|none
  strong = has power/influence + personally invested + actively advocates internally
  weak = present and helpful but lacks influence, personal stake unclear, or not advocating
  lost = champion departed, moved roles, or disengaged
  none = no identifiable champion in the meeting
- champion_evidence: <specific behavioral evidence from the call>
- champion_risk: <if weak/lost, what is the risk and recommended action>
END_CHAMPION_HEALTH

SENTIMENT:
- overall: positive|neutral|negative|mixed
- customer: positive|neutral|negative|mixed|n/a
- engagement: high|moderate|low|disengaged
- forward_looking: yes|no
- competitor_mentions: comma-separated list or "none"
- champion_present: yes|no|unknown
- champion_engaged: yes|no|n/a
- ownership_language: customer|vendor|mixed (does the customer say "our tool" or "your product"?)
- past_tense_references: yes|no (does the customer refer to using the product in past tense?)
- data_export_interest: yes|no (did the customer ask about data export, portability, or switching?)
- internal_advocacy_visible: yes|no (did the customer mention sharing results internally?)
- roadmap_interest: yes|no (did the customer ask about future features or roadmap?)
END_SENTIMENT

Rules for wins:
- Evidence threshold: only extract if specific evidence exists
- Tag each win with its sub-type ([ADOPTION], [EXPANSION], [VALUE_REALIZED], [RELATIONSHIP], [COMMERCIAL], [ADVOCACY])
- Include verbatim quotes via #"..." suffix when available
- If none are apparent, leave the section empty (just the markers)

Rules for risks:
- Tag each risk with urgency: [RED], [YELLOW], or [GREEN_WATCH]
- Name the specific person, competitor, or timeline
- Include verbatim quotes via #"..." suffix when available
- If none are apparent, leave the section empty (just the markers)

Rules for decisions:
- Tag each with [CUSTOMER_COMMITMENT], [INTERNAL_DECISION], or [JOINT_AGREEMENT]
- Include the decision owner via @owner
- Note any conditions or caveats attached to the decision
- If no decisions were made, leave the section empty

Rules for champion health:
- Focus on the PRIMARY champion/advocate at the customer org
- "strong" requires evidence of all three: influence, investment, and advocacy
- "weak" means helpful but missing one or more of the three pillars
- "lost" means champion has departed, moved roles, or visibly disengaged
- If no champion is identifiable from this interaction, use "none"
- champion_risk is only needed for weak/lost status

Rules for sentiment:
- Overall sentiment reflects the general tone of the entire meeting
- Customer sentiment focuses specifically on the customer's tone and language
- Engagement measures how actively participants contributed
- Champion = internal advocate for your product/service at the customer org
- If you cannot determine champion presence, use "unknown"
- ownership_language: "customer" if they say "our tool/system", "vendor" if "your product/service", "mixed" if both
- past_tense_references: "yes" if customer talks about using the product in past tense (churn signal)
- data_export_interest: "yes" if customer asks about data portability or export capabilities
- internal_advocacy_visible: "yes" if customer mentions sharing results or promoting product internally
- roadmap_interest: "yes" if customer asks about future features or product direction

Transcript:
{content}
"#,
        header = header,
        summary_context = summary_context,
        content = wrapped_content,
    )
}

/// Phase 3 prompt: Deep analysis — INTERACTION_DYNAMICS, COMMITMENTS, ROLE_CHANGES.
///
/// Includes Phase 1 summary as context for coherence (~30s).
fn build_phase3_prompt(
    meeting: &CalendarEvent,
    content: &str,
    content_kind: TranscriptContentKind,
    phase1_summary: &str,
) -> String {
    let (header, wrapped_content) = build_prompt_header(meeting, content, content_kind);

    let summary_context = if phase1_summary.is_empty() {
        String::new()
    } else {
        format!(
            "\nPrevious analysis summary (for context): {}\n",
            phase1_summary
        )
    };

    format!(
        r#"{header}
{summary_context}
Respond in exactly this format:

COMMITMENTS:
Mutual agreements, stated goals, success criteria, or outcome targets discussed.
Focus on strategic commitments (not individual action items).
Examples: "Achieve 50% adoption across 3 teams by Q3", "Deliver ROI report before
renewal", "Resolve integration blockers before go-live".
- <commitment> by: YYYY-MM-DD owned_by: us|them|joint #"success criteria"
END_COMMITMENTS

ROLE_CHANGES:
- <person name>: <old role/status> -> <new role/status> #"evidence quote"
END_ROLE_CHANGES

INTERACTION_DYNAMICS:
TALK_BALANCE: <customer_pct>/<internal_pct> or "unclear"
SPEAKER_SENTIMENT:
- <Name>: <positive|neutral|cautious|negative|mixed> — <evidence>
END_SPEAKER_SENTIMENT
ENGAGEMENT_SIGNALS:
- question_density: <high|moderate|low>
- decision_maker_active: <yes|no|unclear>
- forward_looking: <high|moderate|low>
- monologue_risk: <yes|no>
END_ENGAGEMENT_SIGNALS
COMPETITOR_MENTIONS:
- <Competitor>: <context>
END_COMPETITOR_MENTIONS
ESCALATION_LANGUAGE:
- <quote or paraphrase> — <speaker>
END_ESCALATION_LANGUAGE
END_INTERACTION_DYNAMICS

Rules for commitments:
- Focus on strategic commitments, not individual action items (those go in ACTIONS)
- Include target date (by: YYYY-MM-DD) and ownership (owned_by: us|them|joint) when available
- Include success criteria in quotes when discussed
- If no commitments were made, leave the section empty (just the markers)

Rules for role changes:
- Only include if a role change, departure, hire, or promotion is explicitly mentioned
- Include the person's name, their previous and new status, and evidence
- If no role changes were mentioned, leave the section empty (just the markers)

Rules for interaction dynamics:
- Talk balance approximates speaking time split between customer and internal team
- Speaker sentiment should cover the 2-4 most prominent speakers
- Evidence should be a brief quote or paraphrase supporting the sentiment assessment
- Only include competitor mentions if competitors were explicitly named
- Escalation language captures quotes suggesting frustration, urgency, or risk
- If a sub-section has no data, leave it empty (just the markers)

Transcript:
{content}
"#,
        header = header,
        summary_context = summary_context,
        content = wrapped_content,
    )
}

/// Build the meeting-contextualized prompt for transcript analysis.
pub fn build_transcript_prompt(meeting: &CalendarEvent, content: &str) -> String {
    build_transcript_prompt_with_kind(meeting, content, TranscriptContentKind::Transcript)
}

pub fn build_transcript_prompt_with_kind(
    meeting: &CalendarEvent,
    content: &str,
    content_kind: TranscriptContentKind,
) -> String {
    let truncated = truncate_transcript(content);

    let meeting_type = format!("{:?}", meeting.meeting_type).to_lowercase();
    let title = if meeting.title.trim().is_empty() {
        "Untitled meeting"
    } else {
        &meeting.title
    };
    let account_line = match meeting.account.as_deref() {
        Some(a) if !a.trim().is_empty() => format!("Account: {}\n", sanitize_external_field(a)),
        _ => String::new(),
    };
    let date = meeting.end.format("%Y-%m-%d").to_string();
    let source_intro = match content_kind {
        TranscriptContentKind::Transcript => {
            format!("You are analyzing a transcript from a {meeting_type} meeting.")
        }
        TranscriptContentKind::Notes => format!(
            "You are analyzing meeting notes from a {meeting_type} meeting. These notes may already be condensed, so extract actions, decisions, wins, and risks from what is actually present. Do not repeat the notes verbatim or invent detail that is not supported."
        ),
    };

    format!(
        r#"{preamble}{source_intro}

Meeting: "{title}"
{account_line}Date: {date}

IMPORTANT: Focus on the substantive business discussion. Skip social chitchat,
internal team banter, and small talk that typically occurs at the start of calls.
Prioritize customer-facing content — what the customer said, asked, or committed to.

Respond in exactly this format:

SUMMARY: <2-3 sentence executive summary focused on business outcomes and decisions, not a chronological recap. Who met, what was substantively discussed, key outcomes.>

DISCUSSION:
- <Topic 1>: <What was discussed, decided, or committed to. Include direct customer quotes where they reveal priorities, concerns, or sentiment.>
- <Topic 2>: ...
END_DISCUSSION

ANALYSIS: <1-2 sentences of strategic TAM-perspective insight — connect what happened in this meeting to account health, expansion potential, or renewal risk.>

ACTIONS:
- <concise action title> P1/P2/P3 @Account due: YYYY-MM-DD #"context sentence"
END_ACTIONS
WINS:
Extract only verifiable positive outcomes — not vague sentiment. Each win MUST include
a specific, observable event. "Customer seems happy" is NOT a win.

Sub-types (tag each):
- ADOPTION: milestone crossed, feature activated, user activation target met, integration completed
- EXPANSION: interest in additional scope, new department/team, usage ceiling hit, cross-functional mention
- VALUE_REALIZED: customer articulates ROI in their own words, KPI improvement attributed to product, results shared with leadership
- RELATIONSHIP: executive sponsor actively engaged, new champion identified, reference/case study agreement, advisory board join
- COMMERCIAL: renewal confirmed (especially early), upsell/cross-sell, multi-year commitment, budget increase
- ADVOCACY: public endorsement, referral, conference speaking, internal win-sharing to leadership

Format: - [SUB_TYPE] <specific win with evidence> #"verbatim quote if available"
END_WINS
RISKS:
Categorize each risk by urgency. Be specific — name the person, the competitor, the timeline.

RED (critical — requires immediate action):
- Champion departure or executive sponsor disengagement
- Active competitor evaluation or piloting
- Severe usage collapse (<50% utilization mentioned)
- Active escalation (unresolved critical issue)
- Budget elimination or review
- Explicit renewal doubt

YELLOW (moderate — needs a recovery plan):
- Usage decline mentioned but not severe
- Champion role change (internal move)
- Delayed implementation or milestone pushback
- Organizational restructuring affecting ownership
- Repeated feature complaints without resolution
- Reduced meeting attendance by key stakeholders

GREEN_WATCH (early warning — monitor):
- Vague dissatisfaction without specific cause
- New leadership reviewing vendor relationships
- Industry/company headwinds (layoffs, funding concerns)
- Reduced energy or engagement without stated reason

Format: - [RED|YELLOW|GREEN_WATCH] <specific risk with named people/timelines> #"verbatim quote"
END_RISKS
DECISIONS:
- [CUSTOMER_COMMITMENT|INTERNAL_DECISION|JOINT_AGREEMENT] <decision> @owner #"verbatim quote"
END_DECISIONS
COMMITMENTS:
Mutual agreements, stated goals, success criteria, or outcome targets discussed.
Focus on strategic commitments (not individual action items).
Examples: "Achieve 50% adoption across 3 teams by Q3", "Deliver ROI report before
renewal", "Resolve integration blockers before go-live".
- <commitment> by: YYYY-MM-DD owned_by: us|them|joint #"success criteria"
END_COMMITMENTS

Rules for actions:
- TITLE MUST be concise and imperative: verb + object, max 10 words. Not a sentence, not a description — a task.
  - Good: "Follow up on renewal pricing"
  - Bad: "Follow up with the client regarding the renewal discussion they mentioned during the quarterly business review"
- Include priority when urgency is inferable (P1=urgent, P2=normal, P3=low)
- Include @AccountName when action relates to a specific customer/account
- Include due: YYYY-MM-DD when a deadline is mentioned or implied
- Include #"context" with a short sentence explaining WHY this action matters or WHAT was discussed. Use quotes around multi-word context.
  - Good: #"Renewal decision pending CFO approval, budget freeze risk"
  - Bad: #billing
- Use "waiting" or "blocked" in the title if action depends on someone else
- If no metadata can be inferred, just write the action text plainly
- Example: Follow up on renewal P1 @Acme due: 2026-03-15 #"CFO needs pricing comparison before Q2 budget lock"

Rules for wins:
- Evidence threshold: only extract if specific evidence exists
- Tag each win with its sub-type ([ADOPTION], [EXPANSION], [VALUE_REALIZED], [RELATIONSHIP], [COMMERCIAL], [ADVOCACY])
- Include verbatim quotes via #"..." suffix when available
- If none are apparent, leave the section empty (just the markers)

Rules for risks:
- Tag each risk with urgency: [RED], [YELLOW], or [GREEN_WATCH]
- Name the specific person, competitor, or timeline
- Include verbatim quotes via #"..." suffix when available
- If none are apparent, leave the section empty (just the markers)

Rules for decisions:
- Tag each with [CUSTOMER_COMMITMENT], [INTERNAL_DECISION], or [JOINT_AGREEMENT]
- Include the decision owner via @owner
- Note any conditions or caveats attached to the decision
- If no decisions were made, leave the section empty

Rules for commitments:
- Focus on strategic commitments, not individual action items (those go in ACTIONS)
- Include target date (by: YYYY-MM-DD) and ownership (owned_by: us|them|joint) when available
- Include success criteria in quotes when discussed
- If no commitments were made, leave the section empty (just the markers)

CHAMPION_HEALTH:
- champion_name: <name or "unidentified">
- champion_status: strong|weak|lost|none
  strong = has power/influence + personally invested + actively advocates internally
  weak = present and helpful but lacks influence, personal stake unclear, or not advocating
  lost = champion departed, moved roles, or disengaged
  none = no identifiable champion in the meeting
- champion_evidence: <specific behavioral evidence from the call>
- champion_risk: <if weak/lost, what is the risk and recommended action>
END_CHAMPION_HEALTH

ROLE_CHANGES:
- <person name>: <old role/status> -> <new role/status> #"evidence quote"
END_ROLE_CHANGES

SENTIMENT:
- overall: positive|neutral|negative|mixed
- customer: positive|neutral|negative|mixed|n/a
- engagement: high|moderate|low|disengaged
- forward_looking: yes|no
- competitor_mentions: comma-separated list or "none"
- champion_present: yes|no|unknown
- champion_engaged: yes|no|n/a
- ownership_language: customer|vendor|mixed (does the customer say "our tool" or "your product"?)
- past_tense_references: yes|no (does the customer refer to using the product in past tense?)
- data_export_interest: yes|no (did the customer ask about data export, portability, or switching?)
- internal_advocacy_visible: yes|no (did the customer mention sharing results internally?)
- roadmap_interest: yes|no (did the customer ask about future features or roadmap?)
END_SENTIMENT

INTERACTION_DYNAMICS:
TALK_BALANCE: <customer_pct>/<internal_pct> or "unclear"
SPEAKER_SENTIMENT:
- <Name>: <positive|neutral|cautious|negative|mixed> — <evidence>
END_SPEAKER_SENTIMENT
ENGAGEMENT_SIGNALS:
- question_density: <high|moderate|low>
- decision_maker_active: <yes|no|unclear>
- forward_looking: <high|moderate|low>
- monologue_risk: <yes|no>
END_ENGAGEMENT_SIGNALS
COMPETITOR_MENTIONS:
- <Competitor>: <context>
END_COMPETITOR_MENTIONS
ESCALATION_LANGUAGE:
- <quote or paraphrase> — <speaker>
END_ESCALATION_LANGUAGE
END_INTERACTION_DYNAMICS

Rules for champion health:
- Focus on the PRIMARY champion/advocate at the customer org
- "strong" requires evidence of all three: influence, investment, and advocacy
- "weak" means helpful but missing one or more of the three pillars
- "lost" means champion has departed, moved roles, or visibly disengaged
- If no champion is identifiable from this interaction, use "none"
- champion_risk is only needed for weak/lost status

Rules for role changes:
- Only include if a role change, departure, hire, or promotion is explicitly mentioned
- Include the person's name, their previous and new status, and evidence
- If no role changes were mentioned, leave the section empty (just the markers)

Rules for sentiment:
- Overall sentiment reflects the general tone of the entire meeting
- Customer sentiment focuses specifically on the customer's tone and language
- Engagement measures how actively participants contributed
- Champion = internal advocate for your product/service at the customer org
- If you cannot determine champion presence, use "unknown"
- ownership_language: "customer" if they say "our tool/system", "vendor" if "your product/service", "mixed" if both
- past_tense_references: "yes" if customer talks about using the product in past tense (churn signal)
- data_export_interest: "yes" if customer asks about data portability or export capabilities
- internal_advocacy_visible: "yes" if customer mentions sharing results or promoting product internally
- roadmap_interest: "yes" if customer asks about future features or product direction

Rules for interaction dynamics:
- Talk balance approximates speaking time split between customer and internal team
- Speaker sentiment should cover the 2-4 most prominent speakers
- Evidence should be a brief quote or paraphrase supporting the sentiment assessment
- Only include competitor mentions if competitors were explicitly named
- Escalation language captures quotes suggesting frustration, urgency, or risk
- If a sub-section has no data, leave it empty (just the markers)

Transcript:
{content}
"#,
        preamble = INJECTION_PREAMBLE,
        source_intro = source_intro,
        title = encode_high_risk_field(title),
        account_line = account_line,
        date = date,
        content = wrap_user_data(&truncated),
    )
}

/// Build YAML frontmatter for the transcript file.
fn build_frontmatter(meeting: &CalendarEvent, date: &str) -> String {
    let meeting_type = format!("{:?}", meeting.meeting_type).to_lowercase();
    let account_line = meeting
        .account
        .as_deref()
        .map(|a| format!("account: \"{}\"\n", a))
        .unwrap_or_default();
    let now = Utc::now().to_rfc3339();

    format!(
        "---\nmeeting_id: \"{}\"\nmeeting_title: \"{}\"\n{}meeting_type: \"{}\"\nmeeting_date: \"{}\"\nprocessed_at: \"{}\"\nsource: transcript\n---\n",
        meeting.id,
        meeting.title.replace('"', "\\\""),
        account_line,
        meeting_type,
        date,
        now,
    )
}

/// Append wins to the impact log file.
///
/// Uses `OpenOptions::append` to avoid read-modify-write race conditions (I65).
fn append_to_impact_log(workspace: &Path, meeting: &CalendarEvent, wins: &[String]) {
    let impact_log = workspace.join("_today").join("90-impact-log.md");
    let mut content = String::new();

    // If file doesn't exist yet, prepend header
    if !impact_log.exists() {
        content.push_str("# Impact Log\n\n");
    }

    let label = meeting.account.as_deref().unwrap_or(&meeting.title);
    let now = Utc::now();

    for win in wins {
        content.push_str(&format!(
            "- **{}**: {} ({})\n",
            label,
            win,
            now.format("%H:%M")
        ));
    }

    // Atomic append — no read-modify-write race
    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&impact_log)
        .and_then(|mut f| std::io::Write::write_all(&mut f, content.as_bytes()));
}

use crate::util::slugify;

// =============================================================================
// Sentiment & Interaction Dynamics Parsing (I509)
// =============================================================================

/// Extract the text between `start` and `end` markers from a response.
fn extract_block(response: &str, start: &str, end: &str) -> Option<String> {
    let start_idx = response.find(start)?;
    let content_start = start_idx + start.len();
    let end_idx = response[content_start..].find(end)?;
    Some(response[content_start..content_start + end_idx].to_string())
}

/// Parse the SENTIMENT block from AI transcript response.
///
/// Returns `None` if the block is missing entirely. Returns a partial struct
/// if some fields are invalid — only fields that fail to parse are left as
/// their default/None values.
pub fn parse_sentiment_block(response: &str) -> Option<TranscriptSentiment> {
    let block = extract_block(response, "SENTIMENT:", "END_SENTIMENT")?;

    let mut overall = None;
    let mut customer = None;
    let mut engagement = None;
    let mut forward_looking = false;
    let mut competitor_mentions = Vec::new();
    let mut champion_present = None;
    let mut champion_engaged = None;
    let mut ownership_language = None;
    let mut past_tense_references = None;
    let mut data_export_interest = None;
    let mut internal_advocacy_visible = None;
    let mut roadmap_interest = None;

    for line in block.lines() {
        let trimmed = line.trim();
        let kv = if let Some(rest) = trimmed.strip_prefix("- ") {
            rest.trim()
        } else {
            continue;
        };

        if let Some((key, value)) = kv.split_once(':') {
            let key = key.trim().to_lowercase();
            let value = value.trim();

            match key.as_str() {
                "overall" => {
                    let v = value.to_lowercase();
                    if ["positive", "neutral", "negative", "mixed"].contains(&v.as_str()) {
                        overall = Some(v);
                    }
                }
                "customer" => {
                    let v = value.to_lowercase();
                    if ["positive", "neutral", "negative", "mixed", "n/a"].contains(&v.as_str()) {
                        customer = Some(v);
                    }
                }
                "engagement" => {
                    let v = value.to_lowercase();
                    if ["high", "moderate", "low", "disengaged"].contains(&v.as_str()) {
                        engagement = Some(v);
                    }
                }
                "forward_looking" => {
                    forward_looking = value.to_lowercase().trim() == "yes";
                }
                "competitor_mentions" => {
                    let v = value.trim();
                    if !v.is_empty() && v.to_lowercase() != "none" {
                        competitor_mentions = v.split(',').map(|s| s.trim().to_string()).collect();
                    }
                }
                "champion_present" => match value.to_lowercase().trim() {
                    "yes" => champion_present = Some(true),
                    "no" => champion_present = Some(false),
                    _ => {} // "unknown" or invalid → None
                },
                "champion_engaged" => match value.to_lowercase().trim() {
                    "yes" => champion_engaged = Some(true),
                    "no" => champion_engaged = Some(false),
                    _ => {} // "n/a" or invalid → None
                },
                // I554: Expanded sentiment markers
                "ownership_language" => {
                    let v = value.to_lowercase();
                    if ["customer", "vendor", "mixed"].contains(&v.as_str()) {
                        ownership_language = Some(v);
                    }
                }
                "past_tense_references" => {
                    past_tense_references = Some(value.to_lowercase().trim() == "yes");
                }
                "data_export_interest" => {
                    data_export_interest = Some(value.to_lowercase().trim() == "yes");
                }
                "internal_advocacy_visible" => {
                    internal_advocacy_visible = Some(value.to_lowercase().trim() == "yes");
                }
                "roadmap_interest" => {
                    roadmap_interest = Some(value.to_lowercase().trim() == "yes");
                }
                _ => {}
            }
        }
    }

    Some(TranscriptSentiment {
        overall,
        customer,
        engagement,
        forward_looking,
        competitor_mentions,
        champion_present,
        champion_engaged,
        ownership_language,
        past_tense_references,
        data_export_interest,
        internal_advocacy_visible,
        roadmap_interest,
    })
}

/// Parse the INTERACTION_DYNAMICS block from AI transcript response.
///
/// Returns `None` if the block is missing entirely. Gracefully handles
/// missing or malformed sub-blocks by leaving those fields empty/None.
pub fn parse_interaction_dynamics(response: &str) -> Option<InteractionDynamics> {
    let block = extract_block(
        response,
        "INTERACTION_DYNAMICS:",
        "END_INTERACTION_DYNAMICS",
    )?;

    // Parse TALK_BALANCE
    let talk_balance = block
        .lines()
        .find(|l| l.trim().starts_with("TALK_BALANCE:"))
        .and_then(|l| l.trim().strip_prefix("TALK_BALANCE:"))
        .map(|v| v.trim().to_string());

    // Parse SPEAKER_SENTIMENT sub-block
    let speaker_sentiment = extract_block(&block, "SPEAKER_SENTIMENT:", "END_SPEAKER_SENTIMENT")
        .map(|sub| {
            sub.lines()
                .filter_map(|line| {
                    let trimmed = line.trim();
                    let entry = trimmed.strip_prefix("- ")?;
                    // Format: Name: sentiment — evidence
                    let (name, rest) = entry.split_once(':')?;
                    let name = name.trim().to_string();
                    if name.is_empty() {
                        return None;
                    }
                    let rest = rest.trim();
                    // Split on " — " (em dash with spaces) for evidence
                    let (sentiment, evidence) = if let Some(idx) = rest.find(" — ") {
                        (
                            rest[..idx].trim().to_string(),
                            Some(rest[idx + " — ".len()..].trim().to_string()),
                        )
                    } else if let Some(idx) = rest.find(" - ") {
                        // Fallback: plain hyphen with spaces
                        (
                            rest[..idx].trim().to_string(),
                            Some(rest[idx + " - ".len()..].trim().to_string()),
                        )
                    } else {
                        (rest.to_string(), None)
                    };
                    Some(SpeakerSentiment {
                        name,
                        sentiment,
                        evidence,
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    // Parse ENGAGEMENT_SIGNALS sub-block
    let engagement_signals = extract_block(&block, "ENGAGEMENT_SIGNALS:", "END_ENGAGEMENT_SIGNALS")
        .map(|sub| {
            let mut question_density = None;
            let mut decision_maker_active = None;
            let mut forward_looking = None;
            let mut monologue_risk = None;

            for line in sub.lines() {
                let trimmed = line.trim();
                let entry = if let Some(rest) = trimmed.strip_prefix("- ") {
                    rest.trim()
                } else {
                    continue;
                };
                if let Some((key, value)) = entry.split_once(':') {
                    let key = key.trim().to_lowercase();
                    let value = value.trim().to_string();
                    match key.as_str() {
                        "question_density" => question_density = Some(value),
                        "decision_maker_active" => decision_maker_active = Some(value),
                        "forward_looking" => forward_looking = Some(value),
                        "monologue_risk" => {
                            monologue_risk = Some(value.to_lowercase() == "yes");
                        }
                        _ => {}
                    }
                }
            }

            EngagementSignals {
                question_density,
                decision_maker_active,
                forward_looking,
                monologue_risk,
            }
        });

    // Parse COMPETITOR_MENTIONS sub-block
    let competitor_mentions =
        extract_block(&block, "COMPETITOR_MENTIONS:", "END_COMPETITOR_MENTIONS")
            .map(|sub| {
                sub.lines()
                    .filter_map(|line| {
                        let trimmed = line.trim();
                        let entry = trimmed.strip_prefix("- ")?;
                        let (competitor, context) = entry.split_once(':')?;
                        let competitor = competitor.trim().to_string();
                        let context = context.trim().to_string();
                        if competitor.is_empty() {
                            return None;
                        }
                        Some(CompetitorMention {
                            competitor,
                            context,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

    // Parse ESCALATION_LANGUAGE sub-block
    let escalation_signals =
        extract_block(&block, "ESCALATION_LANGUAGE:", "END_ESCALATION_LANGUAGE")
            .map(|sub| {
                sub.lines()
                    .filter_map(|line| {
                        let trimmed = line.trim();
                        let entry = trimmed.strip_prefix("- ")?;
                        if entry.trim().is_empty() {
                            return None;
                        }
                        // Format: quote — speaker
                        let (quote, speaker) = if let Some(idx) = entry.find(" — ") {
                            (
                                entry[..idx].trim().to_string(),
                                Some(entry[idx + " — ".len()..].trim().to_string()),
                            )
                        } else if let Some(idx) = entry.find(" - ") {
                            (
                                entry[..idx].trim().to_string(),
                                Some(entry[idx + " - ".len()..].trim().to_string()),
                            )
                        } else {
                            (entry.trim().to_string(), None)
                        };
                        Some(EscalationSignal { quote, speaker })
                    })
                    .collect()
            })
            .unwrap_or_default();

    Some(InteractionDynamics {
        talk_balance,
        speaker_sentiment,
        engagement_signals,
        competitor_mentions,
        escalation_signals,
    })
}

// =============================================================================
// I554 — Champion Health, Role Changes, Commitments Parsing
// =============================================================================

/// Parse the CHAMPION_HEALTH block from AI transcript response.
///
/// Returns `None` if the block is missing entirely.
pub fn parse_champion_health_block(response: &str) -> Option<ChampionHealth> {
    let block = extract_block(response, "CHAMPION_HEALTH:", "END_CHAMPION_HEALTH")?;

    let mut champion_name = None;
    let mut champion_status = None;
    let mut champion_evidence = None;
    let mut champion_risk = None;

    for line in block.lines() {
        let trimmed = line.trim();
        let kv = if let Some(rest) = trimmed.strip_prefix("- ") {
            rest.trim()
        } else {
            trimmed
        };

        if let Some((key, value)) = kv.split_once(':') {
            let key = key.trim().to_lowercase();
            let value = value.trim();
            if value.is_empty() {
                continue;
            }

            match key.as_str() {
                "champion_name" => {
                    champion_name = Some(value.to_string());
                }
                "champion_status" => {
                    let v = value.to_lowercase();
                    if ["strong", "weak", "lost", "none"].contains(&v.as_str()) {
                        champion_status = Some(v);
                    }
                }
                "champion_evidence" => {
                    champion_evidence = Some(value.to_string());
                }
                "champion_risk" => {
                    champion_risk = Some(value.to_string());
                }
                _ => {}
            }
        }
    }

    // Require at least name and status to produce a valid result
    let name = champion_name.unwrap_or_else(|| "unidentified".to_string());
    let status = champion_status?;

    Some(ChampionHealth {
        champion_name: name,
        champion_status: status,
        champion_evidence,
        champion_risk,
    })
}

/// Parse the ROLE_CHANGES block from AI transcript response.
///
/// Returns an empty vec if the block is missing or contains no entries.
pub fn parse_role_changes_block(response: &str) -> Vec<RoleChange> {
    let block = match extract_block(response, "ROLE_CHANGES:", "END_ROLE_CHANGES") {
        Some(b) => b,
        None => return Vec::new(),
    };

    block
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            let entry = trimmed.strip_prefix("- ")?;
            if entry.trim().is_empty() {
                return None;
            }

            // Format: <person name>: <old role/status> -> <new role/status> #"evidence"
            let (main_part, evidence) = extract_verbatim_quote(entry);

            if let Some((person, role_change)) = main_part.split_once(':') {
                let person_name = person.trim().to_string();
                if person_name.is_empty() {
                    return None;
                }
                let role_change = role_change.trim();
                let (old_status, new_status) =
                    if let Some((old, new)) = role_change.split_once("->") {
                        (Some(old.trim().to_string()), Some(new.trim().to_string()))
                    } else if let Some((old, new)) = role_change.split_once("→") {
                        (Some(old.trim().to_string()), Some(new.trim().to_string()))
                    } else {
                        (None, Some(role_change.to_string()))
                    };

                Some(RoleChange {
                    person_name,
                    old_status,
                    new_status,
                    evidence: evidence.map(|s| s.to_string()),
                })
            } else {
                None
            }
        })
        .collect()
}

/// Parse the COMMITMENTS block from AI transcript response.
///
/// Returns an empty vec if the block is missing or contains no entries.
pub fn parse_commitments_block(response: &str) -> Vec<TranscriptCommitment> {
    let block = match extract_block(response, "COMMITMENTS:", "END_COMMITMENTS") {
        Some(b) => b,
        None => return Vec::new(),
    };

    block
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            let entry = trimmed.strip_prefix("- ")?;
            if entry.trim().is_empty() {
                return None;
            }

            // Extract optional fields: by: YYYY-MM-DD, owned_by: us|them|joint, #"criteria"
            let (main_part, success_criteria) = extract_verbatim_quote(entry);

            let target_date = extract_inline_field(main_part, "by:");
            let owned_by = extract_inline_field(main_part, "owned_by:");

            // The commitment text is everything before the first metadata field.
            // Check owned_by: first (longer prefix) to avoid "by:" matching inside "owned_by:".
            let commitment = main_part
                .split("owned_by:")
                .next()
                .unwrap_or(main_part)
                .split(" by:")
                .next()
                .unwrap_or(main_part)
                .trim()
                .to_string();

            if commitment.is_empty() {
                return None;
            }

            Some(TranscriptCommitment {
                commitment,
                target_date,
                owned_by,
                success_criteria: success_criteria.map(|s| s.to_string()),
            })
        })
        .collect()
}

/// Extract a verbatim quote from a line (text after `#"..."` suffix).
///
/// Returns `(main_text, Some(quote))` or `(original, None)`.
fn extract_verbatim_quote(text: &str) -> (&str, Option<&str>) {
    if let Some(hash_idx) = text.rfind("#\"") {
        let quote_start = hash_idx + 2;
        let main = text[..hash_idx].trim();
        // Find the closing quote
        if let Some(end_idx) = text[quote_start..].find('"') {
            let quote = &text[quote_start..quote_start + end_idx];
            (main, Some(quote))
        } else {
            // No closing quote — treat everything after #" as the quote
            let quote = &text[quote_start..];
            (main, Some(quote.trim_end_matches('"')))
        }
    } else {
        (text, None)
    }
}

/// Extract an inline field value like `by: 2026-06-01` or `owned_by: us` from text.
///
/// For short prefixes like "by:", only matches when preceded by a space or at the start
/// of the text, to avoid matching inside longer field names like "owned_by:".
fn extract_inline_field(text: &str, field_prefix: &str) -> Option<String> {
    // For short prefixes, require a space before or start-of-string
    let idx = if field_prefix.len() <= 3 {
        // Look for ` by:` (space-prefixed) to avoid matching inside `owned_by:`
        let space_prefix = format!(" {}", field_prefix);
        text.find(&space_prefix)
            .map(|i| i + 1) // skip the space to get to "by:"
            .or_else(|| {
                // Also match at start of text
                if text.starts_with(field_prefix) {
                    Some(0)
                } else {
                    None
                }
            })
    } else {
        text.find(field_prefix)
    };

    let idx = idx?;
    let after = &text[idx + field_prefix.len()..];
    let value = after.split_whitespace().next()?.trim().to_string();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

/// Title-case and hyphenate account name for directory routing.
fn sanitize_account_dir(name: &str) -> String {
    name.split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => c.to_uppercase().to_string() + &chars.as_str().to_lowercase(),
            }
        })
        .collect::<Vec<_>>()
        .join("-")
}

/// I631: Route transcript to a project entity directory if a linked project exists.
fn route_to_project(
    meeting: &CalendarEvent,
    db: Option<&ActionDb>,
    workspace: &Path,
    dest_filename: &str,
) -> Option<PathBuf> {
    let entities = meeting.linked_entities.as_ref()?;
    let project_entity = entities.iter().find(|e| e.entity_type == "project")?;
    let db = db?;

    // Validate project exists in DB
    match db.get_project(&project_entity.id) {
        Ok(Some(_)) => {
            let project_dir = sanitize_account_dir(&project_entity.name);
            let path = workspace
                .join("Projects")
                .join(&project_dir)
                .join("Call-Transcripts")
                .join(dest_filename);
            log::info!(
                "Routing transcript to project '{}' directory",
                project_entity.name
            );
            Some(path)
        }
        _ => {
            log::debug!(
                "Project '{}' not found in DB — skipping project routing",
                project_entity.name
            );
            None
        }
    }
}

/// I631: Route transcript to a person entity directory for 1:1 meetings.
fn route_to_person(
    meeting: &CalendarEvent,
    db: Option<&ActionDb>,
    workspace: &Path,
    dest_filename: &str,
) -> Option<PathBuf> {
    // Only route to people for 1:1 meetings
    if meeting.meeting_type != MeetingType::OneOnOne {
        return None;
    }

    let entities = meeting.linked_entities.as_ref()?;
    let person_entity = entities.iter().find(|e| e.entity_type == "person")?;
    let db = db?;

    // Validate person exists in DB
    match db.get_person(&person_entity.id) {
        Ok(Some(_)) => {
            let person_dir = sanitize_account_dir(&person_entity.name);
            let path = workspace
                .join("People")
                .join(&person_dir)
                .join("Call-Transcripts")
                .join(dest_filename);
            log::info!(
                "Routing transcript to person '{}' directory (1:1 meeting)",
                person_entity.name
            );
            Some(path)
        }
        _ => {
            log::debug!(
                "Person '{}' not found in DB — skipping person routing",
                person_entity.name
            );
            None
        }
    }
}

impl Default for TranscriptResult {
    fn default() -> Self {
        Self {
            status: "error".to_string(),
            summary: None,
            destination: None,
            wins: Vec::new(),
            risks: Vec::new(),
            decisions: Vec::new(),
            actions: Vec::new(),
            discussion: Vec::new(),
            analysis: None,
            message: None,
            sentiment: None,
            interaction_dynamics: None,
            champion_health: None,
            role_changes: Vec::new(),
            commitments: Vec::new(),
        }
    }
}

/// I535 Step 10: Build Gong call history context block for transcript processing.
///
/// When in Glean mode and the meeting's account has existing intelligence with
/// `gong_call_summaries`, format the most recent summaries (max 5, newest first)
/// as supplementary context prepended to the transcript prompt.
fn build_gong_pre_context(db: Option<&ActionDb>, meeting: &CalendarEvent) -> Option<String> {
    let db = db?;
    let account_name = meeting.account.as_deref()?;

    // Check if we're in Glean mode
    let mode = crate::context_provider::read_context_mode(db);
    if matches!(mode, crate::context_provider::ContextMode::Local) {
        return None;
    }

    // Look up account entity_id from account name
    let account = db.get_account_by_name(account_name).ok().flatten()?;
    let intel = db.get_entity_intelligence(&account.id).ok().flatten()?;

    if intel.gong_call_summaries.is_empty() {
        return None;
    }

    // Take up to 5 summaries, newest first (assume already sorted by date desc from Glean)
    let mut summaries = intel.gong_call_summaries.clone();
    summaries.sort_by(|a, b| b.date.cmp(&a.date));
    summaries.truncate(5);

    let mut block =
        String::from("SUPPLEMENTARY CONTEXT (prior calls with this account from Gong):\n");
    for s in &summaries {
        block.push_str(&format!(
            "- [{}] \"{}\": {}, sentiment: {}\n",
            s.date, s.title, s.key_topics, s.sentiment,
        ));
    }

    Some(block)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::MeetingType;
    use chrono::Utc;
    use std::path::PathBuf;

    fn test_meeting() -> CalendarEvent {
        CalendarEvent {
            id: "test-meeting-001".to_string(),
            title: "Acme QBR".to_string(),
            start: Utc::now(),
            end: Utc::now(),
            meeting_type: MeetingType::Customer,
            account: Some("Acme Corp".to_string()),
            attendees: vec![],
            is_all_day: false,
            linked_entities: None,
        }
    }

    #[test]
    fn test_build_transcript_prompt() {
        let meeting = test_meeting();
        let prompt = build_transcript_prompt(&meeting, "Hello world transcript");

        // Title is base64-encoded (I469 high-risk field)
        assert!(prompt.contains("<user_data encoding=\"base64\">"));
        // Account name is sanitize_external_field wrapped
        assert!(prompt.contains("<user_data>Acme Corp</user_data>"));
        assert!(prompt.contains("customer"));
        assert!(prompt.contains("Hello world transcript"));
        assert!(prompt.contains("DECISIONS:"));
        assert!(prompt.contains("DISCUSSION:"));
        assert!(prompt.contains("ANALYSIS:"));
        // I509 — sentiment and interaction dynamics sections
        assert!(prompt.contains("SENTIMENT:"));
        assert!(prompt.contains("END_SENTIMENT"));
        assert!(prompt.contains("INTERACTION_DYNAMICS:"));
        assert!(prompt.contains("END_INTERACTION_DYNAMICS"));
        // I554 — new extraction sections
        assert!(prompt.contains("CHAMPION_HEALTH:"));
        assert!(prompt.contains("END_CHAMPION_HEALTH"));
        assert!(prompt.contains("ROLE_CHANGES:"));
        assert!(prompt.contains("END_ROLE_CHANGES"));
        assert!(prompt.contains("COMMITMENTS:"));
        assert!(prompt.contains("END_COMMITMENTS"));
        // I554 — win sub-types
        assert!(prompt.contains("ADOPTION:"));
        assert!(prompt.contains("EXPANSION:"));
        assert!(prompt.contains("VALUE_REALIZED:"));
        assert!(prompt.contains("[SUB_TYPE]"));
        // I554 — risk urgency tiers
        assert!(prompt.contains("RED (critical"));
        assert!(prompt.contains("YELLOW (moderate"));
        assert!(prompt.contains("GREEN_WATCH (early"));
        // I554 — expanded sentiment markers
        assert!(prompt.contains("ownership_language:"));
        assert!(prompt.contains("past_tense_references:"));
        assert!(prompt.contains("data_export_interest:"));
        assert!(prompt.contains("internal_advocacy_visible:"));
        assert!(prompt.contains("roadmap_interest:"));
        // Verify focus on substance over chitchat
        assert!(prompt.contains("Skip social chitchat"));
        // Verify concise title instructions
        assert!(prompt.contains("max 10 words"));
        // Verify quoted context format
        assert!(prompt.contains("#\""));
    }

    #[test]
    fn test_build_transcript_prompt_for_notes() {
        let meeting = test_meeting();
        let prompt = build_transcript_prompt_with_kind(
            &meeting,
            "Condensed meeting notes",
            TranscriptContentKind::Notes,
        );

        assert!(prompt.contains("meeting notes"));
        assert!(prompt.contains("Do not repeat the notes verbatim"));
        assert!(prompt.contains("Condensed meeting notes"));
    }

    #[test]
    fn test_build_transcript_prompt_null_fields() {
        let mut meeting = test_meeting();
        meeting.account = None;
        meeting.title = "".to_string();
        let prompt = build_transcript_prompt(&meeting, "Some transcript");

        // "Untitled meeting" is now base64-encoded (I469)
        assert!(prompt.contains("<user_data encoding=\"base64\">"));
        // Account line should be omitted entirely
        assert!(!prompt.contains("Account:"));
        assert!(prompt.contains("Some transcript"));
    }

    #[test]
    fn test_truncate_transcript_short() {
        let short = "Short transcript content";
        assert_eq!(truncate_transcript(short), short);
    }

    #[test]
    fn test_truncate_transcript_long() {
        // Create content longer than TRANSCRIPT_MAX_CHARS
        let long_content = "A".repeat(70_000);
        let result = truncate_transcript(&long_content);
        assert!(result.len() < long_content.len());
        assert!(result.contains("[... truncated"));
        // Head should be preserved
        assert!(result.starts_with("AAA"));
        // Tail should be preserved
        assert!(result.ends_with("AAA"));
    }

    #[test]
    fn test_frontmatter_generation() {
        let meeting = test_meeting();
        let fm = build_frontmatter(&meeting, "2026-02-07");

        assert!(fm.starts_with("---\n"));
        assert!(fm.ends_with("---\n"));
        assert!(fm.contains("meeting_id: \"test-meeting-001\""));
        assert!(fm.contains("meeting_title: \"Acme QBR\""));
        assert!(fm.contains("account: \"Acme Corp\""));
        assert!(fm.contains("source: transcript"));
    }

    #[test]
    fn test_frontmatter_without_account() {
        let mut meeting = test_meeting();
        meeting.account = None;
        let fm = build_frontmatter(&meeting, "2026-02-07");

        assert!(!fm.contains("account:"));
        assert!(fm.contains("meeting_id:"));
    }

    #[test]
    fn test_destination_with_account() {
        let meeting = test_meeting();
        let date = "2026-02-07";
        let slug = slugify(&meeting.title);
        let dest_filename = format!("{}-{}-transcript.md", date, slug);

        let account_dir = sanitize_account_dir("Acme Corp");
        let workspace = Path::new("/workspace");
        let destination = workspace
            .join("Accounts")
            .join(&account_dir)
            .join("Call-Transcripts")
            .join(&dest_filename);

        assert_eq!(
            destination,
            PathBuf::from(
                "/workspace/Accounts/Acme-Corp/Call-Transcripts/2026-02-07-acme-qbr-transcript.md"
            )
        );
    }

    #[test]
    fn test_destination_without_account() {
        let date = "2026-02-07";
        let slug = slugify("Internal Sync");
        let dest_filename = format!("{}-{}-transcript.md", date, slug);

        let workspace = Path::new("/workspace");
        let destination = workspace.join("_archive").join(date).join(&dest_filename);

        assert_eq!(
            destination,
            PathBuf::from("/workspace/_archive/2026-02-07/2026-02-07-internal-sync-transcript.md")
        );
    }

    #[test]
    fn test_slugify() {
        assert_eq!(slugify("Acme QBR"), "acme-qbr");
        assert_eq!(
            slugify("Weekly Sync — Team Alpha"),
            "weekly-sync-team-alpha"
        );
        assert_eq!(slugify("simple"), "simple");
    }

    // =========================================================================
    // I509 — Sentiment & Interaction Dynamics Parsing
    // =========================================================================

    #[test]
    fn test_parse_sentiment_block_valid() {
        let input = "\
SUMMARY: Test meeting
SENTIMENT:
- overall: positive
- customer: neutral
- engagement: high
- forward_looking: yes
- competitor_mentions: Salesforce, HubSpot
- champion_present: yes
- champion_engaged: no
END_SENTIMENT
ACTIONS:
END_ACTIONS";

        let result = parse_sentiment_block(input).expect("should parse");
        assert_eq!(result.overall.as_deref(), Some("positive"));
        assert_eq!(result.customer.as_deref(), Some("neutral"));
        assert_eq!(result.engagement.as_deref(), Some("high"));
        assert!(result.forward_looking);
        assert_eq!(result.competitor_mentions, vec!["Salesforce", "HubSpot"]);
        assert_eq!(result.champion_present, Some(true));
        assert_eq!(result.champion_engaged, Some(false));
    }

    #[test]
    fn test_parse_sentiment_block_missing() {
        let input = "SUMMARY: No sentiment here\nACTIONS:\nEND_ACTIONS";
        assert!(parse_sentiment_block(input).is_none());
    }

    #[test]
    fn test_parse_sentiment_block_invalid_enums() {
        let input = "\
SENTIMENT:
- overall: fantastic
- customer: amazing
- engagement: moderate
- forward_looking: maybe
- competitor_mentions: none
- champion_present: unknown
- champion_engaged: n/a
END_SENTIMENT";

        let result = parse_sentiment_block(input).expect("should parse partial");
        // Invalid enums → None
        assert!(result.overall.is_none());
        assert!(result.customer.is_none());
        // Valid enum
        assert_eq!(result.engagement.as_deref(), Some("moderate"));
        // "maybe" is not "yes" → false
        assert!(!result.forward_looking);
        // "none" → empty vec
        assert!(result.competitor_mentions.is_empty());
        // "unknown" → None
        assert!(result.champion_present.is_none());
        // "n/a" → None
        assert!(result.champion_engaged.is_none());
    }

    #[test]
    fn test_parse_sentiment_competitor_list() {
        let input = "\
SENTIMENT:
- overall: mixed
- competitor_mentions: Salesforce, HubSpot
END_SENTIMENT";

        let result = parse_sentiment_block(input).expect("should parse");
        assert_eq!(result.competitor_mentions, vec!["Salesforce", "HubSpot"]);
    }

    #[test]
    fn test_parse_sentiment_competitor_none() {
        let input = "\
SENTIMENT:
- overall: positive
- competitor_mentions: none
END_SENTIMENT";

        let result = parse_sentiment_block(input).expect("should parse");
        assert!(result.competitor_mentions.is_empty());
    }

    #[test]
    fn test_parse_interaction_dynamics_valid() {
        let input = "\
INTERACTION_DYNAMICS:
TALK_BALANCE: 60/40
SPEAKER_SENTIMENT:
- Alice: positive — Very enthusiastic about the roadmap
- Bob: cautious — Raised concerns about timeline
END_SPEAKER_SENTIMENT
ENGAGEMENT_SIGNALS:
- question_density: high
- decision_maker_active: yes
- forward_looking: moderate
- monologue_risk: no
END_ENGAGEMENT_SIGNALS
COMPETITOR_MENTIONS:
- Salesforce: Mentioned as current CRM provider
- HubSpot: Evaluated but rejected last quarter
END_COMPETITOR_MENTIONS
ESCALATION_LANGUAGE:
- \"We need this resolved by Friday\" — Alice
- \"This is becoming a blocker\" — Bob
END_ESCALATION_LANGUAGE
END_INTERACTION_DYNAMICS";

        let result = parse_interaction_dynamics(input).expect("should parse");
        assert_eq!(result.talk_balance.as_deref(), Some("60/40"));

        assert_eq!(result.speaker_sentiment.len(), 2);
        assert_eq!(result.speaker_sentiment[0].name, "Alice");
        assert_eq!(result.speaker_sentiment[0].sentiment, "positive");
        assert_eq!(
            result.speaker_sentiment[0].evidence.as_deref(),
            Some("Very enthusiastic about the roadmap")
        );
        assert_eq!(result.speaker_sentiment[1].name, "Bob");

        let eng = result
            .engagement_signals
            .as_ref()
            .expect("should have engagement");
        assert_eq!(eng.question_density.as_deref(), Some("high"));
        assert_eq!(eng.decision_maker_active.as_deref(), Some("yes"));
        assert_eq!(eng.forward_looking.as_deref(), Some("moderate"));
        assert_eq!(eng.monologue_risk, Some(false));

        assert_eq!(result.competitor_mentions.len(), 2);
        assert_eq!(result.competitor_mentions[0].competitor, "Salesforce");
        assert_eq!(
            result.competitor_mentions[0].context,
            "Mentioned as current CRM provider"
        );

        assert_eq!(result.escalation_signals.len(), 2);
        assert!(result.escalation_signals[0]
            .quote
            .contains("need this resolved"));
        assert_eq!(
            result.escalation_signals[0].speaker.as_deref(),
            Some("Alice")
        );
    }

    #[test]
    fn test_parse_interaction_dynamics_missing() {
        let input = "SUMMARY: No dynamics here\nACTIONS:\nEND_ACTIONS";
        assert!(parse_interaction_dynamics(input).is_none());
    }

    #[test]
    fn test_parse_action_line_formats() {
        assert_eq!(
            parse_action_line("- [ ] Follow up on pricing"),
            Some("Follow up on pricing")
        );
        assert_eq!(
            parse_action_line("- [x] Send meeting recap"),
            Some("Send meeting recap")
        );
        assert_eq!(
            parse_action_line("* Confirm renewal owner"),
            Some("Confirm renewal owner")
        );
        assert_eq!(
            parse_action_line("• Draft scope document"),
            Some("Draft scope document")
        );
        assert_eq!(
            parse_action_line("1. Clarify success metrics"),
            Some("Clarify success metrics")
        );
        assert_eq!(
            parse_action_line("2) Assign implementation lead"),
            Some("Assign implementation lead")
        );
        assert_eq!(parse_action_line("Not an action"), None);
    }

    #[test]
    fn test_parse_decisions() {
        let output = "\
SUMMARY: Discussed Q1 roadmap
ACTIONS:
- P2 Follow up on renewal
END_ACTIONS
WINS:
- Expanded to 3 teams
END_WINS
RISKS:
- Budget freeze Q2
END_RISKS
DECISIONS:
- Expand pilot to EMEA starting Q2 — agreed by VP Sales
- Defer mobile launch to Q3
END_DECISIONS";

        let parsed = parse_enrichment_response(output);
        assert_eq!(parsed.decisions.len(), 2);
        assert_eq!(
            parsed.decisions[0],
            "Expand pilot to EMEA starting Q2 — agreed by VP Sales"
        );
        assert_eq!(parsed.decisions[1], "Defer mobile launch to Q3");
    }

    // =========================================================================
    // I554 — Champion Health, Role Changes, Commitments, Expanded Sentiment
    // =========================================================================

    #[test]
    fn test_parse_champion_health_block_valid() {
        let input = "\
CHAMPION_HEALTH:
- champion_name: Sarah Chen
- champion_status: strong
- champion_evidence: Led the roadmap discussion, asked detailed questions about integration
- champion_risk:
END_CHAMPION_HEALTH";

        let result = parse_champion_health_block(input).expect("should parse");
        assert_eq!(result.champion_name, "Sarah Chen");
        assert_eq!(result.champion_status, "strong");
        assert!(result.champion_evidence.is_some());
        assert!(result.champion_risk.is_none()); // empty value should be None
    }

    #[test]
    fn test_parse_champion_health_block_weak() {
        let input = "\
CHAMPION_HEALTH:
- champion_name: Bob Vance
- champion_status: weak
- champion_evidence: Present but mostly silent, deferred to manager
- champion_risk: Single-threaded relationship, need to identify executive sponsor
END_CHAMPION_HEALTH";

        let result = parse_champion_health_block(input).expect("should parse");
        assert_eq!(result.champion_status, "weak");
        assert!(result.champion_risk.is_some());
    }

    #[test]
    fn test_parse_champion_health_block_none() {
        let input = "\
CHAMPION_HEALTH:
- champion_name: unidentified
- champion_status: none
- champion_evidence: No clear advocate in this meeting
END_CHAMPION_HEALTH";

        let result = parse_champion_health_block(input).expect("should parse");
        assert_eq!(result.champion_name, "unidentified");
        assert_eq!(result.champion_status, "none");
    }

    #[test]
    fn test_parse_champion_health_block_missing() {
        let input = "SUMMARY: No champion block\nACTIONS:\nEND_ACTIONS";
        assert!(parse_champion_health_block(input).is_none());
    }

    #[test]
    fn test_parse_role_changes_block_valid() {
        let input = "\
ROLE_CHANGES:
- Sarah Chen: VP Engineering -> CTO #\"promoted last month\"
- Mike Ross: Account Manager -> Departed #\"leaving end of March\"
END_ROLE_CHANGES";

        let result = parse_role_changes_block(input);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].person_name, "Sarah Chen");
        assert_eq!(result[0].old_status.as_deref(), Some("VP Engineering"));
        assert_eq!(result[0].new_status.as_deref(), Some("CTO"));
        assert_eq!(result[0].evidence.as_deref(), Some("promoted last month"));
        assert_eq!(result[1].person_name, "Mike Ross");
    }

    #[test]
    fn test_parse_role_changes_block_empty() {
        let input = "\
ROLE_CHANGES:
END_ROLE_CHANGES";

        let result = parse_role_changes_block(input);
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_role_changes_block_missing() {
        let input = "SUMMARY: No role changes\nACTIONS:\nEND_ACTIONS";
        let result = parse_role_changes_block(input);
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_commitments_block_valid() {
        let input = "\
COMMITMENTS:
- Achieve 50% adoption across 3 teams by: 2026-06-01 owned_by: joint #\"primary success criterion\"
- Deliver ROI report before renewal owned_by: us
END_COMMITMENTS";

        let result = parse_commitments_block(input);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].commitment, "Achieve 50% adoption across 3 teams");
        assert_eq!(result[0].target_date.as_deref(), Some("2026-06-01"));
        assert_eq!(result[0].owned_by.as_deref(), Some("joint"));
        assert_eq!(
            result[0].success_criteria.as_deref(),
            Some("primary success criterion")
        );
        assert_eq!(result[1].commitment, "Deliver ROI report before renewal");
        assert_eq!(result[1].owned_by.as_deref(), Some("us"));
        assert!(result[1].target_date.is_none());
    }

    #[test]
    fn test_parse_commitments_block_missing() {
        let input = "SUMMARY: No commitments\nACTIONS:\nEND_ACTIONS";
        let result = parse_commitments_block(input);
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_sentiment_expanded_markers() {
        let input = "\
SENTIMENT:
- overall: positive
- customer: positive
- engagement: high
- forward_looking: yes
- competitor_mentions: none
- champion_present: yes
- champion_engaged: yes
- ownership_language: customer
- past_tense_references: no
- data_export_interest: no
- internal_advocacy_visible: yes
- roadmap_interest: yes
END_SENTIMENT";

        let result = parse_sentiment_block(input).expect("should parse");
        assert_eq!(result.ownership_language.as_deref(), Some("customer"));
        assert_eq!(result.past_tense_references, Some(false));
        assert_eq!(result.data_export_interest, Some(false));
        assert_eq!(result.internal_advocacy_visible, Some(true));
        assert_eq!(result.roadmap_interest, Some(true));
    }

    #[test]
    fn test_parse_sentiment_churn_signals() {
        let input = "\
SENTIMENT:
- overall: negative
- ownership_language: vendor
- past_tense_references: yes
- data_export_interest: yes
- internal_advocacy_visible: no
- roadmap_interest: no
END_SENTIMENT";

        let result = parse_sentiment_block(input).expect("should parse");
        assert_eq!(result.ownership_language.as_deref(), Some("vendor"));
        assert_eq!(result.past_tense_references, Some(true));
        assert_eq!(result.data_export_interest, Some(true));
        assert_eq!(result.internal_advocacy_visible, Some(false));
        assert_eq!(result.roadmap_interest, Some(false));
    }

    #[test]
    fn test_parse_sentiment_backwards_compat_no_new_fields() {
        // Old-format sentiment without I554 fields should still parse
        let input = "\
SENTIMENT:
- overall: positive
- customer: neutral
- engagement: moderate
- forward_looking: yes
- competitor_mentions: none
- champion_present: yes
- champion_engaged: yes
END_SENTIMENT";

        let result = parse_sentiment_block(input).expect("should parse");
        assert_eq!(result.overall.as_deref(), Some("positive"));
        // New fields should be None when not present
        assert!(result.ownership_language.is_none());
        assert!(result.past_tense_references.is_none());
        assert!(result.data_export_interest.is_none());
        assert!(result.internal_advocacy_visible.is_none());
        assert!(result.roadmap_interest.is_none());
    }

    #[test]
    fn test_extract_verbatim_quote() {
        let (main, quote) = extract_verbatim_quote(
            "[ADOPTION] Deployed to 3 teams #\"we rolled it out to engineering, sales, and support\""
        );
        assert_eq!(main, "[ADOPTION] Deployed to 3 teams");
        assert_eq!(
            quote,
            Some("we rolled it out to engineering, sales, and support")
        );

        // No quote
        let (main, quote) = extract_verbatim_quote("[RED] Champion departing");
        assert_eq!(main, "[RED] Champion departing");
        assert!(quote.is_none());
    }

    #[test]
    fn test_parse_wins_with_subtypes() {
        // Verify parser handles [SUB_TYPE] tagged wins from the new format
        let output = "\
FILE_TYPE: meeting_notes
ACCOUNT: Acme
MEETING: NONE
SUMMARY: Update
ACTIONS:
END_ACTIONS
WINS:
- [ADOPTION] Deployed to 3 new teams in Q1 #\"rolled out across engineering\"
- [VALUE_REALIZED] Reduced reporting time by 65%, saving $30K/month
END_WINS
RISKS:
- [RED] Champion Sarah leaving end of March #\"she mentioned her last day\"
- [YELLOW] Usage declining in APAC team
- [GREEN_WATCH] New VP reviewing vendor stack
END_RISKS
DECISIONS:
END_DECISIONS";

        let parsed = parse_enrichment_response(output);
        assert_eq!(parsed.wins.len(), 2);
        // Sub-type tags are stored as-is in the text (I555 will parse metadata)
        assert!(parsed.wins[0].starts_with("[ADOPTION]"));
        assert!(parsed.wins[1].starts_with("[VALUE_REALIZED]"));
        assert_eq!(parsed.risks.len(), 3);
        assert!(parsed.risks[0].starts_with("[RED]"));
        assert!(parsed.risks[1].starts_with("[YELLOW]"));
        assert!(parsed.risks[2].starts_with("[GREEN_WATCH]"));
    }
}

// ==========================================================================
// I619 — Prompt Evaluation Suite: transcript extraction quality tests
// ==========================================================================

#[cfg(test)]
mod eval_tests {
    use super::*;

    // ── Category 3: Transcript Extraction Quality Tests ──

    #[test]
    fn eval_transcript_wins_have_subtypes() {
        let response =
            include_str!("../intelligence/fixtures/transcript_extraction_full.txt");
        let parsed = parse_enrichment_response(response);

        assert!(
            parsed.wins.len() >= 3,
            "Full transcript must extract 3+ wins, got {}",
            parsed.wins.len()
        );

        let valid_subtypes = [
            "ADOPTION",
            "EXPANSION",
            "VALUE_REALIZED",
            "RELATIONSHIP",
            "COMMERCIAL",
            "ADVOCACY",
        ];
        for win in &parsed.wins {
            let has_subtype = valid_subtypes
                .iter()
                .any(|st| win.contains(&format!("[{}]", st)));
            assert!(
                has_subtype,
                "Win must have a valid sub-type tag: {}",
                win
            );
        }
    }

    #[test]
    fn eval_transcript_risks_have_urgency_tiers() {
        let response =
            include_str!("../intelligence/fixtures/transcript_extraction_full.txt");
        let parsed = parse_enrichment_response(response);

        assert!(
            parsed.risks.len() >= 3,
            "Full transcript must extract 3+ risks, got {}",
            parsed.risks.len()
        );

        let valid_urgencies = ["RED", "YELLOW", "GREEN_WATCH"];
        for risk in &parsed.risks {
            let has_urgency = valid_urgencies
                .iter()
                .any(|u| risk.contains(&format!("[{}]", u)));
            assert!(
                has_urgency,
                "Risk must have urgency tier tag: {}",
                risk
            );
        }

        // Verify we have at least one of each tier
        assert!(
            parsed.risks.iter().any(|r| r.contains("[RED]")),
            "Must have at least one RED risk"
        );
        assert!(
            parsed.risks.iter().any(|r| r.contains("[YELLOW]")),
            "Must have at least one YELLOW risk"
        );
        assert!(
            parsed.risks.iter().any(|r| r.contains("[GREEN_WATCH]")),
            "Must have at least one GREEN_WATCH risk"
        );
    }

    #[test]
    fn eval_champion_departure_flagged_as_lost() {
        let response =
            include_str!("../intelligence/fixtures/transcript_champion_departure.txt");

        let champion = parse_champion_health_block(response);
        assert!(
            champion.is_some(),
            "Champion departure fixture must produce champion health"
        );
        let ch = champion.unwrap();
        assert_eq!(
            ch.champion_status, "lost",
            "Departed champion must have 'lost' status"
        );
        assert_eq!(ch.champion_name, "Mike Torres");
        assert!(
            ch.champion_risk.is_some(),
            "Lost champion must have risk assessment"
        );

        // Verify RED risks present for champion departure
        let parsed = parse_enrichment_response(response);
        assert!(
            parsed.risks.iter().any(|r| r.contains("[RED]")),
            "Champion departure must produce at least one RED risk"
        );
    }

    #[test]
    fn eval_generic_sentiment_not_extracted_as_win() {
        let response =
            include_str!("../intelligence/fixtures/transcript_generic_sentiment.txt");
        let parsed = parse_enrichment_response(response);

        // The generic sentiment fixture should produce zero wins
        assert!(
            parsed.wins.is_empty(),
            "Generic sentiment should NOT produce wins, got: {:?}",
            parsed.wins
        );
    }

    #[test]
    fn eval_transcript_sentiment_parsing() {
        let response =
            include_str!("../intelligence/fixtures/transcript_extraction_full.txt");
        let sentiment = parse_sentiment_block(response);
        assert!(
            sentiment.is_some(),
            "Full transcript must produce sentiment block"
        );
        let s = sentiment.unwrap();

        // Verify core fields
        assert!(
            s.overall.is_some(),
            "Sentiment must have overall rating"
        );
        assert!(
            s.engagement.is_some(),
            "Sentiment must have engagement level"
        );

        // I554 expanded markers
        assert!(
            s.ownership_language.is_some(),
            "Sentiment must have ownership_language (I554)"
        );
        assert!(
            s.roadmap_interest.is_some(),
            "Sentiment must have roadmap_interest (I554)"
        );
        assert!(
            s.internal_advocacy_visible.is_some(),
            "Sentiment must have internal_advocacy_visible (I554)"
        );
    }

    #[test]
    fn eval_transcript_phase3_dynamics_parsing() {
        let response =
            include_str!("../intelligence/fixtures/transcript_phase3_dynamics.txt");

        // Commitments
        let commitments = parse_commitments_block(response);
        assert!(
            commitments.len() >= 2,
            "Must extract 2+ commitments, got {}",
            commitments.len()
        );
        // Verify commitment has ownership
        assert!(
            commitments.iter().any(|c| c.owned_by.is_some()),
            "At least one commitment must have owned_by"
        );
        // Verify commitment has target date
        assert!(
            commitments.iter().any(|c| c.target_date.is_some()),
            "At least one commitment must have target_date"
        );

        // Role changes
        let role_changes = parse_role_changes_block(response);
        assert!(
            role_changes.len() >= 2,
            "Must extract 2+ role changes, got {}",
            role_changes.len()
        );
        assert_eq!(role_changes[0].person_name, "Sarah Chen");
        assert!(
            role_changes[0].old_status.is_some(),
            "Role change must have old status"
        );
        assert!(
            role_changes[0].new_status.is_some(),
            "Role change must have new status"
        );

        // Interaction dynamics
        let dynamics = parse_interaction_dynamics(response);
        assert!(
            dynamics.is_some(),
            "Must parse interaction dynamics"
        );
        let d = dynamics.unwrap();
        assert!(
            d.talk_balance.is_some(),
            "Must have talk balance"
        );
        assert!(
            !d.speaker_sentiment.is_empty(),
            "Must have speaker sentiment entries"
        );
        assert!(
            !d.competitor_mentions.is_empty(),
            "Must have competitor mentions"
        );
        assert!(
            !d.escalation_signals.is_empty(),
            "Must have escalation signals"
        );
    }
}
