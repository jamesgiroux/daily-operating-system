//! P4b — Group meeting where ≥2 external participants share exactly one account_of.

use std::collections::HashMap;

use crate::db::ActionDb;
use super::super::{evidence, primitives, types::{Candidate, EntityRef, LinkRole, LinkingContext, RuleOutcome}};

pub struct P4bGroupShared;

impl super::super::phases::Rule for P4bGroupShared {
    fn id(&self) -> &'static str { "P4b" }

    fn evaluate(&self, ctx: &LinkingContext, db: &ActionDb) -> RuleOutcome {
        let external: Vec<_> = ctx.external_participants().collect();
        if external.len() < 2 {
            return RuleOutcome::Skip;
        }

        // Build a frequency map of account_id → count of external participants
        // whose domain maps to that account.
        let mut account_votes: HashMap<String, usize> = HashMap::new();
        for p in &external {
            if let Some(domain) = primitives::domain_from_email(&p.email) {
                if let Ok(accounts) = primitives::lookup_account_candidates_by_domain(db, &domain) {
                    for acct in accounts {
                        *account_votes.entry(acct.id).or_insert(0) += 1;
                    }
                }
            }
        }

        // Exactly one account must have ≥2 votes for this rule to match.
        let top: Vec<_> = account_votes
            .iter()
            .filter(|(_, &v)| v >= 2)
            .collect();

        if top.len() != 1 {
            return RuleOutcome::Skip;
        }

        let (account_id, &vote_count) = top[0];

        let ev = evidence::matched_evidence(
            ctx,
            &Candidate {
                entity: EntityRef { entity_id: account_id.clone(), entity_type: "account".to_string() },
                role: LinkRole::Primary,
                confidence: 0.90,
                rule_id: "P4b".to_string(),
                evidence: serde_json::json!({}),
            },
            &[],
        );
        let _ = vote_count;

        RuleOutcome::Matched(Candidate {
            entity: EntityRef { entity_id: account_id.clone(), entity_type: "account".to_string() },
            role: LinkRole::Primary,
            confidence: 0.90,
            rule_id: "P4b".to_string(),
            evidence: ev,
        })
    }
}
