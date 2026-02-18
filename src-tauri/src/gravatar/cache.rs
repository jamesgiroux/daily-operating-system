//! SQLite cache layer for Gravatar profile data.

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

/// Cached Gravatar data from the gravatar_cache table.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CachedGravatar {
    pub email: String,
    pub avatar_url: Option<String>,
    pub display_name: Option<String>,
    pub bio: Option<String>,
    pub location: Option<String>,
    pub company: Option<String>,
    pub job_title: Option<String>,
    pub interests_json: Option<String>,
    pub has_gravatar: bool,
    pub fetched_at: String,
    pub person_id: Option<String>,
}

/// Get cached Gravatar data for an email.
pub fn get_cached(conn: &Connection, email: &str) -> Option<CachedGravatar> {
    conn.query_row(
        "SELECT email, avatar_url, display_name, bio, location, company, job_title,
                interests_json, has_gravatar, fetched_at, person_id
         FROM gravatar_cache WHERE email = ?1",
        [email],
        |row| {
            Ok(CachedGravatar {
                email: row.get(0)?,
                avatar_url: row.get(1)?,
                display_name: row.get(2)?,
                bio: row.get(3)?,
                location: row.get(4)?,
                company: row.get(5)?,
                job_title: row.get(6)?,
                interests_json: row.get(7)?,
                has_gravatar: row.get::<_, i32>(8)? != 0,
                fetched_at: row.get(9)?,
                person_id: row.get(10)?,
            })
        },
    )
    .ok()
}

/// Insert or update a cached Gravatar entry.
pub fn upsert_cache(conn: &Connection, data: &CachedGravatar) -> Result<(), String> {
    conn.execute(
        "INSERT INTO gravatar_cache
            (email, avatar_url, display_name, bio, location, company, job_title,
             interests_json, has_gravatar, fetched_at, person_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
         ON CONFLICT(email) DO UPDATE SET
            avatar_url = excluded.avatar_url,
            display_name = excluded.display_name,
            bio = excluded.bio,
            location = excluded.location,
            company = excluded.company,
            job_title = excluded.job_title,
            interests_json = excluded.interests_json,
            has_gravatar = excluded.has_gravatar,
            fetched_at = excluded.fetched_at,
            person_id = excluded.person_id",
        rusqlite::params![
            data.email,
            data.avatar_url,
            data.display_name,
            data.bio,
            data.location,
            data.company,
            data.job_title,
            data.interests_json,
            data.has_gravatar as i32,
            data.fetched_at,
            data.person_id,
        ],
    )
    .map_err(|e| format!("Failed to upsert gravatar cache: {}", e))?;
    Ok(())
}

/// Check if a cached entry is stale (older than 7 days).
pub fn is_stale(fetched_at: &str) -> bool {
    chrono::DateTime::parse_from_rfc3339(fetched_at)
        .map(|dt| {
            let age = chrono::Utc::now() - dt.with_timezone(&chrono::Utc);
            age.num_days() >= 7
        })
        .unwrap_or(true)
}

/// Get the local avatar file path for a person by their person_id.
pub fn get_avatar_url_for_person(conn: &Connection, person_id: &str) -> Option<String> {
    // Join through person_emails to find the cached avatar
    conn.query_row(
        "SELECT gc.avatar_url FROM gravatar_cache gc
         INNER JOIN person_emails pe ON pe.email = gc.email
         WHERE pe.person_id = ?1 AND gc.has_gravatar = 1 AND gc.avatar_url IS NOT NULL
         LIMIT 1",
        [person_id],
        |row| row.get(0),
    )
    .ok()
}

/// Get emails that need fetching: no cache entry or stale cache.
/// Returns (email, person_id) pairs, limited to `max_count`.
pub fn get_stale_emails(conn: &Connection, max_count: usize) -> Result<Vec<(String, Option<String>)>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT pe.email, pe.person_id
             FROM person_emails pe
             LEFT JOIN gravatar_cache gc ON gc.email = pe.email
             WHERE gc.email IS NULL
                OR datetime(gc.fetched_at) < datetime('now', '-7 days')
             GROUP BY pe.email
             LIMIT ?1",
        )
        .map_err(|e| format!("Failed to query stale emails: {}", e))?;

    let rows = stmt
        .query_map([max_count as i64], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
        })
        .map_err(|e| format!("Failed to read stale emails: {}", e))?;

    let mut results = Vec::new();
    for row in rows {
        if let Ok(r) = row {
            results.push(r);
        }
    }
    Ok(results)
}

/// Count cached Gravatar entries.
pub fn count_cached(conn: &Connection) -> i64 {
    conn.query_row("SELECT COUNT(*) FROM gravatar_cache", [], |row| row.get(0))
        .unwrap_or(0)
}
