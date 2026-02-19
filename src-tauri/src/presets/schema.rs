use serde::{Deserialize, Serialize};

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
    /// Stored on `entity_people.relationship_type`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stakeholder_roles: Vec<PresetRoleDefinition>,
    /// Internal team roles for the account team.
    /// Stored on `account_team.role`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub internal_team_roles: Vec<PresetRoleDefinition>,
    pub lifecycle_events: Vec<String>,
    pub prioritization: PresetPrioritization,
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
