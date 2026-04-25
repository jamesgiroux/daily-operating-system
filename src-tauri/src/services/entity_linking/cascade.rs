//! Phase 4 cascade — C1 through C5.

use crate::db::ActionDb;

use super::types::{Candidate, EntityRef, LinkTier, LinkOutcome, LinkingContext, OwnerType};

/// Personal-email providers whose domains must never be merged into
/// account_domains. A stakeholder confirmation for jane@gmail.com doesn't
/// prove gmail.com is Jane-Corp's domain.
const PERSONAL_EMAIL_DOMAINS: &[&str] = &[
    "gmail.com",
    "googlemail.com",
    "yahoo.com",
    "outlook.com",
    "hotmail.com",
    "live.com",
    "icloud.com",
    "me.com",
    "proton.me",
    "protonmail.com",
];

/// Shared self-healing helper: when a stakeholder is confirmed on an account,
/// merge the person's email domain into `account_domains` provided the domain
/// is external (not user-owned, not a personal-email provider). Idempotent —
/// re-runs for an already-registered domain emit no new signal.
///
/// Returns true when a new domain was actually inserted (so callers can emit
/// a one-shot `account_domains_updated` audit signal).
pub(crate) fn backfill_account_domain_from_person(
    db: &ActionDb,
    account_id: &str,
    person_email: &str,
    user_domains: &[String],
) -> bool {
    let domain = match person_email.rsplit_once('@') {
        Some((_, d)) => d.to_lowercase(),
        None => return false,
    };
    if domain.is_empty() {
        return false;
    }
    // Skip user's own domains — these should never land in account_domains.
    if user_domains
        .iter()
        .any(|ud| ud.eq_ignore_ascii_case(&domain))
    {
        return false;
    }
    // Skip personal-email providers.
    if PERSONAL_EMAIL_DOMAINS.iter().any(|p| *p == domain) {
        return false;
    }

    // Already registered? Skip to avoid double-emitting the audit signal
    // (merge_account_domains itself is idempotent via INSERT OR IGNORE,
    // but emit_signal is not — we gate the signal here).
    let already: Option<i64> = db
        .conn_ref()
        .query_row(
            "SELECT 1 FROM account_domains WHERE account_id = ?1 AND domain = ?2 LIMIT 1",
            rusqlite::params![account_id, domain],
            |row| row.get(0),
        )
        .ok();
    if already.is_some() {
        return false;
    }

    if let Err(e) = db.merge_account_domains(account_id, std::slice::from_ref(&domain)) {
        log::warn!(
            "backfill_account_domain_from_person: merge failed for {account_id} / {domain}: {e}"
        );
        return false;
    }

    // Emit audit signal mirroring the C6 path.
    let _ = crate::signals::bus::emit_signal(
        db,
        "account",
        account_id,
        "account_domains_updated",
        "stakeholder_confirmation",
        Some(&domain),
        0.9,
    );
    true
}

