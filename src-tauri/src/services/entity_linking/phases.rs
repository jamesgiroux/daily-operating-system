//! Phase dispatcher and Rule trait.

use crate::db::ActionDb;

use super::types::{LinkRole, LinkingContext, LinkOutcome, LinkTier, RuleOutcome, Trigger};
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

/// Check Phase 1 suppression conditions.
///
/// Returns `Ok(Some(outcome))` if the owner is suppressed and evaluation must
/// stop. Returns `Ok(None)` to continue to Phase 2+. This is called by
/// `evaluate()` BEFORE Phase 2 (person-stub creation) so that declined/self
/// meetings don't generate person rows.
pub fn phase1_suppress(
    ctx: &LinkingContext,
    db: &ActionDb,
    trigger: Trigger,
) -> Result<Option<LinkOutcome>, String> {
    // S1: self-meeting (single attendee) — no facts, no primary.
    let s1 = ctx.attendee_count <= 1 && ctx.participants.len() <= 1;

    // S2: all-hands / broadcast — facts still written in Phase 2 by caller,
    // but no primary. Caller decides whether to run Phase 2 after this check.
    let s2 = ctx.attendee_count >= BROADCAST_ATTENDEE_THRESHOLD
        || (ctx.owner.owner_type == super::types::OwnerType::Email
            && ctx.attendee_count >= BROADCAST_RECIPIENT_THRESHOLD);

    let (rule_id, tier) = if s1 {
        ("S1", LinkTier::Skip)
    } else if s2 {
        ("S2", LinkTier::Skip)
    } else {
        return Ok(None);
    };

    let ev = evidence::suppress_evidence(ctx, rule_id);
    let _ = db.insert_linking_evaluation(&crate::db::entity_linking::LinkingEvaluationWrite {
        owner_type: ctx.owner.owner_type.as_str(),
        owner_id: &ctx.owner.owner_id,
        trigger: trigger.as_str(),
        rule_id: Some(rule_id),
        entity_id: None,
        entity_type: None,
        role: None,
        graph_version: ctx.graph_version,
        evidence_json: &ev.to_string(),
    });

    Ok(Some(LinkOutcome {
        owner: ctx.owner.clone(),
        primary: None,
        related: vec![],
        tier,
        applied_rule: Some(rule_id.to_string()),
    }))
}

// ---------------------------------------------------------------------------
// Phase 2 — Record facts (person stub creation)
// ---------------------------------------------------------------------------

