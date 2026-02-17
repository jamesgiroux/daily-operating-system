//! Account workspace file I/O (I72 / ADR-0047).
//!
//! Each account gets a directory under `Accounts/` in the workspace:
//!   Accounts/{Name}/dashboard.json  — canonical data (app + external tools write here)
//!   Accounts/{Name}/dashboard.md    — rich artifact (generated from JSON + SQLite)
//!
//! Three-way sync (ADR-0047):
//!   App edit → writes dashboard.json → syncs to SQLite → regenerates dashboard.md
//!   External edit to JSON → detected by startup scan → syncs to SQLite
//!   External edit to markdown → no auto-reconcile (markdown is generated)

use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::db::{ActionDb, DbAccount};
use crate::util::{slugify, wrap_user_data};

// =============================================================================
// JSON Schema
// =============================================================================

/// JSON schema for dashboard.json files.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountJson {
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default = "default_entity_type")]
    pub entity_type: String,
    pub structured: AccountStructured,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub company_overview: Option<CompanyOverview>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub strategic_programs: Vec<StrategicProgram>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub custom_sections: Vec<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
}

fn default_version() -> u32 {
    1
}
fn default_entity_type() -> String {
    "account".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountStructured {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arr: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lifecycle: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub renewal_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nps: Option<i32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub account_team: Vec<AccountTeamEntry>,
    /// Legacy import-only field (kept for backward compatibility).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub csm: Option<String>,
    /// Legacy import-only field (kept for backward compatibility).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub champion: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountTeamEntry {
    pub person_id: String,
    pub name: String,
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompanyOverview {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub industry: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headquarters: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enriched_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StrategicProgram {
    pub name: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

// =============================================================================
// Filesystem I/O
// =============================================================================

/// Resolve the directory for an account's workspace files (I70: sanitized name).
pub fn account_dir(workspace: &Path, name: &str) -> PathBuf {
    workspace
        .join("Accounts")
        .join(crate::util::sanitize_for_filesystem(name))
}

/// Resolve the directory for an account, preferring `tracker_path` when set (I114).
///
/// Child accounts have `tracker_path` like `Accounts/Cox/Consumer-Brands` which
/// correctly resolves to a nested directory. Falls back to `account_dir()` for
/// flat accounts without a tracker_path.
pub fn resolve_account_dir(workspace: &Path, account: &DbAccount) -> PathBuf {
    if let Some(ref tp) = account.tracker_path {
        workspace.join(tp)
    } else {
        account_dir(workspace, &account.name)
    }
}

/// Check if a subdirectory name looks like a Business Unit (I114).
///
/// BU directories have human-readable names (no numeric prefix).
/// Internal org folders start with digits (`01-Customer-Information`, `02-Meetings`).
/// App-managed entity subdirs (`Call-Transcripts`, etc.) are excluded (ADR-0059).
/// We already skip `_`/`.`-prefixed dirs elsewhere.
pub fn is_bu_directory(name: &str) -> bool {
    !name.starts_with(|c: char| c.is_ascii_digit())
        && !name.starts_with('_')
        && !name.starts_with('.')
        && !crate::util::MANAGED_ENTITY_DIRS.contains(&name)
}

/// Write `dashboard.json` for an account.
///
/// Merges structured DB fields with narrative JSON fields. If a JSON file
/// already exists, narrative fields (overview, programs, notes) are preserved
/// and only structured fields are updated from the DB.
pub fn write_account_json(
    workspace: &Path,
    account: &DbAccount,
    existing_json: Option<&AccountJson>,
    _db: &ActionDb,
) -> Result<(), String> {
    let dir = resolve_account_dir(workspace, account);
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create {}: {}", dir.display(), e))?;

    let account_team = _db
        .get_account_team(&account.id)
        .unwrap_or_default()
        .into_iter()
        .map(|m| AccountTeamEntry {
            person_id: m.person_id,
            name: m.person_name,
            role: m.role,
            email: Some(m.person_email),
        })
        .collect();

    let json = AccountJson {
        version: 1,
        entity_type: "account".to_string(),
        structured: AccountStructured {
            arr: account.arr,
            health: account.health.clone(),
            lifecycle: account.lifecycle.clone(),
            renewal_date: account.contract_end.clone(),
            nps: account.nps,
            account_team,
            csm: None,
            champion: None,
        },
        company_overview: existing_json.and_then(|j| j.company_overview.clone()),
        strategic_programs: existing_json
            .map(|j| j.strategic_programs.clone())
            .unwrap_or_default(),
        notes: existing_json.and_then(|j| j.notes.clone()),
        custom_sections: existing_json
            .map(|j| j.custom_sections.clone())
            .unwrap_or_default(),
        parent_id: account.parent_id.clone(),
    };

    let path = dir.join("dashboard.json");
    let content =
        serde_json::to_string_pretty(&json).map_err(|e| format!("Serialize error: {}", e))?;
    crate::util::atomic_write_str(&path, &content).map_err(|e| format!("Write error: {}", e))?;

    Ok(())
}

/// Write `dashboard.md` for an account (generated artifact).
///
/// Combines structured data from SQLite with narrative data from JSON
/// and auto-generated sections from meeting/action/capture history.
pub fn write_account_markdown(
    workspace: &Path,
    account: &DbAccount,
    json: Option<&AccountJson>,
    db: &ActionDb,
) -> Result<(), String> {
    let dir = resolve_account_dir(workspace, account);
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create {}: {}", dir.display(), e))?;

    let mut md = String::new();

    // Header
    md.push_str(&format!("# {}\n\n", account.name));

    // Health badge
    if let Some(ref health) = account.health {
        let emoji = match health.as_str() {
            "green" => "🟢",
            "yellow" => "🟡",
            "red" => "🔴",
            _ => "⚪",
        };
        md.push_str(&format!("**Health:** {} {}  \n", emoji, health));
    }
    if let Some(ref lifecycle) = account.lifecycle {
        md.push_str(&format!("**Lifecycle:** {}  \n", lifecycle));
    }
    if let Some(arr) = account.arr {
        md.push_str(&format!("**ARR:** ${:.0}  \n", arr));
    }
    if let Some(ref end) = account.contract_end {
        md.push_str(&format!("**Renewal:** {}  \n", end));
    }
    if let Some(nps) = account.nps {
        md.push_str(&format!("**NPS:** {}  \n", nps));
    }
    md.push('\n');

    if let Ok(team) = db.get_account_team(&account.id) {
        if !team.is_empty() {
            md.push_str("## Account Team\n\n");
            for member in team {
                md.push_str(&format!(
                    "- **{}** — {}\n",
                    member.person_name,
                    member.role.to_uppercase()
                ));
            }
            md.push('\n');
        }
    }

    // Read intelligence.json once (used for Company Overview skip + intelligence sections)
    let intel_data =
        crate::entity_intel::read_intelligence_json(&resolve_account_dir(workspace, account)).ok();

    // Company Overview (from JSON — skipped when intelligence.json has company_context)
    let intel_has_company = intel_data
        .as_ref()
        .and_then(|i| i.company_context.as_ref())
        .is_some();
    if !intel_has_company {
        if let Some(overview) = json.and_then(|j| j.company_overview.as_ref()) {
            md.push_str("## Company Overview\n\n");
            if let Some(ref desc) = overview.description {
                md.push_str(desc);
                md.push_str("\n\n");
            }
            if let Some(ref industry) = overview.industry {
                md.push_str(&format!("**Industry:** {}  \n", industry));
            }
            if let Some(ref size) = overview.size {
                md.push_str(&format!("**Size:** {}  \n", size));
            }
            if let Some(ref hq) = overview.headquarters {
                md.push_str(&format!("**Headquarters:** {}  \n", hq));
            }
            md.push('\n');
        }
    }

    // Strategic Programs (from JSON)
    if let Some(programs) = json.map(|j| &j.strategic_programs) {
        if !programs.is_empty() {
            md.push_str("## Strategic Programs\n\n");
            for p in programs.iter() {
                let status_badge = match p.status.as_str() {
                    "completed" => "✅",
                    "in_progress" => "🔄",
                    "planned" => "📋",
                    _ => "•",
                };
                md.push_str(&format!("- {} **{}** — {}", status_badge, p.name, p.status));
                if let Some(ref notes) = p.notes {
                    md.push_str(&format!(" — {}", notes));
                }
                md.push('\n');
            }
            md.push('\n');
        }
    }

    // Notes (from JSON)
    if let Some(notes) = json.and_then(|j| j.notes.as_ref()) {
        if !notes.is_empty() {
            md.push_str("## Notes\n\n");
            md.push_str(notes);
            md.push_str("\n\n");
        }
    }

    // === Intelligence sections (I134 — from intelligence.json) ===

    if let Some(ref intel) = intel_data {
        let intel_md = crate::entity_intel::format_intelligence_markdown(intel);
        if !intel_md.is_empty() {
            md.push_str(&intel_md);
        }
    }

    // === Auto-generated sections below ===

    // Business Units (I114 — parent accounts only)
    let children = db.get_child_accounts(&account.id).unwrap_or_default();
    if !children.is_empty() {
        md.push_str("## Business Units\n\n");
        for child in &children {
            let health_badge = match child.health.as_deref() {
                Some("green") => "🟢",
                Some("yellow") => "🟡",
                Some("red") => "🔴",
                _ => "⚪",
            };
            let arr_str = child
                .arr
                .map(|a| format!(" — ${:.0}", a))
                .unwrap_or_default();
            md.push_str(&format!(
                "- {} **{}**{}\n",
                health_badge, child.name, arr_str
            ));
        }
        md.push('\n');
    }

    // Recent Meetings
    md.push_str("<!-- auto-generated -->\n");
    md.push_str("## Recent Meetings\n\n");
    match db.get_meetings_for_account(&account.id, 10) {
        Ok(meetings) if !meetings.is_empty() => {
            for m in &meetings {
                md.push_str(&format!(
                    "- **{}** — {} ({})\n",
                    m.start_time.split('T').next().unwrap_or(&m.start_time),
                    m.title,
                    m.meeting_type,
                ));
            }
            md.push('\n');
        }
        _ => {
            md.push_str("_No meetings recorded yet._\n\n");
        }
    }

    // Open Actions
    md.push_str("## Open Actions\n\n");
    match db.get_account_actions(&account.id) {
        Ok(actions) if !actions.is_empty() => {
            for a in &actions {
                let due = a
                    .due_date
                    .as_deref()
                    .map(|d| format!(" (due {})", d))
                    .unwrap_or_default();
                md.push_str(&format!("- [{}] **{}**{}\n", a.priority, a.title, due,));
            }
            md.push('\n');
        }
        _ => {
            md.push_str("_No open actions._\n\n");
        }
    }

    // Recent Captures
    md.push_str("## Recent Captures\n\n");
    match db.get_captures_for_account(&account.id, 90) {
        Ok(captures) if !captures.is_empty() => {
            for c in &captures {
                let icon = match c.capture_type.as_str() {
                    "win" => "🏆",
                    "risk" => "⚠️",
                    "decision" => "📌",
                    _ => "•",
                };
                md.push_str(&format!(
                    "- {} **{}** — {} ({})\n",
                    icon, c.capture_type, c.content, c.meeting_title,
                ));
            }
            md.push('\n');
        }
        _ => {
            md.push_str("_No recent captures._\n\n");
        }
    }

    // Stakeholder Map
    md.push_str("## Stakeholder Map\n\n");
    match db.get_people_for_entity(&account.id) {
        Ok(people) if !people.is_empty() => {
            for p in &people {
                let role_part = p
                    .role
                    .as_deref()
                    .map(|r| format!(" — {}", r))
                    .unwrap_or_default();
                md.push_str(&format!("- **{}**{}", p.name, role_part));
                if let Some(ref org) = p.organization {
                    md.push_str(&format!(" ({})", org));
                }
                md.push('\n');
            }
            md.push('\n');
        }
        _ => {
            md.push_str("_No people linked yet._\n\n");
        }
    }

    // Engagement Signals
    md.push_str("## Engagement Signals\n\n");
    match db.get_stakeholder_signals(&account.id) {
        Ok(signals) => {
            md.push_str(&format!(
                "- **30-day meetings:** {}\n",
                signals.meeting_frequency_30d
            ));
            md.push_str(&format!(
                "- **90-day meetings:** {}\n",
                signals.meeting_frequency_90d
            ));
            md.push_str(&format!("- **Temperature:** {}\n", signals.temperature));
            md.push_str(&format!("- **Trend:** {}\n", signals.trend));
            if let Some(ref last) = signals.last_meeting {
                md.push_str(&format!(
                    "- **Last meeting:** {}\n",
                    last.split('T').next().unwrap_or(last)
                ));
            }
            md.push('\n');
        }
        Err(_) => {
            md.push_str("_No signal data available._\n\n");
        }
    }

    let path = dir.join("dashboard.md");
    crate::util::atomic_write_str(&path, &md).map_err(|e| format!("Write error: {}", e))?;

    Ok(())
}

// =============================================================================
// Read
// =============================================================================

/// Result of reading a dashboard.json file.
pub struct ReadAccountResult {
    pub account: DbAccount,
    pub json: AccountJson,
}

/// Read a dashboard.json file and convert to DbAccount + narrative fields.
///
/// Supports both flat (`Accounts/Acme/dashboard.json`) and nested/child
/// (`Accounts/Cox/Consumer-Brands/dashboard.json`) paths (I114).
///
/// Depth detection: if the grandparent of the JSON file is `Accounts`, this is
/// a flat account. If it's deeper, the immediate parent dir is the BU name and
/// the grandparent is the parent account name.
pub fn read_account_json(path: &Path) -> Result<ReadAccountResult, String> {
    let content = std::fs::read_to_string(path).map_err(|e| format!("Read error: {}", e))?;
    let json: AccountJson =
        serde_json::from_str(&content).map_err(|e| format!("Parse error: {}", e))?;

    // Get file mtime as updated_at
    let updated_at = std::fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
        .map(|t| {
            let dt: chrono::DateTime<Utc> = t.into();
            dt.to_rfc3339()
        })
        .unwrap_or_else(|| Utc::now().to_rfc3339());

    // Depth detection: is grandparent "Accounts"?
    // Flat:   workspace/Accounts/{name}/dashboard.json  → parent.parent.filename == "Accounts"
    // Child:  workspace/Accounts/{parent}/{child}/dashboard.json → parent.parent.filename != "Accounts"
    let account_dir = path.parent().ok_or("No parent dir")?;
    let grandparent = account_dir.parent().ok_or("No grandparent dir")?;
    let grandparent_name = grandparent
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");

    let is_child = grandparent_name != "Accounts";

    if is_child {
        // Child account: Accounts/{parent_name}/{child_name}/dashboard.json
        let child_name = account_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown")
            .to_string();
        let parent_name = grandparent_name.to_string();
        let parent_id = slugify(&parent_name);
        let child_id = format!("{}--{}", parent_id, slugify(&child_name));
        let tracker_path = Some(format!("Accounts/{}/{}", parent_name, child_name));

        Ok(ReadAccountResult {
            account: DbAccount {
                id: child_id,
                name: child_name,
                lifecycle: json.structured.lifecycle.clone(),
                arr: json.structured.arr,
                health: json.structured.health.clone(),
                contract_start: None,
                contract_end: json.structured.renewal_date.clone(),
                nps: json.structured.nps,
                tracker_path,
                parent_id: Some(parent_id),
                is_internal: false,
                updated_at,
                archived: false,
            },
            json,
        })
    } else {
        // Flat account: Accounts/{name}/dashboard.json
        let name = account_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown")
            .to_string();
        let id = slugify(&name);
        let tracker_path = Some(format!("Accounts/{}", name));

        Ok(ReadAccountResult {
            account: DbAccount {
                id,
                name,
                lifecycle: json.structured.lifecycle.clone(),
                arr: json.structured.arr,
                health: json.structured.health.clone(),
                contract_start: None,
                contract_end: json.structured.renewal_date.clone(),
                nps: json.structured.nps,
                tracker_path,
                parent_id: json.parent_id.clone(),
                is_internal: false,
                updated_at,
                archived: false,
            },
            json,
        })
    }
}

// =============================================================================
// Sync
// =============================================================================

/// Startup scan: sync all Accounts/*/dashboard.json files to SQLite.
///
/// For each file: compare file mtime against `accounts.updated_at` in SQLite.
/// If file is newer: parse JSON, update SQLite, regenerate dashboard.md.
/// If SQLite is newer: regenerate dashboard.json + dashboard.md from SQLite.
///
/// Returns the number of accounts synced.
pub fn sync_accounts_from_workspace(workspace: &Path, db: &ActionDb) -> Result<usize, String> {
    let accounts_dir = workspace.join("Accounts");
    let mut synced = 0;

    // Scan existing JSON files in Accounts/
    let entries = if accounts_dir.exists() {
        std::fs::read_dir(&accounts_dir)
            .map_err(|e| format!("Failed to read Accounts/: {}", e))?
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        // Skip non-directories
        if !entry.path().is_dir() {
            continue;
        }

        // Skip system/hidden folders (e.g. _Uncategorized, .DS_Store)
        let dir_name = entry.file_name();
        let name_str = dir_name.to_string_lossy();
        if name_str.starts_with('_') || name_str.starts_with('.') {
            continue;
        }

        let json_path = entry.path().join("dashboard.json");
        if !json_path.exists() {
            // Account dir exists but no JSON file
            let name = name_str;
            if let Ok(Some(db_account)) = db.get_account_by_name(&name) {
                // Already in SQLite — generate files from DB
                let _ = write_account_json(workspace, &db_account, None, db);
                let _ = write_account_markdown(workspace, &db_account, None, db);
                synced += 1;
            } else {
                // New folder discovery — bootstrap minimal record from folder name
                let now = Utc::now().to_rfc3339();
                let id = slugify(&name);
                let new_account = DbAccount {
                    id,
                    name: name.to_string(),
                    lifecycle: None,
                    arr: None,
                    health: None,
                    contract_start: None,
                    contract_end: None,
                    nps: None,
                    tracker_path: Some(format!("Accounts/{}", name)),
                    parent_id: None,
                    is_internal: false,
                    updated_at: now,
                    archived: false,
                };
                if db.upsert_account(&new_account).is_ok() {
                    let _ = write_account_json(workspace, &new_account, None, db);
                    let _ = write_account_markdown(workspace, &new_account, None, db);
                    log::info!("Bootstrapped account '{}' from existing folder", name);
                    synced += 1;
                }
            }
            continue;
        }

        match read_account_json(&json_path) {
            Ok(ReadAccountResult {
                account: file_account,
                json,
            }) => {
                match db.get_account(&file_account.id) {
                    Ok(Some(db_account)) => {
                        if file_account.updated_at > db_account.updated_at {
                            // File is newer — update SQLite, regen markdown
                            let mut merged = file_account;
                            // Preserve DB-only fields
                            merged.contract_start = db_account.contract_start.clone();
                            let _ = db.upsert_account(&merged);
                            let _ = write_account_markdown(workspace, &merged, Some(&json), db);
                            synced += 1;
                        } else if db_account.updated_at > file_account.updated_at {
                            // SQLite is newer — regen both files
                            let _ = write_account_json(workspace, &db_account, Some(&json), db);
                            let _ = write_account_markdown(workspace, &db_account, Some(&json), db);
                            synced += 1;
                        }
                    }
                    Ok(None) => {
                        // New account from file — insert to SQLite
                        let _ = db.upsert_account(&file_account);
                        let _ = write_account_markdown(workspace, &file_account, Some(&json), db);
                        synced += 1;
                    }
                    Err(_) => continue,
                }
            }
            Err(e) => {
                log::warn!("Failed to read {}: {}", json_path.display(), e);
                continue;
            }
        }
    }

    // --- Nested BU scan (I114): discover child accounts under parent directories ---
    // Re-scan each top-level Accounts/ directory for BU subdirectories.
    let top_entries = if accounts_dir.exists() {
        std::fs::read_dir(&accounts_dir)
            .map_err(|e| format!("Failed to re-read Accounts/: {}", e))?
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };

    for entry in top_entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        if !entry.path().is_dir() {
            continue;
        }
        let parent_dir_name = entry.file_name();
        let parent_name_str = parent_dir_name.to_string_lossy();
        if parent_name_str.starts_with('_') || parent_name_str.starts_with('.') {
            continue;
        }

        let parent_id = slugify(&parent_name_str);

        // Scan subdirectories for BU candidates
        let sub_entries = match std::fs::read_dir(entry.path()) {
            Ok(rd) => rd.collect::<Vec<_>>(),
            Err(_) => continue,
        };

        for sub_entry in sub_entries {
            let sub_entry = match sub_entry {
                Ok(e) => e,
                Err(_) => continue,
            };
            if !sub_entry.path().is_dir() {
                continue;
            }
            let sub_name = sub_entry.file_name();
            let sub_name_str = sub_name.to_string_lossy();

            if !is_bu_directory(&sub_name_str) {
                continue;
            }

            let child_json_path = sub_entry.path().join("dashboard.json");
            let child_id = format!("{}--{}", parent_id, slugify(&sub_name_str));

            if child_json_path.exists() {
                // BU has dashboard.json — use depth-aware read_account_json
                match read_account_json(&child_json_path) {
                    Ok(ReadAccountResult {
                        account: file_account,
                        json,
                    }) => match db.get_account(&file_account.id) {
                        Ok(Some(db_account)) => {
                            if file_account.updated_at > db_account.updated_at {
                                let mut merged = file_account;
                                merged.contract_start = db_account.contract_start.clone();
                                let _ = db.upsert_account(&merged);
                                let _ = write_account_markdown(workspace, &merged, Some(&json), db);
                                synced += 1;
                            } else if db_account.updated_at > file_account.updated_at {
                                let _ = write_account_json(workspace, &db_account, Some(&json), db);
                                let _ =
                                    write_account_markdown(workspace, &db_account, Some(&json), db);
                                synced += 1;
                            }
                        }
                        Ok(None) => {
                            let _ = db.upsert_account(&file_account);
                            let _ =
                                write_account_markdown(workspace, &file_account, Some(&json), db);
                            synced += 1;
                        }
                        Err(_) => continue,
                    },
                    Err(e) => {
                        log::warn!("Failed to read child {}: {}", child_json_path.display(), e);
                        continue;
                    }
                }
            } else {
                // BU directory without dashboard.json — bootstrap child record
                if db.get_account(&child_id).ok().flatten().is_none() {
                    let now = Utc::now().to_rfc3339();
                    let new_child = DbAccount {
                        id: child_id,
                        name: sub_name_str.to_string(),
                        lifecycle: None,
                        arr: None,
                        health: None,
                        contract_start: None,
                        contract_end: None,
                        nps: None,
                        tracker_path: Some(format!(
                            "Accounts/{}/{}",
                            parent_name_str, sub_name_str
                        )),
                        parent_id: Some(parent_id.clone()),
                        is_internal: false,
                        updated_at: now,
                        archived: false,
                    };
                    if db.upsert_account(&new_child).is_ok() {
                        let _ = write_account_json(workspace, &new_child, None, db);
                        let _ = write_account_markdown(workspace, &new_child, None, db);
                        log::info!(
                            "Bootstrapped child account '{}/{}' from BU folder",
                            parent_name_str,
                            sub_name_str
                        );
                        synced += 1;
                    }
                }
            }
        }
    }

    // Also check: SQLite accounts that have no workspace dir yet
    if let Ok(all_accounts) = db.get_all_accounts() {
        for account in &all_accounts {
            let dir = resolve_account_dir(workspace, account);
            if !dir.exists() {
                let _ = write_account_json(workspace, account, None, db);
                let _ = write_account_markdown(workspace, account, None, db);
                synced += 1;
            }
        }
    }

    Ok(synced)
}

// =============================================================================
// Enrichment (I74 / ADR-0047)
// =============================================================================

/// Parse Claude's enrichment response into a CompanyOverview.
///
/// Expected format:
/// ```text
/// ENRICHMENT
/// DESCRIPTION: one-paragraph company description
/// INDUSTRY: industry name
/// SIZE: employee count or range
/// HQ: headquarters location
/// END_ENRICHMENT
/// ```
pub fn parse_enrichment_response(response: &str) -> Option<CompanyOverview> {
    let mut in_block = false;
    let mut description = None;
    let mut industry = None;
    let mut size = None;
    let mut headquarters = None;

    for line in response.lines() {
        let trimmed = line.trim();

        if trimmed == "ENRICHMENT" {
            in_block = true;
            continue;
        }
        if trimmed == "END_ENRICHMENT" {
            break;
        }

        if !in_block {
            continue;
        }

        if let Some(val) = trimmed.strip_prefix("DESCRIPTION:") {
            description = Some(val.trim().to_string());
        } else if let Some(val) = trimmed.strip_prefix("INDUSTRY:") {
            industry = Some(val.trim().to_string());
        } else if let Some(val) = trimmed.strip_prefix("SIZE:") {
            size = Some(val.trim().to_string());
        } else if let Some(val) = trimmed.strip_prefix("HQ:") {
            headquarters = Some(val.trim().to_string());
        }
    }

    // Only return if we got at least a description
    if description.is_some() {
        Some(CompanyOverview {
            description,
            industry,
            size,
            headquarters,
            enriched_at: Some(Utc::now().to_rfc3339()),
        })
    } else {
        None
    }
}

// =============================================================================
// Content Index (I124)
// =============================================================================

/// Sync the content index for a single account. Compares filesystem against DB,
/// adds new files, updates changed files, removes deleted files.
///
/// Delegates to the entity-generic `sync_content_index_for_entity()`.
/// Returns `(added, updated, removed)` counts.
pub fn sync_content_index_for_account(
    workspace: &Path,
    db: &ActionDb,
    account: &crate::db::DbAccount,
) -> Result<(usize, usize, usize), String> {
    let account_dir = resolve_account_dir(workspace, account);
    crate::entity_intel::sync_content_index_for_entity(
        &account_dir,
        &account.id,
        "account",
        workspace,
        db,
    )
}

/// Sync content indexes for all accounts and projects. Returns total files indexed.
pub fn sync_all_content_indexes(workspace: &Path, db: &ActionDb) -> Result<usize, String> {
    let accounts = db
        .get_all_accounts()
        .map_err(|e| format!("DB error: {}", e))?;
    let mut total = 0;

    for account in &accounts {
        match sync_content_index_for_account(workspace, db, account) {
            Ok((added, updated, _removed)) => {
                total += added + updated;
            }
            Err(e) => {
                log::warn!("Content index sync failed for '{}': {}", account.name, e);
            }
        }
    }

    // I138: Also sync project content indexes
    total += crate::projects::sync_all_project_content_indexes(workspace, db).unwrap_or(0);

    Ok(total)
}

/// Build file context string for enrichment prompts (I126, updated I139).
///
/// Uses pre-computed summaries from SQLite when available, falling back to
/// text extraction for files without summaries. Priority-ordered (highest first).
/// Capped at 10K chars total.
pub fn build_file_context(_workspace: &Path, db: &ActionDb, account_id: &str) -> String {
    let files = match db.get_entity_files(account_id) {
        Ok(f) => f,
        Err(_) => return String::new(),
    };

    if files.is_empty() {
        return String::new();
    }

    let max_chars: usize = 10_000;
    let mut context_parts: Vec<String> = Vec::new();
    let mut total_chars: usize = 0;

    for file in &files {
        if total_chars >= max_chars {
            break;
        }

        // Prefer pre-computed summary; fall back to extraction
        let text = if let Some(ref summary) = file.summary {
            summary.clone()
        } else {
            let path = std::path::Path::new(&file.absolute_path);
            if !path.exists() {
                continue;
            }
            match crate::processor::extract::extract_text(path) {
                Ok(t) => {
                    let summary = crate::entity_intel::mechanical_summary(&t, 500);
                    if summary.is_empty() {
                        continue;
                    }
                    summary
                }
                Err(_) => continue,
            }
        };

        let remaining = max_chars.saturating_sub(total_chars);
        let truncated = if text.len() > remaining {
            &text[..remaining]
        } else {
            &text
        };

        context_parts.push(format!(
            "--- {} [{}] ---\n{}",
            wrap_user_data(&file.filename),
            file.content_type,
            wrap_user_data(truncated),
        ));
        total_chars += truncated.len();
    }

    if context_parts.is_empty() {
        return String::new();
    }

    format!(
        "\n\nThe following file summaries exist in this account's workspace (by priority):\n\n{}",
        context_parts.join("\n\n")
    )
}

/// Build the Claude Code prompt for account enrichment.
pub fn enrichment_prompt(account_name: &str) -> String {
    format!(
        "Research the company \"{name}\". Use web search to find current information. \
         Return ONLY the structured block below — no other text.\n\n\
         ENRICHMENT\n\
         DESCRIPTION: <one paragraph describing what the company does, their main product/service>\n\
         INDUSTRY: <their primary industry>\n\
         SIZE: <approximate employee count or range, e.g. \"500-1000\">\n\
         HQ: <headquarters city and country>\n\
         END_ENRICHMENT",
        name = wrap_user_data(account_name)
    )
}

/// Enrich an account via Claude Code websearch.
///
/// Calls Claude Code with a research prompt, parses the structured response,
/// updates dashboard.json, SQLite, and dashboard.md.
///
/// Returns the enriched CompanyOverview on success.
pub fn enrich_account(
    workspace: &Path,
    db: &ActionDb,
    account_id: &str,
    pty: &crate::pty::PtyManager,
) -> Result<CompanyOverview, String> {
    let account = db
        .get_account(account_id)
        .map_err(|e| format!("DB error: {}", e))?
        .ok_or_else(|| format!("Account {} not found", account_id))?;

    // I126: Append file context from content index to enrichment prompt
    let file_context = build_file_context(workspace, db, account_id);
    let prompt = format!("{}{}", enrichment_prompt(&account.name), file_context);
    let output = pty
        .spawn_claude(workspace, &prompt)
        .map_err(|e| format!("Claude Code error: {}", e))?;

    let overview = parse_enrichment_response(&output.stdout)
        .ok_or("Could not parse enrichment response — no ENRICHMENT block found")?;

    // Read existing JSON to preserve other narrative fields
    let json_path = account_dir(workspace, &account.name).join("dashboard.json");
    let mut json = if json_path.exists() {
        read_account_json(&json_path)
            .map(|r| r.json)
            .unwrap_or_else(|_| default_account_json(&account))
    } else {
        default_account_json(&account)
    };

    json.company_overview = Some(overview.clone());

    // Write JSON + markdown
    write_account_json(workspace, &account, Some(&json), db)?;
    write_account_markdown(workspace, &account, Some(&json), db)?;

    log::info!(
        "Enriched account '{}' via Claude Code websearch",
        account.name
    );
    Ok(overview)
}

/// Create a minimal AccountJson from a DbAccount (no narrative fields).
fn default_account_json(account: &DbAccount) -> AccountJson {
    AccountJson {
        version: 1,
        entity_type: "account".to_string(),
        structured: AccountStructured {
            arr: account.arr,
            health: account.health.clone(),
            lifecycle: account.lifecycle.clone(),
            renewal_date: account.contract_end.clone(),
            nps: account.nps,
            account_team: Vec::new(),
            csm: None,
            champion: None,
        },
        company_overview: None,
        strategic_programs: Vec::new(),
        notes: None,
        custom_sections: Vec::new(),
        parent_id: account.parent_id.clone(),
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn test_db() -> ActionDb {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("test.db");
        std::mem::forget(dir);
        ActionDb::open_at(path).expect("open")
    }

    fn sample_account(name: &str) -> DbAccount {
        let now = Utc::now().to_rfc3339();
        DbAccount {
            id: slugify(name),
            name: name.to_string(),
            lifecycle: Some("steady-state".to_string()),
            arr: Some(50_000.0),
            health: Some("green".to_string()),
            contract_start: Some("2025-01-01".to_string()),
            contract_end: Some("2026-01-01".to_string()),
            nps: Some(80),
            tracker_path: Some(format!("Accounts/{}", name)),
            parent_id: None,
            is_internal: false,
            updated_at: now,
            archived: false,
        }
    }

    #[test]
    fn test_write_and_read_json() {
        let dir = tempfile::tempdir().expect("tempdir");
        let workspace = dir.path();
        let db = test_db();
        let account = sample_account("Acme Corp");

        write_account_json(workspace, &account, None, &db).unwrap();

        let json_path = workspace.join("Accounts/Acme Corp/dashboard.json");
        assert!(json_path.exists());

        let result = read_account_json(&json_path).unwrap();
        assert_eq!(result.account.id, "acme-corp");
        assert_eq!(result.account.name, "Acme Corp");
        assert_eq!(result.account.arr, Some(50_000.0));
        assert_eq!(result.account.health, Some("green".to_string()));
        assert_eq!(result.account.nps, Some(80));
    }

    #[test]
    fn test_write_markdown() {
        let dir = tempfile::tempdir().expect("tempdir");
        let workspace = dir.path();
        let db = test_db();
        let account = sample_account("Acme Corp");
        db.upsert_account(&account).unwrap();

        let json = AccountJson {
            version: 1,
            entity_type: "account".to_string(),
            structured: AccountStructured {
                arr: account.arr,
                health: account.health.clone(),
                lifecycle: account.lifecycle.clone(),
                renewal_date: account.contract_end.clone(),
                nps: account.nps,
                account_team: Vec::new(),
                csm: None,
                champion: None,
            },
            company_overview: Some(CompanyOverview {
                description: Some("A great company.".to_string()),
                industry: Some("Tech".to_string()),
                size: None,
                headquarters: None,
                enriched_at: None,
            }),
            strategic_programs: vec![StrategicProgram {
                name: "Migration".to_string(),
                status: "in_progress".to_string(),
                notes: Some("Phase 2".to_string()),
            }],
            notes: Some("Key account.".to_string()),
            custom_sections: vec![],
            parent_id: None,
        };

        write_account_markdown(workspace, &account, Some(&json), &db).unwrap();

        let md_path = workspace.join("Accounts/Acme Corp/dashboard.md");
        assert!(md_path.exists());

        let content = std::fs::read_to_string(md_path).unwrap();
        assert!(content.contains("# Acme Corp"));
        assert!(content.contains("🟢"));
        assert!(content.contains("$50000"));
        assert!(content.contains("A great company."));
        assert!(content.contains("Migration"));
        assert!(content.contains("Key account."));
    }

    #[test]
    fn test_sync_from_workspace() {
        let dir = tempfile::tempdir().expect("tempdir");
        let workspace = dir.path();
        let db = test_db();

        // Create an account in SQLite
        let account = sample_account("Beta Inc");
        db.upsert_account(&account).unwrap();

        // Sync should create files for the SQLite-only account
        let synced = sync_accounts_from_workspace(workspace, &db).unwrap();
        assert_eq!(synced, 1);

        let json_path = workspace.join("Accounts/Beta Inc/dashboard.json");
        assert!(json_path.exists());

        let md_path = workspace.join("Accounts/Beta Inc/dashboard.md");
        assert!(md_path.exists());
    }

    #[test]
    fn test_sync_picks_up_new_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let workspace = dir.path();
        let db = test_db();

        // Write a JSON file manually
        let acct_dir = workspace.join("Accounts/New Co");
        std::fs::create_dir_all(&acct_dir).unwrap();
        let json_content = serde_json::json!({
            "version": 1,
            "entityType": "account",
            "structured": {
                "arr": 25000.0,
                "health": "yellow",
                "lifecycle": "ramping"
            }
        });
        std::fs::write(
            acct_dir.join("dashboard.json"),
            serde_json::to_string_pretty(&json_content).unwrap(),
        )
        .unwrap();

        let synced = sync_accounts_from_workspace(workspace, &db).unwrap();
        assert_eq!(synced, 1);

        // Verify it was inserted into SQLite
        let acct = db.get_account("new-co").unwrap();
        assert!(acct.is_some());
        let acct = acct.unwrap();
        assert_eq!(acct.name, "New Co");
        assert_eq!(acct.arr, Some(25000.0));
        assert_eq!(acct.health, Some("yellow".to_string()));
    }

    #[test]
    fn test_preserves_narrative_on_structured_update() {
        let dir = tempfile::tempdir().expect("tempdir");
        let workspace = dir.path();
        let db = test_db();
        let account = sample_account("Gamma Ltd");

        let existing = AccountJson {
            version: 1,
            entity_type: "account".to_string(),
            structured: AccountStructured {
                arr: account.arr,
                health: account.health.clone(),
                lifecycle: account.lifecycle.clone(),
                renewal_date: account.contract_end.clone(),
                nps: account.nps,
                account_team: Vec::new(),
                csm: None,
                champion: None,
            },
            company_overview: Some(CompanyOverview {
                description: Some("Important context.".to_string()),
                industry: None,
                size: None,
                headquarters: None,
                enriched_at: None,
            }),
            strategic_programs: vec![],
            notes: Some("Don't lose these notes.".to_string()),
            custom_sections: vec![],
            parent_id: None,
        };

        // Write with existing narrative data
        write_account_json(workspace, &account, Some(&existing), &db).unwrap();

        // Read back and verify narrative preserved
        let json_path = workspace.join("Accounts/Gamma Ltd/dashboard.json");
        let result = read_account_json(&json_path).unwrap();
        assert_eq!(
            result.json.company_overview.unwrap().description,
            Some("Important context.".to_string())
        );
        assert_eq!(
            result.json.notes,
            Some("Don't lose these notes.".to_string())
        );
    }

    #[test]
    fn test_parse_enrichment_response() {
        let response = "\
Some preamble text from Claude

ENRICHMENT
DESCRIPTION: Acme Corp builds enterprise widgets for Fortune 500 companies.
INDUSTRY: Enterprise Software
SIZE: 500-1000
HQ: San Francisco, USA
END_ENRICHMENT

Some trailing text";

        let overview = parse_enrichment_response(response).unwrap();
        assert_eq!(
            overview.description.unwrap(),
            "Acme Corp builds enterprise widgets for Fortune 500 companies."
        );
        assert_eq!(overview.industry.unwrap(), "Enterprise Software");
        assert_eq!(overview.size.unwrap(), "500-1000");
        assert_eq!(overview.headquarters.unwrap(), "San Francisco, USA");
        assert!(overview.enriched_at.is_some());
    }

    #[test]
    fn test_parse_enrichment_response_missing_block() {
        let response = "No enrichment block here, just regular text.";
        assert!(parse_enrichment_response(response).is_none());
    }

    #[test]
    fn test_parse_enrichment_response_partial() {
        let response = "\
ENRICHMENT
DESCRIPTION: Partial data only.
END_ENRICHMENT";

        let overview = parse_enrichment_response(response).unwrap();
        assert_eq!(overview.description.unwrap(), "Partial data only.");
        assert!(overview.industry.is_none());
        assert!(overview.size.is_none());
    }

    #[test]
    fn test_sync_bootstraps_from_folder_name() {
        let dir = tempfile::tempdir().expect("tempdir");
        let workspace = dir.path();
        let db = test_db();

        // Create account directories with NO dashboard.json and NO SQLite record
        let acct1 = workspace.join("Accounts/Acme Corp");
        let acct2 = workspace.join("Accounts/Beta Industries");
        std::fs::create_dir_all(&acct1).unwrap();
        std::fs::create_dir_all(&acct2).unwrap();

        // Drop a random file in one to simulate existing user content
        std::fs::write(acct1.join("notes.md"), "# Meeting notes\nSome content").unwrap();

        let synced = sync_accounts_from_workspace(workspace, &db).unwrap();
        assert_eq!(synced, 2);

        // Verify SQLite records were created
        let acme = db.get_account("acme-corp").unwrap();
        assert!(acme.is_some());
        let acme = acme.unwrap();
        assert_eq!(acme.name, "Acme Corp");
        assert_eq!(acme.tracker_path, Some("Accounts/Acme Corp".to_string()));

        let beta = db.get_account("beta-industries").unwrap();
        assert!(beta.is_some());

        // Verify dashboard.json was created
        assert!(acct1.join("dashboard.json").exists());
        assert!(acct2.join("dashboard.json").exists());

        // Verify dashboard.md was created
        assert!(acct1.join("dashboard.md").exists());

        // Verify existing files were NOT touched
        let notes = std::fs::read_to_string(acct1.join("notes.md")).unwrap();
        assert!(notes.contains("Meeting notes"));

        // Verify entity bridge fired (entity record exists)
        let entity = db.get_entity("acme-corp").unwrap();
        assert!(entity.is_some());
    }

    #[test]
    fn test_sync_bootstrap_no_duplicates() {
        let dir = tempfile::tempdir().expect("tempdir");
        let workspace = dir.path();
        let db = test_db();

        // Create a bare folder
        std::fs::create_dir_all(workspace.join("Accounts/Delta Co")).unwrap();

        // First sync: bootstraps
        let synced1 = sync_accounts_from_workspace(workspace, &db).unwrap();
        assert_eq!(synced1, 1);

        // Second sync: may re-sync due to timestamp harmonization, but must not duplicate
        let _synced2 = sync_accounts_from_workspace(workspace, &db).unwrap();

        // Critical invariant: exactly one record, not two
        let all = db.get_all_accounts().unwrap();
        let delta_count = all.iter().filter(|a| a.name == "Delta Co").count();
        assert_eq!(
            delta_count, 1,
            "bootstrap must not create duplicates on re-sync"
        );
    }

    // ── I114: Parent-Child tests ────────────────────────────────────────────

    #[test]
    fn test_is_bu_directory() {
        assert!(is_bu_directory("Consumer-Brands"));
        assert!(is_bu_directory("Enterprise"));
        assert!(is_bu_directory("Diversification"));
        assert!(!is_bu_directory("01-Customer-Information"));
        assert!(!is_bu_directory("02-Meetings"));
        assert!(!is_bu_directory("_archive"));
        assert!(!is_bu_directory(".hidden"));
        // App-managed entity subdirs (ADR-0059) must NOT be treated as BUs
        assert!(!is_bu_directory("Call-Transcripts"));
        assert!(!is_bu_directory("Meeting-Notes"));
        assert!(!is_bu_directory("Documents"));
    }

    #[test]
    fn test_child_id_scheme() {
        // Verify the -- separator is unambiguous
        assert_eq!(
            format!("{}--{}", slugify("Cox"), slugify("Consumer Brands")),
            "cox--consumer-brands"
        );
        // slugify collapses consecutive dashes so -- can't appear from slugify alone
        assert_eq!(slugify("Some--Thing"), "some-thing");
    }

    #[test]
    fn test_read_account_json_flat() {
        let dir = tempfile::tempdir().expect("tempdir");
        let workspace = dir.path();
        let accounts_dir = workspace.join("Accounts/Acme");
        std::fs::create_dir_all(&accounts_dir).unwrap();

        let json = r#"{"structured":{"arr":100000}}"#;
        std::fs::write(accounts_dir.join("dashboard.json"), json).unwrap();

        let result = read_account_json(&accounts_dir.join("dashboard.json")).unwrap();
        assert_eq!(result.account.id, "acme");
        assert_eq!(result.account.name, "Acme");
        assert!(result.account.parent_id.is_none());
        assert_eq!(
            result.account.tracker_path,
            Some("Accounts/Acme".to_string())
        );
    }

    #[test]
    fn test_read_account_json_nested_child() {
        let dir = tempfile::tempdir().expect("tempdir");
        let workspace = dir.path();
        let child_dir = workspace.join("Accounts/Cox/Consumer-Brands");
        std::fs::create_dir_all(&child_dir).unwrap();

        let json = r#"{"structured":{"arr":500000,"health":"green"}}"#;
        std::fs::write(child_dir.join("dashboard.json"), json).unwrap();

        let result = read_account_json(&child_dir.join("dashboard.json")).unwrap();
        assert_eq!(result.account.id, "cox--consumer-brands");
        assert_eq!(result.account.name, "Consumer-Brands");
        assert_eq!(result.account.parent_id, Some("cox".to_string()));
        assert_eq!(
            result.account.tracker_path,
            Some("Accounts/Cox/Consumer-Brands".to_string())
        );
        assert_eq!(result.account.arr, Some(500000.0));
    }

    #[test]
    fn test_write_with_tracker_path_uses_correct_dir() {
        let dir = tempfile::tempdir().expect("tempdir");
        let workspace = dir.path();
        let db = test_db();

        // Create a child account with tracker_path pointing to nested dir
        let account = DbAccount {
            id: "cox--consumer-brands".to_string(),
            name: "Consumer-Brands".to_string(),
            lifecycle: None,
            arr: Some(500_000.0),
            health: Some("green".to_string()),
            contract_start: None,
            contract_end: None,
            nps: None,
            tracker_path: Some("Accounts/Cox/Consumer-Brands".to_string()),
            parent_id: Some("cox".to_string()),
            is_internal: false,
            updated_at: Utc::now().to_rfc3339(),
            archived: false,
        };

        write_account_json(workspace, &account, None, &db).unwrap();

        // Should write to nested path, not Accounts/Consumer-Brands/
        let nested = workspace.join("Accounts/Cox/Consumer-Brands/dashboard.json");
        assert!(nested.exists(), "Should write to nested tracker_path");

        let flat = workspace.join("Accounts/Consumer-Brands/dashboard.json");
        assert!(!flat.exists(), "Should NOT write to flat path");
    }

    #[test]
    fn test_sync_discovers_bu_subdirectories() {
        let dir = tempfile::tempdir().expect("tempdir");
        let workspace = dir.path();
        let db = test_db();

        // Create parent with dashboard.json
        let parent_dir = workspace.join("Accounts/TestParent");
        std::fs::create_dir_all(&parent_dir).unwrap();
        std::fs::write(
            parent_dir.join("dashboard.json"),
            r#"{"structured":{"arr":1000000}}"#,
        )
        .unwrap();

        // Create BU child dir (no dashboard.json — should be bootstrapped)
        let child_dir = parent_dir.join("TestChild");
        std::fs::create_dir_all(&child_dir).unwrap();

        // Create numbered internal dir (should NOT be bootstrapped)
        let internal_dir = parent_dir.join("01-Customer-Information");
        std::fs::create_dir_all(&internal_dir).unwrap();

        let synced = sync_accounts_from_workspace(workspace, &db).unwrap();
        assert!(synced >= 2, "Should sync at least parent + child");

        // Parent should exist
        let parent = db.get_account("testparent").unwrap();
        assert!(parent.is_some(), "Parent account should exist");

        // Child should exist with correct parent_id
        let child = db.get_account("testparent--testchild").unwrap();
        assert!(child.is_some(), "Child account should exist");
        let child = child.unwrap();
        assert_eq!(child.parent_id, Some("testparent".to_string()));
        assert_eq!(
            child.tracker_path,
            Some("Accounts/TestParent/TestChild".to_string())
        );

        // Internal dir should NOT be an account
        let internal = db
            .get_account("testparent--01-customer-information")
            .unwrap();
        assert!(internal.is_none(), "Numbered dir should not be an account");
    }

    #[test]
    fn test_get_child_and_top_level_accounts() {
        let db = test_db();

        // Insert parent
        let parent = DbAccount {
            id: "cox".to_string(),
            name: "Cox".to_string(),
            lifecycle: None,
            arr: Some(5_000_000.0),
            health: Some("green".to_string()),
            contract_start: None,
            contract_end: None,
            nps: None,
            tracker_path: Some("Accounts/Cox".to_string()),
            parent_id: None,
            is_internal: false,
            updated_at: Utc::now().to_rfc3339(),
            archived: false,
        };
        db.upsert_account(&parent).unwrap();

        // Insert children
        let child1 = DbAccount {
            id: "cox--consumer-brands".to_string(),
            name: "Consumer-Brands".to_string(),
            lifecycle: None,
            arr: Some(2_000_000.0),
            health: Some("green".to_string()),
            contract_start: None,
            contract_end: None,
            nps: None,
            tracker_path: Some("Accounts/Cox/Consumer-Brands".to_string()),
            parent_id: Some("cox".to_string()),
            is_internal: false,
            updated_at: Utc::now().to_rfc3339(),
            archived: false,
        };
        db.upsert_account(&child1).unwrap();

        let child2 = DbAccount {
            id: "cox--enterprise".to_string(),
            name: "Enterprise".to_string(),
            lifecycle: None,
            arr: Some(3_000_000.0),
            health: Some("yellow".to_string()),
            contract_start: None,
            contract_end: None,
            nps: None,
            tracker_path: Some("Accounts/Cox/Enterprise".to_string()),
            parent_id: Some("cox".to_string()),
            is_internal: false,
            updated_at: Utc::now().to_rfc3339(),
            archived: false,
        };
        db.upsert_account(&child2).unwrap();

        // top-level should only include parent
        let top = db.get_top_level_accounts().unwrap();
        assert_eq!(top.len(), 1);
        assert_eq!(top[0].id, "cox");

        // children query
        let children = db.get_child_accounts("cox").unwrap();
        assert_eq!(children.len(), 2);
        assert!(children.iter().any(|c| c.id == "cox--consumer-brands"));
        assert!(children.iter().any(|c| c.id == "cox--enterprise"));
    }

    #[test]
    fn test_parent_aggregate() {
        let db = test_db();

        // Insert parent
        db.upsert_account(&DbAccount {
            id: "parent".to_string(),
            name: "Parent".to_string(),
            lifecycle: None,
            arr: Some(10_000_000.0),
            health: Some("green".to_string()),
            contract_start: None,
            contract_end: Some("2027-12-31".to_string()),
            nps: None,
            tracker_path: None,
            parent_id: None,
            is_internal: false,
            updated_at: Utc::now().to_rfc3339(),
            archived: false,
        })
        .unwrap();

        // Insert children with different health + ARR + renewal
        db.upsert_account(&DbAccount {
            id: "parent--a".to_string(),
            name: "A".to_string(),
            lifecycle: None,
            arr: Some(3_000_000.0),
            health: Some("green".to_string()),
            contract_start: None,
            contract_end: Some("2026-09-30".to_string()),
            nps: None,
            tracker_path: None,
            parent_id: Some("parent".to_string()),
            is_internal: false,
            updated_at: Utc::now().to_rfc3339(),
            archived: false,
        })
        .unwrap();

        db.upsert_account(&DbAccount {
            id: "parent--b".to_string(),
            name: "B".to_string(),
            lifecycle: None,
            arr: Some(1_500_000.0),
            health: Some("red".to_string()),
            contract_start: None,
            contract_end: Some("2026-03-15".to_string()),
            nps: None,
            tracker_path: None,
            parent_id: Some("parent".to_string()),
            is_internal: false,
            updated_at: Utc::now().to_rfc3339(),
            archived: false,
        })
        .unwrap();

        let agg = db.get_parent_aggregate("parent").unwrap();
        assert_eq!(agg.bu_count, 2);
        assert_eq!(agg.total_arr, Some(4_500_000.0));
        assert_eq!(agg.worst_health.as_deref(), Some("red"));
        assert_eq!(agg.nearest_renewal.as_deref(), Some("2026-03-15"));
    }

    // ── I124: Content Index tests ─────────────────────────────────────────

    #[test]
    fn test_content_index_scan_empty_dir() {
        let dir = tempfile::tempdir().expect("tempdir");
        let workspace = dir.path();
        let db = test_db();
        let account = sample_account("EmptyAccount");
        db.upsert_account(&account).unwrap();

        // Create account dir with only dashboard.json
        let acct_dir = workspace.join("Accounts/EmptyAccount");
        std::fs::create_dir_all(&acct_dir).unwrap();
        std::fs::write(acct_dir.join("dashboard.json"), "{}").unwrap();

        let (added, updated, removed) =
            sync_content_index_for_account(workspace, &db, &account).unwrap();
        assert_eq!(added, 0);
        assert_eq!(updated, 0);
        assert_eq!(removed, 0);

        let files = db.get_entity_files(&account.id).unwrap();
        assert!(files.is_empty());
    }

    #[test]
    fn test_content_index_scan_with_files() {
        let dir = tempfile::tempdir().expect("tempdir");
        let workspace = dir.path();
        let db = test_db();
        let account = sample_account("FileAccount");
        db.upsert_account(&account).unwrap();

        let acct_dir = workspace.join("Accounts/FileAccount");
        std::fs::create_dir_all(&acct_dir).unwrap();
        std::fs::write(acct_dir.join("dashboard.json"), "{}").unwrap();
        std::fs::write(acct_dir.join("notes.md"), "# Notes").unwrap();
        std::fs::write(acct_dir.join("qbr-deck.pptx"), "fake pptx content").unwrap();
        std::fs::write(acct_dir.join("transcript.txt"), "meeting transcript").unwrap();

        let (added, _updated, _removed) =
            sync_content_index_for_account(workspace, &db, &account).unwrap();
        assert_eq!(added, 3);

        let files = db.get_entity_files(&account.id).unwrap();
        assert_eq!(files.len(), 3);
        let filenames: Vec<&str> = files.iter().map(|f| f.filename.as_str()).collect();
        assert!(filenames.contains(&"notes.md"));
        assert!(filenames.contains(&"qbr-deck.pptx"));
        assert!(filenames.contains(&"transcript.txt"));
    }

    #[test]
    fn test_content_index_skip_hidden_files() {
        let dir = tempfile::tempdir().expect("tempdir");
        let workspace = dir.path();
        let db = test_db();
        let account = sample_account("HiddenAccount");
        db.upsert_account(&account).unwrap();

        let acct_dir = workspace.join("Accounts/HiddenAccount");
        std::fs::create_dir_all(&acct_dir).unwrap();
        std::fs::write(acct_dir.join(".hidden"), "hidden file").unwrap();
        std::fs::write(acct_dir.join("_internal"), "internal file").unwrap();
        std::fs::write(acct_dir.join(".DS_Store"), "macOS junk").unwrap();
        std::fs::write(acct_dir.join("dashboard.md"), "generated markdown").unwrap();
        std::fs::write(acct_dir.join("real-file.md"), "# Actual content").unwrap();

        let (added, _, _) = sync_content_index_for_account(workspace, &db, &account).unwrap();
        assert_eq!(added, 1);

        let files = db.get_entity_files(&account.id).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].filename, "real-file.md");
    }

    #[test]
    fn test_content_index_rescan_detects_removal() {
        let dir = tempfile::tempdir().expect("tempdir");
        let workspace = dir.path();
        let db = test_db();
        let account = sample_account("RemovalAccount");
        db.upsert_account(&account).unwrap();

        let acct_dir = workspace.join("Accounts/RemovalAccount");
        std::fs::create_dir_all(&acct_dir).unwrap();
        let temp_file = acct_dir.join("temp.md");
        std::fs::write(&temp_file, "temporary content").unwrap();

        // First scan — file found
        let (added, _, _) = sync_content_index_for_account(workspace, &db, &account).unwrap();
        assert_eq!(added, 1);

        // Delete the file
        std::fs::remove_file(&temp_file).unwrap();

        // Rescan — file removed
        let (_, _, removed) = sync_content_index_for_account(workspace, &db, &account).unwrap();
        assert_eq!(removed, 1);

        let files = db.get_entity_files(&account.id).unwrap();
        assert!(files.is_empty());
    }

    #[test]
    fn test_content_index_rescan_skips_unchanged() {
        let dir = tempfile::tempdir().expect("tempdir");
        let workspace = dir.path();
        let db = test_db();
        let account = sample_account("UnchangedAccount");
        db.upsert_account(&account).unwrap();

        let acct_dir = workspace.join("Accounts/UnchangedAccount");
        std::fs::create_dir_all(&acct_dir).unwrap();
        std::fs::write(acct_dir.join("stable.md"), "# Stable content").unwrap();

        // First scan
        let (added1, _, _) = sync_content_index_for_account(workspace, &db, &account).unwrap();
        assert_eq!(added1, 1);

        // Second scan — file unchanged
        let (added2, updated2, removed2) =
            sync_content_index_for_account(workspace, &db, &account).unwrap();
        assert_eq!(added2, 0);
        assert_eq!(updated2, 0);
        assert_eq!(removed2, 0);
    }

    #[test]
    fn test_content_index_recursive_subdirs() {
        let dir = tempfile::tempdir().expect("tempdir");
        let workspace = dir.path();
        let db = test_db();
        let account = sample_account("RecursiveAccount");
        db.upsert_account(&account).unwrap();

        let acct_dir = workspace.join("Accounts/RecursiveAccount");
        std::fs::create_dir_all(&acct_dir).unwrap();
        std::fs::write(acct_dir.join("dashboard.json"), "{}").unwrap();
        std::fs::write(acct_dir.join("00-Index.md"), "# Index").unwrap();

        // Subdir with files (like 01-Customer-Information/)
        let sub1 = acct_dir.join("01-Customer-Information");
        std::fs::create_dir_all(&sub1).unwrap();
        std::fs::write(sub1.join("success-plan.md"), "# Plan").unwrap();
        std::fs::write(sub1.join("commercial-summary.md"), "# Summary").unwrap();

        // Deeper subdir (like 03-Call-Transcripts/)
        let sub2 = acct_dir.join("03-Call-Transcripts");
        std::fs::create_dir_all(&sub2).unwrap();
        std::fs::write(sub2.join("2025-07-call.md"), "transcript").unwrap();

        let (added, _, _) = sync_content_index_for_account(workspace, &db, &account).unwrap();
        // 00-Index.md + success-plan.md + commercial-summary.md + 2025-07-call.md = 4
        assert_eq!(added, 4);

        let files = db.get_entity_files(&account.id).unwrap();
        assert_eq!(files.len(), 4);

        // Verify relative paths include subdir structure
        let rel_paths: Vec<&str> = files.iter().map(|f| f.relative_path.as_str()).collect();
        assert!(rel_paths
            .iter()
            .any(|p| p.contains("01-Customer-Information/success-plan.md")));
        assert!(rel_paths
            .iter()
            .any(|p| p.contains("03-Call-Transcripts/2025-07-call.md")));
    }

    #[test]
    fn test_content_index_skips_child_account_dirs() {
        let dir = tempfile::tempdir().expect("tempdir");
        let workspace = dir.path();
        let db = test_db();
        let parent = sample_account("ParentCorp");
        db.upsert_account(&parent).unwrap();

        let parent_dir = workspace.join("Accounts/ParentCorp");
        std::fs::create_dir_all(&parent_dir).unwrap();
        std::fs::write(parent_dir.join("dashboard.json"), "{}").unwrap();
        std::fs::write(parent_dir.join("parent-notes.md"), "# Parent notes").unwrap();

        // Regular subdir (should be scanned)
        let internal = parent_dir.join("01-Customer-Information");
        std::fs::create_dir_all(&internal).unwrap();
        std::fs::write(internal.join("info.md"), "# Info").unwrap();

        // Child account boundary (has dashboard.json → should NOT be scanned)
        let child_dir = parent_dir.join("ChildBU");
        std::fs::create_dir_all(&child_dir).unwrap();
        std::fs::write(child_dir.join("dashboard.json"), "{}").unwrap();
        std::fs::write(child_dir.join("child-notes.md"), "# Child notes").unwrap();

        let (added, _, _) = sync_content_index_for_account(workspace, &db, &parent).unwrap();
        // parent-notes.md + info.md = 2 (child-notes.md excluded)
        assert_eq!(added, 2);

        let files = db.get_entity_files(&parent.id).unwrap();
        let filenames: Vec<&str> = files.iter().map(|f| f.filename.as_str()).collect();
        assert!(filenames.contains(&"parent-notes.md"));
        assert!(filenames.contains(&"info.md"));
        assert!(!filenames.contains(&"child-notes.md"));
    }
}
