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

/// A record of a user edit to an intelligence field.
///
/// Stored in intelligence.json to protect user corrections from being
/// overwritten by subsequent AI enrichment cycles.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserEdit {
    /// JSON path to the edited field (e.g. "executiveAssessment", "stakeholderInsights[0].name").
    pub field_path: String,
    /// ISO 8601 timestamp of when the edit was made.
    pub edited_at: String,
}

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

    /// User edits — field paths that the user has manually corrected.
    /// Enrichment cycles preserve these fields instead of overwriting them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub user_edits: Vec<UserEdit>,
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
    /// Whether this file was selected for the prompt context (vs skipped by budget).
    #[serde(default = "default_selected", skip_serializing_if = "is_true")]
    pub selected: bool,
    /// Reason the file was skipped (only set when selected=false).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip_reason: Option<String>,
}

fn default_selected() -> bool {
    true
}

fn is_true(v: &bool) -> bool {
    *v
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

pub(crate) fn default_urgency() -> String {
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

/// Maximum bytes of file content to include in the intelligence prompt context.
/// Keeps prompt size manageable (~10KB) while preserving the most relevant signals.
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
// Field Update (User Edits)
// =============================================================================

/// Navigate a serde_json::Value by a dotted/indexed path and set the value.
///
/// Supports paths like:
/// - `"executiveAssessment"` → root field
/// - `"stakeholderInsights[0].name"` → array index + field
/// - `"currentState.working[0]"` → nested field + array index
/// - `"risks[2].text"` → array index + field
fn set_json_path(root: &mut serde_json::Value, path: &str, value: serde_json::Value) -> Result<(), String> {
    let segments = parse_path_segments(path)?;
    let mut current = root;

    for (i, seg) in segments.iter().enumerate() {
        let is_last = i == segments.len() - 1;
        match seg {
            PathSegment::Field(name) => {
                if is_last {
                    current[name.as_str()] = value;
                    return Ok(());
                }
                current = current
                    .get_mut(name.as_str())
                    .ok_or_else(|| format!("Field '{}' not found at segment '{}'", path, name))?;
            }
            PathSegment::Index(name, idx) => {
                let arr = current
                    .get_mut(name.as_str())
                    .ok_or_else(|| format!("Field '{}' not found", name))?;
                let arr = arr
                    .as_array_mut()
                    .ok_or_else(|| format!("Field '{}' is not an array", name))?;
                if *idx >= arr.len() {
                    return Err(format!("Index {} out of bounds for '{}' (len {})", idx, name, arr.len()));
                }
                if is_last {
                    arr[*idx] = value;
                    return Ok(());
                }
                current = &mut arr[*idx];
            }
        }
    }
    Err(format!("Empty path: '{}'", path))
}

enum PathSegment {
    Field(String),
    Index(String, usize),
}

/// Parse "stakeholderInsights[0].name" into [Index("stakeholderInsights", 0), Field("name")]
fn parse_path_segments(path: &str) -> Result<Vec<PathSegment>, String> {
    let mut segments = Vec::new();
    for part in path.split('.') {
        if let Some(bracket_pos) = part.find('[') {
            let name = &part[..bracket_pos];
            let rest = &part[bracket_pos + 1..];
            let idx_str = rest.trim_end_matches(']');
            let idx: usize = idx_str
                .parse()
                .map_err(|_| format!("Invalid index in path segment: '{}'", part))?;
            segments.push(PathSegment::Index(name.to_string(), idx));
        } else {
            segments.push(PathSegment::Field(part.to_string()));
        }
    }
    Ok(segments)
}

/// Apply a field update to an intelligence.json on disk.
///
/// Reads the file, applies the update via JSON path navigation,
/// records a UserEdit entry, validates by re-parsing, and writes back.
pub fn apply_intelligence_field_update(
    dir: &Path,
    field_path: &str,
    value: &str,
) -> Result<IntelligenceJson, String> {
    let intel_path = dir.join(INTEL_FILENAME);
    let content = std::fs::read_to_string(&intel_path)
        .map_err(|e| format!("Failed to read {}: {}", intel_path.display(), e))?;

    let mut json_val: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse {}: {}", intel_path.display(), e))?;

    // Apply the update
    let new_value: serde_json::Value = serde_json::from_str(value)
        .unwrap_or_else(|_| serde_json::Value::String(value.to_string()));
    set_json_path(&mut json_val, field_path, new_value)?;

    // Record user edit (dedup: replace existing edit for same path)
    let edits = json_val
        .get_mut("userEdits")
        .and_then(|v| v.as_array_mut());
    let edit_entry = serde_json::json!({
        "fieldPath": field_path,
        "editedAt": Utc::now().to_rfc3339(),
    });
    if let Some(arr) = edits {
        arr.retain(|e| e.get("fieldPath").and_then(|v| v.as_str()) != Some(field_path));
        arr.push(edit_entry);
    } else {
        json_val["userEdits"] = serde_json::json!([edit_entry]);
    }

    // Validate by re-parsing into typed struct
    let intel: IntelligenceJson = serde_json::from_value(json_val)
        .map_err(|e| format!("Updated JSON is invalid: {}", e))?;

    // Write back
    write_intelligence_json(dir, &intel)?;

    Ok(intel)
}

/// Replace the stakeholderInsights array and record as user-edited.
pub fn apply_stakeholders_update(
    dir: &Path,
    stakeholders: Vec<StakeholderInsight>,
) -> Result<IntelligenceJson, String> {
    let mut intel = read_intelligence_json(dir)?;
    intel.stakeholder_insights = stakeholders;

    // Record user edit
    let now = Utc::now().to_rfc3339();
    intel.user_edits.retain(|e| e.field_path != "stakeholderInsights");
    intel.user_edits.push(UserEdit {
        field_path: "stakeholderInsights".to_string(),
        edited_at: now,
    });

    write_intelligence_json(dir, &intel)?;
    Ok(intel)
}

/// Resolve entity directory from workspace, entity_type, and DB records.
pub fn resolve_entity_dir(
    workspace: &Path,
    entity_type: &str,
    entity_name: &str,
    account: Option<&DbAccount>,
) -> Result<std::path::PathBuf, String> {
    match entity_type {
        "account" => {
            if let Some(acct) = account {
                Ok(crate::accounts::resolve_account_dir(workspace, acct))
            } else {
                Ok(crate::accounts::account_dir(workspace, entity_name))
            }
        }
        "project" => Ok(crate::projects::project_dir(workspace, entity_name)),
        "person" => Ok(crate::people::person_dir(workspace, entity_name)),
        _ => Err(format!("Unsupported entity type: {}", entity_type)),
    }
}

/// Preserve user-edited fields from an existing intelligence after AI enrichment.
///
/// For each field in `user_edits`, copies the value from `existing` into `new_intel`,
/// then carries forward the `user_edits` list.
pub fn preserve_user_edits(new_intel: &mut IntelligenceJson, existing: &IntelligenceJson) {
    if existing.user_edits.is_empty() {
        return;
    }

    // Serialize both to serde_json::Value for field-level operations
    let existing_val: serde_json::Value = match serde_json::to_value(existing) {
        Ok(v) => v,
        Err(_) => return,
    };
    let mut new_val: serde_json::Value = match serde_json::to_value(&*new_intel) {
        Ok(v) => v,
        Err(_) => return,
    };

    for edit in &existing.user_edits {
        // Read the user-edited value from existing
        if let Some(val) = get_json_path(&existing_val, &edit.field_path) {
            let _ = set_json_path(&mut new_val, &edit.field_path, val.clone());
        }
    }

    // Re-parse and carry forward user_edits
    if let Ok(mut restored) = serde_json::from_value::<IntelligenceJson>(new_val) {
        restored.user_edits = existing.user_edits.clone();
        *new_intel = restored;
    }
}

/// Read a value at a JSON path (for preserve_user_edits).
fn get_json_path<'a>(root: &'a serde_json::Value, path: &str) -> Option<&'a serde_json::Value> {
    let segments = parse_path_segments(path).ok()?;
    let mut current = root;

    for seg in &segments {
        match seg {
            PathSegment::Field(name) => {
                current = current.get(name.as_str())?;
            }
            PathSegment::Index(name, idx) => {
                let arr = current.get(name.as_str())?.as_array()?;
                current = arr.get(*idx)?;
            }
        }
    }
    Some(current)
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
                user_edits: Vec::new(), // Not cached in DB (stored in file only)
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
// Keyword extraction from enrichment response (I305)
// =============================================================================

/// Extract keywords from an AI intelligence response.
/// Parses the JSON to find the `keywords` array and returns it as a JSON string.
pub fn extract_keywords_from_response(response: &str) -> Option<String> {
    // Try to find JSON block in the response
    let json_str = if let Some(start) = response.find('{') {
        let depth_track = response[start..].chars().fold((0i32, 0usize), |(depth, end), ch| {
            let new_depth = match ch {
                '{' => depth + 1,
                '}' => depth - 1,
                _ => depth,
            };
            if new_depth == 0 && depth > 0 {
                (0, end + 1)
            } else {
                (new_depth, end + ch.len_utf8())
            }
        });
        &response[start..start + depth_track.1]
    } else {
        return None;
    };

    let parsed: serde_json::Value = serde_json::from_str(json_str).ok()?;
    let keywords = parsed.get("keywords")?.as_array()?;

    let kw_strings: Vec<String> = keywords
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .filter(|s| !s.is_empty() && s.len() < 100) // Sanity: skip empty or absurdly long entries
        .take(20) // Cap at 20 keywords
        .collect();

    if kw_strings.is_empty() {
        return None;
    }

    serde_json::to_string(&kw_strings).ok()
}

/// Compute a human-readable age string from an ISO 8601 timestamp.

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_utils::test_db;

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
                selected: true,
                skip_reason: None,
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
            user_edits: Vec::new(),
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
            keywords: None,
            keywords_extracted_at: None,
        metadata: None,
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
            keywords: None,
            keywords_extracted_at: None,
        metadata: None,
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
            user_edits: Vec::new(),
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
