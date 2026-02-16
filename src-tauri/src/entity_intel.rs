//! Entity Intelligence I/O and types (I130 / ADR-0057).
//!
//! Three-file entity pattern: dashboard.json (mechanical) + intelligence.json
//! (synthesized) + dashboard.md (artifact). This module owns the intelligence
//! layer — types, file I/O, and migration from the legacy CompanyOverview.
//!
//! Intelligence is entity-generic: the same `IntelligenceJson` schema applies
//! to accounts, projects, and people. The enrichment prompt is parameterized
//! by entity_type (handled in Phase 2).

use std::path::Path;

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::accounts::CompanyOverview;
use crate::db::{ActionDb, DbAccount};
use crate::util::atomic_write_str;

// =============================================================================
// Intelligence JSON Schema
// =============================================================================

/// Top-level intelligence file (intelligence.json).
///
/// Entity-generic — same schema for accounts, projects, and people per ADR-0057.
/// Factual data (ARR, health, lifecycle) stays in dashboard.json. Intelligence
/// is synthesized assessment that the AI produces from all available signals.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct IntelligenceJson {
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default)]
    pub entity_id: String,
    #[serde(default)]
    pub entity_type: String,
    #[serde(default)]
    pub enriched_at: String,
    #[serde(default)]
    pub source_file_count: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_manifest: Vec<SourceManifestEntry>,

    /// Prose assessment: account situation / project status / relationship brief.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub executive_assessment: Option<String>,

    /// Account risks / project blockers / relationship risks.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub risks: Vec<IntelRisk>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub recent_wins: Vec<IntelWin>,

    /// Working / not working / unknowns assessment.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_state: Option<CurrentState>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stakeholder_insights: Vec<StakeholderInsight>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub value_delivered: Vec<ValueItem>,

    /// Prep items for the next meeting with this entity.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_meeting_readiness: Option<MeetingReadiness>,

    /// Company/project context from web search or overview.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub company_context: Option<CompanyContext>,
}

fn default_version() -> u32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceManifestEntry {
    pub filename: String,
    pub modified_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntelRisk {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default = "default_urgency")]
    pub urgency: String,
}

