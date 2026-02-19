//! People workspace file I/O (I51 / ADR-0047).
//!
//! Each person gets a directory under `People/` in the workspace:
//!   People/{Name}/person.json  — canonical data (app + external tools write here)
//!   People/{Name}/person.md    — rich artifact (generated from JSON + SQLite)
//!
//! Three-way sync (ADR-0047):
//!   App edit → writes person.json → syncs to SQLite → regenerates person.md
//!   External edit to JSON → detected by watcher or startup scan → syncs to SQLite
//!   External edit to markdown → "externally modified" indicator (no auto-reconcile)

use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::db::{ActionDb, DbPerson};
use crate::util::{classify_relationship_multi, person_id_from_email};

/// JSON schema for person.json files.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonJson {
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default = "default_entity_type")]
    pub entity_type: String,
    pub structured: PersonStructured,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    /// Entity IDs this person is linked to (ADR-0048: durable in filesystem).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub linked_entities: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub custom_sections: Vec<serde_json::Value>,
}

fn default_version() -> u32 {
    1
}
fn default_entity_type() -> String {
    "person".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonStructured {
    pub email: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(default = "default_relationship")]
    pub relationship: String,
    // Clay enrichment fields (I228)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub linkedin_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub twitter_handle: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub photo_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bio: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub company_industry: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub company_size: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub company_hq: Option<String>,
}

fn default_relationship() -> String {
    "unknown".to_string()
}

/// Dashboard JSON for person entities (I338 — three-file pattern).
///
/// Mechanical facts + cadence, analogous to `AccountJson` for accounts.
/// Written to `People/{Name}/dashboard.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonDashboardJson {
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default = "default_entity_type")]
    pub entity_type: String,
    pub name: String,
    pub email: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    pub relationship: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cadence: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_meeting: Option<String>,
    pub meeting_count: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signals: Option<PersonDashboardSignals>,
}

/// Signal snapshot embedded in `PersonDashboardJson`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonDashboardSignals {
    pub meeting_frequency_30d: i32,
    pub meeting_frequency_90d: i32,
    pub temperature: String,
    pub trend: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_meeting: Option<String>,
}

/// Infer meeting cadence from frequency signals.
///
/// - freq_30d >= 4 → "weekly"
/// - freq_30d 2–3 → "bi-weekly"
/// - freq_30d >= 1 OR freq_90d >= 3 → "monthly"
/// - else → "ad-hoc"
pub fn infer_cadence(freq_30d: i32, freq_90d: i32) -> &'static str {
    if freq_30d >= 4 {
        "weekly"
    } else if freq_30d >= 2 {
        "bi-weekly"
    } else if freq_30d >= 1 || freq_90d >= 3 {
        "monthly"
    } else {
        "ad-hoc"
    }
}

/// Write `dashboard.json` for a person (I338 — three-file pattern).
///
/// Queries signals from SQLite, infers cadence, and writes via `entity_io::write_entity_json`.
pub fn write_person_dashboard_json(
    workspace: &Path,
    person: &DbPerson,
    db: &ActionDb,
) -> Result<(), String> {
    let dir = person_dir(workspace, &person.name);

    let signals = db
        .get_person_signals(&person.id)
        .map_err(|e| format!("Failed to get signals for {}: {}", person.id, e))?;

    let cadence = infer_cadence(signals.meeting_frequency_30d, signals.meeting_frequency_90d);

    let dashboard = PersonDashboardJson {
        version: 1,
        entity_type: "person".to_string(),
        name: person.name.clone(),
        email: person.email.clone(),
        organization: person.organization.clone(),
        role: person.role.clone(),
        relationship: person.relationship.clone(),
        cadence: Some(cadence.to_string()),
        first_meeting: person.first_seen.clone(),
        meeting_count: person.meeting_count,
        signals: Some(PersonDashboardSignals {
            meeting_frequency_30d: signals.meeting_frequency_30d,
            meeting_frequency_90d: signals.meeting_frequency_90d,
            temperature: signals.temperature,
            trend: signals.trend,
            last_meeting: signals.last_meeting,
        }),
    };

    crate::entity_io::write_entity_json(&dir, "dashboard.json", &dashboard)
}

/// Resolve the directory for a person's workspace files.
///
/// I337: Uses `entity_dir()` for consistent filesystem name sanitization.
pub fn person_dir(workspace: &Path, name: &str) -> PathBuf {
    crate::entity_io::entity_dir(workspace, "People", name)
}

