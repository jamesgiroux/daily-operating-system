//! Stakeholder–Person reconciliation (I420).
//!
//! After AI enrichment produces `stakeholderInsights` with informal names,
//! this module matches them against linked Person entities using fuzzy name
//! matching. High-confidence matches (≥0.85) get a deterministic `person_id`;
//! medium-confidence matches (0.6–0.85) get a `suggested_person_id` for user
//! confirmation. User-edited stakeholders are never overwritten.

use super::io::{StakeholderInsight, UserEdit};
use crate::db::DbPerson;

/// Reconcile stakeholder insights against linked Person entities.
///
/// For each stakeholder:
/// 1. If `person_id` is already set, skip (previous reconciliation or user confirmation).
/// 2. If the stakeholder's name path is user-edited, skip.
/// 3. Score against each linked person using Jaro-Winkler + role bonus.
/// 4. Best score ≥ 0.85 → set `person_id` and replace name with canonical name.
/// 5. Best score 0.6–0.85 → set `suggested_person_id`.
/// 6. Best score < 0.6 → leave unlinked.
pub fn reconcile_stakeholders(
    stakeholders: &mut [StakeholderInsight],
    linked_people: &[DbPerson],
    user_edits: &[UserEdit],
) {
    if linked_people.is_empty() {
        return;
    }

    for (idx, stakeholder) in stakeholders.iter_mut().enumerate() {
        // Already reconciled — don't overwrite
        if stakeholder.person_id.is_some() {
            continue;
        }

        // User manually edited this stakeholder's name — don't touch it
        let name_path = format!("stakeholderInsights[{}].name", idx);
        if user_edits.iter().any(|e| e.field_path == name_path) {
            continue;
        }

        // Also skip if the entire stakeholderInsights array is user-edited
        if user_edits
            .iter()
            .any(|e| e.field_path == "stakeholderInsights")
        {
            continue;
        }

        let (best_person, best_score) = find_best_match(stakeholder, linked_people);

        if let Some(person) = best_person {
            if best_score >= 0.85 {
                stakeholder.person_id = Some(person.id.clone());
                stakeholder.name = person.name.clone();
                stakeholder.suggested_person_id = None;
                log::debug!(
                    "I420: auto-linked stakeholder '{}' → person '{}' (score: {:.3})",
                    stakeholder.name,
                    person.id,
                    best_score,
                );
            } else if best_score >= 0.6 {
                stakeholder.suggested_person_id = Some(person.id.clone());
                log::debug!(
                    "I420: suggested link for '{}' → person '{}' (score: {:.3})",
                    stakeholder.name,
                    person.name,
                    best_score,
                );
            }
        }
    }
}

