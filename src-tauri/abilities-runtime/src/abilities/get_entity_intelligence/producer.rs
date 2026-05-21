//! DOS-459 — `get_entity_intelligence` envelope producer.
//!
//! Read-side composition over existing claim/proposal/open-loop substrate.
//! See `.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md` §5.1.
//!
//! W1 scope: envelope shape + facts + open_loops are wired to real readers.
//! Touchpoints / threads / record_entries / metadata_proposals return typed
//! empty states keyed to the producer that will fill them in later substrate
//! waves (DOS-460 touchpoints, DOS-297 threads, DOS-328 metadata proposals).
//! This matches the L0 contract: empty sections carry typed reasons; no
//! "Phase 2" stubs.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};

use super::contracts::{
    CandidateSetRef, ContextDepth as EnvelopeContextDepth, Cursor, CursorState, EmptyReason,
    EntityFact, EntityIntelligenceEnvelope, EntityIntelligenceInput, EntityKind,
    EnvelopeProvenance, EnvelopeProvenanceSource, EnvelopeSection, EnvelopeTrustSummary,
    ExclusionReason, Freshness, HealthStory, InclusionReason, MetadataProposal, NormalizedSubject,
    OpenLoopWithReceipt, Paginated, ProvenanceRef, ReceiptTargetRef, RecordEntry, SectionState,
    SubjectScope, ThreadSummary, Touchpoint, TouchpointBundle, TouchpointKind,
    ENVELOPE_SCHEMA_VERSION,
};
use crate::services::context::{
    EntityTouchpointSnapshot, EntityTouchpointsQuery, EntityTouchpointsReadError,
    EntityTouchpointsSnapshot, TouchpointInclusionReason,
};
use crate::abilities::list_open_loops::{ListOpenLoopsInput, OpenLoopSubject, OpenLoopsResult};
use crate::abilities::provenance::source_time::{parse_source_timestamp, SourceTimestampStatus};
use crate::abilities::provenance::trust::claim_trust_band_from_score;
use crate::abilities::provenance::{
    AbilityExecutionMode, AbilityVersion, FieldAttribution, FieldPath, ProvenanceBuilder,
    ProvenanceBuilderConfig, SchemaVersion, SubjectAttribution, SubjectRef,
};
use crate::abilities::trust::types::TrustBand;
use crate::abilities::{
    AbilityCategory, AbilityContext, AbilityError, AbilityErrorKind, AbilityResult, Actor,
};
use crate::sensitivity::{renderable_claim_text_with_value, RenderActor, RenderSurface};
use crate::types::{claim_allowed_for_prompt_input, IntelligenceClaim};

const ABILITY_NAME: &str = "get_entity_intelligence";

/// Page size cap applied to each list-shape field. Server-side cursor pagination
/// kicks in when the underlying reader returns more than this; W1 reads are bounded
/// by `ContextDepth.claim_levels()` so the cap is informational at this stage.
const DEFAULT_PAGE_SIZE: usize = 50;

pub async fn build_entity_intelligence(
    ctx: &AbilityContext<'_>,
    input: EntityIntelligenceInput,
) -> AbilityResult<EntityIntelligenceEnvelope> {
    validate_schema_version(input.schema_version)?;

    let entity_type_str = input.entity_type.as_lower_str().to_string();
    let entity_id = input.entity_id.trim();
    if entity_id.is_empty() {
        return Err(validation_error("entity_id must be non-empty"));
    }

    let subject_ref = subject_ref_for(input.entity_type.clone(), entity_id);
    let normalized_subject = NormalizedSubject {
        kind: input.entity_type.clone(),
        id: entity_id.to_string(),
        subject_ref: subject_ref.clone(),
        display_label: display_label_for(&input.entity_type, entity_id),
    };

    let active_sections = active_section_set(input.sections.as_ref());

    // ---- compose: facts -----------------------------------------------------
    let facts_active = active_sections.contains(&EnvelopeSection::Facts)
        || active_sections.contains(&EnvelopeSection::Record);
    let claims = if facts_active {
        read_claims(ctx, &entity_type_str, entity_id, &input.depth).await?
    } else {
        Vec::new()
    };
    let render_actor = render_actor_for_context(ctx);
    let render_surface = RenderSurface::TauriEntityDetail;

    let mut envelope_provenance = EnvelopeProvenance::empty();

    let facts = if active_sections.contains(&EnvelopeSection::Facts) {
        build_facts(&claims, &subject_ref, &render_actor, render_surface, &mut envelope_provenance)?
    } else {
        Paginated::empty_stable()
    };

    // ---- compose: open_loops -----------------------------------------------
    let open_loops = if active_sections.contains(&EnvelopeSection::OpenLoops) {
        compose_open_loops(ctx, &input.entity_type, entity_id, &mut envelope_provenance).await?
    } else {
        Paginated::empty_stable()
    };

    // ---- compose: record entries -------------------------------------------
    let record_entries = if active_sections.contains(&EnvelopeSection::Record) {
        build_record_entries(&claims, &render_actor, render_surface, &mut envelope_provenance)?
    } else {
        Paginated::empty_stable()
    };

    // ---- compose: touchpoints (DOS-460) ------------------------------------
    let touchpoints = if active_sections.contains(&EnvelopeSection::Touchpoints) {
        compose_touchpoints(
            ctx,
            &input.entity_type,
            entity_id,
            &subject_ref,
            &render_actor,
            &mut envelope_provenance,
        )
        .await?
    } else {
        not_requested_touchpoints_bundle(&subject_ref)
    };

    // ---- sections without producers yet — typed empties --------------------
    let metadata_proposals = empty_paginated_metadata_proposals();
    let threads = Paginated::<ThreadSummary>::empty_stable();
    let health_story: Option<HealthStory> = None;

    // ---- sections map enumerates ALL EnvelopeSection variants (AC-459.2) ---
    let sections_map = build_sections_map(
        &input.sections,
        SectionFill {
            facts_count: facts.items.len() as u64,
            health_present: health_story.is_some(),
            metadata_proposals_count: metadata_proposals.items.len() as u64,
            open_loops_count: open_loops.items.len() as u64,
            touchpoints_count: count_touchpoints(&touchpoints),
            threads_count: threads.items.len() as u64,
            record_entries_count: record_entries.items.len() as u64,
        },
    );

    // ---- aggregate trust summary -------------------------------------------
    let trust = aggregate_trust(&facts);

    // ---- aggregate sensitivity = max across facts (defaults to Public) -----
    let sensitivity = aggregate_sensitivity(&facts, &record_entries);

    let envelope = EntityIntelligenceEnvelope {
        schema_version: ENVELOPE_SCHEMA_VERSION,
        subject: normalized_subject.clone(),
        sections: sections_map,
        facts,
        health_story,
        metadata_proposals,
        open_loops,
        touchpoints,
        threads,
        record_entries,
        trust,
        provenance: envelope_provenance,
        sensitivity,
    };

    // Attach top-level `AbilityOutput` provenance so the runtime emits a
    // consistent envelope alongside its sibling abilities. The envelope's own
    // `provenance` field is the display-safe per-source index; the outer
    // `Provenance` records the producer call.
    let mut builder = ProvenanceBuilder::new(provenance_config(ctx, input.schema_version));
    let subject_attr = SubjectAttribution::direct_confident(subject_ref);
    builder.set_subject(subject_attr.clone());
    builder
        .attribute(
            FieldPath::new("/schemaVersion").map_err(field_error)?,
            FieldAttribution::constant(subject_attr),
        )
        .map_err(provenance_error)?;
    builder.finalize(envelope).map_err(provenance_error)
}

