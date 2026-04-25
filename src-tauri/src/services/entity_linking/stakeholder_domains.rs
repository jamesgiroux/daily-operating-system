//! Stakeholder-derived domain backfill.
//!
//! Scans `account_stakeholders` → `people` to extract each active external
//! stakeholder's email domain and merges into `account_domains`. Idempotent.
//! Runs at every stakeholder-graph touch-point AND on boot.
//!
//! Production DB audit (2026-04-24) discovered 28/30 active customer accounts
//! had ZERO `account_domains` rows even though their stakeholder graphs were
//! populated — DOS-258's domain-based P4 rules couldn't fire. This module
//! closes the loop by deriving domains from `account_stakeholders` at every
//! mutation path (confirm, cascade c3, calendar-poll cascade epilogue,
//! manual overrides) plus a boot sweep for legacy data.

use std::collections::{BTreeSet, HashMap};

use crate::db::ActionDb;
use crate::google_api::classify::PERSONAL_EMAIL_DOMAINS;

/// Bot / integration hosts that should NEVER be registered as a customer
/// account's domain.
///
/// Extend with any automated/integration host whose email addresses should
/// NOT be treated as a real customer's domain. Match is exact on the
/// rightmost `@...` host portion (case-insensitive) — so to filter a whole
/// subdomain family prefer the narrowest safe host (e.g. `bot.us.zoom.us`
/// rather than `zoom.us`, because real Zoom-employee emails live on the
/// parent domain).
pub const BOT_EMAIL_HOSTS: &[&str] = &[
    // Gong meeting-assistant bots (e.g. wordpress-test@assistant.gong.io).
    "assistant.gong.io",
    // Google calendar group + resource addresses.
    "group.calendar.google.com",
    "resource.calendar.google.com",
    // Calendly booking bots.
    "calendly.com",
    // Zoom meeting bots. NOTE: keep this at the `bot.us.zoom.us`
    // subdomain — real `*.zoom.us` user emails exist.
    "bot.us.zoom.us",
];

/// Extract and filter the domain of an external stakeholder email.
///
/// Returns `None` when the domain is a user domain, a personal-email host,
/// or a known bot host. Lowercased on the way in.
fn filter_domain(email: &str, user_domains: &[String]) -> Option<String> {
    let (_, domain) = email.rsplit_once('@')?;
    let domain = domain.trim().to_lowercase();
    if domain.is_empty() {
        return None;
    }
    if user_domains.iter().any(|d| d.eq_ignore_ascii_case(&domain)) {
        return None;
    }
    if PERSONAL_EMAIL_DOMAINS
        .iter()
        .any(|d| d.eq_ignore_ascii_case(&domain))
    {
        return None;
    }
    if BOT_EMAIL_HOSTS
        .iter()
        .any(|d| d.eq_ignore_ascii_case(&domain))
    {
        return None;
    }
    Some(domain)
}

/// Backfill domains for a single account from its active stakeholders.
///
/// Returns the count of NEW domains registered (not the total — the function
/// is idempotent).
///
/// For each newly-inserted domain a `account_domains_updated` signal is
/// emitted with `source="stakeholder_inference"`. No signal fires when the
/// merge is a no-op.
pub fn backfill_domains_for_account(
    db: &ActionDb,
    account_id: &str,
    user_domains: &[String],
) -> Result<usize, String> {
    let emails = db
        .get_active_external_stakeholder_emails(account_id)
        .map_err(|e| format!("load stakeholder emails for {account_id}: {e}"))?;

    // Dedupe + filter into a sorted set so behavior is deterministic.
    let derived: BTreeSet<String> = emails
        .iter()
        .filter_map(|e| filter_domain(e, user_domains))
        .collect();
    if derived.is_empty() {
        return Ok(0);
    }

    let existing: BTreeSet<String> = db
        .get_account_domains(account_id)
        .map_err(|e| format!("load existing domains for {account_id}: {e}"))?
        .into_iter()
        .map(|d| d.to_lowercase())
        .collect();

    let new_domains: Vec<String> = derived.difference(&existing).cloned().collect();
    if new_domains.is_empty() {
        return Ok(0);
    }

    db.merge_account_domains(account_id, &new_domains)
        .map_err(|e| format!("merge_account_domains for {account_id}: {e}"))?;

    for domain in &new_domains {
        let _ = crate::signals::bus::emit_signal(
            db,
            "account",
            account_id,
            "account_domains_updated",
            "stakeholder_inference",
            Some(domain),
            0.9,
        );
    }

    log::debug!(
        "stakeholder_domains: registered {} new domains for account {}",
        new_domains.len(),
        account_id
    );

    Ok(new_domains.len())
}

