use super::embedded;
use super::schema::{PresetIntelligenceConfig, RolePreset};
use std::collections::HashMap;

pub const INTELLIGENCE_DIMENSION_KEYS: [&str; 6] = [
    "meeting_cadence",
    "email_engagement",
    "stakeholder_coverage",
    "key_advocate_health",
    "financial_proximity",
    "signal_momentum",
];

/// Load an embedded preset by role ID.
pub fn load_preset(role: &str) -> Result<RolePreset, String> {
    if let Some(json) = embedded::get_embedded(role) {
        let preset: RolePreset = serde_json::from_str(json)
            .map_err(|e| format!("Failed to parse embedded preset '{}': {}", role, e))?;
        validate_preset(&preset)?;
        return Ok(preset);
    }
    Err(format!("Unknown preset role: {}", role))
}

/// Load a custom preset from a file path.
pub fn load_custom_preset(path: &std::path::Path) -> Result<RolePreset, String> {
    let content =
        std::fs::read_to_string(path).map_err(|e| format!("Failed to read preset file: {}", e))?;
    let preset: RolePreset =
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse preset: {}", e))?;
    validate_preset(&preset)?;
    Ok(preset)
}

/// Validate a preset has required fields and valid values.
pub fn validate_preset(preset: &RolePreset) -> Result<(), String> {
    if preset.id.is_empty() {
        return Err("Preset id is required".into());
    }
    if preset.name.is_empty() {
        return Err("Preset name is required".into());
    }
    let valid_modes = ["account", "project", "both"];
    if !valid_modes.contains(&preset.default_entity_mode.as_str()) {
        return Err(format!(
            "Invalid entity mode: {}",
            preset.default_entity_mode
        ));
    }
    validate_intelligence(&preset.intelligence)?;
    Ok(())
}

fn validate_intelligence(intelligence: &PresetIntelligenceConfig) -> Result<(), String> {
    if intelligence.system_role.trim().is_empty() {
        return Err("Preset intelligence systemRole is required".into());
    }
    if intelligence.close_concept.trim().is_empty() {
        return Err("Preset intelligence closeConcept is required".into());
    }
    if intelligence.key_advocate_label.trim().is_empty() {
        return Err("Preset intelligence keyAdvocateLabel is required".into());
    }

    let mut expected = INTELLIGENCE_DIMENSION_KEYS.to_vec();
    expected.sort_unstable();
    let mut actual: Vec<&str> = intelligence
        .dimension_weights
        .keys()
        .map(String::as_str)
        .collect();
    actual.sort_unstable();
    if actual != expected {
        return Err(format!(
            "Preset intelligence dimensionWeights keys must be exactly {:?}; got {:?}",
            expected, actual
        ));
    }

    for key in INTELLIGENCE_DIMENSION_KEYS {
        if !intelligence.dimension_labels.contains_key(key) {
            return Err(format!(
                "Preset intelligence dimensionLabels missing key '{}'",
                key
            ));
        }
        if !intelligence.dimension_guidance.contains_key(key) {
            return Err(format!(
                "Preset intelligence dimensionGuidance missing key '{}'",
                key
            ));
        }
    }

    let total: f64 = intelligence.dimension_weights.values().sum();
    if (total - 1.0).abs() > 0.01 {
        return Err(format!(
            "Preset intelligence dimensionWeights must sum to 1.0 (+/- 0.01); got {:.3}",
            total
        ));
    }

    for entry in &intelligence.signal_keywords {
        if entry.keyword.trim().is_empty() {
            return Err("Preset intelligence signalKeywords cannot include empty keyword".into());
        }
        if entry.weight <= 0.0 {
            return Err(format!(
                "Preset intelligence signal keyword '{}' must have positive weight",
                entry.keyword
            ));
        }
    }

    Ok(())
}

pub fn merged_signal_keywords(preset: Option<&RolePreset>) -> Vec<(String, f64)> {
    let mut merged: HashMap<String, f64> = crate::signals::scoring::KEYWORD_WEIGHTS
        .iter()
        .map(|(keyword, weight)| ((*keyword).to_string(), *weight))
        .collect();

    if let Some(preset) = preset {
        for entry in &preset.intelligence.signal_keywords {
            let keyword = entry.keyword.to_lowercase();
            merged
                .entry(keyword)
                .and_modify(|weight| *weight = weight.max(entry.weight))
                .or_insert(entry.weight);
        }
    }

    let mut keywords: Vec<(String, f64)> = merged.into_iter().collect();
    keywords.sort_by(|a, b| a.0.cmp(&b.0));
    keywords
}

