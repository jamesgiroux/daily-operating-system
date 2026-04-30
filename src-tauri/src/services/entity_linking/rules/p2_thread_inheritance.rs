use crate::db::ActionDb;
use super::super::{evidence, primitives, types::{Candidate, EntityRef, LinkRole, LinkingContext, RuleOutcome}};

pub struct P2ThreadInheritance;

impl super::super::phases::Rule for P2ThreadInheritance {
    fn id(&self) -> &'static str { "P2" }

    fn evaluate(
        &self,
        ctx: &crate::services::context::ServiceContext<'_>,
        link_ctx: &LinkingContext,
        db: &ActionDb,
    ) -> Result<RuleOutcome, String> {
        ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
        // Email surface only.
        let Some(thread_id) = &link_ctx.thread_id else {
            return Ok(RuleOutcome::Skip);
        };

        let parent = match db.get_thread_primary_link(thread_id, &link_ctx.owner.owner_id) {
            Ok(Some(p)) => p,
            Ok(None) => {
                // Parent not yet evaluated — enqueue for later flush.
                let _ = db.enqueue_thread_inheritance(thread_id, &link_ctx.owner.owner_id);
                return Ok(RuleOutcome::Skip);
            }
            Err(e) => {
                log::warn!("P2 get_thread_primary_link error: {e}");
                return Ok(RuleOutcome::Skip);
            }
        };

        let sender = link_ctx.from_participant();
        let sender_email = sender.map(|p| p.email.as_str()).unwrap_or("");
        let sender_domain = sender.and_then(|p| primitives::domain_from_email(&p.email));

        // Domain compatibility: child sender domain must be in parent account's
        // domains. If entity_type != 'account', domain check is skipped.
        let domain_ok = sender_domain
            .as_deref()
            .map(|d| {
                parent
                    .account_domains
                    .iter()
                    .any(|pd| pd.eq_ignore_ascii_case(d))
            })
            .unwrap_or(false);

        // Same-sender check: child and parent were sent by the same email address
        // (e.g. a reply from the same contact on a person-primary thread).
        let same_sender = parent
            .parent_sender_email
            .as_deref()
            .map(|ps| ps.eq_ignore_ascii_case(sender_email))
            .unwrap_or(false);

        if !domain_ok && !same_sender {
            return Ok(RuleOutcome::Skip);
        }

        let ev = evidence::thread_inheritance_evidence(
            link_ctx,
            &link_ctx.owner.owner_id,
            &parent.entity_id,
            domain_ok,
        );
        Ok(RuleOutcome::Matched(Candidate {
            entity: EntityRef {
                entity_id: parent.entity_id,
                entity_type: parent.entity_type,
            },
            role: LinkRole::Primary,
            confidence: 0.9,
            rule_id: "P2".to_string(),
            evidence: ev,
        }))
    }
}
