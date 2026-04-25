//! P4d — Email surface: sender domain maps to exactly one account_of.
//!
//! DOS-258 evidence-hierarchy fix: renamed from P4c.

use crate::db::ActionDb;
use super::super::{evidence, primitives, types::{Candidate, EntityRef, LinkRole, LinkingContext, OwnerType, RuleOutcome}};

pub struct P4dSenderDomain;

impl super::super::phases::Rule for P4dSenderDomain {
    fn id(&self) -> &'static str { "P4d" }

    fn evaluate(&self, ctx: &LinkingContext, db: &ActionDb) -> RuleOutcome {
        if ctx.owner.owner_type != OwnerType::Email {
            return RuleOutcome::Skip;
        }

        let sender = match ctx.from_participant() {
            Some(p) => p,
            None => return RuleOutcome::Skip,
        };

        if ctx.is_internal_email(&sender.email) {
            return RuleOutcome::Skip;
        }

        let domain = match primitives::domain_from_email(&sender.email) {
            Some(d) => d,
            None => return RuleOutcome::Skip,
        };

        let accounts = match primitives::lookup_account_candidates_by_domain(db, &domain) {
            Ok(a) => a,
            Err(e) => {
                log::warn!("P4d domain lookup error: {e}");
                return RuleOutcome::Skip;
            }
        };

        if accounts.len() != 1 {
            return RuleOutcome::Skip;
        }

        let account = &accounts[0];
        let ev = evidence::matched_evidence(
            ctx,
            &Candidate {
                entity: EntityRef { entity_id: account.id.clone(), entity_type: "account".to_string() },
                role: LinkRole::Primary,
                confidence: 0.95,
                rule_id: "P4d".to_string(),
                evidence: serde_json::json!({}),
            },
            &[],
        );

        RuleOutcome::Matched(Candidate {
            entity: EntityRef { entity_id: account.id.clone(), entity_type: "account".to_string() },
            role: LinkRole::Primary,
            confidence: 0.95,
            rule_id: "P4d".to_string(),
            evidence: ev,
        })
    }
}
