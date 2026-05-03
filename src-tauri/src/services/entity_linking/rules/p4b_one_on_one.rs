//! P4b — 1:1 internal × external with a unique account_of.
//! Domain evidence outranks title evidence (design principle 5).
//!
//! Evidence-hierarchy fix: renamed from P4a so the stakeholder-inference
//! rule (`P4aStakeholder`) can take the P4a slot.

use crate::db::ActionDb;
use super::super::{evidence, primitives, types::{Candidate, EntityRef, LinkRole, LinkingContext, RuleOutcome}};

pub struct P4bOneOnOne;

impl super::super::phases::Rule for P4bOneOnOne {
    fn id(&self) -> &'static str { "P4b" }

    fn evaluate(
        &self,
        _service_ctx: &crate::services::context::ServiceContext<'_>,
        ctx: &LinkingContext,
        db: &ActionDb,
    ) -> Result<RuleOutcome, String> {
        if !ctx.is_one_on_one() {
            return Ok(RuleOutcome::Skip);
        }

        let internal: Vec<_> = ctx.internal_participants().collect();
        let external: Vec<_> = ctx.external_participants().collect();

        if internal.len() != 1 || external.len() != 1 {
            return Ok(RuleOutcome::Skip);
        }

        let ext = external[0];
        let domain = match primitives::domain_from_email(&ext.email) {
            Some(d) => d,
            None => return Ok(RuleOutcome::Skip),
        };

        let candidates = match primitives::lookup_account_candidates_by_domain(db, &domain) {
            Ok(c) => c,
            Err(e) => {
                log::warn!("P4b domain lookup error: {e}");
                return Ok(RuleOutcome::Skip);
            }
        };

        if candidates.len() != 1 {
            return Ok(RuleOutcome::Skip);
        }

        let account = &candidates[0];

        // Multi-account-active check: if the external person is a stakeholder
        // on 2+ accounts, fall through to P7 (person primary).
        if let Some(person_id) = &ext.person_id {
            match db.is_person_multi_account_active(person_id) {
                Ok(true) => return Ok(RuleOutcome::Skip),
                Err(e) => log::warn!("P4b multi-account-active check error: {e}"),
                Ok(false) => {}
            }
        }

        let ev = evidence::matched_evidence(
            ctx,
            &Candidate {
                entity: EntityRef { entity_id: account.id.clone(), entity_type: "account".to_string() },
                role: LinkRole::Primary,
                confidence: 0.95,
                rule_id: "P4b".to_string(),
                evidence: serde_json::json!({}),
            },
            &[],
        );

        Ok(RuleOutcome::Matched(Candidate {
            entity: EntityRef { entity_id: account.id.clone(), entity_type: "account".to_string() },
            role: LinkRole::Primary,
            confidence: 0.95,
            rule_id: "P4b".to_string(),
            evidence: ev,
        }))
    }
}
