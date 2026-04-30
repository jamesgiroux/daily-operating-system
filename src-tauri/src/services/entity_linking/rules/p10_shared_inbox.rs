//! P10 — Email shared-inbox heuristic.
//! Sender local-part linked to one account ≥3× prior → auto_suggested, not primary.

use crate::db::ActionDb;
use super::super::types::{Candidate, EntityRef, LinkRole, LinkingContext, OwnerType, RuleOutcome};

pub struct P10SharedInbox;

impl super::super::phases::Rule for P10SharedInbox {
    fn id(&self) -> &'static str { "P10" }

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

        let links = match db.count_sender_account_links(&sender.email, 90) {
            Ok(l) => l,
            Err(e) => {
                log::warn!("P10 count_sender_account_links error: {e}");
                return Ok(RuleOutcome::Skip);
            }
        };

        // If the top account has ≥3 prior links, surface as auto_suggested (not primary).
        if let Some((account_id, count)) = links.first() {
            if *count >= 3 {
                return Ok(RuleOutcome::Matched(Candidate {
                    entity: EntityRef {
                        entity_id: account_id.clone(),
                        entity_type: "account".to_string(),
                    },
                    role: LinkRole::AutoSuggested,
                    confidence: 0.60,
                    rule_id: "P10".to_string(),
                    evidence: serde_json::json!({
                        "rule_id": "P10",
                        "sender_email": sender.email,
                        "account_id": account_id,
                        "prior_link_count": count,
                    }),
                }));
            }
        }

        Ok(RuleOutcome::Skip)
    }
}
