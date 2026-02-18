//! Attendee group pattern detection (I307 / ADR-0080 Phase 3).
//!
//! Learns which attendee groups map to which entities by scanning
//! historical meetings. When the same set of attendees appears in
//! multiple meetings linked to the same entity, confidence grows.

use rusqlite::params;
use sha2::{Digest, Sha256};

use crate::db::{ActionDb, DbError};
use crate::prepare::entity_resolver::ResolutionSignal;
use crate::entity::EntityType;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Compute a deterministic, order-independent hash for a set of attendee emails.
pub fn compute_group_hash(emails: &[String]) -> String {
    let mut sorted: Vec<String> = emails.iter().map(|e| e.trim().to_lowercase()).collect();
    sorted.sort();
    sorted.dedup();
    let joined = sorted.join(",");
    let mut hasher = Sha256::new();
    hasher.update(joined.as_bytes());
    hex::encode(hasher.finalize())
}

/// Scan recent meetings (last 90 days) and build/update attendee group patterns.
///
/// For each meeting with linked entities and attendees, computes the group hash
/// and upserts into attendee_group_patterns. Returns the number of patterns updated.
pub fn mine_attendee_patterns(db: &ActionDb) -> Result<usize, DbError> {
    // Get meetings from last 90 days that have attendees
    let meetings: Vec<(String, Option<String>)> = {
        let mut stmt = db.conn_ref().prepare(
            "SELECT id, attendees FROM meetings_history
             WHERE start_time >= date('now', '-90 days')
               AND attendees IS NOT NULL AND attendees != ''
             ORDER BY start_time DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
        })?;
        rows.collect::<Result<Vec<_>, _>>()?
    };

    let mut updated = 0;
    for (meeting_id, attendees_raw) in &meetings {
        let emails = parse_attendee_emails(attendees_raw.as_deref().unwrap_or(""));
        if emails.len() < 2 {
            continue; // Need at least 2 attendees for a meaningful group
        }

        let group_hash = compute_group_hash(&emails);
        let emails_json = serde_json::to_string(&emails).unwrap_or_default();

        // Get linked entities for this meeting
        let entities = db.get_meeting_entities(meeting_id)?;
        for entity in &entities {
            db.upsert_attendee_group_pattern(
                &group_hash,
                &emails_json,
                &entity.id,
                entity.entity_type.as_str(),
            )?;
            updated += 1;
        }
    }

    Ok(updated)
}

