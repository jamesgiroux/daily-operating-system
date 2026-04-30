//! P4d — Email surface: sender domain maps to exactly one account_of.
//!
//! DOS-258 evidence-hierarchy fix: renamed from P4c.

use crate::db::ActionDb;
use super::super::{evidence, primitives, types::{Candidate, EntityRef, LinkRole, LinkingContext, OwnerType, RuleOutcome}};

pub struct P4dSenderDomain;

impl super::super::phases::Rule for P4dSenderDomain {
    fn id(&self) -> &'static str { "P4d" }

    fn evaluate(
        &self,
        _service_ctx: &crate::services::context::ServiceContext<'_>,
        ctx: &LinkingContext,
        db: &ActionDb,
    ) -> Result<RuleOutcome, String> {
        if ctx.owner.owner_type != OwnerType::Email {
            return Ok(RuleOutcome::Skip);
        }

        let sender = match ctx.from_participant() {
            Some(p) => p,
            None => return Ok(RuleOutcome::Skip),
        };

        if ctx.is_internal_email(&sender.email) {
            return Ok(RuleOutcome::Skip);
        }

        let domain = match primitives::domain_from_email(&sender.email) {
            Some(d) => d,
            None => return Ok(RuleOutcome::Skip),
        };

        let accounts = match primitives::lookup_account_candidates_by_domain(db, &domain) {
            Ok(a) => a,
            Err(e) => {
                log::warn!("P4d domain lookup error: {e}");
                return Ok(RuleOutcome::Skip);
            }
        };

        if accounts.len() != 1 {
            return Ok(RuleOutcome::Skip);
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

        Ok(RuleOutcome::Matched(Candidate {
            entity: EntityRef { entity_id: account.id.clone(), entity_type: "account".to_string() },
            role: LinkRole::Primary,
            confidence: 0.95,
            rule_id: "P4d".to_string(),
            evidence: ev,
        }))
    }
}