/// Run all cascade steps and return the final LinkOutcome.
pub fn run_cascade(
    ctx: &LinkingContext,
    primary_candidate: &Option<Candidate>,
    phase3_related: &[Candidate],
    db: &ActionDb,
) -> Result<LinkOutcome, String> {
    let mut related: Vec<EntityRef> = phase3_related
        .iter()
        .map(|c| c.entity.clone())
        .collect();

    let primary_entity = primary_candidate
        .as_ref()
        .map(|c| c.entity.clone());

    // C1 — Related chips already written to DB in phases.rs; just collect for outcome.

    // C2 — Stakeholder review queue (sole post-migration writer to account_stakeholders).
    if let Some(ref primary) = primary_entity {
        if primary.entity_type == "account" {
            let auto_resolved = primary_candidate
                .as_ref()
                .map(|c| !matches!(c.rule_id.as_str(), "P1" | "P3"))
                .unwrap_or(false);

            if auto_resolved {
                c2_suggest_stakeholders(ctx, &primary.entity_id, db);
            } else {
                // C3 — User-set primary: promote domain-matching attendees directly.
                c3_promote_trusted_stakeholders(ctx, &primary.entity_id, db);
            }

            // DOS-258: after any account-primary resolution, sweep the
            // account's stakeholder graph for domains not yet registered.
            // Closes the loop alongside C2/C3 so meeting → primary-account
            // paths self-heal account_domains on every pass.
            if let Err(e) = super::stakeholder_domains::backfill_domains_for_account(
                db,
                &primary.entity_id,
                &ctx.user_domains,
            ) {
                log::warn!(
                    "stakeholder_domains: cascade backfill for {} failed: {e}",
                    primary.entity_id
                );
            }
        }
    }

    // C4 — If primary is a project, surface its account as related.
    if let Some(ref primary) = primary_entity {
        if primary.entity_type == "project" {
            if let Some(account_id) = get_project_account_id(db, &primary.entity_id) {
                let account_ref = EntityRef {
                    entity_id: account_id.clone(),
                    entity_type: "account".to_string(),
                };
                if !related.iter().any(|r| r.entity_id == account_id) {
                    related.push(account_ref);
                    // Write to DB
                    let _ = db.upsert_linked_entity_raw(
                        &crate::db::entity_linking::LinkedEntityRawWrite {
                            owner_type: ctx.owner.owner_type.as_str().to_string(),
                            owner_id: ctx.owner.owner_id.clone(),
                            entity_id: account_id,
                            entity_type: "account".to_string(),
                            role: "related".to_string(),
                            source: "rule:C4".to_string(),
                            rule_id: Some("C4".to_string()),
                            confidence: None,
                            evidence_json: None,
                            graph_version: ctx.graph_version,
                        },
                    );
                }
            }
        }
    }

    // C6 — Meeting-only: backfill attendee domains into account_domains so that
    // newly-linked accounts gain domain knowledge for future matching. Runs for
    // both auto-resolved and user-override primaries (porting the side effect
    // from the deleted background resolver in signals/event_trigger.rs). Email
    // paths are handled separately by the stakeholder queue — see C2.
    if let Some(ref primary) = primary_entity {
        if primary.entity_type == "account" && ctx.owner.owner_type == OwnerType::Meeting {
            c6_backfill_account_domains(ctx, &primary.entity_id, db);
        }
    }

    // C5 — Tier mapping.
    let tier = c5_tier(ctx, &primary_entity);

    let applied_rule = primary_candidate
        .as_ref()
        .map(|c| c.rule_id.clone());

    Ok(LinkOutcome {
        owner: ctx.owner.clone(),
        primary: primary_entity,
        related,
        tier,
        applied_rule,
    })
}

// ---------------------------------------------------------------------------
// C2 helpers
// ---------------------------------------------------------------------------

fn c2_suggest_stakeholders(ctx: &LinkingContext, account_id: &str, db: &ActionDb) {
    // Get account domains to identify domain-matching external participants.
    let account_domains = get_account_domains(db, account_id);

    for p in ctx.external_participants() {
        let p_domain = p
            .email
            .rsplit_once('@')
            .map(|(_, d)| d.to_lowercase());

        let domain_matches = p_domain
            .as_deref()
            .map(|d| account_domains.iter().any(|ad| ad.eq_ignore_ascii_case(d)))
            .unwrap_or(false);

        if !domain_matches {
            continue;
        }

        let person_id = match &p.person_id {
            Some(id) => id,
            None => continue,
        };

        // Skip if already an active/pending stakeholder on this account.
        let already = db
            .is_stakeholder_on_account(account_id, person_id)
            .unwrap_or(false);
        if already {
            continue;
        }

        let data_source = match ctx.owner.owner_type {
            super::types::OwnerType::Meeting => "calendar_attendance",
            super::types::OwnerType::Email | super::types::OwnerType::EmailThread => {
                "email_correspondence"
            }
        };

        let _ = db.suggest_stakeholder_pending(account_id, person_id, data_source, 0.75);
    }
}

// ---------------------------------------------------------------------------
// C6 — Meeting domain backfill
// ---------------------------------------------------------------------------