/// Write `person.json` for a person.
///
/// Queries entity links from SQLite so they persist in the filesystem (ADR-0048).
pub fn write_person_json(workspace: &Path, person: &DbPerson, db: &ActionDb) -> Result<(), String> {
    let dir = person_dir(workspace, &person.name);
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create {}: {}", dir.display(), e))?;

    // Query linked entity IDs so they survive a DB rebuild
    let linked_entities = db
        .get_entities_for_person(&person.id)
        .unwrap_or_default()
        .into_iter()
        .map(|e| e.id)
        .collect();

    let json = PersonJson {
        version: 1,
        entity_type: "person".to_string(),
        structured: PersonStructured {
            email: person.email.clone(),
            organization: person.organization.clone(),
            role: person.role.clone(),
            relationship: person.relationship.clone(),
            linkedin_url: person.linkedin_url.clone(),
            twitter_handle: person.twitter_handle.clone(),
            phone: person.phone.clone(),
            photo_url: person.photo_url.clone(),
            bio: person.bio.clone(),
            company_industry: person.company_industry.clone(),
            company_size: person.company_size.clone(),
            company_hq: person.company_hq.clone(),
        },
        notes: person.notes.clone(),
        linked_entities,
        custom_sections: Vec::new(),
    };

    let path = dir.join("person.json");
    let content =
        serde_json::to_string_pretty(&json).map_err(|e| format!("Serialize error: {}", e))?;
    crate::util::atomic_write_str(&path, &content).map_err(|e| format!("Write error: {}", e))?;

    Ok(())
}

