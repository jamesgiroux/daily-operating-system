//! Intelligence prompt building, response parsing, and enrichment orchestration.
//!
//! Extracted from entity_intel.rs. Contains:
//! - IntelligenceContext assembly from SQLite + files
//! - Prompt construction (initial and incremental modes)
//! - AI response parsing (JSON-first with pipe-delimited fallback)
//! - Entity enrichment orchestrator

use std::path::Path;

use chrono::Utc;
use serde::Deserialize;

use crate::db::{ActionDb, DbAccount};
use crate::util::wrap_user_data;

use super::io::*;

/// Maximum bytes of file content to include in the intelligence prompt context.
/// Keeps prompt size manageable (~10KB) while preserving the most relevant signals.
const MAX_CONTEXT_BYTES: usize = 10_000;

// =============================================================================
// Intelligence Context Assembly (I131)
// =============================================================================

/// Assembled signals for the intelligence enrichment prompt.
#[derive(Debug, Default)]
pub struct IntelligenceContext {
    /// Structured facts (ARR/health/lifecycle or status/milestone/owner).
    pub facts_block: String,
    /// Meeting history from last 90 days.
    pub meeting_history: String,
    /// Open actions for this entity.
    pub open_actions: String,
    /// Recent captures (wins/risks/decisions) from last 90 days.
    pub recent_captures: String,
    /// Recent email-derived signals linked to this entity.
    pub recent_email_signals: String,
    /// Linked stakeholders from entity_people + people.
    pub stakeholders: String,
    /// Source file manifest.
    pub file_manifest: Vec<SourceManifestEntry>,
    /// Extracted text from workspace files (50KB initial, 20KB incremental).
    pub file_contents: String,
    /// Raw text from the 2 most recent call transcripts (for engagement assessment).
    pub recent_transcripts: String,
    /// Serialized prior intelligence for incremental mode.
    pub prior_intelligence: Option<String>,
    /// Next upcoming meeting for this entity.
    pub next_meeting: Option<String>,
}

