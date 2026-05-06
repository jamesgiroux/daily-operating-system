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

#[derive(Debug, Clone, PartialEq, Eq)]
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

/// Render claim-derived ability `data` for the MCP tool surface.
///
/// DOS-412 Track EE intentionally uses design B: a generic JSON walker that
/// recognizes tagged claim text instead of descriptor-declared JSON paths.
/// This is less invasive for the current ability registry because existing
/// descriptors and the `#[ability]` macro do not carry output-path metadata,
/// while the sensitivity module already owns a `RenderableClaimText` policy
/// shape. Ability outputs that emit claim text to MCP must tag that text with
/// claim metadata (`text` plus `claim_id`/`claimId` and `sensitivity`, or the
/// existing `text` plus `policy.claimId`/`policy.sensitivity` shape). Tagged
/// leaves are rendered through `render_policy_for_surface(..., McpTool, ...)`;
/// untagged narrative/text-shaped leaves fail closed and are removed.
pub fn render_mcp_ability_data_for_surface(value: serde_json::Value) -> serde_json::Value {
    render_mcp_ability_data_value(value, None).unwrap_or(serde_json::Value::Null)
}

#[derive(Debug, Clone)]
struct TaggedMcpClaimText {
    text: String,
    claim_id: String,
    sensitivity: ClaimSensitivity,
    originating_actor: String,
}

enum TaggedMcpClaimTextMatch {
    NotTagged,
    Malformed,
    Tagged(TaggedMcpClaimText),
}

fn render_mcp_ability_data_value(
    value: serde_json::Value,
    key: Option<&str>,
) -> Option<serde_json::Value> {
    match value {
        serde_json::Value::Object(object) => match tagged_mcp_claim_text(&object) {
            TaggedMcpClaimTextMatch::Tagged(tagged) => render_tagged_mcp_claim_text(object, tagged),
            TaggedMcpClaimTextMatch::Malformed => None,
            TaggedMcpClaimTextMatch::NotTagged => {
                let mut rendered = serde_json::Map::new();
                for (key, value) in object {
                    if let Some(value) = render_mcp_ability_data_value(value, Some(&key)) {
                        rendered.insert(key, value);
                    }
                }
                Some(serde_json::Value::Object(rendered))
            }
        },
        serde_json::Value::Array(values) => Some(serde_json::Value::Array(
            values
                .into_iter()
                .filter_map(|value| render_mcp_ability_data_value(value, None))
                .collect(),
        )),
        serde_json::Value::String(text) => {
            if key.is_some_and(is_mcp_ability_claim_text_field) {
                None
            } else {
                Some(serde_json::Value::String(text))
            }
        }
        other => Some(other),
    }
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
    let originating_actor =
        string_field(object, &["originating_actor", "originatingActor", "actor"])
            .unwrap_or_else(|| "user".to_string());

    TaggedMcpClaimTextMatch::Tagged(TaggedMcpClaimText {
        text,
        claim_id,
        sensitivity,
        originating_actor,
    })
}

fn render_tagged_mcp_claim_text(
    mut object: serde_json::Map<String, serde_json::Value>,
    tagged: TaggedMcpClaimText,
) -> Option<serde_json::Value> {
    let actor = RenderActor::agent("agent:mcp");
    let mut claim = minimal_policy_claim(tagged.sensitivity, &tagged.originating_actor);
    claim.id = tagged.claim_id;
    claim.text = tagged.text.clone();

    let decision = render_policy_for_surface(&claim, RenderSurface::McpTool, &actor);
    let rendered =
        renderable_from_decision(&claim, &tagged.text, RenderSurface::McpTool, decision)?;
    object.insert("text".to_string(), serde_json::Value::String(rendered.text));
    object.insert(
        "policy".to_string(),
        serde_json::to_value(rendered.policy).unwrap_or(serde_json::Value::Null),
    );
    object.remove("renderPolicy");
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

fn is_mcp_ability_claim_text_field(key: &str) -> bool {
    matches!(
        key,
        "briefing"
            | "content"
            | "context"
            | "description"
            | "detail"
            | "emails"
            | "intelligenceSummary"
            | "intelligence_summary"
            | "meetingContext"
            | "openActions"
            | "open_actions"
            | "outcome"
            | "rationale"
            | "schedule"
            | "snippet"
            | "summary"
            | "text"
            | "title"
    )
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
