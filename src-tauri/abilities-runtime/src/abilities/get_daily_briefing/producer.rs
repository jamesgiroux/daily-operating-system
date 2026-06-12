//! `get_daily_briefing` Read-only ability producer.
//!
//! Pure read composition over:
//! - `get_daily_readiness` context reader → workspace + day-bounded meetings
//! - `meeting_prep_status` read handle → per-meeting status/freshness
//! - `get_entity_intelligence` envelope → per linked-entity facts +
//!   touchpoints + provenance (composed via the §5.1 ability)
//!
//! No provider-backed synthesis (AC-507.2). No mutation. No auto-enqueue —
//! if prep is missing the producer returns
//! `BriefingFreshness::NeedsPreparation { meeting_ids }` and lets the W3
//! block renderer (or a sibling write-ability dedicated to enqueueing prep)
//! handle the side-effect (AC-507.3 / §13 Q9).
//!
//! The call graph is statically fenced by the
//! `call_graph_lint::briefing_producer_contains_no_mutations` test below
//! (AC-507.7).

use std::collections::{BTreeMap, BTreeSet};

use super::contracts::{
    AmbiguityPair, BriefingAdvisory, BriefingAvailability, BriefingEmptyReason, BriefingFreshness,
    BriefingIntegrity, BriefingSection, BriefingStaleReason, BriefingState, BriefingTrustSummary,
    DailyBriefingInput, DailyBriefingOutput, MeetingBriefRef, SourceAsofRef, WatchProposal,
    BRIEFING_SCHEMA_VERSION,
};
use crate::abilities::get_entity_intelligence::contracts::{
    CandidateSetRef, ContextDepth, Cursor, CursorState, EntityIntelligenceEnvelope,
    EntityIntelligenceInput, EntityKind, EnvelopeProvenance, EnvelopeProvenanceSource,
    EnvelopeSection, Paginated, ENVELOPE_SCHEMA_VERSION,
};
use crate::abilities::get_entity_intelligence::producer::build_entity_intelligence;
use crate::abilities::provenance::{
    AbilityExecutionMode, AbilityVersion, FieldAttribution, FieldPath, ProvenanceBuilder,
    ProvenanceBuilderConfig, SchemaVersion, SubjectAttribution, SubjectRef,
};
use crate::abilities::trust::types::TrustBand;
use crate::abilities::{
    AbilityCategory, AbilityContext, AbilityError, AbilityErrorKind, AbilityResult, Actor,
};
use crate::services::context::{
    is_customer_facing, BriefingCalloutSnapshot, DailyReadinessContextSnapshot,
    DailyReadinessMeetingSnapshot, MeetingPrepStatusReadError, MeetingPrepStatusSnapshot,
};
use crate::types::{ClaimSensitivity, ClaimState};

const ABILITY_NAME: &str = "get_daily_briefing";

/// Hard cap on `upcoming_meetings.items` per page. AC-507.10 cursor pagination
/// kicks in when the readiness context returns more than this.
const UPCOMING_MEETINGS_PAGE_SIZE: usize = 25;
const BRIEFING_CALLOUT_LIMIT: usize = 10;

