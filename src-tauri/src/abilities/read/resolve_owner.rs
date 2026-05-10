//! DB-backed owner resolver for typed CommitmentClaim extraction.
//!
//! The resolver is layered and deterministic:
//! 1. user reassignment already stored on the action for this commitment_id
//! 2. exact email / exact account-stakeholder name / exact global person name
//! 3. high-confidence account-stakeholder fuzzy name
//! 4. team labels
//! 5. explicit ambiguous owner

use rusqlite::params;

use crate::abilities::extractors::commitment::{OwnerCandidate, OwnerRef};
use crate::db::ActionDb;

#[derive(Debug, Clone, PartialEq)]
pub struct OwnerResolution {
    pub owner_ref: OwnerRef,
    pub owner_raw: Option<String>,
    pub owner_entity_id: Option<String>,
    pub owner_confidence: Option<f64>,
    pub owner_source: String,
}

impl OwnerResolution {
    pub fn unassigned() -> Self {
        Self {
            owner_ref: OwnerRef::Unassigned,
            owner_raw: None,
            owner_entity_id: None,
            owner_confidence: None,
            owner_source: "unassigned".to_string(),
        }
    }
}

pub fn resolve_owner(
    db: &ActionDb,
    account_id: &str,
    commitment_id: &str,
    owner_raw: Option<&str>,
) -> Result<OwnerResolution, String> {
    if let Some(user_resolution) = user_reassigned_owner(db, commitment_id)? {
        return Ok(user_resolution);
    }

    let Some(raw) = normalize_owner_raw(owner_raw) else {
        return Ok(OwnerResolution::unassigned());
    };

    if let Some(team) = team_owner(&raw) {
        return Ok(team);
    }

    let candidates = exact_email_candidates(db, &raw)?;
    if let Some(resolution) = resolution_from_candidates(&raw, candidates, "exact_email") {
        return Ok(resolution);
    }

    let candidates = exact_account_stakeholder_candidates(db, account_id, &raw)?;
    if let Some(resolution) =
        resolution_from_candidates(&raw, candidates, "exact_account_stakeholder_name")
    {
        return Ok(resolution);
    }

    let candidates = exact_global_name_candidates(db, &raw)?;
    if let Some(resolution) = resolution_from_candidates(&raw, candidates, "exact_person_name") {
        return Ok(resolution);
    }

    let fuzzy = fuzzy_account_stakeholder_candidates(db, account_id, &raw)?;
    if let Some(resolution) = resolution_from_candidates(&raw, fuzzy, "fuzzy_account_stakeholder") {
        return Ok(resolution);
    }

    Ok(ambiguous_resolution(
        &raw,
        "no resolver layer matched this owner",
        Vec::new(),
    ))
}

pub fn resolution_to_columns(
    resolution: &OwnerResolution,
) -> (Option<String>, Option<String>, Option<f64>, String) {
    (
        resolution.owner_raw.clone(),
        resolution.owner_entity_id.clone(),
        resolution.owner_confidence,
        resolution.owner_source.clone(),
    )
}

fn user_reassigned_owner(
    db: &ActionDb,
    commitment_id: &str,
) -> Result<Option<OwnerResolution>, String> {
    let row = db
        .conn_ref()
        .query_row(
            "SELECT owner_raw, owner_entity_id, owner_confidence, owner_source
             FROM actions
             WHERE commitment_id = ?1
               AND owner_source = 'user_reassigned'
             ORDER BY updated_at DESC
             LIMIT 1",
            params![commitment_id],
            |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<f64>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            },
        )
        .map(Some)
        .or_else(|err| match err {
            rusqlite::Error::QueryReturnedNoRows => Ok(None),
            other => Err(other),
        })
        .map_err(|e| e.to_string())?;

    let Some((owner_raw, owner_entity_id, owner_confidence, owner_source)) = row else {
        return Ok(None);
    };

    let owner_ref = if let Some(person_id) = owner_entity_id.as_deref() {
        let display_name = person_display_name(db, person_id)?
            .unwrap_or_else(|| owner_raw.clone().unwrap_or_else(|| person_id.to_string()));
        OwnerRef::Person {
            person_id: person_id.to_string(),
            display_name,
            confidence: owner_confidence.unwrap_or(1.0),
            source: owner_source
                .clone()
                .unwrap_or_else(|| "user_reassigned".to_string()),
        }
    } else if let Some(raw) = owner_raw.as_deref() {
        OwnerRef::Ambiguous {
            raw: raw.to_string(),
            reason: "user reassigned to an unresolved owner".to_string(),
            candidates: Vec::new(),
        }
    } else {
        OwnerRef::Unassigned
    };

    Ok(Some(OwnerResolution {
        owner_ref,
        owner_raw,
        owner_entity_id,
        owner_confidence,
        owner_source: owner_source.unwrap_or_else(|| "user_reassigned".to_string()),
    }))
}

