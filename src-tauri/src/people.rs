//! People workspace file I/O (ADR-0047).
//!
//! Each person gets a directory under `People/` in the workspace:
//!   People/{Name}/person.json  — canonical data (app + external tools write here)
//!   People/{Name}/person.md    — rich artifact (generated from JSON + SQLite)
//!
//! Three-way sync (ADR-0047):
//!   App edit → writes person.json → syncs to SQLite → regenerates person.md
//!   External edit to JSON → detected by watcher or startup scan → syncs to SQLite
//!   External edit to markdown → "externally modified" indicator (no auto-reconcile)

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::db::{ActionDb, DbMeeting, DbPerson, PersonProfileFileMask, PersonSignals};
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
    // Clay enrichment fields
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

/// Compute which person profile fields a file-sync pass actually changed,
/// gating the workspace-file provenance stamp (WR-R1).
fn person_profile_file_mask(file: &DbPerson, db: Option<&DbPerson>) -> PersonProfileFileMask {
    match db {
        Some(db) => PersonProfileFileMask {
            name: file.name != db.name,
            organization: file.organization.is_some() && file.organization != db.organization,
            role: file.role.is_some() && file.role != db.role,
            relationship: db.relationship == "unknown" && file.relationship != db.relationship,
        },
        None => PersonProfileFileMask {
            name: !file.name.trim().is_empty(),
            organization: file.organization.is_some(),
            role: file.role.is_some(),
            relationship: file.relationship != "unknown",
        },
    }
}

/// Dashboard JSON for person entities (three-file pattern).
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

/// Write `dashboard.json` for a person (three-file pattern).
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
/// Uses `entity_dir` for consistent filesystem name sanitization.
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

#[derive(Debug, Clone)]
pub(crate) struct PersonArtifactSnapshot {
    person: DbPerson,
    linked_entities: Vec<crate::entity::DbEntity>,
    intelligence: Option<crate::intelligence::IntelligenceJson>,
    recent_meetings: Vec<DbMeeting>,
    meeting_account_names: HashMap<String, String>,
    signals: Option<PersonSignals>,
}

pub(crate) fn build_person_artifact_snapshot(
    person: &DbPerson,
    db: &ActionDb,
) -> PersonArtifactSnapshot {
    let linked_entities = db.get_entities_for_person(&person.id).unwrap_or_default();
    let intelligence = db.get_entity_intelligence(&person.id).ok().flatten();
    let recent_meetings = db.get_person_meetings(&person.id, 10).unwrap_or_default();
    let meeting_account_names = recent_meetings
        .iter()
        .filter_map(|meeting| {
            let account_name = db
                .get_meeting_entities(&meeting.id)
                .ok()?
                .into_iter()
                .find(|entity| entity.entity_type == crate::entity::EntityType::Account)?
                .name;
            Some((meeting.id.clone(), account_name))
        })
        .collect();
    let signals = db.get_person_signals(&person.id).ok();

    PersonArtifactSnapshot {
        person: person.clone(),
        linked_entities,
        intelligence,
        recent_meetings,
        meeting_account_names,
        signals,
    }
}

pub(crate) fn write_person_artifacts_from_snapshot(
    workspace: &Path,
    snapshot: &PersonArtifactSnapshot,
) -> Result<(), String> {
    write_person_json_from_snapshot(workspace, snapshot)?;
    write_person_markdown_from_snapshot(workspace, snapshot)
}

fn write_person_json_from_snapshot(
    workspace: &Path,
    snapshot: &PersonArtifactSnapshot,
) -> Result<(), String> {
    let person = &snapshot.person;
    let dir = person_dir(workspace, &person.name);
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create {}: {}", dir.display(), e))?;

    let linked_entities = snapshot
        .linked_entities
        .iter()
        .map(|entity| entity.id.clone())
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
    crate::util::atomic_write_str(&path, &content).map_err(|e| format!("Write error: {}", e))
}

