//! General-purpose item relevance scorer (I395).
//!
//! Composes existing signal infrastructure — entity linkage counts, embedding
//! cosine similarity, AI urgency mapping, keyword matching, and time decay —
//! to compute a normalized 0.0–1.0 relevance score for any item.

use crate::db::ActionDb;
use crate::embeddings::EmbeddingModel;
use crate::signals::decay;

/// Context needed to score a single item.
pub struct ScoringContext<'a> {
    pub entity_id: Option<&'a str>,
    pub entity_type: Option<&'a str>,
    pub content_text: &'a str,
    pub urgency: Option<&'a str>,
    pub sentiment: Option<&'a str>,
    pub created_at: &'a str,
}

/// Result of scoring a single item.
pub struct ScoredItem {
    pub total: f64,
    pub entity_score: f64,
    pub relevance_score: f64,
    pub urgency_score: f64,
    pub keyword_score: f64,
    pub recency_score: f64,
    pub reason: String,
}

/// Score a single item against today's context.
///
/// `merged_signal_keywords` is the pre-merged list of (keyword, weight) pairs from
/// `AppState::get_merged_signal_config()`. Callers are responsible for passing the
/// correct merged list — this function does not reach into global state (DOS-176).
///
/// Scoring dimensions:
/// - Entity linkage (0.0–0.30): signal_events count for entity
/// - Meeting relevance (0.0–0.25): entity must have a meeting today + embedding similarity
/// - AI urgency (0.0–0.20): mapped from enrichment urgency field
/// - Keyword relevance (0.0–0.15): business term matching
/// - Recency (0.0–0.10): exponential decay with 14-day half-life
pub fn score_item(
    db: &ActionDb,
    model: Option<&EmbeddingModel>,
    ctx: &ScoringContext,
    todays_meeting_context: &str,
    merged_signal_keywords: &[(String, f64)],
) -> ScoredItem {
    let mut reasons = Vec::new();

    // 1. Entity linkage (0.0–0.30)
    let entity_score = if let (Some(eid), Some(etype)) = (ctx.entity_id, ctx.entity_type) {
        let signal_count = count_entity_signals(db, eid, etype);
        let score = match signal_count {
            0 => 0.10, // Entity exists but no updates yet
            1..=3 => 0.20,
            _ => 0.30,
        };
        // Resolve entity name for product-vocabulary reason string (ADR-0083)
        let entity_label = resolve_entity_name(db, eid, etype);
        reasons.push(entity_label);
        score
    } else {
        0.0
    };

    // 2. Meeting relevance (0.0–0.25) — entity must have a meeting today (I449)
    let relevance_score = if !todays_meeting_context.is_empty() && !ctx.content_text.is_empty() {
        // Only claim meeting relevance if the email's entity actually has a meeting today
        let entity_has_meeting = ctx
            .entity_id
            .map(|eid| entity_has_meeting_today(db, eid))
            .unwrap_or(false);

        if entity_has_meeting {
            if let Some(m) = model {
                let sim = compute_embedding_similarity(m, ctx.content_text, todays_meeting_context);
                let score = (sim.max(0.0) * 0.25).min(0.25);
                if score > 0.15 {
                    reasons.push("relates to today's meetings".to_string());
                }
                score
            } else {
                // No embedding model, but entity has a meeting — give base score
                reasons.push("relates to today's meetings".to_string());
                0.10
            }
        } else {
            0.0
        }
    } else {
        0.0
    };

    // 3. AI urgency (0.0–0.20)
    let urgency_score = match ctx.urgency {
        Some("high") => {
            reasons.push("urgent".to_string());
            0.20
        }
        Some("medium") => 0.08,
        Some("low") => 0.02,
        _ => 0.0,
    };

    // 4. Keyword relevance (0.0–0.15)
    let keyword_score = if !ctx.content_text.is_empty() {
        let lower = ctx.content_text.to_lowercase();
        let mut best_score = 0.0_f64;
        let mut best_keyword = String::new();
        for (keyword, weight) in merged_signal_keywords {
            if lower.contains(keyword.as_str()) && *weight > best_score {
                best_score = *weight;
                best_keyword = keyword.clone();
            }
        }
        if best_score > 0.0 {
            // Product vocabulary (ADR-0083): just the topic, not "X keyword"
            reasons.push(best_keyword);
        }
        best_score
    } else {
        0.0
    };

    // 5. Recency (0.0–0.10) — 14-day half-life decay
    let age = decay::age_days_from_now(ctx.created_at);
    let recency_score = decay::decayed_weight(0.10, age, 14.0);

    // Total clamped to [0.0, 1.0]
    let total = (entity_score + relevance_score + urgency_score + keyword_score + recency_score)
        .clamp(0.0, 1.0);

    let reason = if reasons.is_empty() {
        String::new()
    } else {
        reasons.join(" · ")
    };

    ScoredItem {
        total,
        entity_score,
        relevance_score,
        urgency_score,
        keyword_score,
        recency_score,
        reason,
    }
}