/// Build intelligence context by gathering all signals from SQLite + files.
#[allow(clippy::too_many_arguments)]
pub fn build_intelligence_context(
    _workspace: &Path,
    db: &ActionDb,
    entity_id: &str,
    entity_type: &str,
    account: Option<&DbAccount>,
    project: Option<&crate::db::DbProject>,
    prior: Option<&IntelligenceJson>,
    embedding_model: Option<&crate::embeddings::EmbeddingModel>,
) -> IntelligenceContext {
    let mut ctx = IntelligenceContext::default();

    // --- Facts block ---
    match entity_type {
        "account" => {
            if let Some(acct) = account {
                let mut facts = Vec::new();
                if let Some(ref h) = acct.health {
                    facts.push(format!("Health: {}", h));
                }
                if let Some(ref lc) = acct.lifecycle {
                    facts.push(format!("Lifecycle: {}", lc));
                }
                if let Some(arr) = acct.arr {
                    facts.push(format!("ARR: ${:.0}", arr));
                }
                if let Some(ref end) = acct.contract_end {
                    facts.push(format!("Renewal: {}", end));
                }
                if let Some(nps) = acct.nps {
                    facts.push(format!("NPS: {}", nps));
                }
                if let Ok(team) = db.get_account_team(entity_id) {
                    if !team.is_empty() {
                        let team_line = team
                            .iter()
                            .map(|m| format!("{} ({})", m.person_name, m.role.to_uppercase()))
                            .collect::<Vec<_>>()
                            .join(", ");
                        facts.push(format!("Account team: {}", team_line));
                    }
                }
                ctx.facts_block = facts.join("\n");
            }
        }
        "project" => {
            if let Some(proj) = project {
                let mut facts = Vec::new();
                facts.push(format!("Status: {}", proj.status));
                if let Some(ref ms) = proj.milestone {
                    facts.push(format!("Milestone: {}", ms));
                }
                if let Some(ref owner) = proj.owner {
                    facts.push(format!("Owner: {}", owner));
                }
                if let Some(ref target) = proj.target_date {
                    facts.push(format!("Target: {}", target));
                }
                ctx.facts_block = facts.join("\n");
            }
        }
        "person" => {
            if let Ok(Some(person)) = db.get_person(entity_id) {
                let mut facts = Vec::new();
                if let Some(ref org) = person.organization {
                    facts.push(format!("Organization: {}", org));
                }
                if let Some(ref role) = person.role {
                    facts.push(format!("Role: {}", role));
                }
                facts.push(format!("Relationship: {}", person.relationship));
                if let Some(ref first) = person.first_seen {
                    facts.push(format!("First seen: {}", first));
                }
                if let Some(ref last) = person.last_seen {
                    facts.push(format!("Last seen: {}", last));
                }
                facts.push(format!("Total meetings: {}", person.meeting_count));

                // Signals
                if let Ok(signals) = db.get_person_signals(entity_id) {
                    facts.push(format!("30d meetings: {}", signals.meeting_frequency_30d));
                    facts.push(format!("90d meetings: {}", signals.meeting_frequency_90d));
                    facts.push(format!("Temperature: {}", signals.temperature));
                    facts.push(format!("Trend: {}", signals.trend));
                }

                ctx.facts_block = facts.join("\n");
            }
        }
        _ => {}
    }

    // --- Meeting history (last 90 days) ---
    let meetings = match entity_type {
        "account" => db
            .get_meetings_for_account(entity_id, 20)
            .unwrap_or_default(),
        "project" => db
            .get_meetings_for_project(entity_id, 20)
            .unwrap_or_default(),
        "person" => db.get_person_meetings(entity_id, 20).unwrap_or_default(),
        _ => Vec::new(),
    };
    if !meetings.is_empty() {
        let lines: Vec<String> = meetings
            .iter()
            .map(|m| {
                format!(
                    "- {} | {} | {}",
                    m.start_time,
                    m.title,
                    m.summary.as_deref().unwrap_or("no summary")
                )
            })
            .collect();
        ctx.meeting_history = lines.join("\n");
    }

    // --- Open actions ---
    let actions = match entity_type {
        "account" => db.get_account_actions(entity_id).unwrap_or_default(),
        "project" => db.get_project_actions(entity_id).unwrap_or_default(),
        _ => Vec::new(),
    };
    if !actions.is_empty() {
        let lines: Vec<String> = actions
            .iter()
            .map(|a| {
                let due = a.due_date.as_deref().unwrap_or("no due date");
                let ctx_str = a.context.as_deref().unwrap_or("");
                format!("- [{}] {} (due: {}) {}", a.priority, a.title, due, ctx_str)
            })
            .collect();
        ctx.open_actions = lines.join("\n");
    }

    // --- Recent captures ---
    let captures = match entity_type {
        "account" => db
            .get_captures_for_account(entity_id, 90)
            .unwrap_or_default(),
        "project" => db
            .get_captures_for_project(entity_id, 90)
            .unwrap_or_default(),
        _ => Vec::new(),
    };
    if !captures.is_empty() {
        let lines: Vec<String> = captures
            .iter()
            .map(|c| {
                format!(
                    "- [{}] {} (from: {}, {})",
                    c.capture_type, c.content, c.meeting_title, c.captured_at
                )
            })
            .collect();
        ctx.recent_captures = lines.join("\n");
    }

    // --- Recent email signals ---
    {
        // Fetch more signals for entities with upcoming meetings
        let signal_limit = if ctx.next_meeting.is_some() { 20 } else { 12 };

        if let Ok(signals) = db.list_recent_email_signals_for_entity(entity_id, signal_limit) {
            if !signals.is_empty() {
                // Group signals by email_id for thread context.
                // Signals from the same email message appear together with indentation.
                // Uses Vec to preserve insertion order (most recent first from DB query).
                let mut grouped: Vec<(String, Vec<&crate::db::DbEmailSignal>)> = Vec::new();
                for s in &signals {
                    if let Some(entry) = grouped.iter_mut().find(|(id, _)| id == &s.email_id) {
                        entry.1.push(s);
                    } else {
                        grouped.push((s.email_id.clone(), vec![s]));
                    }
                }

                let mut lines: Vec<String> = Vec::new();

                for (_email_id, group) in &grouped {
                    let is_multi = group.len() > 1;

                    for s in group {
                        // Resolve person_id to name + role if available
                        let sender_info = if let Some(ref pid) = s.person_id {
                            match db.get_person(pid) {
                                Ok(Some(person)) => {
                                    let role = person.role.as_deref().unwrap_or("");
                                    let email =
                                        s.sender_email.as_deref().unwrap_or(&person.email);
                                    if role.is_empty() {
                                        format!("{} <{}>", person.name, email)
                                    } else {
                                        format!("{} ({}) <{}>", person.name, role, email)
                                    }
                                }
                                _ => s
                                    .sender_email
                                    .clone()
                                    .unwrap_or_else(|| "unknown".to_string()),
                            }
                        } else {
                            s.sender_email
                                .clone()
                                .unwrap_or_else(|| "unknown".to_string())
                        };

                        let age_str = compute_signal_age(&s.detected_at);
                        let indent = if is_multi { "  " } else { "" };

                        lines.push(format!(
                            "{indent}- [{}] {} — from: {} (urgency: {}, confidence: {:.2}, {})",
                            s.signal_type,
                            s.signal_text,
                            sender_info,
                            s.urgency.as_deref().unwrap_or("unknown"),
                            s.confidence.unwrap_or(0.0),
                            age_str,
                        ));
                    }
                }

                ctx.recent_email_signals = lines.join("\n");
            }
        }
    }

    // --- Email cadence summary (I319 data) ---
    {
        let conn = db.conn_ref();
        let cadence_result: Result<(i64, f64), _> = conn.query_row(
            "SELECT message_count, rolling_avg \
             FROM entity_email_cadence \
             WHERE entity_id = ?1 AND entity_type = ?2 \
             ORDER BY updated_at DESC LIMIT 1",
            rusqlite::params![entity_id, entity_type],
            |row| Ok((row.get(0)?, row.get(1)?)),
        );

        if let Ok((current_count, rolling_avg)) = cadence_result {
            let trend = if rolling_avg > 0.0 {
                let ratio = current_count as f64 / rolling_avg;
                if ratio < 0.5 {
                    "declining significantly"
                } else if ratio < 0.8 {
                    "declining"
                } else if ratio > 2.0 {
                    "spiking"
                } else if ratio > 1.2 {
                    "increasing"
                } else {
                    "normal"
                }
            } else {
                "no baseline"
            };

            let cadence_line = format!(
                "Email cadence: {}/week (rolling avg {:.0}/week — {})",
                current_count, rolling_avg, trend
            );

            if ctx.recent_email_signals.is_empty() {
                ctx.recent_email_signals = cadence_line;
            } else {
                ctx.recent_email_signals =
                    format!("{}\n{}", cadence_line, ctx.recent_email_signals);
            }
        }
    }

    // --- Stakeholders ---
    let people = db.get_people_for_entity(entity_id).unwrap_or_default();
    if !people.is_empty() {
        let lines: Vec<String> = people
            .iter()
            .map(|p| {
                let role = p.role.as_deref().unwrap_or("unknown role");
                let org = p.organization.as_deref().unwrap_or("");
                format!(
                    "- {} | {} | {} | {} meetings | last seen: {}",
                    p.name,
                    role,
                    org,
                    p.meeting_count,
                    p.last_seen.as_deref().unwrap_or("never")
                )
            })
            .collect();
        ctx.stakeholders = lines.join("\n");
    }

    // --- Entity connections (people only) ---
    if entity_type == "person" {
        let entities = db.get_entities_for_person(entity_id).unwrap_or_default();
        if !entities.is_empty() {
            let mut lines: Vec<String> = Vec::new();
            for ent in &entities {
                // Look up account/project details for health/status
                let ent_type_str = ent.entity_type.as_str();
                let detail = match ent_type_str {
                    "account" => {
                        if let Ok(Some(acct)) = db.get_account(&ent.id) {
                            let health = acct.health.as_deref().unwrap_or("unknown");
                            let lifecycle = acct.lifecycle.as_deref().unwrap_or("");
                            format!("health: {}, lifecycle: {}", health, lifecycle)
                        } else {
                            "no details".to_string()
                        }
                    }
                    "project" => {
                        if let Ok(Some(proj)) = db.get_project(&ent.id) {
                            format!("status: {}", proj.status)
                        } else {
                            "no details".to_string()
                        }
                    }
                    _ => String::new(),
                };
                lines.push(format!("- {} ({}) — {}", ent.name, ent_type_str, detail));
            }
            // Store in stakeholders field (repurposed for person context)
            if ctx.stakeholders.is_empty() {
                ctx.stakeholders = format!("Entity Connections:\n{}", lines.join("\n"));
            } else {
                ctx.stakeholders
                    .push_str(&format!("\n\nEntity Connections:\n{}", lines.join("\n")));
            }
        }
    }

    // --- File manifest + summaries (I286: vector-filtered, budget-capped) ---
    let files = db.get_entity_files(entity_id).unwrap_or_default();
    let is_incremental = prior.is_some();
    let enriched_at = prior.map(|p| p.enriched_at.as_str()).unwrap_or("");

    // Only consider files modified within the last 90 days
    let cutoff_90d = (Utc::now() - chrono::Duration::days(90)).to_rfc3339();

    // Use semantic search to rank files by relevance to entity's current state
    let mut ranked_files: Vec<&crate::db::DbContentFile> = Vec::new();
    let mut seen_file_ids = std::collections::HashSet::new();

    let semantic_query = semantic_gap_query(prior);
    if let Ok(matches) = crate::queries::search::search_entity_content(
        db,
        embedding_model,
        entity_id,
        &semantic_query,
        20,
        0.7,
        0.3,
    ) {
        for item in matches {
            if seen_file_ids.insert(item.content_file_id.clone()) {
                if let Some(file) = files.iter().find(|f| f.id == item.content_file_id) {
                    ranked_files.push(file);
                }
            }
        }
    }

    // Semantic search unavailable or no matches: preserve existing priority+recency behavior.
    if ranked_files.is_empty() {
        ranked_files.extend(files.iter());
    } else {
        for file in &files {
            if seen_file_ids.insert(file.id.clone()) {
                ranked_files.push(file);
            }
        }
    }

    // Collect file summaries within MAX_CONTEXT_BYTES budget.
    // Mandatory files (dashboard.json, recent meeting notes) are always included.
    let mut file_parts: Vec<String> = Vec::new();
    let mut total_bytes = 0usize;
    let mut selected_filenames = std::collections::HashSet::new();

    for file in &ranked_files {
        // Skip files older than 90 days (use filename date when available)
        if content_date_rfc3339(&file.filename, &file.modified_at) < cutoff_90d {
            continue;
        }

        // In incremental mode, only include files modified since last enrichment
        if is_incremental && !enriched_at.is_empty() && file.modified_at.as_str() <= enriched_at {
            continue;
        }

        let is_mandatory = file.content_type == "dashboard"
            || (file.content_type == "notes"
                && content_date_rfc3339(&file.filename, &file.modified_at) >= cutoff_90d);

        if let Some(ref summary) = file.summary {
            let entry = format!(
                "--- {} [{}] ({}) ---\n{}",
                file.filename, file.content_type, file.modified_at, summary
            );
            let entry_bytes = entry.len();

            // Mandatory files always included; others respect budget
            if !is_mandatory && total_bytes + entry_bytes > MAX_CONTEXT_BYTES {
                continue;
            }

            file_parts.push(entry);
            total_bytes += entry_bytes;
            selected_filenames.insert(file.filename.clone());

            // Stop once we've exceeded budget (mandatory files may push us over)
            if total_bytes >= MAX_CONTEXT_BYTES {
                break;
            }
        }
    }

    if !file_parts.is_empty() {
        ctx.file_contents = file_parts.join("\n\n");
    }

    // Build manifest with selected/skipped tracking (I286)
    ctx.file_manifest = files
        .iter()
        .filter(|f| content_date_rfc3339(&f.filename, &f.modified_at) >= cutoff_90d)
        .take(30)
        .map(|f| {
            let is_selected = selected_filenames.contains(&f.filename);
            SourceManifestEntry {
                filename: f.filename.clone(),
                modified_at: f.modified_at.clone(),
                format: Some(f.format.clone()),
                content_type: Some(f.content_type.clone()),
                selected: is_selected,
                skip_reason: if is_selected {
                    None
                } else {
                    Some("budget".to_string())
                },
            }
        })
        .collect();

    // --- Recent call transcripts (for stakeholder engagement assessment) ---
    // Read the raw text of the 2 most recent transcripts (up to 5K chars each).
    // Summaries are too compressed to judge engagement; raw text has the signal.
    {
        let mut transcript_files: Vec<&crate::db::DbContentFile> = files
            .iter()
            .filter(|f| f.content_type == "transcript")
            .collect();
        // Sort by content date descending (most recent first)
        transcript_files.sort_by(|a, b| {
            let da = content_date_rfc3339(&a.filename, &a.modified_at);
            let db_date = content_date_rfc3339(&b.filename, &b.modified_at);
            db_date.cmp(&da)
        });

        let mut transcript_parts: Vec<String> = Vec::new();
        for tf in transcript_files.into_iter().take(2) {
            let path = std::path::Path::new(&tf.absolute_path);
            if let Ok(text) = crate::processor::extract::extract_text(path) {
                let capped = if text.len() > 5000 {
                    format!("{}…", &text[..5000])
                } else {
                    text
                };
                transcript_parts.push(format!(
                    "--- {} ({}) ---\n{}",
                    tf.filename, tf.modified_at, capped
                ));
            }
        }
        if !transcript_parts.is_empty() {
            ctx.recent_transcripts = transcript_parts.join("\n\n");
        }
    }

    // --- Prior intelligence (for incremental mode) ---
    if let Some(p) = prior {
        ctx.prior_intelligence = serde_json::to_string_pretty(p).ok();
    }

    // --- Next meeting ---
    if entity_type == "account" {
        if let Ok(upcoming) = db.get_upcoming_meetings_for_account(entity_id, 1) {
            if let Some(m) = upcoming.first() {
                ctx.next_meeting = Some(format!("{} — {}", m.start_time, m.title));
            }
        }
    }

    ctx
}