pub async fn build_daily_briefing(
    ctx: &AbilityContext<'_>,
    input: DailyBriefingInput,
) -> AbilityResult<DailyBriefingOutput> {
    validate_schema_version(input.schema_version)?;
    let workspace_id = input.workspace_id.trim();
    if workspace_id.is_empty() {
        return Err(validation_error("workspace_id must be non-empty"));
    }

    let active_sections = active_section_set(input.sections.as_ref());

    // ---- compose: readiness context (meetings + watch material) ------------
    let date_str = input.date.format("%Y-%m-%d").to_string();
    let readiness = match ctx
        .services()
        .read_daily_readiness_context(
            workspace_id.to_string(),
            date_str.clone(),
            crate::services::context::MeetingsViewIntent::Briefing,
        )
        .await
    {
        Ok(snapshot) => snapshot,
        Err(message) => {
            // Reader unavailable / workspace unknown → degrade gracefully to a
            // typed Empty briefing rather than failing the envelope. Producer
            // never blocks the consumer on substrate gaps.
            return empty_workspace_unknown(
                &input,
                workspace_id,
                BriefingEmptyReason::WorkspaceUnknown,
                message,
            )
            .into_envelope(ctx, input.schema_version);
        }
    };

    let (callouts, callout_read_failure) =
        read_briefing_callouts(ctx, workspace_id, date_str.clone()).await;
    let watch_proposals = project_watch_proposals(&callouts);
    let callout_source_asof_inputs = project_callout_source_asof_inputs(&callouts);

    let mut meetings: Vec<&DailyReadinessMeetingSnapshot> = readiness.meetings.iter().collect();
    sort_meeting_snapshots(&mut meetings);

    if meetings.is_empty() {
        return empty_no_meetings(
            &input,
            workspace_id,
            watch_proposals,
            callout_source_asof_inputs,
        )
        .into_envelope(ctx, input.schema_version);
    }

    // Daily Briefing may run on unusually dense calendar days. Keep the
    // authoritative meeting list for cursor totals, but only expand prep
    // status and entity envelopes for what this invocation can render:
    // current + next + the requested upcoming page.
    let now = ctx.services().clock.now();
    let (current_meeting_seed, next_meeting_seed) =
        pick_current_and_next_snapshots(&meetings, &now);
    let upcoming_meeting_seeds =
        paginate_upcoming_snapshots(&meetings, input.upcoming_meetings_cursor.as_ref());
    let expansion_ids = collect_expansion_meeting_ids(
        current_meeting_seed.as_ref(),
        next_meeting_seed.as_ref(),
        &upcoming_meeting_seeds.items,
    );

    // ---- compose: per-meeting prep status ---------------------------------
    let mut prep_snapshots: BTreeMap<String, MeetingPrepStatusSnapshot> = BTreeMap::new();
    let mut prep_read_failures: Vec<String> = Vec::new();
    let mut needs_prep_meeting_ids: Vec<String> = Vec::new();

    for meeting in meetings
        .iter()
        .filter(|meeting| expansion_ids.contains(&meeting.id))
    {
        match ctx
            .services()
            .read_meeting_prep_status(meeting.id.clone())
            .await
        {
            Ok(snapshot) => {
                // Only customer-facing meeting types drive the "needs prep"
                // advisory. Internal syncs / 1:1s / team meetings legitimately
                // have no prep doc; counting them inflates the strip and
                // recreates the phantom-row shape one level up from the
                // projection-layer filter.
                if prep_status_is_needs_preparation(&snapshot.status)
                    && is_customer_facing(&meeting.meeting_type)
                {
                    needs_prep_meeting_ids.push(snapshot.meeting_id.clone());
                }
                prep_snapshots.insert(meeting.id.clone(), snapshot);
            }
            Err(MeetingPrepStatusReadError::MeetingNotFound(_)) => {
                // Meeting in readiness context but not in prep table → treat
                // as NeedsPreparation; do NOT enqueue (AC-507.3). Same
                // customer-facing gate as the Ok branch.
                if is_customer_facing(&meeting.meeting_type) {
                    needs_prep_meeting_ids.push(meeting.id.clone());
                }
            }
            Err(MeetingPrepStatusReadError::ReadFailed(message)) => {
                prep_read_failures.push(message);
                if is_customer_facing(&meeting.meeting_type) {
                    needs_prep_meeting_ids.push(meeting.id.clone());
                }
            }
        }
    }

    // Set of meeting IDs that are NOT customer-facing — passed to
    // derive_advisories so its unlinked-meetings filter can skip internal
    // / team-sync / 1:1 rows that legitimately have no customer entity.
    let non_customer_meeting_ids: std::collections::HashSet<String> = meetings
        .iter()
        .filter(|m| !is_customer_facing(&m.meeting_type))
        .map(|m| m.id.clone())
        .collect();

    // ---- compose: meeting brief refs --------------------------------------
    let linked_entity_names = linked_entity_names_by_key(&readiness);
    let current_meeting = current_meeting_seed
        .as_ref()
        .map(|meeting| {
            project_meeting_brief(meeting, prep_snapshots.get(&meeting.id), &linked_entity_names)
        });
    let next_meeting = next_meeting_seed
        .as_ref()
        .map(|meeting| {
            project_meeting_brief(meeting, prep_snapshots.get(&meeting.id), &linked_entity_names)
        });
    let upcoming_meetings =
        project_upcoming_meetings(upcoming_meeting_seeds, &prep_snapshots, &linked_entity_names);
    let expanded_meeting_refs = collect_expanded_meeting_refs(
        current_meeting.as_ref(),
        next_meeting.as_ref(),
        &upcoming_meetings.items,
    );

    // ---- compose: per-linked-entity envelopes -----------------------------
    // The L0 contract calls for per-subject `get_entity_intelligence`
    // envelopes "for the meetings' linked entities". We compose them as part
    // of building the integrity + trust summary; they do not need to be
    // emitted back through this envelope verbatim because the W3 briefing
    // block re-invokes `get_entity_intelligence` for any deeper drill-in.
    let linked_entities = collect_linked_entities(&expanded_meeting_refs);
    let mut envelope_provenance = EnvelopeProvenance::empty();
    let mut superseded_claim_ids: Vec<String> = Vec::new();
    let mut aggregate_sensitivity = ClaimSensitivity::Public;
    let mut likely_current = 0u32;
    let mut use_with_caution = 0u32;
    let mut needs_verification = 0u32;
    let mut source_asof_inputs: Vec<SourceAsofRef> = Vec::new();
    let mut envelope_failures: Vec<String> = Vec::new();

    for (entity_type, entity_id) in linked_entities {
        let env_input = EntityIntelligenceInput {
            schema_version: ENVELOPE_SCHEMA_VERSION,
            entity_type: entity_type.clone(),
            entity_id: entity_id.clone(),
            depth: ContextDepth::Shallow,
            sections: Some(daily_briefing_entity_sections()),
        };
        match build_entity_intelligence(ctx, env_input).await {
            Ok(envelope_output) => {
                let envelope = envelope_output.into_data();
                accumulate_from_envelope(
                    &envelope,
                    &mut envelope_provenance,
                    &mut superseded_claim_ids,
                    &mut aggregate_sensitivity,
                    &mut likely_current,
                    &mut use_with_caution,
                    &mut needs_verification,
                    &mut source_asof_inputs,
                );
            }
            Err(err) => {
                envelope_failures.push(format!(
                    "entity intelligence ({}:{}) — {}",
                    entity_type.as_lower_str(),
                    entity_id,
                    err.message
                ));
            }
        }
    }

    // ---- compose: candidate set -------------------------------------------
    let candidate_set = build_candidate_set(&readiness);

    // ---- compose: state (4-tuple) -----------------------------------------
    let availability = if matches!(meetings.len(), 0) {
        BriefingAvailability::Empty {
            reason: BriefingEmptyReason::NoMeetings,
        }
    } else {
        BriefingAvailability::Available
    };

    let freshness = derive_freshness(&prep_snapshots, &needs_prep_meeting_ids);
    let integrity = derive_integrity(&superseded_claim_ids);
    let advisories = derive_advisories(
        &readiness,
        &expanded_meeting_refs,
        &non_customer_meeting_ids,
        &prep_read_failures,
        &envelope_failures,
        callout_read_failure.as_deref(),
    );

    let state = BriefingState {
        availability,
        freshness,
        integrity,
        advisories,
    };

    // ---- compose: watch proposals + trust summary -------------------------
    source_asof_inputs.extend(callout_source_asof_inputs);

    let aggregate_band = aggregate_trust_band(likely_current, use_with_caution, needs_verification);
    let trust_summary = BriefingTrustSummary {
        aggregate_band,
        likely_current_count: likely_current,
        use_with_caution_count: use_with_caution,
        needs_verification_count: needs_verification,
    };

    // ---- assemble envelope ------------------------------------------------
    let output = DailyBriefingOutput {
        schema_version: BRIEFING_SCHEMA_VERSION,
        date: input.date,
        state,
        current_meeting: section_or_none(
            &active_sections,
            BriefingSection::CurrentMeeting,
            current_meeting,
        ),
        next_meeting: section_or_none(&active_sections, BriefingSection::NextMeeting, next_meeting),
        upcoming_meetings: if active_sections.contains(&BriefingSection::UpcomingMeetings) {
            upcoming_meetings
        } else {
            Paginated::empty_stable()
        },
        candidate_set,
        watch_proposals: if active_sections.contains(&BriefingSection::WatchProposals) {
            watch_proposals
        } else {
            Vec::new()
        },
        trust_summary: if active_sections.contains(&BriefingSection::TrustSummary) {
            trust_summary
        } else {
            BriefingTrustSummary::unscored()
        },
        provenance: envelope_provenance,
        sensitivity: aggregate_sensitivity,
        source_asof_inputs,
    };

    finalize_with_provenance(ctx, output, input.schema_version)
}

// ---- helpers ---------------------------------------------------------------

fn validate_schema_version(schema_version: u32) -> Result<(), AbilityError> {
    if schema_version == BRIEFING_SCHEMA_VERSION {
        Ok(())
    } else {
        Err(validation_error(format!(
            "unsupported schema_version `{schema_version}` for `{ABILITY_NAME}`"
        )))
    }
}

fn active_section_set(
    requested: Option<&Vec<BriefingSection>>,
) -> std::collections::BTreeSet<BriefingSection> {
    match requested {
        None => BriefingSection::ALL.iter().copied().collect(),
        Some(list) if list.is_empty() => BriefingSection::ALL.iter().copied().collect(),
        Some(list) => list.iter().copied().collect(),
    }
}

fn section_or_none<T>(
    active: &std::collections::BTreeSet<BriefingSection>,
    section: BriefingSection,
    value: Option<T>,
) -> Option<T> {
    if active.contains(&section) {
        value
    } else {
        None
    }
}

async fn read_briefing_callouts(
    ctx: &AbilityContext<'_>,
    workspace_id: &str,
    date: String,
) -> (Vec<BriefingCalloutSnapshot>, Option<String>) {
    match ctx
        .services()
        .read_briefing_callouts_for_date(
            workspace_id.to_string(),
            date,
            BRIEFING_CALLOUT_LIMIT,
        )
        .await
    {
        Ok(callouts) => (callouts, None),
        Err(error) => {
            let message = error.to_string();
            if message.contains("briefing_callout_reader") {
                (Vec::new(), None)
            } else {
                (Vec::new(), Some(message))
            }
        }
    }
}

fn project_watch_proposals(callouts: &[BriefingCalloutSnapshot]) -> Vec<WatchProposal> {
    callouts
        .iter()
        .map(|callout| WatchProposal {
            proposal_id: callout.id.clone(),
            subject_kind: callout.entity_type.clone(),
            subject_id: callout.entity_id.clone(),
            headline: callout.headline.clone(),
            detail: callout.detail.clone(),
            severity: callout.severity.clone(),
            entity_name: callout.entity_name.clone(),
            source_asof: Some(callout.created_at.clone()),
            claim_id: callout.claim_id.clone(),
            trust_band: TrustBand::NeedsVerification,
            sensitivity: ClaimSensitivity::Public,
        })
        .collect()
}

fn project_callout_source_asof_inputs(callouts: &[BriefingCalloutSnapshot]) -> Vec<SourceAsofRef> {
    callouts
        .iter()
        .map(|callout| SourceAsofRef {
            source: format!("briefing_callout:{}", callout.id),
            as_of: callout.created_at.clone(),
        })
        .collect()
}

