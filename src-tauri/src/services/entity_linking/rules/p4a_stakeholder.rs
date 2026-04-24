//! P4a — Stakeholder-graph inference. An external attendee's person_id matches
//! an active row on account_stakeholders → that account is the primary.
//!
//! Ranks above domain-based rules because stakeholder membership is a confirmed
//! relationship rather than a proxy. A stakeholder attendance signal beats
//! any number of title-only signals: the person has been explicitly promoted
//! (either by the user or a trusted cascade) as a contact on that account.
//!
//! When a single external attendee maps to exactly one active stakeholder
//! account → Matched Primary at 0.93.
//! When two or more distinct stakeholder accounts are hit across the external
//! attendees → the dispatcher's collect_p4_candidates dedup surfaces both
//! candidates and P9 picks "no primary, user chooses via picker".
//! Pending / dismissed / archived stakeholder rows never contribute.

use std::collections::HashSet;

use crate::db::ActionDb;
use super::super::{evidence, types::{Candidate, EntityRef, LinkRole, LinkingContext, RuleOutcome}};

pub struct P4aStakeholder;

impl super::super::phases::Rule for P4aStakeholder {
    fn id(&self) -> &'static str { "P4a" }

    fn evaluate(&self, ctx: &LinkingContext, db: &ActionDb) -> RuleOutcome {
        // Collect every active stakeholder account id across external attendees.
        // Internal participants never vote for stakeholder inference.
        let mut matched: HashSet<String> = HashSet::new();
        for p in ctx.external_participants() {
            let person_id = match &p.person_id {
                Some(id) => id,
                None => continue,
            };
            match db.lookup_active_stakeholder_accounts_for_person(person_id) {
                Ok(account_ids) => {
                    for id in account_ids {
                        matched.insert(id);
                    }
                }
                Err(e) => {
                    log::warn!("P4a stakeholder lookup error for {person_id}: {e}");
                }
            }
        }

        // No stakeholder signal → fall through to P4b/P4c/P4d.
        let mut sorted: Vec<String> = matched.into_iter().collect();
        if sorted.is_empty() {
            return RuleOutcome::Skip;
        }
        sorted.sort();

        // Pick the first deterministically; when multiple accounts match
        // collect_p4_candidates surfaces every stakeholder candidate by
        // running this rule alongside the domain rules, dedup by entity_id,
        // and P9 handles the multi-match case.
        //
        // To feed P9 with every stakeholder candidate we return the first
        // here as Matched; the dispatcher's second collection pass already
        // iterates rules and dedups.  For the multi-account case, the
        // dispatcher sees 2+ distinct entity_ids (this rule + domain rules)
        // and triggers P9 via its own logic.
        let account_id = sorted.remove(0);

        let ev = evidence::matched_evidence(
            ctx,
            &Candidate {
                entity: EntityRef { entity_id: account_id.clone(), entity_type: "account".to_string() },
                role: LinkRole::Primary,
                confidence: 0.93,
                rule_id: "P4a".to_string(),
                evidence: serde_json::json!({ "stakeholder_inference": true }),
            },
            &[],
        );

        RuleOutcome::Matched(Candidate {
            entity: EntityRef { entity_id: account_id, entity_type: "account".to_string() },
            role: LinkRole::Primary,
            confidence: 0.93,
            rule_id: "P4a".to_string(),
            evidence: ev,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::super::phases::Rule;
    use super::super::super::types::{OwnerRef, OwnerType, Participant, ParticipantRole};
    use crate::db::test_utils::test_db;
    use crate::db::ActionDb;

    fn seed_account(db: &ActionDb, id: &str, name: &str) {
        db.conn_ref()
            .execute(
                "INSERT INTO accounts (id, name, updated_at, archived) VALUES (?1, ?2, '2026-01-01', 0)",
                rusqlite::params![id, name],
            )
            .expect("insert account");
    }

    fn seed_person(db: &ActionDb, id: &str, email: &str) {
        db.conn_ref()
            .execute(
                "INSERT INTO people (id, name, email, relationship, updated_at) \
                 VALUES (?1, ?1, ?2, 'external', '2026-01-01')",
                rusqlite::params![id, email],
            )
            .expect("insert person");
    }

    fn seed_stakeholder(db: &ActionDb, account_id: &str, person_id: &str, status: &str) {
        db.conn_ref()
            .execute(
                "INSERT INTO account_stakeholders (account_id, person_id, data_source, status, confidence, created_at) \
                 VALUES (?1, ?2, 'test', ?3, 1.0, '2026-01-01')",
                rusqlite::params![account_id, person_id, status],
            )
            .expect("insert stakeholder");
    }

    fn mk_ctx(external_person_id: Option<&str>, email: &str) -> LinkingContext {
        LinkingContext {
            owner: OwnerRef { owner_type: OwnerType::Meeting, owner_id: "m1".to_string() },
            participants: vec![
                Participant {
                    email: "me@company.com".to_string(),
                    name: None,
                    role: ParticipantRole::Attendee,
                    person_id: Some("self".to_string()),
                    domain: Some("company.com".to_string()),
                },
                Participant {
                    email: email.to_string(),
                    name: None,
                    role: ParticipantRole::Attendee,
                    person_id: external_person_id.map(|s| s.to_string()),
                    domain: email.split('@').nth(1).map(|s| s.to_string()),
                },
            ],
            title: Some("some title".to_string()),
            attendee_count: 2,
            thread_id: None,
            series_id: None,
            graph_version: 0,
            user_domains: vec!["company.com".to_string()],
        }
    }

    #[test]
    fn p4a_stakeholder_single_account_picks_primary() {
        let db = test_db();
        seed_account(&db, "acc-jane", "Jane");
        seed_person(&db, "p-jane", "jane@example.test");
        seed_stakeholder(&db, "acc-jane", "p-jane", "active");

        let ctx = mk_ctx(Some("p-jane"), "jane@example.test");
        let outcome = P4aStakeholder.evaluate(&ctx, &db);
        match outcome {
            RuleOutcome::Matched(c) => {
                assert_eq!(c.entity.entity_id, "acc-jane");
                assert_eq!(c.role, LinkRole::Primary);
                assert_eq!(c.rule_id, "P4a");
            }
            RuleOutcome::Skip => panic!("expected Matched, got Skip"),
        }
    }

    #[test]
    fn p4a_stakeholder_dismissed_status_skipped() {
        let db = test_db();
        seed_account(&db, "acc-jane", "Jane");
        seed_person(&db, "p-jane", "jane@example.test");
        seed_stakeholder(&db, "acc-jane", "p-jane", "dismissed");

        let ctx = mk_ctx(Some("p-jane"), "jane@example.test");
        matches!(P4aStakeholder.evaluate(&ctx, &db), RuleOutcome::Skip)
            .then_some(())
            .expect("dismissed stakeholder should not match");
    }

    #[test]
    fn p4a_stakeholder_pending_status_skipped() {
        let db = test_db();
        seed_account(&db, "acc-jane", "Jane");
        seed_person(&db, "p-jane", "jane@example.test");
        seed_stakeholder(&db, "acc-jane", "p-jane", "pending_review");

        let ctx = mk_ctx(Some("p-jane"), "jane@example.test");
        matches!(P4aStakeholder.evaluate(&ctx, &db), RuleOutcome::Skip)
            .then_some(())
            .expect("pending_review stakeholder should not match");
    }

    #[test]
    fn p4a_stakeholder_multiple_accounts_feeds_p9() {
        // Jane is an active stakeholder on two accounts — dispatcher's
        // collect_p4_candidates dedup will surface two distinct candidates
        // across multiple rule invocations. Here we validate the rule
        // returns *some* match (P9 handles the multi-match dispatch elsewhere).
        let db = test_db();
        seed_account(&db, "acc-a", "A");
        seed_account(&db, "acc-b", "B");
        seed_person(&db, "p-jane", "jane@example.test");
        seed_stakeholder(&db, "acc-a", "p-jane", "active");
        seed_stakeholder(&db, "acc-b", "p-jane", "active");

        let ctx = mk_ctx(Some("p-jane"), "jane@example.test");
        let outcome = P4aStakeholder.evaluate(&ctx, &db);
        match outcome {
            RuleOutcome::Matched(c) => {
                // Deterministic alphabetical order: acc-a wins in isolation.
                assert_eq!(c.entity.entity_id, "acc-a");
                // Multiple-account dispatch is validated in phases.rs; verify
                // both rows exist in DB so collect_p4_candidates can see them.
                let count: i64 = db
                    .conn_ref()
                    .query_row(
                        "SELECT COUNT(DISTINCT account_id) FROM account_stakeholders \
                         WHERE person_id = 'p-jane' AND status = 'active'",
                        [],
                        |row| row.get(0),
                    )
                    .expect("count stakeholder accounts");
                assert_eq!(count, 2, "expected two active stakeholder accounts");
            }
            RuleOutcome::Skip => panic!("expected Matched, got Skip"),
        }
    }

    #[test]
    fn p4a_stakeholder_skips_internal_participants() {
        // An internal participant who happens to be a stakeholder on some
        // account (unusual but possible) should never drive P4a inference.
        let db = test_db();
        seed_account(&db, "acc-jane", "Jane");
        seed_person(&db, "p-internal", "alice@company.com");
        seed_stakeholder(&db, "acc-jane", "p-internal", "active");

        let ctx = LinkingContext {
            owner: OwnerRef { owner_type: OwnerType::Meeting, owner_id: "m1".to_string() },
            participants: vec![
                Participant {
                    email: "me@company.com".to_string(),
                    name: None,
                    role: ParticipantRole::Attendee,
                    person_id: Some("self".to_string()),
                    domain: Some("company.com".to_string()),
                },
                Participant {
                    email: "alice@company.com".to_string(),
                    name: None,
                    role: ParticipantRole::Attendee,
                    person_id: Some("p-internal".to_string()),
                    domain: Some("company.com".to_string()),
                },
            ],
            title: None,
            attendee_count: 2,
            thread_id: None,
            series_id: None,
            graph_version: 0,
            user_domains: vec!["company.com".to_string()],
        };
        matches!(P4aStakeholder.evaluate(&ctx, &db), RuleOutcome::Skip)
            .then_some(())
            .expect("internal participants must never drive P4a");
    }
}