/// Check whether an entity has a meeting scheduled today (I449).
fn entity_has_meeting_today(db: &ActionDb, entity_id: &str) -> bool {
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let start = format!("{}T00:00:00", today);
    let end = format!("{}T23:59:59", today);

    db.conn_ref()
        .query_row(
            "SELECT COUNT(*) FROM meeting_entities me
             JOIN meetings mh ON me.meeting_id = mh.id
             WHERE me.entity_id = ?1 AND mh.start_time >= ?2 AND mh.start_time <= ?3",
            rusqlite::params![entity_id, start, end],
            |row| row.get::<_, i64>(0),
        )
        .unwrap_or(0)
        > 0
}

/// Count signal_events for a given entity.
fn count_entity_signals(db: &ActionDb, entity_id: &str, entity_type: &str) -> i64 {
    db.conn_ref()
        .query_row(
            "SELECT COUNT(*) FROM signal_events WHERE entity_id = ?1 AND entity_type = ?2",
            rusqlite::params![entity_id, entity_type],
            |row| row.get(0),
        )
        .unwrap_or(0)
}

/// Compute cosine similarity between two texts using the embedding model.
fn compute_embedding_similarity(model: &EmbeddingModel, text_a: &str, text_b: &str) -> f64 {
    let prefix_d = "search_document: ";
    let prefix_q = "search_query: ";

    let vec_a = match model.embed(&format!("{}{}", prefix_d, text_a)) {
        Ok(v) => v,
        Err(_) => return 0.0,
    };
    let vec_b = match model.embed(&format!("{}{}", prefix_q, text_b)) {
        Ok(v) => v,
        Err(_) => return 0.0,
    };

    cosine_similarity(&vec_a, &vec_b)
}

/// Cosine similarity between two vectors.
fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    (dot / (norm_a * norm_b)) as f64
}

