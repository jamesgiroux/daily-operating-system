//! P7 — 1:1 internal × external, no P4 domain match.
//! Primary = the external person.

use crate::db::ActionDb;
use super::super::types::{Candidate, EntityRef, LinkRole, LinkingContext, RuleOutcome};

pub struct P7InternalExternal;

impl super::super::phases::Rule for P7InternalExternal {
    fn id(&self) -> &'static str { "P7" }

    fn evaluate(&self, ctx: &LinkingContext, db: &ActionDb) -> RuleOutcome {
        let _ = db;
        if !ctx.is_one_on_one() {
            return RuleOutcome::Skip;
        }

        let internal: Vec<_> = ctx.internal_participants().collect();
        let external: Vec<_> = ctx.external_participants().collect();

        if internal.len() != 1 || external.len() != 1 {
            return RuleOutcome::Skip;
        }

        let person_id = match &external[0].person_id {
            Some(id) => id.clone(),
            None => return RuleOutcome::Skip,
        };

        RuleOutcome::Matched(Candidate {
            entity: EntityRef { entity_id: person_id, entity_type: "person".to_string() },
            role: LinkRole::Primary,
            confidence: 0.70,
            rule_id: "P7".to_string(),
            evidence: serde_json::json!({
                "rule_id": "P7",
                "shape": "1:1_internal_external_no_domain",
                "external_email": external[0].email,
            }),
        })
    }
}