// ---- helpers ---------------------------------------------------------------

fn validate_schema_version(schema_version: u32) -> Result<(), AbilityError> {
    if schema_version == ENVELOPE_SCHEMA_VERSION {
        Ok(())
    } else {
        Err(validation_error(format!(
            "unsupported schema_version `{schema_version}` for `{ABILITY_NAME}`"
        )))
    }
}

fn subject_ref_for(entity_type: EntityKind, entity_id: &str) -> SubjectRef {
    match entity_type {
        EntityKind::Account => SubjectRef::Account(entity_id.to_string()),
        EntityKind::Project => SubjectRef::Project(entity_id.to_string()),
        EntityKind::Person => SubjectRef::Person(entity_id.to_string()),
    }
}

fn display_label_for(entity_type: &EntityKind, entity_id: &str) -> String {
    // L0 packet §5.1: display_label is part of NormalizedSubject. The renderer (W2)
    // composes friendly labels from claim store + this id; substrate-side we ship a
    // stable fallback that never leaks PII (the id is the user-owned identifier they
    // already chose).
    format!("{}:{}", entity_type.as_lower_str(), entity_id)
}

fn active_section_set(
    requested: Option<&Vec<EnvelopeSection>>,
) -> std::collections::BTreeSet<EnvelopeSection> {
    match requested {
        None => EnvelopeSection::ALL.iter().copied().collect(),
        Some(list) if list.is_empty() => EnvelopeSection::ALL.iter().copied().collect(),
        Some(list) => list.iter().copied().collect(),
    }
}

async fn read_claims(
    ctx: &AbilityContext<'_>,
    entity_type: &str,
    entity_id: &str,
    depth: &EnvelopeContextDepth,
) -> Result<Vec<IntelligenceClaim>, AbilityError> {
    let legacy_depth = depth.clone().into_legacy();
    let levels = match legacy_depth {
        crate::abilities::get_entity_context::ContextDepth::Shallow => 1,
        crate::abilities::get_entity_context::ContextDepth::Standard => 2,
        crate::abilities::get_entity_context::ContextDepth::Deep => 3,
    };
    let claims = ctx
        .services()
        .read_entity_context_claims(
            entity_type.to_string(),
            entity_id.to_string(),
            ctx.entity_context_claim_surface(),
            levels,
        )
        .await
        .map_err(|error| hard_error("entity_intelligence_claim_read_failed", error))?;
    Ok(filter_claims_for_actor(ctx.actor.clone(), claims))
}

fn filter_claims_for_actor(actor: Actor, claims: Vec<IntelligenceClaim>) -> Vec<IntelligenceClaim> {
    if matches!(actor, Actor::Agent) {
        claims
            .into_iter()
            .filter(claim_allowed_for_prompt_input)
            .collect()
    } else {
        claims
    }
}

fn render_actor_for_context(ctx: &AbilityContext<'_>) -> RenderActor {
    match &ctx.actor {
        Actor::User => RenderActor::user(ctx.services().actor, Some(ctx.services().actor)),
        Actor::Agent => RenderActor::agent("agent:get_entity_intelligence"),
        Actor::Admin => RenderActor {
            actor: "admin".to_string(),
            user_id: None,
        },
        Actor::System => RenderActor {
            actor: "system".to_string(),
            user_id: None,
        },
        Actor::SurfaceClient { .. } => RenderActor {
            actor: "surface_client".to_string(),
            user_id: None,
        },
        Actor::McpClient { .. } => RenderActor::agent("mcp_client"),
    }
}

// ---- facts -----------------------------------------------------------------

fn build_facts(
    claims: &[IntelligenceClaim],
    subject_ref: &SubjectRef,
    render_actor: &RenderActor,
    render_surface: RenderSurface,
    provenance: &mut EnvelopeProvenance,
) -> Result<Paginated<EntityFact>, AbilityError> {
    let mut items = Vec::new();
    for claim in claims.iter().take(DEFAULT_PAGE_SIZE) {
        let Some(rendered_text) =
            renderable_claim_text_with_value(claim, &claim.text, render_surface, render_actor)
        else {
            continue;
        };
        let source_id = upsert_provenance_source(provenance, claim);
        items.push(EntityFact {
            claim_id: claim.id.clone(),
            subject_ref: subject_ref.clone(),
            field_path: claim.field_path.clone(),
            claim_type: claim.claim_type.clone(),
            rendered_text,
            trust_band: claim_trust_band_from_score(claim.trust_score),
            freshness: freshness_for_claim(claim),
            source_asof: parse_optional_timestamp(claim.source_asof.as_deref()),
            sensitivity: claim.sensitivity.clone(),
            lifecycle_state: claim.claim_state.clone(),
            surfacing_state: claim.surfacing_state.clone(),
            verification_state: claim.verification_state,
            provenance: ProvenanceRef::from_ids([source_id]),
        });
    }
    let next_cursor = if claims.len() > DEFAULT_PAGE_SIZE {
        Some(Cursor::new(format!("facts:offset={DEFAULT_PAGE_SIZE}")))
    } else {
        None
    };
    let total_hint = Some(claims.len() as u64);
    Ok(Paginated {
        items,
        next_cursor,
        total_hint,
        cursor_state: CursorState::Stable,
    })
}

// ---- open loops ------------------------------------------------------------

async fn compose_open_loops(
    ctx: &AbilityContext<'_>,
    entity_type: &EntityKind,
    entity_id: &str,
    provenance: &mut EnvelopeProvenance,
) -> Result<Paginated<OpenLoopWithReceipt>, AbilityError> {
    let input = ListOpenLoopsInput {
        schema_version: 1,
        entity_type: Some(entity_type.as_lower_str().to_string()),
        entity_id: Some(entity_id.to_string()),
    };
    let snapshot = match crate::abilities::list_open_loops::list_open_loops(ctx, input).await {
        Ok(output) => output.into_data(),
        Err(AbilityError {
            kind: AbilityErrorKind::SubjectNotOwned,
            ..
        }) => {
            // Subject not owned by this workspace — surface as empty section, not envelope failure.
            return Ok(Paginated::empty_stable());
        }
        Err(other) => return Err(other),
    };

    let OpenLoopsResult { loops, .. } = snapshot;
    let items = loops
        .into_iter()
        .map(|open_loop| {
            let receipt_target = ReceiptTargetRef {
                claim_id: open_loop.id.clone(),
                subject_ref: subject_ref_for_open_loop(&open_loop.subject),
                field_path: None,
            };
            let label = format!("open_loop:{}", open_loop.id);
            let source_id = upsert_static_provenance_source(
                provenance,
                EnvelopeProvenanceSource {
                    id: label.clone(),
                    label: open_loop.loop_kind.clone(),
                    source_type: Some("open_loop".to_string()),
                    as_of: parse_optional_timestamp(open_loop.source_asof.as_deref()),
                    redacted: false,
                },
            );
            OpenLoopWithReceipt {
                open_loop,
                receipt_target,
                trust_band: TrustBand::Unscored,
                freshness: Freshness::Unknown,
                provenance: ProvenanceRef::from_ids([source_id]),
            }
        })
        .collect::<Vec<_>>();
    Ok(Paginated::stable(items))
}