fn exact_email_candidates(db: &ActionDb, raw: &str) -> Result<Vec<OwnerCandidate>, String> {
    if !raw.contains('@') {
        return Ok(Vec::new());
    }
    let mut stmt = db
        .conn_ref()
        .prepare(
            "SELECT id, name
             FROM people
             WHERE archived = 0 AND lower(email) = lower(?1)",
        )
        .map_err(|e| e.to_string())?;
    rows_to_candidates(&mut stmt, params![raw], 0.98, "exact_email")
}

fn exact_account_stakeholder_candidates(
    db: &ActionDb,
    account_id: &str,
    raw: &str,
) -> Result<Vec<OwnerCandidate>, String> {
    let mut stmt = db
        .conn_ref()
        .prepare(
            "SELECT p.id, p.name
             FROM account_stakeholders s
             JOIN people p ON p.id = s.person_id
             WHERE s.account_id = ?1
               AND p.archived = 0
               AND lower(trim(p.name)) = lower(trim(?2))",
        )
        .map_err(|e| e.to_string())?;
    rows_to_candidates(
        &mut stmt,
        params![account_id, raw],
        0.96,
        "exact_account_stakeholder_name",
    )
}

fn exact_global_name_candidates(db: &ActionDb, raw: &str) -> Result<Vec<OwnerCandidate>, String> {
    let mut stmt = db
        .conn_ref()
        .prepare(
            "SELECT id, name
             FROM people
             WHERE archived = 0
               AND lower(trim(name)) = lower(trim(?1))",
        )
        .map_err(|e| e.to_string())?;
    rows_to_candidates(&mut stmt, params![raw], 0.92, "exact_person_name")
}

