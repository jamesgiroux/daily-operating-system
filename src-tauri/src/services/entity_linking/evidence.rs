//! Evidence JSON builders for entity_linking_evaluations.
//!
//! Each function returns a serde_json::Value containing the full evidence blob
//! for a rule outcome. Load-bearing fields (plan-eng-review point 11):
//!   matched_text, rejected_candidates, parent_email_id, rule_input_fingerprint

use serde_json::{json, Value};

use super::types::{Candidate, LinkingContext};

/// Build a compact evidence blob for a rule that matched.
pub fn matched_evidence(
    ctx: &LinkingContext,
    matched_candidate: &Candidate,
    rejected: &[Candidate],
) -> Value {
    json!({
        "owner": { "type": ctx.owner.owner_type.as_str(), "id": ctx.owner.owner_id },
        "rule_id": matched_candidate.rule_id,
        "matched": {
            "entity_id": matched_candidate.entity.entity_id,
            "entity_type": matched_candidate.entity.entity_type,
            "confidence": matched_candidate.confidence,
        },
        "rejected_candidates": rejected.iter().map(|c| json!({
            "entity_id": c.entity.entity_id,
            "entity_type": c.entity.entity_type,
            "rule_id": c.rule_id,
            "confidence": c.confidence,
        })).collect::<Vec<_>>(),
        "participant_count": ctx.participants.len(),
        "attendee_count": ctx.attendee_count,
        "graph_version": ctx.graph_version,
    })
}

/// Build evidence for a rule that was skipped (did not match).
pub fn skip_evidence(ctx: &LinkingContext, rule_id: &str, reason: &str) -> Value {
    json!({
        "owner": { "type": ctx.owner.owner_type.as_str(), "id": ctx.owner.owner_id },
        "rule_id": rule_id,
        "skipped": true,
        "reason": reason,
        "graph_version": ctx.graph_version,
    })
}

/// Build evidence for thread inheritance (P2).
pub fn thread_inheritance_evidence(
    ctx: &LinkingContext,
    parent_email_id: &str,
    parent_entity_id: &str,
    domain_matched: bool,
) -> Value {
    json!({
        "owner": { "type": ctx.owner.owner_type.as_str(), "id": ctx.owner.owner_id },
        "rule_id": "P2",
        "parent_email_id": parent_email_id,
        "parent_entity_id": parent_entity_id,
        "domain_matched": domain_matched,
        "thread_id": ctx.thread_id,
        "graph_version": ctx.graph_version,
    })
}

/// Build evidence for title/subject matching (P5).
pub fn title_match_evidence(
    ctx: &LinkingContext,
    matched_text: &str,
    entity_id: &str,
    entity_name: &str,
    stoplist_blocked: bool,
    domain_consistent: bool,
) -> Value {
    json!({
        "owner": { "type": ctx.owner.owner_type.as_str(), "id": ctx.owner.owner_id },
        "rule_id": "P5",
        "matched_text": matched_text,
        "entity_id": entity_id,
        "entity_name": entity_name,
        "stoplist_blocked": stoplist_blocked,
        "domain_consistent": domain_consistent,
        "title": ctx.title,
        "graph_version": ctx.graph_version,
    })
}

/// Build evidence for suppress decisions (Phase 1).
pub fn suppress_evidence(ctx: &LinkingContext, reason: &str) -> Value {
    json!({
        "owner": { "type": ctx.owner.owner_type.as_str(), "id": ctx.owner.owner_id },
        "phase": "suppress",
        "reason": reason,
        "attendee_count": ctx.attendee_count,
    })
}