fn subject_ref_for_open_loop(subject: &OpenLoopSubject) -> SubjectRef {
    match subject.entity_type.as_str() {
        "account" => SubjectRef::Account(subject.entity_id.clone()),
        "project" => SubjectRef::Project(subject.entity_id.clone()),
        "person" => SubjectRef::Person(subject.entity_id.clone()),
        "meeting" => SubjectRef::Meeting(subject.entity_id.clone()),
        _ => SubjectRef::Unknown,
    }
}

// ---- record entries --------------------------------------------------------

fn build_record_entries(
    claims: &[IntelligenceClaim],
    render_actor: &RenderActor,
    render_surface: RenderSurface,
    provenance: &mut EnvelopeProvenance,
) -> Result<Paginated<RecordEntry>, AbilityError> {
    let mut items = Vec::new();
    for claim in claims.iter().take(DEFAULT_PAGE_SIZE) {
        let Some(rendered_text) =
            renderable_claim_text_with_value(claim, &claim.text, render_surface, render_actor)
        else {
            continue;
        };
        let source_id = upsert_provenance_source(provenance, claim);
        let recorded_at = parse_optional_timestamp(Some(claim.observed_at.as_str()))
            .or_else(|| parse_optional_timestamp(Some(claim.created_at.as_str())))
            .unwrap_or_else(Utc::now);
        items.push(RecordEntry {
            claim_id: claim.id.clone(),
            subject_ref: subject_ref_from_claim(claim),
            claim_type: claim.claim_type.clone(),
            recorded_at,
            rendered_text,
            trust_band: claim_trust_band_from_score(claim.trust_score),
            sensitivity: claim.sensitivity.clone(),
            provenance: ProvenanceRef::from_ids([source_id]),
        });
    }
    Ok(Paginated::stable(items))
}

fn subject_ref_from_claim(claim: &IntelligenceClaim) -> SubjectRef {
    // Best-effort parse — fall back to Unknown rather than failing the envelope.
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&claim.subject_ref) else {
        return SubjectRef::Unknown;
    };
    match crate::types::subject_ref_from_json(&value) {
        Ok(crate::types::ClaimSubjectRef::Account { id }) => SubjectRef::Account(id),
        Ok(crate::types::ClaimSubjectRef::Project { id }) => SubjectRef::Project(id),
        Ok(crate::types::ClaimSubjectRef::Person { id }) => SubjectRef::Person(id),
        Ok(crate::types::ClaimSubjectRef::Meeting { id }) => SubjectRef::Meeting(id),
        _ => SubjectRef::Unknown,
    }
}

// ---- empty section helpers (touchpoints, metadata proposals) ---------------

fn empty_paginated_metadata_proposals() -> Paginated<MetadataProposal> {
    Paginated::empty_stable()
}

fn not_requested_touchpoints_bundle(subject_ref: &SubjectRef) -> Paginated<TouchpointBundle> {
    let bundle = TouchpointBundle {
        upcoming: Paginated::empty_stable(),
        recent: Paginated::empty_stable(),
        candidate_set: CandidateSetRef {
            window_start: None,
            window_end: None,
            filter_description: "touchpoints section not requested".to_string(),
        },
        empty_reason: Some(EmptyReason::NotRequested),
        subject_scope: SubjectScope {
            primary: subject_ref.clone(),
            also_includes: Vec::new(),
        },
    };
    Paginated::stable(vec![bundle])
}

// ---- touchpoints (DOS-460) -----------------------------------------------

/// Default upcoming window in days. Matches the `today + horizon` convention
/// used by daily briefing readiness — long enough to surface the next-week
/// cadence touchpoint, short enough to keep the bundle bounded.
const TOUCHPOINTS_UPCOMING_WINDOW_DAYS: u16 = 14;
/// Default recent window in days. Matches the "what happened recently" surface
/// expectation used by entity-detail blocks.
const TOUCHPOINTS_RECENT_WINDOW_DAYS: u16 = 30;
const TOUCHPOINTS_PER_SIDE_CAP: usize = 25;

async fn compose_touchpoints(
    ctx: &AbilityContext<'_>,
    entity_type: &EntityKind,
    entity_id: &str,
    subject_ref: &SubjectRef,
    render_actor: &RenderActor,
    provenance: &mut EnvelopeProvenance,
) -> Result<Paginated<TouchpointBundle>, AbilityError> {
    let now = ctx.services().clock.now();
    let query = EntityTouchpointsQuery {
        entity_type: entity_type.as_lower_str().to_string(),
        entity_id: entity_id.to_string(),
        now,
        upcoming_window_days: TOUCHPOINTS_UPCOMING_WINDOW_DAYS,
        recent_window_days: TOUCHPOINTS_RECENT_WINDOW_DAYS,
        per_side_cap: TOUCHPOINTS_PER_SIDE_CAP,
    };

    let snapshot = match ctx.services().read_entity_touchpoints(query).await {
        Ok(snapshot) => snapshot,
        Err(EntityTouchpointsReadError::SubjectNotOwned { .. }) => {
            return Ok(Paginated::stable(vec![filtered_out_touchpoints_bundle(
                subject_ref,
                &now,
            )]));
        }
        Err(EntityTouchpointsReadError::ReadFailed(message)) => {
            return Ok(Paginated::stable(vec![read_failed_touchpoints_bundle(
                subject_ref,
                &now,
                message,
            )]));
        }
    };

    let bundle = project_touchpoints_bundle(&snapshot, subject_ref, &now, render_actor, provenance);
    Ok(Paginated::stable(vec![bundle]))
}

fn project_touchpoints_bundle(
    snapshot: &EntityTouchpointsSnapshot,
    subject_ref: &SubjectRef,
    now: &DateTime<Utc>,
    render_actor: &RenderActor,
    provenance: &mut EnvelopeProvenance,
) -> TouchpointBundle {
    let upcoming_items: Vec<Touchpoint> = snapshot
        .upcoming
        .iter()
        .map(|raw| project_touchpoint(raw, now, render_actor, provenance))
        .collect();
    let recent_items: Vec<Touchpoint> = snapshot
        .recent
        .iter()
        .map(|raw| project_touchpoint(raw, now, render_actor, provenance))
        .collect();

    let upcoming = Paginated::stable(upcoming_items);
    let recent = Paginated::stable(recent_items);

    let window_start =
        Some(*now - chrono::Duration::days(i64::from(TOUCHPOINTS_RECENT_WINDOW_DAYS)));
    let window_end =
        Some(*now + chrono::Duration::days(i64::from(TOUCHPOINTS_UPCOMING_WINDOW_DAYS)));

    let candidate_set = CandidateSetRef {
        window_start,
        window_end,
        filter_description: snapshot.filter_description.clone(),
    };

    let also_includes = snapshot
        .also_includes
        .iter()
        .map(|(ty, id)| subject_ref_from_pair(ty, id))
        .collect::<Vec<_>>();

    let empty_reason = if upcoming.items.is_empty() && recent.items.is_empty() {
        Some(EmptyReason::NoRelevantTouchpoints)
    } else {
        None
    };

    TouchpointBundle {
        upcoming,
        recent,
        candidate_set,
        empty_reason,
        subject_scope: SubjectScope {
            primary: subject_ref.clone(),
            also_includes,
        },
    }
}

