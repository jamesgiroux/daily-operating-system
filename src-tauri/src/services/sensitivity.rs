//! ADR-0108 output-boundary sensitivity rendering.
//!
//! This module is the single helper for claim-derived text that leaves the
//! Rust boundary for UI or MCP consumers. ADR-0108 sections 1 and 2 require
//! per-surface actor filtering: Tauri app surfaces may show first-party
//! affordances, MCP responses are actor-filtered for agents, and publication,
//! log, and notification surfaces fail closed.

use crate::db::claims::{ClaimSensitivity, ClaimState, IntelligenceClaim, SurfacingState};
use crate::db::ActionDb;
use crate::intelligence::{
    CompanyContext, CurrentState, IntelRisk, IntelWin, IntelligenceJson, StakeholderInsight,
    ValueItem,
};
pub use abilities_runtime::sensitivity::{
    RedactionAffordance, RenderActor, RenderDecision, RenderPolicy, RenderPolicyKind,
    RenderSurface, RenderableClaimText,
};
use chrono::Utc;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct RenderableMcpClaimText {
    pub text: String,
    pub claim_id: String,
    pub sensitivity: ClaimSensitivity,
}

impl RenderableMcpClaimText {
    pub fn from_claim_value(claim: &IntelligenceClaim, text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            claim_id: claim.id.clone(),
            sensitivity: claim.sensitivity.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum McpStaticTextClass {
    AccountName,
    ProjectName,
    PersonName,
    MeetingTitle,
    MeetingType,
    EntityType,
    EntityHealth,
    EntityStatus,
    EntityLifecycle,
    DateTime,
    ContentFilename,
    ContentRelativePath,
    ContentType,
    ActionPriority,
    ActionTitle,
    BriefingNarrative,
    EmailSubject,
    EmailSnippet,
    MeetingSummary,
    MeetingPrepSummary,
    ContentChunk,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct RenderableMcpEntityName {
    pub name: String,
    pub entity_id: String,
}

impl RenderableMcpEntityName {
    pub fn new(name: impl Into<String>, entity_id: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            entity_id: entity_id.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderableMcpStaticText {
    pub text: String,
    pub surface_class: McpStaticTextClass,
}

impl RenderableMcpStaticText {
    pub fn new(text: impl Into<String>, surface_class: McpStaticTextClass) -> Self {
        Self {
            text: text.into(),
            surface_class,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderableMcpText {
    Claim(RenderableMcpClaimText),
    Static(RenderableMcpStaticText),
}

pub fn render_policy_for_surface(
    claim: &IntelligenceClaim,
    surface: RenderSurface,
    actor: &RenderActor,
) -> RenderDecision {
    match claim.sensitivity {
        ClaimSensitivity::Public => public_policy(surface),
        ClaimSensitivity::Internal => internal_policy(surface),
        ClaimSensitivity::Confidential => confidential_policy(claim, surface),
        ClaimSensitivity::UserOnly => user_only_policy(claim, surface, actor),
    }
}

pub fn render_policy_for_surface_name(
    claim: &IntelligenceClaim,
    surface: &str,
    actor: &RenderActor,
) -> RenderDecision {
    let Some(surface) = RenderSurface::from_name(surface) else {
        return RenderDecision::Drop;
    };
    render_policy_for_surface(claim, surface, actor)
}

pub fn render_policy_for_sensitivity_name(
    sensitivity: &str,
    surface: &str,
    originating_actor: &str,
    actor: &RenderActor,
) -> RenderDecision {
    let Some(surface) = RenderSurface::from_name(surface) else {
        return RenderDecision::Drop;
    };
    let sensitivity = match sensitivity.trim().to_ascii_lowercase().as_str() {
        "public" => ClaimSensitivity::Public,
        "internal" => ClaimSensitivity::Internal,
        "confidential" => ClaimSensitivity::Confidential,
        "user_only" => ClaimSensitivity::UserOnly,
        _ => return RenderDecision::Drop,
    };

    let claim = minimal_policy_claim(sensitivity, originating_actor);
    render_policy_for_surface(&claim, surface, actor)
}

pub fn renderable_claim_text(
    claim: &IntelligenceClaim,
    surface: RenderSurface,
    actor: &RenderActor,
) -> Option<RenderableClaimText> {
    renderable_claim_text_with_value(claim, &claim.text, surface, actor)
}

pub fn renderable_claim_text_with_value(
    claim: &IntelligenceClaim,
    value: &str,
    surface: RenderSurface,
    actor: &RenderActor,
) -> Option<RenderableClaimText> {
    let decision = render_policy_for_surface(claim, surface, actor);
    renderable_from_decision(claim, value, surface, decision)
}

pub fn renderable_from_decision(
    claim: &IntelligenceClaim,
    value: &str,
    surface: RenderSurface,
    decision: RenderDecision,
) -> Option<RenderableClaimText> {
    match decision {
        RenderDecision::Render => Some(RenderableClaimText {
            text: value.to_string(),
            policy: RenderPolicy {
                kind: RenderPolicyKind::Render,
                sensitivity: claim.sensitivity.clone(),
                surface,
                claim_id: Some(claim.id.clone()),
                affordance: None,
            },
        }),
        RenderDecision::RenderRedacted { affordance } => Some(RenderableClaimText {
            text: affordance.label().to_string(),
            policy: RenderPolicy {
                kind: RenderPolicyKind::Redacted,
                sensitivity: claim.sensitivity.clone(),
                surface,
                claim_id: Some(claim.id.clone()),
                affordance: Some(affordance),
            },
        }),
        RenderDecision::Drop => None,
    }
}

/// Render a text leaf from a static MCP DTO.
///
/// This is intentionally fail-closed. Claim-derived text must arrive with a
/// durable `claim_id`; the claim is reloaded and its stored sensitivity is used
/// for the MCP policy decision. Non-claim metadata must arrive with an explicit
/// `McpStaticTextClass`, and only classes in the allowlist render. Paraphrased,
/// truncated, summarized, or otherwise generated text without claim metadata is
/// not upgraded to synthetic Internal text. It drops.
pub fn render_mcp_static_text_for_surface(
    db: &ActionDb,
    value: RenderableMcpText,
) -> Option<String> {
    match value {
        RenderableMcpText::Claim(text) => render_mcp_claim_text_for_surface(db, text),
        RenderableMcpText::Static(text) => render_mcp_non_claim_static_text_for_surface(text),
    }
}

pub fn render_mcp_static_json_for_surface<F>(
    db: &ActionDb,
    value: serde_json::Value,
    classify_static_text: &F,
) -> Option<serde_json::Value>
where
    F: Fn(&[String], &str) -> Option<McpStaticTextClass>,
{
    let mut path = Vec::new();
    render_mcp_static_json_at_path(db, value, &mut path, classify_static_text)
}

fn render_mcp_claim_text_for_surface(
    db: &ActionDb,
    value: RenderableMcpClaimText,
) -> Option<String> {
    verify_and_render_authoritative_claim(
        TaggedMcpClaimText {
            text: value.text,
            claim_id: value.claim_id,
            sensitivity: value.sensitivity,
            stored_projection: StoredMcpClaimTextProjection::Text,
        },
        RenderSurface::McpTool,
        &|claim_id| {
            crate::services::claims::load_claim_by_id(db.conn_ref(), claim_id)
                .ok()
                .flatten()
        },
    )
    .map(|rendered| rendered.text)
}

fn render_mcp_non_claim_static_text_for_surface(value: RenderableMcpStaticText) -> Option<String> {
    if is_mcp_non_claim_static_text_allowlisted(value.surface_class) {
        Some(value.text)
    } else {
        None
    }
}

fn is_mcp_non_claim_static_text_allowlisted(surface_class: McpStaticTextClass) -> bool {
    matches!(
        surface_class,
        McpStaticTextClass::AccountName
            | McpStaticTextClass::ProjectName
            | McpStaticTextClass::PersonName
            | McpStaticTextClass::MeetingTitle
            | McpStaticTextClass::MeetingType
            | McpStaticTextClass::EntityType
            | McpStaticTextClass::EntityHealth
            | McpStaticTextClass::EntityStatus
            | McpStaticTextClass::EntityLifecycle
            | McpStaticTextClass::DateTime
            | McpStaticTextClass::ContentFilename
            | McpStaticTextClass::ContentRelativePath
            | McpStaticTextClass::ContentType
            | McpStaticTextClass::ActionPriority
    )
}

fn render_mcp_static_json_at_path<F>(
    db: &ActionDb,
    value: serde_json::Value,
    path: &mut Vec<String>,
    classify_static_text: &F,
) -> Option<serde_json::Value>
where
    F: Fn(&[String], &str) -> Option<McpStaticTextClass>,
{
    match value {
        serde_json::Value::String(text) => {
            let surface_class = classify_static_text(path, &text)?;
            render_mcp_static_text_for_surface(
                db,
                RenderableMcpText::Static(RenderableMcpStaticText::new(text, surface_class)),
            )
            .map(serde_json::Value::String)
        }
        serde_json::Value::Array(items) => Some(serde_json::Value::Array(
            items
                .into_iter()
                .filter_map(|item| {
                    render_mcp_static_json_at_path(db, item, path, classify_static_text)
                })
                .collect(),
        )),
        serde_json::Value::Object(object) => {
            let mut rendered = serde_json::Map::new();
            for (key, value) in object {
                path.push(key.clone());
                if let Some(value) =
                    render_mcp_static_json_at_path(db, value, path, classify_static_text)
                {
                    rendered.insert(key, value);
                }
                path.pop();
            }
            Some(serde_json::Value::Object(rendered))
        }
        other => Some(other),
    }
}

/// Render ability `data` for the MCP tool surface.
///
/// Track GG inverts the MCP output boundary to deny by default. Every
/// string leaf has exactly three possible outcomes:
///
/// 1. Claim text with authoritative metadata renders as the same minimal
///    `{ text, policy }` object used by tagged carriers.
/// 2. Explicit non-content metadata from the narrow key/path allowlist renders
///    as a string.
/// 3. Everything else drops by omission.
///
/// Tagged carrier objects remain authoritative and fail closed. The DTO's
/// `sensitivity` is only a consistency check: the persisted claim is reloaded
/// by `claim_id` through `load_claim_by_id`, and the stored sensitivity/actor
/// drive `render_policy_for_surface(..., McpTool, ...)`. Missing claims,
/// malformed tags, or DTO/stored sensitivity mismatches are dropped.
///
/// Untagged ability DTO strings can only render when the bridge supplies
/// provenance whose field attribution resolves to persisted claims. This keeps
/// Tauri DTOs unchanged while MCP still serializes claim-derived text through
/// the tagged-carrier renderer. Unrecognized tagged-object siblings fail
/// closed by omission; future MCP metadata fields must be deliberately added to
/// the allowlist rather than inherited from ability DTOs.
pub fn render_mcp_ability_data_for_surface(
    db: &ActionDb,
    value: serde_json::Value,
) -> serde_json::Value {
    render_mcp_ability_data_with_claim_lookup(value, None, &|claim_id| {
        crate::services::claims::load_claim_by_id(db.conn_ref(), claim_id)
            .ok()
            .flatten()
    })
}

pub fn render_mcp_ability_data_for_surface_with_provenance(
    db: &ActionDb,
    value: serde_json::Value,
    provenance: &serde_json::Value,
) -> serde_json::Value {
    render_mcp_ability_data_with_claim_lookup(value, Some(provenance), &|claim_id| {
        crate::services::claims::load_claim_by_id(db.conn_ref(), claim_id)
            .ok()
            .flatten()
    })
}

/// Render with no authoritative claim source. Non-claim metadata still walks
/// through the fail-closed ability-data sanitizer, but every tagged claim
/// carrier drops because persisted metadata cannot be verified.
pub(crate) fn render_mcp_ability_data_without_claim_lookup(
    value: serde_json::Value,
) -> serde_json::Value {
    render_mcp_ability_data_with_claim_lookup(value, None, &|_| None)
}

fn render_mcp_ability_data_with_claim_lookup(
    value: serde_json::Value,
    provenance: Option<&serde_json::Value>,
    load_claim: &impl Fn(&str) -> Option<IntelligenceClaim>,
) -> serde_json::Value {
    let mut path = Vec::new();
    render_mcp_ability_data_value(value, &mut path, None, provenance, load_claim)
        .unwrap_or(serde_json::Value::Null)
}

#[derive(Debug, Clone)]
struct TaggedMcpClaimText {
    text: String,
    claim_id: String,
    sensitivity: ClaimSensitivity,
    stored_projection: StoredMcpClaimTextProjection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StoredMcpClaimTextProjection {
    Text,
    EntityContextTitle,
}

enum TaggedMcpClaimTextMatch {
    NotTagged,
    Malformed,
    Tagged(TaggedMcpClaimText),
}

fn render_mcp_ability_data_value(
    value: serde_json::Value,
    path: &mut Vec<String>,
    claim_id_hint: Option<&str>,
    provenance: Option<&serde_json::Value>,
    load_claim: &impl Fn(&str) -> Option<IntelligenceClaim>,
) -> Option<serde_json::Value> {
    match value {
        serde_json::Value::Object(object) => match tagged_mcp_claim_text(&object) {
            TaggedMcpClaimTextMatch::Tagged(tagged) => {
                render_tagged_mcp_claim_text(tagged, load_claim)
            }
            TaggedMcpClaimTextMatch::Malformed => None,
            TaggedMcpClaimTextMatch::NotTagged => {
                if let Some(entity_name) = tagged_mcp_entity_name(&object) {
                    return Some(entity_name);
                }

                let object_claim_id_hint = claim_id_hint_from_object(&object)
                    .or_else(|| claim_id_hint.map(str::to_string));
                let mut rendered = serde_json::Map::new();
                for (key, value) in object {
                    path.push(key.clone());
                    if let Some(value) = render_mcp_ability_data_value(
                        value,
                        path,
                        object_claim_id_hint.as_deref(),
                        provenance,
                        load_claim,
                    ) {
                        rendered.insert(key, value);
                    }
                    path.pop();
                }
                Some(serde_json::Value::Object(rendered))
            }
        },
        serde_json::Value::Array(values) => Some(serde_json::Value::Array(
            values
                .into_iter()
                .enumerate()
                .filter_map(|(index, value)| {
                    path.push(index.to_string());
                    let rendered = render_mcp_ability_data_value(
                        value,
                        path,
                        claim_id_hint,
                        provenance,
                        load_claim,
                    );
                    path.pop();
                    rendered
                })
                .collect(),
        )),
        serde_json::Value::String(text) => {
            if let Some(metadata) = render_mcp_ability_metadata_string(path, &text) {
                return Some(metadata);
            }

            attested_mcp_claim_text_for_leaf(path, &text, claim_id_hint, provenance, load_claim)
                .and_then(|tagged| render_tagged_mcp_claim_text(tagged, load_claim))
        }
        other => Some(other),
    }
}

fn tagged_mcp_entity_name(
    object: &serde_json::Map<String, serde_json::Value>,
) -> Option<serde_json::Value> {
    if !(object.contains_key("name")
        && (object.contains_key("entity_id") || object.contains_key("entityId")))
    {
        return None;
    }

    let name = object.get("name")?.as_str()?.trim();
    if name.is_empty() {
        return None;
    }
    let entity_id = string_field(object, &["entity_id", "entityId"])?;
    if entity_id.trim().is_empty() {
        return None;
    }

    let mut rendered = serde_json::Map::new();
    rendered.insert(
        "name".to_string(),
        serde_json::Value::String(name.to_string()),
    );
    rendered.insert(
        "entity_id".to_string(),
        serde_json::Value::String(entity_id),
    );
    Some(serde_json::Value::Object(rendered))
}

fn tagged_mcp_claim_text(
    object: &serde_json::Map<String, serde_json::Value>,
) -> TaggedMcpClaimTextMatch {
    if !object.contains_key("text") {
        return TaggedMcpClaimTextMatch::NotTagged;
    }

    let has_claim_tag = claim_id_from_mcp_claim_text_object(object).is_some()
        || object.get("policy").is_some()
        || object.get("renderPolicy").is_some()
        || object.get("sensitivity").is_some();
    if !has_claim_tag {
        return TaggedMcpClaimTextMatch::NotTagged;
    }

    let Some(text) = object
        .get("text")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
    else {
        return TaggedMcpClaimTextMatch::Malformed;
    };
    let Some(claim_id) = claim_id_from_mcp_claim_text_object(object) else {
        return TaggedMcpClaimTextMatch::Malformed;
    };
    let Some(sensitivity) = sensitivity_from_mcp_claim_text_object(object) else {
        return TaggedMcpClaimTextMatch::Malformed;
    };
    TaggedMcpClaimTextMatch::Tagged(TaggedMcpClaimText {
        text,
        claim_id,
        sensitivity,
        stored_projection: StoredMcpClaimTextProjection::Text,
    })
}

fn render_tagged_mcp_claim_text(
    tagged: TaggedMcpClaimText,
    load_claim: &impl Fn(&str) -> Option<IntelligenceClaim>,
) -> Option<serde_json::Value> {
    let rendered =
        verify_and_render_authoritative_claim(tagged, RenderSurface::McpTool, load_claim)?;
    safe_tagged_mcp_claim_text_object(rendered)
}

fn verify_and_render_authoritative_claim(
    tagged: TaggedMcpClaimText,
    surface: RenderSurface,
    load_claim: &impl Fn(&str) -> Option<IntelligenceClaim>,
) -> Option<RenderableClaimText> {
    let actor = RenderActor::agent("agent:mcp");
    let claim = load_claim(&tagged.claim_id)?;
    let stored_text = stored_mcp_claim_text(&claim, &tagged.claim_id, tagged.stored_projection)?;

    if claim.sensitivity != tagged.sensitivity {
        log::warn!(
            target: "dailyos_lib::services::sensitivity",
            "MCP claim text sensitivity mismatch claim_id={} stored={:?} emitted={:?}; dropping",
            tagged.claim_id,
            claim.sensitivity,
            tagged.sensitivity
        );
        return None;
    }

    if stored_text != tagged.text {
        log::warn!(
            target: "dailyos_lib::services::sensitivity",
            "MCP claim text mismatch claim_id={}; dropping",
            tagged.claim_id
        );
        return None;
    }

    let decision = render_policy_for_surface(&claim, surface, &actor);
    renderable_from_decision(&claim, &stored_text, surface, decision)
}

fn stored_mcp_claim_text(
    claim: &IntelligenceClaim,
    claim_id: &str,
    projection: StoredMcpClaimTextProjection,
) -> Option<String> {
    if !claim_has_active_surfaced_lifecycle(claim) {
        log::warn!(
            target: "dailyos_lib::services::sensitivity",
            "MCP claim text is not active/surfaced claim_id={}; dropping",
            claim_id
        );
        return None;
    }

    let text = match projection {
        StoredMcpClaimTextProjection::Text => claim.text.clone(),
        StoredMcpClaimTextProjection::EntityContextTitle => entity_context_title_for_claim(claim),
    };

    if text.trim().is_empty() {
        log::warn!(
            target: "dailyos_lib::services::sensitivity",
            "MCP claim text has no stored text claim_id={}; dropping",
            claim_id
        );
        return None;
    }

    Some(text)
}

fn claim_has_active_surfaced_lifecycle(claim: &IntelligenceClaim) -> bool {
    !(claim.claim_state != ClaimState::Active || claim.surfacing_state != SurfacingState::Active)
}

fn entity_context_title_for_claim(claim: &IntelligenceClaim) -> String {
    match claim
        .field_path
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        Some(field_path) => format!("{}: {field_path}", claim.claim_type),
        None => claim.claim_type.clone(),
    }
}

fn safe_tagged_mcp_claim_text_object(rendered: RenderableClaimText) -> Option<serde_json::Value> {
    let mut object = serde_json::Map::new();
    object.insert("text".to_string(), serde_json::Value::String(rendered.text));
    object.insert(
        "policy".to_string(),
        serde_json::to_value(rendered.policy).ok()?,
    );
    Some(serde_json::Value::Object(object))
}

fn claim_id_from_mcp_claim_text_object(
    object: &serde_json::Map<String, serde_json::Value>,
) -> Option<String> {
    string_field(object, &["claim_id", "claimId"])
        .or_else(|| policy_string_field(object, &["claim_id", "claimId"]))
}

fn sensitivity_from_mcp_claim_text_object(
    object: &serde_json::Map<String, serde_json::Value>,
) -> Option<ClaimSensitivity> {
    object
        .get("sensitivity")
        .and_then(claim_sensitivity_from_value)
        .or_else(|| {
            object
                .get("policy")
                .and_then(|policy| policy.get("sensitivity"))
                .and_then(claim_sensitivity_from_value)
        })
        .or_else(|| {
            object
                .get("renderPolicy")
                .and_then(|policy| policy.get("sensitivity"))
                .and_then(claim_sensitivity_from_value)
        })
}

fn string_field(
    object: &serde_json::Map<String, serde_json::Value>,
    keys: &[&str],
) -> Option<String> {
    keys.iter()
        .find_map(|key| object.get(*key).and_then(serde_json::Value::as_str))
        .map(str::to_string)
}

fn policy_string_field(
    object: &serde_json::Map<String, serde_json::Value>,
    keys: &[&str],
) -> Option<String> {
    object
        .get("policy")
        .or_else(|| object.get("renderPolicy"))
        .and_then(serde_json::Value::as_object)
        .and_then(|policy| string_field(policy, keys))
}

fn claim_sensitivity_from_value(value: &serde_json::Value) -> Option<ClaimSensitivity> {
    let text = value.as_str()?;
    match text.trim().to_ascii_lowercase().replace('-', "_").as_str() {
        "public" => Some(ClaimSensitivity::Public),
        "internal" => Some(ClaimSensitivity::Internal),
        "confidential" => Some(ClaimSensitivity::Confidential),
        "user_only" | "useronly" => Some(ClaimSensitivity::UserOnly),
        _ => None,
    }
}

fn claim_id_hint_from_object(
    object: &serde_json::Map<String, serde_json::Value>,
) -> Option<String> {
    let carries_claim_text = [
        "content",
        "context",
        "description",
        "detail",
        "outcome",
        "rationale",
        "summary",
        "text",
        "title",
    ]
    .iter()
    .any(|key| object.get(*key).is_some_and(serde_json::Value::is_string));
    if !carries_claim_text {
        return None;
    }

    string_field(object, &["claim_id", "claimId", "id"])
}

fn attested_mcp_claim_text_for_leaf(
    path: &[String],
    text: &str,
    claim_id_hint: Option<&str>,
    provenance: Option<&serde_json::Value>,
    load_claim: &impl Fn(&str) -> Option<IntelligenceClaim>,
) -> Option<TaggedMcpClaimText> {
    provenance_claim_for_path(path, provenance, load_claim)
        .or_else(|| claim_id_hint.and_then(load_claim))
        .map(|claim| TaggedMcpClaimText {
            text: text.to_string(),
            claim_id: claim.id,
            sensitivity: claim.sensitivity,
            stored_projection: stored_mcp_claim_text_projection_for_path(path),
        })
}

fn stored_mcp_claim_text_projection_for_path(path: &[String]) -> StoredMcpClaimTextProjection {
    if path.len() == 2 && is_array_index(&path[0]) && path[1] == "title" {
        StoredMcpClaimTextProjection::EntityContextTitle
    } else {
        StoredMcpClaimTextProjection::Text
    }
}

fn provenance_claim_for_path(
    path: &[String],
    provenance: Option<&serde_json::Value>,
    load_claim: &impl Fn(&str) -> Option<IntelligenceClaim>,
) -> Option<IntelligenceClaim> {
    let provenance = provenance?;
    let field_attributions = provenance.get("field_attributions")?.as_object()?;
    let pointer = json_pointer_from_path(path);
    let (_field_path, attribution) = field_attributions
        .iter()
        .filter(|(field_path, attribution)| {
            field_path_covers(field_path, &pointer)
                && attribution
                    .get("source_refs")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|refs| !refs.is_empty())
        })
        .max_by_key(|(field_path, _)| field_path.len())?;

    let claims = claims_from_field_attribution(provenance, attribution, load_claim);
    most_cautious_claim(claims)
}

fn claims_from_field_attribution(
    provenance: &serde_json::Value,
    attribution: &serde_json::Value,
    load_claim: &impl Fn(&str) -> Option<IntelligenceClaim>,
) -> Vec<IntelligenceClaim> {
    let mut claims = Vec::new();
    let Some(source_refs) = attribution
        .get("source_refs")
        .and_then(serde_json::Value::as_array)
    else {
        return claims;
    };

    for source_ref in source_refs {
        claims.extend(claims_from_source_ref(provenance, source_ref, load_claim));
    }
    claims
}

fn claims_from_source_ref(
    provenance: &serde_json::Value,
    source_ref: &serde_json::Value,
    load_claim: &impl Fn(&str) -> Option<IntelligenceClaim>,
) -> Vec<IntelligenceClaim> {
    if let Some(source_index) = source_index_from_ref(source_ref) {
        return provenance
            .get("sources")
            .and_then(serde_json::Value::as_array)
            .and_then(|sources| sources.get(source_index))
            .map(|source| claims_from_source(source, load_claim))
            .unwrap_or_default();
    }

    let Some(child_ref) = source_ref.get("child") else {
        return Vec::new();
    };
    let Some(child_field_path) = child_ref
        .get("field_path")
        .and_then(serde_json::Value::as_str)
    else {
        return Vec::new();
    };

    provenance
        .get("children")
        .and_then(serde_json::Value::as_array)
        .map(|children| {
            children
                .iter()
                .flat_map(|child| claims_from_child_provenance(child, child_field_path, load_claim))
                .collect()
        })
        .unwrap_or_default()
}

fn claims_from_child_provenance(
    child: &serde_json::Value,
    child_field_path: &str,
    load_claim: &impl Fn(&str) -> Option<IntelligenceClaim>,
) -> Vec<IntelligenceClaim> {
    let mut claims = Vec::new();
    let Some(field_attributions) = child
        .get("field_attributions")
        .and_then(serde_json::Value::as_object)
    else {
        return claims;
    };

    for (candidate_path, attribution) in field_attributions {
        if field_path_covers(child_field_path, candidate_path) {
            claims.extend(claims_from_field_attribution(
                child,
                attribution,
                load_claim,
            ));
        }
    }
    claims
}

fn source_index_from_ref(source_ref: &serde_json::Value) -> Option<usize> {
    source_ref
        .get("source")
        .and_then(|source| source.get("source_index"))
        .or_else(|| source_ref.get("source_index"))
        .and_then(source_index_value)
}

fn source_index_value(value: &serde_json::Value) -> Option<usize> {
    if let Some(index) = value.as_u64() {
        return usize::try_from(index).ok();
    }
    value
        .get("source_index")
        .and_then(serde_json::Value::as_u64)
        .and_then(|index| usize::try_from(index).ok())
}

fn claims_from_source(
    source: &serde_json::Value,
    load_claim: &impl Fn(&str) -> Option<IntelligenceClaim>,
) -> Vec<IntelligenceClaim> {
    let mut claims = Vec::new();
    let Some(identifiers) = source
        .get("identifiers")
        .and_then(serde_json::Value::as_array)
    else {
        return claims;
    };

    for identifier in identifiers {
        for claim_id in claim_id_candidates_from_identifier(identifier) {
            if claims.iter().any(|claim| claim.id == claim_id) {
                continue;
            }
            if let Some(claim) = load_claim(&claim_id) {
                claims.push(claim);
            }
        }
    }
    claims
}

fn claim_id_candidates_from_identifier(identifier: &serde_json::Value) -> Vec<String> {
    let mut candidates = Vec::new();
    collect_claim_id_candidate_fields(identifier, &mut candidates);
    candidates
}

fn collect_claim_id_candidate_fields(value: &serde_json::Value, candidates: &mut Vec<String>) {
    match value {
        serde_json::Value::Object(object) => {
            for (key, value) in object {
                if matches!(
                    key.as_str(),
                    "signal_id"
                        | "signalId"
                        | "entry_id"
                        | "entryId"
                        | "document_id"
                        | "documentId"
                        | "meeting_id"
                        | "meetingId"
                        | "entity_id"
                        | "entityId"
                ) {
                    if let Some(candidate) = value.as_str().filter(|value| !value.trim().is_empty())
                    {
                        candidates.push(candidate.to_string());
                    }
                }
                collect_claim_id_candidate_fields(value, candidates);
            }
        }
        serde_json::Value::Array(values) => {
            for value in values {
                collect_claim_id_candidate_fields(value, candidates);
            }
        }
        _ => {}
    }
}

fn most_cautious_claim(claims: Vec<IntelligenceClaim>) -> Option<IntelligenceClaim> {
    claims
        .into_iter()
        .max_by_key(|claim| claim_sensitivity_rank(&claim.sensitivity))
}

fn claim_sensitivity_rank(sensitivity: &ClaimSensitivity) -> u8 {
    match sensitivity {
        ClaimSensitivity::Public => 0,
        ClaimSensitivity::Internal => 1,
        ClaimSensitivity::Confidential => 2,
        ClaimSensitivity::UserOnly => 3,
    }
}

fn json_pointer_from_path(path: &[String]) -> String {
    path.iter().fold(String::new(), |mut pointer, token| {
        pointer.push('/');
        pointer.push_str(&token.replace('~', "~0").replace('/', "~1"));
        pointer
    })
}

fn field_path_covers(candidate: &str, leaf: &str) -> bool {
    candidate.is_empty()
        || candidate == leaf
        || leaf
            .strip_prefix(candidate)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

fn is_array_index(value: &str) -> bool {
    !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum McpAbilityMetadataValueClass {
    Identifier,
    EntityKind,
    TemporalScope,
    Timestamp,
    MeetingTitle,
    EntityName,
}

struct McpAbilityMetadataPathRule {
    pattern: &'static [&'static str],
    value_class: McpAbilityMetadataValueClass,
}

const MCP_ABILITY_METADATA_STRING_ALLOWLIST: &[McpAbilityMetadataPathRule] = &[
    // get_entity_context legacy data was a top-level EntityContextEntry array.
    McpAbilityMetadataPathRule {
        pattern: &["*", "id"],
        value_class: McpAbilityMetadataValueClass::Identifier,
    },
    McpAbilityMetadataPathRule {
        pattern: &["*", "entityType"],
        value_class: McpAbilityMetadataValueClass::EntityKind,
    },
    McpAbilityMetadataPathRule {
        pattern: &["*", "entityId"],
        value_class: McpAbilityMetadataValueClass::Identifier,
    },
    McpAbilityMetadataPathRule {
        pattern: &["*", "createdAt"],
        value_class: McpAbilityMetadataValueClass::Timestamp,
    },
    McpAbilityMetadataPathRule {
        pattern: &["*", "updatedAt"],
        value_class: McpAbilityMetadataValueClass::Timestamp,
    },
    // get_entity_context data now carries entries plus optional trajectories.
    McpAbilityMetadataPathRule {
        pattern: &["entries", "*", "id"],
        value_class: McpAbilityMetadataValueClass::Identifier,
    },
    McpAbilityMetadataPathRule {
        pattern: &["entries", "*", "entityType"],
        value_class: McpAbilityMetadataValueClass::EntityKind,
    },
    McpAbilityMetadataPathRule {
        pattern: &["entries", "*", "entityId"],
        value_class: McpAbilityMetadataValueClass::Identifier,
    },
    McpAbilityMetadataPathRule {
        pattern: &["entries", "*", "createdAt"],
        value_class: McpAbilityMetadataValueClass::Timestamp,
    },
    McpAbilityMetadataPathRule {
        pattern: &["entries", "*", "updatedAt"],
        value_class: McpAbilityMetadataValueClass::Timestamp,
    },
    McpAbilityMetadataPathRule {
        pattern: &["trajectory", "engagement_curve", "kind"],
        value_class: McpAbilityMetadataValueClass::Identifier,
    },
    McpAbilityMetadataPathRule {
        pattern: &["trajectory", "engagement_curve", "entity_id"],
        value_class: McpAbilityMetadataValueClass::Identifier,
    },
    McpAbilityMetadataPathRule {
        pattern: &["trajectory", "engagement_curve", "computed_at"],
        value_class: McpAbilityMetadataValueClass::Timestamp,
    },
    McpAbilityMetadataPathRule {
        pattern: &["trajectory", "engagement_curve", "series", "*", "at"],
        value_class: McpAbilityMetadataValueClass::Timestamp,
    },
    McpAbilityMetadataPathRule {
        pattern: &["trajectory", "role_progression", "kind"],
        value_class: McpAbilityMetadataValueClass::Identifier,
    },
    McpAbilityMetadataPathRule {
        pattern: &["trajectory", "role_progression", "entity_id"],
        value_class: McpAbilityMetadataValueClass::Identifier,
    },
    McpAbilityMetadataPathRule {
        pattern: &["trajectory", "role_progression", "computed_at"],
        value_class: McpAbilityMetadataValueClass::Timestamp,
    },
    McpAbilityMetadataPathRule {
        pattern: &["trajectory", "role_progression", "series", "*", "at"],
        value_class: McpAbilityMetadataValueClass::Timestamp,
    },
    McpAbilityMetadataPathRule {
        pattern: &[
            "trajectory",
            "role_progression",
            "series",
            "*",
            "value",
            "started_at",
        ],
        value_class: McpAbilityMetadataValueClass::Timestamp,
    },
    McpAbilityMetadataPathRule {
        pattern: &[
            "trajectory",
            "role_progression",
            "series",
            "*",
            "value",
            "ended_at",
        ],
        value_class: McpAbilityMetadataValueClass::Timestamp,
    },
    McpAbilityMetadataPathRule {
        pattern: &[
            "trajectory",
            "role_progression",
            "series",
            "*",
            "value",
            "title",
        ],
        value_class: McpAbilityMetadataValueClass::EntityName,
    },
    McpAbilityMetadataPathRule {
        pattern: &[
            "trajectory",
            "role_progression",
            "series",
            "*",
            "value",
            "org",
        ],
        value_class: McpAbilityMetadataValueClass::EntityName,
    },
    McpAbilityMetadataPathRule {
        pattern: &[
            "trajectory",
            "role_progression",
            "series",
            "*",
            "value",
            "seniority",
        ],
        value_class: McpAbilityMetadataValueClass::EntityName,
    },
    // prepare_meeting meeting metadata.
    McpAbilityMetadataPathRule {
        pattern: &["meeting", "id"],
        value_class: McpAbilityMetadataValueClass::Identifier,
    },
    McpAbilityMetadataPathRule {
        pattern: &["meeting", "title"],
        value_class: McpAbilityMetadataValueClass::MeetingTitle,
    },
    McpAbilityMetadataPathRule {
        pattern: &["meeting", "starts_at"],
        value_class: McpAbilityMetadataValueClass::Timestamp,
    },
    McpAbilityMetadataPathRule {
        pattern: &["meeting", "ends_at"],
        value_class: McpAbilityMetadataValueClass::Timestamp,
    },
    McpAbilityMetadataPathRule {
        pattern: &["meeting", "attendees", "*", "name"],
        value_class: McpAbilityMetadataValueClass::EntityName,
    },
    McpAbilityMetadataPathRule {
        pattern: &["meeting", "attendees", "*", "person_id"],
        value_class: McpAbilityMetadataValueClass::Identifier,
    },
    McpAbilityMetadataPathRule {
        pattern: &["meeting", "attendees", "*", "account_id"],
        value_class: McpAbilityMetadataValueClass::Identifier,
    },
    // prepare_meeting subject routing and temporal-scope metadata.
    McpAbilityMetadataPathRule {
        pattern: &["topics", "*", "subject", "kind"],
        value_class: McpAbilityMetadataValueClass::EntityKind,
    },
    McpAbilityMetadataPathRule {
        pattern: &["topics", "*", "subject", "id"],
        value_class: McpAbilityMetadataValueClass::Identifier,
    },
    McpAbilityMetadataPathRule {
        pattern: &["topics", "*", "temporal_scope"],
        value_class: McpAbilityMetadataValueClass::TemporalScope,
    },
    McpAbilityMetadataPathRule {
        pattern: &[
            "topics",
            "*",
            "temporal_scope",
            "point_in_time",
            "occurred_at",
        ],
        value_class: McpAbilityMetadataValueClass::Timestamp,
    },
    McpAbilityMetadataPathRule {
        pattern: &["topics", "*", "temporal_scope", "trend", "window_start"],
        value_class: McpAbilityMetadataValueClass::Timestamp,
    },
    McpAbilityMetadataPathRule {
        pattern: &["topics", "*", "temporal_scope", "trend", "window_end"],
        value_class: McpAbilityMetadataValueClass::Timestamp,
    },
    McpAbilityMetadataPathRule {
        pattern: &["attendee_context", "*", "subject", "kind"],
        value_class: McpAbilityMetadataValueClass::EntityKind,
    },
    McpAbilityMetadataPathRule {
        pattern: &["attendee_context", "*", "subject", "id"],
        value_class: McpAbilityMetadataValueClass::Identifier,
    },
    McpAbilityMetadataPathRule {
        pattern: &["attendee_context", "*", "temporal_scope"],
        value_class: McpAbilityMetadataValueClass::TemporalScope,
    },
    McpAbilityMetadataPathRule {
        pattern: &[
            "attendee_context",
            "*",
            "temporal_scope",
            "point_in_time",
            "occurred_at",
        ],
        value_class: McpAbilityMetadataValueClass::Timestamp,
    },
    McpAbilityMetadataPathRule {
        pattern: &[
            "attendee_context",
            "*",
            "temporal_scope",
            "trend",
            "window_start",
        ],
        value_class: McpAbilityMetadataValueClass::Timestamp,
    },
    McpAbilityMetadataPathRule {
        pattern: &[
            "attendee_context",
            "*",
            "temporal_scope",
            "trend",
            "window_end",
        ],
        value_class: McpAbilityMetadataValueClass::Timestamp,
    },
    McpAbilityMetadataPathRule {
        pattern: &["open_loops", "*", "subject", "kind"],
        value_class: McpAbilityMetadataValueClass::EntityKind,
    },
    McpAbilityMetadataPathRule {
        pattern: &["open_loops", "*", "subject", "id"],
        value_class: McpAbilityMetadataValueClass::Identifier,
    },
    McpAbilityMetadataPathRule {
        pattern: &["open_loops", "*", "temporal_scope"],
        value_class: McpAbilityMetadataValueClass::TemporalScope,
    },
    McpAbilityMetadataPathRule {
        pattern: &[
            "open_loops",
            "*",
            "temporal_scope",
            "point_in_time",
            "occurred_at",
        ],
        value_class: McpAbilityMetadataValueClass::Timestamp,
    },
    McpAbilityMetadataPathRule {
        pattern: &["open_loops", "*", "temporal_scope", "trend", "window_start"],
        value_class: McpAbilityMetadataValueClass::Timestamp,
    },
    McpAbilityMetadataPathRule {
        pattern: &["open_loops", "*", "temporal_scope", "trend", "window_end"],
        value_class: McpAbilityMetadataValueClass::Timestamp,
    },
    McpAbilityMetadataPathRule {
        pattern: &["what_changed_since_last", "*", "subject", "kind"],
        value_class: McpAbilityMetadataValueClass::EntityKind,
    },
    McpAbilityMetadataPathRule {
        pattern: &["what_changed_since_last", "*", "subject", "id"],
        value_class: McpAbilityMetadataValueClass::Identifier,
    },
    McpAbilityMetadataPathRule {
        pattern: &["what_changed_since_last", "*", "temporal_scope"],
        value_class: McpAbilityMetadataValueClass::TemporalScope,
    },
    McpAbilityMetadataPathRule {
        pattern: &[
            "what_changed_since_last",
            "*",
            "temporal_scope",
            "point_in_time",
            "occurred_at",
        ],
        value_class: McpAbilityMetadataValueClass::Timestamp,
    },
    McpAbilityMetadataPathRule {
        pattern: &[
            "what_changed_since_last",
            "*",
            "temporal_scope",
            "trend",
            "window_start",
        ],
        value_class: McpAbilityMetadataValueClass::Timestamp,
    },
    McpAbilityMetadataPathRule {
        pattern: &[
            "what_changed_since_last",
            "*",
            "temporal_scope",
            "trend",
            "window_end",
        ],
        value_class: McpAbilityMetadataValueClass::Timestamp,
    },
    McpAbilityMetadataPathRule {
        pattern: &["suggested_outcomes", "*", "subject", "kind"],
        value_class: McpAbilityMetadataValueClass::EntityKind,
    },
    McpAbilityMetadataPathRule {
        pattern: &["suggested_outcomes", "*", "subject", "id"],
        value_class: McpAbilityMetadataValueClass::Identifier,
    },
    McpAbilityMetadataPathRule {
        pattern: &["suggested_outcomes", "*", "temporal_scope"],
        value_class: McpAbilityMetadataValueClass::TemporalScope,
    },
    McpAbilityMetadataPathRule {
        pattern: &[
            "suggested_outcomes",
            "*",
            "temporal_scope",
            "point_in_time",
            "occurred_at",
        ],
        value_class: McpAbilityMetadataValueClass::Timestamp,
    },
    McpAbilityMetadataPathRule {
        pattern: &[
            "suggested_outcomes",
            "*",
            "temporal_scope",
            "trend",
            "window_start",
        ],
        value_class: McpAbilityMetadataValueClass::Timestamp,
    },
    McpAbilityMetadataPathRule {
        pattern: &[
            "suggested_outcomes",
            "*",
            "temporal_scope",
            "trend",
            "window_end",
        ],
        value_class: McpAbilityMetadataValueClass::Timestamp,
    },
];

fn render_mcp_ability_metadata_string(path: &[String], text: &str) -> Option<serde_json::Value> {
    let rule = mcp_ability_metadata_rule_for_path(path)?;
    if mcp_metadata_value_is_valid(rule.value_class, text) {
        return Some(serde_json::Value::String(text.to_string()));
    }

    log::warn!(
        target: "dailyos_lib::services::sensitivity",
        "MCP ability metadata validator rejected path={} class={:?}",
        json_pointer_from_path(path),
        rule.value_class
    );
    None
}

fn mcp_ability_metadata_rule_for_path(
    path: &[String],
) -> Option<&'static McpAbilityMetadataPathRule> {
    MCP_ABILITY_METADATA_STRING_ALLOWLIST
        .iter()
        .find(|rule| mcp_metadata_path_matches(rule.pattern, path))
}

fn mcp_metadata_path_matches(pattern: &[&str], path: &[String]) -> bool {
    pattern.len() == path.len()
        && pattern.iter().zip(path).all(|(expected, actual)| {
            if *expected == "*" {
                is_array_index(actual)
            } else {
                *expected == actual
            }
        })
}

fn mcp_metadata_value_is_valid(value_class: McpAbilityMetadataValueClass, text: &str) -> bool {
    match value_class {
        McpAbilityMetadataValueClass::Identifier => is_mcp_metadata_identifier(text),
        McpAbilityMetadataValueClass::EntityKind => is_mcp_entity_kind(text),
        McpAbilityMetadataValueClass::TemporalScope => is_mcp_temporal_scope(text),
        McpAbilityMetadataValueClass::Timestamp => is_iso8601_timestamp(text),
        McpAbilityMetadataValueClass::MeetingTitle | McpAbilityMetadataValueClass::EntityName => {
            is_mcp_metadata_label(text)
        }
    }
}

fn is_mcp_metadata_identifier(value: &str) -> bool {
    let value = value.trim();
    if value.is_empty() || value.len() > 128 {
        return false;
    }
    if uuid::Uuid::parse_str(value).is_ok() {
        return true;
    }
    value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':' | b'.'))
        && value
            .bytes()
            .any(|byte| matches!(byte, b'-' | b'_' | b':' | b'.'))
}

fn is_mcp_entity_kind(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "account" | "email" | "meeting" | "person" | "project"
    )
}

fn is_mcp_temporal_scope(value: &str) -> bool {
    value.trim().eq_ignore_ascii_case("state")
}

fn is_iso8601_timestamp(value: &str) -> bool {
    chrono::DateTime::parse_from_rfc3339(value.trim()).is_ok()
}

fn is_mcp_metadata_label(value: &str) -> bool {
    let value = value.trim();
    !value.is_empty()
        && value.len() <= 160
        && value.chars().all(|character| !character.is_control())
}

pub fn record_sensitivity_reveal(
    db: &ActionDb,
    claim_id: &str,
    user_id: &str,
    revealed_at: &str,
    reveal_action_id: &str,
) -> Result<(), rusqlite::Error> {
    db.conn_ref().execute(
        "INSERT OR IGNORE INTO sensitivity_reveal_audit
            (claim_id, user_id, revealed_at, reveal_action_id)
         VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![claim_id, user_id, revealed_at, reveal_action_id],
    )?;
    Ok(())
}

pub fn canonicalize_reveal_action_id(reveal_action_id: &str) -> Result<String, String> {
    let parsed = uuid::Uuid::parse_str(reveal_action_id)
        .map_err(|_| "reveal_action_id must be a UUID v4".to_string())?;
    if parsed.get_version() != Some(uuid::Version::Random) {
        return Err("reveal_action_id must be a UUID v4".to_string());
    }
    Ok(parsed.hyphenated().to_string())
}

pub fn validate_canonical_reveal_action_id(reveal_action_id: &str) -> Result<(), String> {
    let canonical = canonicalize_reveal_action_id(reveal_action_id)?;
    if reveal_action_id != canonical {
        return Err(
            "reveal_action_id must be a canonical lowercase hyphenated UUID v4".to_string(),
        );
    }
    Ok(())
}

pub fn reveal_claim_text_for_tauri(
    db: &ActionDb,
    claim_id: &str,
    surface: RenderSurface,
    actor: &RenderActor,
    reveal_action_id: String,
) -> Result<RenderableClaimText, String> {
    let reveal_action_id = canonicalize_reveal_action_id(&reveal_action_id)?;
    let claim = crate::services::claims::load_claim_by_id(db.conn_ref(), claim_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("Claim not found: {claim_id}"))?;
    if !claim_has_active_surfaced_lifecycle(&claim) {
        log::warn!(
            target: "dailyos_lib::services::sensitivity",
            "Tauri reveal claim text is not active/surfaced claim_id={}; dropping",
            claim_id
        );
        // Click-to-reveal audit rows record successful audited reveals only.
        // Lifecycle-denied attempts fail before policy approval, so they warn
        // and skip the audit insert rather than polluting the reveal trail.
        return Err("Claim is not revealable on this surface".to_string());
    }
    match render_policy_for_surface(&claim, surface, actor) {
        RenderDecision::Render => {
            renderable_from_decision(&claim, &claim.text, surface, RenderDecision::Render)
                .ok_or_else(|| "Claim cannot render on this surface".to_string())
        }
        RenderDecision::RenderRedacted {
            affordance:
                RedactionAffordance::ConfidentialClickToReveal {
                    audit_required: true,
                    ..
                },
        } => {
            let user_id = actor.user_id.as_deref().unwrap_or(actor.actor.as_str());
            let now = Utc::now().to_rfc3339();
            // This is a trusted-caller idempotency boundary, not adversary-resistant.
            // The Tauri command surface is first-party and the only client is the DailyOS frontend.
            record_sensitivity_reveal(db, &claim.id, user_id, &now, &reveal_action_id)
                .map_err(|error| error.to_string())?;
            renderable_from_decision(&claim, &claim.text, surface, RenderDecision::Render)
                .ok_or_else(|| "Claim cannot render on this surface".to_string())
        }
        RenderDecision::RenderRedacted { .. } | RenderDecision::Drop => {
            Err("Claim is not revealable on this surface".to_string())
        }
    }
}

pub fn apply_entity_intelligence_render_policy(
    db: &ActionDb,
    entity_type: &str,
    entity_id: &str,
    intelligence: &mut IntelligenceJson,
    surface: RenderSurface,
    actor: &RenderActor,
) {
    let subject_ref = serde_json::json!({
        "kind": entity_type,
        "id": entity_id,
    })
    .to_string();

    let Ok(claims) = crate::services::claims::load_claims_active_for_surface(
        db,
        &subject_ref,
        None,
        render_surface_dismissal_key(surface),
    ) else {
        return;
    };
    let rendered_summary = claims
        .iter()
        .filter(|claim| claim.claim_type == "entity_summary")
        .find_map(|claim| {
            let text = claim_projection_text(claim);
            renderable_claim_text_with_value(claim, &text, surface, actor)
        });
    intelligence.executive_assessment = rendered_summary.as_ref().map(|text| text.text.clone());
    intelligence.executive_assessment_render_policy = rendered_summary
        .as_ref()
        .and_then(|text| serde_json::to_value(&text.policy).ok());

    let risks = rendered_json_values_for_claim_type(&claims, "entity_risk", "text", surface, actor)
        .unwrap_or_default();
    intelligence.risks = risks
        .into_iter()
        .filter_map(|value| serde_json::from_value::<IntelRisk>(value).ok())
        .collect();

    let wins = rendered_json_values_for_claim_type(&claims, "entity_win", "text", surface, actor)
        .unwrap_or_default();
    intelligence.recent_wins = wins
        .into_iter()
        .filter_map(|value| serde_json::from_value::<IntelWin>(value).ok())
        .collect();

    let values = rendered_json_values_for_claim_type(
        &claims,
        "value_delivered",
        "statement",
        surface,
        actor,
    )
    .unwrap_or_default();
    intelligence.value_delivered = values
        .into_iter()
        .filter_map(|value| serde_json::from_value::<ValueItem>(value).ok())
        .collect();

    let stakeholders = rendered_json_values_for_claim_type(
        &claims,
        "stakeholder_engagement",
        "engagement",
        surface,
        actor,
    )
    .unwrap_or_default();
    intelligence.stakeholder_insights = stakeholders
        .into_iter()
        .filter_map(|value| serde_json::from_value::<StakeholderInsight>(value).ok())
        .collect();

    let current_state =
        rendered_texts_for_claim_type(&claims, "entity_current_state", surface, actor)
            .unwrap_or_default();
    intelligence.current_state = if current_state.is_empty() {
        None
    } else {
        Some(CurrentState {
            working: current_state,
            not_working: Vec::new(),
            unknowns: Vec::new(),
        })
    };

    let company_context = rendered_json_values_for_claim_type(
        &claims,
        "company_context",
        "description",
        surface,
        actor,
    )
    .unwrap_or_default();
    intelligence.company_context = company_context
        .into_iter()
        .next()
        .and_then(|value| serde_json::from_value::<CompanyContext>(value).ok());
}

fn render_surface_dismissal_key(surface: RenderSurface) -> &'static str {
    match surface {
        RenderSurface::TauriEntityDetail => "tauri_entity_detail",
        RenderSurface::TauriBriefingPrep => "briefing",
        RenderSurface::TauriMeetingDetail => "tauri_meeting_detail",
        RenderSurface::TauriEmailSummary => "tauri_email_summary",
        RenderSurface::TauriProvenance => "tauri_provenance",
        RenderSurface::TauriReport => "tauri_report",
        RenderSurface::TauriChat => "tauri_chat",
        RenderSurface::McpTool => "mcp_tool",
        RenderSurface::McpToolDetail => "mcp_tool_detail",
        RenderSurface::P2Publication => "p2_publication",
        RenderSurface::LogStructured => "log_structured",
        RenderSurface::PushNotification => "push_notification",
    }
}

fn public_policy(surface: RenderSurface) -> RenderDecision {
    if matches!(surface, RenderSurface::LogStructured) {
        RenderDecision::Drop
    } else {
        RenderDecision::Render
    }
}

fn internal_policy(surface: RenderSurface) -> RenderDecision {
    if surface.is_first_party_tauri() || surface.is_agent_surface() {
        RenderDecision::Render
    } else {
        RenderDecision::Drop
    }
}

fn confidential_policy(claim: &IntelligenceClaim, surface: RenderSurface) -> RenderDecision {
    if surface.allows_reveal() {
        RenderDecision::RenderRedacted {
            affordance: RedactionAffordance::ConfidentialClickToReveal {
                claim_id: claim.id.clone(),
                label: "Confidential claim hidden".to_string(),
                audit_required: true,
            },
        }
    } else if surface.is_first_party_tauri() {
        RenderDecision::RenderRedacted {
            affordance: RedactionAffordance::ConfidentialHidden {
                label: "Confidential claim hidden".to_string(),
            },
        }
    } else {
        RenderDecision::Drop
    }
}

fn user_only_policy(
    claim: &IntelligenceClaim,
    surface: RenderSurface,
    actor: &RenderActor,
) -> RenderDecision {
    if surface.is_first_party_tauri() && actor_owns_user_only_claim(claim, actor) {
        RenderDecision::Render
    } else if surface.is_first_party_tauri() {
        RenderDecision::RenderRedacted {
            affordance: RedactionAffordance::UserOnlyHidden {
                label: "User-only claim hidden".to_string(),
            },
        }
    } else {
        RenderDecision::Drop
    }
}

fn actor_owns_user_only_claim(claim: &IntelligenceClaim, actor: &RenderActor) -> bool {
    if !actor.is_user() {
        return false;
    }
    let claim_actor = claim.actor.trim();
    let actor_label = actor.actor.trim();
    let Some(user_id) = actor.user_id.as_deref() else {
        return claim_actor.eq_ignore_ascii_case("user")
            && actor_label.eq_ignore_ascii_case("user");
    };
    claim_actor.eq_ignore_ascii_case(user_id)
        || claim_actor
            .strip_prefix("user:")
            .is_some_and(|suffix| suffix.eq_ignore_ascii_case(user_id))
        || (claim_actor.eq_ignore_ascii_case("user") && user_id.eq_ignore_ascii_case("user"))
}

fn rendered_json_values_for_claim_type(
    claims: &[IntelligenceClaim],
    claim_type: &str,
    text_key: &str,
    surface: RenderSurface,
    actor: &RenderActor,
) -> Option<Vec<serde_json::Value>> {
    if !claims.iter().any(|claim| claim.claim_type == claim_type) {
        return None;
    }

    Some(
        claims
            .iter()
            .filter(|claim| claim.claim_type == claim_type)
            .filter_map(|claim| {
                let projection_text = claim_projection_text(claim);
                renderable_claim_text_with_value(claim, &projection_text, surface, actor)
                    .map(|rendered| projection_value_with_render_policy(claim, rendered, text_key))
            })
            .collect(),
    )
}

fn rendered_texts_for_claim_type(
    claims: &[IntelligenceClaim],
    claim_type: &str,
    surface: RenderSurface,
    actor: &RenderActor,
) -> Option<Vec<String>> {
    if !claims.iter().any(|claim| claim.claim_type == claim_type) {
        return None;
    }
    Some(
        claims
            .iter()
            .filter(|claim| claim.claim_type == claim_type)
            .filter_map(|claim| {
                let projection_text = claim_projection_text(claim);
                renderable_claim_text_with_value(claim, &projection_text, surface, actor)
                    .map(|rendered| rendered.text)
            })
            .collect(),
    )
}

fn projection_value_with_render_policy(
    claim: &IntelligenceClaim,
    rendered: RenderableClaimText,
    text_key: &str,
) -> serde_json::Value {
    let mut value = if rendered.policy.kind == RenderPolicyKind::Redacted {
        redacted_projection_value(text_key, &rendered.text)
    } else {
        claim_projection_value(claim)
            .unwrap_or_else(|| serde_json::json!({ text_key: rendered.text.clone() }))
    };

    if let serde_json::Value::Object(map) = &mut value {
        if !map.contains_key(text_key) {
            map.insert(
                text_key.to_string(),
                serde_json::Value::String(rendered.text.clone()),
            );
        }
        map.insert(
            "renderPolicy".to_string(),
            serde_json::to_value(&rendered.policy).unwrap_or(serde_json::Value::Null),
        );
        map.insert(
            "claimId".to_string(),
            serde_json::Value::String(claim.id.clone()),
        );
    }
    value
}

fn redacted_projection_value(text_key: &str, label: &str) -> serde_json::Value {
    match text_key {
        "engagement" => serde_json::json!({
            "name": "Sensitive stakeholder",
            "engagement": label,
        }),
        "statement" => serde_json::json!({ "statement": label }),
        "description" => serde_json::json!({ "description": label }),
        _ => serde_json::json!({ text_key: label }),
    }
}

fn claim_projection_value(claim: &IntelligenceClaim) -> Option<serde_json::Value> {
    let metadata = claim.metadata_json.as_deref()?;
    serde_json::from_str::<serde_json::Value>(metadata)
        .ok()?
        .get("legacy_projection_value")
        .cloned()
}

fn claim_projection_text(claim: &IntelligenceClaim) -> String {
    if let Some(value) = claim_projection_value(claim) {
        if let Some(text) = value.as_str() {
            return text.to_string();
        }
        for key in [
            "text",
            "statement",
            "description",
            "engagement",
            "assessment",
        ] {
            if let Some(text) = value.get(key).and_then(|value| value.as_str()) {
                return text.to_string();
            }
        }
    }
    claim.text.clone()
}

fn minimal_policy_claim(sensitivity: ClaimSensitivity, actor: &str) -> IntelligenceClaim {
    IntelligenceClaim {
        id: "policy-fixture".to_string(),
        subject_ref: "{}".to_string(),
        claim_type: "entity_summary".to_string(),
        field_path: None,
        topic_key: None,
        text: "fixture".to_string(),
        dedup_key: "fixture".to_string(),
        item_hash: None,
        actor: actor.to_string(),
        data_source: "test".to_string(),
        source_ref: None,
        source_asof: None,
        observed_at: "2026-05-06T00:00:00Z".to_string(),
        created_at: "2026-05-06T00:00:00Z".to_string(),
        provenance_json: "{}".to_string(),
        metadata_json: None,
        claim_state: crate::db::claims::ClaimState::Active,
        surfacing_state: crate::db::claims::SurfacingState::Active,
        demotion_reason: None,
        reactivated_at: None,
        retraction_reason: None,
        expires_at: None,
        superseded_by: None,
        trust_score: None,
        trust_computed_at: None,
        trust_version: None,
        thread_id: None,
        temporal_scope: crate::db::claims::TemporalScope::State,
        sensitivity,
        verification_state: crate::abilities::feedback::ClaimVerificationState::Active,
        verification_reason: None,
        needs_user_decision_at: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::ActionDb;
    use crate::services::claims::{commit_claim, ClaimProposal, CommittedClaim};
    use crate::services::context::{ExternalClients, FixedClock, SeedableRng, ServiceContext};
    use chrono::{TimeZone, Utc};
    use rusqlite::Connection;

    const TEST_ENTITY_ID: &str = "acct-render-policy-clear";
    const CLAIMS_SCHEMA_SQL: &str = include_str!("../migrations/129_dos_7_claims_schema.sql");
    const PROJECTION_STATUS_SQL: &str =
        include_str!("../migrations/134_dos_301_claim_projection_status.sql");
    const TYPED_FEEDBACK_SQL: &str =
        include_str!("../migrations/135_dos_294_typed_feedback_schema.sql");
    const CLAIM_SURFACE_DISMISSALS_SQL: &str =
        include_str!("../migrations/154_claim_surface_dismissals.sql");
    const MINIMAL_ENTITY_SCHEMA_SQL: &str = r#"
CREATE TABLE accounts (
    id TEXT PRIMARY KEY,
    claim_version INTEGER NOT NULL DEFAULT 0
);
"#;

    fn claim(sensitivity: ClaimSensitivity, actor: &str) -> IntelligenceClaim {
        minimal_policy_claim(sensitivity, actor)
    }

    fn fresh_claim_projection_db() -> ActionDb {
        let conn = Connection::open_in_memory().expect("open in-memory claim projection DB");
        conn.execute_batch(MINIMAL_ENTITY_SCHEMA_SQL)
            .expect("apply minimal entity schema");
        conn.execute(
            "INSERT INTO accounts (id, claim_version) VALUES (?1, 0)",
            [TEST_ENTITY_ID],
        )
        .expect("seed account");
        conn.execute_batch(CLAIMS_SCHEMA_SQL)
            .expect("apply claims schema");
        conn.execute_batch(TYPED_FEEDBACK_SQL)
            .expect("apply typed feedback schema");
        conn.execute_batch(PROJECTION_STATUS_SQL)
            .expect("apply projection status schema");
        conn.execute_batch(CLAIM_SURFACE_DISMISSALS_SQL)
            .expect("apply claim surface dismissals schema");
        ActionDb::from_connection_for_tests(conn)
    }

    fn seed_projection_claim(db: &ActionDb, id: &str, claim_type: &str, text: &str) -> String {
        let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 9, 13, 0, 0).unwrap());
        let rng = SeedableRng::new(310);
        let external = ExternalClients::default();
        let ctx = ServiceContext::new_live(&clock, &rng, &external).with_actor("agent:test");
        let committed = commit_claim(
            &ctx,
            db,
            ClaimProposal {
                id: Some(id.to_string()),
                subject_ref: serde_json::json!({
                    "kind": "account",
                    "id": TEST_ENTITY_ID,
                })
                .to_string(),
                claim_type: claim_type.to_string(),
                field_path: Some(format!("intelligence.{claim_type}")),
                topic_key: None,
                text: text.to_string(),
                actor: "agent:test".to_string(),
                data_source: "user".to_string(),
                source_ref: Some(format!("fixture:{id}")),
                source_asof: Some("2026-05-09T13:00:00Z".to_string()),
                observed_at: "2026-05-09T13:00:00Z".to_string(),
                provenance_json: serde_json::json!({ "source": "projection-clear-regression" })
                    .to_string(),
                metadata_json: None,
                thread_id: None,
                temporal_scope: Some(crate::db::claims::TemporalScope::State),
                sensitivity: Some(ClaimSensitivity::Internal),
                supersedes: None,
                tombstone: None,
            },
        )
        .expect("commit projection claim");

        match committed {
            CommittedClaim::Inserted { claim } => claim.id,
            other => panic!("expected inserted claim, got {other:?}"),
        }
    }

    fn dismiss_claim_on_surface(db: &ActionDb, claim_id: &str, surface: &str) {
        db.conn_ref()
            .execute(
                "INSERT INTO claim_surface_dismissals (
                    claim_id, surface, actor, dismissed_at
                 ) VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![claim_id, surface, "user", "2026-05-09T13:01:00Z"],
            )
            .expect("insert claim surface dismissal");
    }

    fn stale_intelligence() -> IntelligenceJson {
        IntelligenceJson {
            entity_id: TEST_ENTITY_ID.to_string(),
            entity_type: "account".to_string(),
            executive_assessment: Some("stale assessment".to_string()),
            executive_assessment_render_policy: Some(serde_json::json!({ "stale": true })),
            risks: vec![IntelRisk {
                text: "stale risk".to_string(),
                ..Default::default()
            }],
            recent_wins: vec![IntelWin {
                text: "stale win".to_string(),
                ..Default::default()
            }],
            current_state: Some(CurrentState {
                working: vec!["stale current state".to_string()],
                ..Default::default()
            }),
            stakeholder_insights: vec![StakeholderInsight {
                name: "Stale stakeholder".to_string(),
                engagement: Some("stale engagement".to_string()),
                ..Default::default()
            }],
            value_delivered: vec![ValueItem {
                statement: "stale value".to_string(),
                ..Default::default()
            }],
            company_context: Some(CompanyContext {
                description: Some("stale company context".to_string()),
                render_policy: None,
                claim_id: None,
                industry: None,
                size: None,
                headquarters: None,
                additional_context: None,
            }),
            ..Default::default()
        }
    }

    #[test]
    fn tauri_entity_detail_renders_public_internal_and_user_owned_user_only() {
        let actor = RenderActor::user("user", Some("user"));
        assert_eq!(
            render_policy_for_surface(
                &claim(ClaimSensitivity::Public, "agent:test"),
                RenderSurface::TauriEntityDetail,
                &actor
            ),
            RenderDecision::Render
        );
        assert_eq!(
            render_policy_for_surface(
                &claim(ClaimSensitivity::Internal, "agent:test"),
                RenderSurface::TauriEntityDetail,
                &actor
            ),
            RenderDecision::Render
        );
        assert_eq!(
            render_policy_for_surface(
                &claim(ClaimSensitivity::UserOnly, "user"),
                RenderSurface::TauriEntityDetail,
                &actor
            ),
            RenderDecision::Render
        );
    }

    #[test]
    fn tauri_entity_detail_redacts_confidential_with_click_to_reveal() {
        let actor = RenderActor::user("user", Some("user"));
        let decision = render_policy_for_surface(
            &claim(ClaimSensitivity::Confidential, "agent:test"),
            RenderSurface::TauriEntityDetail,
            &actor,
        );
        assert!(matches!(
            decision,
            RenderDecision::RenderRedacted {
                affordance: RedactionAffordance::ConfidentialClickToReveal { .. }
            }
        ));
    }

    #[test]
    fn mcp_drops_confidential_and_user_only() {
        let actor = RenderActor::agent("agent:mcp");
        assert_eq!(
            render_policy_for_surface(
                &claim(ClaimSensitivity::Confidential, "user"),
                RenderSurface::McpTool,
                &actor
            ),
            RenderDecision::Drop
        );
        assert_eq!(
            render_policy_for_surface(
                &claim(ClaimSensitivity::UserOnly, "user"),
                RenderSurface::McpTool,
                &actor
            ),
            RenderDecision::Drop
        );
    }

    #[test]
    fn unknown_surface_and_sensitivity_fail_closed() {
        let actor = RenderActor::user("user", Some("user"));
        assert_eq!(
            render_policy_for_surface_name(
                &claim(ClaimSensitivity::Public, "user"),
                "surprise_surface",
                &actor
            ),
            RenderDecision::Drop
        );
        assert_eq!(
            render_policy_for_sensitivity_name("secret", "tauri_entity_detail", "user", &actor),
            RenderDecision::Drop
        );
    }

    #[test]
    fn entity_intelligence_render_policy_clears_projection_when_all_claims_dismissed() {
        let db = fresh_claim_projection_db();
        let claim_id = seed_projection_claim(
            &db,
            "claim-projection-dismissed-summary",
            "entity_summary",
            "fresh summary hidden on entity detail",
        );
        dismiss_claim_on_surface(&db, &claim_id, "tauri_entity_detail");
        let mut intelligence = stale_intelligence();

        apply_entity_intelligence_render_policy(
            &db,
            "account",
            TEST_ENTITY_ID,
            &mut intelligence,
            RenderSurface::TauriEntityDetail,
            &RenderActor::user("user", Some("user")),
        );

        assert_eq!(intelligence.executive_assessment, None);
        assert_eq!(intelligence.executive_assessment_render_policy, None);
        assert!(intelligence.risks.is_empty());
        assert!(intelligence.recent_wins.is_empty());
        assert!(intelligence.value_delivered.is_empty());
        assert!(intelligence.stakeholder_insights.is_empty());
        assert!(intelligence.current_state.is_none());
        assert!(intelligence.company_context.is_none());
    }

    #[test]
    fn entity_intelligence_render_policy_clears_filtered_type_while_projecting_visible_claims() {
        let db = fresh_claim_projection_db();
        let summary_id = seed_projection_claim(
            &db,
            "claim-projection-dismissed-summary-only",
            "entity_summary",
            "summary hidden on entity detail",
        );
        seed_projection_claim(
            &db,
            "claim-projection-visible-risk",
            "entity_risk",
            "visible risk still projects",
        );
        dismiss_claim_on_surface(&db, &summary_id, "tauri_entity_detail");
        let mut intelligence = stale_intelligence();

        apply_entity_intelligence_render_policy(
            &db,
            "account",
            TEST_ENTITY_ID,
            &mut intelligence,
            RenderSurface::TauriEntityDetail,
            &RenderActor::user("user", Some("user")),
        );

        assert_eq!(intelligence.executive_assessment, None);
        assert_eq!(intelligence.executive_assessment_render_policy, None);
        assert_eq!(intelligence.risks.len(), 1);
        assert_eq!(intelligence.risks[0].text, "visible risk still projects");
        assert!(intelligence.recent_wins.is_empty());
    }
}
