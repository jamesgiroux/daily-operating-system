//! DB-backed trust footprint extraction.
//!
//! This module is the service boundary between SQLite state and the pure trust
//! compiler. It builds value objects only; no DB handle crosses into
//! `abilities::trust`.

use std::collections::{HashMap, HashSet};

use rusqlite::{params, OptionalExtension};
use serde_json::Value;

use crate::abilities::provenance::SubjectRef;
use crate::abilities::trust::types::{EntityFootprint, TargetFootprint};
use crate::db::{ActionDb, DbAccount, DbError};

/// Outcome of attempting to build a TargetFootprint for the recompute path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExtractionOutcome {
    /// Footprint built; safe to compile_trust against this and update.
    Ok {
        footprint: TargetFootprint,
        portfolio_footprints: Vec<EntityFootprint>,
    },
    /// Target row not found, or the claim subject does not match the resolved
    /// entity. Recompute callers must skip the trust update and preserve the
    /// previous score/version.
    SkipExtractorMismatch { reason: ExtractionMismatchReason },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtractionMismatchReason {
    TargetNotFound,
    SubjectRefMismatch,
}

#[derive(Debug, thiserror::Error)]
pub enum ExtractionError {
    #[error("database error: {0}")]
    Db(#[from] DbError),
}

pub fn extract_target_footprint(
    db: &ActionDb,
    subject: &SubjectRef,
    entity_type: &str,
    entity_id: &str,
) -> Result<ExtractionOutcome, ExtractionError> {
    if !entity_exists(db, entity_type, entity_id)? {
        return Ok(ExtractionOutcome::SkipExtractorMismatch {
            reason: ExtractionMismatchReason::TargetNotFound,
        });
    }

    let Some(expected_subject) = subject_for_entity(entity_type, entity_id) else {
        return Ok(ExtractionOutcome::SkipExtractorMismatch {
            reason: ExtractionMismatchReason::SubjectRefMismatch,
        });
    };

    if subject != &expected_subject {
        return Ok(ExtractionOutcome::SkipExtractorMismatch {
            reason: ExtractionMismatchReason::SubjectRefMismatch,
        });
    }

    if normalize_entity_type(entity_type) != "account" {
        return Ok(ExtractionOutcome::SkipExtractorMismatch {
            reason: ExtractionMismatchReason::TargetNotFound,
        });
    }

    let Some(target) = db.get_account(entity_id)? else {
        return Ok(ExtractionOutcome::SkipExtractorMismatch {
            reason: ExtractionMismatchReason::TargetNotFound,
        });
    };

    let accounts_with_domains = db.get_all_accounts_with_domains(false)?;
    let by_id: HashMap<String, (DbAccount, Vec<String>)> = accounts_with_domains
        .into_iter()
        .map(|(account, domains)| (account.id.clone(), (account, domains)))
        .collect();

    let target_domains = db.get_account_domains(entity_id)?;
    let target_aliases = aliases_for_account(&target);
    let related_subjects = related_subjects_for_account(&target, &by_id);
    let portfolio_ids = portfolio_account_ids(&target, &by_id);

    let portfolio_footprints = portfolio_ids
        .into_iter()
        .filter_map(|account_id| by_id.get(&account_id))
        .map(|(account, domains)| entity_footprint(account, domains))
        .collect();

    Ok(ExtractionOutcome::Ok {
        footprint: TargetFootprint {
            subject: expected_subject,
            names: names_for_account(&target),
            domains: domain_variants(&target_domains),
            related_subjects,
            allowed_aliases: target_aliases,
        },
        portfolio_footprints,
    })
}

fn entity_exists(db: &ActionDb, entity_type: &str, entity_id: &str) -> Result<bool, DbError> {
    if normalize_entity_type(entity_type) == "global" {
        return Ok(entity_id.trim().is_empty() || entity_id.eq_ignore_ascii_case("global"));
    }

    let Some(table) = table_for_entity_type(entity_type) else {
        return Ok(false);
    };

    let sql = format!("SELECT 1 FROM {table} WHERE id = ?1 LIMIT 1");
    let found = db
        .conn_ref()
        .query_row(&sql, params![entity_id], |_| Ok(()))
        .optional()?
        .is_some();
    Ok(found)
}

fn table_for_entity_type(entity_type: &str) -> Option<&'static str> {
    match normalize_entity_type(entity_type).as_str() {
        "account" => Some("accounts"),
        "project" => Some("projects"),
        "person" => Some("people"),
        "meeting" => Some("meetings_history"),
        "user" => Some("user_entity"),
        _ => None,
    }
}

