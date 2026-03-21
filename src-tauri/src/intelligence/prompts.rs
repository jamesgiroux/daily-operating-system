//! Intelligence prompt building, response parsing, and enrichment orchestration.
//!
//! Extracted from entity_intel.rs. Contains:
//! - IntelligenceContext assembly from SQLite + files
//! - Prompt construction (initial and incremental modes)
//! - AI response parsing (JSON-first with pipe-delimited fallback)
//! - Entity enrichment orchestrator

use std::path::Path;

use chrono::{Local, TimeZone, Utc};
use serde::Deserialize;

use crate::db::{ActionDb, DbAccount, DbProject};
use crate::helpers::strip_conferencing_noise;
use crate::util::{sanitize_external_field, wrap_user_data, INJECTION_PREAMBLE};

use super::io::*;

/// Maximum bytes of file content to include in the intelligence prompt context.
/// Keeps prompt size manageable (~10KB) while preserving the most relevant signals.
const MAX_CONTEXT_BYTES: usize = 10_000;

// =============================================================================
// Intelligence Context Assembly (I131)
// =============================================================================

/// Assembled signals for the intelligence enrichment prompt.
#[derive(Debug, Default, Clone)]
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
    /// Linked stakeholders from account_stakeholders/entity_members + people.
    pub stakeholders: String,
    /// Canonical contact names with IDs for stakeholder reconciliation (I420).
    pub canonical_contacts: Option<String>,
    /// Deterministic attendance-backed stakeholder presence lines (I527).
    pub verified_stakeholder_presence: Option<String>,
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
    /// Portfolio context for parent accounts (I384).
    /// Contains children's intelligence summaries and signal data for portfolio synthesis.
    pub portfolio_children_context: Option<String>,
    /// Person relationship edges for network intelligence (I391).
    /// Pre-formatted string of edges with effective confidence and types.
    pub relationship_edges: Option<String>,
    /// User professional context block for personalized enrichment (I412).
    pub user_context: Option<String>,
    /// Entity-specific context entries from entity_context_entries table.
    pub entity_context: Option<String>,
    /// I508c: Dimension-aware gap queries for Glean fan-out.
    pub gap_queries: Vec<GapQueryItem>,
    /// I499: Pre-computed account health from algorithmic scoring.
    pub computed_health: Option<super::io::AccountHealth>,
    /// I500: Org-level health data from external sources (Glean/CRM).
    pub org_health: Option<super::io::OrgHealthData>,
    /// I555: Additional context blocks (engagement patterns, champion health, commitments).
    pub extra_blocks: Vec<String>,
}

/// I508c structured gap query item used for local ranking + remote fan-out.
#[derive(Debug, Clone)]
pub struct GapQueryItem {
    /// Dimension key when the gap query targets a specific intelligence dimension.
    pub dimension: Option<String>,
    /// Search query text used for vector ranking / Glean search.
    pub query: String,
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
            .get_meeting_history(entity_id, 90, 20)
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
                let meeting_time = format_meeting_time_for_prompt(&m.start_time);
                let doc = match (&m.summary, &m.transcript_path) {
                    (Some(s), _) if !s.is_empty() => s.clone(),
                    (_, Some(p)) if !p.is_empty() => "transcript available".to_string(),
                    _ => String::new(),
                };
                if doc.is_empty() {
                    format!("- {} | {}", meeting_time, m.title)
                } else {
                    format!("- {} | {} | {}", meeting_time, m.title, doc)
                }
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

    // --- I555: Meeting engagement patterns (last 5 meetings with dynamics) ---
    if entity_type == "account" {
        if let Ok(mut stmt) = db.conn.prepare(
            "SELECT m.start_time, m.title, mid.talk_balance_customer_pct, mid.talk_balance_internal_pct,
                    mid.question_density, mid.decision_maker_active, mid.forward_looking,
                    mch.champion_name, mch.champion_status
             FROM meeting_interaction_dynamics mid
             JOIN meetings m ON m.id = mid.meeting_id
             JOIN meeting_entities me ON me.meeting_id = m.id AND me.entity_id = ?1
             LEFT JOIN meeting_champion_health mch ON mch.meeting_id = m.id
             ORDER BY m.start_time DESC LIMIT 5"
        ) {
            let rows: Vec<(String, String, Option<i32>, Option<i32>, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>)> =
                stmt.query_map(rusqlite::params![entity_id], |row| {
                    Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?, row.get(7)?, row.get(8)?))
                }).map(|r| r.filter_map(|r| r.ok()).collect()).unwrap_or_default();

