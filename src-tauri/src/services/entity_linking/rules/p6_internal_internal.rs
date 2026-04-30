//! P6 — 1:1 internal × internal, no P4/P5 account evidence.
//! Primary = the other internal person (not the user themselves).

use crate::db::ActionDb;
use super::super::types::{Candidate, EntityRef, LinkRole, LinkingContext, RuleOutcome};

pub struct P6InternalInternal;

impl super::super::phases::Rule for P6InternalInternal {
    fn id(&self) -> &'static str { "P6" }

    fn evaluate(
        &self,
        _service_ctx: &crate::services::context::ServiceContext<'_>,
        ctx: &LinkingContext,
        db: &ActionDb,
    ) -> Result<RuleOutcome, String> {
        let _ = db;
        if !ctx.is_one_on_one() {
            return Ok(RuleOutcome::Skip);
        }

        let internal: Vec<_> = ctx.internal_participants().collect();
        if internal.len() != 2 {
            return Ok(RuleOutcome::Skip);
        }

        // The "other" person is whichever participant is not the From sender.
        let sender_email = ctx.from_participant().map(|p| p.email.as_str()).unwrap_or("");
        let other = internal
            .iter()
            .find(|p| p.email != sender_email)
            .or_else(|| internal.first());

        let other = match other {
            Some(p) => p,
            None => return Ok(RuleOutcome::Skip),
        };

        let person_id = match &other.person_id {
            Some(id) => id.clone(),
            None => return Ok(RuleOutcome::Skip),
        };

        Ok(RuleOutcome::Matched(Candidate {
            entity: EntityRef { entity_id: person_id, entity_type: "person".to_string() },
            role: LinkRole::Primary,
            confidence: 0.80,
            rule_id: "P6".to_string(),
            evidence: serde_json::json!({
                "rule_id": "P6",
                "shape": "1:1_internal_internal",
                "other_email": other.email,
            }),
        }))
    }
}
