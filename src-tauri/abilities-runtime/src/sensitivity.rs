use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::types::{ClaimSensitivity, IntelligenceClaim};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ClaimDismissalSurface {
    TauriEntityDetail,
    Briefing,
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
    Worker,
    Eval,
}

impl ClaimDismissalSurface {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TauriEntityDetail => "tauri_entity_detail",
            Self::Briefing => "briefing",
            Self::TauriMeetingDetail => "tauri_meeting_detail",
            Self::TauriEmailSummary => "tauri_email_summary",
            Self::TauriProvenance => "tauri_provenance",
            Self::TauriReport => "tauri_report",
            Self::TauriChat => "tauri_chat",
            Self::McpTool => "mcp_tool",
            Self::McpToolDetail => "mcp_tool_detail",
            Self::P2Publication => "p2_publication",
            Self::LogStructured => "log_structured",
            Self::PushNotification => "push_notification",
            Self::Worker => "worker",
            Self::Eval => "eval",
        }
    }

    pub fn from_name(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "tauri_entity_detail" | "entity_detail" => Some(Self::TauriEntityDetail),
            "briefing" | "tauri_briefing_prep" | "briefing_prep" => Some(Self::Briefing),
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
            "worker" => Some(Self::Worker),
            "eval" => Some(Self::Eval),
            _ => None,
        }
    }
}

impl From<RenderSurface> for ClaimDismissalSurface {
    fn from(surface: RenderSurface) -> Self {
        match surface {
            RenderSurface::TauriEntityDetail => Self::TauriEntityDetail,
            RenderSurface::TauriBriefingPrep => Self::Briefing,
            RenderSurface::TauriMeetingDetail => Self::TauriMeetingDetail,
            RenderSurface::TauriEmailSummary => Self::TauriEmailSummary,
            RenderSurface::TauriProvenance => Self::TauriProvenance,
            RenderSurface::TauriReport => Self::TauriReport,
            RenderSurface::TauriChat => Self::TauriChat,
            RenderSurface::McpTool => Self::McpTool,
            RenderSurface::McpToolDetail => Self::McpToolDetail,
            RenderSurface::P2Publication => Self::P2Publication,
            RenderSurface::LogStructured => Self::LogStructured,
            RenderSurface::PushNotification => Self::PushNotification,
        }
    }
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

    pub fn allows_reveal(self) -> bool {
        matches!(
            self,
            Self::TauriEntityDetail
                | Self::TauriMeetingDetail
                | Self::TauriEmailSummary
                | Self::TauriProvenance
                | Self::TauriReport
        )
    }

    pub fn is_first_party_tauri(self) -> bool {
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

    pub fn is_agent_surface(self) -> bool {
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

    pub fn is_user(&self) -> bool {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ClaimVerificationState {
    #[default]
    Active,
    Contested,
    NeedsUserDecision,
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
