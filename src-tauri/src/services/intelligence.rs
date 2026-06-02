// Intelligence service — extracted from commands.rs
// Business logic for entity intelligence CRUD, enrichment, and risk briefings.

use std::collections::BTreeSet;
use std::path::Path;

use crate::db::ActionDb;
use crate::intel_queue::{
    compose_enrichment_intelligence, gather_enrichment_input,
    persist_enrichment_write_results_via_db_service, run_enrichment,
    run_enrichment_finalize_post_commit_via_db_service, FinalizeMode, IntelPriority, IntelRequest,
};
use crate::pty::AiUsageContext;
use crate::services::context::ServiceContext;
use crate::signals::propagation::PropagationEngine;
use crate::state::AppState;
use chrono::{DateTime, Utc};
use rusqlite::{params, OptionalExtension};
use sha2::{Digest, Sha256};
use tauri::Emitter;

const GENERATED_PROJECTION_SOURCE_REF_PREFIX: &str = "intelligence_projection_source:";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EntityIntelligenceProjectionBackfillReport {
    pub total_rows: usize,
    pub rows_examined: usize,
    pub next_offset: usize,
    pub finished: bool,
    pub rows_backfilled: usize,
    pub claims_inserted: usize,
    pub recompute_jobs_enqueued: usize,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GeneratedProjectionSourcePurgeReport {
    pub claims_withdrawn: usize,
    pub recompute_jobs_enqueued: usize,
}

/// Preserve user-confirmed value_delivered items during re-enrichment.
///
/// Items with `item_source.source == "user_correction"` are user-confirmed and must
/// survive re-enrichment. New AI items are merged in, deduplicating by fuzzy statement match.
fn merge_user_confirmed_values(
    new_intel: &mut crate::intelligence::IntelligenceJson,
    existing: &crate::intelligence::IntelligenceJson,
) {
    // Collect user-confirmed items from existing data
    let user_confirmed: Vec<_> = existing
        .value_delivered
        .iter()
        .filter(|v| {
            v.item_source
                .as_ref()
                .is_some_and(|s| s.source == "user_correction")
        })
        .cloned()
        .collect();

    if user_confirmed.is_empty() {
        return;
    }

    // Build set of existing user-confirmed statements (lowercased, trimmed) for dedup
    let confirmed_statements: std::collections::HashSet<String> = user_confirmed
        .iter()
        .map(|v| v.statement.trim().to_lowercase())
        .collect();

    // Remove AI items that duplicate user-confirmed items
    new_intel
        .value_delivered
        .retain(|v| !confirmed_statements.contains(&v.statement.trim().to_lowercase()));

    // Prepend user-confirmed items (they take priority)
    let mut merged = user_confirmed;
    merged.append(&mut new_intel.value_delivered);
    merged.truncate(10); // Cap at 10
    new_intel.value_delivered = merged;
}

fn blank_entity_intelligence_snapshot(
    entity_id: &str,
    entity_type: &str,
    enriched_at: &str,
) -> crate::intelligence::IntelligenceJson {
    crate::intelligence::IntelligenceJson {
        entity_id: entity_id.to_string(),
        entity_type: entity_type.to_string(),
        enriched_at: enriched_at.to_string(),
        ..Default::default()
    }
}

fn subject_ref_for_entity(entity_type: &str, entity_id: &str) -> Result<String, String> {
    match entity_type {
        "account" | "project" | "person" | "meeting" => Ok(serde_json::json!({
            "kind": entity_type,
            "id": entity_id,
        })
        .to_string()),
        other => Err(format!("Unsupported claim projection subject: {other}")),
    }
}

fn projection_metadata(
    value: serde_json::Value,
    projection_producer: &str,
    projection_origin_subject: Option<&str>,
) -> Option<String> {
    let mut metadata = serde_json::json!({
        "legacy_projection_value": value,
        "projection_producer": projection_producer,
    });
    if let (Some(object), Some(origin_subject)) =
        (metadata.as_object_mut(), projection_origin_subject)
    {
        if let Ok(origin_value) = serde_json::from_str::<serde_json::Value>(origin_subject) {
            object.insert("projection_origin_subject".to_string(), origin_value);
        }
    }
    serde_json::to_string(&metadata).ok()
}

fn source_asof_from_item_source(
    source: Option<&crate::intelligence::io::ItemSource>,
) -> Option<&str> {
    source
        .map(|item_source| item_source.sourced_at.trim())
        .filter(|sourced_at| !sourced_at.is_empty())
}

fn projection_claim_data_source(
    fallback: &str,
    source: Option<&crate::intelligence::io::ItemSource>,
) -> String {
    let fallback = fallback.trim();
    let source = source
        .map(|item_source| item_source.source.trim())
        .filter(|source| !source.is_empty());

    match fallback {
        "glean" => match source {
            Some(source) => authorized_glean_projection_data_source(source)
                .unwrap_or("glean")
                .to_string(),
            None => "glean".to_string(),
        },
        source if source.starts_with("glean_") => source.to_string(),
        "" => "ai_enrichment".to_string(),
        source => source.to_string(),
    }
}

fn authorized_non_glean_projection_data_source(source: &str) -> Option<&'static str> {
    match source.trim().to_ascii_lowercase().as_str() {
        "transcript" => Some("transcript"),
        "meeting" => Some("meeting"),
        "post_meeting" => Some("post_meeting"),
        "calendar" => Some("calendar"),
        "local_file" | "workspace_file" => Some("local_file"),
        "email" | "gmail" | "google" => Some("email"),
        "pty_synthesis" => Some("pty_synthesis"),
        "local_enrichment" => Some("local_enrichment"),
        "ai" | "ai_enrichment" | "ai_inference" => Some("ai_enrichment"),
        _ => None,
    }
}

fn authorized_glean_projection_data_source(source: &str) -> Option<&'static str> {
    match source.trim().to_ascii_lowercase().as_str() {
        "glean" => Some("glean"),
        "glean_crm" | "glean_salesforce" | "salesforce" => Some("glean_crm"),
        "glean_zendesk" | "glean_support" | "zendesk" => Some("glean_zendesk"),
        "glean_gong" | "gong" => Some("glean_gong"),
        "glean_slack" | "glean_chat" | "slack" => Some("glean_chat"),
        "glean_p2" | "p2" => Some("glean_p2"),
        "glean_wordpress" | "wordpress" => Some("glean_wordpress"),
        "glean_org" | "glean_org_directory" => Some("glean_org_directory"),
        "glean_documents" | "glean_document" => Some("glean_documents"),
        source if source.starts_with("glean") => Some("glean"),
        _ => None,
    }
}

fn projection_source_ref(input: &ProjectionClaimInput<'_>) -> Option<String> {
    let item_source = input.item_source?;
    let reference = item_source.reference.as_deref()?.trim();
    if reference.is_empty() {
        return None;
    }

    let mut hasher = Sha256::new();
    for part in [
        input.subject_ref,
        input.claim_type,
        input.field_path,
        item_source.source.trim(),
        item_source.sourced_at.trim(),
        reference,
    ] {
        hasher.update(part.as_bytes());
        hasher.update([0]);
    }
    let digest = hasher.finalize();
    Some(format!(
        "{GENERATED_PROJECTION_SOURCE_REF_PREFIX}{}",
        hex::encode(&digest[..16])
    ))
}

fn parse_projection_timestamp(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|timestamp| timestamp.with_timezone(&Utc))
}

fn projection_item_confidence(source: Option<&crate::intelligence::io::ItemSource>) -> f32 {
    let Some(confidence) = source.map(|item_source| item_source.confidence) else {
        return 0.5;
    };
    if confidence.is_finite() {
        confidence.clamp(0.0, 1.0) as f32
    } else {
        0.5
    }
}

fn projection_provenance_subject_ref(
    subject_kind: &str,
    subject_id: &str,
) -> Result<crate::abilities::provenance::SubjectRef, String> {
    match subject_kind {
        "account" => Ok(crate::abilities::provenance::SubjectRef::Account(
            subject_id.to_string(),
        )),
        "project" => Ok(crate::abilities::provenance::SubjectRef::Project(
            subject_id.to_string(),
        )),
        "person" => Ok(crate::abilities::provenance::SubjectRef::Person(
            subject_id.to_string(),
        )),
        "meeting" => Ok(crate::abilities::provenance::SubjectRef::Meeting(
            subject_id.to_string(),
        )),
        other => Err(format!(
            "unsupported projection provenance subject: {other}"
        )),
    }
}

fn glean_downstream_for_projection_source(
    source: &str,
) -> Option<crate::abilities::provenance::GleanDownstream> {
    match source.trim().to_ascii_lowercase().as_str() {
        "glean_crm" | "glean_salesforce" | "salesforce" => {
            Some(crate::abilities::provenance::GleanDownstream::Salesforce)
        }
        "glean_zendesk" | "glean_support" | "zendesk" => {
            Some(crate::abilities::provenance::GleanDownstream::Zendesk)
        }
        "glean_gong" | "gong" => Some(crate::abilities::provenance::GleanDownstream::Gong),
        "glean_slack" | "glean_chat" | "slack" => {
            Some(crate::abilities::provenance::GleanDownstream::Slack)
        }
        "glean_p2" | "p2" => Some(crate::abilities::provenance::GleanDownstream::P2),
        "glean_wordpress" | "wordpress" => {
            Some(crate::abilities::provenance::GleanDownstream::Wordpress)
        }
        "glean_org" | "glean_org_directory" => {
            Some(crate::abilities::provenance::GleanDownstream::OrgDirectory)
        }
        "glean_documents" | "glean_document" => {
            Some(crate::abilities::provenance::GleanDownstream::Documents)
        }
        source if source.starts_with("glean") => {
            Some(crate::abilities::provenance::GleanDownstream::Unknown)
        }
        _ => None,
    }
}

fn projection_provenance_data_source(source: &str) -> crate::abilities::provenance::DataSource {
    let normalized = source.trim();
    if let Some(downstream) = glean_downstream_for_projection_source(normalized) {
        return crate::abilities::provenance::DataSource::Glean { downstream };
    }
    match normalized.to_ascii_lowercase().as_str() {
        "user" | "user_correction" => crate::abilities::provenance::DataSource::User,
        "google" | "gmail" | "email" => crate::abilities::provenance::DataSource::Google,
        "clay" => crate::abilities::provenance::DataSource::Clay,
        "local_enrichment" | "local_file" | "workspace_file" | "transcript" | "meeting"
        | "post_meeting" | "calendar" => crate::abilities::provenance::DataSource::LocalEnrichment,
        "ai" | "ai_enrichment" | "ai_inference" | "pty_synthesis" => {
            crate::abilities::provenance::DataSource::Ai
        }
        "" => crate::abilities::provenance::DataSource::Ai,
        other => crate::abilities::provenance::DataSource::Other(
            crate::abilities::provenance::SourceName::new(other),
        ),
    }
}

fn projection_provenance_actor(actor: &str) -> crate::abilities::provenance::Actor {
    let actor = actor.trim();
    if actor.eq_ignore_ascii_case("user") || actor.starts_with("user:") {
        return crate::abilities::provenance::Actor::User;
    }
    if let Some(agent) = actor.strip_prefix("agent:") {
        return crate::abilities::provenance::Actor::Agent {
            name: agent.to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        };
    }
    crate::abilities::provenance::Actor::System {
        component: if actor.is_empty() {
            "intelligence_projection".to_string()
        } else {
            actor.to_string()
        },
    }
}

fn projection_source_identifier(
    data_source: &crate::abilities::provenance::DataSource,
    input: &ProjectionClaimInput<'_>,
    subject_id: &str,
    source_asof: Option<DateTime<Utc>>,
    observed_at: DateTime<Utc>,
) -> crate::abilities::provenance::SourceIdentifier {
    if let crate::abilities::provenance::DataSource::Glean { downstream } = data_source {
        return crate::abilities::provenance::SourceIdentifier::OpaqueGleanSource {
            downstream: downstream.clone(),
            opaque_ref: input
                .item_source
                .and_then(|source| source.reference.as_deref())
                .map(str::trim)
                .filter(|reference| !reference.is_empty())
                .unwrap_or(input.field_path)
                .to_string(),
            cited_as_of: source_asof.unwrap_or(observed_at),
        };
    }

    crate::abilities::provenance::SourceIdentifier::Entity {
        entity_id: crate::abilities::provenance::EntityId::new(subject_id.to_string()),
        field: Some(input.field_path.to_string()),
    }
}

fn projection_provenance_json(
    ctx: &ServiceContext<'_>,
    input: &ProjectionClaimInput<'_>,
    observed_at: &str,
    source_ref: Option<&str>,
    claim_data_source: &str,
) -> Result<String, String> {
    let (subject_kind, subject_id) = projection_subject_lookup_parts(input.subject_ref)?;
    let subject_ref = projection_provenance_subject_ref(&subject_kind, &subject_id)?;
    let subject = crate::abilities::provenance::SubjectAttribution::direct_confident(subject_ref);
    let observed_at = parse_projection_timestamp(observed_at).unwrap_or_else(|| ctx.clock.now());
    let source_asof = input.source_asof.and_then(parse_projection_timestamp);
    let data_source = projection_provenance_data_source(claim_data_source);
    let source_identifier =
        projection_source_identifier(&data_source, input, &subject_id, source_asof, observed_at);
    let source = crate::abilities::provenance::SourceAttribution::new(
        data_source,
        vec![source_identifier],
        observed_at,
        source_asof,
        projection_item_confidence(input.item_source),
        None,
    )
    .map_err(|error| format!("projection source provenance invalid: {error}"))?;

    let mut config = crate::abilities::provenance::ProvenanceBuilderConfig::new(
        "claim_shaped_intelligence_projection",
        ctx.clock.now(),
    );
    config.invocation_id = crate::abilities::provenance::InvocationId::new(uuid::Uuid::new_v4());
    config.actor = projection_provenance_actor(input.actor);
    config.mode = ctx.mode.into();
    config.category = crate::abilities::registry::AbilityCategory::Transform;

    let mut builder = crate::abilities::provenance::ProvenanceBuilder::new(config);
    builder.set_subject(subject.clone());
    let source_index = builder.add_source(source);
    let explanation = crate::abilities::provenance::SanitizedExplanation::new(
        "Generated projection committed as claim-shaped intelligence.",
    )
    .map_err(|error| format!("projection provenance explanation invalid: {error}"))?;
    let field_attribution = crate::abilities::provenance::FieldAttribution::llm_synthesis(
        subject,
        vec![crate::abilities::provenance::SourceRef::Source { source_index }],
        crate::abilities::provenance::Confidence::provider_reported(projection_item_confidence(
            input.item_source,
        ))
        .map_err(|error| format!("projection provenance confidence invalid: {error}"))?,
        Some(explanation),
    )
    .map_err(|error| format!("projection field provenance invalid: {error}"))?;
    builder
        .attribute_subtree(
            crate::abilities::provenance::FieldPath::root(),
            field_attribution,
        )
        .map_err(|error| format!("projection provenance attribution failed: {error}"))?;

    let output = serde_json::json!({
        "claimType": input.claim_type,
        "fieldPath": input.field_path,
        "text": input.text,
        "dataSource": claim_data_source,
        "sourceRef": source_ref,
        "sourceAsOf": input.source_asof,
    });
    let output = builder
        .finalize(output)
        .map_err(|error| format!("projection provenance validation failed: {error}"))?;
    serde_json::to_string(output.provenance())
        .map_err(|error| format!("projection provenance serialization failed: {error}"))
}

fn non_empty_join(parts: impl IntoIterator<Item = String>) -> Option<String> {
    let joined = parts
        .into_iter()
        .filter(|part| !part.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    if joined.trim().is_empty() {
        None
    } else {
        Some(joined)
    }
}

fn optional_labeled(label: &str, value: Option<&String>) -> Option<String> {
    value
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(|value| format!("{label}: {value}"))
}

fn optional_labeled_display<T: std::fmt::Display>(label: &str, value: Option<T>) -> Option<String> {
    value.map(|value| format!("{label}: {value}"))
}

fn list_labeled(label: &str, values: &[String]) -> Option<String> {
    if values.is_empty() {
        None
    } else {
        Some(format!("{label}: {}", values.join("; ")))
    }
}

fn current_state_projection_text(state: &crate::intelligence::io::CurrentState) -> Option<String> {
    non_empty_join(
        [
            (!state.working.is_empty()).then(|| format!("Working: {}", state.working.join("; "))),
            (!state.not_working.is_empty())
                .then(|| format!("Not working: {}", state.not_working.join("; "))),
            (!state.unknowns.is_empty())
                .then(|| format!("Unknowns: {}", state.unknowns.join("; "))),
        ]
        .into_iter()
        .flatten(),
    )
}

fn health_projection_text(health: &crate::intelligence::io::AccountHealth) -> Option<String> {
    non_empty_join(
        [
            health.narrative.clone(),
            Some(format!("Health band: {}", health.band)),
            Some(format!("Health score: {:.0}", health.score)),
            health
                .trend
                .rationale
                .as_ref()
                .map(|rationale| format!("Trend: {} - {rationale}", health.trend.direction)),
            health
                .divergence
                .as_ref()
                .map(|divergence| format!("Divergence: {}", divergence.description)),
            list_labeled("Recommended actions", &health.recommended_actions),
        ]
        .into_iter()
        .flatten(),
    )
}

fn recommended_action_projection_text(
    action: &crate::intelligence::io::RecommendedAction,
) -> Option<String> {
    non_empty_join(
        [
            (!action.title.trim().is_empty()).then(|| action.title.clone()),
            (!action.rationale.trim().is_empty())
                .then(|| format!("Rationale: {}", action.rationale)),
            Some(format!("Priority: {}", action.priority)),
            optional_labeled("Suggested due", action.suggested_due.as_ref()),
        ]
        .into_iter()
        .flatten(),
    )
}

fn strategic_priority_projection_text(
    priority: &crate::intelligence::io::StrategicPriority,
) -> Option<String> {
    non_empty_join(
        [
            (!priority.priority.trim().is_empty()).then(|| priority.priority.clone()),
            optional_labeled("Status", priority.status.as_ref()),
            optional_labeled("Owner", priority.owner.as_ref()),
            optional_labeled("Timeline", priority.timeline.as_ref()),
            priority.context.clone(),
            optional_labeled("Source", priority.source.as_ref()),
        ]
        .into_iter()
        .flatten(),
    )
}

fn blocker_projection_text(blocker: &crate::intelligence::io::Blocker) -> Option<String> {
    non_empty_join(
        [
            (!blocker.description.trim().is_empty()).then(|| blocker.description.clone()),
            optional_labeled("Impact", blocker.impact.as_ref()),
            optional_labeled("Owner", blocker.owner.as_ref()),
            optional_labeled("Since", blocker.since.as_ref()),
            optional_labeled("Source", blocker.source.as_ref()),
        ]
        .into_iter()
        .flatten(),
    )
}

fn contract_context_projection_text(
    context: &crate::intelligence::io::ContractContext,
) -> Option<String> {
    non_empty_join(
        [
            optional_labeled("Contract type", context.contract_type.as_ref()),
            context
                .auto_renew
                .map(|value| format!("Auto-renew: {}", if value { "yes" } else { "no" })),
            optional_labeled("Contract start", context.contract_start.as_ref()),
            optional_labeled("Renewal date", context.renewal_date.as_ref()),
            optional_labeled_display(
                "Current ARR",
                context.current_arr.map(|arr| format!("{arr:.0}")),
            ),
            optional_labeled_display("Years remaining", context.multi_year_remaining),
            optional_labeled(
                "Previous renewal outcome",
                context.previous_renewal_outcome.as_ref(),
            ),
            optional_labeled("Procurement", context.procurement_notes.as_ref()),
        ]
        .into_iter()
        .flatten(),
    )
}

fn expansion_signal_projection_text(
    signal: &crate::intelligence::io::ExpansionSignal,
) -> Option<String> {
    non_empty_join(
        [
            (!signal.opportunity.trim().is_empty()).then(|| signal.opportunity.clone()),
            optional_labeled("Stage", signal.stage.as_ref()),
            optional_labeled("Strength", signal.strength.as_ref()),
            optional_labeled_display(
                "ARR impact",
                signal.arr_impact.map(|arr| format!("{arr:.0}")),
            ),
            optional_labeled("Source", signal.source.as_ref()),
        ]
        .into_iter()
        .flatten(),
    )
}

fn agreement_outlook_projection_text(
    outlook: &crate::intelligence::io::AgreementOutlook,
) -> Option<String> {
    non_empty_join(
        [
            outlook.renewal_narrative.clone(),
            optional_labeled("Confidence", outlook.confidence.as_ref()),
            optional_labeled("Expansion potential", outlook.expansion_potential.as_ref()),
            optional_labeled("Recommended start", outlook.recommended_start.as_ref()),
            list_labeled("Risk factors", &outlook.risk_factors),
            list_labeled("Negotiation leverage", &outlook.negotiation_leverage),
            list_labeled("Negotiation risk", &outlook.negotiation_risk),
            outlook.peer_benchmark.as_ref().map(|benchmark| {
                format!(
                    "Peer benchmark: {} ({} source{})",
                    benchmark.narrative,
                    benchmark.source_count,
                    if benchmark.source_count == 1 { "" } else { "s" }
                )
            }),
        ]
        .into_iter()
        .flatten(),
    )
}

fn success_metric_projection_text(
    metric: &crate::intelligence::io::SuccessMetric,
) -> Option<String> {
    non_empty_join(
        [
            (!metric.name.trim().is_empty()).then(|| metric.name.clone()),
            optional_labeled("Target", metric.target.as_ref()),
            optional_labeled("Current", metric.current.as_ref()),
            optional_labeled("Status", metric.status.as_ref()),
            optional_labeled("Owner", metric.owner.as_ref()),
        ]
        .into_iter()
        .flatten(),
    )
}

fn open_commitment_projection_text(
    commitment: &crate::intelligence::io::OpenCommitment,
) -> Option<String> {
    non_empty_join(
        [
            (!commitment.description.trim().is_empty()).then(|| commitment.description.clone()),
            optional_labeled("Owner", commitment.owner.as_ref()),
            optional_labeled("Due", commitment.due_date.as_ref()),
            optional_labeled("Status", commitment.status.as_ref()),
            optional_labeled("Source", commitment.source.as_ref()),
        ]
        .into_iter()
        .flatten(),
    )
}

fn company_context_projection_text(
    context: &crate::intelligence::io::CompanyContext,
) -> Option<String> {
    non_empty_join(
        [
            context.description.clone(),
            context.industry.as_ref().map(|v| format!("Industry: {v}")),
            context.size.as_ref().map(|v| format!("Size: {v}")),
            context
                .headquarters
                .as_ref()
                .map(|v| format!("Headquarters: {v}")),
            context.additional_context.clone(),
        ]
        .into_iter()
        .flatten(),
    )
}

fn stakeholder_engagement_projection_text(
    insight: &crate::intelligence::io::StakeholderInsight,
) -> Option<String> {
    non_empty_join(
        [
            insight.engagement.clone(),
            insight.assessment.clone(),
            insight.role.clone().map(|role| format!("Role: {role}")),
            (!insight.name.trim().is_empty()).then(|| format!("Stakeholder: {}", insight.name)),
        ]
        .into_iter()
        .flatten(),
    )
}

struct ProjectionClaimInput<'a> {
    subject_ref: &'a str,
    projection_origin_subject: Option<&'a str>,
    actor: &'a str,
    data_source: &'a str,
    item_source: Option<&'a crate::intelligence::io::ItemSource>,
    source_asof: Option<&'a str>,
    claim_type: &'a str,
    field_path: &'a str,
    text: &'a str,
    legacy_value: serde_json::Value,
}

enum ProjectionClaimPreflight {
    ExactActiveClaimAlreadyExists,
    Commit { supersedes: Option<String> },
}

fn commit_projection_claim(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    input: ProjectionClaimInput<'_>,
) -> Result<(), String> {
    if input.text.trim().is_empty() {
        return Ok(());
    }
    let historical_backfill = input.data_source == "legacy_intelligence_backfill";
    let claim_data_source = projection_claim_data_source(input.data_source, input.item_source);
    let source_ref = projection_source_ref(&input);
    let supersedes = match projection_claim_preflight(
        db,
        &input,
        source_ref.as_deref(),
        &claim_data_source,
        historical_backfill,
    )? {
        ProjectionClaimPreflight::ExactActiveClaimAlreadyExists => return Ok(()),
        ProjectionClaimPreflight::Commit { supersedes } => supersedes,
    };
    let observed_at = input
        .source_asof
        .map(str::to_string)
        .unwrap_or_else(|| ctx.clock.now().to_rfc3339());
    let provenance_json = projection_provenance_json(
        ctx,
        &input,
        &observed_at,
        source_ref.as_deref(),
        &claim_data_source,
    )?;
    crate::services::claims::commit_claim(
        ctx,
        db,
        crate::services::claims::ClaimProposal {
            id: None,
            expected_claim_version: None,
            subject_ref: input.subject_ref.to_string(),
            claim_type: input.claim_type.to_string(),
            field_path: Some(input.field_path.to_string()),
            topic_key: None,
            text: input.text.to_string(),
            actor: input.actor.to_string(),
            data_source: claim_data_source,
            source_ref,
            source_asof: input.source_asof.map(str::to_string),
            observed_at,
            provenance_json,
            metadata_json: projection_metadata(
                input.legacy_value,
                input.data_source,
                input.projection_origin_subject,
            ),
            thread_id: None,
            temporal_scope: Some(crate::db::claims::TemporalScope::State),
            sensitivity: Some(crate::db::claims::ClaimSensitivity::Internal),
            supersedes,
            tombstone: None,
        },
    )
    .map(|_| ())
    .map_err(|e| format!("commit {} projection claim failed: {e}", input.claim_type))
}

fn projection_claim_preflight(
    db: &ActionDb,
    input: &ProjectionClaimInput<'_>,
    target_source_ref: Option<&str>,
    target_data_source: &str,
    historical_backfill: bool,
) -> Result<ProjectionClaimPreflight, String> {
    let target_source_asof = input.source_asof.map(str::to_string);
    let target_text = crate::services::claims::normalize_claim_text(input.text);
    let (kind, id) = projection_subject_lookup_parts(input.subject_ref)?;

    let exact_exists = db
        .conn_ref()
        .query_row(
            "SELECT 1
               FROM intelligence_claims
              WHERE json_valid(subject_ref) = 1
                AND lower(json_extract(subject_ref, '$.kind')) = lower(?1)
                AND json_extract(subject_ref, '$.id') = ?2
                AND claim_type = ?3
                AND coalesce(field_path, '') = coalesce(?4, '')
                AND text = ?5
                AND (
                    (source_ref IS NULL AND ?6 IS NULL)
                    OR source_ref = ?6
                )
                AND (
                    (source_asof IS NULL AND ?7 IS NULL)
                    OR source_asof = ?7
                )
                AND data_source = ?8
                AND claim_state = 'active'
                AND surfacing_state = 'active'
              LIMIT 1",
            params![
                kind.as_str(),
                id.as_str(),
                input.claim_type,
                input.field_path,
                target_text.as_str(),
                target_source_ref,
                target_source_asof.as_deref(),
                target_data_source
            ],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(|e| format!("projection exact claim preflight failed: {e}"))?
        .is_some();
    if exact_exists {
        return Ok(ProjectionClaimPreflight::ExactActiveClaimAlreadyExists);
    }

    if historical_backfill {
        let same_field_exists = db
            .conn_ref()
            .query_row(
                "SELECT 1
                   FROM intelligence_claims
                  WHERE json_valid(subject_ref) = 1
                    AND lower(json_extract(subject_ref, '$.kind')) = lower(?1)
                    AND json_extract(subject_ref, '$.id') = ?2
                    AND claim_type = ?3
                    AND coalesce(field_path, '') = coalesce(?4, '')
                    AND claim_state = 'active'
                    AND surfacing_state = 'active'
                  LIMIT 1",
                params![
                    kind.as_str(),
                    id.as_str(),
                    input.claim_type,
                    input.field_path
                ],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map_err(|e| format!("projection same-field preflight failed: {e}"))?
            .is_some();
        if same_field_exists {
            return Ok(ProjectionClaimPreflight::ExactActiveClaimAlreadyExists);
        }
    }

    if input.data_source.trim().eq_ignore_ascii_case("glean") {
        let superseded_progressive_id = db
            .conn_ref()
            .query_row(
                "SELECT id
                   FROM intelligence_claims
                  WHERE json_valid(subject_ref) = 1
                    AND lower(json_extract(subject_ref, '$.kind')) = lower(?1)
                    AND json_extract(subject_ref, '$.id') = ?2
                    AND claim_type = ?3
                    AND coalesce(field_path, '') = coalesce(?4, '')
                    AND text = ?5
                    AND claim_state = 'active'
                    AND surfacing_state = 'active'
                    AND (
                        data_source = 'ai_enrichment_progressive'
                        OR (
                            metadata_json IS NOT NULL
                            AND json_valid(metadata_json) = 1
                            AND json_extract(metadata_json, '$.projection_producer') = 'ai_enrichment_progressive'
                        )
                    )
                    AND NOT EXISTS (
                        SELECT 1
                          FROM claim_corroborations cc_local
                         WHERE cc_local.claim_id = intelligence_claims.id
                           AND cc_local.data_source != 'ai_enrichment_progressive'
                           AND cc_local.data_source != 'glean'
                           AND cc_local.data_source NOT LIKE 'glean_%'
                    )
                  ORDER BY created_at DESC
                  LIMIT 1",
                params![
                    kind.as_str(),
                    id.as_str(),
                    input.claim_type,
                    input.field_path,
                    target_text.as_str(),
                ],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|e| format!("projection progressive supersession preflight failed: {e}"))?;
        if let Some(supersedes) = superseded_progressive_id {
            return Ok(ProjectionClaimPreflight::Commit {
                supersedes: Some(supersedes),
            });
        }
    }

    Ok(ProjectionClaimPreflight::Commit { supersedes: None })
}

fn projection_subject_lookup_parts(subject_ref: &str) -> Result<(String, String), String> {
    let value: serde_json::Value = serde_json::from_str(subject_ref)
        .map_err(|error| format!("projection subject_ref is not JSON: {error}"))?;
    let kind = value
        .get("kind")
        .or_else(|| value.get("type"))
        .or_else(|| value.get("entity_type"))
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .ok_or_else(|| "projection subject_ref missing kind".to_string())?;
    let id = value
        .get("id")
        .or_else(|| value.get("entity_id"))
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .ok_or_else(|| "projection subject_ref missing id".to_string())?;
    Ok((kind, id))
}

pub(crate) fn commit_claim_shaped_intelligence_projection(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    intel: &crate::intelligence::IntelligenceJson,
    actor: &str,
    data_source: &str,
) -> Result<(), String> {
    let subject_ref = subject_ref_for_entity(&intel.entity_type, &intel.entity_id)?;

    if let Some(summary) = intel.executive_assessment.as_deref() {
        commit_projection_claim(
            ctx,
            db,
            ProjectionClaimInput {
                subject_ref: &subject_ref,
                projection_origin_subject: None,
                actor,
                data_source,
                item_source: None,
                source_asof: None,
                claim_type: "entity_summary",
                field_path: "executiveAssessment",
                text: summary,
                legacy_value: serde_json::Value::String(summary.to_string()),
            },
        )?;
    }

    if let Some(pull_quote) = intel.pull_quote.as_deref() {
        commit_projection_claim(
            ctx,
            db,
            ProjectionClaimInput {
                subject_ref: &subject_ref,
                projection_origin_subject: None,
                actor,
                data_source,
                item_source: None,
                source_asof: None,
                claim_type: "entity_summary",
                field_path: "pullQuote",
                text: pull_quote,
                legacy_value: serde_json::Value::String(pull_quote.to_string()),
            },
        )?;
    }

    if let Some(health) = intel.health.as_ref() {
        if let Some(text) = health_projection_text(health) {
            commit_projection_claim(
                ctx,
                db,
                ProjectionClaimInput {
                    subject_ref: &subject_ref,
                    projection_origin_subject: None,
                    actor,
                    data_source,
                    item_source: None,
                    source_asof: None,
                    claim_type: "entity_current_state",
                    field_path: "health",
                    text: &text,
                    legacy_value: serde_json::to_value(health)
                        .unwrap_or_else(|_| serde_json::json!({ "narrative": text.clone() })),
                },
            )?;
        }

        for (idx, action) in health.recommended_actions.iter().enumerate() {
            commit_projection_claim(
                ctx,
                db,
                ProjectionClaimInput {
                    subject_ref: &subject_ref,
                    projection_origin_subject: None,
                    actor,
                    data_source,
                    item_source: None,
                    source_asof: None,
                    claim_type: "recommendation",
                    field_path: &format!("health.recommendedActions[{idx}]"),
                    text: action,
                    legacy_value: serde_json::Value::String(action.clone()),
                },
            )?;
        }
    }

    for (idx, risk) in intel.risks.iter().enumerate() {
        commit_projection_claim(
            ctx,
            db,
            ProjectionClaimInput {
                subject_ref: &subject_ref,
                projection_origin_subject: None,
                actor,
                data_source,
                item_source: risk.item_source.as_ref(),
                source_asof: source_asof_from_item_source(risk.item_source.as_ref()),
                claim_type: "entity_risk",
                field_path: &format!("risks[{idx}]"),
                text: &risk.text,
                legacy_value: serde_json::to_value(risk)
                    .unwrap_or_else(|_| serde_json::json!({ "text": &risk.text })),
            },
        )?;
    }

    for (idx, action) in intel.recommended_actions.iter().enumerate() {
        let Some(text) = recommended_action_projection_text(action) else {
            continue;
        };
        commit_projection_claim(
            ctx,
            db,
            ProjectionClaimInput {
                subject_ref: &subject_ref,
                projection_origin_subject: None,
                actor,
                data_source,
                item_source: None,
                source_asof: None,
                claim_type: "recommendation",
                field_path: &format!("recommendedActions[{idx}]"),
                text: &text,
                legacy_value: serde_json::to_value(action)
                    .unwrap_or_else(|_| serde_json::json!({ "title": text.clone() })),
            },
        )?;
    }

    for (idx, win) in intel.recent_wins.iter().enumerate() {
        commit_projection_claim(
            ctx,
            db,
            ProjectionClaimInput {
                subject_ref: &subject_ref,
                projection_origin_subject: None,
                actor,
                data_source,
                item_source: win.item_source.as_ref(),
                source_asof: source_asof_from_item_source(win.item_source.as_ref()),
                claim_type: "entity_win",
                field_path: &format!("recentWins[{idx}]"),
                text: &win.text,
                legacy_value: serde_json::to_value(win)
                    .unwrap_or_else(|_| serde_json::json!({ "text": &win.text })),
            },
        )?;
    }

    if let Some(state) = intel.current_state.as_ref() {
        if let Some(text) = current_state_projection_text(state) {
            commit_projection_claim(
                ctx,
                db,
                ProjectionClaimInput {
                    subject_ref: &subject_ref,
                    projection_origin_subject: None,
                    actor,
                    data_source,
                    item_source: None,
                    source_asof: None,
                    claim_type: "entity_current_state",
                    field_path: "currentState",
                    text: &text,
                    legacy_value: serde_json::to_value(state)
                        .unwrap_or_else(|_| serde_json::json!({ "text": text.clone() })),
                },
            )?;
        }
    }

    for (idx, priority) in intel.strategic_priorities.iter().enumerate() {
        let Some(text) = strategic_priority_projection_text(priority) else {
            continue;
        };
        commit_projection_claim(
            ctx,
            db,
            ProjectionClaimInput {
                subject_ref: &subject_ref,
                projection_origin_subject: None,
                actor,
                data_source,
                item_source: None,
                source_asof: None,
                claim_type: "entity_current_state",
                field_path: &format!("strategicPriorities[{idx}]"),
                text: &text,
                legacy_value: serde_json::to_value(priority)
                    .unwrap_or_else(|_| serde_json::json!({ "priority": text.clone() })),
            },
        )?;
    }

    for (idx, blocker) in intel.blockers.iter().enumerate() {
        let Some(text) = blocker_projection_text(blocker) else {
            continue;
        };
        commit_projection_claim(
            ctx,
            db,
            ProjectionClaimInput {
                subject_ref: &subject_ref,
                projection_origin_subject: None,
                actor,
                data_source,
                item_source: None,
                source_asof: None,
                claim_type: "entity_risk",
                field_path: &format!("blockers[{idx}]"),
                text: &text,
                legacy_value: serde_json::to_value(blocker)
                    .unwrap_or_else(|_| serde_json::json!({ "description": text.clone() })),
            },
        )?;
    }

    if let Some(context) = intel.contract_context.as_ref() {
        if let Some(text) = contract_context_projection_text(context) {
            commit_projection_claim(
                ctx,
                db,
                ProjectionClaimInput {
                    subject_ref: &subject_ref,
                    projection_origin_subject: None,
                    actor,
                    data_source,
                    item_source: None,
                    source_asof: None,
                    claim_type: if intel.entity_type == "account" {
                        "company_context"
                    } else {
                        "entity_current_state"
                    },
                    field_path: "contractContext",
                    text: &text,
                    legacy_value: serde_json::to_value(context)
                        .unwrap_or_else(|_| serde_json::json!({ "summary": text.clone() })),
                },
            )?;
        }
    }

    for (idx, signal) in intel.expansion_signals.iter().enumerate() {
        let Some(text) = expansion_signal_projection_text(signal) else {
            continue;
        };
        commit_projection_claim(
            ctx,
            db,
            ProjectionClaimInput {
                subject_ref: &subject_ref,
                projection_origin_subject: None,
                actor,
                data_source,
                item_source: signal.item_source.as_ref(),
                source_asof: source_asof_from_item_source(signal.item_source.as_ref()),
                claim_type: "entity_current_state",
                field_path: &format!("expansionSignals[{idx}]"),
                text: &text,
                legacy_value: serde_json::to_value(signal)
                    .unwrap_or_else(|_| serde_json::json!({ "opportunity": text.clone() })),
            },
        )?;
    }

    if let Some(outlook) = intel.agreement_outlook.as_ref() {
        if let Some(text) = agreement_outlook_projection_text(outlook) {
            commit_projection_claim(
                ctx,
                db,
                ProjectionClaimInput {
                    subject_ref: &subject_ref,
                    projection_origin_subject: None,
                    actor,
                    data_source,
                    item_source: None,
                    source_asof: None,
                    claim_type: "entity_current_state",
                    field_path: "agreementOutlook",
                    text: &text,
                    legacy_value: serde_json::to_value(outlook)
                        .unwrap_or_else(|_| serde_json::json!({ "summary": text.clone() })),
                },
            )?;
        }
    }

    for (idx, value) in intel.value_delivered.iter().enumerate() {
        commit_projection_claim(
            ctx,
            db,
            ProjectionClaimInput {
                subject_ref: &subject_ref,
                projection_origin_subject: None,
                actor,
                data_source,
                item_source: value.item_source.as_ref(),
                source_asof: source_asof_from_item_source(value.item_source.as_ref()),
                claim_type: "value_delivered",
                field_path: &format!("valueDelivered[{idx}]"),
                text: &value.statement,
                legacy_value: serde_json::to_value(value)
                    .unwrap_or_else(|_| serde_json::json!({ "statement": &value.statement })),
            },
        )?;
    }

    if let Some(metrics) = intel.success_metrics.as_ref() {
        for (idx, metric) in metrics.iter().enumerate() {
            let Some(text) = success_metric_projection_text(metric) else {
                continue;
            };
            commit_projection_claim(
                ctx,
                db,
                ProjectionClaimInput {
                    subject_ref: &subject_ref,
                    projection_origin_subject: None,
                    actor,
                    data_source,
                    item_source: None,
                    source_asof: None,
                    claim_type: "entity_current_state",
                    field_path: &format!("successMetrics[{idx}]"),
                    text: &text,
                    legacy_value: serde_json::to_value(metric)
                        .unwrap_or_else(|_| serde_json::json!({ "name": text.clone() })),
                },
            )?;
        }
    }

    if let Some(commitments) = intel.open_commitments.as_ref() {
        for (idx, commitment) in commitments.iter().enumerate() {
            let Some(text) = open_commitment_projection_text(commitment) else {
                continue;
            };
            commit_projection_claim(
                ctx,
                db,
                ProjectionClaimInput {
                    subject_ref: &subject_ref,
                    projection_origin_subject: None,
                    actor,
                    data_source,
                    item_source: commitment.item_source.as_ref(),
                    source_asof: source_asof_from_item_source(commitment.item_source.as_ref()),
                    claim_type: if intel.entity_type == "account" {
                        "commitment"
                    } else {
                        "entity_current_state"
                    },
                    field_path: &format!("openCommitments[{idx}]"),
                    text: &text,
                    legacy_value: serde_json::to_value(commitment)
                        .unwrap_or_else(|_| serde_json::json!({ "description": text.clone() })),
                },
            )?;
        }
    }

    for (idx, insight) in intel.stakeholder_insights.iter().enumerate() {
        let Some(person_id) = insight.person_id.as_deref() else {
            continue;
        };
        let Some(text) = stakeholder_engagement_projection_text(insight) else {
            continue;
        };
        let person_subject_ref = subject_ref_for_entity("person", person_id)?;
        commit_projection_claim(
            ctx,
            db,
            ProjectionClaimInput {
                subject_ref: &person_subject_ref,
                projection_origin_subject: Some(&subject_ref),
                actor,
                data_source,
                item_source: insight.item_source.as_ref(),
                source_asof: source_asof_from_item_source(insight.item_source.as_ref()),
                claim_type: "stakeholder_engagement",
                field_path: &format!("stakeholderInsights[{idx}].engagement"),
                text: &text,
                legacy_value: serde_json::to_value(insight)
                    .unwrap_or_else(|_| serde_json::json!({ "engagement": text.clone() })),
            },
        )?;
    }

    if intel.entity_type == "account" {
        if let Some(context) = intel.company_context.as_ref() {
            if let Some(text) = company_context_projection_text(context) {
                commit_projection_claim(
                    ctx,
                    db,
                    ProjectionClaimInput {
                        subject_ref: &subject_ref,
                        projection_origin_subject: None,
                        actor,
                        data_source,
                        item_source: None,
                        source_asof: None,
                        claim_type: "company_context",
                        field_path: "companyContext",
                        text: &text,
                        legacy_value: serde_json::to_value(context)
                            .unwrap_or_else(|_| serde_json::json!({ "description": text.clone() })),
                    },
                )?;
            }
        }
    }

    Ok(())
}

/// Backfill claim-shaped projections for legacy entity intelligence rows.
///
/// This is intentionally entity-generic. It reuses the same projection producer
/// as live enrichment instead of teaching downstream surfaces to read legacy
/// `entity_assessment` rows directly.
pub fn backfill_entity_intelligence_projection_claims(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
) -> Result<EntityIntelligenceProjectionBackfillReport, String> {
    backfill_entity_intelligence_projection_claims_batch(ctx, db, 0, usize::MAX)
}

pub fn backfill_entity_intelligence_projection_claims_batch(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    offset: usize,
    limit: usize,
) -> Result<EntityIntelligenceProjectionBackfillReport, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let candidates = entity_intelligence_backfill_candidates(db)?;
    let total_rows = candidates.len();
    let start = offset.min(total_rows);
    let end = start.saturating_add(limit).min(total_rows);
    let mut report = EntityIntelligenceProjectionBackfillReport {
        total_rows,
        rows_examined: end.saturating_sub(start),
        next_offset: end,
        finished: end >= total_rows,
        ..Default::default()
    };

    for (entity_type, entity_id) in candidates[start..end].iter() {
        let intel = match db.get_entity_intelligence(entity_id) {
            Ok(Some(intel)) => intel,
            Ok(None) => continue,
            Err(error) => {
                report.errors.push(format!(
                    "{entity_type}:{entity_id} legacy intelligence read failed: {error}"
                ));
                continue;
            }
        };
        let before_count = match claim_row_count(db) {
            Ok(count) => count,
            Err(error) => {
                report.errors.push(format!(
                    "{entity_type}:{entity_id} pre-backfill claim count failed: {error}"
                ));
                continue;
            }
        };

        let backfill_result = db.with_transaction(|tx| {
            let _projection_guard =
                crate::services::claims::suppress_legacy_projection_for_current_thread();
            let _canonical_match_guard =
                crate::services::claims::suppress_canonical_match_for_current_thread();
            let _shadow_guard =
                crate::services::claims::suppress_shadow_canonicalization_for_current_thread();
            commit_claim_shaped_intelligence_projection(
                ctx,
                tx,
                &intel,
                "agent:entity_intelligence_backfill",
                "legacy_intelligence_backfill",
            )?;
            Ok(false)
        });

        match backfill_result {
            Ok(enqueued_recompute) => {
                report.rows_backfilled += 1;
                if enqueued_recompute {
                    report.recompute_jobs_enqueued += 1;
                }
                match claim_row_count(db) {
                    Ok(after_count) => {
                        report.claims_inserted += after_count.saturating_sub(before_count);
                    }
                    Err(error) => report.errors.push(format!(
                        "{entity_type}:{entity_id} post-backfill claim count failed: {error}"
                    )),
                }
            }
            Err(error) => {
                report.errors.push(format!(
                    "{entity_type}:{entity_id} projection backfill failed: {error}"
                ));
            }
        }
    }

    Ok(report)
}

fn entity_intelligence_backfill_candidates(db: &ActionDb) -> Result<Vec<(String, String)>, String> {
    let exists: bool = db
        .conn_ref()
        .query_row(
            "SELECT EXISTS(
                SELECT 1 FROM sqlite_master
                 WHERE type = 'table' AND name = 'entity_assessment'
             )",
            [],
            |row| row.get(0),
        )
        .map_err(|error| format!("entity_assessment schema probe failed: {error}"))?;
    if !exists {
        return Ok(Vec::new());
    }

    let mut stmt = db
        .conn_ref()
        .prepare(
            "SELECT entity_type, entity_id
               FROM entity_assessment
              WHERE entity_id IS NOT NULL
                AND trim(entity_id) != ''
              ORDER BY entity_type, entity_id",
        )
        .map_err(|error| format!("prepare entity intelligence backfill scan failed: {error}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|error| format!("query entity intelligence backfill scan failed: {error}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("collect entity intelligence backfill scan failed: {error}"))
}

fn claim_row_count(db: &ActionDb) -> Result<usize, String> {
    db.conn_ref()
        .query_row("SELECT COUNT(*) FROM intelligence_claims", [], |row| {
            row.get::<_, usize>(0)
        })
        .map_err(|error| format!("claim count failed: {error}"))
}

fn stage_failure_message(stage: &str) -> &str {
    match stage {
        "context_gather" => "context gather",
        "pty_permit" => "PTY permit acquisition",
        "pty_enrichment" => "Claude PTY enrichment",
        "write_results" => "result writeback",
        "finalize" => "refresh finalization",
        "relationship_persist" => "relationship persistence",
        _ => stage,
    }
}

fn emit_manual_refresh_failed(
    ctx: &ServiceContext<'_>,
    app_handle: Option<&tauri::AppHandle>,
    entity_id: &str,
    entity_type: &str,
    entity_label: &str,
    stage: &str,
    error: &str,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    if let Some(app) = app_handle {
        let payload = serde_json::json!({
            "phase": "failed",
            "message": format!(
                "Insight refresh failed for {} during {}",
                entity_label,
                stage_failure_message(stage)
            ),
            "count": 1,
            "manual": true,
            "entityId": entity_id,
            "entityType": entity_type,
            "stage": stage,
            "error": error,
        });
        if let Err(e) = app.emit("background-work-status", payload.clone()) {
            log::warn!("emit manual refresh background failure status failed: {e}");
        }
        if let Err(e) = app.emit("intelligence-refresh-failed", payload) {
            log::warn!("emit manual refresh failure event failed: {e}");
        }
    }
    Ok(())
}

fn emit_manual_refresh_failed_best_effort(
    ctx: &ServiceContext<'_>,
    app_handle: Option<&tauri::AppHandle>,
    entity_id: &str,
    entity_type: &str,
    entity_label: &str,
    stage: &str,
    error: &str,
) {
    if let Err(e) = emit_manual_refresh_failed(
        ctx,
        app_handle,
        entity_id,
        entity_type,
        entity_label,
        stage,
        error,
    ) {
        log::warn!("emit manual refresh failure notification failed: {e}");
    }
}

fn manual_refresh_error(stage: &str, error: &str) -> String {
    format!(
        "manual refresh failed during {}: {}",
        stage_failure_message(stage),
        error
    )
}

/// Enrich an entity via the intelligence queue (split-lock pattern).
pub async fn enrich_entity(
    ctx: &ServiceContext<'_>,
    entity_id: String,
    entity_type: String,
    state: &std::sync::Arc<AppState>,
    app_handle: Option<&tauri::AppHandle>,
    request_id: &str,
) -> Result<crate::intelligence::IntelligenceJson, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;

    log::warn!(
        "[I535] enrich_entity ENTERED: entity_id={}, type={}, provider={}",
        entity_id,
        entity_type,
        state.context_provider().provider_name(),
    );

    let request = IntelRequest::new(entity_id, entity_type, IntelPriority::Manual);
    let manual_entity_id = request.entity_id.clone();

    if let Some(app) = app_handle {
        if let Err(e) = app.emit(
            "background-work-status",
            serde_json::json!({
                "phase": "started",
                "message": format!("Updating insights for {}…", manual_entity_id),
                "count": 1,
                "manual": true,
            }),
        ) {
            log::warn!("emit manual refresh start status failed for {manual_entity_id}: {e}");
        }
    }

    // Manual refresh: clear circuit breaker so enrichment proceeds
    let entity_id_for_reset = request.entity_id.clone();
    if let Err(e) = state
        .db_write(move |db| {
            crate::self_healing::scheduler::reset_circuit_breaker(db, &entity_id_for_reset);
            Ok(())
        })
        .await
        .map_err(String::from)
    {
        log::warn!("reset circuit breaker before manual refresh failed: {e}");
    }

    let input = match gather_enrichment_input(state, &request) {
        Ok(input) => input,
        Err(e) => {
            log::warn!(
                "[I535] gather_enrichment_input FAILED for {}: {}",
                request.entity_id,
                e
            );
            emit_manual_refresh_failed_best_effort(
                ctx,
                app_handle,
                &request.entity_id,
                &request.entity_type,
                &request.entity_id,
                "context_gather",
                &e,
            );
            return Err(manual_refresh_error("context_gather", &e));
        }
    };

    let ai_config = state
        .config
        .read()
        .as_ref()
        .map(|c| c.ai_models.clone())
        .unwrap_or_default();

    // /ADR-0100: Glean-first enrichment for manual refresh.
    // Try Glean chat if connected, fall back to PTY on failure.
    // Timeout on user-facing permit acquisition — return a friendly message
    // instead of blocking indefinitely when background enrichment is running.
    let _permit = match tokio::time::timeout(
        std::time::Duration::from_secs(10),
        state.permits.user_initiated.acquire(),
    )
    .await
    {
        Ok(Ok(permit)) => permit,
        Ok(Err(_)) => {
            let error = "PTY permit closed";
            emit_manual_refresh_failed_best_effort(
                ctx,
                app_handle,
                &input.entity_id,
                &input.entity_type,
                &input.entity_name,
                "pty_permit",
                error,
            );
            return Err(manual_refresh_error("pty_permit", error));
        }
        Err(_) => {
            let error = "Background work in progress — your refresh is queued and will run shortly";
            emit_manual_refresh_failed_best_effort(
                ctx,
                app_handle,
                &input.entity_id,
                &input.entity_type,
                &input.entity_name,
                "pty_permit",
                error,
            );
            return Err(manual_refresh_error("pty_permit", error));
        }
    };

    //  single coherent snapshot of context state —
    // is_remote + Glean Arc captured under one read-lock acquisition.
    // Avoids the L2 codex race where a Local switch between separate
    // getters could leave callers in a mixed-state world.
    let snap = state.context_snapshot();
    let is_remote = snap.is_remote();
    let glean_endpoint = snap.remote_endpoint();
    log::warn!(
        "[I535] enrich_entity: provider={}, is_remote={}, endpoint={:?}, has_ctx={}, entity={} ({})",
        snap.provider_name(),
        is_remote,
        glean_endpoint.is_some(),
        input.intelligence_context.is_some(),
        input.entity_name,
        input.entity_type,
    );
    let parsed = if is_remote {
        // Try Glean-first path
        let mut glean_result = None;
        if let (Some(_endpoint), Some(ref ctx)) = (&glean_endpoint, &input.intelligence_context) {
            //  route through the snapshot's Glean Arc
            // per ADR-0091. Falls through to PTY when the snapshot shows
            // None (bridge cleared by atomic Local swap). The snapshot
            // captured above is immutable here, so a concurrent settings
            // change cannot perturb the routing decision mid-call.
            let provider = match snap.glean_intelligence_provider.clone() {
                Some(p) => Some(p),
                None => {
                    log::warn!(
                        "[I535] Context-mode snapshot for manual refresh on {} \
                         shows is_remote=true but no Glean Arc; settings raced this \
                         call. Falling through to PTY per ADR-0091.",
                        input.entity_name
                    );
                    None
                }
            };
            // This path is the services::intelligence manual-refresh entry,
            // always user-initiated — pass is_background=false so the UI
            // gets degraded/fallback toasts.
            if let Some(provider) = provider {
                match provider
                    .enrich_entity(
                        &input.entity_id,
                        &input.entity_type,
                        &input.entity_name,
                        ctx,
                        input.relationship.as_deref(),
                        app_handle,
                        false,
                        input.active_preset.as_ref(),
                    )
                    .await
                {
                    Ok(intel) => {
                        log::info!(
                            "[I535] Manual Glean enrichment succeeded for {}",
                            input.entity_name
                        );
                        let inferred = if let Ok(raw) = serde_json::to_string(&intel) {
                            crate::intelligence::extract_inferred_relationships(&raw)
                        } else {
                            Vec::new()
                        };
                        glean_result = Some(crate::intel_queue::EnrichmentParseResult {
                            intel,
                            inferred_relationships: inferred,
                            producer: crate::intel_queue::EnrichmentProducer::Glean,
                        });
                    }
                    Err(e) => {
                        log::warn!(
                            "[I535] Manual Glean enrichment failed for {}, falling back to PTY: {}",
                            input.entity_name,
                            e
                        );
                        // Surface the fallback loudly — otherwise users see
                        // local-sourced items on a Glean-mode account with no
                        // signal that Glean enrichment couldn't complete.
                        {
                            let mut audit = state.audit_log.lock();
                            let actor = abilities_runtime::abilities::registry::Actor::User;
                            let fields = crate::audit_log::AuditFields::new(
                                "data_access",
                                serde_json::json!({
                                    "entity_id": input.entity_id,
                                    "entity_type": input.entity_type,
                                    "entity_name": input.entity_name,
                                    "reason": e.to_string(),
                                }),
                            )
                            .with_request_id(request_id.to_string());
                            if let Err(audit_error) = crate::audit_log::emit_surface_audit(
                                &mut audit,
                                "glean_enrichment_fellback_to_pty",
                                &actor,
                                fields,
                            ) {
                                log::warn!("emit Glean fallback audit entry failed: {audit_error}");
                            }
                        }
                        if let Some(handle) = app_handle {
                            if let Err(emit_error) = handle.emit(
                                "enrichment-glean-fallback",
                                serde_json::json!({
                                    "entity_id": input.entity_id,
                                    "entity_type": input.entity_type,
                                    "entity_name": input.entity_name,
                                    "reason": e.to_string(),
                                }),
                            ) {
                                log::warn!("emit Glean fallback event failed: {emit_error}");
                            }
                        }
                    }
                }
            } // end if let Some(provider) — bridge-empty case skipped Glean and
              // falls through to the PTY path below via glean_result == None.
        }

        match glean_result {
            Some(parsed) => parsed,
            None => {
                // Fallback to PTY
                let input_for_enrichment = input.clone();
                let ai_config_for_enrichment = ai_config.clone();
                let app_handle_clone = app_handle.cloned();
                let pty_result = tauri::async_runtime::spawn_blocking(move || {
                    let usage_context =
                        AiUsageContext::new("intelligence", "manual_entity_enrichment")
                            .with_trigger("manual_refresh");
                    run_enrichment(
                        &input_for_enrichment,
                        &ai_config_for_enrichment,
                        app_handle_clone.as_ref(),
                        usage_context,
                    )
                })
                .await;
                match pty_result {
                    Ok(Ok(parsed)) => parsed,
                    Ok(Err(e)) => {
                        emit_manual_refresh_failed_best_effort(
                            ctx,
                            app_handle,
                            &input.entity_id,
                            &input.entity_type,
                            &input.entity_name,
                            "pty_enrichment",
                            &e,
                        );
                        return Err(manual_refresh_error("pty_enrichment", &e));
                    }
                    Err(e) => {
                        let error = format!("Enrichment task panicked: {}", e);
                        emit_manual_refresh_failed_best_effort(
                            ctx,
                            app_handle,
                            &input.entity_id,
                            &input.entity_type,
                            &input.entity_name,
                            "pty_enrichment",
                            &error,
                        );
                        return Err(manual_refresh_error("pty_enrichment", &error));
                    }
                }
            }
        }
    } else {
        // Local-only: direct PTY path
        let input_for_enrichment = input.clone();
        let ai_config_for_enrichment = ai_config.clone();
        let app_handle_clone = app_handle.cloned();
        let pty_result = tauri::async_runtime::spawn_blocking(move || {
            let usage_context = AiUsageContext::new("intelligence", "manual_entity_enrichment")
                .with_trigger("manual_refresh");
            run_enrichment(
                &input_for_enrichment,
                &ai_config_for_enrichment,
                app_handle_clone.as_ref(),
                usage_context,
            )
        })
        .await;
        match pty_result {
            Ok(Ok(parsed)) => parsed,
            Ok(Err(e)) => {
                emit_manual_refresh_failed_best_effort(
                    ctx,
                    app_handle,
                    &input.entity_id,
                    &input.entity_type,
                    &input.entity_name,
                    "pty_enrichment",
                    &e,
                );
                return Err(manual_refresh_error("pty_enrichment", &e));
            }
            Err(e) => {
                let error = format!("Enrichment task panicked: {}", e);
                emit_manual_refresh_failed_best_effort(
                    ctx,
                    app_handle,
                    &input.entity_id,
                    &input.entity_type,
                    &input.entity_name,
                    "pty_enrichment",
                    &error,
                );
                return Err(manual_refresh_error("pty_enrichment", &error));
            }
        }
    };

    let db = ActionDb::open(std::sync::Arc::new(crate::db::LocalKeychain::new())).map_err(|e| {
        let e = format!("Failed to open DB: {e}");
        emit_manual_refresh_failed_best_effort(
            ctx,
            app_handle,
            &input.entity_id,
            &input.entity_type,
            &input.entity_name,
            "write_results",
            &e,
        );
        manual_refresh_error("write_results", &e)
    })?;
    let prepared = match compose_enrichment_intelligence(
        state,
        &db,
        &input,
        &parsed.intel,
        parsed.producer,
        Some(&ai_config),
    ) {
        Ok(composition) => composition,
        Err(e) => {
            emit_manual_refresh_failed_best_effort(
                ctx,
                app_handle,
                &input.entity_id,
                &input.entity_type,
                &input.entity_name,
                "write_results",
                &e,
            );
            return Err(manual_refresh_error("write_results", &e));
        }
    };
    if let Err(e) =
        persist_enrichment_write_results_via_db_service(state, &input, &prepared, parsed.producer)
            .await
    {
        emit_manual_refresh_failed_best_effort(
            ctx,
            app_handle,
            &input.entity_id,
            &input.entity_type,
            &input.entity_name,
            "write_results",
            &e,
        );
        return Err(manual_refresh_error("write_results", &e));
    }
    let final_intel = prepared.into_intelligence();
    if let Err(e) = run_enrichment_finalize_post_commit_via_db_service(
        state,
        &input,
        &final_intel,
        &parsed.inferred_relationships,
        FinalizeMode::ManualRefresh {
            producer: parsed.producer,
        },
    )
    .await
    {
        emit_manual_refresh_failed_best_effort(
            ctx,
            app_handle,
            &input.entity_id,
            &input.entity_type,
            &input.entity_name,
            "finalize",
            &e,
        );
        return Err(manual_refresh_error("finalize", &e));
    }

    if let Some(app) = app_handle {
        if let Err(e) = app.emit(
            "background-work-status",
            serde_json::json!({
                "phase": "completed",
                "message": format!("Insights updated for {}", input.entity_name),
                "count": 1,
                "manual": true,
            }),
        ) {
            log::warn!(
                "emit manual refresh completion status failed for {}: {e}",
                input.entity_id
            );
        }
    }

    Ok(final_intel)
}

pub fn persist_entity_keywords(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    entity_type: &str,
    entity_id: &str,
    keywords_json: &str,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    if entity_type != "account" && entity_type != "project" {
        return Ok(());
    }

    db.with_transaction(|tx| {
        match entity_type {
            "account" => tx
                .update_account_keywords(entity_id, keywords_json)
                .map_err(|e| format!("keywords update failed: {e}"))?,
            "project" => tx
                .update_project_keywords(entity_id, keywords_json)
                .map_err(|e| format!("keywords update failed: {e}"))?,
            _ => {}
        }

        crate::services::signals::emit(
            ctx,
            tx,
            entity_type,
            entity_id,
            "keywords_updated",
            "ai_enrichment",
            None,
            0.7,
        )
        .map_err(|e| format!("signal emit failed: {e}"))?;

        Ok(())
    })
}

pub fn upsert_assessment_from_enrichment(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    engine: &PropagationEngine,
    entity_type: &str,
    entity_id: &str,
    intel: &crate::intelligence::IntelligenceJson,
) -> Result<(), String> {
    db.with_transaction(|tx| {
        upsert_assessment_from_enrichment_in_active_transaction(
            ctx,
            tx,
            engine,
            EnrichmentAssessmentUpsert {
                entity_type,
                entity_id,
                intel,
                projection_intel: intel,
                projection_data_source: "ai_enrichment",
                cleared_dimensions: &[],
            },
        )
    })
}

pub(crate) struct EnrichmentAssessmentUpsert<'a> {
    pub entity_type: &'a str,
    pub entity_id: &'a str,
    pub intel: &'a crate::intelligence::IntelligenceJson,
    pub projection_intel: &'a crate::intelligence::IntelligenceJson,
    pub projection_data_source: &'a str,
    pub cleared_dimensions: &'a [&'static str],
}

pub(crate) fn upsert_assessment_from_enrichment_in_active_transaction(
    ctx: &ServiceContext<'_>,
    tx: &ActionDb,
    engine: &PropagationEngine,
    upsert: EnrichmentAssessmentUpsert<'_>,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    // Merge value_delivered — preserve user-confirmed items during re-enrichment.
    let mut intel = upsert.intel.clone();
    let mut projection_intel = upsert.projection_intel.clone();
    if let Ok(Some(existing)) = tx.get_entity_intelligence(upsert.entity_id) {
        merge_user_confirmed_values(&mut intel, &existing);
        if upsert.projection_data_source != "glean" {
            merge_user_confirmed_values(&mut projection_intel, &existing);
        }
    }
    withdraw_cleared_dimension_projection_claims(
        ctx,
        tx,
        upsert.entity_type,
        upsert.entity_id,
        upsert.cleared_dimensions,
    )?;
    commit_claim_shaped_intelligence_projection(
        ctx,
        tx,
        &projection_intel,
        "agent:intelligence",
        upsert.projection_data_source,
    )?;
    let refreshed_projection_withdrawal = withdraw_refreshed_projection_claims(
        ctx,
        tx,
        upsert.entity_type,
        upsert.entity_id,
        &projection_intel,
        upsert.projection_data_source,
    )?;
    crate::services::derived_state::upsert_entity_intelligence_legacy_snapshot(ctx, tx, &intel)
        .map_err(|e| e.to_string())?;
    let (signal_id, _) = crate::services::signals::emit_and_propagate(
        ctx,
        tx,
        engine,
        upsert.entity_type,
        upsert.entity_id,
        "entity_intelligence_updated",
        "ai_enrichment",
        None,
        0.8,
    )
    .map_err(|e| format!("signal emit failed: {e}"))?;
    let recompute_subjects = projection_claim_recompute_subjects(
        upsert.entity_type,
        upsert.entity_id,
        &intel,
        &refreshed_projection_withdrawal.affected_subjects,
    );
    enqueue_projection_claim_recomputes(ctx, tx, &signal_id, recompute_subjects);

    // After enrichment, reconcile AI objectives with user objectives
    if upsert.entity_type == "account" {
        if let Err(e) =
            crate::services::success_plans::reconcile_objectives(ctx, tx, upsert.entity_id)
        {
            log::warn!(
                "Objective reconciliation failed for {}: {e}",
                upsert.entity_id
            );
        }
    }

    // DOS Work-tab: Best-effort bridge of AI-inferred commitments → Actions.
    // Enrichment write is the source of truth; bridge errors must not fail it.
    if upsert.entity_type == "account" {
        if let Some(ref commitments) = intel.open_commitments {
            let sync_result =
                crate::services::commitment_bridge::intelligence_commitment_ingestion_items(
                    upsert.entity_type,
                    upsert.entity_id,
                    commitments,
                )
                .and_then(|items| {
                    crate::services::commitment_bridge::sync_ai_commitments_with_ingestion_sources(
                        ctx,
                        tx,
                        upsert.entity_type,
                        upsert.entity_id,
                        &items,
                    )
                });
            match sync_result {
                Ok(summary) => log::info!(
                    "commitment_bridge: {} created, {} updated, {} tombstoned-skip, {} missing-id, {} malformed-id ({}:{})",
                    summary.created,
                    summary.updated,
                    summary.skipped_tombstoned,
                    summary.skipped_missing_id,
                    summary.skipped_malformed_id,
                    upsert.entity_type,
                    upsert.entity_id
                ),
                Err(e) => log::warn!(
                    "commitment_bridge sync failed for {}:{} (non-fatal): {e}",
                    upsert.entity_type,
                    upsert.entity_id
                ),
            }
        }
    }

    Ok(())
}

fn enqueue_projection_claim_recomputes(
    ctx: &ServiceContext<'_>,
    tx: &ActionDb,
    origin_signal_id: &str,
    subjects: BTreeSet<(String, String)>,
) {
    for (subject_type, subject_id) in subjects {
        if let Err(error) = crate::services::invalidation_jobs::enqueue_signal_claim_recompute_in_tx(
            tx,
            origin_signal_id,
            &subject_type,
            &subject_id,
        ) {
            log::error!(
                "intelligence: failed to enqueue claim_recompute for {subject_type}:{subject_id}: {error}"
            );
            if let Err(record_error) = crate::services::mutations::record_pipeline_failure(
                ctx,
                tx,
                "trust_recompute",
                Some(&subject_id),
                Some(&subject_type),
                "invalidation_enqueue_failed",
                Some(&format!(
                    "origin_signal_id={origin_signal_id} error={error}"
                )),
                0,
            ) {
                log::warn!(
                    "intelligence: failed to record claim_recompute enqueue failure for {subject_type}:{subject_id}: {record_error}"
                );
            }
        }
    }
}

fn projection_claim_recompute_subjects(
    entity_type: &str,
    entity_id: &str,
    intel: &crate::intelligence::IntelligenceJson,
    withdrawn_subjects: &[(String, String)],
) -> BTreeSet<(String, String)> {
    let mut subjects = BTreeSet::new();
    subjects.insert((entity_type.to_string(), entity_id.to_string()));
    for insight in &intel.stakeholder_insights {
        if let Some(person_id) = insight.person_id.as_deref() {
            if !person_id.trim().is_empty() {
                subjects.insert(("person".to_string(), person_id.to_string()));
            }
        }
    }
    for (subject_type, subject_id) in withdrawn_subjects {
        if !subject_type.trim().is_empty() && !subject_id.trim().is_empty() {
            subjects.insert((subject_type.clone(), subject_id.clone()));
        }
    }
    subjects
}

struct RefreshedProjectionWithdrawal {
    affected_subjects: Vec<(String, String)>,
}

fn withdraw_refreshed_projection_claims(
    ctx: &ServiceContext<'_>,
    tx: &ActionDb,
    entity_type: &str,
    entity_id: &str,
    projection_intel: &crate::intelligence::IntelligenceJson,
    projection_data_source: &str,
) -> Result<RefreshedProjectionWithdrawal, String> {
    let roots = refreshed_projection_field_path_roots(projection_intel, projection_data_source);
    let producers = refreshed_projection_producers(projection_data_source);
    if roots.is_empty() || producers.is_empty() {
        return Ok(RefreshedProjectionWithdrawal {
            affected_subjects: Vec::new(),
        });
    }
    let retained_claim_keys = projection_claim_retained_keys(projection_intel)?;

    let withdrawal = crate::services::claims::withdraw_generated_projection_claims_for_field_path_roots_by_projection_producer_in_tx(
        ctx,
        tx,
        crate::services::claims::GeneratedProjectionRefreshWithdrawal {
            entity_type,
            entity_id,
            field_path_roots: &roots,
            projection_producers: producers,
            retained_claim_keys: &retained_claim_keys,
            retraction_reason: "projection_refreshed",
        },
    )
    .map_err(|error| format!("withdraw refreshed projection claims failed: {error}"))?;
    if withdrawal.withdrawn > 0 {
        log::info!(
            "intelligence: withdrew {} stale generated projection claim(s) after refreshed projection on {entity_type}:{entity_id}",
            withdrawal.withdrawn
        );
    }
    Ok(RefreshedProjectionWithdrawal {
        affected_subjects: withdrawal.affected_subjects,
    })
}

fn refreshed_projection_producers(projection_data_source: &str) -> &'static [&'static str] {
    match projection_data_source.trim() {
        "glean" => &["glean"],
        "ai_enrichment" => &["ai_enrichment"],
        _ => &[],
    }
}

fn refreshed_projection_field_path_roots(
    intel: &crate::intelligence::IntelligenceJson,
    projection_data_source: &str,
) -> Vec<&'static str> {
    if projection_data_source.trim() != "glean" {
        return all_projection_field_path_roots();
    }

    if !intel.refreshed_fields.is_empty() {
        return refreshed_projection_field_path_roots_from_fields(&intel.refreshed_fields);
    }

    let mut roots = BTreeSet::new();
    if intel.executive_assessment.is_some() {
        roots.insert("executiveAssessment");
    }
    if intel.pull_quote.is_some() {
        roots.insert("pullQuote");
    }
    if intel.health.is_some() {
        roots.insert("health");
    }
    if !intel.risks.is_empty() {
        roots.insert("risks");
    }
    if !intel.recommended_actions.is_empty() {
        roots.insert("recommendedActions");
    }
    if !intel.recent_wins.is_empty() {
        roots.insert("recentWins");
    }
    if intel.current_state.is_some() {
        roots.insert("currentState");
    }
    if !intel.strategic_priorities.is_empty() {
        roots.insert("strategicPriorities");
    }
    if !intel.blockers.is_empty() {
        roots.insert("blockers");
    }
    if intel.contract_context.is_some() {
        roots.insert("contractContext");
    }
    if !intel.expansion_signals.is_empty() {
        roots.insert("expansionSignals");
    }
    if intel.agreement_outlook.is_some() {
        roots.insert("agreementOutlook");
    }
    if !intel.value_delivered.is_empty() {
        roots.insert("valueDelivered");
    }
    if intel.success_metrics.is_some() {
        roots.insert("successMetrics");
    }
    if intel.open_commitments.is_some() {
        roots.insert("openCommitments");
    }
    if !intel.stakeholder_insights.is_empty() {
        roots.insert("stakeholderInsights");
    }
    if intel.company_context.is_some() {
        roots.insert("companyContext");
    }

    roots.into_iter().collect()
}

fn refreshed_projection_field_path_roots_from_fields(fields: &[String]) -> Vec<&'static str> {
    let projection_roots = all_projection_field_path_roots();
    let mut roots = BTreeSet::new();
    for field in fields {
        if let Some(root) = projection_roots
            .iter()
            .copied()
            .find(|root| field_path_matches_projection_root(field, root))
        {
            roots.insert(root);
        }
    }
    roots.into_iter().collect()
}

fn field_path_matches_projection_root(field_path: &str, root: &str) -> bool {
    field_path == root
        || field_path
            .strip_prefix(root)
            .is_some_and(|suffix| suffix.starts_with('[') || suffix.starts_with('.'))
}

fn all_projection_field_path_roots() -> Vec<&'static str> {
    vec![
        "executiveAssessment",
        "pullQuote",
        "health",
        "risks",
        "recommendedActions",
        "recentWins",
        "currentState",
        "strategicPriorities",
        "blockers",
        "contractContext",
        "expansionSignals",
        "agreementOutlook",
        "valueDelivered",
        "successMetrics",
        "openCommitments",
        "stakeholderInsights",
        "companyContext",
    ]
}

fn projection_claim_retained_keys(
    intel: &crate::intelligence::IntelligenceJson,
) -> Result<Vec<(String, String, String, String)>, String> {
    let mut keys = Vec::new();
    let subject_ref = subject_ref_for_entity(&intel.entity_type, &intel.entity_id)?;

    if let Some(summary) = intel.executive_assessment.as_deref() {
        push_projection_claim_key(
            &mut keys,
            &subject_ref,
            "entity_summary",
            "executiveAssessment",
            summary,
        );
    }
    if let Some(pull_quote) = intel.pull_quote.as_deref() {
        push_projection_claim_key(
            &mut keys,
            &subject_ref,
            "entity_summary",
            "pullQuote",
            pull_quote,
        );
    }
    if let Some(health) = intel.health.as_ref() {
        if let Some(text) = health_projection_text(health) {
            push_projection_claim_key(
                &mut keys,
                &subject_ref,
                "entity_current_state",
                "health",
                &text,
            );
        }
        for (idx, action) in health.recommended_actions.iter().enumerate() {
            push_projection_claim_key(
                &mut keys,
                &subject_ref,
                "recommendation",
                &format!("health.recommendedActions[{idx}]"),
                action,
            );
        }
    }
    for (idx, risk) in intel.risks.iter().enumerate() {
        push_projection_claim_key(
            &mut keys,
            &subject_ref,
            "entity_risk",
            &format!("risks[{idx}]"),
            &risk.text,
        );
    }
    for (idx, action) in intel.recommended_actions.iter().enumerate() {
        if let Some(text) = recommended_action_projection_text(action) {
            push_projection_claim_key(
                &mut keys,
                &subject_ref,
                "recommendation",
                &format!("recommendedActions[{idx}]"),
                &text,
            );
        }
    }
    for (idx, win) in intel.recent_wins.iter().enumerate() {
        push_projection_claim_key(
            &mut keys,
            &subject_ref,
            "entity_win",
            &format!("recentWins[{idx}]"),
            &win.text,
        );
    }
    if let Some(state) = intel.current_state.as_ref() {
        if let Some(text) = current_state_projection_text(state) {
            push_projection_claim_key(
                &mut keys,
                &subject_ref,
                "entity_current_state",
                "currentState",
                &text,
            );
        }
    }
    for (idx, priority) in intel.strategic_priorities.iter().enumerate() {
        if let Some(text) = strategic_priority_projection_text(priority) {
            push_projection_claim_key(
                &mut keys,
                &subject_ref,
                "entity_current_state",
                &format!("strategicPriorities[{idx}]"),
                &text,
            );
        }
    }
    for (idx, blocker) in intel.blockers.iter().enumerate() {
        if let Some(text) = blocker_projection_text(blocker) {
            push_projection_claim_key(
                &mut keys,
                &subject_ref,
                "entity_risk",
                &format!("blockers[{idx}]"),
                &text,
            );
        }
    }
    if let Some(context) = intel.contract_context.as_ref() {
        if let Some(text) = contract_context_projection_text(context) {
            let claim_type = if intel.entity_type == "account" {
                "company_context"
            } else {
                "entity_current_state"
            };
            push_projection_claim_key(
                &mut keys,
                &subject_ref,
                claim_type,
                "contractContext",
                &text,
            );
        }
    }
    for (idx, signal) in intel.expansion_signals.iter().enumerate() {
        if let Some(text) = expansion_signal_projection_text(signal) {
            push_projection_claim_key(
                &mut keys,
                &subject_ref,
                "entity_current_state",
                &format!("expansionSignals[{idx}]"),
                &text,
            );
        }
    }
    if let Some(outlook) = intel.agreement_outlook.as_ref() {
        if let Some(text) = agreement_outlook_projection_text(outlook) {
            push_projection_claim_key(
                &mut keys,
                &subject_ref,
                "entity_current_state",
                "agreementOutlook",
                &text,
            );
        }
    }
    for (idx, value) in intel.value_delivered.iter().enumerate() {
        push_projection_claim_key(
            &mut keys,
            &subject_ref,
            "value_delivered",
            &format!("valueDelivered[{idx}]"),
            &value.statement,
        );
    }
    if let Some(metrics) = intel.success_metrics.as_ref() {
        for (idx, metric) in metrics.iter().enumerate() {
            if let Some(text) = success_metric_projection_text(metric) {
                push_projection_claim_key(
                    &mut keys,
                    &subject_ref,
                    "entity_current_state",
                    &format!("successMetrics[{idx}]"),
                    &text,
                );
            }
        }
    }
    if let Some(commitments) = intel.open_commitments.as_ref() {
        for (idx, commitment) in commitments.iter().enumerate() {
            if let Some(text) = open_commitment_projection_text(commitment) {
                let claim_type = if intel.entity_type == "account" {
                    "commitment"
                } else {
                    "entity_current_state"
                };
                push_projection_claim_key(
                    &mut keys,
                    &subject_ref,
                    claim_type,
                    &format!("openCommitments[{idx}]"),
                    &text,
                );
            }
        }
    }
    for (idx, insight) in intel.stakeholder_insights.iter().enumerate() {
        if let Some(person_id) = insight.person_id.as_deref() {
            if let Some(text) = stakeholder_engagement_projection_text(insight) {
                let person_subject_ref = subject_ref_for_entity("person", person_id)?;
                push_projection_claim_key(
                    &mut keys,
                    &person_subject_ref,
                    "stakeholder_engagement",
                    &format!("stakeholderInsights[{idx}].engagement"),
                    &text,
                );
            }
        }
    }
    if intel.entity_type == "account" {
        if let Some(context) = intel.company_context.as_ref() {
            if let Some(text) = company_context_projection_text(context) {
                push_projection_claim_key(
                    &mut keys,
                    &subject_ref,
                    "company_context",
                    "companyContext",
                    &text,
                );
            }
        }
    }

    Ok(keys)
}

fn push_projection_claim_key(
    keys: &mut Vec<(String, String, String, String)>,
    subject_ref: &str,
    claim_type: &str,
    field_path: &str,
    text: &str,
) {
    if text.trim().is_empty() {
        return;
    }
    keys.push((
        subject_ref.to_string(),
        claim_type.to_string(),
        field_path.to_string(),
        crate::services::claims::normalize_claim_text(text),
    ));
}

fn withdraw_cleared_dimension_projection_claims(
    ctx: &ServiceContext<'_>,
    tx: &ActionDb,
    entity_type: &str,
    entity_id: &str,
    cleared_dimensions: &[&'static str],
) -> Result<usize, String> {
    let field_path_roots = projection_field_path_roots_for_cleared_dimensions(cleared_dimensions);
    if field_path_roots.is_empty() {
        return Ok(0);
    }

    let withdrawn =
        crate::services::claims::withdraw_generated_projection_claims_for_field_path_roots_in_tx(
            ctx,
            tx,
            entity_type,
            entity_id,
            &field_path_roots,
            "dimension_not_applicable",
        )
        .map_err(|error| format!("withdraw cleared-dimension projection claims failed: {error}"))?;
    if withdrawn > 0 {
        log::info!(
            "intelligence: withdrew {withdrawn} generated projection claim(s) for cleared dimensions on {entity_type}:{entity_id}"
        );
    }
    Ok(withdrawn)
}

fn projection_field_path_roots_for_cleared_dimensions(
    cleared_dimensions: &[&'static str],
) -> Vec<&'static str> {
    let mut roots = BTreeSet::new();
    for dimension in cleared_dimensions {
        let dimension_roots: &[&'static str] = match *dimension {
            "core_assessment" => &[
                "executiveAssessment",
                "pullQuote",
                "currentState",
                "risks",
                "recentWins",
            ],
            "stakeholder_champion" => &[
                "stakeholderInsights",
                "coverageAssessment",
                "organizationalChanges",
                "internalTeam",
                "relationshipDepth",
            ],
            "commercial_financial" => &[
                "health",
                "contractContext",
                "agreementOutlook",
                "expansionSignals",
                "blockers",
                "productClassification",
            ],
            "strategic_context" => &[
                "companyContext",
                "competitiveContext",
                "strategicPriorities",
                "marketContext",
                "regulatoryContext",
            ],
            "value_success" => &[
                "valueDelivered",
                "successMetrics",
                "successPlanSignals",
                "openCommitments",
            ],
            "engagement_signals" => &[
                "meetingCadence",
                "emailResponsiveness",
                "productAdoption",
                "supportHealth",
                "gongCallSummaries",
                "npsCsat",
            ],
            crate::intelligence::dimension_prompts::ACCOUNT_ONLY_ENGAGEMENT_FIELDS_CLEAR => &[
                "productAdoption",
                "supportHealth",
                "gongCallSummaries",
                "npsCsat",
            ],
            _ => &[],
        };
        roots.extend(dimension_roots.iter().copied());
    }
    roots.into_iter().collect()
}

pub fn purge_glean_generated_projection_claims_for_source_purge(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
) -> Result<GeneratedProjectionSourcePurgeReport, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let subjects = load_glean_generated_projection_claim_subjects(db)?;
    let claims_withdrawn =
        crate::services::claims::withdraw_glean_generated_projection_claims_for_source_purge_in_tx(
            ctx, db,
        )
        .map_err(|e| format!("withdraw Glean generated projection claims failed: {e}"))?;

    let mut recompute_jobs_enqueued = 0usize;
    if !subjects.is_empty() {
        for (subject_type, subject_id) in subjects {
            match enqueue_generated_projection_claim_recompute_in_tx(
                ctx,
                db,
                &subject_type,
                &subject_id,
                "glean_source_purge",
            ) {
                Ok(true) => recompute_jobs_enqueued += 1,
                Ok(false) => {}
                Err(error) => {
                    record_generated_projection_recompute_failure(
                        ctx,
                        db,
                        &subject_type,
                        &subject_id,
                        &error,
                    );
                    return Err(error);
                }
            }
        }
    }

    Ok(GeneratedProjectionSourcePurgeReport {
        claims_withdrawn,
        recompute_jobs_enqueued,
    })
}

fn load_glean_generated_projection_claim_subjects(
    db: &ActionDb,
) -> Result<BTreeSet<(String, String)>, String> {
    let mut stmt = db
        .conn_ref()
        .prepare(
            "SELECT DISTINCT lower(json_extract(subject_ref, '$.kind')),
                            json_extract(subject_ref, '$.id')
               FROM intelligence_claims
              WHERE claim_state IN ('active', 'tombstoned', 'dormant')
                AND json_valid(subject_ref) = 1
                AND (
                    data_source = 'glean'
                    OR data_source LIKE 'glean_%'
                    OR (
                        metadata_json IS NOT NULL
                        AND json_valid(metadata_json) = 1
                        AND json_extract(metadata_json, '$.projection_producer') = 'glean'
                        AND data_source IN ('ai', 'ai_enrichment', 'ai_inference')
                    )
                    OR (
                        EXISTS (
                            SELECT 1
                              FROM claim_corroborations cc
                             WHERE cc.claim_id = intelligence_claims.id
                               AND (
                                   cc.data_source = 'glean'
                                   OR cc.data_source LIKE 'glean_%'
                               )
                        )
                    )
                )
                AND (
                    source_ref LIKE 'intelligence_projection_source:%'
                    OR (
                        json_valid(provenance_json) = 1
                        AND json_extract(provenance_json, '$.ability_name') = 'claim_shaped_intelligence_projection'
                    )
                    OR (
                        metadata_json IS NOT NULL
                        AND json_valid(metadata_json) = 1
                        AND json_type(metadata_json, '$.legacy_projection_value') IS NOT NULL
                    )
                )",
        )
        .map_err(|e| format!("prepare Glean generated projection subject scan failed: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| format!("query Glean generated projection subjects failed: {e}"))?;
    rows.collect::<Result<BTreeSet<_>, _>>()
        .map_err(|e| format!("collect Glean generated projection subjects failed: {e}"))
}

fn enqueue_generated_projection_claim_recompute_in_tx(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    subject_type: &str,
    subject_id: &str,
    reason: &str,
) -> Result<bool, String> {
    let source_claim_version = db
        .current_subject_claim_version(subject_type, subject_id)
        .map_err(|error| {
            format!("read {subject_type}:{subject_id} claim version failed: {error}")
        })?;
    let payload = serde_json::json!({
        "reason": reason,
        "source_claim_version": source_claim_version,
    })
    .to_string();
    let outcome = crate::services::signals::emit_once_for_key(
        ctx,
        db,
        &format!(
            "generated_projection_claims:{reason}:{subject_type}:{subject_id}:{source_claim_version}"
        ),
        subject_type,
        subject_id,
        "generated_projection_claims_updated",
        "claim_projection",
        Some(&payload),
        0.8,
    )
    .map_err(|error| format!("signal emit failed: {error}"))?;
    if outcome.coalesced {
        return Ok(false);
    }

    crate::services::invalidation_jobs::enqueue_signal_claim_recompute_in_tx(
        db,
        &outcome.id,
        subject_type,
        subject_id,
    )?;
    Ok(true)
}

fn record_generated_projection_recompute_failure(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    subject_type: &str,
    subject_id: &str,
    error: &str,
) {
    if let Err(record_error) = crate::services::mutations::record_pipeline_failure(
        ctx,
        db,
        "generated_projection_claims",
        Some(subject_id),
        Some(subject_type),
        "source_purge_recompute_enqueue_failed",
        Some(error),
        0,
    ) {
        log::warn!(
            "generated_projection_claims: failed to record recompute enqueue failure for {subject_type}:{subject_id}: {record_error}"
        );
    }
}

/// Persist an assessment snapshot without emitting enrichment lifecycle signals.
///
/// Progressive-write paths use this helper so the final authoritative write can
/// remain the single point for signal emission and downstream invalidation.
pub fn upsert_assessment_snapshot(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    intel: &crate::intelligence::IntelligenceJson,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    db.with_transaction(|tx| {
        commit_claim_shaped_intelligence_projection(
            ctx,
            tx,
            intel,
            "agent:intel_queue",
            "ai_enrichment_progressive",
        )?;
        crate::services::derived_state::upsert_entity_intelligence_legacy_snapshot(ctx, tx, intel)
            .map_err(|e| e.to_string())?;

        // Path 2c: Store domains from Glean enrichment (if present).
        // When Glean enrichment populates intel.domains (extracted from stakeholder emails),
        // persist them to account_domains for entity resolution.
        // Only applies to account entities.
        if intel.entity_type == "account" && !intel.domains.is_empty() {
            tx.merge_account_domains_enrichment(&intel.entity_id, &intel.domains)
                .map_err(|e| {
                    format!(
                        "Failed to store domains for account {}: {}",
                        intel.entity_id, e
                    )
                })?;
            log::debug!(
                "Intelligence service: stored {} domains for account '{}'",
                intel.domains.len(),
                intel.entity_id
            );
        }

        Ok(())
    })?;

    Ok(())
}

/// Persist a partial enrichment snapshot for UI progress refreshes only.
///
/// Progressive dimension updates are not authoritative evidence. Keep them out
/// of claim projection, feedback cleanup, and side-effect signal paths so a
/// slow multi-dimension refresh cannot create write amplification while the
/// final enrichment commit is still pending.
pub fn upsert_progressive_assessment_snapshot(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    intel: &crate::intelligence::IntelligenceJson,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    db.with_transaction(|tx| {
        crate::services::derived_state::upsert_entity_intelligence_progressive_snapshot(tx, intel)
            .map_err(|e| e.to_string())?;
        Ok(())
    })
}

/// Persist the Glean leading-signals JSON blob on `entity_assessment`
/// and emit the four callout-worthy signals derived from it.
///
/// Wrapped in a transaction so the blob write and signal emissions either all
/// land or all roll back. Source is tagged `glean_leading_signals` at confidence
/// 0.8 (champion_at_risk), 0.75 (competitor_decision_relevant), 0.7
/// (sentiment_divergence), 0.75 (budget_cycle_locked) — matching the tier policy
/// of other Glean-derived signals registered in `signals/callouts.rs`.
pub fn upsert_health_outlook_signals(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    engine: &PropagationEngine,
    entity_type: &str,
    entity_id: &str,
    signals: &crate::intelligence::glean_leading_signals::HealthOutlookSignals,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let blob = serde_json::to_string(signals)
        .map_err(|e| format!("Failed to serialize health_outlook_signals: {e}"))?;

    db.with_transaction(|tx| {
        crate::services::derived_state::upsert_health_outlook_signals_legacy_projection(
            tx,
            entity_id,
            entity_type,
            &blob,
        )
        .map_err(|e| format!("Failed to upsert health_outlook_signals_json: {e}"))?;

        let derived = signals.derive_signals();

        if let Some(payload) = derived.champion_at_risk {
            crate::services::signals::emit_and_propagate(
                ctx,
                tx,
                engine,
                entity_type,
                entity_id,
                "champion_at_risk",
                "glean_leading_signals",
                Some(&payload),
                0.8,
            )
            .map_err(|e| format!("champion_at_risk emit failed: {e}"))?;
        }

        if let Some(payload) = derived.sentiment_divergence {
            crate::services::signals::emit_and_propagate(
                ctx,
                tx,
                engine,
                entity_type,
                entity_id,
                "sentiment_divergence",
                "glean_leading_signals",
                Some(&payload),
                0.7,
            )
            .map_err(|e| format!("sentiment_divergence emit failed: {e}"))?;
        }

        for payload in derived.competitor_decision_relevant {
            crate::services::signals::emit_and_propagate(
                ctx,
                tx,
                engine,
                entity_type,
                entity_id,
                "competitor_decision_relevant",
                "glean_leading_signals",
                Some(&payload),
                0.75,
            )
            .map_err(|e| format!("competitor_decision_relevant emit failed: {e}"))?;
        }

        if let Some(payload) = derived.budget_cycle_locked {
            crate::services::signals::emit_and_propagate(
                ctx,
                tx,
                engine,
                entity_type,
                entity_id,
                "budget_cycle_locked",
                "glean_leading_signals",
                Some(&payload),
                0.75,
            )
            .map_err(|e| format!("budget_cycle_locked emit failed: {e}"))?;
        }

        Ok(())
    })
}

/// Persist AI-inferred person relationships for an enrichment run.
///
/// - Skips invalid/self edges.
/// - Never overwrites strong user-confirmed edges.
/// - Uses deterministic IDs so re-enrichment reinforces instead of duplicating.
/// - Emits `relationship_inferred` only when creating a new AI edge.
pub fn upsert_inferred_relationships_from_enrichment(
    ctx: &ServiceContext<'_>,
    db: &ActionDb,
    engine: &PropagationEngine,
    entity_type: &str,
    entity_id: &str,
    inferred: &[crate::intelligence::prompts::InferredRelationship],
) -> Result<usize, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    if inferred.is_empty() {
        return Ok(0);
    }

    db.with_transaction(|tx| {
        let mut inserted = 0usize;

        for rel in inferred {
            if rel.from_person_id.trim().is_empty()
                || rel.to_person_id.trim().is_empty()
                || rel.from_person_id == rel.to_person_id
            {
                continue;
            }

            if rel
                .relationship_type
                .parse::<crate::db::person_relationships::RelationshipType>()
                .is_err()
            {
                log::warn!(
                    "intelligence service: skipping invalid inferred relationship type '{}'",
                    rel.relationship_type
                );
                continue;
            }

            let direction = if rel.relationship_type == "manager" {
                "directed"
            } else {
                "symmetric"
            };
            let mut from_person_id = rel.from_person_id.clone();
            let mut to_person_id = rel.to_person_id.clone();
            if direction == "symmetric" && from_person_id > to_person_id {
                std::mem::swap(&mut from_person_id, &mut to_person_id);
            }

            let existing = tx
                .get_relationships_between(&from_person_id, &to_person_id)
                .map_err(|e| format!("relationship lookup failed: {e}"))?;
            if existing
                .iter()
                .any(|r| r.source == "user_confirmed" && r.confidence >= 0.8)
            {
                continue;
            }

            let existing_ai = existing.iter().find(|r| r.source == "ai_enrichment");
            let relationship_id = existing_ai
                .map(|r| r.id.clone())
                .unwrap_or_else(|| format!("pr-ai-{from_person_id}-{to_person_id}"));

            tx.upsert_person_relationship(&crate::db::person_relationships::UpsertRelationship {
                id: &relationship_id,
                from_person_id: &from_person_id,
                to_person_id: &to_person_id,
                relationship_type: &rel.relationship_type,
                direction,
                confidence: 0.6,
                context_entity_id: Some(entity_id),
                context_entity_type: Some(entity_type),
                source: "ai_enrichment",
                rationale: rel.rationale.as_deref(),
            })
            .map_err(|e| format!("relationship upsert failed: {e}"))?;

            if existing_ai.is_none() {
                let signal_value = format!(
                    "{from_person_id} -> {to_person_id} ({})",
                    rel.relationship_type
                );
                crate::services::signals::emit_and_propagate(
                    ctx,
                    tx,
                    engine,
                    entity_type,
                    entity_id,
                    "relationship_inferred",
                    "ai_enrichment",
                    Some(signal_value.as_str()),
                    0.6,
                )
                .map_err(|e| format!("relationship_inferred signal failed: {e}"))?;
                inserted += 1;
            }
        }

        Ok(inserted)
    })
}

/// Update a single field in an entity's intelligence.json with signal emission.
pub async fn update_intelligence_field(
    ctx: &ServiceContext<'_>,
    entity_id: &str,
    entity_type: &str,
    field_path: &str,
    value: &str,
    state: &AppState,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let config = state.config.read().clone();
    let config = config.ok_or("No configuration loaded")?;
    let workspace_path = config.workspace_path.clone();

    let entity_id = entity_id.to_string();
    let entity_type = entity_type.to_string();
    let field_path = field_path.to_string();
    let value = value.to_string();
    state
        .db_write(move |db| {
            let workspace = Path::new(&workspace_path);

            let account = if entity_type == "account" {
                db.get_account(&entity_id).map_err(|e| e.to_string())?
            } else {
                None
            };

            let entity_name = match entity_type.as_str() {
                "account" => account.as_ref().map(|a| a.name.clone()),
                "project" => db
                    .get_project(&entity_id)
                    .map_err(|e| e.to_string())?
                    .map(|p| p.name),
                "person" => db
                    .get_person(&entity_id)
                    .map_err(|e| e.to_string())?
                    .map(|p| p.name),
                _ => return Err(format!("Unsupported entity type: {}", entity_type)),
            }
            .ok_or_else(|| format!("{} '{}' not found", entity_type, entity_id))?;

            let dir = crate::intelligence::resolve_entity_dir(
                workspace,
                &entity_type,
                &entity_name,
                account.as_ref(),
            )?;

            // DB is sole source of truth — no filesystem fallback.
            // propagate DB read errors instead of collapsing them into "no row".
            let existing_intel = db
                .get_entity_intelligence(&entity_id)
                .map_err(|e| format!("DB read failed for entity {entity_id}: {e}"))?;
            let intel = match existing_intel {
                Some(existing) => crate::intelligence::apply_intelligence_field_update_in_memory(
                    existing,
                    &field_path,
                    &value,
                )?,
                None => {
                    return Err(format!(
                        "I644: no DB intelligence row for {} — cannot update field",
                        entity_id
                    ));
                }
            };

            // Distinguish curation (delete/clear) from correction (edit).
            // Empty value = user removed the item → curation, no source penalty.
            // Non-empty value = user corrected the item → correction, source penalized.
            let is_curation = value.trim().is_empty() || value == "[]" || value == "null";

            // DB-first ordering. Commit canonical state first; the
            // legacy file cache is written AFTER commit as best-effort.
            db.with_transaction(|tx| {
                tx.upsert_entity_intelligence(&intel)
                    .map_err(|e| e.to_string())?;
                let clock = crate::services::context::SystemClock;
                let rng = crate::services::context::SystemRng;
                let ext = crate::services::context::ExternalClients::default();
                let ctx = crate::services::context::ServiceContext::new_live(&clock, &rng, &ext);
                let (signal_type, source, confidence) = if is_curation {
                    ("intelligence_curated", "user_curation", 0.5)
                } else {
                    ("user_correction", "user_edit", 1.0)
                };
                crate::services::signals::emit(
                    &ctx,
                    tx,
                    &entity_type,
                    &entity_id,
                    signal_type,
                    source,
                    Some(&format!("{{\"field\":\"{}\"}}", field_path)),
                    confidence,
                )
                .map_err(|e| format!("signal emit failed: {e}"))?;
                Ok(())
            })?;

            // Post-commit file write — best-effort cache. DB is canonical from here.
            // routed through the schema-epoch fence so a concurrent
            // migration can preempt stale cache writes.
            crate::intelligence::write_fence::post_commit_fenced_write(
                db,
                &dir,
                &intel,
                &format!("entity={entity_id} field={field_path}"),
            );

            // Self-healing: only record correction (not curation) to lower quality score
            if !is_curation {
                crate::self_healing::feedback::record_enrichment_correction(
                    db,
                    &entity_id,
                    &entity_type,
                    "intel_queue",
                );
            }

            Ok(())
        })
        .await
        .map_err(String::from)
}

/// Bulk-replace the stakeholder list in an entity's intelligence.json.
pub async fn update_stakeholders(
    ctx: &ServiceContext<'_>,
    entity_id: &str,
    entity_type: &str,
    stakeholders: Vec<crate::intelligence::StakeholderInsight>,
    state: &AppState,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let config = state.config.read().clone();
    let config = config.ok_or("No configuration loaded")?;
    let workspace_path = config.workspace_path.clone();
    let active_preset = state.active_preset.read().clone();

    let engine = state.signals.engine.clone();
    let entity_id = entity_id.to_string();
    let entity_type = entity_type.to_string();
    let sourced_at = ctx.clock.now().to_rfc3339();
    let stakeholders = stakeholders
        .into_iter()
        .map(|mut stakeholder| {
            stakeholder.source = Some("user".to_string());
            stakeholder.item_source = Some(crate::intelligence::ItemSource {
                source: "user_correction".to_string(),
                confidence: 1.0,
                sourced_at: sourced_at.clone(),
                reference: Some("user stakeholder edit".to_string()),
            });
            stakeholder
        })
        .collect::<Vec<_>>();

    state
        .db_write(move |db| {
            let workspace = Path::new(&workspace_path);

            let account = if entity_type == "account" {
                db.get_account(&entity_id).map_err(|e| e.to_string())?
            } else {
                None
            };

            let entity_name = match entity_type.as_str() {
                "account" => account.as_ref().map(|a| a.name.clone()),
                "project" => db
                    .get_project(&entity_id)
                    .map_err(|e| e.to_string())?
                    .map(|p| p.name),
                "person" => db
                    .get_person(&entity_id)
                    .map_err(|e| e.to_string())?
                    .map(|p| p.name),
                _ => return Err(format!("Unsupported entity type: {}", entity_type)),
            }
            .ok_or_else(|| format!("{} '{}' not found", entity_type, entity_id))?;

            let dir = crate::intelligence::resolve_entity_dir(
                workspace,
                &entity_type,
                &entity_name,
                account.as_ref(),
            )?;

            // Capture linked stakeholders with scoring-relevant roles before
            // the vec is consumed by the in-memory intelligence update.
            let scoring_roles: Vec<(String, String)> = if entity_type == "account" {
                stakeholders
                    .iter()
                    .filter_map(|s| {
                        let role = s.role.as_deref().unwrap_or("").to_lowercase();
                        let engagement = s.engagement.as_deref().unwrap_or("").to_lowercase();
                        let person_id = s.person_id.as_deref()?;
                        // Check BOTH role and engagement — user may set champion
                        // via either the Team panel (role) or EngagementSelector (engagement)
                        let effective = if !engagement.is_empty() {
                            &engagement
                        } else {
                            &role
                        };
                        if effective.contains("champion")
                            || effective.contains("executive")
                            || effective.contains("technical")
                            || effective.contains("decision")
                        {
                            Some((person_id.to_string(), effective.to_string()))
                        } else {
                            None
                        }
                    })
                    .collect()
            } else {
                Vec::new()
            };

            // DB-first: generated intelligence.json is an export projection,
            // not a fallback authority. If this entity has no intelligence row
            // yet, compose a minimal DB snapshot and let the post-commit
            // projection writer create/refresh the file from canonical state.
            let existing_intel = db
                .get_entity_intelligence(&entity_id)
                .map_err(|e| format!("DB read failed for entity {entity_id}: {e}"))?;
            let base_intel = existing_intel.unwrap_or_else(|| {
                blank_entity_intelligence_snapshot(&entity_id, &entity_type, &sourced_at)
            });
            let intel =
                crate::intelligence::apply_stakeholders_update_in_memory(base_intel, stakeholders)?;

            // DB-first ordering. The legacy file cache is written AFTER
            // the transaction commits.
            db.with_transaction(|tx| {
                tx.upsert_entity_intelligence(&intel)
                    .map_err(|e| e.to_string())?;

                let clock = crate::services::context::SystemClock;
                let rng = crate::services::context::SystemRng;
                let ext = crate::services::context::ExternalClients::default();
                let ctx = crate::services::context::ServiceContext::new_live(&clock, &rng, &ext);

                // Sync scoring-relevant stakeholder roles to account_stakeholders
                // so health scoring (champion health, stakeholder coverage) picks them up.
                // Errors propagate to roll back the entire enrichment write — a failed
                // stakeholder cache rebuild signal must not leave account_stakeholders
                // partially updated. The B2 contract requires atomicity between the
                // membership write and the cache invalidation.
                for (person_id, role) in &scoring_roles {
                    crate::services::accounts::add_team_member_with_cache_rebuild(
                        &ctx, tx, &entity_id, person_id, role,
                    )?;
                }

                // Recompute health immediately so stakeholder changes reflect
                // in key_advocate_health + stakeholder_coverage dimensions without
                // waiting for a full enrichment cycle.
                if entity_type == "account" && !scoring_roles.is_empty() {
                    if let Some(acct) = account.as_ref() {
                        let health =
                            crate::intelligence::health_scoring::compute_account_health_with_preset(
                                tx,
                                acct,
                                intel.org_health.as_ref(),
                                active_preset.as_ref(),
                            );
                        crate::services::derived_state::upsert_entity_health_legacy_projection(
                            tx, &entity_id, "account", &health,
                        )
                        .ok();
                    }
                }

                crate::services::signals::emit_and_propagate(
                    &ctx,
                    tx,
                    &engine,
                    &entity_type,
                    &entity_id,
                    "stakeholders_updated",
                    "user_edit",
                    None,
                    0.9,
                )
                .map_err(|e| format!("signal emit failed: {e}"))?;
                Ok(())
            })?;

            // Post-commit file write — best-effort cache. DB is canonical from here.
            // routed through the schema-epoch fence.
            crate::intelligence::write_fence::post_commit_fenced_write(
                db,
                &dir,
                &intel,
                &format!("entity={entity_id}"),
            );

            Ok(())
        })
        .await
        .map_err(String::from)
}

/// Dismiss an intelligence item, creating a tombstone to prevent re-creation.
///
/// Removes the item from the specified Vec field and adds a `DismissedItem`
/// tombstone that prevents future enrichment from re-creating it.
pub async fn dismiss_intelligence_item(
    ctx: &ServiceContext<'_>,
    entity_id: &str,
    entity_type: &str,
    field: &str,
    item_text: &str,
    state: &AppState,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let config = state.config.read().clone();
    let config = config.ok_or("No configuration loaded")?;
    let workspace_path = config.workspace_path.clone();

    let engine = state.signals.engine.clone();
    let entity_id = entity_id.to_string();
    let entity_type = entity_type.to_string();
    let field = field.to_string();
    let item_text = item_text.to_string();
    let dismissed_at = ctx.clock.now().to_rfc3339();
    state
        .db_write(move |db| {
            let workspace = Path::new(&workspace_path);

            let account = if entity_type == "account" {
                db.get_account(&entity_id).map_err(|e| e.to_string())?
            } else {
                None
            };

            let entity_name = match entity_type.as_str() {
                "account" => account.as_ref().map(|a| a.name.clone()),
                "project" => db
                    .get_project(&entity_id)
                    .map_err(|e| e.to_string())?
                    .map(|p| p.name),
                "person" => db
                    .get_person(&entity_id)
                    .map_err(|e| e.to_string())?
                    .map(|p| p.name),
                _ => return Err(format!("Unsupported entity type: {}", entity_type)),
            }
            .ok_or_else(|| format!("{} '{}' not found", entity_type, entity_id))?;

            let dir = crate::intelligence::resolve_entity_dir(
                workspace,
                &entity_type,
                &entity_name,
                account.as_ref(),
            )?;

            // DB is sole source of truth — no filesystem fallback.
            // propagate DB read errors instead of collapsing them into "no row";
            // the previous `.ok.flatten` masked connection failures behind the "no row" message.
            let existing_intel = db
                .get_entity_intelligence(&entity_id)
                .map_err(|e| format!("DB read failed for entity {entity_id}: {e}"))?;
            let mut intel = existing_intel.ok_or_else(|| {
                format!(
                    "I644: no DB intelligence row for {} — cannot dismiss item",
                    entity_id
                )
            })?;

            // Add tombstone
            intel
                .dismissed_items
                .push(crate::intelligence::DismissedItem {
                    field: field.clone(),
                    content: item_text.clone(),
                    dismissed_at: dismissed_at.clone(),
                });

            // Remove item from the relevant Vec by matching text
            let item_lower = item_text.to_lowercase();
            match field.as_str() {
                "risks" => intel
                    .risks
                    .retain(|r| !r.text.to_lowercase().contains(&item_lower)),
                "recentWins" => intel
                    .recent_wins
                    .retain(|w| !w.text.to_lowercase().contains(&item_lower)),
                "stakeholderInsights" => intel
                    .stakeholder_insights
                    .retain(|s| !s.name.to_lowercase().contains(&item_lower)),
                "valueDelivered" => intel
                    .value_delivered
                    .retain(|v| !v.statement.to_lowercase().contains(&item_lower)),
                "competitiveContext" => intel
                    .competitive_context
                    .retain(|c| !c.competitor.to_lowercase().contains(&item_lower)),
                "organizationalChanges" => intel
                    .organizational_changes
                    .retain(|o| !o.person.to_lowercase().contains(&item_lower)),
                "expansionSignals" => intel
                    .expansion_signals
                    .retain(|e| !e.opportunity.to_lowercase().contains(&item_lower)),
                "openCommitments" => {
                    if let Some(ref mut ocs) = intel.open_commitments {
                        ocs.retain(|c| !c.description.to_lowercase().contains(&item_lower));
                    }
                }
                _ => return Err(format!("Cannot dismiss items from field: {}", field)),
            }

            // DB-first ordering. The transaction commits the canonical
            // state; the legacy `intelligence.json` cache is written AFTER commit
            // and treated as best-effort. A file write failure does not roll back
            // DB state — the projection writer will repair file drift on
            // the next claim touch.
            db.with_transaction(|tx| {
                tx.upsert_entity_intelligence(&intel)
                    .map_err(|e| e.to_string())?;

                // Record feedback event + suppression tombstone.
                // propagate errors so a failed insert no longer leaves
                // a ghost-resurrectable item.
                tx.record_feedback_event(&crate::db::feedback::FeedbackEventInput {
                    entity_id: &entity_id,
                    entity_type: &entity_type,
                    field_key: &field,
                    item_key: Some(&item_text),
                    feedback_type: "dismiss",
                    source_system: None,
                    source_kind: Some("intelligence"),
                    previous_value: Some(&item_text),
                    corrected_value: None,
                    reason: None,
                })
                .map_err(|e| format!("record_feedback_event: {e}"))?;
                tx.create_suppression_tombstone(
                    &entity_id,
                    &field,
                    Some(&item_text),
                    crate::intelligence::canonicalization::maybe_item_hash_for_field(
                        &field,
                        Some(&item_text),
                    )
                    .as_deref(),
                    Some("intelligence"),
                    None,
                )
                .map_err(|e| format!("create_suppression_tombstone: {e}"))?;

                // Shadow-write tombstone claim into the new substrate.
                // Failure logged but not propagated; legacy write above remains
                // authoritative until the claim-read gate migration lands.
                let subject_kind = match entity_type.as_str() {
                    "account" => "Account",
                    "person" => "Person",
                    "project" => "Project",
                    "meeting" => "Meeting",
                    _ => "Account",
                };
                let claim_type = match field.as_str() {
                    "risks" => "risk",
                    "recentWins" | "wins" => "win",
                    _ => "intelligence_field_dismissed",
                };
                crate::services::claims::shadow_write_tombstone_claim(
                    tx,
                    crate::services::claims::ShadowTombstoneClaim {
                        subject_kind,
                        subject_id: &entity_id,
                        claim_type,
                        field_path: Some(&field),
                        text: &item_text,
                        actor: "user",
                        source_scope: Some("intelligence"),
                        observed_at: &dismissed_at,
                        expires_at: None,
                    },
                );

                Ok(())
            })?;

            // Post-commit side effects. emit_and_propagate dispatches engine.propagate
            // which can enqueue cross-entity intel work; running it after commit
            // means a downstream propagation failure cannot roll back the user's
            // dismiss intent. DB is the source of truth; emission failures log.
            let clock = crate::services::context::SystemClock;
            let rng = crate::services::context::SystemRng;
            let ext = crate::services::context::ExternalClients::default();
            let ctx = crate::services::context::ServiceContext::new_live(&clock, &rng, &ext);
            if let Err(e) = crate::services::signals::emit_and_propagate(
                &ctx,
                db,
                &engine,
                &entity_type,
                &entity_id,
                "intelligence_curated",
                "user_curation",
                Some(&format!(
                    "{{\"field\":\"{field}\",\"dismissed\":\"{item_text}\"}}",
                )),
                0.5,
            ) {
                log::warn!(
                    "post-commit signal emission failed; \
                     repair_target=signals_engine \
                     entity={entity_id} field={field}: {e}"
                );
            }

            // Post-commit file write — best-effort cache. DB is canonical from here.
            // routed through the schema-epoch fence.
            crate::intelligence::write_fence::post_commit_fenced_write(
                db,
                &dir,
                &intel,
                &format!("entity={entity_id} field={field}"),
            );

            Ok(())
        })
        .await
        .map_err(String::from)
}

/// Recompute health dimensions for an account without full re-enrichment.
///
/// Called when signals arrive that affect health (meetings, emails, stakeholder changes).
/// Updates both the DB (entity_assessment.health_json + entity_quality) and the
/// in-memory IntelligenceJson so downstream surfaces see fresh scores.
pub fn recompute_entity_health(
    ctx: &ServiceContext<'_>,
    db: &crate::db::ActionDb,
    entity_id: &str,
    entity_type: &str,
) -> Result<(), String> {
    recompute_entity_health_with_preset(ctx, db, entity_id, entity_type, None)
}

/// Recompute health dimensions with active preset weights when available.
pub fn recompute_entity_health_with_preset(
    ctx: &ServiceContext<'_>,
    db: &crate::db::ActionDb,
    entity_id: &str,
    entity_type: &str,
    preset: Option<&crate::presets::schema::RolePreset>,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    if entity_type != "account" {
        return Ok(()); // Health scoring is account-only for now
    }

    // Need the DbAccount for lifecycle weights and contract proximity
    let account = match db.get_account(entity_id).map_err(|e| e.to_string())? {
        Some(a) => a,
        None => return Ok(()), // Account not found, nothing to recompute
    };

    // Get existing intelligence
    let intel = match db.get_entity_intelligence(entity_id).ok().flatten() {
        Some(i) => i,
        None => return Ok(()), // No intelligence yet, nothing to recompute
    };

    // Pass org_health from existing intelligence so the 40/60 baseline
    // blend fires consistently (previously passed None, diverging from enrichment scores)
    let org_health_ref = intel.org_health.as_ref();
    let health = crate::intelligence::health_scoring::compute_account_health_with_preset(
        db,
        &account,
        org_health_ref,
        preset,
    );

    // Health is a computed snapshot, not a clean text claim.
    crate::services::derived_state::upsert_entity_health_legacy_projection(
        db,
        entity_id,
        entity_type,
        &health,
    )
    .map_err(|e| e.to_string())?;

    // Note: disk write (for MCP sidecar) is skipped here because we don't have
    // workspace access. The DB is the primary source; disk catches up on next
    // full enrichment cycle.

    log::info!(
        "Health recomputed for {} after signal arrival (score={:.1}, band={})",
        entity_id,
        health.score,
        health.band,
    );

    Ok(())
}

/// Bulk recompute health scores for all accounts.
/// Called once after deploying formula fixes to ensure consistency.
pub fn bulk_recompute_health(db: &crate::db::ActionDb) -> Result<usize, String> {
    let accounts = db.get_all_accounts().map_err(|e| e.to_string())?;
    let mut recomputed = 0;
    let clock = crate::services::context::SystemClock;
    let rng = crate::services::context::SystemRng;
    let ext = crate::services::context::ExternalClients::default();
    let ctx = ServiceContext::new_live(&clock, &rng, &ext);

    for account in &accounts {
        if let Err(e) = recompute_entity_health(&ctx, db, &account.id, "account") {
            log::warn!("Health recompute failed for {}: {}", account.id, e);
            continue;
        }
        recomputed += 1;
    }

    log::info!(
        "Bulk health recompute complete: {}/{} accounts rescored",
        recomputed,
        accounts.len()
    );
    Ok(recomputed)
}

/// Generate a risk briefing for an account (async, PTY enrichment).
pub async fn generate_risk_briefing(
    state: &std::sync::Arc<AppState>,
    account_id: &str,
    app_handle: Option<tauri::AppHandle>,
) -> Result<crate::types::RiskBriefing, String> {
    let app_state = state.clone();
    let account_id = account_id.to_string();
    let progress_handle = app_handle.clone();

    let task = tauri::async_runtime::spawn_blocking(move || {
        let input = {
            let db =
                crate::db::ActionDb::open(std::sync::Arc::new(crate::db::LocalKeychain::new()))
                    .map_err(|e| format!("Database unavailable: {e}"))?;

            let config_guard = app_state.config.read();
            let config = config_guard
                .as_ref()
                .ok_or_else(|| "Config not initialized".to_string())?;

            let workspace = std::path::Path::new(&config.workspace_path);
            crate::risk_briefing::gather_risk_input(
                workspace,
                &db,
                &account_id,
                config.user_name.clone(),
                config.ai_models.clone(),
                &*app_state.context_provider(),
            )?
        };

        let briefing = crate::risk_briefing::run_risk_enrichment(&input, progress_handle.as_ref())?;

        // Store in reports table for unified tracking
        if let Ok(db) =
            crate::db::ActionDb::open(std::sync::Arc::new(crate::db::LocalKeychain::new()))
        {
            #[allow(
                clippy::let_underscore_must_use,
                reason = "intentional best-effort discard; preserves existing non-blocking behavior"
            )]
            let _ =
                crate::reports::risk::store_risk_briefing_in_reports(&db, &account_id, &briefing);
        }

        Ok(briefing)
    });

    match task.await {
        Ok(result) => result,
        Err(e) => Err(format!("Risk briefing task panicked: {}", e)),
    }
}

/// Read a cached risk briefing for an account (fast, no AI).
pub fn get_risk_briefing(
    db: &ActionDb,
    state: &AppState,
    account_id: &str,
) -> Result<crate::types::RiskBriefing, String> {
    // Try reports table first (DB-backed storage)
    if let Some(briefing) = crate::reports::risk::load_risk_briefing_from_reports(db, account_id) {
        return Ok(briefing);
    }

    // Fall back to disk (legacy path)
    let config_guard = state.config.read();
    let config = config_guard.as_ref().ok_or("Config not initialized")?;

    let account = db
        .get_account(account_id)
        .map_err(|e| format!("DB error: {}", e))?
        .ok_or_else(|| format!("Account not found: {}", account_id))?;

    let workspace = std::path::Path::new(&config.workspace_path);
    let account_dir = crate::accounts::resolve_account_dir(workspace, &account);
    crate::risk_briefing::read_risk_briefing(&account_dir)
}

// =============================================================================
// Recommended Action Track / Dismiss
// =============================================================================

/// Track (accept) a recommended action — creates a real action with
/// source_type "intelligence" and emits a recommendation_accepted signal.
pub async fn track_recommendation(
    ctx: &ServiceContext<'_>,
    entity_id: &str,
    entity_type: &str,
    index: usize,
    state: &AppState,
) -> Result<String, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let engine = state.signals.engine.clone();
    let entity_id = entity_id.to_string();
    let entity_type = entity_type.to_string();
    let now = ctx.clock.now().to_rfc3339();

    state
        .db_write(move |db| {
            // Read current intelligence to find the recommendation
            let intel = db
                .get_entity_intelligence(&entity_id)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| format!("No intelligence found for {}", entity_id))?;

            let rec = intel
                .recommended_actions
                .get(index)
                .ok_or_else(|| format!("Recommendation index {} out of bounds", index))?;

            // Create the action
            let id = uuid::Uuid::new_v4().to_string();
            let action = crate::db::DbAction {
                id: id.clone(),
                title: rec.title.clone(),
                priority: rec.priority,
                status: crate::action_status::UNSTARTED.to_string(),
                created_at: now.clone(),
                due_date: rec.suggested_due.clone(),
                completed_at: None,
                account_id: if entity_type == "account" {
                    Some(entity_id.clone())
                } else {
                    None
                },
                project_id: if entity_type == "project" {
                    Some(entity_id.clone())
                } else {
                    None
                },
                source_type: Some("intelligence".to_string()),
                source_id: Some(entity_id.clone()),
                source_label: Some("Based on account intelligence".to_string()),
                action_kind: crate::action_status::KIND_TASK.to_string(),
                commitment_id: None,
                owner_raw: None,
                owner_entity_id: None,
                owner_confidence: None,
                owner_source: None,
                trust_score: None,
                trust_band: None,
                commitment_source_count: None,
                context: Some(rec.rationale.clone()),
                waiting_on: None,
                updated_at: now,
                person_id: if entity_type == "person" {
                    Some(entity_id.clone())
                } else {
                    None
                },
                account_name: None,
                next_meeting_title: None,
                next_meeting_start: None,
                needs_decision: false,
                decision_owner: None,
                decision_stakes: None,
                linear_identifier: None,
                linear_url: None,
            };

            db.upsert_action(&action).map_err(|e| e.to_string())?;

            // Remove the tracked recommendation from intel to prevent duplicates
            let mut updated_intel = intel.clone();
            if index < updated_intel.recommended_actions.len() {
                updated_intel.recommended_actions.remove(index);
                db.upsert_entity_intelligence(&updated_intel)
                    .map_err(|e| e.to_string())?;
            }

            // Emit recommendation_accepted signal
            let clock = crate::services::context::SystemClock;
            let rng = crate::services::context::SystemRng;
            let ext = crate::services::context::ExternalClients::default();
            let ctx = crate::services::context::ServiceContext::new_live(&clock, &rng, &ext);
            if let Err(e) = crate::services::signals::emit_and_propagate(
                &ctx,
                db,
                &engine,
                &entity_type,
                &entity_id,
                "recommendation_accepted",
                "intelligence",
                Some(&format!(
                    "{{\"action_id\":\"{}\",\"title\":\"{}\"}}",
                    id,
                    rec.title.replace('"', "\\\"")
                )),
                0.8,
            ) {
                log::warn!(
                    "emit recommendation accepted signal failed for {entity_type}:{entity_id}: {e}"
                );
            }

            Ok(id)
        })
        .await
        .map_err(String::from)
}

/// Dismiss a recommended action — removes it from intelligence and
/// emits a recommendation_rejected signal (low confidence correction).
pub async fn dismiss_recommendation(
    ctx: &ServiceContext<'_>,
    entity_id: &str,
    entity_type: &str,
    index: usize,
    state: &AppState,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let config = state.config.read().clone();
    let config = config.ok_or("No configuration loaded")?;
    let workspace_path = config.workspace_path.clone();

    let engine = state.signals.engine.clone();
    let entity_id = entity_id.to_string();
    let entity_type = entity_type.to_string();

    state
        .db_write(move |db| {
            let workspace = Path::new(&workspace_path);

            let account = if entity_type == "account" {
                db.get_account(&entity_id).map_err(|e| e.to_string())?
            } else {
                None
            };

            let entity_name = match entity_type.as_str() {
                "account" => account.as_ref().map(|a| a.name.clone()),
                "project" => db
                    .get_project(&entity_id)
                    .map_err(|e| e.to_string())?
                    .map(|p| p.name),
                "person" => db
                    .get_person(&entity_id)
                    .map_err(|e| e.to_string())?
                    .map(|p| p.name),
                _ => return Err(format!("Unsupported entity type: {}", entity_type)),
            }
            .ok_or_else(|| format!("{} '{}' not found", entity_type, entity_id))?;

            let dir = crate::intelligence::resolve_entity_dir(
                workspace,
                &entity_type,
                &entity_name,
                account.as_ref(),
            )?;

            // DB is sole source of truth — no filesystem fallback.
            let mut intel = db
                .get_entity_intelligence(&entity_id)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| {
                    format!(
                        "DOS-92: no DB intelligence row for {} — cannot dismiss recommendation",
                        entity_id
                    )
                })?;

            if index >= intel.recommended_actions.len() {
                return Err(format!("Recommendation index {} out of bounds", index));
            }

            let removed = intel.recommended_actions.remove(index);

            // DB-first ordering. Commit DB state; file write + signal
            // emission run after as best-effort post-commit work.
            db.upsert_entity_intelligence(&intel)
                .map_err(|e| e.to_string())?;

            // Post-commit signal emission. Failures log; DB is source of truth.
            let clock = crate::services::context::SystemClock;
            let rng = crate::services::context::SystemRng;
            let ext = crate::services::context::ExternalClients::default();
            let ctx = crate::services::context::ServiceContext::new_live(&clock, &rng, &ext);
            if let Err(e) = crate::services::signals::emit_and_propagate(
                &ctx,
                db,
                &engine,
                &entity_type,
                &entity_id,
                "recommendation_rejected",
                "user_correction",
                Some(&format!(
                    "{{\"title\":\"{}\"}}",
                    removed.title.replace('"', "\\\"")
                )),
                0.3,
            ) {
                log::warn!(
                    "post-commit signal emission failed; \
                     repair_target=signals_engine \
                     entity={entity_id}: {e}"
                );
            }

            // Post-commit file write — best-effort cache.
            // routed through the schema-epoch fence.
            crate::intelligence::write_fence::post_commit_fenced_write(
                db,
                &dir,
                &intel,
                &format!("entity={entity_id}"),
            );

            Ok(())
        })
        .await
        .map_err(String::from)
}

///  / Wave 0e: Mark an open commitment as done.
///
/// Removes the commitment at `index` from `openCommitments`, promotes it
/// into `valueDelivered` as a completion record, persists the updated
/// intelligence, and emits a `commitment_completed` signal so downstream
/// health scoring and briefing callouts see the transition.
///
/// Entity lookup and filesystem write mirror `dismiss_recommendation` so
/// the DB and on-disk intelligence.json stay in lockstep.
pub async fn mark_commitment_done(
    ctx: &ServiceContext<'_>,
    entity_id: &str,
    entity_type: &str,
    index: usize,
    state: &AppState,
) -> Result<(), String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let config = state.config.read().clone();
    let config = config.ok_or("No configuration loaded")?;
    let workspace_path = config.workspace_path.clone();

    let engine = state.signals.engine.clone();
    let entity_id = entity_id.to_string();
    let entity_type = entity_type.to_string();
    let now = ctx.clock.now().to_rfc3339();

    state
        .db_write(move |db| {
            let workspace = Path::new(&workspace_path);

            let account = if entity_type == "account" {
                db.get_account(&entity_id).map_err(|e| e.to_string())?
            } else {
                None
            };

            let entity_name = match entity_type.as_str() {
                "account" => account.as_ref().map(|a| a.name.clone()),
                "project" => db
                    .get_project(&entity_id)
                    .map_err(|e| e.to_string())?
                    .map(|p| p.name),
                "person" => db
                    .get_person(&entity_id)
                    .map_err(|e| e.to_string())?
                    .map(|p| p.name),
                _ => return Err(format!("Unsupported entity type: {}", entity_type)),
            }
            .ok_or_else(|| format!("{} '{}' not found", entity_type, entity_id))?;

            let dir = crate::intelligence::resolve_entity_dir(
                workspace,
                &entity_type,
                &entity_name,
                account.as_ref(),
            )?;

            let mut intel = db
                .get_entity_intelligence(&entity_id)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| {
                    format!(
                        "No DB intelligence row for {} — cannot mark commitment done",
                        entity_id
                    )
                })?;

            let commitment = {
                let list = intel
                    .open_commitments
                    .as_mut()
                    .ok_or_else(|| format!("Entity {} has no open commitments", entity_id))?;
                if index >= list.len() {
                    return Err(format!("Commitment index {} out of bounds", index));
                }
                list.remove(index)
            };

            // Promote into value_delivered as a completion record. The
            // "date" field takes now(); the original source is preserved so
            // the Context value-delivered chapter can show provenance.
            intel
                .value_delivered
                .push(crate::intelligence::io::ValueItem {
                    render_policy: None,
                    claim_id: None,
                    date: Some(now.clone()),
                    statement: commitment.description.clone(),
                    source: commitment.source.clone(),
                    impact: None,
                    item_source: Some(crate::intelligence::io::ItemSource {
                        source: "commitment_completed".to_string(),
                        confidence: 0.95,
                        sourced_at: now.clone(),
                        reference: commitment.owner.clone(),
                    }),
                    discrepancy: None,
                });

            // DB-first ordering. Commit canonical state; file + signal
            // run after as best-effort post-commit work.
            db.upsert_entity_intelligence(&intel)
                .map_err(|e| e.to_string())?;

            let clock = crate::services::context::SystemClock;
            let rng = crate::services::context::SystemRng;
            let ext = crate::services::context::ExternalClients::default();
            let ctx = crate::services::context::ServiceContext::new_live(&clock, &rng, &ext);
            if let Err(e) = crate::services::signals::emit_and_propagate(
                &ctx,
                db,
                &engine,
                &entity_type,
                &entity_id,
                "commitment_completed",
                "user_curation",
                Some(&format!(
                    "{{\"description\":\"{}\"}}",
                    commitment.description.replace('"', "\\\"")
                )),
                0.85,
            ) {
                log::warn!(
                    "post-commit signal emission failed; \
                     repair_target=signals_engine \
                     entity={entity_id}: {e}"
                );
            }

            // routed through the schema-epoch fence.
            crate::intelligence::write_fence::post_commit_fenced_write(
                db,
                &dir,
                &intel,
                &format!("entity={entity_id}"),
            );

            Ok(())
        })
        .await
        .map_err(String::from)
}

/// Get recommended actions for all entities (for use in the actions page).
pub fn get_all_recommended_actions(
    db: &ActionDb,
) -> Result<Vec<crate::intelligence::io::RecommendedAction>, String> {
    // Query all entity_assessment rows that have dimensions_json containing recommendedActions
    let conn = db.conn_ref();
    let mut stmt = conn
        .prepare("SELECT dimensions_json FROM entity_assessment WHERE dimensions_json IS NOT NULL")
        .map_err(|e| e.to_string())?;

    let mut all_actions = Vec::new();
    let rows = stmt
        .query_map([], |row| {
            let json: Option<String> = row.get(0)?;
            Ok(json)
        })
        .map_err(|e| e.to_string())?;

    for row in rows {
        if let Ok(Some(json)) = row {
            if let Ok(blob) = serde_json::from_str::<crate::intelligence::io::DimensionsBlob>(&json)
            {
                all_actions.extend(blob.recommended_actions);
            }
        }
    }

    Ok(all_actions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_utils::test_db;
    use crate::db::{AccountType, DbAccount};
    use crate::intelligence::{IntelRisk, IntelligenceJson, ItemSource, StakeholderInsight};
    use crate::services::context::{ExternalClients, FixedClock, SeedableRng, ServiceContext};
    use crate::signals::propagation::PropagationEngine;
    use chrono::TimeZone;
    use rusqlite::params;

    fn test_ctx<'a>(
        clock: &'a FixedClock,
        rng: &'a SeedableRng,
        ext: &'a ExternalClients,
    ) -> ServiceContext<'a> {
        ServiceContext::test_live(clock, rng, ext)
    }

    fn seed_account(db: &ActionDb, account_id: &str) {
        db.upsert_account(&DbAccount {
            id: account_id.to_string(),
            name: format!("Account {account_id}"),
            account_type: AccountType::Customer,
            updated_at: "2026-05-20T00:00:00Z".to_string(),
            ..Default::default()
        })
        .expect("seed account");
    }

    fn seed_person(db: &ActionDb, person_id: &str) {
        db.conn_ref()
            .execute(
                "INSERT INTO people (id, email, name, updated_at)
                 VALUES (?1, ?2, ?3, '2026-05-20T00:00:00Z')",
                params![
                    person_id,
                    format!("{person_id}@example.com"),
                    format!("Person {person_id}")
                ],
            )
            .expect("seed person");
    }

    fn generated_risk_intel(account_id: &str) -> IntelligenceJson {
        IntelligenceJson {
            executive_assessment_render_policy: None,
            entity_id: account_id.to_string(),
            entity_type: "account".to_string(),
            enriched_at: "2026-05-22T12:00:00Z".to_string(),
            risks: vec![IntelRisk {
                render_policy: None,
                claim_id: None,
                text: "Renewal owner has not approved the deployment plan.".to_string(),
                item_source: Some(ItemSource {
                    source: "glean_crm".to_string(),
                    confidence: 0.9,
                    sourced_at: "2026-05-20T10:30:00Z".to_string(),
                    reference: Some("CRM opportunity fixture".to_string()),
                }),
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    fn upsert_glean_assessment_from_enrichment(
        ctx: &ServiceContext<'_>,
        db: &ActionDb,
        engine: &PropagationEngine,
        entity_type: &str,
        entity_id: &str,
        intel: &IntelligenceJson,
    ) -> Result<(), String> {
        db.with_transaction(|tx| {
            super::upsert_assessment_from_enrichment_in_active_transaction(
                ctx,
                tx,
                engine,
                super::EnrichmentAssessmentUpsert {
                    entity_type,
                    entity_id,
                    intel,
                    projection_intel: intel,
                    projection_data_source: "glean",
                    cleared_dimensions: &[],
                },
            )
        })
    }

    fn active_generated_risk_count(db: &ActionDb, account_id: &str) -> i64 {
        db.conn_ref()
            .query_row(
                "SELECT COUNT(*)
                   FROM intelligence_claims
                  WHERE claim_type = 'entity_risk'
                    AND field_path = 'risks[0]'
                    AND claim_state = 'active'
                    AND surfacing_state = 'active'
                    AND json_valid(subject_ref) = 1
                    AND json_extract(subject_ref, '$.id') = ?1",
                params![account_id],
                |row| row.get(0),
            )
            .expect("active generated risk count")
    }

    fn active_generated_risk_id(db: &ActionDb, account_id: &str) -> String {
        db.conn_ref()
            .query_row(
                "SELECT id
                   FROM intelligence_claims
                  WHERE claim_type = 'entity_risk'
                    AND field_path = 'risks[0]'
                    AND claim_state = 'active'
                    AND surfacing_state = 'active'
                    AND json_valid(subject_ref) = 1
                    AND json_extract(subject_ref, '$.id') = ?1
                  ORDER BY created_at DESC
                  LIMIT 1",
                params![account_id],
                |row| row.get(0),
            )
            .expect("active generated risk id")
    }

    fn withdrawn_generated_risk_count(db: &ActionDb, account_id: &str) -> i64 {
        db.conn_ref()
            .query_row(
                "SELECT COUNT(*)
                   FROM intelligence_claims
                  WHERE claim_type = 'entity_risk'
                    AND field_path = 'risks[0]'
                    AND claim_state = 'withdrawn'
                    AND surfacing_state = 'dormant'
                    AND json_valid(subject_ref) = 1
                    AND json_extract(subject_ref, '$.id') = ?1",
                params![account_id],
                |row| row.get(0),
            )
            .expect("withdrawn generated risk count")
    }

    #[test]
    fn generated_risk_claim_carries_item_source_as_source_asof() {
        let db = test_db();
        let engine = PropagationEngine::default();
        let account_id = "acc-generated-risk-source";
        seed_account(&db, account_id);
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 22, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);

        upsert_glean_assessment_from_enrichment(
            &ctx,
            &db,
            &engine,
            "account",
            account_id,
            &generated_risk_intel(account_id),
        )
        .expect("commit generated risk projection");

        let (
            actor,
            data_source,
            source_ref,
            source_asof,
            provenance_json,
            temporal_scope,
            sensitivity,
        ): (
            String,
            String,
            Option<String>,
            Option<String>,
            String,
            String,
            String,
        ) = db
            .conn_ref()
            .query_row(
                "SELECT actor, data_source, source_ref, source_asof, provenance_json, temporal_scope, sensitivity
                   FROM intelligence_claims
                  WHERE claim_type = 'entity_risk'
                    AND field_path = 'risks[0]'
                    AND claim_state = 'active'
                    AND surfacing_state = 'active'",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                    ))
                },
            )
            .expect("read generated risk claim");

        assert_eq!(actor, "agent:intelligence");
        assert_eq!(data_source, "glean_crm");
        assert_eq!(source_asof.as_deref(), Some("2026-05-20T10:30:00Z"));
        assert_eq!(temporal_scope, "state");
        assert_eq!(sensitivity, "internal");
        assert!(
            source_ref
                .as_deref()
                .is_some_and(|value| value.starts_with(GENERATED_PROJECTION_SOURCE_REF_PREFIX)),
            "generated risk should carry a stable source reference"
        );

        let provenance: serde_json::Value =
            serde_json::from_str(&provenance_json).expect("provenance JSON");
        assert_eq!(provenance["provenance_schema_version"], 1);
        assert_eq!(
            provenance["ability_name"],
            "claim_shaped_intelligence_projection"
        );
        assert_eq!(
            provenance["sources"][0]["data_source"]["glean"]["downstream"],
            "salesforce"
        );
        assert_eq!(
            provenance["sources"][0]["source_asof"],
            "2026-05-20T10:30:00Z"
        );
        assert!(
            provenance["field_attributions"]
                .as_object()
                .is_some_and(|fields| !fields.is_empty()),
            "validated provenance envelope should attribute the generated claim"
        );
    }

    #[test]
    fn refreshed_glean_projection_withdraws_stale_same_root_claims() {
        let db = test_db();
        let engine = PropagationEngine::default();
        let account_id = "acc-generated-risk-refresh";
        seed_account(&db, account_id);
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 22, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(54);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);

        let first = generated_risk_intel(account_id);
        upsert_glean_assessment_from_enrichment(&ctx, &db, &engine, "account", account_id, &first)
            .expect("commit first Glean generated risk projection");
        let first_claim_id = active_generated_risk_id(&db, account_id);

        let mut second = generated_risk_intel(account_id);
        second.risks[0].text = "Updated CRM renewal risk needs executive follow-up.".to_string();
        if let Some(source) = second.risks[0].item_source.as_mut() {
            source.sourced_at = "2026-05-22T13:30:00Z".to_string();
            source.reference = Some("Updated CRM opportunity fixture".to_string());
        }
        upsert_glean_assessment_from_enrichment(&ctx, &db, &engine, "account", account_id, &second)
            .expect("commit refreshed Glean generated risk projection");

        assert_eq!(active_generated_risk_count(&db, account_id), 1);
        let active_text: String = db
            .conn_ref()
            .query_row(
                "SELECT text
                   FROM intelligence_claims
                  WHERE claim_type = 'entity_risk'
                    AND field_path = 'risks[0]'
                    AND claim_state = 'active'
                    AND surfacing_state = 'active'
                    AND json_valid(subject_ref) = 1
                    AND json_extract(subject_ref, '$.id') = ?1",
                params![account_id],
                |row| row.get(0),
            )
            .expect("active refreshed risk text");
        assert_eq!(
            active_text,
            "updated crm renewal risk needs executive follow-up."
        );

        let (old_state, old_surface, old_reason): (String, String, Option<String>) = db
            .conn_ref()
            .query_row(
                "SELECT claim_state, surfacing_state, retraction_reason
                   FROM intelligence_claims
                  WHERE id = ?1",
                params![first_claim_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("read first refreshed claim");
        assert_eq!(old_state, "withdrawn");
        assert_eq!(old_surface, "dormant");
        assert_eq!(old_reason.as_deref(), Some("projection_refreshed"));
    }

    #[test]
    fn generated_projection_same_text_new_evidence_reinforces() {
        let db = test_db();
        let engine = PropagationEngine::default();
        let account_id = "acc-generated-risk-reinforce";
        seed_account(&db, account_id);
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 22, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(43);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);

        upsert_glean_assessment_from_enrichment(
            &ctx,
            &db,
            &engine,
            "account",
            account_id,
            &generated_risk_intel(account_id),
        )
        .expect("commit first generated risk projection");
        let first_id = active_generated_risk_id(&db, account_id);

        let mut second = generated_risk_intel(account_id);
        let item_source = second.risks[0]
            .item_source
            .as_mut()
            .expect("risk item source");
        item_source.sourced_at = "2026-05-21T10:30:00Z".to_string();
        item_source.reference = Some("CRM opportunity follow-up fixture".to_string());

        upsert_glean_assessment_from_enrichment(&ctx, &db, &engine, "account", account_id, &second)
            .expect("commit reinforcing generated risk projection");

        assert_eq!(active_generated_risk_count(&db, account_id), 1);
        assert_eq!(
            active_generated_risk_id(&db, account_id),
            first_id,
            "same generated claim with new evidence should reinforce the existing claim"
        );
        let corroboration_count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*)
                   FROM claim_corroborations
                  WHERE claim_id = ?1
                    AND data_source = 'glean_crm'
                    AND source_asof = '2026-05-21T10:30:00Z'",
                params![&first_id],
                |row| row.get(0),
            )
            .expect("corroboration count");
        assert_eq!(corroboration_count, 1);
    }

    #[test]
    fn account_only_engagement_clear_maps_to_account_only_projection_roots() {
        let roots = super::projection_field_path_roots_for_cleared_dimensions(&[
            crate::intelligence::dimension_prompts::ACCOUNT_ONLY_ENGAGEMENT_FIELDS_CLEAR,
        ]);

        assert!(roots.contains(&"productAdoption"));
        assert!(roots.contains(&"supportHealth"));
        assert!(roots.contains(&"gongCallSummaries"));
        assert!(roots.contains(&"npsCsat"));
        assert!(
            !roots.contains(&"meetingCadence"),
            "person/project cadence remains applicable when only account-only engagement fields clear"
        );
        assert!(
            !roots.contains(&"emailResponsiveness"),
            "person/project responsiveness remains applicable when only account-only engagement fields clear"
        );
    }

    #[test]
    fn glean_source_purge_withdraws_generated_projection_claim() {
        let db = test_db();
        let engine = PropagationEngine::default();
        let account_id = "acc-generated-risk-purge";
        seed_account(&db, account_id);
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 22, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(44);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);

        upsert_glean_assessment_from_enrichment(
            &ctx,
            &db,
            &engine,
            "account",
            account_id,
            &generated_risk_intel(account_id),
        )
        .expect("commit generated risk projection");
        assert_eq!(active_generated_risk_count(&db, account_id), 1);

        let report = crate::db::data_lifecycle::purge_source(
            &db,
            crate::db::data_lifecycle::DataSource::Glean,
        )
        .expect("purge Glean");

        assert_eq!(report.generated_projection_claims_withdrawn, 1);
        assert_eq!(report.generated_projection_recompute_jobs_enqueued, 1);
        assert_eq!(active_generated_risk_count(&db, account_id), 0);
        assert_eq!(withdrawn_generated_risk_count(&db, account_id), 1);
    }

    #[test]
    fn explicit_empty_glean_field_withdraws_stale_projection_claim() {
        let db = test_db();
        let engine = PropagationEngine::default();
        let account_id = "acc-generated-risk-empty-refresh";
        seed_account(&db, account_id);
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 22, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(54);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);

        upsert_glean_assessment_from_enrichment(
            &ctx,
            &db,
            &engine,
            "account",
            account_id,
            &generated_risk_intel(account_id),
        )
        .expect("commit generated risk projection");
        assert_eq!(active_generated_risk_count(&db, account_id), 1);

        let empty_risks_refresh = IntelligenceJson {
            entity_id: account_id.to_string(),
            entity_type: "account".to_string(),
            enriched_at: "2026-05-22T13:00:00Z".to_string(),
            refreshed_fields: vec!["risks".to_string()],
            ..Default::default()
        };
        upsert_glean_assessment_from_enrichment(
            &ctx,
            &db,
            &engine,
            "account",
            account_id,
            &empty_risks_refresh,
        )
        .expect("persist explicit empty risks refresh");

        assert_eq!(active_generated_risk_count(&db, account_id), 0);
        assert_eq!(withdrawn_generated_risk_count(&db, account_id), 1);
    }

    #[test]
    fn account_refresh_withdraws_stale_person_subject_stakeholder_projection() {
        let db = test_db();
        let engine = PropagationEngine::default();
        let account_id = "acc-stakeholder-origin-refresh";
        let person_id = "person-stale-stakeholder-origin";
        seed_account(&db, account_id);
        seed_person(&db, person_id);
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 22, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(55);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);

        let first_refresh = IntelligenceJson {
            entity_id: account_id.to_string(),
            entity_type: "account".to_string(),
            enriched_at: "2026-05-22T12:00:00Z".to_string(),
            stakeholder_insights: vec![StakeholderInsight {
                name: "Fixture Stakeholder".to_string(),
                role: Some("Champion".to_string()),
                assessment: Some("Actively backing the rollout.".to_string()),
                engagement: Some("high".to_string()),
                person_id: Some(person_id.to_string()),
                ..Default::default()
            }],
            refreshed_fields: vec!["stakeholderInsights".to_string()],
            ..Default::default()
        };
        upsert_glean_assessment_from_enrichment(
            &ctx,
            &db,
            &engine,
            "account",
            account_id,
            &first_refresh,
        )
        .expect("commit person-subject stakeholder projection");

        let (active_before, origin_id): (i64, String) = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*),
                        json_extract(metadata_json, '$.projection_origin_subject.id')
                   FROM intelligence_claims
                  WHERE claim_type = 'stakeholder_engagement'
                    AND field_path = 'stakeholderInsights[0].engagement'
                    AND claim_state = 'active'
                    AND surfacing_state = 'active'
                    AND json_valid(subject_ref) = 1
                    AND lower(json_extract(subject_ref, '$.kind')) = 'person'
                    AND json_extract(subject_ref, '$.id') = ?1",
                params![person_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("active stakeholder projection with origin metadata");
        assert_eq!(active_before, 1);
        assert_eq!(origin_id, account_id);

        let second_refresh = IntelligenceJson {
            entity_id: account_id.to_string(),
            entity_type: "account".to_string(),
            enriched_at: "2026-05-22T13:00:00Z".to_string(),
            refreshed_fields: vec!["stakeholderInsights".to_string()],
            ..Default::default()
        };
        upsert_glean_assessment_from_enrichment(
            &ctx,
            &db,
            &engine,
            "account",
            account_id,
            &second_refresh,
        )
        .expect("persist stakeholder refresh without stale person");

        let active_after: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*)
                   FROM intelligence_claims
                  WHERE claim_type = 'stakeholder_engagement'
                    AND claim_state = 'active'
                    AND surfacing_state = 'active'
                    AND json_valid(subject_ref) = 1
                    AND lower(json_extract(subject_ref, '$.kind')) = 'person'
                    AND json_extract(subject_ref, '$.id') = ?1",
                params![person_id],
                |row| row.get(0),
            )
            .expect("active stakeholder projection count after refresh");
        assert_eq!(active_after, 0);
        let withdrawn_after: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*)
                   FROM intelligence_claims
                  WHERE claim_type = 'stakeholder_engagement'
                    AND claim_state = 'withdrawn'
                    AND surfacing_state = 'dormant'
                    AND retraction_reason = 'projection_refreshed'
                    AND json_valid(subject_ref) = 1
                    AND lower(json_extract(subject_ref, '$.kind')) = 'person'
                    AND json_extract(subject_ref, '$.id') = ?1",
                params![person_id],
                |row| row.get(0),
            )
            .expect("withdrawn stakeholder projection count after refresh");
        assert_eq!(withdrawn_after, 1);
        let person_recompute_jobs: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*)
                   FROM invalidation_jobs
                  WHERE job_kind = 'claim_recompute'
                    AND subject_type = 'person'
                    AND subject_id = ?1",
                params![person_id],
                |row| row.get(0),
            )
            .expect("person recompute jobs");
        assert!(
            person_recompute_jobs >= 1,
            "withdrawn person-subject projection should enqueue trust recompute"
        );
    }

    #[test]
    fn glean_source_purge_withdraws_generated_projection_without_source_ref() {
        let db = test_db();
        let engine = PropagationEngine::default();
        let account_id = "acc-generated-risk-purge-no-ref";
        seed_account(&db, account_id);
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 22, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(45);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);
        let mut intel = generated_risk_intel(account_id);
        intel.risks[0]
            .item_source
            .as_mut()
            .expect("risk item source")
            .reference = None;

        upsert_glean_assessment_from_enrichment(&ctx, &db, &engine, "account", account_id, &intel)
            .expect("commit generated risk projection without source ref");
        let source_ref: Option<String> = db
            .conn_ref()
            .query_row(
                "SELECT source_ref
                   FROM intelligence_claims
                  WHERE claim_type = 'entity_risk'
                    AND field_path = 'risks[0]'
                    AND claim_state = 'active'
                    AND surfacing_state = 'active'",
                [],
                |row| row.get(0),
            )
            .expect("read generated risk source ref");
        assert_eq!(source_ref, None);

        let report = crate::db::data_lifecycle::purge_source(
            &db,
            crate::db::data_lifecycle::DataSource::Glean,
        )
        .expect("purge Glean");

        assert_eq!(report.generated_projection_claims_withdrawn, 1);
        assert_eq!(report.generated_projection_recompute_jobs_enqueued, 1);
        assert_eq!(active_generated_risk_count(&db, account_id), 0);
        assert_eq!(withdrawn_generated_risk_count(&db, account_id), 1);
    }

    #[test]
    fn glean_source_purge_withdraws_producer_marked_projection_without_item_source() {
        let db = test_db();
        let engine = PropagationEngine::default();
        let account_id = "acc-generated-summary-purge-no-item-source";
        seed_account(&db, account_id);
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 22, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(46);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);
        let intel = IntelligenceJson {
            executive_assessment_render_policy: None,
            entity_id: account_id.to_string(),
            entity_type: "account".to_string(),
            enriched_at: "2026-05-22T12:00:00Z".to_string(),
            executive_assessment: Some("Glean summary without item-level source.".to_string()),
            ..Default::default()
        };

        db.with_transaction(|tx| {
            super::upsert_assessment_from_enrichment_in_active_transaction(
                &ctx,
                tx,
                &engine,
                super::EnrichmentAssessmentUpsert {
                    entity_type: "account",
                    entity_id: account_id,
                    intel: &intel,
                    projection_intel: &intel,
                    projection_data_source: "glean",
                    cleared_dimensions: &[],
                },
            )
        })
        .expect("commit Glean producer projection without item source");
        let (data_source, source_ref, projection_producer): (String, Option<String>, String) = db
            .conn_ref()
            .query_row(
                "SELECT data_source,
                        source_ref,
                        json_extract(metadata_json, '$.projection_producer')
                   FROM intelligence_claims
                  WHERE claim_type = 'entity_summary'
                    AND field_path = 'executiveAssessment'
                    AND claim_state = 'active'
                    AND surfacing_state = 'active'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("read generated summary claim");
        assert_eq!(data_source, "glean");
        assert_eq!(source_ref, None);
        assert_eq!(projection_producer, "glean");

        let report = crate::db::data_lifecycle::purge_source(
            &db,
            crate::db::data_lifecycle::DataSource::Glean,
        )
        .expect("purge Glean");

        assert_eq!(report.generated_projection_claims_withdrawn, 1);
        assert_eq!(report.generated_projection_recompute_jobs_enqueued, 1);
        let active_count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*)
                   FROM intelligence_claims
                  WHERE claim_type = 'entity_summary'
                    AND field_path = 'executiveAssessment'
                    AND claim_state = 'active'",
                [],
                |row| row.get(0),
            )
            .expect("active summary count");
        assert_eq!(active_count, 0);
    }

    #[test]
    fn generated_projection_does_not_trust_model_supplied_privileged_item_source() {
        let db = test_db();
        let engine = PropagationEngine::default();
        let account_id = "acc-generated-risk-untrusted-item-source";
        seed_account(&db, account_id);
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 22, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(48);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);
        let mut intel = generated_risk_intel(account_id);
        let item_source = intel.risks[0]
            .item_source
            .as_mut()
            .expect("risk item source");
        item_source.source = "user_correction".to_string();
        item_source.reference = Some("model-supplied privileged source".to_string());

        db.with_transaction(|tx| {
            super::upsert_assessment_from_enrichment_in_active_transaction(
                &ctx,
                tx,
                &engine,
                super::EnrichmentAssessmentUpsert {
                    entity_type: "account",
                    entity_id: account_id,
                    intel: &intel,
                    projection_intel: &intel,
                    projection_data_source: "glean",
                    cleared_dimensions: &[],
                },
            )
        })
        .expect("commit Glean-produced projection with non-Glean item source");
        let (data_source, projection_producer): (String, String) = db
            .conn_ref()
            .query_row(
                "SELECT data_source,
                        json_extract(metadata_json, '$.projection_producer')
                   FROM intelligence_claims
                  WHERE claim_type = 'entity_risk'
                    AND field_path = 'risks[0]'
                    AND claim_state = 'active'
                    AND surfacing_state = 'active'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("read generated risk claim");
        assert_eq!(data_source, "glean");
        assert_eq!(projection_producer, "glean");

        let pty_account_id = "acc-generated-risk-untrusted-pty-item-source";
        seed_account(&db, pty_account_id);
        let mut pty_intel = generated_risk_intel(pty_account_id);
        pty_intel.risks[0]
            .item_source
            .as_mut()
            .expect("pty risk item source")
            .source = "user_correction".to_string();
        upsert_assessment_from_enrichment(
            &ctx,
            &db,
            &engine,
            "account",
            pty_account_id,
            &pty_intel,
        )
        .expect("commit PTY-produced projection with untrusted item source");
        let pty_data_source: String = db
            .conn_ref()
            .query_row(
                "SELECT data_source
                   FROM intelligence_claims
                  WHERE claim_type = 'entity_risk'
                    AND field_path = 'risks[0]'
                    AND claim_state = 'active'
                    AND surfacing_state = 'active'
                    AND json_valid(subject_ref) = 1
                    AND json_extract(subject_ref, '$.id') = ?1",
                params![pty_account_id],
                |row| row.get(0),
            )
            .expect("read PTY generated risk data source");
        assert_eq!(pty_data_source, "ai_enrichment");

        let report = crate::db::data_lifecycle::purge_source(
            &db,
            crate::db::data_lifecycle::DataSource::Glean,
        )
        .expect("purge Glean");

        assert_eq!(report.generated_projection_claims_withdrawn, 1);
        assert_eq!(report.generated_projection_recompute_jobs_enqueued, 1);
        assert_eq!(
            active_generated_risk_count(&db, account_id),
            0,
            "model-supplied privileged source labels must not survive Glean source purge"
        );
        assert_eq!(
            active_generated_risk_count(&db, pty_account_id),
            1,
            "Glean source purge should not withdraw local producer projections"
        );
    }

    #[test]
    fn glean_projection_coerces_non_glean_item_source_to_glean_boundary() {
        let db = test_db();
        let engine = PropagationEngine::default();
        let account_id = "acc-generated-risk-transcript-source";
        seed_account(&db, account_id);
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 22, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(51);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);
        let mut intel = generated_risk_intel(account_id);
        let item_source = intel.risks[0]
            .item_source
            .as_mut()
            .expect("risk item source");
        item_source.source = "transcript".to_string();
        item_source.confidence = 0.8;
        item_source.reference = Some("meeting transcript fixture".to_string());

        upsert_glean_assessment_from_enrichment(&ctx, &db, &engine, "account", account_id, &intel)
            .expect("commit generated risk projection with transcript evidence");
        let (data_source, projection_producer, provenance_json): (String, String, String) = db
            .conn_ref()
            .query_row(
                "SELECT data_source,
                        json_extract(metadata_json, '$.projection_producer'),
                        provenance_json
                   FROM intelligence_claims
                  WHERE claim_type = 'entity_risk'
                    AND field_path = 'risks[0]'
                    AND claim_state = 'active'
                    AND surfacing_state = 'active'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("read generated risk claim");
        assert_eq!(data_source, "glean");
        assert_eq!(projection_producer, "glean");
        let provenance: serde_json::Value =
            serde_json::from_str(&provenance_json).expect("provenance JSON");
        assert_eq!(
            provenance["sources"][0]["data_source"]["glean"]["downstream"],
            "unknown"
        );

        let report = crate::db::data_lifecycle::purge_source(
            &db,
            crate::db::data_lifecycle::DataSource::Glean,
        )
        .expect("purge Glean");

        assert_eq!(report.generated_projection_claims_withdrawn, 1);
        assert_eq!(report.generated_projection_recompute_jobs_enqueued, 1);
        assert_eq!(active_generated_risk_count(&db, account_id), 0);
    }

    #[test]
    fn progressive_snapshot_does_not_advance_freshness_domains_or_claims() {
        let db = test_db();
        let engine = PropagationEngine::default();
        let account_id = "acc-progressive-cache-only";
        seed_account(&db, account_id);
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 22, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(53);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);
        let final_intel = IntelligenceJson {
            executive_assessment_render_policy: None,
            entity_id: account_id.to_string(),
            entity_type: "account".to_string(),
            enriched_at: "2026-05-22T12:00:00Z".to_string(),
            executive_assessment: Some("Authoritative committed summary.".to_string()),
            ..Default::default()
        };

        upsert_assessment_from_enrichment(&ctx, &db, &engine, "account", account_id, &final_intel)
            .expect("commit authoritative assessment");

        let progressive_intel = IntelligenceJson {
            executive_assessment_render_policy: None,
            entity_id: account_id.to_string(),
            entity_type: "account".to_string(),
            enriched_at: "2026-05-27T12:00:00Z".to_string(),
            executive_assessment: Some("Partial in-flight summary.".to_string()),
            domains: vec!["partial.example".to_string()],
            ..Default::default()
        };

        upsert_progressive_assessment_snapshot(&ctx, &db, &progressive_intel)
            .expect("write progressive UI cache");

        let enriched_at: Option<String> = db
            .conn_ref()
            .query_row(
                "SELECT enriched_at FROM entity_assessment WHERE entity_id = ?1",
                params![account_id],
                |row| row.get(0),
            )
            .expect("read enriched_at");
        assert_eq!(enriched_at.as_deref(), Some("2026-05-22T12:00:00Z"));

        let partial_domain_count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM account_domains WHERE account_id = ?1 AND domain = 'partial.example'",
                params![account_id],
                |row| row.get(0),
            )
            .expect("count partial domains");
        assert_eq!(partial_domain_count, 0);

        let progressive_claim_count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM intelligence_claims WHERE data_source = 'ai_enrichment_progressive'",
                [],
                |row| row.get(0),
            )
            .expect("count progressive claims");
        assert_eq!(progressive_claim_count, 0);
    }

    #[test]
    fn final_glean_projection_supersedes_progressive_projection_with_same_text() {
        let db = test_db();
        let engine = PropagationEngine::default();
        let account_id = "acc-generated-summary-purge-progressive-glean";
        seed_account(&db, account_id);
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 22, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(47);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);
        let intel = IntelligenceJson {
            executive_assessment_render_policy: None,
            entity_id: account_id.to_string(),
            entity_type: "account".to_string(),
            enriched_at: "2026-05-22T12:00:00Z".to_string(),
            executive_assessment: Some(
                "Glean progressive summary without item-level source.".to_string(),
            ),
            ..Default::default()
        };

        upsert_assessment_snapshot(&ctx, &db, &intel)
            .expect("commit progressive generated projection");
        let (claim_id, data_source, projection_producer): (String, String, String) = db
            .conn_ref()
            .query_row(
                "SELECT id,
                        data_source,
                        json_extract(metadata_json, '$.projection_producer')
                   FROM intelligence_claims
                  WHERE claim_type = 'entity_summary'
                    AND field_path = 'executiveAssessment'
                    AND claim_state = 'active'
                    AND surfacing_state = 'active'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("read progressive generated summary claim");
        assert_eq!(data_source, "ai_enrichment_progressive");
        assert_eq!(projection_producer, "ai_enrichment_progressive");

        db.with_transaction(|tx| {
            super::upsert_assessment_from_enrichment_in_active_transaction(
                &ctx,
                tx,
                &engine,
                super::EnrichmentAssessmentUpsert {
                    entity_type: "account",
                    entity_id: account_id,
                    intel: &intel,
                    projection_intel: &intel,
                    projection_data_source: "glean",
                    cleared_dimensions: &[],
                },
            )
        })
        .expect("commit final Glean generated projection");
        let (active_claim_id, active_data_source, active_projection_producer): (
            String,
            String,
            String,
        ) = db
            .conn_ref()
            .query_row(
                "SELECT id,
                        data_source,
                        json_extract(metadata_json, '$.projection_producer')
                   FROM intelligence_claims
                  WHERE claim_type = 'entity_summary'
                    AND field_path = 'executiveAssessment'
                    AND claim_state = 'active'
                    AND surfacing_state = 'active'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("read final active summary claim");
        assert_ne!(active_claim_id, claim_id);
        assert_eq!(active_data_source, "glean");
        assert_eq!(active_projection_producer, "glean");
        let (progressive_claim_state, progressive_surfacing_state, progressive_superseded_by): (
            String,
            String,
            Option<String>,
        ) = db
            .conn_ref()
            .query_row(
                "SELECT claim_state, surfacing_state, superseded_by
                   FROM intelligence_claims
                  WHERE id = ?1",
                params![&claim_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("read superseded progressive claim");
        assert_eq!(progressive_claim_state, "dormant");
        assert_eq!(progressive_surfacing_state, "dormant");
        assert_eq!(
            progressive_superseded_by.as_deref(),
            Some(active_claim_id.as_str())
        );
        let active_count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*)
                   FROM intelligence_claims
                  WHERE claim_type = 'entity_summary'
                    AND field_path = 'executiveAssessment'
                    AND claim_state = 'active'
                    AND surfacing_state = 'active'",
                [],
                |row| row.get(0),
            )
            .expect("active summary count after final Glean write");
        assert_eq!(active_count, 1);
        let corroboration_count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*)
                   FROM claim_corroborations
                  WHERE claim_id = ?1
                    AND data_source = 'glean'",
                params![&claim_id],
                |row| row.get(0),
            )
            .expect("Glean corroboration count");
        assert_eq!(corroboration_count, 0);

        let report = crate::db::data_lifecycle::purge_source(
            &db,
            crate::db::data_lifecycle::DataSource::Glean,
        )
        .expect("purge Glean");

        assert_eq!(report.generated_projection_claims_withdrawn, 1);
        assert_eq!(report.generated_projection_recompute_jobs_enqueued, 1);
        let active_count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*)
                   FROM intelligence_claims
                  WHERE claim_type = 'entity_summary'
                    AND field_path = 'executiveAssessment'
                    AND claim_state = 'active'",
                [],
                |row| row.get(0),
            )
            .expect("active summary count after purge");
        assert_eq!(active_count, 0);
        let withdrawn_count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*)
                   FROM intelligence_claims
                  WHERE claim_type = 'entity_summary'
                    AND field_path = 'executiveAssessment'
                    AND claim_state = 'withdrawn'
                    AND surfacing_state = 'dormant'",
                [],
                |row| row.get(0),
            )
            .expect("withdrawn summary count after purge");
        assert_eq!(withdrawn_count, 1);
    }

    #[test]
    fn glean_source_purge_recomputes_local_projection_with_glean_corroboration() {
        let db = test_db();
        let engine = PropagationEngine::default();
        let account_id = "acc-generated-risk-local-glean-corroboration";
        seed_account(&db, account_id);
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 22, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(52);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);
        let intel = generated_risk_intel(account_id);

        upsert_assessment_from_enrichment(&ctx, &db, &engine, "account", account_id, &intel)
            .expect("commit local generated risk projection");
        let claim_id = active_generated_risk_id(&db, account_id);
        upsert_glean_assessment_from_enrichment(&ctx, &db, &engine, "account", account_id, &intel)
            .expect("reinforce local generated risk projection from Glean");

        assert_eq!(active_generated_risk_id(&db, account_id), claim_id);
        let glean_corroboration_count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*)
                   FROM claim_corroborations
                  WHERE claim_id = ?1
                    AND data_source = 'glean_crm'",
                params![&claim_id],
                |row| row.get(0),
            )
            .expect("Glean corroboration count");
        assert_eq!(glean_corroboration_count, 1);

        let report = crate::db::data_lifecycle::purge_source(
            &db,
            crate::db::data_lifecycle::DataSource::Glean,
        )
        .expect("purge Glean");

        assert_eq!(report.generated_projection_claims_withdrawn, 0);
        assert_eq!(report.generated_projection_recompute_jobs_enqueued, 1);
        assert_eq!(active_generated_risk_count(&db, account_id), 1);
        let remaining_glean_corroborations: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*)
                   FROM claim_corroborations
                  WHERE claim_id = ?1
                    AND (
                        data_source = 'glean'
                        OR data_source LIKE 'glean_%'
                    )",
                params![&claim_id],
                |row| row.get(0),
            )
            .expect("remaining Glean corroboration count");
        assert_eq!(remaining_glean_corroborations, 0);
    }

    #[test]
    fn glean_source_purge_preserves_progressive_projection_with_local_corroboration() {
        let db = test_db();
        let engine = PropagationEngine::default();
        let account_id = "acc-generated-summary-purge-progressive-local";
        seed_account(&db, account_id);
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 22, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(49);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);
        let intel = IntelligenceJson {
            executive_assessment_render_policy: None,
            entity_id: account_id.to_string(),
            entity_type: "account".to_string(),
            enriched_at: "2026-05-22T12:00:00Z".to_string(),
            executive_assessment: Some(
                "Progressive summary with durable local support.".to_string(),
            ),
            ..Default::default()
        };

        upsert_assessment_snapshot(&ctx, &db, &intel)
            .expect("commit progressive generated projection");
        let claim_id: String = db
            .conn_ref()
            .query_row(
                "SELECT id
                   FROM intelligence_claims
                  WHERE claim_type = 'entity_summary'
                    AND field_path = 'executiveAssessment'
                    AND claim_state = 'active'
                    AND surfacing_state = 'active'",
                [],
                |row| row.get(0),
            )
            .expect("read progressive generated summary claim");

        upsert_assessment_from_enrichment(&ctx, &db, &engine, "account", account_id, &intel)
            .expect("reinforce progressive projection from local final write");
        db.with_transaction(|tx| {
            super::upsert_assessment_from_enrichment_in_active_transaction(
                &ctx,
                tx,
                &engine,
                super::EnrichmentAssessmentUpsert {
                    entity_type: "account",
                    entity_id: account_id,
                    intel: &intel,
                    projection_intel: &intel,
                    projection_data_source: "glean",
                    cleared_dimensions: &[],
                },
            )
        })
        .expect("reinforce progressive projection from Glean");

        let local_corroboration_count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*)
                   FROM claim_corroborations
                  WHERE claim_id = ?1
                    AND data_source = 'ai_enrichment'",
                params![&claim_id],
                |row| row.get(0),
            )
            .expect("local corroboration count");
        assert_eq!(local_corroboration_count, 1);
        let glean_corroboration_count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*)
                   FROM claim_corroborations
                  WHERE claim_id = ?1
                    AND data_source = 'glean'",
                params![&claim_id],
                |row| row.get(0),
            )
            .expect("Glean corroboration count");
        assert_eq!(glean_corroboration_count, 1);

        let report = crate::db::data_lifecycle::purge_source(
            &db,
            crate::db::data_lifecycle::DataSource::Glean,
        )
        .expect("purge Glean");

        assert_eq!(report.generated_projection_claims_withdrawn, 0);
        assert_eq!(report.generated_projection_recompute_jobs_enqueued, 1);
        let active_count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*)
                   FROM intelligence_claims
                  WHERE id = ?1
                    AND claim_state = 'active'
                    AND surfacing_state = 'active'",
                params![&claim_id],
                |row| row.get(0),
            )
            .expect("active summary count after purge");
        assert_eq!(active_count, 1);
        let remaining_glean_corroborations: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*)
                   FROM claim_corroborations
                  WHERE claim_id = ?1
                    AND data_source = 'glean'",
                params![&claim_id],
                |row| row.get(0),
            )
            .expect("remaining Glean corroboration count");
        assert_eq!(remaining_glean_corroborations, 0);
    }

    #[test]
    fn glean_source_purge_reissues_glean_origin_projection_with_local_corroboration() {
        let db = test_db();
        let engine = PropagationEngine::default();
        let account_id = "acc-generated-summary-purge-glean-local";
        seed_account(&db, account_id);
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 22, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(50);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);
        let intel = IntelligenceJson {
            executive_assessment_render_policy: None,
            entity_id: account_id.to_string(),
            entity_type: "account".to_string(),
            enriched_at: "2026-05-22T12:00:00Z".to_string(),
            executive_assessment: Some("Glean summary with durable local support.".to_string()),
            ..Default::default()
        };

        db.with_transaction(|tx| {
            super::upsert_assessment_from_enrichment_in_active_transaction(
                &ctx,
                tx,
                &engine,
                super::EnrichmentAssessmentUpsert {
                    entity_type: "account",
                    entity_id: account_id,
                    intel: &intel,
                    projection_intel: &intel,
                    projection_data_source: "glean",
                    cleared_dimensions: &[],
                },
            )
        })
        .expect("commit Glean-origin generated projection");
        let claim_id: String = db
            .conn_ref()
            .query_row(
                "SELECT id
                   FROM intelligence_claims
                  WHERE claim_type = 'entity_summary'
                    AND field_path = 'executiveAssessment'
                    AND claim_state = 'active'
                    AND surfacing_state = 'active'",
                [],
                |row| row.get(0),
            )
            .expect("read Glean-origin generated summary claim");

        upsert_assessment_from_enrichment(&ctx, &db, &engine, "account", account_id, &intel)
            .expect("reinforce Glean-origin projection from local final write");
        let local_corroboration_count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*)
                   FROM claim_corroborations
                  WHERE claim_id = ?1
                    AND data_source = 'ai_enrichment'",
                params![&claim_id],
                |row| row.get(0),
            )
            .expect("local corroboration count");
        assert_eq!(local_corroboration_count, 1);

        let report = crate::db::data_lifecycle::purge_source(
            &db,
            crate::db::data_lifecycle::DataSource::Glean,
        )
        .expect("purge Glean");

        assert_eq!(report.generated_projection_claims_withdrawn, 1);
        assert_eq!(report.generated_projection_recompute_jobs_enqueued, 1);
        let old_active_count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*)
                   FROM intelligence_claims
                  WHERE id = ?1
                    AND claim_state = 'active'
                    AND surfacing_state = 'active'",
                params![&claim_id],
                |row| row.get(0),
            )
            .expect("old active summary count after purge");
        assert_eq!(old_active_count, 0);
        let (
            active_claim_id,
            active_data_source,
            active_source_ref,
            active_projection_producer,
            active_provenance_json,
            active_metadata_json,
        ): (String, String, Option<String>, String, String, String) = db
            .conn_ref()
            .query_row(
                "SELECT id,
                        data_source,
                        source_ref,
                        json_extract(metadata_json, '$.projection_producer'),
                        provenance_json,
                        metadata_json
                   FROM intelligence_claims
                  WHERE claim_type = 'entity_summary'
                    AND field_path = 'executiveAssessment'
                    AND claim_state = 'active'
                    AND surfacing_state = 'active'
                    AND json_valid(subject_ref) = 1
                    AND json_extract(subject_ref, '$.id') = ?1",
                params![account_id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                    ))
                },
            )
            .expect("reissued active summary claim");
        assert_ne!(active_claim_id, claim_id);
        assert_eq!(active_data_source, "ai_enrichment");
        assert_eq!(active_source_ref, None);
        assert_eq!(active_projection_producer, "ai_enrichment");
        let active_provenance: serde_json::Value =
            serde_json::from_str(&active_provenance_json).expect("active provenance JSON");
        assert_eq!(active_provenance["sources"][0]["data_source"], "ai");
        let active_metadata: serde_json::Value =
            serde_json::from_str(&active_metadata_json).expect("active metadata JSON");
        assert_eq!(active_metadata["projection_producer"], "ai_enrichment");
        assert!(
            active_metadata.get("legacy_projection_value").is_none(),
            "reissued local claim metadata must not retain purged Glean projection payload"
        );
        assert!(
            !active_metadata_json.contains("glean"),
            "reissued local claim metadata must not retain Glean source labels"
        );
        let remaining_glean_corroborations: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*)
                   FROM claim_corroborations
                  WHERE claim_id = ?1
                    AND (
                        data_source = 'glean'
                        OR data_source LIKE 'glean_%'
                    )",
                params![&claim_id],
                |row| row.get(0),
            )
            .expect("remaining Glean corroboration count");
        assert_eq!(remaining_glean_corroborations, 0);
    }
}

#[cfg(test)]
mod mutation_smoke_tests {
    use crate::db::test_utils::test_db;
    use crate::db::{AccountType, DbAccount};
    use crate::intel_queue::{
        apply_enrichment_side_writes, compose_enrichment_intelligence_payload,
        run_enrichment_finalize_post_commit, EnrichmentInput, FinalizeMode,
    };
    use crate::intelligence::io::{
        AccountHealth, AdoptionSignals, AgreementOutlook, Blocker, CompetitiveInsight,
        ContractContext, DimensionScore, ExpansionSignal, GongCallSummary, IntelRisk,
        IntelligenceJson, ItemSource, OpenCommitment, OrgChange, OrgHealthData,
        ProductClassification, ProductInfo, RecommendedAction, RelationshipDimensions,
        StakeholderInsight, StrategicPriority, SuccessMetric, SupportHealth,
    };
    use crate::intelligence::prompts::InferredRelationship;
    use crate::intelligence::write_fence::{fenced_write_intelligence_json, FenceCycle};
    use crate::services::context::{ExternalClients, FixedClock, SeedableRng, ServiceContext};
    use crate::signals::propagation::PropagationEngine;
    use crate::state::AppState;
    use chrono::TimeZone;
    use rusqlite::{params, OptionalExtension};
    use std::path::Path;
    use std::sync::Arc;
    use std::time::Duration;

    fn test_ctx<'a>(
        clock: &'a FixedClock,
        rng: &'a SeedableRng,
        ext: &'a ExternalClients,
    ) -> ServiceContext<'a> {
        ServiceContext::test_live(clock, rng, ext)
    }

    fn make_account(id: &str) -> DbAccount {
        DbAccount {
            id: id.to_string(),
            name: format!("Account {id}"),
            lifecycle: Some("active".to_string()),
            arr: Some(100_000.0),
            health: None,
            contract_start: Some("2025-01-01".to_string()),
            contract_end: Some("2027-01-01".to_string()),
            nps: None,
            tracker_path: None,
            parent_id: None,
            account_type: AccountType::Customer,
            updated_at: chrono::Utc::now().to_rfc3339(),
            archived: false,
            keywords: None,
            keywords_extracted_at: None,
            metadata: None,
            ..Default::default()
        }
    }

    fn seed_disk_intelligence(db: &crate::db::ActionDb, dir: &Path, intel: &IntelligenceJson) {
        for attempt in 0..100 {
            match FenceCycle::capture(db) {
                Ok(cycle) => {
                    fenced_write_intelligence_json(&cycle, db, dir, intel)
                        .expect("seed disk intelligence");
                    return;
                }
                Err(err) if err.contains("paused") && attempt < 99 => {
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(err) => panic!("capture seed disk fence: {err}"),
            }
        }
    }

    #[test]
    fn blank_entity_intelligence_snapshot_has_entity_identity_without_disk() {
        let intel = super::blank_entity_intelligence_snapshot(
            "acct-no-disk-fallback",
            "account",
            "2026-05-23T12:00:00Z",
        );

        assert_eq!(intel.entity_id, "acct-no-disk-fallback");
        assert_eq!(intel.entity_type, "account");
        assert_eq!(intel.enriched_at, "2026-05-23T12:00:00Z");
        assert!(intel.stakeholder_insights.is_empty());
    }

    fn signal_count(db: &crate::db::ActionDb, entity_id: &str, signal_type: &str) -> i64 {
        db.conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM signal_events WHERE entity_id = ?1 AND signal_type = ?2",
                params![entity_id, signal_type],
                |row| row.get(0),
            )
            .unwrap_or(0)
    }

    fn claim_recompute_job_count(
        db: &crate::db::ActionDb,
        subject_type: &str,
        subject_id: &str,
    ) -> i64 {
        db.conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM invalidation_jobs
                 WHERE job_kind = 'claim_recompute'
                   AND subject_type = ?1
                   AND subject_id = ?2",
                params![subject_type, subject_id],
                |row| row.get(0),
            )
            .unwrap_or(0)
    }

    fn account_fact_claim_count(db: &crate::db::ActionDb, account_id: &str) -> i64 {
        db.conn_ref()
            .query_row(
                "SELECT COUNT(*)
                   FROM intelligence_claims
                  WHERE claim_type = 'account_fact'
                    AND json_valid(subject_ref) = 1
                    AND lower(json_extract(subject_ref, '$.kind')) = 'account'
                    AND json_extract(subject_ref, '$.id') = ?1",
                params![account_id],
                |row| row.get(0),
            )
            .expect("account fact claim count")
    }

    fn account_source_ref_count(db: &crate::db::ActionDb, account_id: &str) -> i64 {
        db.conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM account_source_refs WHERE account_id = ?1",
                params![account_id],
                |row| row.get(0),
            )
            .expect("account source ref count")
    }

    fn visible_account_source_ref_count(db: &crate::db::ActionDb, account_id: &str) -> usize {
        db.get_account_source_refs(account_id)
            .expect("account source refs")
            .len()
    }

    fn purged_account_source_ref_count(db: &crate::db::ActionDb, account_id: &str) -> i64 {
        db.conn_ref()
            .query_row(
                "SELECT COUNT(*)
                   FROM account_source_refs
                  WHERE account_id = ?1
                    AND source_kind = 'source_purged'",
                params![account_id],
                |row| row.get(0),
            )
            .expect("purged account source ref count")
    }

    fn active_glean_account_fact_claim_count(db: &crate::db::ActionDb, account_id: &str) -> i64 {
        db.conn_ref()
            .query_row(
                "SELECT COUNT(*)
                   FROM intelligence_claims
                  WHERE claim_type = 'account_fact'
                    AND claim_state = 'active'
                    AND surfacing_state = 'active'
                    AND source_ref LIKE 'glean_account_fact:%'
                    AND json_valid(subject_ref) = 1
                    AND lower(json_extract(subject_ref, '$.kind')) = 'account'
                    AND json_extract(subject_ref, '$.id') = ?1",
                params![account_id],
                |row| row.get(0),
            )
            .expect("active Glean account fact claim count")
    }

    fn withdrawn_glean_account_fact_claim_count(db: &crate::db::ActionDb, account_id: &str) -> i64 {
        db.conn_ref()
            .query_row(
                "SELECT COUNT(*)
                   FROM intelligence_claims
                  WHERE claim_type = 'account_fact'
                    AND claim_state = 'withdrawn'
                    AND surfacing_state = 'dormant'
                    AND source_ref LIKE 'glean_account_fact:%'
                    AND json_valid(subject_ref) = 1
                    AND lower(json_extract(subject_ref, '$.kind')) = 'account'
                    AND json_extract(subject_ref, '$.id') = ?1",
                params![account_id],
                |row| row.get(0),
            )
            .expect("withdrawn Glean account fact claim count")
    }

    fn withdrawn_account_fact_claim_count_by_source_ref(
        db: &crate::db::ActionDb,
        account_id: &str,
        source_ref: &str,
    ) -> i64 {
        db.conn_ref()
            .query_row(
                "SELECT COUNT(*)
                   FROM intelligence_claims
                  WHERE claim_type = 'account_fact'
                    AND claim_state = 'withdrawn'
                    AND surfacing_state = 'dormant'
                    AND source_ref = ?2
                    AND json_valid(subject_ref) = 1
                    AND lower(json_extract(subject_ref, '$.kind')) = 'account'
                    AND json_extract(subject_ref, '$.id') = ?1",
                params![account_id, source_ref],
                |row| row.get(0),
            )
            .expect("withdrawn account fact claim count by source ref")
    }

    fn seed_account_domain(db: &crate::db::ActionDb, account_id: &str, domain: &str) {
        db.conn_ref()
            .execute(
                "INSERT INTO account_domains (account_id, domain, source) VALUES (?1, ?2, 'test')",
                params![account_id, domain],
            )
            .expect("seed account domain");
    }

    fn seed_person(db: &crate::db::ActionDb, id: &str, name: &str) {
        db.conn_ref()
            .execute(
                "INSERT INTO people (id, email, name, updated_at)
                 VALUES (?1, ?2, ?3, '2026-05-03T00:00:00Z')",
                params![id, format!("{id}@example.com"), name],
            )
            .expect("seed person");
    }

    #[test]
    fn projection_claim_recompute_enqueue_includes_stakeholder_person_subject() {
        let db = test_db();
        let account = make_account("acc-projection-recompute");
        db.upsert_account(&account).unwrap();
        seed_person(&db, "person-projection-recompute", "Fixture Person");

        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 22, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(7);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);
        let signal_id = crate::services::signals::emit(
            &ctx,
            &db,
            "account",
            "acc-projection-recompute",
            "entity_intelligence_updated",
            "ai_enrichment",
            None,
            0.8,
        )
        .expect("emit origin signal");
        let intel = IntelligenceJson {
            executive_assessment_render_policy: None,
            entity_id: "acc-projection-recompute".to_string(),
            entity_type: "account".to_string(),
            stakeholder_insights: vec![StakeholderInsight {
                name: "Fixture Person".to_string(),
                engagement: Some("Highly engaged in account reviews.".to_string()),
                person_id: Some("person-projection-recompute".to_string()),
                ..Default::default()
            }],
            ..Default::default()
        };

        let subjects = super::projection_claim_recompute_subjects(
            "account",
            "acc-projection-recompute",
            &intel,
            &[],
        );
        super::enqueue_projection_claim_recomputes(&ctx, &db, &signal_id, subjects);

        assert_eq!(
            claim_recompute_job_count(&db, "account", "acc-projection-recompute"),
            1
        );
        assert_eq!(
            claim_recompute_job_count(&db, "person", "person-projection-recompute"),
            1
        );
    }

    fn count_query(db: &crate::db::ActionDb, sql: &str) -> i64 {
        db.conn_ref()
            .query_row(sql, [], |row| row.get(0))
            .expect("count query")
    }

    fn sync_success_count(db: &crate::db::ActionDb, source: &str) -> i64 {
        db.conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM sync_metadata
                 WHERE source = ?1
                   AND last_success_at IS NOT NULL
                   AND consecutive_failures = 0",
                params![source],
                |row| row.get(0),
            )
            .expect("sync success count")
    }

    fn projection_claim_rows(
        db: &crate::db::ActionDb,
        entity_id: &str,
    ) -> Vec<(String, String, String, Option<String>)> {
        let mut stmt = db
            .conn_ref()
            .prepare(
                "SELECT claim_type, coalesce(field_path, ''), text, source_asof
                 FROM intelligence_claims
                 WHERE json_valid(subject_ref) = 1
                   AND json_extract(subject_ref, '$.id') = ?1
                   AND claim_state = 'active'
                   AND surfacing_state = 'active'",
            )
            .expect("prepare claim projection query");
        stmt.query_map(params![entity_id], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })
        .expect("query projection claims")
        .collect::<Result<Vec<_>, _>>()
        .expect("collect projection claims")
    }

    fn assert_projection_claim(
        rows: &[(String, String, String, Option<String>)],
        claim_type: &str,
        field_path: &str,
        text_fragment: &str,
    ) {
        let expected_text = text_fragment.to_ascii_lowercase();
        assert!(
            rows.iter().any(|(row_type, row_path, row_text, _)| {
                row_type == claim_type
                    && row_path == field_path
                    && row_text.to_ascii_lowercase().contains(&expected_text)
            }),
            "expected projected claim type={claim_type} field={field_path} containing {text_fragment:?}; got {rows:?}"
        );
    }

    fn assert_projection_source_asof(
        rows: &[(String, String, String, Option<String>)],
        field_path: &str,
        expected: Option<&str>,
    ) {
        let source_asof = rows
            .iter()
            .find_map(|(_, row_path, _, source_asof)| {
                (row_path == field_path).then_some(source_asof.as_deref())
            })
            .expect("expected projection claim field path");
        assert_eq!(source_asof, expected);
    }

    fn coherence_retry_count(db: &crate::db::ActionDb, entity_id: &str) -> i64 {
        db.conn_ref()
            .query_row(
                "SELECT coherence_retry_count FROM entity_quality WHERE entity_id = ?1",
                params![entity_id],
                |row| row.get(0),
            )
            .expect("coherence retry count")
    }

    fn technical_footprint_count(db: &crate::db::ActionDb, account_id: &str) -> i64 {
        db.conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM account_technical_footprint WHERE account_id = ?1",
                params![account_id],
                |row| row.get(0),
            )
            .expect("technical footprint count")
    }

    fn technical_footprint_projection(
        db: &crate::db::ActionDb,
        account_id: &str,
    ) -> Option<(Option<String>, Option<String>, i64, String, String, String)> {
        db.conn_ref()
            .query_row(
                "SELECT support_tier, CAST(csat_score AS TEXT), open_tickets, source, sourced_at, updated_at
                 FROM account_technical_footprint
                 WHERE account_id = ?1",
                params![account_id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                    ))
                },
            )
            .optional()
            .expect("technical footprint projection")
    }

    fn technical_footprint_source_ref_rows(
        db: &crate::db::ActionDb,
        account_id: &str,
    ) -> Vec<(
        String,
        String,
        String,
        Option<String>,
        String,
        Option<String>,
    )> {
        let mut stmt = db
            .conn_ref()
            .prepare(
                "SELECT field, source_system, source_kind, source_value, observed_at, source_record_ref
                 FROM account_source_refs
                 WHERE account_id = ?1
                   AND field LIKE 'technical_footprint.%'
                   AND source_kind != 'source_purged'
                 ORDER BY field, source_system, source_kind, coalesce(source_value, '')",
            )
            .expect("prepare technical footprint source refs");
        stmt.query_map(params![account_id], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
            ))
        })
        .expect("query technical footprint source refs")
        .collect::<Result<Vec<_>, _>>()
        .expect("collect technical footprint source refs")
    }

    fn technical_footprint_marker_ref_count(db: &crate::db::ActionDb, account_id: &str) -> i64 {
        db.conn_ref()
            .query_row(
                "SELECT COUNT(*)
                 FROM account_source_refs
                 WHERE account_id = ?1
                   AND source_record_ref LIKE 'glean_technical_footprint:%'",
                params![account_id],
                |row| row.get(0),
            )
            .expect("technical footprint marker source ref count")
    }

    fn purged_technical_footprint_ref_count(db: &crate::db::ActionDb, account_id: &str) -> i64 {
        db.conn_ref()
            .query_row(
                "SELECT COUNT(*)
                 FROM account_source_refs
                 WHERE account_id = ?1
                   AND field LIKE 'technical_footprint.%'
                   AND source_kind = 'source_purged'",
                params![account_id],
                |row| row.get(0),
            )
            .expect("purged technical footprint source ref count")
    }

    fn account_fact_source_ref_rows(
        db: &crate::db::ActionDb,
        account_id: &str,
    ) -> Vec<(String, String, String, Option<String>, Option<String>)> {
        let mut stmt = db
            .conn_ref()
            .prepare(
                "SELECT field, source_system, source_kind, source_value, source_record_ref
                 FROM account_source_refs
                 WHERE account_id = ?1
                   AND field NOT LIKE 'technical_footprint.%'
                   AND source_kind != 'source_purged'
                 ORDER BY field, source_system, source_kind, coalesce(source_value, ''), coalesce(source_record_ref, '')",
            )
            .expect("prepare account fact source refs");
        stmt.query_map(params![account_id], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        })
        .expect("query account fact source refs")
        .collect::<Result<Vec<_>, _>>()
        .expect("collect account fact source refs")
    }

    fn round_json_numbers(value: &mut serde_json::Value) {
        match value {
            serde_json::Value::Number(number) => {
                if let Some(raw) = number.as_f64() {
                    let rounded = (raw * 1_000_000.0).round() / 1_000_000.0;
                    *number = serde_json::Number::from_f64(rounded)
                        .expect("health projection number is finite");
                }
            }
            serde_json::Value::Array(values) => {
                for value in values {
                    round_json_numbers(value);
                }
            }
            serde_json::Value::Object(map) => {
                for value in map.values_mut() {
                    round_json_numbers(value);
                }
            }
            _ => {}
        }
    }

    fn normalized_health_json(raw: Option<String>) -> Option<serde_json::Value> {
        raw.map(|json| {
            let mut value: serde_json::Value =
                serde_json::from_str(&json).expect("health projection JSON parses");
            round_json_numbers(&mut value);
            value
        })
    }

    fn normalized_health_score(raw: Option<String>) -> Option<String> {
        raw.map(|score| {
            format!(
                "{:.6}",
                score
                    .parse::<f64>()
                    .expect("entity quality health score is numeric")
            )
        })
    }

    fn health_projection(
        db: &crate::db::ActionDb,
        account_id: &str,
    ) -> (
        Option<serde_json::Value>,
        Option<String>,
        Option<serde_json::Value>,
    ) {
        let health_json = db
            .conn_ref()
            .query_row(
                "SELECT health_json FROM entity_assessment WHERE entity_id = ?1",
                params![account_id],
                |row| row.get(0),
            )
            .optional()
            .expect("health_json");
        let quality = db
            .conn_ref()
            .query_row(
                "SELECT CAST(health_score AS TEXT), health_trend
                 FROM entity_quality
                 WHERE entity_id = ?1",
                params![account_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .expect("health quality");
        let (health_score, health_trend) = quality.unwrap_or((None, None));
        (
            normalized_health_json(health_json),
            normalized_health_score(health_score),
            normalized_health_json(health_trend),
        )
    }

    fn seed_finalize_account(db: &crate::db::ActionDb, entity_id: &str) {
        let account = make_account(entity_id);
        db.upsert_account(&account).unwrap();
        db.conn_ref()
            .execute(
                "INSERT INTO entity_assessment (entity_id, entity_type)
                 VALUES (?1, 'account')",
                params![entity_id],
            )
            .expect("seed entity assessment");
        crate::self_healing::quality::ensure_quality_row(db, entity_id, "account");
    }

    fn glean_signal_evidence_rows(
        db: &crate::db::ActionDb,
        entity_id: &str,
    ) -> Vec<(String, String, String, Option<String>, String)> {
        let mut stmt = db
            .conn_ref()
            .prepare(
                "SELECT id, signal_type, data_source, value, printf('%.6f', confidence)
                   FROM signal_events
                  WHERE entity_id = ?1
                    AND data_source LIKE 'glean%'
                  ORDER BY signal_type, data_source, id",
            )
            .expect("prepare Glean signal evidence query");
        stmt.query_map(params![entity_id], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        })
        .expect("query Glean signal evidence")
        .collect::<Result<Vec<_>, _>>()
        .expect("collect Glean signal evidence")
    }

    fn glean_signal_evidence_projection_rows(
        db: &crate::db::ActionDb,
        entity_id: &str,
    ) -> Vec<(String, String, Option<String>, String)> {
        glean_signal_evidence_rows(db, entity_id)
            .into_iter()
            .map(|(_, signal_type, data_source, value, confidence)| {
                (signal_type, data_source, value, confidence)
            })
            .collect()
    }

    fn account_fact_claim_evidence_rows(
        db: &crate::db::ActionDb,
        account_id: &str,
    ) -> Vec<(
        String,
        String,
        String,
        String,
        Option<String>,
        Option<String>,
    )> {
        let mut stmt = db
            .conn_ref()
            .prepare(
                "SELECT claim_type, coalesce(field_path, ''), text, data_source, source_ref, source_asof
                   FROM intelligence_claims
                  WHERE claim_type = 'account_fact'
                    AND claim_state = 'active'
                    AND surfacing_state = 'active'
                    AND json_valid(subject_ref) = 1
                    AND lower(json_extract(subject_ref, '$.kind')) = 'account'
                    AND json_extract(subject_ref, '$.id') = ?1
                  ORDER BY field_path, text, data_source, coalesce(source_ref, '')",
            )
            .expect("prepare account fact claim evidence query");
        stmt.query_map(params![account_id], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
            ))
        })
        .expect("query account fact claim evidence")
        .collect::<Result<Vec<_>, _>>()
        .expect("collect account fact claim evidence")
    }

    fn active_account_fact_claim_count_for_field(
        db: &crate::db::ActionDb,
        account_id: &str,
        field_path: &str,
    ) -> i64 {
        db.conn_ref()
            .query_row(
                "SELECT COUNT(*)
                   FROM intelligence_claims
                  WHERE claim_type = 'account_fact'
                    AND claim_state = 'active'
                    AND surfacing_state = 'active'
                    AND field_path = ?2
                    AND json_valid(subject_ref) = 1
                    AND lower(json_extract(subject_ref, '$.kind')) = 'account'
                    AND json_extract(subject_ref, '$.id') = ?1",
                params![account_id, field_path],
                |row| row.get(0),
            )
            .expect("active account fact claim count")
    }

    fn seed_account_fact_tombstone(
        db: &crate::db::ActionDb,
        account_id: &str,
        field: &str,
        text: &str,
    ) {
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 23, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(23);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);
        crate::services::claims::commit_claim(
            &ctx,
            db,
            crate::services::claims::ClaimProposal {
                id: None,
                expected_claim_version: None,
                subject_ref: serde_json::json!({
                    "kind": "account",
                    "id": account_id,
                })
                .to_string(),
                claim_type: "account_fact".to_string(),
                field_path: Some(format!("account.{field}")),
                topic_key: Some(field.to_string()),
                text: text.to_string(),
                actor: "user:account_fact_claims".to_string(),
                data_source: "user_input".to_string(),
                source_ref: None,
                source_asof: None,
                observed_at: "2026-05-23T12:00:00Z".to_string(),
                provenance_json: "{}".to_string(),
                metadata_json: None,
                thread_id: None,
                temporal_scope: Some(crate::db::claims::TemporalScope::State),
                sensitivity: Some(crate::db::claims::ClaimSensitivity::Internal),
                supersedes: None,
                tombstone: Some(crate::services::claims::TombstoneSpec {
                    retraction_reason: "user_removal".to_string(),
                    expires_at: None,
                }),
            },
        )
        .expect("seed account fact tombstone");
    }

    fn make_enrichment_input(entity_id: &str, entity_dir: &Path) -> EnrichmentInput {
        EnrichmentInput {
            workspace: entity_dir.to_path_buf(),
            entity_dir: entity_dir.to_path_buf(),
            entity_id: entity_id.to_string(),
            entity_type: "account".to_string(),
            prompt: String::new(),
            file_manifest: Vec::new(),
            file_count: 0,
            computed_health: None,
            entity_name: format!("Account {entity_id}"),
            relationship: None,
            intelligence_context: None,
            active_preset: None,
        }
    }

    fn remote_glean_state() -> Arc<AppState> {
        let state = Arc::new(AppState::new());
        state.set_context_mode_atomic(&crate::context_provider::ContextMode::Glean {
            endpoint: "http://127.0.0.1:9/mcp".to_string(),
        });
        state
    }

    fn make_glean_signal_intel(entity_id: &str) -> IntelligenceJson {
        IntelligenceJson {
            executive_assessment_render_policy: None,
            entity_id: entity_id.to_string(),
            entity_type: "account".to_string(),
            enriched_at: "2026-05-03T01:00:00Z".to_string(),
            executive_assessment: Some("mode contract assessment".to_string()),
            org_health: Some(OrgHealthData {
                health_band: Some("green".to_string()),
                health_score: Some(82.0),
                renewal_likelihood: Some("likely".to_string()),
                support_tier: Some("enterprise".to_string()),
                source: "glean_crm".to_string(),
                gathered_at: "2026-05-03T01:00:00Z".to_string(),
                ..Default::default()
            }),
            support_health: Some(SupportHealth {
                open_tickets: Some(3),
                critical_tickets: Some(0),
                avg_resolution_time: Some("8h".to_string()),
                trend: Some("stable".to_string()),
                csat: Some(92.0),
                source: Some("glean_zendesk".to_string()),
            }),
            contract_context: Some(ContractContext {
                current_arr: Some(125_000.0),
                ..Default::default()
            }),
            agreement_outlook: Some(AgreementOutlook {
                confidence: Some("high".to_string()),
                ..Default::default()
            }),
            product_classification: Some(ProductClassification {
                products: vec![ProductInfo {
                    type_: Some("cms".to_string()),
                    arr: Some(125_000.0),
                    ..Default::default()
                }],
            }),
            ..Default::default()
        }
    }

    fn make_glean_full_finalization_intel(entity_id: &str) -> IntelligenceJson {
        IntelligenceJson {
            organizational_changes: vec![OrgChange {
                change_type: "role_change".to_string(),
                person: "Fixture Stakeholder".to_string(),
                from: Some("legacy owner".to_string()),
                to: Some("new owner".to_string()),
                detected_at: Some("2026-05-03T01:00:00Z".to_string()),
                source: Some("glean_slack".to_string()),
                item_source: None,
                discrepancy: None,
            }],
            ..make_glean_signal_intel(entity_id)
        }
    }

    fn visible_materialization_commitment_rows(
        db: &crate::db::ActionDb,
        account_id: &str,
    ) -> Vec<(String, String, Option<String>, String)> {
        let mut stmt = db
            .conn_ref()
            .prepare(
                "SELECT title, owner, target_date, source
                 FROM captured_commitments
                 WHERE account_id = ?1
                 ORDER BY title, owner, coalesce(target_date, ''), source",
            )
            .expect("prepare commitment materialization query");
        stmt.query_map(params![account_id], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })
        .expect("query commitment materialization")
        .collect::<Result<Vec<_>, _>>()
        .expect("collect commitment materialization")
    }

    fn visible_materialization_product_rows(
        db: &crate::db::ActionDb,
        account_id: &str,
    ) -> Vec<(String, String, String)> {
        let mut stmt = db
            .conn_ref()
            .prepare(
                "SELECT name, printf('%.2f', coalesce(arr_portion, -1.0)), source
                 FROM account_products
                 WHERE account_id = ?1
                 ORDER BY name, source",
            )
            .expect("prepare product materialization query");
        stmt.query_map(params![account_id], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .expect("query product materialization")
        .collect::<Result<Vec<_>, _>>()
        .expect("collect product materialization")
    }

    fn make_visible_materialization_intel(entity_id: &str) -> IntelligenceJson {
        IntelligenceJson {
            executive_assessment_render_policy: None,
            entity_id: entity_id.to_string(),
            entity_type: "account".to_string(),
            enriched_at: "2026-05-03T01:00:00Z".to_string(),
            open_commitments: Some(vec![OpenCommitment {
                commitment_id: None,
                description: "Send reliability recap".to_string(),
                owner: Some("vendor".to_string()),
                due_date: Some("2026-06-01".to_string()),
                source: None,
                status: None,
                item_source: None,
                discrepancy: None,
            }]),
            product_adoption: Some(AdoptionSignals {
                feature_adoption: vec!["Core platform: 75%".to_string()],
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    #[test]
    fn manual_and_queue_refresh_share_materialization_path() {
        let state = remote_glean_state();

        for (producer, expected_commitment_prefix, expected_product_source) in [
            (
                crate::intel_queue::EnrichmentProducer::Pty,
                "pty_enrichment:",
                "ai_inference",
            ),
            (
                crate::intel_queue::EnrichmentProducer::Glean,
                "glean_enrichment:",
                "glean",
            ),
        ] {
            let entity_id = format!("acc-shared-materialization-{}", expected_product_source);
            let intel = make_visible_materialization_intel(&entity_id);

            let queue_db = test_db();
            seed_finalize_account(&queue_db, &entity_id);
            let queue_dir = tempfile::tempdir().expect("queue tempdir");
            let queue_input = make_enrichment_input(&entity_id, queue_dir.path());
            run_enrichment_finalize_post_commit(
                &state,
                &queue_db,
                &queue_input,
                &intel,
                &[],
                FinalizeMode::QueueWorker {
                    is_background: false,
                    producer,
                },
            )
            .expect("queue materialization");

            let manual_db = test_db();
            seed_finalize_account(&manual_db, &entity_id);
            let manual_dir = tempfile::tempdir().expect("manual tempdir");
            let manual_input = make_enrichment_input(&entity_id, manual_dir.path());
            run_enrichment_finalize_post_commit(
                &state,
                &manual_db,
                &manual_input,
                &intel,
                &[],
                FinalizeMode::ManualRefresh { producer },
            )
            .expect("manual materialization");

            let queue_commitments = visible_materialization_commitment_rows(&queue_db, &entity_id);
            assert_eq!(
                queue_commitments,
                visible_materialization_commitment_rows(&manual_db, &entity_id),
                "queue and manual refresh must commit identical visible commitments for {producer:?}"
            );
            assert!(
                queue_commitments
                    .iter()
                    .all(|(_, _, _, source)| source.starts_with(expected_commitment_prefix)),
                "commitment materialization should use producer-owned source labels"
            );

            let queue_products = visible_materialization_product_rows(&queue_db, &entity_id);
            assert_eq!(
                queue_products,
                visible_materialization_product_rows(&manual_db, &entity_id),
                "queue and manual refresh must commit identical visible products for {producer:?}"
            );
            assert_eq!(
                queue_products,
                vec![(
                    "Core platform".to_string(),
                    "0.75".to_string(),
                    expected_product_source.to_string()
                )],
                "product materialization should be normalized and producer-owned"
            );
        }
    }

    #[test]
    fn glean_queue_and_manual_refresh_share_finalization_side_effects() {
        let state = remote_glean_state();
        let entity_id = "acc-finalize-shared-side-effects";
        let intel = make_glean_full_finalization_intel(entity_id);

        let queue_db = test_db();
        seed_finalize_account(&queue_db, entity_id);
        let queue_dir = tempfile::tempdir().expect("queue tempdir");
        let queue_input = make_enrichment_input(entity_id, queue_dir.path());
        run_enrichment_finalize_post_commit(
            &state,
            &queue_db,
            &queue_input,
            &intel,
            &[],
            FinalizeMode::QueueWorker {
                is_background: false,
                producer: crate::intel_queue::EnrichmentProducer::Glean,
            },
        )
        .expect("queue Glean finalize");

        let manual_db = test_db();
        seed_finalize_account(&manual_db, entity_id);
        let manual_dir = tempfile::tempdir().expect("manual tempdir");
        let manual_input = make_enrichment_input(entity_id, manual_dir.path());
        run_enrichment_finalize_post_commit(
            &state,
            &manual_db,
            &manual_input,
            &intel,
            &[],
            FinalizeMode::ManualRefresh {
                producer: crate::intel_queue::EnrichmentProducer::Glean,
            },
        )
        .expect("manual Glean finalize");

        let queue_signals = glean_signal_evidence_rows(&queue_db, entity_id);
        assert!(
            !queue_signals.is_empty(),
            "Glean finalization should emit shared signal evidence"
        );
        assert_eq!(
            glean_signal_evidence_projection_rows(&queue_db, entity_id),
            glean_signal_evidence_projection_rows(&manual_db, entity_id),
            "queue and manual Glean finalization must feed identical signal evidence"
        );
        assert_eq!(
            account_fact_claim_evidence_rows(&queue_db, entity_id),
            account_fact_claim_evidence_rows(&manual_db, entity_id),
            "queue and manual Glean finalization must commit identical account fact claims"
        );
        assert_eq!(
            account_fact_source_ref_rows(&queue_db, entity_id),
            account_fact_source_ref_rows(&manual_db, entity_id),
            "queue and manual Glean finalization must write identical account fact source refs"
        );
        assert_eq!(
            claim_recompute_job_count(&queue_db, "account", entity_id),
            claim_recompute_job_count(&manual_db, "account", entity_id),
            "queue and manual Glean finalization must enqueue the same trust recompute jobs"
        );
        assert!(
            claim_recompute_job_count(&queue_db, "account", entity_id) > 0,
            "full finalization fixture should enqueue trust recompute"
        );
        assert_eq!(
            health_projection(&queue_db, entity_id),
            health_projection(&manual_db, entity_id),
            "queue and manual Glean finalization must recompute identical health projections"
        );
        assert_eq!(technical_footprint_count(&queue_db, entity_id), 1);
        assert_eq!(technical_footprint_count(&manual_db, entity_id), 1);
        assert_eq!(
            technical_footprint_projection(&queue_db, entity_id),
            technical_footprint_projection(&manual_db, entity_id),
            "queue and manual Glean finalization must write the same technical footprint fields"
        );
        let slack_payload: String = queue_db
            .conn_ref()
            .query_row(
                "SELECT value FROM signal_events
                 WHERE entity_id = ?1
                   AND signal_type = 'slack_context_updated'",
                params![entity_id],
                |row| row.get(0),
            )
            .expect("Slack context payload");
        assert!(
            slack_payload.contains("itemHashes") && slack_payload.contains("categories"),
            "Slack context signals should persist metadata and identity hashes"
        );
        assert!(
            !slack_payload.contains("Fixture Stakeholder"),
            "Slack context signals must not persist raw source excerpts"
        );
        assert_eq!(
            technical_footprint_source_ref_rows(&queue_db, entity_id),
            technical_footprint_source_ref_rows(&manual_db, entity_id),
            "queue and manual Glean finalization must write the same technical footprint provenance"
        );
    }

    #[test]
    fn materialization_failure_blocks_export_for_visible_generation_side_effects() {
        let state = remote_glean_state();
        let db = test_db();
        let entity_id = "acc-finalize-side-effect-failure-glean";
        seed_finalize_account(&db, entity_id);
        db.conn_ref()
            .execute_batch(
                "CREATE TRIGGER fail_commitment_insert
                 BEFORE INSERT ON captured_commitments
                 WHEN NEW.account_id = 'acc-finalize-side-effect-failure-glean'
                 BEGIN
                   SELECT RAISE(ABORT, 'forced visible materialization failure');
                 END;",
            )
            .expect("install side-effect failure trigger");
        let dir = tempfile::tempdir().expect("tempdir");
        let input = make_enrichment_input(entity_id, dir.path());
        let intel = make_visible_materialization_intel(entity_id);

        let result = run_enrichment_finalize_post_commit(
            &state,
            &db,
            &input,
            &intel,
            &[],
            FinalizeMode::ManualRefresh {
                producer: crate::intel_queue::EnrichmentProducer::Glean,
            },
        );

        assert!(
            result.is_err_and(|error| error.contains("Glean enrichment side-effect sync failed")),
            "Glean side-effect failure must stop finalize success"
        );
        assert!(
            !dir.path().join("intelligence.json").exists(),
            "Glean generated exports must wait until visible materialization completes"
        );
    }

    #[test]
    fn pty_side_effect_failure_stays_non_fatal_after_commit() {
        let state = remote_glean_state();
        let db = test_db();
        let entity_id = "acc-finalize-side-effect-failure-pty";
        seed_finalize_account(&db, entity_id);
        db.conn_ref()
            .execute_batch(
                "CREATE TRIGGER fail_commitment_insert
                 BEFORE INSERT ON captured_commitments
                 WHEN NEW.account_id = 'acc-finalize-side-effect-failure-pty'
                 BEGIN
                   SELECT RAISE(ABORT, 'forced visible materialization failure');
                 END;",
            )
            .expect("install side-effect failure trigger");
        let dir = tempfile::tempdir().expect("tempdir");
        let input = make_enrichment_input(entity_id, dir.path());
        let intel = make_visible_materialization_intel(entity_id);

        run_enrichment_finalize_post_commit(
            &state,
            &db,
            &input,
            &intel,
            &[],
            FinalizeMode::ManualRefresh {
                producer: crate::intel_queue::EnrichmentProducer::Pty,
            },
        )
        .expect("PTY side-effect failure should remain non-fatal");
        assert!(
            dir.path().join("intelligence.json").exists(),
            "PTY generated exports should continue after non-authoritative side-effect failure"
        );
    }

    #[test]
    fn glean_finalization_same_run_key_is_idempotent_across_queue_and_manual() {
        let state = remote_glean_state();
        let db = test_db();
        let entity_id = "acc-finalize-idempotent";
        seed_finalize_account(&db, entity_id);
        let dir = tempfile::tempdir().expect("tempdir");
        let input = make_enrichment_input(entity_id, dir.path());
        let intel = make_glean_signal_intel(entity_id);

        run_enrichment_finalize_post_commit(
            &state,
            &db,
            &input,
            &intel,
            &[],
            FinalizeMode::ManualRefresh {
                producer: crate::intel_queue::EnrichmentProducer::Glean,
            },
        )
        .expect("manual Glean finalize");
        let signals_after_manual = glean_signal_evidence_rows(&db, entity_id);
        let claims_after_manual = account_fact_claim_evidence_rows(&db, entity_id);
        let recompute_after_manual = claim_recompute_job_count(&db, "account", entity_id);
        assert_eq!(
            signal_count(&db, entity_id, "renewal_data_updated"),
            1,
            "first finalization should emit renewal evidence once"
        );

        run_enrichment_finalize_post_commit(
            &state,
            &db,
            &input,
            &intel,
            &[],
            FinalizeMode::QueueWorker {
                is_background: false,
                producer: crate::intel_queue::EnrichmentProducer::Glean,
            },
        )
        .expect("queue Glean finalize retry");

        assert_eq!(
            glean_signal_evidence_rows(&db, entity_id),
            signals_after_manual,
            "queue/manual overlap must not double-count signal evidence"
        );
        assert_eq!(
            account_fact_claim_evidence_rows(&db, entity_id),
            claims_after_manual,
            "queue/manual overlap must not double-promote account fact claims"
        );
        assert_eq!(
            claim_recompute_job_count(&db, "account", entity_id),
            recompute_after_manual,
            "queue/manual overlap must not enqueue duplicate trust recompute jobs"
        );
        assert_eq!(technical_footprint_count(&db, entity_id), 1);
    }

    #[test]
    fn glean_finalization_same_evidence_different_timestamp_is_idempotent() {
        let state = remote_glean_state();
        let db = test_db();
        let entity_id = "acc-finalize-evidence-idempotent";
        seed_finalize_account(&db, entity_id);
        let dir = tempfile::tempdir().expect("tempdir");
        let input = make_enrichment_input(entity_id, dir.path());
        let mut first = make_glean_full_finalization_intel(entity_id);
        first.enriched_at = "2026-05-03T01:00:00Z".to_string();
        first.executive_assessment = Some("first generated assessment".to_string());

        run_enrichment_finalize_post_commit(
            &state,
            &db,
            &input,
            &first,
            &[],
            FinalizeMode::ManualRefresh {
                producer: crate::intel_queue::EnrichmentProducer::Glean,
            },
        )
        .expect("first Glean finalize");
        let signals_after_first = glean_signal_evidence_rows(&db, entity_id);
        let source_refs_after_first = account_source_ref_count(&db, entity_id);
        let claims_after_first = account_fact_claim_evidence_rows(&db, entity_id);

        let mut second = first.clone();
        second.enriched_at = "2026-05-04T01:00:00Z".to_string();
        second.executive_assessment = Some("second generated assessment".to_string());
        run_enrichment_finalize_post_commit(
            &state,
            &db,
            &input,
            &second,
            &[],
            FinalizeMode::QueueWorker {
                is_background: false,
                producer: crate::intel_queue::EnrichmentProducer::Glean,
            },
        )
        .expect("second Glean finalize");

        assert_eq!(
            glean_signal_evidence_rows(&db, entity_id),
            signals_after_first,
            "volatile render metadata must not change Glean side-effect identity"
        );
        assert_eq!(
            account_source_ref_count(&db, entity_id),
            source_refs_after_first,
            "same Glean evidence must not duplicate account source refs"
        );
        assert_eq!(
            account_fact_claim_evidence_rows(&db, entity_id),
            claims_after_first,
            "same Glean evidence must not duplicate account fact claims"
        );
    }

    #[test]
    fn glean_finalization_same_evidence_different_context_clock_is_idempotent() {
        let state = remote_glean_state();
        let db = test_db();
        let entity_id = "acc-finalize-clock-idempotent";
        seed_finalize_account(&db, entity_id);
        let mut intel = make_glean_full_finalization_intel(entity_id);
        intel.enriched_at.clear();
        if let Some(org_health) = intel.org_health.as_mut() {
            org_health.gathered_at.clear();
        }

        let ext = ExternalClients::default();
        let first_clock =
            FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 23, 12, 0, 0).unwrap());
        let first_rng = SeedableRng::new(11);
        let first_ctx = test_ctx(&first_clock, &first_rng, &ext);
        crate::services::glean_finalization::finalize_glean_enrichment(
            &first_ctx,
            &db,
            state.signals.engine.as_ref(),
            crate::services::glean_finalization::GleanFinalizationInput {
                entity_type: "account",
                entity_id,
                intel: &intel,
                preset: None,
            },
        )
        .expect("first Glean finalization");
        let signals_after_first = glean_signal_evidence_rows(&db, entity_id);
        let source_refs_after_first = account_source_ref_count(&db, entity_id);
        let claims_after_first = account_fact_claim_evidence_rows(&db, entity_id);
        let recompute_after_first = claim_recompute_job_count(&db, "account", entity_id);
        let technical_footprint_after_first = technical_footprint_projection(&db, entity_id);
        let technical_source_refs_after_first = technical_footprint_source_ref_rows(&db, entity_id);
        assert!(
            !technical_source_refs_after_first.is_empty(),
            "fixture should exercise technical footprint source refs"
        );
        assert!(
            technical_source_refs_after_first
                .iter()
                .all(|(_, _, _, _, observed_at, _)| observed_at == "1970-01-01T00:00:00Z"),
            "missing Glean source timestamps should use a stable unknown-source timestamp"
        );

        let second_clock =
            FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 24, 12, 0, 0).unwrap());
        let second_rng = SeedableRng::new(12);
        let second_ctx = test_ctx(&second_clock, &second_rng, &ext);
        crate::services::glean_finalization::finalize_glean_enrichment(
            &second_ctx,
            &db,
            state.signals.engine.as_ref(),
            crate::services::glean_finalization::GleanFinalizationInput {
                entity_type: "account",
                entity_id,
                intel: &intel,
                preset: None,
            },
        )
        .expect("second Glean finalization");

        assert_eq!(
            glean_signal_evidence_rows(&db, entity_id),
            signals_after_first,
            "service clock changes must not duplicate Glean signal evidence"
        );
        assert_eq!(
            account_source_ref_count(&db, entity_id),
            source_refs_after_first,
            "service clock changes must not duplicate account source refs"
        );
        assert_eq!(
            account_fact_claim_evidence_rows(&db, entity_id),
            claims_after_first,
            "service clock changes must not duplicate account fact claims"
        );
        assert_eq!(
            claim_recompute_job_count(&db, "account", entity_id),
            recompute_after_first,
            "service clock changes must not enqueue duplicate recompute jobs"
        );
        assert_eq!(
            technical_footprint_projection(&db, entity_id),
            technical_footprint_after_first,
            "service clock changes must not churn technical footprint timestamps"
        );
        assert_eq!(
            technical_footprint_source_ref_rows(&db, entity_id),
            technical_source_refs_after_first,
            "service clock changes must not churn technical source-ref provenance timestamps"
        );
    }

    #[test]
    fn glean_finalization_signal_ids_are_local_to_each_evidence_class() {
        let state = remote_glean_state();
        let db = test_db();
        let entity_id = "acc-finalize-evidence-local";
        seed_finalize_account(&db, entity_id);
        let dir = tempfile::tempdir().expect("tempdir");
        let input = make_enrichment_input(entity_id, dir.path());
        let first = make_glean_signal_intel(entity_id);

        run_enrichment_finalize_post_commit(
            &state,
            &db,
            &input,
            &first,
            &[],
            FinalizeMode::ManualRefresh {
                producer: crate::intel_queue::EnrichmentProducer::Glean,
            },
        )
        .expect("first Glean finalize");
        assert_eq!(signal_count(&db, entity_id, "renewal_data_updated"), 1);
        assert_eq!(signal_count(&db, entity_id, "support_health_updated"), 1);

        let mut second = first.clone();
        if let Some(support_health) = second.support_health.as_mut() {
            support_health.open_tickets = Some(4);
        }
        run_enrichment_finalize_post_commit(
            &state,
            &db,
            &input,
            &second,
            &[],
            FinalizeMode::QueueWorker {
                is_background: false,
                producer: crate::intel_queue::EnrichmentProducer::Glean,
            },
        )
        .expect("second Glean finalize");

        assert_eq!(
            signal_count(&db, entity_id, "renewal_data_updated"),
            1,
            "unchanged renewal evidence must not be duplicated when support evidence changes"
        );
        assert_eq!(
            signal_count(&db, entity_id, "support_health_updated"),
            2,
            "changed support evidence should emit a new support signal"
        );
    }

    #[test]
    fn purge_source_glean_withdraws_finalizer_account_facts_and_preserves_user_fields() {
        let state = remote_glean_state();
        let db = test_db();
        let entity_id = "acc-finalize-purge";
        seed_finalize_account(&db, entity_id);
        db.upsert_account_fact(
            entity_id,
            "support_tier",
            "user-premium",
            "user",
            "2026-05-01T00:00:00Z",
        )
        .expect("seed user-owned support tier");
        db.upsert_account_technical_footprint(
            entity_id,
            Some("[\"sso\"]"),
            Some("enterprise"),
            Some(0.72),
            Some(850),
            None,
            None,
            11,
            Some("live"),
            "user_edit",
        )
        .expect("seed user-owned technical footprint fields");
        let dir = tempfile::tempdir().expect("tempdir");
        let input = make_enrichment_input(entity_id, dir.path());
        let mut intel = make_glean_full_finalization_intel(entity_id);
        if let Some(support_health) = intel.support_health.as_mut() {
            support_health.open_tickets = None;
        }

        run_enrichment_finalize_post_commit(
            &state,
            &db,
            &input,
            &intel,
            &[],
            FinalizeMode::ManualRefresh {
                producer: crate::intel_queue::EnrichmentProducer::Glean,
            },
        )
        .expect("Glean finalize before purge");
        assert!(
            active_glean_account_fact_claim_count(&db, entity_id) > 0,
            "fixture should create active Glean account fact claims"
        );
        assert!(
            visible_account_source_ref_count(&db, entity_id) > 0,
            "fixture should create visible Glean source refs"
        );
        assert_eq!(
            signal_count(&db, entity_id, "stakeholder_change"),
            1,
            "fixture should create a Glean propagation signal"
        );
        assert_eq!(
            signal_count(&db, entity_id, "slack_context_updated"),
            1,
            "fixture should create a Glean Slack-context signal"
        );
        assert!(
            technical_footprint_marker_ref_count(&db, entity_id) > 0,
            "fixture should create technical footprint source refs"
        );
        let ctx = state.live_service_context();
        crate::services::signals::emit(
            &ctx,
            &db,
            "account",
            entity_id,
            "glean_finalization_degraded",
            "glean_synthesis",
            Some("{\"degradedClasses\":[\"account_fact\"]}"),
            0.5,
        )
        .expect("seed degraded marker signal");
        assert_eq!(
            signal_count(&db, entity_id, "glean_finalization_degraded"),
            1,
            "fixture should include a Glean degraded marker signal"
        );

        let report = crate::db::data_lifecycle::purge_source(
            &db,
            crate::db::data_lifecycle::DataSource::Glean,
        )
        .expect("purge Glean");

        assert!(
            report.account_fact_claims_withdrawn > 0,
            "purge should withdraw Glean-produced account fact claims"
        );
        assert!(
            report.account_source_refs_masked > 0,
            "purge should mask Glean-produced source refs"
        );
        assert!(
            report.account_schema_facts_cleared > 0,
            "purge should clear schema projections that are still Glean-owned"
        );
        assert!(
            report.account_fact_recompute_jobs_enqueued > 0,
            "purge should enqueue claim trust recompute after withdrawing Glean facts"
        );
        assert_eq!(
            active_glean_account_fact_claim_count(&db, entity_id),
            0,
            "Glean-produced account fact claims must stop surfacing"
        );
        assert!(
            withdrawn_glean_account_fact_claim_count(&db, entity_id) > 0,
            "Glean-produced account fact claims should remain as withdrawn audit rows"
        );
        assert_eq!(
            visible_account_source_ref_count(&db, entity_id),
            0,
            "masked source refs must be excluded from runtime source refs"
        );
        assert!(
            purged_account_source_ref_count(&db, entity_id) > 0,
            "masked source refs should remain as non-surfacing lifecycle audit rows"
        );
        let account = db
            .get_account(entity_id)
            .expect("read account")
            .expect("account remains");
        assert_eq!(
            account.support_tier.as_deref(),
            Some("user-premium"),
            "user-owned schema fact should not be cleared by Glean purge"
        );
        assert_eq!(account.arr_range_low, None);
        assert_eq!(account.arr_range_high, None);
        assert_eq!(
            account.primary_product, None,
            "Glean-mediated source-less schema projection should be cleared"
        );
        let footprint = db
            .get_account_technical_footprint(entity_id)
            .expect("read footprint")
            .expect("technical footprint row remains");
        assert_eq!(footprint.integrations_json.as_deref(), Some("[\"sso\"]"));
        assert_eq!(footprint.usage_tier.as_deref(), Some("enterprise"));
        assert_eq!(footprint.active_users, Some(850));
        assert_eq!(footprint.services_stage.as_deref(), Some("live"));
        assert_eq!(
            footprint.support_tier, None,
            "Glean-projected support footprint field should be cleared"
        );
        assert_eq!(
            footprint.csat_score, None,
            "Glean-projected support score should be cleared"
        );
        assert_eq!(
            footprint.open_tickets, 11,
            "pre-existing ticket count should not be overwritten when Glean had no ticket evidence"
        );
        assert_eq!(
            signal_count(&db, entity_id, "stakeholder_change"),
            0,
            "Glean propagation signals must be included in Glean purge"
        );
        assert_eq!(
            signal_count(&db, entity_id, "slack_context_updated"),
            0,
            "Glean Slack-context signals must be included in Glean purge"
        );
        assert_eq!(
            signal_count(&db, entity_id, "glean_finalization_degraded"),
            0,
            "Glean degraded markers must be included in Glean purge"
        );
        assert_eq!(
            technical_footprint_marker_ref_count(&db, entity_id),
            0,
            "purge must remove surfacing technical footprint source refs"
        );
        assert!(
            purged_technical_footprint_ref_count(&db, entity_id) > 0,
            "purge should retain masked technical footprint lifecycle refs"
        );

        crate::db::data_lifecycle::purge_source(&db, crate::db::data_lifecycle::DataSource::Glean)
            .expect("second Glean purge");
        let footprint = db
            .get_account_technical_footprint(entity_id)
            .expect("read footprint after retry")
            .expect("technical footprint row remains after retry");
        assert_eq!(footprint.integrations_json.as_deref(), Some("[\"sso\"]"));
        assert_eq!(footprint.usage_tier.as_deref(), Some("enterprise"));
        assert_eq!(footprint.active_users, Some(850));
        assert_eq!(footprint.services_stage.as_deref(), Some("live"));
        assert_eq!(
            footprint.open_tickets, 11,
            "repeated Glean purge must not clear preserved non-Glean ticket counts"
        );
    }

    #[test]
    fn purge_source_glean_preserves_ambiguous_downstream_account_fact_claims() {
        let db = test_db();
        let entity_id = "acc-legacy-glean-purge";
        seed_finalize_account(&db, entity_id);
        db.upsert_account_fact(
            entity_id,
            "arr_range_low",
            "100",
            "Salesforce",
            "2026-05-01T00:00:00Z",
        )
        .expect("seed legacy schema fact");
        db.upsert_account_source_ref(&crate::db::types::AccountSourceRef {
            account_id: entity_id,
            field: "arr_range_low",
            source_system: "Salesforce",
            source_kind: "fact",
            source_value: Some("100"),
            observed_at: "2026-05-01T00:00:00Z",
            reference_id: None,
        })
        .expect("seed downstream source ref without Glean provenance");
        db.upsert_account_fact(
            entity_id,
            "renewal_likelihood",
            "0.80",
            "Salesforce",
            "2026-05-01T00:00:00Z",
        )
        .expect("seed legacy numeric schema fact");
        db.upsert_account_source_ref(&crate::db::types::AccountSourceRef {
            account_id: entity_id,
            field: "renewal_likelihood",
            source_system: "Salesforce",
            source_kind: "fact",
            source_value: Some("0.80"),
            observed_at: "2026-05-01T00:00:00Z",
            reference_id: None,
        })
        .expect("seed downstream numeric source ref without Glean provenance");

        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 23, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(31);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);
        crate::services::claims::commit_claim(
            &ctx,
            &db,
            crate::services::claims::ClaimProposal {
                id: None,
                expected_claim_version: None,
                subject_ref: serde_json::json!({
                    "kind": "account",
                    "id": entity_id,
                })
                .to_string(),
                claim_type: "account_fact".to_string(),
                field_path: Some("account.arr_range_low".to_string()),
                topic_key: Some("arr_range_low".to_string()),
                text: "ARR lower bound: 100".to_string(),
                actor: "system:account_fact_claims".to_string(),
                data_source: "salesforce".to_string(),
                source_ref: Some("account_fact:legacy-arr-low".to_string()),
                source_asof: Some("2026-05-01T00:00:00Z".to_string()),
                observed_at: "2026-05-01T00:00:00Z".to_string(),
                provenance_json: "{}".to_string(),
                metadata_json: None,
                thread_id: None,
                temporal_scope: Some(crate::db::claims::TemporalScope::State),
                sensitivity: Some(crate::db::claims::ClaimSensitivity::Internal),
                supersedes: None,
                tombstone: None,
            },
        )
        .expect("seed legacy account fact claim");

        let report = crate::db::data_lifecycle::purge_source(
            &db,
            crate::db::data_lifecycle::DataSource::Glean,
        )
        .expect("purge Glean");

        assert_eq!(
            report.account_fact_claims_withdrawn, 0,
            "ambiguous downstream account fact claims must not be withdrawn as Glean"
        );
        assert_eq!(
            report.account_source_refs_masked, 0,
            "source refs without an explicit Glean record ref must not be masked"
        );
        assert_eq!(
            report.account_schema_facts_cleared, 0,
            "schema facts without explicit Glean provenance must not be cleared"
        );
        assert_eq!(
            report.account_fact_recompute_jobs_enqueued, 0,
            "preserving ambiguous downstream facts should not enqueue recompute"
        );
        let account = db
            .get_account(entity_id)
            .expect("read account")
            .expect("account remains");
        assert_eq!(account.arr_range_low, Some(100.0));
        assert_eq!(
            account.renewal_likelihood,
            Some(0.80),
            "numeric downstream schema facts should be preserved when Glean provenance is absent"
        );
        assert_eq!(visible_account_source_ref_count(&db, entity_id), 2);
        assert_eq!(
            withdrawn_account_fact_claim_count_by_source_ref(
                &db,
                entity_id,
                "account_fact:legacy-arr-low"
            ),
            0,
            "downstream account fact claim should not withdraw during Glean purge"
        );
    }

    #[test]
    fn purge_source_glean_preserves_newer_downstream_numeric_account_fact_ref() {
        let db = test_db();
        let entity_id = "acc-glean-purge-newer-downstream-ref";
        seed_finalize_account(&db, entity_id);
        db.upsert_account_fact(
            entity_id,
            "renewal_likelihood",
            "0.80",
            "Salesforce",
            "2026-05-02T00:00:00Z",
        )
        .expect("seed downstream schema fact");
        db.upsert_account_source_ref(&crate::db::types::AccountSourceRef {
            account_id: entity_id,
            field: "renewal_likelihood",
            source_system: "glean",
            source_kind: "fact",
            source_value: Some("0.8"),
            observed_at: "2026-05-01T00:00:00Z",
            reference_id: Some("glean_account_fact:numeric-replay"),
        })
        .expect("seed older Glean source ref");
        db.upsert_account_source_ref(&crate::db::types::AccountSourceRef {
            account_id: entity_id,
            field: "renewal_likelihood",
            source_system: "Salesforce",
            source_kind: "fact",
            source_value: Some("0.80"),
            observed_at: "2026-05-02T00:00:00Z",
            reference_id: Some("salesforce:order-form"),
        })
        .expect("seed newer downstream source ref");

        let report = crate::db::data_lifecycle::purge_source(
            &db,
            crate::db::data_lifecycle::DataSource::Glean,
        )
        .expect("purge Glean");

        assert_eq!(
            report.account_source_refs_masked, 1,
            "explicit Glean source refs should still be masked"
        );
        assert_eq!(
            report.account_schema_facts_cleared, 0,
            "newer downstream source refs should protect schema projection even when numeric text differs"
        );
        let account = db
            .get_account(entity_id)
            .expect("read account")
            .expect("account remains");
        assert_eq!(account.renewal_likelihood, Some(0.80));
        assert_eq!(
            account.renewal_likelihood_source.as_deref(),
            Some("Salesforce")
        );
    }

    #[test]
    fn enrichment_side_effects_are_service_owned_idempotent_and_source_aware() {
        let state = remote_glean_state();
        let db = test_db();
        let entity_id = "acc-enrichment-side-effects";
        db.upsert_account(&make_account(entity_id)).unwrap();
        let dir = tempfile::tempdir().expect("tempdir");
        let input = make_enrichment_input(entity_id, dir.path());
        let intel = IntelligenceJson {
            entity_id: entity_id.to_string(),
            entity_type: "account".to_string(),
            open_commitments: Some(vec![OpenCommitment {
                commitment_id: None,
                description: "Send reliability recap".to_string(),
                owner: Some("vendor".to_string()),
                due_date: Some("2026-06-01".to_string()),
                source: Some("local".to_string()),
                status: None,
                item_source: None,
                discrepancy: None,
            }]),
            product_adoption: Some(AdoptionSignals {
                feature_adoption: vec!["Core platform: 75%".to_string()],
                source: Some("user_correction".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        };

        for _ in 0..2 {
            crate::intel_queue::run_enrichment_post_commit_side_effects(
                &state,
                &input,
                &db,
                &intel,
                crate::intel_queue::EnrichmentProducer::Pty,
            )
            .expect("PTY side effects");
        }

        let commitment_rows: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM captured_commitments WHERE account_id = ?1",
                params![entity_id],
                |row| row.get(0),
            )
            .expect("commitment count");
        assert_eq!(
            commitment_rows, 1,
            "retries should not duplicate semantic commitments"
        );
        let commitment_source: String = db
            .conn_ref()
            .query_row(
                "SELECT source FROM captured_commitments WHERE account_id = ?1",
                params![entity_id],
                |row| row.get(0),
            )
            .expect("commitment source");
        assert!(
            commitment_source.starts_with("pty_enrichment:"),
            "PTY side effects must not be mislabeled as Glean"
        );
        let product_rows: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM account_products WHERE account_id = ?1",
                params![entity_id],
                |row| row.get(0),
            )
            .expect("product count");
        assert_eq!(product_rows, 1);
        let product_source: String = db
            .conn_ref()
            .query_row(
                "SELECT source FROM account_products WHERE account_id = ?1",
                params![entity_id],
                |row| row.get(0),
            )
            .expect("product source");
        assert_eq!(
            product_source, "ai_inference",
            "PTY product side effects must use producer-owned source, not model-owned source text"
        );
        let glean_signal_rows: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM signal_events
                 WHERE entity_id = ?1
                   AND data_source = 'glean'",
                params![entity_id],
                |row| row.get(0),
            )
            .expect("Glean signal count");
        assert_eq!(
            glean_signal_rows, 0,
            "PTY side effects must not emit Glean-scoped signals"
        );
        let user_correction_signal_rows: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM signal_events
                 WHERE entity_id = ?1
                   AND data_source = 'user_correction'",
                params![entity_id],
                |row| row.get(0),
            )
            .expect("user correction signal count");
        assert_eq!(
            user_correction_signal_rows, 0,
            "PTY side effects must not trust model-provided product source labels"
        );
        assert_eq!(
            signal_count(&db, entity_id, "commitment_captured"),
            1,
            "commitment signal should be idempotent"
        );
        assert_eq!(
            signal_count(&db, entity_id, "product_data_updated"),
            1,
            "product signal should be idempotent"
        );

        for _ in 0..2 {
            crate::intel_queue::run_enrichment_post_commit_side_effects(
                &state,
                &input,
                &db,
                &intel,
                crate::intel_queue::EnrichmentProducer::Glean,
            )
            .expect("Glean side effects");
        }

        let commitment_sources: Vec<String> = {
            let mut stmt = db
                .conn_ref()
                .prepare(
                    "SELECT source FROM captured_commitments
                     WHERE account_id = ?1
                     ORDER BY source",
                )
                .expect("prepare commitment source query");
            stmt.query_map(params![entity_id], |row| row.get::<_, String>(0))
                .expect("query commitment sources")
                .collect::<Result<Vec<_>, _>>()
                .expect("collect commitment sources")
        };
        assert_eq!(
            commitment_sources.len(),
            2,
            "PTY and Glean commitments should coexist as producer-owned evidence"
        );
        assert!(commitment_sources
            .iter()
            .any(|source| source.starts_with("pty_enrichment:")));
        assert!(commitment_sources
            .iter()
            .any(|source| source.starts_with("glean_enrichment:")));

        let product_sources: Vec<String> = {
            let mut stmt = db
                .conn_ref()
                .prepare(
                    "SELECT source FROM account_products
                     WHERE account_id = ?1
                     ORDER BY source",
                )
                .expect("prepare product source query");
            stmt.query_map(params![entity_id], |row| row.get::<_, String>(0))
                .expect("query product sources")
                .collect::<Result<Vec<_>, _>>()
                .expect("collect product sources")
        };
        assert_eq!(
            product_sources,
            vec!["ai_inference".to_string(), "glean".to_string()],
            "Glean product evidence should not take ownership of existing non-Glean product rows"
        );

        let report = crate::db::data_lifecycle::purge_source(
            &db,
            crate::db::data_lifecycle::DataSource::Glean,
        )
        .expect("purge Glean side effects");
        assert_eq!(report.enrichment_commitments_deleted, 1);
        assert_eq!(report.account_products_deleted, 1);

        let remaining_commitment_sources: Vec<String> = {
            let mut stmt = db
                .conn_ref()
                .prepare(
                    "SELECT source FROM captured_commitments
                     WHERE account_id = ?1
                     ORDER BY source",
                )
                .expect("prepare remaining commitment query");
            stmt.query_map(params![entity_id], |row| row.get::<_, String>(0))
                .expect("query remaining commitments")
                .collect::<Result<Vec<_>, _>>()
                .expect("collect remaining commitments")
        };
        assert_eq!(remaining_commitment_sources.len(), 1);
        assert!(
            remaining_commitment_sources[0].starts_with("pty_enrichment:"),
            "Glean purge should leave non-Glean commitment evidence intact"
        );

        let remaining_product_sources: Vec<String> = {
            let mut stmt = db
                .conn_ref()
                .prepare(
                    "SELECT source FROM account_products
                     WHERE account_id = ?1
                     ORDER BY source",
                )
                .expect("prepare remaining product query");
            stmt.query_map(params![entity_id], |row| row.get::<_, String>(0))
                .expect("query remaining products")
                .collect::<Result<Vec<_>, _>>()
                .expect("collect remaining products")
        };
        assert_eq!(
            remaining_product_sources,
            vec!["ai_inference".to_string()],
            "Glean purge should leave non-Glean product evidence intact"
        );
    }

    #[test]
    fn intel_queue_keeps_enrichment_side_effect_writes_behind_service_boundary() {
        let source = include_str!("../intel_queue.rs");
        assert!(
            !source.contains("INSERT OR IGNORE INTO captured_commitments"),
            "queue orchestration must not write captured commitments directly"
        );
        assert!(
            !source.contains("upsert_account_product("),
            "queue orchestration must not write account products directly"
        );
    }

    #[test]
    fn glean_finalization_preserves_tombstone_and_reports_pii_safe_degraded_marker() {
        let state = remote_glean_state();
        let db = test_db();
        let entity_id = "acc-finalize-tombstone";
        seed_finalize_account(&db, entity_id);
        seed_account_fact_tombstone(&db, entity_id, "support_tier", "Support tier: enterprise");
        let intel = IntelligenceJson {
            executive_assessment_render_policy: None,
            entity_id: entity_id.to_string(),
            entity_type: "account".to_string(),
            enriched_at: "2026-05-23T12:00:00Z".to_string(),
            executive_assessment: Some("raw finalization prose must not leak".to_string()),
            org_health: Some(OrgHealthData {
                support_tier: Some("enterprise".to_string()),
                source: "glean_crm".to_string(),
                gathered_at: "2026-05-23T12:00:00Z".to_string(),
                ..Default::default()
            }),
            ..Default::default()
        };
        let ctx = state.live_service_context();

        let report = crate::services::glean_finalization::finalize_glean_enrichment(
            &ctx,
            &db,
            state.signals.engine.as_ref(),
            crate::services::glean_finalization::GleanFinalizationInput {
                entity_type: "account",
                entity_id,
                intel: &intel,
                preset: None,
            },
        )
        .expect("Glean finalization should degrade, not bypass lifecycle");

        assert!(
            report.warnings.iter().any(|warning| {
                warning.code == "account_fact_claim_write_failed"
                    && warning.pii_safe_detail.is_some()
            }),
            "blocked account fact claim should produce a typed PII-safe warning"
        );
        assert!(
            report.degraded_classes.contains(
                &crate::services::glean_finalization::GleanFinalizationSideEffect::AccountFact
            ),
            "account fact lifecycle block should be represented as degraded account_fact"
        );
        assert_eq!(
            active_account_fact_claim_count_for_field(&db, entity_id, "account.support_tier"),
            0,
            "Glean finalization must not resurrect a tombstoned account fact"
        );
        let footprint_support_tier: Option<String> = db
            .get_account_technical_footprint(entity_id)
            .expect("read technical footprint")
            .and_then(|footprint| footprint.support_tier);
        assert_eq!(
            footprint_support_tier, None,
            "technical footprint must not bypass a tombstoned support-tier account fact"
        );
        assert_eq!(
            signal_count(&db, entity_id, "glean_finalization_degraded"),
            1,
            "degraded finalization should leave a durable marker"
        );
        let marker_payload: String = db
            .conn_ref()
            .query_row(
                "SELECT value FROM signal_events
                 WHERE entity_id = ?1
                   AND signal_type = 'glean_finalization_degraded'",
                params![entity_id],
                |row| row.get(0),
            )
            .expect("degraded marker payload");
        assert!(marker_payload.contains("account_fact"));
        assert!(!marker_payload.contains("enterprise"));
        assert!(!marker_payload.contains("raw finalization prose"));
    }

    #[test]
    fn glean_finalization_signal_payloads_are_metadata_only() {
        let state = remote_glean_state();
        let db = test_db();
        let entity_id = "acc-finalize-signal-metadata";
        seed_finalize_account(&db, entity_id);
        let mut intel = make_glean_signal_intel(entity_id);
        intel.org_health = Some(OrgHealthData {
            support_tier: Some("raw support tier token".to_string()),
            renewal_likelihood: Some("raw renewal likelihood token".to_string()),
            source: "glean_crm".to_string(),
            gathered_at: "2026-05-23T12:00:00Z".to_string(),
            ..Default::default()
        });
        intel.support_health = Some(SupportHealth {
            open_tickets: Some(4),
            avg_resolution_time: Some("raw support health token".to_string()),
            trend: Some("raw support trend token".to_string()),
            csat: Some(91.0),
            source: Some("glean_zendesk".to_string()),
            critical_tickets: None,
        });
        intel.competitive_context = vec![CompetitiveInsight {
            competitor: "raw competitor token".to_string(),
            threat_level: Some("evaluation".to_string()),
            context: Some("raw competitive context token".to_string()),
            source: Some("glean_slack".to_string()),
            detected_at: Some("2026-05-23".to_string()),
            item_source: None,
            discrepancy: None,
        }];
        intel.organizational_changes = vec![OrgChange {
            change_type: "role_change".to_string(),
            person: "raw person token".to_string(),
            from: Some("raw from token".to_string()),
            to: Some("raw to token".to_string()),
            detected_at: Some("2026-05-23T12:00:00Z".to_string()),
            source: Some("glean_slack".to_string()),
            item_source: None,
            discrepancy: None,
        }];
        intel.gong_call_summaries = vec![GongCallSummary {
            title: "raw gong title token".to_string(),
            date: "2026-05-23".to_string(),
            participants: vec!["raw participant token".to_string()],
            key_topics: "raw gong topics token".to_string(),
            sentiment: "negative".to_string(),
        }];
        intel.health = Some(AccountHealth {
            dimensions: RelationshipDimensions {
                key_advocate_health: DimensionScore {
                    score: 10.0,
                    weight: 1.0,
                    evidence: vec!["raw champion evidence token".to_string()],
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        });

        crate::services::glean_finalization::finalize_glean_enrichment(
            &state.live_service_context(),
            &db,
            state.signals.engine.as_ref(),
            crate::services::glean_finalization::GleanFinalizationInput {
                entity_type: "account",
                entity_id,
                intel: &intel,
                preset: None,
            },
        )
        .expect("Glean finalization");

        let signal_values: Vec<String> = db
            .conn_ref()
            .prepare(
                "SELECT coalesce(value, '')
                   FROM signal_events
                  WHERE entity_id = ?1
                  ORDER BY signal_type",
            )
            .expect("prepare signal value query")
            .query_map(params![entity_id], |row| row.get::<_, String>(0))
            .expect("query signal values")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect signal values");
        let all_values = signal_values.join("\n");
        assert!(
            all_values.contains("payloadHash"),
            "Glean signals should carry metadata-only payload identities"
        );
        for forbidden in [
            "raw support tier token",
            "raw renewal likelihood token",
            "raw support health token",
            "raw support trend token",
            "raw competitor token",
            "raw competitive context token",
            "raw person token",
            "raw from token",
            "raw to token",
            "raw gong title token",
            "raw participant token",
            "raw gong topics token",
            "raw champion evidence token",
        ] {
            assert!(
                !all_values.contains(forbidden),
                "Glean signal payload leaked raw source text: {forbidden}"
            );
        }
    }

    #[test]
    fn support_tier_tombstone_clears_mixed_source_technical_footprint_ref() {
        let state = remote_glean_state();
        let db = test_db();
        let entity_id = "acc-finalize-tombstone-mixed-footprint";
        seed_finalize_account(&db, entity_id);
        let intel = make_glean_signal_intel(entity_id);

        crate::services::glean_finalization::finalize_glean_enrichment(
            &state.live_service_context(),
            &db,
            state.signals.engine.as_ref(),
            crate::services::glean_finalization::GleanFinalizationInput {
                entity_type: "account",
                entity_id,
                intel: &intel,
                preset: None,
            },
        )
        .expect("initial Glean finalization");
        assert!(
            db.get_account_source_refs(entity_id)
                .expect("source refs")
                .iter()
                .any(|source_ref| source_ref.field == "technical_footprint.support_tier"),
            "initial Glean finalization should create support-tier provenance"
        );

        db.update_technical_footprint_field(entity_id, "csat_score", "88")
            .expect("user edits a different technical footprint field");
        seed_account_fact_tombstone(&db, entity_id, "support_tier", "Support tier: enterprise");

        crate::services::glean_finalization::finalize_glean_enrichment(
            &state.live_service_context(),
            &db,
            state.signals.engine.as_ref(),
            crate::services::glean_finalization::GleanFinalizationInput {
                entity_type: "account",
                entity_id,
                intel: &intel,
                preset: None,
            },
        )
        .expect("tombstoned Glean finalization");

        let footprint = db
            .get_account_technical_footprint(entity_id)
            .expect("read technical footprint")
            .expect("technical footprint row remains");
        assert_eq!(
            footprint.support_tier, None,
            "support-tier tombstone must clear Glean projection even on mixed-source rows"
        );
        assert_eq!(
            footprint.csat_score,
            Some(88.0),
            "clearing tombstoned support tier must preserve unrelated user edits"
        );
        assert!(
            db.get_account_source_refs(entity_id)
                .expect("source refs after tombstone")
                .iter()
                .all(|source_ref| source_ref.field != "technical_footprint.support_tier"),
            "support-tier tombstone must retire live technical-footprint source refs"
        );
    }

    #[test]
    fn glean_finalization_preserves_user_corrected_account_facts() {
        let state = remote_glean_state();
        let db = test_db();
        let entity_id = "acc-finalize-user-correction";
        seed_finalize_account(&db, entity_id);
        db.upsert_account_fact(
            entity_id,
            "support_tier",
            "standard",
            "user_correction",
            "2026-05-20T00:00:00Z",
        )
        .expect("seed user-corrected support tier");
        db.upsert_account_fact(
            entity_id,
            "renewal_likelihood",
            "0.20",
            "user",
            "2026-05-20T00:00:00Z",
        )
        .expect("seed user-owned renewal likelihood");
        db.upsert_account_fact(
            entity_id,
            "active_subscription_count",
            "2",
            "user",
            "2026-05-20T00:00:00Z",
        )
        .expect("seed user-owned source-less subscription count");
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 23, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(24);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);
        crate::services::claims::commit_claim(
            &ctx,
            &db,
            crate::services::claims::ClaimProposal {
                id: None,
                expected_claim_version: None,
                subject_ref: serde_json::json!({
                    "kind": "account",
                    "id": entity_id,
                })
                .to_string(),
                claim_type: "account_fact".to_string(),
                field_path: Some("account.support_tier".to_string()),
                topic_key: Some("support_tier".to_string()),
                text: "Support tier: standard".to_string(),
                actor: "user:account_fact_claims".to_string(),
                data_source: "user_input".to_string(),
                source_ref: None,
                source_asof: Some("2026-05-20T00:00:00Z".to_string()),
                observed_at: "2026-05-20T00:00:00Z".to_string(),
                provenance_json: "{}".to_string(),
                metadata_json: None,
                thread_id: None,
                temporal_scope: Some(crate::db::claims::TemporalScope::State),
                sensitivity: Some(crate::db::claims::ClaimSensitivity::Internal),
                supersedes: None,
                tombstone: None,
            },
        )
        .expect("seed user-corrected claim");
        crate::services::claims::commit_claim(
            &ctx,
            &db,
            crate::services::claims::ClaimProposal {
                id: None,
                expected_claim_version: None,
                subject_ref: serde_json::json!({
                    "kind": "account",
                    "id": entity_id,
                })
                .to_string(),
                claim_type: "account_fact".to_string(),
                field_path: Some("account.active_subscription_count".to_string()),
                topic_key: Some("active_subscription_count".to_string()),
                text: "Active subscriptions: 2".to_string(),
                actor: "user:account_fact_claims".to_string(),
                data_source: "user_input".to_string(),
                source_ref: None,
                source_asof: Some("2026-05-20T00:00:00Z".to_string()),
                observed_at: "2026-05-20T00:00:00Z".to_string(),
                provenance_json: "{}".to_string(),
                metadata_json: None,
                thread_id: None,
                temporal_scope: Some(crate::db::claims::TemporalScope::State),
                sensitivity: Some(crate::db::claims::ClaimSensitivity::Internal),
                supersedes: None,
                tombstone: None,
            },
        )
        .expect("seed user-corrected source-less claim");

        let intel = make_glean_signal_intel(entity_id);
        crate::services::glean_finalization::finalize_glean_enrichment(
            &state.live_service_context(),
            &db,
            state.signals.engine.as_ref(),
            crate::services::glean_finalization::GleanFinalizationInput {
                entity_type: "account",
                entity_id,
                intel: &intel,
                preset: None,
            },
        )
        .expect("Glean finalization");

        let account = db
            .get_account(entity_id)
            .expect("read account")
            .expect("account remains");
        assert_eq!(
            account.support_tier.as_deref(),
            Some("standard"),
            "Glean support tier must not overwrite a user correction"
        );
        assert_eq!(
            account.support_tier_source.as_deref(),
            Some("user_correction")
        );
        assert_eq!(
            account.renewal_likelihood,
            Some(0.20),
            "Glean renewal likelihood must not overwrite a user-owned schema fact"
        );
        assert_eq!(account.renewal_likelihood_source.as_deref(), Some("user"));
        assert_eq!(
            account.active_subscription_count,
            Some(2),
            "Glean source-less schema facts must not overwrite a conflicting user claim"
        );
        assert_eq!(
            active_account_fact_claim_count_for_field(&db, entity_id, "account.support_tier"),
            2,
            "Glean finalization should preserve the user claim and land the competing source claim"
        );
        let support_claim_sources: Vec<String> = db
            .conn_ref()
            .prepare(
                "SELECT data_source
                   FROM intelligence_claims
                  WHERE claim_type = 'account_fact'
                    AND field_path = 'account.support_tier'
                    AND claim_state = 'active'
                    AND surfacing_state = 'active'
                    AND json_extract(subject_ref, '$.id') = ?1
                  ORDER BY data_source",
            )
            .expect("prepare support claim source query")
            .query_map(params![entity_id], |row| row.get::<_, String>(0))
            .expect("query support claim sources")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect support claim sources");
        assert_eq!(
            support_claim_sources,
            vec!["user_input".to_string(), "zendesk".to_string()]
        );
        assert_eq!(
            active_account_fact_claim_count_for_field(
                &db,
                entity_id,
                "account.active_subscription_count"
            ),
            2,
            "source-less facts should also compete in claims instead of overwriting user input"
        );
        let subscription_claim_sources: Vec<String> = db
            .conn_ref()
            .prepare(
                "SELECT data_source
                   FROM intelligence_claims
                  WHERE claim_type = 'account_fact'
                    AND field_path = 'account.active_subscription_count'
                    AND claim_state = 'active'
                    AND surfacing_state = 'active'
                    AND json_extract(subject_ref, '$.id') = ?1
                  ORDER BY data_source",
            )
            .expect("prepare subscription claim source query")
            .query_map(params![entity_id], |row| row.get::<_, String>(0))
            .expect("query subscription claim sources")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect subscription claim sources");
        assert_eq!(
            subscription_claim_sources,
            vec!["Salesforce".to_string(), "user_input".to_string()]
        );
    }

    #[test]
    fn glean_finalization_preserves_user_owned_arr_projection() {
        let state = remote_glean_state();
        let db = test_db();
        let entity_id = "acc-finalize-user-arr";
        seed_finalize_account(&db, entity_id);
        db.upsert_account_fact(
            entity_id,
            "arr_range_low",
            "100000",
            "user",
            "2026-05-20T00:00:00Z",
        )
        .expect("seed user-owned ARR low");
        db.upsert_account_fact(
            entity_id,
            "arr_range_high",
            "100000",
            "user",
            "2026-05-20T00:00:00Z",
        )
        .expect("seed user-owned ARR high");
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 23, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(25);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);
        crate::services::claims::commit_claim(
            &ctx,
            &db,
            crate::services::claims::ClaimProposal {
                id: None,
                expected_claim_version: None,
                subject_ref: serde_json::json!({
                    "kind": "account",
                    "id": entity_id,
                })
                .to_string(),
                claim_type: "account_fact".to_string(),
                field_path: Some("account.arr".to_string()),
                topic_key: Some("arr".to_string()),
                text: "arr: 100,000".to_string(),
                actor: "user:account_fact_claims".to_string(),
                data_source: "user_input".to_string(),
                source_ref: None,
                source_asof: Some("2026-05-20T00:00:00Z".to_string()),
                observed_at: "2026-05-20T00:00:00Z".to_string(),
                provenance_json: "{}".to_string(),
                metadata_json: None,
                thread_id: None,
                temporal_scope: Some(crate::db::claims::TemporalScope::State),
                sensitivity: Some(crate::db::claims::ClaimSensitivity::Internal),
                supersedes: None,
                tombstone: None,
            },
        )
        .expect("seed user-owned ARR claim");

        let intel = make_glean_signal_intel(entity_id);
        crate::services::glean_finalization::finalize_glean_enrichment(
            &state.live_service_context(),
            &db,
            state.signals.engine.as_ref(),
            crate::services::glean_finalization::GleanFinalizationInput {
                entity_type: "account",
                entity_id,
                intel: &intel,
                preset: None,
            },
        )
        .expect("Glean finalization");

        let account = db
            .get_account(entity_id)
            .expect("read account")
            .expect("account remains");
        assert_eq!(
            account.arr_range_low,
            Some(100_000.0),
            "Glean ARR must not overwrite source-less schema owned by a user claim"
        );
        assert_eq!(account.arr_range_high, Some(100_000.0));
        assert_eq!(
            active_account_fact_claim_count_for_field(&db, entity_id, "account.arr"),
            2,
            "Glean ARR should land as competing evidence without taking over the schema projection"
        );
        let arr_claim_sources: Vec<String> = db
            .conn_ref()
            .prepare(
                "SELECT data_source
                   FROM intelligence_claims
                  WHERE claim_type = 'account_fact'
                    AND field_path = 'account.arr'
                    AND claim_state = 'active'
                    AND surfacing_state = 'active'
                    AND json_extract(subject_ref, '$.id') = ?1
                  ORDER BY data_source",
            )
            .expect("prepare ARR claim source query")
            .query_map(params![entity_id], |row| row.get::<_, String>(0))
            .expect("query ARR claim sources")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect ARR claim sources");
        assert_eq!(
            arr_claim_sources,
            vec!["Salesforce".to_string(), "user_input".to_string()]
        );
    }

    #[test]
    fn older_same_source_replay_does_not_fork_account_fact_claims() {
        let state = remote_glean_state();
        let db = test_db();
        let entity_id = "acc-finalize-stale-replay";
        seed_finalize_account(&db, entity_id);
        let newer_intel = make_glean_signal_intel(entity_id);
        crate::services::glean_finalization::finalize_glean_enrichment(
            &state.live_service_context(),
            &db,
            state.signals.engine.as_ref(),
            crate::services::glean_finalization::GleanFinalizationInput {
                entity_type: "account",
                entity_id,
                intel: &newer_intel,
                preset: None,
            },
        )
        .expect("newer Glean finalization");

        let mut older_intel = make_glean_signal_intel(entity_id);
        older_intel.enriched_at = "2026-05-01T01:00:00Z".to_string();
        if let Some(org_health) = older_intel.org_health.as_mut() {
            org_health.gathered_at = "2026-05-01T01:00:00Z".to_string();
            org_health.support_tier = Some("standard".to_string());
        }
        if let Some(contract) = older_intel.contract_context.as_mut() {
            contract.current_arr = Some(100_000.0);
        }
        crate::services::glean_finalization::finalize_glean_enrichment(
            &state.live_service_context(),
            &db,
            state.signals.engine.as_ref(),
            crate::services::glean_finalization::GleanFinalizationInput {
                entity_type: "account",
                entity_id,
                intel: &older_intel,
                preset: None,
            },
        )
        .expect("older Glean replay");

        let account = db
            .get_account(entity_id)
            .expect("read account")
            .expect("account remains");
        assert_eq!(account.support_tier.as_deref(), Some("enterprise"));
        assert_eq!(account.arr_range_low, Some(125_000.0));
        assert_eq!(account.arr_range_high, Some(125_000.0));
        let stale_claims: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*)
                   FROM intelligence_claims
                  WHERE claim_type = 'account_fact'
                    AND claim_state = 'active'
                    AND surfacing_state = 'active'
                    AND json_extract(subject_ref, '$.id') = ?1
                    AND (
                        (field_path = 'account.support_tier' AND text LIKE '%standard%')
                        OR (field_path = 'account.arr' AND text LIKE '%100,000%')
                    )",
                params![entity_id],
                |row| row.get(0),
            )
            .expect("stale claim count");
        assert_eq!(
            stale_claims, 0,
            "older same-source replay must not fork stale active account fact claims"
        );
    }

    #[test]
    fn glean_finalization_successful_retry_clears_degraded_marker() {
        let state = remote_glean_state();
        let db = test_db();
        let entity_id = "acc-finalize-recovery";
        seed_finalize_account(&db, entity_id);
        let intel = make_glean_signal_intel(entity_id);
        let ctx = state.live_service_context();

        db.conn_ref()
            .execute_batch(
                "CREATE TRIGGER fail_account_fact_claim_insert
                 BEFORE INSERT ON intelligence_claims
                 WHEN NEW.claim_type = 'account_fact'
                   AND json_valid(NEW.subject_ref) = 1
                   AND json_extract(NEW.subject_ref, '$.id') = 'acc-finalize-recovery'
                 BEGIN
                   SELECT RAISE(ABORT, 'forced account fact claim failure');
                 END;",
            )
            .expect("install account fact failure trigger");

        let first = crate::services::glean_finalization::finalize_glean_enrichment(
            &ctx,
            &db,
            state.signals.engine.as_ref(),
            crate::services::glean_finalization::GleanFinalizationInput {
                entity_type: "account",
                entity_id,
                intel: &intel,
                preset: None,
            },
        )
        .expect("degraded Glean finalization");
        assert!(
            first.degraded_classes.contains(
                &crate::services::glean_finalization::GleanFinalizationSideEffect::AccountFact
            ),
            "first run should record account-fact degradation"
        );
        assert_eq!(
            signal_count(&db, entity_id, "glean_finalization_degraded"),
            1
        );
        assert_eq!(
            signal_count(&db, entity_id, "renewal_data_updated"),
            1,
            "successful signal classes should be emitted once before retry"
        );

        db.conn_ref()
            .execute("DROP TRIGGER fail_account_fact_claim_insert", [])
            .expect("remove account fact failure trigger");
        let second = crate::services::glean_finalization::finalize_glean_enrichment(
            &ctx,
            &db,
            state.signals.engine.as_ref(),
            crate::services::glean_finalization::GleanFinalizationInput {
                entity_type: "account",
                entity_id,
                intel: &intel,
                preset: None,
            },
        )
        .expect("successful Glean finalization retry");

        assert!(
            second.degraded_classes.is_empty(),
            "successful retry should not report stale degradation"
        );
        assert_eq!(
            signal_count(&db, entity_id, "glean_finalization_degraded"),
            0,
            "successful retry should clear the durable degraded marker for the same run key"
        );
        assert_eq!(
            signal_count(&db, entity_id, "renewal_data_updated"),
            1,
            "retry must not duplicate signal evidence that already succeeded"
        );
        assert!(
            account_fact_claim_count(&db, entity_id) > 0,
            "retry should complete previously failed account fact claim writes"
        );
    }

    #[test]
    fn glean_finalization_successful_changed_retry_clears_stale_degraded_marker() {
        let state = remote_glean_state();
        let db = test_db();
        let entity_id = "acc-finalize-recovery-changed";
        seed_finalize_account(&db, entity_id);
        let intel = make_glean_signal_intel(entity_id);
        let ctx = state.live_service_context();

        db.conn_ref()
            .execute_batch(
                "CREATE TRIGGER fail_changed_account_fact_claim_insert
                 BEFORE INSERT ON intelligence_claims
                 WHEN NEW.claim_type = 'account_fact'
                   AND json_valid(NEW.subject_ref) = 1
                   AND json_extract(NEW.subject_ref, '$.id') = 'acc-finalize-recovery-changed'
                 BEGIN
                   SELECT RAISE(ABORT, 'forced account fact claim failure');
                 END;",
            )
            .expect("install account fact failure trigger");

        let first = crate::services::glean_finalization::finalize_glean_enrichment(
            &ctx,
            &db,
            state.signals.engine.as_ref(),
            crate::services::glean_finalization::GleanFinalizationInput {
                entity_type: "account",
                entity_id,
                intel: &intel,
                preset: None,
            },
        )
        .expect("degraded Glean finalization");
        assert!(
            first.degraded_classes.contains(
                &crate::services::glean_finalization::GleanFinalizationSideEffect::AccountFact
            ),
            "first run should record account-fact degradation"
        );
        assert_eq!(
            signal_count(&db, entity_id, "glean_finalization_degraded"),
            1
        );

        db.conn_ref()
            .execute("DROP TRIGGER fail_changed_account_fact_claim_insert", [])
            .expect("remove account fact failure trigger");
        let mut second_intel = make_glean_signal_intel(entity_id);
        second_intel.enriched_at = "2026-05-23T12:05:00Z".to_string();
        if let Some(org_health) = second_intel.org_health.as_mut() {
            org_health.support_tier = Some("premium".to_string());
        }
        let second = crate::services::glean_finalization::finalize_glean_enrichment(
            &ctx,
            &db,
            state.signals.engine.as_ref(),
            crate::services::glean_finalization::GleanFinalizationInput {
                entity_type: "account",
                entity_id,
                intel: &second_intel,
                preset: None,
            },
        )
        .expect("successful changed Glean finalization retry");

        assert!(
            second.degraded_classes.is_empty(),
            "successful changed retry should not report stale degradation"
        );
        assert_eq!(
            signal_count(&db, entity_id, "glean_finalization_degraded"),
            0,
            "successful changed retry should clear older degraded markers for the entity"
        );
    }

    #[test]
    fn recovered_marker_clear_failure_leaves_durable_marker() {
        let state = remote_glean_state();
        let db = test_db();
        let entity_id = "acc-finalize-marker-clear-failure";
        seed_finalize_account(&db, entity_id);
        let intel = make_glean_signal_intel(entity_id);
        let ctx = state.live_service_context();
        crate::services::signals::emit(
            &ctx,
            &db,
            "account",
            entity_id,
            "glean_finalization_degraded",
            "glean_synthesis",
            Some("{\"degradedClasses\":[\"account_fact\"]}"),
            0.5,
        )
        .expect("seed stale degraded marker");
        db.conn_ref()
            .execute_batch(
                "CREATE TRIGGER fail_degraded_marker_delete
                 BEFORE DELETE ON signal_events
                 WHEN OLD.entity_id = 'acc-finalize-marker-clear-failure'
                   AND OLD.signal_type = 'glean_finalization_degraded'
                 BEGIN
                   SELECT RAISE(ABORT, 'forced marker clear failure');
                 END;",
            )
            .expect("install marker delete failure trigger");

        let report = crate::services::glean_finalization::finalize_glean_enrichment(
            &ctx,
            &db,
            state.signals.engine.as_ref(),
            crate::services::glean_finalization::GleanFinalizationInput {
                entity_type: "account",
                entity_id,
                intel: &intel,
                preset: None,
            },
        )
        .expect("marker-clear-degraded Glean finalization");

        assert!(
            report.degraded_classes.contains(
                &crate::services::glean_finalization::GleanFinalizationSideEffect::DegradedMarker
            ),
            "marker clear failure should be represented as degraded marker work"
        );
        assert!(
            report.durable_degraded_marker_emitted,
            "marker clear failure must not return success without a durable marker"
        );
        assert_eq!(
            signal_count(&db, entity_id, "glean_finalization_degraded"),
            2,
            "failed stale-marker cleanup should retain the old marker and emit a current durable marker"
        );
    }

    #[test]
    fn glean_finalization_reports_signal_and_marker_degradation() {
        let state = remote_glean_state();
        let db = test_db();
        let entity_id = "acc-finalize-signal-failure";
        seed_finalize_account(&db, entity_id);
        let intel = make_glean_signal_intel(entity_id);
        let ctx = state.live_service_context();

        db.conn_ref()
            .execute_batch(
                "CREATE TRIGGER fail_glean_signal_and_marker_insert
                 BEFORE INSERT ON signal_events
                 WHEN NEW.entity_id = 'acc-finalize-signal-failure'
                   AND NEW.signal_type IN (
                     'renewal_data_updated',
                     'glean_finalization_degraded'
                   )
                 BEGIN
                   SELECT RAISE(ABORT, 'forced signal failure');
                 END;",
            )
            .expect("install signal failure trigger");

        let error = crate::services::glean_finalization::finalize_glean_enrichment(
            &ctx,
            &db,
            state.signals.engine.as_ref(),
            crate::services::glean_finalization::GleanFinalizationInput {
                entity_type: "account",
                entity_id,
                intel: &intel,
                preset: None,
            },
        )
        .expect_err("marker-less degradation must fail finalization");

        assert!(
            error
                .to_string()
                .contains("degraded without durable marker"),
            "marker-less degradation must report the durable marker failure"
        );
        assert_eq!(
            signal_count(&db, entity_id, "glean_finalization_degraded"),
            0,
            "forced marker insert failure should not be hidden behind success"
        );
    }

    #[test]
    fn glean_finalization_reports_signal_degradation_with_durable_marker() {
        let state = remote_glean_state();
        let db = test_db();
        let entity_id = "acc-finalize-signal-only-failure";
        seed_finalize_account(&db, entity_id);
        let intel = make_glean_signal_intel(entity_id);
        let ctx = state.live_service_context();

        db.conn_ref()
            .execute_batch(
                "CREATE TRIGGER fail_glean_renewal_signal_insert
                 BEFORE INSERT ON signal_events
                 WHEN NEW.entity_id = 'acc-finalize-signal-only-failure'
                   AND NEW.signal_type = 'renewal_data_updated'
                 BEGIN
                   SELECT RAISE(ABORT, 'forced renewal signal failure');
                 END;",
            )
            .expect("install signal failure trigger");

        let report = crate::services::glean_finalization::finalize_glean_enrichment(
            &ctx,
            &db,
            state.signals.engine.as_ref(),
            crate::services::glean_finalization::GleanFinalizationInput {
                entity_type: "account",
                entity_id,
                intel: &intel,
                preset: None,
            },
        )
        .expect("signal-degraded Glean finalization");

        assert!(
            report.degraded_classes.contains(
                &crate::services::glean_finalization::GleanFinalizationSideEffect::Signal
            ),
            "signal failure should be represented as degraded signal work"
        );
        assert!(report.warnings.iter().any(|warning| {
            warning.code == "signal_propagation_failed"
                && warning.signal_type == Some("renewal_data_updated")
                && warning.pii_safe_detail.is_some()
        }));
        assert_eq!(
            signal_count(&db, entity_id, "glean_finalization_degraded"),
            1,
            "signal degradation should leave a durable marker when marker insert succeeds"
        );
        let marker_payload: String = db
            .conn_ref()
            .query_row(
                "SELECT value FROM signal_events
                 WHERE entity_id = ?1
                   AND signal_type = 'glean_finalization_degraded'",
                params![entity_id],
                |row| row.get(0),
            )
            .expect("degraded marker payload");
        assert!(marker_payload.contains("signal"));
        assert!(!marker_payload.contains("enterprise"));
    }

    #[test]
    fn glean_finalization_reports_technical_footprint_degradation() {
        let state = remote_glean_state();
        let db = test_db();
        let entity_id = "acc-finalize-technical-failure";
        seed_finalize_account(&db, entity_id);
        let intel = make_glean_signal_intel(entity_id);
        let ctx = state.live_service_context();

        db.conn_ref()
            .execute_batch(
                "CREATE TRIGGER fail_technical_footprint_insert
                 BEFORE INSERT ON account_technical_footprint
                 WHEN NEW.account_id = 'acc-finalize-technical-failure'
                 BEGIN
                   SELECT RAISE(ABORT, 'forced technical footprint failure');
                 END;",
            )
            .expect("install technical footprint failure trigger");

        let report = crate::services::glean_finalization::finalize_glean_enrichment(
            &ctx,
            &db,
            state.signals.engine.as_ref(),
            crate::services::glean_finalization::GleanFinalizationInput {
                entity_type: "account",
                entity_id,
                intel: &intel,
                preset: None,
            },
        )
        .expect("technical-footprint-degraded Glean finalization");

        assert!(report.degraded_classes.contains(
            &crate::services::glean_finalization::GleanFinalizationSideEffect::TechnicalFootprint
        ));
        assert!(report
            .warnings
            .iter()
            .any(|warning| warning.code == "technical_footprint_write_failed"));
        assert_eq!(
            signal_count(&db, entity_id, "glean_finalization_degraded"),
            1,
            "technical footprint degradation should leave a durable marker"
        );
    }

    #[test]
    fn glean_finalization_reports_trust_recompute_degradation() {
        let state = remote_glean_state();
        let db = test_db();
        let entity_id = "acc-finalize-trust-recompute-failure";
        seed_finalize_account(&db, entity_id);
        let intel = make_glean_signal_intel(entity_id);
        let ctx = state.live_service_context();

        db.conn_ref()
            .execute_batch(
                "CREATE TRIGGER fail_account_fact_recompute_signal_insert
                 BEFORE INSERT ON signal_events
                 WHEN NEW.entity_id = 'acc-finalize-trust-recompute-failure'
                   AND NEW.signal_type = 'account_fact_claims_updated'
                 BEGIN
                   SELECT RAISE(ABORT, 'forced trust recompute enqueue failure');
                 END;",
            )
            .expect("install trust recompute failure trigger");

        let report = crate::services::glean_finalization::finalize_glean_enrichment(
            &ctx,
            &db,
            state.signals.engine.as_ref(),
            crate::services::glean_finalization::GleanFinalizationInput {
                entity_type: "account",
                entity_id,
                intel: &intel,
                preset: None,
            },
        )
        .expect("trust-recompute-degraded Glean finalization");

        assert!(
            report.degraded_classes.contains(
                &crate::services::glean_finalization::GleanFinalizationSideEffect::TrustRecompute
            ),
            "trust recompute enqueue failure should be represented as degraded trust work"
        );
        assert!(report
            .warnings
            .iter()
            .any(|warning| warning.code == "account_fact_recompute_enqueue_failed"));
        assert_eq!(
            signal_count(&db, entity_id, "glean_finalization_degraded"),
            1,
            "trust recompute degradation should leave a durable marker"
        );
    }

    #[test]
    fn glean_finalization_reports_health_recompute_degradation() {
        let state = remote_glean_state();
        let db = test_db();
        let entity_id = "acc-finalize-health-failure";
        seed_finalize_account(&db, entity_id);
        let intel = make_glean_signal_intel(entity_id);
        db.upsert_entity_intelligence(&intel)
            .expect("seed entity intelligence for health recompute");
        let ctx = state.live_service_context();

        db.conn_ref()
            .execute_batch(
                "CREATE TRIGGER fail_health_projection_update
                 BEFORE UPDATE OF health_score ON entity_quality
                 WHEN OLD.entity_id = 'acc-finalize-health-failure'
                 BEGIN
                   SELECT RAISE(ABORT, 'forced health recompute failure');
                 END;",
            )
            .expect("install health recompute failure trigger");

        let report = crate::services::glean_finalization::finalize_glean_enrichment(
            &ctx,
            &db,
            state.signals.engine.as_ref(),
            crate::services::glean_finalization::GleanFinalizationInput {
                entity_type: "account",
                entity_id,
                intel: &intel,
                preset: None,
            },
        )
        .expect("health-recompute-degraded Glean finalization");

        assert!(report.degraded_classes.contains(
            &crate::services::glean_finalization::GleanFinalizationSideEffect::HealthRecompute
        ));
        assert!(report
            .warnings
            .iter()
            .any(|warning| warning.code == "account_health_recompute_failed"));
        assert_eq!(
            signal_count(&db, entity_id, "glean_finalization_degraded"),
            1,
            "health recompute degradation should leave a durable marker"
        );
    }

    fn assert_finalize_mode_contract(
        entity_id: &str,
        mode: FinalizeMode,
        expect_queue_only_effects: bool,
    ) {
        let db = test_db();
        let account = make_account(entity_id);
        db.upsert_account(&account).unwrap();
        db.conn_ref()
            .execute(
                "INSERT INTO entity_assessment (entity_id, entity_type)
                 VALUES (?1, 'account')",
                params![entity_id],
            )
            .expect("seed entity assessment");
        crate::self_healing::quality::ensure_quality_row(&db, entity_id, "account");
        db.conn_ref()
            .execute(
                "UPDATE entity_quality
                 SET coherence_retry_count = 2,
                     coherence_window_start = datetime('now')
                 WHERE entity_id = ?1",
                params![entity_id],
            )
            .expect("seed self-healing retry state");

        let state = remote_glean_state();
        state
            .live_service_context()
            .check_mutation_allowed()
            .expect("live context should allow finalize mutations");
        let dir = tempfile::tempdir().expect("tempdir");
        let input = make_enrichment_input(entity_id, dir.path());
        let intel = make_glean_signal_intel(entity_id);

        let glean_signal_before = signal_count(&db, entity_id, "renewal_data_updated");
        let sync_success_before = sync_success_count(&db, "claude_code");
        let technical_footprint_before = technical_footprint_count(&db, entity_id);
        let retry_before = coherence_retry_count(&db, entity_id);

        run_enrichment_finalize_post_commit(&state, &db, &input, &intel, &[], mode)
            .expect("finalize post commit");

        let glean_signal_after = signal_count(&db, entity_id, "renewal_data_updated");
        let sync_success_after = sync_success_count(&db, "claude_code");
        let technical_footprint_after = technical_footprint_count(&db, entity_id);
        let retry_after = coherence_retry_count(&db, entity_id);

        assert_eq!(
            glean_signal_after,
            glean_signal_before + 1,
            "Glean finalize should emit shared Glean signals for queue and manual refresh"
        );
        assert_eq!(
            technical_footprint_after,
            technical_footprint_before + 1,
            "Glean finalize should run shared technical footprint writes for queue and manual refresh"
        );

        if expect_queue_only_effects {
            assert_eq!(
                sync_success_after,
                sync_success_before + 1,
                "QueueWorker finalize should record claude_code sync success"
            );
            assert_eq!(
                retry_after, 0,
                "QueueWorker finalize should run self-healing completion"
            );
        } else {
            assert_eq!(
                sync_success_after, sync_success_before,
                "ManualRefresh finalize should skip claude_code sync success"
            );
            assert_eq!(
                retry_after, retry_before,
                "ManualRefresh finalize should skip self-healing completion"
            );
        }
    }

    fn assert_finalize_relationship_failure_preserves_ordering(
        entity_id: &str,
        meeting_id: &str,
        from_person_id: &str,
        to_person_id: &str,
        mode: FinalizeMode,
    ) {
        let db = test_db();
        let account = make_account(entity_id);
        db.upsert_account(&account).unwrap();
        seed_person(&db, from_person_id, "Fail Buyer");
        seed_person(&db, to_person_id, "Fail Champion");
        db.conn_ref()
            .execute(
                "INSERT INTO entity_assessment (entity_id, entity_type)
                 VALUES (?1, 'account')",
                params![entity_id],
            )
            .expect("seed entity assessment");
        db.conn_ref()
            .execute(
                "INSERT INTO meetings (id, title, meeting_type, start_time, created_at)
                 VALUES (?1, 'Finalize relationship failure', 'customer',
                         '2999-01-01T00:00:00Z', '2026-05-03T00:00:00Z')",
                params![meeting_id],
            )
            .expect("seed future meeting");
        db.conn_ref()
            .execute(
                "INSERT INTO meeting_prep (meeting_id, prep_frozen_json, prep_frozen_at)
                 VALUES (?1, '{\"status\":\"ready\"}', '2026-05-03T00:00:00Z')",
                params![meeting_id],
            )
            .expect("seed frozen prep");
        db.conn_ref()
            .execute(
                "INSERT INTO meeting_entities (meeting_id, entity_id, entity_type)
                 VALUES (?1, ?2, 'account')",
                params![meeting_id, entity_id],
            )
            .expect("link meeting entity");
        crate::self_healing::quality::ensure_quality_row(&db, entity_id, "account");
        let quality_before = crate::self_healing::quality::get_quality(&db, entity_id)
            .expect("quality row before failure");
        let sync_success_before = sync_success_count(&db, "claude_code");

        db.conn_ref()
            .execute_batch(
                "CREATE TRIGGER fail_finalize_relationship_insert
                 BEFORE INSERT ON person_relationships
                 WHEN NEW.context_entity_id IN ('acc-finalize-rel-fails', 'acc-finalize-rel-fails-queue')
                 BEGIN
                   SELECT RAISE(ABORT, 'forced relationship persist failure');
                 END;",
            )
            .expect("install relationship failure trigger");

        let state = Arc::new(AppState::new());
        state.set_context_mode_atomic(&crate::context_provider::ContextMode::Local);
        state
            .live_service_context()
            .check_mutation_allowed()
            .expect("live context should allow finalize mutations");
        let dir = tempfile::tempdir().expect("tempdir");
        let input = make_enrichment_input(entity_id, dir.path());
        let intel = IntelligenceJson {
            executive_assessment_render_policy: None,
            entity_id: entity_id.to_string(),
            entity_type: "account".to_string(),
            enriched_at: "2026-05-03T01:00:00Z".to_string(),
            executive_assessment: Some("relationship persistence should fail".to_string()),
            ..Default::default()
        };
        let inferred = vec![InferredRelationship {
            from_person_id: from_person_id.to_string(),
            to_person_id: to_person_id.to_string(),
            relationship_type: "collaborator".to_string(),
            rationale: Some("Trigger forces failure before success stamp.".to_string()),
        }];

        let result =
            run_enrichment_finalize_post_commit(&state, &db, &input, &intel, &inferred, mode);
        assert!(
            result.is_err_and(|err| err.contains("forced relationship persist failure")),
            "relationship persistence failure should abort finalize before success"
        );

        let prep_frozen: Option<String> = db
            .conn_ref()
            .query_row(
                "SELECT prep_frozen_json FROM meeting_prep
                 WHERE meeting_id = ?1",
                params![meeting_id],
                |row| row.get(0),
            )
            .expect("read prep_frozen_json");
        assert!(
            prep_frozen.is_some(),
            "prep invalidation must wait until after relationship persistence succeeds"
        );
        let quality_after = crate::self_healing::quality::get_quality(&db, entity_id)
            .expect("quality row after failure");
        assert_eq!(
            quality_after.quality_alpha, quality_before.quality_alpha,
            "success stamp must wait until after relationship persistence succeeds"
        );
        assert_eq!(
            quality_after.last_enrichment_at, quality_before.last_enrichment_at,
            "failed finalize must not stamp last_enrichment_at"
        );
        assert_eq!(
            sync_success_count(&db, "claude_code"),
            sync_success_before,
            "failed finalize must not record queue-worker sync success"
        );
    }

    #[test]
    fn test_persist_entity_keywords() {
        let db = test_db();
        let account = make_account("acc-kw");
        db.upsert_account(&account).unwrap();
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 4, 30, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);

        let keywords_json = r#"["onboarding", "enterprise", "SaaS"]"#;
        super::persist_entity_keywords(&ctx, &db, "account", "acc-kw", keywords_json)
            .expect("persist_entity_keywords");

        // Verify keywords stored
        let stored: Option<String> = db
            .conn_ref()
            .query_row(
                "SELECT keywords FROM accounts WHERE id = 'acc-kw'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(stored.as_deref(), Some(keywords_json));

        // Verify signal
        assert!(
            signal_count(&db, "acc-kw", "keywords_updated") > 0,
            "Expected keywords_updated signal"
        );
    }

    #[test]
    fn test_upsert_assessment_from_enrichment() {
        let db = test_db();
        let engine = PropagationEngine::default();
        let account = make_account("acc-intel");
        db.upsert_account(&account).unwrap();

        let intel = IntelligenceJson {
            executive_assessment_render_policy: None,
            entity_id: "acc-intel".to_string(),
            entity_type: "account".to_string(),
            enriched_at: chrono::Utc::now().to_rfc3339(),
            executive_assessment: Some("Strong account with growing adoption.".to_string()),
            ..Default::default()
        };
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 4, 30, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);

        super::upsert_assessment_from_enrichment(
            &ctx,
            &db,
            &engine,
            "account",
            "acc-intel",
            &intel,
        )
        .expect("upsert_assessment_from_enrichment");

        // Verify entity_assessment row exists
        let exists: bool = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) > 0 FROM entity_assessment WHERE entity_id = 'acc-intel'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(exists, "entity_assessment row should exist");

        // Verify signal
        assert!(
            signal_count(&db, "acc-intel", "entity_intelligence_updated") > 0,
            "Expected entity_intelligence_updated signal"
        );
    }

    #[test]
    fn upsert_assessment_projects_pty_dossier_fields_into_claims() {
        let db = test_db();
        let engine = PropagationEngine::default();
        let account = make_account("acc-pty-projection");
        db.upsert_account(&account).unwrap();

        let intel = IntelligenceJson {
            executive_assessment_render_policy: None,
            entity_id: "acc-pty-projection".to_string(),
            entity_type: "account".to_string(),
            enriched_at: "2026-05-22T12:00:00Z".to_string(),
            pull_quote: Some(
                "Adoption is growing, but the renewal path needs attention.".to_string(),
            ),
            health: Some(AccountHealth {
                narrative: Some(
                    "Signals point to a workable but fragile account posture.".to_string(),
                ),
                recommended_actions: vec!["Schedule an executive alignment review.".to_string()],
                ..Default::default()
            }),
            strategic_priorities: vec![StrategicPriority {
                priority: "Expand the pilot into the operations team.".to_string(),
                status: Some("active".to_string()),
                owner: Some("customer operations".to_string()),
                source: Some("meeting".to_string()),
                timeline: Some("Q3".to_string()),
                context: Some(
                    "Operations adoption is the clearest route to durable value.".to_string(),
                ),
            }],
            blockers: vec![Blocker {
                description: "Procurement still needs security review artifacts.".to_string(),
                owner: Some("account team".to_string()),
                since: Some("2026-05-01".to_string()),
                impact: Some("high".to_string()),
                source: Some("meeting".to_string()),
            }],
            contract_context: Some(ContractContext {
                renewal_date: Some("2026-09-30".to_string()),
                procurement_notes: Some("Legal review needs a four-week lead time.".to_string()),
                ..Default::default()
            }),
            expansion_signals: vec![ExpansionSignal {
                opportunity: "Operations team expansion".to_string(),
                arr_impact: Some(25000.0),
                source: Some("meeting".to_string()),
                stage: Some("evaluating".to_string()),
                strength: Some("moderate".to_string()),
                item_source: Some(ItemSource {
                    source: "meeting".to_string(),
                    confidence: 0.8,
                    sourced_at: "2026-05-21T10:00:00Z".to_string(),
                    reference: Some("meeting fixture".to_string()),
                }),
                discrepancy: None,
            }],
            agreement_outlook: Some(AgreementOutlook {
                confidence: Some("moderate".to_string()),
                renewal_narrative: Some(
                    "The agreement can land if procurement work starts early.".to_string(),
                ),
                recommended_start: Some("June".to_string()),
                risk_factors: vec!["Security review lead time".to_string()],
                ..Default::default()
            }),
            success_metrics: Some(vec![SuccessMetric {
                name: "Weekly active operators".to_string(),
                target: Some("80%".to_string()),
                current: Some("55%".to_string()),
                status: Some("watch".to_string()),
                owner: Some("customer operations".to_string()),
            }]),
            open_commitments: Some(vec![OpenCommitment {
                commitment_id: Some("meeting:generic:1".to_string()),
                description: "Send updated security documentation.".to_string(),
                owner: Some("account team".to_string()),
                due_date: Some("2026-06-01".to_string()),
                source: Some("meeting".to_string()),
                status: Some("open".to_string()),
                item_source: None,
                discrepancy: None,
            }]),
            recommended_actions: vec![RecommendedAction {
                title: "Start procurement prep".to_string(),
                rationale: "Security review lead time is the visible renewal risk.".to_string(),
                priority: 2,
                suggested_due: Some("2026-06-15".to_string()),
            }],
            ..Default::default()
        };
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 22, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);

        super::upsert_assessment_from_enrichment(
            &ctx,
            &db,
            &engine,
            "account",
            "acc-pty-projection",
            &intel,
        )
        .expect("upsert rich pty assessment");

        let rows = projection_claim_rows(&db, "acc-pty-projection");
        assert_projection_claim(&rows, "entity_summary", "pullQuote", "Adoption is growing");
        assert_projection_claim(
            &rows,
            "entity_current_state",
            "health",
            "fragile account posture",
        );
        assert_projection_claim(
            &rows,
            "recommendation",
            "health.recommendedActions[0]",
            "executive alignment",
        );
        assert_projection_claim(
            &rows,
            "recommendation",
            "recommendedActions[0]",
            "Start procurement prep",
        );
        assert_projection_claim(
            &rows,
            "entity_current_state",
            "strategicPriorities[0]",
            "operations team",
        );
        assert_projection_claim(
            &rows,
            "entity_risk",
            "blockers[0]",
            "security review artifacts",
        );
        assert_projection_claim(
            &rows,
            "company_context",
            "contractContext",
            "four-week lead time",
        );
        assert_projection_claim(
            &rows,
            "entity_current_state",
            "expansionSignals[0]",
            "Operations team expansion",
        );
        assert_projection_claim(
            &rows,
            "entity_current_state",
            "agreementOutlook",
            "procurement work starts early",
        );
        assert_projection_claim(
            &rows,
            "entity_current_state",
            "successMetrics[0]",
            "Weekly active operators",
        );
        assert_projection_claim(
            &rows,
            "commitment",
            "openCommitments[0]",
            "updated security documentation",
        );
        assert_projection_source_asof(&rows, "pullQuote", None);
        assert_projection_source_asof(&rows, "expansionSignals[0]", Some("2026-05-21T10:00:00Z"));
    }

    #[test]
    fn projection_claim_reinforces_same_text_when_source_asof_differs() {
        let db = test_db();
        let account = make_account("acc-projection-source-asof");
        db.upsert_account(&account).unwrap();
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 22, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(43);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);
        let subject_ref = super::subject_ref_for_entity("account", "acc-projection-source-asof")
            .expect("subject ref");
        let text = "Adoption is growing, but the renewal path needs attention.";

        crate::services::claims::commit_claim(
            &ctx,
            &db,
            crate::services::claims::ClaimProposal {
                id: None,
                expected_claim_version: None,
                subject_ref,
                claim_type: "entity_summary".to_string(),
                field_path: Some("pullQuote".to_string()),
                topic_key: None,
                text: text.to_string(),
                actor: "agent:intelligence".to_string(),
                data_source: "legacy_intelligence_upsert".to_string(),
                source_ref: None,
                source_asof: Some("2026-05-22T12:00:00Z".to_string()),
                observed_at: "2026-05-22T12:00:00Z".to_string(),
                provenance_json: "{}".to_string(),
                metadata_json: None,
                thread_id: None,
                temporal_scope: None,
                sensitivity: None,
                supersedes: None,
                tombstone: None,
            },
        )
        .expect("seed old projection claim");

        let intel = IntelligenceJson {
            executive_assessment_render_policy: None,
            entity_id: "acc-projection-source-asof".to_string(),
            entity_type: "account".to_string(),
            enriched_at: "2026-05-22T12:00:00Z".to_string(),
            pull_quote: Some(text.to_string()),
            ..Default::default()
        };
        super::commit_claim_shaped_intelligence_projection(
            &ctx,
            &db,
            &intel,
            "agent:intelligence",
            "ai_enrichment",
        )
        .expect("repair projection claim");

        let rows = projection_claim_rows(&db, "acc-projection-source-asof");
        assert_projection_source_asof(&rows, "pullQuote", Some("2026-05-22T12:00:00Z"));
        let active_pull_quote_count = rows
            .iter()
            .filter(|(claim_type, field_path, _, _)| {
                claim_type == "entity_summary" && field_path == "pullQuote"
            })
            .count();
        assert_eq!(active_pull_quote_count, 1);
        let dormant_old_count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*)
                   FROM intelligence_claims
                  WHERE claim_type = 'entity_summary'
                    AND field_path = 'pullQuote'
                    AND source_asof = '2026-05-22T12:00:00Z'
                    AND claim_state = 'dormant'
                    AND surfacing_state = 'dormant'",
                [],
                |row| row.get(0),
            )
            .expect("dormant projection count");
        assert_eq!(dormant_old_count, 0);
    }

    #[test]
    fn cleared_dimensions_withdraw_stale_generated_projection_claims() {
        let db = test_db();
        let engine = PropagationEngine::default();
        let account_id = "acc-cleared-dimension-projection";
        let account = make_account(account_id);
        db.upsert_account(&account).unwrap();
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 22, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(50);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);
        let prior = IntelligenceJson {
            executive_assessment_render_policy: None,
            entity_id: account_id.to_string(),
            entity_type: "account".to_string(),
            enriched_at: "2026-05-22T12:00:00Z".to_string(),
            health: Some(AccountHealth {
                narrative: Some(
                    "Commercial health should not survive internal refresh.".to_string(),
                ),
                ..Default::default()
            }),
            ..Default::default()
        };

        super::upsert_assessment_from_enrichment(&ctx, &db, &engine, "account", account_id, &prior)
            .expect("seed commercial health projection");
        let active_before: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*)
                   FROM intelligence_claims
                  WHERE field_path = 'health'
                    AND claim_state = 'active'
                    AND surfacing_state = 'active'
                    AND json_valid(subject_ref) = 1
                    AND json_extract(subject_ref, '$.id') = ?1",
                params![account_id],
                |row| row.get(0),
            )
            .expect("active health projection count before clear");
        assert_eq!(active_before, 1);

        let dir = tempfile::tempdir().expect("tempdir");
        let mut input = make_enrichment_input(account_id, dir.path());
        input.relationship = Some("internal".to_string());
        let incoming = IntelligenceJson {
            executive_assessment_render_policy: None,
            entity_id: account_id.to_string(),
            entity_type: "account".to_string(),
            enriched_at: "2026-05-22T13:00:00Z".to_string(),
            executive_assessment: Some(
                "Internal account context has no commercial health.".to_string(),
            ),
            ..Default::default()
        };
        let prepared = compose_enrichment_intelligence_payload(
            &db,
            &input,
            &incoming,
            crate::intel_queue::EnrichmentProducer::Pty,
            None,
        )
        .expect("compose internal account refresh");

        assert!(
            prepared
                .cleared_dimensions()
                .contains(&"commercial_financial"),
            "internal account refresh should clear commercial fields"
        );
        assert!(prepared.intelligence().health.is_none());
        db.with_transaction(|tx| {
            apply_enrichment_side_writes(&ctx, tx, &input, &prepared)?;
            super::upsert_assessment_from_enrichment_in_active_transaction(
                &ctx,
                tx,
                &engine,
                super::EnrichmentAssessmentUpsert {
                    entity_type: "account",
                    entity_id: account_id,
                    intel: prepared.intelligence(),
                    projection_intel: prepared.projection_intelligence(),
                    projection_data_source: "ai_enrichment",
                    cleared_dimensions: prepared.cleared_dimensions(),
                },
            )
        })
        .expect("persist cleared-dimension refresh");

        let active_after: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*)
                   FROM intelligence_claims
                  WHERE field_path = 'health'
                    AND claim_state = 'active'
                    AND surfacing_state = 'active'
                    AND json_valid(subject_ref) = 1
                    AND json_extract(subject_ref, '$.id') = ?1",
                params![account_id],
                |row| row.get(0),
            )
            .expect("active health projection count after clear");
        assert_eq!(active_after, 0);
        let withdrawn_after: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*)
                   FROM intelligence_claims
                  WHERE field_path = 'health'
                    AND claim_state = 'withdrawn'
                    AND surfacing_state = 'dormant'
                    AND retraction_reason = 'dimension_not_applicable'
                    AND json_valid(subject_ref) = 1
                    AND json_extract(subject_ref, '$.id') = ?1",
                params![account_id],
                |row| row.get(0),
            )
            .expect("withdrawn health projection count after clear");
        assert_eq!(withdrawn_after, 1);
    }

    #[test]
    fn partial_glean_projection_does_not_relabel_preserved_local_fields() {
        let db = test_db();
        let engine = PropagationEngine::default();
        let account_id = "acc-partial-glean-projection";
        let account = make_account(account_id);
        db.upsert_account(&account).unwrap();
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 22, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(51);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);
        let local = IntelligenceJson {
            executive_assessment_render_policy: None,
            entity_id: account_id.to_string(),
            entity_type: "account".to_string(),
            enriched_at: "2026-05-22T12:00:00Z".to_string(),
            health: Some(AccountHealth {
                narrative: Some(
                    "Local health posture should remain locally attributed.".to_string(),
                ),
                ..Default::default()
            }),
            ..Default::default()
        };
        super::upsert_assessment_from_enrichment(&ctx, &db, &engine, "account", account_id, &local)
            .expect("seed local health projection");

        let dir = tempfile::tempdir().expect("tempdir");
        let input = make_enrichment_input(account_id, dir.path());
        let incoming = IntelligenceJson {
            executive_assessment_render_policy: None,
            entity_id: account_id.to_string(),
            entity_type: "account".to_string(),
            enriched_at: "2026-05-22T13:00:00Z".to_string(),
            risks: vec![IntelRisk {
                render_policy: None,
                claim_id: None,
                text: "CRM renewal risk requires executive follow-up.".to_string(),
                item_source: Some(ItemSource {
                    source: "glean_crm".to_string(),
                    confidence: 0.86,
                    sourced_at: "2026-05-22T12:30:00Z".to_string(),
                    reference: Some("CRM opportunity fixture".to_string()),
                }),
                ..Default::default()
            }],
            ..Default::default()
        };
        let prepared = compose_enrichment_intelligence_payload(
            &db,
            &input,
            &incoming,
            crate::intel_queue::EnrichmentProducer::Glean,
            None,
        )
        .expect("compose partial Glean refresh");

        assert!(
            prepared.intelligence().health.is_some(),
            "legacy snapshot should preserve local health across sparse Glean refresh"
        );
        assert!(
            prepared.projection_intelligence().health.is_none(),
            "Glean projection should include only fields returned by Glean"
        );
        db.with_transaction(|tx| {
            apply_enrichment_side_writes(&ctx, tx, &input, &prepared)?;
            super::upsert_assessment_from_enrichment_in_active_transaction(
                &ctx,
                tx,
                &engine,
                super::EnrichmentAssessmentUpsert {
                    entity_type: "account",
                    entity_id: account_id,
                    intel: prepared.intelligence(),
                    projection_intel: prepared.projection_intelligence(),
                    projection_data_source: "glean",
                    cleared_dimensions: prepared.cleared_dimensions(),
                },
            )
        })
        .expect("persist partial Glean refresh");

        let persisted = db
            .get_entity_intelligence(account_id)
            .expect("read persisted intelligence")
            .expect("persisted intelligence");
        assert!(persisted.health.is_some());
        let glean_health_count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*)
                   FROM intelligence_claims
                  WHERE field_path = 'health'
                    AND data_source LIKE 'glean%'
                    AND claim_state = 'active'
                    AND surfacing_state = 'active'
                    AND json_valid(subject_ref) = 1
                    AND json_extract(subject_ref, '$.id') = ?1",
                params![account_id],
                |row| row.get(0),
            )
            .expect("active Glean health projection count");
        assert_eq!(glean_health_count, 0);
        let local_health_count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*)
                   FROM intelligence_claims
                  WHERE field_path = 'health'
                    AND data_source = 'ai_enrichment'
                    AND claim_state = 'active'
                    AND surfacing_state = 'active'
                    AND json_valid(subject_ref) = 1
                    AND json_extract(subject_ref, '$.id') = ?1",
                params![account_id],
                |row| row.get(0),
            )
            .expect("active local health projection count");
        assert_eq!(local_health_count, 1);
        let risk_data_source: String = db
            .conn_ref()
            .query_row(
                "SELECT data_source
                   FROM intelligence_claims
                  WHERE field_path = 'risks[0]'
                    AND claim_state = 'active'
                    AND surfacing_state = 'active'
                    AND json_valid(subject_ref) = 1
                    AND json_extract(subject_ref, '$.id') = ?1",
                params![account_id],
                |row| row.get(0),
            )
            .expect("active Glean risk projection source");
        assert_eq!(risk_data_source, "glean_crm");
    }

    #[test]
    fn glean_projection_uses_repaired_user_fact_without_preserved_local_fields() {
        let db = test_db();
        let engine = PropagationEngine::default();
        let account_id = "acc-glean-projection-repaired-fact";
        let account = make_account(account_id);
        db.upsert_account(&account).unwrap();
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 22, 12, 0, 0).unwrap());
        let rng = SeedableRng::new(52);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);
        let local = IntelligenceJson {
            executive_assessment_render_policy: None,
            entity_id: account_id.to_string(),
            entity_type: "account".to_string(),
            enriched_at: "2026-05-22T12:00:00Z".to_string(),
            health: Some(AccountHealth {
                narrative: Some("Local health must not be relabeled as Glean.".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        };
        super::upsert_assessment_from_enrichment(&ctx, &db, &engine, "account", account_id, &local)
            .expect("seed local health projection");

        let dir = tempfile::tempdir().expect("tempdir");
        let input = make_enrichment_input(account_id, dir.path());
        let incoming = IntelligenceJson {
            executive_assessment_render_policy: None,
            entity_id: account_id.to_string(),
            entity_type: "account".to_string(),
            enriched_at: "2026-05-22T13:00:00Z".to_string(),
            contract_context: Some(ContractContext {
                contract_type: Some("annual".to_string()),
                renewal_date: Some("2027-01-01".to_string()),
                current_arr: Some(1.0),
                ..Default::default()
            }),
            ..Default::default()
        };
        let prepared = compose_enrichment_intelligence_payload(
            &db,
            &input,
            &incoming,
            crate::intel_queue::EnrichmentProducer::Glean,
            None,
        )
        .expect("compose Glean refresh with corrected account fact");

        assert_eq!(
            prepared
                .intelligence()
                .contract_context
                .as_ref()
                .and_then(|context| context.current_arr),
            Some(100_000.0)
        );
        assert_eq!(
            prepared
                .projection_intelligence()
                .contract_context
                .as_ref()
                .and_then(|context| context.current_arr),
            Some(100_000.0),
            "Glean-generated projection should use repaired account facts"
        );
        assert!(
            prepared.projection_intelligence().health.is_none(),
            "Glean-generated projection should not relabel preserved local health"
        );

        db.with_transaction(|tx| {
            apply_enrichment_side_writes(&ctx, tx, &input, &prepared)?;
            super::upsert_assessment_from_enrichment_in_active_transaction(
                &ctx,
                tx,
                &engine,
                super::EnrichmentAssessmentUpsert {
                    entity_type: "account",
                    entity_id: account_id,
                    intel: prepared.intelligence(),
                    projection_intel: prepared.projection_intelligence(),
                    projection_data_source: "glean",
                    cleared_dimensions: prepared.cleared_dimensions(),
                },
            )
        })
        .expect("persist repaired Glean refresh");

        let contract_text: String = db
            .conn_ref()
            .query_row(
                "SELECT text
                   FROM intelligence_claims
                  WHERE field_path = 'contractContext'
                    AND data_source = 'glean'
                    AND claim_state = 'active'
                    AND surfacing_state = 'active'
                    AND json_valid(subject_ref) = 1
                    AND json_extract(subject_ref, '$.id') = ?1",
                params![account_id],
                |row| row.get(0),
            )
            .expect("active Glean contract projection text");
        assert!(
            contract_text.contains("current arr: 100000"),
            "contract projection text was {contract_text:?}"
        );
        let glean_health_count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*)
                   FROM intelligence_claims
                  WHERE field_path = 'health'
                    AND data_source LIKE 'glean%'
                    AND claim_state = 'active'
                    AND surfacing_state = 'active'
                    AND json_valid(subject_ref) = 1
                    AND json_extract(subject_ref, '$.id') = ?1",
                params![account_id],
                |row| row.get(0),
            )
            .expect("active Glean health projection count");
        assert_eq!(glean_health_count, 0);
    }

    #[test]
    fn update_stakeholders_disk_db_atomicity_under_rollback() {
        let db = test_db();
        let engine = PropagationEngine::default();
        let account = make_account("acc-stakeholder-rollback");
        db.upsert_account(&account).unwrap();

        let dir = tempfile::tempdir().expect("tempdir");
        let old_intel = IntelligenceJson {
            executive_assessment_render_policy: None,
            entity_id: "acc-stakeholder-rollback".to_string(),
            entity_type: "account".to_string(),
            enriched_at: "2026-05-03T00:00:00Z".to_string(),
            executive_assessment: Some("old stakeholder state".to_string()),
            stakeholder_insights: vec![StakeholderInsight {
                render_policy: None,
                claim_id: None,
                name: "Old Owner".to_string(),
                role: Some("buyer".to_string()),
                ..Default::default()
            }],
            ..Default::default()
        };
        db.upsert_entity_intelligence(&old_intel).unwrap();
        seed_disk_intelligence(&db, dir.path(), &old_intel);
        let before_disk =
            std::fs::read_to_string(dir.path().join("intelligence.json")).expect("read seed disk");

        db.conn_ref()
            .execute_batch(
                "CREATE TRIGGER fail_stakeholders_updated_signal
                 BEFORE INSERT ON signal_events
                 WHEN NEW.signal_type = 'stakeholders_updated'
                 BEGIN
                   SELECT RAISE(ABORT, 'forced stakeholders_updated rollback');
                 END;",
            )
            .expect("install rollback trigger");

        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 3, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);
        let new_intel = crate::intelligence::apply_stakeholders_update_in_memory(
            old_intel.clone(),
            vec![StakeholderInsight {
                render_policy: None,
                claim_id: None,
                name: "New Owner".to_string(),
                role: Some("champion".to_string()),
                ..Default::default()
            }],
        )
        .expect("compose stakeholder update");

        let result = db.with_transaction(|tx| {
            tx.upsert_entity_intelligence(&new_intel)
                .map_err(|e| e.to_string())?;
            crate::services::signals::emit_and_propagate(
                &ctx,
                tx,
                &engine,
                "account",
                "acc-stakeholder-rollback",
                "stakeholders_updated",
                "user_edit",
                None,
                0.9,
            )
            .map_err(|e| format!("signal emit failed: {e}"))?;
            Ok(())
        });
        if result.is_ok() {
            crate::intelligence::write_fence::post_commit_fenced_write(
                &db,
                dir.path(),
                &new_intel,
                "test stakeholder rollback",
            );
        }

        assert!(
            result.is_err_and(|err| err.contains("forced stakeholders_updated rollback")),
            "transaction should surface forced rollback"
        );
        let after_disk = std::fs::read_to_string(dir.path().join("intelligence.json"))
            .expect("read disk after rollback");
        assert_eq!(
            after_disk, before_disk,
            "disk cache must not change when stakeholder DB transaction rolls back"
        );
        let persisted = db
            .get_entity_intelligence("acc-stakeholder-rollback")
            .expect("read DB intelligence")
            .expect("existing DB intelligence");
        assert_eq!(
            persisted.executive_assessment.as_deref(),
            Some("old stakeholder state"),
            "DB intelligence must roll back to the pre-update state"
        );
    }

    #[test]
    fn enrich_entity_disk_db_atomicity_under_rollback() {
        let db = test_db();
        let engine = PropagationEngine::default();
        let account = make_account("acc-enrich-rollback");
        db.upsert_account(&account).unwrap();

        let dir = tempfile::tempdir().expect("tempdir");
        let old_intel = IntelligenceJson {
            executive_assessment_render_policy: None,
            entity_id: "acc-enrich-rollback".to_string(),
            entity_type: "account".to_string(),
            enriched_at: "2026-05-03T00:00:00Z".to_string(),
            executive_assessment: Some("old enrichment state".to_string()),
            ..Default::default()
        };
        db.upsert_entity_intelligence(&old_intel).unwrap();
        seed_disk_intelligence(&db, dir.path(), &old_intel);
        let before_disk =
            std::fs::read_to_string(dir.path().join("intelligence.json")).expect("read seed disk");

        db.conn_ref()
            .execute_batch(
                "CREATE TRIGGER fail_entity_intelligence_updated_signal
                 BEFORE INSERT ON signal_events
                 WHEN NEW.signal_type = 'entity_intelligence_updated'
                 BEGIN
                   SELECT RAISE(ABORT, 'forced entity_intelligence_updated rollback');
                 END;",
            )
            .expect("install rollback trigger");

        let new_intel = IntelligenceJson {
            executive_assessment_render_policy: None,
            entity_id: "acc-enrich-rollback".to_string(),
            entity_type: "account".to_string(),
            enriched_at: "2026-05-03T01:00:00Z".to_string(),
            executive_assessment: Some("new enrichment state".to_string()),
            ..Default::default()
        };
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 3, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);

        let result = super::upsert_assessment_from_enrichment(
            &ctx,
            &db,
            &engine,
            "account",
            "acc-enrich-rollback",
            &new_intel,
        );
        if result.is_ok() {
            crate::intel_queue::fenced_write_enrichment_intelligence(&db, dir.path(), &new_intel);
        }

        assert!(
            result.is_err_and(|err| err.contains("forced entity_intelligence_updated rollback")),
            "enrichment persistence should surface forced rollback"
        );
        let after_disk = std::fs::read_to_string(dir.path().join("intelligence.json"))
            .expect("read disk after rollback");
        assert_eq!(
            after_disk, before_disk,
            "disk cache must not change when enrichment DB transaction rolls back"
        );
        let persisted = db
            .get_entity_intelligence("acc-enrich-rollback")
            .expect("read DB intelligence")
            .expect("existing DB intelligence");
        assert_eq!(
            persisted.executive_assessment.as_deref(),
            Some("old enrichment state"),
            "DB intelligence must roll back to the pre-enrichment state"
        );
    }

    #[test]
    fn compose_enrichment_full_path_rollback_atomicity() {
        let db = test_db();
        let engine = PropagationEngine::default();
        let account = make_account("acc-compose-rollback");
        db.upsert_account(&account).unwrap();
        seed_account_domain(&db, "acc-compose-rollback", "compose.example");
        seed_person(&db, "p-compose-rollback", "Existing Buyer");
        db.add_account_team_member("acc-compose-rollback", "p-compose-rollback", "associated")
            .expect("seed account stakeholder");
        db.conn_ref()
            .execute(
                "UPDATE account_stakeholders
                 SET engagement = 'neutral',
                     data_source_engagement = 'ai',
                     assessment = 'prior assessment',
                     data_source_assessment = 'ai',
                     data_source = 'ai'
                 WHERE account_id = ?1 AND person_id = ?2",
                params!["acc-compose-rollback", "p-compose-rollback"],
            )
            .expect("seed account stakeholder metadata");
        db.conn_ref()
            .execute(
                "INSERT INTO suppression_tombstones
                 (entity_id, field_key, item_key, dismissed_at)
                 VALUES (?1, 'risks', ?2, 'not-a-timestamp')",
                params!["acc-compose-rollback", "Rollback malformed risk"],
            )
            .expect("seed malformed suppression tombstone");

        let dir = tempfile::tempdir().expect("tempdir");
        let prior = IntelligenceJson {
            executive_assessment_render_policy: None,
            entity_id: "acc-compose-rollback".to_string(),
            entity_type: "account".to_string(),
            enriched_at: "2026-05-03T00:00:00Z".to_string(),
            executive_assessment: Some("old compose state".to_string()),
            ..Default::default()
        };
        db.upsert_entity_intelligence(&prior).unwrap();
        seed_disk_intelligence(&db, dir.path(), &prior);
        let before_disk =
            std::fs::read_to_string(dir.path().join("intelligence.json")).expect("read seed disk");

        db.conn_ref()
            .execute_batch(
                "CREATE TRIGGER fail_full_enrichment_path_signal
                 BEFORE INSERT ON signal_events
                 WHEN NEW.signal_type = 'entity_intelligence_updated'
                 BEGIN
                   SELECT RAISE(ABORT, 'forced compose full path rollback');
                 END;",
            )
            .expect("install rollback trigger");

        let input = make_enrichment_input("acc-compose-rollback", dir.path());
        let incoming = IntelligenceJson {
            executive_assessment_render_policy: None,
            entity_id: "acc-compose-rollback".to_string(),
            entity_type: "account".to_string(),
            enriched_at: "2026-05-03T01:00:00Z".to_string(),
            executive_assessment: Some("new compose state".to_string()),
            stakeholder_insights: vec![
                StakeholderInsight {
                    render_policy: None,
                    claim_id: None,
                    name: "Existing Buyer".to_string(),
                    person_id: Some("p-compose-rollback".to_string()),
                    role: Some("technical champion".to_string()),
                    assessment: Some("new assessment".to_string()),
                    engagement: Some("strong_advocate".to_string()),
                    ..Default::default()
                },
                StakeholderInsight {
                    render_policy: None,
                    claim_id: None,
                    name: "New Buyer".to_string(),
                    role: Some("economic buyer".to_string()),
                    engagement: Some("engaged".to_string()),
                    ..Default::default()
                },
            ],
            risks: vec![IntelRisk {
                render_policy: None,
                claim_id: None,
                text: "Rollback malformed risk".to_string(),
                item_source: Some(ItemSource {
                    source: "pty_synthesis".to_string(),
                    confidence: 0.5,
                    sourced_at: "2026-05-03T00:30:00Z".to_string(),
                    reference: None,
                }),
                ..Default::default()
            }],
            ..Default::default()
        };
        let prepared = compose_enrichment_intelligence_payload(
            &db,
            &input,
            &incoming,
            crate::intel_queue::EnrichmentProducer::Pty,
            None,
        )
        .expect("compose full path");

        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 3, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);
        let state = AppState::new();
        let stakeholder_count_before = count_query(
            &db,
            "SELECT COUNT(*) FROM account_stakeholders WHERE account_id = 'acc-compose-rollback'",
        );
        let role_count_before = count_query(
            &db,
            "SELECT COUNT(*) FROM account_stakeholder_roles
             WHERE account_id = 'acc-compose-rollback'
               AND person_id = 'p-compose-rollback'
               AND role = 'technical champion'",
        );
        let malformed_count_before = count_query(
            &db,
            "SELECT COUNT(*) FROM suppression_malformed_log
             WHERE entity_id = 'acc-compose-rollback'",
        );
        let signal_count_before = count_query(
            &db,
            "SELECT COUNT(*) FROM signal_events
             WHERE entity_type = 'account'
               AND entity_id = 'acc-compose-rollback'",
        );

        let result = db.with_transaction(|tx| {
            apply_enrichment_side_writes(&ctx, tx, &input, &prepared)?;
            super::upsert_assessment_from_enrichment_in_active_transaction(
                &ctx,
                tx,
                &engine,
                super::EnrichmentAssessmentUpsert {
                    entity_type: "account",
                    entity_id: "acc-compose-rollback",
                    intel: prepared.intelligence(),
                    projection_intel: prepared.projection_intelligence(),
                    projection_data_source: "ai_enrichment",
                    cleared_dimensions: prepared.cleared_dimensions(),
                },
            )
        });
        if result.is_ok() {
            crate::intel_queue::fenced_write_enrichment_intelligence(
                &db,
                dir.path(),
                prepared.intelligence(),
            );
            crate::intel_queue::run_enrichment_post_commit_side_effects(
                &state,
                &input,
                &db,
                prepared.intelligence(),
                crate::intel_queue::EnrichmentProducer::Pty,
            )
            .expect("post-commit side effects");
        }

        assert!(
            result.is_err_and(|err| err.contains("forced compose full path rollback")),
            "full enrichment path should surface forced rollback"
        );
        let suggestion_count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM stakeholder_suggestions WHERE account_id = 'acc-compose-rollback'",
                [],
                |row| row.get(0),
            )
            .expect("count stakeholder suggestions");
        assert_eq!(
            suggestion_count, 0,
            "compose stakeholder side writes must roll back with enrichment upsert"
        );
        let stakeholder_count_after = count_query(
            &db,
            "SELECT COUNT(*) FROM account_stakeholders WHERE account_id = 'acc-compose-rollback'",
        );
        assert_eq!(
            stakeholder_count_after, stakeholder_count_before,
            "stakeholder rows must roll back with enrichment upsert"
        );
        let engagement_after: String = db
            .conn_ref()
            .query_row(
                "SELECT engagement FROM account_stakeholders
                 WHERE account_id = 'acc-compose-rollback'
                   AND person_id = 'p-compose-rollback'",
                [],
                |row| row.get(0),
            )
            .expect("read stakeholder engagement");
        assert_eq!(
            engagement_after, "neutral",
            "stakeholder engagement must roll back with enrichment upsert"
        );
        let role_count_after = count_query(
            &db,
            "SELECT COUNT(*) FROM account_stakeholder_roles
             WHERE account_id = 'acc-compose-rollback'
               AND person_id = 'p-compose-rollback'
               AND role = 'technical champion'",
        );
        assert_eq!(
            role_count_after, role_count_before,
            "stakeholder roles must roll back with enrichment upsert"
        );
        let malformed_count_after = count_query(
            &db,
            "SELECT COUNT(*) FROM suppression_malformed_log
             WHERE entity_id = 'acc-compose-rollback'",
        );
        assert_eq!(
            malformed_count_after, malformed_count_before,
            "malformed suppression audits must roll back with enrichment upsert"
        );
        let signal_count_after = count_query(
            &db,
            "SELECT COUNT(*) FROM signal_events
             WHERE entity_type = 'account'
               AND entity_id = 'acc-compose-rollback'",
        );
        assert_eq!(
            signal_count_after, signal_count_before,
            "signals must roll back with enrichment upsert"
        );
        let after_disk = std::fs::read_to_string(dir.path().join("intelligence.json"))
            .expect("read disk after rollback");
        assert_eq!(
            after_disk, before_disk,
            "disk cache must not change when full enrichment transaction rolls back"
        );
        let persisted = db
            .get_entity_intelligence("acc-compose-rollback")
            .expect("read DB intelligence")
            .expect("existing DB intelligence");
        assert_eq!(
            persisted.executive_assessment.as_deref(),
            Some("old compose state"),
            "DB intelligence must roll back to the pre-compose state"
        );
    }

    #[test]
    fn compose_enrichment_side_write_failure_aborts_upsert() {
        let db = test_db();
        let engine = PropagationEngine::default();
        let account = make_account("acc-side-write-fails");
        db.upsert_account(&account).unwrap();

        let dir = tempfile::tempdir().expect("tempdir");
        let prior = IntelligenceJson {
            executive_assessment_render_policy: None,
            entity_id: "acc-side-write-fails".to_string(),
            entity_type: "account".to_string(),
            enriched_at: "2026-05-03T00:00:00Z".to_string(),
            executive_assessment: Some("prior side-write state".to_string()),
            ..Default::default()
        };
        db.upsert_entity_intelligence(&prior).unwrap();

        db.conn_ref()
            .execute_batch(
                "CREATE TRIGGER fail_stakeholder_suggestion_side_write
                 BEFORE INSERT ON stakeholder_suggestions
                 WHEN NEW.account_id = 'acc-side-write-fails'
                 BEGIN
                   SELECT RAISE(ABORT, 'forced stakeholder suggestion failure');
                 END;",
            )
            .expect("install side-write failure trigger");

        let input = make_enrichment_input("acc-side-write-fails", dir.path());
        let incoming = IntelligenceJson {
            executive_assessment_render_policy: None,
            entity_id: "acc-side-write-fails".to_string(),
            entity_type: "account".to_string(),
            enriched_at: "2026-05-03T01:00:00Z".to_string(),
            executive_assessment: Some("new state that must not persist".to_string()),
            stakeholder_insights: vec![StakeholderInsight {
                render_policy: None,
                claim_id: None,
                name: "Blocked Buyer".to_string(),
                role: Some("economic buyer".to_string()),
                engagement: Some("engaged".to_string()),
                ..Default::default()
            }],
            ..Default::default()
        };
        let prepared = compose_enrichment_intelligence_payload(
            &db,
            &input,
            &incoming,
            crate::intel_queue::EnrichmentProducer::Pty,
            None,
        )
        .expect("compose side-write failure input");

        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 3, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);

        let result = db.with_transaction(|tx| {
            apply_enrichment_side_writes(&ctx, tx, &input, &prepared)?;
            super::upsert_assessment_from_enrichment_in_active_transaction(
                &ctx,
                tx,
                &engine,
                super::EnrichmentAssessmentUpsert {
                    entity_type: "account",
                    entity_id: "acc-side-write-fails",
                    intel: prepared.intelligence(),
                    projection_intel: prepared.projection_intelligence(),
                    projection_data_source: "ai_enrichment",
                    cleared_dimensions: prepared.cleared_dimensions(),
                },
            )
        });

        assert!(
            result.is_err_and(|err| err.contains("forced stakeholder suggestion failure")),
            "side-write failure should abort the enrichment transaction"
        );
        let persisted = db
            .get_entity_intelligence("acc-side-write-fails")
            .expect("read DB intelligence")
            .expect("existing DB intelligence");
        assert_eq!(
            persisted.executive_assessment.as_deref(),
            Some("prior side-write state"),
            "DB intelligence must remain unchanged when side-writes fail"
        );
        assert_eq!(
            signal_count(&db, "acc-side-write-fails", "entity_intelligence_updated"),
            0,
            "upsert signal must not emit after side-write failure"
        );
    }

    #[test]
    fn enrich_entity_invalidates_meeting_preps_and_records_success() {
        let db = test_db();
        let account = make_account("acc-manual-parity");
        db.upsert_account(&account).unwrap();
        seed_person(&db, "p-manual-parity-1", "Manual Buyer");
        seed_person(&db, "p-manual-parity-2", "Manual Champion");
        db.conn_ref()
            .execute(
                "INSERT INTO entity_assessment (entity_id, entity_type)
                 VALUES (?1, 'account')",
                params!["acc-manual-parity"],
            )
            .expect("seed entity assessment");
        db.conn_ref()
            .execute(
                "INSERT INTO meetings (id, title, meeting_type, start_time, created_at)
                 VALUES ('mtg-manual-parity', 'Manual parity review', 'customer',
                         '2999-01-01T00:00:00Z', '2026-05-03T00:00:00Z')",
                [],
            )
            .expect("seed future meeting");
        db.conn_ref()
            .execute(
                "INSERT INTO meeting_prep (meeting_id, prep_frozen_json, prep_frozen_at)
                 VALUES ('mtg-manual-parity', '{\"status\":\"ready\"}', '2026-05-03T00:00:00Z')",
                [],
            )
            .expect("seed frozen prep");
        db.conn_ref()
            .execute(
                "INSERT INTO meeting_entities (meeting_id, entity_id, entity_type)
                 VALUES ('mtg-manual-parity', 'acc-manual-parity', 'account')",
                [],
            )
            .expect("link meeting entity");
        crate::self_healing::quality::ensure_quality_row(&db, "acc-manual-parity", "account");
        let quality_before = crate::self_healing::quality::get_quality(&db, "acc-manual-parity")
            .expect("quality row before success");

        let state = Arc::new(AppState::new());
        let dir = tempfile::tempdir().expect("tempdir");
        let input = make_enrichment_input("acc-manual-parity", dir.path());
        let intel = IntelligenceJson {
            executive_assessment_render_policy: None,
            entity_id: "acc-manual-parity".to_string(),
            entity_type: "account".to_string(),
            enriched_at: "2026-05-03T01:00:00Z".to_string(),
            executive_assessment: Some("manual parity assessment".to_string()),
            ..Default::default()
        };
        let inferred = vec![InferredRelationship {
            from_person_id: "p-manual-parity-1".to_string(),
            to_person_id: "p-manual-parity-2".to_string(),
            relationship_type: "collaborator".to_string(),
            rationale: Some("They coordinate the manual parity rollout.".to_string()),
        }];

        run_enrichment_finalize_post_commit(
            &state,
            &db,
            &input,
            &intel,
            &inferred,
            FinalizeMode::ManualRefresh {
                producer: crate::intel_queue::EnrichmentProducer::Pty,
            },
        )
        .expect("manual finalize");

        let prep_frozen: Option<String> = db
            .conn_ref()
            .query_row(
                "SELECT prep_frozen_json FROM meeting_prep WHERE meeting_id = 'mtg-manual-parity'",
                [],
                |row| row.get(0),
            )
            .expect("read prep_frozen_json");
        assert!(
            prep_frozen.is_none(),
            "manual enrichment post-commit should invalidate frozen meeting prep"
        );
        assert_eq!(
            signal_count(&db, "mtg-manual-parity", "prep_invalidated"),
            1,
            "meeting prep invalidation should emit a prep_invalidated signal"
        );
        assert_eq!(
            count_query(
                &db,
                "SELECT COUNT(*) FROM person_relationships
                 WHERE context_entity_id = 'acc-manual-parity'
                   AND source = 'ai_enrichment'"
            ),
            1,
            "manual finalize should persist inferred relationships before success stamping"
        );
        let quality_after = crate::self_healing::quality::get_quality(&db, "acc-manual-parity")
            .expect("quality row after success");
        assert!(
            quality_after.quality_alpha > quality_before.quality_alpha,
            "manual enrichment success should increment self-healing quality alpha"
        );
        assert!(
            quality_after.last_enrichment_at.is_some(),
            "manual enrichment success should stamp last_enrichment_at"
        );
    }

    #[test]
    fn finalize_post_commit_manual_refresh_skips_queue_only_effects() {
        assert_finalize_mode_contract(
            "acc-finalize-manual-mode",
            FinalizeMode::ManualRefresh {
                producer: crate::intel_queue::EnrichmentProducer::Glean,
            },
            false,
        );
    }

    #[test]
    fn manual_refresh_promotes_account_facts_only_for_glean_producer() {
        let state = remote_glean_state();

        let pty_db = test_db();
        let pty_account_id = "acc-manual-pty-producer";
        pty_db
            .upsert_account(&make_account(pty_account_id))
            .unwrap();
        let dir = tempfile::tempdir().expect("tempdir");
        let input = make_enrichment_input(pty_account_id, dir.path());
        let intel = make_glean_signal_intel(pty_account_id);

        run_enrichment_finalize_post_commit(
            &state,
            &pty_db,
            &input,
            &intel,
            &[],
            FinalizeMode::ManualRefresh {
                producer: crate::intel_queue::EnrichmentProducer::Pty,
            },
        )
        .expect("finalize PTY manual refresh");

        assert_eq!(
            account_fact_claim_count(&pty_db, pty_account_id),
            0,
            "PTY fallback must not stamp Glean account fact claims"
        );
        assert_eq!(
            account_source_ref_count(&pty_db, pty_account_id),
            0,
            "PTY fallback must not write Glean source refs"
        );
        assert_eq!(
            signal_count(&pty_db, pty_account_id, "renewal_data_updated"),
            0,
            "PTY fallback must not emit Glean renewal signals"
        );
        assert_eq!(
            technical_footprint_count(&pty_db, pty_account_id),
            0,
            "PTY fallback must not write Glean technical footprint"
        );

        let queue_pty_db = test_db();
        let queue_pty_account_id = "acc-queue-pty-producer";
        queue_pty_db
            .upsert_account(&make_account(queue_pty_account_id))
            .unwrap();
        let input = make_enrichment_input(queue_pty_account_id, dir.path());
        let intel = make_glean_signal_intel(queue_pty_account_id);
        run_enrichment_finalize_post_commit(
            &state,
            &queue_pty_db,
            &input,
            &intel,
            &[],
            FinalizeMode::QueueWorker {
                is_background: false,
                producer: crate::intel_queue::EnrichmentProducer::Pty,
            },
        )
        .expect("finalize PTY queue fallback");
        assert_eq!(
            account_fact_claim_count(&queue_pty_db, queue_pty_account_id),
            0,
            "PTY queue fallback must not stamp Glean account fact claims"
        );
        assert_eq!(
            account_source_ref_count(&queue_pty_db, queue_pty_account_id),
            0,
            "PTY queue fallback must not write Glean source refs"
        );
        assert_eq!(
            signal_count(&queue_pty_db, queue_pty_account_id, "renewal_data_updated"),
            0,
            "PTY queue fallback must not emit Glean renewal signals"
        );
        assert_eq!(
            technical_footprint_count(&queue_pty_db, queue_pty_account_id),
            0,
            "PTY queue fallback must not write Glean technical footprint"
        );
        assert_eq!(
            signal_count(
                &queue_pty_db,
                queue_pty_account_id,
                "glean_finalization_degraded"
            ),
            0,
            "PTY queue fallback must not emit Glean degraded markers"
        );

        let glean_db = test_db();
        let glean_account_id = "acc-manual-glean-producer";
        glean_db
            .upsert_account(&make_account(glean_account_id))
            .unwrap();
        let input = make_enrichment_input(glean_account_id, dir.path());
        let intel = make_glean_signal_intel(glean_account_id);

        run_enrichment_finalize_post_commit(
            &state,
            &glean_db,
            &input,
            &intel,
            &[],
            FinalizeMode::ManualRefresh {
                producer: crate::intel_queue::EnrichmentProducer::Glean,
            },
        )
        .expect("finalize Glean manual refresh");

        assert!(
            account_fact_claim_count(&glean_db, glean_account_id) > 0,
            "Glean manual refresh should promote account fact claims"
        );
        assert!(
            account_source_ref_count(&glean_db, glean_account_id) > 0,
            "Glean manual refresh should write source refs"
        );
        assert_eq!(
            claim_recompute_job_count(&glean_db, "account", glean_account_id),
            1,
            "Glean fact promotion should enqueue one account trust recompute job"
        );
        assert_eq!(
            signal_count(&glean_db, glean_account_id, "renewal_data_updated"),
            1,
            "Glean manual refresh should emit Glean renewal signals"
        );
        assert_eq!(
            technical_footprint_count(&glean_db, glean_account_id),
            1,
            "Glean manual refresh should write technical footprint evidence"
        );
    }

    #[test]
    fn finalize_post_commit_queue_worker_runs_full_chain() {
        // Supplemental Glean finalize needs a Tauri app handle; this pins the synchronous DB contract.
        assert_finalize_mode_contract(
            "acc-finalize-queue-mode",
            FinalizeMode::QueueWorker {
                is_background: false,
                producer: crate::intel_queue::EnrichmentProducer::Glean,
            },
            true,
        );
    }

    #[test]
    fn enrichment_finalize_does_not_stamp_success_when_relationship_persist_fails() {
        assert_finalize_relationship_failure_preserves_ordering(
            "acc-finalize-rel-fails",
            "mtg-finalize-rel-fails",
            "p-finalize-fail-1",
            "p-finalize-fail-2",
            FinalizeMode::ManualRefresh {
                producer: crate::intel_queue::EnrichmentProducer::Pty,
            },
        );
    }

    #[test]
    fn finalize_post_commit_negative_ordering_guard_runs_in_queue_mode() {
        assert_finalize_relationship_failure_preserves_ordering(
            "acc-finalize-rel-fails-queue",
            "mtg-finalize-rel-fails-queue",
            "p-finalize-fail-queue-1",
            "p-finalize-fail-queue-2",
            FinalizeMode::QueueWorker {
                is_background: false,
                producer: crate::intel_queue::EnrichmentProducer::Pty,
            },
        );
    }

    #[test]
    fn test_recompute_entity_health() {
        let db = test_db();
        let account = make_account("acc-health");
        db.upsert_account(&account).unwrap();

        // Seed minimal intelligence so recompute has something to work with
        let intel = IntelligenceJson {
            executive_assessment_render_policy: None,
            entity_id: "acc-health".to_string(),
            entity_type: "account".to_string(),
            enriched_at: chrono::Utc::now().to_rfc3339(),
            ..Default::default()
        };
        db.upsert_entity_intelligence(&intel).unwrap();
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 4, 30, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);

        super::recompute_entity_health(&ctx, &db, "acc-health", "account")
            .expect("recompute_entity_health");

        // Verify entity_quality updated with health_score
        let score: Option<f64> = db
            .conn_ref()
            .query_row(
                "SELECT health_score FROM entity_quality WHERE entity_id = 'acc-health'",
                [],
                |row| row.get(0),
            )
            .ok();
        assert!(score.is_some(), "entity_quality should have a health_score");
    }

    #[test]
    fn test_recompute_health_skips_non_account() {
        let db = test_db();
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 4, 30, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);

        // Should silently succeed for non-account types
        let result = super::recompute_entity_health(&ctx, &db, "proj-1", "project");
        assert!(result.is_ok(), "Should be Ok for non-account entity type");
    }

    #[test]
    fn test_persist_keywords_skips_unsupported_type() {
        let db = test_db();
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 4, 30, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);

        // Should silently succeed for unsupported entity types
        let result = super::persist_entity_keywords(&ctx, &db, "person", "p-1", r#"["test"]"#);
        assert!(result.is_ok(), "Should be Ok for unsupported entity type");

        // No signal should be emitted
        assert_eq!(
            signal_count(&db, "p-1", "keywords_updated"),
            0,
            "No signal for unsupported type"
        );
    }
}

///  end-to-end: parse Glean JSON → normalize → upsert to DB column → re-read.
///
/// This test exercises the full path from Glean's raw response through
/// `parse_leading_signals`, `upsert_health_outlook_signals`, and the SELECT
/// read that populates `AccountDetailResult.glean_signals`. It validates:
///  1. Bucket dispatch (champion_risk, channel_sentiment, commercial_signals survive)
///  2. JSON roundtrip through the DB column (camelCase storage ↔ camelCase struct)
///  3. Signal emissions (champion_at_risk, sentiment_divergence) fire correctly
#[cfg(test)]
mod dos15_leading_signals_db_tests {
    use crate::db::test_utils::test_db;
    use crate::db::{AccountType, DbAccount};
    use crate::intelligence::glean_leading_signals::{parse_leading_signals, HealthOutlookSignals};
    use crate::services::context::{ExternalClients, FixedClock, SeedableRng, ServiceContext};
    use crate::signals::propagation::PropagationEngine;
    use chrono::TimeZone;

    fn test_ctx<'a>(
        clock: &'a FixedClock,
        rng: &'a SeedableRng,
        ext: &'a ExternalClients,
    ) -> ServiceContext<'a> {
        ServiceContext::test_live(clock, rng, ext)
    }

    fn seed_account(db: &crate::db::ActionDb, id: &str) {
        let account = DbAccount {
            id: id.to_string(),
            name: format!("Test Account {id}"),
            lifecycle: Some("active".to_string()),
            arr: Some(200_000.0),
            account_type: AccountType::Customer,
            ..Default::default()
        };
        db.upsert_account(&account).unwrap();
        db.conn_ref()
            .execute(
                "INSERT OR IGNORE INTO entity_assessment (entity_id) VALUES (?1)",
                rusqlite::params![id],
            )
            .unwrap();
    }

    fn signal_count(db: &crate::db::ActionDb, entity_id: &str, signal_type: &str) -> i64 {
        db.conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM signal_events WHERE entity_id = ?1 AND signal_type = ?2",
                rusqlite::params![entity_id, signal_type],
                |row| row.get(0),
            )
            .unwrap_or(0)
    }

    /// Core roundtrip: parse Glean's snake_case JSON → upsert → SELECT → verify.
    #[test]
    fn glean_json_roundtrip_through_db_column() {
        let db = test_db();
        let engine = PropagationEngine::default();
        let entity_id = "dos15-roundtrip-test";
        seed_account(&db, entity_id);
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 4, 30, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);

        let glean_output = r#"{
            "champion_risk": {
                "champion_name": "Robin Taylor",
                "at_risk": true,
                "risk_level": "high",
                "risk_evidence": ["email response slowed 3x", "missed last QBR"],
                "backup_champion_candidates": [
                    { "name": "Jamie Lee", "role": "VP Ops", "engagement_level": "medium" }
                ]
            },
            "channel_sentiment": {
                "email": { "sentiment": "cooling", "trend_30d": "worsening" },
                "support_tickets": { "sentiment": "frustrated", "trend_30d": "worsening" },
                "divergence_detected": true,
                "divergence_summary": "tickets frustrated while Slack still cordial"
            },
            "commercial_signals": {
                "arr_direction": "flat",
                "payment_behavior": "on-time"
            },
            "quote_wall": [
                {
                    "quote": "We need better integration support.",
                    "speaker": "Robin Taylor",
                    "role": "Director of Data",
                    "date": "2026-03-15",
                    "source": "Gong",
                    "sentiment": "negative"
                }
            ]
        }"#;

        // Step 1: parse Glean's snake_case output into the normalized struct.
        let parsed =
            parse_leading_signals(glean_output).expect("parse_leading_signals should succeed");

        // Verify bucket dispatch before persistence.
        {
            let cr = parsed.champion_risk.as_ref().expect("champion_risk bucket");
            assert!(cr.at_risk, "champion should be at_risk");
            assert_eq!(cr.champion_name.as_deref(), Some("Robin Taylor"));
            assert_eq!(cr.risk_evidence.len(), 2, "2 evidence items");
            assert_eq!(cr.backup_champion_candidates.len(), 1);

            let cs = parsed
                .channel_sentiment
                .as_ref()
                .expect("channel_sentiment bucket");
            assert!(cs.divergence_detected, "divergence_detected should be true");

            let comm = parsed
                .commercial_signals
                .as_ref()
                .expect("commercial_signals bucket");
            assert_eq!(comm.arr_direction.as_deref(), Some("flat"));

            assert_eq!(parsed.quote_wall.len(), 1, "quote_wall should have 1 entry");
        }

        // Step 2: upsert to DB via the service function (also emits signals).
        super::upsert_health_outlook_signals(&ctx, &db, &engine, "account", entity_id, &parsed)
            .expect("upsert_health_outlook_signals should succeed");

        // Step 3: re-read from DB column (mirrors AccountDetailResult assembly).
        let stored_json: Option<String> = db
            .conn_ref()
            .query_row(
                "SELECT health_outlook_signals_json FROM entity_assessment WHERE entity_id = ?1",
                rusqlite::params![entity_id],
                |row| row.get(0),
            )
            .expect("SELECT should succeed");

        let stored_json = stored_json.expect("health_outlook_signals_json should not be NULL");
        let reread: HealthOutlookSignals =
            serde_json::from_str(&stored_json).expect("DB JSON should deserialize");

        // Step 4: verify every populated field survived the roundtrip.
        let cr = reread
            .champion_risk
            .as_ref()
            .expect("champion_risk after roundtrip");
        assert_eq!(cr.champion_name.as_deref(), Some("Robin Taylor"));
        assert!(cr.at_risk);
        assert_eq!(cr.risk_level.as_deref(), Some("high"));
        assert_eq!(cr.risk_evidence.len(), 2);
        assert_eq!(cr.backup_champion_candidates.len(), 1);
        assert_eq!(cr.backup_champion_candidates[0].name, "Jamie Lee");

        let cs = reread
            .channel_sentiment
            .as_ref()
            .expect("channel_sentiment after roundtrip");
        assert!(cs.divergence_detected);
        assert_eq!(
            cs.divergence_summary.as_deref(),
            Some("tickets frustrated while Slack still cordial")
        );

        assert_eq!(reread.quote_wall.len(), 1);
        assert_eq!(
            reread.quote_wall[0].quote,
            "We need better integration support."
        );

        // Step 5: verify derived signals were emitted correctly.
        assert!(
            signal_count(&db, entity_id, "champion_at_risk") > 0,
            "champion_at_risk signal should be emitted"
        );
        assert!(
            signal_count(&db, entity_id, "sentiment_divergence") > 0,
            "sentiment_divergence signal should be emitted"
        );
        // No competitor_decision_relevant or budget_cycle_locked in this fixture.
        assert_eq!(
            signal_count(&db, entity_id, "competitor_decision_relevant"),
            0,
            "no competitor signal expected"
        );
        assert_eq!(
            signal_count(&db, entity_id, "budget_cycle_locked"),
            0,
            "no budget_cycle_locked signal expected"
        );
    }

    /// NULL-safety: reading an account with no Glean enrichment must not panic.
    #[test]
    fn null_column_reads_as_none() {
        let db = test_db();
        let entity_id = "dos15-no-glean";
        seed_account(&db, entity_id);

        let result: Option<String> = db
            .conn_ref()
            .query_row(
                "SELECT health_outlook_signals_json FROM entity_assessment WHERE entity_id = ?1",
                rusqlite::params![entity_id],
                |row| row.get(0),
            )
            .ok()
            .flatten();

        assert!(result.is_none(), "unset column should read as NULL");
    }

    /// Idempotency: calling upsert twice overwrites cleanly — no duplicate rows.
    #[test]
    fn upsert_is_idempotent() {
        let db = test_db();
        let engine = PropagationEngine::default();
        let entity_id = "dos15-idempotent";
        seed_account(&db, entity_id);
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 4, 30, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);

        let first = parse_leading_signals(r#"{"champion_risk": null, "quote_wall": []}"#)
            .expect("parse first");
        super::upsert_health_outlook_signals(&ctx, &db, &engine, "account", entity_id, &first)
            .expect("first upsert");

        let second = parse_leading_signals(
            r#"{"champion_risk": {"champion_name": "New Champion", "at_risk": false, "risk_evidence": []}, "quote_wall": []}"#,
        )
        .expect("parse second");
        super::upsert_health_outlook_signals(&ctx, &db, &engine, "account", entity_id, &second)
            .expect("second upsert");

        // Read back — should reflect second write.
        let stored: Option<String> = db
            .conn_ref()
            .query_row(
                "SELECT health_outlook_signals_json FROM entity_assessment WHERE entity_id = ?1",
                rusqlite::params![entity_id],
                |row| row.get(0),
            )
            .ok()
            .flatten();

        let reread: HealthOutlookSignals =
            serde_json::from_str(&stored.expect("should be set")).expect("deserialize");
        let cr = reread.champion_risk.as_ref().expect("champion_risk");
        assert_eq!(cr.champion_name.as_deref(), Some("New Champion"));
    }
}

#[cfg(test)]
mod inferred_relationship_tests {
    use super::upsert_inferred_relationships_from_enrichment;
    use crate::db::person_relationships::UpsertRelationship;
    use crate::db::test_utils::test_db;
    use crate::intelligence::prompts::InferredRelationship;
    use crate::services::context::{ExternalClients, FixedClock, SeedableRng, ServiceContext};
    use chrono::TimeZone;

    fn test_ctx<'a>(
        clock: &'a FixedClock,
        rng: &'a SeedableRng,
        ext: &'a ExternalClients,
    ) -> ServiceContext<'a> {
        ServiceContext::test_live(clock, rng, ext)
    }

    fn seed_people(db: &crate::db::ActionDb) {
        db.conn_ref()
            .execute(
                "INSERT INTO people (id, email, name, updated_at) VALUES ('p1', 'p1@example.com', 'Alice', '2026-03-01T00:00:00Z')",
                [],
            )
            .expect("seed p1");
        db.conn_ref()
            .execute(
                "INSERT INTO people (id, email, name, updated_at) VALUES ('p2', 'p2@example.com', 'Bob', '2026-03-01T00:00:00Z')",
                [],
            )
            .expect("seed p2");
    }

    #[test]
    fn upsert_inferred_relationships_inserts_and_reinforces_without_duplicates() {
        let db = test_db();
        seed_people(&db);
        let engine = crate::signals::propagation::PropagationEngine::default();
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 4, 30, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);
        let inferred = vec![InferredRelationship {
            from_person_id: "p1".to_string(),
            to_person_id: "p2".to_string(),
            relationship_type: "collaborator".to_string(),
            rationale: Some("They co-own onboarding rollout workstreams.".to_string()),
        }];

        let inserted = upsert_inferred_relationships_from_enrichment(
            &ctx, &db, &engine, "account", "acc-1", &inferred,
        )
        .expect("first upsert");
        assert_eq!(inserted, 1);

        let rels = db
            .get_relationships_between("p1", "p2")
            .expect("relationship query");
        assert_eq!(rels.len(), 1);
        assert_eq!(rels[0].source, "ai_enrichment");
        assert!((rels[0].confidence - 0.6).abs() < f64::EPSILON);
        assert_eq!(rels[0].direction, "symmetric");
        assert_eq!(rels[0].context_entity_id.as_deref(), Some("acc-1"));
        assert_eq!(
            rels[0].rationale.as_deref(),
            Some("They co-own onboarding rollout workstreams.")
        );

        let signal_count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM signal_events
                 WHERE entity_type = 'account'
                   AND entity_id = 'acc-1'
                   AND signal_type = 'relationship_inferred'
                   AND data_source = 'ai_enrichment'",
                [],
                |row| row.get(0),
            )
            .expect("signal count");
        assert_eq!(signal_count, 1);

        let inserted_second = upsert_inferred_relationships_from_enrichment(
            &ctx, &db, &engine, "account", "acc-1", &inferred,
        )
        .expect("second upsert");
        assert_eq!(
            inserted_second, 0,
            "re-enrichment should reinforce, not duplicate"
        );
        assert_eq!(
            db.get_relationships_between("p1", "p2")
                .expect("relationship query 2")
                .len(),
            1
        );
    }

    #[test]
    fn upsert_inferred_relationships_skips_strong_user_confirmed_edges() {
        let db = test_db();
        seed_people(&db);
        let engine = crate::signals::propagation::PropagationEngine::default();
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 4, 30, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let ctx = test_ctx(&clock, &rng, &ext);
        db.upsert_person_relationship(&UpsertRelationship {
            id: "rel-user-1",
            from_person_id: "p1",
            to_person_id: "p2",
            relationship_type: "peer",
            direction: "symmetric",
            confidence: 0.9,
            context_entity_id: Some("acc-1"),
            context_entity_type: Some("account"),
            source: "user_confirmed",
            rationale: None,
        })
        .expect("seed user relationship");

        let inferred = vec![InferredRelationship {
            from_person_id: "p1".to_string(),
            to_person_id: "p2".to_string(),
            relationship_type: "manager".to_string(),
            rationale: Some("Model inferred reporting relationship.".to_string()),
        }];

        let inserted = upsert_inferred_relationships_from_enrichment(
            &ctx, &db, &engine, "account", "acc-1", &inferred,
        )
        .expect("upsert");
        assert_eq!(inserted, 0);

        let rels = db
            .get_relationships_between("p1", "p2")
            .expect("relationship query");
        assert_eq!(rels.len(), 1);
        assert_eq!(rels[0].source, "user_confirmed");
        assert_eq!(rels[0].relationship_type.to_string(), "peer");
    }
}

#[cfg(test)]
mod live_acceptance_tests {
    use std::collections::HashSet;
    use std::path::PathBuf;
    use std::sync::Arc;

    use chrono::Utc;
    use rusqlite::{params, OptionalExtension};

    use super::enrich_entity;
    use crate::db::data_lifecycle::{purge_source, DataSource};
    use crate::db::{ActionDb, DbPerson};
    use crate::intel_queue::{
        apply_enrichment_side_writes, compose_enrichment_intelligence,
        fenced_write_enrichment_intelligence, run_enrichment_post_commit_side_effects,
        EnrichmentInput,
    };
    use crate::intelligence::{
        write_intelligence_json, AccountHealth, ConsistencyStatus, DimensionScore, HealthSource,
        HealthTrend, IntelRisk, IntelligenceJson, RelationshipDimensions,
    };
    use crate::state::AppState;

    /// Live acceptance check for using the user's real local dataset.
    /// Run manually:
    /// `cargo test --lib services::intelligence::live_acceptance_tests::i527_live_end_to_end_data_flow -- --ignored --nocapture`
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ignore = "Live validation: requires configured local DB/workspace and AI runtime"]
    async fn i527_live_end_to_end_data_flow() {
        let state = Arc::new(AppState::new());
        let _ = state.init_db_service().await;

        // Pick a real account/project entity that has attendee history.
        let candidate = state
            .db_read(|db| {
                let mut stmt = db
                    .conn_ref()
                    .prepare(
                        "SELECT me.entity_id, me.entity_type
                         FROM effective_meeting_entities me
                         JOIN meeting_attendees ma ON ma.meeting_id = me.meeting_id
                         WHERE me.entity_type IN ('account', 'project')
                         GROUP BY me.entity_id, me.entity_type
                         ORDER BY COUNT(DISTINCT ma.person_id) DESC,
                                  COUNT(DISTINCT ma.meeting_id) DESC
                         LIMIT 1",
                    )
                    .map_err(|e| format!("prepare candidate query failed: {e}"))?;

                let mut rows = stmt
                    .query([])
                    .map_err(|e| format!("candidate query failed: {e}"))?;

                if let Some(row) = rows
                    .next()
                    .map_err(|e| format!("candidate row read failed: {e}"))?
                {
                    let entity_id: String = row
                        .get(0)
                        .map_err(|e| format!("candidate entity_id read failed: {e}"))?;
                    let entity_type: String = row
                        .get(1)
                        .map_err(|e| format!("candidate entity_type read failed: {e}"))?;
                    Ok(Some((entity_id, entity_type)))
                } else {
                    Ok(None)
                }
            })
            .await
            .expect("failed to select live entity candidate")
            .expect("no account/project with attendee evidence found in live DB");

        let (entity_id, entity_type) = candidate;
        eprintln!(
            "I527 live validation using entity: {} ({})",
            entity_id, entity_type
        );

        // End-to-end path: gather context -> AI enrichment -> deterministic consistency pass ->
        // write intelligence.json + DB cache.
        let ctx = state.live_service_context();
        let request_id = crate::audit_log::new_request_id();
        let intel = enrich_entity(
            &ctx,
            entity_id.clone(),
            entity_type.clone(),
            &state,
            None,
            &request_id,
        )
        .await
        .expect("manual enrich_entity failed");

        assert!(
            intel.consistency_status.is_some(),
            "consistency_status must be set after enrichment write path"
        );
        assert!(
            intel.consistency_checked_at.is_some(),
            "consistency_checked_at must be set after enrichment write path"
        );

        let entity_id_for_db = entity_id.clone();
        let persisted = state
            .db_read(move |db| {
                db.get_entity_intelligence(&entity_id_for_db)
                    .map_err(|e| format!("get_entity_intelligence failed: {e}"))
            })
            .await
            .expect("DB read failed")
            .expect("persisted entity_intelligence row missing after enrichment");

        assert_eq!(
            persisted.consistency_status, intel.consistency_status,
            "DB cache consistency status should match write result",
        );
        assert!(
            persisted.consistency_checked_at.is_some(),
            "DB cache must persist consistency_checked_at"
        );

        // Pull one real linked meeting and run full briefing refresh path.
        let entity_id_for_meeting = entity_id.clone();
        let entity_type_for_meeting = entity_type.clone();
        let meeting_id = state
            .db_read(move |db| {
                db.conn_ref()
                    .query_row(
                        "SELECT meeting_id
                         FROM effective_meeting_entities
                         WHERE entity_id = ?1 AND entity_type = ?2
                         ORDER BY meeting_id DESC
                         LIMIT 1",
                        rusqlite::params![entity_id_for_meeting, entity_type_for_meeting],
                        |row| row.get::<_, String>(0),
                    )
                    .optional()
                    .map_err(|e| format!("meeting lookup failed: {e}"))
            })
            .await
            .expect("meeting lookup query failed")
            .expect("no linked meeting found for entity");

        let request_id = crate::audit_log::new_request_id();
        let refresh = crate::services::meetings::refresh_meeting_briefing_full(
            &ctx,
            &state,
            &meeting_id,
            None,
            &request_id,
        )
        .await
        .expect("refresh_meeting_briefing_full failed");

        assert!(
            refresh.prep_rebuilt_sync || refresh.prep_queued,
            "refresh should rebuild prep sync or queue it"
        );

        let detail = crate::services::meetings::get_meeting_intelligence(&ctx, &state, &meeting_id)
            .await
            .expect("get_meeting_intelligence failed after refresh");
        let prep = detail
            .prep
            .expect("meeting detail should include prep after manual refresh");
        assert!(
            prep.consistency_status.is_some(),
            "meeting prep should include propagated consistency_status"
        );
    }

    /// Live deterministic-guardrail validation for acceptance criteria:
    /// - contradiction auto-correction/flagging
    /// - refresh overwrite (not stuck on corrected/flagged state)
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ignore = "Live validation: mutates one real entity intelligence row, then restores it"]
    async fn i527_live_deterministic_guardrail_acceptance() {
        let state = Arc::new(AppState::new());
        let _ = state.init_db_service().await;

        let workspace_path = state
            .config
            .read()
            .as_ref()
            .map(|c| c.workspace_path.clone())
            .expect("No config loaded");
        let workspace = std::path::Path::new(&workspace_path);

        let candidate = state
            .db_read(|db| {
                let mut stmt = db
                    .conn_ref()
                    .prepare(
                        "SELECT me.entity_id, me.entity_type
                         FROM effective_meeting_entities me
                         JOIN meeting_attendees ma ON ma.meeting_id = me.meeting_id
                         LEFT JOIN signal_events se
                           ON se.entity_id = me.entity_id
                          AND se.entity_type = me.entity_type
                          AND se.superseded_by IS NULL
                          AND se.created_at >= datetime('now', '-14 days')
                         WHERE me.entity_type IN ('account', 'project')
                         GROUP BY me.entity_id, me.entity_type
                         HAVING COUNT(DISTINCT ma.meeting_id) >= 1
                            AND COUNT(DISTINCT se.id) >= 2
                         ORDER BY COUNT(DISTINCT se.id) DESC,
                                  COUNT(DISTINCT ma.meeting_id) DESC
                         LIMIT 1",
                    )
                    .map_err(|e| format!("prepare candidate query failed: {e}"))?;

                let mut rows = stmt
                    .query([])
                    .map_err(|e| format!("candidate query failed: {e}"))?;

                if let Some(row) = rows
                    .next()
                    .map_err(|e| format!("candidate row read failed: {e}"))?
                {
                    let entity_id: String = row
                        .get(0)
                        .map_err(|e| format!("candidate entity_id read failed: {e}"))?;
                    let entity_type: String = row
                        .get(1)
                        .map_err(|e| format!("candidate entity_type read failed: {e}"))?;
                    Ok(Some((entity_id, entity_type)))
                } else {
                    Ok(None)
                }
            })
            .await
            .expect("candidate lookup failed")
            .expect("no suitable live entity with attendee+signal evidence found");

        let (entity_id, entity_type) = candidate;
        let entity_dir = state
            .db_read({
                let entity_id = entity_id.clone();
                let entity_type = entity_type.clone();
                let workspace = workspace.to_path_buf();
                move |db| {
                    let account_opt = if entity_type == "account" {
                        db.get_account(&entity_id).map_err(|e| e.to_string())?
                    } else {
                        None
                    };
                    let entity_name = match entity_type.as_str() {
                        "account" => account_opt
                            .as_ref()
                            .map(|a| a.name.clone())
                            .ok_or_else(|| format!("account not found: {entity_id}"))?,
                        "project" => db
                            .get_project(&entity_id)
                            .map_err(|e| e.to_string())?
                            .map(|p| p.name)
                            .ok_or_else(|| format!("project not found: {entity_id}"))?,
                        _ => return Err(format!("unsupported entity_type: {}", entity_type)),
                    };
                    crate::intelligence::resolve_entity_dir(
                        &workspace,
                        &entity_type,
                        &entity_name,
                        account_opt.as_ref(),
                    )
                }
            })
            .await
            .expect("resolve entity dir failed");

        let facts = state
            .db_read({
                let entity_id = entity_id.clone();
                let entity_type = entity_type.clone();
                move |db| crate::intelligence::build_fact_context(db, &entity_id, &entity_type)
            })
            .await
            .expect("build_fact_context failed");

        let stakeholder = facts
            .stakeholders
            .iter()
            .find(|s| s.attendance_count > 0)
            .expect("candidate entity has no attendance-backed stakeholder")
            .name
            .clone();

        let previous_db = state
            .db_read({
                let entity_id = entity_id.clone();
                move |db| {
                    db.get_entity_intelligence(&entity_id)
                        .map_err(|e| e.to_string())
                }
            })
            .await
            .expect("previous DB read failed");
        let previous_file = previous_db.clone();

        let contradictory = IntelligenceJson {
            executive_assessment_render_policy: None,
            version: 1,
            entity_id: entity_id.clone(),
            entity_type: entity_type.clone(),
            enriched_at: Utc::now().to_rfc3339(),
            executive_assessment: Some(format!(
                "{} has never appeared in a recorded meeting and no new progress signals since the prior assessment.",
                stakeholder
            )),
            risks: vec![IntelRisk {
                render_policy: None,
                claim_id: None,
                text: format!("{} has never appeared in a recorded meeting.", stakeholder),
                source: Some("live-acceptance-test".to_string()),
                urgency: "critical".to_string(),
                item_source: None,
                headline: None,
                evidence: None,
                kind_label: None,
                discrepancy: None,
            }],
            ..Default::default()
        };

        let input = EnrichmentInput {
            workspace: workspace.to_path_buf(),
            entity_dir: entity_dir.clone(),
            entity_id: entity_id.clone(),
            entity_type: entity_type.clone(),
            prompt: String::new(),
            file_manifest: Vec::new(),
            file_count: 0,
            computed_health: None,
            entity_name: String::new(),
            relationship: None,
            intelligence_context: None,
            active_preset: None,
        };

        let db = ActionDb::open(std::sync::Arc::new(crate::db::LocalKeychain::new()))
            .expect("open DB for first enrichment persistence");
        let ctx = state.live_service_context();
        let first_prepared = compose_enrichment_intelligence(
            &state,
            &db,
            &input,
            &contradictory,
            crate::intel_queue::EnrichmentProducer::Pty,
            None,
        )
        .expect("first compose_enrichment_intelligence failed");
        db.with_transaction(|tx| {
            apply_enrichment_side_writes(&ctx, tx, &input, &first_prepared)?;
            super::upsert_assessment_from_enrichment_in_active_transaction(
                &ctx,
                tx,
                &state.signals.engine,
                super::EnrichmentAssessmentUpsert {
                    entity_type: &input.entity_type,
                    entity_id: &input.entity_id,
                    intel: first_prepared.intelligence(),
                    projection_intel: first_prepared.projection_intelligence(),
                    projection_data_source: "ai_enrichment",
                    cleared_dimensions: first_prepared.cleared_dimensions(),
                },
            )
        })
        .expect("first enrichment DB persistence failed");
        fenced_write_enrichment_intelligence(&db, &input.entity_dir, first_prepared.intelligence());
        run_enrichment_post_commit_side_effects(
            &state,
            &input,
            &db,
            first_prepared.intelligence(),
            crate::intel_queue::EnrichmentProducer::Pty,
        )
        .expect("first post-commit side effects");
        let first = first_prepared.into_intelligence();

        let first_assessment = first
            .executive_assessment
            .as_deref()
            .unwrap_or_default()
            .to_lowercase();
        assert!(
            !first_assessment.contains("never appeared"),
            "absence contradiction should be auto-corrected"
        );
        assert!(
            !first_assessment.contains("no new progress signals"),
            "no-progress contradiction should be auto-corrected when 14d signals >= 2"
        );
        assert!(
            matches!(
                first.consistency_status,
                Some(ConsistencyStatus::Corrected) | Some(ConsistencyStatus::Flagged)
            ),
            "contradictory payload must be corrected or flagged"
        );

        let clean = IntelligenceJson {
            executive_assessment_render_policy: None,
            version: 1,
            entity_id: entity_id.clone(),
            entity_type: entity_type.clone(),
            enriched_at: Utc::now().to_rfc3339(),
            executive_assessment: Some("Fresh validated summary from later refresh.".to_string()),
            ..Default::default()
        };
        let db = ActionDb::open(std::sync::Arc::new(crate::db::LocalKeychain::new()))
            .expect("open DB for second enrichment persistence");
        let ctx = state.live_service_context();
        let second_prepared = compose_enrichment_intelligence(
            &state,
            &db,
            &input,
            &clean,
            crate::intel_queue::EnrichmentProducer::Pty,
            None,
        )
        .expect("second compose_enrichment_intelligence failed");
        db.with_transaction(|tx| {
            apply_enrichment_side_writes(&ctx, tx, &input, &second_prepared)?;
            super::upsert_assessment_from_enrichment_in_active_transaction(
                &ctx,
                tx,
                &state.signals.engine,
                super::EnrichmentAssessmentUpsert {
                    entity_type: &input.entity_type,
                    entity_id: &input.entity_id,
                    intel: second_prepared.intelligence(),
                    projection_intel: second_prepared.projection_intelligence(),
                    projection_data_source: "ai_enrichment",
                    cleared_dimensions: second_prepared.cleared_dimensions(),
                },
            )
        })
        .expect("second enrichment DB persistence failed");
        fenced_write_enrichment_intelligence(
            &db,
            &input.entity_dir,
            second_prepared.intelligence(),
        );
        run_enrichment_post_commit_side_effects(
            &state,
            &input,
            &db,
            second_prepared.intelligence(),
            crate::intel_queue::EnrichmentProducer::Pty,
        )
        .expect("second post-commit side effects");
        let second = second_prepared.into_intelligence();

        assert!(
            second
                .executive_assessment
                .as_deref()
                .unwrap_or_default()
                .contains("Fresh validated summary"),
            "later refresh should overwrite prior corrected/flagged output"
        );
        assert!(
            second.consistency_checked_at.is_some(),
            "later refresh must still run a new consistency pass"
        );

        // Restore prior data so live workspace stays unchanged after validation.
        match previous_db {
            Some(prev) => {
                let _ = state
                    .db_write(move |db| {
                        db.upsert_entity_intelligence(&prev)
                            .map_err(|e| e.to_string())
                    })
                    .await
                    .map_err(String::from);
            }
            None => {
                let entity_id_for_delete = entity_id.clone();
                let _ = state
                    .db_write(move |db| {
                        db.delete_entity_intelligence(&entity_id_for_delete)
                            .map_err(|e| e.to_string())
                    })
                    .await
                    .map_err(String::from);
            }
        }

        if let Some(prev_file) = previous_file {
            // Test cleanup: best-effort restore. Bypasses the schema-epoch
            // fence intentionally — this runs at end-of-test to restore the
            // pre-test workspace state and has no live migration to honor.
            // fence-exempt: test-cleanup
            write_intelligence_json(&entity_dir, &prev_file).ok();
        } else {
            std::fs::remove_file(entity_dir.join("intelligence.json")).ok();
        }
    }

    /// Live Janus scenario: if Matt Wickham has attendance evidence, a
    /// "never appeared" claim must be flagged/corrected.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ignore = "Live validation for Janus/Matt evidence path"]
    async fn i527_live_janus_matt_absence_guardrail() {
        let state = Arc::new(AppState::new());
        let _ = state.init_db_service().await;

        let entity_id = state
            .db_read(|db| {
                db.conn_ref()
                    .query_row(
                        "SELECT entity_id
                         FROM effective_meeting_entities
                         WHERE entity_type = 'account'
                           AND LOWER(entity_id) LIKE '%janus%'
                         LIMIT 1",
                        [],
                        |row| row.get::<_, String>(0),
                    )
                    .optional()
                    .map_err(|e| format!("janus lookup failed: {e}"))
            })
            .await
            .expect("janus lookup query failed")
            .expect("No Janus entity linked in meeting_entities");

        let facts = state
            .db_read({
                let entity_id = entity_id.clone();
                move |db| crate::intelligence::build_fact_context(db, &entity_id, "account")
            })
            .await
            .expect("build_fact_context failed for Janus");

        let matt = facts
            .stakeholders
            .iter()
            .find(|s| s.name.to_lowercase().contains("wickham"))
            .expect("Matt Wickham not found in Janus stakeholder facts");
        assert!(
            matt.attendance_count >= 1,
            "Matt Wickham should have deterministic attendance evidence for this scenario"
        );

        let contradictory = IntelligenceJson {
            executive_assessment_render_policy: None,
            version: 1,
            entity_id: entity_id.clone(),
            entity_type: "account".to_string(),
            enriched_at: Utc::now().to_rfc3339(),
            executive_assessment: Some(
                "Matt Wickham has never appeared in a recorded meeting.".to_string(),
            ),
            ..Default::default()
        };

        let report = crate::intelligence::check_consistency(&contradictory, &facts);
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.code == "ABSENCE_CONTRADICTION"),
            "False absence claim should be detected for Janus/Matt"
        );

        let repaired =
            crate::intelligence::apply_deterministic_repairs(&contradictory, &report, &facts);
        let post = crate::intelligence::check_consistency(&repaired, &facts);
        assert!(
            post.findings
                .iter()
                .all(|f| f.code != "ABSENCE_CONTRADICTION"),
            "Deterministic repair should clear Janus/Matt absence contradiction"
        );
    }

    /// Wave 1 live acceptance  on an encrypted snapshot of the
    /// user's real DB. Safe: mutates backup only.
    #[test]
    #[ignore = "Live validation: uses real DB snapshot and performs destructive purge checks on snapshot only"]
    fn wave1_live_snapshot_i503_i528_acceptance() {
        let live_db = ActionDb::open(std::sync::Arc::new(crate::db::LocalKeychain::new()))
            .expect("open live DB");
        let backup_path = crate::db_backup::backup_database(&live_db).expect("create live backup");
        let snapshot_db = ActionDb::open_at(
            PathBuf::from(&backup_path),
            std::sync::Arc::new(crate::db::LocalKeychain::new()),
        )
        .expect("open snapshot backup DB");

        // ---------------------------------------------------------------------
        // structured health write/read + legacy compatibility
        // ---------------------------------------------------------------------
        let structured = IntelligenceJson {
            executive_assessment_render_policy: None,
            version: 1,
            entity_id: "wave1-i503-structured".to_string(),
            entity_type: "account".to_string(),
            enriched_at: Utc::now().to_rfc3339(),
            health: Some(AccountHealth {
                score: 82.0,
                band: "green".to_string(),
                source: HealthSource::Computed,
                confidence: 0.78,
                sufficient_data: false, // Only 1 dimension populated in test
                trend: HealthTrend {
                    direction: "improving".to_string(),
                    rationale: Some("Usage and expansion improved".to_string()),
                    timeframe: "30d".to_string(),
                    confidence: 0.7,
                    ..Default::default()
                },
                dimensions: RelationshipDimensions {
                    meeting_cadence: DimensionScore {
                        score: 80.0,
                        weight: 0.2,
                        evidence: vec!["weekly exec sync".to_string()],
                        trend: "improving".to_string(),
                    },
                    email_engagement: DimensionScore::default(),
                    stakeholder_coverage: DimensionScore::default(),
                    key_advocate_health: DimensionScore::default(),
                    financial_proximity: DimensionScore::default(),
                    signal_momentum: DimensionScore::default(),
                },
                divergence: None,
                narrative: Some("Healthy multi-threaded account".to_string()),
                recommended_actions: vec!["Expand to procurement".to_string()],
            }),
            ..Default::default()
        };
        snapshot_db
            .upsert_entity_intelligence(&structured)
            .expect("upsert structured health");
        let structured_back = snapshot_db
            .get_entity_intelligence("wave1-i503-structured")
            .expect("get structured health row")
            .expect("structured row missing");
        let structured_health = structured_back
            .health
            .expect("structured health should deserialize");
        assert_eq!(structured_health.score, 82.0);
        assert_eq!(structured_health.band, "green");
        assert_eq!(structured_health.trend.direction, "improving");

        snapshot_db
            .conn_ref()
            .execute(
                "INSERT OR REPLACE INTO entity_assessment (entity_id, entity_type, enriched_at, source_file_count)
                 VALUES (?1, 'account', ?2, 0)",
                params!["wave1-i503-legacy", Utc::now().to_rfc3339()],
            )
            .expect("seed legacy entity_assessment row");
        let legacy_trend_json = serde_json::json!({
            "direction": "declining",
            "rationale": "Legacy trend payload"
        })
        .to_string();
        snapshot_db
            .conn_ref()
            .execute(
                "INSERT OR REPLACE INTO entity_quality (entity_id, entity_type, health_score, health_trend)
                 VALUES (?1, 'account', 35.0, ?2)",
                params!["wave1-i503-legacy", legacy_trend_json],
            )
            .expect("seed legacy entity_quality row");
        let legacy_back = snapshot_db
            .get_entity_intelligence("wave1-i503-legacy")
            .expect("get legacy compatibility row")
            .expect("legacy row missing");
        let legacy_health = legacy_back
            .health
            .expect("legacy scalar health should synthesize into structured health");
        assert_eq!(legacy_health.band, "red");
        assert_eq!(legacy_health.score, 35.0);
        assert_eq!(legacy_health.trend.direction, "declining");

        let legacy_dir = tempfile::tempdir().expect("legacy tempdir");
        let legacy_json = serde_json::json!({
            "entityId": "legacy-file-entity",
            "entityType": "account",
            "healthScore": 73.0,
            "healthTrend": {
                "direction": "stable",
                "rationale": "Legacy file compatibility"
            }
        });
        std::fs::write(
            legacy_dir.path().join("intelligence.json"),
            serde_json::to_string_pretty(&legacy_json).expect("serialize legacy intelligence file"),
        )
        .expect("write legacy intelligence file");
        let parsed_legacy_file = crate::intelligence::read_intelligence_json(legacy_dir.path())
            .expect("read legacy intelligence file");
        let file_health = parsed_legacy_file
            .health
            .expect("healthScore/healthTrend should map to structured health");
        assert_eq!(file_health.score, 73.0);
        assert_eq!(file_health.band, "green");
        assert_eq!(file_health.trend.direction, "stable");

        // ---------------------------------------------------------------------
        // purge semantics (glean + google) against snapshot
        // ---------------------------------------------------------------------
        let marker = format!("wave1-i528-{}", Utc::now().timestamp());
        let account_id: String = snapshot_db
            .conn_ref()
            .query_row("SELECT id FROM accounts LIMIT 1", [], |row| row.get(0))
            .expect("load existing account id for FK-safe purge seeding");

        let google_person_id = format!("{marker}-p-google");
        let glean_person_id = format!("{marker}-p-glean");
        let user_person_id = format!("{marker}-p-user");
        let google_enrichment_sources = serde_json::json!({
            "linkedin_url": {"source": "google", "at": "2026-03-07T00:00:00Z"},
            "bio": {"source": "user", "at": "2026-03-07T00:00:00Z"}
        })
        .to_string();
        let person = DbPerson {
            id: google_person_id.clone(),
            email: format!("{marker}-google@example.com"),
            name: format!("{marker}-google"),
            organization: Some("Wave1 Org".to_string()),
            role: Some("Director".to_string()),
            relationship: "external".to_string(),
            notes: None,
            tracker_path: None,
            last_seen: None,
            first_seen: None,
            meeting_count: 0,
            updated_at: Utc::now().to_rfc3339(),
            archived: false,
            linkedin_url: Some("https://linkedin.com/in/wave1".to_string()),
            twitter_handle: None,
            phone: None,
            photo_url: None,
            bio: Some("Wave1 profile".to_string()),
            title_history: None,
            company_industry: None,
            company_size: None,
            company_hq: None,
            last_enriched_at: None,
            enrichment_sources: Some(google_enrichment_sources.clone()),
        };
        snapshot_db
            .upsert_person(&person)
            .expect("seed snapshot person");
        snapshot_db
            .conn_ref()
            .execute(
                "UPDATE people
                 SET linkedin_url = ?1, bio = ?2, enrichment_sources = ?3
                 WHERE id = ?4",
                params![
                    "https://linkedin.com/in/wave1",
                    "Wave1 profile",
                    google_enrichment_sources,
                    google_person_id.clone()
                ],
            )
            .expect("seed people profile fields");
        snapshot_db
            .upsert_person(&DbPerson {
                id: glean_person_id.clone(),
                email: format!("{marker}-glean@example.com"),
                name: format!("{marker}-glean"),
                organization: Some("Wave1 Org".to_string()),
                role: Some("Champion".to_string()),
                relationship: "external".to_string(),
                notes: None,
                tracker_path: None,
                last_seen: None,
                first_seen: None,
                meeting_count: 0,
                updated_at: Utc::now().to_rfc3339(),
                archived: false,
                linkedin_url: None,
                twitter_handle: None,
                phone: None,
                photo_url: None,
                bio: None,
                title_history: None,
                company_industry: None,
                company_size: None,
                company_hq: None,
                last_enriched_at: None,
                enrichment_sources: None,
            })
            .expect("seed glean person");
        snapshot_db
            .upsert_person(&DbPerson {
                id: user_person_id.clone(),
                email: format!("{marker}-user@example.com"),
                name: format!("{marker}-user"),
                organization: Some("Wave1 Org".to_string()),
                role: Some("Champion".to_string()),
                relationship: "external".to_string(),
                notes: None,
                tracker_path: None,
                last_seen: None,
                first_seen: None,
                meeting_count: 0,
                updated_at: Utc::now().to_rfc3339(),
                archived: false,
                linkedin_url: None,
                twitter_handle: None,
                phone: None,
                photo_url: None,
                bio: None,
                title_history: None,
                company_industry: None,
                company_size: None,
                company_hq: None,
                last_enriched_at: None,
                enrichment_sources: None,
            })
            .expect("seed user person");

        let account_glean = account_id.clone();
        let account_google = account_id.clone();
        let account_user = account_id.clone();

        let user_stakeholders_before: i64 = snapshot_db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM account_stakeholders WHERE data_source = 'user'",
                [],
                |row| row.get(0),
            )
            .expect("count user stakeholders before");
        let user_signals_before: i64 = snapshot_db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM signal_events WHERE data_source = 'user'",
                [],
                |row| row.get(0),
            )
            .expect("count user signals before");
        let user_relationships_before: i64 = snapshot_db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM person_relationships WHERE source = 'user_confirmed'",
                [],
                |row| row.get(0),
            )
            .expect("count user relationships before");

        snapshot_db
            .link_person_to_account_with_source(
                &account_glean,
                &glean_person_id,
                "champion",
                "glean",
            )
            .expect("seed glean stakeholder");
        snapshot_db
            .link_person_to_account_with_source(
                &account_google,
                &google_person_id,
                "champion",
                "google",
            )
            .expect("seed google stakeholder");
        snapshot_db
            .link_person_to_account_with_source(&account_user, &user_person_id, "champion", "user")
            .expect("seed user stakeholder");

        let created_at = Utc::now().to_rfc3339();
        crate::services::signals::emit_fixture_event(
            &snapshot_db,
            &format!("{marker}-sig-glean"),
            "account",
            &account_glean,
            "profile_update",
            "glean",
            None,
            0.8,
            None,
            &created_at,
        )
        .expect("seed glean signal");
        crate::services::signals::emit_fixture_event(
            &snapshot_db,
            &format!("{marker}-sig-google"),
            "account",
            &account_google,
            "profile_update",
            "google",
            None,
            0.8,
            None,
            &created_at,
        )
        .expect("seed google signal");
        crate::services::signals::emit_fixture_event(
            &snapshot_db,
            &format!("{marker}-sig-user"),
            "account",
            &account_user,
            "profile_update",
            "user",
            None,
            0.8,
            None,
            &created_at,
        )
        .expect("seed user signal");

        snapshot_db
            .conn_ref()
            .execute(
                "INSERT INTO person_relationships
                 (id, from_person_id, to_person_id, relationship_type, direction, confidence, source)
                 VALUES (?1, ?2, ?2, 'peer', 'symmetric', 0.8, 'glean')",
                params![format!("{marker}-rel-glean"), glean_person_id],
            )
            .expect("seed glean relationship");
        snapshot_db
            .conn_ref()
            .execute(
                "INSERT INTO person_relationships
                 (id, from_person_id, to_person_id, relationship_type, direction, confidence, source)
                 VALUES (?1, ?2, ?2, 'peer', 'symmetric', 0.8, 'google')",
                params![format!("{marker}-rel-google"), google_person_id],
            )
            .expect("seed google relationship");
        snapshot_db
            .conn_ref()
            .execute(
                "INSERT INTO person_relationships
                 (id, from_person_id, to_person_id, relationship_type, direction, confidence, source)
                 VALUES (?1, ?2, ?2, 'peer', 'symmetric', 0.9, 'user_confirmed')",
                params![format!("{marker}-rel-user"), user_person_id],
            )
            .expect("seed user relationship");

        let glean_report = purge_source(&snapshot_db, DataSource::Glean).expect("purge glean");
        assert_eq!(glean_report.source, "glean");
        assert!(
            glean_report.people_cleared >= 1
                && glean_report.signals_deleted >= 1
                && glean_report.relationships_deleted >= 1,
            "glean purge should remove source-owned records"
        );

        let glean_stakeholders_left: i64 = snapshot_db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM account_stakeholders WHERE data_source = 'glean'",
                [],
                |row| row.get(0),
            )
            .expect("count glean stakeholders");
        let glean_signals_left: i64 = snapshot_db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM signal_events WHERE data_source = 'glean'",
                [],
                |row| row.get(0),
            )
            .expect("count glean signals");
        let glean_relationships_left: i64 = snapshot_db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM person_relationships WHERE source = 'glean'",
                [],
                |row| row.get(0),
            )
            .expect("count glean relationships");
        assert_eq!(glean_stakeholders_left, 0);
        assert_eq!(glean_signals_left, 0);
        assert_eq!(glean_relationships_left, 0);

        let user_stakeholders_mid: i64 = snapshot_db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM account_stakeholders WHERE data_source = 'user'",
                [],
                |row| row.get(0),
            )
            .expect("count user stakeholders after glean purge");
        let user_signals_mid: i64 = snapshot_db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM signal_events WHERE data_source = 'user'",
                [],
                |row| row.get(0),
            )
            .expect("count user signals after glean purge");
        let user_relationships_mid: i64 = snapshot_db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM person_relationships WHERE source = 'user_confirmed'",
                [],
                |row| row.get(0),
            )
            .expect("count user relationships after glean purge");
        assert_eq!(user_stakeholders_mid, user_stakeholders_before + 1);
        assert_eq!(user_signals_mid, user_signals_before + 1);
        assert_eq!(user_relationships_mid, user_relationships_before + 1);

        let google_report = purge_source(&snapshot_db, DataSource::Google).expect("purge google");
        assert_eq!(google_report.source, "google");
        assert!(
            google_report.people_cleared >= 1
                && google_report.signals_deleted >= 1
                && google_report.relationships_deleted >= 1,
            "google purge should remove source-owned records"
        );

        let google_stakeholders_left: i64 = snapshot_db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM account_stakeholders WHERE data_source = 'google'",
                [],
                |row| row.get(0),
            )
            .expect("count google stakeholders");
        let google_signals_left: i64 = snapshot_db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM signal_events WHERE data_source = 'google'",
                [],
                |row| row.get(0),
            )
            .expect("count google signals");
        let google_relationships_left: i64 = snapshot_db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM person_relationships WHERE source = 'google'",
                [],
                |row| row.get(0),
            )
            .expect("count google relationships");
        assert_eq!(google_stakeholders_left, 0);
        assert_eq!(google_signals_left, 0);
        assert_eq!(google_relationships_left, 0);

        let user_stakeholders_after: i64 = snapshot_db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM account_stakeholders WHERE data_source = 'user'",
                [],
                |row| row.get(0),
            )
            .expect("count user stakeholders after google purge");
        let user_signals_after: i64 = snapshot_db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM signal_events WHERE data_source = 'user'",
                [],
                |row| row.get(0),
            )
            .expect("count user signals after google purge");
        let user_relationships_after: i64 = snapshot_db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM person_relationships WHERE source = 'user_confirmed'",
                [],
                |row| row.get(0),
            )
            .expect("count user relationships after google purge");
        assert_eq!(user_stakeholders_after, user_stakeholders_before + 1);
        assert_eq!(user_signals_after, user_signals_before + 1);
        assert_eq!(user_relationships_after, user_relationships_before + 1);

        let (linkedin_after, bio_after, sources_after): (
            Option<String>,
            Option<String>,
            Option<String>,
        ) = snapshot_db
            .conn_ref()
            .query_row(
                "SELECT linkedin_url, bio, enrichment_sources FROM people WHERE id = ?1",
                params![google_person_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("read person profile after google purge");
        assert!(
            linkedin_after.is_none(),
            "google-owned linkedin_url should be cleared; got linkedin={:?}, sources={:?}",
            linkedin_after,
            sources_after
        );
        assert_eq!(
            bio_after.as_deref(),
            Some("Wave1 profile"),
            "user-owned bio should remain"
        );
    }

    /// Wave 1 live acceptance  against real data enrichment path.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ignore = "Live validation: runs real AI enrichment and checks inferred relationships in local DB"]
    async fn wave1_live_i504_end_to_end_relationship_acceptance() {
        let state = Arc::new(AppState::new());
        let _ = state.init_db_service().await;

        // Prefer an account that already has AI-inferred edges and >=3 stakeholders
        // for deterministic validation across reruns.
        let candidate = state
            .db_read(|db| {
                db.conn_ref()
                    .query_row(
                        "SELECT pr.context_entity_id
                         FROM person_relationships pr
                         JOIN account_stakeholders s
                           ON s.account_id = pr.context_entity_id
                         WHERE pr.source = 'ai_enrichment'
                           AND pr.context_entity_type = 'account'
                         GROUP BY pr.context_entity_id
                         HAVING COUNT(DISTINCT s.person_id) >= 3
                         ORDER BY COUNT(*) ASC, COUNT(DISTINCT s.person_id) ASC
                         LIMIT 1",
                        [],
                        |row| row.get::<_, String>(0),
                    )
                    .optional()
                    .map_err(|e| format!("preferred candidate query failed: {e}"))
            })
            .await
            .expect("preferred candidate query error");

        let account_id = if let Some(id) = candidate {
            id
        } else {
            state
                .db_read(|db| {
                    db.conn_ref()
                        .query_row(
                            "SELECT s.account_id
                             FROM account_stakeholders s
                             GROUP BY s.account_id
                             HAVING COUNT(DISTINCT s.person_id) >= 3
                             ORDER BY COUNT(DISTINCT s.person_id) ASC
                             LIMIT 1",
                            [],
                            |row| row.get::<_, String>(0),
                        )
                        .map_err(|e| format!("fallback candidate query failed: {e}"))
                })
                .await
                .expect("fallback candidate query error")
        };

        let (before_rows, before_signals): (i64, i64) = state
            .db_read({
                let account_id = account_id.clone();
                move |db| {
                    let rows: i64 = db
                        .conn_ref()
                        .query_row(
                            "SELECT COUNT(*)
                             FROM person_relationships
                             WHERE source = 'ai_enrichment'
                               AND context_entity_type = 'account'
                               AND context_entity_id = ?1",
                            params![account_id],
                            |row| row.get(0),
                        )
                        .map_err(|e| format!("count AI relationships before failed: {e}"))?;
                    let signals: i64 = db
                        .conn_ref()
                        .query_row(
                            "SELECT COUNT(*)
                             FROM signal_events
                             WHERE entity_type = 'account'
                               AND entity_id = ?1
                               AND signal_type = 'relationship_inferred'
                               AND data_source = 'ai_enrichment'",
                            params![account_id],
                            |row| row.get(0),
                        )
                        .map_err(|e| format!("count relationship signals before failed: {e}"))?;
                    Ok((rows, signals))
                }
            })
            .await
            .expect("read i504 pre-state failed");

        let ctx = state.live_service_context();
        let first_request_id = crate::audit_log::new_request_id();
        let _ = enrich_entity(
            &ctx,
            account_id.clone(),
            "account".to_string(),
            &state,
            None,
            &first_request_id,
        )
        .await
        .expect("manual enrich_entity for i504 validation failed");

        let (rows_after_first, ids_after_first, manager_bad, peer_bad, signals_after_first): (
            Vec<(String, f64, String, String, Option<String>)>,
            HashSet<String>,
            i64,
            i64,
            i64,
        ) = state
            .db_read({
                let account_id = account_id.clone();
                move |db| {
                    let mut stmt = db
                        .conn_ref()
                        .prepare(
                            "SELECT id, confidence, relationship_type, direction, context_entity_id
                             FROM person_relationships
                             WHERE source = 'ai_enrichment'
                               AND context_entity_type = 'account'
                               AND context_entity_id = ?1",
                        )
                        .map_err(|e| format!("prepare relationship read failed: {e}"))?;
                    let mapped = stmt
                        .query_map(params![account_id.clone()], |row| {
                            Ok((
                                row.get::<_, String>(0)?,
                                row.get::<_, f64>(1)?,
                                row.get::<_, String>(2)?,
                                row.get::<_, String>(3)?,
                                row.get::<_, Option<String>>(4)?,
                            ))
                        })
                        .map_err(|e| format!("query relationship read failed: {e}"))?;
                    let mut rows = Vec::new();
                    let mut ids = HashSet::new();
                    for row in mapped {
                        let row =
                            row.map_err(|e| format!("relationship row decode failed: {e}"))?;
                        ids.insert(row.0.clone());
                        rows.push(row);
                    }

                    let manager_bad: i64 = db
                        .conn_ref()
                        .query_row(
                            "SELECT COUNT(*)
                             FROM person_relationships
                             WHERE source = 'ai_enrichment'
                               AND context_entity_type = 'account'
                               AND context_entity_id = ?1
                               AND relationship_type = 'manager'
                               AND direction != 'directed'",
                            params![account_id.clone()],
                            |row| row.get(0),
                        )
                        .map_err(|e| format!("manager direction check failed: {e}"))?;
                    let peer_bad: i64 = db
                        .conn_ref()
                        .query_row(
                            "SELECT COUNT(*)
                             FROM person_relationships
                             WHERE source = 'ai_enrichment'
                               AND context_entity_type = 'account'
                               AND context_entity_id = ?1
                               AND relationship_type IN ('peer', 'collaborator')
                               AND direction != 'symmetric'",
                            params![account_id.clone()],
                            |row| row.get(0),
                        )
                        .map_err(|e| format!("peer/collaborator direction check failed: {e}"))?;
                    let signals: i64 = db
                        .conn_ref()
                        .query_row(
                            "SELECT COUNT(*)
                             FROM signal_events
                             WHERE entity_type = 'account'
                               AND entity_id = ?1
                               AND signal_type = 'relationship_inferred'
                               AND data_source = 'ai_enrichment'",
                            params![account_id],
                            |row| row.get(0),
                        )
                        .map_err(|e| {
                            format!("count relationship signals after first failed: {e}")
                        })?;

                    Ok((rows, ids, manager_bad, peer_bad, signals))
                }
            })
            .await
            .expect("read i504 post-first-run state failed");

        assert!(
            !rows_after_first.is_empty(),
            "I504 AC1: account enrichment with >=3 stakeholders should produce ai_enrichment relationship rows"
        );
        for (_, confidence, _, _, context_entity_id) in &rows_after_first {
            assert!(
                (*confidence - 0.6).abs() < 1e-9,
                "I504 AC2: inferred relationship confidence must be 0.6"
            );
            assert_eq!(
                context_entity_id.as_deref(),
                Some(account_id.as_str()),
                "I504 AC2: context_entity_id must be the enriched account"
            );
        }
        assert_eq!(
            manager_bad, 0,
            "I504 AC3: manager relationships must be directed"
        );
        assert_eq!(
            peer_bad, 0,
            "I504 AC3: peer/collaborator relationships must be symmetric"
        );

        let inserted_first_run = (rows_after_first.len() as i64 - before_rows).max(0);
        if inserted_first_run > 0 {
            assert!(
                signals_after_first >= before_signals + inserted_first_run,
                "I504 AC6: relationship_inferred signals should grow with new inserted edges"
            );
        } else {
            assert!(
                signals_after_first > 0,
                "I504 AC6: account should have relationship_inferred signal history for ai_enrichment edges"
            );
        }

        let second_request_id = crate::audit_log::new_request_id();
        let _ = enrich_entity(
            &ctx,
            account_id.clone(),
            "account".to_string(),
            &state,
            None,
            &second_request_id,
        )
        .await
        .expect("second enrich_entity for i504 validation failed");

        let (rows_after_second, ids_after_second, reinforced_after_second): (i64, i64, i64) = state
            .db_read({
                let account_id = account_id.clone();
                move |db| {
                    let rows: i64 = db
                        .conn_ref()
                        .query_row(
                            "SELECT COUNT(*)
                             FROM person_relationships
                             WHERE source = 'ai_enrichment'
                               AND context_entity_type = 'account'
                               AND context_entity_id = ?1",
                            params![account_id.clone()],
                            |row| row.get(0),
                        )
                        .map_err(|e| format!("count rows after second enrichment failed: {e}"))?;
                    let ids: i64 = db
                        .conn_ref()
                        .query_row(
                            "SELECT COUNT(DISTINCT id)
                             FROM person_relationships
                             WHERE source = 'ai_enrichment'
                               AND context_entity_type = 'account'
                               AND context_entity_id = ?1",
                            params![account_id],
                            |row| row.get(0),
                        )
                        .map_err(|e| {
                            format!("count distinct ids after second enrichment failed: {e}")
                        })?;
                    let reinforced: i64 = db
                        .conn_ref()
                        .query_row(
                            "SELECT COUNT(*)
                             FROM person_relationships
                             WHERE source = 'ai_enrichment'
                               AND context_entity_type = 'account'
                               AND context_entity_id = ?1
                               AND last_reinforced_at IS NOT NULL",
                            params![account_id],
                            |row| row.get(0),
                        )
                        .map_err(|e| {
                            format!("count reinforced edges after second enrichment failed: {e}")
                        })?;
                    Ok((rows, ids, reinforced))
                }
            })
            .await
            .expect("read i504 post-second-run state failed");

        assert_eq!(
            rows_after_second, ids_after_second,
            "I504 AC4: re-enrichment must not create duplicate AI relationship IDs"
        );
        assert!(
            rows_after_second >= rows_after_first.len() as i64
                && ids_after_second as usize >= ids_after_first.len(),
            "I504 AC4: second enrichment should preserve or reinforce existing inferred edges"
        );
        assert!(
            reinforced_after_second > 0,
            "I504 AC4: re-enrichment should reinforce existing edges (last_reinforced_at set)"
        );
    }
}
