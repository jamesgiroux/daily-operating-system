//! P4a — Stakeholder-graph inference. External attendee person_ids matching
//! active rows on account_stakeholders provide account evidence.
//!
//! Ranks above domain-based rules because stakeholder membership is a confirmed
//! relationship rather than a proxy. A stakeholder attendance signal beats
//! any number of title-only signals: the person has been explicitly promoted
//! (either by the user or a trusted cascade) as a contact on that account.
//!
//! When external attendees map to exactly one active stakeholder account,
//! the rule returns a primary at 0.93. When two or more distinct stakeholder
//! accounts are hit, the dispatcher surfaces every candidate and P9 picks
//! "no primary, user chooses via picker".
//! Pending / dismissed / archived stakeholder rows never contribute.

use std::collections::HashSet;

use super::super::{
    evidence,
    types::{Candidate, EntityRef, LinkRole, LinkingContext, RuleOutcome},
};
use crate::db::ActionDb;

pub struct P4aStakeholder;

impl P4aStakeholder {
    pub fn collect_candidates(ctx: &LinkingContext, db: &ActionDb) -> Vec<Candidate> {
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

        let mut sorted: Vec<String> = matched.into_iter().collect();
        sorted.sort();

        sorted
            .into_iter()
            .map(|account_id| {
                let candidate = Candidate {
                    entity: EntityRef {
                        entity_id: account_id.clone(),
                        entity_type: "account".to_string(),
                    },
                    role: LinkRole::Primary,
                    confidence: 0.93,
                    rule_id: "P4a".to_string(),
                    evidence: serde_json::json!({ "stakeholder_inference": true }),
                };
                let ev = evidence::matched_evidence(ctx, &candidate, &[]);
                Candidate {
                    evidence: ev,
                    ..candidate
                }
            })
            .collect()
    }
}

impl super::super::phases::Rule for P4aStakeholder {
    fn id(&self) -> &'static str {
        "P4a"
    }

    fn evaluate(&self, ctx: &LinkingContext, db: &ActionDb) -> RuleOutcome {
        let candidates = Self::collect_candidates(ctx, db);
        if candidates.len() != 1 {
            return RuleOutcome::Skip;
        }

        RuleOutcome::Matched(candidates.into_iter().next().expect("one P4a candidate"))
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
        // The dispatcher should see both active stakeholder accounts and route
        // ambiguity to P9 instead of letting isolated rule evaluation pick one.
        let db = test_db();
        seed_account(&db, "acc-a", "A");
        seed_account(&db, "acc-b", "B");
        seed_person(&db, "p-jane", "jane@example.test");
        seed_stakeholder(&db, "acc-a", "p-jane", "active");
        seed_stakeholder(&db, "acc-b", "p-jane", "active");

        let ctx = mk_ctx(Some("p-jane"), "jane@example.test");
        let outcome = P4aStakeholder.evaluate(&ctx, &db);
        assert!(matches!(outcome, RuleOutcome::Skip));

        let candidates = P4aStakeholder::collect_candidates(&ctx, &db);
        let ids: Vec<_> = candidates.iter().map(|c| c.entity.entity_id.as_str()).collect();
        assert_eq!(ids, vec!["acc-a", "acc-b"]);
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
