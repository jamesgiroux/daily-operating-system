//! I429: User data export — ZIP file with human-readable JSON per domain.

use std::io::Write;
use std::path::Path;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

use crate::db::ActionDb;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportReport {
    pub path: String,
    pub timestamp: String,
    pub counts: ExportCounts,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportCounts {
    pub accounts: usize,
    pub people: usize,
    pub projects: usize,
    pub meetings: usize,
    pub actions: usize,
    pub signals: usize,
    pub intelligence: usize,
}

/// Export all user data as a ZIP file containing human-readable JSON per domain.
pub fn export_data_zip(db: &ActionDb, dest_path: &Path) -> Result<ExportReport, String> {
    let conn = db.conn_ref();
    let file =
        std::fs::File::create(dest_path).map_err(|e| format!("Failed to create file: {e}"))?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let timestamp = Utc::now().to_rfc3339();
    let mut counts = ExportCounts {
        accounts: 0,
        people: 0,
        projects: 0,
        meetings: 0,
        actions: 0,
        signals: 0,
        intelligence: 0,
    };

    // Export accounts
    {
        let mut stmt = conn
            .prepare(
                "SELECT a.id, a.name, a.lifecycle, a.account_type, a.updated_at,
                        eq.health_score
                 FROM accounts a
                 LEFT JOIN entity_quality eq ON eq.entity_id = a.id
                 WHERE a.archived = 0",
            )
            .map_err(|e| e.to_string())?;
        let rows: Vec<serde_json::Value> = stmt
            .query_map([], |row| {
                Ok(serde_json::json!({
                    "id": row.get::<_, String>(0)?,
                    "name": row.get::<_, String>(1)?,
                    "lifecycle": row.get::<_, Option<String>>(2)?,
                    "accountType": row.get::<_, Option<String>>(3)?,
                    "updatedAt": row.get::<_, Option<String>>(4)?,
                    "healthScore": row.get::<_, Option<f64>>(5)?,
                }))
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        counts.accounts = rows.len();
        zip.start_file("accounts.json", options)
            .map_err(|e| e.to_string())?;
        zip.write_all(serde_json::to_string_pretty(&rows).unwrap().as_bytes())
            .map_err(|e| e.to_string())?;
    }

    // Export people
    {
        let mut stmt = conn
            .prepare(
                "SELECT id, name, email, role, organization, phone, linkedin_url, notes, updated_at
                 FROM people WHERE archived = 0",
            )
            .map_err(|e| e.to_string())?;
        let rows: Vec<serde_json::Value> = stmt
            .query_map([], |row| {
                Ok(serde_json::json!({
                    "id": row.get::<_, String>(0)?,
                    "name": row.get::<_, String>(1)?,
                    "email": row.get::<_, Option<String>>(2)?,
                    "role": row.get::<_, Option<String>>(3)?,
                    "organization": row.get::<_, Option<String>>(4)?,
                    "phone": row.get::<_, Option<String>>(5)?,
                    "linkedinUrl": row.get::<_, Option<String>>(6)?,
                    "notes": row.get::<_, Option<String>>(7)?,
                    "updatedAt": row.get::<_, Option<String>>(8)?,
                }))
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        counts.people = rows.len();
        zip.start_file("people.json", options)
            .map_err(|e| e.to_string())?;
        zip.write_all(serde_json::to_string_pretty(&rows).unwrap().as_bytes())
            .map_err(|e| e.to_string())?;
    }

    // Export projects
    {
        let mut stmt = conn
            .prepare(
                "SELECT id, name, status, owner, target_date, updated_at
                 FROM projects WHERE archived = 0",
            )
            .map_err(|e| e.to_string())?;
        let rows: Vec<serde_json::Value> = stmt
            .query_map([], |row| {
                Ok(serde_json::json!({
                    "id": row.get::<_, String>(0)?,
                    "name": row.get::<_, String>(1)?,
                    "status": row.get::<_, Option<String>>(2)?,
                    "owner": row.get::<_, Option<String>>(3)?,
                    "targetDate": row.get::<_, Option<String>>(4)?,
                    "updatedAt": row.get::<_, Option<String>>(5)?,
                }))
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        counts.projects = rows.len();
        zip.start_file("projects.json", options)
            .map_err(|e| e.to_string())?;
        zip.write_all(serde_json::to_string_pretty(&rows).unwrap().as_bytes())
            .map_err(|e| e.to_string())?;
    }

    // Export meetings (last 90 days + future)
    {
        let mut stmt = conn
            .prepare(
                "SELECT m.id, m.title, m.meeting_type, m.start_time, m.end_time, m.description
                 FROM meetings m
                 WHERE m.start_time > datetime('now', '-90 days')
                 ORDER BY m.start_time DESC",
            )
            .map_err(|e| e.to_string())?;
        let rows: Vec<serde_json::Value> = stmt
            .query_map([], |row| {
                Ok(serde_json::json!({
                    "id": row.get::<_, String>(0)?,
                    "title": row.get::<_, String>(1)?,
                    "meetingType": row.get::<_, Option<String>>(2)?,
                    "startTime": row.get::<_, Option<String>>(3)?,
                    "endTime": row.get::<_, Option<String>>(4)?,
                    "description": row.get::<_, Option<String>>(5)?,
                }))
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        counts.meetings = rows.len();
        zip.start_file("meetings.json", options)
            .map_err(|e| e.to_string())?;
        zip.write_all(serde_json::to_string_pretty(&rows).unwrap().as_bytes())
            .map_err(|e| e.to_string())?;
    }

    // Export actions
    {
        let mut stmt = conn
            .prepare(
                "SELECT id, title, status, priority, source_type, due_date, completed_at, created_at
                 FROM actions ORDER BY created_at DESC",
            )
            .map_err(|e| e.to_string())?;
        let rows: Vec<serde_json::Value> = stmt
            .query_map([], |row| {
                Ok(serde_json::json!({
                    "id": row.get::<_, String>(0)?,
                    "title": row.get::<_, String>(1)?,
                    "status": row.get::<_, String>(2)?,
                    "priority": row.get::<_, Option<String>>(3)?,
                    "sourceType": row.get::<_, Option<String>>(4)?,
                    "dueDate": row.get::<_, Option<String>>(5)?,
                    "completedAt": row.get::<_, Option<String>>(6)?,
                    "createdAt": row.get::<_, Option<String>>(7)?,
                }))
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        counts.actions = rows.len();
        zip.start_file("actions.json", options)
            .map_err(|e| e.to_string())?;
        zip.write_all(serde_json::to_string_pretty(&rows).unwrap().as_bytes())
            .map_err(|e| e.to_string())?;
    }

    // Export signals (90-day cap)
    {
        let mut stmt = conn
            .prepare(
                "SELECT id, entity_id, entity_type, signal_type, source, confidence, value, created_at
                 FROM signal_events
                 WHERE created_at > datetime('now', '-90 days')
                 ORDER BY created_at DESC",
            )
            .map_err(|e| e.to_string())?;
        let rows: Vec<serde_json::Value> = stmt
            .query_map([], |row| {
                Ok(serde_json::json!({
                    "id": row.get::<_, String>(0)?,
                    "entityId": row.get::<_, Option<String>>(1)?,
                    "entityType": row.get::<_, Option<String>>(2)?,
                    "signalType": row.get::<_, Option<String>>(3)?,
                    "source": row.get::<_, Option<String>>(4)?,
                    "confidence": row.get::<_, Option<f64>>(5)?,
                    "value": row.get::<_, Option<String>>(6)?,
                    "createdAt": row.get::<_, Option<String>>(7)?,
                }))
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        counts.signals = rows.len();
        zip.start_file("signals.json", options)
            .map_err(|e| e.to_string())?;
        zip.write_all(serde_json::to_string_pretty(&rows).unwrap().as_bytes())
            .map_err(|e| e.to_string())?;
    }

    // Export intelligence (entity_assessment)
    {
        let mut stmt = conn
            .prepare(
                "SELECT entity_id, entity_type, enriched_at, executive_assessment,
                        risks_json, recent_wins_json, current_state_json
                 FROM entity_assessment
                 ORDER BY enriched_at DESC",
            )
            .map_err(|e| e.to_string())?;
        let rows: Vec<serde_json::Value> = stmt
            .query_map([], |row| {
                Ok(serde_json::json!({
                    "entityId": row.get::<_, String>(0)?,
                    "entityType": row.get::<_, Option<String>>(1)?,
                    "enrichedAt": row.get::<_, Option<String>>(2)?,
                    "executiveAssessment": row.get::<_, Option<String>>(3)?,
                    "risks": row.get::<_, Option<String>>(4)?,
                    "recentWins": row.get::<_, Option<String>>(5)?,
                    "currentState": row.get::<_, Option<String>>(6)?,
                }))
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        counts.intelligence = rows.len();
        zip.start_file("intelligence.json", options)
            .map_err(|e| e.to_string())?;
        zip.write_all(serde_json::to_string_pretty(&rows).unwrap().as_bytes())
            .map_err(|e| e.to_string())?;
    }

    // Metadata file
    {
        let version = env!("CARGO_PKG_VERSION");
        let metadata = serde_json::json!({
            "exportedAt": &timestamp,
            "version": version,
            "counts": {
                "accounts": counts.accounts,
                "people": counts.people,
                "projects": counts.projects,
                "meetings": counts.meetings,
                "actions": counts.actions,
                "signals": counts.signals,
                "intelligence": counts.intelligence,
            }
        });
        zip.start_file("metadata.json", options)
            .map_err(|e| e.to_string())?;
        zip.write_all(serde_json::to_string_pretty(&metadata).unwrap().as_bytes())
            .map_err(|e| e.to_string())?;
    }

    zip.finish().map_err(|e| e.to_string())?;

    Ok(ExportReport {
        path: dest_path.to_string_lossy().to_string(),
        timestamp,
        counts,
    })
}