fn semantic_gap_query(prior: Option<&IntelligenceJson>) -> String {
    let mut terms = vec!["account status", "risks", "wins", "blockers", "next steps"];

    if let Some(p) = prior {
        if p.risks.is_empty() {
            terms.push("risks concerns blockers challenges");
        }
        if p.recent_wins.is_empty() {
            terms.push("recent wins outcomes delivered value");
        }
        if p.current_state.is_none() {
            terms.push("working not working unknowns");
        }
    } else {
        terms.push("executive assessment context renewal sentiment");
    }

    terms.join(" ")
}

// =============================================================================
// Prompt Builder (I131)
// =============================================================================

/// Build the Claude Code prompt for entity intelligence enrichment.
///
/// Two modes: initial (no prior intelligence — full context + web search) and
/// incremental (has prior intelligence — delta context, no web search).
pub fn build_intelligence_prompt(
    entity_name: &str,
    entity_type: &str,
    ctx: &IntelligenceContext,
    relationship: Option<&str>,
    vocabulary: Option<&crate::presets::schema::PresetVocabulary>,
) -> String {
    build_intelligence_prompt_inner(entity_name, entity_type, ctx, relationship, vocabulary, None)
}

/// Build the intelligence prompt with full preset context including briefing_emphasis.
pub fn build_intelligence_prompt_with_preset(
    entity_name: &str,
    entity_type: &str,
    ctx: &IntelligenceContext,
    relationship: Option<&str>,
    preset: Option<&crate::presets::schema::RolePreset>,
) -> String {
    let vocabulary = preset.map(|p| &p.vocabulary);
    let briefing_emphasis = preset.map(|p| p.briefing_emphasis.as_str());
    build_intelligence_prompt_inner(entity_name, entity_type, ctx, relationship, vocabulary, briefing_emphasis)
}