/// Merge attendee-derived domains into `account_domains` so that newly-linked
/// accounts accumulate domain knowledge for future matching. Ports the side
/// effect from the deleted background resolver (old
/// `signals/event_trigger.rs`): uses `merge_account_domains` (additive) so
/// multiple meetings accumulate rather than clobber, and emits an
/// `account_domains_updated` signal on success for the audit trail.
fn c6_backfill_account_domains(ctx: &LinkingContext, account_id: &str, db: &ActionDb) {
    use crate::google_api::classify::PERSONAL_EMAIL_DOMAINS;

    let mut discovered: Vec<String> = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for p in &ctx.participants {
        let Some(domain) = p.domain.as_deref() else {
            continue;
        };
        let domain = domain.to_lowercase();
        if domain.is_empty() || domain.contains(' ') {
            continue;
        }
        // Filter out user's own domains — matches the old
        // extract_domains_from_attendees semantics. Without this the CSM's
        // domain would attach to every customer account.
        if ctx.user_domains.iter().any(|ud| ud.eq_ignore_ascii_case(&domain)) {
            continue;
        }
        if PERSONAL_EMAIL_DOMAINS.contains(&domain.as_str()) {
            continue;
        }
        if seen.insert(domain.clone()) {
            discovered.push(domain);
        }
    }

    if discovered.is_empty() {
        return;
    }

    match db.merge_account_domains(account_id, &discovered) {
        Ok(()) => {
            log::debug!(
                "C6: stored {} domain(s) for account '{}'",
                discovered.len(),
                account_id
            );
            let _ = crate::signals::bus::emit_signal(
                db,
                "account",
                account_id,
                "account_domains_updated",
                "entity_resolution",
                Some(&format!("{} domains", discovered.len())),
                0.9,
            );
        }
        Err(e) => {
            log::warn!(
                "C6: failed to store domains for account {}: {}",
                account_id,
                e
            );
        }
    }
}

fn c3_promote_trusted_stakeholders(ctx: &LinkingContext, account_id: &str, db: &ActionDb) {
    // User explicitly chose this account — domain-matching attendees become active stakeholders.
    let account_domains = get_account_domains(db, account_id);

    for p in ctx.external_participants() {
        let domain = p
            .email
            .rsplit_once('@')
            .map(|(_, d)| d.to_lowercase());
        let domain_matches = domain
            .as_deref()
            .map(|d| account_domains.iter().any(|ad| ad.eq_ignore_ascii_case(d)))
            .unwrap_or(false);

        if !domain_matches {
            continue;
        }

        let person_id = match &p.person_id {
            Some(id) => id,
            None => continue,
        };

        let already = db
            .is_stakeholder_on_account(account_id, person_id)
            .unwrap_or(false);
        if already {
            continue;
        }

        // Insert directly as active (trusted because user chose the account).
        let _ = db.suggest_stakeholder_pending(account_id, person_id, "user_set_primary", 1.0);
        let _ = db.confirm_stakeholder(account_id, person_id);

        // DOS-258 Tier 3: self-healing — a confirmed stakeholder's external
        // domain is evidence the account owns that domain. Merge it into
        // account_domains so future P4 domain evidence fires without a
        // second manual confirmation.
        let _ = backfill_account_domain_from_person(db, account_id, &p.email, &ctx.user_domains);
    }
}

// ---------------------------------------------------------------------------
// C5 — Tier mapping
// ---------------------------------------------------------------------------

fn c5_tier(ctx: &LinkingContext, primary: &Option<EntityRef>) -> LinkTier {
    match primary {
        None => {
            if ctx.is_one_on_one() {
                LinkTier::Person
            } else {
                LinkTier::Minimal
            }
        }
        Some(e) => match e.entity_type.as_str() {
            "account" => LinkTier::Entity,
            "person" => LinkTier::Person,
            "project" => LinkTier::Entity,
            _ => LinkTier::Minimal,
        },
    }
}

// ---------------------------------------------------------------------------
// DB helpers (private)
// ---------------------------------------------------------------------------

fn get_account_domains(db: &ActionDb, account_id: &str) -> Vec<String> {
    let mut stmt = match db
        .conn_ref()
        .prepare("SELECT domain FROM account_domains WHERE account_id = ?1")
    {
        Ok(s) => s,
        Err(_) => return vec![],
    };
    stmt.query_map(rusqlite::params![account_id], |row| {
        row.get::<_, String>(0)
    })
    .map(|rows| rows.filter_map(|r| r.ok()).collect())
    .unwrap_or_default()
}

