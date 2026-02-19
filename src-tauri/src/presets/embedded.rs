const CS_PRESET: &str = include_str!("../../presets/customer-success.json");
const SALES_PRESET: &str = include_str!("../../presets/sales.json");
const MARKETING_PRESET: &str = include_str!("../../presets/marketing.json");
const PARTNERSHIPS_PRESET: &str = include_str!("../../presets/partnerships.json");
const AGENCY_PRESET: &str = include_str!("../../presets/agency.json");
const CONSULTING_PRESET: &str = include_str!("../../presets/consulting.json");
const PRODUCT_PRESET: &str = include_str!("../../presets/product.json");
const LEADERSHIP_PRESET: &str = include_str!("../../presets/leadership.json");
const DESK_PRESET: &str = include_str!("../../presets/the-desk.json");

/// All embedded presets in display order.
const ALL_PRESETS: &[(&str, &str)] = &[
    ("customer-success", CS_PRESET),
    ("sales", SALES_PRESET),
    ("marketing", MARKETING_PRESET),
    ("partnerships", PARTNERSHIPS_PRESET),
    ("agency", AGENCY_PRESET),
    ("consulting", CONSULTING_PRESET),
    ("product", PRODUCT_PRESET),
    ("leadership", LEADERSHIP_PRESET),
    ("the-desk", DESK_PRESET),
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