fn build_intelligence_prompt_inner(
    entity_name: &str,
    entity_type: &str,
    ctx: &IntelligenceContext,
    relationship: Option<&str>,
    vocabulary: Option<&crate::presets::schema::PresetVocabulary>,
    briefing_emphasis: Option<&str>,
) -> String {
    let is_incremental = ctx.prior_intelligence.is_some();
    let entity_label = match entity_type {
        "account" => vocabulary
            .map(|v| v.entity_noun.as_str())
            .unwrap_or("customer account"),
        "project" => "project",
        "person" => match relationship {
            Some("internal") => "internal teammate / colleague",
            Some("external") => "external stakeholder / customer contact",
            _ => "professional contact",
        },
        _ => "entity",
    };

    let mut prompt = String::with_capacity(4096);

    // System context
    prompt.push_str(&format!(
        "You are building an intelligence assessment for the {label} \"{name}\".\n\n",
        label = entity_label,
        name = wrap_user_data(entity_name)
    ));

    // I313: Inject full vocabulary context for domain-specific framing
    if let Some(vocab) = vocabulary {
        prompt.push_str(&format!(
            "Domain vocabulary: entities are called \"{noun}\" (plural: \"{noun_plural}\"). \
             The primary metric is \"{metric}\". Health is measured as \"{health}\". \
             Risk is framed as \"{risk}\". Success means \"{verb}\". \
             Regular cadence is the \"{cadence}\".\n",
            noun = vocab.entity_noun,
            noun_plural = vocab.entity_noun_plural,
            metric = vocab.primary_metric,
            health = vocab.health_label,
            risk = vocab.risk_label,
            verb = vocab.success_verb,
            cadence = vocab.cadence_noun,
        ));
        if let Some(emphasis) = briefing_emphasis {
            prompt.push_str(&format!(
                "Assessment emphasis: {}\n",
                emphasis,
            ));
        }
        prompt.push('\n');
    }

    if is_incremental {
        prompt.push_str(
            "This is an INCREMENTAL update. Prior intelligence is provided below. \
             Update fields that have new information. Preserve fields that haven't changed. \
             Do NOT use web search.\n\n",
        );
    } else {
        prompt.push_str(
            "This is an INITIAL intelligence build. Use all available context below. \
             Do NOT use web search — work only with the provided signals and file contents.\n\n",
        );
    }

    // Facts
    if !ctx.facts_block.is_empty() {
        prompt.push_str("## Current Facts\n");
        prompt.push_str(&wrap_user_data(&ctx.facts_block));
        prompt.push_str("\n\n");
    }

    // Prior intelligence (incremental only)
    if let Some(ref prior) = ctx.prior_intelligence {
        prompt.push_str("## Prior Intelligence (update, don't replace wholesale)\n");
        prompt.push_str(&wrap_user_data(prior));
        prompt.push_str("\n\n");
    }

    // Next meeting
    if let Some(ref meeting) = ctx.next_meeting {
        prompt.push_str("## Next Meeting\n");
        prompt.push_str(&wrap_user_data(meeting));
        prompt.push_str("\n\n");
    }

    // Signals from SQLite
    if !ctx.meeting_history.is_empty() {
        prompt.push_str("## Meeting History (last 90 days)\n");
        prompt.push_str(&wrap_user_data(&ctx.meeting_history));
        prompt.push_str("\n\n");
    }

    if !ctx.open_actions.is_empty() {
        prompt.push_str("## Open Actions\n");
        prompt.push_str(&wrap_user_data(&ctx.open_actions));
        prompt.push_str("\n\n");
    }

    if !ctx.recent_captures.is_empty() {
        prompt.push_str("## Recent Captures (wins/risks/decisions)\n");
        prompt.push_str(&wrap_user_data(&ctx.recent_captures));
        prompt.push_str("\n\n");
    }

    if !ctx.recent_email_signals.is_empty() {
        prompt.push_str("## Recent Email Signals\n");
        prompt.push_str("Use these signals to inform risk assessment, relationship health, and recommended actions. ");
        prompt.push_str("Weight recent high-confidence signals more heavily. ");
        prompt.push_str("Cadence changes (declining/spiking) may indicate engagement shifts.\n\n");
        prompt.push_str(&wrap_user_data(&ctx.recent_email_signals));
        prompt.push_str("\n\n");
    }

    if !ctx.stakeholders.is_empty() {
        prompt.push_str("## Stakeholders\n");
        prompt.push_str(&wrap_user_data(&ctx.stakeholders));
        prompt.push_str("\n\n");
    }

    // File manifest (always shown so Claude knows what exists)
    if !ctx.file_manifest.is_empty() {
        prompt.push_str("## Workspace Files\n");
        for f in &ctx.file_manifest {
            let ct = f.content_type.as_deref().unwrap_or("general");
            prompt.push_str(&format!(
                "- {} [{}] ({}, {})\n",
                wrap_user_data(&f.filename),
                ct,
                f.format.as_deref().unwrap_or("unknown"),
                f.modified_at
            ));
        }
        prompt.push('\n');
    }

    // File summaries (pre-computed, priority-ordered)
    if !ctx.file_contents.is_empty() {
        if is_incremental {
            prompt.push_str("## New/Modified File Summaries (since last enrichment)\n");
        } else {
            prompt.push_str("## File Summaries (by priority)\n");
        }
        prompt.push_str(&wrap_user_data(&ctx.file_contents));
        prompt.push_str("\n\n");
    }

    // Recent call transcripts (raw text for engagement assessment)
    if !ctx.recent_transcripts.is_empty() {
        prompt.push_str(
            "## Recent Call Transcripts\n\
             Use these transcripts to assess stakeholder engagement. Look for:\n\
             - Who speaks and how often\n\
             - Who asks detailed questions or proposes next steps (high engagement)\n\
             - Who participates but follows rather than leads (medium)\n\
             - Who is mostly silent, reactive, or absent (low)\n\n",
        );
        prompt.push_str(&wrap_user_data(&ctx.recent_transcripts));
        prompt.push_str("\n\n");
    }

    // Writing style instructions
    prompt.push_str(&format!(
        "WRITING RULES:\n\
         - Lead with conclusions, not evidence. State the \"so what\" first.\n\
         - Be concise. Every sentence must earn its place.\n\
         - Do NOT include footnotes, reference numbers, or source citations in prose.\n\
         - Do NOT embed filenames or source references inline in prose.\n\
         - Do NOT narrate chronologically. Synthesize themes and conclusions.\n\
         - Write for a busy executive who has 60 seconds to understand this {}.\n\n",
        entity_label,
    ));

    // Person-specific writing rules based on relationship type
    if entity_type == "person" {
        match relationship {
            Some("internal") => prompt.push_str(
                "PERSON CONTEXT — INTERNAL TEAMMATE:\n\
                 - Focus on collaboration patterns, shared work, and alignment.\n\
                 - WORKING items = productive collaboration signals, shared wins, effective coordination.\n\
                 - NOT_WORKING items = alignment gaps, communication friction, unclear ownership.\n\
                 - Risks should focus on team-level blockers, not relationship health.\n\
                 - Assessment should answer: 'How effectively do we work together?'\n\n",
            ),
            Some("external") => prompt.push_str(
                "PERSON CONTEXT — EXTERNAL STAKEHOLDER:\n\
                 - Focus on relationship health, engagement signals, and influence.\n\
                 - WORKING items = strong engagement, responsiveness, advocacy, trust signals.\n\
                 - NOT_WORKING items = disengagement, unresponsiveness, misalignment, risk of churn.\n\
                 - Risks should focus on relationship risks — champion departure, sentiment shifts.\n\
                 - Assessment should answer: 'What does this person need and how do I navigate them?'\n\n",
            ),
            _ => prompt.push_str(
                "PERSON CONTEXT:\n\
                 - Relationship type is unknown. Infer from available signals whether this is likely \
                   an internal colleague or external contact, and frame the assessment accordingly.\n\n",
            ),
        }
    }

    // Output format instructions
    let p1_framing = match entity_type {
        "account" => "account trajectory",
        "project" => "project trajectory",
        "person" => match relationship {
            Some("internal") => "collaboration dynamic",
            Some("external") => "relationship health",
            _ => "relationship dynamic",
        },
        _ => "overall assessment",
    };
    // JSON output format (I288)
    prompt.push_str(&format!(
        "Return ONLY a JSON object — no other text before or after.\n\
         The JSON must conform exactly to this schema:\n\n\
         ```json\n\
         {{\n\
           \"executiveAssessment\": \"2-4 paragraphs separated by \\\\n\\\\n. \
         Paragraph 1: One-sentence verdict on {p1_framing}. \
         Paragraph 2: Top risk or blocker. Paragraph 3: Biggest opportunity. \
         Paragraph 4 (optional): Key unknowns. No footnotes or references. Max 250 words.\",\n\
           \"risks\": [\n\
             {{\"text\": \"risk description\", \"urgency\": \"critical|watch|low\"}}\n\
           ],\n\
           \"recentWins\": [\n\
             {{\"text\": \"win description\", \"impact\": \"business impact\"}}\n\
           ],\n\
           \"currentState\": {{\n\
             \"working\": [\"what's going well\"],\n\
             \"notWorking\": [\"what needs attention\"],\n\
             \"unknowns\": [\"knowledge gap to resolve\"]\n\
           }},\n\
           \"stakeholderInsights\": [\n\
             {{\"name\": \"...\", \"role\": \"...\", \"assessment\": \"1-2 sentences\", \"engagement\": \"high|medium|low|unknown\"}}\n\
           ],\n\
           \"nextMeetingReadiness\": {{\n\
             \"prepItems\": [\"forward-looking prep item (max 3)\"]\n\
           }}"
    ));

    // Company context fields (initial accounts only)
    if !is_incremental && entity_type == "account" {
        prompt.push_str(
            ",\n\
               \"companyContext\": {\n\
                 \"description\": \"1 paragraph about what the company does\",\n\
                 \"industry\": \"primary industry\",\n\
                 \"size\": \"employee count or range\",\n\
                 \"headquarters\": \"city and country\",\n\
                 \"additionalContext\": \"any additional relevant business context\"\n\
               }",
        );
    }

    // I305: Keyword extraction for entity resolution
    prompt.push_str(
        ",\n\
           \"keywords\": [\"5-15 distinctive keywords/phrases that identify this entity \
         in meeting titles or calendar descriptions. Include product names, project codenames, \
         abbreviations, and commonly used references.\"]",
    );

    prompt.push_str(
        "\n\
         }}\n\
         ```\n\n\
         Engagement criteria for stakeholders — base ONLY on call transcript evidence:\n\
         - high = drives discussion, asks detailed questions, proposes next steps\n\
         - medium = participates and responds but follows rather than leads\n\
         - low = mostly silent, reactive only, brief responses, or absent from recent calls\n\
         - unknown = person not present in available transcripts\n\n\
         Max 3 nextMeetingReadiness.prepItems. Each should answer ONLY: \
         \"What do I need to do or ask before/during this meeting?\"\n",
    );

    prompt
}

// =============================================================================
// Response Parser (I288: JSON-first with pipe-delimited fallback)
// =============================================================================