fn project_touchpoint(
    raw: &EntityTouchpointSnapshot,
    now: &DateTime<Utc>,
    render_actor: &RenderActor,
    provenance: &mut EnvelopeProvenance,
) -> Touchpoint {
    let when = parse_optional_timestamp(raw.starts_at.as_deref()).unwrap_or(*now);
    let id_label = format!("meeting:{}", raw.meeting_id);
    // F2 (L3 cycle-2): touchpoint provenance label routes through an audience-aware
    // scrub before emission. UserTauri audience (RenderActor.is_user()) sees the
    // full meeting title; AgentMcp / agent surfaces see a redacted placeholder.
    // ADR-0108 §3 — provenance labels are themselves user-visible strings and
    // MUST honour the per-audience allowlist gate.
    let (label, redacted) = if render_actor.is_user() {
        (raw.title.clone(), false)
    } else {
        ("Meeting (redacted)".to_string(), true)
    };
    let source_id = upsert_static_provenance_source(
        provenance,
        EnvelopeProvenanceSource {
            id: id_label,
            label,
            source_type: Some("meeting".to_string()),
            as_of: parse_optional_timestamp(raw.source_asof.as_deref()),
            redacted,
        },
    );
    let inclusion_reason = match raw.inclusion_reason {
        TouchpointInclusionReason::SubjectMatch => InclusionReason::SubjectMatch,
        TouchpointInclusionReason::EntityLink => InclusionReason::EntityLink,
        TouchpointInclusionReason::AttendeeMatch => InclusionReason::AttendeeMatch,
        TouchpointInclusionReason::DomainMatch => InclusionReason::DomainMatch,
    };
    let exclusion_reason = raw
        .exclusion_reason
        .as_deref()
        .and_then(parse_exclusion_reason);
    Touchpoint {
        meeting_id: Some(raw.meeting_id.clone()),
        kind: classify_touchpoint_kind(&raw.kind),
        when,
        subject_ref: subject_ref_from_pair(&raw.subject_entity_type, &raw.subject_entity_id),
        inclusion_reason,
        exclusion_reason,
        trust_band: TrustBand::Unscored,
        freshness: freshness_for_meeting(raw, now),
        provenance: ProvenanceRef::from_ids([source_id]),
    }
}

fn classify_touchpoint_kind(raw: &str) -> TouchpointKind {
    let lowered = raw.to_ascii_lowercase();
    if lowered.contains("email") {
        TouchpointKind::EmailThread
    } else if lowered.contains("salesforce") {
        TouchpointKind::Salesforce
    } else if lowered.contains("linear") {
        TouchpointKind::Linear
    } else if lowered.contains("doc") {
        TouchpointKind::Document
    } else {
        TouchpointKind::Meeting
    }
}

fn parse_exclusion_reason(raw: &str) -> Option<ExclusionReason> {
    match raw {
        "subject_mismatch" => Some(ExclusionReason::SubjectMismatch),
        "outside_window" => Some(ExclusionReason::OutsideWindow),
        "low_confidence" => Some(ExclusionReason::LowConfidence),
        "suppressed" => Some(ExclusionReason::Suppressed),
        _ => None,
    }
}

fn freshness_for_meeting(raw: &EntityTouchpointSnapshot, now: &DateTime<Utc>) -> Freshness {
    let Some(when) = parse_optional_timestamp(raw.starts_at.as_deref()) else {
        return Freshness::Unknown;
    };
    if when > *now {
        Freshness::Current
    } else {
        let age = now.signed_duration_since(when);
        if age.num_days() < 7 {
            Freshness::Current
        } else if age.num_days() < 30 {
            Freshness::Aging
        } else {
            Freshness::Stale
        }
    }
}

fn subject_ref_from_pair(entity_type: &str, entity_id: &str) -> SubjectRef {
    match entity_type {
        "account" => SubjectRef::Account(entity_id.to_string()),
        "project" => SubjectRef::Project(entity_id.to_string()),
        "person" => SubjectRef::Person(entity_id.to_string()),
        "meeting" => SubjectRef::Meeting(entity_id.to_string()),
        _ => SubjectRef::Unknown,
    }
}

fn filtered_out_touchpoints_bundle(
    subject_ref: &SubjectRef,
    now: &DateTime<Utc>,
) -> TouchpointBundle {
    TouchpointBundle {
        upcoming: Paginated::empty_stable(),
        recent: Paginated::empty_stable(),
        candidate_set: CandidateSetRef {
            window_start: Some(
                *now - chrono::Duration::days(i64::from(TOUCHPOINTS_RECENT_WINDOW_DAYS)),
            ),
            window_end: Some(
                *now + chrono::Duration::days(i64::from(TOUCHPOINTS_UPCOMING_WINDOW_DAYS)),
            ),
            filter_description: "subject filtered out by workspace scope".to_string(),
        },
        empty_reason: Some(EmptyReason::FilteredOutBySubject),
        subject_scope: SubjectScope {
            primary: subject_ref.clone(),
            also_includes: Vec::new(),
        },
    }
}

fn read_failed_touchpoints_bundle(
    subject_ref: &SubjectRef,
    now: &DateTime<Utc>,
    message: String,
) -> TouchpointBundle {
    TouchpointBundle {
        upcoming: Paginated::empty_stable(),
        recent: Paginated::empty_stable(),
        candidate_set: CandidateSetRef {
            window_start: Some(
                *now - chrono::Duration::days(i64::from(TOUCHPOINTS_RECENT_WINDOW_DAYS)),
            ),
            window_end: Some(
                *now + chrono::Duration::days(i64::from(TOUCHPOINTS_UPCOMING_WINDOW_DAYS)),
            ),
            filter_description: message,
        },
        empty_reason: Some(EmptyReason::PartialFailure {
            advisory: "touchpoints reader unavailable".to_string(),
        }),
        subject_scope: SubjectScope {
            primary: subject_ref.clone(),
            also_includes: Vec::new(),
        },
    }
}

fn count_touchpoints(touchpoints: &Paginated<TouchpointBundle>) -> u64 {
    touchpoints
        .items
        .iter()
        .map(|bundle| (bundle.upcoming.items.len() + bundle.recent.items.len()) as u64)
        .sum()
}

// ---- sections map ----------------------------------------------------------

struct SectionFill {
    facts_count: u64,
    health_present: bool,
    metadata_proposals_count: u64,
    open_loops_count: u64,
    touchpoints_count: u64,
    threads_count: u64,
    record_entries_count: u64,
}

fn build_sections_map(
    requested: &Option<Vec<EnvelopeSection>>,
    fill: SectionFill,
) -> BTreeMap<EnvelopeSection, SectionState> {
    // Normalize the requested list to match `active_section_set()` semantics:
    // both `None` and `Some(empty)` mean "all sections" — never "none requested".
    // Without this, callers that pass `sections: []` get every entry marked
    // `NotRequested` here even though `active_section_set()` populates them.
    let requested_set: Option<std::collections::BTreeSet<EnvelopeSection>> =
        requested.as_ref().and_then(|list| {
            if list.is_empty() {
                None
            } else {
                Some(list.iter().copied().collect())
            }
        });
    let mut sections = BTreeMap::new();
    for section in EnvelopeSection::ALL {
        let state = if let Some(set) = requested_set.as_ref() {
            if !set.contains(section) {
                SectionState::Empty {
                    reason: EmptyReason::NotRequested,
                }
            } else {
                section_state_for(section, &fill)
            }
        } else {
            section_state_for(section, &fill)
        };
        sections.insert(*section, state);
    }
    sections
}