fn get_project_account_id(db: &ActionDb, project_id: &str) -> Option<String> {
    db.conn_ref()
        .query_row(
            "SELECT account_id FROM projects WHERE id = ?1 AND account_id IS NOT NULL",
            rusqlite::params![project_id],
            |row| row.get::<_, String>(0),
        )
        .ok()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_utils::test_db;
    use crate::db::{AccountType, DbAccount};
    use crate::services::entity_linking::types::{
        LinkRole, OwnerRef, OwnerType, Participant, ParticipantRole,
    };
    use chrono::Utc;

    fn sample_account(id: &str) -> DbAccount {
        DbAccount {
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
            account_type: AccountType::Customer,
            updated_at: Utc::now().to_rfc3339(),
            archived: false,
            keywords: None,
            keywords_extracted_at: None,
            metadata: None,
            ..Default::default()
        }
    }

    fn make_participant(email: &str, role: ParticipantRole) -> Participant {
        let domain = email.rsplit_once('@').map(|(_, d)| d.to_lowercase());
        Participant {
            email: email.to_lowercase(),
            name: None,
            role,
            person_id: None,
            domain,
        }
    }

    fn meeting_ctx(participants: Vec<Participant>) -> LinkingContext {
        LinkingContext {
            owner: OwnerRef {
                owner_type: OwnerType::Meeting,
                owner_id: "meet-1".to_string(),
            },
            attendee_count: participants.len(),
            participants,
            title: Some("Quarterly review".to_string()),
            thread_id: None,
            series_id: None,
            graph_version: 1,
            user_domains: vec!["internal.test".to_string()],
        }
    }

    fn account_primary(entity_id: &str, rule_id: &str) -> Option<Candidate> {
        Some(Candidate {
            entity: EntityRef {
                entity_id: entity_id.to_string(),
                entity_type: "account".to_string(),
            },
            role: LinkRole::Primary,
            confidence: 0.9,
            rule_id: rule_id.to_string(),
            evidence: serde_json::json!({}),
        })
    }

    #[test]
    fn c6_merges_external_attendee_domains_for_meeting_primary() {
        let db = test_db();
        db.upsert_account(&sample_account("acc-1")).unwrap();

        let ctx = meeting_ctx(vec![
            make_participant("csm@internal.test", ParticipantRole::Attendee),
            make_participant("alice@customer.example", ParticipantRole::Attendee),
            make_participant("bob@customer.example", ParticipantRole::Attendee),
        ]);

        let primary = account_primary("acc-1", "P7");
        run_cascade(&ctx, &primary, &[], &db).expect("cascade");

        let domains = get_account_domains(&db, "acc-1");
        assert!(
            domains.iter().any(|d| d == "customer.example"),
            "expected customer.example to be merged, got {:?}",
            domains
        );
        assert!(
            !domains.iter().any(|d| d == "internal.test"),
            "user_domains must be filtered out, got {:?}",
            domains
        );
        // Idempotence: re-running should not duplicate.
        let len_before = domains.len();
        run_cascade(&ctx, &primary, &[], &db).expect("cascade rerun");
        let domains_after = get_account_domains(&db, "acc-1");
        assert_eq!(domains_after.len(), len_before, "merge must dedupe");
    }

    #[test]
    fn c6_skips_personal_email_providers() {
        let db = test_db();
        db.upsert_account(&sample_account("acc-2")).unwrap();

        let ctx = meeting_ctx(vec![
            make_participant("csm@internal.test", ParticipantRole::Attendee),
            make_participant("someone@gmail.com", ParticipantRole::Attendee),
        ]);

        run_cascade(&ctx, &account_primary("acc-2", "P7"), &[], &db).expect("cascade");

        let domains = get_account_domains(&db, "acc-2");
        assert!(
            !domains.iter().any(|d| d == "gmail.com"),
            "personal providers must be filtered, got {:?}",
            domains
        );
    }

    #[test]
    fn c6_does_not_run_for_person_primary() {
        let db = test_db();
        db.upsert_account(&sample_account("acc-3")).unwrap();

        let ctx = meeting_ctx(vec![
            make_participant("csm@internal.test", ParticipantRole::Attendee),
            make_participant("alice@customer.example", ParticipantRole::Attendee),
        ]);

        let person_primary = Some(Candidate {
            entity: EntityRef {
                entity_id: "person-1".to_string(),
                entity_type: "person".to_string(),
            },
            role: LinkRole::Primary,
            confidence: 0.9,
            rule_id: "P4a".to_string(),
            evidence: serde_json::json!({}),
        });
        run_cascade(&ctx, &person_primary, &[], &db).expect("cascade");

        // No account was the primary, so account_domains must remain empty
        // for any account.
        let domains = get_account_domains(&db, "acc-3");
        assert!(domains.is_empty(), "expected no domains written, got {:?}", domains);
    }

    #[test]
    fn c6_does_not_run_for_email_owner() {
        let db = test_db();
        db.upsert_account(&sample_account("acc-4")).unwrap();

        let mut ctx = meeting_ctx(vec![
            make_participant("csm@internal.test", ParticipantRole::From),
            make_participant("alice@customer.example", ParticipantRole::To),
        ]);
        // Rewrite owner as an email — stakeholder queue handles email paths.
        ctx.owner = OwnerRef {
            owner_type: OwnerType::Email,
            owner_id: "email-1".to_string(),
        };

        run_cascade(&ctx, &account_primary("acc-4", "P7"), &[], &db).expect("cascade");

        let domains = get_account_domains(&db, "acc-4");
        assert!(
            domains.is_empty(),
            "C6 must only run for meeting owners, got {:?}",
            domains
        );
    }
}

