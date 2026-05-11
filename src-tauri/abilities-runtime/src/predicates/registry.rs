use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::structured_claim::Polarity;

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum PredicateRef {
    AccountHealthStatus,
    AccountRenewalRisk,
    AccountObjectiveStatus,
    CommitmentCaptured,
    CommitmentOwner,
    CommitmentDue,
    ContractApprovalStatus,
    ContractSignatureStatus,
    ProductUsageTrend,
    RelationshipChampionStatus,
    RiskStatus,
    StakeholderRole,
    TopicMentioned,
    Unresolved { text: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PredicateDefinition {
    pub predicate: PredicateRef,
    pub canonical: &'static str,
    pub aliases: &'static [&'static str],
}

impl PredicateRef {
    pub fn registry_id(&self) -> String {
        match self {
            Self::AccountHealthStatus => "account.health_status",
            Self::AccountRenewalRisk => "account.renewal_risk",
            Self::AccountObjectiveStatus => "account.objective_status",
            Self::CommitmentCaptured => "commitment.captured",
            Self::CommitmentOwner => "commitment.owner",
            Self::CommitmentDue => "commitment.due",
            Self::ContractApprovalStatus => "contract.approval_status",
            Self::ContractSignatureStatus => "contract.signature_status",
            Self::ProductUsageTrend => "product.usage_trend",
            Self::RelationshipChampionStatus => "relationship.champion_status",
            Self::RiskStatus => "risk.status",
            Self::StakeholderRole => "stakeholder.role",
            Self::TopicMentioned => "topic.mentioned",
            Self::Unresolved { text } => return format!("unresolved:{}", normalize_alias(text)),
        }
        .to_string()
    }

    pub fn definition(&self) -> Option<&'static PredicateDefinition> {
        PREDICATE_REGISTRY
            .iter()
            .find(|definition| definition.predicate == *self)
    }

    pub fn is_unresolved(&self) -> bool {
        matches!(self, Self::Unresolved { .. })
    }
}

pub const PREDICATE_REGISTRY_VERSION: &str = "predicate-registry:adr-0131:v1";

pub static PREDICATE_REGISTRY: &[PredicateDefinition] = &[
    PredicateDefinition {
        predicate: PredicateRef::AccountHealthStatus,
        canonical: "account health status",
        aliases: &["health", "health status", "account health"],
    },
    PredicateDefinition {
        predicate: PredicateRef::AccountRenewalRisk,
        canonical: "account renewal risk",
        aliases: &["renewal risk", "risk to renewal", "renewal at risk"],
    },
    PredicateDefinition {
        predicate: PredicateRef::AccountObjectiveStatus,
        canonical: "account objective status",
        aliases: &["objective", "goal", "business outcome"],
    },
    PredicateDefinition {
        predicate: PredicateRef::CommitmentCaptured,
        canonical: "commitment captured",
        aliases: &["commitment", "next step", "follow up"],
    },
    PredicateDefinition {
        predicate: PredicateRef::CommitmentOwner,
        canonical: "commitment owner",
        aliases: &["owner", "responsible person", "assignee"],
    },
    PredicateDefinition {
        predicate: PredicateRef::CommitmentDue,
        canonical: "commitment due",
        aliases: &["due date", "deadline", "target date"],
    },
    PredicateDefinition {
        predicate: PredicateRef::ContractApprovalStatus,
        canonical: "contract approval status",
        aliases: &["approval", "approved", "greenlit", "budget approval"],
    },
    PredicateDefinition {
        predicate: PredicateRef::ContractSignatureStatus,
        canonical: "contract signature status",
        aliases: &["signing", "signature", "signed"],
    },
    PredicateDefinition {
        predicate: PredicateRef::ProductUsageTrend,
        canonical: "product usage trend",
        aliases: &["usage", "adoption", "engagement"],
    },
    PredicateDefinition {
        predicate: PredicateRef::RelationshipChampionStatus,
        canonical: "relationship champion status",
        aliases: &["champion", "advocate", "executive sponsor"],
    },
    PredicateDefinition {
        predicate: PredicateRef::RiskStatus,
        canonical: "risk status",
        aliases: &["risk", "blocker", "concern"],
    },
    PredicateDefinition {
        predicate: PredicateRef::StakeholderRole,
        canonical: "stakeholder role",
        aliases: &["role", "persona", "buyer role"],
    },
    PredicateDefinition {
        predicate: PredicateRef::TopicMentioned,
        canonical: "topic mentioned",
        aliases: &["topic", "mentioned", "discussion topic"],
    },
];

pub fn resolve_predicate_alias(
    text: &str,
    polarity: Polarity,
    expected_polarity: Polarity,
) -> Option<PredicateRef> {
    if polarity != expected_polarity {
        return None;
    }
    let normalized = normalize_alias(text);
    PREDICATE_REGISTRY
        .iter()
        .find(|definition| {
            normalize_alias(definition.canonical) == normalized
                || definition
                    .aliases
                    .iter()
                    .any(|alias| normalize_alias(alias) == normalized)
        })
        .map(|definition| definition.predicate.clone())
}

fn normalize_alias(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}