fn subject_for_entity(entity_type: &str, entity_id: &str) -> Option<SubjectRef> {
    match normalize_entity_type(entity_type).as_str() {
        "account" => Some(SubjectRef::Account(entity_id.to_string())),
        "project" => Some(SubjectRef::Project(entity_id.to_string())),
        "person" => Some(SubjectRef::Person(entity_id.to_string())),
        "meeting" => Some(SubjectRef::Meeting(entity_id.to_string())),
        "user" => Some(SubjectRef::User(entity_id.to_string())),
        "global" => Some(SubjectRef::Global),
        _ => None,
    }
}

fn normalize_entity_type(entity_type: &str) -> String {
    entity_type
        .trim()
        .trim_end_matches('s')
        .to_ascii_lowercase()
}

fn related_subjects_for_account(
    target: &DbAccount,
    by_id: &HashMap<String, (DbAccount, Vec<String>)>,
) -> Vec<SubjectRef> {
    let mut related_ids = Vec::new();
    if let Some(parent_id) = target.parent_id.as_deref() {
        related_ids.push(parent_id.to_string());
    }
    related_ids.extend(
        by_id
            .values()
            .filter(|(account, _)| account.parent_id.as_deref() == Some(target.id.as_str()))
            .map(|(account, _)| account.id.clone()),
    );

    let mut seen = HashSet::new();
    related_ids
        .into_iter()
        .filter(|id| id != &target.id && seen.insert(id.clone()))
        .map(SubjectRef::Account)
        .collect()
}

fn portfolio_account_ids(
    target: &DbAccount,
    by_id: &HashMap<String, (DbAccount, Vec<String>)>,
) -> Vec<String> {
    let mut ids = Vec::new();

    if let Some(parent_id) = target.parent_id.as_deref() {
        ids.push(parent_id.to_string());
        ids.extend(
            by_id
                .values()
                .filter(|(account, _)| account.parent_id.as_deref() == Some(parent_id))
                .map(|(account, _)| account.id.clone()),
        );
    }

    ids.extend(
        by_id
            .values()
            .filter(|(account, _)| account.parent_id.as_deref() == Some(target.id.as_str()))
            .map(|(account, _)| account.id.clone()),
    );

    let mut seen = HashSet::new();
    ids.into_iter()
        .filter(|id| id != &target.id && seen.insert(id.clone()))
        .collect()
}

fn entity_footprint(account: &DbAccount, domains: &[String]) -> EntityFootprint {
    EntityFootprint {
        subject: SubjectRef::Account(account.id.clone()),
        names: names_for_account(account),
        domains: domain_variants(domains),
        infrastructure_ids: infrastructure_ids(domains),
    }
}

fn names_for_account(account: &DbAccount) -> Vec<String> {
    let mut names = vec![account.name.clone()];
    names.extend(aliases_for_account(account));
    clean_dedup(names)
}

fn aliases_for_account(account: &DbAccount) -> Vec<String> {
    let Some(metadata) = account.metadata.as_deref() else {
        return Vec::new();
    };
    let Ok(value) = serde_json::from_str::<Value>(metadata) else {
        return Vec::new();
    };

    let mut aliases = Vec::new();
    for key in [
        "alias",
        "aliases",
        "account_aliases",
        "dba",
        "dbas",
        "dba_names",
        "doing_business_as",
        "legal_name",
        "legalName",
    ] {
        collect_alias_values(value.get(key), &mut aliases);
    }
    clean_dedup(aliases)
}

fn collect_alias_values(value: Option<&Value>, aliases: &mut Vec<String>) {
    match value {
        Some(Value::String(alias)) => aliases.push(alias.clone()),
        Some(Value::Array(values)) => {
            for value in values {
                collect_alias_values(Some(value), aliases);
            }
        }
        _ => {}
    }
}

fn domain_variants(domains: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for domain in domains {
        let Some(normalized) = normalize_domain(domain) else {
            continue;
        };
        out.push(normalized.clone());
        if let Some(root) = registrable_domain(&normalized) {
            out.push(root);
        }
    }
    clean_dedup(out)
}

fn normalize_domain(domain: &str) -> Option<String> {
    let trimmed = domain.trim().trim_end_matches('.').to_ascii_lowercase();
    let without_scheme = trimmed
        .split_once("://")
        .map(|(_, rest)| rest)
        .unwrap_or(trimmed.as_str());
    let host = without_scheme
        .split(['/', '?', '#'])
        .next()
        .unwrap_or("")
        .trim()
        .trim_start_matches("*.");
    if host.is_empty() || !host.contains('.') {
        None
    } else {
        Some(host.to_string())
    }
}