/// Signal producer: look up attendee group patterns for a meeting.
///
/// Extracts attendee emails from the meeting JSON, computes the group hash,
/// and returns signals from matching patterns.
pub fn signal_attendee_group_pattern(
    db: &ActionDb,
    meeting: &serde_json::Value,
) -> Vec<ResolutionSignal> {
    let emails = extract_attendee_emails(meeting);
    if emails.len() < 2 {
        return Vec::new();
    }

    let group_hash = compute_group_hash(&emails);

    match db.get_attendee_group_pattern(&group_hash) {
        Ok(Some((entity_id, entity_type, _count, confidence))) => {
            let et = EntityType::from_str_lossy(&entity_type);
            vec![ResolutionSignal {
                entity_id,
                entity_type: et,
                confidence,
                source: "group_pattern".to_string(),
            }]
        }
        _ => Vec::new(),
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Parse attendee emails from the DB format (comma-separated or JSON array).
fn parse_attendee_emails(raw: &str) -> Vec<String> {
    // Try JSON array first
    if let Ok(arr) = serde_json::from_str::<Vec<String>>(raw) {
        return arr
            .into_iter()
            .map(|e| e.trim().to_lowercase())
            .filter(|e| e.contains('@'))
            .collect();
    }
    // Fall back to comma-separated
    raw.split(',')
        .map(|s| s.trim().to_lowercase())
        .filter(|s| s.contains('@'))
        .collect()
}

/// Extract attendee emails from meeting JSON (same logic as entity_resolver).
fn extract_attendee_emails(meeting: &serde_json::Value) -> Vec<String> {
    if let Some(arr) = meeting.get("attendees").and_then(|v| v.as_array()) {
        return arr
            .iter()
            .filter_map(|v| v.as_str())
            .map(|s| s.trim().to_lowercase())
            .filter(|s| s.contains('@'))
            .collect();
    }
    Vec::new()
}

// ---------------------------------------------------------------------------
// ActionDb methods
// ---------------------------------------------------------------------------

impl ActionDb {
    /// Upsert an attendee group pattern, incrementing occurrence_count on conflict.
    pub fn upsert_attendee_group_pattern(
        &self,
        group_hash: &str,
        emails_json: &str,
        entity_id: &str,
        entity_type: &str,
    ) -> Result<(), DbError> {
        // Confidence formula: min(0.85, 0.5 + 0.05 * occurrence_count)
        self.conn_ref().execute(
            "INSERT INTO attendee_group_patterns
                (group_hash, attendee_emails, entity_id, entity_type, occurrence_count, last_seen_at, confidence)
             VALUES (?1, ?2, ?3, ?4, 1, datetime('now'), 0.55)
             ON CONFLICT (group_hash) DO UPDATE SET
                occurrence_count = occurrence_count + 1,
                last_seen_at = datetime('now'),
                confidence = MIN(0.85, 0.5 + 0.05 * (occurrence_count + 1))",
            params![group_hash, emails_json, entity_id, entity_type],
        )?;
        Ok(())
    }

    /// Look up an attendee group pattern by hash.
    /// Returns (entity_id, entity_type, occurrence_count, confidence) or None.
    pub fn get_attendee_group_pattern(
        &self,
        group_hash: &str,
    ) -> Result<Option<(String, String, i32, f64)>, DbError> {
        match self.conn_ref().query_row(
            "SELECT entity_id, entity_type, occurrence_count, confidence
             FROM attendee_group_patterns
             WHERE group_hash = ?1",
            params![group_hash],
            |row| Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i32>(2)?,
                row.get::<_, f64>(3)?,
            )),
        ) {
            Ok(result) => Ok(Some(result)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(DbError::Sqlite(e)),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn test_db() -> ActionDb {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("test.db");
        std::mem::forget(dir);
        ActionDb::open_at(path).expect("open")
    }

    #[test]
    fn test_compute_group_hash_order_independent() {
        let emails_a = vec!["bob@acme.com".to_string(), "alice@acme.com".to_string()];
        let emails_b = vec!["alice@acme.com".to_string(), "bob@acme.com".to_string()];
        assert_eq!(compute_group_hash(&emails_a), compute_group_hash(&emails_b));
    }

    #[test]
    fn test_compute_group_hash_case_insensitive() {
        let emails_a = vec!["Alice@Acme.COM".to_string(), "bob@acme.com".to_string()];
        let emails_b = vec!["alice@acme.com".to_string(), "BOB@ACME.COM".to_string()];
        assert_eq!(compute_group_hash(&emails_a), compute_group_hash(&emails_b));
    }

    #[test]
    fn test_compute_group_hash_deduplicates() {
        let emails_a = vec!["alice@acme.com".to_string(), "alice@acme.com".to_string(), "bob@acme.com".to_string()];
        let emails_b = vec!["alice@acme.com".to_string(), "bob@acme.com".to_string()];
        assert_eq!(compute_group_hash(&emails_a), compute_group_hash(&emails_b));
    }

    #[test]
    fn test_upsert_attendee_group_pattern() {
        let db = test_db();
        let hash = compute_group_hash(&["a@b.com".to_string(), "c@d.com".to_string()]);

        // First insert
        db.upsert_attendee_group_pattern(&hash, "[\"a@b.com\",\"c@d.com\"]", "acme", "account")
            .expect("first upsert");

        let result = db.get_attendee_group_pattern(&hash).unwrap().unwrap();
        assert_eq!(result.0, "acme");
        assert_eq!(result.1, "account");
        assert_eq!(result.2, 1); // occurrence_count
        assert!((result.3 - 0.55).abs() < 0.01); // initial confidence

        // Second upsert — occurrence_count should increment
        db.upsert_attendee_group_pattern(&hash, "[\"a@b.com\",\"c@d.com\"]", "acme", "account")
            .expect("second upsert");

        let result = db.get_attendee_group_pattern(&hash).unwrap().unwrap();
        assert_eq!(result.2, 2); // occurrence_count
        assert!((result.3 - 0.60).abs() < 0.01); // min(0.85, 0.5 + 0.05 * 2)

        // Third upsert
        db.upsert_attendee_group_pattern(&hash, "[\"a@b.com\",\"c@d.com\"]", "acme", "account")
            .expect("third upsert");

        let result = db.get_attendee_group_pattern(&hash).unwrap().unwrap();
        assert_eq!(result.2, 3);
        assert!((result.3 - 0.65).abs() < 0.01); // min(0.85, 0.5 + 0.05 * 3)
    }

    #[test]
    fn test_get_nonexistent_pattern() {
        let db = test_db();
        let result = db.get_attendee_group_pattern("nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_attendee_emails_json() {
        let emails = parse_attendee_emails("[\"Alice@Acme.com\",\"bob@partner.com\"]");
        assert_eq!(emails.len(), 2);
        assert!(emails.contains(&"alice@acme.com".to_string()));
    }

    #[test]
    fn test_parse_attendee_emails_csv() {
        let emails = parse_attendee_emails("alice@acme.com, bob@partner.com");
        assert_eq!(emails.len(), 2);
    }

    #[test]
    fn test_signal_attendee_group_pattern() {
        let db = test_db();
        let emails = vec!["alice@acme.com".to_string(), "bob@partner.com".to_string()];
        let hash = compute_group_hash(&emails);

        // Insert a pattern
        db.upsert_attendee_group_pattern(&hash, "[\"alice@acme.com\",\"bob@partner.com\"]", "acme-1", "account")
            .expect("insert pattern");

        // Create meeting JSON with matching attendees
        let meeting = serde_json::json!({
            "attendees": ["alice@acme.com", "bob@partner.com"]
        });

        let signals = signal_attendee_group_pattern(&db, &meeting);
        assert_eq!(signals.len(), 1);
        assert_eq!(signals[0].entity_id, "acme-1");
        assert_eq!(signals[0].source, "group_pattern");
    }

    #[test]
    fn test_mine_attendee_patterns() {
        let db = test_db();

        // Insert 3 meetings with same attendees linked to same entity
        for i in 1..=3 {
            let meeting_id = format!("m{}", i);
            db.conn_ref().execute(
                "INSERT INTO meetings_history (id, title, meeting_type, start_time, created_at, attendees)
                 VALUES (?1, 'Test Meeting', 'customer', datetime('now', '-1 day'), datetime('now'),
                         '[\"alice@acme.com\",\"bob@partner.com\"]')",
                params![meeting_id],
            ).expect("insert meeting");

            // Create entity if not exists (only once)
            if i == 1 {
                db.conn_ref().execute(
                    "INSERT OR IGNORE INTO entities (id, name, entity_type, updated_at)
                     VALUES ('acme-1', 'Acme Corp', 'account', datetime('now'))",
                    [],
                ).expect("insert entity");
            }

            // Link meeting to entity
            db.conn_ref().execute(
                "INSERT INTO meeting_entities (meeting_id, entity_id, entity_type)
                 VALUES (?1, 'acme-1', 'account')",
                params![meeting_id],
            ).expect("link entity");
        }

        let updated = mine_attendee_patterns(&db).expect("mine patterns");
        assert_eq!(updated, 3, "should update pattern 3 times");

        // Verify pattern was created with correct count
        let emails = vec!["alice@acme.com".to_string(), "bob@partner.com".to_string()];
        let hash = compute_group_hash(&emails);
        let pattern = db.get_attendee_group_pattern(&hash).unwrap().unwrap();
        assert_eq!(pattern.0, "acme-1"); // entity_id
        assert_eq!(pattern.2, 3); // occurrence_count
        assert!((pattern.3 - 0.65).abs() < 0.01); // confidence: min(0.85, 0.5 + 0.05 * 3)
    }
}