/// Intermediate JSON schema for AI response deserialization.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AiIntelResponse {
    #[serde(default)]
    executive_assessment: Option<String>,
    #[serde(default)]
    sources: Vec<String>,
    #[serde(default)]
    risks: Vec<AiRisk>,
    #[serde(default)]
    recent_wins: Vec<AiWin>,
    #[serde(default)]
    current_state: Option<AiCurrentState>,
    #[serde(default)]
    stakeholder_insights: Vec<AiStakeholder>,
    #[serde(default)]
    value_delivered: Vec<AiValue>,
    #[serde(default)]
    next_meeting_readiness: Option<AiMeetingReadiness>,
    #[serde(default)]
    company_context: Option<AiCompanyContext>,
    /// Auto-extracted keywords for entity resolution (I305).
    #[serde(default)]
    keywords: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AiRisk {
    text: String,
    #[serde(default)]
    source: Option<String>,
    #[serde(default = "default_urgency")]
    urgency: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AiWin {
    text: String,
    #[serde(default)]
    source: Option<String>,
    #[serde(default)]
    impact: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AiCurrentState {
    #[serde(default)]
    working: Vec<String>,
    #[serde(default)]
    not_working: Vec<String>,
    #[serde(default)]
    unknowns: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AiStakeholder {
    name: String,
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    assessment: Option<String>,
    #[serde(default)]
    engagement: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AiValue {
    #[serde(default)]
    date: Option<String>,
    statement: String,
    #[serde(default)]
    source: Option<String>,
    #[serde(default)]
    impact: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AiMeetingReadiness {
    #[serde(default)]
    prep_items: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AiCompanyContext {
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    industry: Option<String>,
    #[serde(default)]
    size: Option<String>,
    #[serde(default)]
    headquarters: Option<String>,
    #[serde(default)]
    additional_context: Option<String>,
}

/// Parse Claude's intelligence response into an IntelligenceJson.
///
/// Tries JSON parsing first (new format), then falls back to pipe-delimited
/// parsing for backwards compatibility with in-flight responses.
pub fn parse_intelligence_response(
    response: &str,
    entity_id: &str,
    entity_type: &str,
    source_file_count: usize,
    manifest: Vec<SourceManifestEntry>,
) -> Result<IntelligenceJson, String> {
    // Try JSON first
    let mut intel = if let Some(parsed) =
        try_parse_json_response(response, entity_id, entity_type, source_file_count, &manifest)
    {
        parsed
    } else {
        // Fall back to pipe-delimited format (backwards compat)
        parse_pipe_delimited_response(response, entity_id, entity_type, source_file_count, manifest)?
    };

    // Cap array sizes to prevent oversized output (I296)
    intel.risks.truncate(20);
    intel.recent_wins.truncate(10);
    intel.stakeholder_insights.truncate(20);
    intel.value_delivered.truncate(10);
    if let Some(ref mut state) = intel.current_state {
        state.working.truncate(10);
        state.not_working.truncate(10);
        state.unknowns.truncate(10);
    }
    if let Some(ref mut readiness) = intel.next_meeting_readiness {
        readiness.prep_items.truncate(10);
    }

    Ok(intel)
}

/// Extract a JSON object from the response text.
/// Handles responses with markdown fences or surrounding text.
pub(crate) fn extract_json_from_response(response: &str) -> Option<&str> {
    // Try to find JSON in a ```json code fence
    if let Some(start) = response.find("```json") {
        let json_start = start + 7;
        if let Some(end) = response[json_start..].find("```") {
            return Some(response[json_start..json_start + end].trim());
        }
    }
    // Try generic ``` code fence
    if let Some(start) = response.find("```") {
        let after_fence = start + 3;
        if let Some(nl) = response[after_fence..].find('\n') {
            let json_start = after_fence + nl + 1;
            if let Some(end) = response[json_start..].find("```") {
                let candidate = response[json_start..json_start + end].trim();
                if candidate.starts_with('{') {
                    return Some(candidate);
                }
            }
        }
    }

    // Try raw JSON object
    let trimmed = response.trim();
    if trimmed.starts_with('{') {
        return Some(trimmed);
    }
    // Look for JSON embedded in other text
    if let Some(start) = response.find('{') {
        let candidate = &response[start..];
        let mut depth = 0i32;
        let mut in_string = false;
        let mut escape = false;
        for (i, ch) in candidate.char_indices() {
            if escape {
                escape = false;
                continue;
            }
            if ch == '\\' && in_string {
                escape = true;
                continue;
            }
            if ch == '"' {
                in_string = !in_string;
                continue;
            }
            if in_string {
                continue;
            }
            if ch == '{' {
                depth += 1;
            } else if ch == '}' {
                depth -= 1;
                if depth == 0 {
                    return Some(&candidate[..=i]);
                }
            }
        }
    }
    None
}

/// Try to parse the response as JSON format. Returns None if it fails.
fn try_parse_json_response(
    response: &str,
    entity_id: &str,
    entity_type: &str,
    source_file_count: usize,
    manifest: &[SourceManifestEntry],
) -> Option<IntelligenceJson> {
    let json_str = extract_json_from_response(response)?;
    let ai_resp: AiIntelResponse = serde_json::from_str(json_str).ok()?;

    let current_state = ai_resp.current_state.map(|cs| CurrentState {
        working: cs.working,
        not_working: cs.not_working,
        unknowns: cs.unknowns,
    });

    let next_meeting_readiness = ai_resp.next_meeting_readiness.and_then(|mr| {
        if mr.prep_items.is_empty() {
            None
        } else {
            Some(MeetingReadiness {
                meeting_title: None,
                meeting_date: None,
                prep_items: mr.prep_items,
            })
        }
    });

    let company_context = ai_resp.company_context.map(|cc| CompanyContext {
        description: cc.description,
        industry: cc.industry,
        size: cc.size,
        headquarters: cc.headquarters,
        additional_context: cc.additional_context,
    });

    Some(IntelligenceJson {
        version: 1,
        entity_id: entity_id.to_string(),
        entity_type: entity_type.to_string(),
        enriched_at: Utc::now().to_rfc3339(),
        source_file_count,
        source_manifest: manifest.to_vec(),
        executive_assessment: ai_resp.executive_assessment,
        risks: ai_resp
            .risks
            .into_iter()
            .map(|r| IntelRisk {
                text: r.text,
                source: r.source,
                urgency: r.urgency,
            })
            .collect(),
        recent_wins: ai_resp
            .recent_wins
            .into_iter()
            .map(|w| IntelWin {
                text: w.text,
                source: w.source,
                impact: w.impact,
            })
            .collect(),
        current_state,
        stakeholder_insights: ai_resp
            .stakeholder_insights
            .into_iter()
            .map(|s| StakeholderInsight {
                name: s.name,
                role: s.role,
                assessment: s.assessment,
                engagement: s.engagement,
                source: None,
            })
            .collect(),
        value_delivered: ai_resp
            .value_delivered
            .into_iter()
            .map(|v| ValueItem {
                date: v.date,
                statement: v.statement,
                source: v.source,
                impact: v.impact,
            })
            .collect(),
        next_meeting_readiness,
        company_context,
        user_edits: Vec::new(),
    })
}

/// Parse legacy pipe-delimited format (backwards compatibility).
fn parse_pipe_delimited_response(
    response: &str,
    entity_id: &str,
    entity_type: &str,
    source_file_count: usize,
    manifest: Vec<SourceManifestEntry>,
) -> Result<IntelligenceJson, String> {
    let block = extract_intelligence_block(response)
        .ok_or("No INTELLIGENCE block or JSON found in response")?;

    let mut intel = IntelligenceJson {
        version: 1,
        entity_id: entity_id.to_string(),
        entity_type: entity_type.to_string(),
        enriched_at: Utc::now().to_rfc3339(),
        source_file_count,
        source_manifest: manifest,
        ..Default::default()
    };

    intel.executive_assessment = extract_multiline_field(&block, "EXECUTIVE_ASSESSMENT:");

    for line in block.lines() {
        let trimmed = line.trim();

        if let Some(rest) = trimmed.strip_prefix("RISK:") {
            if let Some(risk) = parse_risk_line(rest) {
                intel.risks.push(risk);
            }
        } else if let Some(rest) = trimmed.strip_prefix("WIN:") {
            if let Some(win) = parse_win_line(rest) {
                intel.recent_wins.push(win);
            }
        } else if let Some(rest) = trimmed.strip_prefix("WORKING:") {
            let state = intel
                .current_state
                .get_or_insert_with(CurrentState::default);
            state.working.push(rest.trim().to_string());
        } else if let Some(rest) = trimmed.strip_prefix("NOT_WORKING:") {
            let state = intel
                .current_state
                .get_or_insert_with(CurrentState::default);
            state.not_working.push(rest.trim().to_string());
        } else if let Some(rest) = trimmed.strip_prefix("UNKNOWN:") {
            let state = intel
                .current_state
                .get_or_insert_with(CurrentState::default);
            state.unknowns.push(rest.trim().to_string());
        } else if let Some(rest) = trimmed.strip_prefix("STAKEHOLDER:") {
            if let Some(sh) = parse_stakeholder_line(rest) {
                intel.stakeholder_insights.push(sh);
            }
        } else if let Some(rest) = trimmed.strip_prefix("VALUE:") {
            if let Some(val) = parse_value_line(rest) {
                intel.value_delivered.push(val);
            }
        } else if let Some(rest) = trimmed.strip_prefix("NEXT_MEETING_PREP:") {
            let readiness = intel
                .next_meeting_readiness
                .get_or_insert_with(|| MeetingReadiness {
                    meeting_title: None,
                    meeting_date: None,
                    prep_items: Vec::new(),
                });
            readiness.prep_items.push(rest.trim().to_string());
        } else if let Some(rest) = trimmed.strip_prefix("COMPANY_DESCRIPTION:") {
            let ctx = intel.company_context.get_or_insert(CompanyContext {
                description: None,
                industry: None,
                size: None,
                headquarters: None,
                additional_context: None,
            });
            ctx.description = Some(rest.trim().to_string());
        } else if let Some(rest) = trimmed.strip_prefix("COMPANY_INDUSTRY:") {
            let ctx = intel.company_context.get_or_insert(CompanyContext {
                description: None,
                industry: None,
                size: None,
                headquarters: None,
                additional_context: None,
            });
            ctx.industry = Some(rest.trim().to_string());
        } else if let Some(rest) = trimmed.strip_prefix("COMPANY_SIZE:") {
            let ctx = intel.company_context.get_or_insert(CompanyContext {
                description: None,
                industry: None,
                size: None,
                headquarters: None,
                additional_context: None,
            });
            ctx.size = Some(rest.trim().to_string());
        } else if let Some(rest) = trimmed.strip_prefix("COMPANY_HQ:") {
            let ctx = intel.company_context.get_or_insert(CompanyContext {
                description: None,
                industry: None,
                size: None,
                headquarters: None,
                additional_context: None,
            });
            ctx.headquarters = Some(rest.trim().to_string());
        } else if let Some(rest) = trimmed.strip_prefix("COMPANY_CONTEXT:") {
            let ctx = intel.company_context.get_or_insert(CompanyContext {
                description: None,
                industry: None,
                size: None,
                headquarters: None,
                additional_context: None,
            });
            ctx.additional_context = Some(rest.trim().to_string());
        }
    }

    Ok(intel)
}

/// Extract the INTELLIGENCE...END_INTELLIGENCE block from response text.
fn extract_intelligence_block(text: &str) -> Option<String> {
    let mut in_block = false;
    let mut lines = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed == "INTELLIGENCE" {
            in_block = true;
            continue;
        }
        if trimmed == "END_INTELLIGENCE" {
            break;
        }
        if in_block {
            lines.push(line);
        }
    }

    if lines.is_empty() {
        None
    } else {
        Some(lines.join("\n"))
    }
}

/// Extract a multi-line field delimited by `FIELD_NAME:` and `END_FIELD_NAME`.
fn extract_multiline_field(block: &str, start_marker: &str) -> Option<String> {
    let end_marker = format!("END_{}", start_marker.trim_end_matches(':'));

    let mut in_field = false;
    let mut lines = Vec::new();

    for line in block.lines() {
        let trimmed = line.trim();
        if let Some(stripped) = trimmed.strip_prefix(start_marker) {
            in_field = true;
            // Include any content on the same line as the marker
            let rest = stripped.trim();
            if !rest.is_empty() {
                lines.push(rest.to_string());
            }
            continue;
        }
        if trimmed == end_marker {
            in_field = false;
            continue;
        }
        if in_field {
            lines.push(line.to_string());
        }
    }

    if lines.is_empty() {
        None
    } else {
        Some(lines.join("\n").trim().to_string())
    }
}

/// Parse: `<text> | SOURCE: <src> | URGENCY: <urgency>`
fn parse_risk_line(rest: &str) -> Option<IntelRisk> {
    let parts: Vec<&str> = rest.split('|').collect();
    let text = parts.first()?.trim().to_string();
    if text.is_empty() {
        return None;
    }
    let source = find_pipe_field(&parts, "SOURCE");
    let urgency = find_pipe_field(&parts, "URGENCY").unwrap_or_else(|| "watch".to_string());

    Some(IntelRisk {
        text,
        source,
        urgency,
    })
}

/// Parse: `<text> | SOURCE: <src> | IMPACT: <impact>`
fn parse_win_line(rest: &str) -> Option<IntelWin> {
    let parts: Vec<&str> = rest.split('|').collect();
    let text = parts.first()?.trim().to_string();
    if text.is_empty() {
        return None;
    }
    let source = find_pipe_field(&parts, "SOURCE");
    let impact = find_pipe_field(&parts, "IMPACT");

    Some(IntelWin {
        text,
        source,
        impact,
    })
}

/// Parse: `<name> | ROLE: <role> | ASSESSMENT: <text> | ENGAGEMENT: <level>`
fn parse_stakeholder_line(rest: &str) -> Option<StakeholderInsight> {
    let parts: Vec<&str> = rest.split('|').collect();
    let name = parts.first()?.trim().to_string();
    if name.is_empty() {
        return None;
    }
    let role = find_pipe_field(&parts, "ROLE");
    let assessment = find_pipe_field(&parts, "ASSESSMENT");
    let engagement = find_pipe_field(&parts, "ENGAGEMENT");

    Some(StakeholderInsight {
        name,
        role,
        assessment,
        engagement,
        source: None,
    })
}

/// Parse: `<date> | <statement> | SOURCE: <src> | IMPACT: <impact>`
fn parse_value_line(rest: &str) -> Option<ValueItem> {
    let parts: Vec<&str> = rest.split('|').collect();
    if parts.len() < 2 {
        return None;
    }
    let date = Some(parts[0].trim().to_string()).filter(|s| !s.is_empty());
    let statement = parts[1].trim().to_string();
    if statement.is_empty() {
        return None;
    }
    let source = find_pipe_field(&parts, "SOURCE");
    let impact = find_pipe_field(&parts, "IMPACT");

    Some(ValueItem {
        date,
        statement,
        source,
        impact,
    })
}

/// Find a named field in pipe-delimited parts: `FIELD: value`.
fn find_pipe_field(parts: &[&str], field: &str) -> Option<String> {
    let prefix = format!("{}:", field);
    for part in parts {
        let trimmed = part.trim();
        if let Some(val) = trimmed.strip_prefix(&prefix) {
            let val = val.trim();
            if !val.is_empty() {
                return Some(val.to_string());
            }
        }
    }
    None
}

// enrich_entity_intelligence removed per ADR-0086 (I376).
// Entity intelligence is now enriched solely via intel_queue::run_enrichment.

fn compute_signal_age(detected_at: &str) -> String {
    let now = chrono::Utc::now();
    match chrono::DateTime::parse_from_rfc3339(detected_at) {
        Ok(dt) => {
            let duration = now.signed_duration_since(dt);
            let days = duration.num_days();
            if days == 0 {
                let hours = duration.num_hours();
                if hours == 0 {
                    "just now".to_string()
                } else if hours == 1 {
                    "1 hour ago".to_string()
                } else {
                    format!("{} hours ago", hours)
                }
            } else if days == 1 {
                "1 day ago".to_string()
            } else if days < 7 {
                format!("{} days ago", days)
            } else if days < 14 {
                "1 week ago".to_string()
            } else {
                format!("{} weeks ago", days / 7)
            }
        }
        Err(_) => detected_at.to_string(), // fallback to raw timestamp
    }
}

// ==========================================================================
// Tests
// ==========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_utils::test_db;

    // ─── Phase 2 tests: prompt builder + response parser ───

    #[test]
    fn test_build_intelligence_prompt_initial() {
        let ctx = IntelligenceContext {
            facts_block: "Health: green\nARR: $100000".to_string(),
            meeting_history: "- 2026-01-15 | QBR | Quarterly review".to_string(),
            open_actions: "- [P1] Follow up on renewal".to_string(),
            recent_captures: "- [win] Expanded seats".to_string(),
            recent_email_signals: String::new(),
            stakeholders: "- Alice | VP Eng | Acme | 5 meetings".to_string(),
            file_manifest: vec![SourceManifestEntry {
                filename: "qbr.md".to_string(),
                modified_at: "2026-01-30".to_string(),
                format: Some("markdown".to_string()),
                content_type: Some("qbr".to_string()),
                selected: true,
                skip_reason: None,
            }],
            file_contents: "--- qbr.md [qbr] (2026-01-30) ---\nContent here".to_string(),
            recent_transcripts: String::new(),
            prior_intelligence: None, // Initial mode
            next_meeting: Some("2026-02-05 — Weekly sync".to_string()),
        };

        let prompt = build_intelligence_prompt("Acme Corp", "account", &ctx, None, None);

        assert!(prompt.contains("INITIAL intelligence build"));
        assert!(prompt.contains("Acme Corp"));
        assert!(prompt.contains("Health: green"));
        assert!(prompt.contains("QBR"));
        assert!(prompt.contains("renewal"));
        // I288: JSON output format
        assert!(prompt.contains("\"companyContext\""));
        assert!(prompt.contains("JSON"));
        // I139: prompt refinements
        assert!(prompt.contains("Lead with conclusions"));
        assert!(prompt.contains("Do NOT include footnotes"));
        assert!(prompt.contains("Max 250 words"));
    }

    #[test]
    fn test_build_intelligence_prompt_incremental() {
        let ctx = IntelligenceContext {
            facts_block: "Status: active".to_string(),
            prior_intelligence: Some(
                r#"{"entityId":"proj","executiveAssessment":"Prior."}"#.to_string(),
            ),
            ..Default::default()
        };

        let prompt = build_intelligence_prompt("Project X", "project", &ctx, None, None);

        assert!(prompt.contains("INCREMENTAL update"));
        assert!(prompt.contains("Prior."));
        assert!(!prompt.contains("\"companyContext\""));
    }

    #[test]
    fn test_build_intelligence_prompt_person_external() {
        let ctx = IntelligenceContext {
            facts_block: "Role: VP Engineering".to_string(),
            meeting_history: "- 2026-02-01 | Weekly sync".to_string(),
            ..Default::default()
        };

        let prompt =
            build_intelligence_prompt("Alice Chen", "person", &ctx, Some("external"), None);

        assert!(prompt.contains("external stakeholder / customer contact"));
        assert!(prompt.contains("EXTERNAL STAKEHOLDER"));
        assert!(prompt.contains("relationship health"));
        assert!(!prompt.contains("INTERNAL TEAMMATE"));
        assert!(!prompt.contains("collaboration dynamic"));
    }

    #[test]
    fn test_build_intelligence_prompt_person_internal() {
        let ctx = IntelligenceContext {
            facts_block: "Role: Engineering Manager".to_string(),
            ..Default::default()
        };

        let prompt =
            build_intelligence_prompt("Bob Kim", "person", &ctx, Some("internal"), None);

        assert!(prompt.contains("internal teammate / colleague"));
        assert!(prompt.contains("INTERNAL TEAMMATE"));
        assert!(prompt.contains("collaboration dynamic"));
        assert!(!prompt.contains("EXTERNAL STAKEHOLDER"));
        // "relationship health" appears as negation in internal rules ("not relationship health")
        // so verify the external p1_framing string is absent instead
        assert!(!prompt.contains("relationship health in plain language"));
    }

    #[test]
    fn test_build_intelligence_prompt_person_unknown() {
        let ctx = IntelligenceContext::default();

        let prompt =
            build_intelligence_prompt("Unknown Person", "person", &ctx, None, None);

        assert!(prompt.contains("professional contact"));
        assert!(prompt.contains("Relationship type is unknown"));
        assert!(prompt.contains("relationship dynamic"));
    }

    #[test]
    fn test_build_intelligence_prompt_with_vocabulary() {
        let vocab = crate::presets::schema::PresetVocabulary {
            entity_noun: "partner".to_string(),
            entity_noun_plural: "partners".to_string(),
            primary_metric: "Deal Value".to_string(),
            health_label: "Engagement".to_string(),
            risk_label: "Deal Risk".to_string(),
            success_verb: "closed".to_string(),
            cadence_noun: "pipeline review".to_string(),
        };
        let ctx = IntelligenceContext {
            facts_block: "Health: green".to_string(),
            ..Default::default()
        };

        let prompt =
            build_intelligence_prompt("Acme Corp", "account", &ctx, None, Some(&vocab));

        // Should use vocabulary noun instead of default "customer account"
        assert!(prompt.contains("partner"));
        assert!(!prompt.contains("customer account"));
    }

    #[test]
    fn test_parse_intelligence_response_full() {
        let response = r#"Some preamble text

INTELLIGENCE
EXECUTIVE_ASSESSMENT:
Acme is in a strong position with growing adoption across teams.
The renewal trajectory is positive but champion departure poses risk.
END_EXECUTIVE_ASSESSMENT
RISK: Champion leaving Q2 | SOURCE: qbr-notes.md | URGENCY: critical
RISK: Budget uncertainty | SOURCE: email | URGENCY: watch
WIN: Expanded to 3 teams | SOURCE: capture | IMPACT: 20% seat growth
WIN: NPS improved to 85 | SOURCE: survey | IMPACT: advocacy
WORKING: Onboarding flow is smooth
WORKING: Support ticket volume down
NOT_WORKING: Reporting integration delayed
UNKNOWN: Budget for next fiscal year
STAKEHOLDER: Alice Chen | ROLE: VP Engineering | ASSESSMENT: Strong advocate, drives adoption | ENGAGEMENT: high
STAKEHOLDER: Bob Kim | ROLE: IT Director | ASSESSMENT: Cautious, needs ROI data | ENGAGEMENT: medium
VALUE: 2026-01-15 | Reduced onboarding time by 40% | SOURCE: qbr-deck.pdf | IMPACT: $50k savings
NEXT_MEETING_PREP: Review reporting blockers status
NEXT_MEETING_PREP: Prepare champion transition plan
NEXT_MEETING_PREP: Bring updated ROI metrics
COMPANY_DESCRIPTION: Enterprise SaaS platform for workflow automation
COMPANY_INDUSTRY: Technology / SaaS
COMPANY_SIZE: 500-1000
COMPANY_HQ: San Francisco, USA
COMPANY_CONTEXT: Recently acquired by larger corp, integration ongoing
END_INTELLIGENCE

Some trailing text"#;

        let manifest = vec![SourceManifestEntry {
            filename: "qbr-notes.md".to_string(),
            modified_at: "2026-01-30".to_string(),
            format: Some("markdown".to_string()),
            content_type: Some("qbr".to_string()),
            selected: true,
            skip_reason: None,
        }];

        let intel = parse_intelligence_response(response, "acme-corp", "account", 1, manifest)
            .expect("should parse");

        assert_eq!(intel.entity_id, "acme-corp");
        assert_eq!(intel.entity_type, "account");
        assert!(intel
            .executive_assessment
            .unwrap()
            .contains("champion departure"));

        assert_eq!(intel.risks.len(), 2);
        assert_eq!(intel.risks[0].text, "Champion leaving Q2");
        assert_eq!(intel.risks[0].urgency, "critical");
        assert_eq!(intel.risks[0].source.as_deref(), Some("qbr-notes.md"));
        assert_eq!(intel.risks[1].urgency, "watch");

        assert_eq!(intel.recent_wins.len(), 2);
        assert_eq!(
            intel.recent_wins[0].impact.as_deref(),
            Some("20% seat growth")
        );

        let state = intel.current_state.unwrap();
        assert_eq!(state.working.len(), 2);
        assert_eq!(state.not_working.len(), 1);
        assert_eq!(state.unknowns.len(), 1);

        assert_eq!(intel.stakeholder_insights.len(), 2);
        assert_eq!(intel.stakeholder_insights[0].name, "Alice Chen");
        assert_eq!(
            intel.stakeholder_insights[0].engagement.as_deref(),
            Some("high")
        );

        assert_eq!(intel.value_delivered.len(), 1);
        assert_eq!(
            intel.value_delivered[0].statement,
            "Reduced onboarding time by 40%"
        );

        let readiness = intel.next_meeting_readiness.unwrap();
        assert_eq!(readiness.prep_items.len(), 3);

        let ctx = intel.company_context.unwrap();
        assert_eq!(
            ctx.description.as_deref(),
            Some("Enterprise SaaS platform for workflow automation")
        );
        assert_eq!(ctx.industry.as_deref(), Some("Technology / SaaS"));
        assert_eq!(ctx.headquarters.as_deref(), Some("San Francisco, USA"));
        assert!(ctx.additional_context.is_some());
    }

    #[test]
    fn test_parse_intelligence_response_partial() {
        let response = "INTELLIGENCE\nEXECUTIVE_ASSESSMENT:\nBrief assessment.\nEND_EXECUTIVE_ASSESSMENT\nRISK: One risk | URGENCY: low\nEND_INTELLIGENCE";

        let intel = parse_intelligence_response(response, "beta", "project", 0, vec![])
            .expect("should parse");

        assert_eq!(
            intel.executive_assessment.as_deref(),
            Some("Brief assessment.")
        );
        assert_eq!(intel.risks.len(), 1);
        assert!(intel.recent_wins.is_empty());
        assert!(intel.stakeholder_insights.is_empty());
        assert!(intel.company_context.is_none());
    }

    #[test]
    fn test_parse_intelligence_response_no_block() {
        let response = "Just some random text with no structured block.";
        let result = parse_intelligence_response(response, "x", "account", 0, vec![]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No INTELLIGENCE block or JSON"));
    }

    #[test]
    fn test_parse_json_response_full() {
        let response = r#"```json
{
  "executiveAssessment": "Acme is in a strong position.\n\nChampion departure poses risk.",
  "sources": ["qbr-notes.md", "email"],
  "risks": [
    {"text": "Champion leaving Q2", "source": "qbr-notes.md", "urgency": "critical"},
    {"text": "Budget uncertainty", "source": "email", "urgency": "watch"}
  ],
  "recentWins": [
    {"text": "Expanded to 3 teams", "source": "capture", "impact": "20% seat growth"}
  ],
  "currentState": {
    "working": ["Onboarding flow"],
    "notWorking": ["Reporting integration"],
    "unknowns": ["Budget for next year"]
  },
  "stakeholderInsights": [
    {"name": "Alice Chen", "role": "VP Engineering", "assessment": "Strong advocate", "engagement": "high"}
  ],
  "valueDelivered": [
    {"date": "2026-01-15", "statement": "Reduced onboarding time by 40%", "source": "qbr-deck.pdf", "impact": "$50k savings"}
  ],
  "nextMeetingReadiness": {
    "prepItems": ["Review reporting blockers", "Prepare champion transition plan"]
  },
  "companyContext": {
    "description": "Enterprise SaaS platform",
    "industry": "Technology",
    "size": "500-1000",
    "headquarters": "San Francisco, USA"
  }
}
```"#;

        let intel = parse_intelligence_response(response, "acme", "account", 2, vec![])
            .expect("should parse JSON");

        assert_eq!(intel.entity_id, "acme");
        assert!(intel.executive_assessment.unwrap().contains("strong position"));
        assert_eq!(intel.risks.len(), 2);
        assert_eq!(intel.risks[0].urgency, "critical");
        assert_eq!(intel.recent_wins.len(), 1);
        assert_eq!(intel.recent_wins[0].impact.as_deref(), Some("20% seat growth"));
        let state = intel.current_state.unwrap();
        assert_eq!(state.working.len(), 1);
        assert_eq!(state.not_working.len(), 1);
        assert_eq!(state.unknowns.len(), 1);
        assert_eq!(intel.stakeholder_insights.len(), 1);
        assert_eq!(intel.stakeholder_insights[0].engagement.as_deref(), Some("high"));
        assert_eq!(intel.value_delivered.len(), 1);
        let readiness = intel.next_meeting_readiness.unwrap();
        assert_eq!(readiness.prep_items.len(), 2);
        let ctx = intel.company_context.unwrap();
        assert_eq!(ctx.industry.as_deref(), Some("Technology"));
    }

    #[test]
    fn test_parse_json_response_raw_no_fence() {
        let response = r#"{"executiveAssessment": "Brief.", "risks": [{"text": "One risk", "urgency": "low"}]}"#;

        let intel = parse_intelligence_response(response, "beta", "project", 0, vec![])
            .expect("should parse raw JSON");

        assert_eq!(intel.executive_assessment.as_deref(), Some("Brief."));
        assert_eq!(intel.risks.len(), 1);
        assert_eq!(intel.risks[0].urgency, "low");
    }

    #[test]
    fn test_parse_json_response_with_surrounding_text() {
        let response = r#"Here is the assessment:
{"executiveAssessment": "Assessment text.", "risks": [], "currentState": {"working": ["Item 1"], "notWorking": [], "unknowns": []}}
Hope this helps!"#;

        let intel = parse_intelligence_response(response, "gamma", "account", 0, vec![])
            .expect("should parse embedded JSON");

        assert_eq!(intel.executive_assessment.as_deref(), Some("Assessment text."));
        assert_eq!(intel.current_state.unwrap().working.len(), 1);
    }

    #[test]
    fn test_parse_risk_line() {
        let risk = parse_risk_line(" Budget cuts | SOURCE: email thread | URGENCY: critical");
        assert!(risk.is_some());
        let r = risk.unwrap();
        assert_eq!(r.text, "Budget cuts");
        assert_eq!(r.source.as_deref(), Some("email thread"));
        assert_eq!(r.urgency, "critical");
    }

    #[test]
    fn test_parse_risk_line_minimal() {
        let risk = parse_risk_line(" Risk text only");
        assert!(risk.is_some());
        let r = risk.unwrap();
        assert_eq!(r.text, "Risk text only");
        assert_eq!(r.urgency, "watch"); // default
        assert!(r.source.is_none());
    }

    #[test]
    fn test_parse_stakeholder_line() {
        let sh = parse_stakeholder_line(
            " Jane Doe | ROLE: CTO | ASSESSMENT: Key decision maker | ENGAGEMENT: high",
        );
        assert!(sh.is_some());
        let s = sh.unwrap();
        assert_eq!(s.name, "Jane Doe");
        assert_eq!(s.role.as_deref(), Some("CTO"));
        assert_eq!(s.engagement.as_deref(), Some("high"));
    }

    #[test]
    fn test_extract_multiline_field() {
        let block = "EXECUTIVE_ASSESSMENT:\nFirst paragraph.\n\nSecond paragraph.\nEND_EXECUTIVE_ASSESSMENT\nRISK: something";
        let result = extract_multiline_field(block, "EXECUTIVE_ASSESSMENT:");
        assert!(result.is_some());
        let text = result.unwrap();
        assert!(text.contains("First paragraph."));
        assert!(text.contains("Second paragraph."));
    }

    #[test]
    fn test_build_intelligence_context_account() {
        let db = test_db();
        let dir = tempfile::tempdir().expect("tempdir");
        let workspace = dir.path();

        let account = DbAccount {
            id: "test-acct".to_string(),
            name: "Test Acct".to_string(),
            lifecycle: Some("steady-state".to_string()),
            arr: Some(100_000.0),
            health: Some("green".to_string()),
            contract_start: None,
            contract_end: Some("2026-12-31".to_string()),
            nps: Some(75),
            tracker_path: None,
            parent_id: None,
            is_internal: false,
            updated_at: Utc::now().to_rfc3339(),
            archived: false,
            keywords: None,
            keywords_extracted_at: None,
        metadata: None,
        };
        db.upsert_account(&account).expect("upsert");

        let ctx = build_intelligence_context(
            workspace,
            &db,
            "test-acct",
            "account",
            Some(&account),
            None,
            None,
            None,
        );

        assert!(ctx.facts_block.contains("Health: green"));
        assert!(ctx.facts_block.contains("ARR: $100000"));
        assert!(ctx.facts_block.contains("Renewal: 2026-12-31"));
        assert!(ctx.prior_intelligence.is_none()); // initial mode
    }

    #[test]
    fn test_compute_signal_age() {
        let now = chrono::Utc::now();

        // Just now
        let recent = now.to_rfc3339();
        assert_eq!(compute_signal_age(&recent), "just now");

        // 2 days ago
        let two_days = (now - chrono::Duration::days(2)).to_rfc3339();
        assert_eq!(compute_signal_age(&two_days), "2 days ago");

        // 1 week ago
        let week = (now - chrono::Duration::days(8)).to_rfc3339();
        assert_eq!(compute_signal_age(&week), "1 week ago");

        // 2 weeks ago
        let two_weeks = (now - chrono::Duration::days(15)).to_rfc3339();
        assert_eq!(compute_signal_age(&two_weeks), "2 weeks ago");

        // Fallback for bad input
        assert_eq!(compute_signal_age("not-a-date"), "not-a-date");
    }
}