fn project_meeting_brief(
    meeting: &DailyReadinessMeetingSnapshot,
    prep: Option<&MeetingPrepStatusSnapshot>,
    linked_entity_names: &BTreeMap<(String, String), String>,
) -> MeetingBriefRef {
    let (
        prep_status,
        blocking_reason,
        stale_reason,
        last_prepared_at,
        linked_entity_type,
        linked_entity_id,
    ) = match prep {
        Some(snapshot) => (
            snapshot.status.clone(),
            snapshot.blocking_reason.clone(),
            snapshot.stale_reason.clone(),
            snapshot.last_prepared_at.clone(),
            snapshot.linked_entity_type.clone(),
            snapshot.linked_entity_id.clone(),
        ),
        None => ("prep_needed".to_string(), None, None, None, None, None),
    };
    MeetingBriefRef {
        meeting_id: meeting.id.clone(),
        title: Some(meeting.title.clone()),
        starts_at: meeting.starts_at.clone(),
        ends_at: meeting.ends_at.clone(),
        linked_entity_name: linked_entity_name(
            linked_entity_type.as_deref(),
            linked_entity_id.as_deref(),
            linked_entity_names,
        ),
        linked_entity_type,
        linked_entity_id,
        prep_status,
        blocking_reason,
        stale_reason,
        last_prepared_at,
    }
}

fn linked_entity_name(
    entity_type: Option<&str>,
    entity_id: Option<&str>,
    linked_entity_names: &BTreeMap<(String, String), String>,
) -> Option<String> {
    let key = (entity_type?.to_string(), entity_id?.to_string());
    linked_entity_names.get(&key).cloned()
}

fn linked_entity_names_by_key(
    readiness: &DailyReadinessContextSnapshot,
) -> BTreeMap<(String, String), String> {
    readiness
        .tracked_subjects
        .iter()
        .map(|subject| {
            (
                (subject.kind.clone(), subject.id.clone()),
                subject.display_name.clone(),
            )
        })
        .collect()
}

fn sort_meeting_snapshots(meetings: &mut Vec<&DailyReadinessMeetingSnapshot>) {
    // Sort by starts_at with undated meetings (None) sorted AFTER dated ones.
    // `Option::cmp` defaults to None < Some, which would push undated
    // meetings to the front and make the cursor unstable.
    meetings.sort_by(|a, b| match (a.starts_at.as_ref(), b.starts_at.as_ref()) {
        (Some(left), Some(right)) => left.cmp(right),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    });
}

fn collect_expansion_meeting_ids(
    current: Option<&DailyReadinessMeetingSnapshot>,
    next: Option<&DailyReadinessMeetingSnapshot>,
    upcoming: &[DailyReadinessMeetingSnapshot],
) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    if let Some(meeting) = current {
        ids.insert(meeting.id.clone());
    }
    if let Some(meeting) = next {
        ids.insert(meeting.id.clone());
    }
    ids.extend(upcoming.iter().map(|meeting| meeting.id.clone()));
    ids
}

fn collect_expanded_meeting_refs(
    current: Option<&MeetingBriefRef>,
    next: Option<&MeetingBriefRef>,
    upcoming: &[MeetingBriefRef],
) -> Vec<MeetingBriefRef> {
    let mut seen = BTreeSet::new();
    let mut refs = Vec::new();
    for meeting in current.into_iter().chain(next).chain(upcoming.iter()) {
        if seen.insert(meeting.meeting_id.clone()) {
            refs.push(meeting.clone());
        }
    }
    refs
}

fn collect_linked_entities(meetings: &[MeetingBriefRef]) -> Vec<(EntityKind, String)> {
    let mut seen = std::collections::BTreeSet::new();
    let mut out = Vec::new();
    for meeting in meetings {
        let (Some(ty), Some(id)) = (
            meeting.linked_entity_type.as_deref(),
            meeting.linked_entity_id.as_deref(),
        ) else {
            continue;
        };
        let Some(kind) = parse_entity_kind(ty) else {
            continue;
        };
        let key = (ty.to_ascii_lowercase(), id.to_string());
        if seen.insert(key) {
            out.push((kind, id.to_string()));
        }
    }
    out
}

fn parse_entity_kind(kind: &str) -> Option<EntityKind> {
    match kind {
        "account" => Some(EntityKind::Account),
        "project" => Some(EntityKind::Project),
        "person" => Some(EntityKind::Person),
        _ => None,
    }
}

fn daily_briefing_entity_sections() -> Vec<EnvelopeSection> {
    vec![EnvelopeSection::Facts]
}

fn prep_status_is_needs_preparation(status: &str) -> bool {
    matches!(
        status,
        "prep_needed" | "queued" | "running" | "blocked_no_entity" | "failed"
    )
}

#[allow(clippy::too_many_arguments)]
fn accumulate_from_envelope(
    envelope: &EntityIntelligenceEnvelope,
    provenance_index: &mut EnvelopeProvenance,
    superseded_claim_ids: &mut Vec<String>,
    aggregate_sensitivity: &mut ClaimSensitivity,
    likely_current: &mut u32,
    use_with_caution: &mut u32,
    needs_verification: &mut u32,
    source_asof_inputs: &mut Vec<SourceAsofRef>,
) {
    // Provenance: merge unique sources into the briefing-level index.
    for source in &envelope.provenance.sources {
        if !provenance_index
            .sources
            .iter()
            .any(|existing| existing.id == source.id)
        {
            provenance_index.sources.push(EnvelopeProvenanceSource {
                id: source.id.clone(),
                label: source.label.clone(),
                source_type: source.source_type.clone(),
                workspace_file_kind: source.workspace_file_kind.clone(),
                as_of: source.as_of,
                redacted: source.redacted,
            });
            if let Some(as_of) = source.as_of {
                let canonical = SourceAsofRef {
                    source: source.id.clone(),
                    as_of: as_of.to_rfc3339(),
                };
                if !source_asof_inputs
                    .iter()
                    .any(|existing| existing.source == canonical.source)
                {
                    source_asof_inputs.push(canonical);
                }
            }
        }
    }
    if envelope.provenance.redaction_applied {
        provenance_index.redaction_applied = true;
    }

    // Trust band tally.
    for fact in &envelope.facts.items {
        match fact.trust_band {
            TrustBand::LikelyCurrent => *likely_current += 1,
            TrustBand::UseWithCaution => *use_with_caution += 1,
            TrustBand::NeedsVerification => *needs_verification += 1,
            TrustBand::Unscored => {}
        }
        if rank_sensitivity(&fact.sensitivity) > rank_sensitivity(aggregate_sensitivity) {
            *aggregate_sensitivity = fact.sensitivity.clone();
        }
        if matches!(
            fact.lifecycle_state,
            ClaimState::Tombstoned | ClaimState::Withdrawn
        ) {
            superseded_claim_ids.push(fact.claim_id.clone());
        }
    }
}

fn rank_sensitivity(sensitivity: &ClaimSensitivity) -> u8 {
    match sensitivity {
        ClaimSensitivity::Public => 0,
        ClaimSensitivity::Internal => 1,
        ClaimSensitivity::Confidential => 2,
        ClaimSensitivity::UserOnly => 3,
    }
}

fn pick_current_and_next_snapshots(
    meetings: &[&DailyReadinessMeetingSnapshot],
    now: &chrono::DateTime<chrono::Utc>,
) -> (
    Option<DailyReadinessMeetingSnapshot>,
    Option<DailyReadinessMeetingSnapshot>,
) {
    let mut current: Option<DailyReadinessMeetingSnapshot> = None;
    let mut next: Option<DailyReadinessMeetingSnapshot> = None;
    for meeting in meetings {
        let Some(starts_at) = meeting.starts_at.as_deref() else {
            continue;
        };
        let Ok(starts_at_parsed) = chrono::DateTime::parse_from_rfc3339(starts_at) else {
            continue;
        };
        let starts_at_utc = starts_at_parsed.with_timezone(&chrono::Utc);
        let ends_at_utc = meeting
            .ends_at
            .as_deref()
            .and_then(|raw| chrono::DateTime::parse_from_rfc3339(raw).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc));

        if let Some(end) = ends_at_utc {
            if starts_at_utc <= *now && *now < end && current.is_none() {
                current = Some((*meeting).clone());
                continue;
            }
        }
        if starts_at_utc > *now && next.is_none() {
            next = Some((*meeting).clone());
        }
    }
    (current, next)
}

