//! Phase dispatcher and Rule trait.

use crate::db::ActionDb;

use super::types::{LinkRole, LinkingContext, LinkOutcome, LinkTier, RuleOutcome};
use super::{cascade, evidence, rules};

pub trait Rule: Send + Sync {
    fn id(&self) -> &'static str;
    fn evaluate(&self, ctx: &LinkingContext, db: &ActionDb) -> RuleOutcome;
}

// ---------------------------------------------------------------------------
// Phase 1 — Suppress
// ---------------------------------------------------------------------------

const BROADCAST_ATTENDEE_THRESHOLD: usize = 50;
const BROADCAST_RECIPIENT_THRESHOLD: usize = 20;

enum Suppression {
    None,
    /// S1: declined / suppressed. No facts written.
    S1Declined,
    /// S2: broadcast / all-hands. Facts written, no primary.
    S2Broadcast,
}

fn phase1(ctx: &LinkingContext) -> Suppression {
    // S1: self-meeting (1 attendee) treated as suppressed.
    if ctx.attendee_count <= 1 && ctx.participants.len() <= 1 {
        return Suppression::S1Declined;
    }
    // S2: large all-hands or broadcast email
    if ctx.attendee_count >= BROADCAST_ATTENDEE_THRESHOLD {
        return Suppression::S2Broadcast;
    }
    // Broadcast email: many recipients, all To/CC (no personalized direct recipient)
    if ctx.owner.owner_type == super::types::OwnerType::Email
        && ctx.attendee_count >= BROADCAST_RECIPIENT_THRESHOLD
    {
        return Suppression::S2Broadcast;
    }
    Suppression::None
}

// ---------------------------------------------------------------------------
// Phase 2 — Record facts (person stub creation)
// ---------------------------------------------------------------------------