fn write_person_markdown_from_snapshot(
    workspace: &Path,
    snapshot: &PersonArtifactSnapshot,
) -> Result<(), String> {
    let person = &snapshot.person;
    let dir = person_dir(workspace, &person.name);
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create {}: {}", dir.display(), e))?;

    let mut md = String::new();
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

    if let Some(ref notes) = person.notes {
        if !notes.is_empty() {
            md.push_str("## Notes\n\n");
            md.push_str(notes);
            md.push_str("\n\n");
        }
    }

    if let Some(ref intel) = snapshot.intelligence {
        let intel_md = crate::intelligence::format_intelligence_markdown(intel);
        if !intel_md.is_empty() {
            md.push_str(&intel_md);
        }
    }

    md.push_str("<!-- auto-generated -->\n");
    md.push_str("## Recent Meetings\n\n");
    if snapshot.recent_meetings.is_empty() {
        md.push_str("_No meetings recorded yet._\n\n");
    } else {
        for meeting in &snapshot.recent_meetings {
            let account_part = snapshot
                .meeting_account_names
                .get(&meeting.id)
                .map(|name| format!(" ({})", name))
                .unwrap_or_default();
            md.push_str(&format!(
                "- **{}** — {}{}\n",
                meeting
                    .start_time
                    .split('T')
                    .next()
                    .unwrap_or(&meeting.start_time),
                meeting.title,
                account_part,
            ));
        }
        md.push('\n');
    }

    md.push_str("## Meeting Signals\n\n");
    if let Some(ref signals) = snapshot.signals {
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
    } else {
        md.push_str("_No signal data available._\n\n");
    }

    md.push_str("## Linked Entities\n\n");
    if snapshot.linked_entities.is_empty() {
        md.push_str("_No linked accounts or projects._\n\n");
    } else {
        for entity in &snapshot.linked_entities {
            md.push_str(&format!(
                "- {} ({})\n",
                entity.name,
                entity.entity_type.as_str()
            ));
        }
        md.push('\n');
    }

    let path = dir.join("person.md");
    crate::util::atomic_write_str(&path, &md).map_err(|e| format!("Write error: {}", e))
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

    // === Intelligence sections (from DB) ===
    if let Some(intel) = db.get_entity_intelligence(&person.id).ok().flatten() {
        let intel_md = crate::intelligence::format_intelligence_markdown(&intel);
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
                    .and_then(|ents| {
                        ents.into_iter()
                            .find(|e| e.entity_type == crate::entity::EntityType::Account)
                    })
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
    // WR-R1: idempotent no-bump backfill for pre-existing person profile
    // fields that have values but no provenance. The distinct
    // `workspace_file:backfilled` label is intentionally honest about unknown
    // pre-provenance origin.
    match db.backfill_missing_person_profile_provenance() {
        Ok(n) if n > 0 => {
            log::info!("WR-R1: backfilled provenance for {n} pre-existing person profile fields")
        }
        Ok(_) => {}
        Err(e) => log::warn!("WR-R1: person profile provenance backfill failed: {e}"),
    }

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
                            let file_mask =
                                person_profile_file_mask(&file_person, Some(&db_person));
                            // Preserve meeting_count and first_seen from DB
                            file_person.meeting_count = db_person.meeting_count;
                            file_person.first_seen = db_person.first_seen.clone();
                            file_person.last_seen = db_person.last_seen.clone();
                            upsert_person_and_restore_entity_links(
                                db,
                                &file_person,
                                &linked_entities,
                            )?;
                            #[allow(
                                clippy::let_underscore_must_use,
                                reason = "intentional best-effort discard; preserves existing non-blocking behavior"
                            )]
                            let _ = db.set_person_profile_file_provenance(
                                file_person.id.as_str(),
                                file_person.updated_at.as_str(),
                                file_mask,
                            );
                            write_person_markdown(workspace, &file_person, db)?;
                            synced += 1;
                        } else if db_person.updated_at > file_person.updated_at {
                            // SQLite is newer — regenerate files from SQLite
                            #[allow(
                                clippy::let_underscore_must_use,
                                reason = "intentional best-effort discard; preserves existing non-blocking behavior"
                            )]
                            let _ = write_person_json(workspace, &db_person, db);
                            #[allow(
                                clippy::let_underscore_must_use,
                                reason = "intentional best-effort discard; preserves existing non-blocking behavior"
                            )]
                            let _ = write_person_markdown(workspace, &db_person, db);
                            synced += 1;
                        }
                        // Equal — no action needed
                    }
                    Ok(None) => {
                        // New person from file — insert to SQLite
                        file_person.first_seen = Some(Utc::now().to_rfc3339());
                        let file_mask = person_profile_file_mask(&file_person, None);
                        upsert_person_and_restore_entity_links(db, &file_person, &linked_entities)?;
                        #[allow(
                            clippy::let_underscore_must_use,
                            reason = "intentional best-effort discard; preserves existing non-blocking behavior"
                        )]
                        let _ = db.set_person_profile_file_provenance(
                            file_person.id.as_str(),
                            file_person.updated_at.as_str(),
                            file_mask,
                        );
                        write_person_markdown(workspace, &file_person, db)?;
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

