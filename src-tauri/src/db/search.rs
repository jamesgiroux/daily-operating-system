//! I427: Full-text search across entities.

use rusqlite::params;
use serde::{Deserialize, Serialize};

use super::DbError;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobalSearchResult {
    pub entity_id: String,
    pub entity_type: String,
    pub name: String,
    pub secondary_text: String,
    pub route: String,
    pub rank: f64,
}

pub trait SearchDb {
    fn search_global(&self, query: &str, limit: i32) -> Result<Vec<GlobalSearchResult>, DbError>;
    fn rebuild_search_index(&self) -> Result<usize, DbError>;
}

impl SearchDb for rusqlite::Connection {
    fn search_global(&self, query: &str, limit: i32) -> Result<Vec<GlobalSearchResult>, DbError> {
        // Sanitize query for FTS5: escape special chars, add prefix matching
        let sanitized = sanitize_fts5_query(query);
        if sanitized.is_empty() {
            return Ok(vec![]);
        }

        let mut stmt = self.prepare(
            "SELECT entity_id, entity_type, name, secondary_text, route, rank
             FROM search_index
             WHERE search_index MATCH ?1
             ORDER BY rank
             LIMIT ?2",
        )?;

        let rows = stmt.query_map(params![sanitized, limit], |row| {
            Ok(GlobalSearchResult {
                entity_id: row.get(0)?,
                entity_type: row.get(1)?,
                name: row.get(2)?,
                secondary_text: row.get(3)?,
                route: row.get(4)?,
                rank: row.get(5)?,
            })
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    fn rebuild_search_index(&self) -> Result<usize, DbError> {
        // Clear existing index
        self.execute("DELETE FROM search_index", [])?;

        let mut count = 0usize;

        // Index accounts (non-archived)
        count += self.execute(
            "INSERT INTO search_index (entity_id, entity_type, name, secondary_text, route)
             SELECT id, 'account', name, COALESCE(lifecycle, ''), '/accounts/' || id
             FROM accounts WHERE archived = 0",
            [],
        )?;

        // Index projects (non-archived)
        count += self.execute(
            "INSERT INTO search_index (entity_id, entity_type, name, secondary_text, route)
             SELECT id, 'project', name, COALESCE(status, ''), '/projects/' || id
             FROM projects WHERE archived = 0",
            [],
        )?;

        // Index people (non-archived)
        count += self.execute(
            "INSERT INTO search_index (entity_id, entity_type, name, secondary_text, route)
             SELECT id, 'person', name, COALESCE(role, '') || ' ' || COALESCE(organization, ''), '/people/' || id
             FROM people WHERE archived = 0",
            [],
        )?;

        // Index meetings (last 90 days + future)
        count += self.execute(
            "INSERT INTO search_index (entity_id, entity_type, name, secondary_text, route)
             SELECT id, 'meeting', title, COALESCE(meeting_type, ''), '/meeting/' || id
             FROM meetings
             WHERE start_time > datetime('now', '-90 days')",
            [],
        )?;

        // Index actions (non-completed, non-archived)
        count += self.execute(
            "INSERT INTO search_index (entity_id, entity_type, name, secondary_text, route)
             SELECT id, 'action', title, COALESCE(source_type, ''), '/actions/' || id
             FROM actions WHERE status NOT IN ('completed', 'cancelled', 'archived')",
            [],
        )?;

        // Index emails (last 90 days)
        count += self.execute(
            "INSERT INTO search_index (entity_id, entity_type, name, secondary_text, route)
             SELECT email_id, 'email', subject, COALESCE(sender_name, sender_email, ''), '/emails'
             FROM emails
             WHERE received_at > datetime('now', '-90 days')",
            [],
        )?;

        Ok(count)
    }
}

/// Sanitize user input for FTS5 MATCH queries.
/// Escapes special characters and adds prefix matching with `*`.
fn sanitize_fts5_query(query: &str) -> String {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    // Split into words, clean each, add prefix * for partial matching.
    // FTS5 prefix queries: `word*` matches any token starting with `word`.
    // Quoting ("word"*) is INVALID — prefix * only works on bare tokens.
    let words: Vec<String> = trimmed
        .split_whitespace()
        .map(|word| {
            // Remove FTS5 special chars (AND OR NOT " * ^ : etc.)
            let clean: String = word
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
                .collect();
            if clean.is_empty() {
                String::new()
            } else {
                format!("{}*", clean)
            }
        })
        .filter(|w| !w.is_empty())
        .collect();

    words.join(" ")
}
