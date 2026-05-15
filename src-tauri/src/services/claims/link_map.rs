//! Declarative claim field -> entity edge compiler.
//!
//! This module is intentionally narrow: it translates configured claim
//! frontmatter-style fields into `claim_edges` rows. It does not add claim
//! types or mutate operational entity-linking tables.

use std::collections::HashSet;

use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

use crate::abilities::claims::CanonicalSubjectType;
use crate::db::claim_invalidation::SubjectRef;
use crate::db::claims::{ClaimState, IntelligenceClaim, SurfacingState};
use crate::services::claims::subject_ref_from_json;

pub const LINK_SOURCE_FRONTMATTER_MAP: &str = "frontmatter_map";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeDirection {
    Forward,
    Incoming,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LinkRule {
    pub field: &'static str,
    pub edge_type: &'static str,
    pub direction: EdgeDirection,
    pub fanout: bool,
    pub subject_type: CanonicalSubjectType,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClaimEdge {
    pub id: String,
    pub from_entity_id: String,
    pub to_entity_id: String,
    pub edge_type: String,
    pub origin_claim_id: String,
    pub link_source: &'static str,
    pub weight: f64,
    pub confidence: f64,
}

pub trait ClaimLinkMap {
    fn rules(&self) -> &'static [LinkRule];
}

pub struct FrontmatterClaimLinkMap;

impl ClaimLinkMap for FrontmatterClaimLinkMap {
    fn rules(&self) -> &'static [LinkRule] {
        CLAIM_LINK_MAP
    }
}

crate::frontmatter_link_map! {
    /// Declarative claim field -> edge type rules for frontmatter-style fields.
    pub const CLAIM_LINK_MAP = [
        {
            field: "account",
            edge_type: "mentions_account",
            direction: EdgeDirection::Forward,
            fanout: false,
            subject_type: CanonicalSubjectType::Meeting,
        },
        {
            field: "project",
            edge_type: "mentions_project",
            direction: EdgeDirection::Forward,
            fanout: false,
            subject_type: CanonicalSubjectType::Meeting,
        },
        {
            field: "stakeholders",
            edge_type: "has_stakeholder",
            direction: EdgeDirection::Forward,
            fanout: true,
            subject_type: CanonicalSubjectType::Account,
        },
        {
            field: "linked_entities",
            edge_type: "has_stakeholder",
            direction: EdgeDirection::Incoming,
            fanout: true,
            subject_type: CanonicalSubjectType::Person,
        },
    ];
}

pub const FRONTMATTER_LINK_MAP: &[LinkRule] = CLAIM_LINK_MAP;

pub fn compile_edges_from_claim(claim: &IntelligenceClaim) -> Vec<ClaimEdge> {
    if claim.claim_state != ClaimState::Active || claim.surfacing_state != SurfacingState::Active {
        return Vec::new();
    }

    let Some(field_path) = claim.field_path.as_deref() else {
        return Vec::new();
    };
    let Some(rule) = CLAIM_LINK_MAP.iter().find(|rule| rule.field == field_path) else {
        return Vec::new();
    };

    let Ok(subject_value) = serde_json::from_str::<Value>(&claim.subject_ref) else {
        return Vec::new();
    };
    let Ok(subject) = subject_ref_from_json(&subject_value) else {
        return Vec::new();
    };
    let Some((subject_type, subject_id)) = subject_type_and_id(&subject) else {
        return Vec::new();
    };
    if subject_type != rule.subject_type {
        return Vec::new();
    }

    let mut targets = target_entity_ids_for_claim(claim, rule.field);
    if !rule.fanout {
        targets.truncate(1);
    }

    let mut seen = HashSet::new();
    targets
        .into_iter()
        .filter(|target_id| !target_id.is_empty() && target_id != &subject_id)
        .filter_map(|target_id| {
            let (from_entity_id, to_entity_id) = match rule.direction {
                EdgeDirection::Forward => (subject_id.clone(), target_id),
                EdgeDirection::Incoming => (target_id, subject_id.clone()),
            };
            let key = (
                from_entity_id.clone(),
                to_entity_id.clone(),
                rule.edge_type.to_string(),
            );
            if !seen.insert(key) {
                return None;
            }

            let edge_type = rule.edge_type.to_string();
            Some(ClaimEdge {
                id: edge_id(&claim.id, &from_entity_id, &to_entity_id, &edge_type),
                from_entity_id,
                to_entity_id,
                edge_type,
                origin_claim_id: claim.id.clone(),
                link_source: LINK_SOURCE_FRONTMATTER_MAP,
                weight: 1.0,
                confidence: claim.trust_score.unwrap_or(1.0),
            })
        })
        .collect()
}

pub(crate) fn metadata_with_structured_field(
    metadata_json: Option<&str>,
    field_path: Option<&str>,
    original_text: &str,
) -> Option<String> {
    let Some(field_path) = field_path else {
        return metadata_json.map(ToOwned::to_owned);
    };
    if !CLAIM_LINK_MAP.iter().any(|rule| rule.field == field_path) {
        return metadata_json.map(ToOwned::to_owned);
    }

    let mut root = metadata_json
        .and_then(|raw| serde_json::from_str::<Value>(raw).ok())
        .and_then(|value| match value {
            Value::Object(map) => Some(map),
            _ => None,
        })
        .unwrap_or_default();
    let field_value = serde_json::from_str::<Value>(original_text)
        .unwrap_or_else(|_| Value::String(original_text.to_string()));

    match root.get_mut("fields") {
        Some(Value::Object(fields)) => {
            fields.entry(field_path.to_string()).or_insert(field_value);
        }
        _ => {
            let mut fields = Map::new();
            fields.insert(field_path.to_string(), field_value);
            root.insert("fields".to_string(), Value::Object(fields));
        }
    }
    root.entry("original_text".to_string())
        .or_insert_with(|| Value::String(original_text.to_string()));

    Some(Value::Object(root).to_string())
}

fn subject_type_and_id(subject: &SubjectRef) -> Option<(CanonicalSubjectType, String)> {
    match subject {
        SubjectRef::Account { id } => Some((CanonicalSubjectType::Account, id.clone())),
        SubjectRef::Meeting { id } => Some((CanonicalSubjectType::Meeting, id.clone())),
        SubjectRef::Person { id } => Some((CanonicalSubjectType::Person, id.clone())),
        SubjectRef::Project { id } => Some((CanonicalSubjectType::Project, id.clone())),
        SubjectRef::Email { id } => Some((CanonicalSubjectType::Email, id.clone())),
        SubjectRef::Multi(_) | SubjectRef::Global => None,
    }
}

fn target_entity_ids_for_claim(claim: &IntelligenceClaim, field: &str) -> Vec<String> {
    claim
        .metadata_json
        .as_deref()
        .and_then(|metadata| target_entity_ids_from_metadata(metadata, field))
        .unwrap_or_else(|| target_entity_ids(&claim.text))
}

fn target_entity_ids_from_metadata(metadata_json: &str, field: &str) -> Option<Vec<String>> {
    let metadata = serde_json::from_str::<Value>(metadata_json).ok()?;
    let Value::Object(root) = metadata else {
        return None;
    };

    if let Some(Value::Object(fields)) = root.get("fields") {
        if let Some(value) = fields.get(field) {
            let targets = target_entity_ids_from_value(value);
            if !targets.is_empty() {
                return Some(targets);
            }
        }
    }

    if let Some(value) = root.get(field) {
        let targets = target_entity_ids_from_value(value);
        if !targets.is_empty() {
            return Some(targets);
        }
    }

    root.get("original_text")
        .map(target_entity_ids_from_value)
        .filter(|targets| !targets.is_empty())
}

fn target_entity_ids(raw: &str) -> Vec<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
        let mut out = Vec::new();
        collect_target_ids(&value, &mut out);
        return stable_dedup(out);
    }

    stable_dedup(
        trimmed
            .split([',', '\n', ';'])
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .map(ToString::to_string)
            .collect(),
    )
}