#[cfg(test)]
mod backfill_tests {
    use super::*;
    use crate::db::test_utils::test_db;

    fn seed_account(db: &ActionDb, id: &str) {
        db.conn_ref()
            .execute(
                "INSERT INTO accounts (id, name, updated_at, archived) VALUES (?1, ?1, '2026-04-20', 0)",
                rusqlite::params![id],
            )
            .expect("seed account");
    }

    fn count_domains(db: &ActionDb, account_id: &str) -> i64 {
        db.conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM account_domains WHERE account_id = ?1",
                rusqlite::params![account_id],
                |row| row.get(0),
            )
            .expect("count")
    }

    fn count_signal(db: &ActionDb, account_id: &str) -> i64 {
        db.conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM signal_events WHERE entity_id = ?1 \
                 AND signal_type = 'account_domains_updated'",
                rusqlite::params![account_id],
                |row| row.get(0),
            )
            .unwrap_or(0)
    }

    #[test]
    fn confirm_stakeholder_backfills_external_domain() {
        let db = test_db();
        seed_account(&db, "acc-jane");

        let inserted = backfill_account_domain_from_person(
            &db,
            "acc-jane",
            "jane@example.test",
            &["company.com".to_string()],
        );
        assert!(inserted, "expected domain insert");
        assert_eq!(count_domains(&db, "acc-jane"), 1, "example.test should be registered");
        assert_eq!(count_signal(&db, "acc-jane"), 1, "one audit signal expected");
    }

    #[test]
    fn confirm_stakeholder_skips_personal_email() {
        let db = test_db();
        seed_account(&db, "acc-jane");

        let inserted = backfill_account_domain_from_person(
            &db,
            "acc-jane",
            "jane@gmail.com",
            &["company.com".to_string()],
        );
        assert!(!inserted, "personal email must not backfill");
        assert_eq!(count_domains(&db, "acc-jane"), 0, "no domain registered");
    }

    #[test]
    fn c3_promote_backfills_external_domain() {
        // End-to-end style: c3_promote path uses the shared helper, so we
        // exercise the helper with the same arguments c3 would pass for a
        // confirmed stakeholder on a domain-matched external attendee.
        let db = test_db();
        seed_account(&db, "acc-acme");

        // Simulate c3: attendee domain matched, now we backfill.
        let inserted = backfill_account_domain_from_person(
            &db,
            "acc-acme",
            "alice@acme.com",
            &["company.com".to_string()],
        );
        assert!(inserted, "c3 promote path should backfill");
        assert_eq!(count_domains(&db, "acc-acme"), 1);
    }

    #[test]
    fn domain_merge_idempotent_no_double_signal() {
        let db = test_db();
        seed_account(&db, "acc-x");

        // First call: should insert and emit.
        let first = backfill_account_domain_from_person(
            &db,
            "acc-x",
            "alice@acme.com",
            &["company.com".to_string()],
        );
        assert!(first);
        let signals_after_first = count_signal(&db, "acc-x");
        assert_eq!(signals_after_first, 1);

        // Second call: domain already present, no signal should fire.
        let second = backfill_account_domain_from_person(
            &db,
            "acc-x",
            "alice@acme.com",
            &["company.com".to_string()],
        );
        assert!(!second, "re-run must report nothing inserted");
        let signals_after_second = count_signal(&db, "acc-x");
        assert_eq!(signals_after_second, 1, "idempotent — signal must not double-emit");
    }
}