fn section_state_for(section: &EnvelopeSection, fill: &SectionFill) -> SectionState {
    match section {
        EnvelopeSection::Facts => count_or_empty(fill.facts_count, EmptyReason::NotProcessedYet),
        EnvelopeSection::Health => {
            if fill.health_present {
                SectionState::Present { item_count: 1 }
            } else {
                SectionState::Empty {
                    reason: EmptyReason::NotProcessedYet,
                }
            }
        }
        EnvelopeSection::MetadataProposals => count_or_empty(
            fill.metadata_proposals_count,
            EmptyReason::NoEvidenceBackedProposal,
        ),
        EnvelopeSection::OpenLoops => count_or_empty(fill.open_loops_count, EmptyReason::Stale),
        EnvelopeSection::Touchpoints => {
            count_or_empty(fill.touchpoints_count, EmptyReason::NoRelevantTouchpoints)
        }
        EnvelopeSection::Threads => {
            count_or_empty(fill.threads_count, EmptyReason::NotProcessedYet)
        }
        EnvelopeSection::Record => {
            count_or_empty(fill.record_entries_count, EmptyReason::NotProcessedYet)
        }
    }
}

fn count_or_empty(count: u64, empty_reason: EmptyReason) -> SectionState {
    if count == 0 {
        SectionState::Empty {
            reason: empty_reason,
        }
    } else {
        SectionState::Present { item_count: count }
    }
}

// ---- aggregate trust + sensitivity ----------------------------------------

fn aggregate_trust(facts: &Paginated<EntityFact>) -> EnvelopeTrustSummary {
    if facts.items.is_empty() {
        return EnvelopeTrustSummary::unscored();
    }
    let aggregate_band = facts
        .items
        .iter()
        .map(|fact| fact.trust_band)
        .reduce(min_trust_band)
        .unwrap_or(TrustBand::Unscored);
    EnvelopeTrustSummary {
        aggregate_band,
        section_caveats: BTreeMap::new(),
    }
}

fn min_trust_band(left: TrustBand, right: TrustBand) -> TrustBand {
    // "Min" = least trusted of the pair. Order: LikelyCurrent > UseWithCaution > NeedsVerification > Unscored.
    fn rank(band: TrustBand) -> u8 {
        match band {
            TrustBand::LikelyCurrent => 3,
            TrustBand::UseWithCaution => 2,
            TrustBand::NeedsVerification => 1,
            TrustBand::Unscored => 0,
        }
    }
    if rank(left) <= rank(right) {
        left
    } else {
        right
    }
}

fn aggregate_sensitivity(
    facts: &Paginated<EntityFact>,
    record_entries: &Paginated<RecordEntry>,
) -> crate::types::ClaimSensitivity {
    use crate::types::ClaimSensitivity::*;
    fn rank(s: &crate::types::ClaimSensitivity) -> u8 {
        match s {
            Public => 0,
            Internal => 1,
            Confidential => 2,
            UserOnly => 3,
        }
    }
    let mut max_rank = 0u8;
    let mut max = Public;
    for fact in facts.items.iter() {
        let r = rank(&fact.sensitivity);
        if r > max_rank {
            max_rank = r;
            max = fact.sensitivity.clone();
        }
    }
    for entry in record_entries.items.iter() {
        let r = rank(&entry.sensitivity);
        if r > max_rank {
            max_rank = r;
            max = entry.sensitivity.clone();
        }
    }
    max
}

// ---- provenance index helpers ---------------------------------------------

fn upsert_provenance_source(provenance: &mut EnvelopeProvenance, claim: &IntelligenceClaim) -> String {
    let id = format!("claim_source:{}", claim.id);
    if provenance.sources.iter().any(|s| s.id == id) {
        return id;
    }
    let source = EnvelopeProvenanceSource {
        id: id.clone(),
        label: claim.data_source.clone(),
        source_type: Some(claim.data_source.clone()),
        as_of: parse_optional_timestamp(claim.source_asof.as_deref()),
        // W1 producer is naive about redaction — the W2 projection layer composes
        // `redact_provenance_for_surface` per DOS-477. Substrate marks redacted=false
        // here; the projection layer flips it when applying surface-specific gates.
        redacted: false,
    };
    provenance.sources.push(source);
    id
}

fn upsert_static_provenance_source(
    provenance: &mut EnvelopeProvenance,
    candidate: EnvelopeProvenanceSource,
) -> String {
    if let Some(existing) = provenance.sources.iter().find(|s| s.id == candidate.id) {
        return existing.id.clone();
    }
    let id = candidate.id.clone();
    provenance.sources.push(candidate);
    id
}

// ---- freshness ------------------------------------------------------------

fn freshness_for_claim(claim: &IntelligenceClaim) -> Freshness {
    let Some(source_asof) = parse_optional_timestamp(claim.source_asof.as_deref()) else {
        return Freshness::Unknown;
    };
    let now = Utc::now();
    let age = now.signed_duration_since(source_asof);
    if age.num_days() < 7 {
        Freshness::Current
    } else if age.num_days() < 30 {
        Freshness::Aging
    } else {
        Freshness::Stale
    }
}

fn parse_optional_timestamp(candidate: Option<&str>) -> Option<DateTime<Utc>> {
    match parse_source_timestamp(candidate, Utc::now(), None) {
        SourceTimestampStatus::Accepted(parsed)
        | SourceTimestampStatus::Implausible { parsed, .. } => Some(parsed),
        SourceTimestampStatus::Malformed(_) | SourceTimestampStatus::Missing => None,
    }
}

// ---- provenance config ----------------------------------------------------

fn provenance_config(ctx: &AbilityContext<'_>, schema_version: u32) -> ProvenanceBuilderConfig {
    let mut config = ProvenanceBuilderConfig::new(ABILITY_NAME, ctx.services().clock.now());
    config.ability_version = AbilityVersion::new(0, 1);
    config.ability_schema_version = SchemaVersion(schema_version);
    config.actor = provenance_actor(ctx.actor.clone());
    config.mode = AbilityExecutionMode::from(ctx.mode());
    config.category = AbilityCategory::Read;
    config
}

fn provenance_actor(actor: Actor) -> crate::abilities::provenance::Actor {
    match actor {
        Actor::User => crate::abilities::provenance::Actor::User,
        Actor::Agent => crate::abilities::provenance::Actor::Agent {
            name: "agent".to_string(),
            version: "unknown".to_string(),
        },
        Actor::Admin => crate::abilities::provenance::Actor::Human {
            role: "admin".to_string(),
            id: "admin".to_string(),
        },
        Actor::System => crate::abilities::provenance::Actor::System {
            component: "dailyos".to_string(),
        },
        Actor::SurfaceClient { .. } => crate::abilities::provenance::Actor::System {
            component: "surface_client".to_string(),
        },
        Actor::McpClient { .. } => crate::abilities::provenance::Actor::Agent {
            name: "mcp".to_string(),
            version: "unknown".to_string(),
        },
    }
}

// ---- errors ----------------------------------------------------------------

fn validation_error(message: impl Into<String>) -> AbilityError {
    AbilityError {
        kind: AbilityErrorKind::Validation,
        message: message.into(),
    }
}

