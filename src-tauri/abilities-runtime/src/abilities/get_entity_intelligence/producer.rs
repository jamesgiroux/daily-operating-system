//! `get_entity_intelligence` envelope producer.
//!
//! Read-side composition over existing claim/proposal/open-loop substrate.
//! See `.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md` §5.1.
//!
//! W1 scope: envelope shape + facts + open_loops are wired to real readers.
//! Touchpoints / threads / record_entries / metadata_proposals return typed
//! empty states keyed to the producer that will fill them in later substrate
//! waves (touchpoints, threads, metadata proposals).
//! This matches the L0 contract: empty sections carry typed reasons; no
//! "Phase 2" stubs.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};

use super::contracts::{
    CandidateSetRef, ContextDepth as EnvelopeContextDepth, Cursor, CursorState, EmptyReason,
    EntityFact, EntityIntelligenceEnvelope, EntityIntelligenceInput, EntityKind,
    EnvelopeProvenance, EnvelopeProvenanceSource, EnvelopeSection, EnvelopeTrustSummary,
    ExclusionReason, Freshness, HealthStory, InclusionReason, MetadataProposal, NormalizedSubject,
    OpenLoopWithReceipt, Paginated, ProvenanceRef, ReceiptTargetRef, RecordEntry, RelationshipEdge,
    RelationshipInclusionReason, RelationshipParticipant, RelationshipTruncation,
    RelationshipsBundle, SectionState, SubjectScope, ThreadSummary, Touchpoint, TouchpointBundle,
    TouchpointKind, ENVELOPE_SCHEMA_VERSION, ENVELOPE_SCHEMA_VERSION_V1,
    ENVELOPE_SCHEMA_VERSION_V2,
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
use crate::sensitivity::{
    renderable_claim_text_with_value, ClaimDismissalSurface, RenderActor, RenderPolicy,
    RenderPolicyKind, RenderSurface, RenderableClaimText,
};
use crate::services::context::{
    EntityNeighborhoodQuery, EntityNeighborhoodReadError, EntityNeighborhoodSnapshot,
    EntityParticipantSnapshot, EntityRelationshipEdgeSnapshot, EntityRelationshipInclusionReason,
    EntityTouchpointSnapshot, EntityTouchpointsQuery, EntityTouchpointsReadError,
    EntityTouchpointsSnapshot, TouchpointInclusionReason,
};
use crate::types::{claim_allowed_for_prompt_input, ClaimSensitivity, IntelligenceClaim};

const ABILITY_NAME: &str = "get_entity_intelligence";

/// Page size cap applied to each list-shape field. Server-side cursor pagination
/// kicks in when the underlying reader returns more than this; W1 reads are bounded
/// by `ContextDepth.claim_levels()` so the cap is informational at this stage.
const DEFAULT_PAGE_SIZE: usize = 50;
const NEIGHBORHOOD_MAX_DEPTH: u8 = 2;
const NEIGHBORHOOD_PER_EDGE_CAP: usize = 50;
const NEIGHBORHOOD_RECENT_TOUCHPOINT_CAP: usize = 5;

pub async fn build_entity_intelligence(
    ctx: &AbilityContext<'_>,
    input: EntityIntelligenceInput,
) -> AbilityResult<EntityIntelligenceEnvelope> {
    validate_schema_version(input.schema_version)?;
    validate_requested_sections(input.schema_version, input.sections.as_ref())?;

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

    let active_sections = active_section_set(input.schema_version, input.sections.as_ref());

    // ---- compose: facts -----------------------------------------------------
    let facts_active = active_sections.contains(&EnvelopeSection::Facts)
        || active_sections.contains(&EnvelopeSection::Record);
    let claims = if facts_active {
        read_claims(ctx, &entity_type_str, entity_id, &input.depth).await?
    } else {
        Vec::new()
    };
    let render_actor = render_actor_for_context(ctx);
    let render_surface = render_surface_for_context(ctx);

    let mut envelope_provenance = EnvelopeProvenance::empty();

    let facts = if active_sections.contains(&EnvelopeSection::Facts) {
        build_facts(
            &claims,
            &render_actor,
            render_surface,
            &mut envelope_provenance,
        )?
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
        build_record_entries(
            &claims,
            &render_actor,
            render_surface,
            &mut envelope_provenance,
        )?
    } else {
        Paginated::empty_stable()
    };

    // ---- compose: relationships -------------------------------------------
    let relationships = if input.schema_version >= ENVELOPE_SCHEMA_VERSION_V2
        && active_sections.contains(&EnvelopeSection::Relationships)
    {
        compose_relationships(
            ctx,
            &input.entity_type,
            entity_id,
            &render_actor,
            render_surface,
            &mut envelope_provenance,
        )
        .await?
    } else if input.schema_version >= ENVELOPE_SCHEMA_VERSION_V2 {
        not_requested_relationships_bundle(&subject_ref)
    } else {
        None
    };

    // ---- compose: touchpoints ---------------------------------------------
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

    // ---- compose: health (Meeting subject only, via prep status) ----------
    // W2 F2 (cycle-1 codex challenge): Meeting Detail's prep-status inner block
    // reads from `services::meeting_prep_status::read::compute_status`. We surface
    // it as the envelope's Health section so the Meeting Detail block can render
    // PrepStatus/blocking_reason/stale_reason without a second invocation.
    let health_story: Option<HealthStory> = if active_sections.contains(&EnvelopeSection::Health)
        && matches!(input.entity_type, EntityKind::Meeting)
    {
        compose_meeting_health(ctx, entity_id, &mut envelope_provenance).await
    } else {
        None
    };

    // ---- sections map enumerates ALL EnvelopeSection variants (AC-459.2) ---
    let sections_map = build_sections_map(
        input.schema_version,
        &input.sections,
        SectionFill {
            facts_count: facts.items.len() as u64,
            health_present: health_story.is_some(),
            metadata_proposals_count: metadata_proposals.items.len() as u64,
            open_loops_count: open_loops.items.len() as u64,
            relationships_count: relationships.as_ref().map(count_relationships).unwrap_or(0),
            relationships_empty_reason: relationship_empty_reason(relationships.as_ref()),
            touchpoints_count: count_touchpoints(&touchpoints),
            threads_count: threads.items.len() as u64,
            record_entries_count: record_entries.items.len() as u64,
        },
    );

    // ---- aggregate trust summary -------------------------------------------
    let trust = aggregate_trust(&facts, relationships.as_ref());

    // ---- aggregate sensitivity = max across facts (defaults to Public) -----
    let sensitivity = aggregate_sensitivity(&facts, &record_entries, relationships.as_ref());

    let envelope = EntityIntelligenceEnvelope {
        schema_version: input.schema_version,
        subject: normalized_subject.clone(),
        sections: sections_map,
        facts,
        health_story,
        metadata_proposals,
        open_loops,
        relationships,
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
    // `Provenance` records only the producer call so high-cardinality source
    // indexes do not get duplicated into a second provenance graph.
    let mut builder = ProvenanceBuilder::new(provenance_config(ctx, input.schema_version));
    let subject_attr = SubjectAttribution::direct_confident(subject_ref);
    builder.set_subject(subject_attr.clone());
    builder
        .attribute(
            FieldPath::root(),
            FieldAttribution::constant(subject_attr.clone()),
        )
        .map_err(provenance_error)?;
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
    if schema_version == ENVELOPE_SCHEMA_VERSION_V1 || schema_version == ENVELOPE_SCHEMA_VERSION {
        Ok(())
    } else {
        Err(validation_error(format!(
            "unsupported schema_version `{schema_version}` for `{ABILITY_NAME}`"
        )))
    }
}

fn validate_requested_sections(
    schema_version: u32,
    requested: Option<&Vec<EnvelopeSection>>,
) -> Result<(), AbilityError> {
    if schema_version < ENVELOPE_SCHEMA_VERSION_V2
        && requested
            .map(|sections| sections.contains(&EnvelopeSection::Relationships))
            .unwrap_or(false)
    {
        return Err(validation_error(
            "relationships section requires get_entity_intelligence schema_version 2",
        ));
    }
    Ok(())
}

fn subject_ref_for(entity_type: EntityKind, entity_id: &str) -> SubjectRef {
    match entity_type {
        EntityKind::Account => SubjectRef::Account(entity_id.to_string()),
        EntityKind::Project => SubjectRef::Project(entity_id.to_string()),
        EntityKind::Person => SubjectRef::Person(entity_id.to_string()),
        EntityKind::Meeting => SubjectRef::Meeting(entity_id.to_string()),
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
    schema_version: u32,
    requested: Option<&Vec<EnvelopeSection>>,
) -> std::collections::BTreeSet<EnvelopeSection> {
    match requested {
        None => EnvelopeSection::all_for_schema(schema_version)
            .iter()
            .copied()
            .collect(),
        Some(list) if list.is_empty() => EnvelopeSection::all_for_schema(schema_version)
            .iter()
            .copied()
            .collect(),
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
    let claims = if matches!(ctx.actor, Actor::Agent | Actor::McpClient { .. }) {
        ctx.services()
            .read_entity_context_prompt_claims_limited(
                entity_type.to_string(),
                entity_id.to_string(),
                ctx.entity_context_claim_surface(),
                levels,
                DEFAULT_PAGE_SIZE + 1,
            )
            .await
    } else {
        ctx.services()
            .read_entity_context_claims_limited(
                entity_type.to_string(),
                entity_id.to_string(),
                ctx.entity_context_claim_surface(),
                levels,
                DEFAULT_PAGE_SIZE + 1,
            )
            .await
    }
    .map_err(|error| hard_error("entity_intelligence_claim_read_failed", error))?;
    Ok(filter_claims_for_actor(ctx.actor.clone(), claims))
}

fn filter_claims_for_actor(actor: Actor, claims: Vec<IntelligenceClaim>) -> Vec<IntelligenceClaim> {
    if matches!(actor, Actor::Agent | Actor::McpClient { .. }) {
        claims
            .into_iter()
            .filter(claim_allowed_for_prompt_input)
            .collect()
    } else {
        claims
    }
}

fn render_surface_for_context(ctx: &AbilityContext<'_>) -> RenderSurface {
    match ctx.entity_context_claim_surface() {
        ClaimDismissalSurface::TauriEntityDetail => RenderSurface::TauriEntityDetail,
        ClaimDismissalSurface::Briefing => RenderSurface::TauriBriefingPrep,
        ClaimDismissalSurface::TauriMeetingDetail => RenderSurface::TauriMeetingDetail,
        ClaimDismissalSurface::TauriEmailSummary => RenderSurface::TauriEmailSummary,
        ClaimDismissalSurface::Action => RenderSurface::Action,
        ClaimDismissalSurface::TauriProvenance => RenderSurface::TauriProvenance,
        ClaimDismissalSurface::TauriReport => RenderSurface::TauriReport,
        ClaimDismissalSurface::TauriChat => RenderSurface::TauriChat,
        ClaimDismissalSurface::McpTool => RenderSurface::McpTool,
        ClaimDismissalSurface::McpToolDetail => RenderSurface::McpToolDetail,
        ClaimDismissalSurface::P2Publication => RenderSurface::P2Publication,
        ClaimDismissalSurface::LogStructured => RenderSurface::LogStructured,
        ClaimDismissalSurface::PushNotification => RenderSurface::PushNotification,
        ClaimDismissalSurface::Worker | ClaimDismissalSurface::Eval => {
            if matches!(&ctx.actor, Actor::Agent | Actor::McpClient { .. }) {
                RenderSurface::McpTool
            } else {
                RenderSurface::TauriEntityDetail
            }
        }
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
            subject_ref: subject_ref_from_claim(claim),
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
        Ok(crate::types::ClaimSubjectRef::Action { id }) => SubjectRef::Action(id),
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

// ---- touchpoints ---------------------------------------------------------

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

// ---- relationships -------------------------------------------------------

async fn compose_relationships(
    ctx: &AbilityContext<'_>,
    entity_type: &EntityKind,
    entity_id: &str,
    render_actor: &RenderActor,
    render_surface: RenderSurface,
    provenance: &mut EnvelopeProvenance,
) -> Result<Option<Paginated<RelationshipsBundle>>, AbilityError> {
    let now = ctx.services().clock.now();
    let subject_ref = subject_ref_for(entity_type.clone(), entity_id);
    let query = EntityNeighborhoodQuery {
        entity_type: entity_type.as_lower_str().to_string(),
        entity_id: entity_id.to_string(),
        now,
        max_depth: NEIGHBORHOOD_MAX_DEPTH,
        per_edge_cap: NEIGHBORHOOD_PER_EDGE_CAP,
        recent_touchpoint_cap: NEIGHBORHOOD_RECENT_TOUCHPOINT_CAP,
    };

    let snapshot = match ctx.services().read_entity_neighborhood(query).await {
        Ok(snapshot) => snapshot,
        Err(EntityNeighborhoodReadError::SubjectNotOwned { .. }) => {
            return Ok(Some(Paginated::stable(vec![
                filtered_out_relationships_bundle(&subject_ref),
            ])));
        }
        Err(EntityNeighborhoodReadError::ReadFailed(message)) => {
            return Ok(Some(Paginated::stable(vec![
                read_failed_relationships_bundle(&subject_ref, message),
            ])));
        }
    };

    Ok(Some(Paginated::stable(vec![project_relationships_bundle(
        &snapshot,
        &subject_ref,
        &now,
        render_actor,
        render_surface,
        provenance,
    )])))
}

fn not_requested_relationships_bundle(
    subject_ref: &SubjectRef,
) -> Option<Paginated<RelationshipsBundle>> {
    Some(Paginated::stable(vec![RelationshipsBundle {
        edges: Paginated::empty_stable(),
        participants: Paginated::empty_stable(),
        candidate_set: CandidateSetRef {
            window_start: None,
            window_end: None,
            filter_description: "relationships section not requested".to_string(),
        },
        empty_reason: Some(EmptyReason::NotRequested),
        subject_scope: SubjectScope {
            primary: subject_ref.clone(),
            also_includes: Vec::new(),
        },
        truncation: RelationshipTruncation {
            edges_truncated: false,
            participants_truncated: false,
            per_edge_cap: NEIGHBORHOOD_PER_EDGE_CAP,
        },
        caveats: Vec::new(),
    }]))
}

fn filtered_out_relationships_bundle(subject_ref: &SubjectRef) -> RelationshipsBundle {
    RelationshipsBundle {
        edges: Paginated::empty_stable(),
        participants: Paginated::empty_stable(),
        candidate_set: CandidateSetRef {
            window_start: None,
            window_end: None,
            filter_description: "subject filtered out by workspace scope".to_string(),
        },
        empty_reason: Some(EmptyReason::FilteredOutBySubject),
        subject_scope: SubjectScope {
            primary: subject_ref.clone(),
            also_includes: Vec::new(),
        },
        truncation: RelationshipTruncation {
            edges_truncated: false,
            participants_truncated: false,
            per_edge_cap: NEIGHBORHOOD_PER_EDGE_CAP,
        },
        caveats: vec!["subject filtered out by workspace scope".to_string()],
    }
}

fn read_failed_relationships_bundle(
    subject_ref: &SubjectRef,
    message: String,
) -> RelationshipsBundle {
    RelationshipsBundle {
        edges: Paginated::empty_stable(),
        participants: Paginated::empty_stable(),
        candidate_set: CandidateSetRef {
            window_start: None,
            window_end: None,
            filter_description: message,
        },
        empty_reason: Some(EmptyReason::PartialFailure {
            advisory: "relationships reader unavailable".to_string(),
        }),
        subject_scope: SubjectScope {
            primary: subject_ref.clone(),
            also_includes: Vec::new(),
        },
        truncation: RelationshipTruncation {
            edges_truncated: false,
            participants_truncated: false,
            per_edge_cap: NEIGHBORHOOD_PER_EDGE_CAP,
        },
        caveats: vec!["relationships reader unavailable".to_string()],
    }
}

fn project_relationships_bundle(
    snapshot: &EntityNeighborhoodSnapshot,
    subject_ref: &SubjectRef,
    now: &DateTime<Utc>,
    render_actor: &RenderActor,
    render_surface: RenderSurface,
    provenance: &mut EnvelopeProvenance,
) -> RelationshipsBundle {
    let mut caveats = snapshot.caveats.clone();
    let edges = snapshot
        .edges
        .iter()
        .map(|edge| {
            project_relationship_edge(
                edge,
                subject_ref,
                now,
                render_actor,
                render_surface,
                provenance,
                &mut caveats,
            )
        })
        .collect::<Vec<_>>();
    let participants = snapshot
        .participants
        .iter()
        .map(|participant| {
            project_relationship_participant(
                participant,
                now,
                render_actor,
                render_surface,
                provenance,
                &mut caveats,
            )
        })
        .collect::<Vec<_>>();

    let mut also_includes = Vec::new();
    for edge in &edges {
        if edge.traversal_depth <= 1
            && edge.related_subject_ref != *subject_ref
            && !also_includes.contains(&edge.related_subject_ref)
        {
            also_includes.push(edge.related_subject_ref.clone());
        }
    }

    let empty_reason = if edges.is_empty() && participants.is_empty() {
        Some(EmptyReason::NoRelevantRelationships)
    } else {
        None
    };

    RelationshipsBundle {
        edges: Paginated::stable(edges),
        participants: Paginated::stable(participants),
        candidate_set: CandidateSetRef {
            window_start: None,
            window_end: Some(*now),
            filter_description: format!(
                "{}:{} neighborhood depth <= {} over existing relationship substrate",
                snapshot.subject_entity_type, snapshot.subject_entity_id, NEIGHBORHOOD_MAX_DEPTH
            ),
        },
        empty_reason,
        subject_scope: SubjectScope {
            primary: subject_ref.clone(),
            also_includes,
        },
        truncation: RelationshipTruncation {
            edges_truncated: snapshot.truncation.edges_truncated,
            participants_truncated: snapshot.truncation.participants_truncated,
            per_edge_cap: snapshot.truncation.per_edge_cap,
        },
        caveats,
    }
}

fn project_relationship_edge(
    raw: &EntityRelationshipEdgeSnapshot,
    subject_ref: &SubjectRef,
    now: &DateTime<Utc>,
    render_actor: &RenderActor,
    render_surface: RenderSurface,
    provenance: &mut EnvelopeProvenance,
    caveats: &mut Vec<String>,
) -> RelationshipEdge {
    let source_id = relationship_source_id(&raw.source_type, &raw.source_id);
    let source_id = upsert_static_provenance_source(
        provenance,
        EnvelopeProvenanceSource {
            id: source_id,
            label: source_label_for_relationship(&raw.source_type, render_actor),
            source_type: Some(raw.source_type.clone()),
            as_of: parse_optional_timestamp(raw.source_asof.as_deref()),
            redacted: !render_actor.is_user(),
        },
    );
    let related_display_label = raw
        .related_display_label
        .as_deref()
        .and_then(|label| {
            renderable_evidence_text(label, &raw.sensitivity, render_surface, render_actor)
        })
        .or_else(|| {
            if raw.related_display_label.is_some() {
                push_unique_caveat(caveats, "relationship label blocked by render policy");
            }
            None
        });

    let freshness = freshness_for_evidence(
        raw.source_asof.as_deref().or(raw.observed_at.as_deref()),
        now,
    );
    RelationshipEdge {
        edge_id: format!(
            "{}:{}:{}:{}",
            raw.edge_type, raw.related_entity_type, raw.related_entity_id, raw.source_id
        ),
        edge_type: raw.edge_type.clone(),
        subject_ref: subject_ref.clone(),
        related_subject_ref: subject_ref_from_pair(
            &raw.related_entity_type,
            &raw.related_entity_id,
        ),
        related_display_label,
        observed_at: parse_optional_timestamp(raw.observed_at.as_deref()),
        source_asof: parse_optional_timestamp(raw.source_asof.as_deref()),
        confidence: raw.confidence,
        sensitivity: raw.sensitivity.clone(),
        inclusion_reason: map_relationship_inclusion(raw.inclusion_reason),
        traversal_depth: raw.traversal_depth,
        trust_band: trust_band_for_confidence(raw.confidence, freshness),
        freshness,
        provenance: ProvenanceRef::from_ids([source_id]),
        caveats: Vec::new(),
    }
}

fn project_relationship_participant(
    raw: &EntityParticipantSnapshot,
    now: &DateTime<Utc>,
    render_actor: &RenderActor,
    render_surface: RenderSurface,
    provenance: &mut EnvelopeProvenance,
    caveats: &mut Vec<String>,
) -> RelationshipParticipant {
    let source_id = relationship_source_id(&raw.source_type, &raw.source_id);
    let source_id = upsert_static_provenance_source(
        provenance,
        EnvelopeProvenanceSource {
            id: source_id,
            label: source_label_for_relationship(&raw.source_type, render_actor),
            source_type: Some(raw.source_type.clone()),
            as_of: parse_optional_timestamp(raw.source_asof.as_deref()),
            redacted: !render_actor.is_user(),
        },
    );
    let display_label = raw
        .display_label
        .as_deref()
        .and_then(|label| {
            renderable_evidence_text(label, &raw.sensitivity, render_surface, render_actor)
        })
        .or_else(|| {
            if raw.display_label.is_some() {
                push_unique_caveat(caveats, "participant label blocked by render policy");
            }
            None
        });
    let role = raw
        .role
        .as_deref()
        .and_then(|role| {
            renderable_evidence_text(role, &raw.sensitivity, render_surface, render_actor)
        })
        .or_else(|| {
            if raw.role.is_some() {
                push_unique_caveat(caveats, "participant role blocked by render policy");
            }
            None
        });
    let relationship = raw
        .relationship
        .as_deref()
        .and_then(|relationship| {
            renderable_evidence_text(relationship, &raw.sensitivity, render_surface, render_actor)
        })
        .or_else(|| {
            if raw.relationship.is_some() {
                push_unique_caveat(caveats, "participant relationship blocked by render policy");
            }
            None
        });

    let freshness = freshness_for_evidence(
        raw.last_seen_at.as_deref().or(raw.source_asof.as_deref()),
        now,
    );
    RelationshipParticipant {
        subject_ref: SubjectRef::Person(raw.person_id.clone()),
        display_label,
        role,
        relationship,
        sensitivity: raw.sensitivity.clone(),
        normalized_touchpoint_count: raw.normalized_touchpoint_count,
        recent_touchpoint_ids: raw.recent_touchpoint_ids.clone(),
        last_seen_at: parse_optional_timestamp(raw.last_seen_at.as_deref()),
        trust_band: trust_band_for_confidence(raw.confidence, freshness),
        freshness,
        provenance: ProvenanceRef::from_ids([source_id]),
        caveats: raw.caveats.clone(),
    }
}

fn relationship_source_id(source_type: &str, source_id: &str) -> String {
    format!("relationship:{source_type}:{source_id}")
}

fn source_label_for_relationship(source_type: &str, render_actor: &RenderActor) -> String {
    if render_actor.is_user() {
        source_type.to_string()
    } else {
        "Relationship evidence".to_string()
    }
}

fn renderable_evidence_text(
    value: &str,
    sensitivity: &ClaimSensitivity,
    render_surface: RenderSurface,
    render_actor: &RenderActor,
) -> Option<RenderableClaimText> {
    let text = sanitize_evidence_text(value);
    if text.is_empty() {
        return None;
    }
    if render_surface.is_agent_surface() && looks_like_prompt_injection(&text) {
        return None;
    }
    if matches!(
        sensitivity,
        ClaimSensitivity::Confidential | ClaimSensitivity::UserOnly
    ) && !render_actor.is_user()
    {
        return Some(RenderableClaimText {
            text: "[redacted]".to_string(),
            policy: RenderPolicy {
                kind: RenderPolicyKind::Redacted,
                sensitivity: sensitivity.clone(),
                surface: render_surface,
                claim_id: None,
                affordance: None,
            },
        });
    }
    Some(RenderableClaimText {
        text,
        policy: RenderPolicy {
            kind: RenderPolicyKind::Render,
            sensitivity: sensitivity.clone(),
            surface: render_surface,
            claim_id: None,
            affordance: None,
        },
    })
}

fn sanitize_evidence_text(value: &str) -> String {
    value
        .chars()
        .filter(|ch| {
            !ch.is_control()
                && !matches!(
                    *ch,
                    '\u{200B}' | '\u{200C}' | '\u{200D}' | '\u{2060}' | '\u{FEFF}'
                )
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn looks_like_prompt_injection(value: &str) -> bool {
    let lowered = value.to_ascii_lowercase();
    [
        "ignore previous",
        "ignore all previous",
        "system prompt",
        "developer message",
        "tool output",
        "do not follow",
        "reveal secrets",
        "<script",
    ]
    .iter()
    .any(|needle| lowered.contains(needle))
}

fn map_relationship_inclusion(
    reason: EntityRelationshipInclusionReason,
) -> RelationshipInclusionReason {
    match reason {
        EntityRelationshipInclusionReason::SubjectMatch => {
            RelationshipInclusionReason::SubjectMatch
        }
        EntityRelationshipInclusionReason::Hierarchy => RelationshipInclusionReason::Hierarchy,
        EntityRelationshipInclusionReason::ExplicitLink => {
            RelationshipInclusionReason::ExplicitLink
        }
        EntityRelationshipInclusionReason::AttendeeMatch => {
            RelationshipInclusionReason::AttendeeMatch
        }
        EntityRelationshipInclusionReason::CoAttendance => {
            RelationshipInclusionReason::CoAttendance
        }
        EntityRelationshipInclusionReason::WorkItem => RelationshipInclusionReason::WorkItem,
        EntityRelationshipInclusionReason::ContentLink => RelationshipInclusionReason::ContentLink,
    }
}

fn trust_band_for_confidence(confidence: f32, freshness: Freshness) -> TrustBand {
    match freshness {
        Freshness::Current => {
            if confidence >= 0.8 {
                TrustBand::LikelyCurrent
            } else if confidence >= 0.55 {
                TrustBand::UseWithCaution
            } else {
                TrustBand::NeedsVerification
            }
        }
        Freshness::Aging | Freshness::Unknown => {
            if confidence >= 0.55 {
                TrustBand::UseWithCaution
            } else {
                TrustBand::NeedsVerification
            }
        }
        Freshness::Stale => TrustBand::NeedsVerification,
    }
}

fn freshness_for_evidence(candidate: Option<&str>, now: &DateTime<Utc>) -> Freshness {
    let Some(when) = parse_optional_timestamp(candidate) else {
        return Freshness::Unknown;
    };
    let age = now.signed_duration_since(when);
    if age.num_days() < 30 {
        Freshness::Current
    } else if age.num_days() < 180 {
        Freshness::Aging
    } else {
        Freshness::Stale
    }
}

fn push_unique_caveat(caveats: &mut Vec<String>, caveat: &str) {
    if !caveats.iter().any(|existing| existing == caveat) {
        caveats.push(caveat.to_string());
    }
}

fn count_relationships(relationships: &Paginated<RelationshipsBundle>) -> u64 {
    relationships
        .items
        .iter()
        .map(|bundle| (bundle.edges.items.len() + bundle.participants.items.len()) as u64)
        .sum()
}

// ---- meeting health (W2 F2 prep status) ---------------------------------

/// Compose `HealthStory` for a Meeting subject from the prep status
/// snapshot. Returns `None` when no reader is attached or the meeting is not
/// found — the envelope renders Health as `Empty { NotProcessedYet }` in that
/// case (see `build_sections_map`).
///
/// W2 F2 wiring: the Meeting Detail composite block invokes
/// `get_entity_intelligence(entity_type=meeting)` and consumes
/// `envelope.health_story` directly. Status / blocking_reason / stale_reason
/// are stringly-typed projections of `PrepStatus` per the read handle
/// contract (`MeetingPrepStatusSnapshot`).
async fn compose_meeting_health(
    ctx: &AbilityContext<'_>,
    meeting_id: &str,
    provenance: &mut EnvelopeProvenance,
) -> Option<HealthStory> {
    let snapshot = match ctx
        .services()
        .read_meeting_prep_status(meeting_id.to_string())
        .await
    {
        Ok(snap) => snap,
        Err(_) => return None,
    };
    Some(project_meeting_health(&snapshot, meeting_id, provenance))
}

/// Pure projection: `MeetingPrepStatusSnapshot` → `HealthStory`. Exposed to
/// tests so the row composition + provenance index upsert can be exercised
/// without spinning a full `AbilityContext`.
fn project_meeting_health(
    snapshot: &crate::services::context::MeetingPrepStatusSnapshot,
    meeting_id: &str,
    provenance: &mut EnvelopeProvenance,
) -> HealthStory {
    let source_id = upsert_static_provenance_source(
        provenance,
        EnvelopeProvenanceSource {
            id: format!("meeting_prep:{meeting_id}"),
            label: "Meeting prep status".to_string(),
            source_type: Some("meeting_prep_status".to_string()),
            as_of: parse_optional_timestamp(snapshot.last_prepared_at.as_deref()),
            redacted: false,
        },
    );
    let prov = ProvenanceRef::from_ids([source_id]);

    let mut rows = Vec::new();
    rows.push(super::contracts::HealthStoryRow {
        label: "Prep status".to_string(),
        body: snapshot.status.clone(),
        evidence_claim_ids: Vec::new(),
        provenance: prov.clone(),
    });
    if let Some(reason) = snapshot.blocking_reason.as_deref() {
        rows.push(super::contracts::HealthStoryRow {
            label: "Blocking reason".to_string(),
            body: reason.to_string(),
            evidence_claim_ids: Vec::new(),
            provenance: prov.clone(),
        });
    }
    if let Some(reason) = snapshot.stale_reason.as_deref() {
        rows.push(super::contracts::HealthStoryRow {
            label: "Stale reason".to_string(),
            body: reason.to_string(),
            evidence_claim_ids: Vec::new(),
            provenance: prov.clone(),
        });
    }
    if let Some(prepared) = snapshot.last_prepared_at.as_deref() {
        rows.push(super::contracts::HealthStoryRow {
            label: "Last prepared".to_string(),
            body: prepared.to_string(),
            evidence_claim_ids: Vec::new(),
            provenance: prov.clone(),
        });
    }

    let headline = Some(format!("Meeting prep: {}", snapshot.status));
    HealthStory { headline, rows }
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
    relationships_count: u64,
    relationships_empty_reason: Option<EmptyReason>,
    touchpoints_count: u64,
    threads_count: u64,
    record_entries_count: u64,
}

fn build_sections_map(
    schema_version: u32,
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
    for section in EnvelopeSection::all_for_schema(schema_version) {
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
        EnvelopeSection::Relationships => count_or_empty(
            fill.relationships_count,
            fill.relationships_empty_reason
                .clone()
                .unwrap_or(EmptyReason::NoRelevantRelationships),
        ),
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

fn relationship_empty_reason(
    relationships: Option<&Paginated<RelationshipsBundle>>,
) -> Option<EmptyReason> {
    let relationships = relationships?;
    if count_relationships(relationships) > 0 {
        return None;
    }
    relationships
        .items
        .iter()
        .find_map(|bundle| bundle.empty_reason.clone())
}

// ---- aggregate trust + sensitivity ----------------------------------------

fn aggregate_trust(
    facts: &Paginated<EntityFact>,
    relationships: Option<&Paginated<RelationshipsBundle>>,
) -> EnvelopeTrustSummary {
    let relationship_bands = relationships
        .into_iter()
        .flat_map(|bundles| bundles.items.iter())
        .flat_map(|bundle| {
            bundle.edges.items.iter().map(|edge| edge.trust_band).chain(
                bundle
                    .participants
                    .items
                    .iter()
                    .map(|participant| participant.trust_band),
            )
        });

    let mut section_caveats = BTreeMap::new();
    if let Some(relationships) = relationships {
        let caveats = relationships
            .items
            .iter()
            .flat_map(|bundle| bundle.caveats.iter())
            .cloned()
            .collect::<Vec<_>>();
        if !caveats.is_empty() {
            section_caveats.insert(EnvelopeSection::Relationships, caveats.join("; "));
        }
    }

    let mut bands = facts
        .items
        .iter()
        .map(|fact| fact.trust_band)
        .chain(relationship_bands)
        .collect::<Vec<_>>();

    if bands.is_empty() {
        return EnvelopeTrustSummary {
            aggregate_band: TrustBand::Unscored,
            section_caveats,
        };
    }

    let aggregate_band = bands
        .drain(..)
        .reduce(min_trust_band)
        .unwrap_or(TrustBand::Unscored);
    EnvelopeTrustSummary {
        aggregate_band,
        section_caveats,
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
    relationships: Option<&Paginated<RelationshipsBundle>>,
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
    if let Some(relationships) = relationships {
        for bundle in &relationships.items {
            for edge in &bundle.edges.items {
                let r = rank(&edge.sensitivity);
                if r > max_rank {
                    max_rank = r;
                    max = edge.sensitivity.clone();
                }
            }
            for participant in &bundle.participants.items {
                let r = rank(&participant.sensitivity);
                if r > max_rank {
                    max_rank = r;
                    max = participant.sensitivity.clone();
                }
            }
        }
    }
    max
}

// ---- provenance index helpers ---------------------------------------------

fn upsert_provenance_source(
    provenance: &mut EnvelopeProvenance,
    claim: &IntelligenceClaim,
) -> String {
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
        // `redact_provenance_for_surface`. Substrate marks redacted=false
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
    let source_timestamp = claim
        .source_asof
        .as_deref()
        .or(Some(claim.observed_at.as_str()));
    let Some(source_asof) = parse_optional_timestamp(source_timestamp) else {
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
    //! Pure shape + section-state tests for the envelope. The full
    //! runtime test using `AbilityContext` lives in the fixture harness (W1
    //! sibling). These tests verify the cycle-1 architecture F5 + correctness F4
    //! contracts — every list-shape field is `Paginated<T>` with `CursorState`,
    //! empty sections carry typed reasons, sections map enumerates all variants.

    use super::*;
    use std::sync::Arc;

    use chrono::TimeZone;

    use super::super::contracts::{ExclusionReason, InclusionReason, Touchpoint, TouchpointKind};
    use crate::abilities::registry::{AbilityContext, McpClientId};
    use crate::abilities::{Actor, NOOP_ABILITY_TRACER};
    use crate::intelligence::provider::ReplayProvider;
    use crate::sensitivity::ClaimVerificationState;
    use crate::services::context::{
        ClaimDismissalSurface, EntityContextClaimReadFuture, EntityContextClaimReadHandle,
        EntityNeighborhoodQuery, EntityNeighborhoodReadError, EntityNeighborhoodReadFuture,
        EntityNeighborhoodReadHandle, EntityNeighborhoodSnapshot, EntityNeighborhoodTruncation,
        EntityParticipantSnapshot, EntityRelationshipEdgeSnapshot,
        EntityRelationshipInclusionReason, FixedClock, ServiceContext, SystemRng,
    };
    use crate::types::{
        ClaimSensitivity, ClaimState, IntelligenceClaim, SurfacingState, TemporalScope,
    };

    struct EmptyClaimReader;

    impl EntityContextClaimReadHandle for EmptyClaimReader {
        fn read_entity_context_claims<'a>(
            &'a self,
            _entity_type: String,
            _entity_id: String,
            _surface: ClaimDismissalSurface,
            _depth: usize,
        ) -> EntityContextClaimReadFuture<'a> {
            Box::pin(async { Ok(Vec::<IntelligenceClaim>::new()) })
        }
    }

    struct PromptSafeBeforeCapClaimReader;

    impl EntityContextClaimReadHandle for PromptSafeBeforeCapClaimReader {
        fn read_entity_context_claims<'a>(
            &'a self,
            _entity_type: String,
            _entity_id: String,
            _surface: ClaimDismissalSurface,
            _depth: usize,
        ) -> EntityContextClaimReadFuture<'a> {
            Box::pin(async { Ok(prompt_safe_before_cap_claims()) })
        }

        fn read_entity_context_prompt_claims_limited<'a>(
            &'a self,
            _entity_type: String,
            _entity_id: String,
            _surface: ClaimDismissalSurface,
            _depth: usize,
            limit: usize,
        ) -> EntityContextClaimReadFuture<'a> {
            Box::pin(async move {
                let mut claims = prompt_safe_before_cap_claims();
                claims.retain(crate::types::claim_allowed_for_prompt_input);
                claims.truncate(limit);
                Ok(claims)
            })
        }
    }

    struct FixtureNeighborhoodReader;

    impl EntityNeighborhoodReadHandle for FixtureNeighborhoodReader {
        fn read_entity_neighborhood<'a>(
            &'a self,
            query: EntityNeighborhoodQuery,
        ) -> EntityNeighborhoodReadFuture<'a> {
            Box::pin(async move {
                Ok(EntityNeighborhoodSnapshot {
                    subject_entity_type: query.entity_type.clone(),
                    subject_entity_id: query.entity_id.clone(),
                    edges: vec![EntityRelationshipEdgeSnapshot {
                        edge_type: "stakeholder".to_string(),
                        related_entity_type: "person".to_string(),
                        related_entity_id: "person-1".to_string(),
                        related_display_label: Some("Example Person".to_string()),
                        source_id: "relationship-source-1".to_string(),
                        source_type: "account_stakeholders".to_string(),
                        observed_at: Some("2026-05-22T15:00:00Z".to_string()),
                        source_asof: Some("2026-05-22T15:00:00Z".to_string()),
                        confidence: 0.95,
                        sensitivity: ClaimSensitivity::Internal,
                        inclusion_reason: EntityRelationshipInclusionReason::ExplicitLink,
                        traversal_depth: 1,
                    }],
                    participants: vec![EntityParticipantSnapshot {
                        person_id: "person-1".to_string(),
                        display_label: Some("Example Person".to_string()),
                        role: Some("Executive sponsor".to_string()),
                        relationship: Some("stakeholder".to_string()),
                        normalized_touchpoint_count: 3,
                        recent_touchpoint_ids: vec!["meeting-1".to_string()],
                        last_seen_at: Some("2026-05-22T15:00:00Z".to_string()),
                        source_id: "participant-source-1".to_string(),
                        source_type: "meeting_attendees".to_string(),
                        source_asof: Some("2026-05-22T15:00:00Z".to_string()),
                        confidence: 0.9,
                        sensitivity: ClaimSensitivity::Internal,
                        caveats: vec!["attendance is not influence".to_string()],
                    }],
                    truncation: EntityNeighborhoodTruncation {
                        edges_truncated: false,
                        participants_truncated: false,
                        per_edge_cap: query.per_edge_cap,
                    },
                    caveats: vec!["fixture caveat".to_string()],
                })
            })
        }
    }

    struct ParticipantOnlyNeighborhoodReader;

    impl EntityNeighborhoodReadHandle for ParticipantOnlyNeighborhoodReader {
        fn read_entity_neighborhood<'a>(
            &'a self,
            query: EntityNeighborhoodQuery,
        ) -> EntityNeighborhoodReadFuture<'a> {
            Box::pin(async move {
                Ok(EntityNeighborhoodSnapshot {
                    subject_entity_type: query.entity_type.clone(),
                    subject_entity_id: query.entity_id.clone(),
                    edges: Vec::new(),
                    participants: vec![EntityParticipantSnapshot {
                        person_id: "person-confidential".to_string(),
                        display_label: Some("Confidential Person".to_string()),
                        role: Some("Executive sponsor".to_string()),
                        relationship: Some("stakeholder".to_string()),
                        normalized_touchpoint_count: 2,
                        recent_touchpoint_ids: vec!["meeting-1".to_string()],
                        last_seen_at: Some("2026-05-22T15:00:00Z".to_string()),
                        source_id: "participant-source-1".to_string(),
                        source_type: "meeting_attendees".to_string(),
                        source_asof: Some("2026-05-22T15:00:00Z".to_string()),
                        confidence: 0.9,
                        sensitivity: ClaimSensitivity::Confidential,
                        caveats: Vec::new(),
                    }],
                    truncation: EntityNeighborhoodTruncation {
                        edges_truncated: false,
                        participants_truncated: false,
                        per_edge_cap: query.per_edge_cap,
                    },
                    caveats: Vec::new(),
                })
            })
        }
    }

    struct FailingNeighborhoodReader;

    impl EntityNeighborhoodReadHandle for FailingNeighborhoodReader {
        fn read_entity_neighborhood<'a>(
            &'a self,
            _query: EntityNeighborhoodQuery,
        ) -> EntityNeighborhoodReadFuture<'a> {
            Box::pin(async {
                Err(EntityNeighborhoodReadError::ReadFailed(
                    "fixture relationship read failed".to_string(),
                ))
            })
        }
    }

    fn fill(facts: u64, open_loops: u64) -> SectionFill {
        SectionFill {
            facts_count: facts,
            health_present: false,
            metadata_proposals_count: 0,
            open_loops_count: open_loops,
            relationships_count: 0,
            relationships_empty_reason: None,
            touchpoints_count: 0,
            threads_count: 0,
            record_entries_count: 0,
        }
    }

    fn claim_fixture(
        id: &str,
        subject_kind: &str,
        subject_id: &str,
        text: &str,
    ) -> IntelligenceClaim {
        IntelligenceClaim {
            id: id.to_string(),
            claim_version: 1,
            subject_ref: serde_json::json!({
                "kind": subject_kind,
                "id": subject_id,
            })
            .to_string(),
            claim_type: "relationship_health".to_string(),
            field_path: Some("/health".to_string()),
            topic_key: None,
            text: text.to_string(),
            dedup_key: format!("{subject_kind}:{subject_id}:{id}"),
            item_hash: None,
            actor: "test".to_string(),
            data_source: "user".to_string(),
            source_ref: None,
            source_asof: Some("2026-05-23T10:00:00Z".to_string()),
            observed_at: "2026-05-23T10:00:00Z".to_string(),
            created_at: "2026-05-23T10:00:00Z".to_string(),
            provenance_json: "{}".to_string(),
            metadata_json: None,
            claim_state: ClaimState::Active,
            surfacing_state: SurfacingState::Active,
            demotion_reason: None,
            reactivated_at: None,
            retraction_reason: None,
            expires_at: None,
            superseded_by: None,
            trust_score: Some(0.91),
            trust_computed_at: None,
            trust_version: None,
            thread_id: None,
            temporal_scope: TemporalScope::State,
            sensitivity: ClaimSensitivity::Internal,
            verification_state: ClaimVerificationState::Active,
            verification_reason: None,
            needs_user_decision_at: None,
        }
    }

    fn prompt_safe_before_cap_claims() -> Vec<IntelligenceClaim> {
        let mut claims = Vec::new();
        for index in 0..=DEFAULT_PAGE_SIZE {
            let mut claim = claim_fixture(
                &format!("claim-confidential-newer-{index}"),
                "account",
                "acct-test-001",
                &format!("Confidential claim {index}"),
            );
            claim.sensitivity = ClaimSensitivity::Confidential;
            claim.created_at = format!("2026-05-23T11:{index:02}:00Z");
            claims.push(claim);
        }

        let mut safe_claim = claim_fixture(
            "claim-internal-older",
            "account",
            "acct-test-001",
            "Older prompt-safe claim should survive the MCP bounded read.",
        );
        safe_claim.sensitivity = ClaimSensitivity::Internal;
        safe_claim.created_at = "2026-05-23T10:00:00Z".to_string();
        claims.push(safe_claim);
        claims
    }

    #[test]
    fn claim_freshness_falls_back_to_observed_at_when_source_asof_missing() {
        let mut claim = claim_fixture(
            "claim-observed-at-only",
            "account",
            "acct-test-001",
            "Observed claim",
        );
        claim.source_asof = None;
        claim.observed_at = chrono::Utc::now().to_rfc3339();

        assert_eq!(freshness_for_claim(&claim), Freshness::Current);
    }

    #[test]
    fn sections_map_enumerates_all_variants_when_no_filter() {
        let map = build_sections_map(ENVELOPE_SCHEMA_VERSION_V1, &None, fill(0, 0));
        assert_eq!(map.len(), EnvelopeSection::V1.len());
        for section in EnvelopeSection::V1 {
            assert!(map.contains_key(section), "missing section {section:?}");
        }
    }

    #[tokio::test]
    async fn producer_finalizes_empty_claim_envelope_with_outer_provenance() {
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 22, 12, 0, 0).unwrap());
        let rng = SystemRng;
        let services = ServiceContext::new_evaluate_default(&clock, &rng)
            .with_entity_context_claim_reader(Arc::new(EmptyClaimReader));
        let provider = ReplayProvider::new(std::collections::HashMap::new());
        let ctx = AbilityContext::new(
            &services,
            &provider,
            &NOOP_ABILITY_TRACER,
            Actor::User,
            None,
            ClaimDismissalSurface::TauriEntityDetail,
        );

        let output = build_entity_intelligence(
            &ctx,
            EntityIntelligenceInput {
                schema_version: 1,
                entity_type: EntityKind::Account,
                entity_id: "acct-test-001".to_string(),
                depth: EnvelopeContextDepth::Deep,
                sections: Some(vec![EnvelopeSection::Facts, EnvelopeSection::Record]),
            },
        )
        .await
        .expect("producer should return a typed empty envelope");

        assert_eq!(output.data().subject.id, "acct-test-001");
        assert!(output.data().facts.items.is_empty());
    }

    #[tokio::test]
    async fn agent_and_mcp_producer_read_prompt_safe_claims_before_page_cap() {
        for actor in [
            Actor::Agent,
            Actor::McpClient {
                client_id: McpClientId::new("mcp-test"),
                conversation_handle: None,
            },
        ] {
            let actor_label = format!("{actor:?}");
            let clock =
                FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 22, 12, 0, 0).unwrap());
            let rng = SystemRng;
            let services = ServiceContext::new_evaluate_default(&clock, &rng)
                .with_entity_context_claim_reader(Arc::new(PromptSafeBeforeCapClaimReader));
            let provider = ReplayProvider::new(std::collections::HashMap::new());
            let ctx = AbilityContext::new(
                &services,
                &provider,
                &NOOP_ABILITY_TRACER,
                actor,
                None,
                ClaimDismissalSurface::McpTool,
            );

            let output = build_entity_intelligence(
                &ctx,
                EntityIntelligenceInput {
                    schema_version: ENVELOPE_SCHEMA_VERSION_V2,
                    entity_type: EntityKind::Account,
                    entity_id: "acct-test-001".to_string(),
                    depth: EnvelopeContextDepth::Standard,
                    sections: Some(vec![EnvelopeSection::Facts]),
                },
            )
            .await
            .expect("prompt actors should preserve prompt-safe claims behind confidential rows");

            assert_eq!(
                output
                    .data()
                    .facts
                    .items
                    .iter()
                    .map(|fact| fact.claim_id.as_str())
                    .collect::<Vec<_>>(),
                vec!["claim-internal-older"],
                "{actor_label} entity intelligence must call the prompt-safe reader before applying the page cap"
            );
        }
    }

    #[tokio::test]
    async fn schema_v2_relationships_section_projects_neighborhood_evidence() {
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 22, 12, 0, 0).unwrap());
        let rng = SystemRng;
        let services = ServiceContext::new_evaluate_default(&clock, &rng)
            .with_entity_context_claim_reader(Arc::new(EmptyClaimReader))
            .with_entity_neighborhood_reader(Arc::new(FixtureNeighborhoodReader));
        let provider = ReplayProvider::new(std::collections::HashMap::new());
        let ctx = AbilityContext::new(
            &services,
            &provider,
            &NOOP_ABILITY_TRACER,
            Actor::User,
            None,
            ClaimDismissalSurface::McpTool,
        );

        let output = build_entity_intelligence(
            &ctx,
            EntityIntelligenceInput {
                schema_version: ENVELOPE_SCHEMA_VERSION_V2,
                entity_type: EntityKind::Account,
                entity_id: "acct-test-001".to_string(),
                depth: EnvelopeContextDepth::Standard,
                sections: Some(vec![EnvelopeSection::Relationships]),
            },
        )
        .await
        .expect("schema v2 should project relationship evidence");

        let relationships = output
            .data()
            .relationships
            .as_ref()
            .expect("schema v2 output should include relationships");
        let bundle = relationships.items.first().expect("relationship bundle");
        assert_eq!(bundle.edges.items.len(), 1);
        assert_eq!(bundle.participants.items.len(), 1);
        assert_eq!(bundle.participants.items[0].normalized_touchpoint_count, 3);
        assert!(matches!(
            output.data().sections.get(&EnvelopeSection::Relationships),
            Some(SectionState::Present { item_count: 2 })
        ));
        assert!(output
            .data()
            .sections
            .contains_key(&EnvelopeSection::Relationships));
    }

    #[tokio::test]
    async fn relationship_reader_failure_marks_relationship_section_partial_failure() {
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 22, 12, 0, 0).unwrap());
        let rng = SystemRng;
        let services = ServiceContext::new_evaluate_default(&clock, &rng)
            .with_entity_context_claim_reader(Arc::new(EmptyClaimReader))
            .with_entity_neighborhood_reader(Arc::new(FailingNeighborhoodReader));
        let provider = ReplayProvider::new(std::collections::HashMap::new());
        let ctx = AbilityContext::new(
            &services,
            &provider,
            &NOOP_ABILITY_TRACER,
            Actor::User,
            None,
            ClaimDismissalSurface::McpTool,
        );

        let output = build_entity_intelligence(
            &ctx,
            EntityIntelligenceInput {
                schema_version: ENVELOPE_SCHEMA_VERSION_V2,
                entity_type: EntityKind::Account,
                entity_id: "acct-test-001".to_string(),
                depth: EnvelopeContextDepth::Standard,
                sections: Some(vec![EnvelopeSection::Relationships]),
            },
        )
        .await
        .expect("relationship read failure should not fail the whole envelope");

        assert!(matches!(
            output.data().sections.get(&EnvelopeSection::Relationships),
            Some(SectionState::Empty {
                reason: EmptyReason::PartialFailure { .. }
            })
        ));
        assert!(output
            .data()
            .trust
            .section_caveats
            .get(&EnvelopeSection::Relationships)
            .is_some_and(|caveat| caveat.contains("relationships reader unavailable")));
    }

    #[tokio::test]
    async fn relationship_participants_contribute_to_envelope_sensitivity() {
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 22, 12, 0, 0).unwrap());
        let rng = SystemRng;
        let services = ServiceContext::new_evaluate_default(&clock, &rng)
            .with_entity_context_claim_reader(Arc::new(EmptyClaimReader))
            .with_entity_neighborhood_reader(Arc::new(ParticipantOnlyNeighborhoodReader));
        let provider = ReplayProvider::new(std::collections::HashMap::new());
        let ctx = AbilityContext::new(
            &services,
            &provider,
            &NOOP_ABILITY_TRACER,
            Actor::User,
            None,
            ClaimDismissalSurface::McpTool,
        );

        let output = build_entity_intelligence(
            &ctx,
            EntityIntelligenceInput {
                schema_version: ENVELOPE_SCHEMA_VERSION_V2,
                entity_type: EntityKind::Account,
                entity_id: "acct-test-001".to_string(),
                depth: EnvelopeContextDepth::Standard,
                sections: Some(vec![EnvelopeSection::Relationships]),
            },
        )
        .await
        .expect("participant-only relationship evidence should compose");

        assert_eq!(output.data().sensitivity, ClaimSensitivity::Confidential);
        let participant = output
            .data()
            .relationships
            .as_ref()
            .and_then(|relationships| relationships.items.first())
            .and_then(|bundle| bundle.participants.items.first())
            .expect("participant relationship evidence");
        assert_eq!(participant.sensitivity, ClaimSensitivity::Confidential);
        assert_eq!(
            participant
                .relationship
                .as_ref()
                .map(|relationship| relationship.text.as_str()),
            Some("[redacted]")
        );
    }

    #[tokio::test]
    async fn schema_v1_rejects_relationships_section_request() {
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 22, 12, 0, 0).unwrap());
        let rng = SystemRng;
        let services = ServiceContext::new_evaluate_default(&clock, &rng)
            .with_entity_context_claim_reader(Arc::new(EmptyClaimReader));
        let provider = ReplayProvider::new(std::collections::HashMap::new());
        let ctx = AbilityContext::new(
            &services,
            &provider,
            &NOOP_ABILITY_TRACER,
            Actor::User,
            None,
            ClaimDismissalSurface::McpTool,
        );

        let err = build_entity_intelligence(
            &ctx,
            EntityIntelligenceInput {
                schema_version: ENVELOPE_SCHEMA_VERSION_V1,
                entity_type: EntityKind::Account,
                entity_id: "acct-test-001".to_string(),
                depth: EnvelopeContextDepth::Standard,
                sections: Some(vec![EnvelopeSection::Relationships]),
            },
        )
        .await
        .expect_err("schema v1 must reject relationships requests");

        assert!(err.message.contains("schema_version 2"));
    }

    #[test]
    fn relationship_trust_is_capped_by_freshness() {
        assert_eq!(
            trust_band_for_confidence(0.95, Freshness::Current),
            TrustBand::LikelyCurrent
        );
        assert_eq!(
            trust_band_for_confidence(0.95, Freshness::Unknown),
            TrustBand::UseWithCaution
        );
        assert_eq!(
            trust_band_for_confidence(0.95, Freshness::Stale),
            TrustBand::NeedsVerification
        );
    }

    #[test]
    fn facts_keep_their_claim_subject_instead_of_request_subject() {
        let claims = vec![claim_fixture(
            "claim-acc-b-health",
            "account",
            "acc-b",
            "Account B risk is rising.",
        )];
        let render_actor = RenderActor::user("user", None::<String>);
        let mut provenance = EnvelopeProvenance::empty();

        let facts = build_facts(
            &claims,
            &render_actor,
            RenderSurface::TauriMeetingDetail,
            &mut provenance,
        )
        .expect("facts render");

        assert_eq!(facts.items.len(), 1);
        assert_eq!(
            facts.items[0].subject_ref,
            SubjectRef::Account("acc-b".to_string())
        );
    }

    #[test]
    fn empty_sections_carry_typed_reasons() {
        let map = build_sections_map(ENVELOPE_SCHEMA_VERSION_V1, &None, fill(0, 0));
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
        let map = build_sections_map(
            ENVELOPE_SCHEMA_VERSION_V1,
            &Some(vec![EnvelopeSection::Facts]),
            fill(2, 0),
        );
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
        let map_empty = build_sections_map(ENVELOPE_SCHEMA_VERSION_V1, &Some(vec![]), fill(3, 0));
        let map_none = build_sections_map(ENVELOPE_SCHEMA_VERSION_V1, &None, fill(3, 0));
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
        let v1 = EnvelopeSection::V1;
        assert_eq!(v1[0], EnvelopeSection::Facts);
        assert_eq!(v1[1], EnvelopeSection::Health);
        assert_eq!(v1[6], EnvelopeSection::Record);

        let v2 = EnvelopeSection::V2;
        assert_eq!(v2[0], EnvelopeSection::Facts);
        assert_eq!(v2[4], EnvelopeSection::Relationships);
        assert_eq!(v2[7], EnvelopeSection::Record);
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
        assert_eq!(
            parsed.exclusion_reason,
            Some(ExclusionReason::OutsideWindow)
        );
    }

    #[test]
    fn entity_kind_lowercase_str_matches_legacy() {
        assert_eq!(EntityKind::Account.as_lower_str(), "account");
        assert_eq!(EntityKind::Project.as_lower_str(), "project");
        assert_eq!(EntityKind::Person.as_lower_str(), "person");
        // W2 F2 (cycle-1 codex challenge) — Meeting subject variant.
        assert_eq!(EntityKind::Meeting.as_lower_str(), "meeting");
    }

    #[test]
    fn meeting_subject_ref_round_trip() {
        // W2 F2 — Meeting EntityKind must materialize as `SubjectRef::Meeting`
        // so the per-fact provenance + envelope subject all carry the meeting
        // discriminator consistently with the touchpoint reader output.
        let sr = subject_ref_for(EntityKind::Meeting, "m-42");
        assert_eq!(sr, SubjectRef::Meeting("m-42".to_string()));
    }

    #[test]
    fn entity_kind_meeting_serializes_snake_case() {
        // The TS mirror at `src/services/entity-intelligence/contracts.ts`
        // expects `"meeting"` for the Meeting variant. Confirm the serde
        // rename keeps the wire shape stable.
        let json = serde_json::to_string(&EntityKind::Meeting).expect("serializes");
        assert_eq!(json, "\"meeting\"");
        let parsed: EntityKind = serde_json::from_str("\"meeting\"").expect("round trips");
        assert!(matches!(parsed, EntityKind::Meeting));
    }

    // ---- touchpoint projection + subject-isolation tests ------------------

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
        assert_eq!(
            bundle.empty_reason,
            Some(EmptyReason::NoRelevantTouchpoints)
        );
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
        assert_eq!(
            classify_touchpoint_kind("internal"),
            TouchpointKind::Meeting
        );
        assert_eq!(
            classify_touchpoint_kind("team_sync"),
            TouchpointKind::Meeting
        );
        assert_eq!(
            classify_touchpoint_kind("email_thread"),
            TouchpointKind::EmailThread
        );
        assert_eq!(
            classify_touchpoint_kind("salesforce_call"),
            TouchpointKind::Salesforce
        );
        assert_eq!(
            classify_touchpoint_kind("linear_update"),
            TouchpointKind::Linear
        );
        assert_eq!(
            classify_touchpoint_kind("google_doc"),
            TouchpointKind::Document
        );
        // Defensive: empty + garbage strings → Meeting, never panic.
        assert_eq!(classify_touchpoint_kind(""), TouchpointKind::Meeting);
        assert_eq!(classify_touchpoint_kind("???"), TouchpointKind::Meeting);
    }

    #[test]
    fn parse_exclusion_reason_strict_allowlist() {
        assert_eq!(
            parse_exclusion_reason("subject_mismatch"),
            Some(ExclusionReason::SubjectMismatch)
        );
        assert_eq!(
            parse_exclusion_reason("outside_window"),
            Some(ExclusionReason::OutsideWindow)
        );
        assert_eq!(
            parse_exclusion_reason("low_confidence"),
            Some(ExclusionReason::LowConfidence)
        );
        assert_eq!(
            parse_exclusion_reason("suppressed"),
            Some(ExclusionReason::Suppressed)
        );
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
        assert!(bundle
            .candidate_set
            .filter_description
            .contains("reader unavailable"));
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

    // ---- W2 F2 Meeting health projection (prep status) -------------------

    fn meeting_prep_snapshot_ready(
        meeting_id: &str,
    ) -> crate::services::context::MeetingPrepStatusSnapshot {
        crate::services::context::MeetingPrepStatusSnapshot {
            meeting_id: meeting_id.to_string(),
            event_id: None,
            linked_entity_type: Some("account".to_string()),
            linked_entity_id: Some("acc-1".to_string()),
            status: "ready".to_string(),
            blocking_reason: None,
            stale_reason: None,
            last_prepared_at: Some("2026-05-21T08:00:00Z".to_string()),
            source_asof_inputs: Vec::new(),
        }
    }

    fn meeting_prep_snapshot_blocked(
        meeting_id: &str,
    ) -> crate::services::context::MeetingPrepStatusSnapshot {
        crate::services::context::MeetingPrepStatusSnapshot {
            meeting_id: meeting_id.to_string(),
            event_id: None,
            linked_entity_type: None,
            linked_entity_id: None,
            status: "blocked_no_entity".to_string(),
            blocking_reason: Some("no_linked_entity".to_string()),
            stale_reason: None,
            last_prepared_at: None,
            source_asof_inputs: Vec::new(),
        }
    }

    #[test]
    fn meeting_health_projection_ready_emits_status_and_last_prepared_rows() {
        // W2 F2 — Ready prep status surfaces "Prep status" + "Last prepared"
        // rows, no blocking/stale reason rows. The headline mirrors the
        // PrepStatus string discriminant so consumers can render a
        // trust-band-tinted summary verbatim.
        let snap = meeting_prep_snapshot_ready("m-1");
        let mut prov = EnvelopeProvenance::empty();
        let health = project_meeting_health(&snap, "m-1", &mut prov);
        assert_eq!(health.headline.as_deref(), Some("Meeting prep: ready"));
        let labels: Vec<&str> = health.rows.iter().map(|r| r.label.as_str()).collect();
        assert_eq!(labels, vec!["Prep status", "Last prepared"]);
        // Provenance index upserted once with the meeting_prep id.
        assert_eq!(prov.sources.len(), 1);
        assert_eq!(prov.sources[0].id, "meeting_prep:m-1");
        assert_eq!(
            prov.sources[0].source_type.as_deref(),
            Some("meeting_prep_status")
        );
        assert!(!prov.sources[0].redacted);
    }

    #[test]
    fn meeting_health_projection_blocked_no_entity_surfaces_blocking_reason() {
        // W2 F2 — BlockedNoEntity status carries a blocking_reason row and
        // omits last_prepared_at (the meeting was never prepared). The
        // renderer consumes this row to surface "Link an account/project to
        // unblock prep" affordance per the render contract.
        let snap = meeting_prep_snapshot_blocked("m-blocked");
        let mut prov = EnvelopeProvenance::empty();
        let health = project_meeting_health(&snap, "m-blocked", &mut prov);
        let labels: Vec<&str> = health.rows.iter().map(|r| r.label.as_str()).collect();
        assert_eq!(labels, vec!["Prep status", "Blocking reason"]);
        let bodies: Vec<&str> = health.rows.iter().map(|r| r.body.as_str()).collect();
        assert!(bodies.contains(&"no_linked_entity"));
        // No "Last prepared" row when last_prepared_at is None.
        assert!(!labels.contains(&"Last prepared"));
    }

    #[test]
    fn meeting_health_projection_stale_surfaces_stale_reason_row() {
        // W2 F2 — Stale prep folds the stale_reason into the HealthStory so
        // the Meeting Detail surface can name *why* prep is stale (upstream
        // claim invalidation).
        let snap = crate::services::context::MeetingPrepStatusSnapshot {
            meeting_id: "m-stale".to_string(),
            event_id: None,
            linked_entity_type: Some("account".to_string()),
            linked_entity_id: Some("acc-1".to_string()),
            status: "stale".to_string(),
            blocking_reason: None,
            stale_reason: Some("upstream_claim_changed".to_string()),
            last_prepared_at: Some("2026-05-01T08:00:00Z".to_string()),
            source_asof_inputs: Vec::new(),
        };
        let mut prov = EnvelopeProvenance::empty();
        let health = project_meeting_health(&snap, "m-stale", &mut prov);
        let labels: Vec<&str> = health.rows.iter().map(|r| r.label.as_str()).collect();
        assert!(labels.contains(&"Stale reason"));
        assert!(labels.contains(&"Last prepared"));
    }

    #[test]
    fn meeting_health_projection_provenance_carries_last_prepared_as_of() {
        // F2 audience-filter (cycle-2 §F2 pattern) — the meeting prep
        // provenance source must carry `as_of` from `last_prepared_at` so
        // freshness rendering at the consumer side can compute freshness
        // without re-reading the snapshot.
        let snap = meeting_prep_snapshot_ready("m-asof");
        let mut prov = EnvelopeProvenance::empty();
        let _ = project_meeting_health(&snap, "m-asof", &mut prov);
        let source = prov
            .sources
            .iter()
            .find(|s| s.id == "meeting_prep:m-asof")
            .expect("prep source present");
        assert!(source.as_of.is_some());
    }

    #[test]
    fn meeting_subject_facts_render_through_audience_for_touchpoint_label() {
        // W2 F2 — touchpoint provenance label routes through the same
        // audience-aware scrub used for Account/Project/Person subjects.
        // The Meeting subject re-uses the existing render_actor gate, so
        // the AgentMcp surface sees redacted titles even when the subject
        // is itself a meeting (no special-case bypass).
        let now = chrono::Utc::now();
        let snapshot = fake_snapshot(
            "meeting",
            "m-host",
            vec![fake_touchpoint(
                "m-related",
                "meeting",
                "m-host",
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
            &SubjectRef::Meeting("m-host".to_string()),
            &now,
            &agent_actor,
            &mut prov,
        );
        assert_eq!(bundle.upcoming.items.len(), 1);
        let meeting_source = prov
            .sources
            .iter()
            .find(|s| s.id == "meeting:m-related")
            .expect("touchpoint meeting source present");
        assert!(
            meeting_source.redacted,
            "agent audience must see touchpoint provenance redacted regardless of meeting subject"
        );
        assert_eq!(meeting_source.label, "Meeting (redacted)");
    }

    #[test]
    fn meeting_subject_user_audience_preserves_touchpoint_title() {
        // Symmetric — UserTauri audience for a Meeting subject sees the raw
        // related-meeting title. The audience filter is end-to-end: the
        // user surface gets full fidelity, the agent surface gets the
        // redacted placeholder, in both cases the envelope still carries the
        // touchpoint shape (no information shape leak).
        let now = chrono::Utc::now();
        let snapshot = fake_snapshot(
            "meeting",
            "m-host",
            vec![fake_touchpoint(
                "m-related",
                "meeting",
                "m-host",
                TouchpointInclusionReason::SubjectMatch,
                now + chrono::Duration::days(1),
            )],
            vec![],
            vec![],
        );
        let mut prov = EnvelopeProvenance::empty();
        let user_actor = RenderActor::user("user", Some("user-1".to_string()));
        let _ = project_touchpoints_bundle(
            &snapshot,
            &SubjectRef::Meeting("m-host".to_string()),
            &now,
            &user_actor,
            &mut prov,
        );
        let meeting_source = prov
            .sources
            .iter()
            .find(|s| s.id == "meeting:m-related")
            .expect("touchpoint meeting source present");
        assert!(!meeting_source.redacted);
        assert_eq!(meeting_source.label, "Meeting m-related");
    }
}
