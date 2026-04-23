const CORE_PRESET: &str = include_str!("../../presets/core.json");
const CS_PRESET: &str = include_str!("../../presets/customer-success.json");
const AFFILIATES_PARTNERSHIPS_PRESET: &str =
    include_str!("../../presets/affiliates-partnerships.json");
const PRODUCT_MARKETING_PRESET: &str = include_str!("../../presets/product-marketing.json");

/// All embedded presets in display order.
const ALL_PRESETS: &[(&str, &str)] = &[
    ("core", CORE_PRESET),
    ("customer-success", CS_PRESET),
    ("affiliates-partnerships", AFFILIATES_PARTNERSHIPS_PRESET),
    ("product-marketing", PRODUCT_MARKETING_PRESET),
];

/// Look up an embedded preset by role ID.
pub fn get_embedded(role: &str) -> Option<&'static str> {
    ALL_PRESETS
        .iter()
        .find(|(id, _)| *id == role)
        .map(|(_, json)| *json)
}

/// List all embedded presets as (id, name, description).
pub fn list_embedded() -> Vec<(String, String, String)> {
    let mut result = Vec::new();
    for (role, json) in ALL_PRESETS {
        if let Ok(preset) = serde_json::from_str::<super::schema::RolePreset>(json) {
            result.push((preset.id, preset.name, preset.description));
        } else {
            result.push((role.to_string(), role.to_string(), String::new()));
        }
    }
    result
}