fn default_urgency() -> String {
    "watch".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntelWin {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub impact: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CurrentState {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub working: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub not_working: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unknowns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StakeholderInsight {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assessment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub engagement: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValueItem {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    pub statement: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub impact: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingReadiness {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meeting_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meeting_date: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub prep_items: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompanyContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub industry: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headquarters: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_context: Option<String>,
}

// =============================================================================
// File I/O
// =============================================================================

const INTEL_FILENAME: &str = "intelligence.json";

/// Read intelligence.json from an entity directory.
pub fn read_intelligence_json(dir: &Path) -> Result<IntelligenceJson, String> {
    let path = dir.join(INTEL_FILENAME);
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
    serde_json::from_str(&content).map_err(|e| format!("Failed to parse {}: {}", path.display(), e))
}

/// Write intelligence.json atomically to an entity directory.
pub fn write_intelligence_json(dir: &Path, intel: &IntelligenceJson) -> Result<(), String> {
    std::fs::create_dir_all(dir)
        .map_err(|e| format!("Failed to create {}: {}", dir.display(), e))?;
    let path = dir.join(INTEL_FILENAME);
    let content =
        serde_json::to_string_pretty(intel).map_err(|e| format!("Serialize error: {}", e))?;
    atomic_write_str(&path, &content).map_err(|e| format!("Write error: {}", e))?;
    Ok(())
}

/// Check if intelligence.json exists in an entity directory.
pub fn intelligence_exists(dir: &Path) -> bool {
    dir.join(INTEL_FILENAME).exists()
}

// =============================================================================
// Migration: CompanyOverview → intelligence.json
// =============================================================================

/// Migrate legacy CompanyOverview from dashboard.json into intelligence.json.
///
/// Non-destructive: creates intelligence.json if it doesn't exist and
/// dashboard.json has a company_overview. Leaves dashboard.json untouched.
/// Returns the created IntelligenceJson, or None if no migration needed.
pub fn migrate_company_overview_to_intelligence(
    workspace: &Path,
    account: &DbAccount,
    overview: &CompanyOverview,
) -> Option<IntelligenceJson> {
    let dir = crate::accounts::resolve_account_dir(workspace, account);

    // Don't overwrite existing intelligence
    if intelligence_exists(&dir) {
        return None;
    }

    // Only migrate if there's actual content
    if overview.description.is_none()
        && overview.industry.is_none()
        && overview.size.is_none()
        && overview.headquarters.is_none()
    {
        return None;
    }

    let intel = IntelligenceJson {
        version: 1,
        entity_id: account.id.clone(),
        entity_type: "account".to_string(),
        enriched_at: overview
            .enriched_at
            .clone()
            .unwrap_or_else(|| Utc::now().to_rfc3339()),
        company_context: Some(CompanyContext {
            description: overview.description.clone(),
            industry: overview.industry.clone(),
            size: overview.size.clone(),
            headquarters: overview.headquarters.clone(),
            additional_context: None,
        }),
        ..Default::default()
    };

    match write_intelligence_json(&dir, &intel) {
        Ok(()) => {
            log::info!(
                "Migrated CompanyOverview → intelligence.json for '{}'",
                account.name
            );
            Some(intel)
        }
        Err(e) => {
            log::warn!(
                "Failed to migrate intelligence for '{}': {}",
                account.name,
                e
            );
            None
        }
    }
}

// =============================================================================
// DB Cache Operations
// =============================================================================

impl ActionDb {
    /// Insert or update the entity_intelligence cache row.
    pub fn upsert_entity_intelligence(
        &self,
        intel: &IntelligenceJson,
    ) -> Result<(), rusqlite::Error> {
        self.conn_ref().execute(
            "INSERT INTO entity_intelligence (
                entity_id, entity_type, enriched_at, source_file_count,
                executive_assessment, risks_json, recent_wins_json,
                current_state_json, stakeholder_insights_json,
                next_meeting_readiness_json, company_context_json
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            ON CONFLICT(entity_id) DO UPDATE SET
                entity_type = excluded.entity_type,
                enriched_at = excluded.enriched_at,
                source_file_count = excluded.source_file_count,
                executive_assessment = excluded.executive_assessment,
                risks_json = excluded.risks_json,
                recent_wins_json = excluded.recent_wins_json,
                current_state_json = excluded.current_state_json,
                stakeholder_insights_json = excluded.stakeholder_insights_json,
                next_meeting_readiness_json = excluded.next_meeting_readiness_json,
                company_context_json = excluded.company_context_json",
            rusqlite::params![
                intel.entity_id,
                intel.entity_type,
                intel.enriched_at,
                intel.source_file_count,
                intel.executive_assessment,
                serde_json::to_string(&intel.risks).ok(),
                serde_json::to_string(&intel.recent_wins).ok(),
                serde_json::to_string(&intel.current_state).ok(),
                serde_json::to_string(&intel.stakeholder_insights).ok(),
                serde_json::to_string(&intel.next_meeting_readiness).ok(),
                serde_json::to_string(&intel.company_context).ok(),
            ],
        )?;
        Ok(())
    }

    /// Get cached entity intelligence.
    pub fn get_entity_intelligence(
        &self,
        entity_id: &str,
    ) -> Result<Option<IntelligenceJson>, rusqlite::Error> {
        let mut stmt = self.conn_ref().prepare(
            "SELECT entity_id, entity_type, enriched_at, source_file_count,
                    executive_assessment, risks_json, recent_wins_json,
                    current_state_json, stakeholder_insights_json,
                    next_meeting_readiness_json, company_context_json
             FROM entity_intelligence WHERE entity_id = ?1",
        )?;

        let result = stmt.query_row(rusqlite::params![entity_id], |row| {
            let risks_json: Option<String> = row.get(5)?;
            let wins_json: Option<String> = row.get(6)?;
            let state_json: Option<String> = row.get(7)?;
            let stakeholder_json: Option<String> = row.get(8)?;
            let readiness_json: Option<String> = row.get(9)?;
            let company_json: Option<String> = row.get(10)?;

            Ok(IntelligenceJson {
                version: 1,
                entity_id: row.get(0)?,
                entity_type: row.get(1)?,
                enriched_at: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                source_file_count: row.get::<_, Option<usize>>(3)?.unwrap_or(0),
                source_manifest: Vec::new(), // Not cached in DB
                executive_assessment: row.get(4)?,
                risks: risks_json
                    .and_then(|j| serde_json::from_str(&j).ok())
                    .unwrap_or_default(),
                recent_wins: wins_json
                    .and_then(|j| serde_json::from_str(&j).ok())
                    .unwrap_or_default(),
                current_state: state_json.and_then(|j| serde_json::from_str(&j).ok()),
                stakeholder_insights: stakeholder_json
                    .and_then(|j| serde_json::from_str(&j).ok())
                    .unwrap_or_default(),
                value_delivered: Vec::new(), // Not cached in DB (stored in file only)
                next_meeting_readiness: readiness_json.and_then(|j| serde_json::from_str(&j).ok()),
                company_context: company_json.and_then(|j| serde_json::from_str(&j).ok()),
            })
        });

        match result {
            Ok(intel) => Ok(Some(intel)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Delete cached entity intelligence.
    pub fn delete_entity_intelligence(&self, entity_id: &str) -> Result<(), rusqlite::Error> {
        self.conn_ref().execute(
            "DELETE FROM entity_intelligence WHERE entity_id = ?1",
            rusqlite::params![entity_id],
        )?;
        Ok(())
    }
}

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
    if let Ok(signals) = db.list_recent_email_signals_for_entity(entity_id, 12) {
        if !signals.is_empty() {
            let lines: Vec<String> = signals
                .iter()
                .map(|s| {
                    format!(
                        "- [{}] {} (urgency: {}, confidence: {:.2}, at: {})",
                        s.signal_type,
                        s.signal_text,
                        s.urgency.as_deref().unwrap_or("unknown"),
                        s.confidence.unwrap_or(0.0),
                        s.detected_at
                    )
                })
                .collect();
            ctx.recent_email_signals = lines.join("\n");
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

    // --- File manifest + summaries (I139: summary-based, priority-ordered) ---
    let files = db.get_entity_files(entity_id).unwrap_or_default();
    let is_incremental = prior.is_some();
    // Summaries are much denser than raw text — raise char caps.
    // Initial: 25K (~50 file summaries at 500 chars each). Incremental: 15K.
    let max_chars: usize = if is_incremental { 15_000 } else { 25_000 };
    let enriched_at = prior.map(|p| p.enriched_at.as_str()).unwrap_or("");

    // Only consider files modified within the last 90 days
    let cutoff_90d = (Utc::now() - chrono::Duration::days(90)).to_rfc3339();

    ctx.file_manifest = files
        .iter()
        .filter(|f| content_date_rfc3339(&f.filename, &f.modified_at) >= cutoff_90d)
        .take(30)
        .map(|f| SourceManifestEntry {
            filename: f.filename.clone(),
            modified_at: f.modified_at.clone(),
            format: Some(f.format.clone()),
            content_type: Some(f.content_type.clone()),
        })
        .collect();

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

    let mut file_parts: Vec<String> = Vec::new();
    let mut total_chars = 0;

    for file in ranked_files {
        // Skip files older than 90 days (use filename date when available)
        if content_date_rfc3339(&file.filename, &file.modified_at) < cutoff_90d {
            continue;
        }

        // In incremental mode, only include files modified since last enrichment
        if is_incremental && !enriched_at.is_empty() && file.modified_at.as_str() <= enriched_at {
            continue;
        }

        // Use pre-computed summary from SQLite — no filesystem I/O on hot path
        if let Some(ref summary) = file.summary {
            file_parts.push(format!(
                "--- {} [{}] ({}) ---\n{}",
                file.filename, file.content_type, file.modified_at, summary
            ));
            total_chars += summary.len();
            if total_chars >= max_chars {
                break;
            }
        }
    }

    if !file_parts.is_empty() {
        ctx.file_contents = file_parts.join("\n\n");
    }

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
) -> String {
    let is_incremental = ctx.prior_intelligence.is_some();
    let entity_label = match entity_type {
        "account" => "customer account",
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
        name = entity_name
    ));

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
        prompt.push_str(&ctx.facts_block);
        prompt.push_str("\n\n");
    }

    // Prior intelligence (incremental only)
    if let Some(ref prior) = ctx.prior_intelligence {
        prompt.push_str("## Prior Intelligence (update, don't replace wholesale)\n");
        prompt.push_str(prior);
        prompt.push_str("\n\n");
    }

    // Next meeting
    if let Some(ref meeting) = ctx.next_meeting {
        prompt.push_str("## Next Meeting\n");
        prompt.push_str(meeting);
        prompt.push_str("\n\n");
    }

    // Signals from SQLite
    if !ctx.meeting_history.is_empty() {
        prompt.push_str("## Meeting History (last 90 days)\n");
        prompt.push_str(&ctx.meeting_history);
        prompt.push_str("\n\n");
    }

    if !ctx.open_actions.is_empty() {
        prompt.push_str("## Open Actions\n");
        prompt.push_str(&ctx.open_actions);
        prompt.push_str("\n\n");
    }

    if !ctx.recent_captures.is_empty() {
        prompt.push_str("## Recent Captures (wins/risks/decisions)\n");
        prompt.push_str(&ctx.recent_captures);
        prompt.push_str("\n\n");
    }

    if !ctx.recent_email_signals.is_empty() {
        prompt.push_str("## Recent Email Signals\n");
        prompt.push_str(&ctx.recent_email_signals);
        prompt.push_str("\n\n");
    }

    if !ctx.stakeholders.is_empty() {
        prompt.push_str("## Stakeholders\n");
        prompt.push_str(&ctx.stakeholders);
        prompt.push_str("\n\n");
    }

    // File manifest (always shown so Claude knows what exists)
    if !ctx.file_manifest.is_empty() {
        prompt.push_str("## Workspace Files\n");
        for f in &ctx.file_manifest {
            let ct = f.content_type.as_deref().unwrap_or("general");
            prompt.push_str(&format!(
                "- {} [{}] ({}, {})\n",
                f.filename,
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
        prompt.push_str(&ctx.file_contents);
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
        prompt.push_str(&ctx.recent_transcripts);
        prompt.push_str("\n\n");
    }

    // Writing style instructions
    prompt.push_str(
        "WRITING RULES:\n\
         - Lead with conclusions, not evidence. State the \"so what\" first.\n\
         - Be concise. Every sentence must earn its place.\n\
         - Use numbered footnote references [1], [2] etc. List sources at the end.\n\
         - Do NOT embed filenames or source references inline in prose.\n\
         - Do NOT narrate chronologically. Synthesize themes and conclusions.\n\
         - Write for a busy executive who has 60 seconds to understand this account.\n\n",
    );

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
    prompt.push_str(&format!(
        "Return ONLY the structured block below — no other text before or after.\n\n\
         INTELLIGENCE\n\
         EXECUTIVE_ASSESSMENT:\n\
         <2-4 paragraphs separated by blank lines (\\n\\n), structured as follows:\n\
         Paragraph 1: One-sentence verdict — {p1_framing} in plain language. Then 2-3 sentences on why.\n\
         Paragraph 2: Top risk or blocker — what could go wrong and what depends on it.\n\
         Paragraph 3: Biggest opportunity — where upside exists and what unlocks it.\n\
         Paragraph 4 (optional): Key unknowns or intelligence gaps that need resolution.\n\
         IMPORTANT: Separate each paragraph with a blank line. Do NOT run paragraphs together.\n\
         Use [1] [2] footnote references. Max 250 words total.>\n\
         SOURCES:\n\
         [1] <filename>\n\
         [2] <filename>\n\
         END_EXECUTIVE_ASSESSMENT\n\
         RISK: <risk text> | SOURCE: <filename> | URGENCY: <critical|watch|low>\n\
         WIN: <win text> | SOURCE: <filename> | IMPACT: <business impact>\n\
         WORKING: <what's going well>\n\
         NOT_WORKING: <what needs attention>\n\
         UNKNOWN: <knowledge gap that should be resolved>\n\
         STAKEHOLDER: <name> | ROLE: <role> | ASSESSMENT: <1-2 sentences> | ENGAGEMENT: <high|medium|low|unknown>\n\
         (Engagement criteria — base ONLY on call transcript evidence above:\n\
          high = drives discussion, asks detailed questions, proposes next steps, references prior conversations\n\
          medium = participates and responds but follows rather than leads\n\
          low = mostly silent, reactive only, brief responses, or absent from recent calls\n\
          unknown = person not present in available transcripts)\n\
         VALUE: <date> | <value statement> | SOURCE: <filename> | IMPACT: <impact>\n\
         NEXT_MEETING_PREP: <preparation item for next meeting>\n",
    ));

    // Company context (initial only)
    if !is_incremental && entity_type == "account" {
        prompt.push_str(
            "COMPANY_DESCRIPTION: <1 paragraph about what the company does>\n\
             COMPANY_INDUSTRY: <primary industry>\n\
             COMPANY_SIZE: <employee count or range>\n\
             COMPANY_HQ: <headquarters city and country>\n\
             COMPANY_CONTEXT: <any additional relevant business context>\n",
        );
    }

    prompt.push_str("END_INTELLIGENCE\n");

    prompt
}

// =============================================================================
// Response Parser (I131)
// =============================================================================

/// Parse Claude's intelligence response into an IntelligenceJson.
pub fn parse_intelligence_response(
    response: &str,
    entity_id: &str,
    entity_type: &str,
    source_file_count: usize,
    manifest: Vec<SourceManifestEntry>,
) -> Result<IntelligenceJson, String> {
    // Find the INTELLIGENCE ... END_INTELLIGENCE block
    let block =
        extract_intelligence_block(response).ok_or("No INTELLIGENCE block found in response")?;

    let mut intel = IntelligenceJson {
        version: 1,
        entity_id: entity_id.to_string(),
        entity_type: entity_type.to_string(),
        enriched_at: Utc::now().to_rfc3339(),
        source_file_count,
        source_manifest: manifest,
        ..Default::default()
    };

    // Parse executive assessment (multi-line between markers)
    intel.executive_assessment = extract_multiline_field(&block, "EXECUTIVE_ASSESSMENT:");

    // Parse single-line fields
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

// =============================================================================
// Enrichment Orchestrator (I131)
// =============================================================================

/// Enrich an entity's intelligence via Claude Code.
///
/// Flow:
/// 1. Read prior intelligence.json (if exists) → determines initial vs incremental
/// 2. Build IntelligenceContext from SQLite + files
/// 3. Build prompt (entity-type parameterized)
/// 4. Spawn Claude Code via PTY (120s timeout)
/// 5. Parse response → IntelligenceJson
/// 6. Write intelligence.json (atomic)
/// 7. Update DB cache
/// 8. Return IntelligenceJson
pub struct EntityEnrichmentTarget<'a> {
    pub entity_id: &'a str,
    pub entity_name: &'a str,
    pub entity_type: &'a str,
    pub account: Option<&'a DbAccount>,
    pub project: Option<&'a crate::db::DbProject>,
}

pub fn enrich_entity_intelligence(
    workspace: &Path,
    db: &ActionDb,
    target: EntityEnrichmentTarget<'_>,
    pty: &crate::pty::PtyManager,
) -> Result<IntelligenceJson, String> {
    let EntityEnrichmentTarget {
        entity_id,
        entity_name,
        entity_type,
        account,
        project,
    } = target;

    // Resolve entity directory
    let entity_dir = match entity_type {
        "account" => {
            if let Some(acct) = account {
                crate::accounts::resolve_account_dir(workspace, acct)
            } else {
                crate::accounts::account_dir(workspace, entity_name)
            }
        }
        "project" => crate::projects::project_dir(workspace, entity_name),
        "person" => crate::people::person_dir(workspace, entity_name),
        _ => return Err(format!("Unsupported entity type: {}", entity_type)),
    };

    // Step 1: Read prior intelligence
    let prior = read_intelligence_json(&entity_dir).ok();

    // Step 2: Build context
    let ctx = build_intelligence_context(
        workspace,
        db,
        entity_id,
        entity_type,
        account,
        project,
        prior.as_ref(),
        None,
    );

    // Step 3: Build prompt
    // For person entities, we'd need the relationship from the DB but this code path
    // doesn't have the person record. The intel_queue path (which is the primary path)
    // handles this correctly. Pass None here for the legacy direct-enrichment path.
    let prompt = build_intelligence_prompt(entity_name, entity_type, &ctx, None);

    // Step 4: Spawn Claude Code
    let output = pty
        .spawn_claude(workspace, &prompt)
        .map_err(|e| format!("Claude Code error: {}", e))?;

    // Step 5: Parse response
    let intel = parse_intelligence_response(
        &output.stdout,
        entity_id,
        entity_type,
        ctx.file_manifest.len(),
        ctx.file_manifest,
    )?;

    // Step 6: Write intelligence.json
    write_intelligence_json(&entity_dir, &intel)?;

    // Step 7: Update DB cache
    let _ = db.upsert_entity_intelligence(&intel);

    log::info!(
        "Enriched intelligence for {} '{}' ({} risks, {} wins, {} stakeholders)",
        entity_type,
        entity_name,
        intel.risks.len(),
        intel.recent_wins.len(),
        intel.stakeholder_insights.len(),
    );

    Ok(intel)
}

// =============================================================================
// Markdown Generation (I134 — three-file dashboard.md)
// =============================================================================

/// Format intelligence sections as markdown for inclusion in dashboard.md.
///
/// Used by both `write_account_markdown()` and `write_project_markdown()` to
/// inject synthesized intelligence into the generated artifact. Returns empty
/// string if there's nothing meaningful to render.
pub fn format_intelligence_markdown(intel: &IntelligenceJson) -> String {
    let mut md = String::new();

    // Executive Assessment — the most important section
    if let Some(ref assessment) = intel.executive_assessment {
        if !assessment.is_empty() {
            md.push_str("## Executive Assessment\n\n");
            md.push_str(assessment);
            md.push_str("\n\n");
            if !intel.enriched_at.is_empty() {
                md.push_str(&format!(
                    "_Last enriched: {}_\n\n",
                    intel
                        .enriched_at
                        .split('T')
                        .next()
                        .unwrap_or(&intel.enriched_at)
                ));
            }
        }
    }

    // Risks
    if !intel.risks.is_empty() {
        md.push_str("## Risks\n\n");
        for r in &intel.risks {
            md.push_str(&format!("- **{}** {}", r.urgency, r.text));
            if let Some(ref source) = r.source {
                md.push_str(&format!(" _(source: {})_", source));
            }
            md.push('\n');
        }
        md.push('\n');
    }

    // Recent Wins
    if !intel.recent_wins.is_empty() {
        md.push_str("## Recent Wins\n\n");
        for w in &intel.recent_wins {
            md.push_str(&format!("- {}", w.text));
            if let Some(ref impact) = w.impact {
                md.push_str(&format!(" \u{2014} {}", impact));
            }
            if let Some(ref source) = w.source {
                md.push_str(&format!(" _(source: {})_", source));
            }
            md.push('\n');
        }
        md.push('\n');
    }

    // Current State
    if let Some(ref state) = intel.current_state {
        let has_content = !state.working.is_empty()
            || !state.not_working.is_empty()
            || !state.unknowns.is_empty();
        if has_content {
            md.push_str("## Current State\n\n");
            if !state.working.is_empty() {
                md.push_str("### What's Working\n\n");
                for item in &state.working {
                    md.push_str(&format!("- {}\n", item));
                }
                md.push('\n');
            }
            if !state.not_working.is_empty() {
                md.push_str("### What's Not Working\n\n");
                for item in &state.not_working {
                    md.push_str(&format!("- {}\n", item));
                }
                md.push('\n');
            }
            if !state.unknowns.is_empty() {
                md.push_str("### Unknowns\n\n");
                for item in &state.unknowns {
                    md.push_str(&format!("- {}\n", item));
                }
                md.push('\n');
            }
        }
    }

    // Next Meeting Readiness
    if let Some(ref readiness) = intel.next_meeting_readiness {
        if !readiness.prep_items.is_empty() {
            md.push_str("## Next Meeting Readiness\n\n");
            if let Some(ref title) = readiness.meeting_title {
                md.push_str(&format!("**{}**", title));
                if let Some(ref date) = readiness.meeting_date {
                    md.push_str(&format!(" on {}", date));
                }
                md.push_str("\n\n");
            }
            for item in &readiness.prep_items {
                md.push_str(&format!("- {}\n", item));
            }
            md.push('\n');
        }
    }

    // Stakeholder Insights
    if !intel.stakeholder_insights.is_empty() {
        md.push_str("## Stakeholder Insights\n\n");
        for s in &intel.stakeholder_insights {
            md.push_str(&format!("### {}", s.name));
            if let Some(ref role) = s.role {
                md.push_str(&format!(" \u{2014} {}", role));
            }
            md.push('\n');
            if let Some(ref assessment) = s.assessment {
                md.push_str(assessment);
            }
            if let Some(ref engagement) = s.engagement {
                md.push_str(&format!(" Engagement: {}.", engagement));
            }
            if let Some(ref source) = s.source {
                md.push_str(&format!(" _(source: {})_", source));
            }
            md.push_str("\n\n");
        }
    }

    // Value Delivered
    if !intel.value_delivered.is_empty() {
        md.push_str("## Value Delivered\n\n");
        for v in &intel.value_delivered {
            md.push_str("- ");
            if let Some(ref date) = v.date {
                md.push_str(&format!("**{}** ", date));
            }
            md.push_str(&v.statement);
            if let Some(ref impact) = v.impact {
                md.push_str(&format!(" \u{2014} {}", impact));
            }
            if let Some(ref source) = v.source {
                md.push_str(&format!(" _(source: {})_", source));
            }
            md.push('\n');
        }
        md.push('\n');
    }

    // Company / Project Context (from web search or overview)
    if let Some(ref ctx) = intel.company_context {
        let has_content = ctx.description.is_some()
            || ctx.industry.is_some()
            || ctx.size.is_some()
            || ctx.headquarters.is_some()
            || ctx.additional_context.is_some();
        if has_content {
            md.push_str("## Company Context\n\n");
            if let Some(ref desc) = ctx.description {
                md.push_str(desc);
                md.push_str("\n\n");
            }
            if let Some(ref industry) = ctx.industry {
                md.push_str(&format!("**Industry:** {}  \n", industry));
            }
            if let Some(ref size) = ctx.size {
                md.push_str(&format!("**Size:** {}  \n", size));
            }
            if let Some(ref hq) = ctx.headquarters {
                md.push_str(&format!("**Headquarters:** {}  \n", hq));
            }
            if let Some(ref additional) = ctx.additional_context {
                md.push_str(&format!("\n{}\n", additional));
            }
            md.push('\n');
        }
    }

    md
}

// =============================================================================
// Content Indexing (shared logic for accounts + projects)
// =============================================================================

/// Files to skip during content indexing (managed by the app).
pub(crate) const CONTENT_SKIP_FILES: &[&str] = &[
    "dashboard.json",
    "dashboard.md",
    "intelligence.json",
    ".DS_Store",
];

/// Recursively collect content files from an entity directory.
///
/// Skips hidden files/dirs, underscore-prefixed dirs, managed files,
/// and child entity boundaries (subdirs containing dashboard.json).
/// Used by both account and project content indexing.
pub(crate) fn collect_content_files(
    dir: &std::path::Path,
    _entity_root: &std::path::Path,
    out: &mut Vec<std::path::PathBuf>,
) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };

        // Skip hidden and underscore-prefixed entries at every level
        if name.starts_with('.') || name.starts_with('_') {
            continue;
        }

        if path.is_dir() {
            // Stop at child entity boundaries — subdirs with their own dashboard.json
            // are separate entities and indexed independently
            if path.join("dashboard.json").exists() {
                continue;
            }
            collect_content_files(&path, _entity_root, out);
        } else {
            if CONTENT_SKIP_FILES.contains(&name.as_str()) {
                continue;
            }
            out.push(path);
        }
    }
}

// =============================================================================
// Content Classification + Mechanical Summary (I139)
// =============================================================================

/// Classify content type from filename and format. Returns `(content_type, priority)`.
///
/// Pure mechanical — no AI cost, deterministic. First pattern match wins.
/// Priority scale: 5 (general) to 10 (dashboard).
pub(crate) fn classify_content(filename: &str, format: &str) -> (&'static str, i32) {
    let lower = filename.to_lowercase();

    if lower.contains("dashboard") {
        return ("dashboard", 10);
    }
    if lower.contains("transcript")
        || lower.contains("recording")
        || lower.contains("call-notes")
        || lower.contains("call_notes")
    {
        return ("transcript", 9);
    }
    if lower.contains("stakeholder")
        || lower.contains("org-chart")
        || lower.contains("relationship")
    {
        return ("stakeholder-map", 9);
    }
    if lower.contains("success-plan")
        || lower.contains("success_plan")
        || lower.contains("strategy")
    {
        return ("success-plan", 8);
    }
    if lower.contains("qbr")
        || (lower.contains("quarterly") && lower.contains("review"))
        || lower.contains("business-review")
    {
        return ("qbr", 8);
    }
    if lower.contains("contract")
        || lower.contains("agreement")
        || lower.contains("sow")
        || lower.contains("msa")
    {
        return ("contract", 7);
    }
    if lower.contains("notes") || lower.contains("memo") || lower.contains("minutes") {
        return ("notes", 7);
    }
    if format == "Pptx" {
        return ("presentation", 6);
    }
    if format == "Xlsx" {
        return ("spreadsheet", 6);
    }

    ("general", 5)
}

/// Extract a semantic content date from a filename as an RFC3339 string.
///
/// Many workspace files follow the pattern `YYYY-MM-DD-description.ext`. The embedded date
/// is the *content* date (when the meeting/event happened), which is more useful for filtering
/// than the filesystem mtime (which reflects when the file was copied/synced).
/// Returns `YYYY-MM-DDT00:00:00+00:00` if a date prefix is found, else `modified_at`.
pub(crate) fn content_date_rfc3339(filename: &str, modified_at: &str) -> String {
    if filename.len() >= 10 {
        let prefix = &filename[..10];
        if prefix.as_bytes()[4] == b'-'
            && prefix.as_bytes()[7] == b'-'
            && prefix[..4].chars().all(|c| c.is_ascii_digit())
            && prefix[5..7].chars().all(|c| c.is_ascii_digit())
            && prefix[8..10].chars().all(|c| c.is_ascii_digit())
        {
            return format!("{}T00:00:00+00:00", prefix);
        }
    }
    modified_at.to_string()
}

/// Apply a recency boost: files from the last 30 days get +1 priority (capped at 10).
///
/// Uses the filename-embedded date when available (more reliable than filesystem mtime
/// for files that have been copied/synced).
pub(crate) fn apply_recency_boost(base_priority: i32, filename: &str, modified_at: &str) -> i32 {
    let cutoff_30d = (Utc::now() - chrono::Duration::days(30)).to_rfc3339();
    let effective_date = content_date_rfc3339(filename, modified_at);
    if effective_date >= cutoff_30d {
        (base_priority + 1).min(10)
    } else {
        base_priority
    }
}

/// Generate a mechanical summary from extracted text.
///
/// Extracts markdown headings as table of contents + first non-heading paragraph
/// as context. Target: ~`max_chars` chars per file. Zero AI cost.
pub(crate) fn mechanical_summary(text: &str, max_chars: usize) -> String {
    if text.is_empty() {
        return String::new();
    }

    let mut headings: Vec<&str> = Vec::new();
    let mut first_paragraph: Option<&str> = None;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with('#') {
            // Strip the leading '#' characters and whitespace for cleaner output
            let heading_text = trimmed.trim_start_matches('#').trim();
            if !heading_text.is_empty() {
                headings.push(heading_text);
            }
        } else if first_paragraph.is_none() {
            // First non-empty, non-heading line is the context paragraph
            first_paragraph = Some(trimmed);
        }
    }

    let mut result = String::new();

    if let Some(para) = first_paragraph {
        result.push_str(para);
    }

    if !headings.is_empty() {
        if !result.is_empty() {
            result.push_str("\n\nSections: ");
        } else {
            result.push_str("Sections: ");
        }
        result.push_str(&headings.join(", "));
    }

    if result.is_empty() {
        // Fallback: take first max_chars of raw text
        let truncated = &text[..text.len().min(max_chars)];
        return truncated.to_string();
    }

    if result.len() > max_chars {
        // Truncate to max_chars at a word boundary if possible
        let truncated = &result[..max_chars];
        if let Some(last_space) = truncated.rfind(' ') {
            return result[..last_space].to_string();
        }
        return truncated.to_string();
    }

    result
}

/// Extract text from a file and produce a mechanical summary.
/// Returns `(extracted_at, summary)`. Both are `None` if extraction fails.
pub(crate) fn extract_and_summarize(path: &std::path::Path) -> (Option<String>, Option<String>) {
    match crate::processor::extract::extract_text(path) {
        Ok(text) if !text.is_empty() => {
            let summary = mechanical_summary(&text, 500);
            let extracted_at = Utc::now().to_rfc3339();
            (
                Some(extracted_at),
                if summary.is_empty() {
                    None
                } else {
                    Some(summary)
                },
            )
        }
        _ => (None, None),
    }
}

/// Sync the content index for any entity. Compares filesystem against DB,
/// adds new files, updates changed files, removes deleted files.
///
/// Entity-generic: works for accounts, projects, and future entity types.
/// Returns `(added, updated, removed)` counts.
pub(crate) fn sync_content_index_for_entity(
    entity_dir: &std::path::Path,
    entity_id: &str,
    entity_type: &str,
    workspace: &std::path::Path,
    db: &ActionDb,
) -> Result<(usize, usize, usize), String> {
    use std::collections::HashMap;

    if !entity_dir.exists() {
        return Ok((0, 0, 0));
    }

    let now = Utc::now().to_rfc3339();
    let mut added = 0usize;
    let mut updated = 0usize;
    let mut removed = 0usize;

    // Build a HashMap of existing DB records for this entity (O(1) lookup)
    let existing = db
        .get_entity_files(entity_id)
        .map_err(|e| format!("DB error: {}", e))?;
    let mut db_map: HashMap<String, crate::db::DbContentFile> =
        existing.into_iter().map(|f| (f.id.clone(), f)).collect();

    // Scan the filesystem recursively
    let mut file_paths: Vec<std::path::PathBuf> = Vec::new();
    collect_content_files(entity_dir, entity_dir, &mut file_paths);

    for path in &file_paths {
        let filename = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };

        // Detect format via existing extract module
        let format = crate::processor::extract::detect_format(path);
        let format_label = format!("{:?}", format);

        // Get file metadata
        let metadata = match std::fs::metadata(path) {
            Ok(m) => m,
            Err(_) => continue,
        };
        let file_size = metadata.len() as i64;
        let modified_at = metadata
            .modified()
            .ok()
            .map(|t| {
                let dt: chrono::DateTime<Utc> = t.into();
                dt.to_rfc3339()
            })
            .unwrap_or_else(|| now.clone());

        // Compute relative path from workspace root
        let relative_path = path
            .strip_prefix(workspace)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| filename.clone());

        // Use path relative to entity dir for stable, collision-free IDs
        let rel_from_entity = path
            .strip_prefix(entity_dir)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| filename.clone());

        let id = crate::util::slugify(&format!("{}/{}", entity_id, rel_from_entity));

        // Classify content type + priority from filename and format
        let (content_type, base_priority) = classify_content(&filename, &format_label);
        let priority = apply_recency_boost(base_priority, &filename, &modified_at);

        // Check if record exists in DB
        if let Some(existing_record) = db_map.remove(&id) {
            // File exists in DB — check if it changed (compare modified_at)
            if existing_record.modified_at != modified_at || existing_record.file_size != file_size
            {
                // File changed — extract summary for new content
                let (extracted_at_val, summary_val) = extract_and_summarize(path);
                let record = crate::db::DbContentFile {
                    id,
                    entity_id: entity_id.to_string(),
                    entity_type: entity_type.to_string(),
                    filename,
                    relative_path,
                    absolute_path: path.to_string_lossy().to_string(),
                    format: format_label,
                    file_size,
                    modified_at,
                    indexed_at: now.clone(),
                    extracted_at: extracted_at_val,
                    summary: summary_val,
                    embeddings_generated_at: None,
                    content_type: content_type.to_string(),
                    priority,
                };
                let _ = db.upsert_content_file(&record);
                updated += 1;
            } else if existing_record.summary.is_none() {
                // Unchanged but never summarized — backfill summary
                let (extracted_at_val, summary_val) = extract_and_summarize(path);
                if summary_val.is_some() {
                    let _ = db.update_content_extraction(
                        &existing_record.id,
                        &extracted_at_val.unwrap_or_else(|| now.clone()),
                        summary_val.as_deref(),
                        Some(content_type),
                        Some(priority),
                    );
                }
            }
            // Unchanged with existing summary — skip
        } else {
            // New file — extract summary + insert
            let (extracted_at_val, summary_val) = extract_and_summarize(path);
            let record = crate::db::DbContentFile {
                id,
                entity_id: entity_id.to_string(),
                entity_type: entity_type.to_string(),
                filename,
                relative_path,
                absolute_path: path.to_string_lossy().to_string(),
                format: format_label,
                file_size,
                modified_at,
                indexed_at: now.clone(),
                extracted_at: extracted_at_val,
                summary: summary_val,
                embeddings_generated_at: None,
                content_type: content_type.to_string(),
                priority,
            };
            let _ = db.upsert_content_file(&record);
            added += 1;
        }
    }

    // Any records left in db_map no longer have matching files — remove them
    for id in db_map.keys() {
        let _ = db.delete_content_file(id);
        removed += 1;
    }

    Ok((added, updated, removed))
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn test_db() -> ActionDb {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("entity_intel_test.db");
        std::mem::forget(dir);
        ActionDb::open_at(path).expect("open test db")
    }

    fn sample_intel() -> IntelligenceJson {
        IntelligenceJson {
            version: 1,
            entity_id: "acme-corp".to_string(),
            entity_type: "account".to_string(),
            enriched_at: "2026-02-01T10:00:00Z".to_string(),
            source_file_count: 3,
            source_manifest: vec![SourceManifestEntry {
                filename: "qbr-notes.md".to_string(),
                modified_at: "2026-01-30T10:00:00Z".to_string(),
                format: Some("markdown".to_string()),
                content_type: Some("qbr".to_string()),
            }],
            executive_assessment: Some(
                "Acme is in a strong position with steady renewal trajectory.".to_string(),
            ),
            risks: vec![IntelRisk {
                text: "Champion leaving in Q2".to_string(),
                source: Some("qbr-notes.md".to_string()),
                urgency: "critical".to_string(),
            }],
            recent_wins: vec![IntelWin {
                text: "Expanded to 3 new teams".to_string(),
                source: Some("capture".to_string()),
                impact: Some("20% seat growth".to_string()),
            }],
            current_state: Some(CurrentState {
                working: vec!["Onboarding flow".to_string()],
                not_working: vec!["Reporting integration".to_string()],
                unknowns: vec!["Budget for next year".to_string()],
            }),
            stakeholder_insights: vec![StakeholderInsight {
                name: "Alice VP".to_string(),
                role: Some("VP Engineering".to_string()),
                assessment: Some("Strong advocate, drives adoption.".to_string()),
                engagement: Some("high".to_string()),
                source: Some("meetings".to_string()),
            }],
            value_delivered: vec![ValueItem {
                date: Some("2026-01-15".to_string()),
                statement: "Reduced onboarding time by 40%".to_string(),
                source: Some("qbr-deck.pdf".to_string()),
                impact: Some("$50k savings".to_string()),
            }],
            next_meeting_readiness: Some(MeetingReadiness {
                meeting_title: Some("Weekly sync".to_string()),
                meeting_date: Some("2026-02-05".to_string()),
                prep_items: vec![
                    "Review reporting blockers".to_string(),
                    "Prepare champion transition plan".to_string(),
                ],
            }),
            company_context: Some(CompanyContext {
                description: Some("Enterprise SaaS platform.".to_string()),
                industry: Some("Technology".to_string()),
                size: Some("500-1000".to_string()),
                headquarters: Some("San Francisco, USA".to_string()),
                additional_context: None,
            }),
        }
    }

    #[test]
    fn test_intelligence_json_roundtrip() {
        let intel = sample_intel();
        let json_str = serde_json::to_string_pretty(&intel).expect("serialize");
        let parsed: IntelligenceJson = serde_json::from_str(&json_str).expect("deserialize");

        assert_eq!(parsed.entity_id, "acme-corp");
        assert_eq!(parsed.entity_type, "account");
        assert_eq!(parsed.risks.len(), 1);
        assert_eq!(parsed.risks[0].urgency, "critical");
        assert_eq!(parsed.recent_wins.len(), 1);
        assert_eq!(parsed.stakeholder_insights.len(), 1);
        assert_eq!(parsed.value_delivered.len(), 1);
        assert!(parsed.next_meeting_readiness.is_some());
        assert!(parsed.company_context.is_some());
        assert_eq!(parsed.source_manifest.len(), 1);
    }

    #[test]
    fn test_intelligence_json_missing_fields() {
        // Minimal JSON — serde should fill defaults for all missing fields
        let json_str = r#"{"entityId": "beta", "entityType": "project"}"#;
        let parsed: IntelligenceJson = serde_json::from_str(json_str).expect("deserialize");

        assert_eq!(parsed.entity_id, "beta");
        assert_eq!(parsed.entity_type, "project");
        assert_eq!(parsed.version, 1);
        assert!(parsed.risks.is_empty());
        assert!(parsed.recent_wins.is_empty());
        assert!(parsed.executive_assessment.is_none());
        assert!(parsed.current_state.is_none());
        assert!(parsed.company_context.is_none());
    }

    #[test]
    fn test_write_read_intelligence_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let intel = sample_intel();

        write_intelligence_json(dir.path(), &intel).expect("write");
        assert!(intelligence_exists(dir.path()));

        let read_back = read_intelligence_json(dir.path()).expect("read");
        assert_eq!(read_back.entity_id, "acme-corp");
        assert_eq!(read_back.risks.len(), 1);
        assert_eq!(read_back.source_file_count, 3);
    }

    #[test]
    fn test_migrate_company_overview() {
        let dir = tempfile::tempdir().expect("tempdir");
        let workspace = dir.path();

        // Create account directory
        let acct_dir = workspace.join("Accounts/Acme Corp");
        std::fs::create_dir_all(&acct_dir).expect("mkdir");

        let account = DbAccount {
            id: "acme-corp".to_string(),
            name: "Acme Corp".to_string(),
            lifecycle: None,
            arr: None,
            health: None,
            contract_start: None,
            contract_end: None,
            nps: None,
            tracker_path: Some("Accounts/Acme Corp".to_string()),
            parent_id: None,
            is_internal: false,
            updated_at: Utc::now().to_rfc3339(),
            archived: false,
        };

        let overview = CompanyOverview {
            description: Some("Cloud platform company.".to_string()),
            industry: Some("SaaS".to_string()),
            size: Some("200-500".to_string()),
            headquarters: Some("NYC".to_string()),
            enriched_at: Some("2026-01-15T10:00:00Z".to_string()),
        };

        let result = migrate_company_overview_to_intelligence(workspace, &account, &overview);
        assert!(result.is_some());

        let intel = result.unwrap();
        assert_eq!(intel.entity_id, "acme-corp");
        assert_eq!(intel.entity_type, "account");
        assert!(intel.company_context.is_some());
        let ctx = intel.company_context.unwrap();
        assert_eq!(ctx.description.as_deref(), Some("Cloud platform company."));
        assert_eq!(ctx.industry.as_deref(), Some("SaaS"));

        // File should exist now
        assert!(intelligence_exists(&acct_dir));

        // Second migration should return None (file already exists)
        let second = migrate_company_overview_to_intelligence(workspace, &account, &overview);
        assert!(second.is_none());
    }

    #[test]
    fn test_migrate_empty_overview_skipped() {
        let dir = tempfile::tempdir().expect("tempdir");
        let workspace = dir.path();
        let acct_dir = workspace.join("Accounts/Empty Corp");
        std::fs::create_dir_all(&acct_dir).expect("mkdir");

        let account = DbAccount {
            id: "empty-corp".to_string(),
            name: "Empty Corp".to_string(),
            lifecycle: None,
            arr: None,
            health: None,
            contract_start: None,
            contract_end: None,
            nps: None,
            tracker_path: Some("Accounts/Empty Corp".to_string()),
            parent_id: None,
            is_internal: false,
            updated_at: Utc::now().to_rfc3339(),
            archived: false,
        };

        let overview = CompanyOverview {
            description: None,
            industry: None,
            size: None,
            headquarters: None,
            enriched_at: None,
        };

        let result = migrate_company_overview_to_intelligence(workspace, &account, &overview);
        assert!(result.is_none());
    }

    #[test]
    fn test_db_upsert_get_entity_intelligence() {
        let db = test_db();
        let intel = sample_intel();

        db.upsert_entity_intelligence(&intel).expect("upsert");

        let fetched = db
            .get_entity_intelligence("acme-corp")
            .expect("get")
            .expect("should exist");

        assert_eq!(fetched.entity_id, "acme-corp");
        assert_eq!(fetched.entity_type, "account");
        assert_eq!(fetched.executive_assessment, intel.executive_assessment);
        assert_eq!(fetched.risks.len(), 1);
        assert_eq!(fetched.risks[0].urgency, "critical");
        assert_eq!(fetched.recent_wins.len(), 1);
        assert_eq!(fetched.stakeholder_insights.len(), 1);
        assert!(fetched.company_context.is_some());
    }

    #[test]
    fn test_db_intelligence_missing_returns_none() {
        let db = test_db();
        let result = db
            .get_entity_intelligence("nonexistent")
            .expect("should not error");
        assert!(result.is_none());
    }

    #[test]
    fn test_db_delete_entity_intelligence() {
        let db = test_db();
        let intel = sample_intel();

        db.upsert_entity_intelligence(&intel).expect("upsert");
        assert!(db.get_entity_intelligence("acme-corp").unwrap().is_some());

        db.delete_entity_intelligence("acme-corp").expect("delete");
        assert!(db.get_entity_intelligence("acme-corp").unwrap().is_none());
    }

    #[test]
    fn test_db_upsert_overwrites() {
        let db = test_db();
        let mut intel = sample_intel();

        db.upsert_entity_intelligence(&intel).expect("first upsert");

        // Update the assessment
        intel.executive_assessment = Some("Updated assessment.".to_string());
        intel.risks.push(IntelRisk {
            text: "New risk".to_string(),
            source: None,
            urgency: "watch".to_string(),
        });

        db.upsert_entity_intelligence(&intel)
            .expect("second upsert");

        let fetched = db.get_entity_intelligence("acme-corp").unwrap().unwrap();
        assert_eq!(
            fetched.executive_assessment.as_deref(),
            Some("Updated assessment.")
        );
        assert_eq!(fetched.risks.len(), 2);
    }

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
            }],
            file_contents: "--- qbr.md [qbr] (2026-01-30) ---\nContent here".to_string(),
            recent_transcripts: String::new(),
            prior_intelligence: None, // Initial mode
            next_meeting: Some("2026-02-05 — Weekly sync".to_string()),
        };

        let prompt = build_intelligence_prompt("Acme Corp", "account", &ctx, None);

        assert!(prompt.contains("INITIAL intelligence build"));
        assert!(prompt.contains("Acme Corp"));
        assert!(prompt.contains("Health: green"));
        assert!(prompt.contains("QBR"));
        assert!(prompt.contains("renewal"));
        assert!(prompt.contains("COMPANY_DESCRIPTION:"));
        assert!(prompt.contains("INTELLIGENCE"));
        assert!(prompt.contains("END_INTELLIGENCE"));
        // I139: prompt refinements
        assert!(prompt.contains("Lead with conclusions"));
        assert!(prompt.contains("footnote references"));
        assert!(prompt.contains("Max 250 words"));
        assert!(prompt.contains("SOURCES:"));
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

        let prompt = build_intelligence_prompt("Project X", "project", &ctx, None);

        assert!(prompt.contains("INCREMENTAL update"));
        assert!(prompt.contains("Prior."));
        assert!(!prompt.contains("COMPANY_DESCRIPTION:"));
    }

    #[test]
    fn test_build_intelligence_prompt_person_external() {
        let ctx = IntelligenceContext {
            facts_block: "Role: VP Engineering".to_string(),
            meeting_history: "- 2026-02-01 | Weekly sync".to_string(),
            ..Default::default()
        };

        let prompt =
            build_intelligence_prompt("Alice Chen", "person", &ctx, Some("external"));

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
            build_intelligence_prompt("Bob Kim", "person", &ctx, Some("internal"));

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
            build_intelligence_prompt("Unknown Person", "person", &ctx, None);

        assert!(prompt.contains("professional contact"));
        assert!(prompt.contains("Relationship type is unknown"));
        assert!(prompt.contains("relationship dynamic"));
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
        assert!(result.unwrap_err().contains("No INTELLIGENCE block"));
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

    // =========================================================================
    // I134: format_intelligence_markdown
    // =========================================================================

    #[test]
    fn test_format_intelligence_markdown_full() {
        let intel = IntelligenceJson {
            version: 1,
            entity_id: "acme".to_string(),
            entity_type: "account".to_string(),
            enriched_at: "2026-02-09T10:00:00Z".to_string(),
            source_file_count: 3,
            source_manifest: vec![],
            executive_assessment: Some("Acme is in strong position for renewal.".to_string()),
            risks: vec![IntelRisk {
                text: "Budget uncertainty for Q3".to_string(),
                source: Some("QBR notes".to_string()),
                urgency: "critical".to_string(),
            }],
            recent_wins: vec![IntelWin {
                text: "Expanded to 3 teams".to_string(),
                source: Some("capture".to_string()),
                impact: Some("20% seat growth".to_string()),
            }],
            current_state: Some(CurrentState {
                working: vec!["Onboarding flow".to_string()],
                not_working: vec!["Reporting delayed".to_string()],
                unknowns: vec!["FY budget".to_string()],
            }),
            stakeholder_insights: vec![StakeholderInsight {
                name: "Alice Chen".to_string(),
                role: Some("VP Engineering".to_string()),
                assessment: Some("Strong advocate.".to_string()),
                engagement: Some("high".to_string()),
                source: None,
            }],
            value_delivered: vec![ValueItem {
                date: Some("2026-01-15".to_string()),
                statement: "Reduced onboarding time by 40%".to_string(),
                source: Some("QBR".to_string()),
                impact: Some("$50k savings".to_string()),
            }],
            next_meeting_readiness: Some(MeetingReadiness {
                meeting_title: Some("Acme QBR".to_string()),
                meeting_date: Some("2026-02-15".to_string()),
                prep_items: vec![
                    "Review blockers".to_string(),
                    "Bring ROI metrics".to_string(),
                ],
            }),
            company_context: Some(CompanyContext {
                description: Some("Enterprise SaaS platform".to_string()),
                industry: Some("Technology".to_string()),
                size: Some("500-1000".to_string()),
                headquarters: Some("San Francisco".to_string()),
                additional_context: None,
            }),
        };

        let md = format_intelligence_markdown(&intel);

        // All sections present
        assert!(md.contains("## Executive Assessment"));
        assert!(md.contains("Acme is in strong position"));
        assert!(md.contains("_Last enriched: 2026-02-09_"));

        assert!(md.contains("## Risks"));
        assert!(md.contains("**critical** Budget uncertainty"));
        assert!(md.contains("_(source: QBR notes)_"));

        assert!(md.contains("## Recent Wins"));
        assert!(md.contains("Expanded to 3 teams"));

        assert!(md.contains("## Current State"));
        assert!(md.contains("### What's Working"));
        assert!(md.contains("### What's Not Working"));
        assert!(md.contains("### Unknowns"));

        assert!(md.contains("## Next Meeting Readiness"));
        assert!(md.contains("**Acme QBR** on 2026-02-15"));
        assert!(md.contains("Review blockers"));

        assert!(md.contains("## Stakeholder Insights"));
        assert!(md.contains("### Alice Chen"));

        assert!(md.contains("## Value Delivered"));
        assert!(md.contains("**2026-01-15** Reduced onboarding"));

        assert!(md.contains("## Company Context"));
        assert!(md.contains("Enterprise SaaS platform"));
        assert!(md.contains("**Industry:** Technology"));
    }

    #[test]
    fn test_format_intelligence_markdown_empty() {
        let intel = IntelligenceJson::default();
        let md = format_intelligence_markdown(&intel);
        assert!(
            md.is_empty(),
            "Empty intelligence should produce empty markdown"
        );
    }

    #[test]
    fn test_format_intelligence_markdown_partial() {
        let intel = IntelligenceJson {
            executive_assessment: Some("Situation looks good.".to_string()),
            enriched_at: "2026-02-09T10:00:00Z".to_string(),
            ..Default::default()
        };
        let md = format_intelligence_markdown(&intel);
        assert!(md.contains("## Executive Assessment"));
        assert!(md.contains("Situation looks good."));
        // No other sections
        assert!(!md.contains("## Risks"));
        assert!(!md.contains("## Recent Wins"));
        assert!(!md.contains("## Current State"));
    }

    // =========================================================================
    // I139: Content classification + mechanical summary tests
    // =========================================================================

    #[test]
    fn test_classify_content_dashboard() {
        let (ct, p) = classify_content("Acme-dashboard.md", "Markdown");
        assert_eq!(ct, "dashboard");
        assert_eq!(p, 10);
    }

    #[test]
    fn test_classify_content_transcript() {
        let (ct, p) = classify_content("call-transcript-2025-01-28.md", "Markdown");
        assert_eq!(ct, "transcript");
        assert_eq!(p, 9);

        let (ct2, _) = classify_content("Weekly-Recording-Notes.md", "Markdown");
        assert_eq!(ct2, "transcript");

        let (ct3, _) = classify_content("customer-call_notes-q4.md", "Markdown");
        assert_eq!(ct3, "transcript");
    }

    #[test]
    fn test_classify_content_stakeholder() {
        let (ct, p) = classify_content("stakeholder-map.md", "Markdown");
        assert_eq!(ct, "stakeholder-map");
        assert_eq!(p, 9);

        let (ct2, _) = classify_content("org-chart-acme.xlsx", "Xlsx");
        assert_eq!(ct2, "stakeholder-map");
    }

    #[test]
    fn test_classify_content_success_plan() {
        let (ct, p) = classify_content("success-plan-2026.md", "Markdown");
        assert_eq!(ct, "success-plan");
        assert_eq!(p, 8);

        let (ct2, _) = classify_content("account_strategy.md", "Markdown");
        assert_eq!(ct2, "success-plan");
    }

    #[test]
    fn test_classify_content_qbr() {
        let (ct, p) = classify_content("Q4-QBR.pptx", "Pptx");
        assert_eq!(ct, "qbr");
        assert_eq!(p, 8);

        let (ct2, _) = classify_content("quarterly-business-review-2025.md", "Markdown");
        assert_eq!(ct2, "qbr");
    }

    #[test]
    fn test_classify_content_contract() {
        let (ct, p) = classify_content("master-agreement-v2.pdf", "Pdf");
        assert_eq!(ct, "contract");
        assert_eq!(p, 7);

        let (ct2, _) = classify_content("sow-phase2.docx", "Docx");
        assert_eq!(ct2, "contract");
    }

    #[test]
    fn test_classify_content_notes() {
        let (ct, p) = classify_content("meeting-notes-jan.md", "Markdown");
        assert_eq!(ct, "notes");
        assert_eq!(p, 7);
    }

    #[test]
    fn test_classify_content_format_fallback_pptx() {
        let (ct, p) = classify_content("slide-deck.pptx", "Pptx");
        assert_eq!(ct, "presentation");
        assert_eq!(p, 6);
    }

    #[test]
    fn test_classify_content_format_fallback_xlsx() {
        let (ct, p) = classify_content("data.xlsx", "Xlsx");
        assert_eq!(ct, "spreadsheet");
        assert_eq!(p, 6);
    }

    #[test]
    fn test_classify_content_default() {
        let (ct, p) = classify_content("random-file.md", "Markdown");
        assert_eq!(ct, "general");
        assert_eq!(p, 5);
    }

    #[test]
    fn test_classify_content_case_insensitive() {
        let (ct, _) = classify_content("ACME-DASHBOARD.MD", "Markdown");
        assert_eq!(ct, "dashboard");

        let (ct2, _) = classify_content("Call-Transcript-Feb.md", "Markdown");
        assert_eq!(ct2, "transcript");
    }

    #[test]
    fn test_recency_boost() {
        let recent = Utc::now().to_rfc3339();
        // No date prefix → falls back to modified_at
        assert_eq!(apply_recency_boost(5, "some-file.md", &recent), 6);
        assert_eq!(apply_recency_boost(10, "some-file.md", &recent), 10); // capped at 10

        let old = "2020-01-01T00:00:00+00:00";
        assert_eq!(apply_recency_boost(5, "some-file.md", old), 5); // no boost

        // Filename date takes precedence over modified_at
        let today = Utc::now().format("%Y-%m-%d").to_string();
        let dated_filename = format!("{}-meeting-notes.md", today);
        // Even with old mtime, recent filename date gets the boost
        assert_eq!(apply_recency_boost(5, &dated_filename, old), 6);

        // Old filename date, even with recent mtime → no boost
        assert_eq!(
            apply_recency_boost(5, "2020-01-15-old-notes.md", &recent),
            5
        );
    }

    #[test]
    fn test_content_date_rfc3339() {
        // Filename with date prefix
        assert_eq!(
            content_date_rfc3339("2024-09-13-meeting.md", "2026-02-09T12:00:00+00:00"),
            "2024-09-13T00:00:00+00:00"
        );
        // No date prefix → falls back to modified_at
        assert_eq!(
            content_date_rfc3339("notes.md", "2026-02-09T12:00:00+00:00"),
            "2026-02-09T12:00:00+00:00"
        );
        // Short filename
        assert_eq!(
            content_date_rfc3339("a.md", "2026-02-09T12:00:00+00:00"),
            "2026-02-09T12:00:00+00:00"
        );
    }

    #[test]
    fn test_mechanical_summary_markdown() {
        let text = "# Account Overview\n\nAcme Corp is a leading SaaS provider.\n\n## Health\n\nCurrently green.\n\n## Risks\n\nBudget uncertainty.\n";
        let summary = mechanical_summary(text, 500);

        assert!(summary.contains("Acme Corp is a leading SaaS provider."));
        assert!(summary.contains("Sections:"));
        assert!(summary.contains("Account Overview"));
        assert!(summary.contains("Health"));
        assert!(summary.contains("Risks"));
    }

    #[test]
    fn test_mechanical_summary_plain_text() {
        let text = "This is a plain text document without any markdown headings. It has some content that should be captured as the first paragraph.";
        let summary = mechanical_summary(text, 500);

        assert!(summary.starts_with("This is a plain text"));
        assert!(!summary.contains("Sections:"));
    }

    #[test]
    fn test_mechanical_summary_empty() {
        let summary = mechanical_summary("", 500);
        assert!(summary.is_empty());
    }

    #[test]
    fn test_mechanical_summary_truncation() {
        let text = "# Header\n\nA very long paragraph that goes on and on. ".repeat(20);
        let summary = mechanical_summary(&text, 100);
        assert!(summary.len() <= 100);
    }

    #[test]
    fn test_mechanical_summary_headings_only() {
        let text = "# Overview\n## Details\n## Timeline\n";
        let summary = mechanical_summary(text, 500);
        assert!(summary.starts_with("Sections:"));
        assert!(summary.contains("Overview"));
        assert!(summary.contains("Details"));
        assert!(summary.contains("Timeline"));
    }

    #[test]
    fn test_entity_files_sorted_by_priority() {
        let db = test_db();
        let now = Utc::now().to_rfc3339();

        // Insert a low-priority file
        let low = crate::db::DbContentFile {
            id: "sort-test/general".to_string(),
            entity_id: "sort-test".to_string(),
            entity_type: "account".to_string(),
            filename: "random.md".to_string(),
            relative_path: "Accounts/Sort/random.md".to_string(),
            absolute_path: "/tmp/workspace/Accounts/Sort/random.md".to_string(),
            format: "Markdown".to_string(),
            file_size: 100,
            modified_at: now.clone(),
            indexed_at: now.clone(),
            extracted_at: None,
            summary: None,
            embeddings_generated_at: None,
            content_type: "general".to_string(),
            priority: 5,
        };
        db.upsert_content_file(&low).unwrap();

        // Insert a high-priority file
        let high = crate::db::DbContentFile {
            id: "sort-test/dashboard".to_string(),
            entity_id: "sort-test".to_string(),
            entity_type: "account".to_string(),
            filename: "dashboard.md".to_string(),
            relative_path: "Accounts/Sort/dashboard.md".to_string(),
            absolute_path: "/tmp/workspace/Accounts/Sort/dashboard.md".to_string(),
            format: "Markdown".to_string(),
            file_size: 200,
            modified_at: now.clone(),
            indexed_at: now.clone(),
            extracted_at: None,
            summary: None,
            embeddings_generated_at: None,
            content_type: "dashboard".to_string(),
            priority: 10,
        };
        db.upsert_content_file(&high).unwrap();

        // Insert a mid-priority file
        let mid = crate::db::DbContentFile {
            id: "sort-test/notes".to_string(),
            entity_id: "sort-test".to_string(),
            entity_type: "account".to_string(),
            filename: "notes.md".to_string(),
            relative_path: "Accounts/Sort/notes.md".to_string(),
            absolute_path: "/tmp/workspace/Accounts/Sort/notes.md".to_string(),
            format: "Markdown".to_string(),
            file_size: 150,
            modified_at: now.clone(),
            indexed_at: now.clone(),
            extracted_at: None,
            summary: None,
            embeddings_generated_at: None,
            content_type: "notes".to_string(),
            priority: 7,
        };
        db.upsert_content_file(&mid).unwrap();

        let files = db.get_entity_files("sort-test").unwrap();
        assert_eq!(files.len(), 3);
        assert_eq!(files[0].content_type, "dashboard"); // priority 10
        assert_eq!(files[1].content_type, "notes"); // priority 7
        assert_eq!(files[2].content_type, "general"); // priority 5
    }
}
