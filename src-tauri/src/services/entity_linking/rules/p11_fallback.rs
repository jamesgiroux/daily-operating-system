//! P11 — Fallback: nothing matched → primary = none.

use crate::db::ActionDb;
use super::super::types::{Candidate, EntityRef, LinkRole, LinkingContext, RuleOutcome};

pub struct P11Fallback;

impl super::super::phases::Rule for P11Fallback {
    fn id(&self) -> &'static str { "P11" }

    fn evaluate(&self, _ctx: &LinkingContext, _db: &ActionDb) -> RuleOutcome {
        // Sentinel empty entity signals "no primary" to the dispatcher.
        RuleOutcome::Matched(Candidate {
            entity: EntityRef { entity_id: String::new(), entity_type: String::new() },
            role: LinkRole::Primary,
            confidence: 0.0,
            rule_id: "P11".to_string(),
            evidence: serde_json::json!({ "rule_id": "P11" }),
        })
    }
}
