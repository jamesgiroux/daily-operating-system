//! Person-to-person relationship storage (I390 — ADR-0088).

use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

use crate::db::{ActionDb, DbError};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Person-to-person relationship types. These describe how two people relate
/// to each other — NOT their role within an account buying committee.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationshipType {
    Peer,
    Manager,
    Mentor,
    Collaborator,
    Ally,
    Partner,
    IntroducedBy,
}

impl fmt::Display for RelationshipType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Peer => "peer",
            Self::Manager => "manager",
            Self::Mentor => "mentor",
            Self::Collaborator => "collaborator",
            Self::Ally => "ally",
            Self::Partner => "partner",
            Self::IntroducedBy => "introduced_by",
        };
        write!(f, "{}", s)
    }
}

impl FromStr for RelationshipType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "peer" => Ok(Self::Peer),
            "manager" => Ok(Self::Manager),
            "reports_to" => Ok(Self::Manager), // legacy alias
            "mentor" => Ok(Self::Mentor),
            "collaborator" => Ok(Self::Collaborator),
            "ally" => Ok(Self::Ally),
            "partner" => Ok(Self::Partner),
            "introduced_by" => Ok(Self::IntroducedBy),
            _ => Err(format!("Unknown relationship type: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonRelationship {
    pub id: String,
    pub from_person_id: String,
    pub to_person_id: String,
    pub from_person_name: Option<String>,
    pub to_person_name: Option<String>,
    pub relationship_type: RelationshipType,
    pub direction: String,
    pub confidence: f64,
    pub effective_confidence: f64,
    pub context_entity_id: Option<String>,
    pub context_entity_type: Option<String>,
    pub context_entity_name: Option<String>,
    pub source: String,
    pub rationale: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub last_reinforced_at: Option<String>,
}

/// Compute time-decayed confidence. User-confirmed edges don't decay.
/// All others: 90-day half-life from last_reinforced_at (or created_at).
pub fn effective_confidence(
    confidence: f64,
    source: &str,
    last_reinforced_at: Option<&str>,
    created_at: &str,
) -> f64 {
    if source == "user_confirmed" {
        return confidence;
    }
    let reference = last_reinforced_at.unwrap_or(created_at);
    // Parse both RFC 3339 ("2026-01-01T00:00:00Z") and SQLite datetime ("2026-01-01 00:00:00")
    let parsed = chrono::DateTime::parse_from_rfc3339(reference)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .or_else(|_| {
            chrono::NaiveDateTime::parse_from_str(reference, "%Y-%m-%d %H:%M:%S")
                .map(|ndt| ndt.and_utc())
        });
    let age_days = chrono::Utc::now()
        .signed_duration_since(parsed.unwrap_or_else(|_| chrono::Utc::now()))
        .num_days() as f64;
    if age_days <= 0.0 {
        return confidence;
    }
    confidence * 2f64.powf(-age_days / 90.0)
}

// ---------------------------------------------------------------------------
// DB functions
// ---------------------------------------------------------------------------

/// Parameters for upserting a person relationship.
pub struct UpsertRelationship<'a> {
    pub id: &'a str,
    pub from_person_id: &'a str,
    pub to_person_id: &'a str,
    pub relationship_type: &'a str,
    pub direction: &'a str,
    pub confidence: f64,
    pub context_entity_id: Option<&'a str>,
    pub context_entity_type: Option<&'a str>,
    pub source: &'a str,
    pub rationale: Option<&'a str>,
}