fn registrable_domain(domain: &str) -> Option<String> {
    let parts: Vec<&str> = domain.split('.').filter(|part| !part.is_empty()).collect();
    if parts.len() < 2 {
        return None;
    }

    let root_len = if parts.len() >= 3
        && parts.last().is_some_and(|part| part.len() == 2)
        && is_common_second_level_tld(parts[parts.len() - 2])
    {
        3
    } else {
        2
    };

    if parts.len() < root_len {
        None
    } else {
        Some(parts[parts.len() - root_len..].join("."))
    }
}

fn is_common_second_level_tld(part: &str) -> bool {
    matches!(
        part,
        "co" | "com" | "org" | "net" | "ac" | "gov" | "edu" | "ltd" | "plc"
    )
}

fn infrastructure_ids(domains: &[String]) -> Vec<String> {
    domains
        .iter()
        .filter_map(|domain| normalize_domain(domain))
        .filter(|domain| domain.starts_with("vip") || domain.contains(".vip"))
        .collect()
}

fn clean_dedup(values: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for value in values {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            continue;
        }
        let key = trimmed.to_ascii_lowercase();
        if seen.insert(key) {
            out.push(trimmed.to_string());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn fresh_db() -> Connection {
        let conn = Connection::open_in_memory().expect("open in-memory db");
        crate::migrations::run_migrations(&conn).expect("apply migrations");
        conn
    }

    fn db_view(conn: &Connection) -> &ActionDb {
        ActionDb::from_conn(conn)
    }

    fn insert_account(
        db: &ActionDb,
        id: &str,
        name: &str,
        parent_id: Option<&str>,
        domains: &[&str],
        metadata: Option<&str>,
    ) {
        db.conn_ref()
            .execute(
                "INSERT INTO accounts \
                 (id, name, account_type, parent_id, metadata, updated_at, archived) \
                 VALUES (?1, ?2, 'customer', ?3, ?4, '2026-05-04T00:00:00Z', 0)",
                params![id, name, parent_id, metadata],
            )
            .expect("insert account");

        for domain in domains {
            db.conn_ref()
                .execute(
                    "INSERT INTO account_domains (account_id, domain, source) \
                     VALUES (?1, ?2, 'user')",
                    params![id, domain.to_ascii_lowercase()],
                )
                .expect("insert account domain");
        }
    }

    fn insert_project(db: &ActionDb, id: &str) {
        db.conn_ref()
            .execute(
                "INSERT INTO projects (id, name, updated_at, archived) \
                 VALUES (?1, 'Fixture Project', '2026-05-04T00:00:00Z', 0)",
                params![id],
            )
            .expect("insert project");
    }

    #[test]
    fn extract_returns_ok_for_existing_account_with_clean_subjectref() {
        let conn = fresh_db();
        let db = db_view(&conn);
        insert_account(
            db,
            "acct-target",
            "Target Account",
            None,
            &["target.test", "app.target.test"],
            None,
        );

        let outcome = extract_target_footprint(
            db,
            &SubjectRef::Account("acct-target".into()),
            "account",
            "acct-target",
        )
        .expect("extract");

        let ExtractionOutcome::Ok {
            footprint,
            portfolio_footprints,
        } = outcome
        else {
            panic!("expected ok outcome");
        };

        assert_eq!(footprint.subject, SubjectRef::Account("acct-target".into()));
        assert!(footprint.names.contains(&"Target Account".to_string()));
        assert!(footprint.domains.contains(&"target.test".to_string()));
        assert!(footprint.domains.contains(&"app.target.test".to_string()));
        assert!(portfolio_footprints.is_empty());
    }

    #[test]
    fn extract_returns_skip_target_not_found_for_missing_entity() {
        let conn = fresh_db();
        let db = db_view(&conn);

        let outcome = extract_target_footprint(
            db,
            &SubjectRef::Account("missing".into()),
            "account",
            "missing",
        )
        .expect("extract");

        assert_eq!(
            outcome,
            ExtractionOutcome::SkipExtractorMismatch {
                reason: ExtractionMismatchReason::TargetNotFound
            }
        );
    }

    #[test]
    fn extract_returns_skip_subject_ref_mismatch_for_id_collision() {
        let conn = fresh_db();
        let db = db_view(&conn);
        insert_account(db, "shared-id", "Shared Account", None, &[], None);
        insert_project(db, "shared-id");

        let outcome = extract_target_footprint(
            db,
            &SubjectRef::Account("shared-id".into()),
            "project",
            "shared-id",
        )
        .expect("extract");

        assert_eq!(
            outcome,
            ExtractionOutcome::SkipExtractorMismatch {
                reason: ExtractionMismatchReason::SubjectRefMismatch
            }
        );
    }

    #[test]
    fn extract_returns_skip_subject_ref_mismatch_for_entity_type_collision() {
        let conn = fresh_db();
        let db = db_view(&conn);
        insert_account(db, "acct-target", "Target Account", None, &[], None);

        let outcome = extract_target_footprint(
            db,
            &SubjectRef::Project("acct-target".into()),
            "account",
            "acct-target",
        )
        .expect("extract");

        assert_eq!(
            outcome,
            ExtractionOutcome::SkipExtractorMismatch {
                reason: ExtractionMismatchReason::SubjectRefMismatch
            }
        );
    }

    #[test]
    fn extract_includes_parent_account_in_portfolio_when_present() {
        let conn = fresh_db();
        let db = db_view(&conn);
        insert_account(
            db,
            "acct-parent",
            "Parent Account",
            None,
            &["parent.test"],
            None,
        );
        insert_account(
            db,
            "acct-child",
            "Child Account",
            Some("acct-parent"),
            &["child.test"],
            None,
        );

        let outcome = extract_target_footprint(
            db,
            &SubjectRef::Account("acct-child".into()),
            "account",
            "acct-child",
        )
        .expect("extract");

        let ExtractionOutcome::Ok {
            footprint,
            portfolio_footprints,
        } = outcome
        else {
            panic!("expected ok outcome");
        };

        assert!(footprint
            .related_subjects
            .contains(&SubjectRef::Account("acct-parent".into())));
        assert!(portfolio_footprints.iter().any(|peer| peer.subject
            == SubjectRef::Account("acct-parent".into())
            && peer.domains.contains(&"parent.test".to_string())));
    }

    #[test]
    fn extract_includes_child_accounts_in_portfolio_when_present() {
        let conn = fresh_db();
        let db = db_view(&conn);
        insert_account(
            db,
            "acct-parent",
            "Parent Account",
            None,
            &["parent.test"],
            None,
        );
        insert_account(
            db,
            "acct-child",
            "Child Account",
            Some("acct-parent"),
            &["child.test"],
            None,
        );

        let outcome = extract_target_footprint(
            db,
            &SubjectRef::Account("acct-parent".into()),
            "account",
            "acct-parent",
        )
        .expect("extract");

        let ExtractionOutcome::Ok {
            footprint,
            portfolio_footprints,
        } = outcome
        else {
            panic!("expected ok outcome");
        };

        assert!(footprint
            .related_subjects
            .contains(&SubjectRef::Account("acct-child".into())));
        assert!(portfolio_footprints.iter().any(|peer| peer.subject
            == SubjectRef::Account("acct-child".into())
            && peer.names.contains(&"Child Account".to_string())));
    }

    #[test]
    fn extract_includes_aliases_when_present() {
        let conn = fresh_db();
        let db = db_view(&conn);
        insert_account(
            db,
            "acct-target",
            "Target Account",
            None,
            &["target.test"],
            Some(r#"{"aliases":["Target Co"],"dba":"Target Labs"}"#),
        );

        let outcome = extract_target_footprint(
            db,
            &SubjectRef::Account("acct-target".into()),
            "account",
            "acct-target",
        )
        .expect("extract");

        let ExtractionOutcome::Ok { footprint, .. } = outcome else {
            panic!("expected ok outcome");
        };

        assert!(footprint.allowed_aliases.contains(&"Target Co".to_string()));
        assert!(footprint
            .allowed_aliases
            .contains(&"Target Labs".to_string()));
        assert!(footprint.names.contains(&"Target Co".to_string()));
    }

    #[test]
    fn extract_propagates_db_error_for_genuine_failure_not_as_skip() {
        let conn = Connection::open_in_memory().expect("open in-memory db");
        let db = db_view(&conn);

        let err = extract_target_footprint(
            db,
            &SubjectRef::Account("acct-target".into()),
            "account",
            "acct-target",
        )
        .expect_err("schema error must propagate");

        assert!(matches!(err, ExtractionError::Db(DbError::Sqlite(_))));
    }
}