fn fuzzy_account_stakeholder_candidates(
    db: &ActionDb,
    account_id: &str,
    raw: &str,
) -> Result<Vec<OwnerCandidate>, String> {
    let mut stmt = db
        .conn_ref()
        .prepare(
            "SELECT p.id, p.name
             FROM account_stakeholders s
             JOIN people p ON p.id = s.person_id
             WHERE s.account_id = ?1
               AND p.archived = 0",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![account_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| e.to_string())?;

    let mut candidates = Vec::new();
    let raw_lc = raw.to_ascii_lowercase();
    for row in rows {
        let (person_id, display_name) = row.map_err(|e| e.to_string())?;
        let score = strsim::jaro_winkler(&raw_lc, &display_name.to_ascii_lowercase());
        if score >= 0.92 {
            candidates.push(OwnerCandidate {
                person_id,
                display_name,
                confidence: score,
                source: "fuzzy_account_stakeholder".to_string(),
            });
        }
    }
    candidates.sort_by(|a, b| {
        b.confidence
            .partial_cmp(&a.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    Ok(candidates)
}

fn rows_to_candidates<P>(
    stmt: &mut rusqlite::Statement<'_>,
    params: P,
    confidence: f64,
    source: &str,
) -> Result<Vec<OwnerCandidate>, String>
where
    P: rusqlite::Params,
{
    let rows = stmt
        .query_map(params, |row| {
            Ok(OwnerCandidate {
                person_id: row.get(0)?,
                display_name: row.get(1)?,
                confidence,
                source: source.to_string(),
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

fn resolution_from_candidates(
    raw: &str,
    candidates: Vec<OwnerCandidate>,
    source: &str,
) -> Option<OwnerResolution> {
    match candidates.len() {
        0 => None,
        1 => {
            let candidate = candidates.into_iter().next()?;
            Some(OwnerResolution {
                owner_ref: OwnerRef::Person {
                    person_id: candidate.person_id.clone(),
                    display_name: candidate.display_name.clone(),
                    confidence: candidate.confidence,
                    source: source.to_string(),
                },
                owner_raw: Some(raw.to_string()),
                owner_entity_id: Some(candidate.person_id),
                owner_confidence: Some(candidate.confidence),
                owner_source: source.to_string(),
            })
        }
        _ => Some(ambiguous_resolution(
            raw,
            "multiple people matched this owner",
            candidates,
        )),
    }
}

fn ambiguous_resolution(
    raw: &str,
    reason: &str,
    candidates: Vec<OwnerCandidate>,
) -> OwnerResolution {
    OwnerResolution {
        owner_ref: OwnerRef::Ambiguous {
            raw: raw.to_string(),
            reason: reason.to_string(),
            candidates,
        },
        owner_raw: Some(raw.to_string()),
        owner_entity_id: None,
        owner_confidence: Some(0.0),
        owner_source: "ambiguous".to_string(),
    }
}

fn team_owner(raw: &str) -> Option<OwnerResolution> {
    let normalized = raw.to_ascii_lowercase();
    let teamish = [
        "us",
        "we",
        "our team",
        "account team",
        "cs team",
        "customer success",
        "product",
        "engineering",
        "legal",
        "finance",
        "customer",
        "them",
        "joint",
    ];
    if teamish
        .iter()
        .any(|label| normalized == *label || normalized.contains(label))
    {
        return Some(OwnerResolution {
            owner_ref: OwnerRef::Team {
                label: raw.to_string(),
                confidence: 0.7,
                source: "team_label".to_string(),
            },
            owner_raw: Some(raw.to_string()),
            owner_entity_id: None,
            owner_confidence: Some(0.7),
            owner_source: "team_label".to_string(),
        });
    }
    None
}

fn person_display_name(db: &ActionDb, person_id: &str) -> Result<Option<String>, String> {
    db.conn_ref()
        .query_row(
            "SELECT name FROM people WHERE id = ?1",
            params![person_id],
            |row| row.get::<_, String>(0),
        )
        .map(Some)
        .or_else(|err| match err {
            rusqlite::Error::QueryReturnedNoRows => Ok(None),
            other => Err(other),
        })
        .map_err(|e| e.to_string())
}

fn normalize_owner_raw(owner_raw: Option<&str>) -> Option<String> {
    let raw = owner_raw?;
    let normalized = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    let trimmed = normalized.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_utils::test_db;

    #[test]
    fn resolves_exact_account_stakeholder() {
        let db = test_db();
        db.conn_ref()
            .execute(
                "INSERT INTO people (id, email, name, updated_at) VALUES ('p-alex', 'alex@example.com', 'Alex Chen', '2026-01-01')",
                [],
            )
            .unwrap();
        db.conn_ref()
            .execute(
                "INSERT INTO account_stakeholders (account_id, person_id, data_source) VALUES ('acct-1', 'p-alex', 'user')",
                [],
            )
            .unwrap();

        let resolved = resolve_owner(&db, "acct-1", "commitment:test", Some("Alex Chen")).unwrap();
        assert_eq!(resolved.owner_entity_id.as_deref(), Some("p-alex"));
        assert_eq!(resolved.owner_source, "exact_account_stakeholder_name");
    }

    #[test]
    fn ambiguous_owner_is_explicit() {
        let db = test_db();
        let resolved = resolve_owner(&db, "acct-1", "commitment:test", Some("A. Person")).unwrap();
        assert!(matches!(resolved.owner_ref, OwnerRef::Ambiguous { .. }));
        assert_eq!(resolved.owner_source, "ambiguous");
    }

    #[test]
    fn user_reassignment_wins_before_new_resolution() {
        let db = test_db();
        db.conn_ref()
            .execute(
                "INSERT INTO people (id, email, name, updated_at) VALUES ('p-jamie', 'jamie@example.com', 'Jamie Lee', '2026-01-01')",
                [],
            )
            .unwrap();
        db.conn_ref()
            .execute(
                "INSERT INTO actions (id, title, priority, status, created_at, updated_at, action_kind, commitment_id, owner_raw, owner_entity_id, owner_confidence, owner_source)
                 VALUES ('a1', 'Send deck', 3, 'unstarted', '2026-01-01', '2026-01-02', 'commitment', 'commitment:test', 'Jamie Lee', 'p-jamie', 1.0, 'user_reassigned')",
                [],
            )
            .unwrap();

        let resolved = resolve_owner(&db, "acct-1", "commitment:test", Some("Alex Chen")).unwrap();
        assert_eq!(resolved.owner_entity_id.as_deref(), Some("p-jamie"));
        assert_eq!(resolved.owner_source, "user_reassigned");
    }
}