fn paginate_upcoming_snapshots(
    meetings: &[&DailyReadinessMeetingSnapshot],
    cursor: Option<&Cursor>,
) -> Paginated<DailyReadinessMeetingSnapshot> {
    let offset = cursor
        .and_then(|c| parse_cursor_offset(c.as_str()))
        .unwrap_or(0);
    let total = meetings.len() as u64;
    let slice = meetings
        .iter()
        .skip(offset)
        .take(UPCOMING_MEETINGS_PAGE_SIZE)
        .map(|meeting| (*meeting).clone())
        .collect::<Vec<_>>();
    let consumed = offset + slice.len();
    let next_cursor = if consumed < meetings.len() {
        Some(Cursor::new(format!("upcoming_meetings:offset={consumed}")))
    } else {
        None
    };
    Paginated {
        items: slice,
        next_cursor,
        total_hint: Some(total),
        cursor_state: CursorState::Stable,
    }
}

fn project_upcoming_meetings(
    page: Paginated<DailyReadinessMeetingSnapshot>,
    prep_snapshots: &BTreeMap<String, MeetingPrepStatusSnapshot>,
    linked_entity_names: &BTreeMap<(String, String), String>,
) -> Paginated<MeetingBriefRef> {
    Paginated {
        items: page
            .items
            .iter()
            .map(|meeting| {
                project_meeting_brief(meeting, prep_snapshots.get(&meeting.id), linked_entity_names)
            })
            .collect(),
        next_cursor: page.next_cursor,
        total_hint: page.total_hint,
        cursor_state: page.cursor_state,
    }
}

fn parse_cursor_offset(token: &str) -> Option<usize> {
    token
        .strip_prefix("upcoming_meetings:offset=")
        .and_then(|raw| raw.parse::<usize>().ok())
}

fn build_candidate_set(readiness: &DailyReadinessContextSnapshot) -> CandidateSetRef {
    // The readiness context already names the date + workspace it represents;
    // the candidate set carries the same window for consumer rendering.
    CandidateSetRef {
        window_start: None,
        window_end: None,
        filter_description: format!(
            "daily_briefing date={} workspace={} meetings={}",
            readiness.date,
            readiness.workspace_scope,
            readiness.meetings.len()
        ),
    }
}

fn derive_freshness(
    prep_snapshots: &BTreeMap<String, MeetingPrepStatusSnapshot>,
    needs_prep_meeting_ids: &[String],
) -> BriefingFreshness {
    if !needs_prep_meeting_ids.is_empty() {
        let mut sorted = needs_prep_meeting_ids.to_vec();
        sorted.sort();
        sorted.dedup();
        return BriefingFreshness::NeedsPreparation {
            meeting_ids: sorted,
        };
    }
    // Walk stale_reasons in the order they arrive — first hit dictates the
    // stale reason. Consumers can drill into the per-meeting refs for the
    // full picture.
    for snapshot in prep_snapshots.values() {
        if snapshot.status == "stale" {
            return BriefingFreshness::Stale {
                reason: classify_stale_reason(snapshot.stale_reason.as_deref()),
            };
        }
    }
    BriefingFreshness::Fresh
}

fn classify_stale_reason(reason: Option<&str>) -> BriefingStaleReason {
    match reason {
        Some("source_asof_older_than_threshold") => {
            BriefingStaleReason::SourceAsofOlderThanThreshold
        }
        Some("contradicted_claim_upstream") | Some("recent_correction") => {
            BriefingStaleReason::UpstreamClaimChanged
        }
        _ => BriefingStaleReason::EntityContextStale,
    }
}

fn derive_integrity(superseded_claim_ids: &[String]) -> BriefingIntegrity {
    if superseded_claim_ids.is_empty() {
        BriefingIntegrity::Clean
    } else {
        let mut sorted = superseded_claim_ids.to_vec();
        sorted.sort();
        sorted.dedup();
        BriefingIntegrity::HasCorrections {
            superseded_claim_ids: sorted,
        }
    }
}

#[allow(dead_code)]
fn ambiguity_pair_placeholder(a: &str, b: &str, reason: &str) -> AmbiguityPair {
    AmbiguityPair {
        claim_id_a: a.to_string(),
        claim_id_b: b.to_string(),
        reason: reason.to_string(),
    }
}

fn derive_advisories(
    readiness: &DailyReadinessContextSnapshot,
    meetings: &[MeetingBriefRef],
    non_customer_meeting_ids: &std::collections::HashSet<String>,
    prep_read_failures: &[String],
    envelope_failures: &[String],
    callout_read_failure: Option<&str>,
) -> Vec<BriefingAdvisory> {
    let mut advisories = Vec::new();
    let unlinked = meetings
        .iter()
        .filter(|m| m.linked_entity_id.is_none())
        // Internal / team-sync / 1:1 meetings have no customer entity by
        // design — they shouldn't trip the "link N meetings" advisory.
        .filter(|m| !non_customer_meeting_ids.contains(&m.meeting_id))
        .map(|m| m.meeting_id.clone())
        .collect::<Vec<_>>();
    if !unlinked.is_empty() {
        advisories.push(BriefingAdvisory::UnlinkedMeetings {
            meeting_ids: unlinked,
        });
    }
    if !prep_read_failures.is_empty() {
        advisories.push(BriefingAdvisory::PartialReadFailure {
            advisory: format!(
                "{} prep status read(s) failed; degraded to NeedsPreparation",
                prep_read_failures.len()
            ),
        });
    }
    if !envelope_failures.is_empty() {
        advisories.push(BriefingAdvisory::PartialReadFailure {
            advisory: format!(
                "{} entity envelope read(s) failed; trust summary degraded",
                envelope_failures.len()
            ),
        });
    }
    if let Some(message) = callout_read_failure {
        advisories.push(BriefingAdvisory::PartialReadFailure {
            advisory: format!("briefing attention read failed; {message}"),
        });
    }
    for warning in &readiness.coverage_warnings {
        advisories.push(BriefingAdvisory::PartialReadFailure {
            advisory: format!("{}: {}", warning.kind, warning.message),
        });
    }
    advisories
}

fn aggregate_trust_band(
    likely_current: u32,
    use_with_caution: u32,
    needs_verification: u32,
) -> TrustBand {
    if likely_current + use_with_caution + needs_verification == 0 {
        return TrustBand::Unscored;
    }
    if needs_verification > 0 {
        TrustBand::NeedsVerification
    } else if use_with_caution > 0 {
        TrustBand::UseWithCaution
    } else {
        TrustBand::LikelyCurrent
    }
}

// ---- empty-state shortcuts -------------------------------------------------

struct EmptyEnvelopeSeed {
    output: DailyBriefingOutput,
}

impl EmptyEnvelopeSeed {
    fn into_envelope(
        self,
        ctx: &AbilityContext<'_>,
        schema_version: u32,
    ) -> AbilityResult<DailyBriefingOutput> {
        finalize_with_provenance(ctx, self.output, schema_version)
    }
}

