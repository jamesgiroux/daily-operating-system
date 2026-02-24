//! Database operations for Google Drive watched sources.

use crate::db::ActionDb;

/// Represents a watched Google Drive source.
#[derive(Debug, Clone)]
pub struct WatchedSource {
    pub id: String,
    pub google_id: String,
    pub name: String,
    pub file_type: String,
    pub google_doc_url: Option<String>,
    pub entity_id: String,
    pub entity_type: String,
    pub last_synced_at: Option<String>,
    pub changes_token: Option<String>,
}

/// Upsert a watched Drive source into the database.
pub fn upsert_watched_source(
    db: &ActionDb,
    google_id: &str,
    name: &str,
    file_type: &str,
    google_doc_url: Option<&str>,
    entity_id: &str,
    entity_type: &str,
) -> Result<String, String> {
    let conn = db.conn_ref();
    let watch_id = format!("drive-watch-{}", uuid::Uuid::new_v4());

    conn.execute(
        "INSERT OR REPLACE INTO drive_watched_sources (
            id, google_id, name, file_type, google_doc_url,
            entity_id, entity_type, created_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, datetime('now'))",
        rusqlite::params![
            &watch_id,
            google_id,
            name,
            file_type,
            google_doc_url,
            entity_id,
            entity_type,
        ],
    )
    .map_err(|e| format!("Failed to upsert watched source: {}", e))?;

    Ok(watch_id)
}

/// Remove a watched Drive source.
pub fn remove_watched_source(db: &ActionDb, watch_id: &str) -> Result<(), String> {
    let conn = db.conn_ref();
    conn.execute("DELETE FROM drive_watched_sources WHERE id = ?", rusqlite::params![watch_id])
        .map_err(|e| format!("Failed to remove watched source: {}", e))?;

    Ok(())
}

/// Get all watched Drive sources.
pub fn get_all_watched_sources(db: &ActionDb) -> Result<Vec<WatchedSource>, String> {
    let conn = db.conn_ref();
    let mut stmt = conn
        .prepare(
            "SELECT id, google_id, name, file_type, google_doc_url,
                    entity_id, entity_type, last_synced_at, changes_token
             FROM drive_watched_sources
             ORDER BY created_at DESC",
        )
        .map_err(|e| format!("Failed to prepare query: {}", e))?;

    let sources = stmt
        .query_map([], |row| {
            Ok(WatchedSource {
                id: row.get(0)?,
                google_id: row.get(1)?,
                name: row.get(2)?,
                file_type: row.get(3)?,
                google_doc_url: row.get(4)?,
                entity_id: row.get(5)?,
                entity_type: row.get(6)?,
                last_synced_at: row.get(7)?,
                changes_token: row.get(8)?,
            })
        })
        .map_err(|e| format!("Failed to execute query: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect results: {}", e))?;

    Ok(sources)
}

/// Mark a watched source as synced and update its changes token.
pub fn mark_synced(db: &ActionDb, watch_id: &str, changes_token: &str) -> Result<(), String> {
    let conn = db.conn_ref();
    conn.execute(
        "UPDATE drive_watched_sources SET last_synced_at = datetime('now'), changes_token = ? WHERE id = ?",
        rusqlite::params![changes_token, watch_id],
    )
    .map_err(|e| format!("Failed to mark synced: {}", e))?;

    Ok(())
}
