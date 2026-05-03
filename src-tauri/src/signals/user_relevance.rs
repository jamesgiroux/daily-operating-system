//! User-context-weighted signal scoring.
//!
//! Multiplies signal relevance by alignment with the user's declared priorities.
//! When no user entity is configured, returns a neutral weight of 1.0.

use crate::types::{AnnualPriority, QuarterlyPriority, UserEntity};

/// Compute a relevance weight for a signal based on alignment with user priorities.
///
/// Returns a multiplier in [1.0, 2.0]:
/// - 1.0 when no user entity or no priorities are set (neutral)
/// - 1.5–2.0 when the signal's entity is directly linked to a priority
/// - 1.2–1.5 when a priority text fuzzy-matches the entity name
/// - 1.0 baseline for entities with no priority alignment (neutral — no change from pre-v0.14.0 behavior)
pub fn compute_user_relevance_weight(
    signal_entity_id: &str,
    entity_name: &str,
    user_entity: Option<&UserEntity>,
) -> f64 {
    let user = match user_entity {
        Some(u) => u,
        None => return 1.0,
    };

    let annual: Vec<AnnualPriority> = user
        .annual_priorities
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();

    let quarterly: Vec<QuarterlyPriority> = user
        .quarterly_priorities
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();

    // No priorities declared → neutral weight
    if annual.is_empty() && quarterly.is_empty() {
        return 1.0;
    }

    // Check for direct entity link in quarterly priorities (strongest signal)
    for qp in &quarterly {
        if let Some(ref linked_id) = qp.linked_entity_id {
            if linked_id == signal_entity_id {
                return 2.0;
            }
        }
    }

    // Check for direct entity link in annual priorities
    for ap in &annual {
        if let Some(ref linked_id) = ap.linked_entity_id {
            if linked_id == signal_entity_id {
                return 1.75;
            }
        }
    }

    // Fuzzy text match: entity name appears in priority text (case-insensitive)
    let name_lower = entity_name.to_lowercase();
    if !name_lower.is_empty() {
        // Quarterly text match is stronger
        for qp in &quarterly {
            if qp.text.to_lowercase().contains(&name_lower) {
                return 1.5;
            }
        }
        for ap in &annual {
            if ap.text.to_lowercase().contains(&name_lower) {
                return 1.3;
            }
        }
    }

    // Entity has no alignment with any priority — mild demotion
    1.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_user(annual: &str, quarterly: &str) -> UserEntity {
        UserEntity {
            id: 1,
            name: Some("Test User".to_string()),
            company: None,
            title: None,
            focus: None,
            value_proposition: None,
            success_definition: None,
            current_priorities: None,
            product_context: None,
            playbooks: None,
            company_bio: None,
            role_description: None,
            how_im_measured: None,
            pricing_model: None,
            differentiators: None,
            objections: None,
            competitive_context: None,
            annual_priorities: if annual.is_empty() {
                None
            } else {
                Some(annual.to_string())
            },
            quarterly_priorities: if quarterly.is_empty() {
                None
            } else {
                Some(quarterly.to_string())
            },
            user_relevance_weight: None,
            created_at: String::new(),
            updated_at: String::new(),
        }
    }

    #[test]
    fn test_no_user_returns_neutral() {
        assert_eq!(compute_user_relevance_weight("e1", "Acme Corp", None), 1.0);
    }

    #[test]
    fn test_no_priorities_returns_neutral() {
        let user = make_user("", "");
        assert_eq!(
            compute_user_relevance_weight("e1", "Acme Corp", Some(&user)),
            1.0
        );
    }

    #[test]
    fn test_quarterly_direct_link() {
        let user = make_user(
            "[]",
            r#"[{"id":"q1","text":"Expand Acme","linkedEntityId":"e1","linkedEntityType":"account","createdAt":"2026-01-01"}]"#,
        );
        assert_eq!(
            compute_user_relevance_weight("e1", "Acme Corp", Some(&user)),
            2.0
        );
    }

    #[test]
    fn test_annual_direct_link() {
        let user = make_user(
            r#"[{"id":"a1","text":"Grow enterprise","linkedEntityId":"e1","linkedEntityType":"account","createdAt":"2026-01-01"}]"#,
            "[]",
        );
        assert_eq!(
            compute_user_relevance_weight("e1", "Acme Corp", Some(&user)),
            1.75
        );
    }

    #[test]
    fn test_quarterly_text_match() {
        let user = make_user(
            "[]",
            r#"[{"id":"q1","text":"Expand Acme Corp relationship","createdAt":"2026-01-01"}]"#,
        );
        assert_eq!(
            compute_user_relevance_weight("e2", "Acme Corp", Some(&user)),
            1.5
        );
    }

    #[test]
    fn test_annual_text_match() {
        let user = make_user(
            r#"[{"id":"a1","text":"Grow the Acme Corp account","createdAt":"2026-01-01"}]"#,
            "[]",
        );
        assert_eq!(
            compute_user_relevance_weight("e2", "Acme Corp", Some(&user)),
            1.3
        );
    }

    #[test]
    fn test_no_match_returns_neutral() {
        let user = make_user(
            r#"[{"id":"a1","text":"Build new product","createdAt":"2026-01-01"}]"#,
            r#"[{"id":"q1","text":"Close enterprise deals","createdAt":"2026-01-01"}]"#,
        );
        assert_eq!(
            compute_user_relevance_weight("e3", "Random Entity", Some(&user)),
            1.0
        );
    }
}
