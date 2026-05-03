//! Phase dispatcher and Rule trait.

use crate::db::ActionDb;
use crate::services::context::ServiceContext;

use super::types::{LinkRole, LinkingContext, LinkOutcome, LinkTier, RuleOutcome, Trigger};
use super::{cascade, evidence, rules};

pub trait Rule: Send + Sync {
    fn id(&self) -> &'static str;
    fn evaluate(
        &self,
        ctx: &ServiceContext<'_>,
        link_ctx: &LinkingContext,
        db: &ActionDb,
    ) -> Result<RuleOutcome, String>;
}

// ---------------------------------------------------------------------------
// Phase 1 — Suppress
// ---------------------------------------------------------------------------

const BROADCAST_ATTENDEE_THRESHOLD: usize = 50;
const BROADCAST_RECIPIENT_THRESHOLD: usize = 20;

/// Result of the Phase 1 suppression check.
pub enum Phase1Result {
    /// S1: self-meeting / declined. No facts, no primary.
    Declined(LinkOutcome),
    /// S2: all-hands / broadcast. **Facts still written** (spec AC#6), no primary.
    Broadcast(LinkOutcome),
    /// Not suppressed — continue to Phase 2+.
    Continue,
}

/// Check Phase 1 suppression conditions.
///
/// The caller is responsible for calling `phase2_record_facts` for `Broadcast`
/// (S2) but NOT for `Declined` (S1). This is called by `evaluate()` before
/// Phase 2 so that self-meetings don't generate person rows.
pub fn phase1_suppress(
    ctx: &ServiceContext<'_>,
    link_ctx: &LinkingContext,
    db: &ActionDb,
    trigger: Trigger,
) -> Result<Phase1Result, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    // S1: self-meeting (single attendee) — no facts, no primary.
    let s1 = link_ctx.attendee_count <= 1 && link_ctx.participants.len() <= 1;

    // S2: all-hands / broadcast — facts still written in Phase 2 (AC#6),
    // but no primary.
    let s2 = link_ctx.attendee_count >= BROADCAST_ATTENDEE_THRESHOLD
        || (link_ctx.owner.owner_type == super::types::OwnerType::Email
            && link_ctx.attendee_count >= BROADCAST_RECIPIENT_THRESHOLD);

    if !s1 && !s2 {
        return Ok(Phase1Result::Continue);
    }

    let rule_id = if s1 { "S1" } else { "S2" };
    let ev = evidence::suppress_evidence(link_ctx, rule_id);
    let _ = db.insert_linking_evaluation(&crate::db::entity_linking::LinkingEvaluationWrite {
        owner_type: link_ctx.owner.owner_type.as_str(),
        owner_id: &link_ctx.owner.owner_id,
        trigger: trigger.as_str(),
        rule_id: Some(rule_id),
        entity_id: None,
        entity_type: None,
        role: None,
        graph_version: link_ctx.graph_version,
        evidence_json: &ev.to_string(),
    });

    let outcome = LinkOutcome {
        owner: link_ctx.owner.clone(),
        primary: None,
        related: vec![],
        tier: LinkTier::Skip,
        applied_rule: Some(rule_id.to_string()),
    };

    if s1 {
        Ok(Phase1Result::Declined(outcome))
    } else {
        Ok(Phase1Result::Broadcast(outcome))
    }
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

fn phase3(
    ctx: &ServiceContext<'_>,
    link_ctx: &LinkingContext,
    db: &ActionDb,
) -> Result<Phase3Result, String> {
    // Run all three P4 domain rules and collect every match.
    // This enables P9 detection (multiple accounts) without requiring P9 to
    // fire as a separate rule — P9 is a dispatcher-level decision, not a rule.
    let p4_candidates = collect_p4_candidates(ctx, link_ctx, db)?;

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
        return Ok(Phase3Result {
            primary_candidate: None,
            related_candidates: related,
            applied_rule: Some("P9".to_string()),
        });
    }

    // Pass the single P4 entity_id to P5 so title evidence can check
    // domain consistency. If no P4 match, P5 runs unconstrained.
    let p4_entity_id = p4_candidates.into_iter().next().map(|c| c.entity.entity_id);

    // Full ordered rule list. P4 rules run again here (read-only, safe)
    // because they need to be part of the first-Matched-wins chain.
    let rule_list = rules::ordered_rules(p4_entity_id);
    let mut related: Vec<super::types::Candidate> = Vec::new();

    for rule in &rule_list {
        match rule.evaluate(ctx, link_ctx, db)? {
            RuleOutcome::Matched(candidate) => {
                // P8/P11 sentinels (empty entity_id) → explicit no-primary.
                if rules::is_no_primary_sentinel(&candidate) {
                    return Ok(Phase3Result {
                        primary_candidate: None,
                        related_candidates: related,
                        applied_rule: Some(candidate.rule_id),
                    });
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

                return Ok(Phase3Result {
                    primary_candidate: Some(candidate),
                    related_candidates: related,
                    applied_rule: None,
                });
            }
            RuleOutcome::Skip => continue,
        }
    }

    Ok(Phase3Result {
        primary_candidate: None,
        related_candidates: related,
        applied_rule: Some("P11".to_string()),
    })
}