fn hard_error(code: impl Into<String>, message: impl Into<String>) -> AbilityError {
    AbilityError {
        kind: AbilityErrorKind::HardError(code.into()),
        message: message.into(),
    }
}

fn provenance_error(error: impl std::fmt::Display) -> AbilityError {
    validation_error(format!("provenance construction failed: {error}"))
}

fn field_error(error: impl std::fmt::Display) -> AbilityError {
    validation_error(format!("field attribution path failed: {error}"))
}

// ---- tests -----------------------------------------------------------------

#[cfg(test)]
mod tests {
    //! Pure shape + section-state tests for the DOS-459 envelope. The full
    //! runtime test using `AbilityContext` lives in the DOS-461 harness (W1
    //! sibling). These tests verify the cycle-1 architecture F5 + correctness F4
    //! contracts — every list-shape field is `Paginated<T>` with `CursorState`,
    //! empty sections carry typed reasons, sections map enumerates all variants.

    use super::*;
    use super::super::contracts::{
        ExclusionReason, InclusionReason, Touchpoint, TouchpointKind,
    };

    fn fill(facts: u64, open_loops: u64) -> SectionFill {
        SectionFill {
            facts_count: facts,
            health_present: false,
            metadata_proposals_count: 0,
            open_loops_count: open_loops,
            touchpoints_count: 0,
            threads_count: 0,
            record_entries_count: 0,
        }
    }

    #[test]
    fn sections_map_enumerates_all_variants_when_no_filter() {
        let map = build_sections_map(&None, fill(0, 0));
        assert_eq!(map.len(), EnvelopeSection::ALL.len());
        for section in EnvelopeSection::ALL {
            assert!(map.contains_key(section), "missing section {section:?}");
        }
    }

    #[test]
    fn empty_sections_carry_typed_reasons() {
        let map = build_sections_map(&None, fill(0, 0));
        for state in map.values() {
            match state {
                SectionState::Empty { reason } => {
                    // Reason is one of the typed variants — never null.
                    let _ = reason;
                }
                SectionState::Present { .. } => panic!("expected empty in zero-fill case"),
            }
        }
    }

    #[test]
    fn requested_subset_marks_excluded_as_not_requested() {
        let map = build_sections_map(&Some(vec![EnvelopeSection::Facts]), fill(2, 0));
        assert!(matches!(
            map.get(&EnvelopeSection::Facts),
            Some(SectionState::Present { item_count: 2 })
        ));
        assert!(matches!(
            map.get(&EnvelopeSection::OpenLoops),
            Some(SectionState::Empty {
                reason: EmptyReason::NotRequested
            })
        ));
    }

    #[test]
    fn empty_sections_list_treated_as_all_sections() {
        // Regression: `sections: Some(vec![])` must match `active_section_set()`
        // semantics — both `None` and `Some(empty)` mean "all sections". Without
        // this, callers asking for "all" via empty list see every section
        // marked `NotRequested` here while `active_section_set()` happily
        // populates the corresponding fields, causing producer/consumer drift.
        let map_empty = build_sections_map(&Some(vec![]), fill(3, 0));
        let map_none = build_sections_map(&None, fill(3, 0));
        assert_eq!(map_empty, map_none, "empty list must equal None semantics");
        // Facts has count=3, so it should be Present (not NotRequested) when
        // caller passes the empty-list form.
        assert!(matches!(
            map_empty.get(&EnvelopeSection::Facts),
            Some(SectionState::Present { item_count: 3 })
        ));
        // None of the variants should be `NotRequested` in the empty-list case.
        for (section, state) in &map_empty {
            if let SectionState::Empty {
                reason: EmptyReason::NotRequested,
            } = state
            {
                panic!("section {section:?} marked NotRequested for empty-list input");
            }
        }
    }

    #[test]
    fn empty_paginated_is_stable_with_no_cursor() {
        let p: Paginated<EntityFact> = Paginated::empty_stable();
        assert!(p.items.is_empty());
        assert!(p.next_cursor.is_none());
        assert!(matches!(p.cursor_state, CursorState::Stable));
        assert_eq!(p.total_hint, Some(0));
    }

    #[test]
    fn cursor_state_invalidated_requires_restart() {
        let state = CursorState::Invalidated {
            reason: "schema changed".to_string(),
            restart_required: true,
        };
        match state {
            CursorState::Invalidated {
                restart_required, ..
            } => assert!(restart_required),
            _ => panic!("expected Invalidated"),
        }
    }

    #[test]
    fn cursor_state_data_shifted_carries_advisory() {
        let state = CursorState::DataShifted {
            advisory: "rows shifted under cursor".to_string(),
        };
        match state {
            CursorState::DataShifted { advisory } => assert!(!advisory.is_empty()),
            _ => panic!("expected DataShifted"),
        }
    }

    #[test]
    fn envelope_section_all_is_canonical_order() {
        let all = EnvelopeSection::ALL;
        assert_eq!(all[0], EnvelopeSection::Facts);
        assert_eq!(all[1], EnvelopeSection::Health);
        assert_eq!(all[6], EnvelopeSection::Record);
    }

    #[test]
    fn min_trust_band_picks_least_trusted() {
        assert_eq!(
            min_trust_band(TrustBand::LikelyCurrent, TrustBand::NeedsVerification),
            TrustBand::NeedsVerification
        );
        assert_eq!(
            min_trust_band(TrustBand::Unscored, TrustBand::LikelyCurrent),
            TrustBand::Unscored
        );
    }

    #[test]
    fn touchpoint_kind_round_trip_through_serde() {
        let original = Touchpoint {
            meeting_id: Some("m-1".to_string()),
            kind: TouchpointKind::Meeting,
            when: chrono::Utc::now(),
            subject_ref: SubjectRef::Account("a-1".to_string()),
            inclusion_reason: InclusionReason::SubjectMatch,
            exclusion_reason: Some(ExclusionReason::OutsideWindow),
            trust_band: TrustBand::LikelyCurrent,
            freshness: Freshness::Current,
            provenance: ProvenanceRef::empty(),
        };
        let json = serde_json::to_string(&original).expect("serializes");
        let parsed: Touchpoint = serde_json::from_str(&json).expect("round trips");
        assert_eq!(parsed.inclusion_reason, InclusionReason::SubjectMatch);
        assert_eq!(parsed.exclusion_reason, Some(ExclusionReason::OutsideWindow));
    }

    #[test]
    fn entity_kind_lowercase_str_matches_legacy() {
        assert_eq!(EntityKind::Account.as_lower_str(), "account");
        assert_eq!(EntityKind::Project.as_lower_str(), "project");
        assert_eq!(EntityKind::Person.as_lower_str(), "person");
    }

    // ---- DOS-460 — touchpoint projection + subject-isolation tests --------

    fn fake_snapshot(
        entity_type: &str,
        entity_id: &str,
        upcoming: Vec<EntityTouchpointSnapshot>,
        recent: Vec<EntityTouchpointSnapshot>,
        also_includes: Vec<(String, String)>,
    ) -> EntityTouchpointsSnapshot {
        EntityTouchpointsSnapshot {
            subject_entity_type: entity_type.to_string(),
            subject_entity_id: entity_id.to_string(),
            upcoming,
            recent,
            also_includes,
            filter_description: format!("{entity_type}:{entity_id} test fixture"),
        }
    }