/// Resolve an entity ID to a human-readable name (ADR-0083: product vocabulary).
/// Falls back to the entity type label if name lookup fails.
fn resolve_entity_name(db: &ActionDb, entity_id: &str, entity_type: &str) -> String {
    match entity_type {
        "account" => db
            .get_account(entity_id)
            .ok()
            .flatten()
            .map(|a| a.name)
            .unwrap_or_else(|| "known account".to_string()),
        "person" => db
            .get_person(entity_id)
            .ok()
            .flatten()
            .map(|p| p.name)
            .unwrap_or_else(|| "known contact".to_string()),
        "project" => db
            .get_project(entity_id)
            .ok()
            .flatten()
            .map(|p| p.name)
            .unwrap_or_else(|| "known project".to_string()),
        _ => "known contact".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::BASE_SIGNAL_KEYWORDS;

    /// Build a base keyword list from the module-level constant for test use.
    fn base_keywords() -> Vec<(String, f64)> {
        BASE_SIGNAL_KEYWORDS
            .iter()
            .map(|&(k, w)| (k.to_string(), w))
            .collect()
    }

    #[test]
    fn test_keyword_matching() {
        let db = crate::db::test_utils::test_db();
        let ctx = ScoringContext {
            entity_id: None,
            entity_type: None,
            content_text: "Discussing the renewal timeline for Q3",
            urgency: None,
            sentiment: None,
            created_at: &chrono::Utc::now().to_rfc3339(),
        };
        let kws = base_keywords();
        let result = score_item(&db, None, &ctx, "", &kws);
        assert!(
            result.keyword_score > 0.0,
            "renewal keyword should score > 0"
        );
        assert!(result.reason.contains("renewal"));
    }

    #[test]
    fn test_urgency_scoring() {
        let db = crate::db::test_utils::test_db();
        let ctx = ScoringContext {
            entity_id: None,
            entity_type: None,
            content_text: "",
            urgency: Some("high"),
            sentiment: None,
            created_at: &chrono::Utc::now().to_rfc3339(),
        };
        let kws = base_keywords();
        let result = score_item(&db, None, &ctx, "", &kws);
        assert!((result.urgency_score - 0.20).abs() < 0.001);
    }

    #[test]
    fn test_total_clamped() {
        let db = crate::db::test_utils::test_db();
        let ctx = ScoringContext {
            entity_id: None,
            entity_type: None,
            content_text: "renewal contract expansion escalation",
            urgency: Some("high"),
            sentiment: None,
            created_at: &chrono::Utc::now().to_rfc3339(),
        };
        let kws = base_keywords();
        let result = score_item(&db, None, &ctx, "", &kws);
        assert!(result.total <= 1.0, "total should be clamped to 1.0");
        assert!(result.total >= 0.0);
    }

    // =========================================================================
    // DOS-176: Preset-aware signal keyword tests
    // =========================================================================

    /// Build a merged keyword list from a preset (mirrors the runtime path).
    fn merged_keywords_for_preset(role: &str) -> Vec<(String, f64)> {
        let preset = crate::presets::loader::load_preset(role)
            .unwrap_or_else(|e| panic!("Failed to load '{}' preset: {}", role, e));
        crate::state::build_merged_signal_config(&preset).signal_keywords
    }

    #[test]
    fn dos176_cs_preset_churn_in_merged_keywords() {
        let kws = merged_keywords_for_preset("customer-success");
        let churn = kws.iter().find(|(k, _)| k == "churn");
        assert!(
            churn.is_some(),
            "CS merged keywords must contain 'churn' (added by preset)"
        );
        let (_, weight) = churn.unwrap();
        assert!(
            (*weight - 0.12).abs() < 0.001,
            "CS 'churn' weight should be 0.12, got {}",
            weight
        );
    }

    #[test]
    fn dos176_affiliates_preset_commission_in_merged_keywords() {
        let kws = merged_keywords_for_preset("affiliates");
        let commission = kws.iter().find(|(k, _)| k == "commission");
        assert!(
            commission.is_some(),
            "Affiliates merged keywords must contain 'commission'"
        );
        let (_, weight) = commission.unwrap();
        assert!(
            *weight > 0.0,
            "Affiliates 'commission' weight should be > 0, got {}",
            weight
        );
    }

    #[test]
    fn dos176_affiliates_preset_churn_absent_from_preset_specific_list() {
        // "churn" is not in the affiliates preset's intelligence.signal_keywords —
        // it is only added by the CS preset. It may appear in the base list
        // but must NOT appear in the affiliates preset's own config.
        let preset = crate::presets::loader::load_preset("affiliates")
            .expect("affiliates preset should load");
        let preset_has_churn = preset
            .intelligence
            .signal_keywords
            .iter()
            .any(|sk| sk.keyword == "churn");
        assert!(
            !preset_has_churn,
            "affiliates preset-specific signal_keywords must NOT include 'churn'"
        );
    }

    #[test]
    fn dos176_max_wins_merge_duplicate_keyword() {
        use crate::presets::schema::{PresetIntelligenceConfig, PresetSignalKeyword, RolePreset};
        // Construct a minimal preset with a keyword that duplicates a base keyword
        // ("renewal" appears in base at 0.15; preset adds it at 0.20 — max wins).
        let preset = RolePreset {
            id: "test".to_string(),
            name: "Test".to_string(),
            description: "test".to_string(),
            default_entity_mode: "account".to_string(),
            vocabulary: crate::presets::schema::PresetVocabulary {
                entity_noun: "account".to_string(),
                entity_noun_plural: "accounts".to_string(),
                primary_metric: "ARR".to_string(),
                health_label: "Health".to_string(),
                risk_label: "Risk".to_string(),
                success_verb: "retained".to_string(),
                cadence_noun: "QBR".to_string(),
            },
            vitals: crate::presets::schema::PresetVitalsConfig {
                account: vec![],
                project: vec![],
                person: vec![],
            },
            metadata: crate::presets::schema::PresetMetadataConfig {
                account: vec![],
                project: vec![],
                person: vec![],
            },
            stakeholder_roles: vec![],
            internal_team_roles: vec![],
            lifecycle_events: vec![],
            prioritization: crate::presets::schema::PresetPrioritization {
                primary_signal: "arr".to_string(),
                secondary_signal: "health".to_string(),
                urgency_drivers: vec![],
            },
            briefing_emphasis: String::new(),
            email_priority_keywords: vec![],
            intelligence: PresetIntelligenceConfig {
                signal_keywords: vec![
                    // "renewal" exists in base at 0.15; preset says 0.20 → max-wins: 0.20
                    PresetSignalKeyword { keyword: "renewal".to_string(), weight: 0.20 },
                    // "commission" is new (not in base)
                    PresetSignalKeyword { keyword: "commission".to_string(), weight: 0.11 },
                ],
                email_signal_types: vec![],
                email_priority_keywords: vec![],
                ..Default::default()
            },
        };

        let merged = crate::state::build_merged_signal_config(&preset);

        // "renewal" should have weight 0.20 (preset wins over base 0.15)
        let renewal = merged.signal_keywords.iter().find(|(k, _)| k == "renewal");
        assert!(renewal.is_some(), "merged list must contain 'renewal'");
        let (_, renewal_weight) = renewal.unwrap();
        assert!(
            (*renewal_weight - 0.20).abs() < 0.001,
            "max-wins: renewal should be 0.20 (preset overrides base 0.15), got {}",
            renewal_weight
        );

        // "commission" (preset-only) should be present
        let commission = merged.signal_keywords.iter().find(|(k, _)| k == "commission");
        assert!(commission.is_some(), "merged list must contain 'commission'");
    }

    #[test]
    fn dos176_cached_merge_updates_on_set_role() {
        // Verify that AppState::set_active_preset updates the cached merged config.
        // Load CS preset → expect 'churn'; load affiliates → expect 'commission'.
        let cs_preset = crate::presets::loader::load_preset("customer-success")
            .expect("CS preset should load");
        let aff_preset = crate::presets::loader::load_preset("affiliates")
            .expect("affiliates preset should load");

        let cs_merged = crate::state::build_merged_signal_config(&cs_preset);
        let aff_merged = crate::state::build_merged_signal_config(&aff_preset);

        assert!(
            cs_merged.signal_keywords.iter().any(|(k, _)| k == "churn"),
            "after set_role to CS, merged keywords must contain 'churn'"
        );
        assert!(
            aff_merged.signal_keywords.iter().any(|(k, _)| k == "commission"),
            "after set_role to affiliates, merged keywords must contain 'commission'"
        );
        // When using affiliates preset the preset-specific keywords should not include 'churn'
        assert!(
            !aff_preset.intelligence.signal_keywords.iter().any(|sk| sk.keyword == "churn"),
            "affiliates preset config should not include 'churn'"
        );
    }
}