            if !rows.is_empty() {
                let mut lines = vec!["## Meeting Engagement Patterns (last 5 meetings)".to_string()];
                for (date, title, cust_pct, int_pct, _qd, _dma, fl, champ_name, champ_status) in &rows {
                    let short_date = date.split('T').next().unwrap_or(date);
                    let talk = match (cust_pct, int_pct) {
                        (Some(c), Some(i)) => format!("Talk: {}% customer / {}% internal", c, i),
                        _ => "Talk: unknown".to_string(),
                    };
                    let champion = match (champ_name, champ_status) {
                        (Some(n), Some(s)) => format!("Champion {n}: {s}"),
                        _ => "Champion: n/a".to_string(),
                    };
                    let fwd = fl.as_deref().unwrap_or("unknown");
                    lines.push(format!("- {short_date} | {title} | {talk} | {champion} | Forward-looking: {fwd}"));
                }
                ctx.extra_blocks.push(lines.join("\n"));
            }
        }

        // --- I555: Champion health trend ---
        if let Ok(mut stmt) = db.conn.prepare(
            "SELECT m.start_time, mch.champion_name, mch.champion_status, mch.champion_evidence
             FROM meeting_champion_health mch
             JOIN meetings m ON m.id = mch.meeting_id
             JOIN meeting_entities me ON me.meeting_id = m.id AND me.entity_id = ?1
             WHERE mch.champion_name IS NOT NULL
             ORDER BY m.start_time DESC LIMIT 5",
        ) {
            let rows: Vec<(String, String, String, Option<String>)> = stmt
                .query_map(rusqlite::params![entity_id], |row| {
                    Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
                })
                .map(|r| r.filter_map(|r| r.ok()).collect())
                .unwrap_or_default();

            if !rows.is_empty() {
                let champion_name = &rows[0].1;
                let statuses: Vec<String> = rows
                    .iter()
                    .map(|(d, _, s, _)| {
                        let short = d.split('T').next().unwrap_or(d);
                        format!("{s} ({short})")
                    })
                    .collect();
                let mut lines = vec![
                    "## Champion Health Trend".to_string(),
                    format!("{champion_name} — {}", statuses.join(", ")),
                ];
                // Add evidence for weak/lost entries
                for (date, _, status, evidence) in &rows {
                    if (status == "weak" || status == "lost") && evidence.is_some() {
                        let short = date.split('T').next().unwrap_or(date);
                        lines.push(format!(
                            "  {short} ({status}): {}",
                            evidence.as_deref().unwrap_or("")
                        ));
                    }
                }
                ctx.extra_blocks.push(lines.join("\n"));
            }
        }

        // --- I555: Open commitments from prior meetings ---
        if let Ok(mut stmt) = db.conn.prepare(
            "SELECT title, owner, target_date, source
             FROM captured_commitments
             WHERE account_id = ?1 AND consumed = 0
             ORDER BY created_at DESC LIMIT 10",
        ) {
            let rows: Vec<(String, Option<String>, Option<String>, Option<String>)> = stmt
                .query_map(rusqlite::params![entity_id], |row| {
                    Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
                })
                .map(|r| r.filter_map(|r| r.ok()).collect())
                .unwrap_or_default();

            if !rows.is_empty() {
                let mut lines = vec!["## Open Commitments (from prior meetings)".to_string()];
                for (title, owner, target, source) in &rows {
                    let owner_str = owner.as_deref().unwrap_or("unassigned");
                    let target_str = target.as_deref().unwrap_or("no target date");
                    let source_str = source
                        .as_deref()
                        .map(|s| format!(", from {s}"))
                        .unwrap_or_default();
                    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                    let overdue = target
                        .as_deref()
                        .map(|t| t < today.as_str())
                        .unwrap_or(false);
                    let tag = if overdue { " [OVERDUE]" } else { "" };
                    lines.push(format!("- \"{title}\" — owned_by: {owner_str}, target: {target_str}{source_str}{tag}"));
                }
                ctx.extra_blocks.push(lines.join("\n"));
            }
        }
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
                                    let email = s.sender_email.as_deref().unwrap_or(&person.email);
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

        // I420: Canonical contacts for stakeholder reconciliation
        let canonical_lines: Vec<String> = people
            .iter()
            .map(|p| {
                format!(
                    "- \"{}\" (role: {}, id: {}, email: {})",
                    p.name,
                    p.role.as_deref().unwrap_or("unknown"),
                    p.id,
                    p.email
                )
            })
            .collect();
        ctx.canonical_contacts = Some(canonical_lines.join("\n"));
    }

    // I527: Deterministic stakeholder meeting presence lines for contradiction-resistant prompts.
    if entity_type == "account" || entity_type == "project" {
        if let Ok(facts) = crate::intelligence::build_fact_context(db, entity_id, entity_type) {
            let lines = crate::intelligence::format_verified_presence_lines(&facts, 8);
            if !lines.is_empty() {
                ctx.verified_stakeholder_presence = Some(lines.join("\n"));
            }
        }
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

    // --- Person relationship edges (I391) ---
    if entity_type == "person" {
        if let Ok(edges) = db.get_relationships_for_person(entity_id) {
            if !edges.is_empty() {
                let mut lines: Vec<String> = Vec::new();
                for edge in &edges {
                    let other_id = if edge.from_person_id == entity_id {
                        &edge.to_person_id
                    } else {
                        &edge.from_person_id
                    };
                    let other_name = db
                        .get_person(other_id)
                        .ok()
                        .flatten()
                        .map(|p| p.name.clone())
                        .unwrap_or_else(|| other_id.clone());
                    let context = match (&edge.context_entity_id, &edge.context_entity_type) {
                        (Some(cid), Some(ctype)) => format!(" [context: {} {}]", ctype, cid),
                        _ => String::new(),
                    };
                    let direction_label = if edge.direction == "symmetric" {
                        "↔"
                    } else {
                        "→"
                    };
                    lines.push(format!(
                        "- {} {} {} ({}, confidence: {:.2}, source: {}){}",
                        if edge.from_person_id == entity_id {
                            "self"
                        } else {
                            &other_name
                        },
                        direction_label,
                        if edge.to_person_id == entity_id {
                            "self"
                        } else {
                            &other_name
                        },
                        edge.relationship_type,
                        edge.effective_confidence,
                        edge.source,
                        context,
                    ));
                }
                // Replace IDs with names for readability
                let header = format!(
                    "{} relationship edges (person-to-person):\n{}",
                    edges.len(),
                    lines.join("\n")
                );
                ctx.relationship_edges = Some(header);
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

    let gap_queries = semantic_gap_queries(prior);
    let semantic_query = gap_queries
        .first()
        .map(|q| q.query.clone())
        .unwrap_or_default();
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
            let skip_reason = if is_selected {
                None
            } else if f.summary.is_none() {
                Some("no_summary".to_string())
            } else {
                Some("budget".to_string())
            };
            SourceManifestEntry {
                filename: f.filename.clone(),
                modified_at: f.modified_at.clone(),
                format: Some(f.format.clone()),
                content_type: Some(f.content_type.clone()),
                selected: is_selected,
                skip_reason,
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
        let transcript_cutoff = (Utc::now() - chrono::Duration::days(365)).to_rfc3339();
        for tf in transcript_files
            .into_iter()
            .filter(|f| content_date_rfc3339(&f.filename, &f.modified_at) >= transcript_cutoff)
            .take(3)
        {
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

    // --- Recent transcript sentiment captures (I509) ---
    if entity_type == "account" {
        if let Ok(captures) = db.get_captures_for_account(entity_id, 90) {
            let sentiment_captures: Vec<_> = captures
                .iter()
                .filter(|c| c.capture_type == "sentiment")
                .take(5)
                .collect();
            if !sentiment_captures.is_empty() {
                let mut parts = Vec::new();
                for cap in &sentiment_captures {
                    parts.push(format!(
                        "- {} ({}): {}",
                        cap.meeting_title, cap.captured_at, cap.content
                    ));
                }
                ctx.recent_transcripts.push_str(&format!(
                    "\n\n## Recent Meeting Sentiment Signals\n{}",
                    parts.join("\n")
                ));
            }
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
                let local_start = format_meeting_time_for_prompt(&m.start_time);
                let mut next = format!("{} — {}", local_start, m.title);
                if let Some(ref desc) = m.description {
                    let cleaned = strip_conferencing_noise(desc);
                    if !cleaned.trim().is_empty() {
                        next.push_str(&format!("\nMeeting Description:\n{}", cleaned.trim()));
                    }
                }
                ctx.next_meeting = Some(next);
            }
        }
    }

    // --- Portfolio context for parent accounts (I384) ---
    // If this account has children, gather their intelligence summaries and signals
    // for portfolio-level synthesis.
    if entity_type == "account" {
        if let Ok(children) = db.get_child_accounts(entity_id) {
            if !children.is_empty() {
                ctx.portfolio_children_context =
                    Some(build_portfolio_children_context(db, &children));
            }
        }
    }

    // --- Portfolio context for parent projects (I388) ---
    // Mirror of account hierarchy: if this project has children, gather their
    // intelligence summaries and signals for portfolio-level synthesis.
    if entity_type == "project" {
        if let Ok(children) = db.get_child_projects(entity_id) {
            if !children.is_empty() {
                ctx.portfolio_children_context =
                    Some(build_project_portfolio_children_context(db, &children));
            }
        }
    }

    // --- User professional context (I412 + I417) ---
    let entity_name_for_ctx = match entity_type {
        "account" => account.map(|a| a.name.as_str()),
        "project" => project.map(|p| p.name.as_str()),
        _ => None,
    };
    ctx.user_context = build_user_context_block(db, embedding_model, entity_name_for_ctx);

    // --- Entity-specific context entries ---
    let entity_entries =
        super::user_context::get_entity_context_for_prompt(db, entity_type, entity_id);
    if !entity_entries.is_empty() {
        let mut block = String::new();
        for (title, content) in &entity_entries {
            block.push_str(&format!("### {}\n{}\n\n", title, content));
        }
        ctx.entity_context = Some(block);
    }

    // I508c: Store gap queries for Glean fan-out
    ctx.gap_queries = gap_queries;

    ctx
}

/// Build a user professional context block from the user_entity table (I412).
///
/// Returns None when all fields are NULL (prompt is identical to pre-v0.14.0).
/// Target: ~150-300 tokens for context injection.
/// When an embedding model and entity name are available, appends top-2 semantically
/// relevant user context entries as a "Professional Knowledge" sub-section (I417).
fn build_user_context_block(
    db: &ActionDb,
    embedding_model: Option<&crate::embeddings::EmbeddingModel>,
    entity_name: Option<&str>,
) -> Option<String> {
    let entity = crate::services::user_entity::get_user_entity_from_db(db).ok()?;

    let mut parts = Vec::new();

    // Identity line
    let mut identity = Vec::new();
    if let Some(ref name) = entity.name {
        identity.push(name.clone());
    }
    if let Some(ref title) = entity.title {
        if let Some(ref company) = entity.company {
            identity.push(format!("{} at {}", title, company));
        } else {
            identity.push(title.clone());
        }
    } else if let Some(ref company) = entity.company {
        identity.push(format!("works at {}", company));
    }
    if !identity.is_empty() {
        parts.push(format!("User: {}", identity.join(", ")));
    }

    if let Some(ref focus) = entity.focus {
        parts.push(format!("Current focus: {}", focus));
    }
    if let Some(ref role_desc) = entity.role_description {
        parts.push(format!("Role: {}", role_desc));
    }
    if let Some(ref measured) = entity.how_im_measured {
        parts.push(format!("Measured by: {}", measured));
    }
    if let Some(ref vp) = entity.value_proposition {
        parts.push(format!("Product value proposition: {}", vp));
    }
    if let Some(ref success) = entity.success_definition {
        parts.push(format!("Success definition: {}", success));
    }
    if let Some(ref product) = entity.product_context {
        parts.push(format!("Product context: {}", product));
    }
    if let Some(ref pricing) = entity.pricing_model {
        parts.push(format!("Pricing model: {}", pricing));
    }
    if let Some(ref diff) = entity.differentiators {
        if let Ok(arr) = serde_json::from_str::<Vec<String>>(diff) {
            if !arr.is_empty() {
                parts.push(format!("Key differentiators: {}", arr.join(", ")));
            }
        }
    }
    if let Some(ref obj) = entity.objections {
        if let Ok(arr) = serde_json::from_str::<Vec<String>>(obj) {
            if !arr.is_empty() {
                parts.push(format!("Common objections: {}", arr.join(", ")));
            }
        }
    }
    if let Some(ref comp) = entity.competitive_context {
        parts.push(format!("Competitive context: {}", comp));
    }
    if let Some(ref priorities) = entity.current_priorities {
        parts.push(format!("Current priorities: {}", priorities));
    }

    // Annual priorities (year-level bets)
    if let Some(ref ap) = entity.annual_priorities {
        if let Ok(arr) = serde_json::from_str::<Vec<crate::types::AnnualPriority>>(ap) {
            if !arr.is_empty() {
                let items: Vec<String> = arr.iter().map(|p| format!("- {}", p.text)).collect();
                parts.push(format!("Annual priorities:\n{}", items.join("\n")));
            }
        }
    }

    // Quarterly priorities (current quarter focus)
    if let Some(ref qp) = entity.quarterly_priorities {
        if let Ok(arr) = serde_json::from_str::<Vec<crate::types::QuarterlyPriority>>(qp) {
            if !arr.is_empty() {
                let items: Vec<String> = arr.iter().map(|p| format!("- {}", p.text)).collect();
                parts.push(format!("This quarter:\n{}", items.join("\n")));
            }
        }
    }

    // I417: Semantic retrieval of user context entries relevant to this entity
    // I413 AC4: Also search file attachments for relevant content
    if let Some(name) = entity_name {
        let mut matches = super::user_context::search_user_context(
            db,
            embedding_model,
            name,
            2,    // top-2 results
            0.82, // similarity threshold — raised from 0.70 to prevent cross-entity bleed
        );

        // Search user attachments (file embeddings)
        let attachment_matches = super::user_context::search_user_attachments(
            db,
            embedding_model,
            name,
            2,    // top-2 attachment chunks
            0.82, // same threshold for consistency
        );

        // Combine both sources, sort by score, keep top-4 total
        matches.extend(attachment_matches);
        matches.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        matches.truncate(4);

        if !matches.is_empty() {
            let mut knowledge = String::from("Professional knowledge:");
            for m in &matches {
                let source_label = if m.source == "attachment" {
                    " (document)"
                } else {
                    ""
                };
                knowledge.push_str(&format!("\n- {}{}: {}", m.title, source_label, m.content));
            }
            parts.push(knowledge);
        }
    }

    if parts.is_empty() {
        return None;
    }

    Some(parts.join("\n"))
}

/// Maximum context bytes for portfolio children data (I384).
///
/// Parent accounts with many children could exceed the standard MAX_CONTEXT_BYTES.
/// Tiered approach: full executive assessment + signals for first 8 children sorted
/// by signal recency, then name-only with health for the rest.
///
/// Rationale: A typical child's context block is ~500-800 bytes (assessment excerpt +
/// signals + health facts). At 8 children with full detail, that's ~5KB. Real parent
/// accounts rarely exceed 10-15 children. 20KB budget accommodates the largest
/// portfolios while keeping total prompt size reasonable alongside the parent's
/// own entity context (~10KB).
const MAX_PORTFOLIO_CONTEXT_BYTES: usize = 20_000;

/// Build portfolio context from children's intelligence for a parent account (I384).
///
/// Gathers each child's intelligence.json (from DB cache) and active signals,
/// then formats them as a context block for the parent's enrichment prompt.
/// Sorted by signal recency so the most active children get full detail.
fn build_portfolio_children_context(db: &ActionDb, children: &[DbAccount]) -> String {
    // Collect child data: (name, id, health, intel, signal_count, latest_signal_time)
    let mut child_data: Vec<(
        &str,         // name
        &str,         // id
        Option<&str>, // health
        Option<IntelligenceJson>,
        Vec<crate::signals::bus::SignalEvent>,
    )> = Vec::new();

    for child in children {
        let intel = db.get_entity_intelligence(&child.id).ok().flatten();
        let signals =
            crate::signals::bus::get_active_signals(db, "account", &child.id).unwrap_or_default();
        child_data.push((
            &child.name,
            &child.id,
            child.health.as_deref(),
            intel,
            signals,
        ));
    }

    // Sort by signal recency (most recent first), then by name
    child_data.sort_by(|a, b| {
        let a_latest =
            a.4.iter()
                .map(|s| s.created_at.as_str())
                .max()
                .unwrap_or("");
        let b_latest =
            b.4.iter()
                .map(|s| s.created_at.as_str())
                .max()
                .unwrap_or("");
        b_latest.cmp(a_latest).then(a.0.cmp(b.0))
    });

    let mut parts: Vec<String> = Vec::new();
    let mut total_bytes = 0usize;
    let mut full_detail_count = 0usize;

    for (name, id, health, intel, signals) in &child_data {
        let health_str = health.unwrap_or("unknown");

        // After 8 children with full detail, switch to summary-only
        if full_detail_count >= 8 || total_bytes >= MAX_PORTFOLIO_CONTEXT_BYTES {
            let summary = format!("- {} [{}]: health={}", name, id, health_str);
            let summary_bytes = summary.len();
            if total_bytes + summary_bytes > MAX_PORTFOLIO_CONTEXT_BYTES {
                parts.push(format!(
                    "... and {} more children (truncated for context budget)",
                    child_data.len() - full_detail_count
                ));
                break;
            }
            parts.push(summary);
            total_bytes += summary_bytes;
            continue;
        }

        let mut block = format!("### {} [{}]\nHealth: {}\n", name, id, health_str);

        // Executive assessment excerpt (first 300 chars)
        if let Some(ref intel_json) = intel {
            if let Some(ref assessment) = intel_json.executive_assessment {
                let excerpt = if assessment.len() > 300 {
                    format!("{}...", &assessment[..300])
                } else {
                    assessment.clone()
                };
                block.push_str(&format!("Assessment: {}\n", excerpt));
            }

            // Risks summary
            if !intel_json.risks.is_empty() {
                let risk_lines: Vec<String> = intel_json
                    .risks
                    .iter()
                    .take(3)
                    .map(|r| format!("  - [{}] {}", r.urgency, r.text))
                    .collect();
                block.push_str("Risks:\n");
                block.push_str(&risk_lines.join("\n"));
                block.push('\n');
            }
        }

        // Active signals (up to 5)
        if !signals.is_empty() {
            let signal_lines: Vec<String> = signals
                .iter()
                .take(5)
                .map(|s| {
                    format!(
                        "  - [{}] {} ({})",
                        s.signal_type,
                        s.value.as_deref().unwrap_or(""),
                        s.created_at
                    )
                })
                .collect();
            block.push_str("Signals:\n");
            block.push_str(&signal_lines.join("\n"));
            block.push('\n');
        }

        let block_bytes = block.len();
        if total_bytes + block_bytes > MAX_PORTFOLIO_CONTEXT_BYTES {
            // Exceeded budget, switch to summary for remaining
            parts.push(format!("- {} [{}]: health={}", name, id, health_str));
            total_bytes += name.len() + id.len() + health_str.len() + 30;
            continue;
        }

        parts.push(block);
        total_bytes += block_bytes;
        full_detail_count += 1;
    }

    parts.join("\n")
}

/// Build portfolio context from children's intelligence for a parent project (I388).
///
/// Mirrors `build_portfolio_children_context` but uses project-appropriate vocabulary
/// (status instead of health, no ARR).
fn build_project_portfolio_children_context(db: &ActionDb, children: &[DbProject]) -> String {
    // Collect child data: (name, id, status, intel, signals)
    let mut child_data: Vec<(
        &str, // name
        &str, // id
        &str, // status
        Option<IntelligenceJson>,
        Vec<crate::signals::bus::SignalEvent>,
    )> = Vec::new();

    for child in children {
        let intel = db.get_entity_intelligence(&child.id).ok().flatten();
        let signals =
            crate::signals::bus::get_active_signals(db, "project", &child.id).unwrap_or_default();
        child_data.push((&child.name, &child.id, &child.status, intel, signals));
    }

    // Sort by signal recency (most recent first), then by name
    child_data.sort_by(|a, b| {
        let a_latest =
            a.4.iter()
                .map(|s| s.created_at.as_str())
                .max()
                .unwrap_or("");
        let b_latest =
            b.4.iter()
                .map(|s| s.created_at.as_str())
                .max()
                .unwrap_or("");
        b_latest.cmp(a_latest).then(a.0.cmp(b.0))
    });

    let mut parts: Vec<String> = Vec::new();
    let mut total_bytes = 0usize;
    let mut full_detail_count = 0usize;

    for (name, id, status, intel, signals) in &child_data {
        // After 8 children with full detail, switch to summary-only
        if full_detail_count >= 8 || total_bytes >= MAX_PORTFOLIO_CONTEXT_BYTES {
            let summary = format!("- {} [{}]: status={}", name, id, status);
            let summary_bytes = summary.len();
            if total_bytes + summary_bytes > MAX_PORTFOLIO_CONTEXT_BYTES {
                parts.push(format!(
                    "... and {} more sub-projects (truncated for context budget)",
                    child_data.len() - full_detail_count
                ));
                break;
            }
            parts.push(summary);
            total_bytes += summary_bytes;
            continue;
        }

        let mut block = format!("### {} [{}]\nStatus: {}\n", name, id, status);

        // Executive assessment excerpt (first 300 chars)
        if let Some(ref intel_json) = intel {
            if let Some(ref assessment) = intel_json.executive_assessment {
                let excerpt = if assessment.len() > 300 {
                    format!("{}...", &assessment[..300])
                } else {
                    assessment.clone()
                };
                block.push_str(&format!("Assessment: {}\n", excerpt));
            }

            // Risks summary
            if !intel_json.risks.is_empty() {
                let risk_lines: Vec<String> = intel_json
                    .risks
                    .iter()
                    .take(3)
                    .map(|r| format!("  - [{}] {}", r.urgency, r.text))
                    .collect();
                block.push_str("Risks:\n");
                block.push_str(&risk_lines.join("\n"));
                block.push('\n');
            }
        }

        // Active signals (up to 5)
        if !signals.is_empty() {
            let signal_lines: Vec<String> = signals
                .iter()
                .take(5)
                .map(|s| {
                    format!(
                        "  - [{}] {} ({})",
                        s.signal_type,
                        s.value.as_deref().unwrap_or(""),
                        s.created_at
                    )
                })
                .collect();
            block.push_str("Signals:\n");
            block.push_str(&signal_lines.join("\n"));
            block.push('\n');
        }

        let block_bytes = block.len();
        if total_bytes + block_bytes > MAX_PORTFOLIO_CONTEXT_BYTES {
            parts.push(format!("- {} [{}]: status={}", name, id, status));
            total_bytes += name.len() + id.len() + status.len() + 30;
            continue;
        }

        parts.push(block);
        total_bytes += block_bytes;
        full_detail_count += 1;
    }

    parts.join("\n")
}

/// I508c: Dimension-aware semantic gap queries for entity enrichment.
///
/// Returns structured gap query items — the first query is used for local vector
/// ranking, and the full set is used for Glean fan-out.
fn semantic_gap_queries(prior: Option<&IntelligenceJson>) -> Vec<GapQueryItem> {
    let mut queries = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut push_query = |dimension: Option<&str>, query: &str| {
        if seen.insert(query.to_string()) {
            queries.push(GapQueryItem {
                dimension: dimension.map(|d| d.to_string()),
                query: query.to_string(),
            });
        }
    };

    // Core query — always included
    push_query(None, "account status risks wins blockers next steps");

    if let Some(p) = prior {
        // Legacy checks
        if p.risks.is_empty() {
            push_query(Some("risks"), "risks concerns blockers challenges");
        }
        if p.recent_wins.is_empty() {
            push_query(Some("recent_wins"), "recent wins outcomes delivered value");
        }
        if p.current_state.is_none() {
            push_query(Some("current_state"), "working not working unknowns");
        }
        // I508a dimension-aware gap checks
        if p.competitive_context.is_empty() {
            push_query(
                Some("competitive_context"),
                "competitive landscape alternatives threats",
            );
        }
        if p.strategic_priorities.is_empty() {
            push_query(
                Some("strategic_priorities"),
                "strategic priorities initiatives roadmap goals",
            );
        }
        if p.coverage_assessment.is_none() {
            push_query(
                Some("coverage_assessment"),
                "stakeholder map org chart role coverage",
            );
        }
        if p.organizational_changes.is_empty() {
            push_query(
                Some("organizational_changes"),
                "leadership changes reorg hiring departures",
            );
        }
        if p.meeting_cadence.is_none() {
            push_query(
                Some("meeting_cadence"),
                "meeting frequency engagement cadence",
            );
        }
        if p.blockers.is_empty() {
            push_query(Some("blockers"), "blockers obstacles delays impediments");
        }
        if p.contract_context.is_none() {
            push_query(
                Some("contract_context"),
                "contract renewal ARR pricing commercial terms",
            );
        }
        if p.expansion_signals.is_empty() {
            push_query(
                Some("expansion_signals"),
                "expansion upsell growth opportunity",
            );
        }
        if p.support_health.is_none() {
            push_query(
                Some("support_health"),
                "support tickets SLA issues incidents",
            );
        }
        if p.nps_csat.is_none() {
            push_query(
                Some("nps_csat"),
                "NPS CSAT satisfaction survey score feedback",
            );
        }
    } else {
        // Initial enrichment — broad set
        push_query(
            Some("executive_assessment"),
            "executive assessment context renewal sentiment",
        );
        push_query(
            Some("competitive_context"),
            "competitive landscape alternatives threats",
        );
        push_query(
            Some("strategic_priorities"),
            "strategic priorities initiatives roadmap",
        );
        push_query(
            Some("contract_context"),
            "contract renewal ARR pricing terms",
        );
        push_query(Some("support_health"), "support tickets satisfaction NPS");
    }

    queries
}

fn format_meeting_time_for_prompt(raw: &str) -> String {
    let value = raw.trim();
    if value.is_empty() {
        return raw.to_string();
    }

    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(value) {
        return dt
            .with_timezone(&Local)
            .format("%Y-%m-%d %-I:%M %p %Z")
            .to_string();
    }

    for fmt in ["%Y-%m-%d %H:%M:%S", "%Y-%m-%d %H:%M"] {
        if let Ok(ndt) = chrono::NaiveDateTime::parse_from_str(value, fmt) {
            if let Some(local_dt) = Local.from_local_datetime(&ndt).single() {
                return local_dt.format("%Y-%m-%d %-I:%M %p %Z").to_string();
            }
            return Local
                .from_utc_datetime(&ndt)
                .format("%Y-%m-%d %-I:%M %p %Z")
                .to_string();
        }
    }

    raw.to_string()
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
    build_intelligence_prompt_inner(
        entity_name,
        entity_type,
        ctx,
        relationship,
        vocabulary,
        None,
    )
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
    build_intelligence_prompt_inner(
        entity_name,
        entity_type,
        ctx,
        relationship,
        vocabulary,
        briefing_emphasis,
    )
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
        "account" => match relationship {
            Some("partner") => "partner organization",
            Some("internal") => "internal organization",
            _ => vocabulary
                .map(|v| v.entity_noun.as_str())
                .unwrap_or("customer account"),
        },
        "project" => "project",
        "person" => match relationship {
            Some("internal") => "internal teammate / colleague",
            Some("external") => "external stakeholder / customer contact",
            _ => "professional contact",
        },
        _ => "entity",
    };

    let mut prompt = String::with_capacity(4096);

    // I468: Injection resistance preamble
    prompt.push_str(INJECTION_PREAMBLE);

    // System context
    prompt.push_str(&format!(
        "You are building an intelligence assessment for the {label} \"{name}\".\n\n",
        label = entity_label,
        name = sanitize_external_field(entity_name)
    ));
    let local_now = Local::now();
    prompt.push_str(&format!(
        "Current local datetime: {}.\n\
         System timezone: {} (UTC offset {}). Interpret all meeting times in this timezone.\n\n",
        local_now.format("%Y-%m-%d %-I:%M %p"),
        local_now.format("%Z"),
        local_now.format("%:z"),
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
            prompt.push_str(&format!("Assessment emphasis: {}\n", emphasis,));
        }
        prompt.push('\n');
    }

    // I412: User professional context — personalizes assessment framing
    if let Some(ref user_ctx) = ctx.user_context {
        prompt.push_str("## Your Professional Context\n");
        prompt.push_str(&wrap_user_data(user_ctx));
        prompt.push_str("\n\n");
    }

    if let Some(ref entity_ctx) = ctx.entity_context {
        prompt.push_str("## User Notes About This Entity\n");
        prompt.push_str(&wrap_user_data(entity_ctx));
        prompt.push_str("\n\n");
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

    // I420: Canonical contacts for deterministic stakeholder naming
    if let Some(ref contacts) = ctx.canonical_contacts {
        prompt.push_str(
            "## Known Contacts (canonical names)\n\
             The following people are confirmed contacts for this entity. When \
             referencing any of them in your stakeholder analysis, use their \
             canonical name EXACTLY as listed. Do not create duplicate entries \
             using nicknames, abbreviations, or partial names.\n\n",
        );
        prompt.push_str(&wrap_user_data(contacts));
        prompt.push_str("\n\n");
    }

    // I527: Attendance-backed presence facts. These are deterministic checks, not model inference.
    if let Some(ref verified) = ctx.verified_stakeholder_presence {
        prompt.push_str(
            "## Verified Stakeholder Meeting Presence\n\
             These lines are deterministic meeting-attendance facts from local records.\n\
             Use them as source-of-truth for attendance claims.\n\n",
        );
        prompt.push_str(&wrap_user_data(verified));
        prompt.push_str("\n\n");
    }

    // File manifest (always shown so Claude knows what exists)
    if !ctx.file_manifest.is_empty() {
        prompt.push_str("## Workspace Files [source: local_file, confidence: 0.85]\n");
        prompt.push_str("Items derived from these files MUST use itemSource.source = \"local_file\" with confidence 0.85.\n");
        for f in &ctx.file_manifest {
            let ct = f.content_type.as_deref().unwrap_or("general");
            prompt.push_str(&format!(
                "- {} [{}] ({}, {})\n",
                sanitize_external_field(&f.filename),
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

    // Portfolio children context for parent accounts (I384)
    if let Some(ref portfolio_ctx) = ctx.portfolio_children_context {
        prompt.push_str("## Portfolio: Child Account Intelligence\n");
        prompt.push_str("This is a PARENT account with child business units. ");
        prompt
            .push_str("Use the intelligence data below to synthesize a portfolio-level view.\n\n");
        prompt.push_str(&wrap_user_data(portfolio_ctx));
        prompt.push_str("\n\n");
    }

    // I391: Person relationship edges for network intelligence
    if let Some(ref edges_ctx) = ctx.relationship_edges {
        prompt.push_str("## Relationship Network (person-to-person edges)\n");
        prompt.push_str("These are typed relationship edges with confidence scores. ");
        prompt.push_str("Use them to assess network health, risks, and opportunities.\n\n");
        prompt.push_str(&wrap_user_data(edges_ctx));
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

    // I555: Extra context blocks (engagement patterns, champion health, commitments)
    for block in &ctx.extra_blocks {
        prompt.push_str(&wrap_user_data(block));
        prompt.push_str("\n\n");
    }

    // Field-level deduplication rules
    prompt.push_str(
        "FIELD SCOPING RULES (critical — avoid redundancy across fields):\n\
         Each item should appear in exactly ONE field. Do not repeat the same event, \
         commitment, or concern across multiple fields. Cross-reference when relevant \
         (e.g., renewalOutlook.riskFactors can say \"champion transition\" without \
         duplicating the full description from organizationalChanges).\n\
         - risks[]: Account-level THREATS to the relationship. Not blockers (those have owners), \
           not commitments (those have due dates), not current-state observations.\n\
         - currentState.notWorking[]: Present-tense observations about what is NOT going well \
           RIGHT NOW. Not future risks, not blockers waiting on someone, not commitments.\n\
         - blockers[]: Specific OBSTACLES with an identifiable owner blocking progress on \
           a known initiative. Must have an owner and a since-date. Not general concerns.\n\
         - openCommitments[]: PROMISES made by either side with a deliverable and timeline. \
           Not strategic goals (those are strategicPriorities). Not blockers. \
           If the context already contains an \"Open Commitments\" section from prior meetings, \
           do NOT re-extract the same items. Supplement only with new commitments not already listed.\n\
         - strategicPriorities[]: The customer's stated BUSINESS OBJECTIVES for the engagement. \
           High-level goals, not tactical commitments or individual blockers.\n\
         - renewalOutlook.riskFactors[]: Factors that could affect the CONTRACT DECISION \
           specifically. Brief references to items detailed elsewhere — not full duplicates.\n\
         - valueDelivered[]: OUTCOMES already achieved. Past tense. Not promises or goals.\n\
         - expansionSignals[]: GROWTH opportunities not yet closed. Not existing commitments.\n\n",
    );

    // Writing style instructions
    prompt.push_str(&format!(
        "WRITING RULES:\n\
         - Lead with conclusions, not evidence. State the \"so what\" first.\n\
         - Be concise. Every sentence must earn its place.\n\
         - Do NOT include footnotes, reference numbers, or source citations in prose.\n\
         - Do NOT embed filenames or source references inline in prose.\n\
         - Do NOT narrate chronologically. Synthesize themes and conclusions.\n\
         - Avoid relative time words (tonight, tomorrow, yesterday, this morning). \
           Use explicit local dates and times instead.\n\
         - Temporal hygiene: rewrite stale time framing from prior intelligence. \
           If text says a past date/time is still upcoming, correct it.\n\
         - Do NOT claim someone \"never attended\" or \"never appeared\" when \
           verified meeting presence data shows attendance.\n\
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

    // Partner-specific framing (I382): partner accounts use distinct vocabulary
    if entity_type == "account" && relationship == Some("partner") {
        prompt.push_str(
            "PARTNER CONTEXT:\n\
             - This is a PARTNER organization, not a customer. Do NOT use customer vocabulary \
               (no renewal_risk, churn risk, spend, NPS, customer health).\n\
             - Focus on: alignment health, joint deliverables, communication cadence, escalation risk.\n\
             - WORKING items = productive joint initiatives, clear ownership, responsive communication.\n\
             - NOT_WORKING items = misaligned priorities, stalled deliverables, communication gaps, unresolved escalations.\n\
             - Risks should focus on partnership risks — priority misalignment, resource constraints, \
               relationship cooling, blocked integrations.\n\
             - Assessment should answer: 'How healthy is this partnership and what needs attention?'\n\n",
        );
    }

    // Output format instructions
    let p1_framing = match entity_type {
        "account" => match relationship {
            Some("partner") => "partnership health",
            _ => "account trajectory",
        },
        "project" => "project trajectory",
        "person" => match relationship {
            Some("internal") => "collaboration dynamic",
            Some("external") => "relationship health",
            _ => "relationship dynamic",
        },
        _ => "overall assessment",
    };
    // I499: Inject pre-computed account health when available
    if let Some(ref computed) = ctx.computed_health {
        prompt.push_str(&format!(
            "## Pre-Computed Account Health (Algorithmic — ADR-0097)\n\
             Score: {score:.0}/100 ({band}) | Confidence: {conf:.0}%\n\
             Dimensions: meeting_cadence={mc:.0} email={em:.0} stakeholder={sc:.0} \
             champion={ch:.0} financial={fp:.0} signal={sm:.0}\n\n\
             Given the pre-computed health above, for the \"health\" field return ONLY \
             \"narrative\" (2-3 sentences explaining the score in business context) and \
             \"recommendedActions\" (3 specific next actions). Do NOT return score, band, \
             dimensions, or confidence — those are computed algorithmically.\n\n",
            score = computed.score,
            band = computed.band,
            conf = computed.confidence * 100.0,
            mc = computed.dimensions.meeting_cadence.score,
            em = computed.dimensions.email_engagement.score,
            sc = computed.dimensions.stakeholder_coverage.score,
            ch = computed.dimensions.champion_health.score,
            fp = computed.dimensions.financial_proximity.score,
            sm = computed.dimensions.signal_momentum.score,
        ));
    }

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

    // I384: Portfolio section for parent accounts with children
    if ctx.portfolio_children_context.is_some() {
        prompt.push_str(
            ",\n\
               \"portfolio\": {\n\
                 \"healthSummary\": \"1-2 sentence executive synthesis of portfolio health across all BUs\",\n\
                 \"hotspots\": [\n\
                   {\"childId\": \"child-account-id\", \"childName\": \"Child Name\", \"reason\": \"one-line reason this BU needs attention (risk or opportunity)\"}\n\
                 ],\n\
                 \"crossBuPatterns\": [\"signal types or themes appearing in 2+ children (e.g., 'budget risk', 'expansion opportunity')\"],\n\
                 \"portfolioNarrative\": \"2-3 paragraph executive synthesis: portfolio trajectory, systemic risks, cross-BU opportunities, and recommended portfolio-level actions\"\n\
               }",
        );
    }

    // I391: Network intelligence section for persons with relationship edges
    if ctx.relationship_edges.is_some() {
        prompt.push_str(
            ",\n\
               \"network\": {\n\
                 \"health\": \"strong|at_risk|weakened|unknown\",\n\
                 \"keyRelationships\": [{\"personId\":\"...\",\"name\":\"...\",\"relationshipType\":\"...\",\"confidence\":0.8,\"signalSummary\":\"1 sentence\"}],\n\
                 \"risks\": [\"network-level risk (e.g., key manager departing, cluster weakening)\"],\n\
                 \"opportunities\": [\"network-level opportunity (e.g., new ally, expanding influence)\"],\n\
                 \"influenceRadius\": 4,\n\
                 \"clusterSummary\": \"2-3 sentence synthesis of this person's network position and dynamics\"\n\
               }",
        );
    } else if entity_type == "person" {
        // Persons with no edges get a minimal stub
        prompt.push_str(
            ",\n\
               \"network\": {\"health\":\"unknown\",\"keyRelationships\":[],\"risks\":[],\"opportunities\":[],\"influenceRadius\":0}",
        );
    }

    // I504: Inferred person-to-person relationships for account/project entities.
    if ctx.canonical_contacts.is_some() && (entity_type == "account" || entity_type == "project") {
        prompt.push_str(
            ",\n\
               \"inferredRelationships\": [\n\
                 {\"fromPersonId\": \"person-id-from-canonical-contacts\", \"toPersonId\": \"person-id-from-canonical-contacts\", \"relationshipType\": \"peer|manager|mentor|collaborator|ally|partner|introduced_by\", \"rationale\": \"1 sentence explaining why this relationship is inferred\"}\n\
               ]",
        );
    }

    // I305: Keyword extraction for entity resolution
    prompt.push_str(
        ",\n\
           \"keywords\": [\"5-15 distinctive keywords/phrases that identify this entity \
         in meeting titles or calendar descriptions. Include product names, project codenames, \
         abbreviations, and commonly used references.\"]",
    );

    // I396/I499: Health section — narrative-only when pre-computed, full when not
    if ctx.computed_health.is_some() {
        // I499: Pre-computed health available — LLM only provides narrative + actions
        prompt.push_str(
            ",\n\
               \"health\": {\n\
                 \"narrative\": \"2-3 sentences explaining the pre-computed health score in business context. Connect the dimension scores to the account's situation.\",\n\
                 \"recommendedActions\": [\"3 specific actions to improve or maintain account health\"]\n\
               }",
        );
    } else {
        // Full health schema — LLM computes everything (no algorithmic baseline available)
        prompt.push_str(
            ",\n\
               \"health\": {\n\
                 \"score\": \"number 0-100. Return only when account has sufficient evidence; omit for sparse accounts.\",\n\
                 \"band\": \"green|yellow|red\",\n\
                 \"source\": \"computed\",\n\
                 \"confidence\": \"number 0.0-1.0\",\n\
                 \"trend\": {\"direction\": \"improving|stable|declining|volatile\", \"rationale\": \"1 sentence\", \"timeframe\": \"30d|90d\", \"confidence\": \"number 0.0-1.0\"},\n\
                 \"dimensions\": {\n\
                   \"meetingCadence\": {\"score\": 0, \"weight\": 0, \"evidence\": [\"signals\"], \"trend\": \"improving|stable|declining\"},\n\
                   \"emailEngagement\": {\"score\": 0, \"weight\": 0, \"evidence\": [\"signals\"], \"trend\": \"improving|stable|declining\"},\n\
                   \"stakeholderCoverage\": {\"score\": 0, \"weight\": 0, \"evidence\": [\"signals\"], \"trend\": \"improving|stable|declining\"},\n\
                   \"championHealth\": {\"score\": 0, \"weight\": 0, \"evidence\": [\"signals\"], \"trend\": \"improving|stable|declining\"},\n\
                   \"financialProximity\": {\"score\": 0, \"weight\": 0, \"evidence\": [\"signals\"], \"trend\": \"improving|stable|declining\"},\n\
                   \"signalMomentum\": {\"score\": 0, \"weight\": 0, \"evidence\": [\"signals\"], \"trend\": \"improving|stable|declining\"}\n\
                 },\n\
                 \"recommendedActions\": [\"specific next action\"]\n\
               }",
        );
    }

    // Shared fields — always present regardless of health mode
    prompt.push_str(
        ",\n\
           \"valueDelivered\": [\n\
         // ONLY include when the customer articulates a measurable business outcome.\n\
         // Must be: quantified (includes a number), attributed (customer connects to your product),\n\
         // and business-relevant (ties to revenue, cost, risk, or speed).\n\
         // BAD: \"The product is useful\" / \"They use it daily\" / \"Team likes it\"\n\
         // GOOD: \"Reduced troubleshooting time by 65%, saving ~$30K/month\"\n\
         // GOOD: \"Onboarded 500 users in 2 weeks vs previous 6 weeks\"\n\
         {\"date\": \"ISO date\", \"statement\": \"quantified outcome\", \
         \"source\": \"meeting|email|capture\", \"impact\": \"revenue|cost|risk|speed\"}],\n\
           \"successMetrics\": [{\"name\": \"short KPI label (max 5 words)\", \"target\": \"short target (e.g. 95%, $500K, 8+)\", \
         \"current\": \"short current value (e.g. $639K, 9, 85%) — max 15 chars, number or grade only, NEVER a sentence\", \
         \"status\": \"on_track|at_risk|behind|achieved\", \
         \"owner\": \"who owns this metric\"}],\n\
           \"openCommitments\": [{\"description\": \"what was committed\", \"owner\": \"who owns it\", \
         \"dueDate\": \"ISO date or null\", \"source\": \"meeting/email where committed\", \
         \"status\": \"open|in_progress|overdue|completed\"}],\n\
           \"relationshipDepth\": {\"championStrength\": \"strong|moderate|weak|none\", \
         \"executiveAccess\": \"direct|indirect|none\", \
         \"stakeholderCoverage\": \"broad|narrow|single_threaded\", \
         \"coverageGaps\": [\"role or team with no relationship\"]}",
    );

    // I508b: Dimension fields — only for accounts
    if entity_type == "account" {
        prompt.push_str(
            ",\n\
               \"competitiveContext\": [{\"competitor\": \"name\", \"threatLevel\": \"displacement|evaluation|mentioned|incumbent\", \"context\": \"1 sentence\", \"detectedAt\": \"ISO date or null\"}],\n\
               \"strategicPriorities\": [{\"priority\": \"...\", \"status\": \"active|at_risk|completed|paused\", \"owner\": \"...\", \"timeline\": \"...\"}],\n\
               \"coverageAssessment\": {\"roleFillRate\": 0.0, \"gaps\": [\"missing role\"], \"covered\": [\"filled role\"], \"level\": \"strong|adequate|thin|critical\"},\n\
               \"organizationalChanges\": [{\"changeType\": \"departure|hire|promotion|reorg|role_change\", \"person\": \"name\", \"from\": \"...\", \"to\": \"...\", \"detectedAt\": \"ISO date\"}],\n\
               \"internalTeam\": [{\"name\": \"...\", \"role\": \"RM|AE|TAM|Division Lead|etc\"}],\n\
               \"meetingCadence\": {\"meetingsPerMonth\": 0.0, \"trend\": \"increasing|stable|declining|erratic\", \"daysSinceLast\": 0, \"assessment\": \"healthy|adequate|sparse|cold\"},\n\
               \"emailResponsiveness\": {\"trend\": \"improving|stable|slowing|gone_quiet\", \"assessment\": \"responsive|normal|slow|unresponsive\"},\n\
               \"blockers\": [{\"description\": \"...\", \"owner\": \"...\", \"since\": \"ISO date\", \"impact\": \"critical|high|moderate|low\"}],\n\
               \"contractContext\": {\"contractType\": \"annual|multi_year|month_to_month\", \"autoRenew\": true, \"renewalDate\": \"ISO date\", \"currentArr\": 0.0},\n\
               \"expansionSignals\": [\n\
               // Cross-departmental interest, usage ceiling hits, proactive internal advocacy,\n\
               // organizational growth (hiring, acquisitions), questions about roadmap/pricing,\n\
               // budget increase mentions. Each must cite specific evidence.\n\
               {\"opportunity\": \"...\", \"arrImpact\": 0.0, \"stage\": \"exploring|evaluating|committed|blocked\", \"strength\": \"strong|moderate|early\"}],\n\
               \"renewalOutlook\": {\"confidence\": \"high|moderate|low\", \"riskFactors\": [\"...\"], \"expansionPotential\": \"...\", \"recommendedStart\": \"ISO date\"},\n\
               \"supportHealth\": {\"openTickets\": 0, \"criticalTickets\": 0, \"trend\": \"improving|stable|degrading\", \"csat\": 0.0},\n\
               \"productAdoption\": {\"adoptionRate\": 0.0, \"trend\": \"growing|stable|declining\", \"featureAdoption\": [\"...\"], \"lastActive\": \"ISO date\"},\n\
               \"npsCsat\": {\"nps\": 0, \"csat\": 0.0, \"surveyDate\": \"ISO date\", \"verbatim\": \"quote\"},\n\
               \"sourceAttribution\": {\"fieldName\": [\"source1\"]}",
        );

        // I554: Success plan signals for accounts
        prompt.push_str(
            ",\n\
               \"successPlanSignals\": {\n\
                 \"statedObjectives\": [{\"objective\": \"...\", \"source\": \"meeting|email|file\", \"owner\": \"...\", \"targetDate\": \"ISO or null\", \"confidence\": \"high|medium|low\"}],\n\
                 \"mutualSuccessCriteria\": [{\"criterion\": \"...\", \"ownedBy\": \"us|them|joint\", \"status\": \"not_started|in_progress|achieved|at_risk\"}],\n\
                 \"milestoneCandidates\": [{\"milestone\": \"...\", \"expectedBy\": \"ISO or null\", \"detectedFrom\": \"source\", \"autoDetectEvent\": \"lifecycle event type or null\"}]\n\
               }",
        );
    }

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
         \"What do I need to do or ask before/during this meeting?\"\n\n\
         For health, successMetrics, openCommitments, and relationshipDepth: \
         return null/empty when the account has fewer than 2 signals. Do NOT hallucinate values \
         for sparse accounts. valueDelivered should include completed commitments and wins. \
         When the user's professional context includes a value proposition, frame valueDelivered \
         entries through that lens — describe value in terms the user would use to communicate \
         it to their stakeholders.\n",
    );

    if entity_type == "account" {
        prompt.push_str(
            "\nFor I508 dimension fields (competitiveContext through sourceAttribution): \
             include ONLY when evidence exists in the provided context (Glean snippets, \
             meeting notes, files, email signals). Return null or empty array when no evidence. \
             Do NOT fabricate. Evidence sources may include workspace files, meeting transcripts, \
             email signals, and Glean documents — extract intelligence from all available sources.\n\n\
             successPlanSignals — Synthesize the strategic objectives for this account from aggregate \
             context. What is this customer trying to achieve with us? What have we mutually committed \
             to? Focus on explicitly stated goals (\"our goal is...\", \"success looks like...\"), mutual \
             commitments beyond individual action items, measurable criteria, and key milestones. \
             Confidence: \"high\" = explicitly stated, \"medium\" = inferred from multiple signals, \
             \"low\" = extrapolated from limited data. Do NOT fabricate objectives — return empty arrays \
             if no stated goals exist.\n\n\
             valueDelivered — ONLY include when the customer articulates a measurable business outcome. \
             Must be quantified (includes a number), attributed (customer connects to your product), \
             and business-relevant (ties to revenue, cost, risk, or speed). Reject vague usage \
             statements like \"they use it daily\" or \"team likes it.\"\n\n\
             expansionSignals — Include strength classification for each signal: \"strong\" = explicit \
             interest with budget or timeline, \"moderate\" = multiple mentions or cross-departmental \
             interest, \"early\" = single mention or indirect signal.\n",
        );
    }

    if ctx.canonical_contacts.is_some() && (entity_type == "account" || entity_type == "project") {
        prompt.push_str(
            "\n\
             For inferredRelationships: analyze meeting co-attendance patterns, email threads, \
             and content context to infer person-to-person relationships. Only use person IDs \
             from the canonical contacts list. Only infer relationships with clear evidence — do \
             not guess. Prefer 'collaborator' for people who work together on projects, \
             'manager' when reporting lines are evident, and 'peer' when people appear at a \
             similar organizational level. Return an empty array when no relationship can be \
             confidently inferred.\n",
        );
    }

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
    /// I576: Concise editorial pull quote.
    #[serde(default)]
    pull_quote: Option<String>,
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
    /// Portfolio intelligence for parent accounts (I384).
    #[serde(default)]
    portfolio: Option<AiPortfolioIntelligence>,
    /// Network intelligence for person entities (I391).
    #[serde(default)]
    network: Option<AiNetworkIntelligence>,
    /// Inferred person-to-person relationships (I391).
    #[serde(default)]
    inferred_relationships: Vec<AiInferredRelationship>,
    /// Auto-extracted keywords for entity resolution (I305).
    #[serde(default)]
    keywords: Vec<String>,
    /// ADR-0097 structured account health payload.
    #[serde(default)]
    health: Option<super::io::AccountHealth>,
    /// Legacy I396: health score (0-100).
    #[serde(default)]
    health_score: Option<f64>,
    /// Legacy I396: health trend direction + rationale.
    #[serde(default)]
    health_trend: Option<AiHealthTrend>,
    /// I396: Success metrics / KPIs the user tracks.
    #[serde(default)]
    success_metrics: Option<Vec<AiSuccessMetric>>,
    /// I396: Open commitments (promises made to/from the account).
    #[serde(default)]
    open_commitments: Option<Vec<AiOpenCommitment>>,
    /// I396: Relationship depth assessment.
    #[serde(default)]
    relationship_depth: Option<AiRelationshipDepth>,
    // I508b: dimension fields — deserialize directly from LLM JSON output
    #[serde(default)]
    competitive_context: Vec<super::io::CompetitiveInsight>,
    #[serde(default)]
    strategic_priorities: Vec<super::io::StrategicPriority>,
    #[serde(default)]
    coverage_assessment: Option<super::io::CoverageAssessment>,
    #[serde(default)]
    organizational_changes: Vec<super::io::OrgChange>,
    #[serde(default)]
    internal_team: Vec<super::io::InternalTeamMember>,
    #[serde(default, alias = "meetingCadenceAssessment")]
    meeting_cadence: Option<super::io::CadenceAssessment>,
    #[serde(default)]
    email_responsiveness: Option<super::io::ResponsivenessAssessment>,
    #[serde(default)]
    blockers: Vec<super::io::Blocker>,
    #[serde(default)]
    contract_context: Option<super::io::ContractContext>,
    #[serde(default)]
    expansion_signals: Vec<super::io::ExpansionSignal>,
    #[serde(default)]
    renewal_outlook: Option<super::io::RenewalOutlook>,
    #[serde(default)]
    support_health: Option<super::io::SupportHealth>,
    #[serde(default)]
    product_adoption: Option<super::io::AdoptionSignals>,
    #[serde(default)]
    nps_csat: Option<super::io::SatisfactionData>,
    #[serde(default)]
    source_attribution: Option<std::collections::HashMap<String, Vec<String>>>,
    /// I554: Success plan signals synthesized from aggregate context.
    #[serde(default)]
    success_plan_signals: Option<crate::types::SuccessPlanSignals>,
}

/// I396: Health trend direction with rationale.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AiHealthTrend {
    direction: String,
    #[serde(default)]
    rationale: Option<String>,
}

fn legacy_health_to_account_health(
    health_score: Option<f64>,
    health_trend: Option<AiHealthTrend>,
) -> Option<super::io::AccountHealth> {
    let score = health_score?;
    let band = if score >= 70.0 {
        "green"
    } else if score >= 40.0 {
        "yellow"
    } else {
        "red"
    };
    Some(super::io::AccountHealth {
        score,
        band: band.to_string(),
        source: super::io::HealthSource::Computed,
        confidence: 0.3,
        trend: super::io::HealthTrend {
            direction: health_trend
                .as_ref()
                .map(|t| t.direction.clone())
                .unwrap_or_else(|| "stable".to_string()),
            rationale: health_trend.and_then(|t| t.rationale),
            timeframe: "30d".to_string(),
            confidence: 0.3,
        },
        dimensions: super::io::RelationshipDimensions::default(),
        divergence: None,
        narrative: None,
        recommended_actions: Vec::new(),
    })
}

/// I396: A success metric / KPI tracked for an entity.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AiSuccessMetric {
    name: String,
    #[serde(default)]
    target: Option<String>,
    #[serde(default)]
    current: Option<String>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    owner: Option<String>,
}

/// I396: An open commitment (promise made to/from the account).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AiOpenCommitment {
    description: String,
    #[serde(default)]
    owner: Option<String>,
    #[serde(default)]
    due_date: Option<String>,
    #[serde(default)]
    source: Option<String>,
    #[serde(default)]
    status: Option<String>,
}

/// I396: Relationship depth assessment.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AiRelationshipDepth {
    #[serde(default)]
    champion_strength: Option<String>,
    #[serde(default)]
    executive_access: Option<String>,
    #[serde(default)]
    stakeholder_coverage: Option<String>,
    #[serde(default)]
    coverage_gaps: Option<Vec<String>>,
}

/// AI response structure for network intelligence (I391).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AiNetworkIntelligence {
    #[serde(default = "super::io::default_network_health")]
    health: String,
    #[serde(default)]
    key_relationships: Vec<AiNetworkKeyRelationship>,
    #[serde(default)]
    risks: Vec<String>,
    #[serde(default)]
    opportunities: Vec<String>,
    #[serde(default)]
    influence_radius: u32,
    #[serde(default)]
    cluster_summary: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AiNetworkKeyRelationship {
    #[serde(default)]
    person_id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    relationship_type: String,
    #[serde(default)]
    confidence: f64,
    #[serde(default)]
    signal_summary: Option<String>,
}

/// AI response structure for an inferred relationship (I391).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AiInferredRelationship {
    #[serde(default)]
    from_person_id: String,
    #[serde(default)]
    to_person_id: String,
    #[serde(default)]
    relationship_type: String,
    #[serde(default, alias = "reason")]
    rationale: Option<String>,
}

/// AI response structure for portfolio intelligence (I384).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AiPortfolioIntelligence {
    #[serde(default)]
    health_summary: Option<String>,
    #[serde(default)]
    hotspots: Vec<AiPortfolioHotspot>,
    #[serde(default)]
    cross_bu_patterns: Vec<String>,
    #[serde(default)]
    portfolio_narrative: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AiPortfolioHotspot {
    child_id: String,
    child_name: String,
    reason: String,
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
    // Try JSON first (includes I470 validation + anomaly detection)
    let mut intel = if let Some(parsed) = try_parse_json_response(
        response,
        entity_id,
        entity_type,
        source_file_count,
        &manifest,
    ) {
        parsed
    } else {
        // Fall back to pipe-delimited format (backwards compat).
        // Run anomaly detection on the raw response even for non-JSON (I470).
        crate::intelligence::validation::check_anomalies_public(response);
        parse_pipe_delimited_response(
            response,
            entity_id,
            entity_type,
            source_file_count,
            manifest,
        )?
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
    // ADR-0097: Clamp structured health score to 0-100 range.
    if let Some(ref mut health) = intel.health {
        health.score = health.score.clamp(0.0, 100.0);
    }
    if let Some(ref mut metrics) = intel.success_metrics {
        metrics.truncate(20);
    }
    if let Some(ref mut commits) = intel.open_commitments {
        commits.truncate(20);
    }

    Ok(intel)
}

/// An inferred person-to-person relationship extracted from AI enrichment (I391).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InferredRelationship {
    pub from_person_id: String,
    pub to_person_id: String,
    pub relationship_type: String,
    pub rationale: Option<String>,
}

/// Extract inferred relationships from an AI enrichment response (I391).
/// Returns empty vec if no valid relationships are found.
pub fn extract_inferred_relationships(response: &str) -> Vec<InferredRelationship> {
    let json_str = match extract_json_from_response(response) {
        Some(s) => s,
        None => return Vec::new(),
    };
    let parsed: serde_json::Value = match serde_json::from_str(json_str) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let arr = match parsed
        .get("inferredRelationships")
        .and_then(|v| v.as_array())
    {
        Some(a) => a,
        None => return Vec::new(),
    };
    arr.iter()
        .filter_map(|entry| {
            let from = entry.get("fromPersonId")?.as_str()?.to_string();
            let to = entry.get("toPersonId")?.as_str()?.to_string();
            let rel_type = entry.get("relationshipType")?.as_str()?.to_string();
            // Validate relationship type
            if rel_type
                .parse::<crate::db::person_relationships::RelationshipType>()
                .is_err()
            {
                return None;
            }
            if from.is_empty() || to.is_empty() {
                return None;
            }
            let rationale = entry
                .get("rationale")
                .or_else(|| entry.get("reason"))
                .and_then(|v| v.as_str())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());
            Some(InferredRelationship {
                from_person_id: from,
                to_person_id: to,
                relationship_type: rel_type,
                rationale,
            })
        })
        .collect()
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

    // I470: Validate structure and run anomaly detection before deserialization
    if let Err(e) = super::validation::validate_intelligence_response(json_str) {
        log::warn!(
            "Intelligence response validation failed for {}: {}",
            entity_id,
            e
        );
        return None;
    }

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

    let portfolio = ai_resp.portfolio.map(|p| PortfolioIntelligence {
        health_summary: p.health_summary,
        hotspots: p
            .hotspots
            .into_iter()
            .map(|h| PortfolioHotspot {
                child_id: h.child_id,
                child_name: h.child_name,
                reason: h.reason,
            })
            .collect(),
        cross_bu_patterns: p.cross_bu_patterns,
        portfolio_narrative: p.portfolio_narrative,
    });

    Some(IntelligenceJson {
        version: 1,
        entity_id: entity_id.to_string(),
        entity_type: entity_type.to_string(),
        enriched_at: Utc::now().to_rfc3339(),
        source_file_count,
        source_manifest: manifest.to_vec(),
        executive_assessment: ai_resp.executive_assessment,
        pull_quote: ai_resp.pull_quote,
        risks: ai_resp
            .risks
            .into_iter()
            .map(|r| IntelRisk {
                text: r.text,
                source: r.source,
                urgency: r.urgency,
                item_source: None,
                discrepancy: None,
            })
            .collect(),
        recent_wins: ai_resp
            .recent_wins
            .into_iter()
            .map(|w| IntelWin {
                text: w.text,
                source: w.source,
                impact: w.impact,
                item_source: None,
                discrepancy: None,
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
                person_id: None,
                suggested_person_id: None,
                item_source: None,
                discrepancy: None,
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
                item_source: None,
                discrepancy: None,
            })
            .collect(),
        next_meeting_readiness,
        company_context,
        portfolio,
        network: ai_resp.network.map(|n| NetworkIntelligence {
            health: n.health,
            key_relationships: n
                .key_relationships
                .into_iter()
                .map(|kr| NetworkKeyRelationship {
                    person_id: kr.person_id,
                    name: kr.name,
                    relationship_type: kr.relationship_type,
                    confidence: kr.confidence,
                    signal_summary: kr.signal_summary,
                })
                .collect(),
            risks: n.risks,
            opportunities: n.opportunities,
            influence_radius: n.influence_radius,
            cluster_summary: n.cluster_summary,
        }),
        user_edits: Vec::new(),
        health: ai_resp.health.or_else(|| {
            legacy_health_to_account_health(ai_resp.health_score, ai_resp.health_trend)
        }),
        org_health: None,
        success_metrics: ai_resp.success_metrics.map(|metrics| {
            metrics
                .into_iter()
                .map(|m| super::io::SuccessMetric {
                    name: m.name,
                    target: m.target,
                    current: m.current,
                    status: m.status,
                    owner: m.owner,
                })
                .collect()
        }),
        open_commitments: ai_resp.open_commitments.map(|commits| {
            commits
                .into_iter()
                .map(|c| super::io::OpenCommitment {
                    description: c.description,
                    owner: c.owner,
                    due_date: c.due_date,
                    source: c.source,
                    status: c.status,
                    item_source: None,
                    discrepancy: None,
                })
                .collect()
        }),
        relationship_depth: ai_resp
            .relationship_depth
            .map(|rd| super::io::RelationshipDepth {
                champion_strength: rd.champion_strength,
                executive_access: rd.executive_access,
                stakeholder_coverage: rd.stakeholder_coverage,
                coverage_gaps: rd.coverage_gaps,
            }),
        consistency_status: None,
        consistency_findings: Vec::new(),
        consistency_checked_at: None,
        // I508b: map LLM-returned dimension fields into IntelligenceJson
        competitive_context: ai_resp.competitive_context,
        strategic_priorities: ai_resp.strategic_priorities,
        coverage_assessment: ai_resp.coverage_assessment,
        organizational_changes: ai_resp.organizational_changes,
        internal_team: ai_resp.internal_team,
        meeting_cadence: ai_resp.meeting_cadence,
        email_responsiveness: ai_resp.email_responsiveness,
        blockers: ai_resp.blockers,
        contract_context: ai_resp.contract_context,
        expansion_signals: ai_resp.expansion_signals,
        renewal_outlook: ai_resp.renewal_outlook,
        support_health: ai_resp.support_health,
        product_adoption: ai_resp.product_adoption,
        nps_csat: ai_resp.nps_csat,
        source_attribution: ai_resp.source_attribution,
        gong_call_summaries: Vec::new(),
        success_plan_signals: ai_resp.success_plan_signals,
        dismissed_items: Vec::new(),
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
        item_source: None,
        discrepancy: None,
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
        item_source: None,
        discrepancy: None,
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
        person_id: None,
        suggested_person_id: None,
        item_source: None,
        discrepancy: None,
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
        item_source: None,
        discrepancy: None,
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

    // ─── Step 2 tests: prompt builder + response parser ───

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
            portfolio_children_context: None,
            canonical_contacts: None,
            verified_stakeholder_presence: None,
            relationship_edges: None,
            user_context: None,
            entity_context: None,
            gap_queries: Vec::new(),
            computed_health: None,
            org_health: None,
            extra_blocks: Vec::new(),
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
    fn test_build_intelligence_prompt_includes_verified_presence() {
        let ctx = IntelligenceContext {
            verified_stakeholder_presence: Some(
                "- Matt Wickham — appears in 2 recorded meetings (last seen 2026-03-01 10:00 AM EST)"
                    .to_string(),
            ),
            ..Default::default()
        };

        let prompt = build_intelligence_prompt("Meridian Asset", "account", &ctx, None, None);
        assert!(prompt.contains("Verified Stakeholder Meeting Presence"));
        assert!(prompt.contains("appears in 2 recorded meetings"));
        assert!(prompt.contains("Do NOT claim someone \"never attended\""));
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

        let prompt = build_intelligence_prompt("Bob Kim", "person", &ctx, Some("internal"), None);

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

        let prompt = build_intelligence_prompt("Unknown Person", "person", &ctx, None, None);

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

        let prompt = build_intelligence_prompt("Acme Corp", "account", &ctx, None, Some(&vocab));

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
        assert!(result
            .unwrap_err()
            .contains("No INTELLIGENCE block or JSON"));
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
        assert!(intel
            .executive_assessment
            .unwrap()
            .contains("strong position"));
        assert_eq!(intel.risks.len(), 2);
        assert_eq!(intel.risks[0].urgency, "critical");
        assert_eq!(intel.recent_wins.len(), 1);
        assert_eq!(
            intel.recent_wins[0].impact.as_deref(),
            Some("20% seat growth")
        );
        let state = intel.current_state.unwrap();
        assert_eq!(state.working.len(), 1);
        assert_eq!(state.not_working.len(), 1);
        assert_eq!(state.unknowns.len(), 1);
        assert_eq!(intel.stakeholder_insights.len(), 1);
        assert_eq!(
            intel.stakeholder_insights[0].engagement.as_deref(),
            Some("high")
        );
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

        assert_eq!(
            intel.executive_assessment.as_deref(),
            Some("Assessment text.")
        );
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
            account_type: crate::db::AccountType::Customer,
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

// ==========================================================================
// I619 — Prompt Evaluation Suite: golden fixture tests
// ==========================================================================

#[cfg(test)]
mod eval_tests {
    use super::*;

    // ── Category 1: Prompt Construction Tests ──

    #[test]
    fn eval_intelligence_prompt_includes_json_output_format() {
        let ctx = IntelligenceContext {
            facts_block: "ARR: $200K".to_string(),
            ..Default::default()
        };
        let prompt = build_intelligence_prompt("TestCo", "account", &ctx, None, None);
        assert!(prompt.contains("JSON"), "Prompt must request JSON output");
        assert!(
            prompt.contains("executiveAssessment"),
            "Prompt must include executiveAssessment field"
        );
        assert!(
            prompt.contains("risks"),
            "Prompt must include risks field schema"
        );
        assert!(
            prompt.contains("stakeholderInsights"),
            "Prompt must include stakeholderInsights"
        );
    }

    #[test]
    fn eval_intelligence_prompt_includes_writing_rules() {
        let ctx = IntelligenceContext::default();
        let prompt = build_intelligence_prompt("TestCo", "account", &ctx, None, None);
        assert!(
            prompt.contains("Lead with conclusions"),
            "Prompt must contain writing rule: lead with conclusions"
        );
        assert!(
            prompt.contains("Do NOT include footnotes"),
            "Prompt must prohibit footnotes"
        );
        assert!(
            prompt.contains("Max 250 words"),
            "Prompt must enforce 250 word limit"
        );
    }

    #[test]
    fn eval_intelligence_prompt_includes_field_scoping_rules() {
        let ctx = IntelligenceContext::default();
        let prompt = build_intelligence_prompt("TestCo", "account", &ctx, None, None);
        assert!(
            prompt.contains("FIELD SCOPING RULES"),
            "Prompt must include field deduplication guidance"
        );
        assert!(
            prompt.contains("openCommitments"),
            "Prompt must mention openCommitments scoping"
        );
    }

    #[test]
    fn eval_intelligence_prompt_includes_injection_preamble() {
        let ctx = IntelligenceContext::default();
        let prompt = build_intelligence_prompt("TestCo", "account", &ctx, None, None);
        assert!(
            prompt.contains("INJECTION_BOUNDARY")
                || prompt.contains("system instructions")
                || prompt.contains(INJECTION_PREAMBLE.split('\n').next().unwrap_or("")),
            "Prompt must include injection resistance preamble"
        );
    }

    #[test]
    fn eval_intelligence_prompt_person_entity_includes_network_schema() {
        let ctx = IntelligenceContext {
            facts_block: "Role: VP Engineering".to_string(),
            ..Default::default()
        };
        let prompt = build_intelligence_prompt("Jane Doe", "person", &ctx, Some("external"), None);
        assert!(
            prompt.contains("network"),
            "Person prompt must include network schema"
        );
        assert!(
            prompt.contains("EXTERNAL STAKEHOLDER"),
            "Person prompt must include person context framing"
        );
    }

    #[test]
    fn eval_intelligence_prompt_partner_excludes_customer_vocab() {
        let ctx = IntelligenceContext::default();
        let prompt = build_intelligence_prompt("PartnerCo", "account", &ctx, Some("partner"), None);
        assert!(
            prompt.contains("PARTNER CONTEXT"),
            "Partner prompt must include partner framing"
        );
        assert!(
            prompt.contains("partner organization"),
            "Partner prompt must use partner vocabulary"
        );
    }

    #[test]
    fn eval_intelligence_prompt_includes_health_schema() {
        let ctx = IntelligenceContext::default();
        let prompt = build_intelligence_prompt("TestCo", "account", &ctx, None, None);
        // Without pre-computed health, prompt should include full health schema
        assert!(
            prompt.contains("\"health\""),
            "Prompt must include health field schema"
        );
        assert!(
            prompt.contains("\"band\""),
            "Prompt must include health band"
        );
    }

    #[test]
    fn eval_intelligence_prompt_with_precomputed_health_requests_narrative_only() {
        let ctx = IntelligenceContext {
            computed_health: Some(super::super::io::AccountHealth {
                score: 72.0,
                band: "green".to_string(),
                confidence: 0.75,
                ..Default::default()
            }),
            ..Default::default()
        };
        let prompt = build_intelligence_prompt("TestCo", "account", &ctx, None, None);
        assert!(
            prompt.contains("Pre-Computed Account Health"),
            "Prompt must acknowledge pre-computed health"
        );
        assert!(
            prompt.contains("narrative"),
            "Prompt must request narrative for pre-computed health"
        );
    }

    // ── Category 2: Response Parsing Tests ──

    #[test]
    fn eval_parse_full_enrichment_response() {
        let response = include_str!("fixtures/enrichment_response_full.json");
        let result = parse_intelligence_response(response, "acme-1", "account", 5, Vec::new());
        assert!(
            result.is_ok(),
            "Full response must parse: {:?}",
            result.err()
        );
        let intel = result.unwrap();

        // Executive assessment present
        assert!(
            intel.executive_assessment.is_some(),
            "Must have executive assessment"
        );
        assert!(
            intel
                .executive_assessment
                .as_ref()
                .unwrap()
                .contains("Acme Corp"),
            "Assessment must mention entity name"
        );

        // Pull quote present
        assert!(intel.pull_quote.is_some(), "Must have pull quote");

        // Risks with urgency
        assert!(intel.risks.len() >= 2, "Must have multiple risks");
        assert!(
            intel.risks.iter().any(|r| r.urgency == "critical"),
            "Must have at least one critical-urgency risk"
        );
        assert!(
            intel.risks.iter().any(|r| r.urgency == "watch"),
            "Must have at least one watch-urgency risk"
        );

        // Wins
        assert!(intel.recent_wins.len() >= 2, "Must have multiple wins");

        // Stakeholder insights
        assert!(
            intel.stakeholder_insights.len() >= 3,
            "Must have multiple stakeholders"
        );
        assert!(
            intel
                .stakeholder_insights
                .iter()
                .any(|s| s.engagement == Some("high".to_string())),
            "Must have high-engagement stakeholder"
        );

        // Current state
        assert!(intel.current_state.is_some(), "Must have current state");
        let cs = intel.current_state.unwrap();
        assert!(!cs.working.is_empty(), "Must have working items");
        assert!(!cs.not_working.is_empty(), "Must have not-working items");
        assert!(!cs.unknowns.is_empty(), "Must have unknowns");

        // Health with dimensions
        assert!(intel.health.is_some(), "Must have health");
        let health = intel.health.unwrap();
        assert!(
            health.score > 0.0 && health.score <= 100.0,
            "Score must be 0-100"
        );
        assert!(
            ["green", "yellow", "red"].contains(&health.band.as_str()),
            "Band must be green/yellow/red"
        );

        // Value delivered with quantification
        assert!(
            intel.value_delivered.len() >= 1,
            "Must have value delivered"
        );

        // Competitive context (I508a dimension field)
        assert!(
            !intel.competitive_context.is_empty(),
            "Must have competitive context"
        );

        // Strategic priorities
        assert!(
            !intel.strategic_priorities.is_empty(),
            "Must have strategic priorities"
        );

        // Coverage assessment
        assert!(
            intel.coverage_assessment.is_some(),
            "Must have coverage assessment"
        );

        // Expansion signals
        assert!(
            !intel.expansion_signals.is_empty(),
            "Must have expansion signals"
        );

        // Success plan signals (I554)
        assert!(
            intel.success_plan_signals.is_some(),
            "Must have success plan signals"
        );

        // Success metrics
        assert!(intel.success_metrics.is_some(), "Must have success metrics");

        // Open commitments
        assert!(
            intel.open_commitments.is_some(),
            "Must have open commitments"
        );

        // Relationship depth
        assert!(
            intel.relationship_depth.is_some(),
            "Must have relationship depth"
        );
    }

    #[test]
    fn eval_parse_sparse_enrichment_handles_missing_fields() {
        let response = include_str!("fixtures/enrichment_response_sparse.json");
        let result = parse_intelligence_response(response, "beta-1", "account", 1, Vec::new());
        assert!(
            result.is_ok(),
            "Sparse response must parse: {:?}",
            result.err()
        );
        let intel = result.unwrap();

        // Should still have executive assessment
        assert!(
            intel.executive_assessment.is_some(),
            "Sparse must have executive assessment"
        );

        // Wins empty is valid
        assert!(
            intel.recent_wins.is_empty(),
            "Sparse response should have empty wins"
        );

        // Health present but low confidence
        assert!(intel.health.is_some(), "Sparse must have health");
        let health = intel.health.unwrap();
        assert!(
            health.confidence < 0.5,
            "Sparse health confidence should be low"
        );
        assert_eq!(health.band, "yellow", "Sparse health should be yellow");
    }

    #[test]
    fn eval_parse_malformed_response_graceful_handling() {
        let response = include_str!("fixtures/enrichment_response_malformed.json");
        // The malformed response has risks as a string instead of array.
        // serde_json will fail to deserialize AiIntelResponse, so try_parse_json_response
        // returns None, and it falls through to pipe-delimited parsing which also fails.
        // Either way, the function should not panic.
        let result = parse_intelligence_response(response, "bad-1", "account", 0, Vec::new());
        // Malformed JSON with wrong types should either produce an error or degrade gracefully.
        // The key assertion is: no panic.
        if let Ok(intel) = &result {
            // If it somehow parsed (unlikely), verify it degraded gracefully
            assert!(
                intel.risks.len() <= 1,
                "Malformed risks should not produce valid entries"
            );
        }
        // Either way, we got here without panicking — success
    }

    #[test]
    fn eval_parse_response_clamps_health_score() {
        // Scores > 100 should be clamped
        let response = r#"{"executiveAssessment":"Test","health":{"score":150,"band":"green","confidence":0.5}}"#;
        let result = parse_intelligence_response(response, "t1", "account", 0, Vec::new());
        assert!(result.is_ok());
        let intel = result.unwrap();
        assert!(intel.health.is_some());
        assert!(
            intel.health.as_ref().unwrap().score <= 100.0,
            "Health score must be clamped to 100"
        );
    }

    #[test]
    fn eval_parse_response_truncates_large_arrays() {
        // Build a response with 25 risks (exceeds 20 cap)
        let mut risks = Vec::new();
        for i in 0..25 {
            risks.push(format!(r#"{{"text":"Risk {}","urgency":"watch"}}"#, i));
        }
        let response = format!(
            r#"{{"executiveAssessment":"Test","risks":[{}]}}"#,
            risks.join(",")
        );
        let result = parse_intelligence_response(&response, "t2", "account", 0, Vec::new());
        assert!(result.is_ok());
        let intel = result.unwrap();
        assert_eq!(intel.risks.len(), 20, "Risks must be truncated to 20");
    }

    #[test]
    fn eval_parse_response_with_markdown_fences() {
        let response = "Here is the assessment:\n```json\n{\"executiveAssessment\":\"Test response in fenced JSON\"}\n```\nEnd.";
        let result = parse_intelligence_response(response, "t3", "account", 0, Vec::new());
        assert!(result.is_ok(), "Must parse JSON from markdown fences");
        let intel = result.unwrap();
        assert_eq!(
            intel.executive_assessment,
            Some("Test response in fenced JSON".to_string())
        );
    }

    #[test]
    fn eval_extract_inferred_relationships() {
        let response = r#"{"inferredRelationships":[
            {"fromPersonId":"p1","toPersonId":"p2","relationshipType":"peer","rationale":"Work together on project X"},
            {"fromPersonId":"p3","toPersonId":"p4","relationshipType":"manager","reason":"Direct report"}
        ]}"#;
        let rels = extract_inferred_relationships(response);
        assert_eq!(rels.len(), 2, "Must extract 2 relationships");
        assert_eq!(rels[0].relationship_type, "peer");
        assert_eq!(
            rels[0].rationale,
            Some("Work together on project X".to_string())
        );
        // "reason" alias should also work
        assert_eq!(rels[1].rationale, Some("Direct report".to_string()));
    }

    #[test]
    fn eval_extract_inferred_relationships_empty_on_bad_input() {
        let rels = extract_inferred_relationships("not json at all");
        assert!(rels.is_empty(), "Bad input must produce empty vec");

        let rels2 = extract_inferred_relationships(r#"{"inferredRelationships":"not an array"}"#);
        assert!(rels2.is_empty(), "Non-array must produce empty vec");
    }
}