/// Create person stubs for participants that don't yet have person_id set.
///
/// Each call to find_or_create_person uses its own internal transaction.
/// This runs BEFORE the phase-3 write transaction.
pub fn phase2_record_facts(ctx: &mut LinkingContext, db: &ActionDb, user_domains: &[String]) {
    for p in &mut ctx.participants {
        if p.person_id.is_some() {
            continue;
        }
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
    // Run all three P4 domain rules and collect every match.
    // This enables P9 detection (multiple accounts) without requiring P9 to
    // fire as a separate rule — P9 is a dispatcher-level decision, not a rule.
    let p4_candidates = collect_p4_candidates(ctx, db);

    // P9: two or more distinct domain-evidence accounts — no primary,
    // all become related chips so the user can pick via EntityLinkPicker.
    if p4_candidates.len() > 1 {
        let related = p4_candidates
            .into_iter()
            .map(|mut c| {
                c.role = LinkRole::Related;
                c
            })
            .collect();
        return Phase3Result {
            primary_candidate: None,
            related_candidates: related,
            applied_rule: Some("P9".to_string()),
        };
    }

    // Pass the single P4 entity_id to P5 so title evidence can check
    // domain consistency. If no P4 match, P5 runs unconstrained.
    let p4_entity_id = p4_candidates.into_iter().next().map(|c| c.entity.entity_id);

    // Full ordered rule list. P4 rules run again here (read-only, safe)
    // because they need to be part of the first-Matched-wins chain.
    let rule_list = rules::ordered_rules(p4_entity_id);
    let mut related: Vec<super::types::Candidate> = Vec::new();

    for rule in &rule_list {
        match rule.evaluate(ctx, db) {
            RuleOutcome::Matched(candidate) => {
                // P8/P11 sentinels (empty entity_id) → explicit no-primary.
                if rules::is_no_primary_sentinel(&candidate) {
                    return Phase3Result {
                        primary_candidate: None,
                        related_candidates: related,
                        applied_rule: Some(candidate.rule_id),
                    };
                }

                // P10 returns AutoSuggested — write as related chip, not primary.
                if candidate.role == LinkRole::AutoSuggested {
                    related.push(candidate);
                    continue;
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

/// Run all P4 domain rules and collect every match.
/// Returns 0 items (no domain evidence), 1 item (normal P4), or ≥2 items (P9).
fn collect_p4_candidates(ctx: &LinkingContext, db: &ActionDb) -> Vec<super::types::Candidate> {
    use rules::{
        p4a_one_on_one::P4aOneOnOne, p4b_group_shared::P4bGroupShared,
        p4c_sender_domain::P4cSenderDomain,
    };
    let mut candidates = Vec::new();
    for rule in [
        &P4aOneOnOne as &dyn Rule,
        &P4bGroupShared as &dyn Rule,
        &P4cSenderDomain as &dyn Rule,
    ] {
        if let RuleOutcome::Matched(c) = rule.evaluate(ctx, db) {
            if !rules::is_no_primary_sentinel(&c) {
                // Deduplicate by entity_id — P4b and P4c could both match
                // the same account for a given email.
                if !candidates
                    .iter()
                    .any(|existing: &super::types::Candidate| {
                        existing.entity.entity_id == c.entity.entity_id
                    })
                {
                    candidates.push(c);
                }
            }
        }
    }
    candidates
}

// ---------------------------------------------------------------------------
// Full phase-3 + phase-4 runner
// ---------------------------------------------------------------------------

/// Run phases 3 and 4 inside the write transaction.
///
/// Phase 2 person-stub creation must already have been done by the caller
/// before entering this function.
pub fn run_phases(ctx: &LinkingContext, db: &ActionDb) -> Result<LinkOutcome, String> {
    let phase3_result = db.with_transaction(|_| {
        // CAS: read graph version inside the transaction. If it changed since
        // the adapter built ctx, retry phase3 once with the fresh snapshot
        // rather than writing stale links (DOS-258 spec: "retry once").
        let current_version = db.get_entity_graph_version().unwrap_or(ctx.graph_version);
        let refreshed_ctx: std::borrow::Cow<LinkingContext> = if current_version != ctx.graph_version {
            log::info!(
                "entity_linking: graph version changed ({} → {}), re-running phase3 \
                 with fresh snapshot",
                ctx.graph_version, current_version
            );
            let mut refreshed = ctx.clone();
            refreshed.graph_version = current_version;
            std::borrow::Cow::Owned(refreshed)
        } else {
            std::borrow::Cow::Borrowed(ctx)
        };
        let ctx = &*refreshed_ctx;

        let dismissals = db
            .get_linking_dismissals(ctx.owner.owner_type.as_str(), &ctx.owner.owner_id)
            .map_err(|e| format!("get_linking_dismissals: {e}"))?;

        let mut p3 = phase3(ctx, db);

        // Suppress primary if user has dismissed it.
        if let Some(ref primary) = p3.primary_candidate {
            let dismissed = dismissals.iter().any(|(eid, etype)| {
                eid == &primary.entity.entity_id && etype == &primary.entity.entity_type
            });
            if dismissed {
                p3.primary_candidate = None;
            }
        }

        // Delete old auto-resolution rows. Preserves source='user' and
        // source='user_dismissed' so user overrides and dismissals survive.
        db.delete_auto_links_for_owner(ctx.owner.owner_type.as_str(), &ctx.owner.owner_id)
            .map_err(|e| format!("delete_auto_links: {e}"))?;

        // Write primary.
        if let Some(ref primary) = p3.primary_candidate {
            let source = format!("rule:{}", primary.rule_id);
            db.upsert_linked_entity_raw(&crate::db::entity_linking::LinkedEntityRawWrite {
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
            })
            .map_err(|e| format!("upsert primary: {e}"))?;
        }

        // Write related chips.
        for related in &p3.related_candidates {
            let source = format!("rule:{}", related.rule_id);
            db.upsert_linked_entity_raw(&crate::db::entity_linking::LinkedEntityRawWrite {
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
            })
            .map_err(|e| format!("upsert related: {e}"))?;
        }

        // Append evaluation record.
        let primary_ref = p3.primary_candidate.as_ref();
        let rule_id = p3
            .applied_rule
            .as_deref()
            .or_else(|| primary_ref.map(|c| c.rule_id.as_str()));
        let ev_json = primary_ref
            .map(|c| c.evidence.to_string())
            .unwrap_or_else(|| "{}".to_string());
        db.insert_linking_evaluation(&crate::db::entity_linking::LinkingEvaluationWrite {
            owner_type: ctx.owner.owner_type.as_str(),
            owner_id: &ctx.owner.owner_id,
            trigger: "evaluate",
            rule_id,
            entity_id: primary_ref.map(|c| c.entity.entity_id.as_str()),
            entity_type: primary_ref.map(|c| c.entity.entity_type.as_str()),
            role: primary_ref.map(|c| c.role.as_str()),
            graph_version: ctx.graph_version,
            evidence_json: &ev_json,
        })
        .map_err(|e| format!("insert_linking_evaluation: {e}"))?;

        Ok(p3)
    })?;

    // Phase 4 (cascade) runs after the write transaction commits.
    cascade::run_cascade(
        ctx,
        &phase3_result.primary_candidate,
        &phase3_result.related_candidates,
        db,
    )
}