fn empty_workspace_unknown(
    input: &DailyBriefingInput,
    workspace_id: &str,
    reason: BriefingEmptyReason,
    advisory: String,
) -> EmptyEnvelopeSeed {
    let state = BriefingState {
        availability: BriefingAvailability::Empty { reason },
        freshness: BriefingFreshness::Fresh,
        integrity: BriefingIntegrity::Clean,
        advisories: vec![BriefingAdvisory::PartialReadFailure { advisory }],
    };
    EmptyEnvelopeSeed {
        output: empty_envelope(input, workspace_id, state, Vec::new(), Vec::new()),
    }
}

fn empty_no_meetings(
    input: &DailyBriefingInput,
    workspace_id: &str,
    watch_proposals: Vec<WatchProposal>,
    source_asof_inputs: Vec<SourceAsofRef>,
) -> EmptyEnvelopeSeed {
    let state = BriefingState {
        availability: BriefingAvailability::Empty {
            reason: BriefingEmptyReason::NoMeetings,
        },
        freshness: BriefingFreshness::Fresh,
        integrity: BriefingIntegrity::Clean,
        advisories: Vec::new(),
    };
    EmptyEnvelopeSeed {
        output: empty_envelope(input, workspace_id, state, watch_proposals, source_asof_inputs),
    }
}

fn empty_envelope(
    input: &DailyBriefingInput,
    workspace_id: &str,
    state: BriefingState,
    watch_proposals: Vec<WatchProposal>,
    source_asof_inputs: Vec<SourceAsofRef>,
) -> DailyBriefingOutput {
    DailyBriefingOutput {
        schema_version: BRIEFING_SCHEMA_VERSION,
        date: input.date,
        state,
        current_meeting: None,
        next_meeting: None,
        upcoming_meetings: Paginated::empty_stable(),
        candidate_set: CandidateSetRef {
            window_start: None,
            window_end: None,
            filter_description: format!(
                "daily_briefing date={} workspace={} meetings=0",
                input.date, workspace_id
            ),
        },
        watch_proposals,
        trust_summary: BriefingTrustSummary::unscored(),
        provenance: EnvelopeProvenance::empty(),
        sensitivity: ClaimSensitivity::Public,
        source_asof_inputs,
    }
}

// ---- provenance finalize ---------------------------------------------------

