//! P6 — 1:1 internal × internal, no P4/P5 account evidence.
//! Primary = the other internal person (not the user themselves).

use crate::db::ActionDb;
use super::super::types::{Candidate, EntityRef, LinkRole, LinkingContext, RuleOutcome};

pub struct P6InternalInternal;

impl super::super::phases::Rule for P6InternalInternal {
    fn id(&self) -> &'static str { "P6" }

    fn evaluate(&self, ctx: &LinkingContext, db: &ActionDb) -> RuleOutcome {
        let _ = db;
        if !ctx.is_one_on_one() {
            return RuleOutcome::Skip;
        }

        let internal: Vec<_> = ctx.internal_participants().collect();
        if internal.len() != 2 {
            return RuleOutcome::Skip;
        }

        // The "other" person is whichever participant is not the From sender.
        let sender_email = ctx.from_participant().map(|p| p.email.as_str()).unwrap_or("");
        let other = internal
            .iter()
            .find(|p| p.email != sender_email)
            .or_else(|| internal.first());

        let other = match other {
            Some(p) => p,
            None => return RuleOutcome::Skip,
        };

        let person_id = match &other.person_id {
            Some(id) => id.clone(),
            None => return RuleOutcome::Skip,
        };

        RuleOutcome::Matched(Candidate {
            entity: EntityRef { entity_id: person_id, entity_type: "person".to_string() },
            role: LinkRole::Primary,
            confidence: 0.80,
            rule_id: "P6".to_string(),
            evidence: serde_json::json!({
                "rule_id": "P6",
                "shape": "1:1_internal_internal",
                "other_email": other.email,
            }),
        })
    }
}
