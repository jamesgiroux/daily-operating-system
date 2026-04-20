use crate::db::ActionDb;
use super::super::{evidence, types::{Candidate, EntityRef, LinkRole, LinkingContext, RuleOutcome}};

pub struct P1UserOverride;

impl super::super::phases::Rule for P1UserOverride {
    fn id(&self) -> &'static str { "P1" }

    fn evaluate(&self, ctx: &LinkingContext, db: &ActionDb) -> RuleOutcome {
        match db.get_user_override_link(ctx.owner.owner_type.as_str(), &ctx.owner.owner_id) {
            Ok(Some((entity_id, entity_type))) => RuleOutcome::Matched(Candidate {
                entity: EntityRef { entity_id: entity_id.clone(), entity_type: entity_type.clone() },
                role: LinkRole::Primary,
                confidence: 1.0,
                rule_id: "P1".to_string(),
                evidence: evidence::matched_evidence(ctx, &Candidate {
                    entity: EntityRef { entity_id, entity_type },
                    role: LinkRole::Primary,
                    confidence: 1.0,
                    rule_id: "P1".to_string(),
                    evidence: serde_json::json!({}),
                }, &[]),
            }),
            Ok(None) => RuleOutcome::Skip,
            Err(e) => {
                log::warn!("P1 DB error for {}/{}: {e}", ctx.owner.owner_type.as_str(), ctx.owner.owner_id);
                RuleOutcome::Skip
            }
        }
    }
}
