use crate::db::ActionDb;
use super::super::types::{Candidate, EntityRef, LinkRole, LinkingContext, RuleOutcome};

pub struct P3SeriesInheritance;

impl super::super::phases::Rule for P3SeriesInheritance {
    fn id(&self) -> &'static str { "P3" }

    fn evaluate(&self, ctx: &LinkingContext, db: &ActionDb) -> RuleOutcome {
        let Some(series_id) = &ctx.series_id else {
            return RuleOutcome::Skip;
        };
        match db.get_series_primary_link(series_id, &ctx.owner.owner_id) {
            Ok(Some((entity_id, entity_type))) => RuleOutcome::Matched(Candidate {
                entity: EntityRef { entity_id, entity_type },
                role: LinkRole::Primary,
                confidence: 0.85,
                rule_id: "P3".to_string(),
                evidence: serde_json::json!({ "rule_id": "P3", "series_id": series_id }),
            }),
            Ok(None) => RuleOutcome::Skip,
            Err(e) => {
                log::warn!("P3 error: {e}");
                RuleOutcome::Skip
            }
        }
    }
}