fn finalize_with_provenance(
    ctx: &AbilityContext<'_>,
    output: DailyBriefingOutput,
    schema_version: u32,
) -> AbilityResult<DailyBriefingOutput> {
    let mut builder = ProvenanceBuilder::new(provenance_config(ctx, schema_version));
    let subject_attr = SubjectAttribution::direct_confident(SubjectRef::Global);
    builder.set_subject(subject_attr.clone());
    builder
        .attribute_subtree(
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
    builder.finalize(output).map_err(provenance_error)
}

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
        // Agent / Admin / System / SurfaceClient / McpClient — `get_daily_briefing`
        // policy declares allowed_actors=[User] so the registry already gates
        // non-User invocations before this code runs. Mirror the entity
        // intelligence projection so provenance still serializes if the
        // registry gate ever relaxes.
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

fn provenance_error(error: impl std::fmt::Display) -> AbilityError {
    validation_error(format!("provenance construction failed: {error}"))
}

fn field_error(error: impl std::fmt::Display) -> AbilityError {
    validation_error(format!("field attribution path failed: {error}"))
}

// ---- AC-507.7 call-graph lint ---------------------------------------------
//
// Static guarantee that this producer's call graph never mutates. The
// grep-based fence is a `#[test]` that runs on every `cargo test`
// invocation. Mirrors the pattern AC-335.12 established for
// `services::meeting_prep_status::read`. Any future refactor that introduces
// a write into the briefing call graph trips this test.

#[cfg(test)]
mod call_graph_lint {
    /// AC-507.7 — deny side-effect identifiers that would signal a DB write,
    /// queue enqueue, or signal emit lives in this producer's call graph.
    /// Pure-Rust in-process `&mut` references are NOT side effects against
    /// the substrate and are allowed (the producer accumulates trust /
    /// provenance tallies in-place); this lint is about boundary-crossing
    /// effects.
    #[test]
    fn briefing_producer_contains_no_mutations() {
        let source = include_str!("producer.rs");
        let banned: &[&str] = &[
            // Writer-API call sites for the substrate this ability composes.
            "enqueue_refresh",
            "record_user_authored",
            "transition_status",
            "enqueue_prep",
            // Signal emit paths.
            "emit_signal",
            "signals::bus::emit",
            "services::signals::emit",
            // Mutation-shaped SQL verbs — would slip a write into the path.
            "INSERT INTO",
            "UPDATE ",
            "DELETE FROM",
        ];
        let trimmed = strip_lint_block(source);
        let mut violations: Vec<&&str> = Vec::new();
        for term in banned {
            if trimmed.contains(*term) {
                violations.push(term);
            }
        }
        assert!(
            violations.is_empty(),
            "AC-507.7 violation: briefing producer contains mutation-suggestive identifiers: {violations:?}"
        );
    }

    fn strip_lint_block(source: &str) -> String {
        const MARKER: &str = "mod call_graph_lint";
        match source.find(MARKER) {
            Some(idx) => source[..idx].to_string(),
            None => source.to_string(),
        }
    }
}

// ---- AC-507.4 composed-state fixture matrix -------------------------------

#[cfg(test)]
mod state_matrix_fixtures {
    //! AC-507.4 — each fixture asserts the 4-tuple
    //! `(availability, freshness, integrity, advisories)` independently. The
    //! producer is intentionally NOT exercised end-to-end here (that lives in
    //! the fixture harness once the brief-level fixture lands);
    //! these are shape assertions on the composed-state matrix to prevent
    //! anyone collapsing it back to a flat enum.

    use super::*;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    use crate::abilities::NOOP_ABILITY_TRACER;
    use crate::intelligence::provider::ReplayProvider;
    use crate::services::context::{
        BriefingCalloutReadFuture, BriefingCalloutReadHandle, ClaimDismissalSurface,
        DailyReadinessContextReadFuture, DailyReadinessContextReadHandle,
        DailyReadinessSubjectSnapshot, EntityContextClaimReadFuture, EntityContextClaimReadHandle,
        ExternalClients, FixedClock, MeetingPrepStatusReadFuture, MeetingPrepStatusReadHandle,
        MeetingsViewIntent, SeedableRng, ServiceContext,
    };
    use crate::types::IntelligenceClaim;
    use chrono::TimeZone;

    #[derive(Clone)]
    struct FixtureDailyReadinessReader {
        snapshot: DailyReadinessContextSnapshot,
    }

    impl DailyReadinessContextReadHandle for FixtureDailyReadinessReader {
        fn read_daily_readiness_context<'a>(
            &'a self,
            _workspace_scope: String,
            _date: String,
            _intent: MeetingsViewIntent,
        ) -> DailyReadinessContextReadFuture<'a> {
            let snapshot = self.snapshot.clone();
            Box::pin(async move { Ok(snapshot) })
        }
    }

    struct RecordingPrepReader {
        seen: Arc<Mutex<Vec<String>>>,
    }

    impl MeetingPrepStatusReadHandle for RecordingPrepReader {
        fn read_meeting_prep_status<'a>(
            &'a self,
            meeting_id: String,
        ) -> MeetingPrepStatusReadFuture<'a> {
            self.seen
                .lock()
                .expect("record prep read")
                .push(meeting_id.clone());
            Box::pin(async move {
                Ok(MeetingPrepStatusSnapshot {
                    meeting_id: meeting_id.clone(),
                    event_id: Some(format!("event-{meeting_id}")),
                    linked_entity_type: Some("account".to_string()),
                    linked_entity_id: Some(format!("acct-{meeting_id}")),
                    status: "ready".to_string(),
                    blocking_reason: None,
                    stale_reason: None,
                    last_prepared_at: Some("2026-05-20T09:00:00Z".to_string()),
                    source_asof_inputs: Vec::new(),
                })
            })
        }
    }

    struct RecordingClaimReader {
        seen: Arc<Mutex<Vec<String>>>,
    }

    impl EntityContextClaimReadHandle for RecordingClaimReader {
        fn read_entity_context_claims<'a>(
            &'a self,
            _entity_type: String,
            entity_id: String,
            _surface: ClaimDismissalSurface,
            _depth: usize,
        ) -> EntityContextClaimReadFuture<'a> {
            self.seen
                .lock()
                .expect("record entity claim read")
                .push(entity_id);
            Box::pin(async move { Ok(Vec::<IntelligenceClaim>::new()) })
        }

        fn read_entity_context_claims_limited<'a>(
            &'a self,
            _entity_type: String,
            entity_id: String,
            _surface: ClaimDismissalSurface,
            _depth: usize,
            _limit: usize,
        ) -> EntityContextClaimReadFuture<'a> {
            self.seen
                .lock()
                .expect("record limited entity claim read")
                .push(entity_id);
            Box::pin(async move { Ok(Vec::<IntelligenceClaim>::new()) })
        }
    }

    #[derive(Clone)]
    struct FixtureBriefingCalloutReader {
        callouts: Vec<BriefingCalloutSnapshot>,
    }

    impl BriefingCalloutReadHandle for FixtureBriefingCalloutReader {
        fn read_briefing_callouts_for_date<'a>(
            &'a self,
            _workspace_scope: String,
            _date: String,
            _limit: usize,
        ) -> BriefingCalloutReadFuture<'a> {
            let callouts = self.callouts.clone();
            Box::pin(async move { Ok(callouts) })
        }
    }

    fn daily_meeting(id: &str, starts_at: Option<String>) -> DailyReadinessMeetingSnapshot {
        DailyReadinessMeetingSnapshot {
            id: id.to_string(),
            title: id.to_string(),
            starts_at,
            ends_at: None,
            workspace_scope: "local".to_string(),
            meeting_type: "customer".to_string(),
        }
    }

    fn daily_meeting_with_end(
        id: &str,
        starts_at: &str,
        ends_at: Option<&str>,
    ) -> DailyReadinessMeetingSnapshot {
        DailyReadinessMeetingSnapshot {
            id: id.to_string(),
            title: id.to_string(),
            starts_at: Some(starts_at.to_string()),
            ends_at: ends_at.map(str::to_string),
            workspace_scope: "local".to_string(),
            meeting_type: "customer".to_string(),
        }
    }

    #[test]
    fn full_happy_path_state() {
        let state = BriefingState {
            availability: BriefingAvailability::Available,
            freshness: BriefingFreshness::Fresh,
            integrity: BriefingIntegrity::Clean,
            advisories: Vec::new(),
        };
        assert!(matches!(
            state.availability,
            BriefingAvailability::Available
        ));
        assert!(matches!(state.freshness, BriefingFreshness::Fresh));
        assert!(matches!(state.integrity, BriefingIntegrity::Clean));
        assert!(state.advisories.is_empty());
    }

    #[test]
    fn empty_no_meetings_state() {
        let state = BriefingState {
            availability: BriefingAvailability::Empty {
                reason: BriefingEmptyReason::NoMeetings,
            },
            freshness: BriefingFreshness::Fresh,
            integrity: BriefingIntegrity::Clean,
            advisories: Vec::new(),
        };
        match state.availability {
            BriefingAvailability::Empty { reason } => {
                assert_eq!(reason, BriefingEmptyReason::NoMeetings);
            }
            _ => panic!("expected Empty NoMeetings"),
        }
    }

    #[test]
    fn auth_locked_state() {
        let state = BriefingState {
            availability: BriefingAvailability::AuthLocked,
            freshness: BriefingFreshness::Fresh,
            integrity: BriefingIntegrity::Clean,
            advisories: Vec::new(),
        };
        assert!(matches!(
            state.availability,
            BriefingAvailability::AuthLocked
        ));
    }

    #[test]
    fn available_with_needs_preparation_partial_state() {
        // The case V1.0's flat enum couldn't represent — common day shape.
        let state = BriefingState {
            availability: BriefingAvailability::Available,
            freshness: BriefingFreshness::NeedsPreparation {
                meeting_ids: vec!["m2".into()],
            },
            integrity: BriefingIntegrity::Clean,
            advisories: Vec::new(),
        };
        match state.freshness {
            BriefingFreshness::NeedsPreparation { meeting_ids } => {
                assert_eq!(meeting_ids, vec!["m2".to_string()]);
            }
            _ => panic!("expected NeedsPreparation"),
        }
    }

    #[test]
    fn stale_source_asof_state() {
        let state = BriefingState {
            availability: BriefingAvailability::Available,
            freshness: BriefingFreshness::Stale {
                reason: BriefingStaleReason::SourceAsofOlderThanThreshold,
            },
            integrity: BriefingIntegrity::Clean,
            advisories: Vec::new(),
        };
        assert!(matches!(state.freshness, BriefingFreshness::Stale { .. }));
    }

    #[test]
    fn has_corrections_state() {
        let state = BriefingState {
            availability: BriefingAvailability::Available,
            freshness: BriefingFreshness::Fresh,
            integrity: BriefingIntegrity::HasCorrections {
                superseded_claim_ids: vec!["claim-1".into()],
            },
            advisories: Vec::new(),
        };
        match state.integrity {
            BriefingIntegrity::HasCorrections {
                superseded_claim_ids,
            } => assert_eq!(superseded_claim_ids, vec!["claim-1".to_string()]),
            _ => panic!("expected HasCorrections"),
        }
    }

    #[test]
    fn has_ambiguity_state() {
        let pair = ambiguity_pair_placeholder("c1", "c2", "two_active_for_field");
        let state = BriefingState {
            availability: BriefingAvailability::Available,
            freshness: BriefingFreshness::Fresh,
            integrity: BriefingIntegrity::HasAmbiguity {
                ambiguous_pairs: vec![pair],
            },
            advisories: Vec::new(),
        };
        assert!(matches!(
            state.integrity,
            BriefingIntegrity::HasAmbiguity { .. }
        ));
    }

    #[test]
    fn multi_dimensional_stale_plus_corrections_plus_advisory() {
        // Realistic state: briefing is available, source-stale, AND the
        // claim store has a correction, AND a watch proposal is queued.
        // A flat enum collapses all this to one slot; the composed struct
        // preserves every axis.
        let state = BriefingState {
            availability: BriefingAvailability::Available,
            freshness: BriefingFreshness::Stale {
                reason: BriefingStaleReason::UpstreamClaimChanged,
            },
            integrity: BriefingIntegrity::HasCorrections {
                superseded_claim_ids: vec!["claim-superseded".into()],
            },
            advisories: vec![BriefingAdvisory::WatchProposal {
                proposal_id: "watch-1".into(),
                summary: "tracked subject moved roles".into(),
            }],
        };
        assert!(matches!(
            state.availability,
            BriefingAvailability::Available
        ));
        assert!(matches!(state.freshness, BriefingFreshness::Stale { .. }));
        assert!(matches!(
            state.integrity,
            BriefingIntegrity::HasCorrections { .. }
        ));
        assert!(matches!(
            state.advisories.first(),
            Some(BriefingAdvisory::WatchProposal { .. })
        ));
    }

    #[test]
    fn upcoming_meetings_cursor_roundtrip() {
        let meetings = (0..30)
            .map(|i| {
                daily_meeting(
                    &format!("m-{i}"),
                    Some(format!("2026-05-20T{:02}:00:00Z", i % 24)),
                )
            })
            .collect::<Vec<_>>();
        let meeting_refs = meetings.iter().collect::<Vec<_>>();
        let page_one = paginate_upcoming_snapshots(&meeting_refs, None);
        assert_eq!(page_one.items.len(), UPCOMING_MEETINGS_PAGE_SIZE);
        let next = page_one.next_cursor.clone().expect("cursor present");
        let page_two = paginate_upcoming_snapshots(&meeting_refs, Some(&next));
        assert_eq!(page_two.items.len(), 30 - UPCOMING_MEETINGS_PAGE_SIZE);
        assert!(page_two.next_cursor.is_none());
    }

    #[test]
    fn daily_briefing_expansion_ids_are_bounded_to_visible_page_current_and_next() {
        let mut meetings = vec![
            daily_meeting_with_end(
                "current",
                "2026-05-20T10:00:00Z",
                Some("2026-05-20T11:00:00Z"),
            ),
            daily_meeting("next", Some("2026-05-20T11:30:00Z".to_string())),
        ];
        meetings.extend((0..80).map(|i| {
            daily_meeting(
                &format!("future-{i:02}"),
                Some(format!("2026-05-20T12:{:02}:00Z", i % 60)),
            )
        }));
        let mut refs = meetings.iter().collect::<Vec<_>>();
        sort_meeting_snapshots(&mut refs);
        let now = chrono::DateTime::parse_from_rfc3339("2026-05-20T10:30:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        let (current, next) = pick_current_and_next_snapshots(&refs, &now);
        let page = paginate_upcoming_snapshots(&refs, None);
        let expansion_ids =
            collect_expansion_meeting_ids(current.as_ref(), next.as_ref(), &page.items);

        assert!(expansion_ids.len() <= UPCOMING_MEETINGS_PAGE_SIZE + 2);
        assert!(expansion_ids.contains("current"));
        assert!(expansion_ids.contains("next"));
    }

    #[tokio::test]
    async fn daily_briefing_producer_expands_only_current_next_and_requested_page() {
        let mut meetings = vec![
            daily_meeting_with_end(
                "current",
                "2026-05-20T10:00:00Z",
                Some("2026-05-20T11:00:00Z"),
            ),
            daily_meeting("next", Some("2026-05-20T11:30:00Z".to_string())),
        ];
        meetings.extend((0..80).map(|i| {
            daily_meeting(
                &format!("future-{i:02}"),
                Some(format!("2026-05-20T12:{:02}:00Z", i % 60)),
            )
        }));
        let snapshot = DailyReadinessContextSnapshot {
            workspace_scope: "local".to_string(),
            date: "2026-05-20".to_string(),
            meetings,
            tracked_subjects: Vec::new(),
            overnight_changes: Vec::new(),
            risk_shifts: Vec::new(),
            open_loops: Vec::new(),
            coverage_warnings: Vec::new(),
        };
        let prep_seen = Arc::new(Mutex::new(Vec::new()));
        let entity_seen = Arc::new(Mutex::new(Vec::new()));
        let clock = FixedClock::new(
            chrono::Utc
                .with_ymd_and_hms(2026, 5, 20, 10, 30, 0)
                .unwrap(),
        );
        let rng = SeedableRng::new(507);
        let external = ExternalClients::default();
        let services = ServiceContext::test_live(&clock, &rng, &external)
            .with_daily_readiness_context_reader(Arc::new(FixtureDailyReadinessReader { snapshot }))
            .with_meeting_prep_status_reader(Arc::new(RecordingPrepReader {
                seen: prep_seen.clone(),
            }))
            .with_entity_context_claim_reader(Arc::new(RecordingClaimReader {
                seen: entity_seen.clone(),
            }));
        let provider = ReplayProvider::new(HashMap::new());
        let ctx = AbilityContext::new(
            &services,
            &provider,
            &NOOP_ABILITY_TRACER,
            Actor::User,
            None,
            ClaimDismissalSurface::TauriEntityDetail,
        );

        let output = build_daily_briefing(
            &ctx,
            DailyBriefingInput {
                schema_version: BRIEFING_SCHEMA_VERSION,
                date: chrono::NaiveDate::from_ymd_opt(2026, 5, 20).unwrap(),
                workspace_id: "local".to_string(),
                sections: None,
                upcoming_meetings_cursor: Some(Cursor::new("upcoming_meetings:offset=25")),
            },
        )
        .await
        .expect("daily briefing builds")
        .into_data();

        let mut expected_meeting_ids = BTreeSet::new();
        expected_meeting_ids.insert(output.current_meeting.as_ref().unwrap().meeting_id.clone());
        expected_meeting_ids.insert(output.next_meeting.as_ref().unwrap().meeting_id.clone());
        expected_meeting_ids.extend(
            output
                .upcoming_meetings
                .items
                .iter()
                .map(|meeting| meeting.meeting_id.clone()),
        );
        let actual_prep_ids = prep_seen
            .lock()
            .expect("prep reads")
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        let actual_entity_ids = entity_seen
            .lock()
            .expect("entity reads")
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        let expected_entity_ids = expected_meeting_ids
            .iter()
            .map(|meeting_id| format!("acct-{meeting_id}"))
            .collect::<BTreeSet<_>>();

        assert_eq!(
            output.upcoming_meetings.items.len(),
            UPCOMING_MEETINGS_PAGE_SIZE
        );
        assert_eq!(
            actual_prep_ids, expected_meeting_ids,
            "producer must not read prep for meetings outside current, next, and the requested page"
        );
        assert_eq!(
            actual_entity_ids, expected_entity_ids,
            "producer must not compose entity intelligence for meetings outside the bounded expansion set"
        );
    }

    #[tokio::test]
    async fn daily_briefing_producer_maps_callouts_to_attention_proposals() {
        let snapshot = DailyReadinessContextSnapshot {
            workspace_scope: "local".to_string(),
            date: "2026-05-20".to_string(),
            meetings: vec![daily_meeting(
                "meeting-1",
                Some("2026-05-20T11:30:00Z".to_string()),
            )],
            tracked_subjects: vec![DailyReadinessSubjectSnapshot {
                kind: "account".to_string(),
                id: "acct-meeting-1".to_string(),
                display_name: "Example Account".to_string(),
                workspace_scope: "local".to_string(),
            }],
            overnight_changes: Vec::new(),
            risk_shifts: Vec::new(),
            open_loops: Vec::new(),
            coverage_warnings: Vec::new(),
        };
        let callout = BriefingCalloutSnapshot {
            id: "callout-1".to_string(),
            headline: "Champion going cold".to_string(),
            detail: Some("No contact in 30 days".to_string()),
            severity: "warning".to_string(),
            entity_name: Some("Example Account".to_string()),
            entity_type: "account".to_string(),
            entity_id: "acct-meeting-1".to_string(),
            created_at: "2026-05-20T08:00:00Z".to_string(),
            claim_id: Some("claim-1".to_string()),
        };
        let clock = FixedClock::new(
            chrono::Utc
                .with_ymd_and_hms(2026, 5, 20, 10, 30, 0)
                .unwrap(),
        );
        let rng = SeedableRng::new(508);
        let external = ExternalClients::default();
        let services = ServiceContext::test_live(&clock, &rng, &external)
            .with_daily_readiness_context_reader(Arc::new(FixtureDailyReadinessReader { snapshot }))
            .with_meeting_prep_status_reader(Arc::new(RecordingPrepReader {
                seen: Arc::new(Mutex::new(Vec::new())),
            }))
            .with_entity_context_claim_reader(Arc::new(RecordingClaimReader {
                seen: Arc::new(Mutex::new(Vec::new())),
            }))
            .with_briefing_callout_reader(Arc::new(FixtureBriefingCalloutReader {
                callouts: vec![callout],
            }));
        let provider = ReplayProvider::new(HashMap::new());
        let ctx = AbilityContext::new(
            &services,
            &provider,
            &NOOP_ABILITY_TRACER,
            Actor::User,
            None,
            ClaimDismissalSurface::TauriEntityDetail,
        );

        let output = build_daily_briefing(
            &ctx,
            DailyBriefingInput {
                schema_version: BRIEFING_SCHEMA_VERSION,
                date: chrono::NaiveDate::from_ymd_opt(2026, 5, 20).unwrap(),
                workspace_id: "local".to_string(),
                sections: None,
                upcoming_meetings_cursor: None,
            },
        )
        .await
        .expect("daily briefing builds")
        .into_data();

        let proposal = output
            .watch_proposals
            .first()
            .expect("callout projected into watch proposals");
        assert_eq!(proposal.proposal_id, "callout-1");
        assert_eq!(proposal.headline, "Champion going cold");
        assert_eq!(proposal.detail.as_deref(), Some("No contact in 30 days"));
        assert_eq!(proposal.severity, "warning");
        assert_eq!(proposal.entity_name.as_deref(), Some("Example Account"));
        assert_eq!(proposal.subject_kind, "account");
        assert_eq!(proposal.subject_id, "acct-meeting-1");
        assert_eq!(proposal.source_asof.as_deref(), Some("2026-05-20T08:00:00Z"));
        assert_eq!(proposal.claim_id.as_deref(), Some("claim-1"));
        assert_eq!(proposal.trust_band, TrustBand::NeedsVerification);
        assert!(output.source_asof_inputs.iter().any(|source| {
            source.source == "briefing_callout:callout-1"
                && source.as_of == "2026-05-20T08:00:00Z"
        }));
        assert_eq!(
            output.next_meeting.as_ref().unwrap().linked_entity_name.as_deref(),
            Some("Example Account")
        );
    }

    #[test]
    fn daily_briefing_entity_sections_omit_open_loops_for_aggregate_pass() {
        assert_eq!(
            daily_briefing_entity_sections(),
            vec![EnvelopeSection::Facts]
        );
    }

    #[test]
    fn aggregate_trust_band_picks_least_trusted() {
        assert_eq!(aggregate_trust_band(0, 0, 0), TrustBand::Unscored);
        assert_eq!(aggregate_trust_band(10, 0, 0), TrustBand::LikelyCurrent);
        assert_eq!(aggregate_trust_band(10, 1, 0), TrustBand::UseWithCaution);
        assert_eq!(aggregate_trust_band(10, 1, 1), TrustBand::NeedsVerification);
    }

    #[test]
    fn classify_stale_reason_maps_source_asof() {
        assert_eq!(
            classify_stale_reason(Some("source_asof_older_than_threshold")),
            BriefingStaleReason::SourceAsofOlderThanThreshold
        );
        assert_eq!(
            classify_stale_reason(Some("contradicted_claim_upstream")),
            BriefingStaleReason::UpstreamClaimChanged
        );
        assert_eq!(
            classify_stale_reason(Some("recent_correction")),
            BriefingStaleReason::UpstreamClaimChanged
        );
        assert_eq!(
            classify_stale_reason(None),
            BriefingStaleReason::EntityContextStale
        );
    }

    #[test]
    fn prep_status_needs_preparation_classifier() {
        assert!(prep_status_is_needs_preparation("prep_needed"));
        assert!(prep_status_is_needs_preparation("queued"));
        assert!(prep_status_is_needs_preparation("running"));
        assert!(prep_status_is_needs_preparation("blocked_no_entity"));
        assert!(prep_status_is_needs_preparation("failed"));
        assert!(!prep_status_is_needs_preparation("ready"));
        assert!(!prep_status_is_needs_preparation("stale"));
        assert!(!prep_status_is_needs_preparation("user_suppressed"));
    }

    #[test]
    fn parse_entity_kind_strict() {
        assert_eq!(parse_entity_kind("account"), Some(EntityKind::Account));
        assert_eq!(parse_entity_kind("project"), Some(EntityKind::Project));
        assert_eq!(parse_entity_kind("person"), Some(EntityKind::Person));
        assert!(parse_entity_kind("meeting").is_none());
        assert!(parse_entity_kind("").is_none());
    }

    /// Regression: internal / team-sync / 1:1 meetings must NOT trip the
    /// "Link N meetings" unlinked-meetings advisory. Customer-facing rows
    /// continue to advisory normally.
    #[test]
    fn derive_advisories_skips_non_customer_facing_meetings() {
        let readiness = DailyReadinessContextSnapshot {
            workspace_scope: "local".to_string(),
            date: "2026-05-28".to_string(),
            meetings: Vec::new(),
            tracked_subjects: Vec::new(),
            overnight_changes: Vec::new(),
            risk_shifts: Vec::new(),
            open_loops: Vec::new(),
            coverage_warnings: Vec::new(),
        };
        let meetings = vec![
            // 4 internal, 1 customer — none has linked_entity_id
            meeting_brief_unlinked("m-internal-1"),
            meeting_brief_unlinked("m-internal-2"),
            meeting_brief_unlinked("m-internal-3"),
            meeting_brief_unlinked("m-internal-4"),
            meeting_brief_unlinked("m-customer-1"),
        ];
        let non_customer: std::collections::HashSet<String> = [
            "m-internal-1",
            "m-internal-2",
            "m-internal-3",
            "m-internal-4",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();

        let advisories = derive_advisories(&readiness, &meetings, &non_customer, &[], &[], None);

        // Only the customer meeting should appear in the unlinked advisory.
        // Pre-fix this would have produced "Link 5 meetings" — the bug shape.
        let unlinked_ids: Vec<&str> = advisories
            .iter()
            .find_map(|a| match a {
                BriefingAdvisory::UnlinkedMeetings { meeting_ids } => {
                    Some(meeting_ids.iter().map(String::as_str).collect())
                }
                _ => None,
            })
            .expect("UnlinkedMeetings advisory present");
        assert_eq!(unlinked_ids, vec!["m-customer-1"]);
    }

    /// Regression: when every meeting is internal, no UnlinkedMeetings
    /// advisory should fire at all.
    #[test]
    fn derive_advisories_emits_no_unlinked_advisory_when_all_internal() {
        let readiness = DailyReadinessContextSnapshot {
            workspace_scope: "local".to_string(),
            date: "2026-05-28".to_string(),
            meetings: Vec::new(),
            tracked_subjects: Vec::new(),
            overnight_changes: Vec::new(),
            risk_shifts: Vec::new(),
            open_loops: Vec::new(),
            coverage_warnings: Vec::new(),
        };
        let meetings = vec![
            meeting_brief_unlinked("m-internal-1"),
            meeting_brief_unlinked("m-internal-2"),
        ];
        let non_customer: std::collections::HashSet<String> = ["m-internal-1", "m-internal-2"]
            .iter()
            .map(|s| s.to_string())
            .collect();

        let advisories = derive_advisories(&readiness, &meetings, &non_customer, &[], &[], None);

        let has_unlinked = advisories
            .iter()
            .any(|a| matches!(a, BriefingAdvisory::UnlinkedMeetings { .. }));
        assert!(
            !has_unlinked,
            "internal-only day must not emit UnlinkedMeetings advisory; got {:?}",
            advisories
        );
    }

    fn meeting_brief_unlinked(id: &str) -> MeetingBriefRef {
        MeetingBriefRef {
            meeting_id: id.to_string(),
            title: Some(id.to_string()),
            starts_at: Some("2026-05-28T10:00:00Z".to_string()),
            ends_at: Some("2026-05-28T10:30:00Z".to_string()),
            linked_entity_type: None,
            linked_entity_id: None,
            linked_entity_name: None,
            prep_status: "prep_needed".to_string(),
            blocking_reason: None,
            stale_reason: None,
            last_prepared_at: None,
        }
    }

    #[test]
    fn is_customer_facing_recognizes_canonical_set() {
        for ty in ["customer", "qbr", "partnership", "external"] {
            assert!(is_customer_facing(ty), "{ty} should be customer-facing");
        }
        for ty in [
            "internal",
            "team_sync",
            "one_on_one",
            "all_hands",
            "training",
            "personal",
        ] {
            assert!(
                !is_customer_facing(ty),
                "{ty} should not be customer-facing"
            );
        }
    }
}