/// List all available embedded presets as (id, name, description).
pub fn get_available_presets() -> Vec<(String, String, String)> {
    embedded::list_embedded()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_preset_cs() {
        let preset = load_preset("customer-success").expect("should load CS preset");
        assert_eq!(preset.id, "customer-success");
        assert_eq!(preset.name, "Customer Success");
    }

    #[test]
    fn test_load_preset_unknown() {
        let result = load_preset("nonexistent");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown preset role"));
    }

    #[test]
    fn test_validate_preset_valid() {
        let preset = load_preset("customer-success").unwrap();
        assert!(validate_preset(&preset).is_ok());
    }

    #[test]
    fn test_validate_preset_empty_id() {
        let mut preset = load_preset("customer-success").unwrap();
        preset.id = String::new();
        assert!(validate_preset(&preset).is_err());
    }

    #[test]
    fn test_validate_preset_invalid_mode() {
        let mut preset = load_preset("customer-success").unwrap();
        preset.default_entity_mode = "invalid".to_string();
        assert!(validate_preset(&preset).is_err());
    }

    #[test]
    fn test_get_available_presets() {
        let presets = get_available_presets();
        assert_eq!(presets.len(), 10, "should have 10 embedded presets");
        let (id, name, desc) = &presets[0];
        assert_eq!(id, "customer-success");
        assert_eq!(name, "Customer Success");
        assert!(!desc.is_empty());
    }

    #[test]
    fn test_all_presets_load_and_validate() {
        let all_ids = [
            "customer-success",
            "sales",
            "marketing",
            "partnerships",
            "agency",
            "consulting",
            "product",
            "leadership",
            "the-desk",
            "affiliates",
        ];
        for id in all_ids {
            let preset =
                load_preset(id).unwrap_or_else(|e| panic!("Failed to load '{}': {}", id, e));
            validate_preset(&preset)
                .unwrap_or_else(|e| panic!("Validation failed for '{}': {}", id, e));
            assert_eq!(preset.id, id);
            assert!(
                !preset.name.is_empty(),
                "preset '{}' should have a name",
                id
            );
            assert!(
                !preset.description.is_empty(),
                "preset '{}' should have a description",
                id
            );
        }
    }

    #[test]
    fn test_all_presets_have_valid_intelligence_keys() {
        for (id, _, _) in get_available_presets() {
            let preset = load_preset(&id).unwrap();
            let mut keys: Vec<&str> = preset
                .intelligence
                .dimension_weights
                .keys()
                .map(String::as_str)
                .collect();
            keys.sort_unstable();
            let mut expected = INTELLIGENCE_DIMENSION_KEYS.to_vec();
            expected.sort_unstable();
            assert_eq!(keys, expected, "preset '{}' has invalid weights", id);
        }
    }

    #[test]
    fn test_validate_preset_rejects_missing_intelligence() {
        let mut preset = load_preset("customer-success").unwrap();
        preset.intelligence = PresetIntelligenceConfig::default();
        let err = validate_preset(&preset).unwrap_err();
        assert!(err.contains("systemRole"));
    }

    #[test]
    fn test_validate_preset_rejects_unknown_dimension_weight_key() {
        let mut preset = load_preset("customer-success").unwrap();
        preset
            .intelligence
            .dimension_weights
            .insert("unknown_dimension".to_string(), 0.1);
        let err = validate_preset(&preset).unwrap_err();
        assert!(err.contains("dimensionWeights keys"));
    }

    #[test]
    fn test_validate_preset_rejects_bad_dimension_weight_sum() {
        let mut preset = load_preset("customer-success").unwrap();
        preset
            .intelligence
            .dimension_weights
            .insert("meeting_cadence".to_string(), 0.5);
        let err = validate_preset(&preset).unwrap_err();
        assert!(err.contains("sum to 1.0"));
    }

    #[test]
    fn test_merged_signal_keywords_uses_max_weight_for_duplicates() {
        let preset = load_preset("customer-success").unwrap();
        let merged = merged_signal_keywords(Some(&preset));
        assert!(merged
            .iter()
            .any(|(keyword, weight)| keyword == "renewal" && *weight == 0.15));
        assert!(merged.iter().any(|(keyword, _)| keyword == "adoption"));
    }
}