/// Create person stubs for all participants that don't have person_id set yet.
/// This runs BEFORE the write transaction so find_or_create_person can acquire
/// its own transaction without deadlocking.
pub fn phase2_record_facts(ctx: &mut LinkingContext, db: &ActionDb, user_domains: &[String]) {
    for p in &mut ctx.participants {
        if p.person_id.is_some() {
            continue;
        }
        // Determine relationship: internal people are peers, external are contacts.
        let is_internal = user_domains
            .iter()
            .any(|ud| p.email.to_lowercase().ends_with(&format!("@{}", ud.to_lowercase())));
        let relationship = if is_internal { "peer" } else { "contact" };

        match super::primitives::find_or_create_person(
            db,
            Some(&p.email),
            p.name.as_deref().unwrap_or(""),
            None,
            relationship,
            user_domains,
        ) {
            Ok(resolution) => {
                let person_id = match resolution {
                    crate::db::people::PersonResolution::FoundByEmail(person) => person.id,
                    crate::db::people::PersonResolution::FoundByName { person, .. } => person.id,
                    crate::db::people::PersonResolution::Created(person) => person.id,
                };
                p.person_id = Some(person_id);
            }
            Err(e) => {
                log::warn!("phase2: find_or_create_person failed for {}: {e}", p.email);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Phase 3 — Select primary (deterministic rule table)
// ---------------------------------------------------------------------------

struct Phase3Result {
    primary_candidate: Option<super::types::Candidate>,
    related_candidates: Vec<super::types::Candidate>,
    applied_rule: Option<String>,
}

fn phase3(ctx: &LinkingContext, db: &ActionDb) -> Phase3Result {
    // First pass: detect if any P4 domain rule fires (needed for P5 context).
    let p4_entity_id = run_p4_probe(ctx, db);

    let rule_list = rules::ordered_rules(p4_entity_id);
    let mut related: Vec<super::types::Candidate> = Vec::new();

    for rule in &rule_list {
        match rule.evaluate(ctx, db) {
            RuleOutcome::Matched(candidate) => {
                // P8/P11 sentinels (empty entity_id) mean "no primary" explicitly.
                if rules::is_no_primary_sentinel(&candidate) {
                    return Phase3Result {
                        primary_candidate: None,
                        related_candidates: related,
                        applied_rule: Some(candidate.rule_id),
                    };
                }
                // P5 may return role=Related when blocked by domain conflict.
                if candidate.role == LinkRole::Related {
                    related.push(candidate);
                    continue;
                }
                return Phase3Result {
                    primary_candidate: Some(candidate),
                    related_candidates: related,
                    applied_rule: None,
                };
            }
            RuleOutcome::Skip => continue,
        }
    }

    Phase3Result {
        primary_candidate: None,
        related_candidates: related,
        applied_rule: Some("P11".to_string()),
    }
}

/// Run only the P4 rules to detect a domain-evidence entity for P5's context.
/// Returns the entity_id if a P4 rule fires, otherwise None.
fn run_p4_probe(ctx: &LinkingContext, db: &ActionDb) -> Option<String> {
    use rules::{p4a_one_on_one::P4aOneOnOne, p4b_group_shared::P4bGroupShared, p4c_sender_domain::P4cSenderDomain};
    for rule in [
        &P4aOneOnOne as &dyn Rule,
        &P4bGroupShared,
        &P4cSenderDomain,
    ] {
        if let RuleOutcome::Matched(c) = rule.evaluate(ctx, db) {
            if !rules::is_no_primary_sentinel(&c) {
                return Some(c.entity.entity_id);
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Full four-phase runner
// ---------------------------------------------------------------------------

/// Run all four phases and return the link outcome.
///
/// Phase 2 person-stub creation must already have been done by the caller
/// (via `phase2_record_facts`) so that `ctx.participants[*].person_id` is
/// populated before we enter the write transaction.
pub fn run_phases(ctx: &LinkingContext, db: &ActionDb) -> Result<LinkOutcome, String> {
    // --- Phase 1 ---
    let suppression = phase1(ctx);
    match suppression {
        Suppression::S1Declined => {
            let ev = evidence::suppress_evidence(ctx, "S1_declined");
            let _ = db.insert_linking_evaluation(&crate::db::entity_linking::LinkingEvaluationWrite {
                owner_type: ctx.owner.owner_type.as_str(),
                owner_id: &ctx.owner.owner_id,
                trigger: "suppress",
                rule_id: Some("S1"),
                entity_id: None,
                entity_type: None,
                role: None,
                graph_version: ctx.graph_version,
                evidence_json: &ev.to_string(),
            });
            return Ok(LinkOutcome {
                owner: ctx.owner.clone(),
                primary: None,
                related: vec![],
                tier: LinkTier::Skip,
                applied_rule: Some("S1".to_string()),
            });
        }
        Suppression::S2Broadcast => {
            // Phase 2 still runs for broadcasts (facts written), but no primary.
            // (Phase 2 already ran before this call.)
            let ev = evidence::suppress_evidence(ctx, "S2_broadcast");
            let _ = db.insert_linking_evaluation(&crate::db::entity_linking::LinkingEvaluationWrite {
                owner_type: ctx.owner.owner_type.as_str(),
                owner_id: &ctx.owner.owner_id,
                trigger: "suppress",
                rule_id: Some("S2"),
                entity_id: None,
                entity_type: None,
                role: None,
                graph_version: ctx.graph_version,
                evidence_json: &ev.to_string(),
            });
            return Ok(LinkOutcome {
                owner: ctx.owner.clone(),
                primary: None,
                related: vec![],
                tier: LinkTier::Skip,
                applied_rule: Some("S2".to_string()),
            });
        }
        Suppression::None => {}
    }

    // --- Phase 3 (inside write transaction) ---
    let phase3_result = db.with_transaction(|_conn| {
        // Re-read graph version inside transaction to detect stale context.
        // (A full retry loop is a TODO; for now we proceed with a warning.)
        if let Ok(current_version) = db.get_entity_graph_version() {
            if current_version != ctx.graph_version {
                log::warn!(
                    "entity_linking: graph version changed during evaluation \
                     ({} → {}); proceeding with stale context",
                    ctx.graph_version, current_version
                );
            }
        }

        // Read dismissals.
        let dismissals = db
            .get_linking_dismissals(ctx.owner.owner_type.as_str(), &ctx.owner.owner_id)
            .unwrap_or_default();

        let mut p3 = phase3(ctx, db);

        // Suppress primary if it was dismissed by the user.
        if let Some(ref primary) = p3.primary_candidate {
            let dismissed = dismissals.iter().any(|(eid, etype)| {
                eid == &primary.entity.entity_id && etype == &primary.entity.entity_type
            });
            if dismissed {
                p3.primary_candidate = None;
            }
        }

        // --- Delete old auto links for this owner ---
        let _ = db.delete_auto_links_for_owner(
            ctx.owner.owner_type.as_str(),
            &ctx.owner.owner_id,
        );

        // --- Write primary ---
        if let Some(ref primary) = p3.primary_candidate {
            let source = format!("rule:{}", primary.rule_id);
            let _ = db.upsert_linked_entity_raw(&crate::db::entity_linking::LinkedEntityRawWrite {
                owner_type: ctx.owner.owner_type.as_str().to_string(),
                owner_id: ctx.owner.owner_id.clone(),
                entity_id: primary.entity.entity_id.clone(),
                entity_type: primary.entity.entity_type.clone(),
                role: "primary".to_string(),
                source,
                rule_id: Some(primary.rule_id.clone()),
                confidence: Some(primary.confidence),
                evidence_json: Some(primary.evidence.to_string()),
                graph_version: ctx.graph_version,
            });
        }

        // --- Write related chips ---
        for related in &p3.related_candidates {
            let source = format!("rule:{}", related.rule_id);
            let _ = db.upsert_linked_entity_raw(&crate::db::entity_linking::LinkedEntityRawWrite {
                owner_type: ctx.owner.owner_type.as_str().to_string(),
                owner_id: ctx.owner.owner_id.clone(),
                entity_id: related.entity.entity_id.clone(),
                entity_type: related.entity.entity_type.clone(),
                role: related.role.as_str().to_string(),
                source,
                rule_id: Some(related.rule_id.clone()),
                confidence: Some(related.confidence),
                evidence_json: Some(related.evidence.to_string()),
                graph_version: ctx.graph_version,
            });
        }

        // --- Append evaluation record ---
        let primary_ref = p3.primary_candidate.as_ref();
        let rule_id = p3
            .applied_rule
            .as_deref()
            .or_else(|| primary_ref.map(|c| c.rule_id.as_str()));
        let ev_json = primary_ref
            .map(|c| c.evidence.to_string())
            .unwrap_or_else(|| "{}".to_string());
        let _ = db.insert_linking_evaluation(&crate::db::entity_linking::LinkingEvaluationWrite {
            owner_type: ctx.owner.owner_type.as_str(),
            owner_id: &ctx.owner.owner_id,
            trigger: "evaluate",
            rule_id,
            entity_id: primary_ref.map(|c| c.entity.entity_id.as_str()),
            entity_type: primary_ref.map(|c| c.entity.entity_type.as_str()),
            role: primary_ref.map(|c| c.role.as_str()),
            graph_version: ctx.graph_version,
            evidence_json: &ev_json,
        });

        Ok(p3)
    })?;

    // --- Phase 4 (cascade, runs after transaction commits) ---
    let outcome = cascade::run_cascade(ctx, &phase3_result.primary_candidate, &phase3_result.related_candidates, db)?;

    Ok(outcome)
}
