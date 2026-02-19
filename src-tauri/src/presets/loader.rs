use super::embedded;
use super::schema::RolePreset;

/// Load an embedded preset by role ID.
pub fn load_preset(role: &str) -> Result<RolePreset, String> {
    if let Some(json) = embedded::get_embedded(role) {
        return serde_json::from_str(json)
            .map_err(|e| format!("Failed to parse embedded preset '{}': {}", role, e));
    }
    Err(format!("Unknown preset role: {}", role))
}

/// Load a custom preset from a file path.
pub fn load_custom_preset(path: &std::path::Path) -> Result<RolePreset, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read preset file: {}", e))?;
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
    Ok(())
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
        assert_eq!(presets.len(), 9, "should have 9 embedded presets");
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
        ];
        for id in all_ids {
            let preset = load_preset(id).unwrap_or_else(|e| panic!("Failed to load '{}': {}", id, e));
            validate_preset(&preset).unwrap_or_else(|e| panic!("Validation failed for '{}': {}", id, e));
            assert_eq!(preset.id, id);
            assert!(!preset.name.is_empty(), "preset '{}' should have a name", id);
            assert!(!preset.description.is_empty(), "preset '{}' should have a description", id);
        }
    }
}
