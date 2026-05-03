use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Role-specific intelligence configuration: AI system role framing, dimension
/// weights and vocabulary, signal keywords, email signal types, and
/// email priority keywords. Merged with base lists at `set_role` time.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetIntelligenceConfig {
    /// AI system role framing (e.g. "customer success intelligence system").
    #[serde(default)]
    pub system_role: String,
    /// Per-dimension base weights (sum ~1.0).
    #[serde(default)]
    pub dimension_weights: HashMap<String, f64>,
    /// Role-specific signal keywords with relevance weights.
    /// Merged with base `KEYWORD_WEIGHTS` at `set_role` time (max-wins on duplicates).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub signal_keywords: Vec<PresetSignalKeyword>,
    /// Role-specific email signal types for boost classification.
    /// Merged with the generic base `BOOST_SIGNAL_TYPES` list.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub email_signal_types: Vec<String>,
    /// Role-specific email subject keywords for high-priority classification.
    /// Merged with base `HIGH_PRIORITY_SUBJECT_KEYWORDS` from `constants.rs`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub email_priority_keywords: Vec<String>,
    /// Per-dimension display labels for health scoring UI.
    #[serde(default)]
    pub dimension_labels: HashMap<String, String>,
    /// Preset-specific word for the "renewal/agreement close" concept
    /// (e.g. "renewal" for CS, "contract" for consulting).
    #[serde(default)]
    pub close_concept: String,
    /// Preset-specific label for the key advocate role
    /// (e.g. "champion" for CS, "executive sponsor" for sales).
    #[serde(default)]
    pub key_advocate_label: String,
    /// Per-dimension guidance strings surfaced in intelligence prompts.
    #[serde(default)]
    pub dimension_guidance: HashMap<String, String>,
}

/// A single keyword with its relevance weight for signal scoring.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetSignalKeyword {
    pub keyword: String,
    pub weight: f64,
}

/// A role preset defining vocabulary, vitals, metadata, and prioritization
/// for a specific user persona (e.g. Customer Success, Sales, Product).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RolePreset {
    pub id: String,
    pub name: String,
    pub description: String,
    pub default_entity_mode: String,
    pub vocabulary: PresetVocabulary,
    pub vitals: PresetVitalsConfig,
    pub metadata: PresetMetadataConfig,
    /// Stakeholder roles for external contacts linked to entities.
    /// Stored on `account_stakeholders.relationship_type` or `entity_members.relationship_type`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stakeholder_roles: Vec<PresetRoleDefinition>,
    /// Internal team roles for the account team.
    /// Stored on `account_stakeholders.role`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub internal_team_roles: Vec<PresetRoleDefinition>,
    pub lifecycle_events: Vec<String>,
    pub prioritization: PresetPrioritization,
    /// Role-specific intelligence configuration: system role framing, dimension
    /// weights, signal keywords, email signal types, and prompt vocabulary.
    /// Merged with base lists at `set_role` time.
    #[serde(default)]
    pub intelligence: PresetIntelligenceConfig,
    pub briefing_emphasis: String,
    /// Role-specific email subject keywords that trigger high-priority classification.
    /// Added to the hardcoded base list in `constants.rs`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub email_priority_keywords: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetVocabulary {
    pub entity_noun: String,
    pub entity_noun_plural: String,
    pub primary_metric: String,
    pub health_label: String,
    pub risk_label: String,
    pub success_verb: String,
    pub cadence_noun: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetVitalsConfig {
    pub account: Vec<PresetVitalField>,
    pub project: Vec<PresetVitalField>,
    pub person: Vec<PresetVitalField>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetVitalField {
    pub key: String,
    pub label: String,
    pub field_type: String,
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column_mapping: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetMetadataConfig {
    pub account: Vec<PresetMetadataField>,
    pub project: Vec<PresetMetadataField>,
    pub person: Vec<PresetMetadataField>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetMetadataField {
    pub key: String,
    pub label: String,
    pub field_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<String>>,
    #[serde(default)]
    pub required: bool,
}

/// A named role (stakeholder or internal team) defined by a preset.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetRoleDefinition {
    pub id: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetPrioritization {
    pub primary_signal: String,
    pub secondary_signal: String,
    pub urgency_drivers: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cs_preset_deserializes() {
        let json = include_str!("../../presets/customer-success.json");
        let preset: RolePreset = serde_json::from_str(json).expect("CS preset should parse");
        assert_eq!(preset.id, "customer-success");
        assert_eq!(preset.default_entity_mode, "account");
        assert_eq!(preset.vocabulary.entity_noun, "account");
        assert_eq!(preset.vitals.account.len(), 5);
        assert_eq!(preset.vitals.person.len(), 0);
        assert_eq!(preset.metadata.account.len(), 0);
        assert_eq!(preset.stakeholder_roles.len(), 7);
        assert_eq!(preset.internal_team_roles.len(), 6);
        assert_eq!(preset.lifecycle_events.len(), 10);
        assert_eq!(preset.prioritization.primary_signal, "arr");
        assert_eq!(preset.intelligence.close_concept, "renewal");
        assert_eq!(preset.intelligence.key_advocate_label, "champion");
    }

    #[test]
    fn test_preset_roundtrip() {
        let json = include_str!("../../presets/customer-success.json");
        let preset: RolePreset = serde_json::from_str(json).unwrap();
        let serialized = serde_json::to_string(&preset).unwrap();
        let roundtrip: RolePreset = serde_json::from_str(&serialized).unwrap();
        assert_eq!(roundtrip.id, preset.id);
        assert_eq!(roundtrip.name, preset.name);
    }
}