pub(crate) fn upsert_person_and_restore_entity_links(
    db: &ActionDb,
    person: &DbPerson,
    linked_entities: &[String],
) -> Result<(), String> {
    let clock = crate::services::context::SystemClock;
    let rng = crate::services::context::SystemRng;
    let ext = crate::services::context::ExternalClients::default();
    let ctx = crate::services::context::ServiceContext::new_live(&clock, &rng, &ext);

    db.with_transaction(|tx| {
        tx.upsert_person(person).map_err(|e| e.to_string())?;

        // Restore entity links from JSON (ADR-0048) through the stakeholder
        // writer path so the cache invalidation signal is committed with the link.
        for entity_id in linked_entities {
            crate::services::people::link_person_to_entity_with_stakeholder_cache_rebuild(
                &ctx,
                tx,
                &person.id,
                entity_id,
                "associated",
            )?;
        }

        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_utils::test_db;

    fn person_field_source(db: &ActionDb, id: &str, field: &str) -> Option<String> {
        let raw: Option<String> = db
            .conn_ref()
            .query_row(
                "SELECT enrichment_sources FROM people WHERE id = ?1",
                rusqlite::params![id],
                |row| row.get(0),
            )
            .expect("query enrichment_sources");
        let sources: std::collections::HashMap<String, crate::db::people::FieldSource> = raw
            .and_then(|json| serde_json::from_str(&json).ok())
            .unwrap_or_default();
        sources.get(field).map(|source| source.source.clone())
    }

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
        assert!(
            dir.join("dashboard.json").exists(),
            "dashboard.json missing"
        );
        assert!(dir.join("person.md").exists(), "person.md missing");
    }

    #[test]
    fn test_sync_people_stamps_profile_file_provenance() {
        let db = test_db();
        let workspace = tempfile::tempdir().expect("workspace");
        let dir = workspace.path().join("People/Pat Example");
        std::fs::create_dir_all(&dir).expect("create person dir");
        let json = serde_json::json!({
            "version": 1,
            "entityType": "person",
            "structured": {
                "email": "pat@example.com",
                "organization": "Example Org",
                "role": "Sponsor",
                "relationship": "external"
            }
        });
        std::fs::write(
            dir.join("person.json"),
            serde_json::to_string_pretty(&json).expect("serialize"),
        )
        .expect("write person json");

        let synced = sync_people_from_workspace(workspace.path(), &db, &[]).expect("sync people");
        assert_eq!(synced, 1);
        assert_eq!(
            person_field_source(&db, "pat-example-com", "organization").as_deref(),
            Some("workspace_file:entity_doc")
        );
        assert_eq!(
            person_field_source(&db, "pat-example-com", "role").as_deref(),
            Some("workspace_file:entity_doc")
        );
    }

    #[test]
    fn test_sync_people_omitted_optional_fields_preserve_db_value_and_provenance() {
        let db = test_db();
        let workspace = tempfile::tempdir().expect("workspace");
        let person_id = person_id_from_email("riley@example.com");
        let mut person = sample_person();
        person.id = person_id.clone();
        person.email = "riley@example.com".to_string();
        person.name = "Riley Example".to_string();
        person.organization = Some("Existing Org".to_string());
        person.role = Some("Principal".to_string());
        person.relationship = "external".to_string();
        person.updated_at = "2026-01-01T00:00:00Z".to_string();
        db.upsert_person(&person).expect("insert person");
        db.set_person_field_source(&person_id, "organization", "glean")
            .expect("organization provenance");

        std::thread::sleep(std::time::Duration::from_millis(10));
        let dir = workspace.path().join("People/Riley Example");
        std::fs::create_dir_all(&dir).expect("create person dir");
        let json = serde_json::json!({
            "version": 1,
            "entityType": "person",
            "structured": {
                "email": "riley@example.com",
                "relationship": "external"
            }
        });
        std::fs::write(
            dir.join("person.json"),
            serde_json::to_string_pretty(&json).expect("serialize"),
        )
        .expect("write person json");

        let synced = sync_people_from_workspace(workspace.path(), &db, &[]).expect("sync people");
        assert_eq!(synced, 1);

        let stored = db
            .get_person_by_email_or_alias("riley@example.com")
            .expect("query person")
            .expect("person exists");
        assert_eq!(stored.organization.as_deref(), Some("Existing Org"));
        assert_eq!(stored.role.as_deref(), Some("Principal"));
        assert_eq!(
            person_field_source(&db, &person_id, "organization").as_deref(),
            Some("glean")
        );
        assert_eq!(
            person_field_source(&db, &person_id, "role").as_deref(),
            Some("workspace_file:backfilled")
        );
    }

    #[test]
    fn test_person_md_includes_intelligence() {
        let db = test_db();
        let person = sample_person();
        let _ = db.upsert_person(&person);

        let workspace = tempfile::tempdir().expect("workspace");
        let dir = person_dir(workspace.path(), &person.name);
        std::fs::create_dir_all(&dir).expect("create dir");

        // Write intelligence to DB (DB as sole source of truth)
        let intel = crate::intelligence::IntelligenceJson {
            version: 1,
            entity_id: person.id.clone(),
            entity_type: "person".to_string(),
            enriched_at: Utc::now().to_rfc3339(),
            executive_assessment: Some("Alice is a key technical leader at Acme.".to_string()),
            ..Default::default()
        };
        db.upsert_entity_intelligence(&intel).expect("upsert intel");

        // Regenerate person.md — it should pick up intelligence from DB
        write_person_markdown(workspace.path(), &person, &db).expect("regen md");

        let md = std::fs::read_to_string(dir.join("person.md")).expect("read md");
        assert!(
            md.contains("Alice is a key technical leader at Acme"),
            "person.md should include executive assessment from DB.\nGot:\n{}",
            md,
        );
    }
}
