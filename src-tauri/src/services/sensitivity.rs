//! ADR-0108 output-boundary sensitivity rendering.
//!
//! This module is the single helper for claim-derived text that leaves the
//! Rust boundary for UI or MCP consumers. ADR-0108 sections 1 and 2 require
//! per-surface actor filtering: Tauri app surfaces may show first-party
//! affordances, MCP responses are actor-filtered for agents, and publication,
//! log, and notification surfaces fail closed.

use crate::db::claims::{ClaimSensitivity, IntelligenceClaim};
use crate::db::ActionDb;
use crate::intelligence::{
    CompanyContext, CurrentState, IntelRisk, IntelWin, IntelligenceJson, StakeholderInsight,
    ValueItem,
};
use chrono::Utc;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RenderSurface {
    TauriEntityDetail,
    TauriBriefingPrep,
    TauriMeetingDetail,
    TauriEmailSummary,
    TauriProvenance,
    TauriReport,
    TauriChat,
    McpTool,
    McpToolDetail,
    P2Publication,
    LogStructured,
    PushNotification,
}

impl RenderSurface {
    pub fn from_name(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "tauri_entity_detail" | "entity_detail" => Some(Self::TauriEntityDetail),
            "tauri_briefing_prep" | "briefing_prep" => Some(Self::TauriBriefingPrep),
            "tauri_meeting_detail" | "meeting_detail" => Some(Self::TauriMeetingDetail),
            "tauri_email_summary" | "email_summary" => Some(Self::TauriEmailSummary),
            "tauri_provenance" | "provenance" => Some(Self::TauriProvenance),
            "tauri_report" | "report" => Some(Self::TauriReport),
            "tauri_chat" | "chat" => Some(Self::TauriChat),
            "mcp_tool" => Some(Self::McpTool),
            "mcp_tool_detail" => Some(Self::McpToolDetail),
            "p2_publication" => Some(Self::P2Publication),
            "log_structured" => Some(Self::LogStructured),
            "push_notification" => Some(Self::PushNotification),
            _ => None,
        }
    }

    fn allows_reveal(self) -> bool {
        matches!(
            self,
            Self::TauriEntityDetail
                | Self::TauriMeetingDetail
                | Self::TauriEmailSummary
                | Self::TauriProvenance
                | Self::TauriReport
        )
    }

    fn is_first_party_tauri(self) -> bool {
        matches!(
            self,
            Self::TauriEntityDetail
                | Self::TauriBriefingPrep
                | Self::TauriMeetingDetail
                | Self::TauriEmailSummary
                | Self::TauriProvenance
                | Self::TauriReport
        )
    }

    fn is_agent_surface(self) -> bool {
        matches!(self, Self::McpTool | Self::McpToolDetail | Self::TauriChat)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderActor {
    pub actor: String,
    pub user_id: Option<String>,
}

impl RenderActor {
    pub fn user(actor: impl Into<String>, user_id: Option<impl Into<String>>) -> Self {
        Self {
            actor: actor.into(),
            user_id: user_id.map(Into::into),
        }
    }

    pub fn agent(actor: impl Into<String>) -> Self {
        Self {
            actor: actor.into(),
            user_id: None,
        }
    }

    fn is_user(&self) -> bool {
        self.actor.trim().eq_ignore_ascii_case("user")
            || self.actor.trim().to_ascii_lowercase().starts_with("user:")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum RedactionAffordance {
    ConfidentialClickToReveal {
        claim_id: String,
        label: String,
        audit_required: bool,
    },
    ConfidentialHidden {
        label: String,
    },
    UserOnlyHidden {
        label: String,
    },
}

impl RedactionAffordance {
    pub fn label(&self) -> &str {
        match self {
            Self::ConfidentialClickToReveal { label, .. }
            | Self::ConfidentialHidden { label }
            | Self::UserOnlyHidden { label } => label,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RenderPolicyKind {
    Render,
    Redacted,
    Drop,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RenderPolicy {
    pub kind: RenderPolicyKind,
    pub sensitivity: ClaimSensitivity,
    pub surface: RenderSurface,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claim_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub affordance: Option<RedactionAffordance>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderDecision {
    Render,
    RenderRedacted { affordance: RedactionAffordance },
    Drop,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RenderableClaimText {
    pub text: String,
    pub policy: RenderPolicy,
}

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
    let actor = RenderActor::agent("agent:mcp");
    let claim = crate::services::claims::load_claim_by_id(db.conn_ref(), &value.claim_id)
        .ok()
        .flatten()?;
    if claim.sensitivity != value.sensitivity {
        return None;
    }
    renderable_claim_text_with_value(&claim, &value.text, RenderSurface::McpTool, &actor)
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
/// DOS-412 Track GG inverts the MCP output boundary to deny by default. Every
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
            if is_mcp_ability_non_content_metadata_string(path) {
                return Some(serde_json::Value::String(text));
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
    })
}

fn render_tagged_mcp_claim_text(
    tagged: TaggedMcpClaimText,
    load_claim: &impl Fn(&str) -> Option<IntelligenceClaim>,
) -> Option<serde_json::Value> {
    let actor = RenderActor::agent("agent:mcp");
    let claim = load_claim(&tagged.claim_id)?;

    if claim.sensitivity != tagged.sensitivity {
        log::warn!(
            target: "dailyos_lib::services::sensitivity",
            "MCP ability data claim sensitivity mismatch claim_id={} stored={:?} emitted={:?}; dropping tagged object",
            tagged.claim_id,
            claim.sensitivity,
            tagged.sensitivity
        );
        return None;
    }

    let decision = render_policy_for_surface(&claim, RenderSurface::McpTool, &actor);
    let rendered =
        renderable_from_decision(&claim, &tagged.text, RenderSurface::McpTool, decision)?;
    safe_tagged_mcp_claim_text_object(rendered)
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
        })
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

fn is_mcp_ability_non_content_metadata_string(path: &[String]) -> bool {
    let Some(key) = metadata_key_for_path(path) else {
        return false;
    };

    // Identifiers are join handles, not prose. They are needed so callers can
    // correlate MCP rows without exposing claim text.
    if is_identifier_metadata_key(key) {
        return true;
    }

    // Enum-shaped state is non-content routing metadata. These values describe
    // lifecycle or type, not user-authored or model-authored claim text.
    if matches!(
        key,
        "actor"
            | "claim_state"
            | "claimState"
            | "entity_type"
            | "entityType"
            | "kind"
            | "lifecycle"
            | "mode"
            | "priority"
            | "sensitivity"
            | "status"
            | "surfacing_state"
            | "surfacingState"
            | "temporal_scope"
            | "temporalScope"
            | "trust_band"
            | "trustBand"
    ) {
        return true;
    }

    // Timestamp-shaped fields are chronology metadata. They can be useful to
    // agents without carrying narrative claim content.
    if is_timestamp_metadata_key(key) {
        return true;
    }

    // Name-shaped fields are allowlisted only for established metadata
    // surfaces. Generated section titles, open-loop owners, and attendee
    // summaries are intentionally excluded unless claim/provenance-attested.
    matches_meeting_metadata_name_path(path) || matches!(key, "display_name" | "displayName")
}

fn metadata_key_for_path(path: &[String]) -> Option<&str> {
    let key = path.last()?.as_str();
    if is_array_index(key) {
        path.iter()
            .rev()
            .skip(1)
            .find(|token| !is_array_index(token))
            .map(String::as_str)
    } else {
        Some(key)
    }
}

fn is_array_index(value: &str) -> bool {
    !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit())
}

fn is_identifier_metadata_key(key: &str) -> bool {
    key == "id"
        || key.ends_with("_id")
        || key.ends_with("Id")
        || matches!(key, "claim_id" | "claimId" | "source_ids" | "sourceIds")
}

fn is_timestamp_metadata_key(key: &str) -> bool {
    key.ends_with("_at")
        || key.ends_with("At")
        || matches!(
            key,
            "source_asof"
                | "sourceAsof"
                | "window_start"
                | "windowStart"
                | "window_end"
                | "windowEnd"
        )
}

fn matches_meeting_metadata_name_path(path: &[String]) -> bool {
    if path == ["meeting", "title"] {
        return true;
    }
    if path.len() == 4
        && path[0] == "meeting"
        && path[1] == "attendees"
        && is_array_index(&path[2])
        && path[3] == "name"
    {
        return true;
    }
    false
}

pub fn record_sensitivity_reveal(
    db: &ActionDb,
    claim_id: &str,
    user_id: &str,
    revealed_at: &str,
) -> Result<(), rusqlite::Error> {
    db.conn_ref().execute(
        "INSERT INTO sensitivity_reveal_audit (claim_id, user_id, revealed_at)
         VALUES (?1, ?2, ?3)",
        rusqlite::params![claim_id, user_id, revealed_at],
    )?;
    Ok(())
}

pub fn reveal_claim_text_for_tauri(
    db: &ActionDb,
    claim_id: &str,
    surface: RenderSurface,
    actor: &RenderActor,
) -> Result<RenderableClaimText, String> {
    let claim = crate::services::claims::load_claim_by_id(db.conn_ref(), claim_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("Claim not found: {claim_id}"))?;
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
            record_sensitivity_reveal(db, &claim.id, user_id, &now)
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

    let Ok(claims) = crate::services::claims::load_claims_active(db, &subject_ref, None) else {
        return;
    };
    if claims.is_empty() {
        return;
    }

    let has_entity_summary = claims
        .iter()
        .any(|claim| claim.claim_type == "entity_summary");
    if has_entity_summary {
        let rendered = claims
            .iter()
            .filter(|claim| claim.claim_type == "entity_summary")
            .find_map(|claim| {
                let text = claim_projection_text(claim);
                renderable_claim_text_with_value(claim, &text, surface, actor)
            });
        intelligence.executive_assessment = rendered.as_ref().map(|text| text.text.clone());
        intelligence.executive_assessment_render_policy = rendered
            .as_ref()
            .and_then(|text| serde_json::to_value(&text.policy).ok());
    }

    let risks = rendered_json_values_for_claim_type(&claims, "entity_risk", "text", surface, actor);
    if let Some(values) = risks {
        intelligence.risks = values
            .into_iter()
            .filter_map(|value| serde_json::from_value::<IntelRisk>(value).ok())
            .collect();
    }

    let wins = rendered_json_values_for_claim_type(&claims, "entity_win", "text", surface, actor);
    if let Some(values) = wins {
        intelligence.recent_wins = values
            .into_iter()
            .filter_map(|value| serde_json::from_value::<IntelWin>(value).ok())
            .collect();
    }

    let values = rendered_json_values_for_claim_type(
        &claims,
        "value_delivered",
        "statement",
        surface,
        actor,
    );
    if let Some(values) = values {
        intelligence.value_delivered = values
            .into_iter()
            .filter_map(|value| serde_json::from_value::<ValueItem>(value).ok())
            .collect();
    }

    let stakeholders = rendered_json_values_for_claim_type(
        &claims,
        "stakeholder_engagement",
        "engagement",
        surface,
        actor,
    );
    if let Some(values) = stakeholders {
        intelligence.stakeholder_insights = values
            .into_iter()
            .filter_map(|value| serde_json::from_value::<StakeholderInsight>(value).ok())
            .collect();
    }

    let current_state =
        rendered_texts_for_claim_type(&claims, "entity_current_state", surface, actor);
    if let Some(items) = current_state {
        intelligence.current_state = if items.is_empty() {
            None
        } else {
            Some(CurrentState {
                working: items,
                not_working: Vec::new(),
                unknowns: Vec::new(),
            })
        };
    }

    let company_context = rendered_json_values_for_claim_type(
        &claims,
        "company_context",
        "description",
        surface,
        actor,
    );
    if let Some(values) = company_context {
        intelligence.company_context = values
            .into_iter()
            .next()
            .and_then(|value| serde_json::from_value::<CompanyContext>(value).ok());
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

    fn claim(sensitivity: ClaimSensitivity, actor: &str) -> IntelligenceClaim {
        minimal_policy_claim(sensitivity, actor)
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
}