fn target_entity_ids_from_value(value: &Value) -> Vec<String> {
    match value {
        Value::String(raw) => target_entity_ids(raw),
        _ => {
            let mut out = Vec::new();
            collect_target_ids(value, &mut out);
            stable_dedup(out)
        }
    }
}

fn collect_target_ids(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::String(s) if !s.trim().is_empty() => out.push(s.trim().to_string()),
        Value::Array(items) => {
            for item in items {
                collect_target_ids(item, out);
            }
        }
        Value::Object(map) => {
            for key in [
                "id",
                "entity_id",
                "account_id",
                "project_id",
                "person_id",
                "meeting_id",
            ] {
                if let Some(Value::String(s)) = map.get(key) {
                    if !s.trim().is_empty() {
                        out.push(s.trim().to_string());
                    }
                }
            }

            for key in [
                "ids",
                "entity_ids",
                "linked_entities",
                "accounts",
                "projects",
                "people",
                "stakeholders",
                "meetings",
                "value",
            ] {
                if let Some(nested) = map.get(key) {
                    collect_target_ids(nested, out);
                }
            }
        }
        _ => {}
    }
}

fn stable_dedup(values: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    values
        .into_iter()
        .filter(|value| seen.insert(value.clone()))
        .collect()
}

fn edge_id(
    origin_claim_id: &str,
    from_entity_id: &str,
    to_entity_id: &str,
    edge_type: &str,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(origin_claim_id.as_bytes());
    hasher.update(b"\0");
    hasher.update(from_entity_id.as_bytes());
    hasher.update(b"\0");
    hasher.update(to_entity_id.as_bytes());
    hasher.update(b"\0");
    hasher.update(edge_type.as_bytes());
    format!("ce-{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abilities::feedback::ClaimVerificationState;
    use crate::db::claims::{ClaimSensitivity, TemporalScope};

    fn subject_for(subject_type: CanonicalSubjectType, id: &str) -> String {
        let kind = subject_type.as_str();
        serde_json::json!({ "kind": kind, "id": id }).to_string()
    }

    fn fixture_claim(rule: &LinkRule, text: &str) -> IntelligenceClaim {
        IntelligenceClaim {
            id: format!("claim-{}", rule.field),
            claim_version: 1,
            subject_ref: subject_for(rule.subject_type, "subject-1"),
            claim_type: "risk".to_string(),
            field_path: Some(rule.field.to_string()),
            topic_key: None,
            text: text.to_string(),
            dedup_key: "dedup".to_string(),
            item_hash: Some("hash".to_string()),
            actor: "agent:test".to_string(),
            data_source: "unit_test".to_string(),
            source_ref: None,
            source_asof: None,
            observed_at: "2026-05-02T12:00:00+00:00".to_string(),
            created_at: "2026-05-02T12:00:00+00:00".to_string(),
            provenance_json: "{}".to_string(),
            metadata_json: None,
            claim_state: ClaimState::Active,
            surfacing_state: SurfacingState::Active,
            demotion_reason: None,
            reactivated_at: None,
            retraction_reason: None,
            expires_at: None,
            superseded_by: None,
            trust_score: Some(0.73),
            trust_computed_at: None,
            trust_version: None,
            thread_id: None,
            temporal_scope: TemporalScope::State,
            sensitivity: ClaimSensitivity::Internal,
            verification_state: ClaimVerificationState::Active,
            verification_reason: None,
            needs_user_decision_at: None,
        }
    }

    #[test]
    fn claim_link_map_direction_property_holds_for_each_entry() {
        assert!(!CLAIM_LINK_MAP.is_empty(), "link map must not be vacuous");

        for rule in CLAIM_LINK_MAP {
            let claim = fixture_claim(rule, r#"["target-1"]"#);
            let edges = compile_edges_from_claim(&claim);
            assert_eq!(edges.len(), 1, "expected one edge for {rule:?}");
            let edge = &edges[0];
            assert_eq!(edge.origin_claim_id, claim.id);
            assert_eq!(edge.edge_type, rule.edge_type);
            assert_eq!(edge.link_source, LINK_SOURCE_FRONTMATTER_MAP);

            match rule.direction {
                EdgeDirection::Forward => {
                    assert_eq!(edge.from_entity_id, "subject-1");
                    assert_eq!(edge.to_entity_id, "target-1");
                }
                EdgeDirection::Incoming => {
                    assert_eq!(edge.from_entity_id, "target-1");
                    assert_eq!(edge.to_entity_id, "subject-1");
                }
            }
        }
    }

    #[test]
    fn subject_type_filter_rejects_wrong_subject_kind() {
        let rule = CLAIM_LINK_MAP
            .iter()
            .find(|rule| rule.subject_type != CanonicalSubjectType::Email)
            .unwrap();
        let mut claim = fixture_claim(rule, r#"["target-1"]"#);
        claim.subject_ref = subject_for(CanonicalSubjectType::Email, "email-1");

        assert!(compile_edges_from_claim(&claim).is_empty());
    }

    #[test]
    fn fanout_controls_target_cardinality() {
        let fanout_rule = CLAIM_LINK_MAP.iter().find(|rule| rule.fanout).unwrap();
        let single_rule = CLAIM_LINK_MAP.iter().find(|rule| !rule.fanout).unwrap();

        assert_eq!(
            compile_edges_from_claim(&fixture_claim(fanout_rule, r#"["a","b","a"]"#)).len(),
            2
        );
        assert_eq!(
            compile_edges_from_claim(&fixture_claim(single_rule, r#"["a","b"]"#)).len(),
            1
        );
    }

    #[test]
    fn target_ids_prefer_structured_fields_and_preserve_case() {
        let rule = CLAIM_LINK_MAP
            .iter()
            .find(|rule| rule.field == "stakeholders")
            .unwrap();
        let mut fields = Map::new();
        fields.insert(
            rule.field.to_string(),
            serde_json::json!(["Person-MixedCase"]),
        );
        let mut root = Map::new();
        root.insert("fields".to_string(), Value::Object(fields));

        let mut claim = fixture_claim(rule, r#"["person-mixedcase"]"#);
        claim.metadata_json = Some(Value::Object(root).to_string());

        let edges = compile_edges_from_claim(&claim);
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].to_entity_id, "Person-MixedCase");
    }

    #[test]
    fn target_ids_fall_back_to_original_text_when_structured_field_missing() {
        let rule = CLAIM_LINK_MAP
            .iter()
            .find(|rule| rule.field == "stakeholders")
            .unwrap();
        let mut claim = fixture_claim(rule, r#"["person-fallback"]"#);
        claim.metadata_json = Some(
            serde_json::json!({
                "fields": {},
                "original_text": r#"["Person-Fallback"]"#
            })
            .to_string(),
        );

        let edges = compile_edges_from_claim(&claim);
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].to_entity_id, "Person-Fallback");
    }

    #[test]
    fn claim_link_map_trait_exposes_frontmatter_rules() {
        let provider = FrontmatterClaimLinkMap;
        assert_eq!(provider.rules(), CLAIM_LINK_MAP);
        assert_eq!(FRONTMATTER_LINK_MAP, CLAIM_LINK_MAP);
    }
}