    fn fake_touchpoint(
        meeting_id: &str,
        entity_type: &str,
        entity_id: &str,
        inclusion: TouchpointInclusionReason,
        when: chrono::DateTime<chrono::Utc>,
    ) -> EntityTouchpointSnapshot {
        EntityTouchpointSnapshot {
            meeting_id: meeting_id.to_string(),
            title: format!("Meeting {meeting_id}"),
            kind: "internal".to_string(),
            starts_at: Some(when.to_rfc3339()),
            ends_at: None,
            subject_entity_type: entity_type.to_string(),
            subject_entity_id: entity_id.to_string(),
            inclusion_reason: inclusion,
            exclusion_reason: None,
            source_asof: Some(when.to_rfc3339()),
        }
    }

    #[test]
    fn project_touchpoints_empty_snapshot_emits_no_relevant_touchpoints() {
        let snapshot = fake_snapshot("account", "acc-1", vec![], vec![], vec![]);
        let mut prov = EnvelopeProvenance::empty();
        let render_actor = RenderActor::user("user", None::<String>);
        let bundle = project_touchpoints_bundle(
            &snapshot,
            &SubjectRef::Account("acc-1".to_string()),
            &chrono::Utc::now(),
            &render_actor,
            &mut prov,
        );
        assert_eq!(bundle.empty_reason, Some(EmptyReason::NoRelevantTouchpoints));
        assert!(bundle.upcoming.items.is_empty());
        assert!(bundle.recent.items.is_empty());
        assert!(bundle.subject_scope.also_includes.is_empty());
    }

    #[test]
    fn project_touchpoints_parent_child_account_scope_appears_in_also_includes() {
        // Parent account "acc-parent" pulled in via scope expansion includes a
        // child-account meeting tagged EntityLink — projection must reflect that
        // scope expansion in `subject_scope.also_includes` so consumers can
        // render "scope includes child accounts".
        let now = chrono::Utc::now();
        let snapshot = fake_snapshot(
            "account",
            "acc-parent",
            vec![fake_touchpoint(
                "m-1",
                "account",
                "acc-child",
                TouchpointInclusionReason::EntityLink,
                now + chrono::Duration::days(2),
            )],
            vec![],
            vec![("account".to_string(), "acc-child".to_string())],
        );
        let mut prov = EnvelopeProvenance::empty();
        let render_actor = RenderActor::user("user", None::<String>);
        let bundle = project_touchpoints_bundle(
            &snapshot,
            &SubjectRef::Account("acc-parent".to_string()),
            &now,
            &render_actor,
            &mut prov,
        );
        assert!(bundle.empty_reason.is_none());
        assert_eq!(bundle.upcoming.items.len(), 1);
        let tp = &bundle.upcoming.items[0];
        assert_eq!(tp.inclusion_reason, InclusionReason::EntityLink);
        assert_eq!(tp.subject_ref, SubjectRef::Account("acc-child".to_string()));
        assert_eq!(
            bundle.subject_scope.also_includes,
            vec![SubjectRef::Account("acc-child".to_string())]
        );
        // Primary remains the requested subject — no bleed.
        assert_eq!(
            bundle.subject_scope.primary,
            SubjectRef::Account("acc-parent".to_string())
        );
    }

    #[test]
    fn project_touchpoints_person_attendee_match_preserves_inclusion_reason() {
        // Person subject found as attendee of an internal multi-account meeting —
        // the AttendeeMatch reason must survive projection so the renderer can
        // distinguish "this person attended" from "this person was the meeting subject".
        let now = chrono::Utc::now();
        let snapshot = fake_snapshot(
            "person",
            "p-1",
            vec![],
            vec![fake_touchpoint(
                "m-2",
                "person",
                "p-1",
                TouchpointInclusionReason::AttendeeMatch,
                now - chrono::Duration::days(3),
            )],
            vec![],
        );
        let mut prov = EnvelopeProvenance::empty();
        let render_actor = RenderActor::user("user", None::<String>);
        let bundle = project_touchpoints_bundle(
            &snapshot,
            &SubjectRef::Person("p-1".to_string()),
            &now,
            &render_actor,
            &mut prov,
        );
        assert_eq!(bundle.recent.items.len(), 1);
        assert_eq!(
            bundle.recent.items[0].inclusion_reason,
            InclusionReason::AttendeeMatch
        );
    }

    #[test]
    fn project_touchpoints_multi_account_meeting_no_subject_bleed() {
        // A meeting tagged with both acc-a and acc-b. The acc-a envelope should
        // only see the acc-a row; subject_ref echoes the matched id, not the
        // requesting id. The fake reader stage guarantees this; this test
        // verifies the projection doesn't accidentally rewrite subject_ref.
        let now = chrono::Utc::now();
        let snapshot = fake_snapshot(
            "account",
            "acc-a",
            vec![fake_touchpoint(
                "m-shared",
                "account",
                "acc-a",
                TouchpointInclusionReason::SubjectMatch,
                now + chrono::Duration::days(1),
            )],
            vec![],
            vec![],
        );
        let mut prov = EnvelopeProvenance::empty();
        let render_actor = RenderActor::user("user", None::<String>);
        let bundle = project_touchpoints_bundle(
            &snapshot,
            &SubjectRef::Account("acc-a".to_string()),
            &now,
            &render_actor,
            &mut prov,
        );
        let tp = &bundle.upcoming.items[0];
        assert_eq!(tp.subject_ref, SubjectRef::Account("acc-a".to_string()));
        // SubjectMatch (not EntityLink) — direct match, not inherited.
        assert_eq!(tp.inclusion_reason, InclusionReason::SubjectMatch);
    }

    #[test]
    fn project_touchpoints_freshness_classification_by_age() {
        let now = chrono::Utc::now();
        // Future meeting = Current; recent meeting = Current; aging = Aging; stale = Stale.
        let snapshot = fake_snapshot(
            "account",
            "acc-1",
            vec![fake_touchpoint(
                "m-future",
                "account",
                "acc-1",
                TouchpointInclusionReason::SubjectMatch,
                now + chrono::Duration::days(7),
            )],
            vec![
                fake_touchpoint(
                    "m-recent",
                    "account",
                    "acc-1",
                    TouchpointInclusionReason::SubjectMatch,
                    now - chrono::Duration::days(3),
                ),
                fake_touchpoint(
                    "m-aging",
                    "account",
                    "acc-1",
                    TouchpointInclusionReason::SubjectMatch,
                    now - chrono::Duration::days(14),
                ),
                fake_touchpoint(
                    "m-stale",
                    "account",
                    "acc-1",
                    TouchpointInclusionReason::SubjectMatch,
                    now - chrono::Duration::days(60),
                ),
            ],
            vec![],
        );
        let mut prov = EnvelopeProvenance::empty();
        let render_actor = RenderActor::user("user", None::<String>);
        let bundle = project_touchpoints_bundle(
            &snapshot,
            &SubjectRef::Account("acc-1".to_string()),
            &now,
            &render_actor,
            &mut prov,
        );
        assert_eq!(bundle.upcoming.items[0].freshness, Freshness::Current);
        assert_eq!(bundle.recent.items[0].freshness, Freshness::Current);
        assert_eq!(bundle.recent.items[1].freshness, Freshness::Aging);
        assert_eq!(bundle.recent.items[2].freshness, Freshness::Stale);
    }