/// Find the best-matching person for a stakeholder.
/// Returns (Option<&DbPerson>, score).
fn find_best_match<'a>(
    stakeholder: &StakeholderInsight,
    linked_people: &'a [DbPerson],
) -> (Option<&'a DbPerson>, f64) {
    let stakeholder_name = stakeholder.name.to_lowercase();
    let mut best_person: Option<&DbPerson> = None;
    let mut best_score: f64 = 0.0;

    for person in linked_people {
        let person_name = person.name.to_lowercase();

        // Base score: Jaro-Winkler similarity
        let mut score = strsim::jaro_winkler(&stakeholder_name, &person_name);

        // Role match bonus: +0.1 if both have roles and they're similar
        if let (Some(ref s_role), Some(ref p_role)) = (&stakeholder.role, &person.role) {
            let role_sim = strsim::jaro_winkler(&s_role.to_lowercase(), &p_role.to_lowercase());
            if role_sim >= 0.7 {
                score += 0.1;
            }
        }

        // Meeting count bonus: +0.05 if the person has meetings with this account
        if person.meeting_count > 0 {
            score += 0.05;
        }

        // Cap at 1.0
        score = score.min(1.0);

        if score > best_score {
            best_score = score;
            best_person = Some(person);
        }
    }

    (best_person, best_score)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_person(id: &str, name: &str, role: Option<&str>, meetings: i32) -> DbPerson {
        DbPerson {
            id: id.to_string(),
            email: format!("{}@example.com", id),
            name: name.to_string(),
            organization: None,
            role: role.map(|r| r.to_string()),
            relationship: "external".to_string(),
            notes: None,
            tracker_path: None,
            last_seen: None,
            first_seen: None,
            meeting_count: meetings,
            updated_at: "2026-01-01T00:00:00Z".to_string(),
            archived: false,
            linkedin_url: None,
            twitter_handle: None,
            phone: None,
            photo_url: None,
            bio: None,
            title_history: None,
            company_industry: None,
            company_size: None,
            company_hq: None,
            last_enriched_at: None,
            enrichment_sources: None,
        }
    }

    fn make_stakeholder(name: &str, role: Option<&str>) -> StakeholderInsight {
        StakeholderInsight {
            name: name.to_string(),
            role: role.map(|r| r.to_string()),
            assessment: None,
            engagement: None,
            source: None,
            person_id: None,
            suggested_person_id: None,
            item_source: None,
            discrepancy: None,
        }
    }

    #[test]
    fn test_exact_name_match_links() {
        let people = vec![make_person("p1", "James Giroux", Some("CSM"), 5)];
        let mut stakeholders = vec![make_stakeholder("James Giroux", Some("CSM"))];

        reconcile_stakeholders(&mut stakeholders, &people, &[]);

        assert_eq!(stakeholders[0].person_id.as_deref(), Some("p1"));
        assert!(stakeholders[0].suggested_person_id.is_none());
        assert_eq!(stakeholders[0].name, "James Giroux");
    }

    #[test]
    fn test_fuzzy_name_match_links() {
        let people = vec![make_person("p1", "James Giroux", None, 3)];
        let mut stakeholders = vec![make_stakeholder("James G.", None)];

        reconcile_stakeholders(&mut stakeholders, &people, &[]);

        // "James G." vs "James Giroux" — Jaro-Winkler should be high enough
        // with meeting bonus to cross 0.6 at minimum
        let has_link =
            stakeholders[0].person_id.is_some() || stakeholders[0].suggested_person_id.is_some();
        assert!(has_link, "Should have either a link or suggestion");
    }

    #[test]
    fn test_no_match_for_unrelated_name() {
        let people = vec![make_person("p1", "Alice Chen", None, 2)];
        let mut stakeholders = vec![make_stakeholder("Bob Smith", None)];

        reconcile_stakeholders(&mut stakeholders, &people, &[]);

        assert!(stakeholders[0].person_id.is_none());
        assert!(stakeholders[0].suggested_person_id.is_none());
    }

    #[test]
    fn test_skips_already_linked() {
        let people = vec![make_person("p1", "Alice Chen", None, 2)];
        let mut stakeholders = vec![StakeholderInsight {
            name: "Ali Chen".to_string(),
            role: None,
            assessment: None,
            engagement: None,
            source: None,
            person_id: Some("p2".to_string()),
            suggested_person_id: None,
            item_source: None,
            discrepancy: None,
        }];

        reconcile_stakeholders(&mut stakeholders, &people, &[]);

        // Should keep existing person_id, not overwrite
        assert_eq!(stakeholders[0].person_id.as_deref(), Some("p2"));
    }

    #[test]
    fn test_skips_user_edited_name() {
        let people = vec![make_person("p1", "James Giroux", None, 5)];
        let mut stakeholders = vec![make_stakeholder("Jim G", None)];
        let edits = vec![UserEdit {
            field_path: "stakeholderInsights[0].name".to_string(),
            edited_at: "2026-02-22T00:00:00Z".to_string(),
        }];

        reconcile_stakeholders(&mut stakeholders, &people, &edits);

        // User edited this name — should not be reconciled
        assert!(stakeholders[0].person_id.is_none());
        assert!(stakeholders[0].suggested_person_id.is_none());
    }

    #[test]
    fn test_skips_when_whole_array_user_edited() {
        let people = vec![make_person("p1", "James Giroux", None, 5)];
        let mut stakeholders = vec![make_stakeholder("James Giroux", None)];
        let edits = vec![UserEdit {
            field_path: "stakeholderInsights".to_string(),
            edited_at: "2026-02-22T00:00:00Z".to_string(),
        }];

        reconcile_stakeholders(&mut stakeholders, &people, &edits);

        assert!(stakeholders[0].person_id.is_none());
    }

    #[test]
    fn test_role_bonus_helps_match() {
        let people = vec![
            make_person("p1", "Alex Kim", Some("VP Engineering"), 3),
            make_person("p2", "Alex Kim", Some("Sales Rep"), 1),
        ];
        let mut stakeholders = vec![make_stakeholder("Alex Kim", Some("VP Engineering"))];

        reconcile_stakeholders(&mut stakeholders, &people, &[]);

        // Both have the same name, but p1 has the matching role
        assert_eq!(stakeholders[0].person_id.as_deref(), Some("p1"));
    }

    #[test]
    fn test_empty_people_is_noop() {
        let mut stakeholders = vec![make_stakeholder("Alice", None)];

        reconcile_stakeholders(&mut stakeholders, &[], &[]);

        assert!(stakeholders[0].person_id.is_none());
        assert!(stakeholders[0].suggested_person_id.is_none());
    }

    #[test]
    fn test_replaces_name_with_canonical_on_auto_link() {
        let people = vec![make_person("p1", "James Giroux", None, 5)];
        let mut stakeholders = vec![make_stakeholder("james giroux", None)];

        reconcile_stakeholders(&mut stakeholders, &people, &[]);

        // Name should be replaced with the canonical form
        assert_eq!(stakeholders[0].name, "James Giroux");
        assert_eq!(stakeholders[0].person_id.as_deref(), Some("p1"));
    }
}