/// Run all P4 rules (stakeholder + domain) and collect every distinct match.
/// Returns 0 items (no P4 evidence), 1 item (normal P4), or ≥2 items (P9).
///
/// Includes P4a stakeholder-inference alongside P4b/P4c/P4d domain rules
/// so a stakeholder match + domain match on the same account dedupes to
/// one candidate, while stakeholder evidence and domain evidence for
/// different accounts produce distinct candidates and trigger the P9 picker.
fn collect_p4_candidates(
    ctx: &ServiceContext<'_>,
    link_ctx: &LinkingContext,
    db: &ActionDb,
) -> Result<Vec<super::types::Candidate>, String> {
    use rules::{
        p4a_stakeholder::P4aStakeholder, p4b_one_on_one::P4bOneOnOne,
        p4c_group_shared::P4cGroupShared, p4d_sender_domain::P4dSenderDomain,
    };
    let mut candidates = Vec::new();

    for c in P4aStakeholder::collect_candidates(link_ctx, db) {
        if !candidates
            .iter()
            .any(|existing: &super::types::Candidate| {
                existing.entity.entity_id == c.entity.entity_id
            })
        {
            candidates.push(c);
        }
    }

    for rule in [
        &P4bOneOnOne as &dyn Rule,
        &P4cGroupShared as &dyn Rule,
        &P4dSenderDomain as &dyn Rule,
    ] {
        if let RuleOutcome::Matched(c) = rule.evaluate(ctx, link_ctx, db)? {
            if !rules::is_no_primary_sentinel(&c) {
                // Deduplicate by entity_id — stakeholder + domain rules could
                // both match the same account for the same participant.
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
    Ok(candidates)
}

// ---------------------------------------------------------------------------
// Full phase-3 + phase-4 runner
// ---------------------------------------------------------------------------

/// Run phases 3 and 4 inside the write transaction.
///
/// Phase 2 person-stub creation must already have been done by the caller
/// before entering this function.
pub fn run_phases(
    ctx: &ServiceContext<'_>,
    link_ctx: &LinkingContext,
    db: &ActionDb,
) -> Result<LinkOutcome, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let phase3_result = db.with_transaction(|_| {
        // CAS: read graph version inside the transaction. If it changed since
        // the adapter built ctx, retry phase3 once with the fresh snapshot
        // rather than writing stale links (spec: "retry once").
        let current_version = db.get_entity_graph_version().unwrap_or(link_ctx.graph_version);
        let refreshed_ctx: std::borrow::Cow<LinkingContext> = if current_version != link_ctx.graph_version {
            log::info!(
                "entity_linking: graph version changed ({} → {}), re-running phase3 \
                 with fresh snapshot",
                link_ctx.graph_version, current_version
            );
            let mut refreshed = link_ctx.clone();
            refreshed.graph_version = current_version;
            std::borrow::Cow::Owned(refreshed)
        } else {
            std::borrow::Cow::Borrowed(link_ctx)
        };
        let link_ctx = &*refreshed_ctx;

        let dismissals = db
            .get_linking_dismissals(link_ctx.owner.owner_type.as_str(), &link_ctx.owner.owner_id)
            .map_err(|e| format!("get_linking_dismissals: {e}"))?;

        let mut p3 = phase3(ctx, link_ctx, db)?;

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
        db.delete_auto_links_for_owner(link_ctx.owner.owner_type.as_str(), &link_ctx.owner.owner_id)
            .map_err(|e| format!("delete_auto_links: {e}"))?;

        // Write primary.
        if let Some(ref primary) = p3.primary_candidate {
            let source = format!("rule:{}", primary.rule_id);
            db.upsert_linked_entity_raw(&crate::db::entity_linking::LinkedEntityRawWrite {
                owner_type: link_ctx.owner.owner_type.as_str().to_string(),
                owner_id: link_ctx.owner.owner_id.clone(),
                entity_id: primary.entity.entity_id.clone(),
                entity_type: primary.entity.entity_type.clone(),
                role: "primary".to_string(),
                source,
                rule_id: Some(primary.rule_id.clone()),
                confidence: Some(primary.confidence),
                evidence_json: Some(primary.evidence.to_string()),
                graph_version: link_ctx.graph_version,
            })
            .map_err(|e| format!("upsert primary: {e}"))?;
        }

        // Write related chips.
        for related in &p3.related_candidates {
            let source = format!("rule:{}", related.rule_id);
            db.upsert_linked_entity_raw(&crate::db::entity_linking::LinkedEntityRawWrite {
                owner_type: link_ctx.owner.owner_type.as_str().to_string(),
                owner_id: link_ctx.owner.owner_id.clone(),
                entity_id: related.entity.entity_id.clone(),
                entity_type: related.entity.entity_type.clone(),
                role: related.role.as_str().to_string(),
                source,
                rule_id: Some(related.rule_id.clone()),
                confidence: Some(related.confidence),
                evidence_json: Some(related.evidence.to_string()),
                graph_version: link_ctx.graph_version,
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
            owner_type: link_ctx.owner.owner_type.as_str(),
            owner_id: &link_ctx.owner.owner_id,
            trigger: "evaluate",
            rule_id,
            entity_id: primary_ref.map(|c| c.entity.entity_id.as_str()),
            entity_type: primary_ref.map(|c| c.entity.entity_type.as_str()),
            role: primary_ref.map(|c| c.role.as_str()),
            graph_version: link_ctx.graph_version,
            evidence_json: &ev_json,
        })
        .map_err(|e| format!("insert_linking_evaluation: {e}"))?;

        Ok(p3)
    })?;

    // Phase 4 (cascade) runs after the write transaction commits.
    cascade::run_cascade(
        ctx,
        link_ctx,
        &phase3_result.primary_candidate,
        &phase3_result.related_candidates,
        db,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::types::{LinkRole, OwnerRef, OwnerType, Participant, ParticipantRole};
    use crate::db::test_utils::test_db;
    use crate::services::context::{ExternalClients, FixedClock, SeedableRng, ServiceContext};
    use chrono::{TimeZone, Utc};

    fn test_ctx<'a>(
        clock: &'a FixedClock,
        rng: &'a SeedableRng,
        ext: &'a ExternalClients,
    ) -> ServiceContext<'a> {
        ServiceContext::test_live(clock, rng, ext)
    }

    fn seed_account(db: &ActionDb, id: &str, name: &str) {
        db.conn_ref()
            .execute(
                "INSERT INTO accounts (id, name, updated_at, archived) VALUES (?1, ?2, '2026-01-01', 0)",
                rusqlite::params![id, name],
            )
            .expect("insert account");
    }

    fn seed_person(db: &ActionDb, id: &str, email: &str) {
        db.conn_ref()
            .execute(
                "INSERT INTO people (id, name, email, relationship, updated_at) \
                 VALUES (?1, ?1, ?2, 'external', '2026-01-01')",
                rusqlite::params![id, email],
            )
            .expect("insert person");
    }

    fn seed_active_stakeholder(db: &ActionDb, account_id: &str, person_id: &str) {
        db.conn_ref()
            .execute(
                "INSERT INTO account_stakeholders (account_id, person_id, data_source, status, confidence, created_at) \
                 VALUES (?1, ?2, 'test', 'active', 1.0, '2026-01-01')",
                rusqlite::params![account_id, person_id],
            )
            .expect("insert stakeholder");
    }

    fn one_on_one_ctx(person_id: &str) -> LinkingContext {
        LinkingContext {
            owner: OwnerRef {
                owner_type: OwnerType::Meeting,
                owner_id: "meeting-1".to_string(),
            },
            participants: vec![
                Participant {
                    email: "user@example.test".to_string(),
                    name: None,
                    role: ParticipantRole::Attendee,
                    person_id: Some("self".to_string()),
                    domain: Some("example.test".to_string()),
                },
                Participant {
                    email: "contact@external.test".to_string(),
                    name: None,
                    role: ParticipantRole::Attendee,
                    person_id: Some(person_id.to_string()),
                    domain: Some("external.test".to_string()),
                },
            ],
            title: Some("Working session".to_string()),
            attendee_count: 2,
            thread_id: None,
            series_id: None,
            graph_version: 0,
            user_domains: vec!["example.test".to_string()],
        }
    }

    #[test]
    fn p4_collection_surfaces_every_stakeholder_candidate_for_p9() {
        let db = test_db();
        seed_account(&db, "acc-a", "Account A");
        seed_account(&db, "acc-b", "Account B");
        seed_person(&db, "person-1", "contact@external.test");
        seed_active_stakeholder(&db, "acc-a", "person-1");
        seed_active_stakeholder(&db, "acc-b", "person-1");

        let ctx = one_on_one_ctx("person-1");
        let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 4, 30, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let service_ctx = test_ctx(&clock, &rng, &ext);
        let candidates = collect_p4_candidates(&service_ctx, &ctx, &db)
            .expect("collect candidates");
        let ids: Vec<_> = candidates
            .iter()
            .map(|candidate| candidate.entity.entity_id.as_str())
            .collect();

        assert_eq!(ids, vec!["acc-a", "acc-b"]);
    }

    #[test]
    fn phase3_turns_multi_account_stakeholder_evidence_into_p9() {
        let db = test_db();
        seed_account(&db, "acc-a", "Account A");
        seed_account(&db, "acc-b", "Account B");
        seed_person(&db, "person-1", "contact@external.test");
        seed_active_stakeholder(&db, "acc-a", "person-1");
        seed_active_stakeholder(&db, "acc-b", "person-1");

        let ctx = one_on_one_ctx("person-1");
        let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 4, 30, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let service_ctx = test_ctx(&clock, &rng, &ext);
        let result = phase3(&service_ctx, &ctx, &db).expect("phase3");

        assert!(result.primary_candidate.is_none());
        assert_eq!(result.applied_rule.as_deref(), Some("P9"));
        assert_eq!(result.related_candidates.len(), 2);
        assert!(result
            .related_candidates
            .iter()
            .all(|candidate| candidate.role == LinkRole::Related));
    }
}
