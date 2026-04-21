use crate::db::ActionDb;
use super::super::{evidence, primitives, types::{Candidate, EntityRef, LinkRole, LinkingContext, RuleOutcome}};

pub struct P2ThreadInheritance;

impl super::super::phases::Rule for P2ThreadInheritance {
    fn id(&self) -> &'static str { "P2" }

    fn evaluate(&self, ctx: &LinkingContext, db: &ActionDb) -> RuleOutcome {
        // Email surface only.
        let Some(thread_id) = &ctx.thread_id else {
            return RuleOutcome::Skip;
        };

        let parent = match db.get_thread_primary_link(thread_id, &ctx.owner.owner_id) {
            Ok(Some(p)) => p,
            Ok(None) => {
                // Parent not yet evaluated — enqueue for later flush.
                let _ = db.enqueue_thread_inheritance(thread_id, &ctx.owner.owner_id);
                return RuleOutcome::Skip;
            }
            Err(e) => {
                log::warn!("P2 get_thread_primary_link error: {e}");
                return RuleOutcome::Skip;
            }
        };

        // Domain compatibility check: child sender domain must be in parent
        // account's domains, or sender must be the same as the parent's sender.
        let sender_domain = ctx
            .from_participant()
            .and_then(|p| primitives::domain_from_email(&p.email));

        let domain_ok = sender_domain
            .as_deref()
            .map(|d| {
                parent
                    .account_domains
                    .iter()
                    .any(|pd| pd.eq_ignore_ascii_case(d))
            })
            .unwrap_or(false);

        if !domain_ok {
            return RuleOutcome::Skip;
        }

        let ev = evidence::thread_inheritance_evidence(
            ctx,
            &ctx.owner.owner_id,
            &parent.entity_id,
            domain_ok,
        );
        RuleOutcome::Matched(Candidate {
            entity: EntityRef {
                entity_id: parent.entity_id,
                entity_type: parent.entity_type,
            },
            role: LinkRole::Primary,
            confidence: 0.9,
            rule_id: "P2".to_string(),
            evidence: ev,
        })
    }
}