/// Batch version — iterates all non-archived customer/partner accounts.
/// Returns `(accounts_touched, total_new_domains)` where
/// `accounts_touched` counts accounts that gained at least one new domain.
pub fn backfill_domains_for_all_accounts(
    db: &ActionDb,
    user_domains: &[String],
) -> Result<(usize, usize), String> {
    let rows = db
        .get_all_active_external_stakeholder_emails()
        .map_err(|e| format!("load all stakeholder emails: {e}"))?;

    // Group emails by account_id — one pass, in memory. Production has ~30
    // active customer accounts × avg <25 stakeholders = O(hundreds) rows.
    let mut by_account: HashMap<String, Vec<String>> = HashMap::new();
    for (account_id, email) in rows {
        by_account.entry(account_id).or_default().push(email);
    }

    let mut accounts_touched = 0usize;
    let mut total_new = 0usize;

    for account_id in by_account.keys() {
        // Delegate to the single-account path so the filter + signal logic
        // lives in one place.
        match backfill_domains_for_account(db, account_id, user_domains) {
            Ok(n) if n > 0 => {
                accounts_touched += 1;
                total_new += n;
            }
            Ok(_) => {}
            Err(e) => {
                log::warn!(
                    "stakeholder_domains: backfill failed for account {account_id}: {e}"
                );
            }
        }
    }

    Ok((accounts_touched, total_new))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::ActionDb;
    use crate::db::types::{DbAccount, DbPerson};
    use chrono::Utc;
    use rusqlite::params;

    fn seed_account(db: &ActionDb, id: &str, account_type: crate::db::AccountType) {
        let now = Utc::now().to_rfc3339();
        let account = DbAccount {
            id: id.to_string(),
            name: id.to_string(),
            lifecycle: None,
            arr: None,
            health: None,
            contract_start: None,
            contract_end: None,
            nps: None,
            tracker_path: None,
            parent_id: None,
            account_type,
            updated_at: now,
            archived: false,
            keywords: None,
            keywords_extracted_at: None,
            metadata: None,
            ..Default::default()
        };
        db.upsert_account(&account).expect("upsert account");
    }

    fn seed_person(db: &ActionDb, id: &str, email: &str, relationship: &str) {
        let now = Utc::now().to_rfc3339();
        let person = DbPerson {
            id: id.to_string(),
            email: email.to_string(),
            name: id.to_string(),
            organization: None,
            role: None,
            relationship: relationship.to_string(),
            notes: None,
            tracker_path: None,
            last_seen: None,
            first_seen: Some(now.clone()),
            meeting_count: 0,
            updated_at: now,
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
        };
        db.upsert_person(&person).expect("upsert person");
    }

    fn link_stakeholder(db: &ActionDb, account_id: &str, person_id: &str, status: &str) {
        db.conn_ref()
            .execute(
                "INSERT OR REPLACE INTO account_stakeholders \
                 (account_id, person_id, data_source, confidence, status, created_at) \
                 VALUES (?1, ?2, 'test', 1.0, ?3, ?4)",
                params![account_id, person_id, status, Utc::now().to_rfc3339()],
            )
            .expect("insert stakeholder");
    }

    fn count_domain_signals(db: &ActionDb, account_id: &str) -> i64 {
        db.conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM signal_events \
                 WHERE entity_type = 'account' AND entity_id = ?1 \
                   AND signal_type = 'account_domains_updated' \
                   AND source = 'stakeholder_inference'",
                params![account_id],
                |row| row.get::<_, i64>(0),
            )
            .unwrap_or(0)
    }

    #[test]
    fn backfill_single_account_registers_external_domain() {
        let db = crate::db::test_utils::test_db();
        seed_account(&db, "acme", crate::db::AccountType::Customer);
        seed_person(&db, "p1", "alice@customer.com", "external");
        link_stakeholder(&db, "acme", "p1", "active");

        let new = backfill_domains_for_account(&db, "acme", &[]).unwrap();
        assert_eq!(new, 1);

        let domains = db.get_account_domains("acme").unwrap();
        assert_eq!(domains, vec!["customer.com".to_string()]);
    }

    #[test]
    fn backfill_skips_internal_relationship() {
        let db = crate::db::test_utils::test_db();
        seed_account(&db, "acme", crate::db::AccountType::Customer);
        seed_person(&db, "p1", "bob@customer.com", "internal");
        link_stakeholder(&db, "acme", "p1", "active");

        let new = backfill_domains_for_account(&db, "acme", &[]).unwrap();
        assert_eq!(new, 0);
        assert!(db.get_account_domains("acme").unwrap().is_empty());
    }

    #[test]
    fn backfill_skips_personal_email() {
        let db = crate::db::test_utils::test_db();
        seed_account(&db, "acme", crate::db::AccountType::Customer);
        seed_person(&db, "p1", "carol@gmail.com", "external");
        link_stakeholder(&db, "acme", "p1", "active");

        let new = backfill_domains_for_account(&db, "acme", &[]).unwrap();
        assert_eq!(new, 0);
        assert!(db.get_account_domains("acme").unwrap().is_empty());
    }

    #[test]
    fn backfill_skips_bot_hosts() {
        let db = crate::db::test_utils::test_db();
        seed_account(&db, "acme", crate::db::AccountType::Customer);
        seed_person(&db, "p1", "bot@assistant.gong.io", "external");
        seed_person(&db, "p2", "room@group.calendar.google.com", "external");
        seed_person(&db, "p3", "slot@calendly.com", "external");
        seed_person(&db, "p4", "meet@bot.us.zoom.us", "external");
        link_stakeholder(&db, "acme", "p1", "active");
        link_stakeholder(&db, "acme", "p2", "active");
        link_stakeholder(&db, "acme", "p3", "active");
        link_stakeholder(&db, "acme", "p4", "active");

        let new = backfill_domains_for_account(&db, "acme", &[]).unwrap();
        assert_eq!(new, 0);
        assert!(db.get_account_domains("acme").unwrap().is_empty());
    }

    #[test]
    fn backfill_skips_user_domains() {
        let db = crate::db::test_utils::test_db();
        seed_account(&db, "acme", crate::db::AccountType::Customer);
        seed_person(&db, "p1", "james@a8c.com", "external");
        link_stakeholder(&db, "acme", "p1", "active");

        let user_domains = vec!["a8c.com".to_string()];
        let new = backfill_domains_for_account(&db, "acme", &user_domains).unwrap();
        assert_eq!(new, 0);
        assert!(db.get_account_domains("acme").unwrap().is_empty());
    }

    #[test]
    fn backfill_skips_dismissed_and_pending_status() {
        let db = crate::db::test_utils::test_db();
        seed_account(&db, "acme", crate::db::AccountType::Customer);
        seed_person(&db, "p1", "alice@customer.com", "external");
        seed_person(&db, "p2", "bob@dismissed.com", "external");
        link_stakeholder(&db, "acme", "p1", "pending_review");
        link_stakeholder(&db, "acme", "p2", "dismissed");

        let new = backfill_domains_for_account(&db, "acme", &[]).unwrap();
        assert_eq!(new, 0);
        assert!(db.get_account_domains("acme").unwrap().is_empty());
    }

    #[test]
    fn backfill_is_idempotent_no_signal_on_rerun() {
        let db = crate::db::test_utils::test_db();
        seed_account(&db, "acme", crate::db::AccountType::Customer);
        seed_person(&db, "p1", "alice@customer.com", "external");
        link_stakeholder(&db, "acme", "p1", "active");

        assert_eq!(backfill_domains_for_account(&db, "acme", &[]).unwrap(), 1);
        assert_eq!(backfill_domains_for_account(&db, "acme", &[]).unwrap(), 0);
        assert_eq!(count_domain_signals(&db, "acme"), 1);
    }

    #[test]
    fn backfill_emits_signal_only_for_new_domains() {
        let db = crate::db::test_utils::test_db();
        seed_account(&db, "acme", crate::db::AccountType::Customer);
        seed_person(&db, "p1", "alice@customer.com", "external");
        seed_person(&db, "p2", "bob@customer.com", "external");
        link_stakeholder(&db, "acme", "p1", "active");
        link_stakeholder(&db, "acme", "p2", "active");

        // First pass registers customer.com once (two stakeholders, same domain).
        assert_eq!(backfill_domains_for_account(&db, "acme", &[]).unwrap(), 1);
        assert_eq!(count_domain_signals(&db, "acme"), 1);

        // Add a third stakeholder on a new domain → one more signal only.
        seed_person(&db, "p3", "carol@subsidiary.com", "external");
        link_stakeholder(&db, "acme", "p3", "active");

        assert_eq!(backfill_domains_for_account(&db, "acme", &[]).unwrap(), 1);
        assert_eq!(count_domain_signals(&db, "acme"), 2);
    }

    #[test]
    fn backfill_batch_covers_all_customer_accounts() {
        let db = crate::db::test_utils::test_db();
        seed_account(&db, "acme", crate::db::AccountType::Customer);
        seed_account(&db, "beta", crate::db::AccountType::Customer);
        seed_account(&db, "partnerco", crate::db::AccountType::Partner);
        seed_account(&db, "myco", crate::db::AccountType::Internal);
        seed_person(&db, "p1", "a@acme-ext.com", "external");
        seed_person(&db, "p2", "b@beta-ext.com", "external");
        seed_person(&db, "p3", "c@partner-ext.com", "external");
        seed_person(&db, "p4", "d@myco-ext.com", "external");
        link_stakeholder(&db, "acme", "p1", "active");
        link_stakeholder(&db, "beta", "p2", "active");
        link_stakeholder(&db, "partnerco", "p3", "active");
        link_stakeholder(&db, "myco", "p4", "active");

        let (touched, new) = backfill_domains_for_all_accounts(&db, &[]).unwrap();
        assert_eq!(touched, 3, "customer + partner accounts only");
        assert_eq!(new, 3);
        assert!(db.get_account_domains("myco").unwrap().is_empty());
        assert_eq!(db.get_account_domains("acme").unwrap(), vec!["acme-ext.com"]);
        assert_eq!(db.get_account_domains("beta").unwrap(), vec!["beta-ext.com"]);
        assert_eq!(
            db.get_account_domains("partnerco").unwrap(),
            vec!["partner-ext.com"]
        );
    }

    #[test]
    fn backfill_batch_counts_accounts_and_domains_correctly() {
        let db = crate::db::test_utils::test_db();
        seed_account(&db, "acme", crate::db::AccountType::Customer);
        seed_account(&db, "beta", crate::db::AccountType::Customer);
        // acme: 2 real domains + 1 bot (excluded).
        seed_person(&db, "p1", "a@acme-ext.com", "external");
        seed_person(&db, "p2", "b@subsidiary.com", "external");
        seed_person(&db, "p3", "gong@assistant.gong.io", "external");
        link_stakeholder(&db, "acme", "p1", "active");
        link_stakeholder(&db, "acme", "p2", "active");
        link_stakeholder(&db, "acme", "p3", "active");
        // beta: 1 real.
        seed_person(&db, "p4", "d@beta-ext.com", "external");
        link_stakeholder(&db, "beta", "p4", "active");

        let (touched, new) = backfill_domains_for_all_accounts(&db, &[]).unwrap();
        assert_eq!(touched, 2);
        assert_eq!(new, 3);
    }
}