    #[test]
    fn classify_touchpoint_kind_falls_back_to_meeting_for_unknown_strings() {
        assert_eq!(classify_touchpoint_kind("internal"), TouchpointKind::Meeting);
        assert_eq!(classify_touchpoint_kind("team_sync"), TouchpointKind::Meeting);
        assert_eq!(classify_touchpoint_kind("email_thread"), TouchpointKind::EmailThread);
        assert_eq!(classify_touchpoint_kind("salesforce_call"), TouchpointKind::Salesforce);
        assert_eq!(classify_touchpoint_kind("linear_update"), TouchpointKind::Linear);
        assert_eq!(classify_touchpoint_kind("google_doc"), TouchpointKind::Document);
        // Defensive: empty + garbage strings → Meeting, never panic.
        assert_eq!(classify_touchpoint_kind(""), TouchpointKind::Meeting);
        assert_eq!(classify_touchpoint_kind("???"), TouchpointKind::Meeting);
    }

    #[test]
    fn parse_exclusion_reason_strict_allowlist() {
        assert_eq!(parse_exclusion_reason("subject_mismatch"), Some(ExclusionReason::SubjectMismatch));
        assert_eq!(parse_exclusion_reason("outside_window"), Some(ExclusionReason::OutsideWindow));
        assert_eq!(parse_exclusion_reason("low_confidence"), Some(ExclusionReason::LowConfidence));
        assert_eq!(parse_exclusion_reason("suppressed"), Some(ExclusionReason::Suppressed));
        // Unknown values must NOT round-trip to a fake reason.
        assert_eq!(parse_exclusion_reason("unknown"), None);
        assert_eq!(parse_exclusion_reason(""), None);
    }

    #[test]
    fn subject_ref_from_pair_known_kinds_round_trip() {
        assert_eq!(
            subject_ref_from_pair("account", "a-1"),
            SubjectRef::Account("a-1".to_string())
        );
        assert_eq!(
            subject_ref_from_pair("project", "p-1"),
            SubjectRef::Project("p-1".to_string())
        );
        assert_eq!(
            subject_ref_from_pair("person", "u-1"),
            SubjectRef::Person("u-1".to_string())
        );
        assert_eq!(
            subject_ref_from_pair("meeting", "m-1"),
            SubjectRef::Meeting("m-1".to_string())
        );
        // Unknown kind → Unknown, never a wildcard subject id.
        assert!(matches!(
            subject_ref_from_pair("bogus", "x"),
            SubjectRef::Unknown
        ));
    }

    #[test]
    fn read_failed_bundle_carries_partial_failure_reason() {
        // When the reader is unavailable, the producer surfaces typed
        // PartialFailure — not a null, not a generic "empty", and not a hard
        // envelope error. Consumers can distinguish "no touchpoints" from
        // "couldn't read touchpoints" by checking empty_reason.
        let bundle = read_failed_touchpoints_bundle(
            &SubjectRef::Account("acc-1".to_string()),
            &chrono::Utc::now(),
            "reader unavailable in test context".to_string(),
        );
        assert!(matches!(
            bundle.empty_reason,
            Some(EmptyReason::PartialFailure { .. })
        ));
        assert!(bundle.candidate_set.filter_description.contains("reader unavailable"));
    }

    #[test]
    fn filtered_out_bundle_marks_subject_filtered_not_no_touchpoints() {
        // SubjectNotOwned must surface as FilteredOutBySubject, not
        // NoRelevantTouchpoints — these are semantically distinct outcomes
        // (workspace boundary vs no data in window).
        let bundle = filtered_out_touchpoints_bundle(
            &SubjectRef::Person("p-1".to_string()),
            &chrono::Utc::now(),
        );
        assert_eq!(bundle.empty_reason, Some(EmptyReason::FilteredOutBySubject));
    }

    // ---- F2 (L3 cycle-2) — touchpoint audience-scrub tests ----------------

    #[test]
    fn project_touchpoint_agent_audience_redacts_raw_meeting_title() {
        // F2 / ADR-0108: AgentMcp / agent-surface audiences MUST NOT see the
        // raw meeting title in the envelope provenance label. The producer
        // routes the title through an audience-aware scrub at projection time.
        let now = chrono::Utc::now();
        let snapshot = fake_snapshot(
            "account",
            "acc-1",
            vec![fake_touchpoint(
                "m-secret",
                "account",
                "acc-1",
                TouchpointInclusionReason::SubjectMatch,
                now + chrono::Duration::days(1),
            )],
            vec![],
            vec![],
        );
        let mut prov = EnvelopeProvenance::empty();
        let agent_actor = RenderActor::agent("mcp_client");
        let bundle = project_touchpoints_bundle(
            &snapshot,
            &SubjectRef::Account("acc-1".to_string()),
            &now,
            &agent_actor,
            &mut prov,
        );
        assert_eq!(bundle.upcoming.items.len(), 1);
        // Agent envelope MUST NOT carry the raw "Meeting m-secret" title; it
        // MUST carry the redacted placeholder and `redacted: true`.
        let secret_title = "Meeting m-secret";
        for source in &prov.sources {
            assert_ne!(
                source.label, secret_title,
                "agent envelope leaked raw meeting title via provenance label"
            );
            if source.id.starts_with("meeting:") {
                assert!(
                    source.redacted,
                    "meeting touchpoint provenance must be marked redacted for agent audience"
                );
                assert_eq!(source.label, "Meeting (redacted)");
            }
        }
        // Property: no field anywhere in the agent envelope contains the raw
        // title. Serialize the bundle + provenance to JSON and assert absence.
        let envelope_json = serde_json::json!({
            "bundle": &bundle,
            "provenance": &prov,
        });
        let serialized = serde_json::to_string(&envelope_json).expect("serializes");
        assert!(
            !serialized.contains(secret_title),
            "agent envelope leaked raw meeting title in serialized form"
        );
    }

    #[test]
    fn project_touchpoint_user_audience_preserves_meeting_title() {
        // Symmetric F2 — UserTauri audience sees the full meeting title.
        let now = chrono::Utc::now();
        let snapshot = fake_snapshot(
            "account",
            "acc-1",
            vec![fake_touchpoint(
                "m-secret",
                "account",
                "acc-1",
                TouchpointInclusionReason::SubjectMatch,
                now + chrono::Duration::days(1),
            )],
            vec![],
            vec![],
        );
        let mut prov = EnvelopeProvenance::empty();
        let user_actor = RenderActor::user("user", Some("user-1".to_string()));
        let bundle = project_touchpoints_bundle(
            &snapshot,
            &SubjectRef::Account("acc-1".to_string()),
            &now,
            &user_actor,
            &mut prov,
        );
        assert_eq!(bundle.upcoming.items.len(), 1);
        let meeting_source = prov
            .sources
            .iter()
            .find(|s| s.id == "meeting:m-secret")
            .expect("meeting source present");
        assert!(!meeting_source.redacted);
        assert_eq!(meeting_source.label, "Meeting m-secret");
    }

    #[test]
    fn not_requested_bundle_marks_section_explicitly() {
        // When the caller filters Touchpoints out of the requested sections,
        // the bundle returns NotRequested (not NoRelevantTouchpoints), so the
        // renderer can hide the section vs render "no touchpoints in window".
        let bundle = not_requested_touchpoints_bundle(&SubjectRef::Account("a".to_string()));
        let inner = &bundle.items[0];
        assert_eq!(inner.empty_reason, Some(EmptyReason::NotRequested));
    }
}