/// Write `person.md` for a person (generated artifact).
pub fn write_person_markdown(
    workspace: &Path,
    person: &DbPerson,
    db: &ActionDb,
) -> Result<(), String> {
    let dir = person_dir(workspace, &person.name);
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create {}: {}", dir.display(), e))?;

    let mut md = String::new();

    // Header
    md.push_str(&format!("# {}\n\n", person.name));
    if let Some(ref org) = person.organization {
        md.push_str(&format!("**Organization:** {}  \n", org));
    }
    if let Some(ref role) = person.role {
        md.push_str(&format!("**Role:** {}  \n", role));
    }
    md.push_str(&format!("**Relationship:** {}  \n", person.relationship));
    md.push_str(&format!("**Email:** {}  \n", person.email));
    md.push('\n');

    // Notes
    if let Some(ref notes) = person.notes {
        if !notes.is_empty() {
            md.push_str("## Notes\n\n");
            md.push_str(notes);
            md.push_str("\n\n");
        }
    }

    // === Intelligence sections (I136 — from intelligence.json) ===
    if let Ok(intel) = crate::entity_intel::read_intelligence_json(&dir) {
        let intel_md = crate::entity_intel::format_intelligence_markdown(&intel);
        if !intel_md.is_empty() {
            md.push_str(&intel_md);
        }
    }

    // Recent Meetings (auto-generated)
    md.push_str("<!-- auto-generated -->\n");
    md.push_str("## Recent Meetings\n\n");
    match db.get_person_meetings(&person.id, 10) {
        Ok(meetings) if !meetings.is_empty() => {
            for m in &meetings {
                let account_part = db
                    .get_meeting_entities(&m.id)
                    .ok()
                    .and_then(|ents| ents.into_iter().find(|e| e.entity_type == crate::entity::EntityType::Account))
                    .map(|e| format!(" ({})", e.name))
                    .unwrap_or_default();
                md.push_str(&format!(
                    "- **{}** — {}{}\n",
                    m.start_time.split('T').next().unwrap_or(&m.start_time),
                    m.title,
                    account_part,
                ));
            }
            md.push('\n');
        }
        _ => {
            md.push_str("_No meetings recorded yet._\n\n");
        }
    }

    // Meeting Signals (auto-generated)
    md.push_str("## Meeting Signals\n\n");
    match db.get_person_signals(&person.id) {
        Ok(signals) => {
            md.push_str(&format!(
                "- **30-day frequency:** {} meetings\n",
                signals.meeting_frequency_30d
            ));
            md.push_str(&format!(
                "- **90-day frequency:** {} meetings\n",
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

    // Linked Entities (auto-generated)
    md.push_str("## Linked Entities\n\n");
    match db.get_entities_for_person(&person.id) {
        Ok(entities) if !entities.is_empty() => {
            for e in &entities {
                md.push_str(&format!("- {} ({})\n", e.name, e.entity_type.as_str()));
            }
            md.push('\n');
        }
        _ => {
            md.push_str("_No linked accounts or projects._\n\n");
        }
    }

    let path = dir.join("person.md");
    crate::util::atomic_write_str(&path, &md).map_err(|e| format!("Write error: {}", e))?;

    Ok(())
}

/// Result of reading a person.json file — includes entity links for ADR-0048 restoration.
pub struct ReadPersonResult {
    pub person: DbPerson,
    /// Entity IDs from the JSON file (ADR-0048: durable in filesystem).
    pub linked_entities: Vec<String>,
}

/// Read a person.json file and convert to DbPerson + linked entity IDs.
pub fn read_person_json(path: &Path) -> Result<ReadPersonResult, String> {
    let content = std::fs::read_to_string(path).map_err(|e| format!("Read error: {}", e))?;
    let json: PersonJson =
        serde_json::from_str(&content).map_err(|e| format!("Parse error: {}", e))?;

    let id = person_id_from_email(&json.structured.email);
    let name = path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("Unknown")
        .to_string();

    // Get file mtime as updated_at
    let updated_at = std::fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
        .map(|t| {
            let dt: chrono::DateTime<Utc> = t.into();
            dt.to_rfc3339()
        })
        .unwrap_or_else(|| Utc::now().to_rfc3339());

    Ok(ReadPersonResult {
        person: DbPerson {
            id,
            email: json.structured.email.to_lowercase(),
            name,
            organization: json.structured.organization,
            role: json.structured.role,
            relationship: json.structured.relationship,
            notes: json.notes,
            tracker_path: Some(path.to_string_lossy().to_string()),
            last_seen: None,
            first_seen: None,
            meeting_count: 0,
            updated_at,
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
        },
        linked_entities: json.linked_entities,
    })
}

/// Startup scan: sync all People/*/person.json files to SQLite.
///
/// For each file: compare file mtime against `people.updated_at` in SQLite.
/// If file is newer: parse JSON, update SQLite, regenerate person.md.
/// If SQLite is newer: regenerate person.json + person.md from SQLite.
///
/// Returns the number of people synced.
pub fn sync_people_from_workspace(
    workspace: &Path,
    db: &ActionDb,
    user_domains: &[String],
) -> Result<usize, String> {
    let people_dir = workspace.join("People");
    if !people_dir.exists() {
        return Ok(0);
    }

    let mut synced = 0;

    let entries =
        std::fs::read_dir(&people_dir).map_err(|e| format!("Failed to read People/: {}", e))?;

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let json_path = entry.path().join("person.json");
        if !json_path.exists() {
            continue;
        }

        match read_person_json(&json_path) {
            Ok(ReadPersonResult {
                person: mut file_person,
                linked_entities,
            }) => {
                // Classify relationship if unknown and user_domains are set
                if file_person.relationship == "unknown" {
                    file_person.relationship =
                        classify_relationship_multi(&file_person.email, user_domains);
                }

                // Check if SQLite has this person and compare timestamps
                match db.get_person_by_email_or_alias(&file_person.email) {
                    Ok(Some(db_person)) => {
                        // Compare: file mtime vs SQLite updated_at
                        if file_person.updated_at > db_person.updated_at {
                            // File is newer — update SQLite
                            // Preserve meeting_count and first_seen from DB
                            file_person.meeting_count = db_person.meeting_count;
                            file_person.first_seen = db_person.first_seen.clone();
                            file_person.last_seen = db_person.last_seen.clone();
                            let _ = db.upsert_person(&file_person);
                            // Restore entity links from JSON (ADR-0048)
                            for entity_id in &linked_entities {
                                let _ = db.link_person_to_entity(
                                    &file_person.id,
                                    entity_id,
                                    "associated",
                                );
                            }
                            let _ = write_person_markdown(workspace, &file_person, db);
                            synced += 1;
                        } else if db_person.updated_at > file_person.updated_at {
                            // SQLite is newer — regenerate files from SQLite
                            let _ = write_person_json(workspace, &db_person, db);
                            let _ = write_person_markdown(workspace, &db_person, db);
                            synced += 1;
                        }
                        // Equal — no action needed
                    }
                    Ok(None) => {
                        // New person from file — insert to SQLite
                        file_person.first_seen = Some(Utc::now().to_rfc3339());
                        let _ = db.upsert_person(&file_person);
                        // Restore entity links from JSON (ADR-0048)
                        for entity_id in &linked_entities {
                            let _ =
                                db.link_person_to_entity(&file_person.id, entity_id, "associated");
                        }
                        let _ = write_person_markdown(workspace, &file_person, db);
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

    Ok(synced)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infer_cadence_weekly() {
        assert_eq!(infer_cadence(4, 10), "weekly");
        assert_eq!(infer_cadence(8, 20), "weekly");
    }

    #[test]
    fn test_infer_cadence_biweekly() {
        assert_eq!(infer_cadence(2, 6), "bi-weekly");
        assert_eq!(infer_cadence(3, 8), "bi-weekly");
    }

    #[test]
    fn test_infer_cadence_monthly() {
        assert_eq!(infer_cadence(1, 2), "monthly");
        assert_eq!(infer_cadence(0, 3), "monthly");
        assert_eq!(infer_cadence(0, 5), "monthly");
    }

    #[test]
    fn test_infer_cadence_ad_hoc() {
        assert_eq!(infer_cadence(0, 0), "ad-hoc");
        assert_eq!(infer_cadence(0, 2), "ad-hoc");
        assert_eq!(infer_cadence(0, 1), "ad-hoc");
    }

    fn test_db() -> ActionDb {
        let dir = tempfile::tempdir().expect("Failed to create temp dir");
        let path = dir.path().join("test.db");
        std::mem::forget(dir);
        let db = ActionDb::open_at(path).expect("open test DB");
        db.conn_ref()
            .execute_batch("PRAGMA foreign_keys = OFF;")
            .expect("disable FK");
        db
    }

    fn sample_person() -> DbPerson {
        DbPerson {
            id: "person_alice_example_com".to_string(),
            email: "alice@example.com".to_string(),
            name: "Alice Example".to_string(),
            organization: Some("Acme Corp".to_string()),
            role: Some("VP Engineering".to_string()),
            relationship: "external".to_string(),
            notes: None,
            tracker_path: None,
            last_seen: Some("2026-02-15T10:00:00Z".to_string()),
            first_seen: Some("2025-06-01T00:00:00Z".to_string()),
            meeting_count: 12,
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
        }
    }

    #[test]
    fn test_write_person_dashboard_json() {
        let db = test_db();
        let person = sample_person();
        let _ = db.upsert_person(&person);

        let workspace = tempfile::tempdir().expect("workspace");
        write_person_dashboard_json(workspace.path(), &person, &db).expect("write dashboard");

        let dir = person_dir(workspace.path(), &person.name);
        let path = dir.join("dashboard.json");
        assert!(path.exists(), "dashboard.json should exist");

        // Round-trip parse
        let content = std::fs::read_to_string(&path).expect("read");
        let parsed: PersonDashboardJson = serde_json::from_str(&content).expect("parse");
        assert_eq!(parsed.name, "Alice Example");
        assert_eq!(parsed.email, "alice@example.com");
        assert_eq!(parsed.entity_type, "person");
        assert_eq!(parsed.relationship, "external");
        assert!(parsed.cadence.is_some());
        assert!(parsed.signals.is_some());
    }

    #[test]
    fn test_three_file_pattern_complete() {
        let db = test_db();
        let person = sample_person();
        let _ = db.upsert_person(&person);

        let workspace = tempfile::tempdir().expect("workspace");
        write_person_json(workspace.path(), &person, &db).expect("person.json");
        write_person_dashboard_json(workspace.path(), &person, &db).expect("dashboard.json");
        write_person_markdown(workspace.path(), &person, &db).expect("person.md");

        let dir = person_dir(workspace.path(), &person.name);
        assert!(dir.join("person.json").exists(), "person.json missing");
        assert!(dir.join("dashboard.json").exists(), "dashboard.json missing");
        assert!(dir.join("person.md").exists(), "person.md missing");
    }

    #[test]
    fn test_person_md_includes_intelligence() {
        let db = test_db();
        let person = sample_person();
        let _ = db.upsert_person(&person);

        let workspace = tempfile::tempdir().expect("workspace");
        let dir = person_dir(workspace.path(), &person.name);
        std::fs::create_dir_all(&dir).expect("create dir");

        // Write a minimal intelligence.json via serde_json (camelCase keys)
        let intel_json = serde_json::json!({
            "version": 1,
            "entityId": person.id,
            "entityType": "person",
            "enrichedAt": Utc::now().to_rfc3339(),
            "executiveAssessment": "Alice is a key technical leader at Acme.",
        });
        let intel_str = serde_json::to_string_pretty(&intel_json).expect("serialize");
        crate::util::atomic_write_str(&dir.join("intelligence.json"), &intel_str)
            .expect("write intel");

        // Regenerate person.md — it should pick up intelligence
        write_person_markdown(workspace.path(), &person, &db).expect("regen md");

        let md = std::fs::read_to_string(dir.join("person.md")).expect("read md");
        assert!(
            md.contains("Alice is a key technical leader at Acme"),
            "person.md should include executive assessment from intelligence.json.\nGot:\n{}",
            md,
        );
    }
}