impl ActionDb {
    /// Upsert a person relationship by id.
    pub fn upsert_person_relationship(&self, rel: &UpsertRelationship<'_>) -> Result<(), DbError> {
        self.conn_ref().execute(
            "INSERT INTO person_relationships (id, from_person_id, to_person_id, relationship_type,
             direction, confidence, context_entity_id, context_entity_type, source, rationale)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(id) DO UPDATE SET
                from_person_id = excluded.from_person_id,
                to_person_id = excluded.to_person_id,
                relationship_type = excluded.relationship_type,
                direction = excluded.direction,
                confidence = excluded.confidence,
                context_entity_id = excluded.context_entity_id,
                context_entity_type = excluded.context_entity_type,
                source = excluded.source,
                rationale = excluded.rationale,
                updated_at = datetime('now'),
                last_reinforced_at = datetime('now')",
            params![
                rel.id,
                rel.from_person_id,
                rel.to_person_id,
                rel.relationship_type,
                rel.direction,
                rel.confidence,
                rel.context_entity_id,
                rel.context_entity_type,
                rel.source,
                rel.rationale,
            ],
        )?;
        Ok(())
    }

    /// Get a single relationship by ID (used before delete to capture person IDs for signaling).
    pub fn get_person_relationship_by_id(
        &self,
        id: &str,
    ) -> Result<Option<(String, String)>, DbError> {
        let mut stmt = self.conn_ref().prepare(
            "SELECT from_person_id, to_person_id FROM person_relationships WHERE id = ?1",
        )?;
        let result = stmt.query_row(params![id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        });
        match result {
            Ok(pair) => Ok(Some(pair)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Delete a person relationship by ID.
    pub fn delete_person_relationship(&self, id: &str) -> Result<(), DbError> {
        self.conn_ref().execute(
            "DELETE FROM person_relationships WHERE id = ?1",
            params![id],
        )?;
        Ok(())
    }

    const RELATIONSHIP_SELECT: &'static str =
        "SELECT pr.id, pr.from_person_id, pr.to_person_id, pr.relationship_type, pr.direction,
                pr.confidence, pr.context_entity_id, pr.context_entity_type, pr.source,
                pr.rationale, pr.created_at, pr.updated_at, pr.last_reinforced_at,
                fp.name AS from_name, tp.name AS to_name,
                e.name AS context_entity_name
         FROM person_relationships pr
         LEFT JOIN people fp ON fp.id = pr.from_person_id
         LEFT JOIN people tp ON tp.id = pr.to_person_id
         LEFT JOIN entities e ON e.id = pr.context_entity_id";

    /// Get all relationships for a person (both from and to edges), with names resolved.
    pub fn get_relationships_for_person(
        &self,
        person_id: &str,
    ) -> Result<Vec<PersonRelationship>, DbError> {
        let sql = format!(
            "{} WHERE pr.from_person_id = ?1 OR pr.to_person_id = ?1 ORDER BY pr.confidence DESC",
            Self::RELATIONSHIP_SELECT
        );
        let mut stmt = self.conn_ref().prepare(&sql)?;
        let rows = stmt.query_map(params![person_id], Self::map_relationship_row)?;
        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    /// Get relationships between two specific people.
    pub fn get_relationships_between(
        &self,
        from_id: &str,
        to_id: &str,
    ) -> Result<Vec<PersonRelationship>, DbError> {
        let sql = format!(
            "{} WHERE (pr.from_person_id = ?1 AND pr.to_person_id = ?2) \
                OR (pr.from_person_id = ?2 AND pr.to_person_id = ?1) \
             ORDER BY pr.confidence DESC",
            Self::RELATIONSHIP_SELECT
        );
        let mut stmt = self.conn_ref().prepare(&sql)?;
        let rows = stmt.query_map(params![from_id, to_id], Self::map_relationship_row)?;
        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    fn map_relationship_row(
        row: &rusqlite::Row<'_>,
    ) -> Result<PersonRelationship, rusqlite::Error> {
        let confidence: f64 = row.get(5)?;
        let source: String = row.get(8)?;
        let rationale: Option<String> = row.get(9)?;
        let created_at: String = row.get(10)?;
        let last_reinforced_at: Option<String> = row.get(12)?;
        let eff = effective_confidence(
            confidence,
            &source,
            last_reinforced_at.as_deref(),
            &created_at,
        );
        let rel_type_str: String = row.get(3)?;
        let relationship_type =
            RelationshipType::from_str(&rel_type_str).unwrap_or(RelationshipType::Peer);
        Ok(PersonRelationship {
            id: row.get(0)?,
            from_person_id: row.get(1)?,
            to_person_id: row.get(2)?,
            from_person_name: row.get(13)?,
            to_person_name: row.get(14)?,
            relationship_type,
            direction: row.get(4)?,
            confidence,
            effective_confidence: eff,
            context_entity_id: row.get(6)?,
            context_entity_type: row.get(7)?,
            context_entity_name: row.get(15)?,
            source,
            rationale,
            created_at,
            updated_at: row.get(11)?,
            last_reinforced_at,
        })
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_utils::test_db;

    #[test]
    fn test_relationship_type_roundtrip() {
        let types = [
            RelationshipType::Peer,
            RelationshipType::Manager,
            RelationshipType::Mentor,
            RelationshipType::Collaborator,
            RelationshipType::Ally,
            RelationshipType::Partner,
            RelationshipType::IntroducedBy,
        ];
        for t in &types {
            let s = t.to_string();
            let parsed = RelationshipType::from_str(&s).unwrap_or_else(|_| panic!("parse {}", s));
            assert_eq!(*t, parsed);
        }
    }

    #[test]
    fn test_effective_confidence_user_confirmed() {
        let c = effective_confidence(0.9, "user_confirmed", None, "2020-01-01T00:00:00Z");
        assert!((c - 0.9).abs() < 0.001, "user_confirmed should not decay");
    }

    #[test]
    fn test_effective_confidence_decays() {
        let ninety_days_ago = (chrono::Utc::now() - chrono::Duration::days(90)).to_rfc3339();
        let c = effective_confidence(1.0, "inferred", None, &ninety_days_ago);
        assert!(
            (c - 0.5).abs() < 0.05,
            "Should decay to ~0.5 after 90 days, got {}",
            c
        );
    }

    #[test]
    fn test_upsert_and_get_relationships() {
        let db = test_db();
        let conn = db.conn_ref();

        conn.execute(
            "INSERT INTO people (id, email, name, updated_at) VALUES ('p1', 'a@test.com', 'Alice', '2026-01-01')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO people (id, email, name, updated_at) VALUES ('p2', 'b@test.com', 'Bob', '2026-01-01')",
            [],
        )
        .unwrap();

        db.upsert_person_relationship(&UpsertRelationship {
            id: "rel-1",
            from_person_id: "p1",
            to_person_id: "p2",
            relationship_type: "manager",
            direction: "directed",
            confidence: 0.9,
            context_entity_id: None,
            context_entity_type: None,
            source: "user_confirmed",
            rationale: None,
        })
        .expect("upsert");

        let rels = db.get_relationships_for_person("p1").expect("get");
        assert_eq!(rels.len(), 1);
        assert_eq!(rels[0].from_person_id, "p1");
        assert_eq!(rels[0].to_person_id, "p2");
        assert_eq!(rels[0].relationship_type, RelationshipType::Manager);
        assert!((rels[0].effective_confidence - 0.9).abs() < 0.001);

        // Also visible from p2's perspective
        let rels2 = db.get_relationships_for_person("p2").expect("get");
        assert_eq!(rels2.len(), 1);

        // Between query
        let between = db.get_relationships_between("p1", "p2").expect("between");
        assert_eq!(between.len(), 1);
    }

    #[test]
    fn test_delete_relationship() {
        let db = test_db();
        let conn = db.conn_ref();
        conn.execute(
            "INSERT INTO people (id, email, name, updated_at) VALUES ('p1', 'a@test.com', 'Alice', '2026-01-01')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO people (id, email, name, updated_at) VALUES ('p2', 'b@test.com', 'Bob', '2026-01-01')",
            [],
        )
        .unwrap();

        db.upsert_person_relationship(&UpsertRelationship {
            id: "rel-1",
            from_person_id: "p1",
            to_person_id: "p2",
            relationship_type: "peer",
            direction: "symmetric",
            confidence: 0.7,
            context_entity_id: None,
            context_entity_type: None,
            source: "inferred",
            rationale: None,
        })
        .unwrap();
        assert_eq!(db.get_relationships_for_person("p1").unwrap().len(), 1);

        db.delete_person_relationship("rel-1").expect("delete");
        assert_eq!(db.get_relationships_for_person("p1").unwrap().len(), 0);
    }

    #[test]
    fn test_context_scoped_relationship() {
        let db = test_db();
        let conn = db.conn_ref();
        conn.execute(
            "INSERT INTO people (id, email, name, updated_at) VALUES ('p1', 'a@test.com', 'Alice', '2026-01-01')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO people (id, email, name, updated_at) VALUES ('p2', 'b@test.com', 'Bob', '2026-01-01')",
            [],
        )
        .unwrap();

        db.upsert_person_relationship(&UpsertRelationship {
            id: "rel-ctx",
            from_person_id: "p1",
            to_person_id: "p2",
            relationship_type: "collaborator",
            direction: "symmetric",
            confidence: 0.8,
            context_entity_id: Some("proj-1"),
            context_entity_type: Some("project"),
            source: "inferred",
            rationale: None,
        })
        .unwrap();

        let rels = db.get_relationships_for_person("p1").unwrap();
        assert_eq!(rels.len(), 1);
        assert_eq!(rels[0].context_entity_id.as_deref(), Some("proj-1"));
        assert_eq!(rels[0].context_entity_type.as_deref(), Some("project"));
    }
}
