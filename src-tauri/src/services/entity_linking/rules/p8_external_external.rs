//! P8 — 1:1 external × external → primary = none.

use crate::db::ActionDb;
use super::super::types::{Candidate, EntityRef, LinkRole, LinkingContext, RuleOutcome};

pub struct P8ExternalExternal;

impl super::super::phases::Rule for P8ExternalExternal {
    fn id(&self) -> &'static str { "P8" }

    fn evaluate(&self, ctx: &LinkingContext, db: &ActionDb) -> RuleOutcome {
        let _ = db;
        if !ctx.is_one_on_one() {
            return RuleOutcome::Skip;
        }
        let internal_count = ctx.internal_participants().count();
        if internal_count != 0 {
            return RuleOutcome::Skip;
        }
        // Both external: signal "no primary" explicitly so the fallback
        // records the right shape rather than a generic P11.
        // We encode this as Matched with a sentinel that phases.rs treats
        // as no-primary.
        RuleOutcome::Matched(Candidate {
            entity: EntityRef { entity_id: String::new(), entity_type: String::new() },
            role: LinkRole::Primary,
            confidence: 0.0,
            rule_id: "P8".to_string(),
            evidence: serde_json::json!({ "rule_id": "P8", "shape": "1:1_external_external" }),
        })
    }
}
