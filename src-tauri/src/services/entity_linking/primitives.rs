//! Thin wrappers around existing DB primitives used throughout the linking engine.
//!
//! These are the only DB functions the engine calls for person and account
//! lookup. No logic is duplicated here — the implementations live in db/people.rs
//! and db/accounts.rs. Callers import from this module so the canonical paths
//! are in one place.

use crate::db::people::PersonResolution;
use crate::db::{ActionDb, DbAccount, DbError, DbPerson};

pub fn find_or_create_person(
    db: &ActionDb,
    email: Option<&str>,
    name: &str,
    organization: Option<&str>,
    relationship: &str,
    user_domains: &[String],
) -> Result<PersonResolution, DbError> {
    db.find_or_create_person(email, name, organization, relationship, user_domains)
}

pub fn get_person_by_email_or_alias(
    db: &ActionDb,
    email: &str,
) -> Result<Option<DbPerson>, DbError> {
    db.get_person_by_email_or_alias(email)
}

pub fn lookup_account_candidates_by_domain(
    db: &ActionDb,
    domain: &str,
) -> Result<Vec<DbAccount>, DbError> {
    db.lookup_account_candidates_by_domain(domain)
}

/// Extract the domain from an email address (everything after the last '@').
/// Returns None for malformed addresses.
pub fn domain_from_email(email: &str) -> Option<String> {
    email.rsplit_once('@').map(|(_, d)| d.to_lowercase())
}
