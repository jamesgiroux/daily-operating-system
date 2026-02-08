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
use crate::util::slugify;

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
    pub ring: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub renewal_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nps: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub csm: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub champion: Option<String>,
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
    workspace.join("Accounts").join(crate::util::sanitize_for_filesystem(name))
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
    let dir = account_dir(workspace, &account.name);
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create {}: {}", dir.display(), e))?;

    let json = AccountJson {
        version: 1,
        entity_type: "account".to_string(),
        structured: AccountStructured {
            arr: account.arr,
            health: account.health.clone(),
            ring: account.ring,
            renewal_date: account.contract_end.clone(),
            nps: account.nps,
            csm: account.csm.clone(),
            champion: account.champion.clone(),
        },
        company_overview: existing_json.and_then(|j| j.company_overview.clone()),
        strategic_programs: existing_json
            .map(|j| j.strategic_programs.clone())
            .unwrap_or_default(),
        notes: existing_json.and_then(|j| j.notes.clone()),
        custom_sections: existing_json
            .map(|j| j.custom_sections.clone())
            .unwrap_or_default(),
    };

    let path = dir.join("dashboard.json");
    let content = serde_json::to_string_pretty(&json)
        .map_err(|e| format!("Serialize error: {}", e))?;
    crate::util::atomic_write_str(&path, &content)
        .map_err(|e| format!("Write error: {}", e))?;

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
    let dir = account_dir(workspace, &account.name);
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
    if let Some(ring) = account.ring {
        md.push_str(&format!("**Tier:** Ring {}  \n", ring));
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
    if let Some(ref csm) = account.csm {
        md.push_str(&format!("**CSM:** {}  \n", csm));
    }
    if let Some(ref champion) = account.champion {
        md.push_str(&format!("**Champion:** {}  \n", champion));
    }
    md.push('\n');

    // Company Overview (from JSON)
    if let Some(ref overview) = json.and_then(|j| j.company_overview.as_ref()) {
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

    // Strategic Programs (from JSON)
    if let Some(ref programs) = json.map(|j| &j.strategic_programs) {
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
    if let Some(ref notes) = json.and_then(|j| j.notes.as_ref()) {
        if !notes.is_empty() {
            md.push_str("## Notes\n\n");
            md.push_str(notes);
            md.push_str("\n\n");
        }
    }

    // === Auto-generated sections below ===

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
                md.push_str(&format!(
                    "- [{}] **{}**{}\n",
                    a.priority, a.title, due,
                ));
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
                    icon,
                    c.capture_type,
                    c.content,
                    c.meeting_title,
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
pub fn read_account_json(path: &Path) -> Result<ReadAccountResult, String> {
    let content =
        std::fs::read_to_string(path).map_err(|e| format!("Read error: {}", e))?;
    let json: AccountJson =
        serde_json::from_str(&content).map_err(|e| format!("Parse error: {}", e))?;

    let name = path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("Unknown")
        .to_string();

    let id = slugify(&name);

    // Get file mtime as updated_at
    let updated_at = std::fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
        .map(|t| {
            let dt: chrono::DateTime<Utc> = t.into();
            dt.to_rfc3339()
        })
        .unwrap_or_else(|| Utc::now().to_rfc3339());

    let tracker_path = path
        .parent()
        .and_then(|p| {
            // Build relative path like "Accounts/Acme Corp"
            let accounts_parent = p.parent()?;
            let dir_name = accounts_parent.file_name()?.to_str()?;
            let account_name = p.file_name()?.to_str()?;
            Some(format!("{}/{}", dir_name, account_name))
        });

    Ok(ReadAccountResult {
        account: DbAccount {
            id,
            name,
            ring: json.structured.ring,
            arr: json.structured.arr,
            health: json.structured.health.clone(),
            contract_start: None, // Not in JSON schema — DB only
            contract_end: json.structured.renewal_date.clone(),
            csm: json.structured.csm.clone(),
            champion: json.structured.champion.clone(),
            nps: json.structured.nps,
            tracker_path,
            updated_at,
        },
        json,
    })
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
pub fn sync_accounts_from_workspace(
    workspace: &Path,
    db: &ActionDb,
) -> Result<usize, String> {
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

        let json_path = entry.path().join("dashboard.json");
        if !json_path.exists() {
            // Account dir exists but no JSON file — generate from SQLite if we have data
            let dir_name = entry.file_name();
            let name = dir_name.to_string_lossy();
            if let Ok(Some(db_account)) = db.get_account_by_name(&name) {
                let _ = write_account_json(workspace, &db_account, None, db);
                let _ = write_account_markdown(workspace, &db_account, None, db);
                synced += 1;
            }
            continue;
        }

        match read_account_json(&json_path) {
            Ok(ReadAccountResult { account: file_account, json }) => {
                match db.get_account(&file_account.id) {
                    Ok(Some(db_account)) => {
                        if file_account.updated_at > db_account.updated_at {
                            // File is newer — update SQLite, regen markdown
                            let mut merged = file_account;
                            // Preserve DB-only fields
                            merged.contract_start = db_account.contract_start.clone();
                            let _ = db.upsert_account(&merged);
                            let _ = write_account_markdown(
                                workspace,
                                &merged,
                                Some(&json),
                                db,
                            );
                            synced += 1;
                        } else if db_account.updated_at > file_account.updated_at {
                            // SQLite is newer — regen both files
                            let _ = write_account_json(
                                workspace,
                                &db_account,
                                Some(&json),
                                db,
                            );
                            let _ = write_account_markdown(
                                workspace,
                                &db_account,
                                Some(&json),
                                db,
                            );
                            synced += 1;
                        }
                    }
                    Ok(None) => {
                        // New account from file — insert to SQLite
                        let _ = db.upsert_account(&file_account);
                        let _ = write_account_markdown(
                            workspace,
                            &file_account,
                            Some(&json),
                            db,
                        );
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

    // Also check: SQLite accounts that have no workspace dir yet
    if let Ok(all_accounts) = db.get_all_accounts() {
        for account in &all_accounts {
            let dir = account_dir(workspace, &account.name);
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
        name = account_name
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

    let prompt = enrichment_prompt(&account.name);
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

    log::info!("Enriched account '{}' via Claude Code websearch", account.name);
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
            ring: account.ring,
            renewal_date: account.contract_end.clone(),
            nps: account.nps,
            csm: account.csm.clone(),
            champion: account.champion.clone(),
        },
        company_overview: None,
        strategic_programs: Vec::new(),
        notes: None,
        custom_sections: Vec::new(),
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
            ring: Some(1),
            arr: Some(50_000.0),
            health: Some("green".to_string()),
            contract_start: Some("2025-01-01".to_string()),
            contract_end: Some("2026-01-01".to_string()),
            csm: Some("Jane".to_string()),
            champion: Some("Bob".to_string()),
            nps: Some(80),
            tracker_path: Some(format!("Accounts/{}", name)),
            updated_at: now,
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
                ring: account.ring,
                renewal_date: account.contract_end.clone(),
                nps: account.nps,
                csm: account.csm.clone(),
                champion: account.champion.clone(),
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
                "ring": 2
            }
        });
        std::fs::write(
            acct_dir.join("dashboard.json"),
            serde_json::to_string_pretty(&json_content).unwrap(),
        ).unwrap();

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
                ring: account.ring,
                renewal_date: account.contract_end.clone(),
                nps: account.nps,
                csm: account.csm.clone(),
                champion: account.champion.clone(),
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
        assert_eq!(result.json.notes, Some("Don't lose these notes.".to_string()));
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
}
